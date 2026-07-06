//! RCR-039 — group-commit / batched-fsync WAL proofs (persistence layer).
//!
//! These exercise `Wal::append_group` on the REAL fsync-durable `FileWal`:
//! durability + deterministic order equal N sequential appends, an un-acked
//! torn group truncates cleanly on recovery, and a foreign record is refused
//! before any byte is written. The fsync-COUNT proof (one sync per group, not
//! N) lives at the kernel layer over a counting `WalStore` double (a real fsync
//! cannot be counted from outside `std`); here we prove the durable BEHAVIOUR.

use arves_persistence::{
    ContentId, FileWal, FileWalStore, PendingRecord, RecordKind, ReplayCursor, ShardKey, Wal,
    WalError, WalRecord, WalStore,
};
use std::fs;
use std::path::{Path, PathBuf};

fn tmp(sub: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    p.push(sub);
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).expect("create tmp dir");
    p
}

fn shard(tenant: &str, workspace: &str) -> ShardKey {
    ShardKey { tenant: tenant.into(), workspace: workspace.into() }
}

fn rec(sh: &ShardKey, content: &[u8], payload: &[u8]) -> PendingRecord {
    PendingRecord {
        shard: sh.clone(),
        term: 0,
        kind: RecordKind::Outcome,
        content: ContentId(content.to_vec()),
        payload: payload.to_vec(),
    }
}

fn drain(wal: &FileWal) -> Vec<WalRecord> {
    let mut cur = wal.replay_from(0).expect("replay");
    let mut out = Vec::new();
    while let Some(r) = cur.next().expect("cursor") {
        out.push(r);
    }
    out
}

fn wal_file(root: &Path) -> PathBuf {
    for e in fs::read_dir(root).expect("read root").flatten() {
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            for f in fs::read_dir(e.path()).expect("read shard dir").flatten() {
                let p = f.path();
                if p.extension().map(|x| x == "wal").unwrap_or(false) {
                    return p;
                }
            }
        }
    }
    panic!("no wal segment under {root:?}");
}

/// A group of N records is durable and recovers, in order, from a FRESH store —
/// proving the coalesced single fsync loses nothing (no acked-but-lost truth).
#[test]
fn group_append_is_durable_and_recovers_in_order() {
    let root = tmp("rcr039-durable");
    let sh = shard("acme", "w1");
    let store = FileWalStore::open_root(&root).unwrap();
    let mut wal = store.open(&sh).unwrap();

    let group: Vec<PendingRecord> = (0..8u8)
        .map(|i| rec(&sh, &[i], &[i, i, i]))
        .collect();
    let offsets = wal.append_group(group).expect("group append");
    assert_eq!(offsets, (0..8).collect::<Vec<_>>(), "dense offsets in input order");
    assert_eq!(wal.head(), 8);

    // A FRESH store over the same durable root recovers every record, in order.
    let store2 = FileWalStore::open_root(&root).unwrap();
    let wal2 = store2.open(&sh).unwrap();
    let recs = drain(&wal2);
    assert_eq!(recs.len(), 8, "all group records durable after coalesced fsync");
    for (i, r) in recs.iter().enumerate() {
        assert_eq!(r.offset, i as u64);
        assert_eq!(r.content.0, vec![i as u8]);
        assert_eq!(r.payload, vec![i as u8; 3]);
    }
    let _ = fs::remove_dir_all(&root);
}

/// `append_group` produces the byte-identical durable log as N sequential
/// `append`s — determinism: committed content + order are a pure function of the
/// input, independent of the number of syncs.
#[test]
fn group_append_bytes_identical_to_sequential() {
    let sh = shard("acme", "w1");

    let root_g = tmp("rcr039-eq-group");
    let store_g = FileWalStore::open_root(&root_g).unwrap();
    let mut wal_g = store_g.open(&sh).unwrap();
    let group: Vec<PendingRecord> = (0..12u8).map(|i| rec(&sh, &[i, 0xAA], &[i; 5])).collect();
    wal_g.append_group(group).unwrap();

    let root_s = tmp("rcr039-eq-seq");
    let store_s = FileWalStore::open_root(&root_s).unwrap();
    let mut wal_s = store_s.open(&sh).unwrap();
    for i in 0..12u8 {
        wal_s.append(rec(&sh, &[i, 0xAA], &[i; 5])).unwrap();
    }

    // Same decoded records ...
    assert_eq!(drain(&wal_g), drain(&wal_s), "group == sequential records");
    // ... AND byte-identical segment files (framing is deterministic).
    let bytes_g = fs::read(wal_file(&root_g)).unwrap();
    let bytes_s = fs::read(wal_file(&root_s)).unwrap();
    assert_eq!(bytes_g, bytes_s, "group segment bytes == sequential segment bytes");

    let _ = fs::remove_dir_all(&root_g);
    let _ = fs::remove_dir_all(&root_s);
}

/// A crash mid-group (before the coalesced fsync makes the tail durable) is
/// modeled by truncating the segment inside the last frame. Recovery truncates
/// the torn tail and replays the intact prefix cleanly — an un-acked group does
/// not corrupt truth (lossless-or-loud; no gap).
#[test]
fn torn_group_tail_truncates_cleanly() {
    let root = tmp("rcr039-torn");
    let sh = shard("acme", "w1");
    let store = FileWalStore::open_root(&root).unwrap();
    let mut wal = store.open(&sh).unwrap();
    wal.append_group((0..6u8).map(|i| rec(&sh, &[i], &[i; 4])).collect()).unwrap();

    // Simulate a crash that persisted only a prefix of the group: lop the last
    // few bytes off the segment (a torn final frame).
    let seg = wal_file(&root);
    let bytes = fs::read(&seg).unwrap();
    assert!(bytes.len() > 6, "segment has content");
    fs::write(&seg, &bytes[..bytes.len() - 3]).unwrap();

    // Recovery drops the torn tail, keeps the intact prefix dense + gap-free.
    let store2 = FileWalStore::open_root(&root).unwrap();
    let wal2 = store2.open(&sh).unwrap();
    let recs = drain(&wal2);
    assert!(recs.len() < 6, "the torn last frame is truncated away");
    for (i, r) in recs.iter().enumerate() {
        assert_eq!(r.offset, i as u64, "recovered prefix is dense and in order");
    }
    assert_eq!(wal2.head(), recs.len() as u64, "head matches the intact prefix");
    let _ = fs::remove_dir_all(&root);
}

/// A group carrying a foreign-shard record is refused BEFORE any byte is written
/// (SHARD-001): the WAL stays empty, no half-written tail.
#[test]
fn group_append_foreign_shard_refused_before_write() {
    let root = tmp("rcr039-foreign");
    let sh = shard("acme", "w1");
    let other = shard("acme", "w2");
    let store = FileWalStore::open_root(&root).unwrap();
    let mut wal = store.open(&sh).unwrap();

    let group = vec![
        rec(&sh, b"a", b"a"),
        rec(&other, b"b", b"b"), // foreign — poisons the group
        rec(&sh, b"c", b"c"),
    ];
    match wal.append_group(group) {
        Err(WalError::UnknownShard(k)) => assert_eq!(k, other),
        other => panic!("expected UnknownShard, got {other:?}"),
    }
    assert_eq!(wal.head(), 0, "nothing written for a refused group");
    assert!(drain(&wal).is_empty());
    let _ = fs::remove_dir_all(&root);
}

/// A group that crosses a segment-rotation boundary is fully durable: the
/// outgoing segment is synced before it is sealed, so no frame is lost.
#[test]
fn group_append_across_rotation_is_durable() {
    let root = tmp("rcr039-rotate");
    let sh = shard("acme", "w1");
    // rotate_every = 3 forces the 7-record group to span three segments.
    let store = FileWalStore::open_root_with_rotation(&root, 3).unwrap();
    let mut wal = store.open(&sh).unwrap();
    wal.append_group((0..7u8).map(|i| rec(&sh, &[i], &[i; 2])).collect()).unwrap();

    let store2 = FileWalStore::open_root_with_rotation(&root, 3).unwrap();
    let wal2 = store2.open(&sh).unwrap();
    let recs = drain(&wal2);
    assert_eq!(recs.len(), 7, "every frame durable across the rotation boundary");
    for (i, r) in recs.iter().enumerate() {
        assert_eq!(r.offset, i as u64);
    }
    let _ = fs::remove_dir_all(&root);
}

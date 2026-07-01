//! I1.5 Persistent WAL - persistence-layer crash-consistency proofs.
//!
//! These exercise the durable substrate directly (no Kernel): disk round-trips
//! through a FRESH store instance, torn-tail truncation, corrupt-frame drop,
//! per-shard isolation, and the SHARD-001 wrong-shard append guard.

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
    ShardKey {
        tenant: tenant.into(),
        workspace: workspace.into(),
    }
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

/// Find the single segment file for a single-shard, sub-rotation-limit test.
/// The shard is a subdirectory of `root` holding `seg-*.wal` files.
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
    panic!("no seg .wal file under {root:?}");
}

/// A committed record is durable on disk before `append` returns: a SEPARATE
/// fresh store (empty cache) sees it immediately, proving fsync, not Arc-sharing.
#[test]
fn append_is_durable_before_return() {
    let dir = tmp("durable_before_return");
    let sh = shard("t1", "w1");

    let store1 = FileWalStore::open_root(&dir).unwrap();
    let mut wal1 = store1.open(&sh).unwrap();
    let off = wal1.append(rec(&sh, b"c1", b"p1")).unwrap();
    assert_eq!(off, 0);

    // Fresh store instance -> empty cache -> must read from disk.
    let store2 = FileWalStore::open_root(&dir).unwrap();
    let wal2 = store2.open(&sh).unwrap();
    assert_eq!(wal2.head(), 1, "record is durable before append returned");
    let recs = drain(&wal2);
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].content.0, b"c1");
    assert_eq!(recs[0].payload, b"p1");
    assert_eq!(recs[0].offset, 0);
}

/// Disk round-trip: append, drop all handles (== process exit), re-open a fresh
/// store, and read back byte-identical records in order.
#[test]
fn round_trip_survives_fresh_store() {
    let dir = tmp("round_trip");
    let sh = shard("t1", "w1");
    {
        let store = FileWalStore::open_root(&dir).unwrap();
        let mut wal = store.open(&sh).unwrap();
        assert_eq!(wal.append(rec(&sh, b"c0", b"p0")).unwrap(), 0);
        assert_eq!(wal.append(rec(&sh, b"c1", b"p1")).unwrap(), 1);
        assert_eq!(wal.append(rec(&sh, b"c2", b"p2")).unwrap(), 2);
    } // handles dropped

    let store = FileWalStore::open_root(&dir).unwrap();
    let wal = store.open(&sh).unwrap();
    assert_eq!(wal.head(), 3);
    let recs = drain(&wal);
    assert_eq!(recs.len(), 3);
    for (i, r) in recs.iter().enumerate() {
        assert_eq!(r.offset, i as u64);
        assert_eq!(r.content.0, format!("c{i}").into_bytes());
        assert_eq!(r.payload, format!("p{i}").into_bytes());
    }
}

/// A torn length prefix at the tail (crash mid-append) is detected and the
/// garbage is truncated on open; committed truth before the tear is intact.
#[test]
fn torn_tail_is_truncated() {
    let dir = tmp("torn_tail");
    let sh = shard("t1", "w1");
    {
        let store = FileWalStore::open_root(&dir).unwrap();
        let mut wal = store.open(&sh).unwrap();
        wal.append(rec(&sh, b"c0", b"p0")).unwrap();
        wal.append(rec(&sh, b"c1", b"p1")).unwrap();
    }
    // Append a bogus frame header claiming a huge body -> torn tail.
    let path = wal_file(&dir);
    let mut bytes = fs::read(&path).unwrap();
    let good_len = bytes.len();
    bytes.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0x7F]); // length ~2.1 GiB, no body
    fs::write(&path, &bytes).unwrap();

    let store = FileWalStore::open_root(&dir).unwrap();
    let wal = store.open(&sh).unwrap();
    assert_eq!(wal.head(), 2, "torn tail dropped, good records kept");
    assert_eq!(drain(&wal).len(), 2);
    // The file was physically truncated back to the good prefix.
    assert_eq!(fs::metadata(&path).unwrap().len() as usize, good_len);
}

/// A bit-flip in the last committed frame fails its CRC; that frame and anything
/// after it are dropped. Earlier truth is untouched (never serve corrupt truth).
#[test]
fn corrupt_last_frame_is_dropped() {
    let dir = tmp("corrupt_frame");
    let sh = shard("t1", "w1");
    {
        let store = FileWalStore::open_root(&dir).unwrap();
        let mut wal = store.open(&sh).unwrap();
        wal.append(rec(&sh, b"c0", b"p0")).unwrap();
        wal.append(rec(&sh, b"c1", b"p1")).unwrap();
        wal.append(rec(&sh, b"c2", b"p2")).unwrap();
    }
    let path = wal_file(&dir);
    let mut bytes = fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF; // corrupt the last frame's trailing CRC
    fs::write(&path, &bytes).unwrap();

    let store = FileWalStore::open_root(&dir).unwrap();
    let wal = store.open(&sh).unwrap();
    assert_eq!(wal.head(), 2, "corrupt frame and suffix dropped");
    let recs = drain(&wal);
    assert_eq!(recs.len(), 2);
    assert_eq!(recs[0].content.0, b"c0");
    assert_eq!(recs[1].content.0, b"c1");
}

/// SHARD-001: each shard is an independent file; both survive and recover their
/// own records; `shards()` enumerates them deterministically from disk.
#[test]
fn multi_shard_isolation_survives_disk() {
    let dir = tmp("multi_shard");
    let a = shard("t1", "w1");
    let b = shard("t2", "w2");
    {
        let store = FileWalStore::open_root(&dir).unwrap();
        let mut wa = store.open(&a).unwrap();
        wa.append(rec(&a, b"a0", b"pa0")).unwrap();
        wa.append(rec(&a, b"a1", b"pa1")).unwrap();
        let mut wb = store.open(&b).unwrap();
        wb.append(rec(&b, b"b0", b"pb0")).unwrap();
    }

    let store = FileWalStore::open_root(&dir).unwrap();
    let mut shards = store.shards();
    shards.sort();
    assert_eq!(shards, vec![a.clone(), b.clone()], "both shards discovered");

    let wa = store.open(&a).unwrap();
    assert_eq!(wa.head(), 2);
    assert_eq!(drain(&wa).len(), 2);
    let wb = store.open(&b).unwrap();
    assert_eq!(wb.head(), 1);
    let rb = drain(&wb);
    assert_eq!(rb.len(), 1);
    assert_eq!(rb[0].content.0, b"b0");
    assert_eq!(rb[0].shard, b, "shard key round-trips through the filename");
}

/// SHARD-001: a WAL refuses a record for a different shard (no cross-shard append).
#[test]
fn wrong_shard_append_rejected() {
    let dir = tmp("wrong_shard");
    let a = shard("t1", "w1");
    let b = shard("t2", "w2");
    let store = FileWalStore::open_root(&dir).unwrap();
    let mut wa = store.open(&a).unwrap();
    match wa.append(rec(&b, b"x", b"y")) {
        Err(WalError::UnknownShard(s)) => assert_eq!(s, b),
        other => panic!("expected UnknownShard, got {other:?}"),
    }
}

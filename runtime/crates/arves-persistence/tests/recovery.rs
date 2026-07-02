//! I1.7 Recovery hardening - persistence-layer fault-injection proofs.
//!
//! From the adversarial recovery hunt: interior-segment corruption must be
//! detected (not silently dropped), and orphan checkpoint .tmp files must be
//! swept on open.

use arves_persistence::{
    ContentId, FileWalStore, PendingRecord, RecordKind, ReplayCursor, ShardKey, Wal, WalError,
    WalStore,
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

fn shard() -> ShardKey {
    ShardKey {
        tenant: "t1".into(),
        workspace: "w1".into(),
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

fn shard_dir(root: &Path) -> PathBuf {
    fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("a shard dir exists")
}

fn seg_path(dir: &Path, start: u64) -> PathBuf {
    dir.join(format!("seg-{start:020}.wal"))
}

fn flip_byte(path: &Path, idx: usize) {
    let mut b = fs::read(path).unwrap();
    b[idx] ^= 0xFF;
    fs::write(path, &b).unwrap();
}

/// Interior (sealed, non-last) segment corruption is DETECTED by replay_from,
/// which fails loudly with Corruption rather than returning a gapped trace.
#[test]
fn interior_segment_corruption_is_detected() {
    let dir = tmp("interior_corruption");
    let sh = shard();
    {
        let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
        let mut wal = store.open(&sh).unwrap();
        for i in 0..6u64 {
            wal.append(rec(&sh, format!("c{i}").as_bytes(), b"p")).unwrap();
        }
        // segments: seg-0[0,1], seg-2[2,3], seg-4[4,5]
    }
    // Corrupt the FIRST frame body of the interior sealed segment seg-2.
    let sd = shard_dir(&dir);
    flip_byte(&seg_path(&sd, 2), 8);

    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let wal = store.open(&sh).unwrap();
    // head is still 6 (open only inspects the last segment), but the range is now
    // gapped at offset 2 -> replay must refuse, not silently drop 2,3.
    match wal.replay_from(0) {
        Err(WalError::Corruption { missing_offset, .. }) => {
            assert_eq!(missing_offset, 2, "gap detected exactly at the corrupt frame")
        }
        other => panic!("expected Corruption, got {other:?}"),
    }
}

/// A healthy multi-segment log replays completely (no false Corruption).
#[test]
fn healthy_multisegment_replay_is_complete() {
    let dir = tmp("healthy_multiseg");
    let sh = shard();
    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let mut wal = store.open(&sh).unwrap();
    for i in 0..5u64 {
        wal.append(rec(&sh, format!("c{i}").as_bytes(), b"p")).unwrap();
    }
    let mut cur = wal.replay_from(0).expect("replay ok");
    let mut n = 0u64;
    while let Some(r) = cur.next().expect("cursor") {
        assert_eq!(r.offset, n);
        n += 1;
    }
    assert_eq!(n, 5, "all records replayed contiguously");
}

/// Orphan `snap-<N>.snap.tmp` files (crash in install_snapshot's fsync->rename
/// window) are swept on open, so they cannot leak without bound.
#[test]
fn orphan_snapshot_tmp_is_swept_on_open() {
    let dir = tmp("orphan_tmp");
    let sh = shard();
    {
        let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
        let mut wal = store.open(&sh).unwrap();
        wal.append(rec(&sh, b"c0", b"p0")).unwrap();
    }
    // Simulate a crash between snapshot fsync and rename.
    let sd = shard_dir(&dir);
    let orphan = sd.join("snap-00000000000000000009.snap.tmp");
    fs::write(&orphan, b"partial-unrenamed-snapshot").unwrap();
    assert!(orphan.exists());

    // Opening the store (recovery entry) must reclaim the orphan.
    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let _wal = store.open(&sh).unwrap();
    assert!(!orphan.exists(), "orphan .snap.tmp swept on open");
}

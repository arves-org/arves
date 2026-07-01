//! I1.6 Checkpoint Semantics - persistence-layer proofs:
//! segment rotation, snapshot install/load, and segment-delete compaction.

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

fn drain_from(wal: &FileWal, from: u64) -> Vec<WalRecord> {
    let mut cur = wal.replay_from(from).expect("replay");
    let mut out = Vec::new();
    while let Some(r) = cur.next().expect("cursor") {
        out.push(r);
    }
    out
}

fn shard_dir(root: &Path) -> PathBuf {
    fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("a shard dir exists")
}

fn count_segments(root: &Path) -> usize {
    fs::read_dir(shard_dir(root))
        .unwrap()
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| x == "wal")
                .unwrap_or(false)
        })
        .count()
}

/// Rotation splits the log into multiple segment files; replay still yields the
/// full, in-order, contiguous record stream across segments.
#[test]
fn segments_rotate_and_replay_contiguously() {
    let dir = tmp("rotate_replay");
    let sh = shard();
    {
        let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
        let mut wal = store.open(&sh).unwrap();
        for i in 0..5u64 {
            let off = wal
                .append(rec(&sh, format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
                .unwrap();
            assert_eq!(off, i);
        }
    }
    // 5 records, 2 per segment -> segments starting at 0,2,4 => 3 files.
    assert_eq!(count_segments(&dir), 3, "log rotated into 3 segments");

    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let wal = store.open(&sh).unwrap();
    assert_eq!(wal.head(), 5);
    let recs = drain_from(&wal, 0);
    assert_eq!(recs.len(), 5);
    for (i, r) in recs.iter().enumerate() {
        assert_eq!(r.offset, i as u64);
        assert_eq!(r.content.0, format!("c{i}").into_bytes());
    }
}

/// A checkpoint is durable and reloadable; compaction deletes only the sealed
/// segments fully covered by it, advancing `earliest`, keeping the tail.
#[test]
fn checkpoint_then_compaction_deletes_covered_segments() {
    let dir = tmp("compact_segments");
    let sh = shard();
    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let mut wal = store.open(&sh).unwrap();
    for i in 0..6u64 {
        wal.append(rec(&sh, format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
            .unwrap();
    }
    // segments: [0,1] [2,3] [4,5]; current segment starts at 4.
    assert_eq!(count_segments(&dir), 3);
    assert_eq!(wal.head(), 6);

    // Kernel would produce the blob; here we store opaque bytes.
    let meta = wal.install_snapshot(3, 7, b"opaque-state-through-3").unwrap();
    assert_eq!(meta.up_to_offset, 3);
    wal.compact(3).unwrap();

    // Segments [0,1] and [2,3] are fully covered (end <= 3) -> deleted. [4,5] kept.
    assert_eq!(count_segments(&dir), 1, "two covered segments deleted");
    assert_eq!(wal.earliest(), 4, "earliest advanced past the checkpoint");
    assert_eq!(wal.head(), 6, "head unchanged by compaction");

    // Tail is intact; pre-checkpoint offsets are gone (live in the snapshot).
    let tail = drain_from(&wal, 4);
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].offset, 4);
    match wal.replay_from(0) {
        Err(WalError::OffsetCompacted { earliest, .. }) => assert_eq!(earliest, 4),
        other => panic!("expected OffsetCompacted, got {other:?}"),
    }
}

/// The checkpoint blob survives to a fresh store and loads back byte-identically.
#[test]
fn snapshot_survives_fresh_store() {
    let dir = tmp("snap_survives");
    let sh = shard();
    let blob = b"kernel-materialized-state";
    {
        let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
        let mut wal = store.open(&sh).unwrap();
        for i in 0..4u64 {
            wal.append(rec(&sh, format!("c{i}").as_bytes(), b"p")).unwrap();
        }
        wal.install_snapshot(3, 9, blob).unwrap();
    }
    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let wal = store.open(&sh).unwrap();
    let (meta, loaded) = wal.load_snapshot().unwrap().expect("snapshot present");
    assert_eq!(meta.up_to_offset, 3);
    assert_eq!(meta.term, 9);
    assert_eq!(loaded, blob, "opaque blob round-trips byte-identically");
}

/// A torn/corrupt checkpoint is ignored on load (never restore corruption).
#[test]
fn corrupt_snapshot_is_ignored() {
    let dir = tmp("snap_corrupt");
    let sh = shard();
    {
        let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
        let mut wal = store.open(&sh).unwrap();
        wal.append(rec(&sh, b"c0", b"p0")).unwrap();
        wal.install_snapshot(0, 1, b"good-state").unwrap();
    }
    // Corrupt the snapshot file's last byte (its CRC).
    let sd = shard_dir(&dir);
    let snap = fs::read_dir(&sd)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "snap").unwrap_or(false))
        .expect("snap file");
    let mut bytes = fs::read(&snap).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    fs::write(&snap, &bytes).unwrap();

    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let wal = store.open(&sh).unwrap();
    assert!(
        wal.load_snapshot().unwrap().is_none(),
        "corrupt checkpoint is not loaded"
    );
}

/// Compaction past the committed head is rejected.
#[test]
fn compact_past_head_rejected() {
    let dir = tmp("compact_past_head");
    let sh = shard();
    let store = FileWalStore::open_root_with_rotation(&dir, 2).unwrap();
    let mut wal = store.open(&sh).unwrap();
    wal.append(rec(&sh, b"c0", b"p0")).unwrap(); // head = 1
    match wal.compact(5) {
        Err(WalError::OffsetOutOfRange { head, .. }) => assert_eq!(head, 1),
        other => panic!("expected OffsetOutOfRange, got {other:?}"),
    }
}

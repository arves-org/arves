//! I1.5 Persistent WAL - Kernel behaviour proofs over the FILE-backed store.
//!
//! These extend the I1.4 walking-skeleton behaviours to real durability:
//!   B7  commit persists a non-empty WAL file on disk.
//!   B8  a fresh Kernel over a fresh store recovers identical truth from disk
//!       (real disk round-trip, NOT the I1.4 Arc-sharing trick).
//!   B9  idempotent commit writes a single durable record.
//!   B10 crash-consistency: a corrupt tail is dropped; truth before it is intact.
//! No Raft, networking, or replication is exercised.

use arves_kernel::{ContentHash, FileKernel, Kernel, ProposedWrite, ShardKey};
use arves_persistence::FileWalStore;
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

fn proposal(content: &[u8], payload: &[u8]) -> ProposedWrite {
    ProposedWrite {
        shard: shard(),
        content: ContentHash(content.to_vec()),
        payload: payload.to_vec(),
    }
}

/// Find the single segment file for a single-shard, sub-rotation-limit test.
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

/// B7: committing produces a non-empty on-disk WAL file.
#[test]
fn behaviour_7_commit_persists_wal_file() {
    let dir = tmp("b7_persist_file");
    let k = FileKernel::new(FileWalStore::open_root(&dir).unwrap());
    k.commit(proposal(b"c1", b"p1")).expect("commit ok");
    let path = wal_file(&dir);
    let len = fs::metadata(&path).unwrap().len();
    assert!(len > 0, "WAL file is non-empty after commit ({len} bytes)");
}

/// B8: a fresh Kernel over a fresh store recovers identical truth from disk.
#[test]
fn behaviour_8_fresh_process_recovers_identical_truth() {
    let dir = tmp("b8_disk_round_trip");
    let expected_hash;
    let expected_count;
    {
        let k1 = FileKernel::new(FileWalStore::open_root(&dir).unwrap());
        k1.commit(proposal(b"c1", b"p1")).unwrap();
        k1.commit(proposal(b"c2", b"p2")).unwrap();
        expected_hash = k1.truth_hash();
        expected_count = k1.committed_count();
        // k1 + its store dropped here == process exit; only the file remains.
    }
    let k2 = FileKernel::recover(FileWalStore::open_root(&dir).unwrap());
    assert_eq!(k2.committed_count(), expected_count);
    assert_eq!(k2.committed_count(), 2);
    assert_eq!(
        k2.truth_hash(),
        expected_hash,
        "truth recovered from disk equals committed truth"
    );
}

/// B9: an idempotent re-commit does not write a second durable record.
#[test]
fn behaviour_9_idempotent_commit_single_record() {
    let dir = tmp("b9_idempotent");
    {
        let k1 = FileKernel::new(FileWalStore::open_root(&dir).unwrap());
        k1.commit(proposal(b"c1", b"p1")).unwrap();
        // Same content again: ORCH-004 no-op, must not append a second frame.
        assert!(k1.commit(proposal(b"c1", b"p1")).is_err());
        assert_eq!(k1.committed_count(), 1);
    }
    let k2 = FileKernel::recover(FileWalStore::open_root(&dir).unwrap());
    assert_eq!(k2.committed_count(), 1, "only one record was ever durable");
}

/// B10: crash-consistency - a corrupt tail frame is dropped on recovery, and the
/// truth committed before it is byte-identical to a clean 2-commit Kernel.
#[test]
fn behaviour_10_corrupt_tail_preserves_prior_truth() {
    let dir = tmp("b10_corrupt_tail");
    {
        let k1 = FileKernel::new(FileWalStore::open_root(&dir).unwrap());
        k1.commit(proposal(b"c1", b"p1")).unwrap();
        k1.commit(proposal(b"c2", b"p2")).unwrap();
        k1.commit(proposal(b"c3", b"p3")).unwrap();
    }
    // Corrupt the last frame on disk (simulated partial/garbled write).
    let path = wal_file(&dir);
    let mut bytes = fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    fs::write(&path, &bytes).unwrap();

    let recovered = FileKernel::recover(FileWalStore::open_root(&dir).unwrap());
    assert_eq!(recovered.committed_count(), 2, "corrupt 3rd frame dropped");

    // Reference: a clean Kernel that committed exactly the first two proposals.
    let clean_dir = tmp("b10_reference");
    let clean = FileKernel::new(FileWalStore::open_root(&clean_dir).unwrap());
    clean.commit(proposal(b"c1", b"p1")).unwrap();
    clean.commit(proposal(b"c2", b"p2")).unwrap();
    assert_eq!(
        recovered.truth_hash(),
        clean.truth_hash(),
        "surviving truth equals a clean two-commit history"
    );
}

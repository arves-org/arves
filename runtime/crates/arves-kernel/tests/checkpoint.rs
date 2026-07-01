//! I1.6 Checkpoint Semantics - Kernel behaviour proofs over the file store.
//!
//! The milestone's headline guarantee: `checkpoint + tail replay` reproduces the
//! SAME truth as `full replay` (deterministic recovery), while compaction bounds
//! the on-disk WAL. Binding requirement #6 of the ratified design.
//!   B12 checkpoint then recover == full replay (all covered by snapshot).
//!   B13 checkpoint + later commits (tail) then recover == clean full history.
//!   B14 compaction reclaims sealed segments; recovery still exact.
//!   B15 checkpoint is idempotent (twice) and recovery is stable.

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

/// Small rotation threshold so a handful of commits exercises segmentation.
fn store(dir: &Path) -> FileWalStore {
    FileWalStore::open_root_with_rotation(dir, 2).unwrap()
}

fn count_segments(root: &Path) -> usize {
    let shard_dir = fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("shard dir");
    fs::read_dir(shard_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "wal").unwrap_or(false))
        .count()
}

/// Reference truth hash for committing `n` proposals (c0..c{n-1}) in a clean dir.
fn clean_hash(dir: &Path, n: usize) -> u64 {
    let k = FileKernel::new(store(dir));
    for i in 0..n {
        k.commit(proposal(format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
            .unwrap();
    }
    k.truth_hash()
}

/// B12: checkpoint (covering everything) then recover == full replay.
#[test]
fn behaviour_12_checkpoint_then_recover_equals_full_replay() {
    let dir = tmp("b12_ckpt_recover");
    let expected;
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..5 {
            k.commit(proposal(format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
                .unwrap();
        }
        expected = k.truth_hash();
        assert_eq!(k.checkpoint().unwrap(), 1, "one shard checkpointed");
    }
    let recovered = FileKernel::recover(store(&dir));
    assert_eq!(recovered.committed_count(), 5);
    assert_eq!(
        recovered.truth_hash(),
        expected,
        "checkpoint + tail replay reproduces full truth"
    );
}

/// B13: checkpoint, then MORE commits (a genuine tail), then recover == a clean
/// full history of all commits.
#[test]
fn behaviour_13_checkpoint_plus_tail_recovers_full_history() {
    let dir = tmp("b13_ckpt_tail");
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..3 {
            k.commit(proposal(format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
                .unwrap();
        }
        k.checkpoint().unwrap();
        // Tail: commits after the checkpoint.
        for i in 3..5 {
            k.commit(proposal(format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
                .unwrap();
        }
    }
    let recovered = FileKernel::recover(store(&dir));
    assert_eq!(recovered.committed_count(), 5);

    let clean_dir = tmp("b13_reference");
    assert_eq!(
        recovered.truth_hash(),
        clean_hash(&clean_dir, 5),
        "snapshot(0..2) + tail(3..4) == clean 5-commit history"
    );
}

/// B14: compaction reclaims sealed segments; recovery remains exact afterward.
#[test]
fn behaviour_14_compaction_reclaims_segments() {
    let dir = tmp("b14_reclaim");
    let expected;
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..6 {
            k.commit(proposal(format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
                .unwrap();
        }
        expected = k.truth_hash();
        assert_eq!(count_segments(&dir), 3, "3 segments before checkpoint");
        k.checkpoint().unwrap();
    }
    // Compaction deleted the sealed covered segments (kept only the current one).
    assert_eq!(count_segments(&dir), 1, "segments reclaimed after checkpoint");

    let recovered = FileKernel::recover(store(&dir));
    assert_eq!(recovered.committed_count(), 6);
    assert_eq!(recovered.truth_hash(), expected, "recovery exact after compaction");
}

/// B15: checkpoint is idempotent (twice) and recovery is stable across repeats.
#[test]
fn behaviour_15_idempotent_checkpoint_and_recovery() {
    let dir = tmp("b15_idempotent");
    let expected;
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..4 {
            k.commit(proposal(format!("c{i}").as_bytes(), format!("p{i}").as_bytes()))
                .unwrap();
        }
        expected = k.truth_hash();
        k.checkpoint().unwrap();
        k.checkpoint().unwrap(); // second checkpoint: must not corrupt or duplicate
    }
    let r1 = FileKernel::recover(store(&dir));
    assert_eq!(r1.committed_count(), 4);
    assert_eq!(r1.truth_hash(), expected);

    // Recover again from the same on-disk state: identical.
    let r2 = FileKernel::recover(store(&dir));
    assert_eq!(r2.truth_hash(), expected, "repeated recovery is stable");
}

//! I1.7 Recovery hardening - Kernel fault-injection proofs ("lossless or loud").
//!
//! Each test targets a defect the adversarial recovery hunt CONFIRMED, proving
//! recovery now either restores all committed truth or fails loudly - never
//! silently returns partial truth, and never panics on a recoverable state.
//!   R-A  compacted prefix + lost snapshot  -> loud RecoveryError (was: silent loss)
//!   R-B  interior segment corruption       -> loud RecoveryError (was: silent gap)
//!   R-C  current-segment corruption + snap  -> lossless restore  (was: panic/brick)
//!   R-OK healthy checkpointed store         -> Ok (fallible path returns success)

use arves_kernel::{ContentHash, FileKernel, Kernel, ProposedWrite, RecoveryError, ShardKey};
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
    ShardKey::new("t1", "w1").expect("valid test shard")
}

fn proposal(content: &[u8], payload: &[u8]) -> ProposedWrite {
    ProposedWrite {
        shard: shard(),
        content: ContentHash(content.to_vec()),
        payload: payload.to_vec(),
    }
}

fn store(dir: &Path) -> FileWalStore {
    FileWalStore::open_root_with_rotation(dir, 2).unwrap()
}

fn shard_dir(root: &Path) -> PathBuf {
    fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir())
        .expect("shard dir")
}

fn seg_path(dir: &Path, start: u64) -> PathBuf {
    dir.join(format!("seg-{start:020}.wal"))
}

fn snap_file(dir: &Path) -> PathBuf {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "snap").unwrap_or(false))
        .expect("a .snap file")
}

fn flip_byte(path: &Path, idx: usize) {
    let mut b = fs::read(path).unwrap();
    b[idx] ^= 0xFF;
    fs::write(path, &b).unwrap();
}

fn flip_last(path: &Path) {
    let n = fs::metadata(path).unwrap().len() as usize;
    flip_byte(path, n - 1);
}

/// Defect A: a compacted prefix whose snapshot is later lost/corrupt must make
/// recovery FAIL LOUDLY, not silently return the surviving tail as if complete.
#[test]
fn recovery_refuses_when_compacted_prefix_snapshot_lost() {
    let dir = tmp("rA_prefix_lost");
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..4 {
            k.commit(proposal(format!("c{i}").as_bytes(), b"p")).unwrap();
        }
        // seg-0[0,1] seg-2[2,3]; checkpoint compacts seg-0 -> [0,1] live only in snap.
        k.checkpoint().unwrap();
    }
    // Corrupt the sole checkpoint: prefix [0,1] is now unrecoverable.
    flip_last(&snap_file(&shard_dir(&dir)));

    match FileKernel::try_recover(store(&dir)) {
        Err(RecoveryError::CompactedPrefixWithoutSnapshot { earliest, .. }) => {
            assert_eq!(earliest, 2, "recovery refuses; prefix below offset 2 is lost")
        }
        Err(other) => panic!("expected CompactedPrefixWithoutSnapshot, got {other:?}"),
        Ok(_) => panic!("recovery must refuse, but it silently succeeded"),
    }
}

/// Defect B: corruption of an interior (sealed) segment must make recovery FAIL
/// LOUDLY with a detected gap, not silently reconstruct a truth set with a hole.
#[test]
fn recovery_refuses_on_interior_segment_corruption() {
    let dir = tmp("rB_interior_corrupt");
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..6 {
            k.commit(proposal(format!("c{i}").as_bytes(), b"p")).unwrap();
        }
        // seg-0[0,1] seg-2[2,3] seg-4[4,5]; no checkpoint.
    }
    flip_byte(&seg_path(&shard_dir(&dir), 2), 8); // corrupt interior seg-2

    match FileKernel::try_recover(store(&dir)) {
        Err(RecoveryError::Corruption { missing_offset, .. }) => {
            assert_eq!(missing_offset, 2, "gap detected exactly at the corrupt frame")
        }
        Err(other) => panic!("expected Corruption, got {other:?}"),
        Ok(_) => panic!("recovery must refuse on a gapped log, but it succeeded"),
    }
}

/// Defect C: if the current segment is corrupt but a snapshot covers all
/// committed truth, recovery must RESTORE LOSSLESSLY from the snapshot - never
/// panic because snapshot.up_to+1 exceeds the (truncated) recovered head.
#[test]
fn recovery_survives_current_segment_corruption_via_snapshot() {
    let dir = tmp("rC_current_corrupt");
    let expected;
    {
        let k = FileKernel::new(FileWalStore::open_root_with_rotation(&dir, 1024).unwrap());
        k.commit(proposal(b"c0", b"p0")).unwrap();
        k.commit(proposal(b"c1", b"p1")).unwrap();
        expected = k.truth_hash();
        k.checkpoint().unwrap(); // snapshot up_to=1; seg-0 kept (it is current)
    }
    // Corrupt the FIRST frame of the current segment; open() will truncate it to
    // empty (head=0), but the snapshot still holds offsets 0 and 1.
    flip_byte(&seg_path(&shard_dir(&dir), 0), 8);

    let recovered = FileKernel::try_recover(FileWalStore::open_root_with_rotation(&dir, 1024).unwrap())
        .expect("recovery must succeed from the snapshot (no panic)");
    assert_eq!(recovered.committed_count(), 2, "both truths restored from snapshot");

    let clean_dir = tmp("rC_reference");
    let clean = FileKernel::new(FileWalStore::open_root_with_rotation(&clean_dir, 1024).unwrap());
    clean.commit(proposal(b"c0", b"p0")).unwrap();
    clean.commit(proposal(b"c1", b"p1")).unwrap();
    assert_eq!(recovered.truth_hash(), expected);
    assert_eq!(recovered.truth_hash(), clean.truth_hash(), "restored truth is exact");
}

/// Sanity: the fallible recovery path returns Ok on a healthy checkpointed store.
#[test]
fn try_recover_ok_on_healthy_checkpointed_store() {
    let dir = tmp("rOK_healthy");
    {
        let k = FileKernel::new(store(&dir));
        for i in 0..4 {
            k.commit(proposal(format!("c{i}").as_bytes(), b"p")).unwrap();
        }
        k.checkpoint().unwrap();
    }
    let k = FileKernel::try_recover(store(&dir)).expect("healthy recovery is Ok");
    assert_eq!(k.committed_count(), 4);
}

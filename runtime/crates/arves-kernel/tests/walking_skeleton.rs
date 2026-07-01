//! I1.4 Walking Skeleton - behaviour proofs.
//!
//! Proves ARVES's first executable behaviour end to end, single node/shard:
//! commit -> WAL -> durable truth -> TruthRef -> replay -> identical truth.
//! No Raft, networking, replication, or scheduling is exercised.

use arves_kernel::{CommitError, ContentHash, Kernel, MemKernel, ProposedWrite, ShardKey};
use arves_persistence::MemWalStore;

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

/// Behaviour 1: commit once -> truth exists.
#[test]
fn behaviour_1_commit_once_truth_exists() {
    let k = MemKernel::new(MemWalStore::new());
    let tr = k.commit(proposal(b"c1", b"p1")).expect("commit ok");
    assert_eq!(tr.index.0, 0, "first truth is at commit index 0");
    assert_eq!(k.committed_count(), 1);
}

/// Behaviour 2: commit twice (same content) -> AlreadyCommitted, no fork.
#[test]
fn behaviour_2_commit_twice_already_committed() {
    let k = MemKernel::new(MemWalStore::new());
    let first = k.commit(proposal(b"c1", b"p1")).expect("first ok");
    match k.commit(proposal(b"c1", b"p1")) {
        Err(CommitError::AlreadyCommitted(existing)) => {
            assert_eq!(existing, first, "idempotent commit resolves to existing truth")
        }
        other => panic!("expected AlreadyCommitted, got {other:?}"),
    }
    assert_eq!(k.committed_count(), 1, "no fork of truth");
}

/// Behaviour 3: replay -> same truth.
#[test]
fn behaviour_3_replay_same_truth() {
    let store = MemWalStore::new();
    let k1 = MemKernel::new(store.clone());
    k1.commit(proposal(b"c1", b"p1")).unwrap();
    k1.commit(proposal(b"c2", b"p2")).unwrap();
    let before = k1.truth_hash();
    let k2 = MemKernel::recover(store.clone());
    assert_eq!(k2.committed_count(), 2);
    assert_eq!(k2.truth_hash(), before, "replayed truth equals committed truth");
}

/// Behaviour 4: crash -> restart -> replay -> truth identical.
#[test]
fn behaviour_4_crash_restart_replay_identical() {
    let store = MemWalStore::new();
    let expected;
    {
        let k = MemKernel::new(store.clone());
        k.commit(proposal(b"a", b"pa")).unwrap();
        k.commit(proposal(b"b", b"pb")).unwrap();
        expected = k.truth_hash();
        // k dropped here == crash: in-memory truth cache gone, WAL persists.
    }
    let recovered = MemKernel::recover(store.clone());
    assert_eq!(recovered.committed_count(), 2);
    assert_eq!(recovered.truth_hash(), expected, "truth survives restart unchanged");
}

/// Behaviour 5: replay twice -> no duplicate truth.
#[test]
fn behaviour_5_replay_twice_no_duplicate() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    k.commit(proposal(b"c1", b"p1")).unwrap();
    let recovered = MemKernel::recover(store.clone());
    let h1 = recovered.truth_hash();
    recovered.replay(); // apply the log a second time into the same kernel
    assert_eq!(recovered.committed_count(), 1, "replay is idempotent");
    assert_eq!(recovered.truth_hash(), h1, "second replay creates no new truth");
}

/// Behaviour 6: truth hash before == after replay.
#[test]
fn behaviour_6_truth_hash_preserved() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    k.commit(proposal(b"x", b"px")).unwrap();
    k.commit(proposal(b"y", b"py")).unwrap();
    let before = k.truth_hash();
    let after = MemKernel::recover(store.clone()).truth_hash();
    assert_eq!(before, after, "truth hash is preserved across replay");
}

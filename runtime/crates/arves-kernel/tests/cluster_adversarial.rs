//! RCR-022 (I2 Stage 4) — ADVERSARIAL distributed proofs at the CLUSTER-KERNEL
//! level, per the I2 design's conformance plan (S-I2-4 partition/CP, the
//! failure-injection classes of §5.3, ORCH-003/004 under distribution).
//!
//! Beyond the Stage-3 proofs: a symmetric partition (minority blocked,
//! majority commits, heal converges to ONE truth), the old-leader-returns
//! path (stale term refused, stale suffix never applied), a duplicate +
//! reordered consensus-message storm (ORCH-004: truth exactly-once at the
//! cluster level), and full-cluster deterministic replay from the WAL
//! (ORCH-003: rebuild EVERY node from its logs → identical `truth_hash`).
//!
//! HONEST SCOPE (unchanged from RCR-019..021): the transport is the
//! in-process deterministic FIFO bus; partitions/duplication/reordering are
//! scripted bus behaviours; time is the injected logical tick. NO network
//! exists and NO network fault-tolerance is claimed. Every test is
//! deterministic: fixed seeds, scripted schedules, no sleeps, no wall clocks,
//! no OS randomness. The RCR-019 harness re-checks the four Raft safety
//! properties after EVERY message step of every test below.

use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{CommitError, ContentHash, Kernel, ProposedWrite, ShardKey};
use std::cell::RefCell;
use std::rc::Rc;

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}

fn skey(t: &str, w: &str) -> ShardKey {
    ShardKey::new(t, w).expect("well-formed shard key")
}

fn pw(t: &str, w: &str, tag: &str) -> ProposedWrite {
    ProposedWrite {
        shard: skey(t, w),
        content: ContentHash(format!("c:{tag}").into_bytes()),
        payload: format!("p:{tag}").into_bytes(),
    }
}

/// Build an N-node, 1-shard cluster with an elected leader.
fn cluster(nodes: usize, seed: u64) -> (Rc<RefCell<ClusterSim>>, ShardId, NodeId) {
    let shard = sid("t1", "w1");
    let mut c = ClusterSim::new(nodes);
    c.add_shard(shard.clone(), seed);
    let leader = c.elect(&shard);
    (Rc::new(RefCell::new(c)), shard, leader)
}

/// Assert every replica holds `count` truths, byte-identical per-shard state,
/// and the same `truth_hash` as `reference` (ONE truth across the cluster).
fn assert_one_truth(c: &ClusterSim, shard: &ShardId, reference: &NodeId, count: usize) {
    let blob = c.shard_state_of(reference, shard);
    let hash = c.truth_hash_of(reference);
    for id in c.node_ids() {
        assert_eq!(c.committed_count_of(&id), count, "replica {id:?} truth count");
        assert_eq!(c.shard_state_of(&id, shard), blob, "replica {id:?} state bytes");
        assert_eq!(c.truth_hash_of(&id), hash, "replica {id:?} truth hash");
    }
}

/// SYMMETRIC PARTITION (S-I2-4, IDR-001 CP): a 5-node group splits 2 (with
/// the old leader) vs 3. The minority side CANNOT commit (`NotReplicated`,
/// zero partial truth anywhere — A-005/A-006); the majority side elects a
/// successor and CAN commit; after heal the cluster converges to exactly ONE
/// truth — the minority's un-replicated suffix is truncated, never applied.
#[test]
fn adversarial_symmetric_partition_minority_blocked_majority_commits_heal_converges() {
    let (cluster, shard, old_leader) = cluster(5, 0x51DE5);
    let k_old = ClusterKernel::new(old_leader.clone(), cluster.clone());

    k_old.commit(pw("t1", "w1", "e1")).expect("pre-fault commit");
    cluster.borrow_mut().settle(4);

    // Symmetric split: {old leader + one follower} vs {the other three}.
    let (minority, majority): (Vec<NodeId>, Vec<NodeId>) = {
        let ids = cluster.borrow().node_ids();
        let peer = ids.iter().find(|n| **n != old_leader).cloned().expect("a peer");
        let minority = vec![old_leader.clone(), peer];
        let majority = ids.into_iter().filter(|n| !minority.contains(n)).collect();
        (minority, majority)
    };
    assert_eq!((minority.len(), majority.len()), (2, 3), "a symmetric 2/3 split");
    cluster.borrow_mut().partition(&shard, &[minority.clone(), majority.clone()]);

    // Minority cannot commit: 2 of 5 is no quorum — CP refuses, no divergence.
    assert_eq!(
        k_old.commit(pw("t1", "w1", "minority")),
        Err(CommitError::NotReplicated),
        "minority side must be honestly unavailable (IDR-001 CP)"
    );
    {
        let c = cluster.borrow();
        for id in c.node_ids() {
            assert_eq!(c.committed_count_of(&id), 1, "replica {id:?}: zero partial truth");
        }
    }

    // Majority can: a successor emerges among the 3 and commits.
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("majority-side leader");
    assert!(majority.contains(&new_leader), "the successor lives on the majority side");
    assert_ne!(new_leader, old_leader);
    let k_new = ClusterKernel::new(new_leader.clone(), cluster.clone());
    k_new.commit(pw("t1", "w1", "e2")).expect("majority-side commit proceeds");

    // Heal: one truth. The minority's un-replicated "minority" entry is
    // truncated by the successor's log — it was acked to nobody and is
    // applied nowhere.
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(80);
    let c = cluster.borrow();
    assert_one_truth(&c, &shard, &new_leader, 2);
}

/// OLD LEADER RETURNS (IDR-004): a deposed leader that still believes it
/// leads a STALE term rejoins. Its stale-term traffic is refused by every
/// peer, it steps down and catches up; its gateway refuses new commits with
/// `NotLeader`; the entry it appended at the stale term was never applied on
/// any replica; the cluster holds exactly one truth.
#[test]
fn adversarial_old_leader_returns_stale_term_refused() {
    let (cluster, shard, old_leader) = cluster(3, 0x01D_1EAD);
    let k_old = ClusterKernel::new(old_leader.clone(), cluster.clone());

    k_old.commit(pw("t1", "w1", "e1")).expect("pre-fault commit");
    cluster.borrow_mut().settle(4);

    // Isolate the leader. It keeps believing it leads its stale term; the
    // proposal it appends there can never commit.
    cluster.borrow_mut().isolate(&shard, &old_leader);
    assert_eq!(
        k_old.commit(pw("t1", "w1", "stale")),
        Err(CommitError::NotReplicated),
        "the stale leader cannot commit without quorum"
    );

    // The majority elects a successor at a HIGHER term and keeps committing.
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("successor");
    assert_ne!(new_leader, old_leader, "a higher-term successor exists");
    let k_new = ClusterKernel::new(new_leader.clone(), cluster.clone());
    k_new.commit(pw("t1", "w1", "e2")).expect("successor commit");
    k_new.commit(pw("t1", "w1", "e3")).expect("successor commit");
    assert_eq!(
        cluster.borrow().committed_count_of(&old_leader),
        1,
        "the isolated old leader saw none of e2/e3"
    );

    // The old leader returns: its stale term is refused, it steps down, its
    // stale suffix is truncated, and it catches up to the successor's log.
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(80);
    assert_eq!(
        cluster.borrow().leader_of(&shard),
        Some(new_leader.clone()),
        "the returning stale leader deposes nobody (leadership check, RCR-020 DR-7)"
    );
    assert!(
        matches!(k_old.commit(pw("t1", "w1", "e4")), Err(CommitError::NotLeader { .. })),
        "the returned old leader's gateway refuses client commits"
    );

    // One truth: exactly e1..e3 everywhere; the "stale" entry applied nowhere.
    let c = cluster.borrow();
    assert_one_truth(&c, &shard, &new_leader, 3);
    let blob = c.shard_state_of(&new_leader, &shard);
    assert!(
        !blob.windows(7).any(|w| w == b"p:stale"),
        "the stale-term entry must never enter truth"
    );
}

/// DUPLICATE + REORDERED consensus messages (ORCH-004 at the cluster level):
/// with every 2nd delivered message duplicated stale and every 3rd pop
/// deferred behind later traffic, commits through the leader — including an
/// explicit client retry — produce truth EXACTLY ONCE on every replica:
/// identical counts, dense identical `CommitIndex`es, byte-identical state.
#[test]
fn adversarial_duplicate_reordered_delivery_truth_exactly_once() {
    let (cluster, shard, leader) = cluster(3, 0xD0_D0);
    cluster.borrow_mut().mangle(&shard, 2, 3);
    let k = ClusterKernel::new(leader.clone(), cluster.clone());

    let mut refs = Vec::new();
    for tag in ["e1", "e2", "e3", "e4", "e5"] {
        refs.push(k.commit(pw("t1", "w1", tag)).expect("commit under the storm"));
    }
    // Dense WAL offsets despite duplicated/reordered replication traffic.
    for (i, tr) in refs.iter().enumerate() {
        assert_eq!(tr.index.0, i as u64, "dense commit index under the storm");
    }
    // Client retry storm: an at-least-once re-proposal resolves idempotently
    // to the SAME TruthRef (ORCH-004) — no second truth, no fork.
    match k.commit(pw("t1", "w1", "e2")) {
        Err(CommitError::AlreadyCommitted(tr)) => assert_eq!(tr, refs[1]),
        other => panic!("expected AlreadyCommitted, got {other:?}"),
    }

    cluster.borrow_mut().settle(12);
    let c = cluster.borrow();
    let (dup, def) = c.mangled_of(&shard);
    assert!(dup > 0 && def > 0, "the storm actually duplicated ({dup}) and reordered ({def})");
    assert_one_truth(&c, &shard, &leader, 5);
}

/// FULL-CLUSTER DETERMINISTIC REPLAY FROM THE WAL (ORCH-003): after commits,
/// a failover and a heal, EVERY node is rebuilt from its own durable log
/// (in-memory truth dropped, deterministic replay — the I1.7 path, applied
/// cluster-wide) and every rebuilt node reproduces the identical
/// `truth_hash`, identical count, and byte-identical per-shard state. Replay,
/// never recompute — engines are not consulted anywhere on this path.
#[test]
fn adversarial_full_cluster_replay_from_wal_rebuilds_identical_truth() {
    let (cluster, shard, leader) = cluster(3, 0x0003_C003);
    let k = ClusterKernel::new(leader.clone(), cluster.clone());

    k.commit(pw("t1", "w1", "e1")).expect("commit e1");
    k.commit(pw("t1", "w1", "e2")).expect("commit e2");
    cluster.borrow_mut().settle(4);

    // Adversarial interlude: depose the leader, fail over, keep committing.
    cluster.borrow_mut().isolate(&shard, &leader);
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("successor");
    let k2 = ClusterKernel::new(new_leader.clone(), cluster.clone());
    k2.commit(pw("t1", "w1", "e3")).expect("post-failover commit");
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(80);

    // Record the cluster's single truth, then rebuild EVERY node from logs.
    let (hash, blob) = {
        let c = cluster.borrow();
        assert_one_truth(&c, &shard, &new_leader, 3);
        (c.truth_hash_of(&new_leader), c.shard_state_of(&new_leader, &shard))
    };
    {
        let mut c = cluster.borrow_mut();
        for id in c.node_ids() {
            c.crash_recover(&id); // drop in-memory truth; replay the local WAL
        }
    }
    let c = cluster.borrow();
    for id in c.node_ids() {
        assert_eq!(c.truth_hash_of(&id), hash, "rebuilt replica {id:?} identical truth_hash");
        assert_eq!(c.committed_count_of(&id), 3, "rebuilt replica {id:?} full truth");
        assert_eq!(c.shard_state_of(&id, &shard), blob, "rebuilt replica {id:?} state bytes");
    }
}

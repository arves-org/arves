//! RCR-021 (I2 Stage 3) — the CLUSTER KERNEL behaviour proofs.
//!
//! The frozen [`Kernel`] gateway over per-shard Raft (design
//! `docs/design/I2_Cluster_Kernel_Design.md` §3.1, §5.2 S-I2-1/-3/-4/-8
//! in-process analogues): leader-only commit (OWN-001/IDR-004), quorum before
//! ack (IDR-001 CP), follower apply producing IDENTICAL truth (ORCH-003 across
//! nodes), gateway idempotency + content-integrity preserved under replication
//! (ORCH-004, RCR-005), `NotReplicated` on lost quorum, and Kernel snapshot
//! install for a crashed/lagging follower (IDR-002: snapshot, then log tail).
//!
//! Every test is deterministic: fixed seeds, scripted faults (bus filters),
//! injected logical ticks — no sleeps, no wall clocks, no OS randomness.

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

/// Build a 3-node, 1-shard cluster with an elected leader.
fn three_nodes(seed: u64) -> (Rc<RefCell<ClusterSim>>, ShardId, NodeId) {
    let shard = sid("t1", "w1");
    let mut c = ClusterSim::new(3);
    c.add_shard(shard.clone(), seed);
    let leader = c.elect(&shard);
    (Rc::new(RefCell::new(c)), shard, leader)
}

/// S-I2-1 in-process analogue (happy path): commit at the leader → quorum →
/// every replica converges to IDENTICAL truth — equal `truth_hash`, equal
/// per-shard state-blob BYTES, dense identical `CommitIndex`es (ORCH-003
/// across nodes; IDR-002 followers apply, never recompute). Then the gateway
/// semantics survive replication: an identical re-proposal resolves to the
/// SAME TruthRef (ORCH-004) and a same-address/different-payload fork is
/// refused with no truth delta anywhere (RCR-005).
#[test]
fn cluster_commit_on_leader_reaches_identical_truth_on_all_nodes() {
    let (cluster, shard, leader) = three_nodes(0xA11CE);
    let k = ClusterKernel::new(leader.clone(), cluster.clone());

    let tr1 = k.commit(pw("t1", "w1", "e1")).expect("leader commit e1");
    let tr2 = k.commit(pw("t1", "w1", "e2")).expect("leader commit e2");
    assert_eq!(tr1.index.0, 0, "dense WAL offsets from 0");
    assert_eq!(tr2.index.0, 1);
    assert_eq!(tr1.shard, skey("t1", "w1"));

    cluster.borrow_mut().settle(4); // heartbeats carry commit to followers
    {
        let c = cluster.borrow();
        let h = c.truth_hash_of(&leader);
        let blob = c.shard_state_of(&leader, &shard);
        assert!(!blob.is_empty());
        for id in c.node_ids() {
            assert_eq!(c.committed_count_of(&id), 2, "replica {id:?} count");
            assert_eq!(c.truth_hash_of(&id), h, "replica {id:?} truth hash");
            assert_eq!(
                c.shard_state_of(&id, &shard),
                blob,
                "replica {id:?} byte-identical shard state"
            );
        }
    }

    // ORCH-004 under replication: identical re-proposal → the SAME truth.
    match k.commit(pw("t1", "w1", "e1")) {
        Err(CommitError::AlreadyCommitted(tr)) => assert_eq!(tr, tr1),
        other => panic!("expected AlreadyCommitted, got {other:?}"),
    }
    // RCR-005 under replication: same address, different payload → refused.
    let mut fork = pw("t1", "w1", "e1");
    fork.payload = b"DIFFERENT".to_vec();
    assert!(matches!(
        k.commit(fork),
        Err(CommitError::ContentIntegrity { .. })
    ));
    // Neither refusal changed truth anywhere.
    let c = cluster.borrow();
    for id in c.node_ids() {
        assert_eq!(c.committed_count_of(&id), 2, "replica {id:?} unchanged");
    }
}

/// OWN-001 / IDR-004: a follower's gateway is NOT authoritative — a client
/// commit against any non-leader replica is refused with `NotLeader{shard}`
/// and leaves zero truth on every replica.
#[test]
fn cluster_follower_commit_refused_not_leader() {
    let (cluster, _shard, leader) = three_nodes(7);
    let follower = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .find(|n| *n != leader)
        .expect("a follower exists");

    let k = ClusterKernel::new(follower, cluster.clone());
    match k.commit(pw("t1", "w1", "e1")) {
        Err(CommitError::NotLeader { shard }) => assert_eq!(shard, skey("t1", "w1")),
        other => panic!("expected NotLeader, got {other:?}"),
    }
    let c = cluster.borrow();
    for id in c.node_ids() {
        assert_eq!(c.committed_count_of(&id), 0, "no partial truth anywhere");
    }
}

/// SHARD-001: a proposal for a shard with no registered Raft group is refused
/// (`UnknownShard`) — there is no cross-shard fallback route to truth.
#[test]
fn cluster_unknown_shard_refused() {
    let (cluster, _shard, leader) = three_nodes(11);
    let k = ClusterKernel::new(leader, cluster);
    match k.commit(pw("t-other", "w-other", "e1")) {
        Err(CommitError::UnknownShard { shard }) => {
            assert_eq!(shard, skey("t-other", "w-other"));
        }
        other => panic!("expected UnknownShard, got {other:?}"),
    }
}

/// S-I2-4 in-process analogue (CP posture, IDR-001): the leader partitioned
/// into a minority cannot commit — `NotReplicated`, zero partial truth
/// (A-005/A-006) — while the majority elects a successor. After heal, the
/// deposed leader's gateway refuses (`NotLeader`), the retried proposal
/// commits FRESH through the new leader (its stale un-replicated suffix is
/// truncated, never applied), and all replicas converge to identical truth,
/// exactly once.
#[test]
fn cluster_quorum_loss_fails_not_replicated_then_recovers_after_heal() {
    let (cluster, shard, old_leader) = three_nodes(7);
    let k_old = ClusterKernel::new(old_leader.clone(), cluster.clone());

    let tr1 = k_old.commit(pw("t1", "w1", "e1")).expect("pre-fault commit");
    assert_eq!(tr1.index.0, 0);
    cluster.borrow_mut().settle(4);

    // Fault: the leader alone on the minority side.
    cluster.borrow_mut().isolate(&shard, &old_leader);
    assert_eq!(
        k_old.commit(pw("t1", "w1", "e2")),
        Err(CommitError::NotReplicated),
        "no quorum, no truth (IDR-001 CP)"
    );
    {
        // No partial truth anywhere: e2 was acked to NOBODY and applied NOWHERE.
        let c = cluster.borrow();
        for id in c.node_ids() {
            assert_eq!(c.committed_count_of(&id), 1, "replica {id:?} holds only e1");
        }
    }

    // Heal; the majority side elected a successor during the outage.
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("leader after heal");
    assert_ne!(new_leader, old_leader, "the isolated leader was deposed");

    // The deposed leader's gateway now refuses honestly.
    assert!(matches!(
        k_old.commit(pw("t1", "w1", "e2")),
        Err(CommitError::NotLeader { .. })
    ));

    // The retry through the NEW leader succeeds — fresh, exactly once.
    let k_new = ClusterKernel::new(new_leader.clone(), cluster.clone());
    let tr2 = k_new.commit(pw("t1", "w1", "e2")).expect("post-heal commit");
    assert_eq!(tr2.index.0, 1, "dense continuation after the discarded suffix");
    cluster.borrow_mut().settle(6);

    let c = cluster.borrow();
    let blob = c.shard_state_of(&new_leader, &shard);
    for id in c.node_ids() {
        assert_eq!(c.committed_count_of(&id), 2, "replica {id:?} exactly-once");
        assert_eq!(c.shard_state_of(&id, &shard), blob, "replica {id:?} identical");
        assert_eq!(c.truth_hash_of(&id), c.truth_hash_of(&new_leader));
    }
}

/// S-I2-8 in-process analogue (crash → snapshot → catch-up, IDR-002/005): a
/// follower crashes (loses in-memory truth, recovers losslessly from its own
/// durable WAL — the I1.7 path) and misses commits while cut off; the leader's
/// Kernel snapshot (a pure function of its applied prefix) is installed —
/// truth state + dense WAL continuation + apply-cursor jump; after heal the
/// raft log tail applies normally, and further commits land on ALL replicas
/// with identical offsets and byte-identical shard state.
#[test]
fn cluster_crashed_follower_snapshot_install_then_catch_up() {
    let (cluster, shard, leader) = three_nodes(42);
    let lagging = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .find(|n| *n != leader)
        .expect("a follower exists");
    let k = ClusterKernel::new(leader.clone(), cluster.clone());

    for tag in ["e1", "e2", "e3"] {
        k.commit(pw("t1", "w1", tag)).expect("pre-crash commit");
    }
    cluster.borrow_mut().settle(4);
    assert_eq!(cluster.borrow().committed_count_of(&lagging), 3);

    // Crash the follower: cut it off and drop its in-memory truth; recovery
    // replays its local durable WAL (lossless or loud, ORCH-003).
    {
        let mut c = cluster.borrow_mut();
        c.isolate(&shard, &lagging);
        c.crash_recover(&lagging);
        assert_eq!(c.committed_count_of(&lagging), 3, "I1 recovery is lossless");
        assert_eq!(c.applied_of(&lagging, &shard), 3);
    }

    // The surviving majority keeps committing; the crashed follower lags.
    for tag in ["e4", "e5", "e6"] {
        k.commit(pw("t1", "w1", tag)).expect("commit on surviving quorum");
    }
    cluster.borrow_mut().settle(4);
    assert_eq!(cluster.borrow().committed_count_of(&lagging), 3, "still behind");

    // Snapshot install from the leader (IDR-002: snapshot, then log tail).
    cluster.borrow_mut().install_snapshot(&shard, &leader, &lagging);
    {
        let c = cluster.borrow();
        assert_eq!(c.committed_count_of(&lagging), 6, "snapshot brought e4..e6");
        assert_eq!(c.applied_of(&lagging, &shard), 6, "apply cursor jumped");
        assert_eq!(
            c.shard_state_of(&lagging, &shard),
            c.shard_state_of(&leader, &shard),
            "byte-identical after install"
        );
    }

    // Heal: the raft log catches the rejoined replica up (its bumped term may
    // force one bounded re-election — standard Raft; safety holds throughout).
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(80);
    let cur_leader = cluster.borrow().leader_of(&shard).expect("leader after rejoin");

    // A post-rejoin commit applies EVERYWHERE at the identical dense offset.
    let k2 = ClusterKernel::new(cur_leader.clone(), cluster.clone());
    let tr7 = k2.commit(pw("t1", "w1", "e7")).expect("post-rejoin commit");
    assert_eq!(tr7.index.0, 6, "aligned offsets after snapshot + tail");
    cluster.borrow_mut().settle(6);

    let c = cluster.borrow();
    let blob = c.shard_state_of(&cur_leader, &shard);
    for id in c.node_ids() {
        assert_eq!(c.committed_count_of(&id), 7, "replica {id:?} full truth");
        assert_eq!(c.shard_state_of(&id, &shard), blob, "replica {id:?} identical");
        assert_eq!(c.truth_hash_of(&id), c.truth_hash_of(&cur_leader));
    }
}

/// Determinism over convenience: two identically-seeded, identically-scripted
/// cluster runs (including a fault and heal) produce the identical truth on
/// every replica — the ORCH-003 replayability property at cluster scope.
#[test]
fn cluster_runs_are_deterministic_for_identical_seeds() {
    let run = |seed: u64| -> Vec<u64> {
        let (cluster, shard, leader) = three_nodes(seed);
        let k = ClusterKernel::new(leader.clone(), cluster.clone());
        k.commit(pw("t1", "w1", "e1")).expect("commit e1");
        cluster.borrow_mut().settle(4);
        let f = cluster
            .borrow()
            .node_ids()
            .into_iter()
            .find(|n| *n != leader)
            .expect("follower");
        cluster.borrow_mut().isolate(&shard, &f);
        k.commit(pw("t1", "w1", "e2")).expect("commit on 2/3 quorum");
        cluster.borrow_mut().heal(&shard);
        cluster.borrow_mut().settle(40);
        let c = cluster.borrow();
        c.node_ids().iter().map(|id| c.truth_hash_of(id)).collect()
    };
    assert_eq!(run(1234), run(1234), "identical seed ⇒ identical truth");
}

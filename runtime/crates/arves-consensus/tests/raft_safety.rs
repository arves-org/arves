//! RCR-019 — Raft SAFETY PROPERTIES as deterministic scenario tests.
//!
//! Each test is a scripted message schedule on the deterministic in-process
//! bus (`arves_consensus::sim`); drops and partitions are bus filters, time is
//! the injected logical tick, randomness is the recorded seed. NO sleeps, NO
//! wall clocks, NO OS entropy — every run is exactly replayable.
//!
//! The four Raft safety properties (paper Fig. 3) are ALSO checked
//! continuously by the harness after every single message step; these tests
//! drive the cluster through the adversarial schedules where each property is
//! actually at risk, then assert the end-state explicitly.
//!
//! HONEST SCOPE: these are in-process simulations of message loss/partition,
//! not network fault-tolerance evidence (transport is a later I2 stage).

use arves_consensus::sim::SimCluster;
use arves_consensus::{
    ConsensusError, ContentHash, EntryKind, Leadership, LogIndex, Membership, NodeId, Outcome,
    ReadTier, Role, ShardConsensus, ShardId, TenantId, WorkspaceId,
};
use std::cell::RefCell;
use std::rc::Rc;

fn outcome(tag: &str) -> EntryKind {
    EntryKind::Outcome(Outcome {
        digest: ContentHash(format!("h:{tag}")),
        payload: tag.as_bytes().to_vec(),
    })
}

fn shard() -> ShardId {
    ShardId::new(TenantId("t1".into()), WorkspaceId("w1".into()))
}

/// Split the cluster around `leader`: (minority incl. leader, majority).
fn split_minority_with_leader(c: &SimCluster, leader: &NodeId) -> (Vec<NodeId>, Vec<NodeId>) {
    let others: Vec<NodeId> = c.node_ids().into_iter().filter(|n| n != leader).collect();
    let minority = vec![leader.clone(), others[0].clone()];
    let majority = others[1..].to_vec();
    (minority, majority)
}

/// ELECTION SAFETY: at most one leader per term (IDR-004), held across a
/// partition that forces competing elections on both sides, and across heal.
#[test]
fn safety_election_safety_at_most_one_leader_per_term() {
    let mut c = SimCluster::new(5, 0xE1EC);
    let leader = c.run_until_leader(300);
    let (minority, majority) = split_minority_with_leader(&c, &leader);
    c.partition(&[minority, majority.clone()]);
    c.run(120); // both sides tick: majority elects, minority churns candidates
    let (new_leader, new_term) = c.current_leader().expect("majority side elects");
    assert!(majority.contains(&new_leader), "new leader must be on the majority side");
    assert!(new_term > 1, "election happened in a later term");
    c.heal();
    c.run(120);
    // The harness panics on any per-step violation; assert the whole history too.
    for (term, leaders) in c.leaders_history() {
        assert!(leaders.len() <= 1, "Election Safety violated in term {term}: {leaders:?}");
    }
}

/// LOG MATCHING: a deposed leader's un-replicated conflicting suffix is
/// truncated on heal; all logs converge identical (same (index,term) ⇒ same
/// prefix at every step — checked continuously by the harness).
#[test]
fn safety_log_matching_conflicting_suffix_truncated() {
    let mut c = SimCluster::new(5, 0x106);
    let old_leader = c.run_until_leader(300);
    let committed = c.propose(&old_leader, outcome("committed-1")).unwrap();
    c.run(4);
    assert_eq!(committed, LogIndex(1));
    // Cut the old leader off with one follower; it appends entries it can
    // never commit (quorum is 3 of 5).
    let (minority, majority) = split_minority_with_leader(&c, &old_leader);
    c.partition(&[minority, majority]);
    c.propose(&old_leader, outcome("doomed-a")).unwrap();
    c.propose(&old_leader, outcome("doomed-b")).unwrap();
    c.run(120); // majority elects a new leader meanwhile
    let (new_leader, _) = c.current_leader().expect("majority leader");
    // The new leader commits DIFFERENT entries at the same indices 2..3.
    c.propose(&new_leader, outcome("kept-a")).unwrap();
    c.propose(&new_leader, outcome("kept-b")).unwrap();
    c.run(4);
    c.heal();
    c.run(200);
    // Convergence: every replica has the new leader's log; doomed-* is gone.
    let reference = c.log_of(&new_leader).to_vec();
    assert_eq!(reference.len(), 3);
    for id in c.node_ids() {
        assert_eq!(c.log_of(&id), &reference[..], "replica {id:?} did not converge");
        assert!(
            !c.log_of(&id).iter().any(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0.starts_with("h:doomed")
            )),
            "conflicting suffix survived on {id:?}"
        );
    }
}

/// LEADER COMPLETENESS: entries committed under an old leader are present in
/// every later leader's log (the §5.4.1 vote check makes a stale-log
/// candidate unelectable). Checked at every leader emergence by the harness;
/// asserted explicitly here.
#[test]
fn safety_leader_completeness_new_leader_has_all_committed() {
    let mut c = SimCluster::new(5, 0xC0);
    let old_leader = c.run_until_leader(300);
    c.propose(&old_leader, outcome("e1")).unwrap();
    c.propose(&old_leader, outcome("e2")).unwrap();
    c.run(4);
    let committed_before = c.committed_history();
    assert_eq!(committed_before.len(), 2);
    // Old leader vanishes entirely.
    c.isolate(&old_leader);
    c.run(200);
    let (new_leader, _) = c.current_leader().expect("new leader");
    assert_ne!(new_leader, old_leader);
    let log = c.log_of(&new_leader);
    for (idx, entry) in &committed_before {
        assert_eq!(
            log.get(*idx as usize - 1),
            Some(entry),
            "Leader Completeness violated: new leader misses committed index {idx}"
        );
    }
}

/// STATE MACHINE SAFETY: across partition churn, failover, and catch-up, no
/// replica ever commits a different entry at the same index. The applied state
/// in Stage 1 is the committed log prefix (Kernel apply wiring is a later
/// stage).
#[test]
fn safety_state_machine_safety_under_partition_churn() {
    let mut c = SimCluster::new(5, 0x57A7E);
    let l1 = c.run_until_leader(300);
    c.propose(&l1, outcome("a")).unwrap();
    c.run(4);
    let (minority, majority) = split_minority_with_leader(&c, &l1);
    c.partition(&[minority, majority]);
    c.propose(&l1, outcome("orphan")).unwrap(); // can never commit
    c.run(120);
    let (l2, _) = c.current_leader().expect("majority leader");
    c.propose(&l2, outcome("b")).unwrap();
    c.propose(&l2, outcome("c")).unwrap();
    c.run(4);
    c.heal();
    c.run(200);
    // Harness asserted per-step; end-state: one committed history, all equal.
    let committed = c.committed_history();
    assert_eq!(committed.len(), 3, "a, b, c committed exactly once each");
    for id in c.node_ids() {
        assert_eq!(c.commit_of(&id), 3, "replica {id:?} commit index");
        for (idx, entry) in &committed {
            assert_eq!(c.log_of(&id).get(*idx as usize - 1), Some(entry));
        }
    }
}

/// S-I2-3 (in-process analogue): leader failover discards un-acked in-flight
/// work and preserves every acked (committed) entry — no partial truth
/// (IDR-004; Amendments A-005/A-006).
#[test]
fn scenario_leader_failover_discards_uncommitted_preserves_acked() {
    let mut c = SimCluster::new(3, 0xFA110);
    let l1 = c.run_until_leader(300);
    let acked = c.propose(&l1, outcome("acked")).unwrap();
    c.run(4);
    assert!(c.commit_of(&l1) >= acked.0, "acked entry is committed");
    // Leader is cut off BEFORE it can replicate the next proposal.
    c.isolate(&l1);
    let unacked = c.propose(&l1, outcome("unacked")).unwrap();
    assert_eq!(unacked, LogIndex(2));
    c.run(60);
    assert!(c.commit_of(&l1) < unacked.0, "isolated leader can never commit it (CP)");
    let (l2, _) = c.current_leader().expect("majority elects");
    assert_ne!(l2, l1);
    c.propose(&l2, outcome("after-failover")).unwrap();
    c.run(4);
    c.heal();
    c.run(200);
    for id in c.node_ids() {
        let log = c.log_of(&id);
        assert!(
            !log.iter().any(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0 == "h:unacked"
            )),
            "uncommitted in-flight work leaked into {id:?}"
        );
        assert!(
            log.iter().any(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0 == "h:acked"
            )),
            "acked commit lost on {id:?}"
        );
    }
}

/// S-I2-4 (in-process analogue, CP posture): a minority-side leader cannot
/// commit — await_commit reports QuorumUnavailable rather than diverging
/// (IDR-001: unavailable > divergent); the majority proceeds.
#[test]
fn scenario_minority_partition_cannot_commit_quorum_unavailable() {
    let cluster = Rc::new(RefCell::new(SimCluster::new(5, 0xCB)));
    let leader = cluster.borrow_mut().run_until_leader(300);
    let (minority, majority) = {
        let c = cluster.borrow();
        split_minority_with_leader(&c, &leader)
    };
    cluster.borrow_mut().partition(&[minority, majority.clone()]);
    // Propose through the frozen contract at the minority leader.
    let handle = arves_consensus::sim::SimShardConsensus::new(shard(), leader.clone(), cluster.clone());
    let idx = handle.propose(&shard(), Outcome {
        digest: ContentHash("h:minority".into()),
        payload: b"minority".to_vec(),
    }).expect("append at (still-)leader succeeds");
    let err = handle.await_commit(&shard(), idx).unwrap_err();
    assert_eq!(err, ConsensusError::QuorumUnavailable, "CP: unavailable, never divergent");
    // Majority side elected and can commit.
    let (new_leader, _) = cluster.borrow().current_leader().expect("majority leader");
    assert!(majority.contains(&new_leader));
    let idx2 = cluster.borrow_mut().propose(&new_leader, outcome("majority")).unwrap();
    cluster.borrow_mut().run(4);
    assert!(cluster.borrow().commit_of(&new_leader) >= idx2.0);
}

/// Follower catch-up: a replica isolated through several commits converges to
/// the identical log and commit index after heal, purely via the leader's
/// next-index backtracking (IDR-002 follower apply; no snapshots in Stage 1).
#[test]
fn scenario_follower_catch_up_after_message_loss() {
    let mut c = SimCluster::new(3, 0xCA7C);
    let leader = c.run_until_leader(300);
    let straggler = c
        .node_ids()
        .into_iter()
        .find(|n| *n != leader)
        .expect("a follower");
    c.isolate(&straggler);
    for i in 0..5 {
        c.propose(&leader, outcome(&format!("e{i}"))).unwrap();
    }
    c.run(8);
    assert_eq!(c.commit_of(&leader), 5, "quorum of 2/3 commits without the straggler");
    assert_eq!(c.commit_of(&straggler), 0);
    c.heal();
    c.run(200);
    assert_eq!(c.commit_of(&straggler), 5, "straggler caught up");
    assert_eq!(c.log_of(&straggler), c.log_of(&leader), "byte-identical log after catch-up");
}

/// OWN-001 through the frozen contract: a client commit sent to a follower's
/// handle is refused with NotLeader carrying a redirect hint — followers are
/// derived replicas, never writers.
#[test]
fn contract_follower_rejects_client_commit_not_leader() {
    let cluster = Rc::new(RefCell::new(SimCluster::new(3, 0xF0110)));
    let leader = cluster.borrow_mut().run_until_leader(300);
    let follower = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .find(|n| *n != leader)
        .expect("a follower");
    let handle = arves_consensus::sim::SimShardConsensus::new(shard(), follower, cluster.clone());
    assert_eq!(handle.role(&shard()).unwrap(), Role::Follower);
    let err = handle
        .propose(&shard(), Outcome { digest: ContentHash("h:x".into()), payload: vec![] })
        .unwrap_err();
    match err {
        ConsensusError::NotLeader { leader: Leadership::Established { node, .. } } => {
            assert_eq!(node, leader, "redirect hint names the actual leader");
        }
        other => panic!("expected NotLeader with redirect, got {other:?}"),
    }
}

/// Frozen-contract edges scoped for Stage 1: unknown shard is refused with
/// UnknownShard (IDR-001: one group per shard); membership change is honestly
/// MembershipRejected (joint consensus is ladder step I2.8); linearizable
/// read_index is leader-only.
#[test]
fn contract_unknown_shard_and_stage_scoped_membership() {
    let cluster = Rc::new(RefCell::new(SimCluster::new(3, 0x5C0)));
    let leader = cluster.borrow_mut().run_until_leader(300);
    let handle = arves_consensus::sim::SimShardConsensus::new(shard(), leader.clone(), cluster.clone());
    let other = ShardId::new(TenantId("t2".into()), WorkspaceId("w9".into()));
    assert_eq!(
        handle.leader(&other).unwrap_err(),
        ConsensusError::UnknownShard(other.clone())
    );
    assert_eq!(
        handle.change_membership(&shard(), Membership::Stable { voters: vec![], learners: vec![] })
            .unwrap_err(),
        ConsensusError::MembershipRejected
    );
    // Linearizable read at the leader reflects the latest commit.
    let idx = handle
        .propose(&shard(), Outcome { digest: ContentHash("h:r".into()), payload: vec![] })
        .unwrap();
    let entry = handle.await_commit(&shard(), idx).unwrap();
    assert_eq!(entry.index, idx);
    assert!(handle.read_index(&shard(), ReadTier::Linearizable).unwrap() >= idx);
    // Linearizable at a follower is refused (leader-only in Stage 1).
    let follower = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .find(|n| *n != leader)
        .expect("a follower");
    let fh = arves_consensus::sim::SimShardConsensus::new(shard(), follower, cluster.clone());
    assert!(matches!(
        fh.read_index(&shard(), ReadTier::Linearizable),
        Err(ConsensusError::NotLeader { .. })
    ));
    // Weaker tiers answer locally with the honest local commit position.
    assert!(fh.read_index(&shard(), ReadTier::Eventual).is_ok());
}

//! RCR-020 — JOINT-CONSENSUS membership change + leadership transfer
//! (I2 Stage 2, parts b + c; design ladder step I2.8; IDR-003).
//!
//! The property under test is the C_old,new overlap rule: while a membership
//! transition is in flight, every electing or committing quorum must contain a
//! majority of BOTH the old and the new configuration — so two disjoint
//! majorities (split-brain) cannot exist at any instant of the transition.
//! The harness additionally re-checks the four Raft safety properties after
//! every single message step of every test.
//!
//! Every test is deterministic: seeded, scripted, injected logical ticks — no
//! sleeps, no wall clocks, no OS entropy. HONEST SCOPE: in-process simulation
//! (bus filters), not network fault-tolerance evidence.

use arves_consensus::sim::{SimCluster, SimShardConsensus};
use arves_consensus::{
    ConsensusError, ContentHash, EntryKind, Membership, NodeId, Outcome, Role, ShardConsensus,
    ShardId, TenantId, WorkspaceId,
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

fn nid(s: &str) -> NodeId {
    NodeId(s.into())
}

fn count_membership_entries(log: &[arves_consensus::LogEntry]) -> (usize, usize) {
    let joint = log
        .iter()
        .filter(|e| matches!(&e.kind, EntryKind::Membership(Membership::Joint { .. })))
        .count();
    let stable = log
        .iter()
        .filter(|e| matches!(&e.kind, EntryKind::Membership(Membership::Stable { .. })))
        .count();
    (joint, stable)
}

/// S-I2-5 shape (add): a node joins mid-stream through the frozen contract.
/// The log must show the full IDR-003 trace — one Joint entry then one Stable
/// entry — and all four replicas (including the joiner) converge
/// byte-identical; commits continue after the transition under the 3-of-4
/// quorum.
#[test]
fn membership_add_node_joint_then_stable_converges() {
    let cluster = Rc::new(RefCell::new(SimCluster::new(3, 0xADD)));
    let leader = cluster.borrow_mut().run_until_leader(300);
    cluster.borrow_mut().propose(&leader, outcome("pre")).unwrap();
    cluster.borrow_mut().run(4);

    // The target names the stable end state; the joint phase is internal.
    let mut voters = cluster.borrow().node_ids();
    cluster.borrow_mut().add_node(nid("n4"));
    voters.push(nid("n4"));

    let handle = SimShardConsensus::new(shard(), leader.clone(), cluster.clone());
    let joint_idx = handle
        .change_membership(&shard(), Membership::Stable { voters: voters.clone(), learners: vec![] })
        .expect("leader begins the joint transition");
    // The joint entry commits under the DUAL majority rule and the leader
    // auto-appends C_new (observable via the frozen contract).
    let joint_entry = handle.await_commit(&shard(), joint_idx).unwrap();
    assert!(
        matches!(joint_entry.kind, EntryKind::Membership(Membership::Joint { .. })),
        "the begun entry is the C_old,new phase"
    );

    // Transition trace: exactly one Joint followed by exactly one Stable.
    cluster.borrow_mut().run(8);
    {
        let c = cluster.borrow();
        let log = c.log_of(&leader);
        assert_eq!(count_membership_entries(log), (1, 1));
        let joint_pos = log
            .iter()
            .position(|e| matches!(&e.kind, EntryKind::Membership(Membership::Joint { .. })))
            .unwrap();
        let stable_pos = log
            .iter()
            .position(|e| matches!(&e.kind, EntryKind::Membership(Membership::Stable { .. })))
            .unwrap();
        assert!(joint_pos < stable_pos, "C_old,new precedes C_new (IDR-003 two-phase)");
        // Effective config on the leader is the stable 4-voter set.
        match c.node(&leader).membership() {
            Membership::Stable { voters: v, .. } => {
                assert_eq!(v.len(), 4);
                assert!(v.contains(&nid("n4")));
            }
            m => panic!("expected stable 4-voter config, got {m:?}"),
        }
    }

    // Post-transition commit under the new quorum; ALL FOUR replicas converge.
    let post = cluster.borrow_mut().propose(&leader, outcome("post")).unwrap();
    cluster.borrow_mut().run(8);
    let c = cluster.borrow();
    assert!(c.commit_of(&leader) >= post.0);
    let reference = c.log_of(&leader).to_vec();
    for id in c.node_ids() {
        assert_eq!(c.log_of(&id), &reference[..], "replica {id:?} did not converge");
        assert_eq!(c.commit_of(&id), c.commit_of(&leader), "replica {id:?} commit");
    }
    // Election Safety held across the whole run (also checked per-step).
    for (term, set) in c.leaders_history() {
        assert!(set.len() <= 1, "term {term} had {set:?}");
    }
}

/// S-I2-5 shape (remove): a follower is removed mid-stream; the group keeps
/// committing under the shrunken quorum, and the removed node — no longer a
/// voter — never becomes leader afterward (a removed node cannot disrupt the
/// group).
#[test]
fn membership_remove_follower_mid_stream_keeps_committing() {
    let mut c = SimCluster::new(4, 0xDE1);
    let leader = c.run_until_leader(300);
    c.propose(&leader, outcome("e1")).unwrap();
    c.propose(&leader, outcome("e2")).unwrap();
    c.run(4);

    let removed = c
        .node_ids()
        .into_iter()
        .find(|n| *n != leader)
        .expect("a follower to remove");
    let remaining: Vec<NodeId> = c.node_ids().into_iter().filter(|n| *n != removed).collect();
    c.change_membership(&leader, Membership::Stable { voters: remaining.clone(), learners: vec![] })
        .unwrap();
    c.run(8);

    // Transition completed; the removed node is not a voter anymore.
    assert_eq!(
        c.node(&leader).membership(),
        Membership::Stable { voters: remaining.clone(), learners: vec![] }
    );

    // Mid-stream continues: kill the removed node entirely; 2-of-3 quorum
    // commits without it.
    c.isolate(&removed);
    let e3 = c.propose(&leader, outcome("e3")).unwrap();
    c.run(60);
    assert!(c.commit_of(&leader) >= e3.0, "new-config quorum commits without the removed node");
    for id in &remaining {
        assert_eq!(c.commit_of(id), c.commit_of(&leader), "replica {id:?} converged");
    }
    // The removed node never led after its removal (it never led at all here,
    // and — being a non-voter — can never campaign again).
    c.heal();
    c.run(120);
    assert_ne!(c.current_leader().expect("leader").0, removed);
    assert_ne!(c.node(&removed).role(), Role::Leader);
}

/// Removing the LEADER itself: once C_new (which excludes it) commits, the
/// leader steps down; the remaining voters elect among themselves and truth
/// committed before the change survives.
#[test]
fn membership_remove_leader_steps_down_and_group_recovers() {
    let mut c = SimCluster::new(3, 0xDEAD);
    let old_leader = c.run_until_leader(300);
    c.propose(&old_leader, outcome("kept")).unwrap();
    c.run(4);

    let remaining: Vec<NodeId> = c.node_ids().into_iter().filter(|n| *n != old_leader).collect();
    c.change_membership(
        &old_leader,
        Membership::Stable { voters: remaining.clone(), learners: vec![] },
    )
    .unwrap();
    // The whole two-phase transition completes in the drain cascade; the
    // leader saw C_new commit and stepped down (Raft §6).
    assert_eq!(c.node(&old_leader).role(), Role::Follower, "removed leader stepped down");

    // The remaining pair elects a successor and keeps committing.
    let new_leader = c.run_until_leader(600);
    assert!(remaining.contains(&new_leader), "successor is a C_new voter");
    let e2 = c.propose(&new_leader, outcome("after")).unwrap();
    c.run(8);
    assert!(c.commit_of(&new_leader) >= e2.0);
    // Pre-change committed truth survived the reconfiguration.
    for id in &remaining {
        assert!(
            c.log_of(id).iter().any(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0 == "h:kept"
            )),
            "committed entry lost on {id:?}"
        );
    }
    for (term, set) in c.leaders_history() {
        assert!(set.len() <= 1, "term {term} had {set:?}");
    }
}

/// THE IDR-003 PROPERTY: no two disjoint majorities exist at any instant of
/// the transition. 3-node group {a,b,c} transits to {leader,n4,n5} (two
/// removed, two added — old and new configs overlap ONLY in the leader).
/// Mid-transition the cluster is split into an old-majority side and a
/// new-majority side:
///   - the old-majority side (leader + one old follower) can NOT commit —
///     joint commits need a C_new majority too;
///   - the new-majority side (one old follower + both joiners) can NOT elect —
///     an election needs a C_old majority too (and joiners without a config
///     never campaign).
/// Neither side decides anything: split-brain is impossible by construction.
/// After heal, the transition completes and the group serves under C_new.
#[test]
fn membership_no_two_disjoint_majorities_during_transition() {
    let mut c = SimCluster::new(3, 0x10DD);
    let leader = c.run_until_leader(300);
    c.propose(&leader, outcome("e1")).unwrap();
    c.run(4);
    let pre_commit = c.commit_of(&leader);
    assert_eq!(pre_commit, 1);

    let old_followers: Vec<NodeId> =
        c.node_ids().into_iter().filter(|n| *n != leader).collect();
    c.add_node(nid("n4"));
    c.add_node(nid("n5"));
    let new_voters = vec![leader.clone(), nid("n4"), nid("n5")];

    // Split BEFORE the change: side A = old majority {leader, f0};
    // side B = new majority {f1, n4, n5}.
    c.partition(&[
        vec![leader.clone(), old_followers[0].clone()],
        vec![old_followers[1].clone(), nid("n4"), nid("n5")],
    ]);
    c.change_membership(
        &leader,
        Membership::Stable { voters: new_voters.clone(), learners: vec![] },
    )
    .unwrap();
    c.propose(&leader, outcome("blocked")).unwrap();
    c.run(80);

    // (1) The old-majority side holds 2/3 of C_old but only 1/3 of C_new:
    //     NOTHING commits during the joint phase.
    assert_eq!(
        c.commit_of(&leader),
        pre_commit,
        "an old-config majority alone must not commit during the transition"
    );
    // (2) The new-majority side holds 3/5-of-union but only 1/3 of C_old:
    //     NOBODY there becomes leader.
    for id in [&old_followers[1], &nid("n4"), &nid("n5")] {
        assert_ne!(c.node(id).role(), Role::Leader, "{id:?} must not lead");
    }
    // The one and only leader in the whole history so far is the old leader.
    assert_eq!(c.current_leader().expect("old leader persists").0, leader);
    for (term, set) in c.leaders_history() {
        assert!(set.len() <= 1, "split-brain: term {term} had {set:?}");
    }

    // Heal: the transition completes (any electable node holds the joint
    // entry — §5.4.1 makes joint-less logs unelectable here) and the group
    // serves under C_new. Let the returning follower's inflated term depose
    // the old-term leader FIRST (run a settle window), then propose in the
    // successor's CURRENT term — with no no-op entry on election (RCR-019
    // DR-2), prior-term entries commit only behind a current-term commit.
    c.heal();
    c.run(30);
    let l2 = c.run_until_leader(600);
    c.propose(&l2, outcome("post-heal")).unwrap();
    c.run(60);
    // A leader outside C_new steps down after C_new commits; settle and
    // re-resolve.
    let l3 = c.run_until_leader(600);
    assert!(new_voters.contains(&l3), "final leader {l3:?} must be a C_new voter");
    let reference = c.log_of(&l3).to_vec();
    assert_eq!(count_membership_entries(&reference), (1, 1), "one Joint then one Stable");
    for tag in ["h:e1", "h:blocked", "h:post-heal"] {
        assert!(
            reference.iter().any(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0 == tag
            )),
            "{tag} missing from the final history"
        );
    }
    for id in &new_voters {
        assert_eq!(c.log_of(id), &reference[..], "C_new replica {id:?} did not converge");
    }
    for (term, set) in c.leaders_history() {
        assert!(set.len() <= 1, "term {term} had {set:?}");
    }
}

/// One reconfiguration in flight per shard (design §3.8): while the joint
/// entry is uncommitted, a second change is refused; via the frozen contract a
/// follower handle is refused with `NotLeader`; a caller-supplied Joint target
/// is always refused (the joint phase is owned by the mechanism, not callers).
#[test]
fn membership_second_change_rejected_while_in_flight() {
    let cluster = Rc::new(RefCell::new(SimCluster::new(3, 0xF117)));
    let leader = cluster.borrow_mut().run_until_leader(300);
    cluster.borrow_mut().propose(&leader, outcome("e1")).unwrap();
    cluster.borrow_mut().run(4);

    let old_followers: Vec<NodeId> = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .filter(|n| *n != leader)
        .collect();
    cluster.borrow_mut().add_node(nid("n4"));
    let mut target_voters = vec![leader.clone(), nid("n4")];
    target_voters.extend(old_followers.iter().cloned());

    // A caller may never submit the joint phase directly.
    assert_eq!(
        cluster
            .borrow_mut()
            .change_membership(
                &leader,
                Membership::Joint {
                    old_voters: vec![leader.clone()],
                    new_voters: target_voters.clone(),
                    learners: vec![],
                },
            )
            .unwrap_err(),
        ConsensusError::MembershipRejected
    );

    // Hold the joint phase open: the leader keeps an old majority (itself +
    // f0) but cannot reach the 3-of-4 C_new majority.
    cluster.borrow_mut().partition(&[
        vec![leader.clone(), old_followers[0].clone()],
        vec![old_followers[1].clone(), nid("n4")],
    ]);
    cluster
        .borrow_mut()
        .change_membership(
            &leader,
            Membership::Stable { voters: target_voters.clone(), learners: vec![] },
        )
        .expect("first change begins");
    // Second change while the first is in flight: refused.
    assert_eq!(
        cluster
            .borrow_mut()
            .change_membership(
                &leader,
                Membership::Stable { voters: vec![leader.clone()], learners: vec![] },
            )
            .unwrap_err(),
        ConsensusError::MembershipRejected
    );
    // Frozen-contract path on a follower: NotLeader.
    let fh = SimShardConsensus::new(shard(), old_followers[0].clone(), cluster.clone());
    assert!(matches!(
        fh.change_membership(
            &shard(),
            Membership::Stable { voters: target_voters.clone(), learners: vec![] }
        ),
        Err(ConsensusError::NotLeader { .. })
    ));

    // Heal → the in-flight transition completes deterministically (every
    // electable node holds the joint entry) once a commit flows.
    cluster.borrow_mut().heal();
    let l2 = cluster.borrow_mut().run_until_leader(600);
    let post = cluster.borrow_mut().propose(&l2, outcome("post")).unwrap();
    cluster.borrow_mut().run(60);
    let c = cluster.borrow();
    assert!(c.commit_of(&l2) >= post.0);
    match c.node(&l2).membership() {
        Membership::Stable { voters: v, .. } => {
            assert_eq!(v.len(), 4, "transition completed to the 4-voter C_new");
            assert!(v.contains(&nid("n4")));
        }
        m => panic!("expected completed stable config, got {m:?}"),
    }
    for (term, set) in c.leaders_history() {
        assert!(set.len() <= 1, "term {term} had {set:?}");
    }
}

/// Part (c): LEADERSHIP TRANSFER — the leader hands off to a caught-up voter
/// via TimeoutNow; the target campaigns immediately at term+1 and wins; the
/// old leader is deposed by the higher term; no committed entry is lost and
/// Election Safety holds across the handover.
#[test]
fn leadership_transfer_hands_over_without_truth_loss() {
    let mut c = SimCluster::new(3, 0x7AF);
    let old_leader = c.run_until_leader(300);
    c.propose(&old_leader, outcome("e1")).unwrap();
    c.propose(&old_leader, outcome("e2")).unwrap();
    c.run(4);
    let commit_before = c.commit_of(&old_leader);
    let old_term = c.node(&old_leader).current_term();

    let target = c
        .node_ids()
        .into_iter()
        .find(|n| *n != old_leader)
        .expect("a follower to receive the lead");
    c.transfer_leadership(&old_leader, &target).expect("transfer fires");

    // The target leads the NEXT term; the old leader stepped down.
    let (new_leader, new_term) = c.current_leader().expect("a leader after transfer");
    assert_eq!(new_leader, target);
    assert_eq!(new_term, old_term.0 + 1, "transfer costs exactly one term");
    assert_eq!(c.node(&old_leader).role(), Role::Follower);

    // No committed truth lost; the group keeps committing under the new lead.
    assert!(c.commit_of(&target) >= commit_before);
    let e3 = c.propose(&target, outcome("e3")).unwrap();
    c.run(8);
    assert!(c.commit_of(&target) >= e3.0);
    let reference = c.log_of(&target).to_vec();
    for id in c.node_ids() {
        assert_eq!(c.log_of(&id), &reference[..], "replica {id:?} did not converge");
    }
    for (term, set) in c.leaders_history() {
        assert!(set.len() <= 1, "term {term} had {set:?}");
    }
}

//! RCR-020 — per-shard consensus instance map (I2 Stage 2, part a).
//!
//! IDR-001: exactly one INDEPENDENT Raft group per immutable
//! `(tenant, workspace)` shard — per-shard leader election, shared-nothing.
//! SHARD-001: blast-radius isolation — one shard's fault cannot touch another
//! shard's leadership, term, log, or commits.
//!
//! Every test is deterministic: seeded, scripted, injected logical ticks —
//! no sleeps, no wall clocks, no OS entropy. HONEST SCOPE: in-process
//! simulation (bus filters), not network fault-tolerance evidence.

use arves_consensus::sim::{SimCluster, SimShardMap};
use arves_consensus::{
    ConsensusError, ContentHash, EntryKind, LogIndex, Outcome, ShardConsensus, ShardId, TenantId,
    WorkspaceId,
};
use std::cell::RefCell;
use std::rc::Rc;

fn shard(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}

fn outcome_of(tag: &str) -> Outcome {
    Outcome { digest: ContentHash(format!("h:{tag}")), payload: tag.as_bytes().to_vec() }
}

/// Two shards, two independent groups behind ONE frozen-contract handle:
/// independent logs, independent indices, zero cross-shard bytes (SHARD-001),
/// and `UnknownShard` for anything unregistered.
#[test]
fn shard_map_routes_to_independent_groups_per_shard() {
    let s1 = shard("t1", "w1");
    let s2 = shard("t2", "w2");
    let c1 = Rc::new(RefCell::new(SimCluster::new(3, 0x51)));
    let c2 = Rc::new(RefCell::new(SimCluster::new(3, 0x52)));
    let l1 = c1.borrow_mut().run_until_leader(300);
    let l2 = c2.borrow_mut().run_until_leader(300);

    let mut map = SimShardMap::new();
    map.register(s1.clone(), l1.clone(), c1.clone());
    map.register(s2.clone(), l2.clone(), c2.clone());
    assert_eq!(map.shards().len(), 2);

    // Independent per-shard logs: indices advance per group, not globally.
    let i1 = map.propose(&s1, outcome_of("s1-e1")).unwrap();
    let i2a = map.propose(&s2, outcome_of("s2-e1")).unwrap();
    let i2b = map.propose(&s2, outcome_of("s2-e2")).unwrap();
    assert_eq!(i1, LogIndex(1));
    assert_eq!((i2a, i2b), (LogIndex(1), LogIndex(2)));
    map.await_commit(&s1, i1).unwrap();
    map.await_commit(&s2, i2b).unwrap();

    // SHARD-001: zero cross-shard leakage — every byte in a group's log
    // belongs to that group's shard.
    for id in c1.borrow().node_ids() {
        assert!(
            c1.borrow().log_of(&id).iter().all(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0.starts_with("h:s1-")
            )),
            "foreign-shard bytes in shard-1 group at {id:?}"
        );
    }
    for id in c2.borrow().node_ids() {
        assert!(
            c2.borrow().log_of(&id).iter().all(|e| matches!(
                &e.kind,
                EntryKind::Outcome(o) if o.digest.0.starts_with("h:s2-")
            )),
            "foreign-shard bytes in shard-2 group at {id:?}"
        );
    }

    // Frozen-contract refusal for an unregistered shard.
    let s3 = shard("t3", "w3");
    assert_eq!(map.leader(&s3).unwrap_err(), ConsensusError::UnknownShard(s3.clone()));
    assert_eq!(
        map.propose(&s3, outcome_of("x")).unwrap_err(),
        ConsensusError::UnknownShard(s3)
    );
}

/// Blast radius = one shard (SHARD-001; design §3.13): isolating shard 1's
/// leader forces a re-election THERE (per-shard election, IDR-004) while
/// shard 2's leader, term, and commits are bit-for-bit untouched — with both
/// clusters ticking through the whole fault window.
#[test]
fn shard_map_leader_fault_blast_radius_is_one_shard() {
    let c1 = Rc::new(RefCell::new(SimCluster::new(3, 0x61)));
    let c2 = Rc::new(RefCell::new(SimCluster::new(3, 0x62)));
    let l1 = c1.borrow_mut().run_until_leader(300);
    let l2 = c2.borrow_mut().run_until_leader(300);

    // Committed work in both shards before the fault.
    c1.borrow_mut().propose(&l1, EntryKind::Outcome(outcome_of("s1-pre"))).unwrap();
    c2.borrow_mut().propose(&l2, EntryKind::Outcome(outcome_of("s2-pre"))).unwrap();
    c1.borrow_mut().run(4);
    c2.borrow_mut().run(4);
    let term2_before = c2.borrow().node(&l2).current_term();
    let commit2_before = c2.borrow().commit_of(&l2);

    // Fault in shard 1 only; time passes in BOTH shards.
    c1.borrow_mut().isolate(&l1);
    for _ in 0..200 {
        c1.borrow_mut().tick();
        c2.borrow_mut().tick();
    }

    // Shard 1 re-elected (per-shard leader election, IDR-004)…
    let (nl1, nt1) = c1.borrow().current_leader().expect("shard-1 majority re-elects");
    assert_ne!(nl1, l1, "a new shard-1 leader emerged");
    assert!(nt1 > 1, "shard-1 election advanced the term");
    // …and its committed entry survived (Leader Completeness held per shard).
    assert!(c1.borrow().log_of(&nl1).iter().any(|e| matches!(
        &e.kind,
        EntryKind::Outcome(o) if o.digest.0 == "h:s1-pre"
    )));

    // Shard 2: same leader, same term, same commit — untouched.
    assert_eq!(c2.borrow().current_leader().expect("shard-2 leader"), (l2.clone(), term2_before.0));
    assert_eq!(c2.borrow().commit_of(&l2), commit2_before);
    // And shard 2 still commits new work.
    let idx = c2.borrow_mut().propose(&l2, EntryKind::Outcome(outcome_of("s2-post"))).unwrap();
    c2.borrow_mut().run(4);
    assert!(c2.borrow().commit_of(&l2) >= idx.0);
}

/// IDR-001: exactly ONE Raft group per shard — a duplicate registration is
/// refused loudly.
#[test]
#[should_panic(expected = "IDR-001 violation")]
fn shard_map_duplicate_group_registration_panics() {
    let s = shard("t1", "w1");
    let c = Rc::new(RefCell::new(SimCluster::new(3, 0x71)));
    let local = c.borrow().node_ids()[0].clone();
    let mut map = SimShardMap::new();
    map.register(s.clone(), local.clone(), c.clone());
    map.register(s, local, c); // second group for the same shard: panic
}

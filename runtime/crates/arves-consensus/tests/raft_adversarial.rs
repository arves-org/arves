//! RCR-022 (I2 Stage 4) — adversarial distributed proofs at the CONSENSUS level.
//!
//! Duplicate + reordered message storms over the deterministic bus
//! (`SimCluster::set_mangling`, counter-scripted — no randomness), with the
//! RCR-019 harness re-checking all four Raft safety properties after EVERY
//! message step. The property under proof: at-least-once, out-of-order
//! delivery NEVER double-commits an entry — every committed log index holds
//! exactly one entry, every proposed outcome commits at most once, and all
//! replicas converge to the identical log (ORCH-004 idempotency at the
//! cluster level; ORCH-003 replayability of the adversarial run itself).
//!
//! Every test is deterministic: fixed seeds, scripted schedules, injected
//! logical ticks — no sleeps, no wall clocks, no OS randomness.

use arves_consensus::sim::SimCluster;
use arves_consensus::{ContentHash, EntryKind, Outcome};
use std::collections::BTreeMap;

fn outcome(tag: &str) -> EntryKind {
    EntryKind::Outcome(Outcome {
        digest: ContentHash(format!("h:{tag}")),
        payload: tag.as_bytes().to_vec(),
    })
}

/// Collect `digest -> occurrence count` over the safety observer's committed
/// history (index -> first committed entry). Any digest counted twice means a
/// double-commit — the exact failure ORCH-004 forbids.
fn committed_digest_counts(c: &SimCluster) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for (_idx, entry) in c.committed_history() {
        if let EntryKind::Outcome(o) = entry.kind {
            *counts.entry(o.digest.0).or_insert(0) += 1;
        }
    }
    counts
}

/// ORCH-004 at cluster level, log form: under a duplicate + reordered
/// delivery storm (every 2nd delivered message duplicated stale, every 3rd
/// pop deferred behind later traffic), every proposed outcome commits EXACTLY
/// once, every replica converges to the identical log and commit index, and
/// the four safety properties held after every message step (the harness
/// panics on any violation). The test bites: it asserts the storm actually
/// duplicated and reordered messages.
#[test]
fn adversarial_dup_reorder_storm_commits_each_entry_exactly_once() {
    let mut c = SimCluster::new(5, 0x5EED_0022);
    let leader = c.run_until_leader(400);
    c.set_mangling(2, 3);

    let tags = ["e1", "e2", "e3", "e4", "e5", "e6"];
    for t in &tags {
        c.propose(&leader, outcome(t)).expect("leader proposal under storm");
    }
    c.run(12); // heartbeats (also mangled) carry commit to every replica

    assert!(c.duplicated > 0, "the storm must actually duplicate messages");
    assert!(c.deferred > 0, "the storm must actually reorder messages");

    // Exactly-once commit per proposed digest; nothing foreign committed.
    let counts = committed_digest_counts(&c);
    assert_eq!(counts.len(), tags.len(), "every proposal committed, nothing extra");
    for t in &tags {
        assert_eq!(counts.get(&format!("h:{t}")), Some(&1), "{t} committed exactly once");
    }

    // Full convergence: identical log + commit index on every replica.
    let ids = c.node_ids();
    let reference = c.log_of(&leader).to_vec();
    for id in &ids {
        assert_eq!(c.commit_of(id), tags.len() as u64, "replica {id:?} commit index");
        assert_eq!(c.log_of(id), &reference[..], "replica {id:?} identical log");
    }
}

/// Duplicate/reorder storm PLUS partition churn and a failover: the deposed
/// leader's un-replicated proposal is either superseded or lost — but NEVER
/// double-committed, and no committed index ever changes identity (State
/// Machine Safety, checked per step). Post-heal proposals through the
/// successor commit exactly once and all replicas converge.
#[test]
fn adversarial_dup_reorder_with_partition_churn_no_double_commit() {
    let mut c = SimCluster::new(5, 0xC0FFEE);
    let old_leader = c.run_until_leader(400);
    c.set_mangling(2, 3);

    c.propose(&old_leader, outcome("pre")).expect("pre-fault proposal");
    c.run(12);
    assert_eq!(committed_digest_counts(&c).get("h:pre"), Some(&1));

    // Cut the leader off mid-stream; its next proposal can never commit.
    c.isolate(&old_leader);
    c.propose(&old_leader, outcome("orphan")).expect("append at the stale leader");
    c.run(60); // the majority elects a successor under the storm

    // Heal and keep committing through the (possibly new) leader.
    c.heal();
    c.run(60);
    let (leader, _) = c.current_leader().expect("leader after heal");
    c.propose(&leader, outcome("post")).expect("post-heal proposal");
    c.run(12);

    // No digest ever committed twice; the survivors committed exactly once.
    let counts = committed_digest_counts(&c);
    for (digest, n) in &counts {
        assert_eq!(*n, 1, "digest {digest} double-committed");
    }
    assert_eq!(counts.get("h:pre"), Some(&1));
    assert_eq!(counts.get("h:post"), Some(&1));

    // Convergence: identical logs and commit indexes everywhere.
    let ids = c.node_ids();
    let reference = c.log_of(&leader).to_vec();
    let commit = c.commit_of(&leader);
    for id in &ids {
        assert_eq!(c.commit_of(id), commit, "replica {id:?} commit index");
        assert_eq!(c.log_of(id), &reference[..], "replica {id:?} identical log");
    }
}

/// The adversarial run itself is replayable (ORCH-003): identical seed +
/// identical scripted schedule (storm, partition, heal included) ⇒ identical
/// full-history digest — the mangling trace is counter-scripted, part of the
/// digest, and carries zero hidden randomness.
#[test]
fn adversarial_runs_are_deterministic_for_identical_seeds() {
    let run = |seed: u64| {
        let mut c = SimCluster::new(5, seed);
        let leader = c.run_until_leader(400);
        c.set_mangling(2, 3);
        c.propose(&leader, outcome("e1")).unwrap();
        c.run(8);
        c.isolate(&leader);
        c.run(40);
        c.heal();
        c.run(40);
        c.digest()
    };
    assert_eq!(run(0xD00D), run(0xD00D), "identical seed ⇒ identical mangled history");
    assert_ne!(run(0xD00D), run(0xBEEF), "different seeds explore different histories");
}

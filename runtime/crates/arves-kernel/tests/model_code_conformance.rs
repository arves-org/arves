//! RCR-040 — MODEL <-> CODE CONFORMANCE WITNESS.
//!
//! Closes the "model-checks the DESIGN, not the Rust code" gap: the TLA+ models
//! in `verification/formal/` prove safety over ALL interleavings of a small
//! abstract state machine, but nothing tied those proofs to the RUNNING
//! reference code. This test is that missing link — it drives the REAL kernel
//! (`RefKernel`) and the REAL cluster kernel (`ClusterKernel` / `ClusterSim`)
//! through the EXACT concrete scenarios the models check, and asserts the code's
//! OBSERVABLE state matches what each model invariant requires at each step.
//!
//! # HONEST SCOPE — what this IS and IS NOT
//!
//! - It IS a **concrete-scenario conformance witness**: for ONE trace per model,
//!   every checked model invariant is mapped to a Rust assertion on the real
//!   code's observable state, so "the model checks" and "the code behaves" are
//!   linked claims, not disconnected ones.
//! - It IS NOT a **refinement proof** or a **formal code proof**. It does not
//!   show the Rust code refines the TLA+ spec over ALL states/interleavings —
//!   that needs model-to-code tooling (Kani / TLA+-level code extraction) and is
//!   recorded as an Open Question (RCR-040 OQ-1). TLC still owns the
//!   all-interleavings argument; this test owns the one-trace bridge.
//! - The mapping table (each model invariant -> the assertion that witnesses it)
//!   lives in `verification/formal/MODEL_CODE_CONFORMANCE.md`.
//!
//! Every step is deterministic: fixed seeds, scripted faults, injected logical
//! ticks — no sleeps, no wall clocks, no OS randomness (ORCH-003 replayable).

use arves_acs::content_id;
use arves_consensus::{ShardId, TenantId, WorkspaceId};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{
    CommitError, ContentHash, Kernel, MemKernel, ProposedWrite, RefKernel, ShardKey,
};
use arves_persistence::MemWalStore;
use std::cell::RefCell;
use std::rc::Rc;

// ===========================================================================
// PART 1 — ARVES_Kernel.tla  (single-shard kernel core, scenario AKF-CS-1)
//
// The model checks four safety properties over the commit gateway (§4 of
// ARVES_Kernel_Formal_Spec.md). AKF-CS-1 (§7) is the byte-exact trace that
// binds those abstract tokens to concrete ACS-001 ContentIds. Here we RUN that
// trace on `RefKernel` and witness each property on the code's real state.
// ===========================================================================

/// The AKF-CS-1 domain tag (0x01 = commit-content) and byte-exact expectations
/// stated in `ARVES_Kernel_Formal_Spec.md` §7. Reproduced here from the runtime's
/// OWN SHA-256 (`arves-acs`, zero new deps) — if either the doc's vectors or the
/// kernel's `truth_hash` fold drifted, this test fails (the anti-drift tripwire
/// the formal spec's Risk (b) calls for).
const COMMIT_DOMAIN_TAG: u8 = 0x01;
const AKF_CS1_TRUTH_HASH: u64 = 0x7bb9_b2e3_0ee7_427c; // decimal 8915353625633964668
const AKF_CS1_TRUTH_HASH_REORDERED: u64 = 0xed1b_740f_02f0_b8f4;

fn cs1_proposal(body: &str) -> ProposedWrite {
    // c = ContentId(0x01 || body); payload = the 34-byte ContentId itself
    // (exactly the AKF-CS-1 setup: "each payload equals the 34-byte ContentId").
    let cid = content_id(COMMIT_DOMAIN_TAG, body.as_bytes());
    ProposedWrite {
        shard: ShardKey::new("acme", "research").expect("well-formed shard key"),
        content: ContentHash(cid.clone()),
        payload: cid,
    }
}

/// AKF-CS-1: RefKernel realizes the model trace and each ARVES_Kernel.tla
/// invariant is witnessed byte-exactly on the running code.
#[test]
fn akf_cs1_kernel_model_conformance_witness() {
    let store = MemWalStore::new();
    let k: MemKernel = RefKernel::new(store.clone());

    // Model tokens c1, c2 bound to concrete ACS-001 ContentIds (§7 table).
    let c1 = cs1_proposal("truth-alpha");
    let c2 = cs1_proposal("truth-beta");
    // Reproduce the §7 ContentId vectors from the runtime's own SHA-256.
    assert_eq!(
        arves_acs::hex(&c1.content.0),
        "12206623c3d81c6f9a6ecf04ee9d474ffbcb31e29bb45ce01070d6bef20506d63f10",
        "AKF-CS-1 c1 ContentId vector"
    );
    assert_eq!(
        arves_acs::hex(&c2.content.0),
        "1220095e3f1504ab8cee6c3b52ad1344d46e21d3ae4cf527cf119308034bf4345a34",
        "AKF-CS-1 c2 ContentId vector"
    );

    // Step 1: Propose(c1), Commit(c1) -> log <<c1>>, truth {c1}, c1@index 0.
    let tr1 = k.commit(c1.clone()).expect("commit c1");
    assert_eq!(tr1.index.0, 0, "c1 at log index 0");
    // Step 2: Propose(c2), Commit(c2) -> log <<c1,c2>>, truth {c1,c2}, c2@index 1.
    let tr2 = k.commit(c2.clone()).expect("commit c2");
    assert_eq!(tr2.index.0, 1, "c2 at log index 1");

    // --- OWN_001 (model §4.1): Count(log,c) <= 1; single writer, no fork. -----
    // committed_count == 2 == number of distinct contents; each appears once.
    assert_eq!(k.committed_count(), 2, "OWN_001: one record per content");

    // --- ORCH_003_ReplayEquiv (model §4.3): Range(log) = truth, byte-exact. ---
    let truth_before = k.truth_hash();
    assert_eq!(
        truth_before, AKF_CS1_TRUTH_HASH,
        "ORCH-003b: canonical-order truth digest is the §7 byte-exact value"
    );
    // Replay reconstructs truth FROM the log (recover = try_replay); the digest
    // is identical before and after — replay is faithful, not a recomputation.
    let replayed: MemKernel = RefKernel::recover(store.clone());
    assert_eq!(
        replayed.truth_hash(),
        truth_before,
        "ORCH-003b: truth_hash identical before and after replay()"
    );
    assert_eq!(replayed.committed_count(), 2);

    // --- ORCH_004 (model §4.2): idempotent, content-addressable commit. -------
    // Step 3: Propose(c1) again, Commit(c1) -> idempotent. Log UNCHANGED, truth
    // UNCHANGED; surfaced as AlreadyCommitted resolving to the step-1 TruthRef.
    match k.commit(c1.clone()) {
        Err(CommitError::AlreadyCommitted(tr)) => {
            assert_eq!(tr, tr1, "ORCH-004: re-commit resolves to the existing TruthRef");
        }
        other => panic!("ORCH-004: expected AlreadyCommitted, got {other:?}"),
    }
    assert_eq!(k.committed_count(), 2, "ORCH-004: no second log record appended");
    assert_eq!(
        k.truth_hash(),
        AKF_CS1_TRUTH_HASH,
        "ORCH-004: truth digest identical after the idempotent re-commit"
    );

    // --- order matters (model §6 note / §7): the SET abstraction is necessary --
    // but not sufficient — the SAME two truths in the OPPOSITE commit order
    // yield a DIFFERENT digest, so the code preserves log ORDER, not just the set.
    let store_r = MemWalStore::new();
    let kr: MemKernel = RefKernel::new(store_r);
    kr.commit(c2.clone()).expect("commit c2 first");
    kr.commit(c1.clone()).expect("commit c1 second");
    assert_eq!(
        kr.truth_hash(),
        AKF_CS1_TRUTH_HASH_REORDERED,
        "reordered commit yields the §7 reordered digest"
    );
    assert_ne!(
        kr.truth_hash(),
        AKF_CS1_TRUTH_HASH,
        "order-sensitivity: reordered truth is a DIFFERENT digest"
    );
}

// ===========================================================================
// PART 2 — ARVES_Cluster.tla  (N-node per-shard Raft, safety invariants §)
//
// The model checks five safety invariants over the distributed commit/leader
// protocol: ElectionSafety, LogMatching, StateMachineSafety, LeaderCompleteness,
// LinearizableCommit. We drive the REAL 3-node ClusterKernel/ClusterSim through
// a concrete trace whose steps mirror the model's actions
// (Timeout/Vote/BecomeLeader -> ClientRequest -> Replicate -> AdvanceCommit,
// then a term-changing failover), and witness every invariant on observable
// state. Server = {n1,n2,n3} matches ARVES_Cluster_MC.cfg.
// ===========================================================================

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}

fn skey(t: &str, w: &str) -> ShardKey {
    ShardKey::new(t, w).expect("well-formed shard key")
}

fn pw(tag: &str) -> ProposedWrite {
    ProposedWrite {
        shard: skey("t1", "w1"),
        content: ContentHash(format!("c:{tag}").into_bytes()),
        payload: format!("p:{tag}").into_bytes(),
    }
}

/// Model invariant `ElectionSafety` (§ "at most one leader per term"): witness
/// on `leaders_by_term_of` — every observed term maps to at most one leader.
fn assert_election_safety(c: &ClusterSim, shard: &ShardId) {
    for (term, leaders) in c.leaders_by_term_of(shard) {
        assert!(
            leaders.len() <= 1,
            "ElectionSafety: term {term} had leaders {leaders:?}"
        );
    }
}

/// Model invariant `LogMatching` (§ "shared (index,term) => shared prefix"):
/// witness pairwise on `log_terms_of` — where two logs agree at an index, their
/// whole preceding term-prefix agrees.
fn assert_log_matching(c: &ClusterSim, shard: &ShardId) {
    let ids = c.node_ids();
    for a in 0..ids.len() {
        for b in (a + 1)..ids.len() {
            let la = c.log_terms_of(&ids[a], shard);
            let lb = c.log_terms_of(&ids[b], shard);
            let min = la.len().min(lb.len());
            for n in (0..min).rev() {
                if la[n] == lb[n] {
                    assert_eq!(
                        la[..=n],
                        lb[..=n],
                        "LogMatching: {:?}/{:?} share index {} term {} but prefixes differ",
                        ids[a],
                        ids[b],
                        n + 1,
                        la[n]
                    );
                    break;
                }
            }
        }
    }
}

/// Model invariants `StateMachineSafety` (§ "no two different entries committed
/// at the same index") and `LinearizableCommit` (§ "committed prefixes of any
/// two replicas agree"): witness that every replica's committed prefix matches
/// the single-valued `committed_terms_of` history, AND that the Kernel TRUTH
/// derived from those committed entries is byte-identical across replicas
/// (ORCH-003 — the code-level consequence the model guarantees).
fn assert_state_machine_and_linearizable(c: &ClusterSim, shard: &ShardId) {
    let committed = c.committed_terms_of(shard); // idx -> term, single-valued map
    for node in c.node_ids() {
        let terms = c.log_terms_of(&node, shard);
        let ci = c.commit_index_of(&node, shard);
        for i in 1..=ci {
            // The node's committed entry at i agrees with the cluster-wide
            // committed history (no divergent entry at a committed index).
            assert_eq!(
                terms.get((i - 1) as usize).copied(),
                committed.get(&i).copied(),
                "StateMachineSafety: {node:?} committed index {i} disagrees with history"
            );
        }
    }
    // LinearizableCommit at the TRUTH level: every replica holds byte-identical
    // per-shard truth and the same commit-order digest.
    let reference = c.node_ids()[0].clone();
    let blob = c.shard_state_of(&reference, shard);
    let hash = c.truth_hash_of(&reference);
    let count = c.committed_count_of(&reference);
    for node in c.node_ids() {
        assert_eq!(c.shard_state_of(&node, shard), blob, "{node:?} byte-identical truth");
        assert_eq!(c.truth_hash_of(&node), hash, "{node:?} identical truth digest");
        assert_eq!(c.committed_count_of(&node), count, "{node:?} identical count");
    }
}

/// The cluster model<->code conformance witness. One deterministic trace whose
/// steps mirror `ARVES_Cluster.tla`'s actions; every model safety invariant is
/// asserted on the running ClusterKernel/ClusterSim's observable state.
#[test]
fn cluster_model_conformance_witness() {
    let shard = sid("t1", "w1");
    let mut sim = ClusterSim::new(3); // Server = {n1,n2,n3} (matches the MC cfg)
    sim.add_shard(shard.clone(), 7);

    // Timeout/Vote/BecomeLeader: exactly one leader emerges for its term.
    let old_leader = sim.elect(&shard);
    let cluster = Rc::new(RefCell::new(sim));
    assert_election_safety(&cluster.borrow(), &shard);

    // ClientRequest + Replicate + AdvanceCommit: commit two entries at the
    // leader; quorum carries them to every replica.
    let k_old = ClusterKernel::new(old_leader.clone(), cluster.clone());
    let tr1 = k_old.commit(pw("e1")).expect("commit e1");
    let tr2 = k_old.commit(pw("e2")).expect("commit e2");
    assert_eq!((tr1.index.0, tr2.index.0), (0, 1), "dense WAL offsets");
    cluster.borrow_mut().settle(4);

    // Witness the convergence invariants after the happy-path replication.
    {
        let c = cluster.borrow();
        assert_state_machine_and_linearizable(&c, &shard);
        assert_log_matching(&c, &shard);
        assert_election_safety(&c, &shard);
        // AdvanceCommit landed on every replica (commit-on-quorum, IDR-001).
        for node in c.node_ids() {
            assert_eq!(c.commit_index_of(&node, &shard), 2, "{node:?} commit index");
        }
    }

    // Capture the committed prefix BEFORE the failover — the entries a
    // later-term leader must retain (LeaderCompleteness witness input).
    let committed_before = cluster.borrow().committed_terms_of(&shard);
    let old_term = cluster
        .borrow()
        .leaders_by_term_of(&shard)
        .keys()
        .copied()
        .max()
        .expect("a leader term exists");

    // Fault (Timeout at a higher term on the majority side): isolate the leader
    // into the minority. Its next commit cannot reach quorum -> NotReplicated,
    // ZERO partial truth anywhere (IDR-001 CP; A-005/A-006).
    cluster.borrow_mut().isolate(&shard, &old_leader);
    assert_eq!(
        k_old.commit(pw("e3")),
        Err(CommitError::NotReplicated),
        "no quorum, no truth (IDR-001 CP)"
    );
    {
        let c = cluster.borrow();
        for node in c.node_ids() {
            assert_eq!(c.committed_count_of(&node), 2, "{node:?} no partial truth");
        }
    }

    // Heal; the majority elected a successor at a strictly HIGHER term during
    // the outage (the model's BecomeLeader at term+1).
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("leader after heal");
    assert_ne!(new_leader, old_leader, "the isolated leader was deposed");
    let new_term = cluster
        .borrow()
        .leaders_by_term_of(&shard)
        .keys()
        .copied()
        .max()
        .expect("a leader term exists");
    assert!(new_term > old_term, "new leader's term {new_term} > old term {old_term}");

    // --- LeaderCompleteness (§): the later-term leader retains every entry ----
    // committed under an earlier term (its log reproduces the pre-failover
    // committed (idx,term) prefix). This is why committed truth is never erased.
    {
        let c = cluster.borrow();
        let new_leader_terms = c.log_terms_of(&new_leader, &shard);
        for (idx, term) in &committed_before {
            assert_eq!(
                new_leader_terms.get((*idx - 1) as usize).copied(),
                Some(*term),
                "LeaderCompleteness: new leader lacks committed entry idx {idx} term {term}"
            );
        }
        assert_election_safety(&c, &shard);
    }

    // ClientRequest through the NEW leader: e3 commits FRESH, exactly once; the
    // deposed leader's un-replicated suffix was truncated, never applied.
    let k_new = ClusterKernel::new(new_leader.clone(), cluster.clone());
    let tr3 = k_new.commit(pw("e3")).expect("post-heal commit e3");
    assert_eq!(tr3.index.0, 2, "dense continuation after the discarded suffix");
    cluster.borrow_mut().settle(6);

    // All five model invariants hold on the final converged state.
    {
        let c = cluster.borrow();
        assert_state_machine_and_linearizable(&c, &shard);
        assert_log_matching(&c, &shard);
        assert_election_safety(&c, &shard);
        for node in c.node_ids() {
            assert_eq!(c.committed_count_of(&node), 3, "{node:?} exactly-once, all three");
        }
        // committed history is single-valued at every index (StateMachineSafety):
        // building the map above never panicked on a divergent index, and the
        // per-replica agreement was asserted — record the covered indices.
        assert_eq!(c.committed_terms_of(&shard).len(), 3, "three committed indices");
    }

    // ORCH-004 under replication (ties the cluster back to the kernel model's
    // idempotency): an identical re-proposal resolves to the SAME truth.
    match k_new.commit(pw("e1")) {
        Err(CommitError::AlreadyCommitted(tr)) => assert_eq!(tr, tr1),
        other => panic!("ORCH-004: expected AlreadyCommitted, got {other:?}"),
    }
    assert_eq!(cluster.borrow().committed_count_of(&new_leader), 3, "no truth delta");
}

/// Determinism (ORCH-003): the whole model<->code cluster witness is a pure
/// function of its seed — two identical runs produce identical observable truth
/// digests on every replica. (The witness is only meaningful if replayable.)
#[test]
fn cluster_conformance_witness_is_deterministic() {
    let run = || -> Vec<u64> {
        let shard = sid("t1", "w1");
        let mut sim = ClusterSim::new(3);
        sim.add_shard(shard.clone(), 7);
        let leader = sim.elect(&shard);
        let cluster = Rc::new(RefCell::new(sim));
        let k = ClusterKernel::new(leader.clone(), cluster.clone());
        k.commit(pw("e1")).expect("e1");
        k.commit(pw("e2")).expect("e2");
        cluster.borrow_mut().settle(4);
        cluster.borrow_mut().isolate(&shard, &leader);
        let _ = k.commit(pw("e3"));
        cluster.borrow_mut().heal(&shard);
        cluster.borrow_mut().settle(60);
        let nl = cluster.borrow().leader_of(&shard).expect("leader");
        ClusterKernel::new(nl, cluster.clone()).commit(pw("e3")).expect("e3");
        cluster.borrow_mut().settle(6);
        let c = cluster.borrow();
        c.node_ids().iter().map(|id| c.truth_hash_of(id)).collect()
    };
    assert_eq!(run(), run(), "identical seed => identical truth on every replica");
}

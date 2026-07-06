//! RCR-028 (I4 Stage 3) — ADVERSARIAL SCHEDULING PROOFS.
//!
//! The destroy pass over the RCR-026/027 scheduling surface, per the I4 design's
//! conformance plan (docs/design/I4_Capability_Scheduling_Design.md §4 proof table,
//! §5.1 axes 4/7/8/10/12, §5.2 "a new distributed scheduling scenario: leader
//! failover mid-dispatch; duplicate dispatch; shard flood"):
//!
//! - (a) STORM / DUPLICATE / REORDER schedules — no double-execution at the truth
//!   boundary; ORCH-004 holds AT THE SCHEDULING LAYER (duplicate submissions
//!   collapse, reordered racing schedules converge, every unique invocation's
//!   truth lands exactly once).
//! - (b) NODE DEATH MID-INVOCATION — the placed node dies between placement and
//!   quorum; work is re-placed WITHOUT a duplicate commit; the content-addressed
//!   idempotency key carries across node death AND scheduler death.
//! - (c) BACKPRESSURE HONESTY — every over-capacity refusal is explicit, accounted
//!   1:1 against submissions, stateless and retriable; a silent drop is impossible
//!   by accounting.
//! - (d) FAILURE ISOLATION — a poisoned capability, even as a storm, cannot block
//!   its shard or the cluster.
//! - (e) LEADERSHIP CHANGE MID-SCHEDULE — an invocation straddling a leadership
//!   change (old leader survives and rejoins) lands exactly once.
//! - (f) POLICY-FLIP RE-SUBMISSION (adversarial revision) — the Governance
//!   `PolicyVerdict` is a caller input NOT encoded in the ORCH-004 dedupe key:
//!   a Deny-quarantined key re-submitted under `Allow` collapses onto the
//!   recorded refusal for the scheduler's lifetime (visible, never silent); a
//!   FRESH scheduler (ORCH-002 discardability) re-admits and completes.
//!
//! Every test is deterministic: fixed seeds, logical ticks, `Cell` counters,
//! scripted partitions/elections/presence — zero wall clocks, zero OS randomness,
//! zero sleeps. HONEST SCOPE: in-process simulation over the I2 `ClusterSim` (no
//! network, no remote execution; placement is a recorded node label — RCR-027
//! DR-9); at-least-once COMPUTE / at-most-once TRUTH is the model (design §6.1);
//! external side effects still rely on declared `EffectClass` honesty (v1.1 debt
//! #2). CAP-001..009 remain PROPOSED and are not enforced.

use arves_capability_fabric::gate::PolicyVerdict;
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as FabricShardKey,
};
use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_control_plane::scheduler::{
    ClusterScheduler, DispatchEnv, EngineHost, InvocationSpec, SchedulerConfig,
    SchedulingDecision, SubmitOutcome, WorkState,
};
use arves_engine_fabric::{
    invocation_key, Determinism, Engine, EngineManifest, IdempotencyKey, Inference, ProposedEffect,
};
use arves_kernel::cluster::ClusterSim;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Deterministic scaffolding (the RCR-027 test vocabulary, reused verbatim)
// ---------------------------------------------------------------------------

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}

fn fshard(s: &ShardId) -> FabricShardKey {
    FabricShardKey::new(s.tenant.0.clone(), s.workspace.0.clone()).expect("well-formed test shard")
}

/// N-node cluster hosting one seeded Raft group per shard, each elected.
fn cluster(nodes: usize, shards: &[ShardId]) -> Rc<RefCell<ClusterSim>> {
    let mut sim = ClusterSim::new(nodes);
    for (i, s) in shards.iter().enumerate() {
        sim.add_shard(s.clone(), 28 + i as u64); // fixed seeds: recorded ⇒ replayable
    }
    let c = Rc::new(RefCell::new(sim));
    for s in shards {
        c.borrow_mut().elect(s);
    }
    c
}

fn bind_cap(
    reg: &mut impl CapabilityRegistry,
    shard: &ShardId,
    cap: &str,
    provider: &ProviderId,
    effect: EffectClass,
) {
    let f = fshard(shard);
    reg.register(&f, CapabilityId(cap.into())).expect("register");
    reg.bind(CapabilityBinding {
        capability: CapabilityId(cap.into()),
        shard: f,
        version: BindingVersion(1),
        provider: provider.clone(),
        contract: InvocationContract {
            input_schema: "acs:bytes".into(),
            output_schema: "acs:bytes".into(),
            effect,
        },
    })
    .expect("bind");
}

fn spec(shard: &ShardId, cap: &str, input: &[u8]) -> InvocationSpec {
    InvocationSpec {
        shard: shard.clone(),
        capability: CapabilityId(cap.into()),
        policy: PolicyVerdict::Allow,
        input: input.to_vec(),
    }
}

fn admitted(out: SubmitOutcome) -> IdempotencyKey {
    match out {
        SubmitOutcome::Admitted { key } => key,
        other => panic!("expected Admitted, got {other:?}"),
    }
}

fn env<'a, R: CapabilityRegistry>(
    cluster: &'a Rc<RefCell<ClusterSim>>,
    registry: &'a R,
    host: &'a EngineHost,
    down: &'a BTreeSet<NodeId>,
) -> DispatchEnv<'a, R> {
    DispatchEnv { cluster, registry, host, down }
}

/// Keys of every FRESH (non-deduped) `Committed` decision in a log — the
/// "landed as new truth" trace. Exactly-once landing means: across every
/// scheduler that raced an invocation, its key appears here exactly once
/// per proposed effect.
fn fresh_commit_keys(sched: &ClusterScheduler) -> Vec<IdempotencyKey> {
    sched
        .decisions()
        .iter()
        .filter_map(|d| match d {
            SchedulingDecision::Committed { key, deduped: false, .. } => Some(key.clone()),
            _ => None,
        })
        .collect()
}

fn count_decisions(sched: &ClusterScheduler, f: impl Fn(&SchedulingDecision) -> bool) -> usize {
    sched.decisions().iter().filter(|d| f(d)).count()
}

/// A well-behaved engine: pure function of its input, emitting `effects`
/// deterministic proposed effects; counts real invocations. Declared `Seeded`
/// so the RCR-012 probe does not double-invoke and the run counter is a true
/// execution count.
struct CountingEngine {
    name: String,
    effects: usize,
    runs: Rc<Cell<u64>>,
}

impl Engine for CountingEngine {
    type Input = Vec<u8>;
    fn manifest(&self) -> EngineManifest {
        EngineManifest {
            name: self.name.clone(),
            version: "1.0.0".into(),
            determinism: Determinism::Seeded,
            idempotency_key: IdempotencyKey("acs-002/1".into()),
            reads: Vec::new(),
            produces: vec!["uci.fact".into()],
            capabilities_required: Vec::new(),
        }
    }
    fn invoke(&self, input: Vec<u8>) -> Inference {
        self.runs.set(self.runs.get() + 1);
        let key = invocation_key(&self.manifest(), &input);
        let proposed_effects = (0..self.effects)
            .map(|i| ProposedEffect {
                target: "uci.fact".into(),
                payload: {
                    let mut p = input.clone();
                    p.push(i as u8);
                    p
                },
            })
            .collect();
        Inference { key, output: input, proposed_effects }
    }
}

fn counting(name: &str, effects: usize, runs: &Rc<Cell<u64>>) -> Box<CountingEngine> {
    Box::new(CountingEngine { name: name.into(), effects, runs: runs.clone() })
}

/// The poison: DECLARES `Deterministic` but varies per invocation — the exact
/// false promise the RCR-012 double-invoke probe refuses (F-POISON vehicle).
struct FalselyDeterministic {
    runs: Rc<Cell<u64>>,
}

impl Engine for FalselyDeterministic {
    type Input = Vec<u8>;
    fn manifest(&self) -> EngineManifest {
        EngineManifest {
            name: "liar".into(),
            version: "1.0.0".into(),
            determinism: Determinism::Deterministic,
            idempotency_key: IdempotencyKey("acs-002/1".into()),
            reads: Vec::new(),
            produces: vec!["uci.fact".into()],
            capabilities_required: Vec::new(),
        }
    }
    fn invoke(&self, input: Vec<u8>) -> Inference {
        let n = self.runs.get();
        self.runs.set(n + 1);
        let key = invocation_key(&self.manifest(), &input);
        let mut output = input;
        output.extend_from_slice(&n.to_be_bytes());
        Inference { key, output, proposed_effects: Vec::new() }
    }
}

// ---------------------------------------------------------------------------
// (a) Storm / duplicate / reorder schedules — ORCH-004 at the scheduling layer
// ---------------------------------------------------------------------------

/// A schedule STORM: 4 unique invocations submitted 11 times in two ORDERINGS
/// by two RACING independent schedulers (forward with interleaved duplicates
/// on one; reversed with a duplicate on the other), interleaved tick-by-tick.
/// No unique invocation is ever double-executed at the truth boundary: every
/// duplicate submission collapses VISIBLY at the ledger, every racing commit
/// resolves idempotently to existing truth, and across BOTH schedulers'
/// decision logs each unique key lands as FRESH truth exactly once. Compute is
/// honestly at-least-once across independent schedulers; TRUTH is at-most-once
/// (design §6.1; ORCH-004).
#[test]
fn storm_duplicate_and_reordered_schedules_never_double_execute_orch004() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);

    let inputs: [&[u8]; 4] = [b"u0", b"u1", b"u2", b"u3"];
    let mut s1 = ClusterScheduler::new(41, SchedulerConfig::default());
    let mut s2 = ClusterScheduler::new(42, SchedulerConfig::default());

    // s1: forward-order storm with immediate duplicates of u0 and u1.
    let keys: Vec<IdempotencyKey> =
        inputs.iter().map(|i| admitted(s1.submit(1, spec(&a, "cap.derive", i), &e))).collect();
    for i in &inputs[..2] {
        assert!(matches!(
            s1.submit(1, spec(&a, "cap.derive", i), &e),
            SubmitOutcome::Deduplicated { .. }
        ));
    }
    // s1 makes partial progress (u0, u1 land as fresh truth).
    s1.dispatch_tick(2, &e);
    s1.dispatch_tick(3, &e);

    // s2: the SAME storm REVERSED (reordered schedule), plus a duplicate.
    let keys_rev: Vec<IdempotencyKey> = inputs
        .iter()
        .rev()
        .map(|i| admitted(s2.submit(4, spec(&a, "cap.derive", i), &e)))
        .collect();
    assert_eq!(
        keys_rev.iter().rev().cloned().collect::<Vec<_>>(),
        keys,
        "both schedulers derive identical content-addressed keys (ORCH-004 identity)"
    );
    assert!(matches!(
        s2.submit(4, spec(&a, "cap.derive", b"u3"), &e),
        SubmitOutcome::Deduplicated { .. }
    ));
    // s2 races through its reversed queue: u3, u2 land fresh; u1, u0 dedupe.
    for tick in 5..9 {
        s2.dispatch_tick(tick, &e);
    }
    // s1 finishes its remaining queue: u2, u3 compute then dedupe at the Kernel.
    s1.dispatch_tick(9, &e);
    s1.dispatch_tick(10, &e);
    assert!(s1.is_idle() && s2.is_idle());

    // Post-completion duplicate storm: collapses forever, executes nothing.
    for i in &inputs {
        assert!(matches!(
            s1.submit(11, spec(&a, "cap.derive", i), &e),
            SubmitOutcome::Deduplicated { .. }
        ));
    }
    s1.dispatch_tick(12, &e);

    // Compute accounting (honest at-least-once): each scheduler executed each
    // unique invocation exactly once; duplicates NEVER re-invoked the engine.
    assert_eq!(runs.get(), 8, "4 unique × 2 independent schedulers; duplicates executed nothing");
    let computed_total = count_decisions(&s1, |d| matches!(d, SchedulingDecision::Computed { .. }))
        + count_decisions(&s2, |d| matches!(d, SchedulingDecision::Computed { .. }));
    assert_eq!(computed_total as u64, runs.get(), "every execution is a recorded decision");
    assert_eq!(
        count_decisions(&s1, |d| matches!(d, SchedulingDecision::Deduplicated { .. })),
        6,
        "s1: 2 in-flight + 4 post-done duplicates collapsed visibly"
    );
    assert_eq!(
        count_decisions(&s2, |d| matches!(d, SchedulingDecision::Deduplicated { .. })),
        1,
        "s2: its duplicate collapsed visibly"
    );

    // EXACTLY-ONCE LANDING: across both racing schedulers' logs, each unique
    // key appears as FRESH (deduped: false) committed truth exactly once.
    let mut fresh = fresh_commit_keys(&s1);
    fresh.extend(fresh_commit_keys(&s2));
    fresh.sort();
    let mut expected = keys.clone();
    expected.sort();
    assert_eq!(fresh, expected, "each unique invocation landed as new truth EXACTLY once");

    // Both ledgers agree every unique invocation is Done.
    for k in &keys {
        assert_eq!(s1.state_of(&a, k), Some(&WorkState::Done { commits: 1 }));
        assert_eq!(s2.state_of(&a, k), Some(&WorkState::Done { commits: 1 }));
    }

    // Truth: exactly 4 commits, byte-identical on every replica — no fork,
    // no duplicate, regardless of storm order.
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 4, "exactly the 4 unique truths at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference, "byte-identical at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// (b) Node death mid-invocation — re-placement, the key carries, no dup commit
// ---------------------------------------------------------------------------

/// The placed node (the shard leader) DIES between placement and quorum: the
/// in-flight invocation surfaces a retriable verdict and re-queues under the
/// SAME key (IDR-004); after the survivors elect a successor, the retry
/// replays FROM THE RECORD (engine never re-invoked — ORCH-003) and commits
/// through the new leader; queued work behind it is VISIBLY re-placed onto the
/// successor. Then the SCHEDULER dies too: a fresh scheduler re-derives the
/// IDENTICAL content-addressed key from the plan alone, recomputes (lawful
/// at-least-once), and every re-commit resolves idempotently — zero duplicate
/// commits (ORCH-004). Epilogue: the dead node rejoins and converges to the
/// same truth bytes.
#[test]
fn node_death_mid_invocation_replaces_and_never_duplicates_commit() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let l1 = cluster.borrow().leader_of(&a).expect("elected");

    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs_w1 = Rc::new(Cell::new(0));
    let pid_w1 = host.host(counting("cap.derive", 2, &runs_w1)); // TWO effects
    bind_cap(&mut reg, &a, "cap.derive", &pid_w1, EffectClass::ProposesWrite);
    let runs_w2 = Rc::new(Cell::new(0));
    let pid_w2 = host.host(counting("cap.more", 1, &runs_w2));
    bind_cap(&mut reg, &a, "cap.more", &pid_w2, EffectClass::ProposesWrite);

    let healthy = BTreeSet::new();
    let mut sched = ClusterScheduler::new(55, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &healthy);
    let w1 = admitted(sched.submit(1, spec(&a, "cap.derive", b"w1"), &e));
    let w2 = admitted(sched.submit(1, spec(&a, "cap.more", b"w2"), &e));

    // NODE DEATH mid-invocation: the leader is cut from the cluster between
    // the placement decision and quorum (AP presence has not noticed yet —
    // the stale-presence honesty of RCR-027 DR-10).
    cluster.borrow_mut().isolate(&a, &l1);
    sched.dispatch_tick(2, &e);
    assert_eq!(runs_w1.get(), 1, "w1 computed once on the dying node's behalf");
    assert!(
        sched.decisions().iter().any(|d| matches!(
            d,
            SchedulingDecision::Placed { key, node, .. } if key == &w1 && node == &l1
        )),
        "w1 was placed on the leader that then died"
    );
    assert!(
        sched.decisions().iter().any(|d| matches!(
            d,
            SchedulingDecision::CommitUnavailable { key, retries: 1, .. } if key == &w1
        )),
        "the death surfaced as a retriable verdict (IDR-004), never a silent loss"
    );
    assert_eq!(sched.queue_depth(&a), 2, "w1 re-queued at the front, w2 still behind it");
    assert!(matches!(sched.state_of(&a, &w1), Some(WorkState::Computed { .. })));

    // The survivors elect a successor; presence now reports the dead node.
    cluster.borrow_mut().settle(60);
    let l2 = cluster.borrow().leader_of(&a).expect("survivors re-elected");
    assert_ne!(l2, l1, "a DIFFERENT node leads after the death");
    let dead: BTreeSet<NodeId> = [l1.clone()].into_iter().collect();
    let e2 = env(&cluster, &reg, &host, &dead);

    // RE-PLACEMENT: w1 replays from its record through the successor; w2 gets
    // a fresh, VISIBLE placement on the successor.
    sched.dispatch_tick(3, &e2);
    assert_eq!(runs_w1.get(), 1, "ORCH-003: the retry NEVER re-invoked the engine");
    assert!(sched
        .decisions()
        .iter()
        .any(|d| matches!(d, SchedulingDecision::ReplayedFromRecord { key, .. } if key == &w1)));
    assert_eq!(sched.state_of(&a, &w1), Some(&WorkState::Done { commits: 2 }));
    sched.dispatch_tick(4, &e2);
    assert!(
        sched.decisions().iter().any(|d| matches!(
            d,
            SchedulingDecision::Placed { key, node, .. } if key == &w2 && node == &l2
        )),
        "queued work is re-placed onto the successor leader (visible re-placement)"
    );
    assert_eq!(sched.state_of(&a, &w2), Some(&WorkState::Done { commits: 1 }));
    assert!(sched.is_idle());

    // SCHEDULER DEATH on top: the key carries because it is derived from
    // CONTENT (manifest identity + canonical input), not from scheduler state.
    drop(sched);
    let mut s2 = ClusterScheduler::new(55, SchedulerConfig::default());
    let w1_again = admitted(s2.submit(10, spec(&a, "cap.derive", b"w1"), &e2));
    assert_eq!(w1_again, w1, "the idempotency key carries across scheduler death");
    let mut tick = 11;
    while !s2.is_idle() {
        s2.dispatch_tick(tick, &e2);
        tick += 1;
        assert!(tick < 24, "bounded deterministic run");
    }
    assert_eq!(runs_w1.get(), 2, "re-admission recomputes (lawful at-least-once compute)");
    assert_eq!(
        count_decisions(&s2, |d| matches!(d, SchedulingDecision::Committed { deduped: true, .. })),
        2,
        "BOTH re-proposed effects resolved idempotently to existing truth"
    );
    assert!(fresh_commit_keys(&s2).is_empty(), "zero duplicate commits after the re-run");
    assert_eq!(s2.state_of(&a, &w1), Some(&WorkState::Done { commits: 2 }));

    // Survivors hold exactly the 3 truths (w1's two effects + w2's one).
    cluster.borrow_mut().settle(5);
    {
        let c = cluster.borrow();
        for n in c.node_ids().iter().filter(|n| **n != l1) {
            assert_eq!(c.committed_count_of(n), 3, "exactly 3 truths at survivor {n:?}");
        }
    }

    // Epilogue: the dead node rejoins and LEARNS the same truth (IDR-002 —
    // outcomes replicate; the rejoined node never recomputes anything).
    cluster.borrow_mut().heal(&a);
    cluster.borrow_mut().settle(80);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 3, "rejoined cluster: 3 truths at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference, "byte-identical at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// (c) Backpressure honesty — explicit, accounted, retriable; never silent
// ---------------------------------------------------------------------------

/// Over-capacity refusals are EXPLICIT and fully ACCOUNTED: every one of the
/// 12 submit calls produces exactly one visible outcome decision (admission,
/// denial, dedup or submit-denial — here 6 admissions + 6 denials, nothing
/// else); a denial leaves NO ledger half-state (stateless, hence retriable —
/// unlike quarantine); re-submission after drain admits and completes; and the
/// final truth accounts for every unique invocation exactly once. A silent
/// drop is impossible by this accounting equation.
#[test]
fn overcapacity_refusals_are_explicit_accounted_and_retriable_never_silent() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(
        67,
        SchedulerConfig { shard_capacity: 2, retry_budget: 3, dispatch_per_tick: 1 },
    );
    let e = env(&cluster, &reg, &host, &down);

    let inputs: Vec<Vec<u8>> = (0..6u8).map(|i| vec![b'f', i]).collect();
    let mut pending: Vec<Vec<u8>> = inputs.clone();
    let mut submit_calls = 0usize;
    let mut denied_outcomes = 0usize;
    let mut admitted_outcomes = 0usize;
    let mut admitted_keys: Vec<IdempotencyKey> = Vec::new();
    let mut tick = 1u64;

    // First wave: capacity 2 admits 2, denies 4 — visibly.
    let mut next_pending = Vec::new();
    for i in pending.drain(..) {
        submit_calls += 1;
        match sched.submit(tick, spec(&a, "cap.derive", &i), &e) {
            SubmitOutcome::Admitted { key } => {
                admitted_outcomes += 1;
                admitted_keys.push(key);
            }
            SubmitOutcome::AdmissionDenied { key } => {
                denied_outcomes += 1;
                // A denial is STATELESS: no ledger entry, no queue slot, no
                // quarantine — the caller may simply retry later.
                assert_eq!(sched.state_of(&a, &key), None, "denial leaves no half-state");
                next_pending.push(i);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    pending = next_pending;
    assert_eq!((admitted_outcomes, denied_outcomes), (2, 4), "capacity 2: 2 in, 4 refused");

    // Drain-and-retry rounds: every refused invocation is retriable and
    // eventually admitted; nothing is lost, nothing needs special recovery.
    while !pending.is_empty() {
        tick += 1;
        while !sched.is_idle() {
            sched.dispatch_tick(tick, &e);
            tick += 1;
            assert!(tick < 64, "bounded deterministic run");
        }
        let mut next = Vec::new();
        for i in pending.drain(..) {
            submit_calls += 1;
            match sched.submit(tick, spec(&a, "cap.derive", &i), &e) {
                SubmitOutcome::Admitted { key } => {
                    admitted_outcomes += 1;
                    admitted_keys.push(key);
                }
                SubmitOutcome::AdmissionDenied { .. } => {
                    denied_outcomes += 1;
                    next.push(i);
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        pending = next;
    }
    while !sched.is_idle() {
        tick += 1;
        sched.dispatch_tick(tick, &e);
        assert!(tick < 64, "bounded deterministic run");
    }

    // THE ACCOUNTING EQUATION — every submit call has exactly one visible
    // outcome decision in the log; outcomes returned to the caller match the
    // log 1:1; no fifth, silent path exists.
    let logged_admitted =
        count_decisions(&sched, |d| matches!(d, SchedulingDecision::Admitted { .. }));
    let logged_denied =
        count_decisions(&sched, |d| matches!(d, SchedulingDecision::AdmissionDenied { .. }));
    let logged_dedup =
        count_decisions(&sched, |d| matches!(d, SchedulingDecision::Deduplicated { .. }));
    let logged_submit_denied =
        count_decisions(&sched, |d| matches!(d, SchedulingDecision::SubmitDenied { .. }));
    assert_eq!(logged_admitted, admitted_outcomes, "admissions logged 1:1");
    assert_eq!(logged_denied, denied_outcomes, "denials logged 1:1 — every refusal visible");
    assert_eq!(
        submit_calls,
        logged_admitted + logged_denied + logged_dedup + logged_submit_denied,
        "every submission is accounted; a silent drop is impossible"
    );
    assert_eq!(logged_admitted, 6, "all 6 unique invocations eventually admitted");

    // Every unique invocation completed exactly once (denials cost retries,
    // never work): 6 distinct keys, 6 executions, 6 truths, byte-identical
    // replicas.
    assert_eq!(runs.get(), 6);
    let distinct: BTreeSet<_> = admitted_keys.iter().cloned().collect();
    assert_eq!(distinct.len(), 6, "6 unique invocations, 6 distinct ORCH-004 keys");
    for key in &admitted_keys {
        assert_eq!(sched.state_of(&a, key), Some(&WorkState::Done { commits: 1 }));
    }
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 6, "all 6 truths at {n:?}, none dropped");
        assert_eq!(c.shard_state_of(n, &a), reference);
    }
}

// ---------------------------------------------------------------------------
// (d) Failure isolation — a poisoned capability cannot block the cluster
// ---------------------------------------------------------------------------

/// A POISON STORM: three distinct invocations of a falsely-`Deterministic`
/// capability interleaved ahead of healthy work in the same shard, plus
/// healthy work in another shard. Every poison quarantines terminally after
/// the RCR-012 probe refusal (2 probe invocations each, never retried); every
/// healthy invocation in BOTH shards completes within a bounded tick budget;
/// re-submitted poison collapses onto its quarantine WITHOUT re-invoking the
/// engine; and the cluster keeps accepting and completing NEW work afterwards
/// — one poisoned capability cannot block its shard, the other tenant, or the
/// cluster (F-POISON; SHARD-001).
#[test]
fn poison_capability_storm_cannot_block_shard_or_cluster() {
    let a = sid("acme", "prod");
    let b = sid("globex", "prod");
    let cluster = cluster(3, &[a.clone(), b.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();

    let liar_runs = Rc::new(Cell::new(0));
    let pid_liar = host.host(Box::new(FalselyDeterministic { runs: liar_runs.clone() }));
    bind_cap(&mut reg, &a, "cap.liar", &pid_liar, EffectClass::ProposesWrite);
    let healthy_runs = Rc::new(Cell::new(0));
    let pid_h = host.host(counting("cap.healthy", 1, &healthy_runs));
    bind_cap(&mut reg, &a, "cap.healthy", &pid_h, EffectClass::ProposesWrite);
    let b_runs = Rc::new(Cell::new(0));
    let pid_b = host.host(counting("cap.b", 1, &b_runs));
    bind_cap(&mut reg, &b, "cap.b", &pid_b, EffectClass::ProposesWrite);

    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(73, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);

    // The storm: poison INTERLEAVED ahead of healthy work in shard A.
    let mut poison_keys = Vec::new();
    let mut healthy_keys = Vec::new();
    for i in 0..3u8 {
        poison_keys.push(admitted(sched.submit(1, spec(&a, "cap.liar", &[b'p', i]), &e)));
        healthy_keys.push(admitted(sched.submit(1, spec(&a, "cap.healthy", &[b'h', i]), &e)));
    }
    let b_keys: Vec<_> = (0..2u8)
        .map(|i| admitted(sched.submit(1, spec(&b, "cap.b", &[b'b', i]), &e)))
        .collect();

    let mut tick = 2;
    while !sched.is_idle() {
        sched.dispatch_tick(tick, &e);
        tick += 1;
        assert!(tick < 20, "the poison storm never wedges the schedule (bounded ticks)");
    }

    // Poison: each unique poison ran exactly the probe's double-invoke, was
    // refused deterministically and quarantined terminally — never retried.
    assert_eq!(liar_runs.get(), 6, "3 unique poisons × the 2-invoke probe; zero retries");
    for k in &poison_keys {
        assert!(matches!(
            sched.state_of(&a, k),
            Some(WorkState::Quarantined { retriable: false, .. })
        ));
    }
    // Healthy work in BOTH shards is untouched by the storm.
    assert_eq!(healthy_runs.get(), 3);
    assert_eq!(b_runs.get(), 2);
    for k in &healthy_keys {
        assert_eq!(sched.state_of(&a, k), Some(&WorkState::Done { commits: 1 }));
    }
    for k in &b_keys {
        assert_eq!(sched.state_of(&b, k), Some(&WorkState::Done { commits: 1 }));
    }

    // Re-submitted poison collapses onto the terminal refusal — the engine is
    // NOT re-invoked (dedupe onto a deterministic refusal is lawful, RCR-027
    // DR-4; the caller can see the refusal via state_of).
    for i in 0..3u8 {
        assert!(matches!(
            sched.submit(tick, spec(&a, "cap.liar", &[b'p', i]), &e),
            SubmitOutcome::Deduplicated { .. }
        ));
    }
    sched.dispatch_tick(tick + 1, &e);
    assert_eq!(liar_runs.get(), 6, "collapsed duplicates never re-invoke the poison");

    // The cluster is NOT blocked: fresh work in both shards still completes.
    let fresh_a = admitted(sched.submit(tick + 2, spec(&a, "cap.healthy", b"after"), &e));
    let fresh_b = admitted(sched.submit(tick + 2, spec(&b, "cap.b", b"after"), &e));
    let mut t2 = tick + 3;
    while !sched.is_idle() {
        sched.dispatch_tick(t2, &e);
        t2 += 1;
        assert!(t2 < tick + 12, "bounded deterministic run");
    }
    assert_eq!(sched.state_of(&a, &fresh_a), Some(&WorkState::Done { commits: 1 }));
    assert_eq!(sched.state_of(&b, &fresh_b), Some(&WorkState::Done { commits: 1 }));

    // Truth: exactly the healthy commits (4 in A, 3 in B), byte-identical
    // everywhere; the poison contributed NOTHING to truth.
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference: Vec<_> =
        vec![(c.shard_state_of(&nodes[0], &a), c.shard_state_of(&nodes[0], &b))];
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 7, "only healthy truths exist at {n:?}");
        assert_eq!(
            vec![(c.shard_state_of(n, &a), c.shard_state_of(n, &b))],
            reference,
            "byte-identical at {n:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// (e) Leadership change mid-schedule — the invocation lands exactly once
// ---------------------------------------------------------------------------

/// A leadership CHANGE (not a death: the old leader survives, steps down and
/// REJOINS as a follower) happens in the middle of a schedule. An invocation
/// committed under the old leader is raced by a second scheduler under the
/// new leader: its re-commit resolves idempotently to the old-era truth
/// (exactly-once landing across the regime change); new work placed after the
/// change follows the new leader; and the rejoined old leader converges to
/// the identical truth bytes without recomputing anything.
#[test]
fn leadership_change_mid_schedule_lands_each_invocation_exactly_once() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let l1 = cluster.borrow().leader_of(&a).expect("elected");

    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);

    // Era 1: X lands under the old leader.
    let mut s1 = ClusterScheduler::new(81, SchedulerConfig::default());
    let x = admitted(s1.submit(1, spec(&a, "cap.derive", b"x"), &e));
    s1.dispatch_tick(2, &e);
    assert_eq!(s1.state_of(&a, &x), Some(&WorkState::Done { commits: 1 }));
    assert!(s1.decisions().iter().any(|d| matches!(
        d,
        SchedulingDecision::Placed { key, node, .. } if key == &x && node == &l1
    )));

    // LEADERSHIP CHANGE mid-schedule: isolate the old leader long enough for
    // the survivors to elect, then let it rejoin as a follower.
    cluster.borrow_mut().isolate(&a, &l1);
    cluster.borrow_mut().settle(60);
    let l2 = cluster.borrow().leader_of(&a).expect("successor elected");
    assert_ne!(l2, l1, "leadership changed hands");
    cluster.borrow_mut().heal(&a);
    cluster.borrow_mut().settle(80);
    assert_eq!(
        cluster.borrow().leader_of(&a),
        Some(l2.clone()),
        "the rejoined old leader stays a follower (higher-term successor rules)"
    );

    // Era 2: a RACING second scheduler re-runs X across the regime change —
    // it recomputes (honest at-least-once) but the commit through the NEW
    // leader resolves to the OLD-era truth: X lands exactly once.
    let mut s2 = ClusterScheduler::new(82, SchedulerConfig::default());
    let x2 = admitted(s2.submit(5, spec(&a, "cap.derive", b"x"), &e));
    assert_eq!(x2, x, "the content-addressed key is regime-independent");
    s2.dispatch_tick(6, &e);
    assert!(s2.is_idle());
    assert_eq!(runs.get(), 2, "at-least-once compute across schedulers (honest)");
    assert!(
        s2.decisions().iter().any(|d| matches!(
            d,
            SchedulingDecision::Committed { key, deduped: true, .. } if key == &x
        )),
        "the re-commit under the new leader resolved to the old-era truth"
    );
    assert!(fresh_commit_keys(&s2).is_empty(), "X never landed a second time");

    // Era 2 continued: NEW work follows the new leader and lands fresh.
    let y = admitted(s1.submit(7, spec(&a, "cap.derive", b"y"), &e));
    s1.dispatch_tick(8, &e);
    assert!(s1.decisions().iter().any(|d| matches!(
        d,
        SchedulingDecision::Placed { key, node, .. } if key == &y && node == &l2
    )));
    assert_eq!(s1.state_of(&a, &y), Some(&WorkState::Done { commits: 1 }));

    // Exactly-once landing at the decision level: across BOTH schedulers,
    // fresh commits are exactly {X (era 1), Y (era 2)} — one each.
    let mut fresh = fresh_commit_keys(&s1);
    fresh.extend(fresh_commit_keys(&s2));
    fresh.sort();
    let mut expected = vec![x.clone(), y.clone()];
    expected.sort();
    assert_eq!(fresh, expected, "each invocation landed exactly once across the regime change");

    // Every node — INCLUDING the rejoined old leader — holds the identical
    // two truths.
    cluster.borrow_mut().settle(10);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 2, "exactly X and Y at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference, "byte-identical at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// (f) Policy-flip re-submission — the Deny-quarantine collapse is PINNED
// ---------------------------------------------------------------------------

/// RCR-028 adversarial revision: the Governance [`PolicyVerdict`] is a CALLER
/// INPUT to the gate, NOT part of the ORCH-004 dedupe key (which is content:
/// shard + capability + manifest identity + canonical input). So an invocation
/// gate-denied under `Deny` quarantines deterministically (`retriable: false`),
/// and a LATER re-submission of the same (shard, capability, input) under
/// `Allow` collapses onto that recorded refusal for THIS scheduler's lifetime —
/// VISIBLY (`Deduplicated` outcome + decision; the refusal stays inspectable
/// via `state_of`), never silently, and without ever invoking the engine. No
/// invariant breaks: the schedule is a discardable plan artifact (ORCH-002),
/// so the policy flip takes effect through a FRESH scheduler, which re-admits
/// the same content-addressed key, executes once, and lands the truth on every
/// replica. Re-admit-on-differing-verdict inside one scheduler's lifetime is
/// deliberately NOT built — it is deferred to the IDR-007 instrument.
#[test]
fn policy_flip_resubmission_collapses_in_scheduler_lifetime_fresh_scheduler_readmits() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);

    // Era 1: submitted under Deny — admitted, then gate-denied at dispatch and
    // quarantined deterministically (terminal within this scheduler).
    let mut s1 = ClusterScheduler::new(91, SchedulerConfig::default());
    let mut denied = spec(&a, "cap.derive", b"flip");
    denied.policy = PolicyVerdict::Deny;
    let k = admitted(s1.submit(1, denied, &e));
    s1.dispatch_tick(2, &e);
    assert_eq!(runs.get(), 0, "the gate blocked BEFORE any engine invocation (F-POLICY)");
    assert!(
        s1.decisions()
            .iter()
            .any(|d| matches!(d, SchedulingDecision::GateDenied { key, .. } if key == &k)),
        "the denial is a recorded decision, never silent"
    );
    assert!(matches!(
        s1.state_of(&a, &k),
        Some(WorkState::Quarantined { retriable: false, .. })
    ));

    // POLICY FLIP within the SAME scheduler: the Allow re-submission of the
    // identical (shard, capability, input) collapses onto the stale refusal —
    // the verdict is not in the key, so this is a true duplicate to dedupe.
    match s1.submit(3, spec(&a, "cap.derive", b"flip"), &e) {
        SubmitOutcome::Deduplicated { key } => assert_eq!(key, k),
        other => panic!("expected Deduplicated onto the refusal, got {other:?}"),
    }
    s1.dispatch_tick(4, &e);
    assert_eq!(runs.get(), 0, "the collapsed Allow re-submission never invoked the engine");
    assert!(
        matches!(s1.state_of(&a, &k), Some(WorkState::Quarantined { retriable: false, .. })),
        "the refusal stays terminal AND visible for this scheduler's lifetime"
    );
    {
        let c = cluster.borrow();
        let nodes = c.node_ids();
        for n in &nodes {
            assert_eq!(c.committed_count_of(n), 0, "the refusal contributed zero truth at {n:?}");
        }
    }

    // The flip takes effect through a FRESH scheduler (ORCH-002: the schedule
    // is discardable): the same content-addressed key re-admits, executes
    // exactly once, and the truth lands byte-identically everywhere.
    drop(s1);
    let mut s2 = ClusterScheduler::new(92, SchedulerConfig::default());
    let k2 = admitted(s2.submit(10, spec(&a, "cap.derive", b"flip"), &e));
    assert_eq!(k2, k, "the ORCH-004 identity is verdict-independent across schedulers");
    s2.dispatch_tick(11, &e);
    assert_eq!(runs.get(), 1, "the fresh scheduler executed the now-allowed work exactly once");
    assert_eq!(s2.state_of(&a, &k), Some(&WorkState::Done { commits: 1 }));
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 1, "exactly the one flipped-to-Allow truth at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference, "byte-identical at {n:?}");
    }
}

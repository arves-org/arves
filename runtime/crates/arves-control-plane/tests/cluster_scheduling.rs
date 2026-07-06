//! RCR-027 (I4 Stage 2) — executable proofs for CLUSTER CAPABILITY SCHEDULING.
//!
//! Design obligations discharged here (docs/design/I4_Capability_Scheduling_Design.md §4):
//! placement under IDR-001 (leader affinity for commit-bearing work, deterministic
//! compute-anywhere for `Pure`), per-shard backpressure + tenant isolation (SHARD-001,
//! axis 8's isolation clause), failure isolation (F-POISON / F-NODE / F-LEADER),
//! idempotent dispatch (ORCH-004 — duplicate submission collapses; duplicate/racing
//! dispatch converges to at-most-one committed truth), replay-from-record retries
//! (ORCH-003 — a retried invocation NEVER re-invokes the engine within recorded state),
//! discardable scheduler state (ORCH-002 crash-rebuild), no scheduler-owned truth
//! (ORCH-001), and bit-identical transcripts for identically-scripted runs (the Stage-2
//! determinism rule).
//!
//! Every test is deterministic: fixed seeds, logical ticks, `Cell` counters, scripted
//! faults — zero wall clocks, zero OS randomness, zero sleeps. In-process simulation
//! over the I2 `ClusterSim` (no network — honest scope; the RCR-012 determinism probe
//! remains a probe).

use arves_capability_fabric::gate::PolicyVerdict;
use arves_capability_fabric::lifecycle::LifecycleRegistry;
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as FabricShardKey,
};
use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_control_plane::scheduler::{
    ClusterScheduler, DispatchEnv, EngineHost, InvocationSpec, PlacementBasis, SchedulerConfig,
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
// Deterministic scaffolding
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
        sim.add_shard(s.clone(), 7 + i as u64); // fixed seeds: recorded ⇒ replayable
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

fn decision_shard(d: &SchedulingDecision) -> &ShardId {
    match d {
        SchedulingDecision::SubmitDenied { shard, .. }
        | SchedulingDecision::Admitted { shard, .. }
        | SchedulingDecision::Deduplicated { shard, .. }
        | SchedulingDecision::AdmissionDenied { shard, .. }
        | SchedulingDecision::Placed { shard, .. }
        | SchedulingDecision::Deferred { shard, .. }
        | SchedulingDecision::GateDenied { shard, .. }
        | SchedulingDecision::Computed { shard, .. }
        | SchedulingDecision::ReplayedFromRecord { shard, .. }
        | SchedulingDecision::Committed { shard, .. }
        | SchedulingDecision::CommitUnavailable { shard, .. }
        | SchedulingDecision::Quarantined { shard, .. }
        | SchedulingDecision::Done { shard, .. } => shard,
    }
}

fn shard_transcript(sched: &ClusterScheduler, shard: &ShardId) -> Vec<String> {
    sched
        .decisions()
        .iter()
        .filter(|d| decision_shard(d) == shard)
        .map(|d| format!("{d:?}"))
        .collect()
}

/// A well-behaved engine: pure function of its input, emitting `effects`
/// deterministic proposed effects; counts real invocations. Declared `Seeded`
/// (a lawful, conservative promise for a reproducible engine) so the RCR-012
/// probe does not double-invoke and the run counter stays a true count of
/// executions.
struct CountingEngine {
    name: String,
    target: String,
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
            produces: vec![self.target.clone()],
            capabilities_required: Vec::new(),
        }
    }
    fn invoke(&self, input: Vec<u8>) -> Inference {
        self.runs.set(self.runs.get() + 1);
        let key = invocation_key(&self.manifest(), &input);
        let proposed_effects = (0..self.effects)
            .map(|i| ProposedEffect {
                target: self.target.clone(),
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
    Box::new(CountingEngine {
        name: name.into(),
        target: "uci.fact".into(),
        effects,
        runs: runs.clone(),
    })
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
// (d) The full distributed cognitive chain
// ---------------------------------------------------------------------------

/// Scheduled invocation → Stage-1 gate (authorize + RCR-012 `invoke_enforced`)
/// → proposed effects → ClusterKernel LEADER commit → quorum → byte-identical
/// truth on every replica. Commit-bearing work is placed with LEADER AFFINITY;
/// `Pure` work is placed compute-anywhere and commits NOTHING (IDR-001).
#[test]
fn full_distributed_chain_gate_engine_leader_commit_replicas_converge() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");

    let mut reg = LifecycleRegistry::new(); // the Stage-1 fabric core (RCR-026)
    let mut host = EngineHost::new();
    let write_runs = Rc::new(Cell::new(0));
    let pid_w = host.host(counting("cap.derive", 1, &write_runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid_w, EffectClass::ProposesWrite);
    let pure_runs = Rc::new(Cell::new(0));
    let pid_p = host.host(counting("cap.inspect", 0, &pure_runs));
    bind_cap(&mut reg, &a, "cap.inspect", &pid_p, EffectClass::Pure);

    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(11, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);

    let wkey = admitted(sched.submit(1, spec(&a, "cap.derive", b"fact-1"), &e));
    let pkey = admitted(sched.submit(1, spec(&a, "cap.inspect", b"look-1"), &e));
    sched.dispatch_tick(2, &e);
    sched.dispatch_tick(3, &e);
    assert!(sched.is_idle(), "both invocations dispatched");

    // Placement policy: commit-bearing → the shard leader; Pure → any healthy node.
    let placed: Vec<_> = sched
        .decisions()
        .iter()
        .filter_map(|d| match d {
            SchedulingDecision::Placed { key, node, basis, .. } => {
                Some((key.clone(), node.clone(), *basis))
            }
            _ => None,
        })
        .collect();
    assert_eq!(placed.len(), 2);
    assert!(
        placed.contains(&(wkey.clone(), leader.clone(), PlacementBasis::LeaderAffinity)),
        "commit-bearing invocation placed on the shard leader (leader affinity)"
    );
    assert!(
        placed
            .iter()
            .any(|(k, n, b)| k == &pkey
                && *b == PlacementBasis::ComputeAnywhere
                && cluster.borrow().node_ids().contains(n)),
        "pure invocation placed compute-anywhere on a cluster node"
    );

    // The chain completed: engine ran once each; effects committed exactly once.
    assert_eq!(write_runs.get(), 1);
    assert_eq!(pure_runs.get(), 1);
    assert_eq!(sched.state_of(&a, &wkey), Some(&WorkState::Done { commits: 1 }));
    assert_eq!(sched.state_of(&a, &pkey), Some(&WorkState::Done { commits: 0 }));
    assert!(sched.decisions().iter().any(|d| matches!(
        d,
        SchedulingDecision::Committed { key, deduped: false, .. } if key == &wkey
    )));

    // Truth replicated: every replica holds the SAME single committed truth.
    // (Followers learn the advanced commit index on subsequent group ticks —
    // drive the deterministic settle loop, then compare.)
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 1, "exactly one truth at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference, "byte-identical truth at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// (a) Placement is deterministic, leader-affine for commits, node-avoiding
// ---------------------------------------------------------------------------

/// Placement is a pure function of (recorded state, seed, tick): two
/// identically-scripted universes place identically; commit-bearing work
/// ALWAYS lands on the shard leader; `Pure` work never lands on a node the
/// scripted (AP) presence input marks down (F-NODE avoidance).
#[test]
fn placement_leader_affine_for_commits_deterministic_anywhere_for_pure() {
    let run = || {
        let a = sid("acme", "prod");
        let cluster = cluster(5, &[a.clone()]);
        let leader = cluster.borrow().leader_of(&a).expect("elected");
        let mut reg = MemRegistry::new();
        let mut host = EngineHost::new();
        let runs = Rc::new(Cell::new(0));
        let pid_w = host.host(counting("cap.derive", 1, &runs));
        bind_cap(&mut reg, &a, "cap.derive", &pid_w, EffectClass::ProposesWrite);
        let pid_p = host.host(counting("cap.inspect", 0, &runs));
        bind_cap(&mut reg, &a, "cap.inspect", &pid_p, EffectClass::Pure);

        // Scripted presence: two non-leader nodes are down (stale-able AP input).
        let down: BTreeSet<NodeId> = cluster
            .borrow()
            .node_ids()
            .into_iter()
            .filter(|n| n != &leader)
            .take(2)
            .collect();

        let mut sched = ClusterScheduler::new(
            99,
            SchedulerConfig { shard_capacity: 16, retry_budget: 3, dispatch_per_tick: 2 },
        );
        let e = env(&cluster, &reg, &host, &down);
        for i in 0..3u8 {
            admitted(sched.submit(1, spec(&a, "cap.derive", &[b'w', i]), &e));
            admitted(sched.submit(1, spec(&a, "cap.inspect", &[b'p', i]), &e));
        }
        let mut tick = 2;
        while !sched.is_idle() {
            sched.dispatch_tick(tick, &e);
            tick += 1;
            assert!(tick < 32, "bounded deterministic run");
        }
        let placements: Vec<_> = sched
            .decisions()
            .iter()
            .filter_map(|d| match d {
                SchedulingDecision::Placed { key, node, basis, .. } => {
                    Some((key.clone(), node.clone(), *basis))
                }
                _ => None,
            })
            .collect();
        (placements, leader, down)
    };

    let (p1, leader, down) = run();
    let (p2, _, _) = run();
    assert_eq!(p1, p2, "identically-scripted universes place identically (pure function)");
    assert_eq!(p1.len(), 6);
    for (_, node, basis) in &p1 {
        match basis {
            PlacementBasis::LeaderAffinity => {
                assert_eq!(node, &leader, "every commit-bearing placement is the shard leader")
            }
            PlacementBasis::ComputeAnywhere => {
                assert!(!down.contains(node), "pure compute never placed on a down node")
            }
        }
    }
    assert_eq!(
        p1.iter().filter(|(_, _, b)| *b == PlacementBasis::LeaderAffinity).count(),
        3,
        "three commit-bearing placements"
    );
}

// ---------------------------------------------------------------------------
// (b) Backpressure per shard + isolation (SHARD-001 / F-OVERLOAD)
// ---------------------------------------------------------------------------

/// Flooding shard A beyond its admission bound denies A's overflow VISIBLY and
/// leaves shard B's entire scheduling transcript BIT-IDENTICAL to a control run
/// without the flood — per-shard backpressure, tenant isolation held.
#[test]
fn backpressure_bounds_one_shard_and_the_other_tenants_transcript_is_untouched() {
    let a = sid("acme", "prod");
    let b = sid("globex", "prod");
    let config = SchedulerConfig { shard_capacity: 2, retry_budget: 3, dispatch_per_tick: 1 };

    let run = |flood: bool| {
        let cluster = cluster(3, &[a.clone(), b.clone()]);
        let mut reg = MemRegistry::new();
        let mut host = EngineHost::new();
        let runs = Rc::new(Cell::new(0));
        let pid_a = host.host(counting("cap.a", 1, &runs));
        bind_cap(&mut reg, &a, "cap.a", &pid_a, EffectClass::ProposesWrite);
        let pid_b = host.host(counting("cap.b", 1, &runs));
        bind_cap(&mut reg, &b, "cap.b", &pid_b, EffectClass::ProposesWrite);

        let down = BTreeSet::new();
        let mut sched = ClusterScheduler::new(5, config);
        let e = env(&cluster, &reg, &host, &down);
        let mut denied = 0;
        if flood {
            for i in 0..5u8 {
                match sched.submit(1, spec(&a, "cap.a", &[b'a', i]), &e) {
                    SubmitOutcome::Admitted { .. } => {}
                    SubmitOutcome::AdmissionDenied { .. } => denied += 1,
                    other => panic!("unexpected {other:?}"),
                }
            }
        }
        // Tenant B's submissions happen at the same ticks in both runs.
        admitted(sched.submit(1, spec(&b, "cap.b", b"b-0"), &e));
        admitted(sched.submit(1, spec(&b, "cap.b", b"b-1"), &e));
        for tick in 2..6 {
            sched.dispatch_tick(tick, &e);
        }
        assert!(sched.is_idle());
        let b_states: Vec<Vec<u8>> = cluster
            .borrow()
            .node_ids()
            .iter()
            .map(|n| cluster.borrow().shard_state_of(n, &b))
            .collect();
        (denied, shard_transcript(&sched, &b), b_states, sched.queue_depth(&a))
    };

    let (denied_ctl, b_ctl, b_states_ctl, _) = run(false);
    let (denied_flood, b_flood, b_states_flood, a_left) = run(true);
    assert_eq!(denied_ctl, 0);
    assert_eq!(denied_flood, 3, "capacity 2: the 3-invocation overflow is DENIED, not queued");
    assert_eq!(a_left, 0, "the two ADMITTED flood items still complete (bounded, not starved)");
    assert_eq!(
        b_flood, b_ctl,
        "tenant B's transcript is bit-identical with and without tenant A's flood (SHARD-001)"
    );
    assert_eq!(b_states_flood, b_states_ctl, "tenant B's committed truth is unaffected");
}

// ---------------------------------------------------------------------------
// (b) Failure isolation: poison + policy denial never starve the queue
// ---------------------------------------------------------------------------

/// A poison capability (false `Deterministic` declaration, refused by the
/// RCR-012 probe) and a policy-denied invocation are QUARANTINED — visibly,
/// terminally, without retry — while work behind them in the SAME shard and in
/// another shard completes untouched (F-POISON / F-POLICY containment).
#[test]
fn poison_and_policy_denials_quarantine_without_starving_shard_or_cluster() {
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
    let denied_runs = Rc::new(Cell::new(0));
    let pid_d = host.host(counting("cap.denied", 1, &denied_runs));
    bind_cap(&mut reg, &a, "cap.denied", &pid_d, EffectClass::ProposesWrite);
    let b_runs = Rc::new(Cell::new(0));
    let pid_b = host.host(counting("cap.b", 1, &b_runs));
    bind_cap(&mut reg, &b, "cap.b", &pid_b, EffectClass::ProposesWrite);

    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(23, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);

    // Shard A queue: poison at the HEAD, then a policy-denied one, then healthy work.
    let liar_key = admitted(sched.submit(1, spec(&a, "cap.liar", b"x"), &e));
    let denied_key = admitted(sched.submit(
        1,
        InvocationSpec {
            shard: a.clone(),
            capability: CapabilityId("cap.denied".into()),
            policy: PolicyVerdict::Deny, // Governance verdict: enforced, not owned
            input: b"y".to_vec(),
        },
        &e,
    ));
    let healthy_key = admitted(sched.submit(1, spec(&a, "cap.healthy", b"z"), &e));
    let b_key = admitted(sched.submit(1, spec(&b, "cap.b", b"w"), &e));

    for tick in 2..6 {
        sched.dispatch_tick(tick, &e);
    }
    assert!(sched.is_idle(), "the poison head never wedges the shard queue");

    // Poison: refused by the fabric probe (invoked exactly twice — the probe's
    // double-invoke, recorded honestly), quarantined, never retried.
    assert_eq!(liar_runs.get(), 2, "probe double-invoke only; no retry of poison");
    assert!(matches!(sched.state_of(&a, &liar_key), Some(WorkState::Quarantined { .. })));
    assert!(sched.decisions().iter().any(|d| matches!(
        d,
        SchedulingDecision::GateDenied { key, denial, .. }
            if key == &liar_key && denial.contains("NondeterministicOutput")
    )));

    // Policy denial: blocked BEFORE the engine ran (F-POLICY), quarantined.
    assert_eq!(denied_runs.get(), 0, "policy Deny blocks before any invocation");
    assert!(matches!(sched.state_of(&a, &denied_key), Some(WorkState::Quarantined { .. })));

    // The shard's healthy work and the other tenant completed normally.
    assert_eq!(healthy_runs.get(), 1);
    assert_eq!(b_runs.get(), 1);
    assert_eq!(sched.state_of(&a, &healthy_key), Some(&WorkState::Done { commits: 1 }));
    assert_eq!(sched.state_of(&b, &b_key), Some(&WorkState::Done { commits: 1 }));
    cluster.borrow_mut().settle(5); // followers apply the advanced commit index
    let c = cluster.borrow();
    for n in c.node_ids() {
        assert_eq!(c.committed_count_of(&n), 2, "exactly the two healthy truths, everywhere");
    }
}

// ---------------------------------------------------------------------------
// (c) Idempotent dispatch: duplicates collapse; racing schedulers converge
// ---------------------------------------------------------------------------

/// The ORCH-004 proof. (1) Within one scheduler, re-submitting the same
/// invocation collapses onto the recorded work — the engine is NEVER
/// re-invoked. (2) Across two independent, racing schedulers (active-active
/// duplicate scheduling), compute is at-least-once (honest) but committed
/// truth is EXACTLY once: the second commit resolves idempotently to the
/// existing truth (`AlreadyCommitted` → `deduped`), never a fork.
#[test]
fn duplicate_submission_and_racing_schedulers_never_fork_truth_orch004() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();

    // (1) duplicate submission collapses at the ledger.
    let mut s1 = ClusterScheduler::new(1, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);
    let key = admitted(s1.submit(1, spec(&a, "cap.derive", b"same-input"), &e));
    match s1.submit(1, spec(&a, "cap.derive", b"same-input"), &e) {
        SubmitOutcome::Deduplicated { key: k2 } => assert_eq!(k2, key, "same ORCH-004 key"),
        other => panic!("expected Deduplicated, got {other:?}"),
    }
    s1.dispatch_tick(2, &e);
    assert!(s1.is_idle());
    assert_eq!(runs.get(), 1, "one queue entry, one execution");
    assert_eq!(s1.state_of(&a, &key), Some(&WorkState::Done { commits: 1 }));

    // Re-submitting AFTER completion still collapses (terminal state kept).
    assert!(matches!(
        s1.submit(3, spec(&a, "cap.derive", b"same-input"), &e),
        SubmitOutcome::Deduplicated { .. }
    ));
    s1.dispatch_tick(4, &e);
    assert_eq!(runs.get(), 1, "no re-execution after Done");

    // (2) a second, independent scheduler races the same invocation.
    let mut s2 = ClusterScheduler::new(2, SchedulerConfig::default());
    let key2 = admitted(s2.submit(5, spec(&a, "cap.derive", b"same-input"), &e));
    assert_eq!(key2, key, "both schedulers derive the identical content-addressed key");
    s2.dispatch_tick(6, &e);
    assert!(s2.is_idle());
    assert_eq!(runs.get(), 2, "at-least-once COMPUTE across independent schedulers (honest)");
    assert!(
        s2.decisions().iter().any(|d| matches!(
            d,
            SchedulingDecision::Committed { key: k, deduped: true, .. } if k == &key
        )),
        "the racing commit resolved idempotently to the existing truth"
    );
    assert_eq!(s2.state_of(&a, &key), Some(&WorkState::Done { commits: 1 }));

    // Exactly ONE committed truth anywhere — at-most-once truth.
    cluster.borrow_mut().settle(5); // followers apply the advanced commit index
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 1, "no fork at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference);
    }
}

// ---------------------------------------------------------------------------
// (c) Leader loss mid-dispatch: requeue, replay-from-record, one truth
// ---------------------------------------------------------------------------

/// IDR-004 + ORCH-003/004: a commit-bearing invocation whose quorum is lost
/// mid-dispatch is re-queued under the SAME key and re-dispatched from its
/// RECORDED inference — the engine is NOT re-invoked by the retry — and after
/// the partition heals, exactly one committed truth exists on every replica.
#[test]
fn leader_loss_mid_dispatch_replays_from_record_and_commits_exactly_once() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let others: Vec<NodeId> = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .filter(|n| n != &leader)
        .collect();

    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();

    let mut sched = ClusterScheduler::new(31, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);
    let key = admitted(sched.submit(1, spec(&a, "cap.derive", b"fact"), &e));

    // Cut the leader away from its followers BEFORE dispatch: compute succeeds,
    // the commit cannot reach quorum (CP posture: unavailable > divergent).
    cluster.borrow_mut().partition(&a, &[vec![leader.clone()], others.clone()]);
    sched.dispatch_tick(2, &e);
    assert_eq!(runs.get(), 1, "the engine computed once");
    assert!(
        sched.decisions().iter().any(|d| matches!(
            d,
            SchedulingDecision::CommitUnavailable { key: k, retries: 1, .. } if k == &key
        )),
        "quorum loss surfaced as a retriable verdict, work re-queued (IDR-004)"
    );
    assert_eq!(sched.queue_depth(&a), 1, "in-flight work discarded and re-queued, not lost");
    assert!(matches!(sched.state_of(&a, &key), Some(WorkState::Computed { .. })));

    // Heal, re-elect, settle — then the retry replays FROM THE RECORD.
    cluster.borrow_mut().heal(&a);
    cluster.borrow_mut().elect(&a);
    cluster.borrow_mut().settle(5);
    sched.dispatch_tick(3, &e);
    assert!(sched.is_idle());
    assert_eq!(runs.get(), 1, "ORCH-003: the retry NEVER re-invoked the engine");
    assert!(sched
        .decisions()
        .iter()
        .any(|d| matches!(d, SchedulingDecision::ReplayedFromRecord { key: k, .. } if k == &key)));
    assert_eq!(sched.state_of(&a, &key), Some(&WorkState::Done { commits: 1 }));

    // Exactly one truth everywhere (fresh commit or idempotent convergence —
    // both lawful; the count is the invariant).
    cluster.borrow_mut().settle(3);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], &a);
    for n in &nodes {
        assert_eq!(c.committed_count_of(n), 1, "exactly one committed truth at {n:?}");
        assert_eq!(c.shard_state_of(n, &a), reference, "byte-identical at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// ORCH-002 / ORCH-001: the scheduler is discardable and owns no truth
// ---------------------------------------------------------------------------

/// Kill the scheduler mid-run (drop ALL queues/ledger/decisions), rebuild a
/// fresh one from the same PLAN, and finish: the committed truth set is
/// IDENTICAL to an uninterrupted reference run — nothing scheduler-local was
/// authoritative (ORCH-002), and the only truth that exists anywhere carries
/// Kernel commit provenance through the shard leader (ORCH-001: the scheduler
/// has no other write path; dropping it loses zero truth).
#[test]
fn scheduler_crash_rebuild_from_plan_converges_to_identical_truth_orch001_orch002() {
    let a = sid("acme", "prod");
    let b = sid("globex", "prod");
    let plan = |shard: &ShardId, cap: &str, i: u8| spec(shard, cap, &[b'p', i]);

    let build = || {
        let cluster = cluster(3, &[a.clone(), b.clone()]);
        let mut reg = MemRegistry::new();
        let mut host = EngineHost::new();
        let runs = Rc::new(Cell::new(0));
        let pid_a = host.host(counting("cap.a", 1, &runs));
        bind_cap(&mut reg, &a, "cap.a", &pid_a, EffectClass::ProposesWrite);
        let pid_b = host.host(counting("cap.b", 2, &runs));
        bind_cap(&mut reg, &b, "cap.b", &pid_b, EffectClass::ProposesWrite);
        (cluster, reg, host, runs)
    };
    let submit_plan = |sched: &mut ClusterScheduler,
                       e: &DispatchEnv<'_, MemRegistry>,
                       tick: u64| {
        for i in 0..2u8 {
            let _ = sched.submit(tick, plan(&a, "cap.a", i), e);
            let _ = sched.submit(tick, plan(&b, "cap.b", i), e);
        }
    };
    let truths = |cluster: &Rc<RefCell<ClusterSim>>| {
        cluster.borrow_mut().settle(5); // deterministic: followers apply commits
        let c = cluster.borrow();
        c.node_ids()
            .iter()
            .map(|n| (c.shard_state_of(n, &a), c.shard_state_of(n, &b), c.committed_count_of(n)))
            .collect::<Vec<_>>()
    };

    // Reference: uninterrupted run.
    let (cluster_ref, reg_ref, host_ref, _) = build();
    let down = BTreeSet::new();
    let mut sref = ClusterScheduler::new(7, SchedulerConfig::default());
    let eref = env(&cluster_ref, &reg_ref, &host_ref, &down);
    submit_plan(&mut sref, &eref, 1);
    let mut tick = 2;
    while !sref.is_idle() {
        sref.dispatch_tick(tick, &eref);
        tick += 1;
        assert!(tick < 32);
    }
    let reference = truths(&cluster_ref);

    // Crash run: identical universe; the first scheduler dies mid-run.
    let (cluster2, reg2, host2, _) = build();
    let e2 = env(&cluster2, &reg2, &host2, &down);
    let mut s1 = ClusterScheduler::new(7, SchedulerConfig::default());
    submit_plan(&mut s1, &e2, 1);
    s1.dispatch_tick(2, &e2); // partial progress only
    let before_drop = truths(&cluster2);
    drop(s1); // ORCH-002: every queue, ledger entry and decision is GONE.
    assert_eq!(truths(&cluster2), before_drop, "dropping the scheduler loses zero truth");

    // Recovery = re-submit the plan to a FRESH scheduler (state reconstructed
    // from plan + Kernel dedupe, design §3.10).
    let mut s2 = ClusterScheduler::new(7, SchedulerConfig::default());
    submit_plan(&mut s2, &e2, 10);
    let mut tick = 11;
    while !s2.is_idle() {
        s2.dispatch_tick(tick, &e2);
        tick += 1;
        assert!(tick < 48);
    }
    assert_eq!(
        truths(&cluster2),
        reference,
        "crash + rebuild-from-plan converges to the identical committed truth set"
    );
    // The already-committed slice resolved idempotently (deduped), never forked.
    assert!(
        s2.decisions()
            .iter()
            .any(|d| matches!(d, SchedulingDecision::Committed { deduped: true, .. })),
        "recovery re-commits resolve to existing truth (ORCH-004), no fork"
    );
}

// ---------------------------------------------------------------------------
// DR-13: the dedupe identity is shard-partitioned and capability-qualified
// ---------------------------------------------------------------------------

/// SHARD-001 negative proof for the dedupe identity (RCR-027 DR-13): the SAME
/// capability, provider and input submitted in TWO shards is TWO independent
/// pieces of work — the second tenant is never silently "satisfied" by the
/// first tenant's record (cross-tenant work suppression), and BOTH shards end
/// with their OWN committed truth (Kernel dedupe is shard-scoped).
#[test]
fn same_capability_provider_and_input_in_two_shards_both_commit_their_own_truth() {
    let a = sid("acme", "prod");
    let b = sid("globex", "prod");
    let cluster = cluster(3, &[a.clone(), b.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    // ONE provider, bound in BOTH shards.
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    bind_cap(&mut reg, &b, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(3, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);

    let key_a = admitted(sched.submit(1, spec(&a, "cap.derive", b"same-input"), &e));
    // Tenant B's IDENTICAL submission must be ADMITTED as independent work,
    // never collapsed onto tenant A's record.
    let key_b = admitted(sched.submit(1, spec(&b, "cap.derive", b"same-input"), &e));

    let mut tick = 2;
    while !sched.is_idle() {
        sched.dispatch_tick(tick, &e);
        tick += 1;
        assert!(tick < 16, "bounded deterministic run");
    }
    assert_eq!(runs.get(), 2, "two shards, two independent computes");
    assert_eq!(sched.state_of(&a, &key_a), Some(&WorkState::Done { commits: 1 }));
    assert_eq!(sched.state_of(&b, &key_b), Some(&WorkState::Done { commits: 1 }));
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    for n in c.node_ids() {
        assert_eq!(c.committed_count_of(&n), 2, "each shard holds its OWN truth at {n:?}");
        assert!(!c.shard_state_of(&n, &a).is_empty(), "tenant A's truth exists");
        assert!(!c.shard_state_of(&n, &b).is_empty(), "tenant B's truth exists");
    }
}

/// Capability-qualification negative proof (RCR-027 DR-13): two DIFFERENT
/// capabilities in ONE shard bound to the SAME provider with the SAME input
/// are two independent work items — the second (policy-DENIED) submission is
/// never reported satisfied-by-dedupe; its Governance verdict IS evaluated at
/// the gate and it quarantines, while the allowed capability commits.
#[test]
fn two_capabilities_sharing_a_provider_never_collapse_and_policy_still_bites() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.shared", 1, &runs));
    // ONE provider, TWO capabilities in the SAME shard.
    bind_cap(&mut reg, &a, "cap.one", &pid, EffectClass::ProposesWrite);
    bind_cap(&mut reg, &a, "cap.two", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(17, SchedulerConfig::default());
    let e = env(&cluster, &reg, &host, &down);

    let key_one = admitted(sched.submit(1, spec(&a, "cap.one", b"in"), &e));
    let key_two = admitted(sched.submit(
        1,
        InvocationSpec {
            shard: a.clone(),
            capability: CapabilityId("cap.two".into()),
            policy: PolicyVerdict::Deny, // must be EVALUATED, never skipped
            input: b"in".to_vec(),
        },
        &e,
    ));
    assert_ne!(key_one, key_two, "the capability id is part of the scheduling identity (DR-13)");

    for tick in 2..6 {
        sched.dispatch_tick(tick, &e);
    }
    assert!(sched.is_idle());
    assert_eq!(runs.get(), 1, "the DENIED capability was never invoked");
    assert_eq!(sched.state_of(&a, &key_one), Some(&WorkState::Done { commits: 1 }));
    assert!(matches!(
        sched.state_of(&a, &key_two),
        Some(WorkState::Quarantined { retriable: false, .. })
    ));
    assert!(sched.decisions().iter().any(|d| matches!(
        d,
        SchedulingDecision::GateDenied { key, .. } if key == &key_two
    )));
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    for n in c.node_ids() {
        assert_eq!(c.committed_count_of(&n), 1, "only the ALLOWED capability committed");
    }
}

// ---------------------------------------------------------------------------
// DR-4: budget-exhausted quarantine is retriable-class — re-admission works
// ---------------------------------------------------------------------------

/// DR-4 (revised): a RETRIABLE-class quarantine (commit retry budget
/// exhausted during prolonged quorum loss) is not a permanent refusal —
/// after the world heals, RE-SUBMITTING the same invocation re-admits it with
/// a fresh budget (it is NOT Deduplicated onto the refusal) and it completes
/// with exactly one committed truth everywhere. Honest note: the recorded
/// inference was discarded at quarantine, so re-admission RECOMPUTES —
/// lawful at-least-once compute, converging at the Kernel because the engine
/// actually reproduces its effects (declared-determinism honesty, v1.1 debt
/// #2).
#[test]
fn budget_exhausted_quarantine_readmits_after_heal_with_fresh_budget() {
    let a = sid("acme", "prod");
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let others: Vec<NodeId> = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .filter(|n| n != &leader)
        .collect();
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(counting("cap.derive", 1, &runs));
    bind_cap(&mut reg, &a, "cap.derive", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let mut sched = ClusterScheduler::new(
        13,
        SchedulerConfig { shard_capacity: 8, retry_budget: 1, dispatch_per_tick: 1 },
    );
    let e = env(&cluster, &reg, &host, &down);
    let key = admitted(sched.submit(1, spec(&a, "cap.derive", b"fact"), &e));

    // Prolonged TOTAL quorum loss (every node isolated — no side can elect a
    // new leader, unlike a leader-vs-majority split where the majority
    // lawfully recovers): the budget (1) exhausts and the work quarantines.
    cluster.borrow_mut().partition(
        &a,
        &[vec![leader.clone()], vec![others[0].clone()], vec![others[1].clone()]],
    );
    sched.dispatch_tick(2, &e); // compute + CommitUnavailable (retries: 1)
    sched.dispatch_tick(3, &e); // replay-from-record + budget exhausted → quarantine
    assert_eq!(sched.queue_depth(&a), 0, "quarantined work leaves the queue (never wedges)");
    assert!(matches!(
        sched.state_of(&a, &key),
        Some(WorkState::Quarantined { retriable: true, .. })
    ));
    assert_eq!(runs.get(), 1, "the engine computed once before quarantine");

    // Heal the world (settle FIRST so the stale pre-partition leader steps
    // down, then elect a real quorum leader), then RE-SUBMIT the same
    // invocation: it RE-ADMITS with a fresh budget instead of collapsing
    // onto the refusal.
    cluster.borrow_mut().heal(&a);
    cluster.borrow_mut().settle(5);
    cluster.borrow_mut().elect(&a);
    match sched.submit(10, spec(&a, "cap.derive", b"fact"), &e) {
        SubmitOutcome::Admitted { key: k } => assert_eq!(k, key, "same identity, fresh budget"),
        other => panic!("expected re-admission after retriable quarantine, got {other:?}"),
    }
    let mut tick = 11;
    while !sched.is_idle() {
        sched.dispatch_tick(tick, &e);
        cluster.borrow_mut().settle(1); // drive the healed group deterministically
        tick += 1;
        assert!(tick < 24, "bounded deterministic run");
    }
    assert_eq!(runs.get(), 2, "re-admission lawfully RECOMPUTES (the record was discarded)");
    assert_eq!(sched.state_of(&a, &key), Some(&WorkState::Done { commits: 1 }));
    cluster.borrow_mut().settle(3);
    let c = cluster.borrow();
    for n in c.node_ids() {
        assert_eq!(c.committed_count_of(&n), 1, "exactly one committed truth at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// (b) Bit-identical transcripts: the scheduler is a deterministic function
// ---------------------------------------------------------------------------

/// Two identically-scripted runs (same seeds, same ticks, same submissions,
/// same scripted node-presence, same flood) produce BIT-IDENTICAL decision
/// transcripts and byte-identical committed truth — scheduling is a pure
/// function of (recorded state, seed, tick, recorded observations).
#[test]
fn identically_scripted_runs_produce_bit_identical_transcripts_and_truth() {
    let run = || {
        let a = sid("acme", "prod");
        let b = sid("globex", "prod");
        let cluster = cluster(4, &[a.clone(), b.clone()]);
        let mut reg = MemRegistry::new();
        let mut host = EngineHost::new();
        let runs = Rc::new(Cell::new(0));
        let pid_a = host.host(counting("cap.a", 1, &runs));
        bind_cap(&mut reg, &a, "cap.a", &pid_a, EffectClass::ProposesWrite);
        let pid_p = host.host(counting("cap.pure", 0, &runs));
        bind_cap(&mut reg, &a, "cap.pure", &pid_p, EffectClass::Pure);
        let pid_b = host.host(counting("cap.b", 1, &runs));
        bind_cap(&mut reg, &b, "cap.b", &pid_b, EffectClass::ProposesWrite);
        let liar_runs = Rc::new(Cell::new(0));
        let pid_l = host.host(Box::new(FalselyDeterministic { runs: liar_runs }));
        bind_cap(&mut reg, &b, "cap.liar", &pid_l, EffectClass::ProposesWrite);

        // Scripted presence: one NON-LEADER node down for the whole run
        // (chosen deterministically; a down leader would lawfully defer all
        // commit-bearing work — a different scenario than this script).
        let down: BTreeSet<NodeId> = {
            let c = cluster.borrow();
            let la = c.leader_of(&a).expect("elected");
            let lb = c.leader_of(&b).expect("elected");
            c.node_ids()
                .into_iter()
                .filter(|n| n != &la && n != &lb)
                .take(1)
                .collect()
        };
        let mut sched = ClusterScheduler::new(
            2026,
            SchedulerConfig { shard_capacity: 3, retry_budget: 2, dispatch_per_tick: 1 },
        );
        let e = env(&cluster, &reg, &host, &down);
        // Script: flood A past capacity; mix pure work; poison in B; policy deny.
        for i in 0..5u8 {
            let _ = sched.submit(1, spec(&a, "cap.a", &[b'a', i]), &e);
        }
        let _ = sched.submit(1, spec(&a, "cap.pure", b"look"), &e);
        let _ = sched.submit(1, spec(&b, "cap.liar", b"poison"), &e);
        let _ = sched.submit(
            1,
            InvocationSpec {
                shard: b.clone(),
                capability: CapabilityId("cap.b".into()),
                policy: PolicyVerdict::ApprovalRequired, // blocks (no HITL surface)
                input: b"needs-approval".to_vec(),
            },
            &e,
        );
        let _ = sched.submit(1, spec(&b, "cap.b", b"fine"), &e);
        let mut tick = 2;
        while !sched.is_idle() {
            sched.dispatch_tick(tick, &e);
            tick += 1;
            assert!(tick < 40, "bounded run");
        }
        cluster.borrow_mut().settle(5); // followers apply the advanced commit index
        let c = cluster.borrow();
        let states: Vec<_> = c
            .node_ids()
            .iter()
            .map(|n| (c.shard_state_of(n, &a), c.shard_state_of(n, &b)))
            .collect();
        (sched.transcript(), states)
    };

    let (t1, s1) = run();
    let (t2, s2) = run();
    assert!(!t1.is_empty());
    assert_eq!(t1, t2, "bit-identical decision transcripts");
    assert_eq!(s1, s2, "byte-identical committed truth");
    // The script provably exercised the interesting paths (a no-op run can't pass).
    let joined = t1.join("\n");
    assert!(joined.contains("AdmissionDenied"), "backpressure bit");
    assert!(joined.contains("GateDenied"), "poison bit");
    assert!(joined.contains("PolicyBlocked"), "policy-approval block bit");
    assert!(joined.contains("deduped: false"), "fresh commits happened");
}

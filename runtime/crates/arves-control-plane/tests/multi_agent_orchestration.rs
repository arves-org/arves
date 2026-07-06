//! RCR-030 (I5 Stage 2) — executable proofs for MULTI-AGENT ORCHESTRATION
//! over ONE shared truth base (design §3.1.2/§3.1.3/§3.8, the G1
//! `enterprise-os.mjs` reference semantics at runtime level):
//!
//! (a) concurrent agent proposals through the I4 scheduler into the cluster —
//!     agents never commit (ORCH-001); the schedule stays a discardable plan
//!     artifact (ORCH-002);
//! (b) shared-truth concurrency — duplicates and agreeing decisions converge
//!     to ONE truth (ORCH-004 across agents); CONFLICTING decisions on one
//!     subject resolve deterministically FIRST-COMMITTED-WINS in shard log
//!     order, the loser receiving the winner's identity + a committed
//!     compliance event (never a silent overwrite);
//! (c) cross-agent consistency reads — an agent's next step sees prior
//!     committed truth per the I3 ladder, with LABELED guarantees per tier;
//! (d) decision/compliance truth flows — policy checks read COMMITTED policy
//!     truths; approvals are SEPARATE committed truths (proposer ≠ approver);
//!     refusals are committed compliance truths.
//!
//! HONEST LANGUAGE: every "agent" here is a DETERMINISTIC TEST ACTOR (a
//! registered identity driven by scripted calls), NOT an AI model. Identity is
//! structural, not cryptographic (v2.0 debt #8). Agent interleavings are
//! SCRIPTED, SEEDED SCHEDULES — the permutation proof runs every order of the
//! contended proposals explicitly.
//!
//! Every test is deterministic: fixed seeds, logical ticks, scripted
//! elections/settles/isolation — zero wall clocks, zero OS randomness, zero
//! sleeps. In-process simulation over the I2 `ClusterSim` (no network).

use arves_capability_fabric::gate::PolicyVerdict;
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as FabricShardKey,
};
use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_control_plane::agents::{
    attributed_effects, register_agent, AgentDefinition, AgentId,
};
use arves_control_plane::multi_agent::{
    approvals_on, commit_approval, commit_policy, compliance_on, decision_of, decisions_on,
    propose_decision, submit_attributed_effect, ComplianceOutcome, FlowError, PolicyRecord,
    ProposalOutcome,
};
use arves_control_plane::scheduler::{
    ClusterScheduler, DispatchEnv, EngineHost, SchedulerConfig, SchedulingDecision, SubmitOutcome,
};
use arves_engine_fabric::{
    invocation_key, Determinism, Engine, EngineManifest, IdempotencyKey, Inference, ProposedEffect,
};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::ShardKey as KernelShardKey;
use arves_lcw::world::WorldView;
use arves_lcw::ShardKey as LcwShardKey;
use arves_query::distributed::{floor_of, ClusterQuery, LAG_UNATTESTABLE};
use arves_query::{Query, QueryError, ReadScope, ReadTier, StalenessBound};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Deterministic scaffolding (the RCR-027/028/029 harness discipline)
// ---------------------------------------------------------------------------

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}

fn lshard(t: &str, w: &str) -> LcwShardKey {
    LcwShardKey { tenant: t.into(), workspace: w.into() }
}

fn kshard(t: &str, w: &str) -> KernelShardKey {
    KernelShardKey::new(t, w).expect("well-formed test shard")
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

fn env<'a, R: CapabilityRegistry>(
    cluster: &'a Rc<RefCell<ClusterSim>>,
    registry: &'a R,
    host: &'a EngineHost,
    down: &'a BTreeSet<NodeId>,
) -> DispatchEnv<'a, R> {
    DispatchEnv { cluster, registry, host, down }
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

/// The deterministic AGENT ACTOR engine: a pure echo — its single proposed
/// effect is the invocation input VERBATIM (the RCR-029 attribution envelope
/// passes through untouched, so the committed truth carries the Who inside
/// the addressed content). Declared `Seeded` so the RCR-012 probe does not
/// double-invoke and `runs` counts true executions.
struct EchoActor {
    runs: Rc<Cell<u64>>,
}

impl Engine for EchoActor {
    type Input = Vec<u8>;
    fn manifest(&self) -> EngineManifest {
        EngineManifest {
            name: "agent-actor".into(),
            version: "1.0.0".into(),
            determinism: Determinism::Seeded,
            idempotency_key: IdempotencyKey("acs-002/1".into()),
            reads: Vec::new(),
            produces: vec!["agent.effect".into()],
            capabilities_required: Vec::new(),
        }
    }
    fn invoke(&self, input: Vec<u8>) -> Inference {
        self.runs.set(self.runs.get() + 1);
        let key = invocation_key(&self.manifest(), &input);
        let proposed_effects =
            vec![ProposedEffect { target: "agent.effect".into(), payload: input.clone() }];
        Inference { key, output: input, proposed_effects }
    }
}

fn leader_kernel(cluster: &Rc<RefCell<ClusterSim>>, leader: &NodeId) -> ClusterKernel {
    ClusterKernel::new(leader.clone(), Rc::clone(cluster))
}

fn world_head(
    cluster: &Rc<RefCell<ClusterSim>>,
    node: &NodeId,
    shard: &LcwShardKey,
) -> WorldView {
    let store = cluster.borrow().wal_store_of(node);
    WorldView::at_head(&store, shard).expect("world at head")
}

/// Register a deterministic test-actor identity as committed truth through
/// the shard leader's frozen gateway (RCR-029 Stage 1).
fn register(
    cluster: &Rc<RefCell<ClusterSim>>,
    leader: &NodeId,
    shard: &KernelShardKey,
    name: &str,
) -> AgentId {
    let kernel = leader_kernel(cluster, leader);
    let def = AgentDefinition {
        name: name.into(),
        agent_type: "Worker".into(),
        owner: "ops@acme".into(),
        purpose: "deterministic test actor (NOT an AI model)".into(),
        definition_version: 1,
    };
    register_agent(&kernel, shard, &def).expect("registration commits").id
}

/// Propose a decision through the flow, committing via the shard leader and
/// reconciling against the leader's at-head store.
fn propose(
    cluster: &Rc<RefCell<ClusterSim>>,
    leader: &NodeId,
    basis: &WorldView,
    agent: &AgentId,
    subject: &str,
    action: &str,
) -> ProposalOutcome {
    let kernel = leader_kernel(cluster, leader);
    let store = cluster.borrow().wal_store_of(leader);
    propose_decision(&kernel, &store, basis, agent, subject, action).expect("flow completes")
}

fn assert_replicas_identical(cluster: &Rc<RefCell<ClusterSim>>, shard: &ShardId) {
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], shard);
    for n in &nodes {
        assert_eq!(c.shard_state_of(n, shard), reference, "byte-identical truth at {n:?}");
    }
}

// ---------------------------------------------------------------------------
// (a) Agent proposals flow through the I4 scheduler; agents never commit
// ---------------------------------------------------------------------------

/// N agents' proposals enter the cluster ONLY through the scheduler's dispatch
/// pipeline (gate → echo actor → proposed effect → shard-leader Kernel commit).
/// ORCH-001: refusals commit nothing and only Kernel commits create state.
/// ORCH-002: dropping the scheduler moves zero truth; a fresh scheduler
/// re-submitting the same plan converges by Kernel dedupe (zero fresh commits).
#[test]
fn agent_proposals_flow_through_the_scheduler_and_only_the_kernel_commits() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");

    let w1 = register(&cluster, &leader, &kw, "worker-1");
    let w2 = register(&cluster, &leader, &kw, "worker-2");
    let world = world_head(&cluster, &leader, &lw);

    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(Box::new(EchoActor { runs: runs.clone() }));
    bind_cap(&mut reg, &a, "cap.propose", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);

    // Scripted interleaving: w1, w2, w1, w2 — four distinct effects.
    let mut sched = ClusterScheduler::new(30, SchedulerConfig::default());
    let plan: Vec<(&AgentId, &[u8])> = vec![
        (&w1, b"draft:q3-plan"),
        (&w2, b"draft:q3-budget"),
        (&w1, b"note:supplier-x"),
        (&w2, b"note:supplier-y"),
    ];
    for (i, (agent, effect)) in plan.iter().enumerate() {
        let out = submit_attributed_effect(
            &mut sched,
            i as u64 + 1,
            &world,
            agent,
            CapabilityId("cap.propose".into()),
            PolicyVerdict::Allow,
            effect,
            &e,
        )
        .expect("registered agent admitted");
        assert!(matches!(out, SubmitOutcome::Admitted { .. }));
    }
    for t in 10..20 {
        sched.dispatch_tick(t, &e);
    }
    assert!(sched.is_idle());
    assert_eq!(runs.get(), 4, "each distinct proposal computed once");

    // ORCH-001 refusal path: an UNREGISTERED identity is refused BEFORE the
    // queue — nothing scheduled, nothing committed.
    let before = cluster.borrow().committed_count_of(&leader);
    let decisions_before = sched.decisions().len();
    let ghost = AgentId::of(
        &kw,
        &AgentDefinition {
            name: "ghost".into(),
            agent_type: "Worker".into(),
            owner: "ops@acme".into(),
            purpose: "never registered".into(),
            definition_version: 1,
        },
    );
    let refused = submit_attributed_effect(
        &mut sched,
        50,
        &world,
        &ghost,
        CapabilityId("cap.propose".into()),
        PolicyVerdict::Allow,
        b"illicit",
        &e,
    );
    assert_eq!(refused, Err(FlowError::NotRegistered { agent: ghost.hex() }));
    assert_eq!(sched.decisions().len(), decisions_before, "refusal queued nothing");
    assert_eq!(cluster.borrow().committed_count_of(&leader), before, "refusal committed nothing");

    // Truth on every replica: 2 registrations + 4 attributed effects, and the
    // attribution trail yields Who/What/When back out per agent (§3.19).
    assert_replicas_identical(&cluster, &a);
    for n in cluster.borrow().node_ids() {
        assert_eq!(cluster.borrow().committed_count_of(&n), 6);
        let w = world_head(&cluster, &n, &lw);
        let trail = attributed_effects(&w);
        assert_eq!(trail.len(), 4);
        assert_eq!(trail.iter().filter(|(who, _, _)| *who == w1).count(), 2);
        assert_eq!(trail.iter().filter(|(who, _, _)| *who == w2).count(), 2);
    }

    // ORCH-002: the schedule is a discardable plan artifact. Drop it; truth
    // unmoved. A FRESH scheduler re-submitting the identical plan converges
    // at the Kernel: every commit resolves deduped, zero fresh truth.
    drop(sched);
    assert_eq!(cluster.borrow().committed_count_of(&leader), 6);
    let mut rebuilt = ClusterScheduler::new(31, SchedulerConfig::default());
    for (i, (agent, effect)) in plan.iter().enumerate() {
        submit_attributed_effect(
            &mut rebuilt,
            100 + i as u64,
            &world,
            agent,
            CapabilityId("cap.propose".into()),
            PolicyVerdict::Allow,
            effect,
            &e,
        )
        .expect("re-admitted");
    }
    for t in 110..120 {
        rebuilt.dispatch_tick(t, &e);
    }
    assert!(rebuilt.is_idle());
    let fresh_commits = rebuilt
        .decisions()
        .iter()
        .filter(|d| matches!(d, SchedulingDecision::Committed { deduped: false, .. }))
        .count();
    assert_eq!(fresh_commits, 0, "rebuilt plan converged entirely by dedupe (ORCH-004)");
    assert_eq!(cluster.borrow().committed_count_of(&leader), 6, "zero new truth");
}

// ---------------------------------------------------------------------------
// (b) Duplicates and agreeing decisions converge to ONE truth (ORCH-004)
// ---------------------------------------------------------------------------

/// Convergence at every grade: (1) a duplicate proposal of one agent collapses
/// at the scheduler ledger; (2) the same proposal raced through TWO schedulers
/// lands exactly one fresh commit (Kernel dedupe); (3) two DIFFERENT agents
/// proposing the same decision content converge to ONE derived decision truth
/// — both when the second sees the first (pre-check) and when they race their
/// commits in (serialization-point agreement).
#[test]
fn duplicate_and_agreeing_proposals_converge_to_one_truth_orch004_across_agents() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let a1 = register(&cluster, &leader, &kw, "analyst-1");
    let a2 = register(&cluster, &leader, &kw, "analyst-2");
    let world = world_head(&cluster, &leader, &lw);

    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(Box::new(EchoActor { runs: runs.clone() }));
    bind_cap(&mut reg, &a, "cap.propose", &pid, EffectClass::ProposesWrite);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);

    // (1) same agent, same effect, same scheduler: ledger collapse — the
    // duplicate is VISIBLE (Deduplicated) and the actor is not re-invoked.
    let mut s1 = ClusterScheduler::new(41, SchedulerConfig::default());
    let first = submit_attributed_effect(
        &mut s1, 1, &world, &a1, CapabilityId("cap.propose".into()),
        PolicyVerdict::Allow, b"finding:alpha", &e,
    )
    .unwrap();
    assert!(matches!(first, SubmitOutcome::Admitted { .. }));
    let dup = submit_attributed_effect(
        &mut s1, 2, &world, &a1, CapabilityId("cap.propose".into()),
        PolicyVerdict::Allow, b"finding:alpha", &e,
    )
    .unwrap();
    assert!(matches!(dup, SubmitOutcome::Deduplicated { .. }));
    for t in 3..8 {
        s1.dispatch_tick(t, &e);
    }
    assert_eq!(runs.get(), 1, "duplicate never re-invoked the actor");

    // (2) the SAME proposal raced through a SECOND scheduler: at-least-once
    // compute, at-most-once TRUTH — the racing commit resolves deduped.
    let mut s2 = ClusterScheduler::new(42, SchedulerConfig::default());
    submit_attributed_effect(
        &mut s2, 10, &world, &a1, CapabilityId("cap.propose".into()),
        PolicyVerdict::Allow, b"finding:alpha", &e,
    )
    .unwrap();
    for t in 11..16 {
        s2.dispatch_tick(t, &e);
    }
    let fresh_across_both = [s1.decisions(), s2.decisions()]
        .iter()
        .flat_map(|d| d.iter())
        .filter(|d| matches!(d, SchedulingDecision::Committed { deduped: false, .. }))
        .count();
    assert_eq!(fresh_across_both, 1, "exactly one fresh commit across racing schedulers");
    assert_eq!(cluster.borrow().committed_count_of(&leader), 3, "2 registrations + 1 effect");

    // (3) decision-level convergence across DIFFERENT agents.
    let basis_stale = world_head(&cluster, &leader, &lw); // captured BEFORE any decision
    let d1 = propose(&cluster, &leader, &basis_stale, &a1, "plan/q3", "expand");
    let winner = match &d1 {
        ProposalOutcome::Committed { decision, .. } => decision.clone(),
        other => panic!("first proposal must commit, got {other:?}"),
    };
    // (3a) fresh basis (sees the prior): converge WITHOUT committing anything.
    let before = cluster.borrow().committed_count_of(&leader);
    let fresh_basis = world_head(&cluster, &leader, &lw);
    let d2 = propose(&cluster, &leader, &fresh_basis, &a2, "plan/q3", "expand");
    assert_eq!(
        d2,
        ProposalOutcome::Converged { winner: winner.clone(), superseded_attempt: None },
        "agreement converges onto the ONE decision truth"
    );
    assert_eq!(cluster.borrow().committed_count_of(&leader), before, "agreement committed nothing");
    // (3b) STALE basis (the scripted race: neither saw the other): the loser's
    // record commits but NEVER derives; outcome converges with the attempt
    // reported honestly.
    let d3 = propose(&cluster, &leader, &basis_stale, &a2, "plan/q3", "expand");
    match d3 {
        ProposalOutcome::Converged { winner: w, superseded_attempt: Some(_) } => {
            assert_eq!(w, winner, "the raced agreement still converges to the first-committed")
        }
        other => panic!("raced agreement must converge with a superseded attempt, got {other:?}"),
    }
    // One derived decision on the subject, on EVERY replica; no compliance
    // events (agreement is not a conflict).
    assert_replicas_identical(&cluster, &a);
    for n in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &n, &lw);
        assert_eq!(decision_of(&w, "plan/q3").expect("derived"), winner);
        assert_eq!(compliance_on(&w, "plan/q3"), vec![]);
    }
}

// ---------------------------------------------------------------------------
// (b) Conflicting decisions: FIRST-COMMITTED-WINS, loser receives the winner
// ---------------------------------------------------------------------------

/// The scripted check-then-commit race (design §3.8(5)): two agents pass the
/// pre-check on the same stale basis and both commit. Shard log order — total
/// per shard (IDR-001/IDR-005) — decides deterministically: the first-committed
/// record derives; the loser receives the WINNER's identity and the conflict is
/// committed compliance truth citing it (G1 `proposeDecision` at runtime
/// level). The rule is order-of-commit, not identity: the mirrored schedule
/// flips the winner. Same schedule re-run ⇒ byte-identical truth.
#[test]
fn conflicting_decisions_resolve_first_committed_wins_with_loser_receiving_winner() {
    // The schedule is parameterized by which agent proposes first.
    let run = |first_is_a1: bool| -> (Vec<u8>, String, String) {
        let a = sid("acme", "research");
        let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
        let cluster = cluster(3, &[a.clone()]);
        let leader = cluster.borrow().leader_of(&a).expect("elected");
        let a1 = register(&cluster, &leader, &kw, "dept-eng");
        let a2 = register(&cluster, &leader, &kw, "dept-legal");
        // BOTH capture the same basis before any decision exists (the TOCTOU).
        let stale = world_head(&cluster, &leader, &lw);

        let (first, first_action, second, second_action) = if first_is_a1 {
            (&a1, "approve", &a2, "reject")
        } else {
            (&a2, "reject", &a1, "approve")
        };
        let d_first = propose(&cluster, &leader, &stale, first, "contract/msa-9", first_action);
        let winner = match &d_first {
            ProposalOutcome::Committed { decision, .. } => decision.clone(),
            other => panic!("first commit wins, got {other:?}"),
        };
        assert_eq!(winner.record.agent, *first);

        // The second agent still holds the STALE basis: pre-check passes, its
        // record commits — and the serialization point reports the loss.
        let d_second =
            propose(&cluster, &leader, &stale, second, "contract/msa-9", second_action);
        match &d_second {
            ProposalOutcome::Conflict { winner: w, superseded_attempt, .. } => {
                assert_eq!(*w, winner, "the loser receives the WINNER's identity");
                assert!(superseded_attempt.is_some(), "the losing record was committed, then lost");
            }
            other => panic!("racing conflicting proposal must lose, got {other:?}"),
        }

        // A third proposal with a FRESH basis is refused at the pre-check:
        // NO decision record is committed, only the compliance event.
        let a3 = register(&cluster, &leader, &kw, "dept-fin");
        let before = cluster.borrow().committed_count_of(&leader);
        let fresh = world_head(&cluster, &leader, &lw);
        let d_third = propose(&cluster, &leader, &fresh, &a3, "contract/msa-9", "escalate");
        match &d_third {
            ProposalOutcome::Conflict { winner: w, superseded_attempt: None, .. } => {
                assert_eq!(*w, winner)
            }
            other => panic!("fresh-basis conflict must be refused pre-commit, got {other:?}"),
        }
        assert_eq!(
            cluster.borrow().committed_count_of(&leader),
            before + 1,
            "pre-check conflict commits exactly the compliance event, never a decision"
        );

        // Derivation is identical on EVERY replica: one decision truth, the
        // losing record visible only as a superseded attempt, and the
        // compliance ledger citing the winner from both losers.
        assert_replicas_identical(&cluster, &a);
        for n in cluster.borrow().node_ids() {
            let w = world_head(&cluster, &n, &lw);
            let derived = decision_of(&w, "contract/msa-9").expect("derived");
            assert_eq!(derived, winner, "first-committed-wins on {n:?}");
            let all = decisions_on(&w, "contract/msa-9");
            assert_eq!(all.len(), 2, "winner + the raced (superseded) attempt");
            assert_eq!(all[0], winner);
            let events = compliance_on(&w, "contract/msa-9");
            assert_eq!(events.len(), 2, "each loser recorded exactly one conflict event");
            for (ev, _, _) in &events {
                match &ev.outcome {
                    ComplianceOutcome::Conflict { prior_id, prior_action, .. } => {
                        assert_eq!(*prior_id, winner.id_hex, "the event cites the winner");
                        assert_eq!(*prior_action, winner.record.action);
                    }
                    other => panic!("expected conflict outcome, got {other:?}"),
                }
            }
        }
        let state = cluster.borrow().shard_state_of(&leader, &a);
        (state, winner.record.action.clone(), winner.record.agent.hex())
    };

    // Determinism: the same schedule twice ⇒ byte-identical truth.
    let (state_a, action_a, who_a) = run(true);
    let (state_b, action_b, who_b) = run(true);
    assert_eq!(state_a, state_b, "identical schedule ⇒ byte-identical shard state");
    assert_eq!((action_a.as_str(), &who_a), ("approve", &who_b));
    assert_eq!(action_a, action_b);
    // The mirrored schedule flips the winner: the rule is order, not identity.
    let (_, action_m, who_m) = run(false);
    assert_eq!(action_m, "reject");
    assert_ne!(who_m, who_a, "the mirrored schedule crowns the other agent");
}

// ---------------------------------------------------------------------------
// (d) Policy gate reads committed policy truth; approvals are separate truths
// ---------------------------------------------------------------------------

/// The G1 E1-hardened flow at runtime grade: the policy check reads COMMITTED
/// policy truths from the shared world; "approved" means a SEPARATE committed
/// approval truth by a registered agent ≠ the proposer (never a
/// proposer-supplied claim); refusals are committed compliance truths; the
/// admitted decision CITES its authorizing approvals (the audit Why).
#[test]
fn policy_gate_reads_committed_policy_truth_and_approvals_are_separate_truths() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let a1 = register(&cluster, &leader, &kw, "proposer");
    let a2 = register(&cluster, &leader, &kw, "approver");

    // Governance commits the policy AS TRUTH (Vol 9 Part 10: the Control
    // Plane enforces it, never owns it).
    let kernel = leader_kernel(&cluster, &leader);
    let policy = PolicyRecord { name: "legal-review".into(), scope: "contract/".into(), version: 1 };
    let committed_policy = commit_policy(&kernel, &kw, &policy).expect("policy committed");
    assert!(committed_policy.fresh);

    // (1) No approval exists: BLOCKED, and the refusal is committed truth.
    let basis = world_head(&cluster, &leader, &lw);
    let blocked = propose(&cluster, &leader, &basis, &a1, "contract/acme-msa", "sign");
    let first_compliance = match &blocked {
        ProposalOutcome::Blocked { policy: name, compliance } => {
            assert_eq!(name, "legal-review");
            compliance.clone()
        }
        other => panic!("unapproved decision must be blocked, got {other:?}"),
    };

    // (2) SELF-approval does not satisfy the gate (proposer ≠ approver) — and
    // the identical repeated refusal CONVERGES onto the SAME recorded
    // compliance truth (ORCH-004: even refusals are content-addressed; a
    // duplicate blocked attempt never forks the audit ledger).
    let basis = world_head(&cluster, &leader, &lw);
    commit_approval(&kernel, &basis, &a1, "contract/acme-msa").expect("self-approval commits");
    let basis = world_head(&cluster, &leader, &lw);
    match propose(&cluster, &leader, &basis, &a1, "contract/acme-msa", "sign") {
        ProposalOutcome::Blocked { compliance, .. } => assert_eq!(
            compliance, first_compliance,
            "a proposer can never authorize itself; the duplicate refusal converged (ORCH-004)"
        ),
        other => panic!("self-approved decision must stay blocked, got {other:?}"),
    }

    // (3) An UNREGISTERED approver is refused before any commit.
    let ghost = AgentId::of(
        &kw,
        &AgentDefinition {
            name: "ghost".into(),
            agent_type: "Worker".into(),
            owner: "ops@acme".into(),
            purpose: "never registered".into(),
            definition_version: 1,
        },
    );
    let before = cluster.borrow().committed_count_of(&leader);
    assert_eq!(
        commit_approval(&kernel, &basis, &ghost, "contract/acme-msa"),
        Err(FlowError::NotRegistered { agent: ghost.hex() })
    );
    assert_eq!(cluster.borrow().committed_count_of(&leader), before);

    // (4) A registered second agent approves — a SEPARATE committed truth —
    // and the decision now commits, CITING that approval (and NOT the
    // self-approval: cites are qualifying approvals only).
    let approval = commit_approval(&kernel, &basis, &a2, "contract/acme-msa").expect("approved");
    let basis = world_head(&cluster, &leader, &lw);
    let decided = propose(&cluster, &leader, &basis, &a1, "contract/acme-msa", "sign");
    let decision = match &decided {
        ProposalOutcome::Committed { decision, .. } => decision.clone(),
        other => panic!("approved decision must commit, got {other:?}"),
    };
    assert_eq!(decision.record.cites, vec![approval.id_hex.clone()], "the Why cites the approval");
    // The cited approval exists at a LOWER offset than the decision — the
    // evidence provably preceded the decision in the one committed history.
    let w = world_head(&cluster, &leader, &lw);
    let (_, approval_at) = w.get(&approval.id_hex).expect("approval is addressable truth");
    assert!(approval_at < decision.committed_at, "authorization precedes the decision");

    // (5) The compliance ledger is replayable audit truth on EVERY replica:
    // ONE recorded refusal (the identical second attempt converged onto it),
    // citing the policy truth.
    assert_replicas_identical(&cluster, &a);
    for n in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &n, &lw);
        let events = compliance_on(&w, "contract/acme-msa");
        assert_eq!(events.len(), 1, "the blocked attempts converged to one committed refusal");
        for (ev, _, _) in &events {
            assert_eq!(ev.agent, a1);
            match &ev.outcome {
                ComplianceOutcome::Blocked { policy: name, policy_id } => {
                    assert_eq!(name, "legal-review");
                    assert_eq!(*policy_id, committed_policy.id_hex);
                }
                other => panic!("expected blocked outcome, got {other:?}"),
            }
        }
        assert_eq!(decision_of(&w, "contract/acme-msa").unwrap(), decision);
    }

    // (6) HONEST LIMIT, pinned (OQ-3 class, module doc): the policy gate reads
    // the DECLARED basis — a policy committed AFTER a caller's basis does not
    // retro-block that caller's proposal. The serialization-point admission
    // re-check is a later-stage IDR obligation, said out loud, not hidden.
    let pre_policy_basis = world_head(&cluster, &leader, &lw);
    let hr_policy = PolicyRecord { name: "hr-review".into(), scope: "hr/".into(), version: 1 };
    commit_policy(&kernel, &kw, &hr_policy).expect("second policy committed");
    assert!(
        matches!(
            propose(&cluster, &leader, &pre_policy_basis, &a1, "hr/raise-7", "grant"),
            ProposalOutcome::Committed { .. }
        ),
        "pinned honest limit: a pre-policy basis does not see the new policy (recorded debt)"
    );
    let fresh = world_head(&cluster, &leader, &lw);
    assert!(
        matches!(
            propose(&cluster, &leader, &fresh, &a1, "hr/raise-8", "grant"),
            ProposalOutcome::Blocked { .. }
        ),
        "a fresh basis enforces the committed policy"
    );
}

// ---------------------------------------------------------------------------
// (c) Cross-agent consistency reads: the I3 ladder, labeled guarantees
// ---------------------------------------------------------------------------

/// Agent B's next step sees agent A's committed decision per the I3 ladder:
/// Linearizable reads confirm currency with the leader (and see the decision);
/// on a partitioned replica Linearizable and BoundedStaleness REFUSE honestly
/// while Eventual serves stale data with exact labels — and a stale basis can
/// never mint a second derived truth (the serialization point catches it).
#[test]
fn cross_agent_reads_follow_the_i3_ladder_with_labeled_guarantees() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let a1 = register(&cluster, &leader, &kw, "dept-ops");
    let a2 = register(&cluster, &leader, &kw, "dept-sec");

    // Agent A commits a decision through the flow (leader gateway).
    let basis = world_head(&cluster, &leader, &lw);
    let d1 = propose(&cluster, &leader, &basis, &a1, "ops/deploy-42", "ship");
    let (truth1, decision1) = match d1 {
        ProposalOutcome::Committed { truth, decision } => (truth, decision),
        other => panic!("expected commit, got {other:?}"),
    };
    let floor = floor_of(&truth1);
    cluster.borrow_mut().settle(5);

    // Agent B's next step, from a FOLLOWER, at the Linearizable tier: sees
    // A's committed decision, with the label saying exactly that.
    let followers: Vec<NodeId> =
        cluster.borrow().node_ids().into_iter().filter(|n| *n != leader).collect();
    let (f1, f2) = (followers[0].clone(), followers[1].clone());
    let q = ClusterQuery::new(f1.clone(), Rc::clone(&cluster));
    let scope = ReadScope::linearizable("acme/research".into());
    let proj = q.read(&scope, &decision1.id_hex).expect("linearizable read serves");
    assert_eq!(proj.served_tier, ReadTier::Linearizable);
    assert!(proj.observed_at >= floor, "read-your-writes floor reached");
    // The served bytes ARE the decision record; B's next step derives from
    // exactly that observed version and detects the conflict deterministically.
    let store_f1 = cluster.borrow().wal_store_of(&f1);
    let world_b = WorldView::at_version(&store_f1, &lw, proj.observed_at).expect("B's basis");
    assert_eq!(decision_of(&world_b, "ops/deploy-42").expect("visible"), decision1);
    match propose(&cluster, &leader, &world_b, &a2, "ops/deploy-42", "rollback") {
        ProposalOutcome::Conflict { winner, superseded_attempt: None, .. } => {
            assert_eq!(winner, decision1, "B's next step saw A's truth and was refused pre-commit")
        }
        other => panic!("expected pre-check conflict, got {other:?}"),
    }

    // Partition one replica; commit MORE truth it cannot see.
    cluster.borrow_mut().isolate(&a, &f2);
    let basis = world_head(&cluster, &leader, &lw);
    let d2 = propose(&cluster, &leader, &basis, &a1, "ops/deploy-43", "ship");
    let decision2 = match d2 {
        ProposalOutcome::Committed { decision, .. } => decision,
        other => panic!("expected commit, got {other:?}"),
    };
    cluster.borrow_mut().settle(3);

    let q_stale = ClusterQuery::new(f2.clone(), Rc::clone(&cluster));
    // Linearizable on the partitioned replica: honest refusal, never stale-as-fresh.
    assert_eq!(
        q_stale.read(&ReadScope::linearizable("acme/research".into()), &decision2.id_hex).err(),
        Some(QueryError::LeaderUnavailable),
        "cannot confirm currency with the leader ⇒ refuse"
    );
    // BoundedStaleness: lag is unattestable ⇒ refused with the sentinel.
    match q_stale
        .read(
            &ReadScope::bounded("acme/research".into(), StalenessBound { max_lag: 1_000 }),
            &decision2.id_hex,
        )
        .err()
    {
        Some(QueryError::StalenessBoundExceeded { observed_lag, .. }) => {
            assert_eq!(observed_lag, LAG_UNATTESTABLE)
        }
        other => panic!("expected staleness refusal, got {other:?}"),
    }
    // Eventual: SERVES, stale but exactly labeled — the new decision is
    // honestly absent; the old one is served at the replica's own position.
    let ev_scope = ReadScope::eventual("acme/research".into());
    assert!(
        matches!(q_stale.read(&ev_scope, &decision2.id_hex), Err(QueryError::NotFound { .. })),
        "the isolated replica honestly does not have the new truth yet"
    );
    let stale_read = q_stale.read(&ev_scope, &decision1.id_hex).expect("old truth still served");
    assert_eq!(stale_read.served_tier, ReadTier::Eventual);
    let leader_head = world_head(&cluster, &leader, &lw).observed_at();
    assert!(stale_read.observed_at < leader_head, "staleness is visible in the label");

    // A stale basis can never mint a second derived truth: the proposal built
    // on the isolated replica's world commits through the LEADER and loses at
    // the serialization point.
    let store_f2 = cluster.borrow().wal_store_of(&f2);
    let stale_world = WorldView::at_head(&store_f2, &lw).expect("stale world");
    assert!(decision_of(&stale_world, "ops/deploy-43").is_none(), "the basis is truly stale");
    match propose(&cluster, &leader, &stale_world, &a2, "ops/deploy-43", "rollback") {
        ProposalOutcome::Conflict { winner, superseded_attempt: Some(_), .. } => {
            assert_eq!(winner, decision2, "the loser received the winner it could not see")
        }
        other => panic!("expected serialization-point conflict, got {other:?}"),
    }

    // Heal: the isolated replica converges to the identical truth bytes.
    cluster.borrow_mut().heal(&a);
    assert_replicas_identical(&cluster, &a);
    for n in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &n, &lw);
        assert_eq!(decision_of(&w, "ops/deploy-43").unwrap(), decision2);
    }
}

// ---------------------------------------------------------------------------
// (b) Seeded interleavings: every permutation, one derived truth, no forks
// ---------------------------------------------------------------------------

/// ALL six permutations of three agents racing conflicting decisions on one
/// subject from one shared stale basis — interleaved with scheduler-borne
/// effect proposals (including a duplicate). Per schedule: the first-executed
/// proposal wins (first-committed-wins is schedule-determined, never
/// identity-determined), every loser receives the winner + a committed
/// conflict event, exactly ONE decision derives, duplicates never fork, and
/// all replicas hold byte-identical truth. Same schedule twice ⇒ identical
/// bytes.
#[test]
fn seeded_interleaving_permutations_yield_one_derived_truth_and_no_forks() {
    const SUBJECT: &str = "plan/q4";
    let actions = ["expand", "hold", "cut"];
    let permutations: [[usize; 3]; 6] =
        [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];

    let run = |order: &[usize; 3]| -> (usize, Vec<u8>) {
        let a = sid("acme", "research");
        let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
        let cluster = cluster(3, &[a.clone()]);
        let leader = cluster.borrow().leader_of(&a).expect("elected");
        let agents = [
            register(&cluster, &leader, &kw, "actor-0"),
            register(&cluster, &leader, &kw, "actor-1"),
            register(&cluster, &leader, &kw, "actor-2"),
        ];
        // ONE shared stale basis: nobody sees anybody (the scripted race).
        let stale = world_head(&cluster, &leader, &lw);

        let mut reg = MemRegistry::new();
        let mut host = EngineHost::new();
        let runs = Rc::new(Cell::new(0));
        let pid = host.host(Box::new(EchoActor { runs: runs.clone() }));
        bind_cap(&mut reg, &a, "cap.propose", &pid, EffectClass::ProposesWrite);
        let down = BTreeSet::new();
        let e = env(&cluster, &reg, &host, &down);
        let mut sched = ClusterScheduler::new(77, SchedulerConfig::default());

        // The interleaved schedule: for each slot, one decision proposal AND
        // one scheduler-borne effect (actor-0's effect submitted twice — the
        // duplicate). Effects dispatch between proposals: truly interleaved.
        let mut outcomes: Vec<(usize, ProposalOutcome)> = Vec::new();
        let mut tick = 1u64;
        for (slot, &i) in order.iter().enumerate() {
            let out = propose(&cluster, &leader, &stale, &agents[i], SUBJECT, actions[i]);
            outcomes.push((i, out));
            let effect: &[u8] = if slot == 0 { b"metric:latency" } else { b"metric:latency-2" };
            // actor-0 submits in every slot — slot>0 payloads repeat slot 1's
            // effect for actor-0, making a true duplicate in slots 1 and 2.
            let dup_effect: &[u8] = if slot == 0 { effect } else { b"metric:latency-2" };
            submit_attributed_effect(
                &mut sched,
                tick,
                &stale,
                &agents[0],
                CapabilityId("cap.propose".into()),
                PolicyVerdict::Allow,
                dup_effect,
                &e,
            )
            .expect("registered actor admitted or deduplicated");
            sched.dispatch_tick(tick + 1, &e);
            sched.dispatch_tick(tick + 2, &e);
            tick += 10;
        }
        for t in tick..tick + 5 {
            sched.dispatch_tick(t, &e);
        }
        assert!(sched.is_idle());

        // Schedule-level invariants.
        let winner_index = order[0];
        for (i, out) in &outcomes {
            if *i == winner_index {
                assert!(
                    matches!(out, ProposalOutcome::Committed { .. }),
                    "the first-executed proposal wins its schedule"
                );
            } else {
                match out {
                    ProposalOutcome::Conflict { winner, superseded_attempt: Some(_), .. } => {
                        assert_eq!(winner.record.agent, agents[winner_index]);
                        assert_eq!(winner.record.action, actions[winner_index]);
                    }
                    other => panic!("every raced loser must lose loudly, got {other:?}"),
                }
            }
        }

        // Truth-level invariants, on EVERY replica.
        assert_replicas_identical(&cluster, &a);
        for n in cluster.borrow().node_ids() {
            let w = world_head(&cluster, &n, &lw);
            let derived = decision_of(&w, SUBJECT).expect("exactly one decision derives");
            assert_eq!(derived.record.agent, agents[winner_index]);
            assert_eq!(derived.record.action, actions[winner_index]);
            assert_eq!(decisions_on(&w, SUBJECT).len(), 3, "all attempts traced, one derives");
            let events = compliance_on(&w, SUBJECT);
            assert_eq!(events.len(), 2, "each loser recorded exactly one conflict event");
            for (ev, _, _) in &events {
                match &ev.outcome {
                    ComplianceOutcome::Conflict { prior_id, .. } => {
                        assert_eq!(*prior_id, derived.id_hex)
                    }
                    other => panic!("expected conflict, got {other:?}"),
                }
            }
            // Scheduler-borne effects: 2 unique envelopes, the duplicate
            // collapsed — the attributed trail carries exactly 2 effects.
            assert_eq!(attributed_effects(&w).len(), 2, "duplicate proposals never fork");
        }
        let state = cluster.borrow().shard_state_of(&leader, &a);
        (winner_index, state)
    };

    let mut states = Vec::new();
    for order in &permutations {
        let (winner, state) = run(order);
        assert_eq!(winner, order[0], "the winner is the schedule's first proposal — always");
        states.push(state);
    }
    // Determinism: an identical schedule re-run is byte-identical truth.
    let (_, replay) = run(&permutations[0]);
    assert_eq!(replay, states[0], "same scripted schedule ⇒ byte-identical shard state");
}

// ---------------------------------------------------------------------------
// Amendment A1 pin: an ungoverned (unregistered-Who) decision never derives
// ---------------------------------------------------------------------------

/// The RCR-029-A1 analog at the decision surface: the frozen gateway has no
/// principal and does not verify `content == ACS-hash(payload)`, so a caller
/// CAN lawfully commit a hand-crafted decision record wearing an UNREGISTERED
/// Who into the shard WAL. Pinned: such a record NEVER derives — it cannot
/// win, block, or supersede a governed decision — while remaining visible as
/// raw, append-only trace. HONEST-LIMIT PIN: the same smuggle wearing a
/// REGISTERED identity DOES derive (any in-process caller can wear any
/// registered identity under v1.x — v2.0 debt #8, kept loud, never hidden).
/// POLICY-BYPASS PIN (adversarial revision A2): the same raw-gateway path also
/// bypasses the POLICY/approval gate entirely — a worn registered-Who record
/// on a POLICY-SCOPED subject derives with NO approval ever committed and with
/// unverifiable cites (derivation checks registration ONLY, never
/// cites/policy) — the same v1.x debt-#8 class, pinned as loud as the
/// identity half.
#[test]
fn smuggled_ungoverned_decision_never_derives() {
    use arves_control_plane::multi_agent::AgentDecisionRecord;
    use arves_kernel::{ContentHash, Kernel, ProposedWrite};

    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let a1 = register(&cluster, &leader, &kw, "worker-1");
    let ghost = AgentId::of(
        &kw,
        &AgentDefinition {
            name: "ghost".into(),
            agent_type: "Worker".into(),
            owner: "ops@acme".into(),
            purpose: "never registered".into(),
            definition_version: 1,
        },
    );

    // Smuggle: a decision wearing the UNREGISTERED ghost, committed FIRST on
    // the subject, directly through the raw frozen gateway (lawful under v1.x).
    let kernel = leader_kernel(&cluster, &leader);
    let smuggled = AgentDecisionRecord {
        subject: "plan/x".into(),
        action: "sabotage".into(),
        agent: ghost.clone(),
        cites: vec![],
    };
    let payload = smuggled.canonical_bytes();
    let content = arves_acs::content_id(arves_acs::domain::COMMIT_CONTENT, &payload);
    let smuggled_hex = arves_acs::hex(&content);
    kernel
        .commit(ProposedWrite { shard: kw.clone(), content: ContentHash(content), payload })
        .expect("the raw gateway lawfully admits the smuggle (no principal in v1.x)");

    // The ungoverned record NEVER derives: it cannot pre-empt the subject.
    let w = world_head(&cluster, &leader, &lw);
    assert!(w.contains(&smuggled_hex), "the smuggle IS visible raw trace (append-only honesty)");
    assert_eq!(decision_of(&w, "plan/x"), None, "an ungoverned Who never derives");

    // A governed proposal on the same subject WINS despite committing later.
    let basis = world_head(&cluster, &leader, &lw);
    let governed = match propose(&cluster, &leader, &basis, &a1, "plan/x", "proceed") {
        ProposalOutcome::Committed { decision, .. } => decision,
        other => panic!("the governed decision must commit and derive, got {other:?}"),
    };
    assert_replicas_identical(&cluster, &a);
    for n in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &n, &lw);
        let derived = decision_of(&w, "plan/x").expect("governed decision derives");
        assert_eq!(derived, governed);
        assert_eq!(derived.record.agent, a1);
        assert_eq!(decisions_on(&w, "plan/x").len(), 1, "only governed records derive");
    }

    // HONEST-LIMIT PIN: wearing a REGISTERED identity through the raw gateway
    // DOES derive — identity is structural, not cryptographic, under v1.x.
    let worn = AgentDecisionRecord {
        subject: "plan/y".into(),
        action: "worn-identity".into(),
        agent: a1.clone(),
        cites: vec![],
    };
    let payload = worn.canonical_bytes();
    let content = arves_acs::content_id(arves_acs::domain::COMMIT_CONTENT, &payload);
    kernel
        .commit(ProposedWrite { shard: kw.clone(), content: ContentHash(content), payload })
        .expect("lawful commit");
    let w = world_head(&cluster, &leader, &lw);
    assert_eq!(
        decision_of(&w, "plan/y").expect("derives").record.action,
        "worn-identity",
        "v1.x structural limit pinned: a registered identity can be worn by any caller"
    );

    // POLICY-BYPASS PIN (adversarial revision A2): Governance commits a policy
    // over "plan/" — the governed FLOW now blocks unapproved decisions in that
    // scope — yet a hand-crafted record wearing the registered identity,
    // committed through the raw frozen gateway, derives on a policy-scoped
    // subject with NO approval truth in existence and with unverifiable cites:
    // derivation checks registration ONLY, never cites/policy. Same v1.x
    // debt-#8 class as the worn identity above, kept loud, never hidden.
    let plan_policy =
        PolicyRecord { name: "plan-review".into(), scope: "plan/".into(), version: 1 };
    commit_policy(&kernel, &kw, &plan_policy).expect("policy committed as truth");
    let fresh = world_head(&cluster, &leader, &lw);
    assert!(
        matches!(
            propose(&cluster, &leader, &fresh, &a1, "plan/z-flow", "flow-attempt"),
            ProposalOutcome::Blocked { .. }
        ),
        "the governed FLOW enforces the committed policy on this scope"
    );
    let bypass = AgentDecisionRecord {
        subject: "plan/z".into(),
        action: "policy-bypass".into(),
        agent: a1.clone(),
        cites: vec!["never-a-committed-approval".into()],
    };
    let payload = bypass.canonical_bytes();
    let content = arves_acs::content_id(arves_acs::domain::COMMIT_CONTENT, &payload);
    kernel
        .commit(ProposedWrite { shard: kw.clone(), content: ContentHash(content), payload })
        .expect("the raw gateway lawfully admits it (no principal, no policy gate in v1.x)");
    let w = world_head(&cluster, &leader, &lw);
    assert!(
        approvals_on(&w, "plan/z").is_empty(),
        "no approval truth exists anywhere for the subject"
    );
    let derived = decision_of(&w, "plan/z")
        .expect("pinned honest limit: the smuggle derives DESPITE the committed policy");
    assert_eq!(derived.record.action, "policy-bypass", "no approval was ever required of it");
    assert_eq!(
        derived.record.cites,
        vec!["never-a-committed-approval".to_string()],
        "cites are unverified at derivation: the smuggle's Why is arbitrary bytes"
    );
}

// ---------------------------------------------------------------------------
// Reconciliation honesty: a behind store cannot arbitrate
// ---------------------------------------------------------------------------

/// The serialization-point reconciliation refuses LOUDLY when handed a store
/// that has not applied the flow's own just-acked commit (lossless-or-loud:
/// a behind arbiter could crown the wrong winner). The committed record is NOT
/// rolled back — re-proposing against a caught-up store converges idempotently
/// onto it (ORCH-004): no retry can fork truth.
#[test]
fn reconcile_store_behind_is_refused_loudly_and_reproposal_converges() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let a1 = register(&cluster, &leader, &kw, "worker-1");
    // Let every replica apply the registration (its shard WAL now exists),
    // THEN isolate a follower so its local store falls behind the truth.
    cluster.borrow_mut().settle(5);
    let behind: NodeId =
        cluster.borrow().node_ids().into_iter().filter(|n| *n != leader).last().expect("follower");
    cluster.borrow_mut().isolate(&a, &behind);

    let basis = world_head(&cluster, &leader, &lw);
    let kernel = leader_kernel(&cluster, &leader);
    let behind_store = cluster.borrow().wal_store_of(&behind);
    // Commit succeeds at the leader (quorum of 2/3) but the BEHIND store
    // cannot arbitrate the serialization point: refuse, never guess.
    match propose_decision(&kernel, &behind_store, &basis, &a1, "ops/task-1", "run") {
        Err(FlowError::ReconcileBehind { .. }) => {}
        other => panic!("a behind arbiter must be refused loudly, got {other:?}"),
    }

    // The record IS committed truth (not rolled back); re-proposing against a
    // caught-up store converges onto it — same agent, same content, one truth.
    let leader_store = cluster.borrow().wal_store_of(&leader);
    let before = cluster.borrow().committed_count_of(&leader);
    let fresh_basis = world_head(&cluster, &leader, &lw);
    match propose_decision(&kernel, &leader_store, &fresh_basis, &a1, "ops/task-1", "run") {
        Ok(ProposalOutcome::Converged { winner, superseded_attempt: None }) => {
            assert_eq!(winner.record.agent, a1);
            assert_eq!(winner.record.action, "run");
        }
        other => panic!("re-proposal must converge onto the committed record, got {other:?}"),
    }
    assert_eq!(cluster.borrow().committed_count_of(&leader), before, "no second truth");

    // Heal; the once-behind replica converges byte-identically and derives
    // the same single decision.
    cluster.borrow_mut().heal(&a);
    assert_replicas_identical(&cluster, &a);
    let w = world_head(&cluster, &behind, &lw);
    assert_eq!(decision_of(&w, "ops/task-1").expect("derived").record.agent, a1);
}

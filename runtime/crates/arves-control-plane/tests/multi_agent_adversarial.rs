//! RCR-031 (I5 Stage 3) — ADVERSARIAL MULTI-AGENT PROOFS per the I5 design's
//! conformance plan (`docs/design/I5_MultiAgent_Runtime_Design.md` §4/§5):
//!
//! (a) AGENT STORMS — N agents × M proposals under seeded schedule
//!     permutations: the FINAL TRUTH SET is identical across ALL permutations
//!     (the order-independence proof; the shard log ORDER differs per
//!     schedule — honestly asserted — but the truth SET and the attribution
//!     trail SET never do; ORCH-004 at storm scale);
//! (b) BYZANTINE-ISH LAWFUL-API MISUSE — an agent replaying another agent's
//!     proposal cannot forge attribution (the Who rides INSIDE the addressed
//!     content: a replay dedupes to the SAME truth, a rebind of the address to
//!     a re-attributed payload is refused by the RCR-005 content-integrity
//!     gate, a re-wrap is a DIFFERENT truth under the re-wrapper's own Who);
//!     duplicate floods across racing schedulers can never double-commit;
//! (c) PARTITION DURING MULTI-AGENT WORK — minority-side proposals fail
//!     honestly (CP: no quorum, no truth — never a silent fork), the majority
//!     keeps working, heal converges byte-identically, and no ACKED attributed
//!     truth is ever lost (IDR-001/004);
//! (d) FULL-CLUSTER DETERMINISTIC REPLAY INCLUDING ATTRIBUTION — every node
//!     rebuilt from its own WAL reproduces identical truth AND an identical
//!     attribution trail / decision derivation / compliance ledger (ORCH-003:
//!     the WAL is the multi-agent decision trace, Who included).
//!
//! HONEST LANGUAGE: every "agent" here is a DETERMINISTIC TEST ACTOR (a
//! registered identity driven by scripted, seeded schedules), NOT an AI model.
//! Identity is structural, not cryptographic (v2.0 debt #8 / design OQ-1) —
//! the worn-identity limit is pinned by RCR-029/030 tests and is not
//! re-claimed otherwise here. "Byzantine-ish" means LAWFUL-API misuse under
//! the trusted-single-host model: adversaries use the public runtime surface
//! (including the raw frozen gateway), never memory corruption or a network
//! adversary (no network exists — in-process `ClusterSim`).
//!
//! Every test is deterministic: fixed seeds, logical ticks, scripted
//! elections/settles/partitions, seeded xorshift permutations — zero wall
//! clocks, zero OS randomness, zero sleeps.

use arves_capability_fabric::gate::PolicyVerdict;
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as FabricShardKey,
};
use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_control_plane::agents::{
    attributed_effects, attribution_of, encode_attributed, propose_attributed, register_agent,
    AgentDefinition, AgentError, AgentId,
};
use arves_control_plane::multi_agent::{
    commit_approval, commit_policy, compliance_on, decision_of, propose_decision,
    submit_attributed_effect, PolicyRecord, ProposalOutcome,
};
use arves_control_plane::scheduler::{
    ClusterScheduler, DispatchEnv, EngineHost, SchedulerConfig, SchedulingDecision, SubmitOutcome,
};
use arves_engine_fabric::{
    invocation_key, Determinism, Engine, EngineManifest, IdempotencyKey, Inference, ProposedEffect,
};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{CommitError, ContentHash, Kernel, ProposedWrite, ShardKey as KernelShardKey};
use arves_lcw::world::WorldView;
use arves_lcw::ShardKey as LcwShardKey;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Deterministic scaffolding (the RCR-029/030 harness discipline)
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

fn cluster(nodes: usize, shards: &[ShardId]) -> Rc<RefCell<ClusterSim>> {
    let mut sim = ClusterSim::new(nodes);
    for (i, s) in shards.iter().enumerate() {
        sim.add_shard(s.clone(), 31 + i as u64); // fixed seeds: recorded ⇒ replayable
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
            effect: EffectClass::ProposesWrite,
        },
    })
    .expect("bind");
}

/// The deterministic AGENT ACTOR engine (RCR-030 harness): a pure echo — the
/// attribution envelope passes through verbatim, so the committed truth
/// carries the Who inside the addressed content.
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

fn assert_replicas_identical(cluster: &Rc<RefCell<ClusterSim>>, shard: &ShardId) {
    cluster.borrow_mut().settle(5);
    let c = cluster.borrow();
    let nodes = c.node_ids();
    let reference = c.shard_state_of(&nodes[0], shard);
    for n in &nodes {
        assert_eq!(c.shard_state_of(n, shard), reference, "byte-identical truth at {n:?}");
    }
}

/// Deterministic xorshift64 (fixed seed ⇒ recorded, replayable permutations —
/// zero OS randomness).
fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// A seeded Fisher–Yates permutation of `0..n` (pure function of the seed).
fn seeded_permutation(n: usize, seed: u64) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..n).collect();
    let mut s = seed;
    for i in (1..n).rev() {
        let j = (xorshift(&mut s) % (i as u64 + 1)) as usize;
        idx.swap(i, j);
    }
    idx
}

// ---------------------------------------------------------------------------
// (a) Agent storms: seeded schedule permutations ⇒ one identical truth set
// ---------------------------------------------------------------------------

/// N=3 agents × M=4 proposals each (12 distinct effects) + 3 injected
/// duplicates = 15 submissions, executed under SEVEN schedules: identity,
/// reverse, and five seeded xorshift permutations. Per schedule everything
/// flows through the I4 scheduler into the shard leader's frozen gateway.
/// The ORDER-INDEPENDENCE proof: the final TRUTH SET (content id → payload)
/// and the per-agent attribution trail SET are IDENTICAL across ALL
/// schedules, on EVERY replica — while the shard log ORDER honestly differs
/// (asserted: at least one permutation lands different state bytes than the
/// identity schedule, so the set-equality claim is not vacuous). Duplicates
/// collapse visibly at the scheduler ledger and never fork truth (ORCH-004).
#[test]
fn agent_storm_truth_set_identical_across_all_seeded_schedule_permutations() {
    const AGENTS: usize = 3;
    const EFFECTS_PER_AGENT: usize = 4;

    // The submission plan: (agent index, effect payload). 12 distinct + the
    // first effect of each agent repeated once (the injected duplicates).
    let mut plan: Vec<(usize, Vec<u8>)> = Vec::new();
    for a in 0..AGENTS {
        for e in 0..EFFECTS_PER_AGENT {
            plan.push((a, format!("eff:{a}:{e}").into_bytes()));
        }
    }
    for a in 0..AGENTS {
        plan.push((a, format!("eff:{a}:0").into_bytes())); // duplicate
    }
    let n = plan.len();
    assert_eq!(n, 15);

    // Seven scripted schedules: identity + reverse + five seeded permutations.
    let mut schedules: Vec<Vec<usize>> = vec![(0..n).collect(), (0..n).rev().collect()];
    for seed in [0x9E37_79B9_7F4A_7C15u64, 0x0BAD_5EED, 0xDEAD_BEEF, 0x1234_5678, 0xC0FF_EE11] {
        schedules.push(seeded_permutation(n, seed));
    }

    let run = |order: &[usize]| -> (BTreeMap<String, Vec<u8>>, Vec<(String, Vec<u8>)>, Vec<u8>) {
        let a = sid("acme", "research");
        let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
        let cluster = cluster(3, &[a.clone()]);
        let leader = cluster.borrow().leader_of(&a).expect("elected");
        let agents: Vec<AgentId> = (0..AGENTS)
            .map(|i| register(&cluster, &leader, &kw, &format!("storm-actor-{i}")))
            .collect();
        let world = world_head(&cluster, &leader, &lw);

        let mut reg = MemRegistry::new();
        let mut host = EngineHost::new();
        let runs = Rc::new(Cell::new(0));
        let pid = host.host(Box::new(EchoActor { runs: runs.clone() }));
        bind_cap(&mut reg, &a, "cap.storm", &pid);
        let down = BTreeSet::new();
        let e = env(&cluster, &reg, &host, &down);

        // The storm: queue the WHOLE permuted schedule first (capacity 32 —
        // admission never bites, so every schedule submits the same plan),
        // then dispatch to idle. Deterministic ticks throughout.
        let mut sched = ClusterScheduler::new(
            9090,
            SchedulerConfig { shard_capacity: 32, retry_budget: 3, dispatch_per_tick: 1 },
        );
        let mut deduplicated = 0usize;
        for (t, &slot) in order.iter().enumerate() {
            let (agent, effect) = &plan[slot];
            match submit_attributed_effect(
                &mut sched,
                t as u64 + 1,
                &world,
                &agents[*agent],
                CapabilityId("cap.storm".into()),
                PolicyVerdict::Allow,
                effect,
                &e,
            )
            .expect("registered actor admitted or deduplicated")
            {
                SubmitOutcome::Deduplicated { .. } => deduplicated += 1,
                SubmitOutcome::Admitted { .. } => {}
                other => panic!("storm submission must never be refused, got {other:?}"),
            }
        }
        assert_eq!(deduplicated, AGENTS, "each injected duplicate collapsed VISIBLY at the ledger");
        let mut tick = 100u64;
        while !sched.is_idle() && tick < 200 {
            sched.dispatch_tick(tick, &e);
            tick += 1;
        }
        assert!(sched.is_idle(), "the storm drains deterministically");
        assert_eq!(runs.get(), (AGENTS * EFFECTS_PER_AGENT) as u64, "each distinct proposal once");

        // Truth-level invariants on EVERY replica.
        assert_replicas_identical(&cluster, &a);
        let mut truth_map = BTreeMap::new();
        let mut trail_set = Vec::new();
        for node in cluster.borrow().node_ids() {
            assert_eq!(
                cluster.borrow().committed_count_of(&node),
                AGENTS + AGENTS * EFFECTS_PER_AGENT,
                "3 registrations + 12 effects — duplicates never fork (ORCH-004)"
            );
            let w = world_head(&cluster, &node, &lw);
            let map: BTreeMap<String, Vec<u8>> =
                w.iter().map(|(id, p, _)| (id.to_string(), p.to_vec())).collect();
            let trail = attributed_effects(&w);
            for (i, agent) in agents.iter().enumerate() {
                assert_eq!(
                    trail.iter().filter(|(who, _, _)| who == agent).count(),
                    EFFECTS_PER_AGENT,
                    "agent {i}'s attribution trail is complete on {node:?}"
                );
            }
            // A sorted SET view of the trail (who, what) — order-independent.
            let mut set: Vec<(String, Vec<u8>)> =
                trail.into_iter().map(|(who, what, _)| (who.hex(), what)).collect();
            set.sort();
            if truth_map.is_empty() {
                truth_map = map;
                trail_set = set;
            } else {
                assert_eq!(map, truth_map, "replica truth maps agree within the schedule");
                assert_eq!(set, trail_set, "replica trails agree within the schedule");
            }
        }
        let state = cluster.borrow().shard_state_of(&leader, &a);
        (truth_map, trail_set, state)
    };

    let (reference_map, reference_trail, identity_state) = run(&schedules[0]);
    let mut any_order_differs = false;
    for order in &schedules[1..] {
        let (map, trail, state) = run(order);
        assert_eq!(map, reference_map, "the FINAL TRUTH SET is schedule-independent");
        assert_eq!(trail, reference_trail, "the attribution trail SET is schedule-independent");
        any_order_differs |= state != identity_state;
    }
    assert!(
        any_order_differs,
        "honesty check: the shard log ORDER differs across permutations — the set-level \
         order-independence claim is proven over genuinely different interleavings"
    );
    // Determinism: the identical schedule re-run is byte-identical state.
    let (_, _, replay) = run(&schedules[0]);
    assert_eq!(replay, identity_state, "same seeded schedule ⇒ byte-identical shard state");
}

// ---------------------------------------------------------------------------
// (b) Lawful-API misuse: replay/rebind/re-wrap cannot forge attribution;
//     duplicate floods cannot double-commit
// ---------------------------------------------------------------------------

/// Agent B tries every LAWFUL way to steal or corrupt agent A's attribution:
/// (1) REPLAY A's exact committed envelope through the raw gateway — dedupes
/// to the SAME truth (ORCH-004), attribution stays A, zero new truth;
/// (2) REBIND A's content address to a payload re-attributed to B — refused
/// by the RCR-005 content-integrity gate (an address can never be re-bound;
/// attribution is immutable once addressed); (3) RE-WRAP A's effect bytes
/// under B's own identity — a DIFFERENT truth attributed to B; A's original
/// row is untouched and the two Whos never blur. Then the duplicate FLOOD:
/// the same proposal submitted 4× through each of TWO racing schedulers plus
/// direct re-proposals — exactly ONE fresh commit ever lands (at-least-once
/// compute, at-most-once truth). HONEST LIMIT (not re-claimed here): a caller
/// wearing a REGISTERED identity through the raw gateway is pinned by
/// RCR-029/030 — structural, not cryptographic, identity under v1.x.
#[test]
fn replay_rebind_and_rewrap_cannot_forge_attribution_and_floods_never_double_commit() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let leader = cluster.borrow().leader_of(&a).expect("elected");
    let alice = register(&cluster, &leader, &kw, "author-alice");
    let bob = register(&cluster, &leader, &kw, "adversary-bob");
    let kernel = leader_kernel(&cluster, &leader);

    // Alice commits an attributed effect through the governed path.
    let world = world_head(&cluster, &leader, &lw);
    let effect: &[u8] = b"finding:quarterly";
    let committed =
        propose_attributed(&kernel, &world, &alice, effect).expect("alice's proposal commits");
    assert!(committed.fresh);
    let alice_envelope = encode_attributed(&alice, effect);
    let alice_content =
        arves_acs::content_id(arves_acs::domain::COMMIT_CONTENT, &alice_envelope);
    let alice_hex = arves_acs::hex(&alice_content);
    let before = cluster.borrow().committed_count_of(&leader);

    // (1) REPLAY: bob re-commits alice's EXACT envelope through the raw
    // gateway. The Kernel dedupes to the SAME truth — no double commit, and
    // the attribution is still alice's (the Who is inside the content).
    match kernel.commit(ProposedWrite {
        shard: kw.clone(),
        content: ContentHash(alice_content.clone()),
        payload: alice_envelope.clone(),
    }) {
        Err(CommitError::AlreadyCommitted(t)) => {
            assert_eq!(t, committed.truth, "the replay resolves to alice's ORIGINAL truth")
        }
        other => panic!("a replay must dedupe, got {other:?}"),
    }
    assert_eq!(cluster.borrow().committed_count_of(&leader), before, "replay committed nothing");

    // (2) REBIND: bob crafts a payload attributing alice's effect to HIMSELF
    // and proposes it under alice's ORIGINAL content address. The RCR-005
    // content-integrity gate refuses the fork: an address can never be
    // re-bound, so committed attribution can never be rewritten.
    let forged = encode_attributed(&bob, effect);
    match kernel.commit(ProposedWrite {
        shard: kw.clone(),
        content: ContentHash(alice_content.clone()),
        payload: forged.clone(),
    }) {
        Err(CommitError::ContentIntegrity { .. }) => {}
        other => panic!("an address rebind must be refused (RCR-005), got {other:?}"),
    }
    assert_eq!(cluster.borrow().committed_count_of(&leader), before, "rebind committed nothing");
    let w = world_head(&cluster, &leader, &lw);
    let (who, what) = attribution_of(&w, &alice_hex).expect("alice's truth is addressable");
    assert_eq!((who, what.as_slice()), (alice.clone(), effect), "attribution UNCHANGED");

    // (3) RE-WRAP: bob proposes alice's effect BYTES under his own identity —
    // lawful, but it is a DIFFERENT truth attributed to BOB; alice's original
    // row is untouched. Attribution distinguishes the two forever.
    let rewrap = propose_attributed(&kernel, &w, &bob, effect).expect("bob's re-wrap commits");
    assert!(rewrap.fresh, "a re-wrap is NEW truth, never a silent takeover");
    assert_ne!(rewrap.truth, committed.truth);
    assert_replicas_identical(&cluster, &a);
    for node in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &node, &lw);
        let trail = attributed_effects(&w);
        assert_eq!(trail.len(), 2, "exactly two attributed truths exist on {node:?}");
        assert_eq!(trail.iter().filter(|(who, _, _)| *who == alice).count(), 1);
        assert_eq!(trail.iter().filter(|(who, _, _)| *who == bob).count(), 1);
        let (who, what) = attribution_of(&w, &alice_hex).expect("addressable");
        assert_eq!((who, what.as_slice()), (alice.clone(), effect), "alice's Who survives");
    }

    // DUPLICATE FLOOD: the same (agent, effect, capability) submitted 4×
    // through each of TWO racing schedulers, then re-proposed directly twice.
    // Exactly ONE fresh commit lands; the trail gains exactly ONE row.
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(Box::new(EchoActor { runs: runs.clone() }));
    bind_cap(&mut reg, &a, "cap.flood", &pid);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);
    let flood_effect: &[u8] = b"metric:flood";
    let world = world_head(&cluster, &leader, &lw);
    let mut s1 = ClusterScheduler::new(61, SchedulerConfig::default());
    let mut s2 = ClusterScheduler::new(62, SchedulerConfig::default());
    for t in 1..=4u64 {
        for sched in [&mut s1, &mut s2] {
            submit_attributed_effect(
                sched,
                t,
                &world,
                &alice,
                CapabilityId("cap.flood".into()),
                PolicyVerdict::Allow,
                flood_effect,
                &e,
            )
            .expect("flood submissions are lawful");
        }
    }
    for t in 10..30u64 {
        s1.dispatch_tick(t, &e);
        s2.dispatch_tick(t, &e);
    }
    assert!(s1.is_idle() && s2.is_idle());
    let fresh_across_both = [s1.decisions(), s2.decisions()]
        .iter()
        .flat_map(|d| d.iter())
        .filter(|d| matches!(d, SchedulingDecision::Committed { deduped: false, .. }))
        .count();
    assert_eq!(fresh_across_both, 1, "the flood landed exactly ONE fresh commit");
    // Direct re-proposals converge idempotently onto the same truth.
    let w = world_head(&cluster, &leader, &lw);
    let r1 = propose_attributed(&kernel, &w, &alice, flood_effect).expect("re-propose");
    let r2 = propose_attributed(&kernel, &w, &alice, flood_effect).expect("re-propose again");
    assert!(!r1.fresh && !r2.fresh, "direct duplicates resolve to existing truth");
    assert_eq!(r1.truth, r2.truth);
    assert_replicas_identical(&cluster, &a);
    for node in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &node, &lw);
        assert_eq!(
            attributed_effects(&w)
                .iter()
                .filter(|(_, what, _)| what.as_slice() == flood_effect)
                .count(),
            1,
            "the flood produced exactly ONE attributed truth on {node:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// (c) Partition during multi-agent work: minority fails honestly, heal
//     converges, no acked attributed truth is ever lost
// ---------------------------------------------------------------------------

/// Mid-run, the shard leader is partitioned into a minority of one. The
/// minority-side agent proposal FAILS HONESTLY (CP: no quorum ⇒ no truth,
/// `NotReplicated` — never a fork, never a silent success); the majority
/// elects a successor and the agents keep working (attributed effects AND the
/// decision flow both commit through the new leader). Heal ⇒ every replica
/// converges byte-identically; every ACKED attributed truth survives on all
/// replicas; the refused minority proposal is cleanly retriable and lands
/// EXACTLY ONCE (IDR-001/IDR-004; ORCH-004).
#[test]
fn partition_minority_proposals_fail_honestly_and_heal_loses_no_attributed_truth() {
    let a = sid("acme", "research");
    let (lw, kw) = (lshard("acme", "research"), kshard("acme", "research"));
    let cluster = cluster(3, &[a.clone()]);
    let old_leader = cluster.borrow().leader_of(&a).expect("elected");
    let a1 = register(&cluster, &old_leader, &kw, "dept-eng");
    let a2 = register(&cluster, &old_leader, &kw, "dept-ops");
    cluster.borrow_mut().settle(5); // registrations on every replica

    // PARTITION: the leader alone on the minority side.
    cluster.borrow_mut().isolate(&a, &old_leader);

    // Minority-side proposal: refused honestly, nothing committed.
    let minority_world = world_head(&cluster, &old_leader, &lw);
    let minority_kernel = leader_kernel(&cluster, &old_leader);
    let before = cluster.borrow().committed_count_of(&old_leader);
    let minority_effect: &[u8] = b"draft:minority-report";
    match propose_attributed(&minority_kernel, &minority_world, &a1, minority_effect) {
        Err(AgentError::Commit(CommitError::NotReplicated))
        | Err(AgentError::Commit(CommitError::NotLeader { .. })) => {}
        other => panic!("a minority-side proposal must fail honestly, got {other:?}"),
    }
    assert_eq!(
        cluster.borrow().committed_count_of(&old_leader),
        before,
        "the refused proposal committed NOTHING (no partial truth — IDR-004)"
    );

    // The MAJORITY elects a successor and multi-agent work continues.
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&a).expect("majority re-elected");
    assert_ne!(new_leader, old_leader, "leadership moved to the majority side");
    let kernel = leader_kernel(&cluster, &new_leader);
    let world = world_head(&cluster, &new_leader, &lw);
    let acked = propose_attributed(&kernel, &world, &a2, b"note:majority-work")
        .expect("majority-side agent work commits");
    assert!(acked.fresh);
    // The decision flow also proceeds on the majority (agents keep deciding).
    let store = cluster.borrow().wal_store_of(&new_leader);
    let basis = world_head(&cluster, &new_leader, &lw);
    let decided = propose_decision(&kernel, &store, &basis, &a2, "ops/failover-plan", "activate")
        .expect("flow completes on the majority");
    let decision = match decided {
        ProposalOutcome::Committed { decision, .. } => decision,
        other => panic!("expected commit, got {other:?}"),
    };

    // HEAL: the minority replica converges byte-identically; no acked truth
    // was lost anywhere; the decision derives identically on the healed node.
    cluster.borrow_mut().heal(&a);
    assert_replicas_identical(&cluster, &a);
    for node in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &node, &lw);
        let trail = attributed_effects(&w);
        assert_eq!(
            trail.iter().filter(|(who, what, _)| *who == a2 && what == b"note:majority-work").count(),
            1,
            "the ACKED attributed truth survives on {node:?}"
        );
        assert_eq!(decision_of(&w, "ops/failover-plan").expect("derived"), decision);
    }

    // The refused minority proposal is cleanly RETRIABLE: re-proposed through
    // the current leader it lands exactly once — no loss, no duplication.
    let world = world_head(&cluster, &new_leader, &lw);
    propose_attributed(&kernel, &world, &a1, minority_effect).expect("retry lands");
    assert_replicas_identical(&cluster, &a);
    for node in cluster.borrow().node_ids() {
        let w = world_head(&cluster, &node, &lw);
        assert_eq!(
            attributed_effects(&w)
                .iter()
                .filter(|(who, what, _)| *who == a1 && what.as_slice() == minority_effect)
                .count(),
            1,
            "the retried proposal exists EXACTLY ONCE on {node:?} (ORCH-004)"
        );
    }
}

// ---------------------------------------------------------------------------
// (d) Full-cluster deterministic replay INCLUDING attribution (ORCH-003)
// ---------------------------------------------------------------------------

/// A rich two-tenant multi-agent history (registrations, scheduler-borne and
/// direct attributed effects, policy + approval + blocked/admitted decisions,
/// a conflict race with its compliance events) is committed; then EVERY node
/// of the cluster is crash-recovered — rebuilt from its own durable WAL. The
/// rebuild reproduces, on every node: byte-identical per-shard truth,
/// an IDENTICAL attribution trail (who / what / commit offset), the identical
/// derived decision per subject, and the identical compliance ledger — the
/// ORCH-003 claim extended over attribution: the WAL is the multi-agent
/// decision trace, Who included. Tenants never blur (SHARD-001 after replay).
#[test]
fn full_cluster_replay_from_wal_rebuilds_identical_truth_and_attribution_trail() {
    let a = sid("acme", "research");
    let g = sid("globex", "research");
    let (lwa, kwa) = (lshard("acme", "research"), kshard("acme", "research"));
    let (lwg, kwg) = (lshard("globex", "research"), kshard("globex", "research"));
    let cluster = cluster(3, &[a.clone(), g.clone()]);
    let la = cluster.borrow().leader_of(&a).expect("acme elected");
    let lg = cluster.borrow().leader_of(&g).expect("globex elected");

    // --- Build the multi-agent history. ---
    let a1 = register(&cluster, &la, &kwa, "acme-analyst");
    let a2 = register(&cluster, &la, &kwa, "acme-approver");
    let g1 = register(&cluster, &lg, &kwg, "globex-worker");
    let ka = leader_kernel(&cluster, &la);
    let kg = leader_kernel(&cluster, &lg);

    // Scheduler-borne attributed effects in acme (incl. a collapsed duplicate).
    let world_a = world_head(&cluster, &la, &lwa);
    let mut reg = MemRegistry::new();
    let mut host = EngineHost::new();
    let runs = Rc::new(Cell::new(0));
    let pid = host.host(Box::new(EchoActor { runs: runs.clone() }));
    bind_cap(&mut reg, &a, "cap.work", &pid);
    let down = BTreeSet::new();
    let e = env(&cluster, &reg, &host, &down);
    let mut sched = ClusterScheduler::new(71, SchedulerConfig::default());
    for (t, effect) in [b"eff:report".as_slice(), b"eff:audit", b"eff:report"].iter().enumerate() {
        submit_attributed_effect(
            &mut sched,
            t as u64 + 1,
            &world_a,
            &a1,
            CapabilityId("cap.work".into()),
            PolicyVerdict::Allow,
            effect,
            &e,
        )
        .expect("admitted or deduplicated");
    }
    for t in 10..18u64 {
        sched.dispatch_tick(t, &e);
    }
    assert!(sched.is_idle());
    // A direct attributed effect in globex (the other tenant's trail).
    let world_g = world_head(&cluster, &lg, &lwg);
    propose_attributed(&kg, &world_g, &g1, b"eff:globex-only").expect("globex work commits");

    // Policy + approval + decisions in acme: one BLOCKED (compliance truth),
    // one admitted citing its approval, one CONFLICT race (loser recorded).
    let policy = PolicyRecord { name: "legal-review".into(), scope: "contract/".into(), version: 1 };
    commit_policy(&ka, &kwa, &policy).expect("policy committed");
    let store_la = cluster.borrow().wal_store_of(&la);
    let basis = world_head(&cluster, &la, &lwa);
    match propose_decision(&ka, &store_la, &basis, &a1, "contract/msa", "sign") {
        Ok(ProposalOutcome::Blocked { .. }) => {}
        other => panic!("unapproved decision must be blocked, got {other:?}"),
    }
    let basis = world_head(&cluster, &la, &lwa);
    commit_approval(&ka, &basis, &a2, "contract/msa").expect("peer approval commits");
    let basis = world_head(&cluster, &la, &lwa);
    let admitted = match propose_decision(&ka, &store_la, &basis, &a1, "contract/msa", "sign") {
        Ok(ProposalOutcome::Committed { decision, .. }) => decision,
        other => panic!("approved decision must commit, got {other:?}"),
    };
    // The scripted conflict race on a policy-free subject.
    let stale = world_head(&cluster, &la, &lwa);
    let winner = match propose_decision(&ka, &store_la, &stale, &a1, "plan/q4", "expand") {
        Ok(ProposalOutcome::Committed { decision, .. }) => decision,
        other => panic!("first proposal wins, got {other:?}"),
    };
    match propose_decision(&ka, &store_la, &stale, &a2, "plan/q4", "cut") {
        Ok(ProposalOutcome::Conflict { winner: w, .. }) => assert_eq!(w, winner),
        other => panic!("raced conflicting proposal must lose, got {other:?}"),
    }
    cluster.borrow_mut().settle(5);

    // --- Capture the multi-agent observable state per node, per shard. ---
    type Trail = Vec<(String, Vec<u8>, u64)>;
    let capture = |cluster: &Rc<RefCell<ClusterSim>>| -> Vec<(Vec<u8>, Vec<u8>, Trail, Trail, Vec<String>, usize, u64, u64)> {
        let c = cluster.borrow();
        c.node_ids()
            .iter()
            .map(|n| {
                let store = c.wal_store_of(n);
                let wa = WorldView::at_head(&store, &lwa).expect("acme world");
                let wg = WorldView::at_head(&store, &lwg).expect("globex world");
                let trail = |w: &WorldView| -> Trail {
                    attributed_effects(w)
                        .into_iter()
                        .map(|(who, what, at)| (who.hex(), what, at))
                        .collect()
                };
                let decisions: Vec<String> = ["contract/msa", "plan/q4"]
                    .iter()
                    .map(|s| decision_of(&wa, s).expect("derived").id_hex)
                    .collect();
                let compliance =
                    compliance_on(&wa, "contract/msa").len() + compliance_on(&wa, "plan/q4").len();
                (
                    c.shard_state_of(n, &a),
                    c.shard_state_of(n, &g),
                    trail(&wa),
                    trail(&wg),
                    decisions,
                    compliance,
                    wa.world_digest(),
                    wg.world_digest(),
                )
            })
            .collect()
    };
    let before = capture(&cluster);
    // Replica agreement BEFORE the crash (the reference is meaningful).
    assert!(before.windows(2).all(|w| w[0] == w[1]), "replicas agree before the crash");
    let (_, _, trail_a, trail_g, decisions, compliance, _, _) = &before[0];
    assert_eq!(trail_a.len(), 2, "acme: 2 attributed effects (duplicate collapsed)");
    assert_eq!(trail_g.len(), 1, "globex: 1 attributed effect");
    assert!(trail_a.iter().all(|(who, _, _)| *who == a1.hex()));
    assert!(trail_g.iter().all(|(who, _, _)| *who == g1.hex()));
    assert_eq!(decisions[0], admitted.id_hex);
    assert_eq!(decisions[1], winner.id_hex);
    assert_eq!(*compliance, 2, "one Blocked + one Conflict event recorded as truth");

    // --- CRASH-RECOVER every node: rebuild from each node's own WAL. ---
    {
        let mut c = cluster.borrow_mut();
        for n in c.node_ids() {
            c.crash_recover(&n);
        }
    }
    let after = capture(&cluster);
    assert_eq!(
        after, before,
        "full-cluster rebuild from the WAL reproduces IDENTICAL truth AND an identical \
         attribution trail / decision derivation / compliance ledger on every node (ORCH-003)"
    );
    // SHARD-001 after replay: the tenants' attribution trails never blur.
    for (_, _, trail_a, trail_g, _, _, _, _) in &after {
        assert!(trail_a.iter().all(|(who, _, _)| *who != g1.hex()));
        assert!(trail_g.iter().all(|(who, _, _)| *who != a1.hex()));
    }
}

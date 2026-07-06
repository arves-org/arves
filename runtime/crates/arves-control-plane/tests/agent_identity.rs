//! RCR-029 (I5 Stage 1) — executable proofs for AGENT IDENTITY + ATTRIBUTION.
//!
//! Design obligations discharged (docs/design/I5_MultiAgent_Runtime_Design.md
//! §3.1.1 identity/registry, §3.10 registry-recovers-with-truth, §3.19 audit
//! Who/When/What, §4 SHARD-001 "agent identity is shard-bound for life"):
//! registration is idempotent content-addressed truth; identities are
//! addressable from the LCW shared world on EVERY replica; every attributed
//! effect carries its agent identity inside the committed truth trail and
//! round-trips back out; the structural gate refuses unregistered identities;
//! and the v1.x honest limit (attribution is NOT cryptographic authN — v2.0
//! debt #8 / OQ-1) is PINNED by test, not hidden.
//!
//! Every test is deterministic: fixed seeds, logical ticks, scripted
//! elections — zero wall clocks, zero OS randomness, zero sleeps. All agents
//! are deterministic test actors, NOT AI models.

use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_control_plane::agents::{
    attributed_effects, attribution_of, decode_definition, find_agent, is_registered,
    propose_attributed, register_agent, AgentDefinition, AgentError, AgentId,
};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{ContentHash, Kernel, ProposedWrite, RefKernel, ShardKey as KShardKey};
use arves_lcw::world::WorldView;
use arves_lcw::ShardKey as LcwShardKey;
use arves_persistence::MemWalStore;
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Deterministic scaffolding
// ---------------------------------------------------------------------------

fn kshard(t: &str, w: &str) -> KShardKey {
    KShardKey::new(t, w).expect("well-formed test shard")
}

fn lshard(t: &str, w: &str) -> LcwShardKey {
    LcwShardKey { tenant: t.into(), workspace: w.into() }
}

fn worker(name: &str, owner: &str) -> AgentDefinition {
    AgentDefinition {
        name: name.into(),
        agent_type: "Worker".into(),
        owner: owner.into(),
        purpose: "deterministic test actor (NOT an AI model)".into(),
        definition_version: 1,
    }
}

/// 3-node cluster with one elected shard group; returns (sim, leader).
fn cluster(shard: &ShardId, seed: u64) -> (Rc<RefCell<ClusterSim>>, NodeId) {
    let sim = Rc::new(RefCell::new(ClusterSim::new(3)));
    sim.borrow_mut().add_shard(shard.clone(), seed); // fixed seed: replayable
    let leader = sim.borrow_mut().elect(shard);
    (sim, leader)
}

// ---------------------------------------------------------------------------
// (a) Identity: content-addressed truth, idempotent registration
// ---------------------------------------------------------------------------

#[test]
fn registration_commits_content_addressed_truth_and_is_idempotent_orch004() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let def = worker("ledger-worker", "ops@acme");

    // Register: the identity IS the content address of the canonical body.
    let reg = register_agent(&kernel, &ks, &def).unwrap();
    assert!(reg.fresh);
    assert_eq!(reg.id, AgentId::of(&ks, &def));
    assert_eq!(reg.truth.content.0, reg.id.bytes().to_vec());

    // Idempotent re-registration: SAME truth, never a fork (ORCH-004).
    let again = register_agent(&kernel, &ks, &def).unwrap();
    assert!(!again.fresh);
    assert_eq!(again.truth, reg.truth);
    assert_eq!(kernel.committed_count(), 1, "one registration, one truth");

    // A NEW definition version is a NEW identity (versioned registry,
    // Vol 14 Part 20) — a second, distinct truth.
    let mut v2 = def.clone();
    v2.definition_version = 2;
    let reg2 = register_agent(&kernel, &ks, &v2).unwrap();
    assert!(reg2.fresh);
    assert_ne!(reg2.id, reg.id);
    assert_eq!(kernel.committed_count(), 2);

    // Governance minima refuse BEFORE the gateway: no owner, no agent.
    let mut ownerless = def.clone();
    ownerless.owner.clear();
    assert!(matches!(
        register_agent(&kernel, &ks, &ownerless),
        Err(AgentError::InvalidDefinition(_))
    ));
    assert_eq!(kernel.committed_count(), 2, "refusal committed nothing");
}

// ---------------------------------------------------------------------------
// (b) Addressability from the shared world, on every replica
// ---------------------------------------------------------------------------

#[test]
fn registered_identity_is_addressable_from_the_shared_world_on_every_replica() {
    let sid = ShardId::new(TenantId("acme".into()), WorkspaceId("research".into()));
    let (sim, leader) = cluster(&sid, 7);
    let gateway = ClusterKernel::new(leader, sim.clone());
    let ks = kshard("acme", "research");
    let def = worker("ledger-worker", "ops@acme");

    // Register through the LEADER's frozen gateway (quorum-acked truth).
    let reg = register_agent(&gateway, &ks, &def).unwrap();
    sim.borrow_mut().settle(50);

    // The identity is addressable from EVERY replica's shared world — same
    // registration truth, same decoded definition (the registry recovers with
    // the truth base; it never lives in orchestrator memory, design §3.10).
    let ls = lshard("acme", "research");
    for node in sim.borrow().node_ids() {
        let store = sim.borrow().wal_store_of(&node);
        let world = WorldView::at_head(&store, &ls).unwrap();
        assert!(is_registered(&world, &reg.id), "replica {node:?} must address the identity");
        assert_eq!(find_agent(&world, &reg.id), Some(def.clone()));
        // The raw payload decodes to (tenant, workspace, def) — shard-bound.
        let (payload, at) = world.get(&reg.id.hex()).unwrap();
        assert_eq!(at, reg.truth.index.0, "registration sits at its commit index");
        assert_eq!(
            decode_definition(payload),
            Some(("acme".into(), "research".into(), def.clone()))
        );
    }
}

#[test]
fn identity_is_shard_bound_for_life_shard001() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let shard_a = kshard("acme", "research");
    let shard_b = kshard("globex", "research");
    let def = worker("ledger-worker", "ops");

    // The same definition registered in two shards is TWO identities.
    let reg_a = register_agent(&kernel, &shard_a, &def).unwrap();
    let reg_b = register_agent(&kernel, &shard_b, &def).unwrap();
    assert_ne!(reg_a.id, reg_b.id, "identity is shard-bound (SHARD-001)");

    // Each identity is addressable ONLY through its own shard's world.
    let world_a = WorldView::at_head(&store, &lshard("acme", "research")).unwrap();
    let world_b = WorldView::at_head(&store, &lshard("globex", "research")).unwrap();
    assert!(is_registered(&world_a, &reg_a.id) && !is_registered(&world_a, &reg_b.id));
    assert!(is_registered(&world_b, &reg_b.id) && !is_registered(&world_b, &reg_a.id));

    // Attribution across the boundary is refused: shard A's world never
    // authorizes shard B's identity (no cross-tenant identity exists).
    assert_eq!(
        propose_attributed(&kernel, &world_a, &reg_b.id, b"cross-shard-attempt").unwrap_err(),
        AgentError::NotRegistered { agent: reg_b.id.hex() }
    );
}

#[test]
fn smuggled_foreign_shard_definition_is_refused_shard001() {
    // RCR-029 amendment A1 pin (adversarial finding): the frozen
    // `Kernel::commit` does NOT verify content == ACS-hash(payload) — RCR-005
    // admission only rejects same-address/different-payload forks — so a
    // caller can lawfully commit a shard-B-BODIED canonical definition INTO
    // shard A's WAL under shard B's identity as the content address. The
    // structural gate must still refuse: an identity is addressable ONLY
    // through its own shard's world (SHARD-001).
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let shard_a = kshard("acme", "research");
    let shard_b = kshard("globex", "research");
    let def = worker("ledger-worker", "ops");
    let id_b = AgentId::of(&shard_b, &def);

    // The smuggle: shard B's canonical body, committed into shard A's trace
    // through the lawful public gateway. This commit SUCCEEDS (v1.x honest
    // limit of the gateway) — the gate below is what refuses the identity.
    kernel
        .commit(ProposedWrite {
            shard: shard_a.clone(),
            content: ContentHash(id_b.bytes().to_vec()),
            payload: def.canonical_bytes(&shard_b),
        })
        .expect("the frozen gateway admits the smuggled record");

    // The decoded shard (globex) is not this world's shard (acme): refused.
    let world_a = WorldView::at_head(&store, &lshard("acme", "research")).unwrap();
    assert!(
        !is_registered(&world_a, &id_b),
        "a shard-B-bodied record in shard A's WAL must never register B's identity there"
    );
    assert_eq!(find_agent(&world_a, &id_b), None);

    // And attribution in shard A to the shard-B-bound identity stays refused,
    // committing nothing beyond the smuggled record itself.
    assert_eq!(
        propose_attributed(&kernel, &world_a, &id_b, b"cross-shard-smuggle").unwrap_err(),
        AgentError::NotRegistered { agent: id_b.hex() }
    );
    assert_eq!(kernel.committed_count(), 1, "refusal committed nothing");
}

// ---------------------------------------------------------------------------
// (c) Attribution round-trip: the Who inside the committed What
// ---------------------------------------------------------------------------

#[test]
fn attribution_round_trips_through_the_committed_truth_trail_on_every_replica() {
    let sid = ShardId::new(TenantId("acme".into()), WorkspaceId("research".into()));
    let (sim, leader) = cluster(&sid, 11);
    let gateway = ClusterKernel::new(leader, sim.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");

    // Two registered agents (deterministic test actors) propose effects.
    let alice = register_agent(&gateway, &ks, &worker("alice", "ops@acme")).unwrap();
    let bob = register_agent(&gateway, &ks, &worker("bob", "ops@acme")).unwrap();
    sim.borrow_mut().settle(50);

    let world = {
        let store = sim.borrow().wal_store_of(&sim.borrow().node_ids()[0]);
        WorldView::at_head(&store, &ls).unwrap()
    };
    let e1 = propose_attributed(&gateway, &world, &alice.id, b"decision: reorder stock").unwrap();
    let e2 = propose_attributed(&gateway, &world, &bob.id, b"decision: audit ledger").unwrap();
    assert!(e1.fresh && e2.fresh);
    sim.borrow_mut().settle(50);

    // Round-trip on EVERY replica: the truth trail itself carries Who —
    // read the effect back by its content id and decode the identity out.
    for node in sim.borrow().node_ids() {
        let store = sim.borrow().wal_store_of(&node);
        let world_n = WorldView::at_head(&store, &ls).unwrap();
        let id1_hex = arves_acs::hex(&e1.truth.content.0);
        let id2_hex = arves_acs::hex(&e2.truth.content.0);
        assert_eq!(
            attribution_of(&world_n, &id1_hex),
            Some((alice.id.clone(), b"decision: reorder stock".to_vec()))
        );
        assert_eq!(
            attribution_of(&world_n, &id2_hex),
            Some((bob.id.clone(), b"decision: audit ledger".to_vec()))
        );
        // The audit walk (design §3.19): commit-ordered (Who, What, When).
        let trail = attributed_effects(&world_n);
        assert_eq!(trail.len(), 2, "exactly the two attributed effects");
        assert_eq!(trail[0].0, alice.id);
        assert_eq!(trail[1].0, bob.id);
        assert_eq!(trail[0].2, e1.truth.index.0);
        assert_eq!(trail[1].2, e2.truth.index.0);
        // Registrations are visible but are NOT attributed effects (the
        // decoder never confuses the two envelopes).
        assert_eq!(world_n.len(), 4, "2 registrations + 2 effects");
    }
}

#[test]
fn duplicate_attributed_effects_converge_to_one_truth_orch004() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");
    let alice = register_agent(&kernel, &ks, &worker("alice", "ops")).unwrap();
    let world = WorldView::at_head(&store, &ls).unwrap();

    // The same (agent, effect) proposed twice — e.g. a retry storm across
    // racing schedulers — lands EXACTLY ONE truth (content addressing).
    let first = propose_attributed(&kernel, &world, &alice.id, b"effect-x").unwrap();
    let second = propose_attributed(&kernel, &world, &alice.id, b"effect-x").unwrap();
    assert!(first.fresh);
    assert!(!second.fresh, "duplicate collapsed, never forked");
    assert_eq!(first.truth, second.truth);
    assert_eq!(kernel.committed_count(), 2, "1 registration + 1 effect");

    // The SAME effect bytes attributed to a DIFFERENT agent are a DIFFERENT
    // truth: the Who is inside the addressed content, so attribution can
    // never be silently rewritten (the trail is append-only, IDR-005).
    let bob = register_agent(&kernel, &ks, &worker("bob", "ops")).unwrap();
    let world2 = WorldView::at_head(&store, &ls).unwrap();
    let third = propose_attributed(&kernel, &world2, &bob.id, b"effect-x").unwrap();
    assert!(third.fresh);
    assert_ne!(third.truth.content, first.truth.content);
}

// ---------------------------------------------------------------------------
// (d) The structural gate — and its honest v1.x limit, pinned
// ---------------------------------------------------------------------------

#[test]
fn unregistered_identity_is_refused_and_the_gate_reads_committed_truth_only() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");

    // Bootstrap the shard's trace so a world view exists (one governed agent).
    let seed = register_agent(&kernel, &ks, &worker("seed", "ops")).unwrap();
    let world_v1 = WorldView::at_head(&store, &ls).unwrap();

    // A never-registered identity is refused (structural gate) and commits
    // NOTHING — a refusal is not truth.
    let ghost = AgentId::of(&ks, &worker("ghost", "nobody"));
    assert_eq!(
        propose_attributed(&kernel, &world_v1, &ghost, b"effect").unwrap_err(),
        AgentError::NotRegistered { agent: ghost.hex() }
    );
    assert_eq!(kernel.committed_count(), 1);

    // The gate reads COMMITTED truth at the caller's declared basis, not
    // orchestrator memory: registering ghost AFTER the view was taken does
    // not change the old view's verdict; a REFRESHED view admits it. (This is
    // the runtime-grade check the design elevates from G1's in-process maps.)
    let ghost_reg = register_agent(&kernel, &ks, &worker("ghost", "nobody")).unwrap();
    assert_eq!(ghost_reg.id, ghost);
    assert!(!is_registered(&world_v1, &ghost), "stale view: stays refused");
    let world_v2 = WorldView::at_head(&store, &ls).unwrap();
    assert!(is_registered(&world_v2, &ghost));
    assert!(propose_attributed(&kernel, &world_v2, &ghost, b"effect").unwrap().fresh);

    // And the seed agent still resolves — the gate is per-identity.
    assert!(is_registered(&world_v2, &seed.id));
}

#[test]
fn attribution_is_structural_not_cryptographic_v1x_limit_pinned() {
    // HONEST-LIMIT PIN (design §3.16 / NON-GOAL 4 / RUNTIME_FREEZE v2.0 debt
    // #8 / OQ-1): v1.x has no principal on Kernel::commit, so NOTHING binds
    // the CALLER to the identity it attributes to. This test EXISTS to keep
    // that limit loud: one caller lawfully wears two registered identities.
    // The v2.0 authenticated-commit RCR is the instrument that will flip this
    // test's meaning — until then, deployments beyond a trusted host are
    // unsafe and I5 must keep saying so.
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");
    let alice = register_agent(&kernel, &ks, &worker("alice", "ops")).unwrap();
    let bob = register_agent(&kernel, &ks, &worker("bob", "ops")).unwrap();
    let world = WorldView::at_head(&store, &ls).unwrap();

    // The SAME caller attributes to alice AND to bob — both succeed. That is
    // the trusted-single-host model, stated executably.
    assert!(propose_attributed(&kernel, &world, &alice.id, b"as-alice").unwrap().fresh);
    assert!(propose_attributed(&kernel, &world, &bob.id, b"as-bob").unwrap().fresh);

    // What v1.x DOES guarantee even so: VIA `propose_attributed`, the worn
    // identity must be REGISTERED truth — no fabricated Who enters the trail
    // THROUGH THAT PATH. (Scoped honestly, RCR-029 amendment A2: a hand-
    // crafted envelope committed directly through the raw frozen gateway can
    // still carry an arbitrary claimed Who, and the readers report the
    // CLAIMED Who without a registration cross-check.)...
    let fake = AgentId::of(&ks, &worker("fabricated", "x"));
    assert!(matches!(
        propose_attributed(&kernel, &world, &fake, b"as-nobody"),
        Err(AgentError::NotRegistered { .. })
    ));
    // ...and the trail records the CLAIMED Who immutably and readably.
    let world2 = WorldView::at_head(&store, &ls).unwrap();
    let trail = attributed_effects(&world2);
    assert_eq!(trail.len(), 2);
    assert_eq!((&trail[0].0, trail[0].1.as_slice()), (&alice.id, b"as-alice".as_slice()));
    assert_eq!((&trail[1].0, trail[1].1.as_slice()), (&bob.id, b"as-bob".as_slice()));
}

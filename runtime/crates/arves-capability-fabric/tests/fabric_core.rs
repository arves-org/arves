//! RCR-026 (I4 Stage 1) — executable proofs for the CAPABILITY FABRIC CORE:
//! binding lifecycle (register → bind → rebind → revoke) with append-only
//! supersession, authoritative resolve (a stale binding is NEVER served after a
//! rebind; revocation BITES), pinned replay resolution, and the authorization gate
//! (unbound hard-deny, engine-identity binding, Capabilities-Required honouring,
//! enforced-not-owned policy, EffectClass validation, RCR-012 wiring).
//!
//! Every test is deterministic: fixed inputs, zero clocks, zero OS randomness,
//! zero sleeps. Design: `docs/design/I4_Capability_Scheduling_Design.md`
//! (§3.1.1/§3.1.2/§3.1.4, §3.5 steps 1/3/10, §3.8, §3.9 F-UNBOUND/F-POLICY).
//! Registered invariants exercised: OWN-001, LAYER-001 (edge checked by the
//! architecture gate), SHARD-001, ORCH-001 (structural: this crate has no kernel
//! path), ORCH-004 (via the RCR-012 key enforcement). CAP-00n are cited ONLY as
//! (PROPOSED — CCP-GATE required) and carry no conformance weight.

use std::cell::Cell;

use arves_capability_fabric::gate::{
    authorize, engine_provider_id, invoke_gated, GateDenial, PolicyVerdict,
};
use arves_capability_fabric::lifecycle::{LifecycleEvent, LifecycleRegistry};
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, ProviderId, RegistryError, ShardKey,
};
use arves_engine_fabric::{
    invocation_key, Determinism, Engine, EngineManifest, FabricViolation, IdempotencyKey,
    Inference, ProposedEffect,
};

// ---------------------------------------------------------------------------
// Deterministic fixtures
// ---------------------------------------------------------------------------

fn shard() -> ShardKey {
    ShardKey::new("t1", "w1").expect("valid test shard")
}

fn cap(name: &str) -> CapabilityId {
    CapabilityId(name.to_string())
}

fn binding_for(
    shard: &ShardKey,
    capability: &str,
    version: u64,
    provider: &str,
    effect: EffectClass,
) -> CapabilityBinding {
    CapabilityBinding {
        capability: cap(capability),
        shard: shard.clone(),
        version: BindingVersion(version),
        provider: ProviderId(provider.to_string()),
        contract: InvocationContract {
            input_schema: "acs:uci.fact".into(),
            output_schema: "acs:uci.fact".into(),
            effect,
        },
    }
}

/// A deterministic test engine with an invocation counter (Cell — single-threaded,
/// the RCR-012 test pattern), a configurable required-capability set, and a switch
/// for whether it proposes an effect. Conformant: returns the fabric-derived
/// ORCH-004 key.
struct CountingEngine {
    name: String,
    requires: Vec<String>,
    propose: bool,
    calls: Cell<u32>,
}

impl CountingEngine {
    fn new(name: &str, requires: &[&str], propose: bool) -> Self {
        Self {
            name: name.to_string(),
            requires: requires.iter().map(|s| s.to_string()).collect(),
            propose,
            calls: Cell::new(0),
        }
    }
}

impl Engine for CountingEngine {
    type Input = Vec<u8>;

    fn manifest(&self) -> EngineManifest {
        EngineManifest {
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            determinism: Determinism::Deterministic,
            idempotency_key: IdempotencyKey("acs-002/1".to_string()),
            reads: Vec::new(),
            produces: vec!["uci.fact".to_string()],
            capabilities_required: self.requires.clone(),
        }
    }

    fn invoke(&self, input: Vec<u8>) -> Inference {
        self.calls.set(self.calls.get() + 1);
        let key = invocation_key(&self.manifest(), &input);
        let proposed_effects = if self.propose {
            vec![ProposedEffect { target: "uci.fact".to_string(), payload: input.clone() }]
        } else {
            Vec::new()
        };
        Inference { key, output: input, proposed_effects }
    }
}

/// The provider id string the gate derives for a `CountingEngine` named `name`.
fn provider_of(name: &str) -> String {
    format!("engine:{name}@1.0.0")
}

// ===========================================================================
// LIFECYCLE (design §3.5 steps 1/10)
// ===========================================================================

// Parity with the frozen contract: declare-before-bind, bind, authoritative
// resolve — the LifecycleRegistry honours the exact MemRegistry semantics.
#[test]
fn lifecycle_register_bind_resolve_and_declare_before_bind() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    // Bind before declare is refused (frozen contract).
    assert!(matches!(
        r.bind(binding_for(&s, "derive.fact", 1, &provider_of("derive.fact"), EffectClass::ProposesWrite)),
        Err(RegistryError::UndeclaredCapability(_))
    ));
    r.register(&s, cap("derive.fact")).unwrap();
    r.bind(binding_for(&s, "derive.fact", 1, &provider_of("derive.fact"), EffectClass::ProposesWrite))
        .unwrap();
    let got = r.resolve(&s, &cap("derive.fact")).unwrap();
    assert_eq!(got.provider, ProviderId(provider_of("derive.fact")));
    assert_eq!(got.version, BindingVersion(1));
    // Unbound capability reports Unbound.
    assert!(matches!(r.resolve(&s, &cap("nope")), Err(RegistryError::Unbound { .. })));
}

// RESOLVE AUTHORITY: after a rebind, the stale binding is NEVER served again by
// `resolve` — but stays readable via the PINNED replay surface (ORCH-003 basis:
// replay uses the binding that actually ran; CAP-009 (PROPOSED)). Supersession is
// strictly monotonic and append-only.
#[test]
fn rebind_supersedes_stale_binding_never_served_again() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    r.register(&s, cap("derive.fact")).unwrap();
    r.bind(binding_for(&s, "derive.fact", 1, "engine:a@1.0.0", EffectClass::ProposesWrite)).unwrap();
    r.bind(binding_for(&s, "derive.fact", 2, "engine:b@1.0.0", EffectClass::ProposesWrite)).unwrap();

    // Authoritative resolve serves ONLY the latest version — repeatedly (idempotent read).
    for _ in 0..3 {
        let active = r.resolve(&s, &cap("derive.fact")).unwrap();
        assert_eq!(active.version, BindingVersion(2));
        assert_eq!(active.provider, ProviderId("engine:b@1.0.0".into()));
    }

    // Non-monotonic rebinds are refused with the exact current/offered pair.
    assert_eq!(
        r.bind(binding_for(&s, "derive.fact", 2, "engine:c@1.0.0", EffectClass::ProposesWrite)),
        Err(RegistryError::NonMonotonicVersion { current: BindingVersion(2), offered: BindingVersion(2) })
    );
    assert!(matches!(
        r.bind(binding_for(&s, "derive.fact", 1, "engine:c@1.0.0", EffectClass::ProposesWrite)),
        Err(RegistryError::NonMonotonicVersion { .. })
    ));

    // The superseded v1 stays readable for replay — pinned, never active.
    let pinned = r.resolve_pinned(&s, &cap("derive.fact"), BindingVersion(1)).unwrap();
    assert_eq!(pinned.provider, ProviderId("engine:a@1.0.0".into()));
    // Unknown pinned version is Unbound.
    assert!(matches!(
        r.resolve_pinned(&s, &cap("derive.fact"), BindingVersion(9)),
        Err(RegistryError::Unbound { .. })
    ));

    // History is the full append-only chain, in order.
    let h = r.history(&s, &cap("derive.fact"));
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].version(), BindingVersion(1));
    assert_eq!(h[1].version(), BindingVersion(2));
    assert!(matches!(&h[0], LifecycleEvent::Bound(b) if b.provider.0 == "engine:a@1.0.0"));
}

// REVOCATION BITES: a revoke is an appended tombstone at a strictly higher version
// (never a deletion, RCR-026 DR-3); resolve is a hard Unbound afterwards; a stale
// (pre-revoke) version can never rebind; only a strictly-superseding rebind
// reactivates. Errors reuse the frozen RegistryError vocabulary.
#[test]
fn revocation_bites_and_is_append_only() {
    let s = shard();
    let mut r = LifecycleRegistry::new();

    // Revoking an undeclared capability is UndeclaredCapability.
    assert!(matches!(
        r.revoke(&s, &cap("ghost"), BindingVersion(1)),
        Err(RegistryError::UndeclaredCapability(_))
    ));
    // Declared but never bound: nothing active to revoke -> Unbound.
    r.register(&s, cap("derive.fact")).unwrap();
    assert!(matches!(
        r.revoke(&s, &cap("derive.fact"), BindingVersion(1)),
        Err(RegistryError::Unbound { .. })
    ));

    r.bind(binding_for(&s, "derive.fact", 1, "engine:a@1.0.0", EffectClass::ProposesWrite)).unwrap();

    // A non-superseding revoke version is refused (well-ordered supersession).
    assert_eq!(
        r.revoke(&s, &cap("derive.fact"), BindingVersion(1)),
        Err(RegistryError::NonMonotonicVersion { current: BindingVersion(1), offered: BindingVersion(1) })
    );

    // Revoke at v5: resolve is now a hard Unbound (the gate's F-UNBOUND surface).
    r.revoke(&s, &cap("derive.fact"), BindingVersion(5)).unwrap();
    assert!(matches!(r.resolve(&s, &cap("derive.fact")), Err(RegistryError::Unbound { .. })));

    // Double revoke: nothing active -> Unbound (the chain stays meaningful).
    assert!(matches!(
        r.revoke(&s, &cap("derive.fact"), BindingVersion(6)),
        Err(RegistryError::Unbound { .. })
    ));

    // Replaying an old version cannot resurrect the capability: rebind must
    // strictly supersede the TOMBSTONE, not the last bound version.
    assert_eq!(
        r.bind(binding_for(&s, "derive.fact", 4, "engine:a@1.0.0", EffectClass::ProposesWrite)),
        Err(RegistryError::NonMonotonicVersion { current: BindingVersion(5), offered: BindingVersion(4) })
    );
    assert!(matches!(
        r.bind(binding_for(&s, "derive.fact", 5, "engine:a@1.0.0", EffectClass::ProposesWrite)),
        Err(RegistryError::NonMonotonicVersion { .. })
    ));
    r.bind(binding_for(&s, "derive.fact", 6, "engine:a2@1.0.0", EffectClass::ProposesWrite)).unwrap();
    assert_eq!(r.resolve(&s, &cap("derive.fact")).unwrap().version, BindingVersion(6));

    // The revoked-era binding stays readable for replay (pinned), the tombstone
    // version itself is not a binding, and the chain is the full ordered record.
    assert_eq!(
        r.resolve_pinned(&s, &cap("derive.fact"), BindingVersion(1)).unwrap().provider,
        ProviderId("engine:a@1.0.0".into())
    );
    assert!(matches!(
        r.resolve_pinned(&s, &cap("derive.fact"), BindingVersion(5)),
        Err(RegistryError::Unbound { .. })
    ));
    let h = r.history(&s, &cap("derive.fact"));
    assert_eq!(
        h.iter().map(LifecycleEvent::version).collect::<Vec<_>>(),
        vec![BindingVersion(1), BindingVersion(5), BindingVersion(6)]
    );
    assert!(matches!(h[1], LifecycleEvent::Revoked { version: BindingVersion(5) }));
}

// SHARD-001: lifecycle state is partitioned by the immutable shard key — versions,
// revocations and resolves in one shard never leak into another.
#[test]
fn shard001_lifecycle_isolated_per_shard() {
    let sa = ShardKey::new("tenant-a", "w1").unwrap();
    let sb = ShardKey::new("tenant-b", "w1").unwrap();
    let mut r = LifecycleRegistry::new();
    r.register(&sa, cap("derive.fact")).unwrap();
    r.register(&sb, cap("derive.fact")).unwrap();
    r.bind(binding_for(&sa, "derive.fact", 7, "engine:a@1.0.0", EffectClass::ProposesWrite)).unwrap();
    r.bind(binding_for(&sb, "derive.fact", 1, "engine:b@1.0.0", EffectClass::ProposesWrite)).unwrap();

    // Independent version spaces (v1 in B is fine even though A is at v7).
    assert_eq!(r.resolve(&sa, &cap("derive.fact")).unwrap().provider.0, "engine:a@1.0.0");
    assert_eq!(r.resolve(&sb, &cap("derive.fact")).unwrap().provider.0, "engine:b@1.0.0");

    // Revocation in A does not touch B.
    r.revoke(&sa, &cap("derive.fact"), BindingVersion(8)).unwrap();
    assert!(matches!(r.resolve(&sa, &cap("derive.fact")), Err(RegistryError::Unbound { .. })));
    assert_eq!(r.resolve(&sb, &cap("derive.fact")).unwrap().version, BindingVersion(1));

    // A shard with no lifecycle at all resolves Unbound and has an empty history.
    let sc = ShardKey::new("tenant-c", "w1").unwrap();
    assert!(matches!(r.resolve(&sc, &cap("derive.fact")), Err(RegistryError::Unbound { .. })));
    assert!(r.history(&sc, &cap("derive.fact")).is_empty());
}

// DETERMINISM: registry state is a pure function of the recorded operation
// sequence — two independently-scripted identical runs produce EQUAL registries
// (derive(PartialEq) over the full chains) and equal resolve/history answers.
#[test]
fn lifecycle_is_deterministic_pure_function_of_recorded_ops() {
    let s = shard();
    let script = |r: &mut LifecycleRegistry| {
        r.register(&s, cap("derive.fact")).unwrap();
        r.register(&s, cap("net.fetch")).unwrap();
        r.bind(binding_for(&s, "derive.fact", 1, "engine:a@1.0.0", EffectClass::ProposesWrite)).unwrap();
        r.bind(binding_for(&s, "net.fetch", 1, "engine:f@1.0.0", EffectClass::IdempotentEffect)).unwrap();
        r.bind(binding_for(&s, "derive.fact", 2, "engine:b@1.0.0", EffectClass::ProposesWrite)).unwrap();
        r.revoke(&s, &cap("net.fetch"), BindingVersion(3)).unwrap();
        // Refused operations must not perturb state either.
        let _ = r.bind(binding_for(&s, "derive.fact", 2, "engine:x@1.0.0", EffectClass::Pure));
        let _ = r.revoke(&s, &cap("net.fetch"), BindingVersion(9));
    };
    let (mut r1, mut r2) = (LifecycleRegistry::new(), LifecycleRegistry::new());
    script(&mut r1);
    script(&mut r2);
    assert_eq!(r1, r2, "identical scripts -> bit-identical lifecycle state");
    assert_eq!(
        r1.resolve(&s, &cap("derive.fact")).unwrap(),
        r2.resolve(&s, &cap("derive.fact")).unwrap()
    );
    assert_eq!(r1.history(&s, &cap("net.fetch")), r2.history(&s, &cap("net.fetch")));
}

// ===========================================================================
// AUTHORIZATION GATE (design §3.1.1/§3.1.2; F-UNBOUND / F-POLICY)
// ===========================================================================

// F-UNBOUND is a hard deny, and it denies BEFORE the engine is ever invoked.
#[test]
fn gate_unbound_capability_is_hard_denied_before_invocation() {
    let s = shard();
    let r = LifecycleRegistry::new();
    let e = CountingEngine::new("derive.fact", &[], true);
    let denial = invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
        .expect_err("unbound must deny");
    assert!(matches!(denial, GateDenial::Unbound { .. }));
    assert_eq!(e.calls.get(), 0, "denied invocation never reaches the engine");
}

// The gate binds to engine IDENTITY (engine:{name}@{version}), not the capability
// name: an impostor engine presenting under an authorized capability is refused —
// the exact semantic the bridge exercises, formalized fabric-side (CAP-002-style,
// (PROPOSED — CCP-GATE required); registered basis Vol 9 Part 3).
#[test]
fn gate_binds_to_engine_identity_not_capability_name() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    r.register(&s, cap("derive.fact")).unwrap();
    r.bind(binding_for(&s, "derive.fact", 1, &provider_of("derive.fact"), EffectClass::ProposesWrite))
        .unwrap();

    let impostor = CountingEngine::new("evil.engine", &[], true);
    let denial =
        invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &impostor, b"x".to_vec())
            .expect_err("impostor must be refused");
    match denial {
        GateDenial::ProviderMismatch { expected, bound } => {
            assert_eq!(expected, engine_provider_id(&impostor.manifest()));
            assert_eq!(bound.0, provider_of("derive.fact"));
        }
        other => panic!("expected ProviderMismatch, got {other:?}"),
    }
    assert_eq!(impostor.calls.get(), 0);

    // The rightful engine passes.
    let rightful = CountingEngine::new("derive.fact", &[], true);
    invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &rightful, b"x".to_vec())
        .expect("bound identity passes the gate");
}

// Capabilities Required are honoured VIA the fabric (Engine Graph Parts 3/10;
// design §3.1.1): every declared requirement must have an active binding in the
// SAME shard — an unbound requirement denies, a revoked requirement denies again.
#[test]
fn gate_requires_every_declared_capability_bound_in_shard() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    r.register(&s, cap("derive.fact")).unwrap();
    r.bind(binding_for(&s, "derive.fact", 1, &provider_of("derive.fact"), EffectClass::ProposesWrite))
        .unwrap();

    let e = CountingEngine::new("derive.fact", &["net.fetch"], true);
    // Declared requirement with no active binding -> deny, naming the requirement.
    assert_eq!(
        invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
            .expect_err("unbound requirement must deny"),
        GateDenial::RequiredCapabilityUnbound { required: "net.fetch".to_string() }
    );
    assert_eq!(e.calls.get(), 0);

    // Bind the requirement (existence in the SAME shard is what is checked, DR-9).
    r.register(&s, cap("net.fetch")).unwrap();
    r.bind(binding_for(&s, "net.fetch", 1, "provider:http@1", EffectClass::IdempotentEffect)).unwrap();
    invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
        .expect("all requirements bound -> pass");

    // Revoking the requirement makes the SAME invocation deny again (revocation
    // bites at the gate, not just at resolve).
    r.revoke(&s, &cap("net.fetch"), BindingVersion(2)).unwrap();
    assert_eq!(
        invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
            .expect_err("revoked requirement must deny"),
        GateDenial::RequiredCapabilityUnbound { required: "net.fetch".to_string() }
    );
}

// Policy is enforced, never owned (Vol 9 Part 10): Deny AND ApprovalRequired both
// BLOCK (F-POLICY — the safety gate blocks, it does not warn), before any engine
// invocation; Allow proceeds. The verdict is a caller-supplied Governance input.
#[test]
fn gate_policy_enforced_not_owned_blocks_before_invocation() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    r.register(&s, cap("derive.fact")).unwrap();
    r.bind(binding_for(&s, "derive.fact", 1, &provider_of("derive.fact"), EffectClass::ProposesWrite))
        .unwrap();
    let e = CountingEngine::new("derive.fact", &[], true);

    for blocked in [PolicyVerdict::Deny, PolicyVerdict::ApprovalRequired] {
        assert_eq!(
            invoke_gated(&r, &s, &cap("derive.fact"), blocked, &e, b"x".to_vec())
                .expect_err("blocked policy must refuse"),
            GateDenial::PolicyBlocked(blocked)
        );
    }
    assert_eq!(e.calls.get(), 0, "policy blocks BEFORE the engine runs");

    invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
        .expect("Allow proceeds");
    assert!(e.calls.get() > 0);
}

// EffectClass validation (design §3.1.2(c)): a Pure-declared capability whose
// engine proposes effects is refused (nothing escapes); a ProposesWrite binding
// passes the SAME engine's proposals through — as PROPOSALS only (this crate has
// no kernel: committing is structurally impossible here, ORCH-001).
#[test]
fn gate_effectclass_pure_capability_must_propose_nothing() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    r.register(&s, cap("read.fact")).unwrap();
    r.bind(binding_for(&s, "read.fact", 1, &provider_of("read.fact"), EffectClass::Pure)).unwrap();

    // Engine proposes 1 effect under a Pure binding -> refused.
    let proposing = CountingEngine::new("read.fact", &[], true);
    assert_eq!(
        invoke_gated(&r, &s, &cap("read.fact"), PolicyVerdict::Allow, &proposing, b"x".to_vec())
            .expect_err("a proposing 'pure' capability is a contract violation"),
        GateDenial::PureEffectViolation { proposed: 1 }
    );

    // A genuinely effect-free engine passes under Pure.
    let pure = CountingEngine::new("read.fact", &[], false);
    let ok = invoke_gated(&r, &s, &cap("read.fact"), PolicyVerdict::Allow, &pure, b"x".to_vec())
        .expect("effect-free engine passes a Pure binding");
    assert!(ok.inference.proposed_effects.is_empty());

    // The same proposing engine under a ProposesWrite binding: proposals pass
    // through AS proposals (routing them to the Kernel is the caller's job).
    let mut r2 = LifecycleRegistry::new();
    r2.register(&s, cap("read.fact")).unwrap();
    r2.bind(binding_for(&s, "read.fact", 1, &provider_of("read.fact"), EffectClass::ProposesWrite))
        .unwrap();
    let out = invoke_gated(&r2, &s, &cap("read.fact"), PolicyVerdict::Allow, &proposing, b"x".to_vec())
        .expect("ProposesWrite may propose");
    assert_eq!(out.inference.proposed_effects.len(), 1);
    assert_eq!(out.authorization.effect, EffectClass::ProposesWrite);
}

// The gated path WIRES RCR-012: a mis-keyed engine (ORCH-004 violation) and a
// falsely-declared Deterministic engine are refused by the fabric enforcement
// inside the gate — after authorization, before anything escapes.
#[test]
fn gate_wires_rcr012_fabric_enforcement() {
    let s = shard();
    let mut r = LifecycleRegistry::new();

    // Mis-keyed engine: returns IdempotencyKey::default() instead of the fabric key.
    struct MisKeyed;
    impl Engine for MisKeyed {
        type Input = Vec<u8>;
        fn manifest(&self) -> EngineManifest {
            EngineManifest {
                name: "mis.keyed".into(),
                version: "1.0.0".into(),
                determinism: Determinism::Deterministic,
                idempotency_key: IdempotencyKey("acs-002/1".into()),
                reads: vec![],
                produces: vec!["uci.fact".into()],
                capabilities_required: vec![],
            }
        }
        fn invoke(&self, input: Vec<u8>) -> Inference {
            Inference { key: IdempotencyKey::default(), output: input, proposed_effects: vec![] }
        }
    }
    r.register(&s, cap("mis.keyed")).unwrap();
    r.bind(binding_for(&s, "mis.keyed", 1, "engine:mis.keyed@1.0.0", EffectClass::Pure)).unwrap();
    assert!(matches!(
        invoke_gated(&r, &s, &cap("mis.keyed"), PolicyVerdict::Allow, &MisKeyed, b"x".to_vec()),
        Err(GateDenial::Fabric(FabricViolation::KeyMismatch { .. }))
    ));

    // Falsely-deterministic engine: declares Deterministic, varies per call
    // (deterministically scripted via a counter — no clocks, no randomness).
    struct FalselyDeterministic {
        calls: Cell<u8>,
    }
    impl Engine for FalselyDeterministic {
        type Input = Vec<u8>;
        fn manifest(&self) -> EngineManifest {
            EngineManifest {
                name: "false.det".into(),
                version: "1.0.0".into(),
                determinism: Determinism::Deterministic,
                idempotency_key: IdempotencyKey("acs-002/1".into()),
                reads: vec![],
                produces: vec!["uci.fact".into()],
                capabilities_required: vec![],
            }
        }
        fn invoke(&self, input: Vec<u8>) -> Inference {
            let n = self.calls.get();
            self.calls.set(n + 1);
            Inference {
                key: invocation_key(&self.manifest(), &input),
                output: vec![n], // differs on the probe's second invocation
                proposed_effects: vec![],
            }
        }
    }
    r.register(&s, cap("false.det")).unwrap();
    r.bind(binding_for(&s, "false.det", 1, "engine:false.det@1.0.0", EffectClass::Pure)).unwrap();
    assert_eq!(
        invoke_gated(
            &r,
            &s,
            &cap("false.det"),
            PolicyVerdict::Allow,
            &FalselyDeterministic { calls: Cell::new(0) },
            b"x".to_vec()
        )
        .expect_err("false determinism declaration must be refused"),
        GateDenial::Fabric(FabricViolation::NondeterministicOutput)
    );
}

// PINNING (design §3.8 rebind-during-flight): a gated invocation pins the binding
// version active at authorization; a concurrent rebind supersedes FUTURE
// authorizations only — the issued Authorization keeps the version that actually
// ran (ORCH-003 basis; CAP-009 (PROPOSED)), and a fresh authorize never serves the
// stale version. SHARD-001: the authorization is scoped to its shard — the same
// capability in another shard is denied.
#[test]
fn gate_pins_binding_version_and_scopes_to_shard() {
    let s = shard();
    let mut r = LifecycleRegistry::new();
    r.register(&s, cap("derive.fact")).unwrap();
    r.bind(binding_for(&s, "derive.fact", 1, &provider_of("derive.fact"), EffectClass::ProposesWrite))
        .unwrap();
    let e = CountingEngine::new("derive.fact", &[], true);

    let first = invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
        .expect("v1 invocation");
    assert_eq!(first.authorization.pinned_version, BindingVersion(1));

    // Rebind to v2 (same engine identity — a contract revision, not a new provider).
    r.bind(binding_for(&s, "derive.fact", 2, &provider_of("derive.fact"), EffectClass::ProposesWrite))
        .unwrap();

    // The already-issued invocation still records v1 (replay uses what ran)…
    assert_eq!(first.authorization.pinned_version, BindingVersion(1));
    // …and the pinned binding remains reconstructible from the lifecycle history.
    assert_eq!(
        r.resolve_pinned(&s, &cap("derive.fact"), first.authorization.pinned_version).unwrap().version,
        BindingVersion(1)
    );

    // A NEW invocation pins v2; a fresh authorize can never yield the stale v1.
    let second = invoke_gated(&r, &s, &cap("derive.fact"), PolicyVerdict::Allow, &e, b"x".to_vec())
        .expect("v2 invocation");
    assert_eq!(second.authorization.pinned_version, BindingVersion(2));
    let auth = authorize(&r, &s, &cap("derive.fact"), &e.manifest(), PolicyVerdict::Allow).unwrap();
    assert_eq!(auth.pinned_version, BindingVersion(2), "stale binding never served after rebind");

    // SHARD-001 at the gate: no binding exists for this capability in another
    // shard, so authorization there is a hard deny.
    let other = ShardKey::new("tenant-b", "w1").unwrap();
    assert!(matches!(
        authorize(&r, &other, &cap("derive.fact"), &e.manifest(), PolicyVerdict::Allow),
        Err(GateDenial::Unbound { .. })
    ));
}

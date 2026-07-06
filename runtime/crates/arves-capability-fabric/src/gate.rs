//! The capability AUTHORIZATION GATE + gated invocation path (RCR-026, I4 Stage 1).
//!
//! Design basis: `docs/design/I4_Capability_Scheduling_Design.md` §3.1.2
//! (authorization: (a) an active binding in the shard — `Unbound` is a hard deny;
//! (b) policy gates owned by Governance and *enforced* here, Vol 9 Part 10; (c) the
//! declared `EffectClass`), §3.1.1 (resolve `Capabilities Required` per shard,
//! Engine Graph Part 3/10 "honour Capabilities Required via the Capability Fabric"),
//! §3.1.4 (idempotency-key enforcement — delegated to the RCR-012 fabric-enforced
//! invocation), and §3.8 (a dispatched invocation PINS its `BindingVersion`).
//!
//! This module **formalizes fabric-side the exact gate semantics the bridge already
//! exercises** (`arves-bridge::invoke` steps 1/1b/2): authoritative binding
//! resolution, the engine-IDENTITY check (the binding's provider must name this
//! exact `engine:{name}@{version}` — a name-only gate would let any engine run
//! under an authorized capability; Vol 9 Part 3 registered basis, CAP-002 states it
//! directly but is (PROPOSED — CCP-GATE required)), and invocation through
//! [`invoke_enforced`] (RCR-012: fabric-derived ORCH-004 key verification + the
//! best-effort determinism probe).
//!
//! ## What this module is — and, emphatically, is not
//!
//! It is a **stateless validation + composition surface**. It owns NO state (the
//! caller's registry owns the bindings, OWN-001), **selects nothing** (the
//! capability id is the caller's plan input — selection stays the Control Plane's,
//! Vol 9 Part 3; no "smart fabric", design §6.1), **commits nothing** (this crate
//! has no dependency on any kernel — a commit is structurally impossible here,
//! ORCH-001), **owns no policy** (the [`PolicyVerdict`] is a Governance-owned input,
//! enforced-not-owned, Vol 9 Part 10), and **records nothing** (decision-trace
//! emission is a later I4 stage; granularity is design OQ-10). The frozen registry
//! contract's "the fabric does not perform the invocation itself" language governs
//! the [`CapabilityRegistry`](crate::CapabilityRegistry) surface, which is
//! byte-unchanged; this gate delegates the invocation DOWNWARD to the Engine layer
//! (`arves-capability-fabric` rank 70 → `arves-engine-fabric` rank 60, the one new
//! LAYER-001 edge of RCR-026).
//!
//! ## Honest scope
//!
//! Single-process, trusted-host (v1.0 threat model): this is capability/policy
//! gating, NOT cryptographic principal authentication (design §3.16;
//! RUNTIME_FREEZE #8 v2.0 debt). The determinism probe inherited from RCR-012 is a
//! probe, not a proof. For `EffectClass::IdempotentEffect` the idempotency of the
//! EXTERNAL effect remains a provider DECLARATION the fabric cannot verify (design
//! §3.1.4 caveat; v1.1 debt #2). Proposed CAP-00n invariants are cited only as
//! (PROPOSED — CCP-GATE required) and carry no conformance weight.

use arves_engine_fabric::{invoke_enforced, Engine, EngineManifest, FabricViolation, Inference};

use crate::{
    BindingVersion, CapabilityId, CapabilityRegistry, EffectClass, ProviderId, RegistryError,
    ShardKey,
};

/// A Governance-owned policy decision, supplied BY the caller and enforced — never
/// computed — here (Vol 9 Part 10: the Control Plane/gate "enforces and sequences"
/// policy but never owns it; design §3.2 input row "Policy decisions
/// (allow/deny/approval-required)").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyVerdict {
    /// Policy allows the invocation.
    Allow,
    /// Policy denies the invocation — the gate MUST block (F-POLICY, design §3.9).
    Deny,
    /// Policy requires human/HITL approval. In Stage 1 there is no approval
    /// sequencing surface (that is Control-Plane machinery, a later stage), so the
    /// gate BLOCKS — refusal, never a silent allow (RCR-026 DR-5).
    ApprovalRequired,
}

/// Why the gate refused an invocation. Every denial happens BEFORE any effect can
/// exist: `Unbound`/`ProviderMismatch`/`RequiredCapabilityUnbound`/`PolicyBlocked`
/// are pre-invocation; `Fabric` is the RCR-012 refusal; `PureEffectViolation` refuses
/// the *result* of a `Pure`-declared capability that proposed effects (nothing is
/// returned to the caller, so the undeclared proposals go nowhere).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateDenial {
    /// No active binding for the invoked capability in this shard — hard deny
    /// (F-UNBOUND, design §3.9; fabric `RegistryError::Unbound` surface). By
    /// DEFINITION, ANY `resolve` refusal is recorded as F-UNBOUND: authorization
    /// requires an active binding to be resolvable, so a foreign
    /// [`CapabilityRegistry`] implementation returning some other
    /// [`RegistryError`](crate::RegistryError) variant from `resolve` (the in-repo
    /// registries return only `Unbound`) denies under this same class — the shared
    /// meaning is "no active binding was resolvable".
    Unbound {
        /// Capability that failed to resolve.
        capability: CapabilityId,
        /// Shard the authorization was scoped to (SHARD-001).
        shard: ShardKey,
    },
    /// The engine's manifest declares a required capability (Engine Graph Part 3
    /// "Capabilities Required") with no active binding in this shard — the runtime
    /// must "honour Capabilities Required via the Capability Fabric" (Part 10).
    RequiredCapabilityUnbound {
        /// The declared-but-unbound required capability name.
        required: String,
    },
    /// The resolved binding names a different provider than the presenting engine:
    /// the gate binds to engine IDENTITY (`engine:{name}@{version}`), not just the
    /// capability name (Vol 9 Part 3; CAP-002 (PROPOSED — CCP-GATE required)).
    ProviderMismatch {
        /// Provider identity derived from the presenting engine's manifest.
        expected: ProviderId,
        /// Provider the registry's active binding actually names.
        bound: ProviderId,
    },
    /// The Governance-supplied policy verdict blocks the invocation (`Deny`, or
    /// `ApprovalRequired` with no approval surface in Stage 1 — RCR-026 DR-5).
    PolicyBlocked(PolicyVerdict),
    /// A binding declared `EffectClass::Pure` but the engine's inference proposed
    /// effects — a read-only capability must propose NOTHING (design §3.1.2(c);
    /// frozen `EffectClass::Pure` doc: "No observable side effects").
    PureEffectViolation {
        /// How many effects the engine tried to propose.
        proposed: usize,
    },
    /// The RCR-012 fabric enforcement refused the invocation (mis-keyed inference,
    /// ORCH-004, or a falsely-declared deterministic engine caught by the probe).
    Fabric(FabricViolation),
}

/// Proof token of a passed authorization gate: WHAT was authorized, in WHICH shard,
/// under WHICH pinned binding version and provider, with WHICH declared effect
/// class. The pinned version is what a dispatch records so replay uses the binding
/// that actually ran (design §3.8/§3.11; ORCH-003 registered basis; CAP-009
/// (PROPOSED — CCP-GATE required)). A concurrent rebind supersedes FUTURE
/// authorizations only — it never rewrites an issued `Authorization`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authorization {
    /// The invoked capability.
    pub capability: CapabilityId,
    /// The shard the authorization is scoped to (SHARD-001 — valid nowhere else).
    pub shard: ShardKey,
    /// The binding version active at authorization time, pinned for the dispatch.
    pub pinned_version: BindingVersion,
    /// The provider the capability was bound to (identity-checked).
    pub provider: ProviderId,
    /// The declared effect class the invocation result will be validated against.
    pub effect: EffectClass,
}

/// The result of a gated invocation: the authorization proof plus the engine's
/// [`Inference`] — **proposals only**. Nothing in here is committed; routing
/// `inference.proposed_effects` to the Kernel commit gateway is the caller's
/// (bridge/Control Plane) job (ORCH-001; design §3.1.2(c)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatedInvocation {
    /// The gate's proof token, incl. the pinned binding version.
    pub authorization: Authorization,
    /// The engine's inference: output + PROPOSED effects, under the verified
    /// ORCH-004 key.
    pub inference: Inference,
}

/// The provider identity an engine presents: `engine:{name}@{version}` — the exact
/// scheme the bridge's gate already exercises (`arves-bridge::invoke` step 1b),
/// formalized here as the fabric's single definition.
pub fn engine_provider_id(manifest: &EngineManifest) -> ProviderId {
    ProviderId(format!("engine:{}@{}", manifest.name, manifest.version))
}

/// Authorize one capability invocation for one presenting engine in one shard.
///
/// Checks, in this fixed, documented order (RCR-026 DR-9 — deterministic; each is a
/// hard deny):
///
/// 1. **Active binding** (design §3.1.2(a)): resolve the invoked capability in
///    `shard`; no active binding → [`GateDenial::Unbound`] (F-UNBOUND).
/// 2. **Engine identity** (Vol 9 Part 3; CAP-002 (PROPOSED)): the binding's provider
///    must equal [`engine_provider_id`] of the presenting engine's manifest →
///    else [`GateDenial::ProviderMismatch`].
/// 3. **Capabilities Required** (Engine Graph Parts 3/10; design §3.1.1): every
///    capability the engine's manifest declares as required must have an active
///    binding in the SAME shard → else [`GateDenial::RequiredCapabilityUnbound`].
///    (Existence of an active binding only; the identity check of step 2 applies to
///    the invoked capability — RCR-026 DR-9.)
/// 4. **Policy** (design §3.1.2(b); Vol 9 Part 10 enforced-not-owned): the supplied
///    [`PolicyVerdict`] must be `Allow`; `Deny`/`ApprovalRequired` block
///    ([`GateDenial::PolicyBlocked`], F-POLICY).
///
/// On success returns the [`Authorization`] proof with the binding version PINNED.
/// Pure function of `(registry state, arguments)`: no clocks, no randomness, no
/// side effects (`&impl CapabilityRegistry` — resolve is a side-effect-free read).
pub fn authorize(
    registry: &impl CapabilityRegistry,
    shard: &ShardKey,
    capability: &CapabilityId,
    manifest: &EngineManifest,
    policy: PolicyVerdict,
) -> Result<Authorization, GateDenial> {
    // 1. Active binding — hard deny if unbound (F-UNBOUND). Any resolve refusal is
    //    F-UNBOUND by definition (see `GateDenial::Unbound`); the registry-reported
    //    fields are preserved when the error carries them, so a foreign registry
    //    implementation cannot skew the denial record.
    let binding = registry.resolve(shard, capability).map_err(|e| match e {
        RegistryError::Unbound { capability, shard } => GateDenial::Unbound { capability, shard },
        _ => GateDenial::Unbound { capability: capability.clone(), shard: shard.clone() },
    })?;

    // 2. Engine identity, not just capability name (CAP-002-style, proposed).
    let expected = engine_provider_id(manifest);
    if binding.provider != expected {
        return Err(GateDenial::ProviderMismatch { expected, bound: binding.provider });
    }

    // 3. Honour Capabilities Required via the fabric (Engine Graph Part 10).
    for required in &manifest.capabilities_required {
        if registry.resolve(shard, &CapabilityId(required.clone())).is_err() {
            return Err(GateDenial::RequiredCapabilityUnbound { required: required.clone() });
        }
    }

    // 4. Enforce (never own) the Governance policy verdict (F-POLICY blocks).
    match policy {
        PolicyVerdict::Allow => {}
        blocked => return Err(GateDenial::PolicyBlocked(blocked)),
    }

    Ok(Authorization {
        capability: capability.clone(),
        shard: shard.clone(),
        pinned_version: binding.version,
        provider: binding.provider,
        effect: binding.contract.effect,
    })
}

/// Validate an engine's [`Inference`] against the binding's declared
/// [`EffectClass`] (design §3.1.2(c)):
///
/// - [`EffectClass::Pure`] — the inference must propose ZERO effects (read-only by
///   declaration; a proposing "pure" capability is refused,
///   [`GateDenial::PureEffectViolation`]).
/// - [`EffectClass::IdempotentEffect`] / [`EffectClass::ProposesWrite`] — proposals
///   pass through AS PROPOSALS; committing them is exclusively the Kernel's, via the
///   caller (ORCH-001). External-effect idempotency stays a provider declaration the
///   fabric cannot verify (design §3.1.4 caveat; v1.1 debt #2) — honestly NOT
///   checked here.
pub fn validate_effects(effect: EffectClass, inference: &Inference) -> Result<(), GateDenial> {
    if effect == EffectClass::Pure && !inference.proposed_effects.is_empty() {
        return Err(GateDenial::PureEffectViolation { proposed: inference.proposed_effects.len() });
    }
    Ok(())
}

/// The Stage-1 capability-gated invocation path:
/// `authorize` → [`invoke_enforced`] (RCR-012) → [`validate_effects`].
///
/// A denial at ANY step returns before anything escapes; a blocked policy or a
/// missing binding refuses BEFORE the engine is invoked at all. On success the
/// caller receives the [`GatedInvocation`] — authorization proof (with the pinned
/// binding version, §3.8) plus the inference whose effects are PROPOSALS ONLY;
/// this function commits nothing and can commit nothing (no kernel exists in this
/// crate, ORCH-001).
///
/// Note (RCR-012 inheritance): a `Determinism::Deterministic` engine is
/// double-invoked by the probe, so a conformant deterministic engine runs twice
/// per gated invocation — a probe, not a proof, and the invocation stays
/// idempotent under its ORCH-004 key.
///
/// Manifest-stability assumption (trusted-host caveat, design §3.16 — same class
/// as the determinism-probe honesty note): `authorize` checks one
/// `engine.manifest()` snapshot, and [`invoke_enforced`] re-fetches the manifest
/// internally for ORCH-004 key derivation. The gate ASSUMES an engine returns a
/// stable manifest across those calls; an engine presenting inconsistent manifests
/// could show one identity to the gate and another to the fabric enforcement. This
/// is within the declared single-process trusted-host v1.0 threat model — the gate
/// does not defend against a hostile in-process `Engine` implementation. Threading
/// a single manifest snapshot through both steps is a later-stage improvement.
pub fn invoke_gated<E>(
    registry: &impl CapabilityRegistry,
    shard: &ShardKey,
    capability: &CapabilityId,
    policy: PolicyVerdict,
    engine: &E,
    input: Vec<u8>,
) -> Result<GatedInvocation, GateDenial>
where
    E: Engine<Input = Vec<u8>>,
{
    let manifest = engine.manifest();
    let authorization = authorize(registry, shard, capability, &manifest, policy)?;
    let inference = invoke_enforced(engine, input).map_err(GateDenial::Fabric)?;
    validate_effects(authorization.effect, &inference)?;
    Ok(GatedInvocation { authorization, inference })
}

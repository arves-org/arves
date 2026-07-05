//! ARVES :: arves-engine-fabric
//!
//! Purpose: Pure, stateless engines behind the Engine ABI; they produce
//! *inference*, never *truth*. An engine is a deterministic-by-default
//! computation that maps an input to an [`Inference`]. Engines own nothing
//! persistent: any state change they wish to see happen is emitted as a
//! *proposed effect* only. Whether those effects ever become truth is decided
//! elsewhere (Control Plane plans it; only the Kernel commits it).
//!
//! Governing (registered-normative): ORCH-001 (only the Kernel owns truth),
//! ORCH-003 (replay from decision trace), ORCH-004 (idempotent +
//! content-addressable invocation), LAYER-001. Grounded-in (proposed, NOT yet
//! ratified — carry no conformance weight): ENG-001..005, G-001, QUERY-001,
//! CAP-001..009 of the Invariant Registry v1.0. Frozen source: ARVES Engine
//! Graph Specification v1.0.
//! Layer: Data Plane (Engine layer of LAYER-001).
//!
//! ## Position in the ARVES layering (LAYER-001)
//!
//! `Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query ->
//! Engine -> Capability -> Execution (+ Control Plane)`. Dependencies flow
//! downward only. The Engine layer sits *above* Query and *below* Capability:
//! engines read through the read-only Query layer (QUERY-001) and never reach
//! sideways or upward into truth-owning layers.
//!
//! ## Two planes
//!
//! Engines are a Data-Plane concern: they *carry* computation. They do not
//! decide what runs (that is the Control Plane, ORCH-002 — plans, never state)
//! and they do not own the truth their inference is derived from (that is the
//! Kernel, ORCH-001 / G-001). An engine invocation is a pure function of its
//! input plus the read-snapshot it was given; it has no ambient authority to
//! mutate the world.
//!
//! ## Governing invariants (cited inline throughout)
//!
//! Cited by their **registered-normative** IDs (Invariant Registry v1.0, Part 2).
//! The engine properties named below are the fabric's *engineering expression*
//! of these invariants; they are NOT the frozen `ENG-00n` statements (Registry
//! Part 4). The frozen `ENG-00n` numbering differs from any earlier draft use in
//! this crate — do not read a property here as "ENG-001", etc. Where a proposed
//! invariant is the nearest grounding it is cited as "(proposed)" and carries no
//! conformance weight until it passes the CCP-GATE.
//!
//! - **Engine purity (Engine Graph Spec Part 4; grounds proposed ENG-001).**
//!   `invoke` is a pure function of its input and the declared read-set; no
//!   hidden side effects, no ambient I/O.
//! - **Engines own no persistent state (OWN-001; grounds proposed ENG-001).**
//!   Engines are stateless between invocations — one owner per state, and it is
//!   never an engine.
//! - **Writes are proposals, not commits (ORCH-001; grounds proposed ENG-002).**
//!   An engine emits [`ProposedEffect`]s; it can never itself mutate truth
//!   (commit is the Kernel's alone, ORCH-001 / proposed G-001).
//! - **Declared reads/produces (Engine Graph Spec Part 3; grounds proposed
//!   ENG-005).** An engine declares what it
//!   [`reads`](EngineManifest::reads) and [`produces`](EngineManifest::produces)
//!   up front in its [`EngineManifest`]; the declaration is a contract the
//!   fabric can check against.
//! - **Declared determinism + required capabilities (Engine Graph Spec
//!   Parts 3/6; grounds proposed ENG-004/ENG-005).** An engine declares its
//!   [`Determinism`] class and the
//!   [`capabilities_required`](EngineManifest::capabilities_required) to run.
//! - **ORCH-004 (registered): Idempotent + content-addressable invocation.**
//!   Every engine invocation is keyed by an [`IdempotencyKey`] derived from
//!   `(manifest identity, canonicalized input, read-snapshot)`, so replaying the
//!   same invocation yields the same [`Inference`] (supports ORCH-003: replay
//!   from recorded decision trace, not recomputation).
//!
//! ## STATUS
//!
//! I1 CONTRACT + FABRIC ENFORCEMENT (RCR-012). This crate defines the Engine
//! Fabric interfaces/types AND, since RCR-012, the fabric-owned invocation
//! discipline: [`invocation_key`] (the fabric derives the ORCH-004
//! content-addressable key — engines no longer self-mint it) and
//! [`invoke_enforced`] (key verification + a double-invoke determinism probe, so
//! a false `Determinism::Deterministic` declaration is refused, not trusted).
//! Engine *business* logic still lives outside this crate; the exercised
//! engine logic in the reference runtime flows through the SDK/Bridge in
//! `products/` (see RUNTIME_FREEZE_v1.0.md, guarantee alignment), and the bridge
//! invokes engines exclusively through [`invoke_enforced`].
//! Frozen specification governs; this crate *implements*, never changes it.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Identity & addressing
// ---------------------------------------------------------------------------

/// A stable, human-readable engine name (e.g. `"summarize.text"`).
///
/// The `(name, version)` pair identifies an engine implementation for the
/// purposes of the [`EngineManifest`] and idempotency keying (Engine Graph Spec
/// Part 3 Identity group; ORCH-004).
pub type EngineName = String;

/// A version string for an engine implementation.
///
/// Distinct versions are distinct engines: they may differ in behaviour, so a
/// re-version invalidates prior [`IdempotencyKey`]s (ORCH-004).
pub type EngineVersion = String;

/// A declared, content-addressable name of a state slice an engine reads or
/// produces (Engine Graph Spec Part 3 Type-contract group).
///
/// Reads name inputs pulled through the read-only Query layer (proposed
/// QUERY-001); produces name the *shapes* of [`ProposedEffect`]s the engine may
/// emit — which, per the Engine Graph Spec Part 3, are the spec's "Writes"
/// (proposed effects), NOT the spec's distinct "Produces" (output-artifact
/// types); see [`EngineManifest::produces`]. These are declarations, not
/// handles: naming a resource here does not grant an engine authority over it
/// (ORCH-001 / OWN-001).
pub type ResourceName = String;

/// A declared capability an engine requires in order to run (Engine Graph Spec
/// Part 3 "Capabilities Required"; proposed CAP-001..009).
///
/// The Capability layer (one layer *below* Engine consumers, one *above* in the
/// call sense) binds these names to concrete grants; the Engine layer only
/// declares the requirement, it never resolves or holds the binding itself.
pub type CapabilityName = String;

/// A content-addressable key for a single engine invocation (ORCH-004).
///
/// Derived from `(manifest identity, canonicalized input, read-snapshot)`. Two
/// invocations sharing an `IdempotencyKey` MUST yield the same [`Inference`],
/// which is what lets the runtime replay from a recorded decision trace rather
/// than recomputing (ORCH-003) and lets the Kernel dedupe committed outcomes
/// (IDR: replicate committed *outcomes*, not invocations).
///
/// For [`Determinism::Nondeterministic`] engines the key still addresses the
/// *invocation*, but the recorded output — not a recomputation — is
/// authoritative on replay.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct IdempotencyKey(pub String);

// ---------------------------------------------------------------------------
// Determinism classification (Engine Graph Spec Parts 3/6)
// ---------------------------------------------------------------------------

/// The determinism class an engine declares for itself (Engine Graph Spec
/// Part 3 Determinism field; drives replay per Part 6 / ORCH-003).
///
/// This is a *promise the engine makes to the fabric*, not something the fabric
/// infers. It governs how the runtime may treat repeat invocations and replay
/// (ORCH-003, ORCH-004).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Determinism {
    /// Bit-for-bit reproducible from `(input, read-snapshot)` alone. Same
    /// [`IdempotencyKey`] always implies the same [`Inference`]; the runtime may
    /// freely cache, dedupe, and skip recomputation (ORCH-003, ORCH-004).
    Deterministic,

    /// Reproducible *given an explicit seed* recorded alongside the invocation.
    /// The seed is part of what the [`IdempotencyKey`] addresses; replay reuses
    /// the recorded seed rather than drawing a fresh one.
    Seeded,

    /// Not reproducible by recomputation (e.g. wraps a nondeterministic external
    /// oracle). The recorded [`Inference`] in the decision trace is
    /// authoritative on replay (ORCH-003); the runtime must never silently
    /// recompute a nondeterministic engine and treat the result as equivalent.
    Nondeterministic,
}

impl Default for Determinism {
    /// Default to the strongest promise so that an unspecified engine is treated
    /// conservatively as reproducible (Engine Graph Spec Part 6).
    fn default() -> Self {
        Determinism::Deterministic
    }
}

// ---------------------------------------------------------------------------
// Manifest (Engine Graph Spec Part 3 — PARTIAL / I1 subset)
// ---------------------------------------------------------------------------

/// The self-description an engine publishes to the fabric (Engine Graph Spec
/// Part 3 — the Engine Node Contract / ABI).
///
/// The manifest is a *contract*: it declares identity, determinism, the
/// idempotency-key scheme, and the read/produce/capability sets up front so the
/// fabric can plan, key, and audit invocations without executing them. A
/// manifest describes intent and shape only — it grants no authority and owns no
/// state (ORCH-001, OWN-001).
///
/// **SCOPE NOTE — this is an I1 SUBSET of the Part-3 manifest, not the full ABI.**
/// The frozen Engine Graph Specification Part 3 defines the following normative
/// manifest fields that this struct DOES NOT yet model; they are **DEFERRED**
/// and MUST NOT be read as intentionally absent from the ABI:
///
/// - **Preconditions** (Type contract) — conditions that must hold before invocation.
/// - **Produces** (Type contract) — inference/ontology *output-artifact types*,
///   distinct from this struct's [`produces`](EngineManifest::produces) which
///   maps to the spec's **Writes** (proposed effects). See that field's note.
/// - **Failure Policy** (Execution) — behaviour on failure (fail/degrade/escalate).
/// - **Retry Policy** (Execution) — retry count, backoff, recovery.
/// - **Timeout** (Execution) — max execution bound.
/// - **Confidence** (Planning metadata) — declared/estimated output confidence.
/// - **Cost** (Planning metadata) — declared/estimated cost (token/compute).
/// - **Latency** (Planning metadata) — declared/estimated latency.
///
/// Modelling these is a runtime-forward item (v1.1+ per RUNTIME_FREEZE_v1.0.md),
/// not a spec gap — the frozen spec is correct and complete; this crate carries
/// a deliberate I1 subset. Do not treat this struct as the full manifest ABI.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EngineManifest {
    /// Stable engine name; with `version`, identifies the implementation
    /// (Engine Graph Spec Part 3 Identity group).
    pub name: EngineName,

    /// Implementation version; a bump is a distinct engine and invalidates
    /// prior idempotency keys (Engine Graph Spec Parts 3/11; ORCH-004).
    pub version: EngineVersion,

    /// Declared determinism class (Engine Graph Spec Part 3). See [`Determinism`].
    pub determinism: Determinism,

    /// The scheme/version tag describing how this engine's [`IdempotencyKey`] is
    /// derived (ORCH-004). Naming the scheme in the manifest lets the fabric
    /// reject invocations keyed under an incompatible scheme.
    pub idempotency_key: IdempotencyKey,

    /// Declared read-set: the resources this engine may read, pulled through the
    /// read-only Query layer (Engine Graph Spec Part 3 "Reads"; proposed
    /// QUERY-001). Reading outside this set is a contract violation.
    pub reads: Vec<ResourceName>,

    /// Declared produce-set: the shapes of [`ProposedEffect`]s this engine may
    /// emit. Producing anything outside this set is a contract violation;
    /// nothing here is a commitment to *persist* (ORCH-001).
    ///
    /// **SCOPE NOTE — maps to the spec's "Writes", not "Produces".** In the
    /// frozen Engine Graph Specification Part 3 these are two distinct manifest
    /// fields: **Writes** = "PROPOSED state effects to be committed by Kernel
    /// (never direct truth)", and **Produces** = "Inference/ontology artifacts
    /// emitted as output". Despite its Rust name, THIS field is the spec's
    /// **Writes** (proposed effects; see [`ProposedEffect`]). The spec's distinct
    /// **Produces** (output-artifact *types*) is **DEFERRED / undeclared** in this
    /// crate — the [`Inference::output`] bytes carry the artifact, but its
    /// ontology type is not declared here. Renaming this field to `writes` is a
    /// Runtime Change Request (out of scope for this doc-only alignment).
    pub produces: Vec<ResourceName>,

    /// Declared capabilities required to run (Engine Graph Spec Part 3
    /// "Capabilities Required"; proposed CAP-001..009). Resolved by the
    /// Capability layer; the Engine layer only declares them.
    pub capabilities_required: Vec<CapabilityName>,
}

// ---------------------------------------------------------------------------
// Proposed effects (Engine Graph Spec Parts 3/4 — the spec's "Writes")
// ---------------------------------------------------------------------------

/// A write an engine *wishes* to see happen — a proposal, never a commit
/// (Engine Graph Spec Part 4; ORCH-001).
///
/// This is the concrete form of a spec **Write** (Engine Graph Spec Part 3:
/// "PROPOSED state effects to be committed by Kernel (never direct truth)").
/// Engines are pure and own no persistent state (Engine Graph Spec Part 4;
/// OWN-001), so they cannot mutate truth. Instead they emit `ProposedEffect`s
/// naming the intended change. Whether a proposal becomes truth is decided by
/// the Control Plane (as a plan, ORCH-002) and committed exclusively by the
/// Kernel through the shard leader (ORCH-001; proposed G-001; IDR: engines run
/// anywhere, commit only via shard leader).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProposedEffect {
    /// The declared resource this effect targets; MUST appear in the engine
    /// manifest's [`produces`](EngineManifest::produces) set (Engine Graph Spec
    /// Part 3).
    pub target: ResourceName,

    /// Opaque, serialized payload describing the proposed change. The Engine
    /// layer treats this as inert bytes; interpretation and commit belong to the
    /// Kernel (Engine Graph Spec Part 4; ORCH-001; proposed G-001).
    pub payload: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Inference (the sole output of an engine)
// ---------------------------------------------------------------------------

/// The result of an [`Engine::invoke`] — *inference*, not truth.
///
/// An `Inference` bundles the engine's produced output with any
/// [`ProposedEffect`]s and the [`IdempotencyKey`] under which it was computed
/// (ORCH-004). It carries no authority: it is a claim about what *could* follow,
/// which the Control Plane may plan on and the Kernel may (or may not) commit.
/// Persisting an `Inference`'s effects is out of scope for this layer
/// (Engine Graph Spec Part 4; ORCH-001).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Inference {
    /// The idempotency key this inference was computed under; ties the output to
    /// its invocation for replay/dedupe (ORCH-003, ORCH-004).
    pub key: IdempotencyKey,

    /// Opaque, serialized primary output of the engine (the "inference" proper).
    /// Carries the artifact of the spec's **Produces** (Engine Graph Spec
    /// Part 3), but as inert bytes — the artifact's ontology *type* is not
    /// declared in this I1 manifest subset (see [`EngineManifest`] scope note).
    pub output: Vec<u8>,

    /// Proposed effects the engine would like committed — proposals only; the
    /// spec's **Writes** (Engine Graph Spec Parts 3/4; ORCH-001).
    pub proposed_effects: Vec<ProposedEffect>,
}

// ---------------------------------------------------------------------------
// The Engine ABI (Engine Graph Spec Parts 3/4; ORCH-004)
// ---------------------------------------------------------------------------

/// The pure Engine ABI (Engine Graph Spec Part 4; ORCH-004).
///
/// An `Engine` is a **pure**, **stateless** computation: given an input (and,
/// implicitly, the read-snapshot addressed by the [`IdempotencyKey`]) it returns
/// an [`Inference`]. Implementations MUST honour the following contract:
///
/// - **Purity (Engine Graph Spec Part 4).** [`invoke`](Engine::invoke) has no
///   side effects beyond returning its result. No ambient I/O, no clocks, no RNG
///   except a recorded seed for [`Determinism::Seeded`] (Engine Graph Spec
///   Part 6).
/// - **No ownership (OWN-001).** The engine holds no persistent state across
///   invocations; `&self` is configuration/identity only.
/// - **Proposals, not commits (Engine Graph Spec Part 4; ORCH-001).** Any
///   desired write is returned as a [`ProposedEffect`] inside the [`Inference`];
///   the engine never commits truth (ORCH-001; proposed G-001).
/// - **Declared surface (Engine Graph Spec Part 3).** Behaviour stays within the
///   engine's [`EngineManifest`]: it reads only declared `reads`, produces only
///   declared `produces`, and requires only declared
///   `capabilities_required`.
/// - **Idempotent + content-addressable (ORCH-004).** Repeat invocation under
///   the same [`IdempotencyKey`] yields the same [`Inference`], enabling
///   replay-from-trace (ORCH-003) rather than recomputation.
///
/// This is a data-plane contract (carry, not decide) and depends downward only
/// (LAYER-001).
pub trait Engine {
    /// The engine-specific, deserialized input type.
    type Input;

    /// Return this engine's manifest (Engine Graph Spec Part 3). Pure, cheap, and
    /// invariant across invocations.
    fn manifest(&self) -> EngineManifest;

    /// Purely compute an [`Inference`] from `input` (Engine Graph Spec Part 4;
    /// ORCH-004).
    ///
    /// Implementations MUST NOT mutate external truth, perform ambient I/O, or
    /// retain state; all intended writes are returned as
    /// [`ProposedEffect`]s (Engine Graph Spec Part 4; ORCH-001). The returned
    /// [`Inference::key`] MUST match the idempotency key of this invocation
    /// (ORCH-004).
    fn invoke(&self, input: Self::Input) -> Inference;
}

// ---------------------------------------------------------------------------
// Fabric-enforced invocation (RCR-012) — derive the key, verify the promise.
// ---------------------------------------------------------------------------

/// Derive the content-addressable [`IdempotencyKey`] for one invocation — **the
/// FABRIC's derivation, not the engine's** (RCR-012; ORCH-004).
///
/// `scheme:name@version:<ACS-001 ContentId of the input under the INVOCATION domain>`
/// — the scheme tag comes from [`EngineManifest::idempotency_key`], the identity from
/// the manifest, and the input address is the standard ACS-001 content address
/// (`0x12 0x20 ‖ SHA-256(0x04 ‖ input)`), so two invocations share a key **iff** they
/// share engine identity and canonical input bytes.
///
/// Honest scope: the ORCH-004 derivation names `(manifest identity, canonicalized
/// input, read-snapshot)`. I1 engines have empty declared read-sets (`reads: []`), so
/// the read-snapshot component is vacuous here; when Query-layer reads land (I2+), the
/// snapshot address joins this derivation — an additive extension of the same scheme.
pub fn invocation_key(manifest: &EngineManifest, input: &[u8]) -> IdempotencyKey {
    IdempotencyKey(format!(
        "{}:{}@{}:{}",
        manifest.idempotency_key.0,
        manifest.name,
        manifest.version,
        arves_acs::hex(&arves_acs::content_id(arves_acs::domain::INVOCATION, input))
    ))
}

/// Why the fabric refused an engine invocation (RCR-012).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FabricViolation {
    /// The engine's returned [`Inference::key`] does not equal the fabric-derived
    /// key — the engine is mis-keying its own invocations, so its outputs cannot be
    /// replayed/deduped safely (ORCH-004). Refused before any effect is considered.
    KeyMismatch { expected: IdempotencyKey, got: IdempotencyKey },
    /// A self-declared [`Determinism::Deterministic`] engine returned two different
    /// [`Inference`]s for the same input — its declaration is FALSE. Refused: the
    /// runtime must never cache/dedupe/replay on a broken promise (ORCH-003/004).
    NondeterministicOutput,
}

/// Invoke an engine **under fabric enforcement** (RCR-012) — closes the v1.1 debt
/// *"the fabric derives/enforces the idempotency key rather than trusting an engine's
/// self-declared `Determinism`"*:
///
/// 1. The fabric derives the expected [`IdempotencyKey`] itself ([`invocation_key`])
///    and VERIFIES the engine's returned `Inference.key` equals it — a mis-keyed
///    engine is refused ([`FabricViolation::KeyMismatch`]).
/// 2. A self-declared [`Determinism::Deterministic`] engine is **double-invoked** and
///    the two `Inference`s compared bit-for-bit — a false declaration is refused
///    ([`FabricViolation::NondeterministicOutput`]) instead of trusted.
///    [`Determinism::Seeded`]/[`Determinism::Nondeterministic`] engines are not
///    re-invoked (their recorded inference is authoritative on replay, ORCH-003);
///    their key is still verified.
///
/// Honest scope: the double-invoke is a **probe, not a proof** — an engine whose
/// nondeterminism is input-scoped or slower than back-to-back invocation can evade it
/// (the same honesty boundary as the authoring kit's certify probe). The key check,
/// by contrast, is exact.
pub fn invoke_enforced<E>(engine: &E, input: Vec<u8>) -> Result<Inference, FabricViolation>
where
    E: Engine<Input = Vec<u8>>,
{
    let manifest = engine.manifest();
    let expected = invocation_key(&manifest, &input);
    let first = engine.invoke(input.clone());
    if first.key != expected {
        return Err(FabricViolation::KeyMismatch { expected, got: first.key });
    }
    if manifest.determinism == Determinism::Deterministic {
        let second = engine.invoke(input);
        if second != first {
            return Err(FabricViolation::NondeterministicOutput);
        }
    }
    Ok(first)
}

// ---------------------------------------------------------------------------
// Reference engine: a concrete, pure engine over a deterministic transform.
// ---------------------------------------------------------------------------

/// A concrete reference [`Engine`]: a pure, deterministic transform
/// `Fn(&[u8]) -> Vec<u8>` that emits its result as a single [`ProposedEffect`].
///
/// It owns no state and performs no I/O (Engine Graph Spec Part 4; OWN-001),
/// returns proposals rather than committing (Engine Graph Spec Part 4;
/// ORCH-001), and is deterministic (Engine Graph Spec Part 6) — so it satisfies
/// the *purity/ownership* shape of the ABI. It is a **minimal reference example**
/// used to exercise the wiring; it is **not** a full-fidelity engine and does not
/// on its own demonstrate an end-to-end cognitive work chain. The runtime (here,
/// the bridge) is what commits its proposed effect(s) via the Kernel.
///
/// **ORCH-004 conformant since RCR-012:** [`invoke`](PureEngine::invoke) returns the
/// fabric-derived [`invocation_key`] (it previously returned a documented
/// NON-CONFORMANT `IdempotencyKey::default()` placeholder; that runtime-forward item
/// is closed — see `runtime/rcr/RCR-012.md`).
pub struct PureEngine<F> {
    name: EngineName,
    target: ResourceName,
    transform: F,
}

impl<F> PureEngine<F> {
    /// A pure engine named `name` that emits its transform's output as a proposed effect
    /// targeting the declared `target` resource.
    pub fn new(name: impl Into<EngineName>, target: impl Into<ResourceName>, transform: F) -> Self {
        Self { name: name.into(), target: target.into(), transform }
    }
}

impl<F: Fn(&[u8]) -> Vec<u8>> Engine for PureEngine<F> {
    type Input = Vec<u8>;

    fn manifest(&self) -> EngineManifest {
        EngineManifest {
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            determinism: Determinism::Deterministic,
            idempotency_key: IdempotencyKey("acs-002/1".to_string()),
            reads: Vec::new(),
            produces: vec![self.target.clone()],
            capabilities_required: Vec::new(),
        }
    }

    fn invoke(&self, input: Vec<u8>) -> Inference {
        let output = (self.transform)(&input);
        // RCR-012: the key is the FABRIC's derivation — content-addressable from
        // (manifest identity, ACS-001 address of the canonical input), per ORCH-004.
        // This closes the NON-CONFORMANT `IdempotencyKey::default()` placeholder this
        // body carried through I1 (the documented runtime-forward item).
        let key = invocation_key(&self.manifest(), &input);
        Inference {
            key,
            proposed_effects: vec![ProposedEffect { target: self.target.clone(), payload: output.clone() }],
            output,
        }
    }
}

#[cfg(test)]
mod pure_engine_tests {
    use super::*;

    #[test]
    fn pure_engine_proposes_its_transform_output() {
        let e = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
        let inf = e.invoke(b"hello".to_vec());
        assert_eq!(inf.output, b"hello");
        assert_eq!(inf.proposed_effects.len(), 1);
        assert_eq!(inf.proposed_effects[0].target, "uci.fact");
        assert_eq!(inf.proposed_effects[0].payload, b"hello");
        assert_eq!(e.manifest().produces, vec!["uci.fact".to_string()]);
    }
}

#[cfg(test)]
mod rcr012_fabric_enforcement_tests {
    use super::*;
    use std::cell::Cell;

    // The fabric-derived key is content-addressable (ORCH-004): same identity + same
    // input -> same key; different input OR different version -> different key; and the
    // key embeds the ACS-001 ContentId of the input under the INVOCATION domain.
    #[test]
    fn invocation_key_is_content_addressable() {
        let e = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
        let m = e.manifest();
        let k1 = invocation_key(&m, b"hello");
        assert_eq!(k1, invocation_key(&m, b"hello"), "same identity+input -> same key");
        assert_ne!(k1, invocation_key(&m, b"world"), "different input -> different key");
        let mut m2 = m.clone();
        m2.version = "2.0.0".into();
        assert_ne!(k1, invocation_key(&m2, b"hello"), "re-version -> prior keys invalidated");
        let cid_hex = arves_acs::hex(&arves_acs::content_id(arves_acs::domain::INVOCATION, b"hello"));
        assert!(k1.0.ends_with(&cid_hex), "key embeds the ACS-001 invocation address");
    }

    // RCR-012 closes the documented PureEngine placeholder: the returned Inference.key
    // IS the fabric derivation, not IdempotencyKey::default().
    #[test]
    fn pure_engine_returns_the_fabric_key() {
        let e = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
        let inf = e.invoke(b"hello".to_vec());
        assert_eq!(inf.key, invocation_key(&e.manifest(), b"hello"));
        assert_ne!(inf.key, IdempotencyKey::default());
    }

    // A conformant deterministic engine passes enforcement; the result carries the key.
    #[test]
    fn enforced_accepts_a_conformant_deterministic_engine() {
        let e = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
        let inf = invoke_enforced(&e, b"hello".to_vec()).expect("conformant engine accepted");
        assert_eq!(inf.key, invocation_key(&e.manifest(), b"hello"));
    }

    /// An adversarial engine that self-mints a wrong key (the pre-RCR-012 behaviour).
    struct MisKeyed;
    impl Engine for MisKeyed {
        type Input = Vec<u8>;
        fn manifest(&self) -> EngineManifest {
            EngineManifest { name: "mis.keyed".into(), version: "1.0.0".into(), ..Default::default() }
        }
        fn invoke(&self, input: Vec<u8>) -> Inference {
            Inference { key: IdempotencyKey("self-minted".into()), output: input, proposed_effects: vec![] }
        }
    }

    // The fabric refuses a mis-keyed engine BEFORE any effect is considered — the key
    // is the fabric's derivation, not the engine's claim.
    #[test]
    fn enforced_refuses_a_mis_keyed_engine() {
        match invoke_enforced(&MisKeyed, b"x".to_vec()) {
            Err(FabricViolation::KeyMismatch { expected, got }) => {
                assert_eq!(got.0, "self-minted");
                assert!(expected.0.contains("mis.keyed@1.0.0"));
            }
            other => panic!("expected KeyMismatch, got {other:?}"),
        }
    }

    /// An engine that DECLARES Deterministic but embeds a per-invocation counter in its
    /// output — the exact false promise the v1.1 debt said must be enforced, not trusted.
    struct FalselyDeterministic {
        ctr: Cell<u64>,
    }
    impl Engine for FalselyDeterministic {
        type Input = Vec<u8>;
        fn manifest(&self) -> EngineManifest {
            EngineManifest {
                name: "liar".into(),
                version: "1.0.0".into(),
                determinism: Determinism::Deterministic,
                ..Default::default()
            }
        }
        fn invoke(&self, input: Vec<u8>) -> Inference {
            let n = self.ctr.get();
            self.ctr.set(n + 1);
            let mut output = input.clone();
            output.extend_from_slice(&n.to_be_bytes());
            Inference { key: invocation_key(&self.manifest(), &input), output, proposed_effects: vec![] }
        }
    }

    // The double-invoke probe catches the false Determinism declaration.
    #[test]
    fn enforced_refuses_a_false_determinism_declaration() {
        let e = FalselyDeterministic { ctr: Cell::new(0) };
        assert_eq!(invoke_enforced(&e, b"x".to_vec()), Err(FabricViolation::NondeterministicOutput));
    }

    /// A declared-Nondeterministic engine: varying output is LAWFUL (its recorded
    /// inference is authoritative on replay, ORCH-003) — but its key must still be the
    /// fabric derivation.
    struct DeclaredNondet {
        ctr: Cell<u64>,
    }
    impl Engine for DeclaredNondet {
        type Input = Vec<u8>;
        fn manifest(&self) -> EngineManifest {
            EngineManifest {
                name: "oracle".into(),
                version: "1.0.0".into(),
                determinism: Determinism::Nondeterministic,
                ..Default::default()
            }
        }
        fn invoke(&self, input: Vec<u8>) -> Inference {
            let n = self.ctr.get();
            self.ctr.set(n + 1);
            Inference { key: invocation_key(&self.manifest(), &input), output: n.to_be_bytes().to_vec(), proposed_effects: vec![] }
        }
    }

    #[test]
    fn enforced_allows_declared_nondeterminism_but_still_verifies_the_key() {
        let e = DeclaredNondet { ctr: Cell::new(0) };
        // Accepted: no double-invoke for a declared-Nondeterministic engine.
        let first = invoke_enforced(&e, b"x".to_vec()).expect("declared nondet accepted");
        assert_eq!(first.key, invocation_key(&e.manifest(), b"x"));
        // Exactly ONE invocation was consumed by enforcement (no hidden probe).
        assert_eq!(e.ctr.get(), 1);
    }
}

// ---------------------------------------------------------------------------
// Compile-only sanity: skeleton wiring, no logic (I1).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod skeleton_contract_smoke {
    use super::*;

    /// A trivial engine proving the ABI compiles; encodes no behaviour (I1).
    struct NoopEngine;

    impl Engine for NoopEngine {
        type Input = Vec<u8>;

        fn manifest(&self) -> EngineManifest {
            EngineManifest {
                name: "noop".to_string(),
                version: "0.0.0".to_string(),
                determinism: Determinism::Deterministic,
                idempotency_key: IdempotencyKey::default(),
                reads: Vec::new(),
                produces: Vec::new(),
                capabilities_required: Vec::new(),
            }
        }

        fn invoke(&self, _input: Self::Input) -> Inference {
            Inference::default()
        }
    }

    #[test]
    fn abi_is_object_shaped_and_defaults_exist() {
        let e = NoopEngine;
        let _m = e.manifest();
        let _out = e.invoke(Vec::new());
        assert_eq!(Determinism::default(), Determinism::Deterministic);
    }
}

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
//! I1 CONTRACT-ONLY (by design, not unfinished). This crate defines the Engine
//! Fabric interfaces/types; it carries no engine execution logic. The exercised
//! engine logic in the reference runtime flows through the SDK/Bridge in
//! `products/` (see RUNTIME_FREEZE_v1.0.md, guarantee alignment). Any bodies
//! here are trivial placeholders that exist only so the contract compiles.
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
/// **NON-CONFORMANT re: ORCH-004.** As written, [`invoke`](PureEngine::invoke)
/// does not honour the [`Engine::invoke`] key contract (see the note on its
/// body). Treat this type as illustrative, not as a conformant reference engine.
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
        // NON-CONFORMANT PLACEHOLDER (ORCH-004; violates the Engine::invoke
        // contract documented above — "the returned Inference::key MUST match
        // the idempotency key of this invocation"). This returns an empty
        // IdempotencyKey::default() instead of a key derived from
        // (manifest identity, canonicalized input, read-snapshot), so it is NOT
        // content-addressable and MUST NOT be relied on for replay/dedupe. Left
        // as a placeholder for the I1 CONTRACT-ONLY milestone; deriving the real
        // key is a runtime-forward item, not a doc change.
        Inference {
            key: IdempotencyKey::default(),
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

//! ARVES :: arves-engine-fabric
//!
//! Purpose: Pure, stateless engines behind the Engine ABI; they produce
//! *inference*, never *truth*. An engine is a deterministic-by-default
//! computation that maps an input to an [`Inference`]. Engines own nothing
//! persistent: any state change they wish to see happen is emitted as a
//! *proposed effect* only. Whether those effects ever become truth is decided
//! elsewhere (Control Plane plans it; only the Kernel commits it).
//!
//! Governing: ENG-001..005 (proposed), ORCH-004; Engine Graph Spec.
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
//! - **ENG-001 (proposed): Engine purity.** `invoke` is a pure function of its
//!   input and the declared read-set; no hidden side effects, no ambient I/O.
//! - **ENG-002 (proposed): Engines own no persistent state.** Engines are
//!   stateless between invocations (aligns with OWN-001: one owner per state —
//!   and it is never an engine).
//! - **ENG-003 (proposed): Writes are proposals, not commits.** An engine emits
//!   [`ProposedEffect`]s; it can never itself mutate truth (defers commit to
//!   the Kernel per G-001 / ORCH-001).
//! - **ENG-004 (proposed): Declared reads/produces.** An engine declares what it
//!   [`reads`](EngineManifest::reads) and [`produces`](EngineManifest::produces)
//!   up front in its [`EngineManifest`]; the declaration is a contract the
//!   fabric can check against.
//! - **ENG-005 (proposed): Declared determinism + required capabilities.** An
//!   engine declares its [`Determinism`] class and the
//!   [`capabilities_required`](EngineManifest::capabilities_required) to run.
//! - **ORCH-004: Idempotent + content-addressable invocation.** Every engine
//!   invocation is keyed by an [`IdempotencyKey`] derived from
//!   `(manifest identity, canonicalized input, read-snapshot)`, so replaying the
//!   same invocation yields the same [`Inference`] (supports ORCH-003 replay
//!   from recorded decision trace, not recomputation).
//!
//! ## STATUS
//!
//! I1 skeleton — interfaces/contracts only, NO implementation yet. Frozen
//! specification governs; this crate *implements*, never changes it. Bodies
//! here are trivial placeholders that exist only so the contract compiles;
//! they encode no logic.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Identity & addressing
// ---------------------------------------------------------------------------

/// A stable, human-readable engine name (e.g. `"summarize.text"`).
///
/// The `(name, version)` pair identifies an engine implementation for the
/// purposes of the [`EngineManifest`] and idempotency keying (ENG-004,
/// ORCH-004).
pub type EngineName = String;

/// A version string for an engine implementation.
///
/// Distinct versions are distinct engines: they may differ in behaviour, so a
/// re-version invalidates prior [`IdempotencyKey`]s (ORCH-004).
pub type EngineVersion = String;

/// A declared, content-addressable name of a state slice an engine reads or
/// produces (ENG-004).
///
/// Reads name inputs pulled through the read-only Query layer (QUERY-001);
/// produces name the *shapes* of [`ProposedEffect`]s the engine may emit. These
/// are declarations, not handles: naming a resource here does not grant an
/// engine authority over it (ORCH-001 / OWN-001).
pub type ResourceName = String;

/// A declared capability an engine requires in order to run (ENG-005,
/// CAP-001..009 proposed).
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
// Determinism classification (ENG-005)
// ---------------------------------------------------------------------------

/// The determinism class an engine declares for itself (ENG-005).
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
    /// conservatively as reproducible (ENG-005).
    fn default() -> Self {
        Determinism::Deterministic
    }
}

// ---------------------------------------------------------------------------
// Manifest (ENG-004, ENG-005)
// ---------------------------------------------------------------------------

/// The self-description an engine publishes to the fabric (ENG-004, ENG-005).
///
/// The manifest is a *contract*: it declares identity, determinism, the
/// idempotency-key scheme, and the read/produce/capability sets up front so the
/// fabric can plan, key, and audit invocations without executing them. A
/// manifest describes intent and shape only — it grants no authority and owns no
/// state (ORCH-001, OWN-001).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EngineManifest {
    /// Stable engine name; with `version`, identifies the implementation
    /// (ENG-004).
    pub name: EngineName,

    /// Implementation version; a bump is a distinct engine and invalidates
    /// prior idempotency keys (ORCH-004).
    pub version: EngineVersion,

    /// Declared determinism class (ENG-005). See [`Determinism`].
    pub determinism: Determinism,

    /// The scheme/version tag describing how this engine's [`IdempotencyKey`] is
    /// derived (ORCH-004). Naming the scheme in the manifest lets the fabric
    /// reject invocations keyed under an incompatible scheme.
    pub idempotency_key: IdempotencyKey,

    /// Declared read-set: the resources this engine may read, pulled through the
    /// read-only Query layer (ENG-004, QUERY-001). Reading outside this set is a
    /// contract violation.
    pub reads: Vec<ResourceName>,

    /// Declared produce-set: the shapes of [`ProposedEffect`]s this engine may
    /// emit (ENG-004). Producing anything outside this set is a contract
    /// violation; nothing here is a commitment to *persist* (ENG-003).
    pub produces: Vec<ResourceName>,

    /// Declared capabilities required to run (ENG-005, CAP-001..009 proposed).
    /// Resolved by the Capability layer; the Engine layer only declares them.
    pub capabilities_required: Vec<CapabilityName>,
}

// ---------------------------------------------------------------------------
// Proposed effects (ENG-003)
// ---------------------------------------------------------------------------

/// A write an engine *wishes* to see happen — a proposal, never a commit
/// (ENG-003).
///
/// Engines are pure and own no persistent state (ENG-001, ENG-002, OWN-001), so
/// they cannot mutate truth. Instead they emit `ProposedEffect`s naming the
/// intended change. Whether a proposal becomes truth is decided by the Control
/// Plane (as a plan, ORCH-002) and committed exclusively by the Kernel through
/// the shard leader (G-001, ORCH-001; IDR: engines run anywhere, commit only via
/// shard leader).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProposedEffect {
    /// The declared resource this effect targets; MUST appear in the engine
    /// manifest's [`produces`](EngineManifest::produces) set (ENG-004).
    pub target: ResourceName,

    /// Opaque, serialized payload describing the proposed change. The Engine
    /// layer treats this as inert bytes; interpretation and commit belong to the
    /// Kernel (ENG-003, G-001).
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
/// (ENG-003).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Inference {
    /// The idempotency key this inference was computed under; ties the output to
    /// its invocation for replay/dedupe (ORCH-003, ORCH-004).
    pub key: IdempotencyKey,

    /// Opaque, serialized primary output of the engine (the "inference" proper).
    pub output: Vec<u8>,

    /// Proposed effects the engine would like committed — proposals only
    /// (ENG-003).
    pub proposed_effects: Vec<ProposedEffect>,
}

// ---------------------------------------------------------------------------
// The Engine ABI (ENG-001..005, ORCH-004)
// ---------------------------------------------------------------------------

/// The pure Engine ABI (ENG-001, ORCH-004).
///
/// An `Engine` is a **pure**, **stateless** computation: given an input (and,
/// implicitly, the read-snapshot addressed by the [`IdempotencyKey`]) it returns
/// an [`Inference`]. Implementations MUST honour the following contract:
///
/// - **Purity (ENG-001).** [`invoke`](Engine::invoke) has no side effects beyond
///   returning its result. No ambient I/O, no clocks, no RNG except a recorded
///   seed for [`Determinism::Seeded`] (ENG-005).
/// - **No ownership (ENG-002, OWN-001).** The engine holds no persistent state
///   across invocations; `&self` is configuration/identity only.
/// - **Proposals, not commits (ENG-003).** Any desired write is returned as a
///   [`ProposedEffect`] inside the [`Inference`]; the engine never commits truth
///   (G-001, ORCH-001).
/// - **Declared surface (ENG-004, ENG-005).** Behaviour stays within the
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

    /// Return this engine's manifest (ENG-004, ENG-005). Pure, cheap, and
    /// invariant across invocations.
    fn manifest(&self) -> EngineManifest;

    /// Purely compute an [`Inference`] from `input` (ENG-001, ORCH-004).
    ///
    /// Implementations MUST NOT mutate external truth, perform ambient I/O, or
    /// retain state; all intended writes are returned as
    /// [`ProposedEffect`]s (ENG-003). The returned [`Inference::key`] MUST match
    /// the idempotency key of this invocation (ORCH-004).
    fn invoke(&self, input: Self::Input) -> Inference;
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

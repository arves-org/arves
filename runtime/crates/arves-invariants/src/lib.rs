//! ARVES :: arves-invariants
//!
//! Purpose: Machine-checkable invariant catalog + property-test scaffolding for
//!          runtime proofs. This crate is the single source of *identifiers* for
//!          the registered ARVES invariants, and the trait surface by which the
//!          reference implementation demonstrates that it upholds them.
//! Governing: ORCH-001..004, OWN-001, LAYER-001, SHARD-001 (registered).
//!            PROPOSED (informative, pending CCP): G-001, QUERY-001, LCW-001,
//!            PERSIST-001, CAP-001..009, ENG-001..005.
//! Layer: cross-cutting (spans every layer; owns no truth of its own).
//!
//! STATUS: I1 IMPLEMENTED (catalog). Provides the populated invariant-identifier
//! catalog (ORCH-001..004, OWN/LAYER/SHARD-001 constants + `REGISTERED` list),
//! concrete `Layer` rank / `may_depend_on` logic, and the property-test trait
//! surface. The frozen specification governs; this crate implements, never
//! changes it.
//!
//! # Position in the ARVES chain
//!
//! ARVES flows Theory -> Spec -> Contracts -> Behaviour -> Conformance ->
//! Implementation. The *implementation proves the spec; it never changes it.*
//! This crate sits at the Contracts/Conformance seam: the [`catalog`] module
//! reifies the frozen invariant IDs as `&'static str` constants so that runtime
//! code, conformance harnesses, and decision traces can reference an invariant
//! by a compiler-checked symbol rather than a stringly-typed literal. The
//! [`PropertyCheck`] trait scaffold is how a subject under test asserts, at
//! runtime, that a named invariant held for a concrete observation.
//!
//! # Two planes
//!
//! ARVES separates a Control Plane (decides; owns no truth) from a Data Plane
//! (carries truth). Several invariants here (`ORCH-001..004`) constrain the
//! Control Plane; others (`OWN-001`, `SHARD-001`) constrain ownership and
//! partitioning of Data Plane state; `LAYER-001` constrains the whole stack.
//!
//! Nothing in this crate *owns* truth (per `ORCH-001`): it only *describes*
//! invariants and offers a checking surface. It carries NO dependencies and is
//! std-only by design, so it can be linked by any layer without inverting the
//! downward-only dependency rule of `LAYER-001`.

#![forbid(unsafe_code)]

// ===========================================================================
// Invariant identity
// ===========================================================================

/// The lifecycle status of an invariant within the ARVES governance process.
///
/// ARVES freezes its Spec Era: `Registered` invariants are normative and MUST
/// hold. `Proposed` invariants are *informative* and pending ratification by
/// the Change Control Process (CCP); they may be checked opportunistically but
/// carry no conformance weight until registered.
///
/// Design principles (the `O-001..007` ontology set) are captured as
/// [`InvariantStatus::DesignPrinciple`]: they shape design but are not
/// point-in-time runtime assertions in the way registered invariants are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvariantStatus {
    /// Normative and frozen; MUST hold. (e.g. `ORCH-001..004`, `OWN-001`,
    /// `LAYER-001`, `SHARD-001`.)
    Registered,
    /// Informative, pending CCP ratification. (e.g. `G-001`, `QUERY-001`,
    /// `LCW-001`, `PERSIST-001`, `CAP-001..009`, `ENG-001..005`.)
    Proposed,
    /// A design principle rather than a runtime assertion. (e.g. `O-001..007`.)
    DesignPrinciple,
}

/// Which ARVES plane an invariant primarily governs.
///
/// Cross-cutting invariants (such as `LAYER-001` and `SHARD-001`) constrain
/// both planes; `Neither` is reserved for invariants that describe the *seam*
/// between planes rather than a single plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Plane {
    /// Governs the Control Plane, which decides but owns no truth.
    Control,
    /// Governs the Data Plane, which carries truth.
    Data,
    /// Cross-cutting: constrains both planes or the whole layer stack.
    CrossCutting,
}

/// A compiler-checkable descriptor of a single registered/proposed invariant.
///
/// The canonical instances live in [`catalog`]. Runtime code should refer to an
/// invariant by its [`InvariantId::id`] (a stable `&'static str` such as
/// `"ORCH-001"`), which is exactly the token recorded into a decision trace or
/// a conformance report. The struct is deliberately data-only: it carries no
/// behaviour, so it can never accrete truth (`ORCH-001`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvariantId {
    /// Stable identifier token, e.g. `"ORCH-001"`. This is the citation key.
    pub id: &'static str,
    /// One-line normative statement of the invariant.
    pub statement: &'static str,
    /// Governance lifecycle status (registered / proposed / design principle).
    pub status: InvariantStatus,
    /// Which plane the invariant primarily governs.
    pub plane: Plane,
}

impl InvariantId {
    /// Construct a descriptor. `const` so the catalog can be fully evaluated at
    /// compile time.
    pub const fn new(
        id: &'static str,
        statement: &'static str,
        status: InvariantStatus,
        plane: Plane,
    ) -> Self {
        Self {
            id,
            statement,
            status,
            plane,
        }
    }
}

// ===========================================================================
// Catalog of registered invariants
// ===========================================================================

/// The catalog of ARVES invariant identifiers.
///
/// Each `&'static str` const is the *citation key* for one invariant; each
/// [`InvariantId`] const bundles that key with its normative statement,
/// governance status, and plane. Downstream crates should cite these symbols
/// (e.g. `arves_invariants::catalog::ORCH_001`) rather than duplicating string
/// literals, so a rename or restatement is caught by the compiler.
///
/// The catalog is intentionally exhaustive for the *registered* set and offers
/// the currently named *proposed* set for convenience; proposed entries carry
/// [`InvariantStatus::Proposed`] and MUST NOT be treated as normative until the
/// CCP registers them.
pub mod catalog {
    use super::{InvariantId, InvariantStatus, Plane};

    // --- ORCH: Control Plane orchestration invariants (registered) ---------

    /// `ORCH-001`: The Control Plane owns no truth; only the Kernel owns truth.
    ///
    /// The plane that *decides* must never become a plane that *remembers*.
    /// Any persistent authoritative state observed on the Control Plane is a
    /// violation. See also `OWN-001` (one owner per state) and the proposed
    /// `G-001` (Kernel is sole truth + commit gateway).
    pub const ORCH_001: &str = "ORCH-001";

    /// `ORCH-002`: The Control Plane produces plans, never persistent state.
    ///
    /// Its outputs are *plans* (proposed sequences of Data Plane invocations),
    /// which are ephemeral inputs to the truth path, not durable records.
    pub const ORCH_002: &str = "ORCH-002";

    /// `ORCH-003`: Replay is from a recorded decision trace, not recomputation.
    ///
    /// To reconstruct a past state, the runtime replays committed outcomes from
    /// the recorded trace (the Raft log doubles as WAL and decision trace, per
    /// `IDR-*`). Re-deriving decisions by re-running the Control Plane is a
    /// violation, because that would make the (truth-less) Control Plane
    /// authoritative.
    pub const ORCH_003: &str = "ORCH-003";

    /// `ORCH-004`: Every engine/capability invocation is idempotent and
    /// content-addressable.
    ///
    /// Re-issuing the same invocation (same content-address) yields the same
    /// outcome and commits at most once. This is what makes replay (`ORCH-003`)
    /// and at-least-once delivery safe on the truth path.
    pub const ORCH_004: &str = "ORCH-004";

    // --- OWN / LAYER / SHARD: structural invariants (registered) -----------

    /// `OWN-001`: Exactly one owner per unit of state.
    ///
    /// No two components may claim authority over the same state. Combined with
    /// `ORCH-001`, this pins truth ownership to the Kernel.
    pub const OWN_001: &str = "OWN-001";

    /// `LAYER-001`: Layer dependencies are downward-only.
    ///
    /// The stack is Reality -> Information Platform -> Kernel -> Persistence ->
    /// LCW -> Query -> Engine -> Capability -> Execution, alongside the Control
    /// Plane. A layer may depend only on layers below it; upward or cyclic
    /// dependencies are violations. See [`LayerRank`].
    pub const LAYER_001: &str = "LAYER-001";

    /// `SHARD-001`: State is partitioned by tenant/workspace, and the shard key
    /// is immutable.
    ///
    /// Each tenant/workspace maps to a shard (one per-shard Raft group, per
    /// `IDR-001`). Once assigned, a datum's shard key never changes; there is
    /// no cross-shard atomic commit (coordination is via sagas).
    pub const SHARD_001: &str = "SHARD-001";

    // --- Rich descriptors for the registered set ---------------------------

    /// [`InvariantId`] descriptor for [`ORCH_001`].
    pub const ORCH_001_DESC: InvariantId = InvariantId::new(
        ORCH_001,
        "Control Plane owns no truth; only the Kernel owns truth.",
        InvariantStatus::Registered,
        Plane::Control,
    );

    /// [`InvariantId`] descriptor for [`ORCH_002`].
    pub const ORCH_002_DESC: InvariantId = InvariantId::new(
        ORCH_002,
        "Control Plane produces plans, never persistent state.",
        InvariantStatus::Registered,
        Plane::Control,
    );

    /// [`InvariantId`] descriptor for [`ORCH_003`].
    pub const ORCH_003_DESC: InvariantId = InvariantId::new(
        ORCH_003,
        "Replay from recorded decision trace, not recomputation.",
        InvariantStatus::Registered,
        Plane::Control,
    );

    /// [`InvariantId`] descriptor for [`ORCH_004`].
    pub const ORCH_004_DESC: InvariantId = InvariantId::new(
        ORCH_004,
        "Every engine/capability invocation is idempotent and content-addressable.",
        InvariantStatus::Registered,
        Plane::CrossCutting,
    );

    /// [`InvariantId`] descriptor for [`OWN_001`].
    pub const OWN_001_DESC: InvariantId = InvariantId::new(
        OWN_001,
        "Exactly one owner per unit of state.",
        InvariantStatus::Registered,
        Plane::Data,
    );

    /// [`InvariantId`] descriptor for [`LAYER_001`].
    pub const LAYER_001_DESC: InvariantId = InvariantId::new(
        LAYER_001,
        "Layer dependencies are downward-only.",
        InvariantStatus::Registered,
        Plane::CrossCutting,
    );

    /// [`InvariantId`] descriptor for [`SHARD_001`].
    pub const SHARD_001_DESC: InvariantId = InvariantId::new(
        SHARD_001,
        "Partition by tenant/workspace; shard key is immutable.",
        InvariantStatus::Registered,
        Plane::Data,
    );

    /// All registered (normative, frozen) invariant descriptors, in ID order.
    ///
    /// Conformance harnesses iterate this array to enumerate the obligations a
    /// subject under test must satisfy.
    pub const REGISTERED: &[InvariantId] = &[
        ORCH_001_DESC,
        ORCH_002_DESC,
        ORCH_003_DESC,
        ORCH_004_DESC,
        OWN_001_DESC,
        LAYER_001_DESC,
        SHARD_001_DESC,
    ];
}

// ===========================================================================
// LAYER-001: downward-only layer ordering
// ===========================================================================

/// The ARVES layer stack, ordered from highest (`Reality`) to lowest
/// (`Execution`), with the `ControlPlane` running alongside.
///
/// Governing invariant: `LAYER-001` (dependencies are downward-only). The
/// discriminant ordering encodes rank so that a dependency check can compare
/// two layers: a layer may depend only on layers with a numerically greater
/// rank (i.e. lower in the stack). See [`Layer::rank`] and
/// [`Layer::may_depend_on`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// Top of stack: the external world / ground truth source.
    Reality = 0,
    /// Information Platform: ingest/normalization of Reality into the system.
    InformationPlatform = 1,
    /// Kernel: sole owner of cognitive truth and the commit gateway.
    Kernel = 2,
    /// Persistence: durable store beneath the Kernel.
    Persistence = 3,
    /// LCW: owns Working Memory (proposed `LCW-001`).
    Lcw = 4,
    /// Query: read-only projection layer (proposed `QUERY-001`).
    Query = 5,
    /// Engine: derivation/compute over queried state.
    Engine = 6,
    /// Capability: schedulable capability surface above execution.
    Capability = 7,
    /// Execution: bottom of the primary stack; runs invocations.
    Execution = 8,
    /// Control Plane: decides; owns no truth (`ORCH-001`). Runs *alongside* the
    /// stack rather than within its linear ordering.
    ControlPlane = 9,
}

/// A numeric rank used to compare layer depth for `LAYER-001` checks.
///
/// Lower rank = higher in the stack (closer to Reality). A layer may depend
/// only on layers of strictly greater rank.
pub type LayerRank = u8;

impl Layer {
    /// The layer's depth rank. Higher value = lower in the stack.
    ///
    /// Cites `LAYER-001`: rank is the basis for the downward-only test.
    pub const fn rank(self) -> LayerRank {
        self as u8
    }

    /// Whether `self` is permitted to depend on `other` under `LAYER-001`.
    ///
    /// True iff `other` is strictly lower in the stack (greater rank). The
    /// `ControlPlane` runs alongside the stack and is excluded from the linear
    /// downward-only ordering; a full implementation gates it separately (it
    /// owns no truth per `ORCH-001`). This scaffold implements only the linear
    /// comparison for the primary stack.
    pub const fn may_depend_on(self, other: Layer) -> bool {
        (self.rank() as u16) < (other.rank() as u16)
    }
}

// ===========================================================================
// SHARD-001: partitioning
// ===========================================================================

/// An opaque, immutable shard key deriving a partition from a tenant/workspace.
///
/// Governing invariant: `SHARD-001` (partition by tenant/workspace; key is
/// immutable). The inner value is private and there is deliberately no setter:
/// once constructed, a `ShardKey` cannot be mutated, encoding immutability in
/// the type. Each shard corresponds to one per-shard Raft group (`IDR-001`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShardKey(String);

impl ShardKey {
    /// Derive a shard key from a tenant and workspace identifier.
    ///
    /// Cites `SHARD-001`. The resulting key is immutable for the lifetime of
    /// the datum it partitions; there is no cross-shard atomic commit
    /// (`IDR-*`), so callers must not attempt to re-key data across shards.
    pub fn new(tenant: &str, workspace: &str) -> Self {
        // Trivial, deterministic composition; a real implementation would use a
        // stable content-addressed encoding. Kept simple to compile clean.
        let mut s = String::with_capacity(tenant.len() + workspace.len() + 1);
        s.push_str(tenant);
        s.push('/');
        s.push_str(workspace);
        ShardKey(s)
    }

    /// Borrow the opaque key token. No mutation is exposed (`SHARD-001`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ===========================================================================
// PropertyCheck: runtime-proof trait scaffold
// ===========================================================================

/// The identity + verdict of a single invariant check.
///
/// A [`CheckOutcome`] is the atom recorded when a subject under test asserts an
/// invariant. On the truth path these outcomes are what get committed and
/// replayed (`ORCH-003`), never the computation that produced them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The invariant held for the observed sample.
    Holds,
    /// The invariant was violated for the observed sample.
    Violated,
    /// The invariant did not apply to this sample (vacuously satisfied).
    NotApplicable,
}

/// A structured report of one [`PropertyCheck`] evaluation.
///
/// Carries the cited invariant id and the verdict. Deliberately data-only so it
/// can be serialized into a conformance report or decision trace without the
/// checker owning any truth (`ORCH-001`).
#[derive(Debug, Clone)]
pub struct CheckReport {
    /// The invariant that was checked (its `&'static str` citation key).
    pub invariant: &'static str,
    /// The verdict of the check.
    pub outcome: CheckOutcome,
    /// Optional human-readable detail (e.g. the violating witness).
    pub detail: Option<String>,
}

impl CheckReport {
    /// Build a report for `invariant` with `outcome` and no detail.
    pub fn new(invariant: &'static str, outcome: CheckOutcome) -> Self {
        Self {
            invariant,
            outcome,
            detail: None,
        }
    }

    /// True iff this report does not represent a violation.
    pub fn passed(&self) -> bool {
        !matches!(self.outcome, CheckOutcome::Violated)
    }
}

/// A property-based runtime proof that a subject upholds one named invariant.
///
/// This is the core scaffold for runtime proofs. An implementor binds itself to
/// a single invariant via [`PropertyCheck::INVARIANT`] (one of the [`catalog`]
/// citation keys) and, given an observed `Subject` sample, returns a
/// [`CheckReport`]. Property tests / the conformance harness generate `Subject`
/// samples and require every produced report to [`CheckReport::passed`].
///
/// Governing invariants (by construction of the surface):
/// - The check *observes* and *reports*; it never mutates the subject's truth
///   (`ORCH-001`, `OWN-001`) and produces no persistent state (`ORCH-002`).
/// - Because the trait is pure and deterministic over its `Subject`, the same
///   input yields the same report, aligning with idempotence/content-addressing
///   (`ORCH-004`) and trace-based replay (`ORCH-003`).
///
/// Method bodies are intentionally omitted (signatures only): this crate is a
/// skeleton of contracts, not logic.
pub trait PropertyCheck {
    /// The concrete observation this check is evaluated against.
    ///
    /// For `LAYER-001` this might be a pair of [`Layer`]s; for `SHARD-001` a
    /// [`ShardKey`] transition; for `OWN-001` an ownership assignment.
    type Subject;

    /// The invariant this checker proves, cited by its [`catalog`] key.
    ///
    /// e.g. `const INVARIANT: &'static str = catalog::LAYER_001;`
    const INVARIANT: &'static str;

    /// Evaluate the invariant against one observed `subject`.
    ///
    /// Must be pure and side-effect free (owns no truth; `ORCH-001`).
    fn check(&self, subject: &Self::Subject) -> CheckReport;

    /// Convenience: the invariant descriptor from the catalog, if registered.
    ///
    /// Default scaffold returns `None`; a full implementation would look up
    /// [`catalog::REGISTERED`] by [`PropertyCheck::INVARIANT`].
    fn descriptor(&self) -> Option<InvariantId> {
        None
    }
}

/// A collection of [`PropertyCheck`]-style obligations that a subject-under-test
/// claims to satisfy, for the conformance harness to drive.
///
/// This is the fan-out surface: the conformance layer asks a candidate
/// implementation for its [`Suite::obligations`] (the invariant ids it commits
/// to upholding) and then samples each. Kept as a trait so the harness depends
/// only on this contract, preserving downward-only layering (`LAYER-001`).
pub trait Suite {
    /// The registered invariant ids this suite promises to uphold.
    fn obligations(&self) -> &'static [&'static str];
}

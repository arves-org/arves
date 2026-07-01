//! ARVES :: arves-ontology
//!
//! Purpose: `uci.*` cognitive type registry + mandatory aspects
//! (Identity / Provenance / Trust / Temporal / TenantScope) that every
//! cognitive entity in the ARVES Data Plane must carry.
//!
//! Governing: `O-001..007` (Ontology Design Principles, Ontology Spec Part 3 —
//! *definitional, NOT runtime-provable*; they shape types, not proofs).
//! Cross-cutting registered invariants this crate must not contradict:
//! `OWN-001` (one owner per state), `SHARD-001` (immutable tenant/workspace
//! partition key), `ORCH-001` (only the Kernel owns truth), `LAYER-001`
//! (downward-only layering).
//!
//! Layer: Data Plane / type system. This crate defines the *vocabulary* the
//! Data Plane carries and the Kernel commits; it owns no runtime truth and
//! produces no persistent state.
//!
//! Chain position: Theory -> Spec -> **Contracts** -> Behaviour -> Conformance
//! -> Implementation. This crate sits at the Contracts stage: it is a
//! SKELETON of contracts, never logic. The frozen specification governs; this
//! crate implements it and never changes it.
//!
//! ## The seven Ontology Design Principles (`O-001..007`)
//!
//! Verbatim from the frozen **Universal Cognitive Ontology Specification,
//! Part 3** (citations in this crate were aligned to these statements as a
//! post-verification traceability fix). They are design principles, **not**
//! registered invariants, and are therefore never subjected to the
//! executable-proof obligation:
//!
//! - `O-001` — *Everything is a Cognitive Entity.* Realized by the
//!   [`CognitiveEntity`] super-trait and the [`RootType`] space.
//! - `O-002` — *Every Entity has Identity.* Realized by the [`Identity`] aspect
//!   and [`EntityUrn`].
//! - `O-003` — *Every Observation has Provenance.* Realized by the
//!   [`Provenance`] aspect and [`ProvenanceRecord`] / [`Origin`].
//! - `O-004` — *Truth emerges from validated Evidence.* Realized by the
//!   [`Trust`] aspect and [`Confidence`] (evidence-weighted assurance).
//! - `O-005` — *Derivation is not Inheritance* (lineage edges are relations,
//!   not subtypes). Realized by composing aspects rather than subclassing, by
//!   [`Origin::Derived`] as a lineage relation, and by the composed
//!   (non-inherited) [`RootType`] space.
//! - `O-006` — *Every type is versioned and registered.* Realized by
//!   [`TypeVersion`] / [`TypeRegistration`] / [`TypeRegistry`].
//! - `O-007` — *The Ontology defines meaning, not storage; truth is owned by
//!   the Kernel* (`ORCH-001`). Types here are storage-independent meaning only.
//!
//! Note: temporality (the [`Temporal`] aspect / [`BiTemporal`]) is NOT an
//! O-principle; it derives from the frozen temporal model and supports `O-003`
//! (provenance is time-situated) and `ORCH-003` (replay from a recorded trace).
//!
//! STATUS: I1 skeleton — interfaces/contracts only, NO implementation yet.
//! Method bodies are omitted; the few defaulted/const bodies present are
//! trivial and carry no logic.

#![forbid(unsafe_code)]

// =============================================================================
// Identity primitives  (O-002 identity, O-006 registration, O-007, SHARD-001)
// =============================================================================

/// Namespace of a cognitive type or entity within the `uci.*` type space.
///
/// Governing: `O-006` (versioned + registered type space), `O-007` (a namespace
/// carries meaning only, never storage location).
///
/// A namespace is the dotted authority segment of an [`EntityUrn`] — e.g.
/// `uci.core`, `uci.observation`. It scopes type names so that independently
/// authored ontologies cannot collide.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Namespace(pub String);

/// Monotonic, immutable version stamp of a *registered type* (not of an
/// entity instance).
///
/// Governing: `O-006` (every type is versioned and registered).
///
/// A type version, once registered in the [`TypeRegistry`], is frozen: a new
/// meaning is a new version, never an in-place edit. This mirrors the frozen
/// Spec Era discipline of the corpus at the ontology level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVersion {
    /// Breaking-change counter for the type's meaning.
    pub major: u32,
    /// Backward-compatible refinement counter.
    pub minor: u32,
}

/// Stable, storage-independent identity of a cognitive entity.
///
/// Governing: `O-002` (every entity has identity — the anchoring principle of
/// this type), `O-007` (identity is meaning, not a storage address),
/// `SHARD-001` (the tenant/workspace partition an entity belongs to is
/// immutable, so identity implies a fixed shard).
///
/// # Shape
///
/// An `EntityUrn` is a URN of the canonical form:
///
/// ```text
/// urn:arves:<namespace>:<type-name>@<major>.<minor>:<local-id>
/// ```
///
/// The identity is *content- and location-independent*: the same entity keeps
/// the same `EntityUrn` regardless of which replica, shard leader, or storage
/// engine currently holds it. This satisfies `O-002` and keeps the ontology
/// decoupled from persistence (`O-007`).
///
/// # Immutability
///
/// Per `SHARD-001`, the tenant/workspace an entity is partitioned by is part of
/// its identity contract and MUST NOT change over the entity's lifetime. Per
/// `OWN-001`, exactly one owner (the Kernel of the owning shard) may bind an
/// `EntityUrn` to committed truth.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityUrn {
    /// Authority namespace within the `uci.*` type space (`O-006`).
    pub namespace: Namespace,
    /// Registered type name of the entity (`O-006`).
    pub type_name: String,
    /// Version of the *type* this entity conforms to (`O-006`).
    pub type_version: TypeVersion,
    /// Locally-unique, immutable identifier of the individual entity (`O-002`).
    pub local_id: String,
}

impl EntityUrn {
    /// URN scheme prefix shared by all ARVES cognitive identities.
    ///
    /// Governing: `O-002`. A const marker only — carries no logic.
    pub const SCHEME: &'static str = "urn:arves";
}

// =============================================================================
// Tenant / workspace scope  (SHARD-001, OWN-001)
// =============================================================================

/// Immutable partition coordinate identifying the tenant that owns an entity.
///
/// Governing: `SHARD-001` (partition by tenant; key is immutable).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenantId(pub String);

/// Immutable partition coordinate identifying the workspace within a tenant.
///
/// Governing: `SHARD-001` (partition by tenant/workspace; key is immutable).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceId(pub String);

/// The immutable `(tenant, workspace)` partition key of an entity — the
/// sharding coordinate for per-shard Raft groups.
///
/// Governing: `SHARD-001` (one Raft group per tenant/workspace; key immutable),
/// `OWN-001` (the shard leader is the single owner of that partition's truth).
///
/// Every cognitive entity is bound to exactly one `ShardKey`; the binding is
/// established at creation and never mutated (there is no cross-shard atomic
/// commit — cross-partition change is saga-orchestrated, per IDR).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShardKey {
    /// Owning tenant (`SHARD-001`).
    pub tenant: TenantId,
    /// Owning workspace within the tenant (`SHARD-001`).
    pub workspace: WorkspaceId,
}

// =============================================================================
// Provenance / trust / temporal value types  (O-003, O-004, O-005; temporal
// supports O-003 + ORCH-003)
// =============================================================================

/// Discriminates how a datum came to exist, feeding [`Provenance`].
///
/// Governing: `O-003` (every observation has provenance), `O-005` (derivation
/// is a lineage *relation*, not inheritance — see [`Origin::Derived`]). Aligns
/// with `ORCH-003` (replay from a recorded decision trace): observed/derived
/// facts must record enough origin to be reconstructed, not recomputed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Origin {
    /// Ingested from outside the runtime (a raw observation of Reality).
    Observed,
    /// Produced by an engine/capability invocation over prior truth. Derivation
    /// is recorded as a lineage relation to the producing invocation, NOT as
    /// type inheritance (`O-005`); the id is content-addressable (`ORCH-004`).
    Derived {
        /// Content-addressable id of the producing invocation (`ORCH-004`).
        invocation_id: String,
    },
    /// Asserted by an external principal (e.g. a human or upstream system).
    Asserted,
}

/// Universal origin record attached to every cognitive entity.
///
/// Governing: `O-003` (every observation has provenance — nothing is anonymous),
/// `ORCH-001` (only the Kernel commits the truth this describes),
/// `ORCH-004` (derived origins are content-addressable & idempotent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceRecord {
    /// How this datum came to exist (`O-003`).
    pub origin: Origin,
    /// Identity of the source/principal responsible for the datum (`O-003`).
    pub source: String,
}

/// Explicit, inspectable confidence attached to a cognitive entity.
///
/// Governing: `O-004` (truth emerges from validated evidence — confidence is
/// the evidence-weighted assurance that a claim is true, made first-class and
/// inspectable rather than assumed).
///
/// Modelled as a normalized `[0.0, 1.0]` confidence. The ontology defines the
/// *meaning* of trust (`O-007`); scoring policy lives elsewhere (Control Plane
/// / capability policy).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Confidence(pub f64);

/// A timestamp expressed as nanoseconds since the Unix epoch (UTC).
///
/// Governing: supports `ORCH-003` (replay). A pure value type; the clock source
/// and physical representation are storage concerns (`O-007`). Temporality is
/// not itself an O-principle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(pub i64);

/// Bitemporal coordinate: when a fact was true of the world vs. when the
/// system recorded it.
///
/// Governing: supports `O-003` (provenance is time-situated) and `ORCH-003`
/// (replay). Separating *valid time* from *transaction time* is what lets
/// `ORCH-003` replay produce the same answer the system had at a past instant.
/// Temporality is an aspect derived from the frozen temporal model, not an
/// O-principle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BiTemporal {
    /// When the fact became true of the world (valid time).
    pub valid_from: Timestamp,
    /// When the runtime learned/recorded the fact (transaction time).
    pub recorded_at: Timestamp,
}

// =============================================================================
// Mandatory aspect traits  (O-001 realized via the aspect bundle:
// Identity=O-002, Provenance=O-003, Trust=O-004, Temporal supports ORCH-003,
// TenantScope=SHARD-001)
// =============================================================================

/// Aspect: identity. Every cognitive entity exposes its [`EntityUrn`].
///
/// Governing: `O-002` (every entity has identity).
///
/// Contract: the returned identity is immutable for the entity's lifetime and
/// is independent of storage/representation (`O-007`).
pub trait Identity {
    /// The stable, storage-independent identity of this entity (`O-002`).
    fn urn(&self) -> &EntityUrn;
}

/// Aspect: provenance. Every cognitive entity exposes how it came to be.
///
/// Governing: `O-003` (every observation has provenance).
pub trait Provenance {
    /// The origin/source record for this entity (`O-003`).
    fn provenance(&self) -> &ProvenanceRecord;
}

/// Aspect: trust. Every cognitive entity exposes explicit confidence.
///
/// Governing: `O-004` (truth emerges from validated evidence — trust is
/// explicit and evidence-weighted, never an implicit assumption).
pub trait Trust {
    /// The inspectable, evidence-weighted confidence of this entity (`O-004`).
    fn confidence(&self) -> Confidence;
}

/// Aspect: temporality. Every cognitive entity exposes bitemporal coordinates.
///
/// Governing: supports `ORCH-003` (replay) and `O-003` (provenance is
/// time-situated). Not an O-principle in its own right.
pub trait Temporal {
    /// The valid-time / transaction-time coordinate of this entity.
    fn temporal(&self) -> BiTemporal;
}

/// Aspect: tenant scope. Every cognitive entity is bound to an immutable shard.
///
/// Governing: `SHARD-001` (immutable tenant/workspace partition key),
/// `OWN-001` (single owner per partition).
///
/// Contract: the returned [`ShardKey`] is fixed at creation and never changes.
pub trait TenantScope {
    /// The immutable `(tenant, workspace)` partition of this entity
    /// (`SHARD-001`).
    fn shard_key(&self) -> &ShardKey;
}

// =============================================================================
// The cognitive entity super-trait  (O-001)
// =============================================================================

/// Root classification of every cognitive entity in the `uci.*` type space.
///
/// Governing: `O-001` (everything is a cognitive entity — every root type is
/// one), `O-006` (every type is registered), `O-007` (these are meaning
/// categories, not storage tables).
///
/// Root types are *composed*, not subclassed: `derivation != inheritance`
/// (`O-005`). New root types may be added under a new [`TypeVersion`]; hence
/// this enum is `#[non_exhaustive]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RootType {
    /// A raw, provenance-bearing perception of Reality (`O-003`).
    Observation,
    /// A time-stamped occurrence of significance.
    Event,
    /// A committed, trusted assertion about the world (`ORCH-001`).
    Fact,
    /// A desired future state driving cognition.
    Goal,
    /// A conceptual thing referenced by observations/facts.
    Entity,
    /// A directed relationship between two cognitive entities (`O-005`).
    Relation,
    /// A derived conclusion produced by an engine/capability (`ORCH-004`).
    Inference,
    /// A policy/constraint the Control Plane evaluates at its decision boundary.
    Policy,
}

/// The mandatory-aspect bundle every cognitive entity MUST satisfy.
///
/// Governing: `O-001` (everything is a cognitive entity — this super-trait *is*
/// the O-001 contract), composing `O-002` ([`Identity`]), `O-003`
/// ([`Provenance`]), `O-004` ([`Trust`]), the temporal aspect ([`Temporal`],
/// supporting `ORCH-003`), and `SHARD-001` ([`TenantScope`]).
///
/// A `CognitiveEntity` is the unit of meaning the Data Plane carries and the
/// Kernel commits (`ORCH-001`). This trait is a *contract*, not a base class:
/// concrete `uci.*` types compose aspects, honoring `derivation != inheritance`
/// (`O-005`). It defines meaning only and prescribes no storage (`O-007`).
pub trait CognitiveEntity:
    Identity + Provenance + Trust + Temporal + TenantScope
{
    /// The root category of this entity (`O-001`, `O-006`).
    fn root_type(&self) -> RootType;

    /// The registered type + version this entity conforms to (`O-006`).
    fn type_version(&self) -> TypeVersion;
}

// =============================================================================
// Type registry contract  (O-006, O-007)
// =============================================================================

/// A registration record for one versioned `uci.*` type.
///
/// Governing: `O-006` (every type is versioned and registered).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRegistration {
    /// Namespace + type name of the registered type (`O-006`).
    pub namespace: Namespace,
    /// Registered type name (`O-006`).
    pub type_name: String,
    /// Frozen version of this registration (`O-006`).
    pub version: TypeVersion,
    /// Root category this type belongs to (`O-001`, `O-006`).
    pub root_type: RootType,
}

/// Read-only view over the set of registered `uci.*` types.
///
/// Governing: `O-006` (versioned + registered), `O-007` (defines meaning, not
/// storage — a lookup, never a persistence engine), `ORCH-001` (the registry
/// describes meaning; it does not commit truth).
///
/// This is a query contract: it resolves a type descriptor to its frozen
/// [`TypeRegistration`]. Population of the registry is out of scope for this
/// skeleton.
pub trait TypeRegistry {
    /// Resolve a registered type at a specific version, if present (`O-006`).
    fn resolve(
        &self,
        namespace: &Namespace,
        type_name: &str,
        version: TypeVersion,
    ) -> Option<&TypeRegistration>;
}

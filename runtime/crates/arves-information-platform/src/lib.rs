//! ARVES :: arves-information-platform
//!
//! Purpose: Connectors + canonicalization; emits proposed writes to the Kernel.
//! Governing: Ontology (O-001..007 design principles; mandatory aspects
//!   Identity/Provenance/Trust/Temporal/TenantScope); ORCH-001 (writes are
//!   proposed, never direct truth).
//! Layer: Information Platform (Data Plane). Per LAYER-001 the layer stack is
//!   downward-only: Reality -> Information Platform -> Kernel -> Persistence ->
//!   LCW -> Query -> Engine -> Capability -> Execution (+ Control Plane). This
//!   crate sits directly beneath Reality and directly above the Kernel: it is
//!   the boundary at which raw external sources become ontology-shaped
//!   proposals.
//!
//! STATUS: I1 skeleton - interfaces/contracts only, NO implementation yet.
//! Frozen specification governs; this crate implements, never changes it.
//!
//! # Role in the ARVES chain
//!
//! ARVES flows Theory -> Spec -> Contracts -> Behaviour -> Conformance ->
//! Implementation (the implementation proves the spec, never changes it). This
//! crate is the *ingress boundary of Reality*: it reaches into external,
//! untrusted, heterogeneous sources (documents, event streams, APIs, sensors,
//! human input) and turns each observation into a [`ProposedWrite`] -- a
//! canonical, ontology-shaped, tenant-scoped, provenance-bearing *proposal*. A
//! proposal is not truth. Nothing in this crate writes truth, mutates committed
//! state, or asserts durability.
//!
//! # The single load-bearing contract (ORCH-001)
//!
//! **ORCH-001: the Control Plane owns no truth; only the Kernel owns truth.**
//! By extension no Data-Plane layer below the Kernel may originate truth
//! either -- it may only *propose*. A [`Connector`] therefore has exactly one
//! output shape, [`ProposedWrite`], and there is deliberately no method on any
//! type in this crate that commits, persists, or acknowledges truth. The
//! Kernel is the sole commit gateway (see `arves-kernel`; PROPOSED G-001); this
//! crate hands proposals *toward* it and never past it. This crate lives on the
//! truth-carrying Data Plane, but it produces proposals, not truth.
//!
//! # Canonicalization to the Ontology
//!
//! Ingestion is a *pure, total* mapping from a raw [`Source`] observation into
//! the `uci.*` ontology (see `arves-ontology`; O-001..007 design principles).
//! Canonicalization is where the mandatory ontology aspects are attached to
//! every proposal:
//!
//! * **Identity** -- a stable, content-derived identity for the proposed node.
//! * **Provenance** -- which source, which connector, which raw observation.
//! * **Trust** -- the connector's asserted confidence, never a Kernel verdict.
//! * **Temporal** -- observation / valid-from time from the source.
//! * **TenantScope** -- the owning tenant/workspace shard (SHARD-001).
//!
//! # Idempotence & content-addressing (ORCH-004 / ORCH-003)
//!
//! Every proposal carries a [`ContentHash`] over its canonical form so that
//! re-ingesting the same observation yields the same proposal. This makes the
//! ingest path replay-safe and lets the Kernel deduplicate: consistent with
//! ORCH-004 (invocations are idempotent + content-addressable) and ORCH-003
//! (replay is driven by a recorded decision trace, not recomputation).
//!
//! # Sharding (SHARD-001 / IDR-001)
//!
//! Every proposal is partitioned by an immutable [`TenantScope`] key. A single
//! ingest call never straddles shards; cross-shard effects are the Kernel's
//! concern (per-shard Raft groups and sagas, IDR-001..005), never the
//! connector's.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Scoping & identity aspects (Ontology mandatory aspects; SHARD-001)
// ---------------------------------------------------------------------------

/// Immutable tenant/workspace partition key for a proposal.
///
/// Governing: SHARD-001 (partition by tenant/workspace; key immutable) and the
/// Ontology's mandatory **TenantScope** aspect. Every [`ProposedWrite`] is
/// scoped to exactly one `TenantScope`; a connector must not emit a single
/// proposal spanning two scopes. Downstream, the Kernel routes each proposal to
/// the owning per-shard Raft group (IDR-001). Mirrors the Kernel's `ShardKey`;
/// this crate keeps its own copy to stay dependency-free (LAYER-001 downward
/// dependencies are not inverted by a shared type).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TenantScope {
    /// Owning tenant. Immutable once assigned (SHARD-001).
    pub tenant: String,
    /// Workspace within the tenant. Immutable once assigned (SHARD-001).
    pub workspace: String,
}

/// Content-addressable digest of a canonical proposal payload.
///
/// Governing: ORCH-004 (content-addressable) + ORCH-003 (replay from trace).
/// The hash is computed over the *canonical* ontology form so that identical
/// observations collapse to identical hashes regardless of raw encoding. This
/// is the deduplication and idempotence key the Kernel keys on; it is an
/// opaque, transport-neutral value here (no hashing implementation is fixed by
/// this skeleton).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContentHash(pub Vec<u8>);

/// Stable ontology identity for a proposed cognitive node.
///
/// Governing: the Ontology's mandatory **Identity** aspect. Derived
/// deterministically during canonicalization so re-ingestion of the same
/// observation targets the same logical node (supporting ORCH-004 idempotence).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OntologyId(pub String);

// ---------------------------------------------------------------------------
// Sources (the Reality boundary, top of LAYER-001)
// ---------------------------------------------------------------------------

/// A handle to an external, untrusted origin of observations.
///
/// Governing: LAYER-001 (Reality is the top layer; the Information Platform is
/// the first layer that observes it). A `Source` names *where* an observation
/// came from and *when / for whom* it was observed. It carries no truth: it is
/// a pointer into Reality, not a claim about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The connector kind expected to interpret this source.
    pub kind: SourceKind,
    /// Opaque locator for the origin (URI, stream offset, file path, ...).
    pub locator: String,
    /// Tenant/workspace scope the observation belongs to (SHARD-001).
    pub scope: TenantScope,
    /// Raw, uninterpreted observation bytes as received from Reality.
    pub payload: Vec<u8>,
    /// Source-asserted observation time (feeds the Temporal aspect).
    pub observed_at: Timestamp,
}

/// The category of external system a [`Connector`] knows how to read.
///
/// Informative taxonomy for I1; the frozen spec does not enumerate connector
/// kinds, so this list is deliberately open-ended via [`SourceKind::Other`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// A discrete document (text, structured file, page).
    Document,
    /// An append-only external event / message stream.
    EventStream,
    /// A request/response external service.
    Api,
    /// A sensor / telemetry emitter.
    Sensor,
    /// Direct human-authored input.
    Human,
    /// Any source kind not otherwise enumerated.
    Other(String),
}

/// Opaque, source-asserted wall-clock instant.
///
/// Feeds the Ontology **Temporal** aspect. Kept as an opaque scalar in the
/// skeleton (no clock / timezone semantics are fixed here); the Kernel, not the
/// connector, assigns any authoritative commit time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub u64);

// ---------------------------------------------------------------------------
// Ontology aspects attached during canonicalization
// ---------------------------------------------------------------------------

/// Stable identifier of a registered [`Connector`] instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectorId(pub String);

/// Provenance aspect: the audit chain from Reality to this proposal.
///
/// Governing: Ontology mandatory **Provenance** aspect; supports ORCH-003
/// (recorded decision trace). Records which connector produced the proposal and
/// from which source observation, so the proposal is fully attributable without
/// re-reading Reality.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// Identifier of the connector that produced this proposal.
    pub connector: ConnectorId,
    /// The source kind the observation came from.
    pub source_kind: SourceKind,
    /// Opaque locator of the originating observation.
    pub source_locator: String,
    /// Content hash of the raw observation payload (pre-canonicalization).
    pub raw_hash: ContentHash,
}

/// Trust aspect: the *connector's* asserted confidence in a proposal.
///
/// Governing: Ontology mandatory **Trust** aspect; bounded by ORCH-001. This is
/// an assertion by an untrusted lower layer, **not** a truth verdict: only the
/// Kernel adjudicates. A high value here never implies commitment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trust {
    /// Connector-asserted confidence in `[0.0, 1.0]`. Advisory only.
    pub confidence: f64,
}

/// Temporal aspect derived from the source observation.
///
/// Governing: Ontology mandatory **Temporal** aspect. Distinguishes when the
/// fact was true in Reality (`valid_from`) from when it was observed
/// (`observed_at`). Commit / system time is assigned later by the Kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Temporal {
    /// When the connector observed the fact.
    pub observed_at: Timestamp,
    /// Source-asserted time from which the fact holds, if known.
    pub valid_from: Option<Timestamp>,
}

// ---------------------------------------------------------------------------
// The proposed write (never truth)
// ---------------------------------------------------------------------------

/// A canonical, ontology-shaped **proposal** to change cognitive state.
///
/// Governing: ORCH-001 (never truth) + Ontology (mandatory aspects). A
/// `ProposedWrite` is the *only* output of ingestion and the *only* thing this
/// crate hands toward the Kernel. It asserts nothing durable: the Kernel is the
/// sole owner of truth (OWN-001) and the sole commit gateway (PROPOSED G-001),
/// and it alone may accept, reject, transform, or reconcile a proposal.
///
/// Every aspect mandated by the Ontology is present so the proposal is
/// self-describing and shard-routable without re-reading Reality: Identity
/// ([`OntologyId`]), Provenance, Trust, Temporal, and TenantScope. The
/// [`ContentHash`] over the canonical form gives ORCH-004 idempotence. This is
/// the Data-Plane payload the Kernel's own `ProposedWrite` mirrors on ingress.
#[derive(Clone, Debug, PartialEq)]
pub struct ProposedWrite {
    /// Content-addressable identity of the canonical proposal (ORCH-004).
    pub hash: ContentHash,
    /// Immutable tenant/workspace routing key (SHARD-001).
    pub scope: TenantScope,
    /// Stable ontology identity of the proposed node (Identity aspect).
    pub target: OntologyId,
    /// The proposed mutation, expressed in the ontology.
    pub operation: ProposedOperation,
    /// Provenance aspect: audit chain from Reality.
    pub provenance: Provenance,
    /// Trust aspect: connector-asserted (advisory) confidence.
    pub trust: Trust,
    /// Temporal aspect: observation / validity time.
    pub temporal: Temporal,
}

/// The kind of ontology-level change a proposal requests.
///
/// Governing: Ontology. Deliberately coarse in the I1 skeleton -- the concrete
/// `uci.*` node/edge payloads live in `arves-ontology`; this crate references
/// them opaquely as [`CanonicalNode`] so no cross-crate dependency is required
/// (respecting the std-only skeleton rule and LAYER-001).
#[derive(Clone, Debug, PartialEq)]
pub enum ProposedOperation {
    /// Propose asserting a new canonical node.
    Assert(CanonicalNode),
    /// Propose updating an existing canonical node.
    Update(CanonicalNode),
    /// Propose retracting a previously asserted node by identity.
    Retract(OntologyId),
}

/// Opaque canonical (ontology-shaped) node payload.
///
/// Governing: Ontology. A placeholder for the `uci.*` cognitive type carried by
/// a proposal. The skeleton keeps this crate std-only and dependency-free; a
/// later milestone binds this to the real `arves-ontology` types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalNode {
    /// Ontology type name (`uci.*`) the payload conforms to.
    pub type_name: String,
    /// Canonicalized, encoding-normalized payload bytes.
    pub canonical_bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why an ingest could not produce a [`ProposedWrite`].
///
/// Note (ORCH-001): a failure to ingest is *not* a truth outcome. It means no
/// proposal was formed; it never implies anything was (or was not) committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IngestError {
    /// The source could not be decoded into the expected shape.
    Malformed(String),
    /// The source kind is not handled by this connector.
    UnsupportedSource(SourceKind),
    /// The observation could not be mapped into the `uci.*` ontology.
    Uncanonicalizable(String),
    /// The source's declared scope was missing or inconsistent (SHARD-001).
    ScopeViolation(String),
}

// ---------------------------------------------------------------------------
// The Connector trait
// ---------------------------------------------------------------------------

/// Ingests observations from Reality and canonicalizes them into
/// ontology-shaped **proposals**.
///
/// Governing: ORCH-001 (never writes truth directly) + Ontology
/// (canonicalization to the `uci.*` type system with all mandatory aspects).
///
/// # Contract
///
/// * **Never truth (ORCH-001).** [`Connector::ingest`] returns a
///   [`ProposedWrite`] and nothing more. There is no commit, persist, or
///   acknowledge method on this trait, by design: only the Kernel owns and
///   commits truth (ORCH-001, OWN-001, PROPOSED G-001).
/// * **Total canonicalization (Ontology).** A successful ingest yields a
///   proposal that already carries every mandatory Ontology aspect (Identity,
///   Provenance, Trust, Temporal, TenantScope). No downstream layer needs to
///   re-read the raw source to interpret the proposal.
/// * **Idempotent + content-addressable (ORCH-004 / ORCH-003).** For a given
///   [`Source`], `ingest` is a pure function of the observation: equal
///   observations produce equal [`ContentHash`]es and equal proposals, making
///   the path replay-safe.
/// * **Single shard (SHARD-001).** The produced proposal is scoped to exactly
///   one immutable [`TenantScope`]; a connector never emits a cross-shard
///   proposal.
///
/// Implementations are expected to be stateless with respect to truth: any
/// internal cursors / offsets are connector bookkeeping, never cognitive state.
pub trait Connector {
    /// Stable identity of this connector (feeds [`Provenance::connector`]).
    fn id(&self) -> ConnectorId;

    /// Source kinds this connector can interpret.
    fn accepts(&self) -> &[SourceKind];

    /// Canonicalize one observation from Reality into a proposal.
    ///
    /// Governing: ORCH-001 + Ontology. Returns a [`ProposedWrite`] carrying all
    /// mandatory aspects, or an [`IngestError`] if no proposal could be formed.
    /// This method never writes, commits, or persists truth (ORCH-001).
    fn ingest(&self, source: Source) -> Result<ProposedWrite, IngestError>;
}

//! ARVES :: arves-query
//!
//! Purpose: Strictly read-only projections over Kernel/LCW/Persistence (read tiers).
//! Governing: QUERY-001 (proposed); read tiers from IDR-001.
//! Layer: Data Plane
//!
//! STATUS at v1.0 freeze: CONTRACT-ONLY (by design). Defines the read-only
//! projection/query interfaces and types; the distributed query engine is
//! deferred to I3. Any `fn` bodies present are trivial placeholders so the
//! contract compiles. Frozen specification governs; this crate implements,
//! never changes it.
//!
//! STATUS since RCR-023 (I3 Stage 1, per `docs/design/I3_Distributed_Query_Design.md`):
//! the "CONTRACT-ONLY" wording above is superseded — this crate now ALSO
//! carries the single-node QUERY CORE in [`projection`] (a WAL-replay
//! read path implementing the [`Query`] trait; additive, the RCR-008/019
//! pattern). Every frozen v1.0 type and trait signature in this file is
//! byte-unchanged. The DISTRIBUTED query fabric (routing, replica sets,
//! real read-index, scatter-gather) remains future I3 stages.
//!
//! STATUS since RCR-024 (I3 Stage 2, same design): [`distributed`] adds
//! DISTRIBUTED READS over the I2 cluster substrate — shard-aware routing,
//! the IDR-001 consistency ladder in honest in-process-simulated form
//! (leader-consistent read-index vs labeled follower/AP staleness), bounded
//! tenant-internal scatter-gather (additive types; the frozen single-shard
//! `Query` trait cannot carry a merged result — design §3.3/§6.2), and a
//! read-your-writes floor (additive carrier, design OQ-5). Every frozen v1.0
//! type and trait signature in this file remains byte-unchanged.
//!
//! # Position in the layer chain
//!
//! Per LAYER-001 the runtime is a downward-only stack:
//! `Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query ->
//! Engine -> Capability -> Execution` alongside the Control Plane. The Query
//! layer sits *below* Engine and *above* LCW: it exposes read views that upper
//! layers (Engine, Capability, Execution) consume, and it reads from the
//! layers beneath it (Kernel truth, Persistence durable log, LCW working
//! memory). It never reaches upward and never writes downward.
//!
//! # The read-only contract (QUERY-001, proposed)
//!
//! QUERY-001 (proposed, informative pending CCP) states that the Query layer is
//! **read-only**: it produces *projections* over state owned elsewhere and
//! mutates nothing. This crate encodes that contract in the type system so that
//! an implementation cannot accidentally acquire write authority:
//!
//! - The [`Query`] trait takes `&self` on every method. No method returns a
//!   handle, token, or capability that could be used to commit state.
//! - Ownership is not transferred here. Per OWN-001 every piece of state has
//!   exactly one owner; Query owns none of the state it reads. Kernel owns
//!   cognitive truth (ORCH-001, G-001 proposed), LCW owns Working Memory
//!   (LCW-001 proposed), Persistence owns the durable store (PERSIST-001
//!   proposed). Query merely *observes* these owners.
//! - All commits flow exclusively through the Kernel commit gateway
//!   (ORCH-001 / G-001 proposed). Reading through this crate can never be a
//!   commit path.
//!
//! # Truth is CP, observability is AP
//!
//! Per the IDR set, the *truth* path is CP (consistent under partition) while
//! *observability* is AP (available under partition). Query is the observability
//! surface, so callers must choose how much consistency they are willing to pay
//! for. That choice is expressed by [`ReadTier`], derived directly from the read
//! tiers named in IDR-001:
//!
//! - [`ReadTier::Linearizable`] - reads reflect the latest Kernel-committed
//!   outcome (ORCH-003: outcomes are the recorded decision trace). Requires
//!   routing through / confirming with the per-shard Raft leader
//!   (IDR-001..004), so it is the most consistent and the most expensive tier.
//! - [`ReadTier::BoundedStaleness`] - reads may lag committed truth by at most a
//!   caller-supplied bound (see [`StalenessBound`]); cheaper, still bounded.
//! - [`ReadTier::Eventual`] - reads may come from any replica and may be
//!   arbitrarily stale but never *incorrect* for the version returned; cheapest
//!   and most available.
//!
//! # Sharding (SHARD-001)
//!
//! State is partitioned by tenant/workspace with an immutable shard key
//! (SHARD-001). Every read is scoped to a [`ShardKey`]; there is no cross-shard
//! atomic read here, consistent with the "no cross-shard atomic commit" IDR
//! (cross-shard composition is a saga concern in higher layers, never a Query
//! primitive).
//!
//! # Scope of this skeleton
//!
//! This is an I1 interface skeleton: traits, enums, structs, and type aliases
//! with contracts expressed in signatures and docs. There is no query planner,
//! no I/O, and no storage engine here. Distributed query execution is an I3
//! concern (milestone: I3 Distributed Query); this crate only fixes the
//! read-only *shape* that I3 must honor.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod distributed;
pub mod projection;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// Immutable partition key: identifies the tenant/workspace shard a read is
/// scoped to.
///
/// Per SHARD-001 the shard key is immutable once assigned; every projection is
/// read within exactly one shard. Represented here as an opaque owned string in
/// the skeleton; a richer typed key is an implementation concern.
pub type ShardKey = String;

/// Opaque, content-addressable identity of a projection target (an entity,
/// view, or aggregate the caller wishes to observe).
///
/// Content-addressability aligns with ORCH-004 (content-addressable
/// invocations); a projection is named by *what* it is, not by a mutable
/// handle. Represented as an opaque string in the skeleton.
pub type ProjectionId = String;

/// Monotonic version stamp of a Kernel-committed outcome.
///
/// Per ORCH-003, truth is the recorded decision trace of committed *outcomes*;
/// a version identifies a point in that trace. Query returns the version a
/// projection was observed at so callers can reason about staleness relative to
/// their chosen [`ReadTier`].
pub type Version = u64;

/// Logical timestamp in milliseconds, used to express staleness bounds.
///
/// This is a logical duration, not a wall-clock instant; the skeleton keeps it
/// dependency-free (std-only) rather than pulling in a time crate.
pub type Millis = u64;

// ---------------------------------------------------------------------------
// ReadTier
// ---------------------------------------------------------------------------

/// Consistency tier requested for a read, per the read tiers named in
/// IDR-001 ("Read tiers: linearizable / bounded-staleness / eventual").
///
/// The tier expresses the caller's position on the consistency/availability
/// trade-off. Truth is CP and observability is AP; this enum lets a caller opt
/// into more consistency (at higher cost/latency, and reduced availability
/// under partition) or more availability (accepting staleness).
///
/// Ordering note: variants are declared from strongest to weakest consistency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadTier {
    /// Strongest tier: the read reflects the most recent Kernel-committed
    /// outcome as of the read.
    ///
    /// Requires confirming currency with the per-shard Raft leader
    /// (IDR-001..004), because only the leader can attest to the latest
    /// committed outcome (ORCH-003). Least available under partition; highest
    /// latency.
    Linearizable,

    /// Middle tier: the read may lag committed truth, but by no more than a
    /// caller-supplied bound (see [`StalenessBound`]).
    ///
    /// May be served by a follower replica that is provably within the bound,
    /// trading a little freshness for lower latency and higher availability.
    BoundedStaleness,

    /// Weakest tier: the read may be served from any replica and may be
    /// arbitrarily stale.
    ///
    /// The returned data is never *wrong* for the [`Version`] it reports; it may
    /// simply be old. Cheapest and most available under partition (fully AP).
    Eventual,
}

impl ReadTier {
    /// Returns `true` if this tier requires the per-shard Raft leader to serve
    /// or attest the read (only [`ReadTier::Linearizable`]).
    ///
    /// Informative helper for routing; contract only, no I/O.
    pub const fn requires_leader(self) -> bool {
        matches!(self, ReadTier::Linearizable)
    }

    /// Returns `true` if this tier may be served by a follower/replica without
    /// contacting the leader ([`ReadTier::BoundedStaleness`] and
    /// [`ReadTier::Eventual`]).
    pub const fn allows_replica(self) -> bool {
        !self.requires_leader()
    }
}

/// Maximum staleness a [`ReadTier::BoundedStaleness`] read is permitted to
/// exhibit, expressed as a logical age in [`Millis`].
///
/// A bounded-staleness read must be served from state no older than this bound
/// relative to committed truth, or the read must fail with
/// [`QueryError::StalenessBoundExceeded`]. Ignored by the other tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StalenessBound {
    /// Maximum permitted lag behind committed truth, in milliseconds.
    pub max_lag: Millis,
}

impl StalenessBound {
    /// Constructs a staleness bound of `max_lag` milliseconds.
    pub const fn new(max_lag: Millis) -> Self {
        Self { max_lag }
    }
}

// ---------------------------------------------------------------------------
// Read scope / request shape
// ---------------------------------------------------------------------------

/// Describes *how* and *from where* a projection should be read, without
/// naming *what* is read (the target is the [`ProjectionId`] passed to a
/// [`Query`] method).
///
/// This bundles the shard scope (SHARD-001), the chosen consistency
/// [`ReadTier`] (IDR-001), and, for bounded-staleness, the [`StalenessBound`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadScope {
    /// Immutable shard the read is scoped to (SHARD-001). No read crosses
    /// shards atomically.
    pub shard: ShardKey,

    /// Requested consistency tier (IDR-001).
    pub tier: ReadTier,

    /// Staleness bound, meaningful only when `tier == ReadTier::BoundedStaleness`.
    /// `None` for other tiers.
    pub bound: Option<StalenessBound>,
}

impl ReadScope {
    /// Constructs a linearizable read scope for `shard`.
    pub fn linearizable(shard: ShardKey) -> Self {
        Self { shard, tier: ReadTier::Linearizable, bound: None }
    }

    /// Constructs a bounded-staleness read scope for `shard` with the given
    /// `bound`.
    pub fn bounded(shard: ShardKey, bound: StalenessBound) -> Self {
        Self { shard, tier: ReadTier::BoundedStaleness, bound: Some(bound) }
    }

    /// Constructs an eventual-consistency read scope for `shard`.
    pub fn eventual(shard: ShardKey) -> Self {
        Self { shard, tier: ReadTier::Eventual, bound: None }
    }
}

// ---------------------------------------------------------------------------
// Projection (read result)
// ---------------------------------------------------------------------------

/// A read-only view returned by a [`Query`] method.
///
/// A `Projection` carries the observed payload together with the metadata a
/// caller needs to reason about consistency: the [`Version`] the data was seen
/// at and the [`ReadTier`] that actually served it (which is never *stronger*
/// than the tier requested). Per QUERY-001 (proposed) a projection is a pure
/// observation - holding one grants no authority to mutate anything.
///
/// The payload type `T` is chosen by the caller/implementation; the skeleton is
/// generic and imposes no serialization or storage format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection<T> {
    /// Identity of the projected target (content-addressable; ORCH-004).
    pub id: ProjectionId,

    /// Committed-outcome version this projection was observed at (ORCH-003).
    pub observed_at: Version,

    /// The tier that actually served the read. Guaranteed to be no stronger
    /// than the requested tier.
    pub served_tier: ReadTier,

    /// The observed, read-only payload.
    pub value: T,
}

impl<T> Projection<T> {
    /// Borrows the projected value without transferring ownership.
    ///
    /// Deliberately returns `&T` (never `&mut T`): a projection is read-only per
    /// QUERY-001 (proposed).
    pub fn value(&self) -> &T {
        &self.value
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Error returned by a [`Query`] operation.
///
/// These are *read* failures only. There are deliberately no write/commit
/// errors here, because the Query layer never commits (QUERY-001 proposed;
/// commits flow only through the Kernel gateway, ORCH-001 / G-001 proposed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// The requested [`ProjectionId`] does not exist in the given shard.
    NotFound {
        /// The projection target that was not found.
        id: ProjectionId,
    },

    /// The requested [`ShardKey`] is unknown or not served here (SHARD-001).
    UnknownShard {
        /// The shard key that could not be resolved.
        shard: ShardKey,
    },

    /// A [`ReadTier::Linearizable`] read could not confirm currency with the
    /// per-shard Raft leader (IDR-001..004), e.g. no leader is reachable.
    LeaderUnavailable,

    /// A [`ReadTier::BoundedStaleness`] read could not be served within the
    /// caller's [`StalenessBound`].
    StalenessBoundExceeded {
        /// The bound the caller required.
        requested: StalenessBound,
        /// The actual observed lag that violated the bound.
        observed_lag: Millis,
    },

    /// A bounded-staleness scope was supplied without a [`StalenessBound`], or a
    /// non-bounded scope supplied one. The read scope is internally
    /// inconsistent.
    MalformedScope,
}

impl core::fmt::Display for QueryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QueryError::NotFound { id } => {
                write!(f, "projection not found: {id}")
            }
            QueryError::UnknownShard { shard } => {
                write!(f, "unknown shard: {shard}")
            }
            QueryError::LeaderUnavailable => {
                write!(f, "linearizable read: per-shard Raft leader unavailable")
            }
            QueryError::StalenessBoundExceeded { requested, observed_lag } => write!(
                f,
                "staleness bound exceeded: requested max_lag={}ms, observed_lag={}ms",
                requested.max_lag, observed_lag
            ),
            QueryError::MalformedScope => {
                write!(f, "malformed read scope: tier/bound mismatch")
            }
        }
    }
}

impl std::error::Error for QueryError {}

/// Convenience result alias for Query operations.
pub type QueryResult<T> = Result<T, QueryError>;

// ---------------------------------------------------------------------------
// Query trait (READ-ONLY)
// ---------------------------------------------------------------------------

/// The read-only projection surface of the runtime (QUERY-001, proposed).
///
/// A `Query` reads projections over state owned by the Kernel (truth,
/// ORCH-001 / G-001 proposed), the LCW (Working Memory, LCW-001 proposed), and
/// Persistence (durable log, PERSIST-001 proposed). It **mutates nothing**:
///
/// - Every method takes `&self`; none takes `&mut self` and none consumes
///   `self`. An implementation therefore cannot expose mutation through this
///   trait.
/// - Every method returns a [`Projection`] (a read-only observation) or a
///   [`QueryError`]. No method returns a commit handle, write token, or lock.
/// - Ownership of read state is never taken (OWN-001): Query observes owners,
///   it does not become one.
///
/// The consistency of a read is governed by the caller-supplied [`ReadScope`]
/// (shard + [`ReadTier`] + optional [`StalenessBound`]), per IDR-001. Truth is
/// CP; this observability surface is AP and lets callers dial the trade-off.
///
/// Distributed execution of these reads is an I3 concern (milestone: I3
/// Distributed Query). This trait only fixes the read-only contract that any
/// such implementation must uphold; it contains no logic.
pub trait Query {
    /// The value type carried by projections this `Query` produces.
    ///
    /// Left associated (rather than a method generic) so a concrete
    /// implementation binds a single projection payload representation.
    type View;

    /// Reads a single projection identified by `id`, using the consistency and
    /// shard scope described by `scope`.
    ///
    /// # Contract
    /// - Read-only: observes state, mutates nothing (QUERY-001 proposed).
    /// - The result's `served_tier` is never stronger than `scope.tier`.
    /// - Scoped to `scope.shard` only; never crosses shards (SHARD-001).
    ///
    /// # Errors
    /// Returns [`QueryError::NotFound`] if `id` is absent in the shard,
    /// [`QueryError::UnknownShard`] if the shard is not served,
    /// [`QueryError::LeaderUnavailable`] for a linearizable read with no
    /// reachable leader, [`QueryError::StalenessBoundExceeded`] if a
    /// bounded-staleness read cannot meet its bound, or
    /// [`QueryError::MalformedScope`] if `scope` is internally inconsistent.
    fn read(
        &self,
        scope: &ReadScope,
        id: &ProjectionId,
    ) -> QueryResult<Projection<Self::View>>;

    /// Returns whether a projection identified by `id` exists within
    /// `scope.shard`, without materializing its value.
    ///
    /// Read-only existence check (QUERY-001 proposed). Consistency follows
    /// `scope.tier` exactly as for [`Query::read`].
    ///
    /// # Errors
    /// As for [`Query::read`], excluding [`QueryError::NotFound`] (a missing
    /// projection yields `Ok(false)`, not an error).
    fn exists(&self, scope: &ReadScope, id: &ProjectionId) -> QueryResult<bool>;

    /// Returns the latest [`Version`] observable for `id` under `scope`.
    ///
    /// Lets a caller reason about staleness (relative to the recorded outcome
    /// trace, ORCH-003) without transferring the payload.
    ///
    /// # Errors
    /// As for [`Query::read`].
    fn latest_version(
        &self,
        scope: &ReadScope,
        id: &ProjectionId,
    ) -> QueryResult<Version>;
}

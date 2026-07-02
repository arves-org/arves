//! ARVES :: arves-lcw
//!
//! Purpose: Living Cognitive World: single owner of Working Memory / live state.
//! Governing: OWN-001; LCW-001 (proposed). Amendment-001.
//! Layer: Data Plane
//!
//! STATUS: CONTRACT-ONLY (by design). Defines the Living Cognitive World /
//! Working-Memory interfaces and value types with small compiling helper/wrapper
//! constructors; carries no live-state management logic (deferred). Frozen
//! specification governs; this crate implements, never changes it.
//!
//! # Position in the ARVES layering
//!
//! The runtime layers are strictly downward-only (LAYER-001):
//!
//! ```text
//! Reality -> Information Platform -> Kernel -> Persistence -> LCW
//!         -> Query -> Engine -> Capability -> Execution   (+ Control Plane)
//! ```
//!
//! The **Living Cognitive World (LCW)** sits directly below Persistence and
//! above Query. It is a **Data Plane** component: it carries state, it does not
//! decide (deciding belongs to the Control Plane, which owns no truth per
//! ORCH-001). LCW hosts the runtime's *live, mutable* scratch space -- the
//! **Working Memory** -- while a cognitive computation is in flight.
//!
//! # What LCW owns, and what it explicitly does NOT own
//!
//! By **OWN-001** every piece of state has exactly one owner. LCW is the single
//! owner of **Working Memory** and, by **LCW-001 (proposed)**, of nothing else.
//! Concretely:
//!
//! * LCW **owns** Working Memory: the live, mutable, in-flight cognitive state
//!   of an active session/shard. This is a *working set*, not a system of
//!   record.
//! * LCW **is NOT truth.** Only the **Kernel** owns cognitive truth and is the
//!   sole commit gateway (ORCH-001; G-001 proposed). Working Memory is derived,
//!   speculative, and may be discarded, rebuilt, or contradicted by the Kernel
//!   at any commit boundary.
//! * LCW does **not** persist anything durably. Durable, append-only storage is
//!   owned by Persistence (PERSIST-001 proposed; IDR-005 append-only WAL).
//! * LCW does **not** serve as a read API for external consumers. Read-only
//!   projections are owned by Query (QUERY-001 proposed).
//!
//! The distinction "live/mutable but NOT truth" is the whole point of this
//! crate: Working Memory is where an engine may mutate freely and cheaply, and
//! from which only *committed outcomes* (never the raw mutations) are ever
//! promoted toward the Kernel (IDR-002: replicate committed OUTCOMES, not
//! invocations).
//!
//! # Sharding
//!
//! Working Memory is partitioned by tenant/workspace and its shard key is
//! immutable (**SHARD-001**). A [`WorkingMemory`] instance is scoped to exactly
//! one [`ShardKey`]; there is no cross-shard live state and no cross-shard
//! atomic mutation (mirroring IDR: no cross-shard atomic commit).
//!
//! # This crate is a contract, not an engine
//!
//! Everything below is interface-level: traits, structs, enums, and type
//! aliases with no business logic. Method bodies are intentionally absent so
//! that the *specification* -- not this scaffold -- remains authoritative
//! (Theory -> Spec -> Contracts -> Behaviour -> Conformance -> Implementation).

#![forbid(unsafe_code)]

use core::fmt;

// ---------------------------------------------------------------------------
// Identity & addressing
// ---------------------------------------------------------------------------

/// Immutable partition key for Working Memory.
///
/// Working Memory is partitioned by tenant/workspace and the key is **immutable**
/// once assigned (**SHARD-001**). A [`WorkingMemory`] view is bound to exactly
/// one `ShardKey` for its entire lifetime; rebinding is not expressible in this
/// contract by design.
///
/// The pairing of `tenant` and `workspace` matches the per-shard consensus group
/// boundary (IDR-001: one Raft group per tenant/workspace).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShardKey {
    /// Tenant identity. Immutable for the lifetime of the shard (SHARD-001).
    pub tenant: String,
    /// Workspace identity within the tenant. Immutable (SHARD-001).
    pub workspace: String,
}

impl fmt::Display for ShardKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.tenant, self.workspace)
    }
}

/// Logical address of a single cell of live state inside one shard's
/// Working Memory.
///
/// A `StateKey` is meaningful only relative to a [`ShardKey`]; there is no global
/// live-state namespace (SHARD-001 forbids cross-shard live state). Keys are
/// opaque to LCW: interpretation is a concern of the layers above.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateKey(pub String);

impl fmt::Display for StateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Monotonic revision counter for a single [`StateKey`] within Working Memory.
///
/// Because Working Memory is **live and mutable but NOT truth** (OWN-001,
/// LCW-001 proposed), revisions track *local* mutation ordering only. They are
/// not the Kernel's committed version and MUST NOT be presented as truth or as a
/// durable version stamp (that is the Kernel's role, ORCH-001; G-001 proposed).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Revision(pub u64);

impl Revision {
    /// The revision of a cell that has never been written.
    pub const ZERO: Revision = Revision(0);

    /// Returns the next revision after `self`.
    ///
    /// Skeleton helper only; carries no persistence or commit semantics.
    #[must_use]
    pub fn next(self) -> Revision {
        Revision(self.0.saturating_add(1))
    }
}

// ---------------------------------------------------------------------------
// Live state payload
// ---------------------------------------------------------------------------

/// Opaque, live, mutable payload held in Working Memory for one [`StateKey`].
///
/// LCW treats the payload as bytes. It is intentionally *not* a truth-bearing
/// value: it may be speculative, half-computed, or superseded. Only the Kernel
/// turns anything into truth (ORCH-001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveValue {
    /// Raw bytes of the live state. Semantics are defined by upper layers.
    pub bytes: Vec<u8>,
}

impl LiveValue {
    /// Wraps raw bytes as a [`LiveValue`]. Skeleton constructor only.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

/// A revisioned snapshot of one cell of live state, as returned by a read.
///
/// The bundled [`Revision`] is local mutation ordering within Working Memory,
/// not a committed Kernel version (see [`Revision`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveState {
    /// The addressed cell.
    pub key: StateKey,
    /// The current live payload.
    pub value: LiveValue,
    /// Local revision at which `value` was observed.
    pub revision: Revision,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes for Working Memory operations.
///
/// These describe *contract* failures only; no I/O or durability errors appear
/// here because LCW performs no durable I/O (that belongs to Persistence,
/// PERSIST-001 proposed).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LcwError {
    /// The requested [`StateKey`] has no live value in this shard.
    NotFound(StateKey),
    /// A conditional write's expected [`Revision`] did not match the current
    /// revision (optimistic-concurrency failure on live state).
    RevisionConflict {
        /// The cell whose revision did not match.
        key: StateKey,
        /// Revision the caller expected.
        expected: Revision,
        /// Revision actually observed.
        actual: Revision,
    },
    /// The operation targeted a shard other than the one this view is bound to.
    /// Cross-shard live state is forbidden (SHARD-001).
    WrongShard {
        /// Shard this view is bound to.
        expected: ShardKey,
        /// Shard the caller referenced.
        actual: ShardKey,
    },
}

impl fmt::Display for LcwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LcwError::NotFound(k) => write!(f, "live state not found: {k}"),
            LcwError::RevisionConflict {
                key,
                expected,
                actual,
            } => write!(
                f,
                "revision conflict on {key}: expected {expected:?}, actual {actual:?}"
            ),
            LcwError::WrongShard { expected, actual } => {
                write!(f, "wrong shard: bound to {expected}, referenced {actual}")
            }
        }
    }
}

impl std::error::Error for LcwError {}

/// Convenience result alias for Working Memory operations.
pub type LcwResult<T> = Result<T, LcwError>;

// ---------------------------------------------------------------------------
// Write intent
// ---------------------------------------------------------------------------

/// Precondition for a [`WorkingMemory::put`] operation.
///
/// Working Memory is mutable, but writes may still be conditioned on the caller's
/// view of local revision to detect concurrent mutation. This is *local*
/// optimistic concurrency over live state only -- it is **not** a Kernel commit
/// and grants no truth or durability guarantee (ORCH-001; G-001 proposed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PutCondition {
    /// Write unconditionally, overwriting any current value.
    Always,
    /// Write only if the cell is currently absent.
    IfAbsent,
    /// Write only if the current revision equals the given value.
    IfRevision(Revision),
}

// ---------------------------------------------------------------------------
// Core contract: WorkingMemory
// ---------------------------------------------------------------------------

/// The single-owner contract over one shard's **Working Memory**.
///
/// # Ownership (OWN-001, LCW-001 proposed)
///
/// An implementor of `WorkingMemory` is the *sole* owner of the live, mutable
/// cognitive state for its bound [`ShardKey`]. No other component may mutate that
/// state directly; there is exactly one owner per state (OWN-001).
///
/// # Not truth
///
/// The state exposed here is **live and mutable, and is NOT truth.** It exists to
/// let engines compute cheaply against a working set. Truth is owned solely by
/// the Kernel, which is also the sole commit gateway (ORCH-001; G-001 proposed).
/// Nothing observed through this trait may be treated as durable or authoritative.
///
/// # Data Plane, not Control Plane
///
/// `WorkingMemory` *carries* state; it does not *decide*. Planning and decisions
/// are the Control Plane's job, and the Control Plane owns no truth and produces
/// no persistent state (ORCH-001, ORCH-002).
///
/// # Sharding (SHARD-001)
///
/// Each instance is bound to one immutable [`ShardKey`]. Operations that
/// reference a different shard fail with [`LcwError::WrongShard`]. There is no
/// cross-shard live state and no cross-shard atomic mutation.
///
/// This is a skeleton: only signatures are defined; behaviour is governed by the
/// frozen specification, not by this crate.
pub trait WorkingMemory {
    /// The immutable shard this Working Memory view is bound to (SHARD-001).
    fn shard(&self) -> &ShardKey;

    /// Reads the current live state for `key` within this shard.
    ///
    /// Returns [`LcwError::NotFound`] if the cell has never been written (or was
    /// cleared). The returned value is a live snapshot and MUST NOT be treated as
    /// committed truth (ORCH-001).
    fn get(&self, key: &StateKey) -> LcwResult<LiveState>;

    /// Writes `value` to `key` subject to `condition`, returning the new local
    /// [`Revision`].
    ///
    /// This mutates Working Memory only. It performs no durable write (that is
    /// Persistence, PERSIST-001 proposed) and no commit (that is the Kernel,
    /// G-001 proposed). A failing `condition` yields [`LcwError::RevisionConflict`]
    /// (or the write is skipped for [`PutCondition::IfAbsent`]).
    fn put(
        &mut self,
        key: &StateKey,
        value: LiveValue,
        condition: PutCondition,
    ) -> LcwResult<Revision>;

    /// Removes the live cell for `key` from Working Memory, if present.
    ///
    /// Returns `true` if a value was removed. This affects live state only and
    /// implies nothing about truth or durability.
    fn evict(&mut self, key: &StateKey) -> LcwResult<bool>;

    /// Reports whether a live value currently exists for `key`.
    fn contains(&self, key: &StateKey) -> bool;
}

// ---------------------------------------------------------------------------
// Session / lifecycle contract
// ---------------------------------------------------------------------------

/// Factory/lifecycle contract for obtaining a [`WorkingMemory`] view bound to a
/// shard.
///
/// A `LiveWorkspace` hands out per-shard [`WorkingMemory`] views. Binding is
/// keyed by an **immutable** [`ShardKey`] (SHARD-001). The workspace itself owns
/// no truth (ORCH-001) and produces no persistent state (ORCH-002); it merely
/// scopes ownership of live state to a shard.
pub trait LiveWorkspace {
    /// Concrete Working Memory view produced for a shard.
    type Memory: WorkingMemory;

    /// Opens (creating if necessary) the Working Memory view for `shard`.
    ///
    /// Since LCW is the single owner of Working Memory (OWN-001, LCW-001
    /// proposed), a given shard has at most one active live view; the contract
    /// does not permit two concurrent owners of the same live state.
    fn open(&self, shard: &ShardKey) -> LcwResult<Self::Memory>;

    /// Discards all live state for `shard`.
    ///
    /// Because Working Memory is not truth, discarding it is always safe with
    /// respect to the system of record: the Kernel retains truth and Persistence
    /// retains the durable trace (ORCH-001; IDR-005). This is how a shard's live
    /// scratch space is reclaimed or rebuilt.
    fn discard(&self, shard: &ShardKey) -> LcwResult<()>;
}

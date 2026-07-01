//! ARVES :: arves-persistence
//!
//! Purpose: Durable, append-only Write-Ahead Log (WAL) + snapshots of
//! Kernel-committed state. In the ARVES reference runtime the Raft log,
//! the WAL, and the decision trace are the *same artifact* viewed from three
//! angles (IDR-005): consensus replicates it, durability persists it, and
//! replay reads it. This crate defines the durable-store contract only.
//!
//! Governing invariants:
//! - IDR-005: Raft log = WAL = decision trace (append-only). This crate is the
//!   durable face of that single artifact.
//! - IDR-002: replicate committed OUTCOMES, not invocations; therefore WAL
//!   records are committed decisions/outcomes, never raw engine calls.
//! - IDR-003 (embodied as ORCH-003): recovery is *replay from the recorded
//!   decision trace*, never recomputation. `replay_from` exists precisely so
//!   truth is reconstructed by re-reading committed records.
//! - ORCH-003: replay from recorded decision trace, not recomputation.
//! - PERSIST-001 (PROPOSED, informative, pending CCP): persistence is a
//!   *durable store only* -- it owns no cognitive truth, makes no decisions,
//!   and never mutates records after append. It stores what the Kernel commits.
//! - SHARD-001: partition by tenant/workspace; the shard key is immutable.
//!   Each shard has its own WAL (one Raft group per tenant/workspace, IDR-001).
//! - OWN-001: one owner per state -- the Kernel owns truth; this store is the
//!   Kernel's durable substrate, not an independent owner of truth (ORCH-001).
//! - LAYER-001: Persistence sits below LCW and above the Kernel in the
//!   downward-only layer stack; it depends on nothing above it.
//!
//! Layer: Persistence (Data Plane). Truth path is CP (consistent); this store
//! is the CP durability tier. (Observability/AP tiers live elsewhere.)
//!
//! STATUS: I1 skeleton - interfaces/contracts only, NO implementation yet.
//! Frozen specification governs; this crate implements, never changes it.
//! All method bodies are intentionally absent (trait signatures only).

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Core value types
// ---------------------------------------------------------------------------

/// Monotonic position of a record within a single shard's WAL.
///
/// Offsets are dense, gap-free, and strictly increasing per shard. Because the
/// WAL *is* the Raft log (IDR-005), an `Offset` also names a position in the
/// replicated decision trace. Offsets are meaningful only within one shard
/// (SHARD-001): they are never comparable across shards.
pub type Offset = u64;

/// Raft term in which a record was committed.
///
/// Carried alongside each record so replay can distinguish leader epochs
/// (per-shard leader election, IDR-001) without consulting the consensus layer.
pub type Term = u64;

/// Immutable shard key: `(tenant, workspace)`.
///
/// Per SHARD-001 the shard key is immutable for the life of the data it names,
/// and all partitioning is by tenant/workspace. One `ShardKey` maps to exactly
/// one WAL and one Raft group (IDR-001).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShardKey {
    /// Tenant identifier. Immutable once assigned (SHARD-001).
    pub tenant: String,
    /// Workspace identifier within the tenant. Immutable once assigned (SHARD-001).
    pub workspace: String,
}

/// Content address of a record payload (e.g. a hash digest).
///
/// Supports ORCH-004 (every engine/capability invocation is idempotent and
/// content-addressable): committed outcomes carry the address of the content
/// they pertain to, so a replayed record is recognizable and de-duplicable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContentId(pub Vec<u8>);

/// The kind of committed fact a WAL record carries.
///
/// Per IDR-002 the WAL records *outcomes* of committed decisions, not the
/// invocations that produced them. `Snapshot` markers let replay start from a
/// compacted checkpoint instead of the log head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordKind {
    /// A committed decision/outcome produced by the Kernel commit gateway.
    /// This is the substance of the decision trace (IDR-005, ORCH-003).
    Outcome,
    /// A membership change committed via joint consensus (IDR-001).
    Membership,
    /// A marker indicating a snapshot was taken at this offset; state at or
    /// below the marker is captured by the referenced snapshot.
    SnapshotMarker,
    /// A no-op / barrier record (e.g. a new-leader barrier). Carries no truth.
    Barrier,
}

/// A single append-only WAL record: one entry in the Raft log = WAL =
/// decision trace (IDR-005).
///
/// Records are write-once. Once appended and committed they are never mutated
/// or reordered (append-only WAL, IDR-005); this is the physical expression of
/// PERSIST-001's "durable store only" stance -- the store never edits truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalRecord {
    /// Shard this record belongs to (SHARD-001). Records never cross shards;
    /// there is no cross-shard atomic commit (sagas instead, IDR-001).
    pub shard: ShardKey,
    /// Dense, strictly increasing position within the shard's WAL.
    pub offset: Offset,
    /// Raft term in which the record was committed (IDR-001 leader epochs).
    pub term: Term,
    /// What kind of committed fact this record carries.
    pub kind: RecordKind,
    /// Content address of the payload, enabling idempotent, content-addressable
    /// replay (ORCH-004).
    pub content: ContentId,
    /// Opaque, already-serialized committed outcome. This crate treats the
    /// payload as bytes only; interpretation belongs to the Kernel (OWN-001).
    pub payload: Vec<u8>,
}

/// A record staged for append, before the store assigns its final `Offset`.
///
/// The caller (the shard leader path, IDR-001) supplies the committed content;
/// the store owns offset assignment so offsets stay dense and gap-free.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRecord {
    /// Target shard (SHARD-001).
    pub shard: ShardKey,
    /// Term under which this outcome was committed.
    pub term: Term,
    /// Kind of committed fact.
    pub kind: RecordKind,
    /// Content address of the payload (ORCH-004).
    pub content: ContentId,
    /// Serialized committed outcome bytes.
    pub payload: Vec<u8>,
}

/// Durable metadata describing a snapshot of shard state at a given offset.
///
/// A snapshot captures the materialized state produced by replaying every
/// record up to and including `up_to_offset`, so future replay can start at the
/// snapshot instead of the log head. The snapshot is derived state, not new
/// truth: the log remains authoritative (ORCH-003, IDR-005).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotMeta {
    /// Shard the snapshot belongs to (SHARD-001).
    pub shard: ShardKey,
    /// Highest offset included in the snapshot (inclusive).
    pub up_to_offset: Offset,
    /// Term of the record at `up_to_offset`.
    pub term: Term,
    /// Content address of the serialized snapshot blob (ORCH-004).
    pub content: ContentId,
}

/// Errors surfaced by the durable store.
///
/// The variants describe *durability/consistency* failures only. This store
/// makes no cognitive decisions (PERSIST-001, ORCH-001): it neither validates
/// business meaning nor arbitrates truth -- it only reports whether the durable
/// contract held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalError {
    /// The requested shard has no WAL in this store.
    UnknownShard(ShardKey),
    /// A read/replay started from an offset that has been compacted away below
    /// the earliest retained record; callers should replay from a snapshot.
    OffsetCompacted { shard: ShardKey, earliest: Offset },
    /// The requested offset is beyond the current committed head.
    OffsetOutOfRange { shard: ShardKey, head: Offset },
    /// This node is not the leader for the shard, so it may not append; only
    /// the shard leader commits (IDR-001). The store refuses to fabricate truth.
    NotLeader(ShardKey),
    /// Underlying durable-medium failure (I/O, fsync, corruption detected).
    Durability(String),
    /// A detected append-only violation (attempt to overwrite/reorder a
    /// committed record). Rejecting this upholds the append-only WAL (IDR-005).
    AppendOnlyViolation { shard: ShardKey, offset: Offset },
}

/// Read cursor over a shard's WAL, yielding records in offset order.
///
/// Iteration is forward-only and gap-free within the requested range,
/// preserving the total order that makes the log a faithful decision trace
/// (IDR-005, ORCH-003).
pub trait ReplayCursor {
    /// Advance and return the next record, `Ok(None)` at end of the range, or a
    /// durability error. Records arrive in strictly increasing `Offset` order.
    fn next(&mut self) -> Result<Option<WalRecord>, WalError>;

    /// Offset the cursor will read next (its current position).
    fn position(&self) -> Offset;
}

// ---------------------------------------------------------------------------
// The WAL contract
// ---------------------------------------------------------------------------

/// Append-only Write-Ahead Log for one shard's committed decision trace.
///
/// This is the central contract of the Persistence layer. The Raft log, the
/// WAL, and the decision trace are one and the same artifact (IDR-005); this
/// trait is its durable, per-shard face.
///
/// Contract obligations (durable-store only, PERSIST-001 proposed):
/// - **Append-only** (IDR-005): `append` only ever adds; committed records are
///   never mutated, reordered, or deleted (compaction only drops a prefix
///   already captured by a snapshot).
/// - **Outcomes, not invocations** (IDR-002): callers append committed
///   outcomes; the store does not observe or record raw engine invocations.
/// - **Owns no truth** (ORCH-001, OWN-001): the store persists what the Kernel
///   commits; it arbitrates durability, never meaning.
/// - **Per-shard** (SHARD-001, IDR-001): each `Wal` instance is scoped to one
///   immutable `ShardKey`; there is no cross-shard atomic append (sagas span
///   shards at a higher layer, IDR-001).
/// - **Replay, not recompute** (ORCH-003): recovery reads records back via
///   `replay_from`; state is reconstructed from the trace, never recomputed.
///
/// All methods are signatures only in this I1 skeleton; no bodies are provided.
pub trait Wal {
    /// Concrete replay cursor produced by [`Wal::replay_from`].
    type Cursor: ReplayCursor;

    /// The immutable shard this WAL serves (SHARD-001).
    fn shard(&self) -> &ShardKey;

    /// Append one committed record, returning the durable `Offset` assigned to
    /// it. Appends are strictly increasing and gap-free.
    ///
    /// Only the shard leader may append (IDR-001); a non-leader must receive
    /// [`WalError::NotLeader`]. The record must be durable (survive crash)
    /// before this returns `Ok`. Attempting to write a position that already
    /// holds a committed record is an [`WalError::AppendOnlyViolation`]
    /// (append-only WAL, IDR-005).
    fn append(&mut self, record: PendingRecord) -> Result<Offset, WalError>;

    /// Capture a snapshot of materialized shard state up to and including the
    /// current committed head, returning its durable metadata.
    ///
    /// Snapshots enable prefix compaction and faster replay; they are derived
    /// state, not new truth -- the log stays authoritative (ORCH-003, IDR-005).
    fn snapshot(&mut self) -> Result<SnapshotMeta, WalError>;

    /// Open a forward cursor that replays committed records starting at
    /// `offset` (inclusive) through the current committed head.
    ///
    /// This is the recovery primitive mandated by ORCH-003 / IDR-003: truth is
    /// reconstructed by *re-reading the recorded decision trace*, never by
    /// recomputation. If `offset` precedes the earliest retained record the
    /// store returns [`WalError::OffsetCompacted`] so the caller can start from
    /// a snapshot instead.
    fn replay_from(&self, offset: Offset) -> Result<Self::Cursor, WalError>;

    /// Offset one past the last committed record (the append point).
    fn head(&self) -> Offset;

    /// Earliest offset still retained (records below this were compacted after
    /// being captured by a snapshot).
    fn earliest(&self) -> Offset;
}

/// Opens and manages per-shard [`Wal`] instances.
///
/// Partitioning is strictly by tenant/workspace with an immutable key
/// (SHARD-001), and each shard corresponds to one Raft group (IDR-001). This
/// factory is the store's entry point; it neither decides nor owns truth
/// (ORCH-001, PERSIST-001 proposed) -- it only provisions durable logs.
pub trait WalStore {
    /// The [`Wal`] implementation this store hands out.
    type Wal: Wal;

    /// Open (creating if absent) the durable WAL for `shard`.
    fn open(&self, shard: &ShardKey) -> Result<Self::Wal, WalError>;

    /// List the shards for which this store holds a durable WAL.
    fn shards(&self) -> Vec<ShardKey>;
}


// =============================================================================
// I1.4 Walking Skeleton: in-memory durable substrate (concrete impls).
//
// Scope: single process / single node / single shard. This is the append-only
// log the Kernel replays across a simulated restart (a dropped Kernel plus a
// fresh recover over the same store). File-backed durability is I1.5; this
// crate still owns NO truth (PERSIST-001) and never mutates a committed record
// (append-only, IDR-005).
// =============================================================================

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type SharedLog = Arc<Mutex<Vec<WalRecord>>>;

/// In-memory, append-only WAL for one shard. Cloning shares the same log.
#[derive(Clone)]
pub struct MemWal {
    shard: ShardKey,
    log: SharedLog,
}

/// Forward, gap-free replay cursor over a snapshot of a shard log.
pub struct MemCursor {
    records: Vec<WalRecord>,
    pos: usize,
    start: Offset,
}

impl ReplayCursor for MemCursor {
    fn next(&mut self) -> Result<Option<WalRecord>, WalError> {
        if self.pos < self.records.len() {
            let r = self.records[self.pos].clone();
            self.pos += 1;
            Ok(Some(r))
        } else {
            Ok(None)
        }
    }
    fn position(&self) -> Offset {
        self.start + self.pos as Offset
    }
}

impl Wal for MemWal {
    type Cursor = MemCursor;

    fn shard(&self) -> &ShardKey {
        &self.shard
    }

    fn append(&mut self, record: PendingRecord) -> Result<Offset, WalError> {
        // Single-node I1.4: this node is always the leader for its shard.
        let mut log = self.log.lock().expect("wal poisoned");
        let offset = log.len() as Offset;
        log.push(WalRecord {
            shard: record.shard,
            offset,
            term: record.term,
            kind: record.kind,
            content: record.content,
            payload: record.payload,
        });
        Ok(offset)
    }

    fn snapshot(&mut self) -> Result<SnapshotMeta, WalError> {
        // Minimal marker; real compaction is I1.6.
        let log = self.log.lock().expect("wal poisoned");
        let head = log.len() as Offset;
        let term = log.last().map(|r| r.term).unwrap_or(0);
        Ok(SnapshotMeta {
            shard: self.shard.clone(),
            up_to_offset: head.saturating_sub(1),
            term,
            content: ContentId(Vec::new()),
        })
    }

    fn replay_from(&self, offset: Offset) -> Result<Self::Cursor, WalError> {
        let log = self.log.lock().expect("wal poisoned");
        let head = log.len() as Offset;
        if offset > head {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head,
            });
        }
        let records = log[offset as usize..].to_vec();
        Ok(MemCursor {
            records,
            pos: 0,
            start: offset,
        })
    }

    fn head(&self) -> Offset {
        self.log.lock().expect("wal poisoned").len() as Offset
    }

    fn earliest(&self) -> Offset {
        0
    }
}

/// In-memory [`WalStore`] - the durable substrate that survives a simulated
/// restart. Cloning shares the same per-shard logs (Arc), so a fresh Kernel can
/// recover the truth an earlier Kernel committed.
#[derive(Clone, Default)]
pub struct MemWalStore {
    inner: Arc<Mutex<HashMap<ShardKey, SharedLog>>>,
}

impl MemWalStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl WalStore for MemWalStore {
    type Wal = MemWal;

    fn open(&self, shard: &ShardKey) -> Result<Self::Wal, WalError> {
        let mut map = self.inner.lock().expect("store poisoned");
        let log = map
            .entry(shard.clone())
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())))
            .clone();
        Ok(MemWal {
            shard: shard.clone(),
            log,
        })
    }

    fn shards(&self) -> Vec<ShardKey> {
        self.inner
            .lock()
            .expect("store poisoned")
            .keys()
            .cloned()
            .collect()
    }
}

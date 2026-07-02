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
//! STATUS: I1 in progress. The traits below are the frozen contract; concrete
//! implementations exist for the reference runtime -- `MemWal`/`MemWalStore`
//! (I1.4, in-memory) and `FileWal`/`FileWalStore` (I1.5, fsync-durable and
//! crash-consistent on disk). Frozen specification governs; this crate
//! implements, never changes it.

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
    /// The retained log is not dense/complete over the requested range: a
    /// committed record is missing (e.g. an interior segment is corrupt or a
    /// frame failed its CRC), so replay cannot faithfully reconstruct truth.
    /// Recovery MUST fail loudly on this rather than return a gapped trace
    /// (ORCH-003 losslessness; "lossless or loud").
    Corruption {
        /// The shard whose log is incomplete.
        shard: ShardKey,
        /// The first offset that was expected but not found (the gap point).
        missing_offset: Offset,
        /// Human-readable detail; not a stable API surface.
        detail: String,
    },
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

    /// Durably store a checkpoint of materialized shard state covering offsets
    /// `0..=up_to_offset`, returning its metadata (I1.6).
    ///
    /// The `state` blob is produced by the **Kernel** (the sole truth owner,
    /// ORCH-001) and is treated here as **opaque bytes**: this store never
    /// interprets it (PERSIST-001). The checkpoint must be durable (fsync +
    /// atomic rename) BEFORE any segment is compacted, so a crash never loses
    /// truth. Snapshots are derived state; the log stays authoritative
    /// (ORCH-003, IDR-005).
    ///
    /// Governing: RT-001 (this activates the previously-reserved
    /// [`SnapshotMeta`] / [`RecordKind::SnapshotMarker`] surface; it is Reference
    /// Runtime interface evolution, not a specification change).
    fn install_snapshot(
        &mut self,
        up_to_offset: Offset,
        term: Term,
        state: &[u8],
    ) -> Result<SnapshotMeta, WalError>;

    /// Load the latest durable checkpoint (metadata + opaque blob), if any, for
    /// restore. A torn/corrupt checkpoint is ignored (never restore corruption);
    /// recovery then falls back to an older checkpoint or full replay.
    fn load_snapshot(&self) -> Result<Option<(SnapshotMeta, Vec<u8>)>, WalError>;

    /// Drop WAL segments fully captured by a durable checkpoint at/below
    /// `up_to_offset`. Compaction ONLY deletes fully-covered segment files; it
    /// never rewrites a surviving record (append-only, IDR-005). After this,
    /// [`Wal::earliest`] advances to the first retained offset.
    fn compact(&mut self, up_to_offset: Offset) -> Result<(), WalError>;

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

/// Shared in-memory log for one shard: the records, a `base` offset that
/// advances on compaction (so the log models a droppable prefix), and the latest
/// in-memory checkpoint. Cloning [`MemWal`] shares this via `Arc`.
#[derive(Default)]
struct MemLog {
    records: Vec<WalRecord>,
    base: Offset,
    snapshot: Option<(SnapshotMeta, Vec<u8>)>,
}
type SharedLog = Arc<Mutex<MemLog>>;

/// In-memory, append-only WAL for one shard. Cloning shares the same log.
#[derive(Clone)]
pub struct MemWal {
    shard: ShardKey,
    log: SharedLog,
}

/// Forward, gap-free replay cursor over a decoded slice of a shard log. Shared
/// by both the in-memory (`MemWal`) and file-backed (`FileWal`) WALs: each hands
/// the cursor an already-materialized, offset-ordered record slice to iterate.
#[derive(Debug)]
pub struct VecReplayCursor {
    records: Vec<WalRecord>,
    pos: usize,
    start: Offset,
}

impl ReplayCursor for VecReplayCursor {
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
    type Cursor = VecReplayCursor;

    fn shard(&self) -> &ShardKey {
        &self.shard
    }

    fn append(&mut self, record: PendingRecord) -> Result<Offset, WalError> {
        // Single-node: this node is always the leader for its shard (SHARD-001:
        // no cross-shard append).
        if record.shard != self.shard {
            return Err(WalError::UnknownShard(record.shard));
        }
        let mut log = self.log.lock().expect("wal poisoned");
        let offset = log.base + log.records.len() as Offset;
        log.records.push(WalRecord {
            shard: record.shard,
            offset,
            term: record.term,
            kind: record.kind,
            content: record.content,
            payload: record.payload,
        });
        Ok(offset)
    }

    fn install_snapshot(
        &mut self,
        up_to_offset: Offset,
        term: Term,
        state: &[u8],
    ) -> Result<SnapshotMeta, WalError> {
        let mut log = self.log.lock().expect("wal poisoned");
        let head = log.base + log.records.len() as Offset;
        if head > 0 && up_to_offset > head - 1 {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head,
            });
        }
        // Opaque blob (Kernel-produced); we only fingerprint it for the meta.
        let meta = SnapshotMeta {
            shard: self.shard.clone(),
            up_to_offset,
            term,
            content: ContentId(crc32_ieee(state).to_le_bytes().to_vec()),
        };
        log.snapshot = Some((meta.clone(), state.to_vec()));
        Ok(meta)
    }

    fn load_snapshot(&self) -> Result<Option<(SnapshotMeta, Vec<u8>)>, WalError> {
        Ok(self.log.lock().expect("wal poisoned").snapshot.clone())
    }

    fn compact(&mut self, up_to_offset: Offset) -> Result<(), WalError> {
        let mut log = self.log.lock().expect("wal poisoned");
        let head = log.base + log.records.len() as Offset;
        if head == 0 {
            return Ok(());
        }
        if up_to_offset > head - 1 {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head,
            });
        }
        if up_to_offset < log.base {
            return Ok(()); // prefix already dropped
        }
        let drop_count = (up_to_offset + 1 - log.base) as usize;
        log.records.drain(0..drop_count);
        log.base = up_to_offset + 1;
        Ok(())
    }

    fn replay_from(&self, offset: Offset) -> Result<Self::Cursor, WalError> {
        let log = self.log.lock().expect("wal poisoned");
        let head = log.base + log.records.len() as Offset;
        if offset < log.base {
            return Err(WalError::OffsetCompacted {
                shard: self.shard.clone(),
                earliest: log.base,
            });
        }
        if offset > head {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head,
            });
        }
        let start_idx = (offset - log.base) as usize;
        let records = log.records[start_idx..].to_vec();
        Ok(VecReplayCursor {
            records,
            pos: 0,
            start: offset,
        })
    }

    fn head(&self) -> Offset {
        let log = self.log.lock().expect("wal poisoned");
        log.base + log.records.len() as Offset
    }

    fn earliest(&self) -> Offset {
        self.log.lock().expect("wal poisoned").base
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
            .or_insert_with(|| Arc::new(Mutex::new(MemLog::default())))
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


// =============================================================================
// I1.5/I1.6 Persistent WAL: file-backed, fsync-durable, crash-consistent, and
// SEGMENTED with checkpointing (concrete).
//
// Scope: single process / single node. Each shard is a DIRECTORY under the root
// containing append-only segment files (`seg-<start>.wal`) plus checkpoint files
// (`snap-<up_to>.snap`). A committed record is fsync'd BEFORE `append` returns
// (I1.5) and survives a real process exit; recovery is snapshot + tail replay
// (I1.6), so the WAL no longer grows unbounded.
//
// Durability / crash-consistency contract honoured here:
//   * append-only (IDR-005): frames are only ever added; committed frames are
//     never mutated or reordered. Compaction ONLY deletes fully-covered sealed
//     segment files -- it never rewrites a survivor.
//   * fsync-before-Ok: `File::sync_all` (FlushFileBuffers on Windows) completes
//     before `append` returns Ok -- the durability obligation of `Wal::append`.
//   * crash-safe checkpoints: written to `.tmp`, fsync'd, atomically renamed;
//     durable BEFORE any segment is compacted (never lose truth on a crash).
//   * torn-tail detection: each frame carries a CRC32 over its body; a crash
//     mid-write leaves a partial/garbage frame in the current segment, detected
//     on open and truncated away. Truth committed before the tear is intact.
//   * self-describing frames: length-prefixed body + trailing CRC, fixed-order
//     little-endian fields -> deterministic, dependency-free encoding.
// Snapshot creation belongs to the KERNEL (ORCH-001); this store treats the
// snapshot blob as opaque bytes (PERSIST-001). Still no networking/replication
// (arves-consensus, later). Interface evolution recorded under RT-001.
// =============================================================================

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// On-disk frame format version. Bumping this is a format migration, not a
/// silent change; decoders reject any other version (treated as corruption).
const WAL_FRAME_VERSION: u8 = 1;

/// Dependency-free CRC32 (IEEE 802.3, reflected polynomial 0xEDB88320).
/// Deterministic across runs and platforms; used to detect a torn/corrupt frame
/// at the WAL tail after a crash. Correctness over speed (reference runtime).
fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Dependency-free SHA-256 (FIPS 180-4). Used by `FileWal::integrity_digest` (RCR-002) to fold a
/// tamper-evident hash-chain over the committed record trace. Deterministic across runs/platforms;
/// no external crates (the workspace has zero non-`arves-*` dependencies).
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b); h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f); h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Reversible, filesystem-safe encoding of a shard-key component's bytes.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    fn nib(c: u8) -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        out.push((nib(bytes[i])? << 4) | nib(bytes[i + 1])?);
        i += 2;
    }
    Some(out)
}

// -- record framing -----------------------------------------------------------

fn put_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn put_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn put_bytes(buf: &mut Vec<u8>, b: &[u8]) {
    put_u32(buf, b.len() as u32);
    buf.extend_from_slice(b);
}

fn kind_to_u8(k: &RecordKind) -> u8 {
    match k {
        RecordKind::Outcome => 0,
        RecordKind::Membership => 1,
        RecordKind::SnapshotMarker => 2,
        RecordKind::Barrier => 3,
    }
}
fn u8_to_kind(v: u8) -> Option<RecordKind> {
    match v {
        0 => Some(RecordKind::Outcome),
        1 => Some(RecordKind::Membership),
        2 => Some(RecordKind::SnapshotMarker),
        3 => Some(RecordKind::Barrier),
        _ => None,
    }
}

/// Serialize one record BODY (exactly the bytes the CRC covers) in fixed field
/// order. The `offset` is stored so replay can validate density/position.
fn encode_body(
    shard: &ShardKey,
    offset: Offset,
    term: Term,
    kind: &RecordKind,
    content: &ContentId,
    payload: &[u8],
) -> Vec<u8> {
    let mut b = Vec::new();
    b.push(WAL_FRAME_VERSION);
    b.push(kind_to_u8(kind));
    put_u64(&mut b, term);
    put_u64(&mut b, offset);
    put_bytes(&mut b, shard.tenant.as_bytes());
    put_bytes(&mut b, shard.workspace.as_bytes());
    put_bytes(&mut b, &content.0);
    put_bytes(&mut b, payload);
    b
}

/// Minimal, fully bounds-checked reader over a body slice. Any short read yields
/// `None`, which the caller treats as corruption (tail truncation).
struct BodyReader<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> BodyReader<'a> {
    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }
    fn u32(&mut self) -> Option<u32> {
        let end = self.pos.checked_add(4)?;
        let s = self.b.get(self.pos..end)?;
        self.pos = end;
        Some(u32::from_le_bytes(s.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        let end = self.pos.checked_add(8)?;
        let s = self.b.get(self.pos..end)?;
        self.pos = end;
        Some(u64::from_le_bytes(s.try_into().ok()?))
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let n = self.u32()? as usize;
        let end = self.pos.checked_add(n)?;
        let s = self.b.get(self.pos..end)?;
        self.pos = end;
        Some(s.to_vec())
    }
    fn done(&self) -> bool {
        self.pos == self.b.len()
    }
}

/// Decode a BODY into a `WalRecord` for `shard` at `expected_offset`. Returns
/// `None` on ANY structural mismatch (version, kind, bad utf-8, trailing bytes,
/// offset/shard mismatch) -- every such case is treated as corruption so replay
/// stops and the tail is truncated. This upholds "never serve corrupt truth".
fn decode_body(body: &[u8], shard: &ShardKey, expected_offset: Offset) -> Option<WalRecord> {
    let mut r = BodyReader { b: body, pos: 0 };
    let version = r.u8()?;
    if version != WAL_FRAME_VERSION {
        return None;
    }
    let kind = u8_to_kind(r.u8()?)?;
    let term = r.u64()?;
    let offset = r.u64()?;
    let tenant = String::from_utf8(r.bytes()?).ok()?;
    let workspace = String::from_utf8(r.bytes()?).ok()?;
    let content = ContentId(r.bytes()?);
    let payload = r.bytes()?;
    if !r.done() {
        return None; // trailing garbage inside the framed body
    }
    // Integrity: offsets are dense & in position (SHARD-001 per-shard order), and
    // the frame must belong to the file's shard.
    if offset != expected_offset {
        return None;
    }
    if tenant != shard.tenant || workspace != shard.workspace {
        return None;
    }
    Some(WalRecord {
        shard: shard.clone(),
        offset,
        term,
        kind,
        content,
        payload,
    })
}

/// Decode every intact frame from `bytes`, stopping at the first torn/corrupt
/// frame (a crash mid-append). Returns the decoded records plus the byte length
/// of the good prefix, so callers can `set_len` to truncate the torn tail
/// (crash-consistency; append-only, IDR-005).
fn decode_all(bytes: &[u8], shard: &ShardKey, expected_start: Offset) -> (Vec<WalRecord>, usize) {
    let mut recs = Vec::new();
    let mut pos = 0usize;
    let mut expected: Offset = expected_start;
    loop {
        if pos == bytes.len() {
            break; // clean end of log
        }
        let frame_start = pos;
        // frame = [u32 body_len][body][u32 crc(body)]
        if pos + 4 > bytes.len() {
            break; // torn length prefix -> good prefix ends at frame_start
        }
        let body_len =
            u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let body_start = pos + 4;
        let body_end = match body_start.checked_add(body_len) {
            Some(v) => v,
            None => break,
        };
        if body_end + 4 > bytes.len() {
            pos = frame_start;
            break; // torn body or missing CRC
        }
        let body = &bytes[body_start..body_end];
        let stored_crc =
            u32::from_le_bytes(bytes[body_end..body_end + 4].try_into().unwrap());
        if crc32_ieee(body) != stored_crc {
            pos = frame_start;
            break; // corrupt frame
        }
        match decode_body(body, shard, expected) {
            Some(rec) => {
                recs.push(rec);
                expected += 1;
                pos = body_end + 4;
            }
            None => {
                pos = frame_start;
                break; // structural corruption
            }
        }
    }
    (recs, pos)
}

// -- file-backed, segmented WAL + checkpoints (I1.6) --------------------------

/// A parsed segment file: its start offset and path.
struct SegInfo {
    start: Offset,
    path: PathBuf,
}

fn seg_name(start: Offset) -> String {
    format!("seg-{start:020}.wal")
}
fn parse_seg(name: &str) -> Option<Offset> {
    name.strip_prefix("seg-")?.strip_suffix(".wal")?.parse().ok()
}
fn snap_name(up_to: Offset) -> String {
    format!("snap-{up_to:020}.snap")
}
fn parse_snap(name: &str) -> Option<Offset> {
    name.strip_prefix("snap-")?.strip_suffix(".snap")?.parse().ok()
}

/// List a shard directory's segment files, sorted by start offset (ascending).
fn list_segments(dir: &Path) -> Vec<SegInfo> {
    let mut segs = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Some(n) = e.file_name().to_str() {
                if let Some(start) = parse_seg(n) {
                    segs.push(SegInfo {
                        start,
                        path: e.path(),
                    });
                }
            }
        }
    }
    segs.sort_by_key(|s| s.start);
    segs
}

/// Checkpoint file format version.
const SNAP_VERSION: u8 = 1;

/// Encode a checkpoint file: version | up_to | term | blob_len | blob | crc32.
/// The blob is the Kernel's opaque materialized-state bytes (ORCH-001).
fn encode_snapshot(up_to: Offset, term: Term, blob: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(blob.len() + 25);
    b.push(SNAP_VERSION);
    put_u64(&mut b, up_to);
    put_u64(&mut b, term);
    put_bytes(&mut b, blob);
    let crc = crc32_ieee(&b);
    put_u32(&mut b, crc);
    b
}

/// Decode a checkpoint file. `None` if torn/corrupt (ignored on load, never
/// restore corruption).
fn decode_snapshot(bytes: &[u8]) -> Option<(Offset, Term, Vec<u8>)> {
    if bytes.len() < 4 {
        return None;
    }
    let body = &bytes[..bytes.len() - 4];
    let stored_crc = u32::from_le_bytes(bytes[bytes.len() - 4..].try_into().ok()?);
    if crc32_ieee(body) != stored_crc {
        return None;
    }
    let mut r = BodyReader { b: body, pos: 0 };
    if r.u8()? != SNAP_VERSION {
        return None;
    }
    let up_to = r.u64()?;
    let term = r.u64()?;
    let blob = r.bytes()?;
    if !r.done() {
        return None;
    }
    Some((up_to, term, blob))
}

struct FileWalInner {
    dir: PathBuf,
    /// Append handle to the current (last) segment.
    current: File,
    /// Start offset of the current segment.
    current_start: Offset,
    /// Records already written to the current segment.
    current_count: u64,
    /// Offset one past the last durable record (the append point).
    head: Offset,
    /// First retained offset (advances as compaction deletes covered segments).
    earliest: Offset,
    /// Max records per segment before rotation.
    rotate_every: u64,
}

/// File-backed, fsync-durable, append-only, SEGMENTED WAL for one shard (I1.6).
/// A shard is a directory of `seg-<start>.wal` segments plus `snap-<up_to>.snap`
/// checkpoints. Cloning shares the open handle + head (Arc). The durable
/// substrate is the filesystem, so truth outlives every handle and process.
#[derive(Clone)]
pub struct FileWal {
    shard: ShardKey,
    inner: Arc<Mutex<FileWalInner>>,
}

impl FileWal {
    /// Roll to a fresh segment starting at the current head. The previous segment
    /// becomes a sealed, deletable-on-compaction file. Append-only is preserved:
    /// a sealed segment is never rewritten.
    fn rotate(inner: &mut FileWalInner) -> Result<(), WalError> {
        let path = inner.dir.join(seg_name(inner.head));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| WalError::Durability(format!("rotate open: {e}")))?;
        inner.current = file;
        inner.current_start = inner.head;
        inner.current_count = 0;
        Ok(())
    }

    /// RCR-002 — tamper-EVIDENT integrity digest over the committed trace.
    ///
    /// Folds a SHA-256 hash-chain over every retained record in offset order:
    /// `d₀ = SHA256(genesis(shard))`, `dᵢ = SHA256(dᵢ₋₁ ‖ bodyᵢ)`, where `bodyᵢ` is the record's
    /// canonical serialization. ANY alteration of ANY committed record changes the digest —
    /// including a tamper that repairs the per-frame CRC32 (which `decode_all` would otherwise
    /// accept). Intended use is *anchoring*: a trusted holder (the Kernel, or a checkpoint) records
    /// the expected digest and compares later; a mismatch proves tampering.
    ///
    /// SCOPE (honest): this is tamper-EVIDENCE, not tamper-PROOFing. A fully hostile host that
    /// rewrites the whole trace AND the anchor cannot be stopped by a hash-chain alone — that needs
    /// cryptographic signatures + an authenticated commit path (a v2.0 change, outside v1.0's
    /// trusted-single-host threat model). This method adds the chain such a scheme will sign, and
    /// closes the "edit one committed record + repair its CRC" hole today.
    pub fn integrity_digest(&self) -> Result<[u8; 32], WalError> {
        let inner = self.inner.lock().expect("filewal poisoned");
        // Genesis binds the chain to the shard identity (SHARD-001), so digests are not
        // comparable across shards.
        let mut genesis = self.shard.tenant.as_bytes().to_vec();
        genesis.push(0);
        genesis.extend_from_slice(self.shard.workspace.as_bytes());
        let mut running = sha256(&genesis);
        for seg in list_segments(&inner.dir) {
            let bytes = fs::read(&seg.path)
                .map_err(|e| WalError::Durability(format!("integrity read: {e}")))?;
            let (recs, _good) = decode_all(&bytes, &self.shard, seg.start);
            for r in recs {
                let body = encode_body(&r.shard, r.offset, r.term, &r.kind, &r.content, &r.payload);
                let mut buf = running.to_vec();
                buf.extend_from_slice(&body);
                running = sha256(&buf);
            }
        }
        Ok(running)
    }
}

impl Wal for FileWal {
    type Cursor = VecReplayCursor;

    fn shard(&self) -> &ShardKey {
        &self.shard
    }

    fn append(&mut self, record: PendingRecord) -> Result<Offset, WalError> {
        // SHARD-001: a WAL only accepts records for its own shard.
        if record.shard != self.shard {
            return Err(WalError::UnknownShard(record.shard));
        }
        let mut inner = self.inner.lock().expect("filewal poisoned");
        // WAL rotation: seal the current segment and start a new one at head.
        if inner.current_count >= inner.rotate_every {
            FileWal::rotate(&mut inner)?;
        }
        let offset = inner.head;
        let body = encode_body(
            &self.shard,
            offset,
            record.term,
            &record.kind,
            &record.content,
            &record.payload,
        );
        let mut frame = Vec::with_capacity(body.len() + 8);
        put_u32(&mut frame, body.len() as u32);
        frame.extend_from_slice(&body);
        put_u32(&mut frame, crc32_ieee(&body));
        inner
            .current
            .write_all(&frame)
            .map_err(|e| WalError::Durability(format!("append write: {e}")))?;
        // DURABILITY: fsync before Ok (Wal::append contract; FlushFileBuffers).
        inner
            .current
            .sync_all()
            .map_err(|e| WalError::Durability(format!("fsync: {e}")))?;
        inner.current_count += 1;
        inner.head = offset + 1;
        Ok(offset)
    }

    fn install_snapshot(
        &mut self,
        up_to_offset: Offset,
        term: Term,
        state: &[u8],
    ) -> Result<SnapshotMeta, WalError> {
        let inner = self.inner.lock().expect("filewal poisoned");
        if inner.head > 0 && up_to_offset > inner.head - 1 {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head: inner.head,
            });
        }
        // Crash-safe: write .tmp, fsync, atomic rename, best-effort dir fsync.
        // The checkpoint is durable BEFORE any segment is ever compacted, so a
        // crash between snapshot and compaction can never lose truth.
        let final_path = inner.dir.join(snap_name(up_to_offset));
        let tmp_path = inner.dir.join(format!("{}.tmp", snap_name(up_to_offset)));
        let encoded = encode_snapshot(up_to_offset, term, state);
        {
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&tmp_path)
                .map_err(|e| WalError::Durability(format!("snap tmp open: {e}")))?;
            f.write_all(&encoded)
                .map_err(|e| WalError::Durability(format!("snap write: {e}")))?;
            f.sync_all()
                .map_err(|e| WalError::Durability(format!("snap fsync: {e}")))?;
        }
        fs::rename(&tmp_path, &final_path)
            .map_err(|e| WalError::Durability(format!("snap rename: {e}")))?;
        // Best-effort directory fsync (std has no portable dir-fsync; documented).
        if let Ok(d) = File::open(&inner.dir) {
            let _ = d.sync_all();
        }
        Ok(SnapshotMeta {
            shard: self.shard.clone(),
            up_to_offset,
            term,
            content: ContentId(crc32_ieee(state).to_le_bytes().to_vec()),
        })
    }

    fn load_snapshot(&self) -> Result<Option<(SnapshotMeta, Vec<u8>)>, WalError> {
        let inner = self.inner.lock().expect("filewal poisoned");
        // Highest valid up_to wins; skip torn/corrupt checkpoints.
        let mut snaps: Vec<Offset> = Vec::new();
        if let Ok(rd) = fs::read_dir(&inner.dir) {
            for e in rd.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if let Some(up_to) = parse_snap(n) {
                        snaps.push(up_to);
                    }
                }
            }
        }
        snaps.sort_unstable();
        for up_to in snaps.into_iter().rev() {
            let path = inner.dir.join(snap_name(up_to));
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if let Some((decoded_up_to, term, blob)) = decode_snapshot(&bytes) {
                if decoded_up_to != up_to {
                    continue; // filename/content mismatch -> ignore
                }
                let meta = SnapshotMeta {
                    shard: self.shard.clone(),
                    up_to_offset: up_to,
                    term,
                    content: ContentId(crc32_ieee(&blob).to_le_bytes().to_vec()),
                };
                return Ok(Some((meta, blob)));
            }
        }
        Ok(None)
    }

    fn compact(&mut self, up_to_offset: Offset) -> Result<(), WalError> {
        let mut inner = self.inner.lock().expect("filewal poisoned");
        if inner.head == 0 {
            return Ok(());
        }
        if up_to_offset > inner.head - 1 {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head: inner.head,
            });
        }
        let segs = list_segments(&inner.dir);
        // Delete only SEALED segments (never the current) fully at/below up_to.
        // Sealed segment i spans [start_i, start_{i+1}-1]; deletable iff its end
        // <= up_to. Append-only: whole covered files are unlinked, never rewritten.
        for i in 0..segs.len() {
            if segs[i].start == inner.current_start {
                continue; // never delete the active append segment
            }
            let end = if i + 1 < segs.len() {
                segs[i + 1].start.saturating_sub(1)
            } else {
                inner.head.saturating_sub(1)
            };
            if end <= up_to_offset {
                fs::remove_file(&segs[i].path)
                    .map_err(|e| WalError::Durability(format!("compact unlink: {e}")))?;
            }
        }
        // Delete checkpoints strictly superseded by the one at up_to_offset.
        if let Ok(rd) = fs::read_dir(&inner.dir) {
            for e in rd.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if let Some(u) = parse_snap(n) {
                        if u < up_to_offset {
                            let _ = fs::remove_file(e.path());
                        }
                    }
                }
            }
        }
        // Recompute earliest from the retained segments.
        let remaining = list_segments(&inner.dir);
        inner.earliest = remaining
            .first()
            .map(|s| s.start)
            .unwrap_or(inner.current_start);
        Ok(())
    }

    fn replay_from(&self, offset: Offset) -> Result<Self::Cursor, WalError> {
        let inner = self.inner.lock().expect("filewal poisoned");
        if offset < inner.earliest {
            return Err(WalError::OffsetCompacted {
                shard: self.shard.clone(),
                earliest: inner.earliest,
            });
        }
        if offset > inner.head {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head: inner.head,
            });
        }
        // Reconstruct truth from the recorded trace (ORCH-003): decode retained
        // segments in offset order and keep records at/after `offset`.
        let mut records = Vec::new();
        for seg in list_segments(&inner.dir) {
            let bytes = fs::read(&seg.path)
                .map_err(|e| WalError::Durability(format!("replay read: {e}")))?;
            let (recs, _good) = decode_all(&bytes, &self.shard, seg.start);
            for r in recs {
                if r.offset >= offset {
                    records.push(r);
                }
            }
        }
        // INTEGRITY (I1.7): the range [offset, head) MUST be dense and complete.
        // decode_all stops at the first torn/corrupt frame, so a corrupt INTERIOR
        // (sealed) segment silently drops its suffix -- detect that here and fail
        // loudly instead of returning a gapped trace ("lossless or loud").
        let mut expected = offset;
        for r in &records {
            if r.offset != expected {
                return Err(WalError::Corruption {
                    shard: self.shard.clone(),
                    missing_offset: expected,
                    detail: format!(
                        "non-contiguous replay: expected offset {expected}, found {}",
                        r.offset
                    ),
                });
            }
            expected += 1;
        }
        if expected != inner.head {
            return Err(WalError::Corruption {
                shard: self.shard.clone(),
                missing_offset: expected,
                detail: format!(
                    "incomplete replay: reached offset {expected}, head is {}",
                    inner.head
                ),
            });
        }
        Ok(VecReplayCursor {
            records,
            pos: 0,
            start: offset,
        })
    }

    fn head(&self) -> Offset {
        self.inner.lock().expect("filewal poisoned").head
    }

    fn earliest(&self) -> Offset {
        self.inner.lock().expect("filewal poisoned").earliest
    }
}

/// Default segment-rotation threshold (records per segment).
const DEFAULT_ROTATE_EVERY: u64 = 1024;

/// File-backed [`WalStore`]: one directory per shard beneath a root
/// (`<hex tenant>__<hex workspace>/`) holding segment + checkpoint files.
/// Cloning shares the open-WAL cache (Arc); the durable substrate is the
/// filesystem, so a FRESH store over the same root recovers the truth a prior
/// (even dead) process committed.
#[derive(Clone)]
pub struct FileWalStore {
    root: PathBuf,
    rotate_every: u64,
    open_wals: Arc<Mutex<HashMap<ShardKey, Arc<Mutex<FileWalInner>>>>>,
}

impl FileWalStore {
    /// Open a file-backed store rooted at `root`, creating it if absent. Uses the
    /// default rotation threshold. This is the durable entry point on startup.
    pub fn open_root<P: Into<PathBuf>>(root: P) -> Result<Self, WalError> {
        Self::open_root_with_rotation(root, DEFAULT_ROTATE_EVERY)
    }

    /// Open a store with an explicit segment-rotation threshold (records per
    /// segment). Small values exercise rotation/compaction in tests.
    pub fn open_root_with_rotation<P: Into<PathBuf>>(
        root: P,
        rotate_every: u64,
    ) -> Result<Self, WalError> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|e| WalError::Durability(format!("create root: {e}")))?;
        Ok(FileWalStore {
            root,
            rotate_every: rotate_every.max(1),
            open_wals: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn dir_for(&self, shard: &ShardKey) -> PathBuf {
        let name = format!(
            "{}__{}",
            hex_encode(shard.tenant.as_bytes()),
            hex_encode(shard.workspace.as_bytes())
        );
        self.root.join(name)
    }
}

impl WalStore for FileWalStore {
    type Wal = FileWal;

    fn open(&self, shard: &ShardKey) -> Result<Self::Wal, WalError> {
        let mut cache = self.open_wals.lock().expect("store poisoned");
        if let Some(inner) = cache.get(shard) {
            return Ok(FileWal {
                shard: shard.clone(),
                inner: inner.clone(),
            });
        }
        let dir = self.dir_for(shard);
        fs::create_dir_all(&dir)
            .map_err(|e| WalError::Durability(format!("create shard dir: {e}")))?;

        // Sweep orphan checkpoint temp files left by a crash in install_snapshot's
        // fsync->rename window (I1.7). They are harmless to recovery (parse_snap
        // ignores them) but would otherwise leak without bound across crash
        // cycles, so recovery reclaims them.
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.flatten() {
                if let Some(n) = e.file_name().to_str() {
                    if n.starts_with("snap-") && n.ends_with(".snap.tmp") {
                        let _ = fs::remove_file(e.path());
                    }
                }
            }
        }

        // Recover head/earliest by scanning the directory. Only the LAST (current)
        // segment can have a torn tail (a crash mid-append); repair it there.
        let segs = list_segments(&dir);
        let (current_start, current_count, head, earliest) = if segs.is_empty() {
            (0u64, 0u64, 0u64, 0u64)
        } else {
            let earliest = segs[0].start;
            let last = segs.last().unwrap();
            let bytes = fs::read(&last.path)
                .map_err(|e| WalError::Durability(format!("open read: {e}")))?;
            let (recs, good_len) = decode_all(&bytes, shard, last.start);
            if good_len != bytes.len() {
                let f = OpenOptions::new()
                    .write(true)
                    .open(&last.path)
                    .map_err(|e| WalError::Durability(format!("open for truncate: {e}")))?;
                f.set_len(good_len as u64)
                    .map_err(|e| WalError::Durability(format!("truncate: {e}")))?;
                f.sync_all()
                    .map_err(|e| WalError::Durability(format!("truncate fsync: {e}")))?;
            }
            let count = recs.len() as u64;
            (last.start, count, last.start + count, earliest)
        };

        let current_path = dir.join(seg_name(current_start));
        let current = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&current_path)
            .map_err(|e| WalError::Durability(format!("open append: {e}")))?;

        let inner = Arc::new(Mutex::new(FileWalInner {
            dir,
            current,
            current_start,
            current_count,
            head,
            earliest,
            rotate_every: self.rotate_every,
        }));
        cache.insert(shard.clone(), inner.clone());
        Ok(FileWal {
            shard: shard.clone(),
            inner,
        })
    }

    fn shards(&self) -> Vec<ShardKey> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(&self.root) {
            for entry in rd.flatten() {
                // Shards are DIRECTORIES named <hex tenant>__<hex workspace>.
                if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    continue;
                }
                let name = entry.file_name();
                let name = match name.to_str() {
                    Some(n) => n,
                    None => continue,
                };
                let (a, b) = match name.split_once("__") {
                    Some(p) => p,
                    None => continue,
                };
                let (tb, wb) = match (hex_decode(a), hex_decode(b)) {
                    (Some(t), Some(w)) => (t, w),
                    _ => continue,
                };
                match (String::from_utf8(tb), String::from_utf8(wb)) {
                    (Ok(tenant), Ok(workspace)) => out.push(ShardKey { tenant, workspace }),
                    _ => continue,
                }
            }
        }
        // Deterministic order for replay across shards (SHARD-001: offsets are
        // still per-shard; this only fixes iteration order, not a global index).
        out.sort();
        out
    }
}

#[cfg(test)]
mod rcr002_integrity {
    //! RCR-002: the WAL integrity digest is deterministic and detects a CRC-repaired tamper of a
    //! committed record — the exact hole a per-frame CRC cannot cover.
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("arves-rcr002-{}-{}", tag, std::process::id()))
    }
    fn shard() -> ShardKey {
        ShardKey { tenant: "acme".into(), workspace: "w1".into() }
    }
    fn append_n(wal: &mut FileWal, sh: &ShardKey, n: u64) {
        for i in 0..n {
            wal.append(PendingRecord {
                shard: sh.clone(),
                term: 1,
                kind: RecordKind::Outcome,
                content: ContentId(vec![i as u8]),
                payload: vec![i as u8; 8],
            })
            .unwrap();
        }
    }

    #[test]
    fn integrity_digest_is_deterministic_across_reopen() {
        let root = tmp_root("det");
        let _ = fs::remove_dir_all(&root);
        let sh = shard();
        let store = FileWalStore::open_root(&root).unwrap();
        let mut wal = store.open(&sh).unwrap();
        append_n(&mut wal, &sh, 5);
        let d1 = wal.integrity_digest().unwrap();
        // A FRESH store over the same durable root must recompute the SAME digest.
        let store2 = FileWalStore::open_root(&root).unwrap();
        let wal2 = store2.open(&sh).unwrap();
        assert_eq!(d1, wal2.integrity_digest().unwrap(), "digest must be deterministic");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn integrity_digest_detects_crc_fixed_tamper() {
        let root = tmp_root("tamper");
        let _ = fs::remove_dir_all(&root);
        let sh = shard();
        let store = FileWalStore::open_root(&root).unwrap();
        let mut wal = store.open(&sh).unwrap();
        append_n(&mut wal, &sh, 5);
        let d1 = wal.integrity_digest().unwrap();

        // TAMPER a committed record on disk: flip a payload byte AND repair its CRC32, so the frame
        // still passes decode_all (the exact attack a per-frame CRC cannot detect).
        let dir = root.join(format!(
            "{}__{}",
            hex_encode(sh.tenant.as_bytes()),
            hex_encode(sh.workspace.as_bytes())
        ));
        let seg = dir.join(seg_name(0));
        let mut bytes = fs::read(&seg).unwrap();
        let body_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let flip = 4 + body_len - 1; // last payload byte of the first frame
        bytes[flip] ^= 0xFF;
        let new_crc = crc32_ieee(&bytes[4..4 + body_len]);
        bytes[4 + body_len..8 + body_len].copy_from_slice(&new_crc.to_le_bytes());
        fs::write(&seg, &bytes).unwrap();

        // Recovery ACCEPTS the tampered record (CRC now matches) — CRC is blind to this...
        let store2 = FileWalStore::open_root(&root).unwrap();
        let wal2 = store2.open(&sh).unwrap();
        assert_eq!(wal2.head(), 5, "recovery accepts the CRC-repaired frame");
        // ...but the integrity digest CHANGES — the tamper is detected.
        assert_ne!(d1, wal2.integrity_digest().unwrap(), "digest must change on tamper");
        let _ = fs::remove_dir_all(&root);
    }
}

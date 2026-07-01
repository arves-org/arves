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

/// Forward, gap-free replay cursor over a decoded slice of a shard log. Shared
/// by both the in-memory (`MemWal`) and file-backed (`FileWal`) WALs: each hands
/// the cursor an already-materialized, offset-ordered record slice to iterate.
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
        Ok(VecReplayCursor {
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


// =============================================================================
// I1.5 Persistent WAL: file-backed, fsync-durable, crash-consistent (concrete).
//
// Scope: single process / single node; each shard is one append-only file under
// a root directory. This is where persistence becomes REAL for the first time:
// a committed record is fsync'd to disk BEFORE `append` returns, survives a full
// process exit, and is recovered byte-identically by a fresh process that only
// has the directory path (ORCH-003 replay from the recorded trace).
//
// Durability / crash-consistency contract honoured here:
//   * append-only (IDR-005): frames are only ever added; committed frames are
//     never mutated or reordered (prefix compaction is deferred to I1.6).
//   * fsync-before-Ok: `File::sync_all` (FlushFileBuffers on Windows) completes
//     before `append` returns Ok -- the durability obligation of `Wal::append`.
//   * torn-tail detection: each frame carries a CRC32 over its body; a crash
//     mid-write leaves a partial/garbage frame at the tail, detected on open and
//     truncated away. Truth committed before the tear is intact.
//   * self-describing frames: length-prefixed body + trailing CRC, fixed-order
//     little-endian fields -> deterministic, dependency-free encoding.
// Still owns NO truth (PERSIST-001); still no networking/replication (that is
// arves-consensus, later). Snapshots stay minimal markers (I1.6).
// =============================================================================

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

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
fn decode_all(bytes: &[u8], shard: &ShardKey) -> (Vec<WalRecord>, usize) {
    let mut recs = Vec::new();
    let mut pos = 0usize;
    let mut expected: Offset = 0;
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

// -- file-backed WAL ----------------------------------------------------------

struct FileWalInner {
    path: PathBuf,
    /// Append handle (opened `create(true).append(true)`).
    file: File,
    /// Offset one past the last durable record (the append point). The shard key
    /// itself lives on the [`FileWal`] handle (used for framing + validation).
    head: Offset,
}

/// File-backed, fsync-durable, append-only WAL for one shard. Cloning shares the
/// same open handle + `head` (Arc), matching `MemWal`'s intra-process semantics.
/// The durable substrate is the file itself, so truth outlives every handle.
#[derive(Clone)]
pub struct FileWal {
    shard: ShardKey,
    inner: Arc<Mutex<FileWalInner>>,
}

impl Wal for FileWal {
    type Cursor = VecReplayCursor;

    fn shard(&self) -> &ShardKey {
        &self.shard
    }

    fn append(&mut self, record: PendingRecord) -> Result<Offset, WalError> {
        // SHARD-001: a WAL only accepts records for its own shard; there is no
        // cross-shard append.
        if record.shard != self.shard {
            return Err(WalError::UnknownShard(record.shard));
        }
        let mut inner = self.inner.lock().expect("filewal poisoned");
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
            .file
            .write_all(&frame)
            .map_err(|e| WalError::Durability(format!("append write: {e}")))?;
        // DURABILITY: the record must survive a crash BEFORE we return Ok
        // (the Wal::append contract). sync_all == FlushFileBuffers on Windows.
        inner
            .file
            .sync_all()
            .map_err(|e| WalError::Durability(format!("fsync: {e}")))?;
        inner.head = offset + 1;
        Ok(offset)
    }

    fn snapshot(&mut self) -> Result<SnapshotMeta, WalError> {
        // Minimal marker; real compaction/checkpoint is I1.6.
        let inner = self.inner.lock().expect("filewal poisoned");
        Ok(SnapshotMeta {
            shard: self.shard.clone(),
            up_to_offset: inner.head.saturating_sub(1),
            term: 0,
            content: ContentId(Vec::new()),
        })
    }

    fn replay_from(&self, offset: Offset) -> Result<Self::Cursor, WalError> {
        let inner = self.inner.lock().expect("filewal poisoned");
        // Re-read the fsync'd bytes; reconstruct truth from the trace (ORCH-003).
        let bytes = fs::read(&inner.path)
            .map_err(|e| WalError::Durability(format!("replay read: {e}")))?;
        let (recs, _good_len) = decode_all(&bytes, &self.shard);
        let head = recs.len() as Offset;
        if offset > head {
            return Err(WalError::OffsetOutOfRange {
                shard: self.shard.clone(),
                head,
            });
        }
        let records = recs[offset as usize..].to_vec();
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
        0
    }
}

/// File-backed [`WalStore`]: one append-only file per shard beneath a root
/// directory (`<hex tenant>__<hex workspace>.wal`). Cloning shares the open-WAL
/// cache (Arc); the durable substrate is the filesystem, so a FRESH store over
/// the same root recovers the truth a prior (even dead) process committed.
#[derive(Clone)]
pub struct FileWalStore {
    root: PathBuf,
    open_wals: Arc<Mutex<HashMap<ShardKey, Arc<Mutex<FileWalInner>>>>>,
}

impl FileWalStore {
    /// Open a file-backed store rooted at `root`, creating the directory if
    /// absent. This is the durable entry point a process uses on startup.
    pub fn open_root<P: Into<PathBuf>>(root: P) -> Result<Self, WalError> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|e| WalError::Durability(format!("create root: {e}")))?;
        Ok(FileWalStore {
            root,
            open_wals: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn path_for(&self, shard: &ShardKey) -> PathBuf {
        let name = format!(
            "{}__{}.wal",
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
        let path = self.path_for(shard);
        // Recover `head` and repair any torn tail from whatever is on disk. This
        // scan IS the recovery path for a fresh process (empty cache).
        let head = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|e| WalError::Durability(format!("open read: {e}")))?;
            let (recs, good_len) = decode_all(&bytes, shard);
            if good_len != bytes.len() {
                // Crash-consistency: drop the torn/garbage tail so future appends
                // continue from the last durable record (append-only, IDR-005).
                let f = OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .map_err(|e| WalError::Durability(format!("open for truncate: {e}")))?;
                f.set_len(good_len as u64)
                    .map_err(|e| WalError::Durability(format!("truncate: {e}")))?;
                f.sync_all()
                    .map_err(|e| WalError::Durability(format!("truncate fsync: {e}")))?;
            }
            recs.len() as Offset
        } else {
            0
        };
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| WalError::Durability(format!("open append: {e}")))?;
        let inner = Arc::new(Mutex::new(FileWalInner { path, file, head }));
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
                let name = entry.file_name();
                let name = match name.to_str() {
                    Some(n) => n,
                    None => continue,
                };
                let stem = match name.strip_suffix(".wal") {
                    Some(s) => s,
                    None => continue,
                };
                let (a, b) = match stem.split_once("__") {
                    Some(p) => p,
                    None => continue,
                };
                let (tb, wb) = match (hex_decode(a), hex_decode(b)) {
                    (Some(t), Some(w)) => (t, w),
                    _ => continue,
                };
                match (String::from_utf8(tb), String::from_utf8(wb)) {
                    (Ok(tenant), Ok(workspace)) => {
                        out.push(ShardKey { tenant, workspace })
                    }
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

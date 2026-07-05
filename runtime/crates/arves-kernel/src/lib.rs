//! ARVES :: arves-kernel
//!
//! Purpose: Owner of cognitive TRUTH and the SOLE commit gateway.
//! Governing: ORCH-001, OWN-001; G-001 (proposed). Vol 9 v2; Amendments.
//! Layer: Data Plane (LAYER-001 position: Kernel, below Information Platform,
//!        above Persistence).
//!
//! STATUS: I1 IMPLEMENTED. Working single-node reference Kernel:
//! `RefKernel`/`MemKernel`/`FileKernel` with idempotent content-addressed commit,
//! fsync-durable WAL, deterministic replay/recovery, snapshot install and
//! checkpointing (see `tests/` — walking_skeleton, persistent_wal, recovery,
//! checkpoint). The frozen specification governs; this crate implements, never
//! changes it.
//!
//! # Role in the frozen architecture
//!
//! The Kernel is the one component in the ARVES runtime that *owns truth*.
//! Every other component is either a producer of *proposed* writes (the
//! Information Platform, the Execution layer routing outcomes) or a consumer of
//! *committed* truth (the Query layer, read-only). Nothing becomes truth until
//! it passes through [`Kernel::commit`].
//!
//! ## Why there is exactly one write path
//!
//! - **ORCH-001** - the Control Plane owns *no* truth; only the Kernel owns
//!   truth. Orchestrators decide *what to attempt*, but the decision only
//!   becomes fact by being committed here.
//! - **OWN-001** - one owner per state. The Kernel is that single owner for
//!   cognitive truth; there is no second door through which state may mutate.
//! - **G-001 (proposed, informative, pending CCP)** - the Kernel is the sole
//!   truth owner and the sole commit gateway. This crate is the Rust surface of
//!   that proposition.
//!
//! ## What this trait deliberately does NOT expose
//!
//! There are **no read methods** on [`Kernel`]. Reads are the exclusive concern
//! of the Query layer (QUERY-001, proposed: query is read-only), which projects
//! over Kernel/LCW/Persistence at the appropriate read tier
//! (linearizable / bounded-staleness / eventual). Exposing a read here would
//! blur the single-writer contract and invite callers to treat the Kernel as a
//! store. It is a *gateway*, not a database.
//!
//! ## Commit semantics (context, not implemented here)
//!
//! A committed write is expected (per IDR-001..005) to be replicated as an
//! *outcome* through a per-shard Raft group whose log doubles as the WAL and the
//! decision trace (ORCH-003 replay reads that trace; it never recomputes).
//! Commits are content-addressable and idempotent (ORCH-004): re-committing an
//! identical proposal must resolve to the same [`TruthRef`] rather than forking
//! truth. Shard placement is by immutable tenant/workspace key (SHARD-001) and
//! there is no cross-shard atomic commit - multi-shard effects are sagas, not
//! single commits. None of that machinery lives in this crate; it is the
//! contract that implementors (arves-consensus, arves-persistence) must honour.

#![forbid(unsafe_code)]

use core::fmt;

/// Immutable partition key locating the shard that owns a piece of truth.
///
/// Governing: SHARD-001 (partition by tenant/workspace; key immutable),
/// IDR-001 (one Raft group per tenant/workspace).
///
/// The Kernel is *per-shard*: a given [`ProposedWrite`] is committed by the
/// leader of the shard identified by this key. The key is treated as opaque and
/// immutable - once assigned to a piece of state it never changes, which is what
/// lets shard placement be stable and lets replay be deterministic.
///
/// **Opaque since RCR-017 (SHARD-001-F2):** the fields are private, so a key is
/// immutable *by type* — no caller outside this crate can mutate a part in place
/// or construct a degenerate key. Construction goes through [`ShardKey::new`]
/// (each part non-empty and at most [`ShardKey::MAX_PART_BYTES`] bytes); reads go
/// through [`ShardKey::tenant`] / [`ShardKey::workspace`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShardKey {
    /// Tenant identifier (outermost tenancy boundary).
    tenant: String,
    /// Workspace identifier within the tenant.
    workspace: String,
}

/// Why a [`ShardKey`] could not be constructed (RCR-017): the degenerate keys the
/// opaque type makes unrepresentable.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ShardKeyError {
    /// The tenant part was empty.
    EmptyTenant,
    /// The workspace part was empty.
    EmptyWorkspace,
    /// The tenant part exceeded [`ShardKey::MAX_PART_BYTES`] bytes (carries the length).
    TenantTooLong(usize),
    /// The workspace part exceeded [`ShardKey::MAX_PART_BYTES`] bytes (carries the length).
    WorkspaceTooLong(usize),
}

impl fmt::Display for ShardKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShardKeyError::EmptyTenant => write!(f, "shard tenant must be non-empty"),
            ShardKeyError::EmptyWorkspace => write!(f, "shard workspace must be non-empty"),
            ShardKeyError::TenantTooLong(n) => {
                write!(f, "shard tenant is {n} bytes (max {})", ShardKey::MAX_PART_BYTES)
            }
            ShardKeyError::WorkspaceTooLong(n) => {
                write!(f, "shard workspace is {n} bytes (max {})", ShardKey::MAX_PART_BYTES)
            }
        }
    }
}

impl std::error::Error for ShardKeyError {}

impl ShardKey {
    /// Upper bound on each part's byte length (tenant, workspace). Generous for any
    /// real tenancy scheme while keeping keys bounded for logs, file names and wire
    /// tokens (the bridge's own per-token cap of 64 bytes is stricter and unaffected).
    pub const MAX_PART_BYTES: usize = 256;

    /// The ONLY public constructor (RCR-017): rejects the degenerate keys —
    /// an empty part, or a part longer than [`ShardKey::MAX_PART_BYTES`] bytes —
    /// so they are unrepresentable rather than merely discouraged (SHARD-001:
    /// the key is immutable and well-formed by construction).
    pub fn new(
        tenant: impl Into<String>,
        workspace: impl Into<String>,
    ) -> Result<Self, ShardKeyError> {
        let (tenant, workspace) = (tenant.into(), workspace.into());
        if tenant.is_empty() {
            return Err(ShardKeyError::EmptyTenant);
        }
        if workspace.is_empty() {
            return Err(ShardKeyError::EmptyWorkspace);
        }
        if tenant.len() > Self::MAX_PART_BYTES {
            return Err(ShardKeyError::TenantTooLong(tenant.len()));
        }
        if workspace.len() > Self::MAX_PART_BYTES {
            return Err(ShardKeyError::WorkspaceTooLong(workspace.len()));
        }
        Ok(ShardKey { tenant, workspace })
    }

    /// Tenant identifier (outermost tenancy boundary) — read-only accessor.
    pub fn tenant(&self) -> &str {
        &self.tenant
    }

    /// Workspace identifier within the tenant — read-only accessor.
    pub fn workspace(&self) -> &str {
        &self.workspace
    }
}

/// Content address of a payload: the identity used to make commits
/// idempotent and content-addressable.
///
/// Governing: ORCH-004 (every invocation idempotent + content-addressable).
///
/// Two proposals bearing the same [`ContentHash`] denote the same intended
/// truth; committing the second is a no-op that resolves to the same
/// [`TruthRef`] as the first. The byte layout of the hash is intentionally
/// unspecified at the skeleton stage.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ContentHash(pub Vec<u8>);

/// A monotonically increasing position in a shard's committed log.
///
/// Governing: IDR-004/IDR-005 (Raft log = WAL = decision trace; append-only).
///
/// Because the Raft log *is* the write-ahead log and the decision trace, a
/// commit index uniquely orders committed outcomes within a shard and is the
/// anchor ORCH-003 replay walks. It is *not* a wall-clock time and carries no
/// cross-shard meaning.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CommitIndex(pub u64);

/// A candidate mutation *offered* to the Kernel. It is NOT truth yet.
///
/// Governing: ORCH-001 (producers own no truth), OWN-001, G-001 (proposed).
///
/// Proposed writes originate outside the Kernel - the Information Platform
/// canonicalizes inbound data into proposals, and the Execution layer routes
/// action outcomes as proposals. The Control Plane may have *decided* to attempt
/// this write, but under ORCH-001 that decision is not truth: only
/// [`Kernel::commit`] can promote a `ProposedWrite` to a [`TruthRef`].
///
/// The payload shape is intentionally left opaque (a byte vector plus its
/// content hash) at the skeleton stage; richer typing arrives once the ontology
/// (`arves-ontology`) surface is wired in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProposedWrite {
    /// The shard whose leader is authoritative for this write (SHARD-001).
    pub shard: ShardKey,
    /// Content address of the payload; drives idempotency (ORCH-004).
    pub content: ContentHash,
    /// Opaque canonicalized payload bytes (typed later via arves-ontology).
    pub payload: Vec<u8>,
}

/// A durable, content-addressed handle to a piece of *committed* truth.
///
/// Governing: OWN-001 (single owner), ORCH-004 (content-addressable),
/// IDR-005 (append-only WAL position).
///
/// A `TruthRef` is what the caller receives once - and only once - a proposal
/// has been accepted and replicated as an outcome. It names *what* was committed
/// (`content`), *where* (`shard`), and *at which* position in the shard's
/// append-only log (`index`). Query-layer reads dereference truth by such
/// references; the Kernel itself never reads on the caller's behalf.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct TruthRef {
    /// The shard that owns this truth (SHARD-001).
    pub shard: ShardKey,
    /// Content address of the committed payload (ORCH-004).
    pub content: ContentHash,
    /// Position in the shard's committed log (IDR-004/IDR-005).
    pub index: CommitIndex,
}

/// Why a [`Kernel::commit`] did not produce new truth.
///
/// Governing: ORCH-001, OWN-001, ORCH-004, SHARD-001; IDR-001..005.
///
/// Note that [`CommitError::AlreadyCommitted`] is a *reconciliation* signal, not
/// a failure of correctness: under ORCH-004 an identical re-proposal must map
/// back to the truth that already exists rather than fork it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CommitError {
    /// This node is not the current leader for the target shard, so it may not
    /// commit. Only the shard leader commits (IDR-002/IDR-003). The caller
    /// should retry against the leader.
    NotLeader {
        /// The shard whose leadership was required.
        shard: ShardKey,
    },
    /// An identical proposal (same [`ContentHash`]) was already committed;
    /// commit is idempotent (ORCH-004). The prior truth is returned so callers
    /// can proceed without forking truth (OWN-001).
    AlreadyCommitted(TruthRef),
    /// The proposal targets a shard this Kernel does not own or cannot route to
    /// (SHARD-001). There is no cross-shard atomic commit; such intent must be
    /// expressed as a saga, not a single commit.
    UnknownShard {
        /// The shard key that could not be resolved.
        shard: ShardKey,
    },
    /// The proposal was structurally or semantically rejected before it could
    /// become truth (e.g. failed canonicalization or invariant check upstream).
    Rejected {
        /// Human-readable reason; not a stable API surface.
        reason: String,
    },
    /// **Content-integrity violation (RCR-005):** the proposal's [`ContentHash`] is
    /// already bound to a *different* payload. A content address MUST denote exactly
    /// one content (ORCH-004 idempotency is only sound when *address ⇒ content*;
    /// OWN-001, one owner per state). The Kernel rejects this fork rather than
    /// silently returning the prior truth for a mismatched re-proposal — closing the
    /// "same address, different content" hole at the sole commit gateway. (This is the
    /// Kernel-owned half of address integrity; recomputing the ACS-001 multihash from
    /// the payload pre-image is deliberately NOT done here — see RCR-005 §"layering".)
    ContentIntegrity {
        /// The shard the mismatched proposal targeted (SHARD-001).
        shard: ShardKey,
    },
    /// Replication of the committed outcome did not reach quorum, so no truth
    /// was durably established (IDR-001, truth is CP under CAP). The write may be
    /// retried; it must remain idempotent (ORCH-004).
    NotReplicated,
}

impl fmt::Display for CommitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitError::NotLeader { shard } => {
                write!(f, "not leader for shard {}/{}", shard.tenant, shard.workspace)
            }
            CommitError::AlreadyCommitted(_) => {
                write!(f, "proposal already committed (idempotent no-op)")
            }
            CommitError::UnknownShard { shard } => {
                write!(f, "unknown shard {}/{}", shard.tenant, shard.workspace)
            }
            CommitError::Rejected { reason } => write!(f, "proposal rejected: {reason}"),
            CommitError::ContentIntegrity { shard } => write!(
                f,
                "content-integrity violation on shard {}/{}: the content address is already \
                 bound to a different payload",
                shard.tenant, shard.workspace
            ),
            CommitError::NotReplicated => write!(f, "commit did not reach quorum"),
        }
    }
}

impl std::error::Error for CommitError {}

/// The SOLE commit gateway for cognitive truth in the ARVES runtime.
///
/// Governing: **ORCH-001** (Control Plane owns no truth; only the Kernel owns
/// truth), **OWN-001** (one owner per state), **G-001 (proposed)** (Kernel is
/// the sole truth owner and commit gateway). Contextually: LAYER-001
/// (downward-only layering) places the Kernel above Persistence and below the
/// Information Platform; IDR-001..005 constrain *how* a commit is replicated.
///
/// # Contract
///
/// - [`commit`](Kernel::commit) is the **only** way state becomes truth. If a
///   mutation did not pass through this method, it is not truth (OWN-001).
/// - There are intentionally **no read methods**. Reads belong to the Query
///   layer (QUERY-001, proposed: read-only). The Kernel is a gateway, not a
///   store; adding a getter here would violate the single-responsibility split
///   that ORCH-001 and OWN-001 encode.
/// - `commit` is expected to be **idempotent and content-addressable**
///   (ORCH-004): committing an identical [`ProposedWrite`] twice yields the same
///   [`TruthRef`] (surfaced as [`CommitError::AlreadyCommitted`]), never a fork.
/// - A commit is authoritative only on the **shard leader** (IDR-002/IDR-003);
///   non-leaders return [`CommitError::NotLeader`]. There is **no cross-shard
///   atomic commit** (IDR-004) - multi-shard intent is a saga.
///
/// # Non-goals (skeleton)
///
/// No method bodies, replication, or storage live here. This trait is the
/// contract that arves-consensus (Raft replication), arves-persistence (WAL /
/// snapshots) and arves-runtime (wiring) must satisfy.
pub trait Kernel {
    /// Offer a [`ProposedWrite`] to become truth.
    ///
    /// Returns a [`TruthRef`] naming the newly (or already, per ORCH-004)
    /// committed truth, or a [`CommitError`] explaining why nothing was
    /// committed.
    ///
    /// Governing: ORCH-001, OWN-001, ORCH-004, G-001 (proposed).
    ///
    /// This is the entire write surface of the Kernel. It takes the proposal by
    /// value because a proposal is consumed by the act of being committed - it
    /// either becomes referenced truth or is rejected, and either way the caller
    /// should reason in terms of the returned [`TruthRef`], not the original
    /// proposal.
    fn commit(&self, proposed: ProposedWrite) -> Result<TruthRef, CommitError>;
}


// =============================================================================
// I1.4/I1.5 reference Kernel: concrete commit gateway + deterministic replay.
//
// Single process / node. NO Raft, networking, replication, scheduler, engine
// graph, or API. Proves the first executable behaviour:
//   ProposedWrite -> commit() -> WAL.append() -> durable truth -> TruthRef
//   -> replay() -> same truth.
// The concrete Kernel is generic over the durable substrate (`RefKernel<S>`), so
// the SAME logic runs over the in-memory WAL (`MemKernel`, I1.4) and the
// fsync-durable on-disk WAL (`FileKernel`, I1.5). The Kernel stays the SOLE
// commit gateway (ORCH-001/OWN-001); reads are not on the trait.
// `truth_hash`/`committed_count` are introspection helpers for the behaviour
// proofs, NOT the Query layer (milestone I3).
// =============================================================================

use std::collections::HashMap;
use std::sync::Mutex;

use arves_persistence::{
    ContentId, FileWalStore, MemWalStore, PendingRecord, RecordKind, ReplayCursor,
    ShardKey as PShardKey, Wal, WalError, WalStore,
};

fn to_pshard(s: &ShardKey) -> PShardKey {
    PShardKey {
        tenant: s.tenant.clone(),
        workspace: s.workspace.clone(),
    }
}
/// Crate-internal reverse mapping (recovery/replay path). Constructs directly —
/// lawful only inside this crate (RCR-017 keeps the fields private): a persistence
/// key replayed from the WAL was originally written under a constructor-validated
/// [`ShardKey`], so re-validating here would only re-check our own invariant.
fn from_pshard(s: &PShardKey) -> ShardKey {
    ShardKey {
        tenant: s.tenant.clone(),
        workspace: s.workspace.clone(),
    }
}

/// Deterministic, dependency-free FNV-1a-64 fold. Proves the committed truth
/// set is bit-identical before and after replay (ORCH-003) without relying on
/// any hasher whose seed could vary across runs.
fn fnv1a_64(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// -- checkpoint state blob codec (I1.6) ---------------------------------------
//
// The Kernel owns truth (ORCH-001), so the Kernel serializes a shard's
// materialized truths into an opaque blob that Persistence stores verbatim. The
// blob is deterministic (fixed-order little-endian) so a snapshot + tail replay
// reproduces the same truth set as a from-zero replay.

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

/// Serialize a shard's truths `(offset, content, payload)` in offset order.
fn encode_shard_blob(entries: &[(u64, Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut b = Vec::new();
    put_u32(&mut b, entries.len() as u32);
    for (offset, content, payload) in entries {
        put_u64(&mut b, *offset);
        put_bytes(&mut b, content);
        put_bytes(&mut b, payload);
    }
    b
}

/// Decode a shard state blob. `None` on any structural mismatch.
fn decode_shard_blob(b: &[u8]) -> Option<Vec<(u64, Vec<u8>, Vec<u8>)>> {
    let mut pos = 0usize;
    fn take<'a>(b: &'a [u8], pos: &mut usize, n: usize) -> Option<&'a [u8]> {
        let end = pos.checked_add(n)?;
        let s = b.get(*pos..end)?;
        *pos = end;
        Some(s)
    }
    let count = u32::from_le_bytes(take(b, &mut pos, 4)?.try_into().ok()?) as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = u64::from_le_bytes(take(b, &mut pos, 8)?.try_into().ok()?);
        let clen = u32::from_le_bytes(take(b, &mut pos, 4)?.try_into().ok()?) as usize;
        let content = take(b, &mut pos, clen)?.to_vec();
        let plen = u32::from_le_bytes(take(b, &mut pos, 4)?.try_into().ok()?) as usize;
        let payload = take(b, &mut pos, plen)?.to_vec();
        out.push((offset, content, payload));
    }
    if pos != b.len() {
        return None; // trailing garbage
    }
    Some(out)
}

struct KernelState {
    committed: Vec<(TruthRef, Vec<u8>)>,
    index: HashMap<(String, String, Vec<u8>), usize>,
}

/// Why recovery could not faithfully reconstruct committed truth (I1.7).
///
/// Recovery is contractually **lossless or loud**: it either restores every
/// committed truth or fails with one of these, never silently returns a
/// partially-recovered Kernel. In a future replicated runtime (I2) a node that
/// hits one of these repairs from a peer; a single-node caller surfaces it and
/// refuses to start.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RecoveryError {
    /// A prefix `[0..earliest)` was compacted away but no valid snapshot loads,
    /// so that committed truth is unrecoverable on this node (defect A).
    CompactedPrefixWithoutSnapshot {
        /// The shard whose compacted prefix is unrecoverable.
        shard: ShardKey,
        /// First still-retained offset; everything below it is lost.
        earliest: u64,
    },
    /// The retained log is not dense/complete: a committed record is missing
    /// (corrupt/interior segment), so the trace has a gap (defect B).
    Corruption {
        /// The shard whose log is incomplete.
        shard: ShardKey,
        /// The first offset expected but not found.
        missing_offset: u64,
        /// Human-readable detail; not a stable API surface.
        detail: String,
    },
    /// Any other durable-store failure encountered during recovery.
    Wal {
        /// The shard being recovered when the error occurred.
        shard: ShardKey,
        /// Human-readable detail; not a stable API surface.
        detail: String,
    },
}

impl RecoveryError {
    /// Map a [`WalError`] raised during recovery to a [`RecoveryError`],
    /// preserving a detected gap as [`RecoveryError::Corruption`].
    fn from_wal(shard: &ShardKey, e: WalError) -> Self {
        match e {
            WalError::Corruption {
                missing_offset,
                detail,
                ..
            } => RecoveryError::Corruption {
                shard: shard.clone(),
                missing_offset,
                detail,
            },
            other => RecoveryError::Wal {
                shard: shard.clone(),
                detail: format!("{other:?}"),
            },
        }
    }
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryError::CompactedPrefixWithoutSnapshot { shard, earliest } => write!(
                f,
                "compacted prefix [0..{}) for shard {}/{} has no recoverable snapshot",
                earliest, shard.tenant, shard.workspace
            ),
            RecoveryError::Corruption {
                shard,
                missing_offset,
                detail,
            } => write!(
                f,
                "corrupt/incomplete log for shard {}/{} at offset {}: {}",
                shard.tenant, shard.workspace, missing_offset, detail
            ),
            RecoveryError::Wal { shard, detail } => write!(
                f,
                "recovery failure for shard {}/{}: {}",
                shard.tenant, shard.workspace, detail
            ),
        }
    }
}

impl std::error::Error for RecoveryError {}

/// The reference Kernel: the sole commit gateway. Owns truth in memory, backed
/// by an append-only [`WalStore`] it replays on recovery. Generic over the
/// durable substrate so identical logic runs over the in-memory WAL
/// ([`MemKernel`], I1.4) and the fsync-durable on-disk WAL ([`FileKernel`],
/// I1.5). Implements [`Kernel`].
pub struct RefKernel<S: WalStore> {
    store: S,
    state: Mutex<KernelState>,
}

/// In-memory reference Kernel (I1.4): truth cached in memory over an in-memory
/// WAL. Alias of [`RefKernel`] over [`MemWalStore`]. "Restart" = drop the Kernel
/// while an `Arc`-shared log survives (durability is simulated, not on disk).
pub type MemKernel = RefKernel<MemWalStore>;

/// File-backed reference Kernel (I1.5): identical logic over a fsync-durable,
/// crash-consistent on-disk WAL. Truth survives a real process exit and is
/// recovered by a fresh process from the directory alone (ORCH-003 replay).
pub type FileKernel = RefKernel<FileWalStore>;

impl<S: WalStore> RefKernel<S> {
    /// Create an empty Kernel over a durable store (no replay).
    pub fn new(store: S) -> Self {
        RefKernel {
            store,
            state: Mutex::new(KernelState {
                committed: Vec::new(),
                index: HashMap::new(),
            }),
        }
    }

    /// Recover a Kernel by REPLAYING the durable WAL (ORCH-003). Convenience
    /// wrapper over [`try_recover`](RefKernel::try_recover) that PANICS loudly on
    /// an unrecoverable durable state -- it never returns a partially-recovered
    /// Kernel ("lossless or loud").
    pub fn recover(store: S) -> Self {
        Self::try_recover(store)
            .unwrap_or_else(|e| panic!("unrecoverable durable state: {e}"))
    }

    /// Fallible recovery: reconstruct the truth set, or return a [`RecoveryError`]
    /// if the durable state cannot be faithfully restored (I1.7). A future
    /// replicated node (I2) will repair from a peer on such an error; a
    /// single-node caller surfaces it and refuses to start on partial truth.
    pub fn try_recover(store: S) -> Result<Self, RecoveryError> {
        let k = RefKernel::new(store);
        k.try_replay()?;
        Ok(k)
    }

    /// Recover the truth set for every shard (panicking wrapper over
    /// [`try_replay`](RefKernel::try_replay); kept for the idempotent-replay
    /// behaviour proofs).
    pub fn replay(&self) {
        self.try_replay().unwrap_or_else(|e| panic!("replay failed: {e}"))
    }

    /// Recover every shard idempotently via **checkpoint + tail replay** (I1.6),
    /// hardened to be LOSSLESS OR LOUD (I1.7):
    /// - if a prefix was compacted (`earliest > 0`) but no valid snapshot loads,
    ///   the compacted truth is unrecoverable -> fail loudly (defect A);
    /// - if the snapshot covers beyond the recovered head (a corrupt segment
    ///   truncated the tail below the snapshot), the snapshot IS the full truth;
    ///   skip the empty tail instead of panicking (defect C);
    /// - `replay_from` rejects a gapped/incomplete tail, surfacing here as
    ///   [`RecoveryError::Corruption`] (defect B).
    ///
    /// A record whose content is already present is skipped (ORCH-003 idempotent).
    pub fn try_replay(&self) -> Result<(), RecoveryError> {
        let shards = self.store.shards();
        for sh in shards {
            let kernel_shard = from_pshard(&sh);
            let wal = self
                .store
                .open(&sh)
                .map_err(|e| RecoveryError::from_wal(&kernel_shard, e))?;
            // load_snapshot() -> install_state() -> replay(tail).
            let base = match wal
                .load_snapshot()
                .map_err(|e| RecoveryError::from_wal(&kernel_shard, e))?
            {
                Some((meta, blob)) => {
                    self.install_state(&kernel_shard, &blob);
                    meta.up_to_offset + 1
                }
                None => {
                    let earliest = wal.earliest();
                    if earliest > 0 {
                        // Defect A: a compacted prefix with no recoverable snapshot
                        // means truths [0..earliest) are gone. Refuse loudly rather
                        // than silently return partial truth.
                        return Err(RecoveryError::CompactedPrefixWithoutSnapshot {
                            shard: kernel_shard,
                            earliest,
                        });
                    }
                    0
                }
            };
            let head = wal.head();
            if base > head {
                // Defect C: the snapshot already covers every committed offset;
                // there is no tail (a corrupt segment may have truncated head below
                // the snapshot). The installed snapshot state stands. Lossless.
                continue;
            }
            let mut cur = wal
                .replay_from(base)
                .map_err(|e| RecoveryError::from_wal(&kernel_shard, e))?;
            let mut state = self.state.lock().expect("state poisoned");
            while let Some(rec) = cur
                .next()
                .map_err(|e| RecoveryError::from_wal(&kernel_shard, e))?
            {
                let key = (
                    rec.shard.tenant.clone(),
                    rec.shard.workspace.clone(),
                    rec.content.0.clone(),
                );
                if state.index.contains_key(&key) {
                    continue;
                }
                let tr = TruthRef {
                    shard: from_pshard(&rec.shard),
                    content: ContentHash(rec.content.0.clone()),
                    index: CommitIndex(rec.offset),
                };
                let pos = state.committed.len();
                state.committed.push((tr, rec.payload.clone()));
                state.index.insert(key, pos);
            }
        }
        Ok(())
    }

    /// Serialize a shard's materialized truths into an opaque state blob, in
    /// offset order. The Kernel owns truth (ORCH-001), so snapshot production is
    /// exclusively the Kernel's job; Persistence stores the bytes verbatim.
    pub fn snapshot_shard(&self, shard: &ShardKey) -> Vec<u8> {
        let state = self.state.lock().expect("state poisoned");
        let mut entries: Vec<(u64, Vec<u8>, Vec<u8>)> = state
            .committed
            .iter()
            .filter(|(tr, _)| tr.shard == *shard)
            .map(|(tr, payload)| (tr.index.0, tr.content.0.clone(), payload.clone()))
            .collect();
        entries.sort_by_key(|(offset, _, _)| *offset);
        encode_shard_blob(&entries)
    }

    /// Install a shard state blob into the truth set (restore path). Idempotent:
    /// a truth whose content is already present is skipped.
    pub fn install_state(&self, shard: &ShardKey, blob: &[u8]) {
        let entries = decode_shard_blob(blob).unwrap_or_default();
        let mut state = self.state.lock().expect("state poisoned");
        for (offset, content, payload) in entries {
            let key = (shard.tenant.clone(), shard.workspace.clone(), content.clone());
            if state.index.contains_key(&key) {
                continue;
            }
            let tr = TruthRef {
                shard: shard.clone(),
                content: ContentHash(content),
                index: CommitIndex(offset),
            };
            let pos = state.committed.len();
            state.committed.push((tr, payload));
            state.index.insert(key, pos);
        }
    }

    /// Take a durable checkpoint for every shard: produce the state blob (Kernel
    /// owns truth), `install_snapshot` it (Persistence stores opaque bytes), then
    /// `compact` the WAL prefix it covers. Returns the number of shards
    /// checkpointed. The snapshot is durable BEFORE compaction (no truth loss).
    pub fn checkpoint(&self) -> Result<usize, String> {
        let mut n = 0;
        for psh in self.store.shards() {
            let mut wal = self.store.open(&psh).map_err(|e| format!("open: {e:?}"))?;
            let head = wal.head();
            if head == 0 {
                continue;
            }
            let kernel_shard = from_pshard(&psh);
            let blob = self.snapshot_shard(&kernel_shard);
            wal.install_snapshot(head - 1, 0, &blob)
                .map_err(|e| format!("install_snapshot: {e:?}"))?;
            wal.compact(head - 1).map_err(|e| format!("compact: {e:?}"))?;
            n += 1;
        }
        Ok(n)
    }

    /// Introspection (NOT the Query layer): number of committed truths.
    pub fn committed_count(&self) -> usize {
        self.state.lock().expect("state poisoned").committed.len()
    }

    /// Introspection (NOT the Query layer): deterministic hash of the committed
    /// truth set in commit order. Equal before and after replay iff identical.
    pub fn truth_hash(&self) -> u64 {
        let state = self.state.lock().expect("state poisoned");
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for (tr, payload) in &state.committed {
            h = fnv1a_64(h, tr.shard.tenant.as_bytes());
            h = fnv1a_64(h, tr.shard.workspace.as_bytes());
            h = fnv1a_64(h, &tr.content.0);
            h = fnv1a_64(h, &tr.index.0.to_le_bytes());
            h = fnv1a_64(h, payload);
        }
        h
    }
}

impl<S: WalStore> Kernel for RefKernel<S> {
    fn commit(&self, proposed: ProposedWrite) -> Result<TruthRef, CommitError> {
        let mut state = self.state.lock().expect("state poisoned");
        Self::commit_inner(&mut state, &self.store, proposed)
    }
}

impl<S: WalStore> RefKernel<S> {
    /// The single-proposal commit body, factored so [`Kernel::commit`] and
    /// [`RefKernel::commit_batch`] (RCR-013) run the IDENTICAL gateway logic under
    /// one caller-held state lock — batch is not a second, divergent commit path.
    fn commit_inner(
        state: &mut KernelState,
        store: &S,
        proposed: ProposedWrite,
    ) -> Result<TruthRef, CommitError> {
        let key = (
            proposed.shard.tenant.clone(),
            proposed.shard.workspace.clone(),
            proposed.content.0.clone(),
        );
        // ORCH-004: an identical re-proposal resolves to existing truth, never a fork.
        if let Some(&pos) = state.index.get(&key) {
            let (existing_tr, existing_payload) = &state.committed[pos];
            // RCR-005 content-integrity: the address MUST bind exactly one content. A
            // re-proposal under the same ContentHash but with a DIFFERENT payload is a
            // caller-supplied address that does not match its content — reject the fork
            // instead of silently returning the prior truth (ORCH-004 is only sound when
            // address ⇒ content; OWN-001). This is enforced with the Kernel's own state,
            // needing no ACS-001 coupling (see RCR-005).
            if *existing_payload != proposed.payload {
                return Err(CommitError::ContentIntegrity {
                    shard: proposed.shard.clone(),
                });
            }
            return Err(CommitError::AlreadyCommitted(existing_tr.clone()));
        }
        let pshard = to_pshard(&proposed.shard);
        let mut wal = store.open(&pshard).map_err(|e| CommitError::Rejected {
            reason: format!("wal open: {e:?}"),
        })?;
        let offset = wal
            .append(PendingRecord {
                shard: pshard,
                term: 0,
                kind: RecordKind::Outcome,
                content: ContentId(proposed.content.0.clone()),
                payload: proposed.payload.clone(),
            })
            .map_err(|e| CommitError::Rejected {
                reason: format!("wal append: {e:?}"),
            })?;
        let tr = TruthRef {
            shard: proposed.shard.clone(),
            content: proposed.content.clone(),
            index: CommitIndex(offset),
        };
        let pos = state.committed.len();
        state.committed.push((tr.clone(), proposed.payload));
        state.index.insert(key, pos);
        Ok(tr)
    }
}

// =============================================================================
// RCR-013 — same-shard atomic batch commit (v1.1 backlog item 3).
// =============================================================================

/// One outcome inside a successful [`RefKernel::commit_batch`]: the truth and whether
/// it was newly committed (`fresh`) or resolved to already-existing truth (ORCH-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchOutcome {
    /// The (new or pre-existing) committed truth.
    pub truth: TruthRef,
    /// `true` iff this batch entry created the truth; `false` = idempotent resolve.
    pub fresh: bool,
}

/// Why a [`RefKernel::commit_batch`] did not commit the whole batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchError {
    /// The batch mixes shards. There is **no cross-shard atomic commit** (IDR-004:
    /// multi-shard intent is a saga, never a single commit) — refused up front,
    /// nothing applied.
    CrossShard {
        /// The batch's shard (from its first proposal).
        expected: ShardKey,
        /// The differing shard found at `index`.
        found: ShardKey,
        /// Index of the offending proposal.
        index: usize,
    },
    /// Validation refused the batch — **nothing was applied** (all-or-nothing over
    /// the whole validation class: a content-integrity fork against committed truth,
    /// or two batch entries binding the same address to different payloads).
    Refused {
        /// Index of the offending proposal.
        index: usize,
        /// The underlying refusal.
        cause: CommitError,
    },
    /// A WAL append failed mid-apply (host-level I/O). The `applied` prefix IS
    /// durable truth (each entry passed the full gateway); the remainder was not
    /// attempted. Surfaced loudly — the batch does not pretend to be a WAL
    /// transaction (see the honesty note on [`RefKernel::commit_batch`]).
    PartialApply {
        /// Outcomes for the prefix that committed before the failure.
        applied: Vec<BatchOutcome>,
        /// Index of the proposal whose append failed.
        index: usize,
        /// The underlying failure.
        cause: CommitError,
    },
}

impl<S: WalStore> RefKernel<S> {
    /// Commit several proposals to ONE shard as a batch — **all-or-nothing across
    /// the validation class** (RCR-013, v1.1 backlog item 3).
    ///
    /// Under a single state lock (no interleaving with other commits):
    ///
    /// 1. **Validate (no mutation):** every proposal must target the batch's shard
    ///    (cross-shard intent is a saga per IDR-004 — [`BatchError::CrossShard`]);
    ///    no proposal may fork committed truth (RCR-005 content-integrity) or bind
    ///    the same address to two different payloads *within* the batch
    ///    ([`BatchError::Refused`], nothing applied).
    /// 2. **Apply:** each proposal runs the IDENTICAL single-commit gateway logic
    ///    (`commit_inner`). An identical duplicate (pre-committed or intra-batch)
    ///    resolves idempotently to `fresh: false` (ORCH-004), never a fork.
    ///
    /// **Honest atomicity boundary:** the all-or-nothing guarantee covers the entire
    /// validation class — the failure modes a caller can *cause*. A mid-apply WAL
    /// **I/O** failure (host-level) leaves the already-appended prefix as durable
    /// truth and is surfaced loudly as [`BatchError::PartialApply`]; the reference
    /// WAL has no multi-record transaction primitive to roll a prefix back. Making
    /// the apply phase itself transactional is consensus-era work (I2: a Raft log
    /// entry naturally carries a batch) — recorded in `runtime/rcr/RCR-013.md`.
    pub fn commit_batch(&self, proposals: Vec<ProposedWrite>) -> Result<Vec<BatchOutcome>, BatchError> {
        if proposals.is_empty() {
            return Ok(Vec::new());
        }
        let mut state = self.state.lock().expect("state poisoned");

        // ---- Phase 1: validate everything before touching anything. ----
        let shard = proposals[0].shard.clone();
        let mut in_batch: HashMap<&[u8], &[u8]> = HashMap::new();
        for (i, p) in proposals.iter().enumerate() {
            if p.shard != shard {
                return Err(BatchError::CrossShard { expected: shard, found: p.shard.clone(), index: i });
            }
            // Intra-batch fork: same address, different payload inside ONE batch.
            match in_batch.get(p.content.0.as_slice()) {
                Some(prev) if *prev != p.payload.as_slice() => {
                    return Err(BatchError::Refused {
                        index: i,
                        cause: CommitError::ContentIntegrity { shard: p.shard.clone() },
                    });
                }
                _ => {
                    in_batch.insert(p.content.0.as_slice(), p.payload.as_slice());
                }
            }
            // Fork against already-committed truth (RCR-005) — refuse the WHOLE batch.
            let key = (p.shard.tenant.clone(), p.shard.workspace.clone(), p.content.0.clone());
            if let Some(&pos) = state.index.get(&key) {
                if state.committed[pos].1 != p.payload {
                    return Err(BatchError::Refused {
                        index: i,
                        cause: CommitError::ContentIntegrity { shard: p.shard.clone() },
                    });
                }
            }
        }

        // ---- Phase 2: apply through the identical single-commit gateway. ----
        let mut out = Vec::with_capacity(proposals.len());
        for (i, p) in proposals.into_iter().enumerate() {
            match Self::commit_inner(&mut state, &self.store, p) {
                Ok(truth) => out.push(BatchOutcome { truth, fresh: true }),
                Err(CommitError::AlreadyCommitted(truth)) => out.push(BatchOutcome { truth, fresh: false }),
                Err(cause) => return Err(BatchError::PartialApply { applied: out, index: i, cause }),
            }
        }
        Ok(out)
    }
}

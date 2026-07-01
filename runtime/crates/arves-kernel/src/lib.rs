//! ARVES :: arves-kernel
//!
//! Purpose: Owner of cognitive TRUTH and the SOLE commit gateway.
//! Governing: ORCH-001, OWN-001; G-001 (proposed). Vol 9 v2; Amendments.
//! Layer: Data Plane (LAYER-001 position: Kernel, below Information Platform,
//!        above Persistence).
//!
//! STATUS: I1 skeleton - interfaces/contracts only, NO implementation yet.
//! Frozen specification governs; this crate implements, never changes it.
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
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShardKey {
    /// Tenant identifier (outermost tenancy boundary).
    pub tenant: String,
    /// Workspace identifier within the tenant.
    pub workspace: String,
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

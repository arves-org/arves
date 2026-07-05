//! ARVES :: arves-consensus
//!
//! Purpose: Per-shard Raft: CP truth replication (leader/followers, joint consensus).
//! Governing: IDR-001..005; SHARD-001 (partition by tenant/workspace).
//! Layer: cross-cutting (truth path; serves the Kernel's commit gateway).
//!
//! STATUS: CONTRACT (frozen v1.0) + Stage-1 deterministic Raft CORE (RCR-019)
//! + Stage-2 multi-shard map, JOINT-CONSENSUS membership and leadership
//! transfer (RCR-020), per `docs/design/I2_Cluster_Kernel_Design.md`. The
//! v1.0 wording "trait/type declarations only, no logic" is superseded by
//! RCR-019/020: the frozen contract surface in this file is UNCHANGED (no
//! type or trait signature touched), and the implementation lives additively
//! BEHIND it — [`raft`] (deterministic, in-process, message-passing Raft
//! state machine for one shard: seeded election timeouts via injected tick,
//! log replication, quorum commit, follower catch-up; since RCR-020 also
//! IDR-003 joint-consensus reconfiguration with the C_old,new dual-majority
//! rule, leadership transfer, and the thesis-§4.2.3 leadership check) and
//! [`sim`] (the deterministic MessageBus/step-function harness, the first
//! [`ShardConsensus`] impl, and since RCR-020 `SimShardMap` — the per-shard
//! consensus instance map: one independent Raft group per immutable
//! [`ShardId`], IDR-001). HONEST SCOPE: in-process simulation only — no
//! network transport, no durability wiring (WAL-as-Raft-log is a later I2
//! stage), no real read tiers (I2.9), no snapshots (OQ-1), no learner
//! catch-up/promotion protocol. Frozen specification governs; this crate
//! implements, never changes it.
//!
//! # What this crate is (and is not)
//!
//! This crate defines the *contracts* for the ARVES consensus substrate: one
//! Raft group per shard, where a shard is identified by `(tenant, workspace)`.
//! At v1.0 it was a **skeleton** - trait and type declarations only, no logic;
//! since RCR-019 the [`raft`]/[`sim`] modules provide the Stage-1 deterministic
//! core behind these contracts. Wire formats, real timers, storage engines,
//! and RPC transports remain deferred (OQ-1..9 of the I2 design → IDRs).
//!
//! ## Ground truth this crate is built on
//!
//! - **IDR-001** - The Kernel Control Plane runs **per-shard Raft**: exactly one
//!   Raft group per `(tenant, workspace)` shard. See [`ShardId`] and
//!   [`ShardConsensus`]. There is no single global consensus group.
//! - **IDR-002** - Consensus replicates **committed OUTCOMES, not invocations**.
//!   Engines/capabilities are executed *before* proposal; only the resulting,
//!   content-addressable outcome is proposed and replicated. See [`Outcome`] and
//!   [`ShardConsensus::propose`]. This is why the type replicated by this crate is
//!   an outcome payload, never an engine call. It aligns with ORCH-004
//!   (idempotent + content-addressable invocation) held upstream by the Kernel.
//! - **IDR-005** - The Raft log **is** the Write-Ahead Log **is** the decision
//!   trace (an **IDR-001 refinement**: IDR-001 + IDR-005 + ORCH-003 converge on one
//!   ordered source for replay). A single append-only sequence serves durability,
//!   replication, and replay (ORCH-003: replay from the recorded trace, never
//!   recomputation). See [`LogIndex`], [`Term`], and [`LogEntry`]. Engines may run
//!   anywhere, but the only path to truth is a commit through the shard leader.
//! - **IDR-003** - Membership changes use **joint consensus** (two-phase
//!   C_old,new -> C_new); there is **no cross-shard atomic commit** (cross-shard
//!   effects are coordinated by sagas above this layer, not by 2PC here). See
//!   [`Membership`] and [`ShardConsensus::change_membership`].
//! - **SHARD-001** - State is partitioned by `(tenant, workspace)`; the shard key
//!   is **immutable**. [`ShardId`] exposes no mutators and its fields are only
//!   set at construction; re-sharding is a migration concern, never a mutation.
//!
//! ## Ownership boundaries (why consensus owns no truth)
//!
//! Per **ORCH-001 / OWN-001**, only the **Kernel** owns cognitive truth and is the
//! sole commit gateway. This crate provides the *replication mechanism* the Kernel
//! uses to make a committed outcome durable and agreed-upon across a shard's
//! replicas; it does not decide *what* is true. The consensus layer is the
//! truth-path plumbing under the Kernel, not an owner of state. Truth is CP
//! (this crate); observability/read models are AP and live elsewhere (`arves-query`).

// ---------------------------------------------------------------------------
// Stage-1 implementation modules (RCR-019) — additive, behind the frozen
// contract below. See each module's HONEST SCOPE header.
// ---------------------------------------------------------------------------

pub mod raft;
pub mod sim;

// ---------------------------------------------------------------------------
// Shard identity (SHARD-001)
// ---------------------------------------------------------------------------

/// A tenant identifier - the outer partition key of a shard.
///
/// Newtype over an owned string so that the *shape* of the boundary is explicit
/// at every call site. Concrete encodings (UUID, ULID, etc.) are deferred to a
/// later milestone; this is a contract, not a decision.
///
/// Governing: SHARD-001 (partition by tenant/workspace; key immutable).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TenantId(pub String);

/// A workspace identifier - the inner partition key of a shard.
///
/// Governing: SHARD-001 (partition by tenant/workspace; key immutable).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspaceId(pub String);

/// The immutable key that names one shard, and therefore one Raft group.
///
/// Per **SHARD-001**, state is partitioned by `(tenant, workspace)` and the shard
/// key is **immutable**: once constructed, a `ShardId` is never mutated. Per
/// **IDR-001**, there is exactly one Raft group per `ShardId`. The fields are
/// public for construction, but the type intentionally exposes no setters -
/// treat instances as frozen after creation. Re-partitioning is a migration
/// (a new shard + data movement), never a mutation of an existing key.
///
/// Governing: SHARD-001, IDR-001.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShardId {
    /// Outer partition key. Immutable after construction (SHARD-001).
    pub tenant: TenantId,
    /// Inner partition key. Immutable after construction (SHARD-001).
    pub workspace: WorkspaceId,
}

impl ShardId {
    /// Construct the immutable shard key from its two parts.
    ///
    /// This is the only supported way to name a shard. There is deliberately no
    /// mutator; see the type-level note on SHARD-001 immutability.
    pub fn new(tenant: TenantId, workspace: WorkspaceId) -> Self {
        Self { tenant, workspace }
    }
}

// ---------------------------------------------------------------------------
// Raft primitives (IDR-005: log = WAL = decision trace)
// ---------------------------------------------------------------------------

/// A Raft term - a logical clock that increments once per election.
///
/// Terms totally order leadership: a higher term always wins. Used to reject
/// stale leaders and stale proposals.
///
/// Governing: IDR-005 (append-only log), IDR-004 (per-shard leader election).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Term(pub u64);

/// A position in the shard's append-only replicated log.
///
/// Because the Raft log *is* the WAL *is* the decision trace (**IDR-005**), this
/// single monotonic index addresses durability, replication progress, and replay
/// position simultaneously. Indices are dense and never reused.
///
/// Governing: IDR-005 (log = WAL = decision trace; append-only), ORCH-003 (replay
/// keys off the recorded trace).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct LogIndex(pub u64);

/// Identity of a replica participating in a shard's Raft group.
///
/// One `NodeId` corresponds to one voting or learning member within a single
/// shard group. The same physical node may host many shards (many groups); this
/// id names its role *within one group*.
///
/// Governing: IDR-001 (one group per shard), IDR-003 (membership).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub String);

// ---------------------------------------------------------------------------
// The replicated payload: committed OUTCOMES, not invocations (IDR-002)
// ---------------------------------------------------------------------------

/// A content address for an outcome payload.
///
/// Per **ORCH-004**, every engine/capability invocation is idempotent and
/// content-addressable; the digest of its committed outcome is what this crate
/// carries. The concrete hash function is deferred - this newtype only fixes the
/// contract that outcomes are addressed by content, enabling dedup and
/// deterministic replay (ORCH-003).
///
/// Governing: ORCH-004, IDR-002.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(pub String);

/// The unit this crate replicates: a **committed outcome**, never an invocation.
///
/// **IDR-002** is the defining constraint: engines and capabilities run *before*
/// consensus (they may run anywhere), and only the *resulting* outcome is
/// proposed and replicated. Replicating outcomes rather than commands keeps the
/// replicated log deterministic and side-effect-free on apply, and makes replay
/// (ORCH-003) a pure fold over recorded outcomes rather than a re-execution.
///
/// The `payload` is opaque bytes at this layer (the Kernel owns its meaning per
/// ORCH-001/OWN-001); `digest` is its content address (ORCH-004).
///
/// Governing: IDR-002, ORCH-004, ORCH-001, OWN-001.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// Content address of `payload` (ORCH-004: content-addressable).
    pub digest: ContentHash,
    /// Opaque, already-decided outcome bytes. Meaning is owned by the Kernel,
    /// not by consensus (ORCH-001 / OWN-001). Never an engine invocation (IDR-002).
    pub payload: Vec<u8>,
}

/// One entry in the append-only shard log (log = WAL = decision trace, IDR-005).
///
/// Every committed entry carries the term under which it was proposed, its dense
/// index, and either an [`Outcome`] (the normal case, IDR-002) or a membership
/// transition (IDR-003). The log is append-only: entries are never mutated in
/// place, which is what makes the log usable as the decision trace for replay
/// (ORCH-003).
///
/// Governing: IDR-005, IDR-002, IDR-003, ORCH-003.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// Election term this entry was created under (IDR-005/IDR-004).
    pub term: Term,
    /// Dense, monotonic position in the log (IDR-005).
    pub index: LogIndex,
    /// The replicated content of this entry.
    pub kind: EntryKind,
}

/// The two things a shard log can carry.
///
/// Normal traffic is [`EntryKind::Outcome`] (IDR-002). Reconfiguration is
/// [`EntryKind::Membership`], applied via joint consensus (IDR-003). No variant
/// carries an engine invocation, by construction.
///
/// Governing: IDR-002, IDR-003.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A committed outcome to be applied to the shard state machine.
    Outcome(Outcome),
    /// A configuration change (joint-consensus phase encoded in [`Membership`]).
    Membership(Membership),
}

// ---------------------------------------------------------------------------
// Leadership and roles (IDR-004: per-shard leader election)
// ---------------------------------------------------------------------------

/// A replica's current Raft role within one shard group.
///
/// Only the [`Role::Leader`] may accept proposals (see [`ShardConsensus::propose`]);
/// this is the single-writer property that makes the shard's log a total order.
/// Engines may run anywhere, but the only route to truth is a commit through the
/// leader (IDR-005).
///
/// Governing: IDR-004 (per-shard leader election), IDR-005.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Sole proposer/committer for the current term.
    Leader,
    /// Replicates from the leader; may vote.
    Follower,
    /// Campaigning for leadership in some term.
    Candidate,
    /// Receives log but does not vote (used during joint-consensus catch-up).
    Learner,
}

/// A snapshot of who leads a shard, if anyone, and under which term.
///
/// Returned by [`ShardConsensus::leader`]. `Absent` models the window during an
/// election when no leader is established for the shard.
///
/// Governing: IDR-004, IDR-001.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Leadership {
    /// A leader is established for `term` at node `node`.
    Established {
        /// The node currently acting as leader for the shard.
        node: NodeId,
        /// The term under which that leadership holds.
        term: Term,
    },
    /// No leader is currently known (election in progress / partition).
    Absent,
}

// ---------------------------------------------------------------------------
// Membership via joint consensus; no cross-shard atomicity (IDR-003)
// ---------------------------------------------------------------------------

/// A shard's membership configuration, capturing the joint-consensus phase.
///
/// **IDR-003** mandates joint consensus for reconfiguration: the group transits
/// through a combined `C_old,new` phase (where quorums of *both* old and new
/// configurations are required) before committing to `C_new`. This enum makes
/// that two-phase transition explicit rather than mutating a member set in place.
///
/// Note: there is deliberately **no cross-shard membership**; each `Membership`
/// scopes exactly one [`ShardId`]. Cross-shard effects are handled by sagas above
/// this layer, never by an atomic multi-shard commit (IDR-003).
///
/// Governing: IDR-003.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Membership {
    /// A single, stable configuration.
    Stable {
        /// Voting members of the shard group.
        voters: Vec<NodeId>,
        /// Non-voting catch-up members (IDR-003 learner phase).
        learners: Vec<NodeId>,
    },
    /// The transitional joint configuration `C_old,new` (IDR-003).
    Joint {
        /// The outgoing voter set.
        old_voters: Vec<NodeId>,
        /// The incoming voter set.
        new_voters: Vec<NodeId>,
        /// Learners catching up before promotion.
        learners: Vec<NodeId>,
    },
}

// ---------------------------------------------------------------------------
// Read tiers (IDR-001: linearizable / bounded-staleness / eventual)
// ---------------------------------------------------------------------------

/// Consistency tier requested by a reader of shard state.
///
/// **IDR-001** defines three read tiers over the truth path. Truth is CP; readers
/// choose how much staleness they will tolerate. `Linearizable` reads must be
/// serviced through the leader (or a leader-confirmed read); weaker tiers may be
/// served from followers or projections (`arves-query`).
///
/// Governing: IDR-001 (read tiers), OWN-001.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadTier {
    /// Strongest: reflects all commits up to the read (leader-confirmed).
    Linearizable,
    /// Bounded lag: no older than an agreed staleness bound.
    BoundedStaleness,
    /// Weakest: may lag arbitrarily; convergent over time.
    Eventual,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes surfaced by the consensus contract.
///
/// These name the *contractual* outcomes callers must handle; exhaustive wire-level
/// error taxonomy is deferred. Notably [`ConsensusError::NotLeader`] enforces the
/// single-writer route to truth (IDR-005/IDR-004), and there is no variant for a
/// cross-shard atomic commit because none exists (IDR-003).
///
/// Governing: IDR-005, IDR-004, IDR-003.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsensusError {
    /// This replica is not the leader; the caller should redirect to `leader`
    /// (which may be [`Leadership::Absent`] during an election).
    NotLeader {
        /// Best-known current leadership, to aid redirection.
        leader: Leadership,
    },
    /// A leader election is in progress for this shard; retry after backoff.
    ElectionInProgress,
    /// The referenced shard has no Raft group on this node.
    UnknownShard(ShardId),
    /// A membership change was rejected (e.g. overlapping joint transition).
    MembershipRejected,
    /// The proposal could not reach a quorum before timing out.
    QuorumUnavailable,
}

/// Convenience alias for consensus-fallible results.
pub type ConsensusResult<T> = Result<T, ConsensusError>;

// ---------------------------------------------------------------------------
// The core contract (IDR-001..005, SHARD-001)
// ---------------------------------------------------------------------------

/// Per-shard Raft consensus: propose, commit, and observe leadership for a shard.
///
/// This is the central contract of the crate and the mechanism the Kernel uses to
/// replicate truth. Each implementor instance is bound to (or dispatches over)
/// shards, where **one Raft group exists per [`ShardId`]** (IDR-001). The trait
/// replicates **committed outcomes, not invocations** (IDR-002) through an
/// **append-only log that is simultaneously the WAL and the decision trace**
/// (IDR-005), with **joint-consensus membership and no cross-shard atomic commit**
/// (IDR-003), all partitioned by the **immutable `(tenant, workspace)` key**
/// (SHARD-001).
///
/// The Kernel remains the sole owner of truth and the commit gateway
/// (ORCH-001 / OWN-001); this trait is the replication substrate beneath it, not a
/// truth owner itself. No method here interprets an [`Outcome`]'s payload.
///
/// # Method-body policy (skeleton)
///
/// Method signatures only; no default bodies are provided. This is a contracts
/// crate for the I1 milestone (Distributed Runtime); implementations arrive later.
///
/// Governing: IDR-001, IDR-002, IDR-003, IDR-004, IDR-005, SHARD-001; ORCH-001,
/// ORCH-003, ORCH-004, OWN-001.
pub trait ShardConsensus {
    /// Propose an already-decided [`Outcome`] for replication on `shard`.
    ///
    /// The outcome must already have been produced by an engine/capability run
    /// (engines run anywhere; IDR-002) and be content-addressed (ORCH-004). This
    /// call *replicates* it; it does not execute anything. Only the current
    /// leader may accept a proposal - non-leaders return
    /// [`ConsensusError::NotLeader`] so the caller can redirect (IDR-005/IDR-004).
    ///
    /// On success, returns the [`LogIndex`] the outcome was appended at; the entry
    /// is not yet necessarily committed (see [`ShardConsensus::await_commit`]).
    ///
    /// Governing: IDR-002 (outcomes not invocations), IDR-005 (append-only log),
    /// IDR-004 (leader-only), ORCH-004 (content-addressable), SHARD-001.
    fn propose(&self, shard: &ShardId, outcome: Outcome) -> ConsensusResult<LogIndex>;

    /// Block/await until the entry at `index` on `shard` is committed (durably
    /// replicated to a quorum) and return its committed [`LogEntry`].
    ///
    /// "Committed" means the entry has been agreed by a quorum and is therefore a
    /// permanent part of the decision trace (IDR-005), eligible for apply/replay
    /// (ORCH-003). Bodies are deferred; the signature fixes that commit is a
    /// distinct step from proposal.
    ///
    /// Governing: IDR-005, ORCH-003, IDR-001.
    fn await_commit(&self, shard: &ShardId, index: LogIndex) -> ConsensusResult<LogEntry>;

    /// Report the current [`Leadership`] for `shard`, if any is established.
    ///
    /// Used by callers to route proposals to the leader and by observers to track
    /// per-shard elections (IDR-004). May return [`Leadership::Absent`] during an
    /// election.
    ///
    /// Governing: IDR-004 (per-shard leader election), IDR-001.
    fn leader(&self, shard: &ShardId) -> ConsensusResult<Leadership>;

    /// This node's current [`Role`] within `shard`'s group.
    ///
    /// Governing: IDR-004, IDR-001.
    fn role(&self, shard: &ShardId) -> ConsensusResult<Role>;

    /// Read committed shard state at the requested consistency [`ReadTier`].
    ///
    /// `Linearizable` reads are serviced through the leader; weaker tiers may be
    /// served from followers/projections (IDR-001 read tiers). Returns the highest
    /// [`LogIndex`] guaranteed visible under the requested tier. This crate returns
    /// a log position, not truth content - the Kernel owns truth (ORCH-001).
    ///
    /// Governing: IDR-001 (read tiers), ORCH-001 / OWN-001.
    fn read_index(&self, shard: &ShardId, tier: ReadTier) -> ConsensusResult<LogIndex>;

    /// Begin a joint-consensus membership change for `shard`.
    ///
    /// Per **IDR-003**, the group transits through a [`Membership::Joint`] phase
    /// before committing the target [`Membership::Stable`] configuration. The
    /// change itself is replicated as an [`EntryKind::Membership`] log entry
    /// (IDR-005) and is scoped to exactly one shard - there is **no cross-shard
    /// atomic reconfiguration** (IDR-003).
    ///
    /// Governing: IDR-003 (joint consensus; no cross-shard atomic commit),
    /// IDR-005, SHARD-001.
    fn change_membership(
        &self,
        shard: &ShardId,
        target: Membership,
    ) -> ConsensusResult<LogIndex>;
}

// ---------------------------------------------------------------------------
// Skeleton self-check (trivial, compiling; no logic under test)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Confirms the shard key is constructed from its immutable parts (SHARD-001).
    #[test]
    fn shard_id_constructs_from_parts() {
        let shard = ShardId::new(TenantId("t".into()), WorkspaceId("w".into()));
        assert_eq!(shard.tenant, TenantId("t".into()));
        assert_eq!(shard.workspace, WorkspaceId("w".into()));
    }

    /// Confirms an entry carries an outcome, not an invocation (IDR-002).
    #[test]
    fn log_entry_carries_outcome() {
        let entry = LogEntry {
            term: Term(1),
            index: LogIndex(0),
            kind: EntryKind::Outcome(Outcome {
                digest: ContentHash("h".into()),
                payload: Vec::new(),
            }),
        };
        assert_eq!(entry.term, Term(1));
        assert!(matches!(entry.kind, EntryKind::Outcome(_)));
    }
}

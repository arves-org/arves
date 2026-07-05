//! ARVES :: arves-consensus :: raft — deterministic per-shard Raft CORE.
//!
//! I2 Stage 1 (RCR-019) + Stage 2 (RCR-020), implemented BEHIND the frozen
//! v1.0 contract of this crate, additively — no frozen type or trait signature
//! is changed. Approved design: `docs/design/I2_Cluster_Kernel_Design.md`
//! (§3.7: the Raft state machine is "a deterministic function of (messages,
//! timers-as-events); the network adapter is a shell around it").
//!
//! # What this module is
//!
//! One shard's Raft state machine as a **pure step function**: terms, leader
//! election with randomized-but-SEEDED timeouts driven by an injected logical
//! `tick()`, log append/replication, commit-index advance on quorum, and
//! follower catch-up by next-index backtracking. Stage 2 (RCR-020) adds
//! **joint-consensus membership change** (IDR-003: C_old,new overlap rule — a
//! decision requires majorities of BOTH configurations while joint, so two
//! disjoint majorities cannot exist during a transition), **leadership
//! transfer** ([`MsgBody::TimeoutNow`]), and the thesis-§4.2.3 **leadership
//! check** (a replica in fresh contact with a current leader ignores
//! non-transfer vote requests, so removed servers cannot disrupt the group —
//! the design §3.21 "disruptive rejoin" mitigation). It consumes and produces
//! [`Envelope`] values; it never touches a socket, a wall clock, or OS
//! randomness (constitution: Deterministic over Dynamic; design §3.7).
//!
//! # HONEST SCOPE (Stage 1 + Stage 2)
//!
//! - **In-process only.** Transport is deliberately absent; the deterministic
//!   [`crate::sim`] harness is the only vehicle (design ladder I2.1..I2.6 run
//!   in-process; sockets arrive at the step where they are the property under
//!   test). No network fault-tolerance is claimed.
//! - **No durability wiring.** The log lives in memory inside the core. IDR-005
//!   (Raft log IS the WAL IS the decision trace) is discharged at the
//!   persistence-wiring stage; Stage 1/2 create NO durable artifact, hence no
//!   second-log drift (design §3.21 risk 1).
//! - **Joint-consensus membership IS implemented** (ladder step I2.8, RCR-020):
//!   a configuration takes effect when its entry is APPENDED (Raft §6); while
//!   joint, elections and commits require majorities of both C_old and C_new;
//!   the leader auto-appends C_new once C_old,new commits, and steps down if
//!   it is not in C_new. Learner catch-up/promotion protocol is NOT
//!   implemented — learner ids are carried opaquely and replicated to, but no
//!   promotion logic exists (recorded as RCR-020 DR-4).
//! - **No real read tiers** (I2.9) and **no snapshot install** (OQ-1): a
//!   far-behind or freshly-joining replica catches up by log backtracking only.
//!
//! # Payload opacity (ORCH-001 / OWN-001)
//!
//! This module moves [`LogEntry`]/[`Outcome`](crate::Outcome) values; it exposes
//! no accessor that interprets an outcome payload. Consensus decides ordering,
//! never meaning.
//!
//! Governing: IDR-001 (per-shard group, CP), IDR-002 (committed outcomes, never
//! invocations), IDR-004 (per-shard election; stale leaders rejected by term),
//! IDR-005 (append-only log semantics), ORCH-003/004, OWN-001, SHARD-001.

use crate::{
    ConsensusError, ConsensusResult, EntryKind, Leadership, LogEntry, LogIndex, Membership,
    NodeId, Role, Term,
};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Deterministic seeded randomness (no OS entropy, ever)
// ---------------------------------------------------------------------------

/// SplitMix64 — a tiny, well-known deterministic PRNG. The ONLY randomness in
/// this crate; always constructed from an explicit, recorded seed so every
/// election-timeout draw is replayable (design resolution DR-3 in RCR-019).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetRng(u64);

impl DetRng {
    /// Construct from an explicit seed. Callers must record the seed.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next raw 64-bit value (SplitMix64).
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform draw in `[lo, hi)`. Used for the randomized election timeout.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        debug_assert!(hi > lo);
        lo + self.next_u64() % (hi - lo)
    }
}

// ---------------------------------------------------------------------------
// Logical-time constants (ticks). Wall clocks never appear in this module.
// Stage-1 compile-time constants; externalizing them is the OQ-4 IDR
// (design §3.22) — recorded as design resolution DR-3 in RCR-019.
// ---------------------------------------------------------------------------

/// Leader sends AppendEntries (heartbeat/replication) every N logical ticks.
pub const HEARTBEAT_INTERVAL: u64 = 2;
/// Election timeout lower bound (inclusive), in logical ticks.
pub const ELECTION_TIMEOUT_MIN: u64 = 10;
/// Election timeout upper bound (exclusive), in logical ticks.
pub const ELECTION_TIMEOUT_MAX: u64 = 20;

// ---------------------------------------------------------------------------
// Messages (the Raft RPCs, as values on an in-process bus)
// ---------------------------------------------------------------------------

/// A Raft protocol message body. These are the classic four RPC halves; wire
/// format is deliberately NOT decided here (OQ-3 IDR) — Stage 1 messages are
/// in-process values only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MsgBody {
    /// Candidate solicits a vote (Raft §5.2/§5.4.1: up-to-date check fields).
    RequestVote {
        /// Candidate's term.
        term: Term,
        /// Index of candidate's last log entry (0 = empty log).
        last_log_index: u64,
        /// Term of candidate's last log entry (Term(0) = empty log).
        last_log_term: Term,
        /// `true` iff this campaign was initiated by a leadership transfer
        /// ([`MsgBody::TimeoutNow`]): it bypasses the thesis-§4.2.3 leadership
        /// check so a deliberate handover is never mistaken for a disruptive
        /// removed-server campaign (RCR-020).
        transfer: bool,
    },
    /// Vote response.
    VoteReply {
        /// Responder's current term (for stale-candidate step-down).
        term: Term,
        /// Whether the vote was granted.
        granted: bool,
    },
    /// Leader replicates entries / asserts leadership (empty = heartbeat).
    AppendEntries {
        /// Leader's term.
        term: Term,
        /// Index of the entry immediately preceding `entries` (0 = from start).
        prev_log_index: u64,
        /// Term of the entry at `prev_log_index` (Term(0) if none).
        prev_log_term: Term,
        /// Entries to replicate (leader's log suffix from `prev_log_index + 1`).
        entries: Vec<LogEntry>,
        /// Leader's commit index — followers advance commit up to the match point.
        leader_commit: u64,
    },
    /// AppendEntries response.
    AppendReply {
        /// Responder's current term (for stale-leader step-down).
        term: Term,
        /// Whether the consistency check passed and entries were appended.
        success: bool,
        /// On success: highest index known replicated on the responder.
        match_index: u64,
    },
    /// Leadership transfer (Raft thesis §3.10, Stage 2 / RCR-020): the current
    /// leader instructs a caught-up voter to time out IMMEDIATELY and campaign.
    /// The recipient starts an election at `term + 1` without waiting for its
    /// randomized deadline; the old leader is then deposed by the higher term.
    TimeoutNow {
        /// The transferring leader's current term (stale transfers are ignored).
        term: Term,
    },
}

impl MsgBody {
    /// The term carried by this message (drives Raft's "higher term wins" rule).
    pub fn term(&self) -> Term {
        match self {
            MsgBody::RequestVote { term, .. }
            | MsgBody::VoteReply { term, .. }
            | MsgBody::AppendEntries { term, .. }
            | MsgBody::AppendReply { term, .. }
            | MsgBody::TimeoutNow { term } => *term,
        }
    }
}

/// One addressed message on the in-process bus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    /// Sending replica.
    pub from: NodeId,
    /// Destination replica.
    pub to: NodeId,
    /// Raft RPC body.
    pub body: MsgBody,
}

// ---------------------------------------------------------------------------
// Voter configuration (IDR-003 joint consensus) — Stage 2, RCR-020
// ---------------------------------------------------------------------------

/// The effective voter configuration of one replica, derived from the log.
///
/// `new == None` is a stable configuration; `new == Some(..)` is the joint
/// phase `C_old,new` (IDR-003). The joint decision rule ([`Self::satisfied_by`])
/// requires majorities of BOTH sets, which is exactly what removes the
/// two-disjoint-majorities window during a membership transition: any electing
/// or committing quorum while joint intersects every other such quorum in both
/// configurations.
///
/// Per Raft §6, a configuration takes effect as soon as its entry is APPENDED
/// to the log (not when committed); on log truncation the configuration rolls
/// back to the latest surviving Membership entry (or the construction-time
/// base). See [`RaftNode::refresh_config_from_log`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct VoterConfig {
    /// C_old (or the sole configuration when `new` is `None`).
    old: BTreeSet<NodeId>,
    /// C_new while in the joint phase.
    new: Option<BTreeSet<NodeId>>,
    /// Non-voting replication targets, carried opaquely (no promotion logic —
    /// RCR-020 DR-4).
    learners: BTreeSet<NodeId>,
}

impl VoterConfig {
    fn stable(voters: BTreeSet<NodeId>) -> Self {
        Self { old: voters, new: None, learners: BTreeSet::new() }
    }

    fn from_membership(m: &Membership) -> Self {
        match m {
            Membership::Stable { voters, learners } => Self {
                old: voters.iter().cloned().collect(),
                new: None,
                learners: learners.iter().cloned().collect(),
            },
            Membership::Joint { old_voters, new_voters, learners } => Self {
                old: old_voters.iter().cloned().collect(),
                new: Some(new_voters.iter().cloned().collect()),
                learners: learners.iter().cloned().collect(),
            },
        }
    }

    /// The joint-consensus decision rule (IDR-003): `set` decides (elects or
    /// commits) iff it contains a majority of C_old AND, while joint, a
    /// majority of C_new. An empty voter set is never satisfied (a joining
    /// replica with no configuration can neither vote itself in nor commit).
    fn satisfied_by(&self, set: &BTreeSet<NodeId>) -> bool {
        fn majority(voters: &BTreeSet<NodeId>, set: &BTreeSet<NodeId>) -> bool {
            !voters.is_empty() && set.intersection(voters).count() >= voters.len() / 2 + 1
        }
        majority(&self.old, set)
            && self.new.as_ref().map_or(true, |nv| majority(nv, set))
    }

    /// Is `id` a voter in ANY phase of this configuration?
    fn is_voter(&self, id: &NodeId) -> bool {
        self.old.contains(id) || self.new.as_ref().is_some_and(|nv| nv.contains(id))
    }

    /// All voters (both phases while joint).
    fn voters(&self) -> BTreeSet<NodeId> {
        let mut v = self.old.clone();
        if let Some(nv) = &self.new {
            v.extend(nv.iter().cloned());
        }
        v
    }

    /// Everyone the leader replicates to: voters plus learners.
    fn replication_targets(&self) -> BTreeSet<NodeId> {
        let mut t = self.voters();
        t.extend(self.learners.iter().cloned());
        t
    }
}

// ---------------------------------------------------------------------------
// The per-replica Raft state machine
// ---------------------------------------------------------------------------

/// One replica's Raft state machine for ONE shard group (IDR-001: one group
/// per shard). Deterministic: identical construction + identical event
/// sequence (ticks and envelopes) ⇒ identical state and outputs.
///
/// Log indexing (design resolution DR-1 in RCR-019): the first entry sits at
/// `LogIndex(1)`; index 0 is the standard Raft "empty log" sentinel used only
/// in `prev_log_index` arithmetic. Indices are dense and never reused, per the
/// frozen [`LogIndex`] contract.
#[derive(Debug)]
pub struct RaftNode {
    id: NodeId,
    /// Construction-time voter set, in force until the first Membership entry
    /// appears in the log (empty for a joining replica — RCR-020).
    base_voters: BTreeSet<NodeId>,
    /// Effective configuration = latest Membership entry in the log, else base
    /// (config takes effect on APPEND, rolls back on truncation — Raft §6).
    config: VoterConfig,
    /// Log index of the entry that set `config` (0 = construction-time base).
    config_index: u64,
    current_term: Term,
    voted_for: Option<NodeId>,
    /// `log[i]` has `index == LogIndex(i as u64 + 1)`.
    log: Vec<LogEntry>,
    commit_index: u64,
    role: Role,
    leader_hint: Option<NodeId>,
    votes_granted: BTreeSet<NodeId>,
    next_index: BTreeMap<NodeId, u64>,
    match_index: BTreeMap<NodeId, u64>,
    election_elapsed: u64,
    election_deadline: u64,
    heartbeat_elapsed: u64,
    /// Ticks since this replica last heard from a CURRENT-term leader
    /// (thesis §4.2.3 leadership check, RCR-020): while fresh
    /// (< [`ELECTION_TIMEOUT_MIN`]), non-transfer RequestVotes are ignored
    /// entirely (no term bump, no reply) so a removed server that keeps
    /// campaigning can never disrupt a healthy group. Initialized STALE so
    /// bootstrap elections proceed normally.
    leader_contact_elapsed: u64,
    rng: DetRng,
}

impl RaftNode {
    /// Construct a follower with a seeded randomized election deadline.
    ///
    /// `members` MUST contain `id`. The seed is the node's entire randomness
    /// budget — record it and the run is replayable.
    pub fn new(id: NodeId, members: Vec<NodeId>, seed: u64) -> Self {
        assert!(members.contains(&id), "members must include self");
        Self::with_base(id, members.into_iter().collect(), seed)
    }

    /// Construct a JOINING replica (RCR-020): it knows no voter configuration
    /// until it receives a log carrying one, so it can neither campaign nor
    /// vote itself into existence. The leader replicates to it once a joint
    /// configuration naming it is appended (IDR-003).
    pub fn new_joining(id: NodeId, seed: u64) -> Self {
        Self::with_base(id, BTreeSet::new(), seed)
    }

    fn with_base(id: NodeId, base_voters: BTreeSet<NodeId>, seed: u64) -> Self {
        let mut rng = DetRng::new(seed);
        let election_deadline = rng.range(ELECTION_TIMEOUT_MIN, ELECTION_TIMEOUT_MAX);
        Self {
            id,
            config: VoterConfig::stable(base_voters.clone()),
            base_voters,
            config_index: 0,
            current_term: Term(0),
            voted_for: None,
            log: Vec::new(),
            commit_index: 0,
            role: Role::Follower,
            leader_hint: None,
            votes_granted: BTreeSet::new(),
            next_index: BTreeMap::new(),
            match_index: BTreeMap::new(),
            election_elapsed: 0,
            election_deadline,
            heartbeat_elapsed: 0,
            leader_contact_elapsed: ELECTION_TIMEOUT_MAX, // stale: never heard a leader
            rng,
        }
    }

    // -- accessors (read-only observation surface) --------------------------

    /// This replica's id.
    pub fn id(&self) -> &NodeId {
        &self.id
    }
    /// Current role (IDR-004).
    pub fn role(&self) -> Role {
        self.role
    }
    /// Current term.
    pub fn current_term(&self) -> Term {
        self.current_term
    }
    /// Highest committed index (quorum-replicated; IDR-001).
    pub fn commit_index(&self) -> LogIndex {
        LogIndex(self.commit_index)
    }
    /// The append-only log (read-only view; IDR-005 semantics).
    pub fn log(&self) -> &[LogEntry] {
        &self.log
    }
    /// Best-known leadership for redirection (frozen contract shape).
    pub fn leadership(&self) -> Leadership {
        match &self.leader_hint {
            Some(node) => Leadership::Established { node: node.clone(), term: self.current_term },
            None => Leadership::Absent,
        }
    }
    /// Index of the last log entry (0 = empty).
    pub fn last_log_index(&self) -> u64 {
        self.log.len() as u64
    }

    fn last_log_term(&self) -> Term {
        self.log.last().map(|e| e.term).unwrap_or(Term(0))
    }

    /// Whether this replica is a voter in its effective configuration.
    /// Joining replicas (empty base, no config entry yet) and removed replicas
    /// are NOT voters: they never campaign, so a removed node cannot disrupt
    /// the group (IDR-003).
    pub fn is_voter(&self) -> bool {
        self.config.is_voter(&self.id)
    }

    /// Current effective membership view (for observability/tests):
    /// the frozen [`Membership`] shape of the effective configuration.
    pub fn membership(&self) -> Membership {
        match &self.config.new {
            Some(nv) => Membership::Joint {
                old_voters: self.config.old.iter().cloned().collect(),
                new_voters: nv.iter().cloned().collect(),
                learners: self.config.learners.iter().cloned().collect(),
            },
            None => Membership::Stable {
                voters: self.config.old.iter().cloned().collect(),
                learners: self.config.learners.iter().cloned().collect(),
            },
        }
    }

    /// Replication targets: everyone in the effective config except self.
    fn peers(&self) -> Vec<NodeId> {
        self.config
            .replication_targets()
            .into_iter()
            .filter(|p| *p != self.id)
            .collect()
    }

    /// Vote solicitation targets: voters (both phases while joint) except self.
    fn voter_peers(&self) -> Vec<NodeId> {
        self.config.voters().into_iter().filter(|p| *p != self.id).collect()
    }

    /// Recompute the effective configuration from the log: the LATEST
    /// Membership entry wins; with none, the construction-time base holds.
    /// Called after every append/truncation that may carry or remove a
    /// Membership entry (config-on-append + rollback-on-truncation, Raft §6).
    fn refresh_config_from_log(&mut self) {
        match self.log.iter().rev().find_map(|e| match &e.kind {
            EntryKind::Membership(m) => Some((e.index.0, VoterConfig::from_membership(m))),
            EntryKind::Outcome(_) => None,
        }) {
            Some((idx, cfg)) => {
                self.config = cfg;
                self.config_index = idx;
            }
            None => {
                self.config = VoterConfig::stable(self.base_voters.clone());
                self.config_index = 0;
            }
        }
    }

    /// Leader-side: make sure every replication target has next/match state
    /// (new targets appear when a joint configuration is appended).
    fn ensure_peer_indices(&mut self) {
        let next = self.last_log_index() + 1;
        for p in self.peers() {
            self.next_index.entry(p.clone()).or_insert(next);
            self.match_index.entry(p).or_insert(0);
        }
    }

    fn reset_election_timer(&mut self) {
        self.election_elapsed = 0;
        self.election_deadline = self.rng.range(ELECTION_TIMEOUT_MIN, ELECTION_TIMEOUT_MAX);
    }

    fn become_follower(&mut self, term: Term) {
        self.current_term = term;
        self.role = Role::Follower;
        self.voted_for = None;
        self.votes_granted.clear();
        self.leader_hint = None;
        self.reset_election_timer();
    }

    /// Same-term step-down (a leader that committed a C_new excluding itself).
    /// `voted_for` is deliberately PRESERVED: resetting it at an unchanged term
    /// could grant a second vote in the same term and break Election Safety.
    fn step_down_keep_term(&mut self) {
        self.role = Role::Follower;
        self.votes_granted.clear();
        self.leader_hint = None;
        self.reset_election_timer();
    }

    // -- injected logical time ----------------------------------------------

    /// Advance logical time by one tick. May fire an election timeout
    /// (follower/candidate) or a heartbeat (leader). The ONLY clock is this
    /// injected tick — determinism over convenience.
    pub fn tick(&mut self) -> Vec<Envelope> {
        match self.role {
            Role::Leader => {
                self.heartbeat_elapsed += 1;
                if self.heartbeat_elapsed >= HEARTBEAT_INTERVAL {
                    self.heartbeat_elapsed = 0;
                    self.broadcast_append()
                } else {
                    Vec::new()
                }
            }
            _ => {
                self.election_elapsed += 1;
                self.leader_contact_elapsed = self.leader_contact_elapsed.saturating_add(1);
                if self.election_elapsed >= self.election_deadline {
                    self.start_election(false)
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn start_election(&mut self, transfer: bool) -> Vec<Envelope> {
        if !self.is_voter() {
            // Non-voters (joining replicas without a config, removed nodes,
            // learners) never campaign — a removed node cannot disrupt the
            // group (IDR-003; RCR-020).
            self.reset_election_timer();
            return Vec::new();
        }
        self.current_term = Term(self.current_term.0 + 1);
        self.role = Role::Candidate;
        self.voted_for = Some(self.id.clone());
        self.votes_granted = BTreeSet::from([self.id.clone()]);
        self.leader_hint = None;
        self.reset_election_timer();
        if self.config.satisfied_by(&self.votes_granted) {
            // Single-replica group: elected immediately (I1's quorum=1 shape).
            return self.become_leader();
        }
        let (term, lli, llt) = (self.current_term, self.last_log_index(), self.last_log_term());
        self.voter_peers()
            .into_iter()
            .map(|to| Envelope {
                from: self.id.clone(),
                to,
                body: MsgBody::RequestVote {
                    term,
                    last_log_index: lli,
                    last_log_term: llt,
                    transfer,
                },
            })
            .collect()
    }

    fn become_leader(&mut self) -> Vec<Envelope> {
        self.role = Role::Leader;
        self.leader_hint = Some(self.id.clone());
        let next = self.last_log_index() + 1;
        for p in self.peers() {
            self.next_index.insert(p.clone(), next);
            self.match_index.insert(p, 0);
        }
        self.heartbeat_elapsed = 0;
        // Design resolution DR-2 (RCR-019): NO no-op entry on election — the
        // frozen EntryKind carries only Outcome|Membership, and adding a NoOp
        // variant would change the frozen contract. Raft §5.4.2 is preserved:
        // prior-term entries commit only once a current-term entry reaches
        // quorum (see advance_commit's term guard).
        self.advance_commit();
        // A new leader may inherit an already-committed configuration entry
        // (e.g. it crashed into leadership mid-transition): continue IDR-003.
        let mut out = self.maybe_finalize_membership();
        out.extend(self.broadcast_append());
        out
    }

    fn append_for(&self, peer: &NodeId) -> Envelope {
        let next = self.next_index.get(peer).copied().unwrap_or(self.last_log_index() + 1);
        let prev = next.saturating_sub(1);
        let prev_log_term =
            if prev == 0 { Term(0) } else { self.log[prev as usize - 1].term };
        let entries = if next <= self.last_log_index() {
            self.log[(next - 1) as usize..].to_vec()
        } else {
            Vec::new()
        };
        Envelope {
            from: self.id.clone(),
            to: peer.clone(),
            body: MsgBody::AppendEntries {
                term: self.current_term,
                prev_log_index: prev,
                prev_log_term,
                entries,
                leader_commit: self.commit_index,
            },
        }
    }

    fn broadcast_append(&self) -> Vec<Envelope> {
        self.peers().iter().map(|p| self.append_for(p)).collect()
    }

    // -- client proposal (leader-only path to the log; IDR-004/005) ---------

    /// Append an already-decided entry at the leader and emit replication
    /// messages. Non-leaders refuse with [`ConsensusError::NotLeader`]
    /// (OWN-001: followers are derived replicas, never writers).
    ///
    /// Returns the appended [`LogIndex`]; the entry is NOT yet committed
    /// (commit is a quorum event — see [`RaftNode::commit_index`]).
    pub fn client_propose(&mut self, kind: EntryKind) -> ConsensusResult<(LogIndex, Vec<Envelope>)> {
        if self.role != Role::Leader {
            return Err(ConsensusError::NotLeader { leader: self.leadership() });
        }
        if matches!(kind, EntryKind::Membership(_)) {
            // The joint-consensus discipline is NOT bypassable: a raw
            // single-step Membership proposal could create two disjoint
            // majorities. The only reconfiguration door is
            // [`RaftNode::change_membership`] (IDR-003; RCR-020 DR-2).
            return Err(ConsensusError::MembershipRejected);
        }
        let index = self.last_log_index() + 1;
        self.log.push(LogEntry { term: self.current_term, index: LogIndex(index), kind });
        // Single-replica group commits immediately (quorum = 1).
        self.advance_commit();
        Ok((LogIndex(index), self.broadcast_append()))
    }

    /// Begin a JOINT-CONSENSUS membership change (IDR-003; Stage 2, RCR-020).
    ///
    /// The caller names the TARGET stable configuration; the joint phase is
    /// internal. The leader appends `C_old,new` (effective immediately on
    /// append), replicates it under the dual-majority rule, auto-appends
    /// `C_new` once the joint entry commits, and steps down if it is not in
    /// `C_new`. Returns the [`LogIndex`] of the JOINT entry ("begin", per the
    /// frozen trait doc).
    ///
    /// Refusals (all [`ConsensusError::MembershipRejected`]):
    /// - a reconfiguration is already in flight (joint phase active, or a
    ///   configuration entry not yet committed) — one transition per shard
    ///   (design §3.8);
    /// - the target is not a non-empty `Membership::Stable` (callers never
    ///   submit a joint config; that phase is owned by this method).
    pub fn change_membership(
        &mut self,
        target: Membership,
    ) -> ConsensusResult<(LogIndex, Vec<Envelope>)> {
        if self.role != Role::Leader {
            return Err(ConsensusError::NotLeader { leader: self.leadership() });
        }
        if self.config.new.is_some() || self.config_index > self.commit_index {
            return Err(ConsensusError::MembershipRejected);
        }
        let (new_voters, learners) = match target {
            Membership::Stable { voters, learners } if !voters.is_empty() => (voters, learners),
            _ => return Err(ConsensusError::MembershipRejected),
        };
        let joint = Membership::Joint {
            old_voters: self.config.old.iter().cloned().collect(),
            new_voters,
            learners,
        };
        let index = self.last_log_index() + 1;
        self.log.push(LogEntry {
            term: self.current_term,
            index: LogIndex(index),
            kind: EntryKind::Membership(joint),
        });
        // Config takes effect on APPEND (Raft §6): from here on, elections and
        // commits need majorities of BOTH C_old and C_new.
        self.refresh_config_from_log();
        self.ensure_peer_indices();
        self.advance_commit(); // single-replica degenerate case
        let mut out = self.maybe_finalize_membership();
        out.extend(self.broadcast_append());
        Ok((LogIndex(index), out))
    }

    /// Leadership transfer (Raft thesis §3.10; Stage 2, RCR-020): hand the
    /// lead to `target` without an availability gap longer than one election.
    ///
    /// - Not leader → [`ConsensusError::NotLeader`].
    /// - `target` is self or not a voter → [`ConsensusError::MembershipRejected`]
    ///   (documented reuse of the frozen error: the target is not a usable
    ///   member of the current voter configuration — RCR-020 DR-6).
    /// - `target` not fully caught up → emits a catch-up AppendEntries and does
    ///   NOT transfer yet (the caller retries after replication).
    /// - Otherwise emits [`MsgBody::TimeoutNow`]; the target campaigns at
    ///   `term + 1` immediately and this leader is deposed by the higher term.
    pub fn transfer_leadership(&mut self, target: &NodeId) -> ConsensusResult<Vec<Envelope>> {
        if self.role != Role::Leader {
            return Err(ConsensusError::NotLeader { leader: self.leadership() });
        }
        if *target == self.id || !self.config.is_voter(target) {
            return Err(ConsensusError::MembershipRejected);
        }
        if self.match_index.get(target).copied().unwrap_or(0) < self.last_log_index() {
            return Ok(vec![self.append_for(target)]);
        }
        Ok(vec![Envelope {
            from: self.id.clone(),
            to: target.clone(),
            body: MsgBody::TimeoutNow { term: self.current_term },
        }])
    }

    /// Leader-side commit advance: highest `n > commit_index` replicated on a
    /// joint-rule quorum AND belonging to the current term (Raft §5.4.2 —
    /// never count replicas for a prior-term entry).
    fn advance_commit(&mut self) {
        if self.role != Role::Leader {
            return;
        }
        let mut n = self.last_log_index();
        while n > self.commit_index {
            if self.log[n as usize - 1].term == self.current_term && self.commit_quorum_met(n) {
                self.commit_index = n;
                break;
            }
            n -= 1;
        }
    }

    /// Do the holders of log index `n` satisfy the effective configuration?
    /// While joint this needs majorities of BOTH C_old and C_new (IDR-003).
    /// The leader's own log always holds `n`, but it only counts toward a
    /// majority if it is a voter of the respective set (a leader excluded from
    /// C_new can still commit C_new without counting itself).
    fn commit_quorum_met(&self, n: u64) -> bool {
        let mut holders: BTreeSet<NodeId> = self
            .match_index
            .iter()
            .filter(|(_, m)| **m >= n)
            .map(|(p, _)| p.clone())
            .collect();
        holders.insert(self.id.clone());
        self.config.satisfied_by(&holders)
    }

    /// Drive the two-phase IDR-003 transition forward after any commit
    /// advance: once `C_old,new` commits, append `C_new`; once `C_new`
    /// commits and self is not in it, step down.
    fn maybe_finalize_membership(&mut self) -> Vec<Envelope> {
        if self.role != Role::Leader
            || self.config_index == 0
            || self.config_index > self.commit_index
        {
            return Vec::new();
        }
        if let Some(new_voters) = self.config.new.clone() {
            // Phase 2: the joint entry is committed under the dual-majority
            // rule — append the target stable configuration.
            let stable = Membership::Stable {
                voters: new_voters.iter().cloned().collect(),
                learners: self.config.learners.iter().cloned().collect(),
            };
            let index = self.last_log_index() + 1;
            self.log.push(LogEntry {
                term: self.current_term,
                index: LogIndex(index),
                kind: EntryKind::Membership(stable),
            });
            self.refresh_config_from_log();
            self.ensure_peer_indices();
            self.advance_commit(); // single-replica degenerate case
            let mut out = self.maybe_finalize_membership(); // may step down (1-node case)
            out.extend(self.broadcast_append());
            out
        } else if !self.is_voter() {
            // C_new is committed and excludes this leader: replication duty
            // done — step down (Raft §6; IDR-004 in-flight work discarded by
            // the successor's election, no partial truth).
            self.step_down_keep_term();
            Vec::new()
        } else {
            Vec::new()
        }
    }

    // -- the step function ---------------------------------------------------

    /// Consume one message; return the messages it provokes. Pure with respect
    /// to (state, input): no clock, no I/O, no unseeded randomness.
    pub fn step(&mut self, env: Envelope) -> Vec<Envelope> {
        // Leadership check (thesis §4.2.3; RCR-020): while this replica is a
        // leader or in fresh contact with one, a NON-transfer RequestVote is
        // ignored entirely — no term bump, no reply. This is what stops a
        // REMOVED server (no longer replicated to, campaigning forever) from
        // deposing healthy leaders by term inflation. Deliberate handovers
        // carry `transfer: true` and bypass the check.
        if let MsgBody::RequestVote { transfer: false, .. } = &env.body {
            if self.role == Role::Leader || self.leader_contact_elapsed < ELECTION_TIMEOUT_MIN {
                return Vec::new();
            }
        }
        if env.body.term() > self.current_term {
            // Higher term always wins (IDR-004: stale leadership dies by term).
            self.become_follower(env.body.term());
        }
        match env.body {
            MsgBody::RequestVote { term, last_log_index, last_log_term, transfer: _ } => {
                // §5.4.1 up-to-date check: candidate's log must not be behind.
                let up_to_date = last_log_term > self.last_log_term()
                    || (last_log_term == self.last_log_term()
                        && last_log_index >= self.last_log_index());
                let grant = term == self.current_term
                    && up_to_date
                    && (self.voted_for.is_none() || self.voted_for.as_ref() == Some(&env.from));
                if grant {
                    self.voted_for = Some(env.from.clone());
                    self.reset_election_timer();
                }
                vec![Envelope {
                    from: self.id.clone(),
                    to: env.from,
                    body: MsgBody::VoteReply { term: self.current_term, granted: grant },
                }]
            }
            MsgBody::VoteReply { term, granted } => {
                if self.role == Role::Candidate && term == self.current_term && granted {
                    self.votes_granted.insert(env.from);
                    // Joint rule (IDR-003): while in C_old,new an election
                    // needs majorities of BOTH configurations.
                    if self.config.satisfied_by(&self.votes_granted) {
                        return self.become_leader();
                    }
                }
                Vec::new()
            }
            MsgBody::AppendEntries { term, prev_log_index, prev_log_term, entries, leader_commit } => {
                if term < self.current_term {
                    // Stale leader: refuse; our term in the reply forces step-down.
                    return vec![Envelope {
                        from: self.id.clone(),
                        to: env.from,
                        body: MsgBody::AppendReply {
                            term: self.current_term,
                            success: false,
                            match_index: 0,
                        },
                    }];
                }
                // term == current_term here (higher handled at entry).
                if self.role == Role::Candidate {
                    self.role = Role::Follower;
                }
                self.leader_hint = Some(env.from.clone());
                self.reset_election_timer();
                self.leader_contact_elapsed = 0; // fresh leader contact (§4.2.3)
                // Log-consistency check (Log Matching maintenance).
                let prev_ok = prev_log_index == 0
                    || (prev_log_index <= self.last_log_index()
                        && self.log[prev_log_index as usize - 1].term == prev_log_term);
                if !prev_ok {
                    return vec![Envelope {
                        from: self.id.clone(),
                        to: env.from,
                        body: MsgBody::AppendReply {
                            term: self.current_term,
                            success: false,
                            match_index: 0,
                        },
                    }];
                }
                let match_point = prev_log_index + entries.len() as u64;
                for e in entries {
                    let i = e.index.0;
                    if i <= self.last_log_index() {
                        if self.log[i as usize - 1].term != e.term {
                            // Conflicting suffix: truncate, then append the
                            // leader's entry. Only FOLLOWERS ever truncate —
                            // the Leader Append-Only property holds by
                            // construction (no truncation path exists outside
                            // this follower branch).
                            self.log.truncate(i as usize - 1);
                            self.log.push(e);
                        }
                        // else: identical entry already present — skip.
                    } else {
                        self.log.push(e);
                    }
                }
                // Appended entries may carry a Membership; a truncation may
                // have removed one — recompute the effective configuration
                // (config-on-append + rollback-on-truncation, Raft §6; RCR-020).
                self.refresh_config_from_log();
                if leader_commit > self.commit_index {
                    // Bound by the verified match point, never the raw local
                    // tail (a stale un-truncated suffix must not commit).
                    self.commit_index = leader_commit.min(match_point.max(prev_log_index));
                }
                vec![Envelope {
                    from: self.id.clone(),
                    to: env.from,
                    body: MsgBody::AppendReply {
                        term: self.current_term,
                        success: true,
                        match_index: match_point,
                    },
                }]
            }
            MsgBody::AppendReply { term, success, match_index } => {
                if self.role != Role::Leader || term != self.current_term {
                    return Vec::new();
                }
                if success {
                    let m = self.match_index.entry(env.from.clone()).or_insert(0);
                    *m = (*m).max(match_index);
                    let m = *m;
                    self.next_index.insert(env.from.clone(), m + 1);
                    self.advance_commit();
                    // A commit advance may complete an IDR-003 phase: joint
                    // committed → append C_new; C_new committed without self
                    // → step down (RCR-020).
                    self.maybe_finalize_membership()
                } else {
                    // Follower catch-up: back the next index off by one and
                    // retry immediately (deterministic backtracking).
                    let n = self.next_index.entry(env.from.clone()).or_insert(1);
                    *n = (*n).saturating_sub(1).max(1);
                    vec![self.append_for(&env.from)]
                }
            }
            MsgBody::TimeoutNow { term } => {
                // Leadership transfer (RCR-020): campaign immediately, with
                // the transfer flag so voters bypass the §4.2.3 leadership
                // check. Stale transfers (term < ours) are ignored;
                // non-voters never campaign (start_election's guard).
                if term == self.current_term {
                    return self.start_election(true);
                }
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentHash, Outcome};

    fn nid(s: &str) -> NodeId {
        NodeId(s.into())
    }

    fn outcome(tag: &str) -> EntryKind {
        EntryKind::Outcome(Outcome {
            digest: ContentHash(format!("h:{tag}")),
            payload: tag.as_bytes().to_vec(),
        })
    }

    /// Randomized election timeouts are SEEDED: identical seed ⇒ identical
    /// draw sequence; the only randomness is the recorded seed (DR-3).
    #[test]
    fn seeded_timeouts_are_deterministic() {
        let mut a = DetRng::new(42);
        let mut b = DetRng::new(42);
        let da: Vec<u64> = (0..8).map(|_| a.range(ELECTION_TIMEOUT_MIN, ELECTION_TIMEOUT_MAX)).collect();
        let db: Vec<u64> = (0..8).map(|_| b.range(ELECTION_TIMEOUT_MIN, ELECTION_TIMEOUT_MAX)).collect();
        assert_eq!(da, db);
        assert!(da.iter().all(|d| (ELECTION_TIMEOUT_MIN..ELECTION_TIMEOUT_MAX).contains(d)));
        // Two nodes with the same seed draw the same initial deadline.
        let n1 = RaftNode::new(nid("a"), vec![nid("a")], 7);
        let n2 = RaftNode::new(nid("b"), vec![nid("b")], 7);
        assert_eq!(n1.election_deadline, n2.election_deadline);
    }

    /// A single-replica group (quorum = 1) elects itself and commits
    /// immediately on propose — the I1 degenerate shape, first index is 1 (DR-1).
    #[test]
    fn single_node_group_elects_self_and_commits_dense_from_one() {
        let mut n = RaftNode::new(nid("solo"), vec![nid("solo")], 1);
        let mut out = Vec::new();
        for _ in 0..ELECTION_TIMEOUT_MAX {
            out.extend(n.tick());
        }
        assert_eq!(n.role(), Role::Leader);
        assert!(out.is_empty(), "no peers, no messages");
        let (i1, _) = n.client_propose(outcome("e1")).unwrap();
        let (i2, _) = n.client_propose(outcome("e2")).unwrap();
        assert_eq!(i1, LogIndex(1));
        assert_eq!(i2, LogIndex(2));
        assert_eq!(n.commit_index(), LogIndex(2));
    }

    /// Stale-term AppendEntries is refused, and a reply carrying a higher term
    /// steps a stale leader down (IDR-004: a stale leader can never commit).
    #[test]
    fn stale_term_append_rejected_and_higher_term_steps_leader_down() {
        let members = vec![nid("a"), nid("b")];
        let mut b = RaftNode::new(nid("b"), members.clone(), 2);
        b.become_follower(Term(5));
        let stale = Envelope {
            from: nid("a"),
            to: nid("b"),
            body: MsgBody::AppendEntries {
                term: Term(3),
                prev_log_index: 0,
                prev_log_term: Term(0),
                entries: vec![],
                leader_commit: 0,
            },
        };
        let replies = b.step(stale);
        assert!(matches!(
            replies[0].body,
            MsgBody::AppendReply { term: Term(5), success: false, .. }
        ));
        // The stale leader consumes that reply and steps down.
        let mut a = RaftNode::new(nid("a"), members, 3);
        // Force a into a stale leadership at term 3.
        a.current_term = Term(3);
        a.role = Role::Leader;
        a.leader_hint = Some(nid("a"));
        let out = a.step(replies[0].clone());
        assert_eq!(a.role(), Role::Follower);
        assert_eq!(a.current_term(), Term(5));
        assert!(out.is_empty());
    }

    /// A follower never grants a vote to a candidate whose log is behind
    /// (§5.4.1 — the mechanism behind Leader Completeness).
    #[test]
    fn vote_refused_to_candidate_with_stale_log() {
        let members = vec![nid("a"), nid("b")];
        let mut b = RaftNode::new(nid("b"), members, 4);
        b.log.push(LogEntry { term: Term(2), index: LogIndex(1), kind: outcome("e1") });
        b.current_term = Term(2);
        let ask = Envelope {
            from: nid("a"),
            to: nid("b"),
            body: MsgBody::RequestVote {
                term: Term(3),
                last_log_index: 0,
                last_log_term: Term(0),
                transfer: false,
            },
        };
        let replies = b.step(ask);
        assert!(matches!(replies[0].body, MsgBody::VoteReply { granted: false, .. }));
    }

    /// RCR-020, thesis §4.2.3 leadership check: a replica in fresh contact
    /// with a current leader IGNORES a non-transfer RequestVote (no reply, no
    /// term bump) — a removed server campaigning forever cannot disrupt the
    /// group — while a transfer campaign bypasses the check.
    #[test]
    fn leadership_check_ignores_disruptive_vote_but_not_transfer() {
        let members = vec![nid("a"), nid("b"), nid("c")];
        let mut b = RaftNode::new(nid("b"), members, 21);
        // b hears from the current-term leader a.
        let heartbeat = Envelope {
            from: nid("a"),
            to: nid("b"),
            body: MsgBody::AppendEntries {
                term: Term(1),
                prev_log_index: 0,
                prev_log_term: Term(0),
                entries: vec![],
                leader_commit: 0,
            },
        };
        b.become_follower(Term(1));
        b.step(heartbeat);
        // Disruptive campaign from c at a HIGHER term: ignored entirely.
        let disruptive = Envelope {
            from: nid("c"),
            to: nid("b"),
            body: MsgBody::RequestVote {
                term: Term(9),
                last_log_index: 0,
                last_log_term: Term(0),
                transfer: false,
            },
        };
        assert!(b.step(disruptive).is_empty(), "no reply to a disruptive campaign");
        assert_eq!(b.current_term(), Term(1), "no term inflation from a disruptive campaign");
        // The SAME request flagged as a leadership transfer is processed.
        let handover = Envelope {
            from: nid("c"),
            to: nid("b"),
            body: MsgBody::RequestVote {
                term: Term(9),
                last_log_index: 0,
                last_log_term: Term(0),
                transfer: true,
            },
        };
        let replies = b.step(handover);
        assert!(matches!(replies[0].body, MsgBody::VoteReply { granted: true, .. }));
        assert_eq!(b.current_term(), Term(9));
    }

    /// RCR-020 DR-2: the joint discipline is not bypassable — a raw
    /// Membership proposal through the client path is refused; the only
    /// reconfiguration door is `change_membership` (IDR-003).
    #[test]
    fn client_propose_refuses_raw_membership_entry() {
        let mut n = RaftNode::new(nid("solo"), vec![nid("solo")], 9);
        for _ in 0..ELECTION_TIMEOUT_MAX {
            n.tick();
        }
        assert_eq!(n.role(), Role::Leader);
        let err = n
            .client_propose(EntryKind::Membership(Membership::Stable {
                voters: vec![nid("solo"), nid("x")],
                learners: vec![],
            }))
            .unwrap_err();
        assert_eq!(err, ConsensusError::MembershipRejected);
    }

    /// RCR-020: a joining replica knows no voter configuration — it never
    /// campaigns, never advances its term, and emits nothing on its own.
    #[test]
    fn joining_node_never_campaigns_without_config() {
        let mut j = RaftNode::new_joining(nid("j"), 11);
        let mut out = Vec::new();
        for _ in 0..10 * ELECTION_TIMEOUT_MAX {
            out.extend(j.tick());
        }
        assert_eq!(j.role(), Role::Follower);
        assert_eq!(j.current_term(), Term(0));
        assert!(out.is_empty(), "a config-less joiner must stay silent");
        assert!(!j.is_voter());
    }

    /// RCR-020 DR-6: leadership transfer refuses a non-leader source
    /// (NotLeader) and a target outside the voter configuration
    /// (MembershipRejected, documented reuse).
    #[test]
    fn transfer_rejects_non_leader_source_and_non_voter_target() {
        let members = vec![nid("a"), nid("b")];
        let mut f = RaftNode::new(nid("a"), members.clone(), 12);
        assert!(matches!(
            f.transfer_leadership(&nid("b")),
            Err(ConsensusError::NotLeader { .. })
        ));
        let mut solo = RaftNode::new(nid("s"), vec![nid("s")], 13);
        for _ in 0..ELECTION_TIMEOUT_MAX {
            solo.tick();
        }
        assert_eq!(solo.role(), Role::Leader);
        assert_eq!(
            solo.transfer_leadership(&nid("stranger")).unwrap_err(),
            ConsensusError::MembershipRejected
        );
        assert_eq!(
            solo.transfer_leadership(&nid("s")).unwrap_err(),
            ConsensusError::MembershipRejected,
            "self-transfer is meaningless and refused"
        );
    }
}

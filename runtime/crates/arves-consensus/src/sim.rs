//! ARVES :: arves-consensus :: sim — deterministic in-process MessageBus harness.
//!
//! I2 Stage 1 (RCR-019). The design (§3.7) requires the Raft core to be "a
//! deterministic function of (messages, timers-as-events)" exercised by
//! "simulation tests with scripted message schedules" (§3.21) — this module is
//! that vehicle. It is a TEST/SIMULATION harness, not a network: message drops
//! and partitions are expressed as **bus filters**, delivery is FIFO and
//! deterministic, and time is the injected logical tick. HONEST SCOPE: nothing
//! here provides or claims network transport or network fault-tolerance.
//!
//! The harness continuously checks the four Raft safety properties as the
//! cluster runs (any violation panics loudly with the property name):
//!
//! 1. **Election Safety** — at most one leader per term (IDR-004).
//! 2. **Leader Completeness** — an entry committed under term T appears in the
//!    log of every leader of a term > T.
//! 3. **State Machine Safety** — no two replicas ever commit different entries
//!    at the same index (the committed prefix is the applied state in Stage 1;
//!    Kernel apply wiring is a later I2 stage).
//! 4. **Log Matching** — if two logs share (index, term), the logs are
//!    identical through that index.
//!
//! (Leader Append-Only holds by construction: the only truncation path in the
//! core is the follower conflict branch of `AppendEntries`.)
//!
//! [`SimShardConsensus`] is the first implementation of the frozen
//! [`ShardConsensus`] contract: one handle is bound to one local replica of
//! one shard group (client-to-replica semantics). Stage 2 (RCR-020):
//! `change_membership` now BEGINS a real joint-consensus transition (IDR-003)
//! and [`SimShardMap`] provides the per-shard consensus instance map — one
//! independent [`SimCluster`] Raft group per immutable `ShardId` (IDR-001,
//! SHARD-001), dispatched through the same frozen contract. Still stage-scoped
//! honestly: `read_index` maps Linearizable to the leader's commit index,
//! weaker tiers to the local commit index (real tiers are I2.9).
//! Stage 4 (RCR-022): [`SimCluster::set_mangling`] adds deterministic
//! duplicate/reordered delivery (counter-scripted, no randomness) so the
//! adversarial distributed proofs can storm the bus while the four safety
//! properties stay checked after every step (ORCH-004 at cluster level).

use crate::raft::{Envelope, MsgBody, RaftNode};
use crate::{
    ConsensusError, ConsensusResult, EntryKind, Leadership, LogEntry, LogIndex, Membership,
    NodeId, Outcome, ReadTier, Role, ShardConsensus, ShardId,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

/// Deliver-or-drop predicate: `true` = deliver. Partitions/drops are filters.
pub type BusFilter = Box<dyn Fn(&Envelope) -> bool>;

/// Bound on the ticks [`SimShardConsensus::await_commit`] will drive before
/// honestly reporting `QuorumUnavailable` (CP posture: unavailable > divergent).
pub const AWAIT_COMMIT_MAX_TICKS: u64 = 200;

/// Bound on the catch-up rounds [`SimCluster::transfer_leadership`] will drive
/// before honestly reporting `QuorumUnavailable` (deterministic budget).
pub const TRANSFER_MAX_ROUNDS: u64 = 64;

/// A deterministic, in-process cluster of [`RaftNode`]s for ONE shard group,
/// with a FIFO message bus, filter-expressed faults, and continuous safety
/// checking. Identical seed + identical scripted schedule ⇒ identical history.
pub struct SimCluster {
    nodes: BTreeMap<NodeId, RaftNode>,
    queue: VecDeque<Envelope>,
    filters: Vec<BusFilter>,
    /// Deterministic seed source for later-joining replicas (RCR-020): drawing
    /// from the same recorded budget keeps add-node runs replayable.
    seeder: crate::raft::DetRng,
    /// Messages dropped by filters (observability of the scripted faults).
    pub dropped: u64,
    /// Adversarial delivery mangling (RCR-022, I2 Stage 4): every
    /// `dup_every`-th delivered envelope is re-enqueued at the BACK after
    /// delivery (a stale duplicate that arrives AFTER later traffic —
    /// duplicate + reordered in one), and every `defer_every`-th popped
    /// envelope is pushed to the back INSTEAD of being delivered (pure
    /// reordering). `0` disables. Counter-scripted, not random: identical
    /// schedules stay identical (determinism over convenience).
    dup_every: u64,
    defer_every: u64,
    /// Position counter driving the mangling schedule (increments per pop).
    mangle_pops: u64,
    /// Duplicates injected / deliveries deferred (observability + digest).
    pub duplicated: u64,
    pub deferred: u64,
    /// term -> the (asserted ≤1) leaders observed in that term.
    leaders_by_term: BTreeMap<u64, BTreeSet<NodeId>>,
    /// index -> (first committed entry, the ENTRY'S OWN term — see the
    /// Leader-Completeness note in [`Self::observe_all`], RCR-022 revision).
    committed: BTreeMap<u64, (LogEntry, u64)>,
}

impl SimCluster {
    /// Build `n` replicas `n1..nN` of one shard group. `seed` is the entire
    /// randomness budget: each node's timeout seed derives from it.
    pub fn new(n: usize, seed: u64) -> Self {
        let ids: Vec<NodeId> = (1..=n).map(|i| NodeId(format!("n{i}"))).collect();
        let mut seeder = crate::raft::DetRng::new(seed);
        let nodes = ids
            .iter()
            .map(|id| (id.clone(), RaftNode::new(id.clone(), ids.clone(), seeder.next_u64())))
            .collect();
        Self {
            nodes,
            queue: VecDeque::new(),
            filters: Vec::new(),
            seeder,
            dropped: 0,
            dup_every: 0,
            defer_every: 0,
            mangle_pops: 0,
            duplicated: 0,
            deferred: 0,
            leaders_by_term: BTreeMap::new(),
            committed: BTreeMap::new(),
        }
    }

    /// Enable deterministic adversarial delivery mangling (RCR-022, I2 Stage
    /// 4): every `dup_every`-th delivered envelope is ALSO re-enqueued at the
    /// back of the queue (a stale duplicate delivered after later traffic —
    /// duplication AND reordering relative to everything behind it), and every
    /// `defer_every`-th popped envelope is pushed to the back instead of being
    /// delivered (reordering). `0` disables an arm. The schedule is a plain
    /// position counter — no randomness — so identically-scripted runs stay
    /// byte-identical (ORCH-003 replayability of the harness itself).
    ///
    /// # Panics
    /// If either arm is `1` (mangling EVERY message would defer forever /
    /// duplicate unboundedly — a harness programming error, refused loudly).
    pub fn set_mangling(&mut self, dup_every: u64, defer_every: u64) {
        assert!(dup_every != 1 && defer_every != 1, "every-1 mangling would never terminate");
        self.dup_every = dup_every;
        self.defer_every = defer_every;
    }

    /// Disable delivery mangling (counters are retained for observability).
    pub fn clear_mangling(&mut self) {
        self.dup_every = 0;
        self.defer_every = 0;
    }

    /// Add a JOINING replica to the simulation (RCR-020). It participates in
    /// nothing until a joint configuration naming it is appended via
    /// [`SimCluster::change_membership`] (IDR-003); its timeout seed comes
    /// from the cluster's recorded seed budget (replayable).
    pub fn add_node(&mut self, id: NodeId) {
        assert!(
            !self.nodes.contains_key(&id),
            "node {id:?} already exists in the simulation"
        );
        let seed = self.seeder.next_u64();
        self.nodes.insert(id.clone(), RaftNode::new_joining(id, seed));
    }

    /// All node ids, in deterministic order.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().cloned().collect()
    }

    /// Read-only view of one replica.
    pub fn node(&self, id: &NodeId) -> &RaftNode {
        &self.nodes[id]
    }

    // -- scripted faults (drops/partitions as bus filters) -------------------

    /// Install a raw filter (deliver iff ALL filters return true).
    pub fn push_filter(&mut self, f: BusFilter) {
        self.filters.push(f);
    }

    /// Heal every fault: remove all filters.
    pub fn heal(&mut self) {
        self.filters.clear();
    }

    /// Partition the cluster into disjoint groups; messages cross a group
    /// boundary are dropped. Replaces existing filters.
    pub fn partition(&mut self, groups: &[Vec<NodeId>]) {
        let mut side: BTreeMap<NodeId, usize> = BTreeMap::new();
        for (g, members) in groups.iter().enumerate() {
            for m in members {
                side.insert(m.clone(), g);
            }
        }
        self.filters.clear();
        self.push_filter(Box::new(move |e: &Envelope| {
            match (side.get(&e.from), side.get(&e.to)) {
                (Some(a), Some(b)) => a == b,
                _ => false, // unlisted node = isolated
            }
        }));
    }

    /// Isolate one replica from everyone (both directions).
    pub fn isolate(&mut self, id: &NodeId) {
        let rest: Vec<NodeId> = self.node_ids().into_iter().filter(|n| n != id).collect();
        self.partition(&[vec![id.clone()], rest]);
    }

    // -- deterministic execution ---------------------------------------------

    /// One logical tick on every node (deterministic id order), then drain the
    /// bus to quiescence. Safety properties are checked after every step.
    pub fn tick(&mut self) {
        let ids = self.node_ids();
        for id in &ids {
            let out = self.nodes.get_mut(id).expect("node").tick();
            self.queue.extend(out);
        }
        self.observe_all();
        self.drain();
    }

    /// Run `n` ticks.
    pub fn run(&mut self, n: u64) {
        for _ in 0..n {
            self.tick();
        }
    }

    /// Run ticks until exactly one live leader exists and the bus is quiet;
    /// panics after `max_ticks` (deterministic — a fixed budget, not a wall
    /// clock). Returns the leader id.
    pub fn run_until_leader(&mut self, max_ticks: u64) -> NodeId {
        for _ in 0..max_ticks {
            self.tick();
            if let Some((id, _)) = self.current_leader() {
                if self.queue.is_empty() {
                    return id;
                }
            }
        }
        panic!("no leader within {max_ticks} ticks");
    }

    /// Deliver queued messages until the bus is empty, applying filters and
    /// (when enabled) the deterministic duplicate/defer mangling schedule
    /// (RCR-022). A fixed pop budget guards against a non-terminating mangle
    /// schedule — exceeded loudly, never silently.
    fn drain(&mut self) {
        const DRAIN_POP_BUDGET: u64 = 1_000_000;
        let mut pops: u64 = 0;
        while let Some(env) = self.queue.pop_front() {
            pops += 1;
            assert!(pops <= DRAIN_POP_BUDGET, "drain exceeded its deterministic pop budget");
            self.mangle_pops += 1;
            // Reorder arm: push to the back INSTEAD of delivering. The counter
            // advances per pop, so a lone message cannot be deferred twice in a
            // row (defer_every >= 2) — the drain always terminates.
            if self.defer_every != 0 && self.mangle_pops % self.defer_every == 0 {
                self.deferred += 1;
                self.queue.push_back(env);
                continue;
            }
            if !self.filters.iter().all(|f| f(&env)) {
                self.dropped += 1;
                continue;
            }
            // Duplicate arm: deliver now AND re-enqueue a stale copy at the
            // back — it will arrive after later traffic (dup + reorder).
            if self.dup_every != 0 && self.mangle_pops % self.dup_every == 0 {
                self.duplicated += 1;
                self.queue.push_back(env.clone());
            }
            let out = match self.nodes.get_mut(&env.to) {
                Some(n) => n.step(env),
                None => Vec::new(),
            };
            self.queue.extend(out);
            self.observe_all();
        }
    }

    /// The live leader with the highest term, if any. (During partitions a
    /// deposed leader may transiently still believe it leads a LOWER term —
    /// that is legal Raft; Election Safety is per-term and checked separately.)
    pub fn current_leader(&self) -> Option<(NodeId, u64)> {
        self.nodes
            .values()
            .filter(|n| n.role() == Role::Leader)
            .map(|n| (n.id().clone(), n.current_term().0))
            .max_by_key(|(_, t)| *t)
    }

    /// Propose an entry at a specific replica (leader-only; non-leaders refuse
    /// with `NotLeader` — OWN-001). Returns the appended index; commit is a
    /// separate quorum event.
    pub fn propose(&mut self, at: &NodeId, kind: EntryKind) -> ConsensusResult<LogIndex> {
        let node = self.nodes.get_mut(at).expect("propose target must be a cluster member");
        let (index, msgs) = node.client_propose(kind)?;
        self.queue.extend(msgs);
        self.observe_all();
        self.drain();
        Ok(index)
    }

    /// Begin a joint-consensus membership change at replica `at` (RCR-020).
    /// Leader-only; refusals surface the frozen errors (`NotLeader`,
    /// `MembershipRejected`). Returns the index of the JOINT entry; the
    /// C_old,new → C_new completion is driven by subsequent message flow
    /// (deterministic; blocked honestly while quorums are unreachable).
    pub fn change_membership(
        &mut self,
        at: &NodeId,
        target: Membership,
    ) -> ConsensusResult<LogIndex> {
        let node = self.nodes.get_mut(at).expect("membership target must be a cluster member");
        let (index, msgs) = node.change_membership(target)?;
        self.queue.extend(msgs);
        self.observe_all();
        self.drain();
        Ok(index)
    }

    /// Leadership transfer `from` → `to` (RCR-020): drives deterministic
    /// catch-up rounds until the target is up to date, then delivers
    /// `TimeoutNow`. Fails honestly with `QuorumUnavailable` if the target
    /// cannot be caught up within [`TRANSFER_MAX_ROUNDS`] (e.g. partitioned).
    pub fn transfer_leadership(&mut self, from: &NodeId, to: &NodeId) -> ConsensusResult<()> {
        for _ in 0..TRANSFER_MAX_ROUNDS {
            let msgs = self
                .nodes
                .get_mut(from)
                .expect("transfer source must be a cluster member")
                .transfer_leadership(to)?;
            let fired = msgs.iter().any(|m| matches!(m.body, MsgBody::TimeoutNow { .. }));
            self.queue.extend(msgs);
            self.observe_all();
            self.drain();
            if fired {
                return Ok(());
            }
        }
        Err(ConsensusError::QuorumUnavailable)
    }

    /// Commit index of one replica.
    pub fn commit_of(&self, id: &NodeId) -> u64 {
        self.nodes[id].commit_index().0
    }

    /// Log of one replica.
    pub fn log_of(&self, id: &NodeId) -> &[LogEntry] {
        self.nodes[id].log()
    }

    /// Read-index PRECONDITION input (Raft §6.4 / §8, RCR-024 revision): does
    /// `id`'s replica have a COMMITTED entry of its CURRENT term? Until a
    /// freshly elected leader commits one entry of its own term, its commit
    /// index may EXCLUDE prior-term quorum-committed entries — the §5.4.2
    /// term guard in `advance_commit` refuses to count them, and RCR-019
    /// DR-2 deliberately appends NO no-op entry on election — so that commit
    /// index is NOT yet a valid read-index. The empty log is trivially
    /// current: nothing was ever proposed, so commit index 0 covers all
    /// committed truth (the Raft election restriction guarantees a leader's
    /// log contains every committed entry). Read-only introspection.
    pub fn has_committed_in_current_term(&self, id: &NodeId) -> bool {
        let node = &self.nodes[id];
        if node.log().is_empty() {
            return true;
        }
        let committed = node.commit_index().0;
        committed > 0 && node.log()[committed as usize - 1].term == node.current_term()
    }

    /// The safety-observer's committed history: index -> first committed entry.
    pub fn committed_history(&self) -> BTreeMap<u64, LogEntry> {
        self.committed.iter().map(|(i, (e, _))| (*i, e.clone())).collect()
    }

    /// The safety-observer's leader history: term -> leaders observed.
    pub fn leaders_history(&self) -> &BTreeMap<u64, BTreeSet<NodeId>> {
        &self.leaders_by_term
    }

    /// Deterministic digest of the full cluster state + safety history —
    /// two identically-seeded, identically-scripted runs must produce equal
    /// digests (replayability proof).
    pub fn digest(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
        let mut eat = |bytes: &[u8]| {
            for b in bytes {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        for (id, n) in &self.nodes {
            eat(id.0.as_bytes());
            eat(&n.current_term().0.to_le_bytes());
            eat(&n.commit_index().0.to_le_bytes());
            eat(&[match n.role() {
                Role::Leader => 1,
                Role::Follower => 2,
                Role::Candidate => 3,
                Role::Learner => 4,
            }]);
            for e in n.log() {
                eat(&e.term.0.to_le_bytes());
                eat(&e.index.0.to_le_bytes());
                match &e.kind {
                    EntryKind::Outcome(o) => {
                        eat(&[1]);
                        eat(o.digest.0.as_bytes());
                        eat(&o.payload);
                    }
                    // RCR-020: membership entries are part of the replayable
                    // history too (the cluster's own reconfiguration trace,
                    // design §3.19).
                    EntryKind::Membership(Membership::Stable { voters, learners }) => {
                        eat(&[2]);
                        for v in voters {
                            eat(v.0.as_bytes());
                        }
                        eat(&[3]);
                        for l in learners {
                            eat(l.0.as_bytes());
                        }
                    }
                    EntryKind::Membership(Membership::Joint {
                        old_voters,
                        new_voters,
                        learners,
                    }) => {
                        eat(&[4]);
                        for v in old_voters {
                            eat(v.0.as_bytes());
                        }
                        eat(&[5]);
                        for v in new_voters {
                            eat(v.0.as_bytes());
                        }
                        eat(&[6]);
                        for l in learners {
                            eat(l.0.as_bytes());
                        }
                    }
                }
            }
        }
        eat(&self.dropped.to_le_bytes());
        // RCR-022: the mangling trace is part of the replayable history too —
        // identically-scripted adversarial runs must mangle identically.
        eat(&self.duplicated.to_le_bytes());
        eat(&self.deferred.to_le_bytes());
        h
    }

    // -- continuous safety checking -------------------------------------------

    fn observe_all(&mut self) {
        // Election Safety + Leader Completeness.
        let mut newly_seen: Vec<(u64, NodeId)> = Vec::new();
        for (id, n) in &self.nodes {
            if n.role() == Role::Leader {
                let term = n.current_term().0;
                let set = self.leaders_by_term.entry(term).or_default();
                if set.insert(id.clone()) {
                    newly_seen.push((term, id.clone()));
                }
                assert!(
                    set.len() <= 1,
                    "SAFETY VIOLATION (Election Safety): term {term} has leaders {set:?}"
                );
            }
        }
        for (term, id) in newly_seen {
            let leader_log = self.nodes[&id].log();
            for (idx, (entry, c_term)) in &self.committed {
                if *c_term < term {
                    assert!(
                        leader_log.get(*idx as usize - 1) == Some(entry),
                        "SAFETY VIOLATION (Leader Completeness): leader {id:?} of term {term} \
                         lacks entry committed at index {idx} (entry term {c_term})"
                    );
                }
            }
        }
        // State Machine Safety over every replica's committed prefix.
        //
        // RCR-022 revision: the term recorded alongside a committed entry is the
        // ENTRY'S OWN term (`entry.term`), not the observing node's current term.
        // The observer's term can run ahead of the true commit term, which made
        // the `c_term < leader_term` gate in the Leader Completeness check skip
        // legitimate comparisons. Recording `entry.term` is sound and strictly
        // stronger: any leader FIRST observed while the entry already sits in
        // `committed` was necessarily elected after the commit (a candidate of a
        // term ≤ the commit term cannot win a majority once the committing
        // majority has moved past it), so by Leader Completeness it must contain
        // every such entry.
        for n in self.nodes.values() {
            for i in 1..=n.commit_index().0 {
                let entry = n.log()[i as usize - 1].clone();
                let entry_term = entry.term.0;
                match self.committed.entry(i) {
                    std::collections::btree_map::Entry::Vacant(v) => {
                        v.insert((entry, entry_term));
                    }
                    std::collections::btree_map::Entry::Occupied(o) => assert!(
                        o.get().0 == entry,
                        "SAFETY VIOLATION (State Machine Safety): index {i} committed twice \
                         with different entries: {:?} vs {entry:?}",
                        o.get().0
                    ),
                }
            }
        }
        // Log Matching, pairwise.
        let ids = self.node_ids();
        for a in 0..ids.len() {
            for b in a + 1..ids.len() {
                let (la, lb) = (self.nodes[&ids[a]].log(), self.nodes[&ids[b]].log());
                let min = la.len().min(lb.len());
                for i in (0..min).rev() {
                    if la[i].term == lb[i].term {
                        assert!(
                            la[..=i] == lb[..=i],
                            "SAFETY VIOLATION (Log Matching): {:?} and {:?} share (index {}, \
                             term {:?}) but prefixes differ",
                            ids[a],
                            ids[b],
                            i + 1,
                            la[i].term
                        );
                        break;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// First implementation of the frozen ShardConsensus contract
// ---------------------------------------------------------------------------

/// A [`ShardConsensus`] handle bound to ONE local replica of ONE simulated
/// shard group — client-to-replica semantics: proposals sent through a
/// follower's handle are refused with `NotLeader` (OWN-001; IDR-004).
///
/// Single-threaded test vehicle by design (`Rc<RefCell<..>>`): the Stage-1
/// core is exercised in-process only. Thread-safe/transport-backed
/// implementations are later I2 stages.
pub struct SimShardConsensus {
    shard: ShardId,
    local: NodeId,
    cluster: Rc<RefCell<SimCluster>>,
}

impl SimShardConsensus {
    /// Bind a handle to `local` within `cluster`, serving exactly `shard`
    /// (IDR-001: one group per shard).
    pub fn new(shard: ShardId, local: NodeId, cluster: Rc<RefCell<SimCluster>>) -> Self {
        Self { shard, local, cluster }
    }

    fn check_shard(&self, shard: &ShardId) -> ConsensusResult<()> {
        if *shard != self.shard {
            return Err(ConsensusError::UnknownShard(shard.clone()));
        }
        Ok(())
    }
}

impl ShardConsensus for SimShardConsensus {
    fn propose(&self, shard: &ShardId, outcome: Outcome) -> ConsensusResult<LogIndex> {
        self.check_shard(shard)?;
        self.cluster.borrow_mut().propose(&self.local, EntryKind::Outcome(outcome))
    }

    /// Wait (by driving deterministic ticks) until the local replica's commit
    /// index reaches `index`, returning the committed entry at that index.
    ///
    /// # Identity caveat — `Ok` does NOT mean "my proposal committed"
    ///
    /// The frozen contract is index-only. After a failover, a deposed leader's
    /// un-replicated entry at `index` can be superseded, and a DIFFERENT entry
    /// committed by the new leader can legally occupy that index — this method
    /// then returns `Ok` with that foreign entry (legal Raft, legal under the
    /// frozen signature; safety holds, no replica diverges). Callers MUST
    /// compare the returned [`LogEntry`]'s `Outcome` digest against their own
    /// proposal before treating the commit as theirs. The Kernel commit-path
    /// wiring RCR must perform exactly that check (RCR-019 DR-8).
    fn await_commit(&self, shard: &ShardId, index: LogIndex) -> ConsensusResult<LogEntry> {
        self.check_shard(shard)?;
        let mut c = self.cluster.borrow_mut();
        for _ in 0..AWAIT_COMMIT_MAX_TICKS {
            if c.commit_of(&self.local) >= index.0 && index.0 >= 1 {
                return Ok(c.log_of(&self.local)[index.0 as usize - 1].clone());
            }
            c.tick();
        }
        // CP posture (IDR-001): honestly unavailable, never divergent.
        Err(ConsensusError::QuorumUnavailable)
    }

    fn leader(&self, shard: &ShardId) -> ConsensusResult<Leadership> {
        self.check_shard(shard)?;
        Ok(self.cluster.borrow().node(&self.local).leadership())
    }

    fn role(&self, shard: &ShardId) -> ConsensusResult<Role> {
        self.check_shard(shard)?;
        Ok(self.cluster.borrow().node(&self.local).role())
    }

    fn read_index(&self, shard: &ShardId, tier: ReadTier) -> ConsensusResult<LogIndex> {
        self.check_shard(shard)?;
        let c = self.cluster.borrow();
        let n = c.node(&self.local);
        match tier {
            ReadTier::Linearizable => {
                // Stage-1 honest mapping: leader-only; real read-index
                // protocol (or leases, OQ-6) is ladder step I2.9.
                if n.role() != Role::Leader {
                    return Err(ConsensusError::NotLeader { leader: n.leadership() });
                }
                Ok(n.commit_index())
            }
            ReadTier::BoundedStaleness | ReadTier::Eventual => Ok(n.commit_index()),
        }
    }

    /// Begin a joint-consensus membership change (IDR-003; Stage 2, RCR-020):
    /// the caller names the target `Membership::Stable`; the shard's leader
    /// appends `C_old,new`, replicates it under the dual-majority overlap
    /// rule, then auto-appends `C_new` once the joint entry commits. Returns
    /// the JOINT entry's index (this "begins" the change, per the frozen
    /// trait doc — completion is observable via `await_commit` and
    /// subsequent proposals).
    fn change_membership(&self, shard: &ShardId, target: Membership) -> ConsensusResult<LogIndex> {
        self.check_shard(shard)?;
        self.cluster.borrow_mut().change_membership(&self.local, target)
    }
}

// ---------------------------------------------------------------------------
// Per-shard consensus instance map (IDR-001: one Raft group per shard)
// ---------------------------------------------------------------------------

/// The per-shard consensus instance map (Stage 2, RCR-020): each immutable
/// [`ShardId`] owns exactly ONE independent [`SimCluster`] Raft group with its
/// own leader election, log, and faults (IDR-001; SHARD-001 shared-nothing —
/// one shard's quorum loss cannot touch another shard, design §3.13).
///
/// Implements the frozen [`ShardConsensus`] contract by dispatching every call
/// to the group registered for the named shard; unregistered shards are
/// refused with [`ConsensusError::UnknownShard`]. Duplicate registration
/// panics loudly: IDR-001 fixes exactly one group per shard.
pub struct SimShardMap {
    groups: BTreeMap<ShardId, SimShardConsensus>,
}

impl SimShardMap {
    /// An empty map: no shards, no groups.
    pub fn new() -> Self {
        Self { groups: BTreeMap::new() }
    }

    /// Register the (single) group serving `shard`, with this handle bound to
    /// local replica `local` (client-to-replica semantics, as in
    /// [`SimShardConsensus`]).
    ///
    /// # Panics
    /// On a duplicate shard: exactly one Raft group exists per shard
    /// (IDR-001); a second registration is a harness programming error.
    pub fn register(&mut self, shard: ShardId, local: NodeId, cluster: Rc<RefCell<SimCluster>>) {
        assert!(
            !self.groups.contains_key(&shard),
            "IDR-001 violation: shard {shard:?} already has its Raft group (exactly one per shard)"
        );
        self.groups.insert(shard.clone(), SimShardConsensus::new(shard, local, cluster));
    }

    /// The registered shards, in deterministic order.
    pub fn shards(&self) -> Vec<ShardId> {
        self.groups.keys().cloned().collect()
    }

    fn group(&self, shard: &ShardId) -> ConsensusResult<&SimShardConsensus> {
        self.groups.get(shard).ok_or_else(|| ConsensusError::UnknownShard(shard.clone()))
    }
}

impl Default for SimShardMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ShardConsensus for SimShardMap {
    fn propose(&self, shard: &ShardId, outcome: Outcome) -> ConsensusResult<LogIndex> {
        self.group(shard)?.propose(shard, outcome)
    }

    fn await_commit(&self, shard: &ShardId, index: LogIndex) -> ConsensusResult<LogEntry> {
        self.group(shard)?.await_commit(shard, index)
    }

    fn leader(&self, shard: &ShardId) -> ConsensusResult<Leadership> {
        self.group(shard)?.leader(shard)
    }

    fn role(&self, shard: &ShardId) -> ConsensusResult<Role> {
        self.group(shard)?.role(shard)
    }

    fn read_index(&self, shard: &ShardId, tier: ReadTier) -> ConsensusResult<LogIndex> {
        self.group(shard)?.read_index(shard, tier)
    }

    fn change_membership(&self, shard: &ShardId, target: Membership) -> ConsensusResult<LogIndex> {
        self.group(shard)?.change_membership(shard, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContentHash;

    fn outcome(tag: &str) -> EntryKind {
        EntryKind::Outcome(Outcome {
            digest: ContentHash(format!("h:{tag}")),
            payload: tag.as_bytes().to_vec(),
        })
    }

    /// Three replicas elect exactly one leader per term (IDR-004) under the
    /// deterministic bus with no faults.
    #[test]
    fn three_nodes_elect_exactly_one_leader() {
        let mut c = SimCluster::new(3, 0xA11CE);
        let leader = c.run_until_leader(200);
        assert_eq!(c.node(&leader).role(), Role::Leader);
        for (term, set) in c.leaders_history() {
            assert!(set.len() <= 1, "term {term} had {set:?}");
        }
    }

    /// Propose at the leader → quorum ack → commit advances → every replica
    /// converges to the identical log and commit index (IDR-002 apply flow).
    #[test]
    fn replicated_commit_advances_on_quorum_and_replicas_converge() {
        let mut c = SimCluster::new(3, 7);
        let leader = c.run_until_leader(200);
        let i1 = c.propose(&leader, outcome("e1")).unwrap();
        let i2 = c.propose(&leader, outcome("e2")).unwrap();
        assert_eq!((i1, i2), (LogIndex(1), LogIndex(2)));
        c.run(4); // heartbeats carry leader_commit to followers
        let ids = c.node_ids();
        for id in &ids {
            assert_eq!(c.commit_of(id), 2, "replica {id:?} commit");
            assert_eq!(c.log_of(id), c.log_of(&leader), "replica {id:?} log");
        }
    }

    /// Identical seed + identical scripted schedule (including a partition and
    /// heal) ⇒ identical full history digest. Determinism over convenience.
    #[test]
    fn determinism_identical_seed_identical_history() {
        let run = |seed: u64| {
            let mut c = SimCluster::new(5, seed);
            let leader = c.run_until_leader(300);
            c.propose(&leader, outcome("e1")).unwrap();
            c.isolate(&leader);
            c.run(60);
            c.heal();
            c.run(60);
            c.digest()
        };
        assert_eq!(run(1234), run(1234));
        assert_ne!(run(1234), run(4321), "different seeds explore different histories");
    }
}

//! ARVES :: arves-kernel :: cluster — the CLUSTER KERNEL (I2 Stage 3, RCR-021).
//!
//! Wires the frozen [`Kernel`] commit gateway over the per-shard Raft substrate
//! (`arves-consensus`, RCR-019/020) per `docs/design/I2_Cluster_Kernel_Design.md`
//! §3.1: a `ProposedWrite` is accepted ONLY at the shard leader
//! ([`CommitError::NotLeader`] everywhere else — OWN-001/IDR-004); the leader
//! validates + content-addresses + dedupes through the SAME gateway logic as
//! [`RefKernel`] (the shared `admission`/`commit_inner` head — ORCH-004
//! idempotency, RCR-005 content-integrity; never forked); the already-decided
//! OUTCOME (never an invocation — IDR-002) is proposed to the shard's Raft
//! group; the caller is acked only after quorum commit; and committed entries
//! are applied in log order on EVERY replica, so follower truth is IDENTICAL —
//! same `ContentHash`es, same `CommitIndex`es, same per-shard state-blob bytes
//! (deterministic replay across nodes, ORCH-003).
//!
//! Replication order (IDR-002/IDR-005): leader raft log → follower raft logs →
//! Kernel snapshot install for a far-behind replica ([`ClusterSim::install_snapshot`])
//! → per-replica durable WAL apply. Lost quorum surfaces as
//! [`CommitError::NotReplicated`] (IDR-001 CP posture: unavailable > divergent).
//!
//! # HONEST SCOPE — what is simulated vs real
//!
//! - **In-process simulation only.** The Raft groups are the deterministic
//!   RCR-019 [`SimCluster`]s: transport is a FIFO bus, faults are scripted bus
//!   filters, time is the injected logical tick. NO network exists and NO
//!   network fault-tolerance is claimed (design §3.7: sockets arrive at the
//!   step where they are the property under test).
//! - **Real:** the leader-only commit gateway over real quorum replication, the
//!   follower apply loop producing byte-identical per-shard truth state, the
//!   ORCH-004/RCR-005 gateway semantics preserved under replication, the
//!   Kernel-level snapshot install + log-tail catch-up, and the CP refusal
//!   behaviours (`NotLeader`, `NotReplicated`). Snapshot honesty: the install
//!   MECHANICS are real, but the transfer is ORCHESTRATED BY THE HARNESS —
//!   tests invoke [`ClusterSim::install_snapshot`] explicitly; there is no
//!   leader-initiated InstallSnapshot step in the consensus protocol yet
//!   (protocol-driven snapshotting, with raft-log compaction, is OQ-1 / a
//!   later stage).
//! - **IDR-005 unification is still deferred** (RCR-019 DR-4): the Raft log
//!   lives in memory inside the consensus core; each replica's durable WAL
//!   receives the applied outcomes through the shared `commit_inner`. Exactly
//!   one durable artifact exists per replica (its WAL); making the Raft log
//!   itself the durable WAL is the persistence-wiring stage.
//! - **Raft state (log/term/vote) is not crash-durable in this stage**: the
//!   crash model here ([`ClusterSim::crash_recover`]) loses and recovers KERNEL
//!   truth from the local durable WAL (the proven I1.7 lossless-or-loud path);
//!   raft-state durability arrives with the IDR-005 unification stage.
//! - Read tiers (I2.9), wire format (OQ-3), raft-log compaction (OQ-1) and the
//!   cluster form of the RCR-013 batch (a Raft entry carrying a batch — see
//!   RCR-021 design resolutions) remain out of scope.
//!
//! # Ownership (OWN-001 / ORCH-001)
//!
//! The Kernel stays the sole owner of truth: consensus moves opaque outcome
//! bytes and decides ordering, never meaning. Replica kernels are PRIVATE to
//! [`ClusterSim`] — the only write door is [`ClusterKernel::commit`] on the
//! shard leader; introspection accessors are read-only. Membership entries in
//! the replicated log are consensus MECHANISM state (design §3.6 kind 2) and
//! never enter the Kernel.

use crate::{
    decode_shard_blob, CommitError, ContentHash, Kernel, MemKernel, ProposedWrite, RefKernel,
    ShardKey, TruthRef,
};
use arves_consensus::sim::SimCluster;
use arves_consensus::{
    ConsensusError, ContentHash as ConsensusDigest, EntryKind, NodeId, Outcome, Role, ShardId,
    TenantId, WorkspaceId,
};
use arves_persistence::{
    ContentId, MemWalStore, PendingRecord, RecordKind, ShardKey as PShardKey, Wal, WalStore,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// Bound on the logical ticks one [`ClusterKernel::commit`] drives while waiting
/// for quorum before honestly reporting [`CommitError::NotReplicated`]
/// (deterministic budget, not a wall clock — IDR-001 CP posture).
pub const COMMIT_WAIT_MAX_TICKS: u64 = 200;

// ---------------------------------------------------------------------------
// Frozen-type bridges (SHARD-001: the two shard keys are the same immutable
// (tenant, workspace) identity in two frozen crates' shapes)
// ---------------------------------------------------------------------------

fn kernel_shard(s: &ShardId) -> ShardKey {
    ShardKey::new(s.tenant.0.clone(), s.workspace.0.clone())
        .expect("a registered ShardId maps to a well-formed ShardKey (SHARD-001)")
}

fn shard_id(k: &ShardKey) -> ShardId {
    ShardId::new(
        TenantId(k.tenant().to_string()),
        WorkspaceId(k.workspace().to_string()),
    )
}

fn persistence_shard(k: &ShardKey) -> PShardKey {
    PShardKey {
        tenant: k.tenant().to_string(),
        workspace: k.workspace().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Replicated payload codec: the Outcome payload IS the ProposedWrite, encoded
// deterministically (length-prefixed little-endian, same discipline as the
// Kernel snapshot blob codec). Consensus never decodes it (ORCH-001) — only
// this module's apply path does.
// ---------------------------------------------------------------------------

fn encode_proposed(p: &ProposedWrite) -> Vec<u8> {
    let parts: [&[u8]; 4] = [
        p.shard.tenant().as_bytes(),
        p.shard.workspace().as_bytes(),
        &p.content.0,
        &p.payload,
    ];
    let mut b = Vec::new();
    for part in parts {
        b.extend_from_slice(&(part.len() as u32).to_le_bytes());
        b.extend_from_slice(part);
    }
    b
}

fn decode_proposed(b: &[u8]) -> Option<ProposedWrite> {
    fn take<'a>(b: &'a [u8], pos: &mut usize, n: usize) -> Option<&'a [u8]> {
        let end = pos.checked_add(n)?;
        let s = b.get(*pos..end)?;
        *pos = end;
        Some(s)
    }
    fn take_part<'a>(b: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
        let len = u32::from_le_bytes(take(b, pos, 4)?.try_into().ok()?) as usize;
        take(b, pos, len)
    }
    let mut pos = 0usize;
    let tenant = String::from_utf8(take_part(b, &mut pos)?.to_vec()).ok()?;
    let workspace = String::from_utf8(take_part(b, &mut pos)?.to_vec()).ok()?;
    let content = take_part(b, &mut pos)?.to_vec();
    let payload = take_part(b, &mut pos)?.to_vec();
    if pos != b.len() {
        return None; // trailing garbage
    }
    Some(ProposedWrite {
        shard: ShardKey::new(tenant, workspace).ok()?,
        content: ContentHash(content),
        payload,
    })
}

/// Deterministic hex form of the kernel content address, carried as the opaque
/// consensus digest. (Unifying the two frozen `ContentHash` shapes with ACS-001
/// is OQ-2 → its own IDR; this mapping decides nothing — identity checks below
/// compare the full decoded proposal, not this string.)
fn consensus_digest(content: &ContentHash) -> ConsensusDigest {
    let mut s = String::with_capacity(content.0.len() * 2);
    for b in &content.0 {
        s.push_str(&format!("{b:02x}"));
    }
    ConsensusDigest(s)
}

fn map_consensus(e: ConsensusError, shard: &ShardKey) -> CommitError {
    match e {
        // Absent leadership / mid-election: this node may not commit — the
        // frozen CommitError names that refusal NotLeader (IDR-004).
        ConsensusError::NotLeader { .. } | ConsensusError::ElectionInProgress => {
            CommitError::NotLeader { shard: shard.clone() }
        }
        ConsensusError::UnknownShard(_) => CommitError::UnknownShard { shard: shard.clone() },
        // No quorum, no truth (IDR-001 CP): retriable, idempotent (ORCH-004).
        ConsensusError::QuorumUnavailable => CommitError::NotReplicated,
        ConsensusError::MembershipRejected => CommitError::Rejected {
            reason: "consensus refused the operation (membership discipline)".into(),
        },
    }
}

// ---------------------------------------------------------------------------
// Replicas and the cluster harness
// ---------------------------------------------------------------------------

/// Gateway verdict of applying ONE committed Outcome entry at one replica
/// (recorded per (shard, raft index) so `commit` can report ITS entry's
/// verdict — sim introspection, deterministic and identical on every replica).
#[derive(Clone, Debug, PartialEq, Eq)]
enum ApplyVerdict {
    /// Fresh truth: the gateway appended it to this replica's durable WAL.
    Fresh(TruthRef),
    /// Idempotent resolve to already-existing truth (ORCH-004) — no WAL append,
    /// no fork. Happens when a retry raced its own earlier entry into the log.
    Duplicate(TruthRef),
    /// Deterministic content-integrity refusal (RCR-005): the committed log
    /// carried a same-address/different-payload fork attempt; EVERY replica's
    /// gateway skips it identically (no truth delta anywhere).
    Fork,
}

/// One node's Kernel replica: truth + durable WAL + raft-apply cursors.
/// PRIVATE: the only write path is the leader's [`ClusterKernel::commit`].
struct Replica {
    /// The durable substrate (Arc-shared in memory: survives `crash_recover`).
    store: MemWalStore,
    /// The replica's Kernel — the SAME `RefKernel` gateway as single-node I1.
    kernel: MemKernel,
    /// Highest raft log index applied to `kernel`, per shard (dense, in order).
    applied: BTreeMap<ShardId, u64>,
    /// Apply verdicts per (shard, raft index) — introspection for `commit`.
    verdicts: BTreeMap<(ShardId, u64), ApplyVerdict>,
}

/// A deterministic in-process cluster: N nodes (`n1..nN`), each hosting one
/// Kernel replica and a raft replica of EVERY registered shard group (IDR-001:
/// one independent Raft group per immutable `ShardId`; a physical node hosts
/// many groups). Faults are scripted per shard group; time is the logical tick.
pub struct ClusterSim {
    groups: BTreeMap<ShardId, SimCluster>,
    replicas: BTreeMap<NodeId, Replica>,
}

impl ClusterSim {
    /// Build `nodes` replicas `n1..nN` with empty truth and no shard groups.
    pub fn new(nodes: usize) -> Self {
        let replicas = (1..=nodes)
            .map(|i| {
                let store = MemWalStore::new();
                let kernel = RefKernel::new(store.clone());
                (
                    NodeId(format!("n{i}")),
                    Replica {
                        store,
                        kernel,
                        applied: BTreeMap::new(),
                        verdicts: BTreeMap::new(),
                    },
                )
            })
            .collect();
        Self { groups: BTreeMap::new(), replicas }
    }

    /// Register the (single) Raft group for `shard` across all nodes. `seed` is
    /// the group's entire randomness budget (recorded ⇒ replayable).
    ///
    /// # Panics
    /// On a duplicate shard (IDR-001: exactly one group per shard) or a shard
    /// id that does not map to a well-formed kernel `ShardKey`.
    pub fn add_shard(&mut self, shard: ShardId, seed: u64) {
        assert!(
            !self.groups.contains_key(&shard),
            "IDR-001 violation: shard {shard:?} already has its Raft group"
        );
        let _ = kernel_shard(&shard); // validate loudly at registration
        let group = SimCluster::new(self.replicas.len(), seed);
        assert_eq!(
            group.node_ids(),
            self.replicas.keys().cloned().collect::<Vec<_>>(),
            "every node hosts a replica of every group"
        );
        self.groups.insert(shard, group);
    }

    /// All node ids, in deterministic order.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.replicas.keys().cloned().collect()
    }

    /// Drive `shard`'s group until it has a leader; returns the leader id.
    pub fn elect(&mut self, shard: &ShardId) -> NodeId {
        let leader = self
            .groups
            .get_mut(shard)
            .expect("elect: unregistered shard")
            .run_until_leader(400);
        Self::apply_all(&self.groups, &mut self.replicas);
        leader
    }

    /// Current leader of `shard`'s group, if any (highest-term live leader).
    pub fn leader_of(&self, shard: &ShardId) -> Option<NodeId> {
        self.groups
            .get(shard)
            .and_then(|g| g.current_leader().map(|(id, _)| id))
    }

    /// Run `ticks` logical ticks on every group, applying committed entries to
    /// every replica after each tick (the deterministic settle loop).
    pub fn settle(&mut self, ticks: u64) {
        for _ in 0..ticks {
            for g in self.groups.values_mut() {
                g.tick();
            }
            Self::apply_all(&self.groups, &mut self.replicas);
        }
    }

    // -- scripted faults (delegated to the shard group's bus filters) --------

    /// Cut one node off `shard`'s group (both directions).
    pub fn isolate(&mut self, shard: &ShardId, node: &NodeId) {
        self.groups.get_mut(shard).expect("isolate: unregistered shard").isolate(node);
    }

    /// Partition `shard`'s group into disjoint sides.
    pub fn partition(&mut self, shard: &ShardId, sides: &[Vec<NodeId>]) {
        self.groups.get_mut(shard).expect("partition: unregistered shard").partition(sides);
    }

    /// Heal every fault on `shard`'s group.
    pub fn heal(&mut self, shard: &ShardId) {
        self.groups.get_mut(shard).expect("heal: unregistered shard").heal();
    }

    /// Enable deterministic duplicate/reordered delivery on `shard`'s group
    /// bus (RCR-022, I2 Stage 4 — counter-scripted, zero randomness; see
    /// `SimCluster::set_mangling`). The ORCH-004 gateway must keep truth
    /// exactly-once under the storm.
    pub fn mangle(&mut self, shard: &ShardId, dup_every: u64, defer_every: u64) {
        self.groups
            .get_mut(shard)
            .expect("mangle: unregistered shard")
            .set_mangling(dup_every, defer_every);
    }

    /// `(duplicated, deferred)` mangling counters of `shard`'s group bus —
    /// lets a test assert the storm actually happened (a bite, not a no-op).
    pub fn mangled_of(&self, shard: &ShardId) -> (u64, u64) {
        let g = &self.groups[shard];
        (g.duplicated, g.deferred)
    }

    // -- read-only introspection (NOT the Query layer) ------------------------

    /// Deterministic hash of one replica's committed truth set (single-shard
    /// comparisons; the hash folds in apply order, which across MULTIPLE shards
    /// may legally differ between replicas — use [`ClusterSim::shard_state_of`]
    /// for the cross-node, per-shard byte comparison).
    pub fn truth_hash_of(&self, node: &NodeId) -> u64 {
        self.replicas[node].kernel.truth_hash()
    }

    /// Number of committed truths at one replica.
    pub fn committed_count_of(&self, node: &NodeId) -> usize {
        self.replicas[node].kernel.committed_count()
    }

    /// One replica's per-shard truth state blob (offset-ordered, deterministic
    /// bytes) — the cross-node byte-equality instrument (ORCH-003 across nodes).
    pub fn shard_state_of(&self, node: &NodeId, shard: &ShardId) -> Vec<u8> {
        self.replicas[node].kernel.snapshot_shard(&kernel_shard(shard))
    }

    /// Highest raft index applied to one replica's Kernel for `shard`.
    pub fn applied_of(&self, node: &NodeId, shard: &ShardId) -> u64 {
        self.replicas[node].applied.get(shard).copied().unwrap_or(0)
    }

    // -- crash / snapshot install ---------------------------------------------

    /// Crash one replica's Kernel (lose all in-memory truth) and recover it by
    /// deterministic replay of its local durable WAL — the proven I1.7
    /// lossless-or-loud path (ORCH-003: replay, never recompute).
    ///
    /// HONEST SCOPE: raft in-memory state (log/term/vote) is NOT crash-modeled
    /// in this stage; its durability is the IDR-005 unification stage. The
    /// `applied` cursors stay valid because the WAL is in lockstep with applied
    /// truth — and even a stale cursor would only cause idempotent re-applies
    /// (ORCH-004).
    pub fn crash_recover(&mut self, node: &NodeId) {
        let r = self.replicas.get_mut(node).expect("crash_recover: unknown replica");
        r.kernel = RefKernel::recover(r.store.clone());
    }

    /// Kernel snapshot install for a lagging follower (IDR-002: "snapshot
    /// install, then log tail"): the source's per-shard truth state blob — a
    /// pure function of its applied log prefix — is materialized at the target
    /// into (1) the local durable WAL as a DENSE continuation (so post-snapshot
    /// applies assign the identical `CommitIndex`es as every peer), (2) the
    /// truth state (the idempotent I1 restore path), and (3) the raft-apply
    /// cursor, which jumps to the source's applied position so the log tail
    /// beyond the snapshot applies normally afterwards.
    ///
    /// HONEST SCOPE: this transfer is harness-orchestrated (tests call it
    /// explicitly); the Raft leader does not initiate InstallSnapshot in this
    /// stage — protocol-driven snapshotting (with log compaction) is OQ-1.
    pub fn install_snapshot(&mut self, shard: &ShardId, from: &NodeId, to: &NodeId) {
        assert!(self.groups.contains_key(shard), "install_snapshot: unregistered shard");
        let kshard = kernel_shard(shard);
        let (blob, covered) = {
            let src = &self.replicas[from];
            (
                src.kernel.snapshot_shard(&kshard),
                src.applied.get(shard).copied().unwrap_or(0),
            )
        };
        let entries = decode_shard_blob(&blob).expect("snapshot blob is Kernel-encoded");
        let dst = self.replicas.get_mut(to).expect("install_snapshot: unknown replica");
        let pshard = persistence_shard(&kshard);
        let mut wal = dst.store.open(&pshard).expect("wal open");
        for (offset, content, payload) in &entries {
            if *offset < wal.head() {
                continue; // already durable locally
            }
            assert_eq!(
                *offset,
                wal.head(),
                "snapshot blob must continue the local WAL densely (no gaps)"
            );
            wal.append(PendingRecord {
                shard: pshard.clone(),
                term: 0,
                kind: RecordKind::Outcome,
                content: ContentId(content.clone()),
                payload: payload.clone(),
            })
            .expect("wal append during snapshot install");
        }
        dst.kernel.install_state(&kshard, &blob);
        let cur = dst.applied.entry(shard.clone()).or_insert(0);
        *cur = (*cur).max(covered);
    }

    // -- the apply loop (IDR-002: followers apply, never recompute) -----------

    /// Apply every newly committed raft entry, per shard, per replica, in dense
    /// log-index order, through the SAME `RefKernel` gateway as a single-node
    /// commit. Deterministic: identical logs ⇒ identical truth on every replica
    /// (ORCH-003). Membership entries are consensus mechanism state and never
    /// enter the Kernel; a committed fork attempt is refused IDENTICALLY on
    /// every replica (RCR-005 under replication).
    fn apply_all(groups: &BTreeMap<ShardId, SimCluster>, replicas: &mut BTreeMap<NodeId, Replica>) {
        for (sid, group) in groups {
            let kshard = kernel_shard(sid);
            for (node, replica) in replicas.iter_mut() {
                let committed = group.commit_of(node);
                let from = replica.applied.get(sid).copied().unwrap_or(0);
                if committed <= from {
                    continue;
                }
                for i in (from + 1)..=committed {
                    let entry = group.log_of(node)[i as usize - 1].clone();
                    if let EntryKind::Outcome(o) = entry.kind {
                        let pw = decode_proposed(&o.payload)
                            .expect("replicated outcome payload is Kernel-encoded (RCR-021)");
                        assert_eq!(
                            pw.shard, kshard,
                            "SHARD-001: no cross-shard bytes in a shard's log"
                        );
                        let verdict = match replica.kernel.commit(pw) {
                            Ok(tr) => ApplyVerdict::Fresh(tr),
                            Err(CommitError::AlreadyCommitted(tr)) => ApplyVerdict::Duplicate(tr),
                            Err(CommitError::ContentIntegrity { .. }) => ApplyVerdict::Fork,
                            // Anything else would be a HOST failure diverging one
                            // replica from the deterministic apply — refuse to
                            // continue on partial truth (lossless or loud).
                            Err(e) => panic!(
                                "replica {node:?} failed to apply committed entry {i}: {e}"
                            ),
                        };
                        replica.verdicts.insert((sid.clone(), i), verdict);
                    }
                    replica.applied.insert(sid.clone(), i);
                }
            }
        }
    }

    // -- the leader commit path (the heart of RCR-021) ------------------------

    /// The commit gateway body used by [`ClusterKernel::commit`], executing at
    /// replica `node`.
    fn commit_at(&mut self, node: &NodeId, proposed: ProposedWrite) -> Result<TruthRef, CommitError> {
        let sid = shard_id(&proposed.shard);
        if !self.groups.contains_key(&sid) {
            // SHARD-001: no group, no route — and never a cross-shard fallback.
            return Err(CommitError::UnknownShard { shard: proposed.shard });
        }
        // OWN-001 / IDR-004: only the shard leader's gateway is authoritative.
        if self.groups[&sid].node(node).role() != Role::Leader {
            return Err(CommitError::NotLeader { shard: proposed.shard });
        }
        // Gateway admission BEFORE replication — the IDENTICAL validation head
        // `commit_inner` runs (ORCH-004 dedupe resolves to the existing TruthRef;
        // RCR-005 refuses a same-address/different-payload fork). Never forked.
        self.replicas[node].kernel.admission_check(&proposed)?;
        // IDR-002: replicate the already-decided OUTCOME, opaque to consensus.
        let encoded = encode_proposed(&proposed);
        let outcome = Outcome {
            digest: consensus_digest(&proposed.content),
            payload: encoded.clone(),
        };
        let idx = self
            .groups
            .get_mut(&sid)
            .expect("checked above")
            .propose(node, EntryKind::Outcome(outcome))
            .map_err(|e| map_consensus(e, &proposed.shard))?;
        // Quorum wait: drive deterministic ticks, applying commits as they land.
        for _ in 0..COMMIT_WAIT_MAX_TICKS {
            if self.groups[&sid].commit_of(node) >= idx.0 {
                break;
            }
            self.groups.get_mut(&sid).expect("checked above").tick();
            Self::apply_all(&self.groups, &mut self.replicas);
        }
        if self.groups[&sid].commit_of(node) < idx.0 {
            // IDR-001 CP posture: no quorum, no truth — honestly unavailable.
            // Retriable; ORCH-004 keeps the retry idempotent. (The pending
            // entry either commits later — a retry then dedupes — or is
            // truncated by the next leader: no partial truth, A-005/A-006.)
            return Err(CommitError::NotReplicated);
        }
        // RCR-019 DR-8: the frozen index-only contract means a failover can
        // legally commit a DIFFERENT entry at our index — verify identity
        // before claiming this commit as ours.
        let ours = match &self.groups[&sid].log_of(node)[idx.0 as usize - 1].kind {
            EntryKind::Outcome(o) => o.payload == encoded,
            EntryKind::Membership(_) => false,
        };
        if !ours {
            return Err(CommitError::NotReplicated);
        }
        // Apply through the shared gateway on every replica, then report OUR
        // entry's verdict from this leader's deterministic apply.
        Self::apply_all(&self.groups, &mut self.replicas);
        match self.replicas[node].verdicts.get(&(sid, idx.0)) {
            Some(ApplyVerdict::Fresh(tr)) => Ok(tr.clone()),
            Some(ApplyVerdict::Duplicate(tr)) => Err(CommitError::AlreadyCommitted(tr.clone())),
            Some(ApplyVerdict::Fork) => Err(CommitError::ContentIntegrity { shard: proposed.shard }),
            None => unreachable!("a committed entry at the leader must have been applied"),
        }
    }
}

// ---------------------------------------------------------------------------
// The frozen Kernel trait over the cluster: one handle per replica
// ---------------------------------------------------------------------------

/// The CLUSTER KERNEL: a [`Kernel`] handle bound to ONE replica of the shared
/// [`ClusterSim`] (client-to-replica semantics, as in the frozen consensus
/// contract's first impl). `commit` is authoritative ONLY when this replica
/// leads the target shard's Raft group; every other replica refuses with
/// [`CommitError::NotLeader`] (OWN-001: followers are derived replicas, never
/// writers). A leader commit reaches quorum through the shard's Raft log
/// BEFORE the caller is acked, then applies through the identical `RefKernel`
/// gateway on every replica (IDR-001/002; ORCH-003/004).
pub struct ClusterKernel {
    node: NodeId,
    cluster: Rc<RefCell<ClusterSim>>,
}

impl ClusterKernel {
    /// Bind a commit handle to replica `node` of `cluster`.
    ///
    /// # Panics
    /// If `node` is not a replica of the cluster (harness programming error).
    pub fn new(node: NodeId, cluster: Rc<RefCell<ClusterSim>>) -> Self {
        assert!(
            cluster.borrow().replicas.contains_key(&node),
            "ClusterKernel must bind an existing replica"
        );
        Self { node, cluster }
    }
}

impl Kernel for ClusterKernel {
    fn commit(&self, proposed: ProposedWrite) -> Result<TruthRef, CommitError> {
        self.cluster.borrow_mut().commit_at(&self.node, proposed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The replicated-payload codec round-trips exactly and rejects trailing
    /// garbage (the apply path must never guess at a malformed payload).
    #[test]
    fn proposed_write_codec_round_trips_and_rejects_garbage() {
        let p = ProposedWrite {
            shard: ShardKey::new("t1", "w1").unwrap(),
            content: ContentHash(vec![0x12, 0x20, 0xAB]),
            payload: b"payload bytes".to_vec(),
        };
        let enc = encode_proposed(&p);
        assert_eq!(decode_proposed(&enc), Some(p));
        let mut garbage = enc.clone();
        garbage.push(0);
        assert_eq!(decode_proposed(&garbage), None, "trailing garbage refused");
        assert_eq!(decode_proposed(&enc[..enc.len() - 1]), None, "truncation refused");
    }

    /// The consensus digest mapping is deterministic hex (OQ-2 stays open; the
    /// digest is opaque and never load-bearing for identity).
    #[test]
    fn consensus_digest_is_deterministic_hex() {
        let c = ContentHash(vec![0x00, 0xFF, 0x0A]);
        assert_eq!(consensus_digest(&c).0, "00ff0a");
        assert_eq!(consensus_digest(&c), consensus_digest(&c.clone()));
    }
}

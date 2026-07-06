//! RCR-025 (I3 Stage 3) — ADVERSARIAL READ PROOFS per the approved design's
//! conformance plan (`docs/design/I3_Distributed_Query_Design.md` §4 proof
//! rows + §5.2 Stage-3 fault-injected slice).
//!
//! Every test is deterministic: fixed seeds, scripted faults (bus filters /
//! counter-scripted mangling), injected logical ticks — no wall clocks, no OS
//! randomness, no sleeps. Truths enter ONLY through the Kernel commit gateway
//! (single-node `RefKernel` / leader-only `ClusterKernel` — OWN-001); the
//! query fabric reconstructs them by WAL replay of the serving replica's own
//! durable store (ORCH-001 — no Kernel truth accessor is consumed).
//!
//! Proof map (this stage):
//! - **(a) Torn-read impossibility** — `torn_read_impossibility_*`: a query
//!   never observes a partially-applied RCR-013 batch. At every observation
//!   point reachable by a reader in this deterministic harness (control is
//!   outside `commit_batch`, which appends the whole batch under one state
//!   lock before returning), the WAL head sits on a batch boundary, so every
//!   served fold contains each batch all-or-none — and every served
//!   `observed_at` is provably a batch boundary. A REFUSED batch (intra-batch
//!   fork, or a fork against committed truth) changes NOTHING visible.
//! - **(b) Replay equivalence** — `replay_equivalence_*`: on EVERY replica,
//!   the projection rebuilt from that replica's WAL equals what the live
//!   query surface served (same position, same bytes, same digest), converged
//!   replicas' rebuilds are equal to each other, a crash/recover changes
//!   nothing, and any served read is reproducible after the fact by a pinned
//!   rebuild at its `observed_at` (design §3.11/§3.19).
//! - **(c) Partition reads** — `minority_partition_*`: minority-side follower
//!   reads keep serving on the AP tier — bit-identical to the pre-partition
//!   capture, honestly labeled, fabricating NOTHING (majority-only truth is
//!   `NotFound`/absent, the visible universe is exactly the old prefix) —
//!   while both strong tiers refuse; heal converges every replica's
//!   projection to equality (IDR-001/IDR-005 CP truth / AP observability).
//! - **(d) Determinism under message storms** — `query_results_deterministic_*`:
//!   with deterministic duplicate/reordered consensus delivery ACTIVE (and
//!   proven to have bitten), two independent full runs produce bit-identical
//!   query transcripts (mid-storm AND converged), and all replicas converge
//!   to identical folds.

use std::cell::RefCell;
use std::rc::Rc;

use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{BatchError, ContentHash, Kernel, MemKernel, ProposedWrite, ShardKey as KShardKey};
use arves_persistence::{ContentId, MemWalStore, ShardKey as WalShardKey, Wal, WalStore};
use arves_query::projection::{projection_id_for, ShardProjection, WalQuery};
use arves_query::{Query, QueryError, ReadScope, ReadTier, StalenessBound};

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}
fn wshard(t: &str, w: &str) -> WalShardKey {
    WalShardKey { tenant: t.into(), workspace: w.into() }
}
fn pw(t: &str, w: &str, content: &[u8], payload: &[u8]) -> ProposedWrite {
    ProposedWrite {
        shard: KShardKey::new(t, w).expect("well-formed shard key"),
        content: ContentHash(content.to_vec()),
        payload: payload.to_vec(),
    }
}
fn pid(content: &[u8]) -> String {
    projection_id_for(&ContentId(content.to_vec()))
}
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Commit through the shard leader's gateway (the only lawful write path).
fn commit_at_leader(
    cluster: &Rc<RefCell<ClusterSim>>,
    shard: &ShardId,
    content: &[u8],
    payload: &[u8],
) {
    let leader = cluster.borrow().leader_of(shard).expect("a leader exists");
    let k = ClusterKernel::new(leader, cluster.clone());
    k.commit(pw(&shard.tenant.0, &shard.workspace.0, content, payload))
        .expect("leader commit");
}

// ---------------------------------------------------------------------------
// (a) Torn-read impossibility: RCR-013 batches are atomic in the projection.
// ---------------------------------------------------------------------------

/// A query NEVER observes a partially-applied batch. `commit_batch` (RCR-013)
/// appends the whole batch under one state lock before returning, so every
/// observation point a reader can reach in this deterministic harness sees the
/// WAL head on a batch boundary; the served fold therefore contains each batch
/// all-or-none, on every tier, and every served `observed_at` IS a batch
/// boundary. A refused batch (intra-batch fork, fork against committed truth)
/// leaves the projection bit-identical. HONEST LIMITS, stated: (1) the trace
/// itself has per-record offsets, so the internal replay facility
/// `ShardProjection::at_version` CAN be pinned mid-batch — that surface is a
/// replay/audit tool, not a `Query`-trait read, and no `Query` read ever
/// serves such a position (asserted below); (2) `BatchError::PartialApply`
/// (a mid-batch host I/O failure) is out of reach of `MemWalStore` and stays
/// the RCR-013 honest atomicity boundary — its cluster form (a Raft entry
/// carrying a batch) is explicitly deferred (RCR-021 honest scope).
#[test]
fn torn_read_impossibility_batches_all_or_none_on_every_reachable_observation() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    let sh = wshard("acme", "research");
    let text = || -> String { "acme/research".into() };

    // Seed truth so the shard exists and an Eventual fold can go stale.
    k.commit(pw("acme", "research", b"seed", b"seed-truth")).expect("seed");
    let q = WalQuery::new(store.clone());
    let ev = ReadScope::eventual(text());
    let lin = ReadScope::linearizable(text());
    let bs = ReadScope::bounded(text(), StalenessBound::new(0));
    let mut served_positions: Vec<u64> = Vec::new();

    // Bootstrap the standing fold at head=1 (a batch boundary: the seed is a
    // batch of one).
    let p_seed = q.read(&ev, &pid(b"seed")).expect("bootstrap read");
    served_positions.push(p_seed.observed_at);

    // Batch B1: three records committed atomically (all-or-nothing, RCR-013).
    let b1_ids = [pid(b"b1-a"), pid(b"b1-b"), pid(b"b1-c")];
    k.commit_batch(vec![
        pw("acme", "research", b"b1-a", b"batch1-alpha"),
        pw("acme", "research", b"b1-b", b"batch1-beta"),
        pw("acme", "research", b"b1-c", b"batch1-gamma"),
    ])
    .expect("batch B1 commits whole");
    let head_after_b1 = store.open(&sh).expect("wal").head();
    assert_eq!(head_after_b1, 4, "head moved by the FULL batch (1 -> 4), never a prefix");

    // Observation point 1 — the STALE fold (Eventual, position 1): NONE of B1
    // is visible. The none-arm of all-or-none.
    for id in &b1_ids {
        assert_eq!(q.exists(&ev, id), Ok(false), "stale fold sees NO member of B1");
    }
    // Observation point 2 — strong tiers catch up: ALL of B1 is visible at
    // once, at the batch-boundary position 4. The all-arm.
    for id in &b1_ids {
        let p = q.read(&lin, id).expect("linearizable read serves the whole batch");
        assert_eq!(p.observed_at, 4, "served position is the batch boundary");
        served_positions.push(p.observed_at);
    }
    // The refreshed fold now serves B1 completely on the weak tier too.
    for id in &b1_ids {
        assert_eq!(q.exists(&ev, id), Ok(true));
    }

    // Batch B2: two more records; boundary 4 -> 6.
    let b2_ids = [pid(b"b2-a"), pid(b"b2-b")];
    k.commit_batch(vec![
        pw("acme", "research", b"b2-a", b"batch2-alpha"),
        pw("acme", "research", b"b2-b", b"batch2-beta"),
    ])
    .expect("batch B2 commits whole");
    let p = q.read(&bs, &b2_ids[0]).expect("bounded read");
    assert_eq!(p.observed_at, 6);
    served_positions.push(p.observed_at);
    assert_eq!(q.exists(&bs, &b2_ids[1]), Ok(true), "whole batch, same observation");

    // REFUSED batches change NOTHING visible (all-or-nothing bites the whole
    // validation class; the projection is bit-identical afterwards).
    let digest_before = ShardProjection::at_head(&store, &sh).expect("fold").fold_digest();
    // Intra-batch fork: one address, two payloads.
    assert!(matches!(
        k.commit_batch(vec![
            pw("acme", "research", b"forked", b"payload-one"),
            pw("acme", "research", b"forked", b"payload-two"),
        ]),
        Err(BatchError::Refused { .. })
    ));
    // Fork against committed truth in the LAST slot: even the clean first
    // entry must not appear (all-or-nothing).
    assert!(matches!(
        k.commit_batch(vec![
            pw("acme", "research", b"clean-new", b"would-be-fine-alone"),
            pw("acme", "research", b"b1-a", b"DIFFERENT-payload-for-committed-address"),
        ]),
        Err(BatchError::Refused { .. })
    ));
    assert_eq!(store.open(&sh).expect("wal").head(), 6, "refused batches appended nothing");
    let after = ShardProjection::at_head(&store, &sh).expect("fold");
    assert_eq!(after.fold_digest(), digest_before, "projection bit-identical after refusals");
    assert_eq!(q.exists(&lin, &pid(b"clean-new")), Ok(false), "no partial refusal residue");
    assert_eq!(q.exists(&lin, &pid(b"forked")), Ok(false));

    // Every position the Query surface EVER served is a batch boundary…
    let boundaries = [1u64, 4, 6];
    for pos in &served_positions {
        assert!(
            boundaries.contains(pos),
            "Query served position {pos}, which is not a batch boundary {boundaries:?}"
        );
    }
    // …and pinned rebuilds AT those boundaries hold each batch all-or-none.
    let batches: [&[String]; 2] = [&b1_ids, &b2_ids];
    for v in boundaries {
        let pinned = ShardProjection::at_version(&store, &sh, v).expect("pinned rebuild");
        for batch in batches {
            let visible = batch.iter().filter(|id| pinned.get(id).is_some()).count();
            assert!(
                visible == 0 || visible == batch.len(),
                "torn batch at boundary {v}: {visible}/{} visible",
                batch.len()
            );
        }
    }
    // Honest counterpoint, demonstrated: the internal replay facility CAN pin
    // a mid-batch trace position (offset 2 = inside B1) — that is the audit
    // surface replaying the per-record trace, and the assertions above prove
    // the `Query` surface never serves such a position.
    let mid = ShardProjection::at_version(&store, &sh, 2).expect("mid-batch replay pin");
    let mid_visible = b1_ids.iter().filter(|id| mid.get(id).is_some()).count();
    assert_eq!(mid_visible, 1, "the trace is per-record; only the Query surface is batch-atomic");
}

// ---------------------------------------------------------------------------
// (b) Replay equivalence: rebuilt-from-WAL == live projection, every replica.
// ---------------------------------------------------------------------------

/// On EVERY replica of the cluster: an independent projection rebuilt from
/// that replica's durable WAL equals what the live query surface served (same
/// trace position, same bytes); two rebuilds are equal; converged replicas'
/// rebuilds are equal ACROSS nodes (digest and `PartialEq`); a full-cluster
/// crash/recover changes nothing; and every served read is reproducible after
/// the fact by a pinned rebuild at its `observed_at` (ORCH-003; design
/// §3.11 replay + §3.19 auditability).
#[test]
fn replay_equivalence_rebuilt_from_wal_equals_live_projection_on_every_replica() {
    let acme = sid("acme", "w1");
    let globex = sid("globex", "w1");
    let mut c = ClusterSim::new(3);
    c.add_shard(acme.clone(), 0xA0);
    c.add_shard(globex.clone(), 0xB0);
    c.elect(&acme);
    c.elect(&globex);
    let cluster = Rc::new(RefCell::new(c));
    commit_at_leader(&cluster, &acme, b"a1", b"acme-truth-one");
    commit_at_leader(&cluster, &acme, b"a2", b"acme-truth-two");
    commit_at_leader(&cluster, &acme, b"a3", b"acme-truth-three");
    commit_at_leader(&cluster, &globex, b"g1", b"globex-truth-one");
    cluster.borrow_mut().settle(6);

    let cases = [
        (wshard("acme", "w1"), "acme/w1".to_string(), pid(b"a2"), b"acme-truth-two".as_slice(), 3u64),
        (wshard("globex", "w1"), "globex/w1".to_string(), pid(b"g1"), b"globex-truth-one".as_slice(), 1u64),
    ];

    let mut folds_per_shard: Vec<Vec<ShardProjection>> = vec![Vec::new(), Vec::new()];
    for node in cluster.borrow().node_ids() {
        let q = ClusterQueryHandle::new(&node, &cluster);
        for (i, (wsh, text, id, bytes, applied)) in cases.iter().enumerate() {
            // The live read, via the frozen Query trait.
            let served = q.0.read(&ReadScope::eventual(text.clone()), id).expect("live read");
            assert!(contains(&served.value, bytes));
            assert_eq!(served.observed_at, *applied);

            // Independent rebuild from THIS replica's own durable WAL.
            let store = cluster.borrow().wal_store_of(&node);
            let rebuilt = ShardProjection::at_head(&store, wsh).expect("rebuild");
            let rebuilt_again = ShardProjection::at_head(&store, wsh).expect("rebuild again");
            assert_eq!(rebuilt, rebuilt_again, "two rebuilds over one prefix are EQUAL");
            assert_eq!(rebuilt.applied(), served.observed_at, "rebuilt position == served position");
            let (value, _) = rebuilt.get(id).expect("rebuilt fold holds the served id");
            assert_eq!(value, served.value.as_slice(), "rebuilt bytes == served bytes");

            // Auditability (§3.19): a pinned rebuild at the served
            // `observed_at` reproduces exactly what the caller saw.
            let pinned = ShardProjection::at_version(&store, wsh, served.observed_at)
                .expect("pinned rebuild at observed_at");
            assert_eq!(pinned.get(id).expect("pinned holds id").0, served.value.as_slice());

            folds_per_shard[i].push(rebuilt);
        }
    }
    // Converged replica-equality ACROSS nodes: every replica's rebuild of a
    // shard is equal (PartialEq) and digest-equal to every other's.
    for folds in &folds_per_shard {
        assert_eq!(folds.len(), 3);
        assert!(folds.windows(2).all(|w| w[0] == w[1]), "replica folds diverge");
        assert!(folds.windows(2).all(|w| w[0].fold_digest() == w[1].fold_digest()));
    }

    // Full-cluster crash/recover (replay, never recompute): the live surface
    // and the rebuilds are unchanged on every replica.
    {
        let mut c = cluster.borrow_mut();
        for node in c.node_ids() {
            c.crash_recover(&node);
        }
    }
    for node in cluster.borrow().node_ids() {
        let q = ClusterQueryHandle::new(&node, &cluster);
        for (i, (wsh, text, id, bytes, applied)) in cases.iter().enumerate() {
            let served = q.0.read(&ReadScope::linearizable(text.clone()), id).expect("post-crash read");
            assert!(contains(&served.value, bytes));
            assert_eq!(served.observed_at, *applied);
            let store = cluster.borrow().wal_store_of(&node);
            let rebuilt = ShardProjection::at_head(&store, wsh).expect("post-crash rebuild");
            assert_eq!(rebuilt.fold_digest(), folds_per_shard[i][0].fold_digest());
        }
    }
}

/// Thin wrapper so the tests construct the distributed read handle uniformly.
struct ClusterQueryHandle(arves_query::distributed::ClusterQuery);
impl ClusterQueryHandle {
    fn new(node: &NodeId, cluster: &Rc<RefCell<ClusterSim>>) -> Self {
        Self(arves_query::distributed::ClusterQuery::new(node.clone(), cluster.clone()))
    }
}

// ---------------------------------------------------------------------------
// (c) Partition reads: labeled-stale service, zero fabrication, heal converges.
// ---------------------------------------------------------------------------

/// Under a 2/3 minority partition (5 nodes), every minority-side follower
/// keeps serving the AP tier — BIT-IDENTICAL to its pre-partition capture,
/// honestly labeled (`Eventual`, old `observed_at`) — and fabricates NOTHING:
/// majority-only truth is `NotFound`/absent on every read form, and the whole
/// visible universe is exactly the old prefix. Both strong tiers refuse
/// (CP truth). On heal, EVERY replica's projection converges to equality and
/// every tier serves the new truth (IDR-001/IDR-005 CP/AP split; design §3.7).
#[test]
fn minority_partition_reads_labeled_stale_never_fabricated_and_heal_converges() {
    let shard = sid("t1", "w1");
    let mut c = ClusterSim::new(5);
    c.add_shard(shard.clone(), 0x517A6E);
    let leader = c.elect(&shard);
    let cluster = Rc::new(RefCell::new(c));
    commit_at_leader(&cluster, &shard, b"e1", b"pre-partition-one");
    commit_at_leader(&cluster, &shard, b"e2", b"pre-partition-two");
    cluster.borrow_mut().settle(6);

    let nodes = cluster.borrow().node_ids();
    let minority: Vec<NodeId> = nodes.iter().filter(|n| **n != leader).take(2).cloned().collect();
    let majority: Vec<NodeId> = nodes.iter().filter(|n| !minority.contains(n)).cloned().collect();
    assert_eq!((minority.len(), majority.len()), (2, 3));

    let ev = ReadScope::eventual("t1/w1".into());
    let lin = ReadScope::linearizable("t1/w1".into());
    let bound = StalenessBound::new(1_000_000);
    let bs = ReadScope::bounded("t1/w1".into(), bound);

    // Pre-partition capture at each minority replica (bit-exact baselines).
    let baselines: Vec<_> = minority
        .iter()
        .map(|n| {
            let q = ClusterQueryHandle::new(n, &cluster);
            let p = q.0.read(&ev, &pid(b"e1")).expect("pre-partition read");
            assert_eq!(p.observed_at, 2);
            p
        })
        .collect();

    // Partition: {2 followers} vs {leader + 2} — the majority retains quorum.
    cluster.borrow_mut().partition(&shard, &[minority.clone(), majority.clone()]);
    commit_at_leader(&cluster, &shard, b"e3", b"majority-only-three");
    commit_at_leader(&cluster, &shard, b"e4", b"majority-only-four");
    cluster.borrow_mut().settle(4);

    for (i, n) in minority.iter().enumerate() {
        let q = ClusterQueryHandle::new(n, &cluster);
        // AP service continues, BIT-IDENTICAL to the pre-partition capture:
        // same bytes, same position, same label — stale, never wrong.
        let stale = q.0.read(&ev, &pid(b"e1")).expect("minority AP read serves");
        assert_eq!(stale, baselines[i], "minority read == pre-partition capture, bit-equal");
        // ZERO fabrication: majority-only truth does not exist here in ANY
        // read form…
        for id in [pid(b"e3"), pid(b"e4")] {
            assert_eq!(q.0.read(&ev, &id), Err(QueryError::NotFound { id: id.clone() }));
            assert_eq!(q.0.exists(&ev, &id), Ok(false));
            assert_eq!(q.0.latest_version(&ev, &id), Err(QueryError::NotFound { id }));
        }
        // …and the visible universe is EXACTLY the old prefix (2 records).
        let store = cluster.borrow().wal_store_of(n);
        let fold = ShardProjection::at_head(&store, &wshard("t1", "w1")).expect("fold");
        assert_eq!((fold.applied(), fold.len()), (2, 2), "nothing invented, nothing lost");
        // CP truth refuses on the minority side.
        assert_eq!(q.0.read(&lin, &pid(b"e4")), Err(QueryError::LeaderUnavailable));
        assert!(matches!(
            q.0.read(&bs, &pid(b"e4")),
            Err(QueryError::StalenessBoundExceeded { .. })
        ));
    }
    // The majority leader's linearizable read reflects quorum truth meanwhile.
    let ql = ClusterQueryHandle::new(&leader, &cluster);
    let fresh = ql.0.read(&lin, &pid(b"e4")).expect("majority linearizable read");
    assert!(contains(&fresh.value, b"majority-only-four"));
    assert_eq!(fresh.observed_at, 4);

    // Heal: minority term inflation may depose the leader; a fresh election
    // yields a leader that (per RCR-019 DR-2, no election no-op) may hold NO
    // committed entry of its current term — so per the DR-8 read-index
    // precondition the strong tiers stay honestly refused until one commit
    // lands in the new term. Commit that marker (the design's operational
    // answer), then prove full convergence.
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(80);
    commit_at_leader(&cluster, &shard, b"e5", b"post-heal-marker");
    cluster.borrow_mut().settle(8);
    let folds: Vec<ShardProjection> = cluster
        .borrow()
        .node_ids()
        .iter()
        .map(|n| {
            let store = cluster.borrow().wal_store_of(n);
            ShardProjection::at_head(&store, &wshard("t1", "w1")).expect("fold")
        })
        .collect();
    assert!(folds.windows(2).all(|w| w[0] == w[1]), "healed projections converge, all 5 equal");
    assert_eq!(folds[0].applied(), 5, "e1..e5 all applied everywhere");
    for n in &minority {
        let q = ClusterQueryHandle::new(n, &cluster);
        let p = q.0.read(&lin, &pid(b"e3")).expect("strong tier serves after heal");
        assert!(contains(&p.value, b"majority-only-three"));
        assert!(q.0.read(&bs, &pid(b"e4")).is_ok(), "zero lag provable again");
        let now = q.0.read(&ev, &pid(b"e1")).expect("AP read");
        assert_eq!(now.observed_at, 5, "the AP fold advanced past the stale capture");
    }
}

// ---------------------------------------------------------------------------
// (d) Query determinism under message storms.
// ---------------------------------------------------------------------------

/// One full storm run: two shards of one tenant under deterministic
/// duplicate/reordered consensus delivery, an interleaved commit workload,
/// mid-storm AP observations, convergence, then strong-tier reads, gathers
/// and per-replica fold digests — all appended to one textual transcript.
/// Returns `(transcript, (dup1, defer1), (dup2, defer2))`.
fn storm_run() -> (Vec<String>, (u64, u64), (u64, u64)) {
    let w1 = sid("acme", "w1");
    let w2 = sid("acme", "w2");
    let mut c = ClusterSim::new(3);
    c.add_shard(w1.clone(), 0x570A11);
    c.add_shard(w2.clone(), 0x570A22);
    c.elect(&w1);
    c.elect(&w2);
    // The storm: counter-scripted duplication + deferral on BOTH shard buses
    // (zero randomness — identically-scripted runs must stay byte-identical).
    c.mangle(&w1, 3, 4);
    c.mangle(&w2, 2, 5);
    let cluster = Rc::new(RefCell::new(c));

    // Interleaved workload committed THROUGH the storm.
    commit_at_leader(&cluster, &w1, b"s1", b"storm-w1-one");
    commit_at_leader(&cluster, &w2, b"s1", b"storm-w2-one");
    commit_at_leader(&cluster, &w1, b"s2", b"storm-w1-two");
    commit_at_leader(&cluster, &w2, b"s2", b"storm-w2-two");
    commit_at_leader(&cluster, &w1, b"s3", b"storm-w1-three");

    let mut transcript = Vec::new();
    // Mid-storm AP observations on every replica (possibly stale — that is
    // fine; the claim is DETERMINISM, and Eventual never refuses).
    for node in cluster.borrow().node_ids() {
        let q = ClusterQueryHandle::new(&node, &cluster);
        for (text, id) in [("acme/w1", pid(b"s1")), ("acme/w2", pid(b"s2"))] {
            let r = q.0.read(&ReadScope::eventual(text.into()), &id);
            transcript.push(format!("mid-storm {node:?} {text} {id}: {r:?}"));
        }
    }

    // Converge (mangling stays ACTIVE — deferred/duplicated delivery included).
    cluster.borrow_mut().settle(40);

    // Strong-tier reads on every replica, the tenant gather, and every
    // replica's independent fold digests.
    for node in cluster.borrow().node_ids() {
        let q = ClusterQueryHandle::new(&node, &cluster);
        for (text, id) in [
            ("acme/w1", pid(b"s1")),
            ("acme/w1", pid(b"s3")),
            ("acme/w2", pid(b"s2")),
        ] {
            let r = q.0.read(&ReadScope::linearizable(text.into()), &id);
            transcript.push(format!("converged {node:?} {text} {id}: {r:?}"));
        }
        let g = q.0.gather_read("acme", ReadTier::Eventual, None, &pid(b"s1"));
        transcript.push(format!("gather {node:?}: {g:?}"));
        let store = cluster.borrow().wal_store_of(&node);
        for wsh in [wshard("acme", "w1"), wshard("acme", "w2")] {
            let fold = ShardProjection::at_head(&store, &wsh).expect("fold");
            transcript.push(format!(
                "fold {node:?} {}/{}: applied={} digest={:016x}",
                wsh.tenant,
                wsh.workspace,
                fold.applied(),
                fold.fold_digest()
            ));
        }
    }
    let c = cluster.borrow();
    (transcript, c.mangled_of(&w1), c.mangled_of(&w2))
}

/// Two independent, identically-scripted storm runs produce BIT-IDENTICAL
/// query transcripts (mid-storm and converged), the storm provably BIT (both
/// mangling arms fired on both shard buses), and within each run all replicas
/// converged to identical folds (the transcript's digest lines are asserted
/// equal across nodes).
#[test]
fn query_results_deterministic_and_replicas_converge_under_message_storms() {
    let (t1, w1_counters_a, w2_counters_a) = storm_run();
    let (t2, w1_counters_b, w2_counters_b) = storm_run();

    // The storm actually happened — both arms, both shard buses.
    assert!(w1_counters_a.0 > 0 && w1_counters_a.1 > 0, "w1 storm bit: {w1_counters_a:?}");
    assert!(w2_counters_a.0 > 0 && w2_counters_a.1 > 0, "w2 storm bit: {w2_counters_a:?}");
    // Determinism: counters and the full transcript are bit-identical.
    assert_eq!(w1_counters_a, w1_counters_b);
    assert_eq!(w2_counters_a, w2_counters_b);
    assert_eq!(t1, t2, "two identically-scripted storm runs must be bit-identical");

    // Replica convergence within the run: per shard, all three fold lines are
    // identical except for the node id (same applied, same digest).
    for shard_text in ["acme/w1", "acme/w2"] {
        let tails: Vec<&str> = t1
            .iter()
            .filter(|l| l.starts_with("fold ") && l.contains(shard_text))
            .map(|l| l.split(':').nth(1).expect("fold line tail"))
            .collect();
        assert_eq!(tails.len(), 3);
        assert!(tails.windows(2).all(|w| w[0] == w[1]), "replicas diverged on {shard_text}: {tails:?}");
    }
}

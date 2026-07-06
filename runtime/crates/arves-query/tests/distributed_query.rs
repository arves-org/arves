//! RCR-024 (I3 Stage 2) — executable proofs for DISTRIBUTED READS over the I2
//! cluster substrate (`ClusterSim`, RCR-021).
//!
//! Every test is deterministic: fixed seeds, scripted faults (bus filters),
//! injected logical ticks — no wall clocks, no OS randomness, no sleeps.
//! Truths enter ONLY through the leader-only `ClusterKernel` gateway
//! (OWN-001); the query fabric then reconstructs them by WAL replay of each
//! replica's own durable store (ORCH-001 — no Kernel truth accessor is used).
//!
//! Proof map (design §4 rows, Stage-2 distributed form):
//! - IDR-001/IDR-005 CP/AP ladder — `partitioned_follower_*`,
//!   `deposed_minority_leader_*`: a leader-consistent read reflects
//!   quorum-committed truth; a follower read after partition is served
//!   HONESTLY STALE (labeled `Eventual`, stale `observed_at`) while the
//!   strong tiers refuse (`LeaderUnavailable` / `StalenessBoundExceeded`
//!   with the unattestable-lag sentinel).
//! - ORCH-003 across nodes — `linearizable_read_index_*`: converged replicas'
//!   folds are EQUAL (replica-equality, distributed form).
//! - Read-your-writes (OQ-5 additive carrier) — `read_your_writes_*`: a
//!   returned `TruthRef` floor is honored at a current replica and refused
//!   as `BelowFloor` (never a false `NotFound`) at a lagging one.
//! - Scatter-gather (§3.7; OQ-3 yes / OQ-4 fail-whole) —
//!   `scatter_gather_*`: deterministic merge order and bit-equal results
//!   across two independent runs; per-shard version vector; whole-read
//!   failure on any sub-read failure.
//! - SHARD-001 cluster-wide — `cluster_wide_isolation_*`: tenant A never
//!   sees tenant B on ANY replica at ANY tier, single-shard or gathered;
//!   reads change no state anywhere.

use std::cell::RefCell;
use std::rc::Rc;

use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{ContentHash, Kernel, ProposedWrite, ShardKey as KShardKey, TruthRef};
use arves_persistence::ContentId;
use arves_query::distributed::{floor_of, ClusterQuery, FloorReadError, LAG_UNATTESTABLE};
use arves_query::projection::projection_id_for;
use arves_query::{Query, QueryError, ReadScope, ReadTier, StalenessBound};

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
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
) -> TruthRef {
    let leader = cluster.borrow().leader_of(shard).expect("a leader exists");
    let k = ClusterKernel::new(leader, cluster.clone());
    k.commit(pw(&shard.tenant.0, &shard.workspace.0, content, payload))
        .expect("leader commit")
}

/// A 3-node cluster with one shard `t1/w1`, elected and settled.
fn one_shard_cluster(seed: u64) -> (Rc<RefCell<ClusterSim>>, ShardId, NodeId) {
    let shard = sid("t1", "w1");
    let mut c = ClusterSim::new(3);
    c.add_shard(shard.clone(), seed);
    let leader = c.elect(&shard);
    (Rc::new(RefCell::new(c)), shard, leader)
}

fn a_follower(cluster: &Rc<RefCell<ClusterSim>>, leader: &NodeId) -> NodeId {
    cluster
        .borrow()
        .node_ids()
        .into_iter()
        .find(|n| n != leader)
        .expect("a follower exists")
}

/// Leader-consistent reads reflect quorum-committed truth, the read-index
/// admits a CURRENT follower too (serve at >= the leader's commit index), and
/// converged replicas' reads are identical (ORCH-003 across nodes).
#[test]
fn linearizable_read_index_serves_quorum_truth_at_leader_and_current_followers() {
    let (cluster, shard, leader) = one_shard_cluster(0xC0FFEE);
    commit_at_leader(&cluster, &shard, b"e1", b"truth-one");
    commit_at_leader(&cluster, &shard, b"e2", b"truth-two");
    cluster.borrow_mut().settle(4); // heartbeats carry commit to followers

    let lin = ReadScope::linearizable("t1/w1".into());
    let mut projections = Vec::new();
    for node in cluster.borrow().node_ids() {
        // EVERY current replica passes the read-index and serves the same
        // quorum-committed truth — leader and caught-up followers alike.
        let q = ClusterQuery::new(node.clone(), cluster.clone());
        let p = q.read(&lin, &pid(b"e2")).expect("linearizable read at {node:?}");
        assert!(contains(&p.value, b"truth-two"));
        assert_eq!(p.observed_at, 2, "both offsets folded");
        assert_eq!(p.served_tier, ReadTier::Linearizable, "labeled as served");
        assert_eq!(q.latest_version(&lin, &pid(b"e1")), Ok(0));
        assert_eq!(q.latest_version(&lin, &pid(b"e2")), Ok(1));
        projections.push(p);
    }
    // Replica-equality, distributed form: identical reads on every node.
    assert!(projections.windows(2).all(|w| w[0] == w[1]));

    // MalformedScope precedes routing on the distributed path too.
    let q = ClusterQuery::new(leader, cluster.clone());
    let bad = ReadScope { shard: "t1/w1".into(), tier: ReadTier::Linearizable, bound: Some(StalenessBound::new(1)) };
    assert_eq!(q.read(&bad, &pid(b"e1")), Err(QueryError::MalformedScope));
    // Unknown shards are refused (no group, no route).
    assert_eq!(
        q.exists(&ReadScope::eventual("ghost/w".into()), &pid(b"e1")),
        Err(QueryError::UnknownShard { shard: "ghost/w".into() })
    );
}

/// THE STALENESS-HONESTY PROOF (IDR-005 CP truth / AP observability): after a
/// partition, the isolated follower keeps serving Eventual reads — visibly
/// STALE and LABELED so (old `observed_at`, `served_tier: Eventual`, new truth
/// honestly absent) — while both strong tiers refuse honestly, and the
/// majority leader's linearizable read reflects the quorum-committed truth.
/// After heal + settle, the follower converges and every tier serves again.
#[test]
fn partitioned_follower_serves_labeled_stale_eventual_and_refuses_strong_tiers() {
    let (cluster, shard, leader) = one_shard_cluster(0xBEEF);
    commit_at_leader(&cluster, &shard, b"e1", b"old-truth");
    cluster.borrow_mut().settle(4);

    let follower = a_follower(&cluster, &leader);
    cluster.borrow_mut().isolate(&shard, &follower);

    // New truth commits on the majority side while the follower is cut off.
    commit_at_leader(&cluster, &shard, b"e2", b"new-truth");
    cluster.borrow_mut().settle(4);

    let qf = ClusterQuery::new(follower.clone(), cluster.clone());
    let ev = ReadScope::eventual("t1/w1".into());

    // AP observability: the stale follower still serves — and says so.
    let stale = qf.read(&ev, &pid(b"e1")).expect("eventual read stays available");
    assert_eq!(stale.served_tier, ReadTier::Eventual, "staleness is labeled");
    assert_eq!(stale.observed_at, 1, "the fold honestly reports the OLD position");
    assert!(contains(&stale.value, b"old-truth"));
    // The new truth is honestly absent here — stale, never wrong.
    assert_eq!(qf.read(&ev, &pid(b"e2")), Err(QueryError::NotFound { id: pid(b"e2") }));

    // CP truth: the lagging follower cannot confirm currency -> refuse.
    let lin = ReadScope::linearizable("t1/w1".into());
    assert_eq!(qf.read(&lin, &pid(b"e2")), Err(QueryError::LeaderUnavailable));
    // Bounded: zero lag is unprovable and no time<->index mapping exists
    // (OQ-2) -> honest refusal with the unattestable sentinel.
    let bound = StalenessBound::new(1_000_000);
    let bs = ReadScope::bounded("t1/w1".into(), bound);
    assert_eq!(
        qf.read(&bs, &pid(b"e2")),
        Err(QueryError::StalenessBoundExceeded { requested: bound, observed_lag: LAG_UNATTESTABLE })
    );

    // Meanwhile the leader-consistent read reflects quorum-committed truth.
    let ql = ClusterQuery::new(leader.clone(), cluster.clone());
    let fresh = ql.read(&lin, &pid(b"e2")).expect("leader linearizable read");
    assert!(contains(&fresh.value, b"new-truth"));
    assert_eq!(fresh.observed_at, 2);

    // Heal: the follower catches up and EVERY tier serves the new truth.
    // (60 ticks: the isolated follower's term inflation may force a
    // re-election on heal — the same settle budget RCR-021's tests use.)
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(60);
    let caught = qf.read(&lin, &pid(b"e2")).expect("follower is current again");
    assert_eq!(caught.observed_at, 2);
    assert!(qf.read(&bs, &pid(b"e2")).is_ok(), "zero lag provable again");
    assert_eq!(qf.exists(&ev, &pid(b"e2")), Ok(true), "eventual converged");
}

/// A deposed minority leader can NEVER serve a linearizable read of the past
/// as current: the directory names the higher-term majority leader, the
/// deposed node's applied index is behind that leader's commit index, and the
/// read refuses (`LeaderUnavailable`) — while its Eventual reads keep serving
/// the old prefix, labeled honestly (CP truth vs AP observability, IDR-001).
#[test]
fn deposed_minority_leader_refuses_linearizable_but_serves_labeled_eventual() {
    let (cluster, shard, old_leader) = one_shard_cluster(0xDEAD);
    commit_at_leader(&cluster, &shard, b"e1", b"pre-partition-truth");
    cluster.borrow_mut().settle(4);

    // Partition the old leader into a minority; the majority elects anew.
    let others: Vec<NodeId> = cluster
        .borrow()
        .node_ids()
        .into_iter()
        .filter(|n| n != &old_leader)
        .collect();
    cluster
        .borrow_mut()
        .partition(&shard, &[vec![old_leader.clone()], others.clone()]);
    // Drive ticks until the majority side times out and elects at a higher
    // term; the deposed node still believes it leads its LOWER term (legal
    // Raft) — the directory names the highest-term live leader.
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("majority leader");
    assert_ne!(new_leader, old_leader, "majority elected a successor");

    // New truth commits at the new leader (quorum = the majority side).
    commit_at_leader(&cluster, &shard, b"e2", b"post-partition-truth");
    cluster.borrow_mut().settle(4);

    let q_old = ClusterQuery::new(old_leader.clone(), cluster.clone());
    let lin = ReadScope::linearizable("t1/w1".into());
    // The deposed node cannot confirm currency with the (higher-term) leader.
    assert_eq!(q_old.read(&lin, &pid(b"e2")), Err(QueryError::LeaderUnavailable));
    assert_eq!(q_old.read(&lin, &pid(b"e1")), Err(QueryError::LeaderUnavailable));
    // But its AP surface still observes the old prefix, honestly labeled.
    let ev = ReadScope::eventual("t1/w1".into());
    let stale = q_old.read(&ev, &pid(b"e1")).expect("AP read serves");
    assert_eq!((stale.served_tier, stale.observed_at), (ReadTier::Eventual, 1));

    // The new leader's linearizable read IS the quorum-committed truth.
    let q_new = ClusterQuery::new(new_leader, cluster.clone());
    let fresh = q_new.read(&lin, &pid(b"e2")).expect("new leader serves");
    assert!(contains(&fresh.value, b"post-partition-truth"));
}

/// Read-your-writes via the returned `TruthRef` (OQ-5 additive carrier): the
/// committer's floor is honored wherever the write has been applied, and a
/// lagging replica answers `BelowFloor` — never a false `NotFound` for truth
/// that exists.
#[test]
fn read_your_writes_floor_honored_at_current_replica_below_floor_at_lagging() {
    let (cluster, shard, leader) = one_shard_cluster(0xF00D);
    commit_at_leader(&cluster, &shard, b"e1", b"first");
    cluster.borrow_mut().settle(4);

    let follower = a_follower(&cluster, &leader);
    cluster.borrow_mut().isolate(&shard, &follower);
    let tr = commit_at_leader(&cluster, &shard, b"e2", b"my-write");
    let floor = floor_of(&tr);
    assert_eq!(floor, 2, "e2 sits at WAL offset 1; positions >= 2 reflect it");

    // At the leader (current): the caller provably reads its own write —
    // even on the weakest tier, because the floor does the guaranteeing.
    let ql = ClusterQuery::new(leader.clone(), cluster.clone());
    let ev = ReadScope::eventual("t1/w1".into());
    let p = ql.read_at_least(&ev, &pid(b"e2"), floor).expect("read-your-writes");
    assert!(contains(&p.value, b"my-write"));
    assert!(p.observed_at >= floor);

    // At the isolated follower: BelowFloor — explicitly NOT NotFound.
    let qf = ClusterQuery::new(follower.clone(), cluster.clone());
    assert_eq!(
        qf.read_at_least(&ev, &pid(b"e2"), floor),
        Err(FloorReadError::BelowFloor { floor: 2, applied: 1 })
    );

    // After heal + settle the same floor is satisfied at the follower.
    cluster.borrow_mut().heal(&shard);
    cluster.borrow_mut().settle(60);
    let caught = qf.read_at_least(&ev, &pid(b"e2"), floor).expect("floor reached");
    assert!(contains(&caught.value, b"my-write"));

    // On a fold that HAS reached the floor, a missing id is honest NotFound
    // (the floor speaks first only while the fold is behind it).
    assert_eq!(
        qf.read_at_least(&ev, &pid(b"never"), floor),
        Err(FloorReadError::Query(QueryError::NotFound { id: pid(b"never") }))
    );
}

/// A 3-node cluster with shards acme/w1, acme/w2 and globex/w1, fixed seeds,
/// fixed commits — the multi-shard fixture for scatter-gather and isolation.
fn multi_shard_cluster() -> (Rc<RefCell<ClusterSim>>, ShardId, ShardId, ShardId) {
    let (a1, a2, g1) = (sid("acme", "w1"), sid("acme", "w2"), sid("globex", "w1"));
    let mut c = ClusterSim::new(3);
    c.add_shard(a1.clone(), 0xA1);
    c.add_shard(a2.clone(), 0xA2);
    c.add_shard(g1.clone(), 0x61);
    c.elect(&a1);
    c.elect(&a2);
    c.elect(&g1);
    let cluster = Rc::new(RefCell::new(c));
    // The same content-addressed document lives in BOTH acme workspaces
    // (same id, per-workspace payloads), plus one w2-only document and one
    // globex document that must never leak into an acme read.
    commit_at_leader(&cluster, &a1, b"doc", b"acme-w1-copy");
    commit_at_leader(&cluster, &a2, b"doc", b"acme-w2-copy");
    commit_at_leader(&cluster, &a2, b"only2", b"acme-w2-only");
    commit_at_leader(&cluster, &g1, b"gdoc", b"globex-secret");
    cluster.borrow_mut().settle(6);
    (cluster, a1, a2, g1)
}

/// Scatter-gather: tenant-internal fan-out with a per-shard version vector,
/// deterministic merge order, BIT-EQUAL results across two independent runs
/// of the whole cluster, and fail-WHOLE on any sub-read failure (OQ-4).
#[test]
fn scatter_gather_is_deterministic_non_atomic_union_and_fails_whole() {
    let run = || {
        let (cluster, ..) = multi_shard_cluster();
        let node = cluster.borrow().node_ids()[0].clone();
        let q = ClusterQuery::new(node, cluster.clone());
        (
            q.gather_read("acme", ReadTier::Linearizable, None, &pid(b"doc")).expect("gather doc"),
            q.gather_read("acme", ReadTier::Linearizable, None, &pid(b"only2")).expect("gather only2"),
        )
    };
    let (doc_a, only2_a) = run();
    let (doc_b, only2_b) = run();
    // Deterministic across runs: same seeds, same merged result, bit-equal.
    assert_eq!(doc_a, doc_b);
    assert_eq!(only2_a, only2_b);

    // Non-atomic union shape: BOTH acme shards contribute a part for `doc`,
    // each carrying its OWN version — no fabricated global version exists.
    assert_eq!(doc_a.served_tier, ReadTier::Linearizable);
    assert_eq!(
        doc_a.versions.keys().collect::<Vec<_>>(),
        vec!["acme/w1", "acme/w2"],
        "deterministic ascending merge order; globex is structurally absent"
    );
    assert_eq!(doc_a.parts.len(), 2);
    assert!(contains(&doc_a.parts["acme/w1"].value, b"acme-w1-copy"));
    assert!(contains(&doc_a.parts["acme/w2"].value, b"acme-w2-copy"));
    // `only2` exists in w2 alone: w1 contributes to the version vector only.
    assert_eq!(only2_a.versions.len(), 2);
    assert_eq!(only2_a.parts.keys().collect::<Vec<_>>(), vec!["acme/w2"]);

    // OQ-4 (fail-whole): lag the gathering node on acme/w1, then a
    // linearizable gather fails ENTIRELY — no silent partial union — even
    // though acme/w2 alone could still serve.
    let (cluster, a1, ..) = multi_shard_cluster();
    let w1_leader = cluster.borrow().leader_of(&a1).expect("leader");
    let reader = a_follower(&cluster, &w1_leader);
    cluster.borrow_mut().isolate(&a1, &reader);
    commit_at_leader(&cluster, &a1, b"late", b"post-isolation");
    cluster.borrow_mut().settle(4);
    let q = ClusterQuery::new(reader, cluster.clone());
    assert_eq!(
        q.gather_read("acme", ReadTier::Linearizable, None, &pid(b"doc")),
        Err(QueryError::LeaderUnavailable),
        "one lagging shard fails the whole gather (never a silent partial)"
    );
    // An unknown tenant has no route at all.
    assert_eq!(
        q.gather_read("initech", ReadTier::Eventual, None, &pid(b"doc")),
        Err(QueryError::UnknownShard { shard: "initech".into() })
    );
}

/// SHARD-001 cluster-wide: on EVERY replica and EVERY tier, an acme-scoped
/// read never returns globex's payload, id-existence, or bytes — single-shard
/// and gathered alike — and the whole barrage changes no state anywhere
/// (Layer Matrix "Writes: NOTHING" / ORCH-004 idempotent reads).
#[test]
fn cluster_wide_isolation_on_every_replica_and_tier_and_reads_write_nothing() {
    let (cluster, ..) = multi_shard_cluster();
    let counts_before: Vec<usize> = {
        let c = cluster.borrow();
        c.node_ids().iter().map(|n| c.committed_count_of(n)).collect()
    };

    let scopes = [
        ReadScope::linearizable("acme/w1".into()),
        ReadScope::bounded("acme/w1".into(), StalenessBound::new(0)),
        ReadScope::eventual("acme/w1".into()),
    ];
    for node in cluster.borrow().node_ids() {
        let q = ClusterQuery::new(node.clone(), cluster.clone());
        for scope in &scopes {
            let p = q.read(scope, &pid(b"doc")).expect("acme read at {node:?}");
            assert!(contains(&p.value, b"acme-w1-copy"));
            assert!(!contains(&p.value, b"globex-secret"), "no foreign bytes");
            assert_eq!(p.served_tier, scope.tier, "never stronger than requested");
            // Globex's document does not exist inside acme's scope.
            assert_eq!(q.exists(scope, &pid(b"gdoc")), Ok(false));
            assert_eq!(
                q.read(scope, &pid(b"gdoc")),
                Err(QueryError::NotFound { id: pid(b"gdoc") })
            );
        }
        // The gathered union is tenant-pure in both directions.
        let acme = q.gather_read("acme", ReadTier::Eventual, None, &pid(b"doc")).expect("gather");
        assert!(acme.versions.keys().all(|k| k.starts_with("acme/")));
        assert!(acme.parts.values().all(|p| !contains(&p.value, b"globex-secret")));
        let globex = q.gather_read("globex", ReadTier::Eventual, None, &pid(b"gdoc")).expect("gather");
        assert_eq!(globex.parts.keys().collect::<Vec<_>>(), vec!["globex/w1"]);
        assert!(globex.parts.values().all(|p| !contains(&p.value, b"acme-w1-copy")));
    }

    // The read barrage mutated NOTHING on any replica.
    let c = cluster.borrow();
    let counts_after: Vec<usize> =
        c.node_ids().iter().map(|n| c.committed_count_of(n)).collect();
    assert_eq!(counts_before, counts_after, "reads change no truth anywhere");
}

/// REGRESSION (RCR-024 DR-8, adversarial-hunt blocker): the Raft §6.4
/// read-index PRECONDITION. Schedule: e2 is committed and ACKED at leader A;
/// A is isolated BEFORE the commit-index heartbeat reaches the followers; the
/// majority elects B at a higher term. B's log CONTAINS e2 (Leader
/// Completeness) but B's commit index still ends in the PRIOR term — the
/// §5.4.2 term guard (and RCR-019 DR-2's deliberate no-election-no-op) keeps
/// e2 out of it until a current-term entry commits. Before the fix,
/// Linearizable and zero-lag Bounded reads at B were ADMITTED on that invalid
/// read-index and silently MISSED the acked write (NotFound/false labeled
/// Linearizable). Now both strong tiers REFUSE until B commits in its own
/// term, after which the acked write is provably visible.
#[test]
fn new_leader_without_current_term_commit_refuses_strong_reads_until_it_commits() {
    let (cluster, shard, old_leader) = one_shard_cluster(0x5EED);
    commit_at_leader(&cluster, &shard, b"e1", b"first-truth");
    cluster.borrow_mut().settle(4);
    // e2: quorum-committed and ACKED at leader A...
    commit_at_leader(&cluster, &shard, b"e2", b"acked-write");
    // ...but A is cut off BEFORE any heartbeat can advertise commit=2.
    cluster.borrow_mut().isolate(&shard, &old_leader);
    cluster.borrow_mut().settle(60);
    let new_leader = cluster.borrow().leader_of(&shard).expect("majority elects");
    assert_ne!(new_leader, old_leader, "a successor leads at a higher term");
    // The hazard is real in this schedule: the new leader has NO committed
    // entry of its current term (its commit index predates the acked e2).
    assert!(
        !cluster.borrow().has_committed_in_current_term(&new_leader, &shard),
        "precondition schedule: the new leader's commit index ends in a prior term"
    );

    let q = ClusterQuery::new(new_leader.clone(), cluster.clone());
    let lin = ReadScope::linearizable("t1/w1".into());
    // Linearizable refuses — it must NEVER answer NotFound for the acked e2.
    assert_eq!(q.read(&lin, &pid(b"e2")), Err(QueryError::LeaderUnavailable));
    assert_eq!(q.exists(&lin, &pid(b"e2")), Err(QueryError::LeaderUnavailable));
    // Zero-lag Bounded refuses too: no valid read-index, no lag proof.
    let bound = StalenessBound::new(0);
    let bs = ReadScope::bounded("t1/w1".into(), bound);
    assert_eq!(
        q.read(&bs, &pid(b"e2")),
        Err(QueryError::StalenessBoundExceeded { requested: bound, observed_lag: LAG_UNATTESTABLE })
    );
    // AP observability stays available and labeled (IDR-005) — and is never
    // wrong for the version it reports.
    let ev = ReadScope::eventual("t1/w1".into());
    let stale = q.read(&ev, &pid(b"e1")).expect("eventual serves");
    assert_eq!(stale.served_tier, ReadTier::Eventual);

    // A current-term commit at the new leader validates its read-index; the
    // previously acked write is then provably included — never silently lost.
    commit_at_leader(&cluster, &shard, b"e3", b"current-term-entry");
    cluster.borrow_mut().settle(4);
    assert!(cluster.borrow().has_committed_in_current_term(&new_leader, &shard));
    let q = ClusterQuery::new(new_leader, cluster.clone());
    let p = q.read(&lin, &pid(b"e2")).expect("acked write now provably served");
    assert!(contains(&p.value, b"acked-write"));
    assert!(q.read(&bs, &pid(b"e2")).is_ok(), "zero lag provable again");
}

/// REGRESSION (RCR-024 DR-9): gather sub-reads route on the TYPED `ShardId`,
/// never on the re-parsed text form. The kernel `ShardKey` legally permits
/// `/` inside a part (RCR-023 DR-2), so tenant `a/b` + workspace `c` and
/// tenant `a` + workspace `b/c` share the ambiguous text `"a/b/c"`. Before
/// the fix, the fan-out rebuilt the text and re-parsed it at the first `/`,
/// serving tenant `a`'s bytes inside a gather labeled for tenant `a/b`.
#[test]
fn gather_routes_on_typed_shard_identity_never_reparsed_text() {
    let slashy = sid("a/b", "c"); // adversarial: '/' inside the tenant part
    let plain = sid("a", "b/c"); // same text form "a/b/c", different tenant
    let mut c = ClusterSim::new(3);
    c.add_shard(slashy.clone(), 0x51);
    c.add_shard(plain.clone(), 0x52);
    c.elect(&slashy);
    c.elect(&plain);
    let cluster = Rc::new(RefCell::new(c));
    commit_at_leader(&cluster, &slashy, b"d", b"slash-tenant-bytes");
    commit_at_leader(&cluster, &plain, b"d", b"plain-tenant-bytes");
    cluster.borrow_mut().settle(6);

    let node = cluster.borrow().node_ids()[0].clone();
    let q = ClusterQuery::new(node, cluster.clone());
    // A gather labeled for tenant "a/b" carries ONLY tenant "a/b"'s shard.
    let g = q.gather_read("a/b", ReadTier::Eventual, None, &pid(b"d")).expect("gather a/b");
    assert_eq!(g.parts.len(), 1, "exactly the one registered shard of tenant a/b");
    assert!(g.parts.values().all(|p| contains(&p.value, b"slash-tenant-bytes")));
    assert!(
        g.parts.values().all(|p| !contains(&p.value, b"plain-tenant-bytes")),
        "tenant a's bytes must never appear under the tenant-a/b label"
    );
    // And the converse: tenant "a"'s gather never carries tenant "a/b" bytes.
    let g = q.gather_read("a", ReadTier::Eventual, None, &pid(b"d")).expect("gather a");
    assert_eq!(g.parts.len(), 1);
    assert!(g.parts.values().all(|p| contains(&p.value, b"plain-tenant-bytes")));
    assert!(g.parts.values().all(|p| !contains(&p.value, b"slash-tenant-bytes")));
}

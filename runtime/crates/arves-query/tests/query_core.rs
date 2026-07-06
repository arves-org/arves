//! RCR-023 (I3 Stage 1) — executable proofs for the single-node QUERY CORE.
//!
//! Every test is deterministic: no wall clocks, no OS randomness, fixed
//! inputs, in-memory store. Truths enter ONLY through the real Kernel commit
//! gateway (`arves-kernel` is a dev-dependency — a test-only edge, not an
//! architectural one); the query core then reconstructs them by WAL replay
//! (ORCH-001: committed truth only; no Kernel read hook exists or is used).
//!
//! Proof map (design §4 rows, Stage-1 form):
//! - SHARD-001 — `shard001_*`: tenant A never sees tenant B on any tier;
//!   foreign records structurally never enter a fold.
//! - ORCH-003 — `orch003_*`: projection fold digest equals the Kernel's
//!   `truth_hash` basis (incl. across a Kernel recover); independent builds at
//!   the same version are equal; checkpoint ⊕ suffix ≡ full replay.
//! - OWN-001 / ORCH-004 — `read_only_*`: reads mutate nothing anywhere (WAL
//!   head + truth_hash invariant) and identical reads return identical
//!   projections.
//! - IDR-001 tiers (single-node degenerate) — `tier_*`: Eventual is honestly
//!   stale, Linearizable/BoundedStaleness catch up; `served_tier` is never
//!   stronger than requested; MalformedScope precedes all I/O.

use arves_kernel::{ContentHash, Kernel, MemKernel, ProposedWrite, ShardKey as KShardKey};
use arves_persistence::{ContentId, MemWalStore, ShardKey as WalShardKey, Wal, WalStore};
use arves_query::projection::{projection_id_for, shard_scope_text, ShardProjection, WalQuery};
use arves_query::{Query, QueryError, ReadScope, ReadTier, StalenessBound};

fn kshard(t: &str, w: &str) -> KShardKey {
    KShardKey::new(t, w).expect("valid shard")
}
fn wshard(t: &str, w: &str) -> WalShardKey {
    WalShardKey { tenant: t.into(), workspace: w.into() }
}
fn commit(k: &MemKernel, t: &str, w: &str, content: &[u8], payload: &[u8]) {
    k.commit(ProposedWrite {
        shard: kshard(t, w),
        content: ContentHash(content.to_vec()),
        payload: payload.to_vec(),
    })
    .expect("commit");
}
fn pid(content: &[u8]) -> String {
    projection_id_for(&ContentId(content.to_vec()))
}
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// Two tenants, one store: an acme-scoped read on EVERY tier never returns
/// globex's payload, ids, or existence; unknown/unparseable shards are refused.
#[test]
fn shard001_tenant_a_never_sees_tenant_b_on_any_tier() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    commit(&k, "acme", "research", b"a1", b"acme-truth");
    commit(&k, "globex", "research", b"g1", b"globex-truth");

    let q = WalQuery::new(store.clone());
    let scopes = [
        ReadScope::linearizable("acme/research".into()),
        ReadScope::bounded("acme/research".into(), StalenessBound::new(0)),
        ReadScope::eventual("acme/research".into()),
    ];
    for scope in &scopes {
        // Acme's truth is served, and its payload carries no globex bytes.
        let p = q.read(scope, &pid(b"a1")).expect("acme read");
        assert!(contains(&p.value, b"acme-truth"));
        assert!(!contains(&p.value, b"globex-truth"));
        assert_eq!(p.served_tier, scope.tier); // never stronger than requested
        // Globex's projection id does NOT exist inside acme's scope.
        assert_eq!(
            q.read(scope, &pid(b"g1")),
            Err(QueryError::NotFound { id: pid(b"g1") })
        );
        assert_eq!(q.exists(scope, &pid(b"g1")), Ok(false));
    }
    // Unknown and unparseable shards are refused (SHARD-001 scoping).
    assert_eq!(
        q.read(&ReadScope::eventual("initech/research".into()), &pid(b"a1")),
        Err(QueryError::UnknownShard { shard: "initech/research".into() })
    );
    assert_eq!(
        q.read(&ReadScope::eventual("no-separator".into()), &pid(b"a1")),
        Err(QueryError::UnknownShard { shard: "no-separator".into() })
    );
    // And the fold itself holds ONLY acme rows.
    let proj = ShardProjection::at_head(&store, &wshard("acme", "research")).expect("fold");
    assert_eq!(proj.len(), 1);
    assert!(proj.get(&pid(b"g1")).is_none());
}

/// The projection is the committed truth basis: its fold digest equals the
/// Kernel's `truth_hash` on a dedicated single-shard store — before AND after
/// a Kernel restart/recover (both sides are pure functions of the same WAL).
#[test]
fn orch003_fold_digest_equals_kernel_truth_hash_basis() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    commit(&k, "acme", "research", b"c1", b"truth-one");
    commit(&k, "acme", "research", b"c2", b"truth-two");
    commit(&k, "acme", "research", b"c3", b"truth-three");

    let proj = ShardProjection::at_head(&store, &wshard("acme", "research")).expect("fold");
    assert_eq!(proj.len(), 3);
    assert_eq!(proj.fold_digest(), k.truth_hash(), "projection == kernel truth basis");

    // Kernel restart: recover replays the same WAL; the basis must not move.
    drop(k);
    let recovered = MemKernel::recover(store.clone());
    assert_eq!(proj.fold_digest(), recovered.truth_hash());

    // Replica equality (ORCH-003): an independent build over the same prefix
    // is EQUAL, not merely digest-equal.
    let again = ShardProjection::at_head(&store, &wshard("acme", "research")).expect("fold");
    assert_eq!(proj, again);
}

/// Deterministic snapshot-at-index reads: two builds pinned at version v are
/// equal; a pinned fold caught up to head equals a fresh full replay
/// (checkpoint ⊕ suffix ≡ full replay, design §3.11 Stage-1 form); a pinned
/// fold does not see later commits.
#[test]
fn orch003_snapshot_at_index_deterministic_and_suffix_equivalent() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    for (c, p) in [(b"k1", b"v1"), (b"k2", b"v2"), (b"k3", b"v3"), (b"k4", b"v4")] {
        commit(&k, "acme", "research", c, p);
    }
    let sh = wshard("acme", "research");

    let p2a = ShardProjection::at_version(&store, &sh, 2).expect("pin at 2");
    let p2b = ShardProjection::at_version(&store, &sh, 2).expect("pin at 2");
    assert_eq!(p2a, p2b, "same prefix, same fold — deterministic");
    assert_eq!(p2a.applied(), 2);
    assert_eq!(p2a.len(), 2);
    // The pinned fold reflects exactly WAL[0..2): k3 is invisible at v=2.
    assert!(p2a.get(&pid(b"k3")).is_none());
    assert_eq!(p2a.latest(&pid(b"k2")), Some(1)); // commit offset of k2

    // checkpoint ⊕ suffix ≡ full replay.
    let mut caught = p2a.clone();
    caught.catch_up(&store).expect("suffix");
    let full = ShardProjection::at_head(&store, &sh).expect("full");
    assert_eq!(caught, full);
    assert_eq!(caught.fold_digest(), full.fold_digest());
    assert_eq!(full.applied(), 4);
}

/// Single-node tier semantics, honestly: Eventual serves the standing fold
/// (observably stale after new commits), Linearizable and BoundedStaleness
/// catch up to the committed head; `served_tier` equals the requested tier.
#[test]
fn tier_semantics_eventual_is_stale_strong_tiers_catch_up() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    commit(&k, "acme", "research", b"t1", b"first");

    let q = WalQuery::new(store.clone());
    // Bootstrap the shard fold at head=1 via a linearizable read.
    let p1 = q.read(&ReadScope::linearizable("acme/research".into()), &pid(b"t1")).expect("read");
    assert_eq!(p1.observed_at, 1);
    assert_eq!(p1.served_tier, ReadTier::Linearizable);

    // New truth lands AFTER the bootstrap.
    commit(&k, "acme", "research", b"t2", b"second");

    // Eventual: the standing fold is served WITHOUT refresh — t2 is honestly
    // not visible yet, and the reported version says so (stale, never wrong).
    let ev = ReadScope::eventual("acme/research".into());
    assert_eq!(q.read(&ev, &pid(b"t2")), Err(QueryError::NotFound { id: pid(b"t2") }));
    let p_old = q.read(&ev, &pid(b"t1")).expect("stale read");
    assert_eq!(p_old.observed_at, 1, "eventual read reports the stale trace position");
    assert_eq!(p_old.served_tier, ReadTier::Eventual);

    // BoundedStaleness (bound 0ms): the single-node core attests lag 0 by
    // catching up to its own head, then serves the new truth.
    let bs = ReadScope::bounded("acme/research".into(), StalenessBound::new(0));
    let p2 = q.read(&bs, &pid(b"t2")).expect("bounded read");
    assert_eq!(p2.observed_at, 2);
    assert_eq!(p2.served_tier, ReadTier::BoundedStaleness);
    assert_eq!(q.latest_version(&bs, &pid(b"t2")), Ok(1)); // commit offset of t2

    // After the refresh, Eventual sees the advanced fold (bootstrap-then-lag).
    assert_eq!(q.exists(&ev, &pid(b"t2")), Ok(true));
}

/// An internally inconsistent scope is rejected BEFORE any I/O — even the
/// shard string (garbage here) is never consulted.
#[test]
fn malformed_scope_is_rejected_before_routing() {
    let q = WalQuery::new(MemWalStore::new());
    // BoundedStaleness without a bound.
    let s1 = ReadScope {
        shard: "not-even-a-shard".into(),
        tier: ReadTier::BoundedStaleness,
        bound: None,
    };
    assert_eq!(q.read(&s1, &"x".to_string()), Err(QueryError::MalformedScope));
    // Linearizable / Eventual carrying a bound.
    for tier in [ReadTier::Linearizable, ReadTier::Eventual] {
        let s = ReadScope {
            shard: "not-even-a-shard".into(),
            tier,
            bound: Some(StalenessBound::new(5)),
        };
        assert_eq!(q.exists(&s, &"x".to_string()), Err(QueryError::MalformedScope));
    }
}

/// Reads mutate NOTHING anywhere (OWN-001 / Layer Matrix "Writes: NOTHING";
/// ORCH-004 idempotency): after a barrage of reads on every tier, the WAL head
/// and the Kernel truth_hash are unchanged, and identical reads return
/// identical projections.
#[test]
fn read_only_reads_change_no_state_and_are_idempotent() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    commit(&k, "acme", "research", b"r1", b"stable-truth");

    let sh = wshard("acme", "research");
    let head_before = store.open(&sh).expect("wal").head();
    let hash_before = k.truth_hash();

    let q = WalQuery::new(store.clone());
    let scopes = [
        ReadScope::linearizable(shard_scope_text(&sh)),
        ReadScope::bounded(shard_scope_text(&sh), StalenessBound::new(0)),
        ReadScope::eventual(shard_scope_text(&sh)),
    ];
    let mut first: Option<arves_query::Projection<Vec<u8>>> = None;
    for _ in 0..10 {
        for scope in &scopes {
            let p = q.read(scope, &pid(b"r1")).expect("read");
            assert!(contains(&p.value, b"stable-truth"));
            let _ = q.exists(scope, &pid(b"r1")).expect("exists");
            let _ = q.latest_version(scope, &pid(b"r1")).expect("latest");
            if scope.tier == ReadTier::Linearizable {
                match &first {
                    None => first = Some(p),
                    Some(f) => assert_eq!(&p, f, "identical reads, identical projections"),
                }
            }
        }
        // Missing ids and unknown shards are also pure observations.
        let _ = q.read(&scopes[2], &pid(b"missing"));
        let _ = q.read(&ReadScope::eventual("ghost/ws".into()), &pid(b"r1"));
    }

    assert_eq!(store.open(&sh).expect("wal").head(), head_before, "WAL untouched by reads");
    assert_eq!(k.truth_hash(), hash_before, "truth untouched by reads");
}

/// Contract edges: `exists` on a missing id is `Ok(false)` (not an error);
/// `read`/`latest_version` on a missing id are `NotFound`; observed_at is the
/// trace position while latest_version is the entry's commit offset (< it).
#[test]
fn contract_edges_not_found_exists_false_version_vocabulary() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    commit(&k, "acme", "research", b"e1", b"one");
    commit(&k, "acme", "research", b"e2", b"two");

    let q = WalQuery::new(store.clone());
    let lin = ReadScope::linearizable("acme/research".into());
    assert_eq!(q.exists(&lin, &pid(b"nope")), Ok(false));
    assert_eq!(
        q.latest_version(&lin, &pid(b"nope")),
        Err(QueryError::NotFound { id: pid(b"nope") })
    );
    let p = q.read(&lin, &pid(b"e1")).expect("read");
    assert_eq!(p.observed_at, 2, "trace position: two offsets folded");
    assert_eq!(q.latest_version(&lin, &pid(b"e1")), Ok(0), "commit offset of e1");
    assert_eq!(q.latest_version(&lin, &pid(b"e2")), Ok(1), "commit offset of e2");
}

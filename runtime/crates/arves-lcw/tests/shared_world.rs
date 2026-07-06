//! RCR-029 (I5 Stage 1) — executable proofs for the LCW SHARED-TRUTH SURFACE.
//!
//! Design obligations discharged here (docs/design/I5_MultiAgent_Runtime_Design.md):
//! the coherent, versioned world view agents share (§3.1.2 / §5.3 LCW node
//! evidence "consistent world view"), working memory as derived, disposable,
//! never-truth state (§3.6 class 2, §3.10 rebuild-from-truth), and the frozen
//! `WorkingMemory`/`LiveWorkspace` contracts implemented for the first time.
//!
//! Every test is deterministic: fixed seeds, logical ticks, scripted elections
//! — zero wall clocks, zero OS randomness, zero sleeps. Commits are produced
//! ONLY through the frozen Kernel gateway (single-node `RefKernel` or the I2
//! `ClusterSim` leader) — the view is proven against real committed truth.
//! HONEST SCOPE: in-process simulation; no network; the "agents" implied by
//! the multi-view tests are deterministic test actors, NOT AI models.

use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{ContentHash, Kernel, MemKernel, ProposedWrite, RefKernel, ShardKey as KShardKey};
use arves_lcw::world::{MemLiveWorkspace, MemWorkingMemory, WorldError, WorldView};
use arves_lcw::{
    LcwError, LiveValue, LiveWorkspace, PutCondition, Revision, ShardKey, StateKey, WorkingMemory,
};
use arves_persistence::MemWalStore;
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Deterministic scaffolding
// ---------------------------------------------------------------------------

fn lshard(t: &str, w: &str) -> ShardKey {
    ShardKey { tenant: t.into(), workspace: w.into() }
}

fn kshard(t: &str, w: &str) -> KShardKey {
    KShardKey::new(t, w).expect("well-formed test shard")
}

/// Commit `payload` through the frozen single-node gateway (content address =
/// payload-derived bytes; distinct payloads => distinct addresses).
fn commit(kernel: &MemKernel, shard: &KShardKey, tag: u8, payload: &[u8]) {
    let mut content = vec![tag];
    content.extend_from_slice(payload);
    kernel
        .commit(ProposedWrite {
            shard: shard.clone(),
            content: ContentHash(content),
            payload: payload.to_vec(),
        })
        .expect("test commit");
}

// ---------------------------------------------------------------------------
// 1. Frozen WorkingMemory contract semantics (first implementation)
// ---------------------------------------------------------------------------

#[test]
fn working_memory_conditions_revisions_and_eviction_are_deterministic() {
    let sh = lshard("acme", "research");
    let mut wm = MemWorkingMemory::new(sh.clone());
    let k = StateKey("cell-a".into());

    // Never-written cell: NotFound; contains false; revision vocabulary ZERO.
    assert_eq!(wm.get(&k), Err(LcwError::NotFound(k.clone())));
    assert!(!wm.contains(&k));

    // First write lands at revision 1 (ZERO.next()).
    let r1 = wm.put(&k, LiveValue::new(b"v1".to_vec()), PutCondition::Always).unwrap();
    assert_eq!(r1, Revision(1));
    assert_eq!(wm.get(&k).unwrap().value.bytes, b"v1");

    // IfAbsent on an existing cell: SKIPPED — Ok(current revision), no mutation
    // (RCR-029 DR-5: the frozen contract's "write is skipped" reading).
    let r_skip = wm.put(&k, LiveValue::new(b"vX".to_vec()), PutCondition::IfAbsent).unwrap();
    assert_eq!(r_skip, Revision(1));
    assert_eq!(wm.get(&k).unwrap().value.bytes, b"v1", "IfAbsent never overwrote");

    // IfRevision mismatch: RevisionConflict carrying expected vs actual.
    let err = wm
        .put(&k, LiveValue::new(b"v2".to_vec()), PutCondition::IfRevision(Revision(7)))
        .unwrap_err();
    assert_eq!(
        err,
        LcwError::RevisionConflict { key: k.clone(), expected: Revision(7), actual: Revision(1) }
    );

    // IfRevision match: write proceeds, revision increments locally.
    let r2 = wm.put(&k, LiveValue::new(b"v2".to_vec()), PutCondition::IfRevision(Revision(1))).unwrap();
    assert_eq!(r2, Revision(2));

    // Evict: true once, false after; cell truly gone.
    assert_eq!(wm.evict(&k), Ok(true));
    assert_eq!(wm.evict(&k), Ok(false));
    assert_eq!(wm.get(&k), Err(LcwError::NotFound(k.clone())));

    // The binding is immutable (SHARD-001): the view reports its one shard.
    assert_eq!(wm.shard(), &sh);
}

#[test]
fn live_workspace_enforces_one_active_view_per_shard_own001() {
    let ws = MemLiveWorkspace::new();
    let a = lshard("acme", "research");
    let b = lshard("globex", "research");

    // First open per shard succeeds; a SECOND concurrent owner is refused
    // (the frozen contract's single-owner rule, enforced not just documented).
    let view_a = ws.open(&a).expect("first open");
    assert_eq!(view_a.shard(), &a);
    assert_eq!(ws.open(&a).unwrap_err(), LcwError::AlreadyOpen { shard: a.clone() });

    // A DIFFERENT shard is unaffected (SHARD-001 scoping of ownership).
    let _view_b = ws.open(&b).expect("other shard opens independently");

    // Discard releases the slot; reopening is lawful (working memory is
    // disposable — nothing durable was lost, ORCH-001/Persistence split).
    ws.discard(&a).expect("discard");
    let _view_a2 = ws.open(&a).expect("reopen after discard");

    // Discarding a never-opened shard is a lawful no-op.
    ws.discard(&lshard("initech", "ops")).expect("no-op discard");
}

// ---------------------------------------------------------------------------
// 2. WorldView coherence: identical across re-reads, stable at its version
// ---------------------------------------------------------------------------

#[test]
fn world_view_at_a_version_is_identical_across_rereads_and_stable_under_later_commits() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");

    commit(&kernel, &ks, 1, b"fact-1");
    commit(&kernel, &ks, 2, b"fact-2");
    commit(&kernel, &ks, 3, b"fact-3");

    // Re-read equality: two independent builds at the same commit index are
    // EQUAL — structurally and by digest (the coherence property).
    let v2a = WorldView::at_version(&store, &ls, 2).unwrap();
    let v2b = WorldView::at_version(&store, &ls, 2).unwrap();
    assert_eq!(v2a, v2b);
    assert_eq!(v2a.world_digest(), v2b.world_digest());
    assert_eq!(v2a.observed_at(), 2);
    assert_eq!(v2a.len(), 2);

    // Different versions are different worlds (versioned, never ambient).
    let v3 = WorldView::at_version(&store, &ls, 3).unwrap();
    assert_ne!(v2a, v3);
    assert_eq!(v3.len(), 3);

    // Stability: MORE commits landing does not move an already-taken version —
    // a rebuilt view at 2 is byte-equal to the pre-commit build.
    commit(&kernel, &ks, 4, b"fact-4");
    let v2c = WorldView::at_version(&store, &ls, 2).unwrap();
    assert_eq!(v2a, v2c, "a versioned view is stable at its version");

    // at_head reflects everything committed so far.
    let head = WorldView::at_head(&store, &ls).unwrap();
    assert_eq!(head.observed_at(), 4);
    assert_eq!(head.len(), 4);

    // The digest basis IS the Kernel's truth basis: for this single-shard node
    // the world digest equals the committing Kernel's truth_hash — the shared
    // world is committed truth, nothing else (OWN-001).
    assert_eq!(head.world_digest(), kernel.truth_hash());
}

#[test]
fn world_view_refusals_are_loud_never_partial() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");
    commit(&kernel, &ks, 1, b"fact-1");

    // Unknown shard: refused (SHARD-001 — never a cross-shard fallback).
    let ghost = lshard("ghost", "ws");
    assert_eq!(
        WorldView::at_version(&store, &ghost, 0).unwrap_err(),
        WorldError::UnknownShard(ghost.clone())
    );

    // Beyond the committed head: the trace does not reach that point —
    // refused loudly, never a silently shorter world.
    assert_eq!(
        WorldView::at_version(&store, &ls, 99).unwrap_err(),
        WorldError::BeyondHead { shard: ls.clone(), requested: 99, head: 1 }
    );

    // Version 0 is the lawful empty world.
    let v0 = WorldView::at_version(&store, &ls, 0).unwrap();
    assert!(v0.is_empty());
    assert_eq!(v0.observed_at(), 0);
}

// ---------------------------------------------------------------------------
// 3. Coherence ACROSS REPLICAS (the I2 cluster substrate)
// ---------------------------------------------------------------------------

#[test]
fn world_view_at_commit_index_n_is_identical_on_every_replica() {
    use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};

    let sid = ShardId::new(TenantId("acme".into()), WorkspaceId("research".into()));
    let sim = Rc::new(RefCell::new(ClusterSim::new(3)));
    sim.borrow_mut().add_shard(sid.clone(), 7); // fixed seed: recorded => replayable
    let leader: NodeId = sim.borrow_mut().elect(&sid);

    // Commit three truths through the leader's frozen gateway (quorum-acked).
    let gateway = ClusterKernel::new(leader, sim.clone());
    let ks = kshard("acme", "research");
    for (tag, payload) in [(1u8, b"fact-1".as_slice()), (2, b"fact-2"), (3, b"fact-3")] {
        let mut content = vec![tag];
        content.extend_from_slice(payload);
        gateway
            .commit(ProposedWrite {
                shard: ks.clone(),
                content: ContentHash(content),
                payload: payload.to_vec(),
            })
            .expect("leader commit");
    }
    sim.borrow_mut().settle(50); // flush follower applies (logical ticks)

    // The Stage-1 coherence proof: at EVERY commit index N, the world view is
    // IDENTICAL on every replica — same entries, same digest — and identical
    // to a re-read on the same replica.
    let ls = lshard("acme", "research");
    let nodes = sim.borrow().node_ids();
    for n in 0..=3u64 {
        let views: Vec<WorldView> = nodes
            .iter()
            .map(|node| {
                let store = sim.borrow().wal_store_of(node);
                WorldView::at_version(&store, &ls, n).expect("replica view")
            })
            .collect();
        for v in &views[1..] {
            assert_eq!(&views[0], v, "view at commit index {n} diverged across replicas");
            assert_eq!(views[0].world_digest(), v.world_digest());
        }
        // Re-read on the first replica: equal again (pure function of the log).
        let store0 = sim.borrow().wal_store_of(&nodes[0]);
        assert_eq!(views[0], WorldView::at_version(&store0, &ls, n).unwrap());
        assert_eq!(views[0].len() as u64, n, "each commit index adds exactly one truth here");
    }
}

// ---------------------------------------------------------------------------
// 4. Hydration: rebuilt from truth; divergence never flows back (not truth)
// ---------------------------------------------------------------------------

#[test]
fn hydrated_working_memory_is_derived_disposable_and_never_writes_back() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    let ks = kshard("acme", "research");
    let ls = lshard("acme", "research");
    commit(&kernel, &ks, 1, b"fact-1");
    commit(&kernel, &ks, 2, b"fact-2");

    let view = WorldView::at_head(&store, &ls).unwrap();
    let truth_before = kernel.truth_hash();

    // Two agents (deterministic test actors, NOT AI models) hydrate their own
    // working memories from the SAME (shard, version): byte-identical worlds.
    let ws = MemLiveWorkspace::new();
    let mut wm_a = ws.open(&ls).unwrap();
    assert_eq!(view.hydrate_into(&mut wm_a).unwrap(), 2);
    let mut wm_b = MemWorkingMemory::new(ls.clone());
    assert_eq!(view.hydrate_into(&mut wm_b).unwrap(), 2);
    for (id, payload, _at) in view.iter() {
        let key = StateKey(id.to_string());
        assert_eq!(wm_a.get(&key).unwrap().value.bytes, payload);
        assert_eq!(wm_b.get(&key).unwrap().value.bytes, payload);
    }

    // Working memory is live and mutable — an agent scribbles freely...
    let (first_id, _, _) = view.iter().next().unwrap();
    let first_key = StateKey(first_id.to_string());
    wm_a.put(&first_key, LiveValue::new(b"speculative-overwrite".to_vec()), PutCondition::Always)
        .unwrap();
    wm_a.evict(&StateKey(view.iter().nth(1).unwrap().0.to_string())).unwrap();

    // ...and NOTHING flows back: committed truth and a rebuilt view are
    // byte-identical to before (OWN-001 — the view has no write surface; this
    // crate cannot reach Kernel::commit by construction).
    assert_eq!(kernel.truth_hash(), truth_before);
    assert_eq!(WorldView::at_head(&store, &ls).unwrap(), view);

    // Discard + re-hydrate: the lossy-by-design rebuild path (§3.10) restores
    // the exact shared world.
    ws.discard(&ls).unwrap();
    let mut wm_a2 = ws.open(&ls).unwrap();
    view.hydrate_into(&mut wm_a2).unwrap();
    assert_eq!(wm_a2.get(&first_key).unwrap().value.bytes, view.get(first_id).unwrap().0);
}

#[test]
fn hydration_into_a_foreign_shard_memory_is_refused_shard001() {
    let store = MemWalStore::new();
    let kernel = RefKernel::new(store.clone());
    commit(&kernel, &kshard("acme", "research"), 1, b"fact-1");

    let view = WorldView::at_head(&store, &lshard("acme", "research")).unwrap();
    let mut foreign = MemWorkingMemory::new(lshard("globex", "research"));
    assert_eq!(
        view.hydrate_into(&mut foreign).unwrap_err(),
        LcwError::WrongShard {
            expected: lshard("globex", "research"),
            actual: lshard("acme", "research"),
        }
    );
    assert!(foreign.is_empty(), "a refused hydration writes nothing");
}

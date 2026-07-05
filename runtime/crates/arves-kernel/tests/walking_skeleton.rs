//! I1.4 Walking Skeleton - behaviour proofs.
//!
//! Proves ARVES's first executable behaviour end to end, single node/shard:
//! commit -> WAL -> durable truth -> TruthRef -> replay -> identical truth.
//! No Raft, networking, replication, or scheduling is exercised.

use arves_kernel::{CommitError, ContentHash, Kernel, MemKernel, ProposedWrite, ShardKey};
use arves_persistence::MemWalStore;

fn shard() -> ShardKey {
    ShardKey {
        tenant: "t1".into(),
        workspace: "w1".into(),
    }
}

/// True iff `needle` occurs as a contiguous subsequence of `haystack` (used to assert one
/// tenant's payload does NOT appear in another tenant's shard snapshot).
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn proposal(content: &[u8], payload: &[u8]) -> ProposedWrite {
    ProposedWrite {
        shard: shard(),
        content: ContentHash(content.to_vec()),
        payload: payload.to_vec(),
    }
}

/// Behaviour 1: commit once -> truth exists.
#[test]
fn behaviour_1_commit_once_truth_exists() {
    let k = MemKernel::new(MemWalStore::new());
    let tr = k.commit(proposal(b"c1", b"p1")).expect("commit ok");
    assert_eq!(tr.index.0, 0, "first truth is at commit index 0");
    assert_eq!(k.committed_count(), 1);
}

/// Behaviour 2: commit twice (same content) -> AlreadyCommitted, no fork.
#[test]
fn behaviour_2_commit_twice_already_committed() {
    let k = MemKernel::new(MemWalStore::new());
    let first = k.commit(proposal(b"c1", b"p1")).expect("first ok");
    match k.commit(proposal(b"c1", b"p1")) {
        Err(CommitError::AlreadyCommitted(existing)) => {
            assert_eq!(existing, first, "idempotent commit resolves to existing truth")
        }
        other => panic!("expected AlreadyCommitted, got {other:?}"),
    }
    assert_eq!(k.committed_count(), 1, "no fork of truth");
}

/// Behaviour 3: replay -> same truth.
#[test]
fn behaviour_3_replay_same_truth() {
    let store = MemWalStore::new();
    let k1 = MemKernel::new(store.clone());
    k1.commit(proposal(b"c1", b"p1")).unwrap();
    k1.commit(proposal(b"c2", b"p2")).unwrap();
    let before = k1.truth_hash();
    let k2 = MemKernel::recover(store.clone());
    assert_eq!(k2.committed_count(), 2);
    assert_eq!(k2.truth_hash(), before, "replayed truth equals committed truth");
}

/// Behaviour 4: crash -> restart -> replay -> truth identical.
#[test]
fn behaviour_4_crash_restart_replay_identical() {
    let store = MemWalStore::new();
    let expected;
    {
        let k = MemKernel::new(store.clone());
        k.commit(proposal(b"a", b"pa")).unwrap();
        k.commit(proposal(b"b", b"pb")).unwrap();
        expected = k.truth_hash();
        // k dropped here == crash: in-memory truth cache gone, WAL persists.
    }
    let recovered = MemKernel::recover(store.clone());
    assert_eq!(recovered.committed_count(), 2);
    assert_eq!(recovered.truth_hash(), expected, "truth survives restart unchanged");
}

/// Behaviour 5: replay twice -> no duplicate truth.
#[test]
fn behaviour_5_replay_twice_no_duplicate() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    k.commit(proposal(b"c1", b"p1")).unwrap();
    let recovered = MemKernel::recover(store.clone());
    let h1 = recovered.truth_hash();
    recovered.replay(); // apply the log a second time into the same kernel
    assert_eq!(recovered.committed_count(), 1, "replay is idempotent");
    assert_eq!(recovered.truth_hash(), h1, "second replay creates no new truth");
}

/// Behaviour 6: truth hash before == after replay.
#[test]
fn behaviour_6_truth_hash_preserved() {
    let store = MemWalStore::new();
    let k = MemKernel::new(store.clone());
    k.commit(proposal(b"x", b"px")).unwrap();
    k.commit(proposal(b"y", b"py")).unwrap();
    let before = k.truth_hash();
    let after = MemKernel::recover(store.clone()).truth_hash();
    assert_eq!(before, after, "truth hash is preserved across replay");
}

/// Behaviour 7 (RCR-005): content-integrity at the sole commit gateway. A content
/// address MUST bind exactly one payload. An IDENTICAL re-proposal is an idempotent
/// no-op (Behaviour 2); a re-proposal under the SAME ContentHash but a DIFFERENT
/// payload is a caller-supplied address that does not match its content and MUST be
/// rejected as `ContentIntegrity` — never silently accepted as the prior truth and
/// never forked (ORCH-004 sound only when address ⇒ content; OWN-001).
#[test]
fn behaviour_7_content_integrity_same_address_different_payload() {
    let k = MemKernel::new(MemWalStore::new());
    k.commit(proposal(b"c1", b"p1")).expect("first commit ok");
    // Same address, same payload -> idempotent no-op (not an integrity violation).
    assert!(matches!(
        k.commit(proposal(b"c1", b"p1")),
        Err(CommitError::AlreadyCommitted(_))
    ));
    // Same address, DIFFERENT payload -> content-integrity violation, rejected.
    match k.commit(proposal(b"c1", b"p2-different")) {
        Err(CommitError::ContentIntegrity { shard: got }) => {
            assert_eq!(got, shard(), "reports the mismatched shard");
        }
        other => panic!("expected ContentIntegrity, got {other:?}"),
    }
    assert_eq!(k.committed_count(), 1, "the mismatched fork was not committed");
}

/// Behaviour 8 (RCR-007 / SHARD-001): tenant/workspace isolation at the truth gateway.
/// Two tenants commit under the SAME content bytes but DIFFERENT shards — they are
/// **distinct** truths (the shard is part of identity; no cross-tenant dedup), and neither
/// tenant's payload appears in the other tenant's shard snapshot. This is the executable
/// "a shard MUST NOT contain cross-tenant data" proof (SHARD-001), replacing the prior
/// structural-only citation.
#[test]
fn behaviour_8_two_tenant_isolation() {
    let k = MemKernel::new(MemWalStore::new());
    let acme = ShardKey { tenant: "acme".into(), workspace: "research".into() };
    let globex = ShardKey { tenant: "globex".into(), workspace: "research".into() };

    // Same content address, two tenants, distinct payloads -> two truths (shard-scoped).
    let tr1 = k
        .commit(ProposedWrite {
            shard: acme.clone(),
            content: ContentHash(b"same-cid".to_vec()),
            payload: b"acme-secret".to_vec(),
        })
        .expect("acme commit ok");
    let tr2 = k
        .commit(ProposedWrite {
            shard: globex.clone(),
            content: ContentHash(b"same-cid".to_vec()),
            payload: b"globex-secret".to_vec(),
        })
        .expect("globex commit ok");

    assert_eq!(k.committed_count(), 2, "same content under two tenants is NOT deduplicated");
    assert_eq!(tr1.shard, acme);
    assert_eq!(tr2.shard, globex);

    // Each shard's snapshot carries ONLY its own tenant's payload — no cross-tenant leak.
    let snap_acme = k.snapshot_shard(&acme);
    let snap_globex = k.snapshot_shard(&globex);
    assert!(contains_bytes(&snap_acme, b"acme-secret"), "acme shard is missing its own truth");
    assert!(contains_bytes(&snap_globex, b"globex-secret"), "globex shard is missing its own truth");
    assert!(!contains_bytes(&snap_acme, b"globex-secret"), "SHARD-001: acme snapshot leaked globex data");
    assert!(!contains_bytes(&snap_globex, b"acme-secret"), "SHARD-001: globex snapshot leaked acme data");
}

/// Behaviour 9 (RCR-013): same-shard atomic batch commit — all-or-nothing across the
/// validation class; idempotent duplicates resolve, never fork; cross-shard refused
/// up front (IDR-004: multi-shard intent is a saga, not a commit).
#[test]
fn behaviour_9_batch_commit_atomic_validation() {
    use arves_kernel::{BatchError, BatchOutcome};

    // (a) A clean batch commits every proposal; outcomes are ordered and fresh.
    let k = MemKernel::new(MemWalStore::new());
    let out = k
        .commit_batch(vec![proposal(b"c1", b"p1"), proposal(b"c2", b"p2"), proposal(b"c3", b"p3")])
        .expect("clean batch commits");
    assert_eq!(out.len(), 3);
    assert!(out.iter().all(|o: &BatchOutcome| o.fresh));
    assert_eq!(k.committed_count(), 3);

    // (b) Re-batching the same proposals is IDEMPOTENT (ORCH-004): same truths, fresh=false.
    let again = k
        .commit_batch(vec![proposal(b"c1", b"p1"), proposal(b"c2", b"p2")])
        .expect("idempotent re-batch resolves");
    assert!(again.iter().all(|o| !o.fresh));
    assert_eq!(again[0].truth, out[0].truth);
    assert_eq!(k.committed_count(), 3, "no duplicate truth from a re-batch");

    // (c) ALL-OR-NOTHING: a batch whose LAST entry forks committed truth (same address,
    // different payload — RCR-005 content-integrity) commits NOTHING, including the
    // valid entries before it.
    let before = k.committed_count();
    match k.commit_batch(vec![proposal(b"c9", b"fresh"), proposal(b"c1", b"DIFFERENT")]) {
        Err(BatchError::Refused { index, cause: CommitError::ContentIntegrity { .. } }) => {
            assert_eq!(index, 1);
        }
        other => panic!("expected Refused/ContentIntegrity, got {other:?}"),
    }
    assert_eq!(k.committed_count(), before, "nothing from the refused batch was applied");

    // (d) An INTRA-batch fork (one address, two different payloads inside the batch)
    // is refused up front — nothing applied.
    match k.commit_batch(vec![proposal(b"cx", b"a"), proposal(b"cx", b"b")]) {
        Err(BatchError::Refused { index: 1, cause: CommitError::ContentIntegrity { .. } }) => {}
        other => panic!("expected intra-batch ContentIntegrity refusal, got {other:?}"),
    }
    assert_eq!(k.committed_count(), before);

    // (e) An intra-batch IDENTICAL duplicate is lawful: the second entry resolves
    // idempotently to the first's truth (fresh=false), never a fork.
    let dup = k
        .commit_batch(vec![proposal(b"cd", b"same"), proposal(b"cd", b"same")])
        .expect("identical duplicate resolves");
    assert!(dup[0].fresh && !dup[1].fresh);
    assert_eq!(dup[0].truth, dup[1].truth);

    // (f) Cross-shard batches are refused (IDR-004: saga, never a single commit).
    let other_shard = ProposedWrite {
        shard: ShardKey { tenant: "t2".into(), workspace: "w1".into() },
        content: ContentHash(b"c1".to_vec()),
        payload: b"p1".to_vec(),
    };
    match k.commit_batch(vec![proposal(b"ca", b"pa"), other_shard]) {
        Err(BatchError::CrossShard { index: 1, .. }) => {}
        other => panic!("expected CrossShard refusal, got {other:?}"),
    }

    // (g) The empty batch is a lawful no-op.
    assert_eq!(k.commit_batch(Vec::new()).expect("empty batch"), Vec::new());
}

//! RCR-036 — authenticated commit behaviour proofs.
//!
//! Closes the open half of v2.0 security debt #8 ("`Kernel::commit` carries no
//! principal/authN") for the trusted-key model: an HMAC-SHA256 MAC over the
//! committed record proves a commit came from a key-holder and detects any
//! tampering — cryptographically. HONEST SCOPE: symmetric/shared-key
//! (repudiable), NOT public-key non-repudiation (a separate v2.0 RCR).
//!
//! Guarantees proven here:
//!   1. a VALID MAC admits (truth grows, anchor advances);
//!   2. a FORGED MAC is rejected (`AuthenticationFailed`; no truth; anchor still);
//!   3. a TAMPERED payload (MAC bound to other bytes) is rejected;
//!   4. an UNKNOWN principal (no provisioned key) is rejected;
//!   5. the authenticated anchor changes IFF the authenticated trail changes
//!      (deterministic + replayable across two independent Kernels);
//!   6. the UNAUTHENTICATED path is unchanged (anchor stays at genesis);
//!   7. anchoring composes with RCR-002: the WAL `integrity_digest` also advances.

use arves_kernel::auth::{commit_mac, genesis_anchor};
use arves_kernel::{
    CommitError, ContentHash, FileKernel, Kernel, MemKernel, Principal, ProposedWrite, ShardKey,
};
use arves_persistence::{FileWalStore, MemWalStore};
use std::fs;
use std::path::PathBuf;

const KEY: &[u8] = b"shared-secret-key-for-alice-0001";

fn shard() -> ShardKey {
    ShardKey::new("t1", "w1").expect("valid test shard")
}

fn proposal(content: &[u8], payload: &[u8]) -> ProposedWrite {
    ProposedWrite {
        shard: shard(),
        content: ContentHash(content.to_vec()),
        payload: payload.to_vec(),
    }
}

fn alice() -> Principal {
    Principal("alice".to_string())
}

fn tmp(sub: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    p.push(sub);
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).expect("create tmp dir");
    p
}

// 1. A valid MAC admits: truth grows and the anchor advances off genesis.
#[test]
fn valid_mac_admits_and_advances_anchor() {
    let k = MemKernel::new(MemWalStore::new());
    k.register_principal(alice(), KEY);

    let p = proposal(b"c0", b"payload-0");
    let mac = commit_mac(KEY, &alice(), &p);

    assert_eq!(k.authenticated_digest(), genesis_anchor());
    let tr = k.commit_authenticated(p, alice(), mac).expect("valid MAC admits");
    assert_eq!(tr.index.0, 0);
    assert_eq!(k.committed_count(), 1);
    assert_ne!(
        k.authenticated_digest(),
        genesis_anchor(),
        "anchor must advance after an authenticated commit"
    );
}

// 2. A forged MAC is rejected: no truth committed, anchor unchanged.
#[test]
fn forged_mac_rejected() {
    let k = MemKernel::new(MemWalStore::new());
    k.register_principal(alice(), KEY);

    let p = proposal(b"c0", b"payload-0");
    let forged = [0xABu8; 32]; // not a MAC anyone could compute without the key

    let before = k.authenticated_digest();
    let err = k
        .commit_authenticated(p, alice(), forged)
        .expect_err("forged MAC must be rejected");
    assert!(matches!(err, CommitError::AuthenticationFailed { .. }));
    assert_eq!(k.committed_count(), 0, "no truth on a forged MAC");
    assert_eq!(k.authenticated_digest(), before, "anchor unchanged on rejection");
}

// 3. A tampered payload is rejected: the MAC is bound to the ORIGINAL bytes, so
//    presenting it with a different payload fails verification.
#[test]
fn tampered_payload_rejected() {
    let k = MemKernel::new(MemWalStore::new());
    k.register_principal(alice(), KEY);

    let honest = proposal(b"c0", b"honest-payload");
    let mac = commit_mac(KEY, &alice(), &honest);

    // Attacker keeps the captured MAC but swaps the payload the Kernel will store.
    let tampered = proposal(b"c0", b"TAMPERED-payload");
    let err = k
        .commit_authenticated(tampered, alice(), mac)
        .expect_err("tampered payload must be rejected");
    assert!(matches!(err, CommitError::AuthenticationFailed { .. }));
    assert_eq!(k.committed_count(), 0);
}

// 4. An unknown principal (no provisioned key) cannot authenticate.
#[test]
fn unknown_principal_rejected() {
    let k = MemKernel::new(MemWalStore::new());
    // NOTE: no register_principal call.
    let p = proposal(b"c0", b"payload-0");
    let mac = commit_mac(KEY, &alice(), &p);
    let err = k
        .commit_authenticated(p, alice(), mac)
        .expect_err("unknown principal must be rejected");
    assert!(matches!(err, CommitError::AuthenticationFailed { .. }));
    assert_eq!(k.committed_count(), 0);
}

// 5. The anchor changes IFF the authenticated trail changes — and is
//    deterministic across two independent Kernels fed the same trail.
#[test]
fn anchor_changes_iff_trail_changes() {
    let build = || {
        let k = MemKernel::new(MemWalStore::new());
        k.register_principal(alice(), KEY);
        let p0 = proposal(b"c0", b"p0");
        let m0 = commit_mac(KEY, &alice(), &p0);
        k.commit_authenticated(p0, alice(), m0).unwrap();
        let a1 = k.authenticated_digest();

        let p1 = proposal(b"c1", b"p1");
        let m1 = commit_mac(KEY, &alice(), &p1);
        k.commit_authenticated(p1, alice(), m1).unwrap();
        let a2 = k.authenticated_digest();
        (k, a1, a2)
    };

    let (k, a1, a2) = build();
    assert_ne!(a1, a2, "distinct authenticated records must change the anchor");

    // Same trail on an independent Kernel => identical anchor (deterministic).
    let (_k2, b1, b2) = build();
    assert_eq!(a1, b1);
    assert_eq!(a2, b2);

    // A rejected authenticated commit does NOT change the anchor.
    let before = k.authenticated_digest();
    let bad = proposal(b"c2", b"p2");
    let _ = k.commit_authenticated(bad, alice(), [0u8; 32]);
    assert_eq!(k.authenticated_digest(), before, "rejection leaves the trail alone");

    // An idempotent re-commit (ORCH-004) resolves to existing truth and does NOT
    // re-fold the anchor: no new record entered the trail.
    let dup = proposal(b"c1", b"p1");
    let dmac = commit_mac(KEY, &alice(), &dup);
    let e = k.commit_authenticated(dup, alice(), dmac).unwrap_err();
    assert!(matches!(e, CommitError::AlreadyCommitted(_)));
    assert_eq!(k.authenticated_digest(), before, "idempotent re-commit adds no trail entry");
}

// 6. The unauthenticated path is unchanged: plain `commit` still works and leaves
//    the authenticated anchor at genesis (backward compatible v1.0 trusted-host).
#[test]
fn unauthenticated_path_unchanged() {
    let k = MemKernel::new(MemWalStore::new());
    let tr = k.commit(proposal(b"c0", b"p0")).expect("plain commit works");
    assert_eq!(tr.index.0, 0);
    assert_eq!(k.committed_count(), 1);
    assert_eq!(
        k.authenticated_digest(),
        genesis_anchor(),
        "unauthenticated commit must not touch the authenticated anchor"
    );
}

// 7. Anchoring composes with RCR-002: an authenticated commit also advances the
//    WAL `integrity_digest` (the two tamper-evidence chains move together).
#[test]
fn authenticated_commit_advances_wal_integrity_digest() {
    let dir = tmp("rcr036_wal_digest");
    let pshard = arves_persistence::ShardKey {
        tenant: "t1".to_string(),
        workspace: "w1".to_string(),
    };
    let store = FileWalStore::open_root(&dir).unwrap();
    let k = FileKernel::new(store);
    k.register_principal(alice(), KEY);

    let p = proposal(b"c0", b"payload-0");
    let mac = commit_mac(KEY, &alice(), &p);
    k.commit_authenticated(p, alice(), mac).unwrap();

    // Re-open the on-disk WAL and read the RCR-002 hash-chain digest: a committed
    // authenticated record is now part of the tamper-evident trace.
    use arves_persistence::WalStore;
    let wal_store = FileWalStore::open_root(&dir).unwrap();
    let wal = wal_store.open(&pshard).unwrap();
    let digest_after = wal.integrity_digest().unwrap();

    // Genesis digest (no records) for the same shard, computed on an empty dir.
    let empty_dir = tmp("rcr036_wal_digest_empty");
    let empty_store = FileWalStore::open_root(&empty_dir).unwrap();
    // touch the shard so its (empty) WAL exists
    let empty_wal = empty_store.open(&pshard).unwrap();
    let digest_empty = empty_wal.integrity_digest().unwrap();

    assert_ne!(
        digest_after, digest_empty,
        "the WAL integrity digest must advance once an authenticated record is committed"
    );
}

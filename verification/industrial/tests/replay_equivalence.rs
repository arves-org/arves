//! L4 replay-equivalence stress — after a fault storm, rebuild EVERY node from
//! its own durable WAL and assert byte-for-byte identical truth, over many
//! seeds (ORCH-003: truth is a pure function of the log — replay, never
//! recompute).
//!
//! HONEST SCOPE: in-process deterministic cluster; the crash model drops each
//! replica's in-memory Kernel truth and rebuilds it by replaying that replica's
//! own durable WAL. Raft in-memory state is not crash-modeled here (that is the
//! IDR-005 unification stage) — this proves the KERNEL-truth replay path.

use arves_industrial::{run_replay_equivalence, sweep_len, FaultConfig};

/// SINGLE-SHARD sweep — the literal "identical `truth_hash` after WAL rebuild,
/// over many seeds" property. With one shard, apply order == WAL order, so the
/// order-sensitive `truth_hash` is a valid per-node identity and MUST survive
/// the crash→replay round-trip unchanged, every seed.
#[test]
fn replay_equivalence_single_shard_identical_truth_hash_256_storms() {
    let cfg = FaultConfig { nodes: 5, shards: 1, rounds: 40 };
    let n = sweep_len(256);
    let stats = run_replay_equivalence(0..n, cfg).expect("truth_hash replay equivalence held");

    assert_eq!(stats.scenarios, n, "every storm was rebuilt from WAL");
    // 5 nodes × n scenarios independent per-node WAL rebuilds verified (full band = 1280).
    assert_eq!(stats.node_rebuilds_verified, n * 5, "every replica rebuild verified");

    eprintln!(
        "[L4 replay-equivalence · single-shard truth_hash] scenarios={} node_rebuilds_verified={}",
        stats.scenarios, stats.node_rebuilds_verified
    );
}

/// MULTI-SHARD sweep — the order-INDEPENDENT equivalence: every node rebuilt
/// from WAL reproduces byte-identical per-shard truth state (`shard_state_of`)
/// across a 3-shard fault storm. (`truth_hash` is intentionally NOT compared
/// here; it folds in apply order, which legitimately differs across shards per
/// the frozen contract — the per-shard bytes are the meaningful ORCH-003 check.)
#[test]
fn replay_equivalence_multi_shard_identical_state_bytes_192_storms() {
    let cfg = FaultConfig { nodes: 5, shards: 3, rounds: 40 };
    let n = sweep_len(192);
    let stats = run_replay_equivalence(0..n, cfg).expect("per-shard bytes replay equivalence held");

    assert_eq!(stats.scenarios, n);
    assert_eq!(stats.node_rebuilds_verified, n * 5);

    eprintln!(
        "[L4 replay-equivalence · multi-shard bytes] scenarios={} node_rebuilds_verified={}",
        stats.scenarios, stats.node_rebuilds_verified
    );
}

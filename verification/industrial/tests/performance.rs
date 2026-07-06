//! L4 performance harness — correctness smoke over the REAL fsync-durable
//! Kernel. The heavy sweep is the `l4_report` binary (which RECORDS numbers);
//! this test guards the measurement path itself and its ONLY hard gate: the
//! deterministic truth survives a WAL round-trip byte-for-byte. Timing is never
//! asserted, so this test cannot flake.

use arves_industrial::{measure_load, measure_load_grouped};

#[test]
fn performance_measure_load_recovers_identical_truth() {
    let dir = std::env::temp_dir().join(format!("arves_l4_perf_smoke_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create wal dir");

    let p = measure_load(&dir, 300);

    assert_eq!(p.commits, 300);
    assert!(p.correct, "recovered truth_hash must equal committed truth_hash");
    assert!(p.commit_throughput > 0.0 && p.replay_throughput > 0.0, "throughput recorded");
    eprintln!(
        "[L4 perf smoke] {} commits: {:.0} c/s commit (fsync-bound), {:.0} rec/s replay",
        p.commits, p.commit_throughput, p.replay_throughput
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// RCR-039: the group-commit (batched-fsync) path recovers identical truth AND
/// produces the byte-identical `truth_hash` as the per-commit path for the same
/// load — coalescing the fsync changes only throughput, never committed truth.
#[test]
fn performance_group_commit_matches_per_commit_truth() {
    let pid = std::process::id();
    let dir_s = std::env::temp_dir().join(format!("arves_l4_grp_single_{pid}"));
    let dir_g = std::env::temp_dir().join(format!("arves_l4_grp_group_{pid}"));
    for d in [&dir_s, &dir_g] {
        let _ = std::fs::remove_dir_all(d);
        std::fs::create_dir_all(d).expect("create wal dir");
    }

    let single = measure_load(&dir_s, 300);
    let grouped = measure_load_grouped(&dir_g, 300, 64);

    assert!(grouped.correct, "grouped recovered truth_hash must equal committed");
    assert_eq!(
        grouped.truth_hash, single.truth_hash,
        "group-commit truth_hash MUST equal the per-commit path (determinism)"
    );
    eprintln!(
        "[L4 group smoke] 300 commits: per-commit {:.0} c/s vs group {:.0} c/s ({:.1}x)",
        single.commit_throughput,
        grouped.commit_throughput,
        grouped.commit_throughput / single.commit_throughput.max(f64::MIN_POSITIVE),
    );

    for d in [&dir_s, &dir_g] {
        let _ = std::fs::remove_dir_all(d);
    }
}

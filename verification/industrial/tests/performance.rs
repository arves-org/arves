//! L4 performance harness — correctness smoke over the REAL fsync-durable
//! Kernel. The heavy sweep is the `l4_report` binary (which RECORDS numbers);
//! this test guards the measurement path itself and its ONLY hard gate: the
//! deterministic truth survives a WAL round-trip byte-for-byte. Timing is never
//! asserted, so this test cannot flake.

use arves_industrial::measure_load;

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

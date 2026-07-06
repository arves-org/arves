//! L4 fault-injection at scale — the deterministic ClusterSim driven through a
//! LARGE number of seeded adversarial schedules, asserting the cluster-Kernel
//! SAFETY contract (convergence + acked-write durability) held EVERY time.
//!
//! HONEST SCOPE: in-process deterministic cluster (FIFO bus, scripted faults,
//! logical tick). No network, no network fault-tolerance claimed. Seeded ⇒ any
//! failure replays bit-for-bit.

use arves_industrial::{run_fault_injection, sweep_len, FaultConfig};

/// The headline sweep: 512 distinct adversarial scenarios (5 nodes × 3 shards ×
/// 48 rounds each) all converge to ONE truth with zero acked-write loss. The
/// full 512-seed band is the documented headline evidence and runs by default;
/// set `ARVES_L4_SMOKE` (see `sweep_len`) to run a fast prefix band in CI.
#[test]
fn fault_injection_512_scenarios_all_converge_and_preserve_acked_truth() {
    let cfg = FaultConfig { nodes: 5, shards: 3, rounds: 48 };
    let n = sweep_len(512);
    let stats = run_fault_injection(0..n, cfg).expect("no safety violation across the sweep");

    assert_eq!(stats.scenarios, n, "every scenario ran to a verdict");
    // The sweep must actually DO adversarial work (not a vacuous pass): at least
    // one fault and more than one real commit per scenario, at whatever band size.
    assert!(stats.faults_injected > n, "faults were injected at scale: {}", stats.faults_injected);
    assert!(stats.commits_ok > n, "real truth was committed under fault: {}", stats.commits_ok);
    assert!(
        stats.acked_verified >= stats.commits_ok,
        "every acked commit was re-verified durable after convergence: {} >= {}",
        stats.acked_verified,
        stats.commits_ok
    );

    eprintln!(
        "[L4 fault-injection] scenarios={} faults={} commits_ok={} commits_refused={} acked_verified={}",
        stats.scenarios,
        stats.faults_injected,
        stats.commits_ok,
        stats.commits_refused,
        stats.acked_verified
    );
}

/// A denser topology (7 nodes × 4 shards) over a smaller seed band — proves the
/// safety contract is not an artifact of the 5×3 shape.
#[test]
fn fault_injection_dense_topology_7x4_converges() {
    let cfg = FaultConfig { nodes: 7, shards: 4, rounds: 40 };
    let n = sweep_len(128);
    let stats = run_fault_injection(1000..1000 + n, cfg).expect("no safety violation (7x4)");
    assert_eq!(stats.scenarios, n);
    assert!(stats.faults_injected > 0 && stats.commits_ok > 0);
}

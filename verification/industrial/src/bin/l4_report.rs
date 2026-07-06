//! ARVES :: L4 performance harness (binary) — measures REAL commit throughput +
//! WAL-replay time over the fsync-durable Kernel at increasing load, and writes
//! the numbers to `L4_REPORT.md` (measured, not promised).
//!
//! Run: `cargo run --release --bin l4_report` from `verification/industrial/`.
//!
//! HONEST SCOPE: single host, one process, single shard. Two durability paths are
//! measured side by side:
//!   * **per-commit** — one `fsync` per commit (`FileKernel::commit`), the
//!     original fsync-per-commit ceiling; and
//!   * **group-commit (RCR-039)** — a batch of commits shares ONE `fsync`
//!     (`FileKernel::commit_group`), closing that ceiling WITHOUT weakening
//!     durability (truth is acked only after the coalesced fsync).
//! Throughput is fsync-bound and machine-specific; the report states the host.
//! The hard gate is the DETERMINISTIC correctness check (recovered truth_hash ==
//! committed truth_hash) AND that the grouped truth_hash equals the per-commit
//! truth_hash for the same load; timing is recorded, never gated (cannot flake).

use arves_industrial::{measure_load, measure_load_grouped, PerfPoint};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Group size for the coalesced-fsync path (commits per fsync).
const GROUP_SIZE: u64 = 64;

fn main() {
    // Loads chosen so total fsync count stays bounded (fsync-per-commit is slow,
    // especially FlushFileBuffers on Windows). Override with CLI args if wanted.
    let loads: Vec<u64> = {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.is_empty() {
            vec![500, 1_000, 2_000, 4_000]
        } else {
            args.iter().filter_map(|a| a.parse().ok()).collect()
        }
    };

    let base = std::env::temp_dir().join(format!(
        "arves_l4_perf_{}",
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
    ));

    let mut points: Vec<(PerfPoint, PerfPoint)> = Vec::new();
    for &load in &loads {
        // per-commit (one fsync per commit)
        let dir1 = base.join(format!("single_{load}"));
        let _ = fs::remove_dir_all(&dir1);
        fs::create_dir_all(&dir1).expect("create wal dir");
        eprintln!("[l4_report] measuring per-commit load={load} ...");
        let single = measure_load(&dir1, load);
        let _ = fs::remove_dir_all(&dir1);

        // group-commit (one fsync per GROUP_SIZE commits)
        let dir2 = base.join(format!("group_{load}"));
        let _ = fs::remove_dir_all(&dir2);
        fs::create_dir_all(&dir2).expect("create wal dir");
        eprintln!("[l4_report] measuring group-commit load={load} (group={GROUP_SIZE}) ...");
        let grouped = measure_load_grouped(&dir2, load, GROUP_SIZE);
        let _ = fs::remove_dir_all(&dir2);

        // DETERMINISM gate: coalescing the fsync must not change committed truth.
        assert_eq!(
            single.truth_hash, grouped.truth_hash,
            "group-commit truth_hash MUST equal the per-commit truth_hash (load={load})"
        );

        eprintln!(
            "[l4_report]   per-commit {:.0} c/s | group {:.0} c/s | speedup {:.1}x | correct={}",
            single.commit_throughput,
            grouped.commit_throughput,
            grouped.commit_throughput / single.commit_throughput.max(f64::MIN_POSITIVE),
            single.correct && grouped.correct
        );
        points.push((single, grouped));
    }
    let _ = fs::remove_dir_all(&base);

    let report = render_report(&points);
    let out = PathBuf::from("L4_REPORT.md");
    fs::write(&out, &report).expect("write L4_REPORT.md");
    eprintln!("[l4_report] wrote {}", out.display());
    print!("{report}");
}

fn render_report(points: &[(PerfPoint, PerfPoint)]) -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let all_correct = points.iter().all(|(s, g)| s.correct && g.correct);
    let all_truth_equal = points.iter().all(|(s, g)| s.truth_hash == g.truth_hash);

    let mut s = String::new();
    s.push_str("# ARVES L4 Industrial Evidence — Performance Report\n\n");
    s.push_str(
        "> **Auto-generated** by `cargo run --release --bin l4_report` \
         (`verification/industrial/`). Numbers are **measured on the host below**, \
         not promised. Re-run to regenerate.\n\n",
    );
    s.push_str("## Environment\n\n");
    s.push_str(&format!("- Host OS / arch: `{os}` / `{arch}`\n"));
    s.push_str("- Build profile: `release` (recommended) — see the invoking command.\n");
    s.push_str("- Storage: the host's default temp filesystem (`std::env::temp_dir()`).\n");
    s.push_str(
        "- Durability model: **REAL `FileWal`, fsync-before-Ok** \
         (`File::sync_all` / `FlushFileBuffers` on Windows). Two paths are measured: \
         **per-commit** = one fsync per commit (`FileKernel::commit`); \
         **group-commit (RCR-039)** = one fsync per group of \
         commits (`FileKernel::commit_group`, `Wal::append_group`) — truth is acked \
         only AFTER the coalesced fsync, so durability is not weakened.\n",
    );
    s.push_str(&format!("- Group size (commits per fsync, group path): **{GROUP_SIZE}**.\n"));
    s.push_str("- Scope: **single host, single process, single shard.** No network, no cluster here (that is the fault-injection tier).\n\n");

    s.push_str("## Measured results\n\n");
    s.push_str("| Commits | Per-commit throughput (c/s, 1 fsync/commit) | Group-commit throughput (c/s, 1 fsync/group) | Speedup | Group replay (rec/s) | truth_hash (both paths) | Correct |\n");
    s.push_str("|--------:|--------------------------------------------:|---------------------------------------------:|--------:|---------------------:|:-----------------------:|:-------:|\n");
    for (single, grouped) in points {
        let speedup = grouped.commit_throughput / single.commit_throughput.max(f64::MIN_POSITIVE);
        // truth_hash is identical across both paths (asserted at measure time).
        s.push_str(&format!(
            "| {} | {:.0} | {:.0} | {:.1}x | {:.0} | `{:#018x}` | {} |\n",
            single.commits,
            single.commit_throughput,
            grouped.commit_throughput,
            speedup,
            grouped.replay_throughput,
            single.truth_hash,
            if single.correct && grouped.correct && single.truth_hash == grouped.truth_hash {
                "YES"
            } else {
                "**NO**"
            },
        ));
    }
    s.push('\n');

    s.push_str("## Reading these numbers\n\n");
    s.push_str(
        "- **The per-commit column is the fsync-per-commit ceiling** the earlier L4 report \
         measured: each commit calls `sync_all` before returning `Ok`, so throughput reflects \
         the host's durable-write latency, not a CPU ceiling.\n",
    );
    s.push_str(&format!(
        "- **The group-commit column is RCR-039**: a batch of {GROUP_SIZE} commits shares ONE \
         fsync via `Wal::append_group`, amortizing the durability cost. The **Speedup** column \
         is the measured ratio on this host — the concrete evidence that the deferred v1.1 \
         group-commit debt is closed.\n",
    ));
    s.push_str(
        "- **Durability is NOT weakened.** A grouped commit is acked only AFTER the coalesced \
         fsync makes the whole group durable (no ack-before-durable); a crash mid-group leaves \
         an un-acked tail whose torn final frame recovery truncates, while any fully-written \
         un-acked frames survive and are idempotently reconciled on retry (ORCH-004). Proven by \
         `runtime/crates/arves-persistence/tests/group_commit.rs` and \
         `runtime/crates/arves-kernel/tests/group_commit.rs`.\n",
    );
    s.push_str(&format!(
        "- **Determinism gate:** for every load the grouped `truth_hash` equals the per-commit \
         `truth_hash` (coalescing changes only the fsync count, never committed truth or order). \
         all_truth_equal = **{}**.\n",
        if all_truth_equal { "true" } else { "FALSE" }
    ));
    s.push_str(&format!(
        "- **Correctness gate:** every load (both paths) recovered a `truth_hash` byte-identical \
         to the committed one. all_correct = **{}**. Timing is recorded, never asserted, so this \
         harness cannot flake.\n\n",
        if all_correct { "true" } else { "FALSE" }
    ));
    s.push_str("_Companion tiers (fault-injection, replay-equivalence) run as `cargo test` in this crate; see `README.md`._\n");
    s
}

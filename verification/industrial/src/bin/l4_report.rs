//! ARVES :: L4 performance harness (binary) — measures REAL commit throughput +
//! WAL-replay time over the fsync-durable Kernel at increasing load, and writes
//! the numbers to `L4_REPORT.md` (measured, not promised).
//!
//! Run: `cargo run --release --bin l4_report` from `verification/industrial/`.
//!
//! HONEST SCOPE: single host, one process, one commit per fsync (no batching —
//! batch-commit is deferred v1.1 debt, `RUNTIME_FREEZE_v1.0.md`). Throughput is
//! therefore fsync-bound and machine-specific; the report states the host. The
//! only hard gate is the DETERMINISTIC correctness check (recovered truth_hash
//! == committed truth_hash); timing is recorded, never gated (cannot flake).

use arves_industrial::{measure_load, PerfPoint};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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

    let mut points: Vec<PerfPoint> = Vec::new();
    for &load in &loads {
        let dir = base.join(format!("load_{load}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create wal dir");
        eprintln!("[l4_report] measuring load={load} ...");
        let p = measure_load(&dir, load);
        eprintln!(
            "[l4_report]   commit {:.0} c/s ({:.3}s), replay {:.0} rec/s ({:.3}s), correct={}",
            p.commit_throughput, p.commit_secs, p.replay_throughput, p.replay_secs, p.correct
        );
        points.push(p);
        let _ = fs::remove_dir_all(&dir);
    }
    let _ = fs::remove_dir_all(&base);

    let report = render_report(&points);
    // Write next to the crate (CWD when run via `cargo run` in the crate dir).
    let out = PathBuf::from("L4_REPORT.md");
    fs::write(&out, &report).expect("write L4_REPORT.md");
    eprintln!("[l4_report] wrote {}", out.display());
    print!("{report}");
}

fn render_report(points: &[PerfPoint]) -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let all_correct = points.iter().all(|p| p.correct);

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
        "- Durability model: **REAL `FileWal`, fsync-before-Ok on every commit** \
         (`File::sync_all` / `FlushFileBuffers` on Windows). **One commit = one fsync** \
         (no batching — batch-commit is deferred v1.1 debt, `runtime/RUNTIME_FREEZE_v1.0.md`).\n",
    );
    s.push_str("- Scope: **single host, single process, single shard.** No network, no cluster here (that is the fault-injection tier).\n\n");

    s.push_str("## Measured results\n\n");
    s.push_str("| Commits | Commit time (s) | Commit throughput (commits/s, fsync-bound) | Replay time (s) | Replay throughput (records/s) | truth_hash | Correct (replay == commit) |\n");
    s.push_str("|--------:|----------------:|-------------------------------------------:|----------------:|------------------------------:|:----------:|:--------------------------:|\n");
    for p in points {
        s.push_str(&format!(
            "| {} | {:.3} | {:.0} | {:.4} | {:.0} | `{:#018x}` | {} |\n",
            p.commits,
            p.commit_secs,
            p.commit_throughput,
            p.replay_secs,
            p.replay_throughput,
            p.truth_hash,
            if p.correct { "YES" } else { "**NO**" },
        ));
    }
    s.push('\n');

    s.push_str("## Reading these numbers\n\n");
    s.push_str(
        "- **Commit throughput is fsync-bound.** Each commit calls `sync_all` before returning \
         `Ok` — the number reflects the host's durable-write latency, not a CPU ceiling. This is \
         the honest cost of the CP durability guarantee; a batched commit path (deferred v1.1) \
         would raise it and is the motivation for that RCR.\n",
    );
    s.push_str(
        "- **Replay is CPU/parse-bound** (no fsync) and is the path a crashed node uses to rebuild \
         truth — records/s here bounds recovery time.\n",
    );
    s.push_str(&format!(
        "- **Correctness gate:** every load recovered a `truth_hash` byte-identical to the committed \
         one. all_correct = **{}**. Timing is recorded, never asserted, so this harness cannot \
         flake.\n\n",
        if all_correct { "true" } else { "FALSE" }
    ));
    s.push_str("_Companion tiers (fault-injection, replay-equivalence) run as `cargo test` in this crate; see `README.md` and `L4_REPORT.md` header counts._\n");
    s
}

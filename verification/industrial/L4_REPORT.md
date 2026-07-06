# ARVES L4 Industrial Evidence — Performance Report

> **Auto-generated** by `cargo run --release --bin l4_report` (`verification/industrial/`). Numbers are **measured on the host below**, not promised. Re-run to regenerate.

## Environment

- Host OS / arch: `windows` / `x86_64`
- Build profile: `release` (recommended) — see the invoking command.
- Storage: the host's default temp filesystem (`std::env::temp_dir()`).
- Durability model: **REAL `FileWal`, fsync-before-Ok** (`File::sync_all` / `FlushFileBuffers` on Windows). Two paths are measured: **per-commit** = one fsync per commit (`FileKernel::commit`); **group-commit (RCR-039)** = one fsync per group of commits (`FileKernel::commit_group`, `Wal::append_group`) — truth is acked only AFTER the coalesced fsync, so durability is not weakened.
- Group size (commits per fsync, group path): **64**.
- Scope: **single host, single process, single shard.** No network, no cluster here (that is the fault-injection tier).

## Measured results

| Commits | Per-commit throughput (c/s, 1 fsync/commit) | Group-commit throughput (c/s, 1 fsync/group) | Speedup | Group replay (rec/s) | truth_hash (both paths) | Correct |
|--------:|--------------------------------------------:|---------------------------------------------:|--------:|---------------------:|:-----------------------:|:-------:|
| 500 | 1260 | 47983 | 38.1x | 234357 | `0xee0bdc57358228e1` | YES |
| 1000 | 1335 | 51076 | 38.3x | 233863 | `0x1a339336f144e489` | YES |
| 2000 | 1379 | 35584 | 25.8x | 370439 | `0xe1555faff64285cd` | YES |
| 4000 | 1306 | 33736 | 25.8x | 215494 | `0x2584dc964d27e251` | YES |

## Reading these numbers

- **The per-commit column is the fsync-per-commit ceiling** the earlier L4 report measured: each commit calls `sync_all` before returning `Ok`, so throughput reflects the host's durable-write latency, not a CPU ceiling.
- **The group-commit column is RCR-039**: a batch of 64 commits shares ONE fsync via `Wal::append_group`, amortizing the durability cost. The **Speedup** column is the measured ratio on this host — the concrete evidence that the deferred v1.1 group-commit debt is closed.
- **Durability is NOT weakened.** A grouped commit is acked only AFTER the coalesced fsync makes the whole group durable (no ack-before-durable); a crash mid-group leaves an un-acked tail whose torn final frame recovery truncates, while any fully-written un-acked frames survive and are idempotently reconciled on retry (ORCH-004). Proven by `runtime/crates/arves-persistence/tests/group_commit.rs` and `runtime/crates/arves-kernel/tests/group_commit.rs`.
- **Determinism gate:** for every load the grouped `truth_hash` equals the per-commit `truth_hash` (coalescing changes only the fsync count, never committed truth or order). all_truth_equal = **true**.
- **Correctness gate:** every load (both paths) recovered a `truth_hash` byte-identical to the committed one. all_correct = **true**. Timing is recorded, never asserted, so this harness cannot flake.

_Companion tiers (fault-injection, replay-equivalence) run as `cargo test` in this crate; see `README.md`._

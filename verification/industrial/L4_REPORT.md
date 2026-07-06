# ARVES L4 Industrial Evidence — Performance Report

> **Auto-generated** by `cargo run --release --bin l4_report` (`verification/industrial/`). Numbers are **measured on the host below**, not promised. Re-run to regenerate.

## Environment

- Host OS / arch: `windows` / `x86_64`
- Build profile: `release` (recommended) — see the invoking command.
- Storage: the host's default temp filesystem (`std::env::temp_dir()`).
- Durability model: **REAL `FileWal`, fsync-before-Ok on every commit** (`File::sync_all` / `FlushFileBuffers` on Windows). **One commit = one fsync** (no batching — batch-commit is deferred v1.1 debt, `runtime/RUNTIME_FREEZE_v1.0.md`).
- Scope: **single host, single process, single shard.** No network, no cluster here (that is the fault-injection tier).

## Measured results

| Commits | Commit time (s) | Commit throughput (commits/s, fsync-bound) | Replay time (s) | Replay throughput (records/s) | truth_hash | Correct (replay == commit) |
|--------:|----------------:|-------------------------------------------:|----------------:|------------------------------:|:----------:|:--------------------------:|
| 500 | 0.522 | 957 | 0.0065 | 76605 | `0xee0bdc57358228e1` | YES |
| 1000 | 0.792 | 1263 | 0.0069 | 145111 | `0x1a339336f144e489` | YES |
| 2000 | 1.392 | 1437 | 0.0049 | 410442 | `0xe1555faff64285cd` | YES |
| 4000 | 3.778 | 1059 | 0.0371 | 107775 | `0x2584dc964d27e251` | YES |

## Reading these numbers

- **Commit throughput is fsync-bound.** Each commit calls `sync_all` before returning `Ok` — the number reflects the host's durable-write latency, not a CPU ceiling. This is the honest cost of the CP durability guarantee; a batched commit path (deferred v1.1) would raise it and is the motivation for that RCR.
- **Replay is CPU/parse-bound** (no fsync) and is the path a crashed node uses to rebuild truth — records/s here bounds recovery time.
- **Correctness gate:** every load recovered a `truth_hash` byte-identical to the committed one. all_correct = **true**. Timing is recorded, never asserted, so this harness cannot flake.

_Companion tiers (fault-injection, replay-equivalence) run as `cargo test` in this crate; see `README.md` and `L4_REPORT.md` header counts._

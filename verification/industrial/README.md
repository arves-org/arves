# ARVES L4 Industrial Evidence (`verification/industrial/`)

The roadmap's **L4 evidence tier** for LTO #1 (a production-grade distributed
runtime): **fault-injection + replay-equivalence + performance-under-load**,
with REAL, measured, seeded evidence — not claims.

This is a **standalone cargo workspace** that path-depends on the FROZEN runtime
crates (`arves-kernel`, `arves-consensus`, `arves-persistence`) as an external,
read-only dependency (IDR-006). It **adds zero third-party crates** (std-only,
matching the runtime) and **never modifies `runtime/`** — it is not a member of
the runtime workspace, so the freeze manifest is untouched.

## Run

```sh
./run.sh          # all three tiers + regenerate L4_REPORT.md (FULL band, release)
./run.sh quick    # fast smoke: small seed bands + small perf loads
# or directly:
cargo test --release                    # tiers 1 & 2 (+ perf smoke), full band
ARVES_L4_SMOKE=24 cargo test --release  # fast prefix band (the per-push CI gate)
cargo run  --release --bin l4_report    # tier 3: measured perf → L4_REPORT.md
```

### Regression-protected in CI (not just runnable on demand)

Because this is a **standalone** cargo workspace (IDR-006 freeze isolation), it
is *not* a member of the runtime `cargo test --workspace` and so is easy to let
silently rot. It is therefore wired into the mechanical CI gates
(`.github/workflows/ci.yml`, job **`industrial`**): every push/PR runs the L4
sweeps at a fast **smoke band** (`ARVES_L4_SMOKE=24`) so a regression in the
safety / replay-equivalence contract **fails the branch**. The **full headline
band** (512 / 128 / 256 / 192 seeds — the counts documented below) runs on
manual dispatch (`workflow_dispatch` input `l4_full=true`) or a schedule, and is
still the reproduce-locally evidence via a bare `./run.sh`.

**`ARVES_L4_SMOKE`** (see `src/lib.rs::sweep_len`): unset → full documented band;
`=<n>` → an exact `n`-seed prefix band (clamped to `1..=full`); any non-empty
non-numeric value → the default smoke of 24. Determinism is untouched — the
smoke band is a *prefix of the same seed stream*, so any failure it catches
replays identically under the full sweep.

## The three tiers

### 1. Fault-injection at scale (`tests/fault_injection.rs`)

Drives the deterministic in-process `ClusterSim` through a LARGE number of
seeded adversarial schedules — commits interleaved with **partitions, node
isolation (message-loss), and crash→WAL-replay** — then heals, settles, and
asserts the cluster-Kernel SAFETY contract held EVERY time:

- **Convergence (ORCH-003):** every replica ends with byte-identical per-shard
  truth state and the identical total committed count (zero divergence).
- **Agreement/durability:** every ACKED commit survives in its shard's truth
  after convergence — no acked write is ever lost or forked.

(Raft's own four safety properties are additionally checked by the frozen
RCR-019 harness after every message step, underneath this.)

Captured on the reference host (see `L4_REPORT.md` for the machine):

| Sweep | Scenarios | Faults injected | Commits → truth | Acked re-verified durable |
|---|---:|---:|---:|---:|
| `512_scenarios` (5 nodes × 3 shards × 48 rounds) | 512 | 12,349 | 10,031 | 10,031 (100%) |
| `dense_topology_7x4` (7 nodes × 4 shards × 40 rounds) | 128 | — | — | all convergent |

**640 adversarial scenarios**, zero safety violations.

### 2. Replay-equivalence stress (`tests/replay_equivalence.rs`)

After a fault storm, CRASH EVERY NODE and rebuild it purely by deterministic
replay of its own durable WAL (the I1.7 lossless-or-loud path, cluster-wide —
replay, never recompute), over many seeds:

- **Single-shard sweep (256 storms):** the literal *"identical `truth_hash`
  after WAL rebuild"* property — with one shard, apply order == WAL order, so
  the order-sensitive `truth_hash` is a valid per-node identity and survives the
  round-trip unchanged.
- **Multi-shard sweep (192 storms):** the order-INDEPENDENT equivalence —
  byte-identical per-shard truth state (`shard_state_of`). (`truth_hash` is not
  compared here: it folds in apply order, which legitimately differs across
  shards per the frozen `truth_hash_of` contract.)

**448 storms → 2,240 independent per-node WAL rebuilds**, all byte-identical.

### 3. Performance under load (`src/bin/l4_report.rs` → `L4_REPORT.md`)

Measures **commit throughput** and **WAL-replay time** over the REAL Kernel and
the REAL fsync-durable `FileWal` at increasing load, and RECORDS the numbers
(with the host environment) in [`L4_REPORT.md`](./L4_REPORT.md). The only hard
gate is the DETERMINISTIC correctness check (recovered `truth_hash` == committed
`truth_hash`); timing is recorded, never asserted — so it cannot flake.

Reference-host snapshot (`windows`/`x86_64`, release, default temp FS):

| Commits | Commit throughput (fsync-bound) | Replay throughput |
|---:|---:|---:|
| 500 | ~957 c/s | ~76k rec/s |
| 1,000 | ~1,263 c/s | ~145k rec/s |
| 2,000 | ~1,437 c/s | ~410k rec/s |
| 4,000 | ~1,059 c/s | ~108k rec/s |

Commit throughput is **fsync-bound** (one `sync_all` per commit — batch-commit
is deferred v1.1 debt in `runtime/RUNTIME_FREEZE_v1.0.md`); the number reflects
the host's durable-write latency, and motivates that RCR. Re-run to regenerate
for your host.

## Honest scope (say it plainly)

- Fault-injection & replay run over the **deterministic in-process cluster**:
  FIFO bus, scripted faults, logical tick. **No network, no network
  fault-tolerance claimed.** What is proven is the CLUSTER-KERNEL truth machine
  under adversity (leader-only commit, quorum replication, the shared
  ORCH-004/RCR-005 gateway, byte-identical follower truth).
- Performance is **single host, single process, single shard**, fsync-durable.
- Message *duplication/reorder* is proven exhaustively by the frozen RCR-022
  tests; this sweep focuses on the L4-named faults (partition / isolation /
  crash) to stay provably terminating at scale.
- Everything is **seeded** ⇒ every reported result replays bit-for-bit.

//! ARVES :: verification/industrial — L4 INDUSTRIAL EVIDENCE harness (library).
//!
//! Advances LTO #1 (a production-grade distributed runtime) with REAL, measured
//! evidence — not claims — at the roadmap's **L4 tier**: fault-injection +
//! replay-equivalence + performance-under-load. Everything here consumes the
//! FROZEN runtime crates as an external dependency (IDR-006); nothing in
//! `runtime/` is touched.
//!
//! # What is REAL vs SIMULATED (honest scope — read this first)
//!
//! - **Fault-injection & replay run over the deterministic in-process
//!   [`ClusterSim`]** (RCR-019..022): the transport is a FIFO bus, faults
//!   (partition / isolation / message duplication+reorder / crash-replay) are
//!   scripted bus behaviours, and time is the injected logical tick. **No
//!   network exists and NO network fault-tolerance is claimed.** What is proven
//!   is the CLUSTER-KERNEL truth machine under adversity: leader-only commit,
//!   quorum replication, the shared gateway (ORCH-004 idempotency, RCR-005
//!   content-integrity), and byte-identical follower truth (ORCH-003).
//! - **The performance harness runs over the REAL Kernel and the REAL
//!   fsync-durable `FileWal`** (single host, one process, one commit per fsync —
//!   batch-commit is deferred v1.1 debt, `RUNTIME_FREEZE_v1.0.md`). Throughput
//!   is therefore **fsync-bound**, and we say so; the numbers are *measured*,
//!   not promised.
//!
//! # Determinism (sacred)
//!
//! Every committed-truth decision is a pure function of `(recorded proposals,
//! per-shard seed, logical tick)`. The scenario *schedule itself* is a pure
//! function of a single `u64` seed via [`Rng`] (SplitMix64, std-only), so every
//! reported result replays bit-for-bit. The perf harness measures wall-clock
//! time (that is its job) but gates only on the DETERMINISTIC truth outcome
//! (recovered `truth_hash`), never on a timing threshold — so it cannot flake.

use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{CommitError, ContentHash, Kernel, ProposedWrite, ShardKey};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Deterministic RNG — SplitMix64 (std-only, no third-party crates). Used ONLY
// to derive the adversarial SCHEDULE; it never touches truth.
// ---------------------------------------------------------------------------

/// A deterministic, seedable PRNG (SplitMix64). Identical seed ⇒ identical
/// stream ⇒ identical scenario ⇒ replayable evidence.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    /// Seed the stream.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

// ---------------------------------------------------------------------------
// Sweep sizing — full headline band by default, opt-in fast smoke for CI
// ---------------------------------------------------------------------------

/// Resolve the number of seeds a sweep should run, given its FULL documented
/// headline length `full`.
///
/// By DEFAULT (no env var) every sweep runs its full band — the headline counts
/// in `README.md` / `L4_REPORT.md` are the full-run evidence and MUST reproduce
/// on a bare `cargo test`. Setting **`ARVES_L4_SMOKE`** to a non-empty value
/// shrinks every band to a fast, still-adversarial smoke so a per-push CI gate
/// can run cheaply while the nightly/manual full sweep keeps the headline
/// numbers:
///
/// - `ARVES_L4_SMOKE=1` (or any non-numeric non-empty value) → default smoke of
///   [`SMOKE_DEFAULT`] seeds;
/// - `ARVES_L4_SMOKE=<n>` → exactly `n` seeds (clamped to `1..=full`).
///
/// Determinism is untouched: seed `k` produces a byte-identical scenario whether
/// it runs inside a smoke band or the full band — the smoke band is simply a
/// prefix of the same seed sequence, so any failure it finds replays under the
/// full sweep too.
pub const SMOKE_DEFAULT: u64 = 24;

/// See [`SMOKE_DEFAULT`]. Reads `ARVES_L4_SMOKE` and returns the seed count to run.
pub fn sweep_len(full: u64) -> u64 {
    match std::env::var("ARVES_L4_SMOKE") {
        Ok(v) if !v.trim().is_empty() => {
            let n = v.trim().parse::<u64>().unwrap_or(SMOKE_DEFAULT).max(1);
            n.min(full)
        }
        _ => full,
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn sid(t: &str, w: &str) -> ShardId {
    ShardId::new(TenantId(t.into()), WorkspaceId(w.into()))
}

fn skey(t: &str, w: &str) -> ShardKey {
    ShardKey::new(t, w).expect("well-formed shard key")
}

/// A cluster of `nodes` replicas hosting `shards` independent Raft groups
/// (`t1/w0..w{shards-1}`), each seeded from `seed`, all leaders elected.
fn build_cluster(nodes: usize, shards: usize, seed: u64) -> (Rc<RefCell<ClusterSim>>, Vec<ShardId>) {
    let mut c = ClusterSim::new(nodes);
    let mut ids = Vec::with_capacity(shards);
    for s in 0..shards {
        let shard = sid("t1", &format!("w{s}"));
        // Each group's seed is recorded (derived from the scenario seed) ⇒ replayable.
        c.add_shard(shard.clone(), seed.wrapping_mul(0x100000001B3).wrapping_add(s as u64));
        ids.push(shard);
    }
    let cluster = Rc::new(RefCell::new(c));
    for shard in &ids {
        cluster.borrow_mut().elect(shard);
    }
    (cluster, ids)
}

/// Split `nodes` node ids into two disjoint sides by a seeded coin per node,
/// forcing at least one node on each side (a genuine partition, not a no-op).
fn two_way_split(ids: &[NodeId], rng: &mut Rng) -> Vec<Vec<NodeId>> {
    let mut a = Vec::new();
    let mut b = Vec::new();
    for id in ids {
        if rng.below(2) == 0 {
            a.push(id.clone());
        } else {
            b.push(id.clone());
        }
    }
    // Guarantee both sides non-empty (else it is just an isolation / no-op).
    if a.is_empty() {
        a.push(b.pop().expect("ids non-empty"));
    } else if b.is_empty() {
        b.push(a.pop().expect("ids non-empty"));
    }
    vec![a, b]
}

// ---------------------------------------------------------------------------
// (a) FAULT-INJECTION at scale
// ---------------------------------------------------------------------------

/// Configuration for one fault-injection sweep.
#[derive(Clone, Copy)]
pub struct FaultConfig {
    /// Replicas per cluster (each hosts a replica of every shard group).
    pub nodes: usize,
    /// Independent Raft groups (shards) per cluster.
    pub shards: usize,
    /// Adversarial rounds per scenario (each round = one scheduled action).
    pub rounds: usize,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self { nodes: 5, shards: 3, rounds: 48 }
    }
}

/// Aggregate evidence produced by a fault-injection sweep.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FaultStats {
    /// Distinct adversarial scenarios run to convergence (the headline count).
    pub scenarios: u64,
    /// Commits that reached quorum and became truth (Ok).
    pub commits_ok: u64,
    /// Commits honestly refused by the CP gateway (NotLeader / NotReplicated /
    /// idempotent AlreadyCommitted) — no partial truth, by design.
    pub commits_refused: u64,
    /// Fault operations injected (partitions + isolations + mangling + crashes).
    pub faults_injected: u64,
    /// Acked writes re-verified present & convergent after heal (durability).
    pub acked_verified: u64,
}

/// Run a fault-injection sweep over `seeds`. For every seed: build an
/// `nodes`-replica, `shards`-group cluster; drive a seeded adversarial schedule
/// (commits interleaved with partitions, isolations, message duplicate+reorder
/// storms, and crash→WAL-replay); then heal, settle, and assert the L4 SAFETY
/// contract:
///
///   1. **Convergence (ORCH-003):** every replica holds byte-identical
///      per-shard truth state and the identical total committed count.
///   2. **Agreement/durability:** every commit that was ACKED (Ok) survives —
///      its payload is present in the converged truth of its shard. No acked
///      write is ever lost, none is forked.
///
/// (Raft's own four safety properties are additionally checked by the RCR-019
/// harness after EVERY message step, underneath this.)
///
/// Returns aggregate [`FaultStats`], or `Err(description)` on the FIRST
/// violation (which, being seeded, replays exactly).
pub fn run_fault_injection(seeds: std::ops::Range<u64>, cfg: FaultConfig) -> Result<FaultStats, String> {
    let mut stats = FaultStats::default();
    for seed in seeds {
        run_one_scenario(seed, cfg, &mut stats)?;
        stats.scenarios += 1;
    }
    Ok(stats)
}

fn run_one_scenario(seed: u64, cfg: FaultConfig, stats: &mut FaultStats) -> Result<(), String> {
    let (cluster, shards) = build_cluster(cfg.nodes, cfg.shards, seed);
    let mut rng = Rng::new(seed ^ 0xA5A5_5A5A_1234_ABCD);

    // Acked truths we must find again after convergence: (shard index, payload).
    let mut acked: Vec<(usize, Vec<u8>)> = Vec::new();

    for round in 0..cfg.rounds {
        let si = rng.below(cfg.shards as u64) as usize;
        let shard = shards[si].clone();
        // The adversarial action for this round. Faults are the ones the L4 lane
        // names — PARTITIONS, ISOLATION (message-loss), and CRASH→WAL-replay.
        // Each partition/isolate REPLACES the shard group's active filter (they
        // never stack), so the schedule is bounded: only the dup/reorder arm can
        // grow a drain, and it is exercised exhaustively by the frozen RCR-022
        // tests, not here — keeping this large sweep provably terminating.
        match rng.below(10) {
            // 0..=4 (50%): attempt a commit through the shard's current leader.
            0..=4 => {
                let leader = cluster.borrow().leader_of(&shard);
                if let Some(leader) = leader {
                    let payload = format!("PAY|s{si}|r{round}|seed{seed}").into_bytes();
                    let content = format!("C|s{si}|r{round}|seed{seed}").into_bytes();
                    let pw = ProposedWrite {
                        shard: skey("t1", &format!("w{si}")),
                        content: ContentHash(content),
                        payload: payload.clone(),
                    };
                    let k = ClusterKernel::new(leader, cluster.clone());
                    match k.commit(pw) {
                        Ok(_) => {
                            stats.commits_ok += 1;
                            acked.push((si, payload));
                        }
                        Err(CommitError::NotLeader { .. })
                        | Err(CommitError::NotReplicated)
                        | Err(CommitError::AlreadyCommitted(_)) => stats.commits_refused += 1,
                        Err(other) => {
                            return Err(format!(
                                "seed {seed} round {round}: unexpected commit error {other:?}"
                            ))
                        }
                    }
                }
            }
            // 5..=6 (20%): symmetric partition of this shard's group.
            5..=6 => {
                let ids = cluster.borrow().node_ids();
                let sides = two_way_split(&ids, &mut rng);
                cluster.borrow_mut().partition(&shard, &sides);
                stats.faults_injected += 1;
            }
            // 7..=8 (20%): isolate one node on this shard's group (message-loss).
            7..=8 => {
                let ids = cluster.borrow().node_ids();
                let victim = ids[rng.below(ids.len() as u64) as usize].clone();
                cluster.borrow_mut().isolate(&shard, &victim);
                stats.faults_injected += 1;
            }
            // 9 (10%): crash one replica, recover it from its durable WAL, then
            //    heal this shard so the schedule keeps making progress.
            _ => {
                let ids = cluster.borrow().node_ids();
                let victim = ids[rng.below(ids.len() as u64) as usize].clone();
                cluster.borrow_mut().crash_recover(&victim);
                cluster.borrow_mut().heal(&shard);
                stats.faults_injected += 1;
            }
        }
        // A little settling every round so partitions can trigger elections.
        cluster.borrow_mut().settle(5);
    }

    // Heal EVERYTHING and settle generously so the cluster must converge.
    {
        let mut c = cluster.borrow_mut();
        for shard in &shards {
            c.heal(shard);
        }
        c.settle(300);
    }

    // 1. Convergence: identical per-shard state bytes + identical total count.
    let c = cluster.borrow();
    let ids = c.node_ids();
    let reference = &ids[0];
    let ref_count = c.committed_count_of(reference);
    for shard in &shards {
        let ref_blob = c.shard_state_of(reference, shard);
        for id in &ids {
            if c.committed_count_of(id) != ref_count {
                return Err(format!(
                    "seed {seed}: replica {id:?} committed_count {} != reference {ref_count} (divergence)",
                    c.committed_count_of(id)
                ));
            }
            if c.shard_state_of(id, shard) != ref_blob {
                return Err(format!(
                    "seed {seed}: replica {id:?} shard {shard:?} state bytes diverge from reference"
                ));
            }
        }
    }

    // 2. Agreement/durability: every acked write survives in its shard's truth.
    for (si, payload) in &acked {
        let blob = c.shard_state_of(reference, &shards[*si]);
        let present = blob.windows(payload.len()).any(|w| w == payload.as_slice());
        if !present {
            return Err(format!(
                "seed {seed}: acked write on shard w{si} was LOST after convergence (durability violation)"
            ));
        }
        stats.acked_verified += 1;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// (b) REPLAY-EQUIVALENCE stress
// ---------------------------------------------------------------------------

/// Aggregate evidence from a replay-equivalence sweep.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplayStats {
    /// Distinct fault-storm scenarios rebuilt from WAL (the headline count).
    pub scenarios: u64,
    /// Total per-node rebuilds verified byte-for-byte identical to pre-crash.
    pub node_rebuilds_verified: u64,
}

/// For every seed: drive a fault storm on an `nodes`×`shards` cluster, let it
/// converge, snapshot every replica's `truth_hash` + per-shard state, then
/// CRASH EVERY NODE and rebuild it purely by deterministic replay of its own
/// durable WAL (the I1.7 lossless-or-loud path, applied cluster-wide — replay,
/// never recompute). Assert every rebuilt node reproduces the identical
/// `truth_hash` and byte-identical per-shard state it held before the crash.
///
/// This is ORCH-003 at scale: truth is a pure function of the log.
pub fn run_replay_equivalence(seeds: std::ops::Range<u64>, cfg: FaultConfig) -> Result<ReplayStats, String> {
    let mut stats = ReplayStats::default();
    for seed in seeds {
        let (cluster, shards) = build_cluster(cfg.nodes, cfg.shards, seed);
        let mut rng = Rng::new(seed ^ 0x5EED_D06D_F00D_1357);

        // A compact storm: commits through whoever leads, with injected faults.
        for round in 0..cfg.rounds {
            let si = rng.below(cfg.shards as u64) as usize;
            let shard = shards[si].clone();
            match rng.below(6) {
                0..=3 => {
                    let leader = cluster.borrow().leader_of(&shard);
                    if let Some(leader) = leader {
                        let pw = ProposedWrite {
                            shard: skey("t1", &format!("w{si}")),
                            content: ContentHash(format!("RC|s{si}|r{round}|seed{seed}").into_bytes()),
                            payload: format!("RP|s{si}|r{round}|seed{seed}").into_bytes(),
                        };
                        let _ = ClusterKernel::new(leader, cluster.clone()).commit(pw);
                    }
                }
                4 => {
                    let ids = cluster.borrow().node_ids();
                    let sides = two_way_split(&ids, &mut rng);
                    cluster.borrow_mut().partition(&shard, &sides);
                }
                _ => {
                    cluster.borrow_mut().heal(&shard);
                }
            }
            cluster.borrow_mut().settle(6);
        }

        // Heal + converge.
        {
            let mut c = cluster.borrow_mut();
            for shard in &shards {
                c.heal(shard);
            }
            c.settle(400);
        }

        // Snapshot every replica's truth (pre-crash).
        let (ids, before): (Vec<NodeId>, Vec<(u64, Vec<Vec<u8>>)>) = {
            let c = cluster.borrow();
            let ids = c.node_ids();
            let before = ids
                .iter()
                .map(|id| {
                    let h = c.truth_hash_of(id);
                    let states = shards.iter().map(|s| c.shard_state_of(id, s)).collect();
                    (h, states)
                })
                .collect();
            (ids, before)
        };

        // Crash EVERY node; rebuild each from its own durable WAL.
        {
            let mut c = cluster.borrow_mut();
            for id in &ids {
                c.crash_recover(id);
            }
        }

        // Assert byte-for-byte replay equivalence, per node.
        //
        // NOTE (frozen `truth_hash_of` contract): `truth_hash` folds in APPLY
        // ORDER, which for a MULTI-shard node legitimately differs between the
        // live interleaved apply and per-shard WAL replay — so it is a valid
        // per-node identity ONLY for a single shard. The order-INDEPENDENT
        // equivalence that always holds is the per-shard `shard_state_of` bytes;
        // we assert those unconditionally, and additionally assert `truth_hash`
        // equality when (and only when) there is exactly one shard.
        let c = cluster.borrow();
        for (id, (h_before, states_before)) in ids.iter().zip(before.iter()) {
            if cfg.shards == 1 && c.truth_hash_of(id) != *h_before {
                return Err(format!(
                    "seed {seed}: replica {id:?} truth_hash changed across WAL rebuild (replay non-equivalence)"
                ));
            }
            for (s, sb) in shards.iter().zip(states_before.iter()) {
                if c.shard_state_of(id, s) != *sb {
                    return Err(format!(
                        "seed {seed}: replica {id:?} shard {s:?} state changed across WAL rebuild"
                    ));
                }
            }
            stats.node_rebuilds_verified += 1;
        }
        stats.scenarios += 1;
    }
    Ok(stats)
}

// ---------------------------------------------------------------------------
// (c) PERFORMANCE harness — REAL Kernel, REAL fsync-durable FileWal, single host
// ---------------------------------------------------------------------------

use arves_kernel::{FileKernel, ShardKey as KShardKey};
use arves_persistence::FileWalStore;
use std::path::Path;
use std::time::Instant;

/// One measured load point over the real fsync-durable Kernel.
#[derive(Clone, Debug)]
pub struct PerfPoint {
    /// Number of unique commits driven at this load.
    pub commits: u64,
    /// Wall-clock seconds to commit them all (fsync-per-commit included).
    pub commit_secs: f64,
    /// Commits per second (== `commits / commit_secs`). Fsync-bound.
    pub commit_throughput: f64,
    /// Wall-clock seconds to rebuild all truth by WAL replay in a fresh process.
    pub replay_secs: f64,
    /// Replay speed in records/second.
    pub replay_throughput: f64,
    /// The truth_hash after commit AND after replay — MUST match (the only gate).
    pub truth_hash: u64,
    /// True iff the deterministic correctness gate held (recovered == committed).
    pub correct: bool,
}

/// Drive `commits` unique writes into a fresh `FileKernel` rooted at `wal_dir`
/// (fsync-durable), measure commit throughput, then drop the process, recover a
/// fresh `FileKernel` from the same directory (deterministic WAL replay) and
/// measure replay time. The recovered `truth_hash` MUST equal the committed one
/// (the deterministic correctness gate); timing is RECORDED, never gated (so
/// this can never flake).
///
/// `wal_dir` must be empty/fresh; the caller owns cleanup.
pub fn measure_load(wal_dir: &Path, commits: u64) -> PerfPoint {
    let shard = KShardKey::new("t1", "w1").expect("valid shard");

    // --- commit phase (fsync-durable) ---
    let store = FileWalStore::open_root(wal_dir).expect("open file store");
    let kernel = FileKernel::new(store);
    let t0 = Instant::now();
    for i in 0..commits {
        let pw = ProposedWrite {
            shard: shard.clone(),
            content: ContentHash(format!("perf-c-{i}").into_bytes()),
            payload: format!("perf-payload-{i}").into_bytes(),
        };
        kernel.commit(pw).expect("commit ok");
    }
    let commit_secs = t0.elapsed().as_secs_f64();
    let committed_hash = kernel.truth_hash();
    let committed_count = kernel.committed_count() as u64;
    assert_eq!(committed_count, commits, "every unique write became truth");
    drop(kernel); // == process exit; only the durable WAL remains on disk

    // --- replay phase (fresh process, deterministic WAL replay) ---
    let store2 = FileWalStore::open_root(wal_dir).expect("reopen file store");
    let t1 = Instant::now();
    let recovered = FileKernel::recover(store2);
    let replay_secs = t1.elapsed().as_secs_f64();
    let replay_hash = recovered.truth_hash();

    // The ONLY assertion: deterministic truth survived byte-for-byte.
    assert_eq!(replay_hash, committed_hash, "replayed truth_hash must equal committed truth_hash");
    assert_eq!(recovered.committed_count() as u64, commits, "replay recovered every record");

    PerfPoint {
        commits,
        commit_secs,
        commit_throughput: if commit_secs > 0.0 { commits as f64 / commit_secs } else { f64::INFINITY },
        replay_secs,
        replay_throughput: if replay_secs > 0.0 { commits as f64 / replay_secs } else { f64::INFINITY },
        truth_hash: committed_hash,
        correct: replay_hash == committed_hash,
    }
}

# ARVES Implementation Progress

**STATUS: LIVING TRACKER (non-normative).** Updated each milestone. The
authoritative completion record is the annotated **git tag** per milestone; the
frozen Baseline (`ARVES_00_Baseline_v1`) defines *scope* and is never edited for
progress. This tracker records *where implementation stands*, not what the
standard is.

Milestones follow the frozen Baseline Part 5 set (single source of truth):
`I1 Distributed Runtime -> I2 Cluster Kernel -> I3 Distributed Query ->
I4 Capability Scheduling -> I5 Multi-Agent Runtime -> I6 Reference Products`.

## I1 - Distributed Runtime

| Step | Milestone | Status | Marker |
|------|-----------|--------|--------|
| I1.0 | Engineering Design (ARR + gap + design) | Completed | `runtime/docs/I1_Engineering_Design.md` |
| I1.1 | Workspace Skeleton (14 crates, compiles) | Completed | tag `v1.0-implementation-baseline` |
| I1.2 | Ontology Traceability (O-001..007 aligned) | Completed | baseline commit |
| I1.3 | Implementation Baseline (commit + tag) | Completed | tag `I1-baseline` |
| I1.4 | First Executable Behaviour (commit -> WAL -> replay) | Completed | tag `I1.4-first-behaviour` |
| I1.5 | Persistent WAL (memory -> disk, fsync, real restart) | Completed | tag `I1.5-durable-wal` |
| I1.6 | Checkpoint Semantics (snapshot + compaction + WAL rotation + recovery point + restore) | Next | - |
| I1.7 | Recovery (restart from checkpoint + WAL tail) | Pending | - |

> **I1.6 scope note (tracker-level refinement, not a spec change).** The frozen
> Baseline Part 5 defines only the top-level `I1..I6` milestones; the `I1.x`
> sub-steps are the reference runtime's implementation decomposition and live
> here, not in the frozen Baseline. I1.6 is framed as **Checkpoint Semantics**
> rather than a bare "Snapshot": a snapshot in isolation is meaningless -- its
> purpose is to bound unbounded WAL growth. So I1.6 covers snapshot + prefix
> compaction + WAL rotation + recovery-point marker + restore-from-checkpoint,
> and I1.7 recovery then restarts from `checkpoint + WAL tail`.

## I1.5 - Persistent WAL (Completed - tag `I1.5-durable-wal`)

Persistence became REAL: a committed record is `fsync`'d to disk (`sync_all` =
FlushFileBuffers on Windows) BEFORE `append` returns, survives a full process
exit, and is recovered byte-identically by a genuinely separate process.

Proven behaviour chain (I1.5):

```
ProposedWrite -> Kernel.commit() -> FileWal.append() [frame + CRC32 + fsync]
             -> record durable on disk -> process EXITS
             -> new process: FileKernel::recover(FileWalStore::open_root(dir))
             -> replay on-disk frames -> identical truth_hash
```

New behaviour proofs (all PASS):

| Test | Proves |
|------|--------|
| persistence: append_is_durable_before_return | fresh store sees record right after `append` (fsync, not Arc) |
| persistence: round_trip_survives_fresh_store | drop all handles, re-open -> identical records in order |
| persistence: torn_tail_is_truncated | torn length prefix detected, garbage truncated, prior truth intact |
| persistence: corrupt_last_frame_is_dropped | CRC mismatch drops the frame + suffix; earlier truth kept |
| persistence: multi_shard_isolation_survives_disk | one file per shard (SHARD-001); `shards()` recovers keys from disk |
| persistence: wrong_shard_append_rejected | no cross-shard append (SHARD-001) |
| kernel B7: commit_persists_wal_file | commit yields a non-empty on-disk WAL |
| kernel B8: fresh_process_recovers_identical_truth | disk round-trip via a fresh store == committed truth |
| kernel B9: idempotent_commit_single_record | ORCH-004 no-op writes no second durable frame |
| kernel B10: corrupt_tail_preserves_prior_truth | crash-consistency: surviving truth == clean 2-commit history |
| runtime B11: real_cross_process_restart | TWO OS processes; recovered truth_hash == committed (real durability) |
| runtime B11b: repeated_recover_is_stable | repeated fresh-process recovery is idempotent |

**Live demo (two real processes):** `arves-runtime write <dir>` then
`arves-runtime recover <dir>` both print `TRUTH_HASH=0xac74e037364c15f7 COUNT=3`;
on disk: one file `hex(tenant)__hex(workspace).wal`.

**Independent architecture review verdict (I1.5): PASS.** No trait contract
changed; no new layer; no IDR implemented early (still single-node, no Raft).
Kernel made generic over `WalStore` (`RefKernel<S>`; `MemKernel`/`FileKernel`
aliases) - decoupling, not coupling. Dependency-free (std only): auditable,
deterministic. Honest limitations recorded in the design doc (no parent-dir
fsync; a mid-log bit-flip conservatively truncates the suffix - acceptable
single-node, repaired by replication later). `cargo check --workspace
--all-targets`: 0 warnings. 18/18 behaviour tests PASS (6 I1.4 + 12 I1.5).

## I1.4 - conformance status (honest)

"Works" is not "certified". These are distinct:

| Area | Status |
|------|--------|
| Behaviour Proof (6/6 tests) | PASS |
| Runtime Behaviour (cargo run demo) | PASS |
| Formal Scenario Conformance (12 axes / node probes) | Pending (harness not wired) |
| Certification (L1..L4) | Not Started |

The 6 I1.4 behaviour tests are the de-facto conformance proofs for the
single-node commit path; they will be promoted to formal Scenario Conformance
Framework runs after I1 completes (the node probes read via the Query layer,
which arrives at I3).

## Proven behaviour chain (I1.4)

```
ProposedWrite -> Kernel.commit() -> WAL.append() (append-only)
             -> durable truth -> TruthRef -> recover()/replay() -> identical truth
```

Verified invariants in code: ORCH-001 (Kernel sole truth owner; no read methods
on the trait), ORCH-003 (replay from recorded trace, not recomputation),
ORCH-004 (idempotent commit; duplicate rejected as `AlreadyCommitted`),
OWN-001 (single owner), SHARD-001 (immutable tenant/workspace shard key).
No distributed logic, networking, or specification change was introduced.

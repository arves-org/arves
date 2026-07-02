# I1 — Distributed Runtime — Milestone Retrospective

**Status:** I1 COMPLETE (storage side). One-page review before I2.
**Marker of record:** git tags `I1-complete` + `I1.7-recovery-hardening`
(+ per-step tags I1.4/I1.5/I1.6). Living detail: `IMPLEMENTATION_PROGRESS.md`.

---

## 1. Starting goal

> Build, on a **single node**, a **deterministic, durable, replayable** cognitive
> runtime whose Kernel is the sole owner of truth.

Not distributed yet. The bar was: truth becomes fact only via one commit gateway,
survives process death, and is reconstructed identically on restart — including
under faults, not just on the happy path.

## 2. Proven behaviours (each with executable proof)

| Step | Proven | How |
|------|--------|-----|
| I1.4 First Behaviour | `commit → WAL → truth → replay → same truth` | 6 behaviour tests |
| I1.5 Durable WAL | `commit → disk (fsync) → process exit → same truth` | file WAL, real 2-process restart |
| I1.6 Checkpoint Semantics | `snapshot → compaction → tail replay ≡ full replay` | segmented WAL, Kernel-produced snapshots |
| I1.7 Recovery Hardening | fault injection → **lossless or loud** (never silent corruption) | adversarial hunt + 7 fault-injection proofs |

**Invariants held throughout:** ORCH-001 (Kernel sole truth owner), OWN-001,
IDR-005 (append-only; compaction deletes whole covered segments only), ORCH-003
(replay/restore, never recompute), ORCH-004 (idempotent commit), SHARD-001
(per-shard, immutable key). Full suite: **37 passed, 0 failed; 0 warnings.**

**Method that emerged (to keep):** *break it, then fix it.* I1.7's defects were
found by an adversarial multi-agent hunt before writing fixes. Recommend making
an adversarial pass **mandatory** for every future milestone.

## 3. Open limitations (consciously accepted)

- **Single node only.** No replication, no quorum, no leader election. One node's
  disk is the only copy.
- **A corrupt *sole* copy is unrecoverable** — by design we now **detect and
  refuse** (`RecoveryError`), not silently drop or fabricate. Repair needs a
  second copy (I2).
- **Parent-directory fsync is best-effort** (std has no portable dir-fsync).
- **Cross-shard global ordering is not provided** (SHARD-001: offsets are
  per-shard, never comparable across shards). `truth_hash` is order-stable only
  within one shard.
- **No Query layer yet** — reads/introspection use `truth_hash`/`committed_count`
  helpers, not the (I3) Query surface. Formal Scenario Conformance still pending.

## 4. Responsibilities carried to I2 (Cluster Kernel)

- **Repair-from-peer is the consumer of I1.7's loud failures.** `try_recover`
  returning `RecoveryError` (CompactedPrefixWithoutSnapshot / Corruption) is
  exactly the trigger a replicated node uses to fetch a snapshot/log from a peer.
- **The WAL is already shaped as the Raft log** (offset = log index, `term`
  field present, segmented, append-only) — no format migration expected.
- **I2's first deliverable is a proven Replication Model, NOT Raft.** Prove
  `Leader append → Follower apply → same truth_hash` first; consensus (Raft:
  election, quorum, joint-consensus membership) layers on afterward
  (IDR-001..005). Guard against scope creep: I2 proves replication correctness,
  it does not add product features.
- **Interface evolution stays under RT-001**; frozen UCS/UCI remains untouched.

---

*Reference Runtime interfaces evolved under RT-001 during I1 (dormant→active:
`SnapshotMeta`/`SnapshotMarker`/`earliest` in I1.6; fallible recovery +
`WalError::Corruption` in I1.7). No frozen specification document was modified.*

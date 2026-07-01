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
| I1.5 | Persistent WAL (memory -> disk, fsync, real restart) | Next | - |
| I1.6 | Snapshot (compaction / checkpoint) | Pending | - |
| I1.7 | Recovery (restart from snapshot + WAL tail) | Pending | - |

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

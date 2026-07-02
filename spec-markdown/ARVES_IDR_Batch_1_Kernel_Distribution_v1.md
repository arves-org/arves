> **Rendered from `ARVES_IDR_Batch_1_Kernel_Distribution_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Implementation Decision Records - Batch 1: Kernel Distribution (IDR-001..005)

STATUS: REFERENCE IMPLEMENTATION DECISIONS (NOT A NORMATIVE STANDARD) - IMPLEMENTATION ERA

# Preamble - Era Shift

The Specification Era is closed and ARR is at PASS. These Implementation Decision Records (IDR) are the ADR analogue: they record engineering choices for the reference implementation. An IDR implements, but NEVER changes, the frozen specification.

**The goal of the Implementation Era is not to change the specification, but to prove that the specification can be implemented at production scale.**

# IDR-001 - Consensus Strategy

**Context: **The Kernel is the single owner of cognitive truth (ORCH-001, G-001, OWN-001). Distributed truth must not diverge. State is partitioned by tenant/workspace (SHARD-001).

**Decision: The Universal Cognitive Kernel SHALL operate as a CP system, using Raft, with one independent Raft group PER SHARD (tenant/workspace).**

**Alternatives rejected: **AP/CRDT for truth - contradicts single-source-of-truth (truth would be "eventually", not "one"). Global single-leader - does not scale to 10,000 nodes.

## Engineering Refinements (what makes CP scale)

- Per-shard consensus: each tenant/workspace shard is its own Raft group with its own leader. Tenant isolation (Vol 2) means no cross-tenant consistency is required.

- Replicate committed OUTCOMES, not engine invocations: engines are nondeterministic (LLMs); the leader runs the engine and replicates the resulting committed state transition; followers apply it, they do NOT recompute (ORCH-003).

- Engines run anywhere: engine compute is pure/stateless (LAYER-001) and distributes across nodes; only the COMMIT goes through the shard leader. Compute scaling is separated from consensus.

- The Raft log IS the WAL IS the decision trace: IDR-001 + IDR-005 + ORCH-003 converge on one ordered source for replay.

- No cross-shard atomic commit in v1: operations are single-shard atomic; cross-shard coordination uses sagas/compensation (Amendment-006), not distributed transactions.

## Consequences

- Leader election, consensus and commit quorum required (per shard).

- Writes are linearizable; replay is deterministic from the committed log.

- Read consistency is configurable (see tiers below).

## Non-goals (may remain AP)

Metrics, logs, monitoring, tracing, presence and capability statistics may remain eventually-consistent (AP / CRDT). Truth is CP; observability is AP.

## Read Consistency Tiers

| Tier | Guarantee | Path |
| --- | --- | --- |
| Linearizable | Latest committed truth | Through leader (read-index) |
| Bounded-staleness | Recent within a bound | Follower read |
| Eventual | Best-effort | Read/geo replica |

**Spec invariants upheld: **ORCH-001 (truth = Kernel), G-001, OWN-001, QUERY-001 (read-only), SHARD-001 (per-tenant partition).

# IDR-002 - Replication Strategy

**Decision: **Per-shard Leader -> Followers replication via the Raft log; periodic Snapshots + append-only WAL. Followers apply committed OUTCOMES (never recompute engines).

**Consequences: **Follower reads enabled (bounded-staleness); read/geo replicas allowed; truth is always committed through consensus.

# IDR-003 - Membership Strategy

**Decision: **Raft Joint Consensus for safe cluster membership changes (add/remove nodes without split-brain).

# IDR-004 - Leader Election

**Decision: **Raft leader election, one leader per shard. Leader loss triggers re-election; in-flight uncommitted work is discarded (no partial truth, Amendment-005/006).

# IDR-005 - Storage

**Decision: **Append-only WAL + Snapshots. The WAL is the ordered record of committed truth mutations and is the single source for deterministic replay (ORCH-003).

# CP / AP Boundary (summary)

| Concern | Model |
| --- | --- |
| Cognitive truth (Kernel state) | CP (Raft, per shard) |
| Plans / Engine Graph (Control Plane) | CP within shard context |
| Metrics / logs / tracing | AP (eventual) |
| Presence / capability statistics | AP (CRDT) |

# Next - I1 Distributed Runtime

With IDR-001..005 recorded, I1 begins real distributed engineering: implement per-shard Raft groups, the commit path (engine-anywhere -> leader commit -> follower apply), WAL/snapshot storage, and the read-tier query paths. Code, not specification.

*Final Definition  IDR Batch 1 = The Kernel Distribution Decisions of the ARVES Reference Implementation - CP truth on per-shard Raft.*

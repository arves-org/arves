> **Rendered from `ARVES_00_Architecture_Readiness_Review_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES v1.0 Architecture Readiness Review (ARR) v1.0

STATUS: READINESS REVIEW (IMPLEMENTATION-ERA ENTRY GATE)

# Part 1 - Purpose

A final review before the Implementation Era. The Specification Era asked "what is the correct architecture?"; the Implementation Era asks "can this architecture actually run distributed at production scale?" - a different question. This ARR is the entry gate to M10.

**The goal of the Implementation Era is not to change the specification, but to prove that the specification can be implemented at production scale.**

# Part 2 - Review Method & Verdicts

Six dimensions are reviewed against the frozen v1.0 corpus. Each item gets a verdict: READY (spec is sufficient), GAP (spec must be amended via CCP before M10), or IDR (a mechanism decision deferred to an Implementation Decision Record).

# Part 3 - Layer Independence

Layers: Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine -> Capability -> Execution.

| Check | Verdict | Note |
| --- | --- | --- |
| Downward-only dependencies | GAP | No explicit invariant; only the "Separation of Concerns" principle |
| No lateral coupling | GAP | Not stated normatively - add LAYER-001 |
| Cross-cutting via Control Plane / Event Fabric | READY | Two-plane model (Vol 9) covers this |
| Query is read-only projection | READY | Query reads state; must not mutate (confirm in contract) |

Finding F5: introduce LAYER-001 (dependencies point downward only; no lateral calls; cross-cutting concerns traverse the Control Plane or Event Fabric).

# Part 4 - Distributed Readiness

Per component: replicate / partition / replay / migrate / version / replace.

| Component | Replicate | Partition | Replay | Ready? |
| --- | --- | --- | --- | --- |
| Engine Fabric | Yes | Yes | Yes | READY (pure + idempotent, ORCH-004) |
| Control Plane | Yes | n/a | Yes | READY (stateless over Kernel, ORCH-002) |
| Capability Fabric | Yes | Yes | n/a | READY |
| Query | Yes | Yes | n/a | READY (by tenant/workspace) |
| LCW | Partial | Yes | Partial | GAP (declare tenant/workspace as shard key) |
| Persistence | Yes | Yes | Yes | IDR (WAL/replication strategy) |
| Kernel (truth) | Hard | Hard | Partial | IDR - highest risk (consensus/CAP) |

Finding F1: the stateless/pure components are ready; the stateful truth owner (Kernel) is the primary distributed risk. CAP posture and consensus are undecided (deferred to v2 as Federated Kernel / Consensus Extensions) - the first M10 IDRs. Finding F7: declare tenant/workspace as the partition/shard key.

# Part 5 - Runtime Ownership

Ownership of runtime responsibilities. Mostly clean thanks to Single Ownership + ORCH-001.

| Responsibility | Owner | Verdict |
| --- | --- | --- |
| Truth | Kernel | READY (ORCH-001) |
| Policy | Governance | READY |
| Execution | Execution Layer | READY |
| Planning (computation) | Engine | READY |
| Plan / Engine Graph (artifact) | Control Plane | GAP - see F3 |
| Capability Selection | Capability Planner | READY |

# Part 6 - State Ownership (most critical review)

Every state must have exactly ONE owner. Two conflicts found in the frozen corpus.

| State | Declared owner(s) | Verdict |
| --- | --- | --- |
| Truth | Kernel | READY |
| Execution state | Execution Layer | READY |
| Working Memory | Cognitive Core (Vol 4 Part 25) AND LCW | CONFLICT - F2 |
| Plans / Engine Graph | Engine (ARR draft) AND Control Plane (Vol 9) | CONFLICT - F3 |
| Capability Binding | Capability Fabric | READY |
| Persisted knowledge | Persistence / Information Core | READY |

Finding F2 (High): Working Memory is claimed by both Cognitive Core (Vol 4 Part 25 "Owns ... Working Memory") and the LCW. Resolve to a single owner via amendment before M10. Finding F3: the plan/Engine Graph is a Control Plane artifact (Vol 9 Part 3); the Engine owns inference only. Clarify by amendment.

# Part 7 - Control Plane Review

Is Vol 9 sufficient for distributed orchestration?

| Concern | Covered? | Note |
| --- | --- | --- |
| Orchestration | Yes | Vol 9 Engine Graph |
| Scheduling | Partial | Implicit; cluster-wide scheduling is I4 |
| Retries | Yes | ABI Retry Policy + ORCH-004 |
| Cancellation | No | GAP - F4 |
| Priorities / preemption | No | GAP - F4 |
| Human interrupts | Yes | Vol 9 Part 10 (HITL) |
| Policy interrupts | Yes | Vol 9 Part 10 |

Finding F4: add cancellation and priority/preemption semantics to Vol 9 before I4 (cluster-wide capability scheduling).

# Part 8 - Failure Review

The single largest unwritten area. Each failure must have a defined home before M10.

| Failure mode | Handled by |
| --- | --- |
| Capability timeout | Spec (ABI Timeout/Failure Policy) |
| Duplicate replay | Spec (ORCH-004 idempotency) |
| Control-plane crash | Spec (ORCH-002 stateless restart) |
| Node crash | IDR |
| Network partition | IDR (CAP posture - F1) |
| Split brain | IDR (consensus - F1) |
| Partial execution | GAP (define compensation/saga semantics) |
| Rollback | GAP (define rollback model) |
| Replay failure | IDR |
| Human cancellation | GAP (ties to F4 cancellation) |

Finding F6: define a failure taxonomy in the architecture (which failures the spec handles vs which are IDR mechanisms). The Kernel CAP posture (F1) is the single most important undecided item.

# Part 9 - Findings Summary (ranked)

Freeze does not mean flawless; the ARR is the gate that catches pre-implementation defects. GAP findings are resolved by amendment (MINOR) via the CCP process WITHOUT reopening the Specification Era; IDR findings are mechanism decisions for M10.

| ID | Finding | Severity | Resolution |
| --- | --- | --- | --- |
| F1 | Kernel distributed truth: CAP posture + consensus undecided | High | IDR (I1/I2) |
| F2 | Working Memory has two owners (Vol 4 vs LCW) | High | Amendment (CCP) |
| F3 | Plan/Engine Graph ownership ambiguous (Engine vs Control Plane) | Medium | Amendment |
| F4 | Vol 9 lacks cancellation + priority/preemption | Medium | Amendment |
| F5 | No explicit layering invariant | Medium | Amendment (LAYER-001) |
| F6 | Failure taxonomy not placed in architecture | Medium | Amendment + IDR |
| F7 | Partition/shard key (tenant/workspace) not declared | Low | Amendment |

# Part 10 - New Document Type: Implementation Decision Record (IDR)

The Implementation Era writes IDRs, not specifications - the ADR (Architecture Decision Record) analogue. An IDR records a mechanism decision that implements, but never changes, the frozen spec.

IDR template: ID, Title, Context, Decision, Alternatives, Consequences, Affected Components, Spec Invariants Upheld.

- IDR-001 Distributed replay - Decision: append-only WAL rather than Kafka.

- IDR-015 Consensus - Decision: the Kernel is not leaderless.

# Part 11 - Entry Gate Decision

M10 may begin once: (a) all GAP findings (F2, F3, F4, F5, F6-taxonomy, F7) are resolved by amendment; and (b) all IDR findings (F1, F6-mechanisms) have an opened IDR. The Specification Era does not reopen; amendments and IDRs are the only instruments.

# Part 12 - Success Criteria

The architecture is proven implementation-ready: layers independent, every state singly owned, every component distribution-assessed, and every failure mode assigned a home.

*Final Definition  Architecture Readiness Review = The Entry Gate from Specification to Implementation.*

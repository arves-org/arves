> **Rendered from `ARVES_00_Amendments_CCP_Batch_1_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES v1.0 Amendments - CCP Batch 1 (ARR Resolution)

STATUS: AMENDMENT RECORD (MINOR / CCP) - RESOLVES ARR FINDINGS WITHOUT REOPENING THE SPECIFICATION ERA

# Part 1 - Purpose

Resolve the GAP and CONFLICT findings of the Architecture Readiness Review (ARR) via amendments (MINOR) under the Reference Lifecycle CCP process. The Specification Era is NOT reopened; these are the first real use of the ARVES change process on ARVES itself.

# Amendment-001 - Working Memory Ownership (resolves F2)

**Decision: **LCW is the SINGLE owner of Working Memory (the live, mutable cognitive/world state).

**Detail: **The Cognitive Core USES Working Memory but does not own it. Per-invocation engine scratch is ephemeral, engine-local, and is NOT system state (engine purity).

**Affected: **Vol 4 Part 25: "Owns ... Working Memory" -> "Uses Working Memory (owned by LCW)".

**Invariant upheld: **OWN-001 (single owner per state); ORCH-001 (truth stays in the Kernel).

# Amendment-002 - Plan Ownership (resolves F3)

**Decision: **The Control Plane is the SINGLE owner of the Plan / Engine Graph artifact.

**Detail: **The Planning Engine PRODUCES plan proposals as inference; it does not own the plan. The executable Engine Graph is a Control Plane plan artifact - never truth.

**Affected: **Engine ABI: Planning engine Produces a plan-proposal type; Vol 9 confirmed as owner.

**Invariant upheld: **ORCH-001 (plan artifact != truth); OWN-001.

# Amendment-003 - LAYER-001 & Layer Responsibility Matrix (resolves F5)

**Invariant LAYER-001: **Dependencies point downward only; no lateral peer-layer calls; cross-cutting concerns traverse the Control Plane or Event Fabric.

Layer Responsibility Matrix:

| Layer | Owns | Reads | Writes | Cannot |
| --- | --- | --- | --- | --- |
| Reality | External world | - | Emits raw signals | Be mutated except via Execution |
| Information Platform | Provider/connector registry, canonicalization | Reality (raw) | Proposed canonical observations to Kernel | Own truth or hold cognitive state |
| Kernel | TRUTH (committed canonical state) | Proposed writes/inference | Commits truth; emits events | Orchestrate, plan or execute |
| Persistence | Durable store of committed state/events | Kernel commits | Durable records | Interpret meaning or decide |
| LCW | Working Memory / live world state | Kernel truth, events | Mutable live state (not truth) | Own truth or be authoritative store |
| Query | Read projections/views | Kernel, LCW, Persistence | NOTHING (read-only) | Mutate any state |
| Engine | Nothing persistent (pure) | State via Query | Inference + proposed effects/plan proposals | Mutate truth, own plans, or persist state |
| Capability | Capability registry & bindings | Plan requirements | Capability bindings | Own truth or plans |
| Execution | In-flight execution state | Execution plan | Actions to world; outcomes proposed to Kernel | Own truth or plan |
| Control Plane | Plan/Engine Graph, orchestration | Goals, state | Plans, schedules | Own truth or persist state (ORCH-001/002) |

# Amendment-004 - Shard Identity (resolves F7)

**Entity Key: **Ontology urn + id (Identity aspect, Ontology Spec Part 4).

**Partition / Shard Key: **tenant_id (primary) + workspace_id (secondary). All state partitioned by tenant/workspace; no cross-tenant data in a single shard.

**Routing Key: **tenant_id + correlation_id (Event Envelope).

**Invariant SHARD-001: **Partition by tenant/workspace; the partition key is immutable for an entity lifetime.

**Affected: **Vol 2 (tenant isolation), Vol 9 / ARVES-21 (envelope), Ontology (Identity aspect).

# Amendment-005 - Cancellation & Priority (resolves F4; added to reach 0 GAP)

**Cancellation: **Every engine invocation and plan is cancellable. Cancellation is cooperative and idempotent (ORCH-004); cancelled work emits a cancellation event and leaves no partial truth (uncommitted proposed writes are discarded).

**Priority / Preemption: **Plans and tasks carry a priority; the scheduler may preempt lower priority work. Preempted work is checkpointed to its decision trace and is replayable (ORCH-003).

**Affected: **Vol 9 Control Plane; Engine ABI adds Cancellation semantics + Priority field.

# Amendment-006 - Failure Taxonomy (resolves F6 taxonomy; mechanisms -> IDR)

**Model: **Partial execution is rolled back by NOT committing (uncommitted proposed writes are discarded, since the Kernel owns truth and engines are pure). Already-committed effects are compensated by explicit compensation actions recorded in the decision trace (saga-style).

Failure placement:

| Failure | Home |
| --- | --- |
| Capability timeout | Spec (ABI Timeout/Failure) |
| Duplicate replay | Spec (ORCH-004) |
| Control-plane crash | Spec (ORCH-002) |
| Partial execution / rollback | Amendment-006 (compensation model) |
| Human cancellation | Amendment-005 |
| Node crash / partition / split brain | IDR (F1 mechanisms) |
| Replay failure | IDR |

# New Invariants Registered

| ID | Invariant |
| --- | --- |
| OWN-001 | Every state has exactly one owner. |
| LAYER-001 | Dependencies downward only; no lateral coupling; cross-cutting via Control Plane/Event Fabric. |
| SHARD-001 | State is partitioned by tenant/workspace; partition key is immutable. |

# Post-Amendment ARR Status

| Finding | Status |
| --- | --- |
| F2 Working Memory ownership | RESOLVED (A-001) |
| F3 Plan ownership | RESOLVED (A-002) |
| F5 Layer invariant | RESOLVED (A-003) |
| F7 Shard identity | RESOLVED (A-004) |
| F4 Cancellation/priority | RESOLVED (A-005) |
| F6 Failure taxonomy | RESOLVED (A-006); mechanisms -> IDR |
| F1 Kernel consensus/CAP | OPEN as IDR (expected) |

**ARR RESULT: PASS  -  0 GAP, 0 CONFLICT. Only accepted IDRs remain open (F1 + F6 mechanisms).**

# Next - Implementation Decision Records

With ARR at PASS, the Implementation Era may begin with IDRs: IDR-001 Consensus, IDR-002 Replication, IDR-003 Membership, IDR-004 Leader Election, IDR-005 Storage. IDRs implement, but never change, the frozen specification.

*Final Definition  CCP Batch 1 = The First Amendment Cycle of ARVES, clearing the Architecture Readiness Review to PASS.*

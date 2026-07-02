> **Rendered from `ARVES_OS_Volume_1_Engineering_Constitution_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engineering Operating Manual - Volume 1: Engineering Constitution v1.0

STATUS: AEOS CONSTITUTION LAYER (NORMATIVE) - The governing volume of the ARVES Engineering Operating System

# Part 1 - Purpose & Scope

This is the constitutional volume of the ARVES Engineering Operating System (AEOS). It governs every engineering activity in the ARVES repository and defines who ARVES is built by, under what rules, and toward what end. It is normative: Volumes 2-6 (Workflow, Playbooks, Manuals) operate under it, and CLAUDE.md is its per-session enactment.

# Part 2 - UCS / UCI Split

ARVES is a Universal Cognitive Infrastructure expressed in two version lines. UCS (Universal Cognitive Standard) is the standard - ontology, contracts, engine ABI, conformance, certification. UCI is the reference implementation - Information Platform, Kernel, LCW, Persistence, Query, Fabrics, Runtime. A UCI runtime declares which UCS version it implements; independent runtimes may implement UCS without using UCI.

# Part 3 - Era Model

The Specification Era is COMPLETE and FROZEN (2026-07-01). The Implementation Era is in progress. No base standard is authored after the freeze; the focus is proving the frozen specification at production scale.

| Era | Status | Output |
| --- | --- | --- |
| Specification Era | Complete / Frozen | UCS, UCI spec, Ontology, Control Plane, ABI, Conformance, Lifecycle, ARR, Baseline |
| Implementation Era | In progress | Working code, IDRs, conformance reports, benchmarks, certification |

# Part 4 - The Primary Principle

Preserve the dependency chain and never reverse it:

Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation.

Implementation serves Specification. Specification never serves Implementation. Implementation proves the specification; it never changes it.

# Part 5 - Engineering Doctrine & Philosophy

Always prefer:

- Simple over Clever; Explicit over Implicit; Deterministic over Dynamic.

- Contracts over Assumptions; Architecture over Features; Correctness over Speed.

- Replayability over Convenience; Proof over Opinion; Standards over Frameworks.

- Engineering over Prompt Engineering.

# Part 6 - The Ten Non-Negotiable Rules

| # | Rule |
| --- | --- |
| 1 | Never modify the frozen specification. |
| 2 | Never invent architecture. |
| 3 | Never introduce new architectural layers. |
| 4 | Never bypass architectural ownership. |
| 5 | Never violate registered invariants. |
| 6 | Never add features because they seem useful. |
| 7 | Never optimize before correctness. |
| 8 | Never duplicate ownership. |
| 9 | Never couple runtime components unnecessarily. |
| 10 | Every engineering decision must be traceable to the frozen specification. |

# Part 7 - The Two Planes & The Layers

Control Plane (decides) vs Data Plane (carries); the Kernel never becomes the Control Plane. Layers, with downward-only dependencies (LAYER-001):

Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine -> Capability -> Execution, plus the cross-cutting Control Plane.

# Part 8 - Registered Invariants (normative)

| ID | Statement | Source |
| --- | --- | --- |
| ORCH-001 | Control Plane owns no truth; only the Kernel owns truth. | Vol 9 v2 Part 5 |
| ORCH-002 | Control Plane produces plans, never persistent state. | Vol 9 v2 Part 5 |
| ORCH-003 | Execution is replayable from a recorded decision trace, not recomputation. | Vol 9 v2 Part 5 |
| ORCH-004 | Every engine/capability invocation is idempotent and content-addressable. | Vol 9 v2 Part 5 |
| OWN-001 | Every state has exactly one owner. | Amendments A-001 |
| LAYER-001 | Downward-only deps; no lateral coupling; cross-cutting via Control Plane/Event Fabric. | Amendments A-003 |
| SHARD-001 | Partition by tenant/workspace; partition key immutable. | Amendments A-004 |

Ontology Design Principles O-001..007 are definitional (not runtime-provable). Proposed invariants G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005 are INFORMATIVE and pending CCP; they are not enforced as registered until ratified. See ARVES_00_Invariant_Registry.

# Part 9 - Current Distributed Decisions (IDR-001..005)

| Concern | Decision |
| --- | --- |
| Kernel | CP (Consistency First) |
| Consensus | Per-shard Raft (one group per tenant/workspace) |
| Replication | Leader -> Followers -> Snapshots -> WAL |
| Membership | Joint Consensus |
| Leader Election | Per shard |
| Storage | Append-only WAL = decision trace |
| Truth / Observability | Truth CP; Observability AP |

# Part 10 - Change Management

On discovering a specification issue: STOP. Do not implement around it. Classify and route:

| Issue | Instrument |
| --- | --- |
| Minor wording | CCP Amendment |
| Architectural ambiguity | Architecture Review |
| Engineering decision | IDR |
| Specification change | Next Major Version (via CCP -> Review -> Amendment -> Ratification -> Baseline v2) |

Never silently change the architecture. The frozen Baseline Part 5 milestone set (I1 Distributed Runtime, I2 Cluster Kernel, I3 Distributed Query, I4 Capability Scheduling, I5 Multi-Agent Runtime, I6 Reference Products) is the single source of truth and changes only through this chain.

# Part 11 - Definition of DONE

A milestone is DONE only when all of the following are PASS:

- Architecture PASS; Conformance PASS; Certification PASS; Independent Review PASS.

- Invariant coverage 100%; Replay PASS; Distributed tests PASS.

- No architecture drift; no specification drift.

# Part 12 - REJECT Criteria

Reject any work that:

- Modifies the frozen specification without a ratified CCP.

- Introduces a new layer, invariant, or milestone name by fiat.

- Creates a second owner for any state or concept.

- Cannot be traced to a frozen document, contract, invariant, or IDR.

- Cannot be reproduced by an independent implementation from the specification alone.

# Part 13 - When In Doubt

Never ask "Can this work?" Always ask "Does this preserve the standard?" If not, reject it.

# Part 14 - Relationship to the AEOS

This volume is the Constitution layer of the AEOS. Volume 2 defines the Workflow; Volumes 3-5 are the Playbooks; Volume 6 is the Certification & Review Manual. The frozen UCS/UCI v1.0 is the Specification layer; source code is the Runtime layer. CLAUDE.md enacts this constitution each session.

*Final Definition  Engineering Constitution = The Governing Law of the ARVES Engineering Operating System.*

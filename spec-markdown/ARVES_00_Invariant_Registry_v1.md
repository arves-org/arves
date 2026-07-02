> **Rendered from `ARVES_00_Invariant_Registry_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Invariant Registry v1.0

STATUS: INFORMATIVE COMPANION (INVARIANT REGISTER) - Registered entries are normative from their frozen source; Proposed entries carry NO conformance weight until ratified via CCP (Reference Lifecycle Part 6).

# Part 1 - Purpose & Standing

Single register of every ARVES invariant referenced by the Engineering Constitution (CLAUDE.md). It was produced by an independent audit of the frozen .docx corpus. Registered invariants are reproduced verbatim from their frozen source and are normative. Proposed invariants are grounded in the frozen corpus but were never formally ratified; they are INFORMATIVE and must pass the CCP process (with a conformance scenario, per Reference Lifecycle Part 6 CCP-GATE) before becoming normative. Proof status for all invariants is currently "pending" - no runtime code exists yet; each must gain an executable proof during its owning milestone.

# Part 2 - Registered Invariants (normative, frozen)

| ID | Statement | Source | Proof |
| --- | --- | --- | --- |
| OWN-001 | Every state has exactly one owner (Amendment-001: LCW is the single owner of Working Memory). | Amendments CCP Batch 1 - A-001 + registry table | pending |
| LAYER-001 | Dependencies point downward only; no lateral peer-layer calls; cross-cutting traverses the Control Plane or Event Fabric. | Amendments CCP Batch 1 - A-003 | pending |
| SHARD-001 | Partition by tenant/workspace; the partition key is immutable for an entity lifetime. | Amendments CCP Batch 1 - A-004 | pending |
| ORCH-001 | The Control Plane owns no truth. Only the Kernel owns cognitive truth. | Vol 9 Cognitive Control Plane v2 - Part 5 | pending |
| ORCH-002 | The Control Plane produces plans, never persistent state. | Vol 9 Cognitive Control Plane v2 - Part 5 | pending |
| ORCH-003 | Every execution is replayable from the same Goal, State, Policies, Capabilities and Runtime Fingerprint - via a recorded decision trace, not by recomputation. | Vol 9 Cognitive Control Plane v2 - Part 5 | pending |
| ORCH-004 | Every engine and capability invocation is idempotent and content-addressable. | Vol 9 Cognitive Control Plane v2 - Part 5 | pending |

# Part 3 - Ontology Design Principles (definitional, NOT runtime-provable)

Per the drift audit, these are Design Principles from the Ontology Spec, not runtime invariants; they are not subject to the "executable runtime proof" obligation.

| ID | Principle | Source |
| --- | --- | --- |
| O-001 | Everything is a Cognitive Entity. | Ontology Spec - Part 3 |
| O-002 | Every Entity has Identity. | Ontology Spec - Part 3 |
| O-003 | Every Observation has Provenance. | Ontology Spec - Part 3 |
| O-004 | Truth emerges from validated Evidence. | Ontology Spec - Part 3 |
| O-005 | Derivation is not Inheritance (lineage edges are relations, not subtypes). | Ontology Spec - Part 3 |
| O-006 | Every type is versioned and registered. | Ontology Spec - Part 3 |
| O-007 | The Ontology defines meaning, not storage; truth is owned by the Kernel (ORCH-001). | Ontology Spec - Part 3 |

# Part 4 - Proposed Invariants (informative; require CCP ratification)

Grounded in the frozen corpus (verified consistent, 0 contradictions) but not yet ratified. Each must enter via a CCP Amendment/IDR with a conformance scenario before CLAUDE.md may treat it as registered.

| ID | Proposed Statement | Grounded in | Proof |
| --- | --- | --- | --- |
| G-001 | The Kernel is the single global source of committed cognitive truth and the sole commit gateway: no state becomes truth except by commit through the Kernel. | IDR-001 citation; Kernel row of Layer Matrix; upholds ORCH-001/OWN-001 | pending |
| QUERY-001 | The Query layer is strictly read-only: serves projections over Kernel/LCW/Persistence and never mutates state. | Query row of Layer Matrix; IDR-001 "QUERY-001 (read-only)" | pending |
| LCW-001 | LCW is the single owner of Working Memory - the live, mutable cognitive/world state - which is never truth and never an authoritative store. | Amendment-001; LCW row of Layer Matrix; upholds OWN-001 | pending |
| PERSIST-001 | Persistence is a durable store of Kernel-committed state/events only; it never interprets meaning and never decides. | Persistence row of Layer Matrix | pending |
| CAP-001 | A capability is bound by the Capability Fabric, never owned as truth; the Fabric owns only registry and bindings. | Capability row of Layer Matrix; Vol 9 Part 3 | pending |
| CAP-002 | Capability selection is a Control Plane concern; the Capability Fabric only carries bindings and never selects/orchestrates. | Vol 9 Parts 2-3 (Planner=Control, Fabric=Data) | pending |
| CAP-003 | Every capability invocation is idempotent and content-addressable, so retries are safe by construction. | ORCH-004; Engine Graph Part 7 | pending |
| CAP-004 | A capability never mutates truth directly; state effects are proposed writes routed to the Kernel. | ORCH-001; Layer Matrix; Engine Graph Part 4 | pending |
| CAP-005 | Capabilities are declared as manifest requirements and bound by the runtime at execution time. | Engine Graph Parts 3 & 10 | pending |
| CAP-006 | Every capability invocation is side-effect-honest: external effects are declared and recorded in the decision trace. | Engine Graph Parts 4/15; Vol 9 Part 6 (mild extrapolation) | pending |
| CAP-007 | Every capability invocation carries a correlation_id and is recorded in the decision trace for replay. | ORCH-003; Vol 9 Parts 6-8; Engine Graph Part 10 | pending |
| CAP-008 | Every capability invocation is cancellable; cancellation is cooperative and idempotent, leaving no partial truth. | Amendment-005; ORCH-004 (broadened from engine to capability) | pending |
| CAP-009 | Every capability binding is versioned and captured in the Runtime Fingerprint for valid replay/conformance. | Vol 9 Part 6 (Runtime Fingerprint); Engine Graph Part 11 | pending |
| ENG-001 | An engine is a pure, stateless Data Plane function: owns nothing persistent; per-invocation scratch is ephemeral, not system state. | Vol 9 Part 3; Engine row of Layer Matrix; Amendment-001; Engine Graph Part 4 | pending |
| ENG-002 | An engine reads state and produces inference; it never mutates truth; writes are proposed effects only the Kernel may commit. | Engine Graph Part 4; ORCH-001 | pending |
| ENG-003 | Every engine invocation is idempotent and content-addressable, carrying an idempotency key. | ORCH-004; Engine Graph Parts 3/7 | pending |
| ENG-004 | Nondeterministic engines are replayed from the recorded decision trace, not recomputed; only Deterministic/Seeded engines may be recomputed. | ORCH-003; Engine Graph Part 6 | pending |
| ENG-005 | Every engine is defined by a semantically versioned, content-addressable manifest; a graph pins engine versions for replay/conformance. | Engine Graph Parts 9 & 11 | pending |

# Part 5 - Summary & Governance

14 registered (7 invariants + 7 ontology principles) - normative from frozen sources. 23 proposed - informative, pending CCP ratification with conformance scenarios. All 37 verified consistent with the frozen corpus (0 contradictions). No proposed invariant may be enforced as normative, and CLAUDE.md must not claim it as registered, until it passes the CCP-GATE. Adding/altering invariants uses CCP/Amendment/IDR - never a silent edit.

*Final Definition  Invariant Registry = The Audited Register of ARVES Invariants - what is proven-normative vs proposed.*

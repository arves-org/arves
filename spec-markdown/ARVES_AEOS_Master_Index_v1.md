> **Rendered from `ARVES_AEOS_Master_Index_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engineering Operating System (AEOS) - Master Index v1.0

STATUS: OPERATING SYSTEM MASTER INDEX (NORMATIVE) - The single entry point to how ARVES is engineered

# Part 1 - Purpose

AEOS is not a prompt collection; it is the official working system for any engineer or AI operating on ARVES. It organizes the corpus into four layers and defines the standard task format and operating loop for the Implementation Era.

# Part 2 - The Four Layers

| Layer | Contains | Role |
| --- | --- | --- |
| Constitution | OS Vol 1 (Engineering Constitution); CLAUDE.md; Invariant Registry; IDR Batch 1; Baseline; Freeze Record; Amendments; ARR | Governing law + frozen decisions |
| Workflow | OS Vol 2 (UltraCode Workflow); Vol 3 (Engineering Playbook); Vol 4 (Implementation Playbook); Vol 5 (Distributed Systems Playbook); Vol 6 (Certification & Review Manual); + Engineering Handbook (non-normative) | How work is done, reviewed, tested, certified |
| Specification | Frozen UCS/UCI v1.0: Ontology, Engine Graph ABI, Scenario Conformance, Reference Lifecycle, Cognitive Control Plane v2, and the original corpus (Vol 1-18, Catalogs) | WHAT is true (frozen, unchanged) |
| Runtime | Source code (per milestone I1..I6) - not yet written | The reference implementation (UCI) |

# Part 3 - Standard Task Template

From AEOS onward every milestone task uses this format; the constitution supplies the discipline, the task supplies only milestone + objective + criteria.

ARVES Engineering Operating System
Current Baseline: UCS/UCI v1.0
Current Era: Implementation Era
Current Milestone: I<n> - <frozen Baseline name>

Objective: Advance the implementation while preserving the frozen specification.

Success Criteria: Architecture PASS; Conformance PASS; Certification PASS; Independent Review PASS.

Constraints: The frozen specification SHALL NOT be modified. Any specification change requires CCP. Any implementation decision requires IDR.

Expected Deliverables: 1 Architecture Readiness Check; 2 Gap Analysis; 3 Engineering Design; 4 Critical Review; 5 Implementation; 6 Tests; 7 Conformance Report; 8 Independent Architecture Review.

Definition of Done: the milestone is complete only when every deliverable receives PASS.

# Part 4 - The Operating Loop

AEOS (Frozen) -> Task Prompt -> Architecture Review -> Engineering -> Tests -> Conformance -> Independent Review -> PASS -> Next Milestone.

The frozen Baseline milestone set (single source of truth): I1 Distributed Runtime -> I2 Cluster Kernel -> I3 Distributed Query -> I4 Capability Scheduling -> I5 Multi-Agent Runtime -> I6 Reference Products. New milestone names require CCP -> Review -> Amendment -> Ratification -> Baseline v2.

# Part 5 - Verification Status (AEOS volumes)

The 6 volumes were adversarially verified. Results:

| Check | Verdict |
| --- | --- |
| Spec-drift / phantom invariants / milestones (per volume) | PASS (6/6) |
| Cross-document consistency | PASS (3 high findings fixed: Vol 3 sibling naming, Vol 2/Vol 3 CP-vs-Control-Plane overload, milestone->level reconciled to Vol 6) |
| Specification traceability | PASS |
| Circular dependency | PASS |
| Architecture ownership | PASS |
| Future extensibility | PASS |

# Part 6 - Sealing

With AEOS complete and verified, no further specification documents, constitutions or playbooks are authored. Subsequent output is code, IDRs, conformance/benchmark/certification reports. The focus is I1 Distributed Runtime. Specification is frozen; implementation proves it.

*Final Definition  AEOS = The Operating System of ARVES Engineering - Constitution, Workflow, Specification, Runtime.*

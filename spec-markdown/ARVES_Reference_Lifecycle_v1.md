> **Rendered from `ARVES_Reference_Lifecycle_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Reference Lifecycle v1.0

STATUS: PROCESS CONSTITUTION (HOW A COGNITIVE STANDARD IS BORN, RATIFIED AND FROZEN) - FINAL NORMATIVE DOCUMENT OF THE SPECIFICATION ERA

# Part 1 - Purpose

Define how a cognitive concept becomes a frozen ARVES standard: the development methodology from idea to certified ecosystem. This is the last normative document of the Specification Era. Precedent: W3C Process Document, IETF RFC 2026, Kubernetes KEP, Semantic Versioning.

# Part 2 - Position & Authority

This document governs every UCS and UCI artifact and the process by which they change. When it is ratified, the Specification Era closes and the Implementation Era begins.

# Part 3 - The Lifecycle Stages (with exit criteria)

Each stage produces an artifact and closes only on its exit criterion. No stage is skipped.

| Stage | Produces | Exit criterion |
| --- | --- | --- |
| Idea | Problem statement | Problem is stated and scoped |
| Research | Prior-art & feasibility survey | Alternatives assessed |
| Theory | Conceptual model | Model is coherent and bounded |
| Formalization | Invariants + machine-checkable properties | Properties are testable (see Part 4) |
| Specification | Normative spec document | Passes independent-implementability test |
| Ontology | Registered types (uci.*) | Types registered and versioned |
| Contract | Interface & data contracts | Contracts explicit and typed |
| Reference Behaviour | Defined expected behaviour | Behaviour is unambiguous |
| Conformance | Scenario suite + PASS criteria | Suite exists and is versioned |
| Reference Runtime | Runnable implementation | Passes conformance at a level |
| Certification | Attested conformance at a level | Certificate issued against spec version |
| Reference Product | Product on a certified runtime | Product passes its scenario set |
| Reference Ecosystem | Third-party certified implementations | >=1 independent certified runtime |

# Part 4 - Formalization (what distinguishes ARVES from a framework)

Formalization turns Theory into invariants and machine-checkable properties that Conformance can assert - not prose. It is the bridge from idea to testable contract.

- Produces named invariants (e.g. ORCH-001..004, O-001..007).

- Produces properties a scenario can check (isolation held, plan replayable, no truth in control plane).

- Maps every theoretical claim to at least one testable assertion; unfalsifiable claims do not advance.

# Part 5 - Artifact Maturity Model

Every artifact carries a status. This systematizes the corpus "FROZEN AFTER APPROVAL" marker and resolves version drift.

| Status | Meaning |
| --- | --- |
| Draft | Under authoring; non-normative |
| Candidate | Feature-complete; under review and conformance definition |
| Ratified | Approved; normative for its version |
| Frozen | Locked; changes only by amendment or new major version |
| Deprecated | Scheduled for removal; still valid for one major cycle |
| Superseded | Replaced by a newer artifact (e.g. Vol 3/13/19 by the Ontology) |

# Part 6 - Change Governance (Cognitive Change Proposal)

All changes flow through a Cognitive Change Proposal (CCP) - the RFC/KEP analogue.

| CCP State | Requirement to advance |
| --- | --- |
| Proposed | Problem statement + affected artifacts |
| Accepted | Owner review; scope agreed |
| Formalized | Invariants/properties defined (Part 4) |
| Conformance-defined | At least one conformance scenario written |
| Ratified | Reference behaviour + passing conformance |
| Frozen | Versioned and locked |

Hard rule (CCP-GATE): No behaviour is ratified without a conformance scenario.

# Part 7 - Versioning & Amendment Policy

Standards are stable, not continuously changing. Semantic versioning applies at the standard level.

| Change type | Version effect |
| --- | --- |
| Breaking change to types/contracts/ABI | MAJOR (new major version) |
| Backward-compatible addition | MINOR (amendment) |
| Clarification, no behaviour change | PATCH |
| Removal of a type/engine | Deprecate one major cycle, then MAJOR |

Conformance suites are pinned to a spec version; a result is always stated as "N% at Level Lx against Spec vB / Suite vA".

# Part 8 - Two-Track Versioning (UCS and UCI)

UCS (the standard) and UCI (the reference implementation) have independent version lines. A UCI runtime declares which UCS version it implements - as a compiler declares which language standard it targets. A third-party runtime may implement a UCS version without using UCI.

# Part 9 - The Specification Freeze

Upon ratification of this document the following clause takes effect:

**UCS v1.0 and UCI v1.0 are hereby normatively FROZEN. From this point, changes are made ONLY by amendment (MINOR) or a new MAJOR version, via the CCP process.**

# Part 10 - Era Transition

The Specification Era closes; the Implementation Era begins. After freeze, no new base standards are written.

- Specification Era (complete): Foundation, Reference Model, Standards, Ontology, Runtime Architecture, Reference Runtime, Control Plane, Scenario Conformance, Engine ABI, Reference Lifecycle.

- Implementation Era (next): Distributed Runtime (M10), Multi-Agent Runtime (M11), Reference Products (M12), Enterprise Runtime, Marketplace, Certification Program, ARVES v2.

# Part 11 - Independent-Implementability Test

Acceptance bar: an independent team, given only this document, can propose, formalize, conformance-test, ratify and certify a change to ARVES without consulting the original authors. If not, the process is descriptive and must be made more normative.

# Part 12 - Success Criteria

ARVES has a single, normative, versioned process by which every standard is born, proven, ratified, frozen and superseded - independent of any individual.

*Final Definition  Reference Lifecycle = The Constitutional Process of ARVES, and the Close of the Specification Era.*

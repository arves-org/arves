> **Rendered from `ARVES_Universal_Cognitive_Ontology_Specification_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Universal Cognitive Ontology Specification v1.0

STATUS: SEMANTIC CONSTITUTION (MASTER TYPE SYSTEM OF THE UNIVERSAL COGNITIVE STANDARD)

# Part 1 - Purpose

Define the universal cognitive concepts ARVES recognizes and the type system every other standard derives from. This is the semantic constitution of the Universal Cognitive Standard (UCS). It answers: which cognitive concepts exist, how they relate, and how a runtime names their types.

# Part 2 - Position & Authority

This document is the ROOT of UCS. It is the normative dependency that closes the Engine Graph ABI (Reads/Writes/Produces resolve to types defined here). It SUPERSEDES the entity and relationship lists in Volume 3, Volume 13 and ARVES-19, which become non-normative once this registry is frozen. UCS (Ontology, Contracts, Engine ABI, Conformance, Certification) is the standard; UCI (Information Platform, Kernel, LCW, Persistence, Query, Runtime) is its reference implementation.

# Part 3 - Design Principles

| ID | Principle |
| --- | --- |
| O-001 | Everything is a Cognitive Entity. |
| O-002 | Every Entity has Identity. |
| O-003 | Every Observation has Provenance. |
| O-004 | Truth emerges from validated Evidence. |
| O-005 | Derivation is not Inheritance (lineage edges are relations, not subtypes). |
| O-006 | Every type is versioned and registered. |
| O-007 | The Ontology defines meaning, not storage; truth is owned by the Kernel (ORCH-001). |

# Part 4 - Cross-Cutting Aspects (mandatory mixins)

Shared attributes are defined ONCE as aspects and attached to every type - not copied per entity. This makes O-002 and O-003 mechanical and removes attribute duplication.

| Aspect | Provides |
| --- | --- |
| Identity | Stable identifier and type urn |
| Provenance | Source, collector, transformation, timestamps |
| Trust | Trust score and verification status |
| Temporal | Valid From, Valid To, Observed At (Vol 13 temporal model) |
| Tenant Scope | Tenant/workspace boundary (no scope, no meaning) |

# Part 5 - Root Type Lattice (the ONLY inheritance graph)

Inheritance (is-a) is reserved for genuine subtyping. The lattice is shallow: CognitiveEntity is the root (O-001); the root types below are its subtypes; domain entities (Part 8) subtype these. Derivation and decomposition are NOT part of this graph (see Part 7).

# Part 6 - Root Types

| URN | Type | Meaning |
| --- | --- | --- |
| uci.entity | Entity | A thing with identity in the cognitive world |
| uci.observation | Observation | A raw sensed/received input with provenance |
| uci.event | Event | Something that happened, with time and consequence |
| uci.signal | Signal | A low-level indication feeding observation |
| uci.fact | Fact | A validated truth claim |
| uci.evidence | Evidence | Support for or against a claim |
| uci.relationship | Relationship | A typed semantic link between entities |
| uci.knowledge | Knowledge | Structured, reusable meaning |
| uci.memory | Memory | Persisted cognitive recall |
| uci.goal | Goal | A desired future state |
| uci.intent | Intent | A committed direction toward a goal |
| uci.decision | Decision | A selected course of action |
| uci.policy | Policy | A governance rule |
| uci.constraint | Constraint | A limit on plans/actions |
| uci.capability | Capability | A functional ability that can be bound |
| uci.resource | Resource | A usable asset (document, API, model, tool) |
| uci.execution | Execution | A performed unit of action |
| uci.outcome | Outcome | The observed result of execution |

# Part 7 - Semantic Relations (NOT inheritance)

Lineage, decomposition and support are first-class RELATIONS, not subtype edges (O-005). This resolves the common error of modeling Observation->Fact or Goal->Task as inheritance.

| Relation | From -> To | Semantics |
| --- | --- | --- |
| supports | Observation -> Fact | Observation provides support for a fact (not is-a) |
| derived_from | Fact -> Evidence | A fact is derived from validated evidence |
| belongs_to | Fact -> Knowledge | A fact is part of a knowledge body |
| decomposes_into | Goal -> Plan -> Task -> Execution | Refinement, not subtyping |
| produces | Execution -> Outcome | Execution yields an outcome |
| causes | Event -> Event | Causal linkage between events |
| governs | Policy -> Execution | Policy constrains/authorizes execution |
| constrains | Constraint -> Plan | Constraint limits a plan |

# Part 8 - Domain Subtype Mapping (reconciles Vol 3 / 13 / 19)

Corpus vocabulary is preserved by mapping each domain term onto a root type via is-a. Nothing is lost; there is now ONE hierarchy.

| Corpus term | Root type (is-a) |
| --- | --- |
| Person, Organization, Team, Agent, Device | uci.entity |
| Workspace, Project, Conversation, Document, Location | uci.entity |
| Knowledge Object, Insight, Hypothesis, Lesson | uci.knowledge / uci.fact |
| Strategy, Plan | uci.goal (via decomposes_into) |
| Task | uci.execution (executable unit) |
| Provider data, raw input | uci.observation / uci.signal |

# Part 9 - Type Registry

Every type is registered and versioned. This registry is the single source of truth for cognitive types and the schema layer the corpus previously lacked.

- Naming: uci.<type>@<version> (e.g. uci.fact@1).

- Registry entry: { urn, version, aspects, schema, relations }.

- Versioning: backward-compatible = minor; breaking = major + new urn version.

- Governance: single owner, changelog; a runtime states which registry version it targets.

# Part 10 - Relationship to the Engine Graph ABI

The ABI loop now closes: Engine Manifest -> Input Type -> Ontology Registry -> Output Type. Engine Reads/Produces/Writes reference uci.* urns; any conformant runtime resolves types through this registry.

# Part 11 - Independent-Implementability Test

Acceptance bar: an independent team, given only this specification, can build a type system that interoperates with any conformant ARVES runtime - same types, same relations, same aspects. If not, the document is descriptive and must be made more normative.

# Part 12 - Success Criteria

ARVES has one versioned, registered, provenance-aware cognitive type system that all standards derive from and every runtime shares.

*Final Definition  Universal Cognitive Ontology = The Shared Meaning System and Master Type Registry of ARVES.*

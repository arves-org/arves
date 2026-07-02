> **Rendered from `ARVES_00_Documentation_Index_v2.1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Documentation Index v2.1

STATUS: MASTER REGISTRY (AUTHORITATIVE DOCUMENT REGISTER) - Supersedes Documentation Index v2.0. Complete register at the close of the Specification Era.

# Purpose

The single entry point to the frozen ARVES corpus. Every document with its status, version, type (Normative / Informative / Implementation Decision), conformance applicability, and its supersession or governing document. This register, not any prose, is the complete list.

# Legend

Status: Frozen | Ratified | Active | Superseded | Informative. Type: Normative (binding) · Informative (explanatory) · Impl-Decision (reference implementation, non-normative). Conf. = conformance required.

| Document | Ver | Status | Type | Conf. | Supersedes / Governed by |
| --- | --- | --- | --- | --- | --- |
| CLAUDE.md (Engineering Constitution) | v1.0 | Frozen | Normative | governs | Governed by Reference Lifecycle; refs Invariant Registry, Baseline, Freeze Record |
| ARVES_00 Documentation Index | v2.1 | Frozen | Informative | No | Supersedes Index v2.0 |
| ARVES_00 Documentation Index | v2.0 | Superseded | Informative | No | Superseded by v2.1 |
| ARVES_00 Documentation Index | v1.0 | Superseded | Informative | No | Superseded by v2.0 |
| ARVES_00 Invariant Registry | v1 | Frozen | Informative + Normative refs | No | Registers invariants; proposed entries pending CCP |
| ARVES_00 Specification Freeze Record | v1 | Frozen | Normative | No | v1.0 signature; Freeze Date 2026-07-01 |
| ARVES_00 Baseline | v1 | Frozen | Normative | No | v1.0 scope of record |
| ARVES_00 Architecture Readiness Review (ARR) | v1 | Frozen | Normative | No | Entry gate; result PASS after Amendments |
| ARVES_00 Amendments - CCP Batch 1 | v1 | Frozen | Normative | partial | Registers OWN/LAYER/SHARD-001; A-001..006; governed by Reference Lifecycle |
| ARVES_00 Gap Analysis | v1 | Informative | Informative | No | - |
| ARVES IDR Batch 1 - Kernel Distribution | v1 | Active | Impl-Decision | No | IDR-001..005; governed by Reference Lifecycle; non-normative |
| Universal Cognitive Ontology Spec | v1 | Frozen | Normative | Yes | Supersedes entity/rel lists in Vol 3/13/19 |
| Engine Graph Specification (ABI) | v1 | Frozen | Normative | Yes | Depends on Ontology |
| Scenario Conformance Framework | v1 | Frozen | Normative | Yes | Validates Engine ABI & runtimes |
| Reference Lifecycle | v1 | Frozen | Normative | Yes | Process constitution; governs CCP/IDR/Amendment |
| Vol 9 Cognitive Control Plane | v2 | Frozen | Normative | Yes | Supersedes Vol 9 v1; registers ORCH-001..004 |
| Master Blueprint | v2 | Ratified | Informative | No | Overview; coverage note below |
| Vol 1 Foundation Constitution | v2 | Frozen | Normative | No | - |
| Vol 2 Tenant & Identity Constitution | v2 | Frozen | Normative | Yes | - |
| Vol 3 Information Core Bible | v1 | Ratified | Normative | Yes | Entity lists superseded by Ontology |
| Vol 4 Cognitive Core Bible | v1 | Ratified | Normative | Yes | Working Memory ownership amended (A-001 -> LCW) |
| Vol 5 Strategic Core Bible | v1 | Ratified | Normative | Yes | - |
| Vol 6 Experience Core Bible | v1 | Ratified | Normative | No | - |
| Vol 7 Evolution Core Bible | v1 | Ratified | Normative | No | - |
| Vol 8 Embodied Core Bible | v1 | Ratified | Normative | No | Aspirational scope (Baseline Part 4) |
| Vol 9 Runtime & Event Fabric Bible | v1 | Superseded | Normative | No | Reclassified as Data Plane runtime under Vol 9 v2 |
| Vol 10 Engineering Bible | v1 | Ratified | Normative | No | - |
| Vol 11 Capability Atlas | v1 | Ratified | Normative | No | L3/L4 aspirational (Baseline Part 4) |
| Vol 12 Domain Atlas | v1 | Ratified | Informative | No | - |
| Vol 13 Ontology & Knowledge Atlas | v1 | Ratified | Normative | No | Entity/rel lists superseded by Ontology |
| Vol 14 Agent Architecture Atlas | v1 | Ratified | Normative | No | - |
| Vol 15 Product & UX Atlas | v1 | Ratified | Informative | No | - |
| Vol 16 Reference Architecture Atlas | v1 | Ratified | Normative | No | - |
| Vol 17 Security & Governance Atlas | v1 | Ratified | Normative | Yes | - |
| Vol 18 Deployment & Operations Atlas | v1 | Ratified | Normative | No | - |
| ARVES-19 Canonical Ontology | v1 | Superseded | Normative | No | Superseded by Universal Cognitive Ontology |
| ARVES-20 Service Catalog | v1 | Ratified | Normative | No | - |
| ARVES-21 Event Catalog | v1 | Ratified | Normative | Yes | - |
| ARVES-22 Data Catalog | v1 | Ratified | Normative | No | - |
| ARVES-23 Agent Catalog | v1 | Ratified | Normative | No | - |
| ARVES-24 API Catalog | v1 | Ratified | Normative | Yes | - |
| (25) unassigned | - | - | - | - | Numbering gap - reserved |
| ARVES-26 Master Backlog | v1 | Ratified | Informative | No | Execution backlog |

# Documentation Integrity Check

Run at the close of the Specification Era. Result: PASS (with recorded notes).

| Check | Result | Note |
| --- | --- | --- |
| Broken references | PASS | CLAUDE.md, Freeze Record, ARR, IDR all resolve to existing files; Invariant Registry now present |
| Superseded chains | PASS | Index v1->v2->v2.1; Vol 9 v1->v2; ARVES-19->Ontology; Vol 3/13 entity lists superseded-in-part |
| Single owner per document | PASS | Each document has exactly one type/owner |
| Single source per invariant | PASS | ORCH-* only in Vol 9 v2; OWN/LAYER/SHARD only in Amendments; O-* only in Ontology; no dual definition |
| Duplicate registration | PASS | Superseded index versions marked; no two documents claim the same authority |
| Invariant reference integrity | PASS | All CLAUDE.md invariant refs resolve to Registry; proposed vs registered separated |

# Notes

- Master Blueprint predates the catalogs and Specification-Era standards; this register (not the Blueprint prose) is the complete list.

- Document 25 remains unassigned (reserved).

- IDR Batch 1 is an Implementation Decision (non-normative): it implements but never changes the frozen spec.

- Proposed invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005) are informative until ratified via CCP.

*Final Definition  Documentation Index v2.1 = The Sealed Master Registry of the ARVES Specification Era.*

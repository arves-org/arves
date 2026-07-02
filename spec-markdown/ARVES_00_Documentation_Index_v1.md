> **Rendered from `ARVES_00_Documentation_Index_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Documentation Index v1.0

STATUS: MASTER INDEX (NAVIGATION & DOCUMENT REGISTER)

# Purpose

This index is the top-level map of the ARVES documentation corpus. It registers every document, its version, its category, and its role, so readers know where to start and which document is authoritative for a given concern. It also records known structural notes (numbering gaps and version drift) so the corpus can be read accurately.

# How to Read the Corpus

Start with the Master Blueprint for the whole-platform picture. Read Volume 1 (Foundation) and Volume 2 (Tenant & Identity) next for the governing principles. The Core Bibles (Volumes 3-9) define each intelligence core in the Information -> Cognitive -> Strategic -> Experience -> Evolution (+ Embodied) cycle. Volumes 10-18 cover engineering, capability, domain, ontology, agent, product, architecture, security and operations. The Catalogs (19-26) are the executable inventories (Ontology, Service, Event, Data, Agent, API) and the Master Backlog.

# Document Register

| Doc | Title | Ver | Category | Authoritative For |
| --- | --- | --- | --- | --- |
| Blueprint | ARVES Master Blueprint | v2 | Umbrella Reference | Whole-platform architecture overview |
| Vol 1 | Foundation Constitution | v2 | Foundational | Mission, vision, design & architectural principles |
| Vol 2 | Tenant & Identity Constitution | v2 | Foundational | Tenancy, identity, roles, permissions, ownership |
| Vol 3 | Information Core Bible | v1 | Core Bible | Providers, ingestion, canonical knowledge, provenance |
| Vol 4 | Cognitive Core Bible | v1 | Core Bible | Context, memory, reasoning, decision, reflection |
| Vol 5 | Strategic Core Bible | v1 | Core Bible | Goals, planning, tradeoffs, simulation, priorities |
| Vol 6 | Experience Core Bible | v1 | Core Bible | Workspace, search, conversation, voice, presence |
| Vol 7 | Evolution Core Bible | v1 | Core Bible | Learning, reflection, calibration, benchmarking |
| Vol 8 | Embodied Core Bible | v1 | Core Bible | Vision, sensors, navigation, robotics, world state |
| Vol 9 | Runtime & Event Fabric Bible | v1 | Core Bible | Events, commands, tasks, workflows, orchestration |
| Vol 10 | Engineering Bible | v1 | Atlas / Standards | Engineering standards, stack, testing, CI/CD, SRE |
| Vol 11 | Capability Atlas | v1 | Atlas / Standards | L1-L4 capability map and maturity model |
| Vol 12 | Domain Atlas | v1 | Atlas / Standards | Business & intelligence domains |
| Vol 13 | Ontology & Knowledge Atlas | v1 | Atlas / Standards | Knowledge model, trust, provenance (see note) |
| Vol 14 | Agent Architecture Atlas | v1 | Atlas / Standards | Agent model, memory, delegation, governance |
| Vol 15 | Product & UX Atlas | v1 | Atlas / Standards | Product tiers, workspaces, UX surfaces |
| Vol 16 | Reference Architecture Atlas | v1 | Atlas / Standards | Layers, services, data & model architecture |
| Vol 17 | Security & Governance Atlas | v1 | Atlas / Standards | Security domains, governance, compliance, audit |
| Vol 18 | Deployment & Operations Atlas | v1 | Atlas / Standards | Deployment models, SRE, FinOps, DR |
| 19 | Canonical Ontology | v1 | Catalog | AUTHORITATIVE entity/relationship model (see note) |
| 20 | Service Catalog | v1 | Catalog | Authoritative service inventory |
| 21 | Event Catalog | v1 | Catalog | Authoritative event contracts |
| 22 | Data Catalog | v1 | Catalog | Authoritative data assets & storage |
| 23 | Agent Catalog | v1 | Catalog | Authoritative agent registry |
| 24 | API Catalog | v1 | Catalog | Authoritative API contracts |
| 25 | (missing / unassigned) | - | Gap | No document present - see Structural Notes |
| 26 | Master Backlog | v1 | Catalog | Execution/implementation backlog |

# Structural Notes

## Numbering gap: document 25

The sequence runs 24 (API Catalog) directly to 26 (Master Backlog). No document 25 exists in the corpus. Either assign document 25 (a natural slot would be an "Integration / Connector Catalog" or "Metrics & KPI Catalog") or renumber the Master Backlog to 25 and document the decision here.

## Version drift: v1 vs v2

The Master Blueprint, Volume 1, and Volume 2 are at v2; every other document is v1. There is no changelog recording what v2 changed or confirming the v1 documents remain consistent with the v2 foundation. Recommendation: add a one-line changelog to each v2 document and confirm alignment.

## Authoritative-source conflicts (ontology)

Three documents each present a "canonical" entity/relationship model and they disagree:

- ARVES-19 (Canonical Ontology) lists 16 entities including Workspace, Document and Location, and includes the relationships SUPPORTS and USES.

- Volume 13 (Ontology & Knowledge Atlas) lists 13 entities and omits Workspace, Document and Location; its relationship list omits SUPPORTS and USES.

- Volume 3 (Information Core) lists a further variant that adds Email and Meeting.

Resolution: treat ARVES-19 (Canonical Ontology) as the single source of truth for entities and relationships. Volumes 3 and 13 should reference it rather than restate a divergent list. Until reconciled, ARVES-19 governs.

## Coverage claim

The Master Blueprint states "Volumes 1-18 define ..." but does not account for the Catalogs (19-26). This index supersedes that statement as the complete register of the corpus.

# Legend

- Foundational - governing constitutions; read first.

- Core Bible - deep specification of one intelligence core.

- Atlas / Standards - cross-cutting maps and engineering standards.

- Catalog - executable inventories (ontology, services, events, data, agents, APIs) and backlog.

- Umbrella Reference - the whole-platform overview.

*Final Definition  Documentation Index = The Navigation Layer and Document Register of the ARVES Corpus.*

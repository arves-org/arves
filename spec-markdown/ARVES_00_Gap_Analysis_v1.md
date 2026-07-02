> **Rendered from `ARVES_00_Gap_Analysis_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Gap Analysis v1.0

STATUS: GAP ANALYSIS (WHAT IS MISSING ACROSS THE CORPUS)

# Executive Summary

The ARVES corpus (26 documents) is a strong, internally consistent VISION and framework: the Information -> Cognitive -> Strategic -> Experience -> Evolution (+ Embodied) spine is applied uniformly, ownership and tenancy principles are sound, and the catalog structure is the right shape. However it is NOT yet an implementable specification. Almost every entry is a heading plus a one-line restatement; the concrete design layer (schemas, contracts, diagrams, examples, numbers) is absent. This document registers exactly what is missing, grouped into six areas, with impact and recommended action for each.

Maturity today: roughly 90% "naming & structure", 10% "content". The single highest-value next step is to deepen one vertical slice end-to-end and reconcile the conflicting canonical models, rather than adding more headings.

# Gap Register

| Area | Missing Item | Impact | Recommended Action | Source Doc |
| --- | --- | --- | --- | --- |
| Content depth | Real entity schemas (field types, required, cardinality) | High | Replace copy-paste definitions with typed schemas per entity | ARVES-19, Vol 3, Vol 13 |
| Content depth | Example payloads / sample data | High | Add one concrete JSON example per entity, event, API | ARVES-19/21/22/24 |
| Content depth | API signatures (paths, verbs, params, errors) | High | Specify endpoints, request/response bodies, status & error models | ARVES-24 |
| Content depth | Event payload schemas & versioning examples | High | Define payload schema + sample message per event | ARVES-21 |
| Content depth | Storage design (tables, indexes, tenant isolation) | High | Add physical data model + isolation mechanics | ARVES-22 |
| Visuals | Architecture / C4 / sequence / ER / deployment diagrams | High | Add at least layer, ER and one runtime sequence diagram | Vol 16, Vol 9 |
| Visuals | Dependency graphs (service, capability, event producer/consumer) | Medium | Fill the catalog templates with actual dependency links | ARVES-20/21, Vol 11 |
| Engineering realism | Committed vs. aspirational split (AGI/robotics) | High | Tag near-term vs north-star; sequence the roadmap | Blueprint, Vol 8/11, ARVES-26 |
| Engineering realism | Concrete non-functional targets (SLO/SLA, scale) | Medium | Set p99 latency, uptime %, throughput, capacity numbers | Vol 10, Vol 18 |
| Engineering realism | FinOps / budget policy (token, compute) | Medium | Define concrete cost budgets & routing cost policy | Vol 18, Vol 10 |
| Consistency | Conflicting canonical entity/relationship models | High | Make ARVES-19 the single source of truth; others reference it | Vol 3, Vol 13, ARVES-19 |
| Consistency | Numbering gap (document 25 missing) | Low | Assign #25 or renumber; record decision | Corpus-wide |
| Consistency | Version drift (v2 vs v1, no changelog) | Low | Add changelog to v2 docs; confirm v1 alignment | Blueprint, Vol 1, Vol 2 |
| Consistency | No traceability matrix (backlog->capability->service->event->API) | Medium | Build a trace matrix linking every backlog item | ARVES-26 |
| Product layer | Personas, user stories, acceptance criteria | Medium | Add personas and end-to-end user scenarios | Vol 15, ARVES-26 |
| Product layer | Wireframes / UX flows | Medium | Add at least one end-to-end UX flow | Vol 15 |
| Governance/Ops | RACI / ownership matrix (team -> core/service) | Medium | Define who owns each core and service | Vol 2, ARVES-20 |
| Governance/Ops | Concrete test strategy (coverage targets, examples) | Medium | Set coverage goals and example test cases | Vol 10 |
| Governance/Ops | Compliance control mapping (GDPR/SOC2/ISO -> mechanism) | Medium | Map each control to a concrete platform mechanism | Vol 17 |
| Governance/Ops | Glossary / terminology dictionary | Low | Define all specialized terms in one place | Corpus-wide |

# 1. Content Depth (most critical)

- No real schemas: no field types, requiredness, length, format, or cardinality anywhere. In ARVES-19 all 16 entities share the identical definition, which cannot be correct.

- No example payloads or sample data: not a single JSON event, API request/response body, or example record exists.

- No API signatures: ARVES-24 lists API names only - no paths, HTTP verbs, parameters, status codes, or error models.

- No event schemas: ARVES-21 lists envelope fields but no per-event payload schema, versioning example, or sample message.

- No storage design: ARVES-22 says which data lives in which store but gives no table/collection design, indexes, or partition/tenant isolation mechanics.

# 2. Visual & Structural Assets

- Not a single diagram exists - no architecture/layer, C4, sequence, ER, data-flow, deployment topology, or event-flow diagram.

- No dependency maps: service-to-service, capability-to-capability, or event producer/consumer graphs. The catalogs define templates but leave them unfilled.

# 3. Engineering Realism

- No committed-vs-aspirational split: SaaS identity and L4 AGI / robotics / drones carry the same "FROZEN/approved" weight. The roadmap has no timing, sequencing, dependencies, or team/effort estimates.

- Non-functional requirements are not concrete: SLO/SLA is "measured" but no real targets (p99 latency, uptime %, throughput). No capacity/scale numbers.

- No cost/FinOps model: model routing says "manage cost" but there is no concrete token/compute budget policy.

# 4. Consistency / Governance Gaps

- Single-source-of-truth conflict: Docs 3, 13 and 19 each present a different "canonical" entity/relationship model (Workspace/Document/Location, SUPPORTS/USES, Email/Meeting differences).

- Numbering gap: document 25 is missing (24 jumps to 26).

- Version drift: Blueprint/Vol1/Vol2 = v2, the rest v1, with no changelog.

- No traceability: the Backlog (26) says every item must trace to a capability/service/event, but no actual trace matrix (backlog -> capability -> service -> event -> API) exists.

# 5. Product / User Layer

- No personas, user scenarios, user stories, or acceptance criteria. The Product Atlas (15) lists surfaces but describes no single end-to-end flow (e.g. user creates a goal -> agent assigned -> result returned).

- No wireframes or UX flows.

# 6. Governance / Operations Detail

- No RACI / ownership matrix (which team owns which core/service).

- Test strategy is not concrete: Vol 10 lists test types but gives no coverage target or example.

- No compliance control mapping: GDPR/SOC2/ISO27001 are named, but controls are not mapped to mechanisms.

- No glossary / terminology dictionary despite many specialized terms.

# Recommended Closure Order

- Reconcile the canonical model: make ARVES-19 authoritative; update Vol 3 and Vol 13 to reference it. (Removes the most damaging inconsistency.)

- Deepen one vertical slice end-to-end (e.g. Person): entity schema -> Data Contract -> Service -> Events -> API, with real field types and one example payload. This becomes the template for all others and proves the catalog templates work.

- Add core diagrams: layer architecture, ER for the canonical entities, and one runtime sequence (goal -> plan -> agent -> execution -> result).

- Split committed vs. aspirational and sequence the roadmap with dependencies and rough effort.

- Set concrete non-functional targets and a traceability matrix.

- Fill the remaining governance/product gaps: RACI, personas/stories, compliance mapping, glossary.

*Final Definition  Gap Analysis = The Register of What the ARVES Corpus Must Add to Become Buildable.*

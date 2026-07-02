> **Rendered from `ARVES_OS_Volume_3_Engineering_Playbook_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Volume-3: Engineering Playbook v1.0

STATUS: ENGINEERING PLAYBOOK (VOLUME 3 OF THE ARVES ENGINEERING OPERATING MANUAL) - NORMATIVE PROCESS STANDARD, FROZEN AFTER APPROVAL. Specification Era FROZEN as of 2026-07-01; Implementation Era in progress.

# Part 1 - Purpose and Position of This Playbook

Turn the frozen ARVES specification into disciplined engineering practice without ever changing the specification.

Volume 3 is the process spine of the ARVES Engineering Operating Manual. Volume 1 (Engineering Constitution) governs and Volume 2 (UltraCode Workflow) defines the process; Volumes 3-6 are the Playbooks and Certification Manual under them. (The separate legacy domain corpus - Foundation, Tenant/Identity, Core Bibles - is NOT part of this AEOS set.) This Playbook establishes HOW an ARVES engineer designs, self-reviews, gap-analyses, review-gates, codes, organizes, and commits work so that Implementation proves the Specification and never mutates it.

The Playbook is deliberately code-light at the design stage. Design artifacts describe behaviour, guarantees, and trade-offs. Code appears only after a design has passed self-review, gap analysis, and Architecture Readiness Review (ARR).

Non-negotiable framing rules for every reader:

- The dependency chain is never reversed: Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation. Implementation proves the spec; it does not amend it.

- The Control Plane decides; the Data Plane carries. The Kernel never becomes the Control Plane.

- Only REGISTERED invariants are normative. PROPOSED invariants are informative and must be flagged "proposed (pending CCP)". Ontology principles O-001..O-007 are definitional, not runtime-provable.

- Distributed content is grounded strictly in IDR-001..IDR-005 (reference implementation decisions, non-normative). No new architecture, layers, invariants, or milestones may be invented.

# Part 2 - Normative Vocabulary and Source of Authority

Precise words prevent architectural drift. This Part fixes the meaning of every term the Playbook uses to gate work.

| Term | Meaning | Authority Class |
| --- | --- | --- |
| UCS | Universal Cognitive Standard - the standard itself | Frozen standard |
| UCI | Universal Cognitive Infrastructure reference implementation of UCS | Reference implementation |
| Registered invariant | Normative, enforceable rule (ORCH/OWN/LAYER/SHARD families) | Normative |
| Proposed invariant | Candidate rule pending CCP; informative only | Informative |
| Ontology principle O-001..007 | Definitional design principle, not runtime-provable | Definitional |
| IDR | Implementation Decision Record; a reference choice, non-normative | Non-normative |
| CCP | Cognitive Change Proposal - the instrument to change the standard | Change instrument |
| Amendment | Ratified change to a frozen artifact via CCP flow | Change instrument |
| ARR | Architecture Readiness Review producing READY/GAP/IDR verdicts | Process gate |

Rule of authority precedence when documents conflict: Frozen Standard (UCS) > Registered Invariants > Ratified Amendments > IDRs > Proposed invariants > working design notes. An engineer who finds a genuine conflict raises a CCP; they do not silently resolve it in code.

# Part 3 - The Layer Map and Responsibility Matrix Every Design Must Respect

Every design must place itself precisely on the layer map before any other analysis begins.

Layers (downward-only dependencies, LAYER-001; cross-cutting only via Control Plane / Event Fabric):

- Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine -> Capability -> Execution, plus the cross-cutting Control Plane.

Layer Responsibility Matrix (Owns / Reads / Writes / Cannot). This matrix is the first thing an ARR checks.

| Layer | Owns | Writes | Cannot |
| --- | --- | --- | --- |
| Kernel | TRUTH (commits) | Committed outcomes only | Orchestrate / plan / execute |
| LCW | Working Memory / live state | Live state (not truth) | Assert truth |
| Query | Nothing (READ-ONLY) | Nothing | Write any state |
| Engine | Nothing persistent (pure/stateless) | Proposed effects only | Commit; hold persistent state |
| Capability Fabric | Registry + bindings | Registry/bindings | Own truth or plans |
| Control Plane | Plan / Engine Graph | Plans (never persistent state) | Own truth or persistent state |

Governing registered invariants for this Part: LAYER-001 (downward-only, no lateral coupling), OWN-001 (every state has exactly one owner), ORCH-001 (only the Kernel owns truth), ORCH-002 (Control Plane produces plans, never persistent state).

# Part 4 - The Engineering Design Standard: Structure and Rationale

A design is complete only when all mandated sections are present, each answered concretely, with no code.

Every ARVES component design document (a "Design") MUST contain the following 22 sections, in this order. Empty or "N/A" sections are permitted only with an explicit justification; silent omission fails ARR.

| # | Section | Core Question It Answers |
| --- | --- | --- |
| 1 | Responsibilities | What single purpose does this component own? |
| 2 | Inputs | What does it consume, and from which layer? |
| 3 | Outputs | What does it produce, and who may commit it? |
| 4 | Dependencies | What does it depend on (downward only)? |
| 5 | Lifecycle | How is it created, versioned, retired? |
| 6 | State Model | What states exist and who owns each (OWN-001)? |
| 7 | Distributed Behaviour | How does it behave across shards/nodes? |
| 8 | Concurrency | What runs in parallel and how is order enforced? |
| 9 | Failure Modes | How can it fail, partially or fully? |
| 10 | Recovery | How is a healthy state restored? |
| 11 | Replay | How is behaviour reproduced from the decision trace? |
| 12 | Consistency | Which consistency tier per read/write path? |
| 13 | Availability | What is available during partition/failure? |
| 14 | Scalability | How does it grow with tenants/volume? |
| 15 | Performance | What latency/throughput targets and why? |
| 16 | Security | Trust boundaries, authz, tenant isolation? |
| 17 | Observability | What is emitted to see inside it? |
| 18 | Metrics | What numbers prove health and SLOs? |
| 19 | Auditability | What is provable after the fact and how? |
| 20 | Trade-offs | What was consciously sacrificed and why? |
| 21 | Risks | What could go wrong beyond known failure modes? |
| 22 | Open Questions | What is undecided and who must decide? |

No code at design stage. Designs may show state names, message shapes as prose, sequence descriptions, and tables - never source code, class definitions, or SQL. Code is a downstream proof of the design, authored only after ARR verdict READY.

# Part 5 - Design Standard Deep Dive: Responsibilities through Dependencies

The first four sections bind a component to the layer map and the ownership model before any dynamic behaviour is described.

Responsibilities. State exactly one primary responsibility in one sentence. If two sentences are needed, the component is probably two components. Explicitly list what the component does NOT do, quoting the "Cannot" column of the Responsibility Matrix for its layer (Part 3). Example discipline: an Engine design states "Produces inference as proposed effects; cannot commit - only the Kernel commits."

Inputs. Enumerate every input with: source layer, whether it is truth (Kernel), live state (LCW), a read projection (Query), or a plan (Control Plane), and its consistency expectation. An input from a lateral layer is a LAYER-001 violation and must be rejected at design time.

Outputs. For each output declare: who is permitted to commit it. Engines and the Control Plane produce proposals/plans; only the Kernel commits truth (ORCH-001, ORCH-002). Content-addressability of every produced artifact must be stated where ORCH-004 applies.

Dependencies. List downward dependencies only. Cross-cutting needs (events, decisions) route through the Control Plane or Event Fabric, never through a sibling layer. Any dependency that reverses the chain Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation is forbidden.

- Checklist: Is the responsibility a single sentence?

- Checklist: Does the "Cannot" list match the layer matrix exactly?

- Checklist: Are all inputs downward or cross-cutting-via-Control-Plane?

- Checklist: Is every committed output routed through the Kernel?

# Part 6 - Design Standard Deep Dive: Lifecycle, State Model, and Ownership

State is the most dangerous thing an engineer designs; ownership must be unambiguous.

Lifecycle. Describe creation, activation, versioning, deprecation, and retirement. Every type is versioned and registered (ontology principle O-006, definitional). Align lifecycle stages with the frozen Reference Lifecycle (Idea -> Research -> Theory -> Formalization -> Specification -> Ontology -> Contract -> Reference Behaviour -> Conformance -> Reference Runtime -> Certification -> Reference Product -> Reference Ecosystem) and maturity Draft -> Candidate -> Ratified -> Frozen -> Deprecated -> Superseded where the component is a standard artifact.

State Model. Enumerate every distinct state and its transitions. For each unit of state name its single owner (OWN-001). If a state appears to have two owners, the design is wrong; split ownership or introduce an explicit hand-off through the Control Plane.

| State Class | Owner Layer | Persisted? | Consistency on Read |
| --- | --- | --- | --- |
| Committed truth | Kernel | Yes (WAL + snapshot) | Linearizable via leader |
| Working memory / live state | LCW | Live, not truth | Bounded-staleness / eventual |
| Read projection | Query | No (derived) | Per requested tier |
| Plan / Engine Graph | Control Plane | No (never persistent) | N/A - not truth |
| Proposed effect | Engine | No (pre-commit) | N/A until Kernel commits |

The CCP-GATE reminder: no behaviour is ratified without a conformance scenario. A state model that cannot be exercised by a conformance scenario is not yet ratifiable.

# Part 7 - Design Standard Deep Dive: Distributed Behaviour, Concurrency, and Consistency

Distributed behaviour must be grounded only in IDR-001..IDR-005; engineers do not invent replication schemes.

Distributed Behaviour (grounded in IDR-001). The Kernel is CP (the CAP consistency class - Consistent + Partition-tolerant - NOT the ARVES Control Plane, which owns no truth per ORCH-001) realized as per-shard Raft: one Raft group per tenant/workspace shard (SHARD-001, partition key immutable). Engines run anywhere; only the commit goes through the shard leader. The Raft log IS the WAL IS the decision trace. There is NO cross-shard atomic commit in v1 - cross-shard work uses sagas.

Replication and membership. IDR-002: leader -> followers replication plus snapshots plus WAL. IDR-003: membership via Raft joint consensus. IDR-004: per-shard leader election. IDR-005: append-only WAL plus snapshots. Truth is CP; observability (metrics/logs/tracing/presence) is AP.

Concurrency. Describe what executes in parallel and how ordering is enforced. Because replication carries committed OUTCOMES not engine invocations (ORCH-003), engines may run concurrently and even redundantly; correctness comes from idempotent, content-addressable invocation (ORCH-004) and single-leader commit.

Consistency tiers. Every read path must declare its tier:

| Tier | Path | Use When |
| --- | --- | --- |
| Linearizable | Through the shard leader | Truth must be current (authorization, commit checks) |
| Bounded-staleness | Follower read | Recent-enough truth acceptable (dashboards) |
| Eventual | Replica read | High-volume, staleness tolerable (analytics) |

Design rule: default to the weakest tier that is still correct for the use case, and justify any use of linearizable reads by the cost of leader load.

# Part 8 - Design Standard Deep Dive: Failure Modes, Recovery, and Replay

A design that has not enumerated its failure modes has not been designed.

Failure model (Amendment-006, ratified). Partial execution is rolled back by NOT committing - uncommitted work simply never becomes truth. Committed effects are compensated saga-style. Node crash, partition, and split-brain are handled by the IDR mechanisms (Raft leader election, joint-consensus membership, single-leader commit).

Cancellation (Amendment-005, ratified). Cancellation is cooperative and idempotent and produces NO partial truth. Priority and preemption use checkpoint -> replay: a preempted unit checkpoints, yields, and later replays from its recorded decision trace.

Recovery. Describe how a node/shard returns to health: rejoin the Raft group, catch up from leader via WAL/snapshot (IDR-002/005), resume as follower, and only re-assume leadership through election (IDR-004). No recovery path may fabricate truth that was never committed.

Replay (ORCH-003). Execution is replayable from the recorded decision trace, NOT by recomputation. The Raft log = WAL = decision trace is the single source for replay. A design must state exactly which trace entries reproduce its behaviour and confirm that replaying them yields identical committed outcomes.

- Failure checklist: Is every partial-failure path resolved by "do not commit"?

- Failure checklist: Is every committed side effect paired with a compensating saga step?

- Failure checklist: Is replay defined against the decision trace, never recomputation?

- Failure checklist: Are split-brain and partition delegated to IDR mechanisms, not re-solved?

# Part 9 - Design Standard Deep Dive: Availability, Scalability, Performance

These sections translate architectural guarantees into operational promises.

Availability. State what remains available under each failure. Truth is CP: during a partition the minority side cannot commit (correct by design). Observability is AP and stays available. A design must not promise write availability that violates the CP nature of the shard leader.

Scalability. Growth is by sharding on tenant/workspace (SHARD-001), partition key immutable. State how adding shards adds capacity, and confirm no design step requires cross-shard atomic commits (which v1 does not provide - use sagas).

Performance. Declare latency and throughput targets per path and tie them to the consistency tier chosen in Part 7. Leader-linearizable reads cost more than follower reads; the design must justify the mix.

| Dimension | Lever | Constraint |
| --- | --- | --- |
| Availability | CP for truth, AP for observability | Minority side cannot commit |
| Scalability | More shards (tenant/workspace) | Partition key immutable (SHARD-001) |
| Performance | Consistency-tier selection | Leader is the linearizable bottleneck |
| Cross-shard work | Sagas | No atomic cross-shard commit in v1 |

# Part 10 - Design Standard Deep Dive: Security, Observability, Metrics, Auditability

A component is only trustworthy if it is isolated, observable, measured, and provable after the fact.

Security. Declare trust boundaries and enforce tenant isolation at the shard boundary (SHARD-001). Every entity has identity and every observation has provenance (ontology principles O-002, O-003, definitional). Authorization checks that must reflect current truth use linearizable reads through the leader.

Observability. Emit structured logs, traces, and presence on the AP observability plane. Observability data is never treated as truth and never gates a commit.

Metrics. Define the numbers that prove SLOs: commit latency at the leader, replication lag leader->follower, saga completion/compensation counts, replay-equivalence checks, per-shard load. Metrics live on the AP plane.

Auditability. Because the Raft log = WAL = decision trace, every committed outcome is provable after the fact by replay of the trace (ORCH-003). Truth emerges from validated evidence (O-004, definitional). Auditability sections must point to the exact trace that establishes each claim.

| Concern | Plane | Primary Evidence |
| --- | --- | --- |
| Truth / audit | CP | Raft log = WAL = decision trace |
| Metrics | AP | Leader commit latency, replication lag |
| Logs / traces | AP | Structured events, spans |
| Isolation | CP boundary | Immutable per-tenant/workspace shard key |

# Part 11 - Design Standard Deep Dive: Trade-offs, Risks, and Open Questions

Mature engineering names what it sacrificed and what it does not yet know.

Trade-offs. Every meaningful design chooses; state the axis and the sacrifice explicitly. Common ARVES trade-offs:

- CP over A for truth: correctness of commit over write availability during partition.

- Weaker read tier over freshness: throughput over strict currency where correct.

- Sagas over atomicity: operability across shards over cross-shard atomic commit (not available in v1).

- Replay-from-trace over recompute: determinism and audit over recomputation flexibility (ORCH-003).

Risks. Enumerate risks beyond enumerated failure modes: hot-shard load on a leader, saga compensation gaps, decision-trace growth, proposed invariants that may change under CCP. Any reliance on a PROPOSED invariant is itself a risk and must be flagged.

Open Questions. List undecided points and route each to its owner and change instrument (CCP, Amendment, IDR, or next major version). Open questions are not defects; hiding them is.

# Part 12 - The Critical Self-Review Standard: Destroy Your Own Design

Before anyone else reviews a design, its author must sincerely attempt to break it.

Critical Self-Review is an adversarial pass the author performs on their own design. The goal is to find the failure before production does. A design submitted to ARR without a completed self-review is returned unread.

The Failure-Hunt Checklist (the author must produce a written answer to each):

- Truth ownership: Does anything other than the Kernel commit truth? (ORCH-001) If yes, the design is broken.

- Plan/state separation: Does the Control Plane hold persistent state? (ORCH-002) If yes, broken.

- Single owner: Does any state have zero or two owners? (OWN-001) If yes, broken.

- Layering: Is there any lateral or upward dependency? (LAYER-001) If yes, broken.

- Sharding: Is the partition key ever mutated, or is a cross-shard atomic commit assumed? (SHARD-001 / IDR-001) If yes, broken.

- Idempotency: Is every engine/capability invocation idempotent and content-addressable? (ORCH-004) If no, broken.

- Replay: Can behaviour be replayed from the trace without recomputation? (ORCH-003) If no, broken.

- Partial failure: Is there any path that leaves partial truth on failure? (Amendment-006) If yes, broken.

- Cancellation: Does cancellation leave partial truth or is it non-idempotent? (Amendment-005) If yes, broken.

- Proposed reliance: Does correctness depend on a PROPOSED invariant treated as normative? If yes, broken until CCP.

- Consistency: Is any read stronger than necessary, or too weak to be correct? Re-justify.

- Split-brain: Is split-brain re-solved in the design instead of delegated to IDR mechanisms? If yes, broken.

Self-review verdict: the author records PASS only when every item is answered and no "broken" condition remains. Otherwise the author revises and repeats.

# Part 13 - The Gap Analysis Process

Gap Analysis compares a design against the frozen specification and finds what is missing, contradictory, or unproven.

Gap Analysis runs after self-review and before ARR. It is a structured diff between (a) what the specification/contracts require and (b) what the design provides. Each finding is classified and routed.

| Gap Type | Definition | Routing |
| --- | --- | --- |
| Missing behaviour | Spec requires it; design omits it | Author revises design |
| Contradiction | Design conflicts with a registered invariant | Author revises; may need CCP if spec unclear |
| Unproven claim | Design asserts guarantee with no conformance scenario | Add conformance scenario (CCP-GATE) |
| Proposed dependency | Design leans on a PROPOSED invariant | Flag proposed (pending CCP); reduce reliance |
| Spec ambiguity | Specification itself is unclear | Raise CCP; do not resolve in code |
| Out-of-scope | Design does more than its responsibility | Split or remove |

Gap Analysis output is a table of findings, each with type, severity, owner, and instrument. A design with any open Contradiction gap cannot enter ARR. Unproven claims must be paired with a planned conformance scenario before ARR, honoring CCP-GATE (no behaviour ratified without a conformance scenario).

# Part 14 - The Architecture Review and ARR Process

The Architecture Readiness Review is the formal gate between design and implementation.

The ARR evaluates a design across fixed dimensions and issues one verdict per dimension, then an overall verdict. Reviewers are peers plus at least one owner of an adjacent layer.

ARR dimensions (each dimension maps to design sections in Parts 5-11):

| Dimension | What Is Checked | Backing Invariants |
| --- | --- | --- |
| Layering & Ownership | Correct layer placement, single owners | LAYER-001, OWN-001 |
| Truth & Control | Kernel-only commit, plan/state split | ORCH-001, ORCH-002 |
| Distribution | Grounded in IDR-001..005; sharding correct | SHARD-001, IDR-001..005 |
| Replay & Idempotency | Trace-based replay, idempotent invocations | ORCH-003, ORCH-004 |
| Failure & Recovery | No partial truth; saga compensation; recovery | Amendment-005/006 |
| Consistency & Availability | Correct tiers; CP-truth honored | IDR tiers |
| Security & Isolation | Tenant isolation; provenance; identity | SHARD-001, O-002/003 |
| Observability & Audit | AP observability; trace-based audit | ORCH-003 |
| Conformance | Every behaviour has a scenario | CCP-GATE |

ARR verdicts, per the frozen ARR record semantics:

- READY - the dimension is satisfied; implementation may proceed for it.

- GAP - a defect exists that the author must fix; design returns to author. Re-review required.

- IDR - the dimension is satisfied only by a reference implementation decision; record or reference the relevant IDR (IDR-001..005) and mark the choice non-normative.

Overall verdict: a design is ARR-READY only when every dimension is READY or explicitly IDR-backed. Any single GAP blocks implementation. IDR verdicts are permitted and expected for distributed dimensions - they simply record that the reference runtime, not the standard, made the choice.

# Part 15 - Code-Writing Rules (Post-ARR Implementation Era)

Code is written only to prove the design; it never redefines it.

Implementation Era is in progress, but code follows the frozen specification. Rules:

- Implementation proves the spec and never changes it. A code change that would alter behaviour requires a CCP first.

- Only the Kernel commits truth (ORCH-001). No engine, query path, or control-plane code writes committed state.

- The Control Plane produces plans, never persistent state (ORCH-002). Plan objects are transient.

- Every engine/capability invocation is idempotent and content-addressable (ORCH-004). Re-invocation with the same content yields the same result and commits at most once.

- Replay reads the decision trace; code must never rebuild truth by recomputation (ORCH-003).

- Reads select the weakest correct consistency tier; linearizable reads through the leader are justified in code comments referencing the design.

- Cross-shard operations are implemented as sagas with explicit compensation; no code assumes cross-shard atomic commit (IDR-001).

- Any code depending on a PROPOSED invariant (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005) is marked with a "proposed (pending CCP)" comment and isolated behind a clear boundary.

- Cancellation paths are cooperative and idempotent and leave no partial truth (Amendment-005).

- Partial failures resolve by not committing; committed effects get compensating saga steps (Amendment-006).

Milestone alignment. Code lands against the frozen milestones exactly: I1 Distributed Runtime, I2 Cluster Kernel, I3 Distributed Query, I4 Capability Scheduling, I5 Multi-Agent Runtime, I6 Reference Products. No other milestone names are used in commits, branches, or tickets.

# Part 16 - File and Repository Organization

Structure follows the layer map so that a file location reveals its responsibility and its allowed dependencies.

Organizing principles:

- Top-level modules mirror layers: information-platform, kernel, persistence, lcw, query, engine, capability, execution, plus a cross-cutting control-plane and event-fabric.

- Downward-only dependency rule (LAYER-001) is enforced by module boundaries; a module may import only from layers below it or from cross-cutting modules.

- Contracts live separately from behaviour, which lives separately from implementation, mirroring Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation.

- Conformance scenarios live beside the behaviour they prove; CCP-GATE means no ratified behaviour without its scenario file.

- Per-shard concerns (Raft group, WAL, snapshots) are localized in the kernel/persistence modules and never leak upward.

- Proposed-invariant-dependent code is quarantined in clearly named boundaries so it can be revised when a CCP resolves the proposal.

| Directory (illustrative) | Layer / Concern | May Depend On |
| --- | --- | --- |
| kernel/ | Truth + commit (Raft, WAL) | persistence, information-platform |
| lcw/ | Working memory / live state | kernel (read), persistence |
| query/ | Read-only projections | kernel (read), lcw (read) |
| engine/ | Pure inference (proposed effects) | query (read), contracts |
| capability/ | Registry + bindings | engine, contracts |
| control-plane/ | Plans / Engine Graph | cross-cutting only |
| conformance/ | Reference scenarios | behaviour under test |

# Part 17 - Commit Rules and Change Instruments

Commits are small, provable, and traceable to a design and a milestone.

Commit discipline:

- One logical change per commit, each traceable to an ARR-READY design and a frozen milestone (I1..I6).

- A commit that changes behaviour must reference the CCP or Amendment that authorized it. Frozen specification content is never edited by a code commit.

- Commits touching distributed behaviour reference the relevant IDR (IDR-001..005) and mark the choice non-normative.

- Commits adding behaviour include the conformance scenario in the same change set (CCP-GATE).

- Commit messages state the layer, the invariant(s) upheld, and the milestone.

Change instruments and when to use them:

| Instrument | Use When | Effect |
| --- | --- | --- |
| CCP | A behaviour/standard element must change | Enters change flow; may ratify a new behaviour |
| Amendment | A frozen artifact must be adjusted via CCP | Ratified change (e.g., Amendment-005/006) |
| IDR | Reference runtime makes a non-normative choice | Records implementation decision (IDR-001..005) |
| Next major version | Change too large for CCP/Amendment | Deferred to a future standard version |

# Part 18 - Conformance and Certification Alignment

Every behaviour an engineer builds must be provable by conformance, and the runtime must be certifiable.

Conformance model. Twelve axes drive reference scenarios which drive node probes which yield verdicts. Verdicts are invariant/property-based (PASS/PARTIAL/FAIL), NOT golden-output comparisons.

The twelve conformance axes:

| Axis | Axis | Axis |
| --- | --- | --- |
| Information-intensive | Event-driven | Human-collaboration |
| Multi-step planning | Long-running | Physical-world |
| Safety-critical | High-volume streaming | Multi-agent |
| Policy-heavy | Autonomous | Recovery/replay |

Certification levels the engineering effort targets:

| Level | Scope |
| --- | --- |
| L1 Core Runtime | Core commit/truth runtime conforms |
| L2 Cognitive Control | Control-plane planning conforms |
| L3 Distributed | Sharded, replicated distributed runtime conforms |
| L4 Multi-Agent | Multi-agent runtime conforms |
| Certified Product | A product built on ARVES without modifying the standard |

Certification goals guiding the Playbook: a production distributed runtime, a complete conformance suite, Independent Runtime A and Independent Runtime B both passing certification, third-party certification, enterprise runtime, SDKs, marketplace, cloud, and real products built on ARVES without modifying the standard. CCP-GATE remains absolute: no behaviour is ratified without a conformance scenario.

# Part 19 - Consolidated Review Checklists

Use these as the last pass before submitting a design, opening a review, or landing code.

Design submission checklist:

- All 22 design sections present and concrete; no code.

- Component placed on the layer map; "Cannot" list matches the Responsibility Matrix.

- Every state has exactly one owner (OWN-001).

- All committed outputs route through the Kernel (ORCH-001).

- Distributed behaviour grounded only in IDR-001..005; SHARD-001 partition key immutable.

- Replay defined against the decision trace, not recomputation (ORCH-003).

- Invocations idempotent and content-addressable (ORCH-004).

- Failure resolves by not committing; cross-shard via sagas (Amendment-006, IDR-001).

- Every PROPOSED invariant reliance flagged "proposed (pending CCP)".

- Milestones referenced use exactly I1..I6.

ARR reviewer checklist:

- Every dimension verdict recorded as READY, GAP, or IDR.

- No dimension left GAP in the final verdict.

- IDR verdicts reference a specific IDR and are marked non-normative.

- Every behaviour has a conformance scenario (CCP-GATE).

- No PROPOSED invariant is presented as normative.

Code-landing checklist:

- Change traceable to an ARR-READY design and a milestone I1..I6.

- Behaviour changes reference a CCP/Amendment; frozen spec untouched.

- Kernel-only commit preserved; Control Plane holds no persistent state.

- Consistency tier is the weakest correct one and is justified.

- Conformance scenario included in the same change set.

# Part 20 - Anti-Patterns This Playbook Exists to Prevent

Name the failure and it loses its power; each anti-pattern maps to the invariant it violates.

| Anti-Pattern | Why It Is Wrong | Violated Rule |
| --- | --- | --- |
| Kernel starts orchestrating | Kernel must only own truth | ORCH-001 / Two-plane rule |
| Control Plane persists state | Plans are transient, not truth | ORCH-002 |
| Two owners for one state | Ambiguous ownership breaks recovery | OWN-001 |
| Lateral/upward dependency | Breaks the layer map | LAYER-001 |
| Mutable partition key | Breaks sharding and isolation | SHARD-001 |
| Cross-shard atomic commit in v1 | Not provided; must use sagas | IDR-001 |
| Replay by recomputation | Non-deterministic, unauditable | ORCH-003 |
| Non-idempotent invocation | Double-commit on retry | ORCH-004 |
| Partial truth on failure | Corrupts truth | Amendment-006 |
| Proposed invariant as normative | Standard not yet ratified it | CCP discipline |
| Inventing new milestones | Baseline names are frozen | Milestone freeze I1..I6 |
| Editing the frozen spec in code | Implementation proves, not amends | Dependency chain |

# Part 21 - Worked Design Skeleton (Illustrative, No Code)

A concrete, code-free walkthrough showing how a shard-scoped commit path would be designed against this standard.

Component: a Kernel commit path for a single tenant/workspace shard. This is illustrative and grounded entirely in IDR-001.

- Responsibilities: commit validated outcomes to truth for one shard. Cannot orchestrate, plan, or execute.

- Inputs: proposed effects from Engines (pre-commit, not truth); plans from the Control Plane (transient).

- Outputs: committed outcomes appended to the Raft log = WAL = decision trace; only this path commits.

- Dependencies: persistence (WAL/snapshot), information platform; downward only.

- State Model: uncommitted proposal -> validated -> committed outcome. Owner of committed truth: Kernel (OWN-001).

- Distributed Behaviour: per-shard Raft group; commit through the shard leader only (IDR-001, IDR-004).

- Concurrency: engines run concurrently anywhere; single-leader commit serializes truth; replication carries outcomes not invocations (ORCH-003).

- Failure Modes: leader loss -> election (IDR-004); minority partition cannot commit (CP).

- Recovery: rejoin group, catch up via WAL/snapshot (IDR-002/005).

- Replay: replay committed outcomes from the log; never recompute (ORCH-003).

- Consistency: linearizable through leader for commit checks; follower reads bounded-staleness.

- Cross-shard: not atomic; coordinate via sagas with compensation (IDR-001, Amendment-006).

- Conformance: recovery/replay and high-volume-streaming axes exercise this path (CCP-GATE).

This skeleton would enter self-review (Part 12), gap analysis (Part 13), and ARR (Part 14). Its distributed dimensions would likely earn IDR verdicts referencing IDR-001..005 - which is expected and acceptable.

# Part 22 - How the Playbook Is Maintained

The Playbook is a frozen process standard; it changes only through the same instruments it governs.

- The Playbook itself follows the dependency chain: it cannot loosen a registered invariant without a CCP.

- When a PROPOSED invariant is ratified via CCP, this Playbook is updated to move it from informative to normative.

- New IDRs are referenced, not embedded; the Playbook cites IDR-001..005 and any future IDRs by number.

- Milestone names remain exactly I1..I6 until a new Baseline is ratified.

- Maturity of this document follows Draft -> Candidate -> Ratified -> Frozen -> Deprecated -> Superseded.

*Final Definition  Engineering Playbook = the disciplined path from frozen ARVES specification to proven implementation - design without code, destroy your own design, close every gap, pass ARR, and let implementation prove the standard it may never change.*

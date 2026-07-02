> **Rendered from `ARVES_OS_Volume_2_UltraCode_Workflow_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engineering Operating Manual - Volume 2: The UltraCode Workflow

STATUS: OPERATING MANUAL (IMPLEMENTATION-ERA, NORMATIVE PROCESS). Specification Era FROZEN as of 2026-07-01. Implementation Era IN PROGRESS. This volume governs HOW engineering work is executed; it never alters the frozen specification.

# Part 1 - Purpose, Scope, and the Prime Directive

Volume 2 defines the UltraCode Workflow: the mandatory 15-phase process every engineering task follows from request to certified verdict. It is the operational counterpart to the frozen specification corpus. Where Volume 1 describes WHAT ARVES is, this volume describes HOW work is admitted, designed, built, proven, and certified without weakening the standard.

ARVES is Universal Cognitive Infrastructure. UCS (Universal Cognitive Standard) is the standard; UCI is its reference implementation. The Specification Era is FROZEN. The Implementation Era is in progress. The purpose of implementation is to PROVE the specification, never to modify it.

The Prime Directive of the UltraCode Workflow:

- The dependency chain is one-directional and never reversed: Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation.

- Implementation proves the spec and never changes it. A workflow that would require changing the frozen spec MUST stop and raise a change instrument (CCP, Amendment, IDR, or next major version).

- The Kernel owns TRUTH. The Control Plane decides but owns no truth. The Kernel never becomes the Control Plane.

- No phase may be skipped, merged, or reordered. Fan-out parallelizes work WITHIN the discipline of the phases; it never removes a phase.

Scope of this volume: the 15 phases in full (purpose, inputs, outputs/artifact, exit criteria), the before-writing-code questions, how a milestone task is issued and driven, and how to fan work out across phases without shortcuts.

# Part 2 - Architectural Ground Truth the Workflow Enforces

Every phase is anchored to the frozen architecture. The workflow is the mechanism by which these facts are checked on each task. The two planes and the layer stack are the non-negotiable frame.

Two planes: the Control Plane DECIDES; the Data Plane CARRIES. The Kernel never becomes the Control Plane.

Layers (downward-only dependencies, per LAYER-001): Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine -> Capability -> Execution, plus the cross-cutting Control Plane.

| Layer | Owns | Reads/Writes | Cannot |
| --- | --- | --- | --- |
| Kernel | TRUTH (commits) | Writes committed outcomes | Orchestrate / plan / execute |
| LCW | Working Memory / live state | Reads+writes live state (not truth) | Own truth |
| Query | Nothing (read-only projections) | Reads only; writes nothing | Write any state |
| Engine | Nothing persistent | Produces inference; proposes effects | Commit; only Kernel commits |
| Capability Fabric | Registry + bindings | Reads registry; binds | Own truth or persistent state |
| Control Plane | Plan / Engine Graph | Produces plans | Own truth or persistent state |

The workflow treats these rows as invariants-in-practice. Any design that lets a non-owner write an owned state, or lets the Control Plane hold persistent state, fails at Contract Mapping or Critical Self-Review.

# Part 3 - The Normative Vocabulary: Invariants, Principles, Decisions

The workflow distinguishes three tiers of statement. Confusing them is itself a defect. Reviewers enforce the tier labels.

REGISTERED invariants (normative; may be cited as binding):

- ORCH-001 - Control Plane owns no truth; only the Kernel owns truth.

- ORCH-002 - Control Plane produces plans, never persistent state.

- ORCH-003 - Execution is replayable from the recorded decision trace, NOT by recomputation.

- ORCH-004 - Every engine/capability invocation is idempotent and content-addressable.

- OWN-001 - Every state has exactly one owner.

- LAYER-001 - Downward-only dependencies; no lateral coupling; cross-cutting via Control Plane / Event Fabric.

- SHARD-001 - Partition by tenant/workspace; the partition key is immutable.

Ontology DESIGN PRINCIPLES O-001..O-007 (definitional, NOT runtime-provable invariants): everything is a cognitive entity; every entity has identity; every observation has provenance; truth emerges from validated evidence; derivation is not inheritance; every type is versioned+registered; ontology defines meaning not storage. These guide modeling but are never cited as runtime invariants in Conformance.

PROPOSED invariants (INFORMATIVE only - must always be marked "proposed, pending CCP"; never presented as registered/normative): G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..CAP-009, ENG-001..ENG-005. A task may rely on a proposed invariant only as design intent, and must record the pending-CCP dependency in its Gap Analysis.

IDR reference implementation decisions IDR-001..IDR-005 (non-normative but binding on the reference runtime UCI): all distributed design is grounded here (see Part 6 and Part 17).

# Part 4 - The UltraCode Workflow at a Glance (15 Phases)

Every engineering task - from a one-line fix to a milestone slice - passes through all fifteen phases in order. The phases divide into four movements: Admit (1), Understand (2-8), Build (9-12), Prove (13-15).

| # | Phase | Movement | Primary Artifact |
| --- | --- | --- | --- |
| 1 | ARR - Acceptance / Readiness Review | Admit | Admission Record |
| 2 | Affected UCI Node Analysis | Understand | Node Impact Map |
| 3 | Specification Mapping | Understand | Spec Trace |
| 4 | Contract Mapping | Understand | Contract Delta |
| 5 | Invariant Mapping | Understand | Invariant Obligation Set |
| 6 | Ownership Analysis | Understand | Ownership Ledger |
| 7 | IDR Mapping | Understand | IDR Binding Sheet |
| 8 | Gap Analysis | Understand | Gap Register |
| 9 | Engineering Design | Build | Design Dossier |
| 10 | Critical Self-Review | Build | Self-Review Log |
| 11 | Implementation | Build | Code + Decision Trace |
| 12 | Testing | Build | Test Suite + Results |
| 13 | Conformance | Prove | Conformance Verdict |
| 14 | Independent Architecture Review | Prove | IAR Sign-off |
| 15 | Certification Verdict | Prove | Certification Record |

The Understand movement (phases 2-8) is where most defects are prevented. The workflow deliberately front-loads analysis: no code is written until phase 11, and phase 11 cannot start until phase 10 passes.

# Part 5 - Phase 1: ARR (Acceptance / Readiness Review)

Purpose: admit a task into the workflow. ARR is the entry gate that confirms the task belongs in the Implementation Era, does not require changing the frozen spec, and has a clear certifiable outcome.

Inputs:

- Task request (from a milestone slice I1..I6, a defect, or an approved change instrument).

- The frozen specification corpus and the registered invariant registry.

- Current milestone context and the Master Backlog entry, if any.

Outputs / Artifact: the Admission Record - task id, milestone linkage, one-sentence outcome, and an explicit statement that no frozen-spec change is required (or a pointer to the CCP/Amendment/IDR that authorizes it).

Exit criteria:

- Task maps to exactly one milestone (I1..I6) or an approved change instrument.

- The outcome is certifiable (there exists at least one conformance axis it will exercise).

- No spec change is implied; if one is, the task is rejected back to the change-instrument process (CCP-GATE applies).

- An owner and a target certification level (L1..L4 or Certified Product) are recorded.

# Part 6 - The Before-Writing-Code Questions

Before any implementation begins (i.e., before phase 11 is entered), the engineer must be able to answer every question below in writing. Unanswered questions block the task at Critical Self-Review. These questions are the compression of phases 2-10 into a checklist.

- Which UCI nodes are affected, and does the change respect downward-only dependencies (LAYER-001)?

- Which frozen specification clauses does this task realize? (Spec Trace exists.)

- Which contracts change, and are the changes additive/compatible or breaking?

- Which registered invariants (ORCH-001..004, OWN-001, LAYER-001, SHARD-001) does this task have an obligation to preserve?

- For every state touched, who is the single owner (OWN-001)? Does any non-owner attempt to write it?

- Does the Control Plane remain truth-free and state-free (ORCH-001, ORCH-002)?

- Is every engine/capability invocation idempotent and content-addressable (ORCH-004)?

- Is the outcome replayable from the recorded decision trace and NOT from recomputation (ORCH-003)?

- Is the partition key by tenant/workspace, and is it immutable (SHARD-001)?

- Which IDR (IDR-001..005) governs the distributed behaviour, and is the design consistent with per-shard Raft, replicated committed outcomes, and saga-based cross-shard flow?

- Which proposed invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-*, ENG-*) does the design lean on, and are they flagged as proposed (pending CCP)?

- What is the failure behaviour: partial execution rolled back by NOT committing; committed effects compensated saga-style (Amendment-006)? Is cancellation cooperative + idempotent with no partial truth (Amendment-005)?

- Which conformance axes and reference scenarios will prove this task, and what is the target certification level?

If any answer is "unknown", the task returns to the relevant Understand-movement phase. Guessing is a workflow violation.

# Part 7 - Phase 2: Affected UCI Node Analysis

Purpose: identify every node of the UCI reference implementation the task touches, and confirm the change stays within the layer stack and its downward-only dependency rule.

Inputs: Admission Record; the layer stack (Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine -> Capability -> Execution + Control Plane); the Layer Responsibility Matrix.

Outputs / Artifact: the Node Impact Map - the set of affected nodes, the layer of each, and the dependency edges the task uses.

Exit criteria:

- Every affected node is placed in exactly one layer.

- All dependency edges point downward only; no lateral coupling is introduced (LAYER-001). Cross-cutting concerns route via Control Plane or Event Fabric.

- The Kernel is not asked to orchestrate/plan/execute, and the Control Plane is not asked to carry data.

# Part 8 - Phase 3: Specification Mapping

Purpose: trace the task to the frozen specification it realizes, proving the task proves the spec rather than inventing behaviour.

Inputs: Node Impact Map; frozen specification corpus; Reference Lifecycle stage of each touched concept.

Outputs / Artifact: the Spec Trace - a table mapping each unit of work to the specific frozen clause it implements.

Exit criteria:

- Every unit of work maps to a frozen, Ratified-or-Frozen spec clause.

- No unit of work exists that has no spec basis (such work is out of scope and returns to the change-instrument process).

- No mapping implies editing the spec; direction is Specification -> Implementation only.

| Work Unit | Frozen Clause Realized | Lifecycle Maturity |
| --- | --- | --- |
| Commit path | Kernel owns truth; commit semantics | Frozen |
| Plan production | Control Plane produces plans (ORCH-002) | Ratified |
| Read projection | Query is read-only | Frozen |

# Part 9 - Phase 4: Contract Mapping

Purpose: determine which contracts (service, event, data, API, agent) the task changes and classify each change as additive/compatible or breaking. Contracts sit above Behaviour in the dependency chain (Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation).

Inputs: Spec Trace; the Service/Event/Data/API/Agent catalogs; existing contract versions.

Outputs / Artifact: the Contract Delta - per contract, the change type, version bump, and compatibility statement.

Exit criteria:

- Every contract touched has a declared version change and compatibility class.

- Breaking changes are justified against a change instrument; silent breaks are forbidden.

- Contracts remain the interface the Behaviour and Conformance layers depend on; Implementation conforms to them, not the reverse.

# Part 10 - Phase 5: Invariant Mapping

Purpose: enumerate the registered invariants the task is obligated to preserve, and record how each will be checked. Proposed invariants are listed separately and always flagged.

Inputs: Node Impact Map; Contract Delta; the invariant registry.

Outputs / Artifact: the Invariant Obligation Set - each obligation with an enforcement point and a conformance hook.

Exit criteria:

- Every registered invariant relevant to the touched nodes appears with a concrete enforcement point.

- Proposed invariants used by the design are listed and marked "proposed (pending CCP)".

- Each obligation names the conformance probe/property that will verify it (invariant/property based, not golden-output).

| Invariant | Obligation on This Task | Enforcement / Probe |
| --- | --- | --- |
| ORCH-001 | Control Plane holds no truth | Static: CP has no commit path; probe asserts truth only via Kernel |
| ORCH-003 | Replay from decision trace | Property: replay(trace) == observed outcome; no recompute |
| ORCH-004 | Idempotent + content-addressable calls | Property: f(x) keyed by content; repeat yields same id |
| OWN-001 | Single owner per state | Static ownership check + write-authority probe |
| SHARD-001 | Immutable tenant/workspace key | Property: partition key rejects mutation |
| G-001 (proposed, pending CCP) | Design intent only | Recorded as pending-CCP dependency |

# Part 11 - Phase 6: Ownership Analysis

Purpose: prove OWN-001 for every state the task reads or writes - exactly one owner per state - and that write authority is respected across the layer matrix.

Inputs: Node Impact Map; Layer Responsibility Matrix; Invariant Obligation Set.

Outputs / Artifact: the Ownership Ledger - per state: owner, readers, writers, and the layer that commits it.

Exit criteria:

- Every state has exactly one declared owner (OWN-001).

- Truth is owned only by the Kernel; live/working state only by LCW; Query writes nothing; Engine owns nothing persistent and only proposes effects; Capability Fabric owns registry+bindings only; Control Plane owns Plan/Engine Graph and no persistent state.

- No non-owner writes an owned state; proposed effects from engines are committed only by the Kernel.

# Part 12 - Phase 7: IDR Mapping

Purpose: bind the distributed aspects of the task to the reference implementation decisions IDR-001..IDR-005, which are non-normative but authoritative for the UCI reference runtime.

Inputs: Node Impact Map; Ownership Ledger; the IDR set; read-consistency tiers.

Outputs / Artifact: the IDR Binding Sheet - which IDR governs each distributed concern and how the design conforms.

Exit criteria (all distributed content grounded in IDR-001..005):

- Kernel-as-Control-Plane distribution uses per-shard Raft: one Raft group per tenant/workspace shard (IDR-001).

- Committed OUTCOMES are replicated, NOT engine invocations; engines run anywhere, only the commit goes through the shard leader (IDR-001).

- The Raft log = WAL = decision trace (IDR-001, IDR-005); append-only WAL + snapshots (IDR-005).

- No cross-shard atomic commit in v1; cross-shard flows use sagas (IDR-001).

- Replication is leader -> followers with snapshots + WAL (IDR-002); membership via Raft joint consensus (IDR-003); per-shard leader election (IDR-004).

- Truth path = CP (the CAP consistency class, served via the Kernel / shard leader - NOT the ARVES Control Plane, which owns no truth per ORCH-001); observability (metrics/logs/tracing/presence) = AP path. Read tier chosen from: linearizable (through leader), bounded-staleness (follower), eventual (replica).

# Part 13 - Phase 8: Gap Analysis

Purpose: surface everything the task needs that does not yet exist or is not yet ratified - missing contracts, pending-CCP invariants, absent conformance scenarios, or IDR ambiguities - before design begins.

Inputs: all Understand-movement artifacts (phases 2-7).

Outputs / Artifact: the Gap Register - each gap with type, blocking/non-blocking status, and resolution path (change instrument or in-task).

Exit criteria:

- Every dependency on a proposed invariant is recorded as a pending-CCP gap.

- Every behaviour to be ratified has (or schedules) a conformance scenario - CCP-GATE: no behaviour ratified without a conformance scenario.

- Blocking gaps are resolved or routed to a change instrument (CCP / Amendment / IDR / next major version) before phase 9.

# Part 14 - Phase 9: Engineering Design

Purpose: produce the design that satisfies the spec, contracts, invariants, ownership, and IDR bindings established upstream. Design is the first place solutions are chosen; it may not reopen frozen decisions.

Inputs: all Understand-movement artifacts plus the resolved Gap Register.

Outputs / Artifact: the Design Dossier - component design, data/effect flow, plan/engine-graph shape, failure and cancellation handling, and the mapping from design elements to invariant obligations.

Exit criteria:

- Design realizes the Spec Trace and honors the Contract Delta.

- Design shows the Control Plane producing plans only, and the Kernel committing outcomes only (ORCH-001, ORCH-002).

- Design records decision traces sufficient for replay (ORCH-003) and makes every invocation idempotent + content-addressable (ORCH-004).

- Failure model is explicit: partial execution rolled back by NOT committing; committed effects compensated saga-style (Amendment-006). Cancellation is cooperative + idempotent, no partial truth; priority/preemption uses checkpoint -> replay (Amendment-005).

- Every design element traces to an invariant obligation or a spec clause; nothing is unmotivated.

# Part 15 - Phase 10: Critical Self-Review

Purpose: the engineer adversarially reviews their own design against the before-writing-code questions and the invariant obligations, BEFORE any code is written. This is the last gate of the Build movement entry.

Inputs: Design Dossier; the before-writing-code checklist (Part 6); Invariant Obligation Set.

Outputs / Artifact: the Self-Review Log - each checklist item marked pass/fail with evidence, and every fail resolved or escalated.

Exit criteria:

- All before-writing-code questions are answered in writing with no "unknown".

- No registered invariant obligation is left without an enforcement point.

- No frozen-spec change is implied; if discovered now, the task halts and returns to ARR / change instrument.

- Only after a clean Self-Review Log may Implementation (phase 11) begin.

# Part 16 - Phase 11: Implementation

Purpose: write the code that realizes the approved design. Implementation proves the spec; it is the LAST link in the dependency chain and never feeds back into it.

Inputs: Design Dossier; clean Self-Review Log; Contract Delta.

Outputs / Artifact: source code plus a recorded decision trace (Raft log = WAL = decision trace, per IDR-001/005).

Exit criteria:

- Code conforms to the frozen contracts (Implementation conforms to Contracts, never the reverse).

- Commits go only through the Kernel / shard leader; engines produce inference and propose effects only.

- Every invocation is idempotent and content-addressable; outcomes are replayable from the trace, not recomputed.

- No spec, contract, or invariant was silently altered to make code pass; such a change is a defect.

# Part 17 - Phase 12: Testing

Purpose: verify the implementation behaves as designed under normal, failure, and concurrency conditions, including the distributed mechanisms from the IDR bindings.

Inputs: code + decision trace; Invariant Obligation Set; failure/cancellation model.

Outputs / Artifact: the Test Suite + Results - unit, property, failure-injection, and replay tests.

Exit criteria:

- Property tests assert each registered-invariant obligation (idempotence, single-owner writes, immutable partition key, replay equals observed).

- Failure-injection tests confirm partial execution is rolled back by NOT committing and committed effects are compensated saga-style.

- Distributed tests exercise leader election, follower replication, snapshots + WAL, joint-consensus membership, and cross-shard sagas (IDR-001..005).

- Cancellation tests confirm cooperative + idempotent cancellation with no partial truth, and checkpoint -> replay for preemption.

# Part 18 - Phase 13: Conformance

Purpose: prove the task against the conformance framework - reference scenarios and node probes producing an invariant/property-based verdict, NOT a golden-output comparison.

Inputs: Test Suite + Results; the conformance axes; reference scenarios; node probes.

Outputs / Artifact: the Conformance Verdict - PASS / PARTIAL / FAIL per exercised axis, with probe evidence.

The 12 conformance axes: information-intensive, event-driven, human-collaboration, multi-step planning, long-running, physical-world, safety-critical, high-volume streaming, multi-agent, policy-heavy, autonomous, recovery/replay.

Exit criteria:

- Each behaviour ratified by the task is backed by a conformance scenario (CCP-GATE).

- Verdict is invariant/property based; no golden-output pass is accepted.

- PARTIAL/FAIL results are triaged: either the code is fixed (return to phase 11) or a gap is escalated to a change instrument.

| Axis (sample) | Reference Scenario Probes | Verdict Basis |
| --- | --- | --- |
| recovery/replay | Replay from WAL/decision trace | ORCH-003 property holds |
| multi-agent | Concurrent agents, saga cross-shard | OWN-001 + saga compensation |
| long-running | Checkpoint -> preempt -> replay | Amendment-005 cancellation |

# Part 19 - Phase 14: Independent Architecture Review (IAR)

Purpose: an engineer NOT on the task independently reviews the architecture-level correctness: layer discipline, ownership, plane separation, and IDR grounding. This mirrors the corpus ARR discipline at task scope.

Inputs: Design Dossier; Conformance Verdict; all Understand-movement artifacts.

Outputs / Artifact: the IAR Sign-off - independent findings and a go/no-go on architecture.

Exit criteria:

- Independent confirmation that dependencies are downward-only (LAYER-001) and no lateral coupling was introduced.

- Independent confirmation the Control Plane owns no truth and no persistent state (ORCH-001, ORCH-002), and the Kernel did not become the Control Plane.

- Independent confirmation distributed design matches IDR-001..005 and the chosen read-consistency tier is justified.

- All findings are resolved or explicitly accepted with rationale before certification.

# Part 20 - Phase 15: Certification Verdict

Purpose: issue the final, recorded verdict that the task meets its target certification level and may be integrated. Certification is invariant/property based and tied to the reference-lifecycle stage.

Inputs: Conformance Verdict; IAR Sign-off; Invariant Obligation Set; target level from ARR.

Outputs / Artifact: the Certification Record - level achieved, evidence links, and any accepted gaps.

Certification levels: L1 Core Runtime, L2 Cognitive Control, L3 Distributed, L4 Multi-Agent, and Certified Product.

Exit criteria:

- All registered-invariant obligations verified; all pending-CCP dependencies recorded, none masquerading as normative.

- Conformance verdict is PASS for the target axes; PARTIAL is admissible only with an escalated, tracked gap.

- IAR Sign-off is go; the target certification level (L1..L4 / Certified Product) is justified by evidence.

- No frozen-spec change occurred; the task demonstrably proves the spec.

# Part 21 - How a Milestone Task Is Issued and Driven

Milestones are frozen (Baseline Part 5) and are used EXACTLY as named: I1 Distributed Runtime, I2 Cluster Kernel, I3 Distributed Query, I4 Capability Scheduling, I5 Multi-Agent Runtime, I6 Reference Products. No other milestone names exist.

Issuing a milestone task:

- A milestone is sliced into tasks; each slice becomes one Admission Record at ARR and links back to its milestone id (I1..I6).

- Each task carries a target certification level appropriate to its milestone (e.g., distributed slices target L3; multi-agent slices target L4).

- The slice states its outcome and the conformance axes it will exercise before admission.

Driving a milestone task through the 15 phases:

- Understand movement (phases 2-8) produces the Node Impact Map, Spec Trace, Contract Delta, Invariant Obligation Set, Ownership Ledger, IDR Binding Sheet, and Gap Register for the slice.

- Build movement (phases 9-12) designs, self-reviews, implements, and tests, recording the decision trace for replay.

- Prove movement (phases 13-15) certifies via conformance, independent architecture review, and the certification verdict.

| Milestone | Typical Target Level | Dominant IDR / Axis Emphasis |
| --- | --- | --- |
| I1 Distributed Runtime | L3 Distributed | IDR-001/002/005; recovery/replay |
| I2 Cluster Kernel | L3 Distributed | IDR-001/003/004; safety-critical, replay |
| I3 Distributed Query | L3 Distributed | Read tiers; high-volume streaming (read-only) |
| I4 Capability Scheduling | L2 -> L3 | ORCH-004; policy-heavy, multi-step planning |
| I5 Multi-Agent Runtime | L4 Multi-Agent | sagas, OWN-001; multi-agent, autonomous |
| I6 Reference Products | Certified Product | end-to-end axes; independent runtimes A and B |

# Part 22 - Fanning Work Out Across Phases Without Shortcuts

Parallelism is allowed and encouraged, but only within the discipline of the phases. Fan-out never removes, merges, or reorders a phase; it distributes work while preserving every gate.

Rules of disciplined fan-out:

- Phase order is a happens-before constraint: an artifact cannot be produced before its inputs exist. Phases 2-7 can proceed in parallel across independent nodes, but all must complete before Gap Analysis (phase 8) closes.

- The Understand movement must fully complete before the Build movement begins for a given slice; no slice starts Implementation on partial understanding.

- Independent slices of one milestone may run their own 15-phase pipelines concurrently; shared contracts are coordinated via the Contract Delta so no two slices break the same interface silently.

- Critical Self-Review (phase 10) and Independent Architecture Review (phase 14) are never performed by the same person for the same slice; independence is structural.

- Cross-slice truth conflicts are prevented by SHARD-001 (partition by tenant/workspace) and OWN-001 (single owner); cross-shard interaction is saga-based (IDR-001), never a shortcut atomic commit.

- A green test or a passing demo is NOT a substitute for Conformance; only an invariant/property-based verdict certifies.

Anti-shortcut guardrails (any one of these halts the pipeline):

- Skipping a phase, or producing a downstream artifact without its upstream inputs.

- Citing a proposed invariant (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-*, ENG-*) as if registered/normative.

- Editing the frozen spec, a contract, or a registered invariant to make code pass.

- Letting the Control Plane hold truth or persistent state, or the Kernel orchestrate/plan/execute.

- Inventing a milestone name, layer, invariant, or IDR beyond the frozen set.

# Part 23 - Phase-to-Artifact-to-Gate Ledger

A single reference for auditors: each phase, its artifact, and the gate condition that lets the task advance.

| Phase | Artifact | Gate to Advance |
| --- | --- | --- |
| 1 ARR | Admission Record | Maps to I1..I6 or change instrument; no spec change |
| 2 Node Analysis | Node Impact Map | Downward-only deps confirmed (LAYER-001) |
| 3 Spec Mapping | Spec Trace | Every work unit maps to a frozen clause |
| 4 Contract Mapping | Contract Delta | All contract changes classified + versioned |
| 5 Invariant Mapping | Invariant Obligation Set | Registered obligations have enforcement points |
| 6 Ownership | Ownership Ledger | Exactly one owner per state (OWN-001) |
| 7 IDR Mapping | IDR Binding Sheet | Distributed design grounded in IDR-001..005 |
| 8 Gap Analysis | Gap Register | Blocking gaps resolved or escalated |
| 9 Design | Design Dossier | Realizes spec/contracts; failure model explicit |
| 10 Self-Review | Self-Review Log | All before-code questions answered, no unknowns |
| 11 Implementation | Code + Decision Trace | Conforms to contracts; commit via Kernel only |
| 12 Testing | Test Suite + Results | Invariant/failure/distributed tests pass |
| 13 Conformance | Conformance Verdict | PASS on target axes; property-based |
| 14 IAR | IAR Sign-off | Independent architecture go |
| 15 Certification | Certification Record | Target level (L1..L4/Product) achieved |

# Part 24 - Change Instruments and the Reference Lifecycle in the Workflow

When a task cannot proceed without changing something frozen, it does not proceed - it raises a change instrument. The workflow routes to exactly four instruments: CCP (Cognitive Change Proposal), Amendment, IDR, or next major version.

Reference Lifecycle (13 stages) governs maturity: Idea -> Research -> Theory -> Formalization -> Specification -> Ontology -> Contract -> Reference Behaviour -> Conformance -> Reference Runtime -> Certification -> Reference Product -> Reference Ecosystem. Maturity states: Draft -> Candidate -> Ratified -> Frozen -> Deprecated -> Superseded.

CCP-GATE binds the whole workflow: no behaviour is ratified without a conformance scenario. A task that would ratify behaviour must carry (or schedule) that scenario in its Gap Register before design.

Ecosystem endpoint the workflow serves: a production distributed runtime, a complete conformance suite, Independent Runtime A and Independent Runtime B both passing certification, third-party certification, an enterprise runtime, SDKs, a marketplace, cloud, and real products built on ARVES without modifying the standard.

*Final Definition  The UltraCode Workflow is the 15-phase discipline by which implementation PROVES the frozen ARVES specification - never changes it - fanning work out across phases without ever skipping a gate.*

# Reconciliation Note - Milestone-to-Level Mapping

Any milestone-to-certification-level pairing in this volume is illustrative only. The CANONICAL milestone-to-level mapping is defined in Volume 6 (Certification & Review Manual), Part 8; on any conflict, Volume 6 governs.

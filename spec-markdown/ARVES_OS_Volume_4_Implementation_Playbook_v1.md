> **Rendered from `ARVES_OS_Volume_4_Implementation_Playbook_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engineering Operating Manual - Volume 4: Implementation Playbook

**STATUS: Implementation Era (Active) | Specification Era FROZEN as of 2026-07-01 | Normative for UCI reference implementation | Non-normative for the UCS standard | Version 1.0 | Dependency direction: Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation (never reversed)**

This volume is the Implementation Playbook for building the Universal Cognitive Infrastructure (UCI), the reference implementation of the Universal Cognitive Standard (UCS). It governs HOW we build, never WHAT the standard means. The Specification Era is frozen. Implementation proves the specification and never changes it. Where a rule here would require altering a frozen contract, behaviour, or invariant, the rule is wrong and the implementer must stop and raise a change instrument (CCP, Amendment, IDR, or next major version) instead of redesigning while implementing.

Legend for normative weight used throughout this volume: REGISTERED invariants (ORCH-001..004, OWN-001, LAYER-001, SHARD-001) are normative and binding. Ontology principles O-001..007 are definitional design principles, not runtime-provable invariants. PROPOSED invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005) are INFORMATIVE only and are marked "proposed (pending CCP)" at every appearance; they must never be implemented as if ratified. IDR-001..005 are reference implementation decisions (non-normative) that ground all distributed mechanics in this volume.

# Part 1 - Purpose, Scope, and the Implementation Contract

Volume 4 defines the discipline of turning a frozen specification into running, certifiable software without mutating the specification. It is the operating contract between the people who wrote the standard and the people who build the runtime. The single hardest rule in this volume is also the simplest to state: never redesign while implementing.

## 1.1 What this volume governs

- The nine implementation properties every UCI component must exhibit.

- The twelve mandatory test types and when each is required.

- The IDR (Implementation Decision Record) process and template.

- The CCP (Cognitive Change Proposal) process, template, and the CCP-GATE.

- Per-milestone success criteria and the Definition of DONE for milestones I1 through I6.

## 1.2 What this volume must NOT do

- It must not introduce new layers, invariants, or milestones beyond the frozen Baseline.

- It must not reinterpret the meaning of any ratified contract or behaviour.

- It must not promote a PROPOSED invariant to normative status; only a ratified CCP can.

- It must not let the Kernel become the Control Plane, nor let the Control Plane own truth.

## 1.3 The dependency chain is one-directional

Every artifact in the build traces upward to something frozen. Implementation is the last link and the only mutable one. When reality disagrees with the specification, the specification wins and the implementation is defective; the remedy is a change instrument, not a quiet fix.

| Stage | Owner Era | Mutable now? | Implementation obligation |
| --- | --- | --- | --- |
| Theory | Frozen | No | Honour; never contradict. |
| Specification | Frozen | No | Prove via conformance; never edit. |
| Contracts | Frozen | No | Bind to exactly; version on change via CCP. |
| Behaviour | Frozen | No | Reproduce observable behaviour; no golden-output coupling. |
| Conformance | Frozen | No | Pass scenarios; verdict is invariant/property based. |
| Implementation (UCI) | Active | Yes | Deterministic, replayable, replaceable, observable, auditable, conformant, versioned, testable, independent. |

# Part 2 - The Nine Implementation Properties (The Implementation Standard)

Every UCI component, at every layer, must exhibit the nine properties below. These are the acceptance predicate for "is this implemented correctly", independent of feature completeness. A component that lacks any one of these is not DONE regardless of what it can do.

| # | Property | Definition | Grounded in | Primary test types |
| --- | --- | --- | --- | --- |
| 1 | Deterministic | Same recorded inputs + same decision trace -> same committed outcome; no hidden nondeterminism in the commit path. | ORCH-003, ORCH-004 | Replay, property, invariant |
| 2 | Replayable | State reconstructed from the recorded decision trace (WAL), NOT by recomputation of engine invocations. | ORCH-003, IDR-005 | Replay, recovery |
| 3 | Replaceable | Any component swappable behind its contract without changing callers; enables Independent Runtimes A and B. | LAYER-001, contracts | Conformance, integration, architecture |
| 4 | Observable | Metrics, logs, tracing, presence emitted on the AP path; observability never becomes a source of truth. | IDR (Truth=CP, observability=AP) | Integration, stress, failure-injection |
| 5 | Auditable | Every committed effect traceable to a provenance-bearing decision in the WAL; who/what/when reconstructable. | ORCH-001, O-003 (principle) | Replay, conformance, invariant |
| 6 | Conformant | Passes the relevant reference conformance scenarios for its certification level. | Conformance suite, CCP-GATE | Conformance, certification |
| 7 | Versioned | Every type/contract versioned + registered; breaking change requires a new version via CCP. | O-006 (principle), change instruments | Architecture, conformance |
| 8 | Testable | Exercisable in isolation and in composition; deterministic seams for injection and replay. | Test types 1-12 | All 12 test types |
| 9 | Independent | No lateral coupling; cross-cutting concerns flow through Control Plane / Event Fabric only. | LAYER-001 | Architecture, distributed |

## 2.1 The prime directive: never redesign while implementing

When an implementer discovers that the specification appears wrong, incomplete, or awkward, the correct action is to STOP and raise a change instrument. Editing behaviour, relaxing a contract, or "improving" an invariant inside a pull request is prohibited. The implementation records the friction as an IDR (if it is a reference-implementation choice) or escalates to a CCP (if it would change meaning). This preserves the property that implementation proves the spec.

## 2.2 Two planes, one truth

The Control Plane decides; the Data Plane carries. The Kernel owns TRUTH and never becomes the Control Plane. The Control Plane owns Plan and Engine Graph and never owns truth or persistent state (ORCH-001, ORCH-002). Every implementation task must be able to answer: which plane is this code on, and does it respect that the Kernel is the only committer of truth?

# Part 3 - The Layer Model as an Implementation Constraint

Dependencies are downward-only (LAYER-001). No lateral coupling between peer layers; cross-cutting concerns are mediated by the Control Plane or the Event Fabric. The layer stack is fixed:

Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine -> Capability -> Execution, plus the cross-cutting Control Plane.

## 3.1 Layer Responsibility Matrix (implementation view)

| Layer | Owns | Reads | Writes | Cannot |
| --- | --- | --- | --- | --- |
| Kernel | TRUTH (commits) | Committed state, proposed effects | Commits to WAL | Orchestrate / plan / execute |
| LCW | Working Memory / live state (NOT truth) | Kernel truth, events | Live/working state only | Own or assert truth |
| Query | Nothing | Committed + working state | Nothing (READ-ONLY) | Mutate any state |
| Engine | Nothing persistent | Inputs, context | Proposed effects only (Kernel commits) | Commit truth; hold persistent state |
| Capability Fabric | Registry + bindings | Registry, contracts | Registry/binding records | Own truth or plan graph |
| Control Plane | Plan / Engine Graph | Contracts, capability registry | Plans (never persistent state) | Own truth; hold persistent state |

## 3.2 Architecture-test consequences

- An import from a higher layer into a lower layer is an architecture-test failure.

- A lateral import between peers (e.g., Query -> Engine) that bypasses Control Plane/Event Fabric fails.

- Any write path in Query, or any commit path outside the Kernel, fails the invariant test.

- Any persistent state held by Engine or Control Plane fails ORCH-002 checks.

# Part 4 - Registered Invariants as Executable Guards

The registered invariants are normative and must be encoded as executable guards (invariant tests) that run in CI and, where feasible, as runtime assertions on the commit path. They are the non-negotiable contract of the runtime.

| Invariant | Statement | How the implementation enforces it |
| --- | --- | --- |
| ORCH-001 | Control Plane owns no truth; only Kernel owns truth. | Commit API exposed only by Kernel; CP has no persistence handle; invariant test asserts no CP write to committed store. |
| ORCH-002 | Control Plane produces plans, never persistent state. | Plans are values passed to Kernel; CP holds no durable store; architecture test forbids CP persistence deps. |
| ORCH-003 | Execution replayable from recorded decision trace, NOT recomputation. | Replay reconstructs from WAL/decision trace; replay test forbids re-invoking engines to rebuild state. |
| ORCH-004 | Every engine/capability invocation idempotent + content-addressable. | Invocations keyed by content hash; dedup at commit; property test: repeat invocation -> same outcome, no double effect. |
| OWN-001 | Every state has exactly one owner. | Ownership registry; static check maps each state type to one owner layer; invariant test flags multi-owner state. |
| LAYER-001 | Downward-only deps; no lateral coupling; cross-cutting via CP/Event Fabric. | Architecture test on the dependency graph; build fails on upward/lateral edges. |
| SHARD-001 | Partition by tenant/workspace; partition key immutable. | Shard key set at creation, never updatable; invariant test rejects any key mutation path. |

## 4.1 Ontology design principles (definitional, not runtime-provable)

O-001..007 shape types and meaning but are NOT enforced as runtime invariants. They inform reviews and modelling, not CI gates.

- O-001 everything is a cognitive entity.

- O-002 every entity has identity.

- O-003 every observation has provenance.

- O-004 truth emerges from validated evidence.

- O-005 derivation is not inheritance.

- O-006 every type versioned + registered.

- O-007 ontology defines meaning, not storage.

## 4.2 Proposed invariants (INFORMATIVE only - pending CCP)

The following are PROPOSED and pending CCP. They must be labelled "proposed (pending CCP)" wherever referenced and must NOT be enforced as normative gates. They may inform design exploration and draft tests marked as non-blocking.

| Proposed id (pending CCP) | Concern area | Implementation status |
| --- | --- | --- |
| G-001 (proposed, pending CCP) | Global/cross-cutting guarantee | Informative; not enforced. |
| QUERY-001 (proposed, pending CCP) | Query read-only guarantees | Informative; Query read-only is also implied by matrix. |
| LCW-001 (proposed, pending CCP) | Working memory semantics | Informative; not enforced. |
| PERSIST-001 (proposed, pending CCP) | Persistence guarantees | Informative; not enforced. |
| CAP-001..009 (proposed, pending CCP) | Capability fabric guarantees | Informative; not enforced. |
| ENG-001..005 (proposed, pending CCP) | Engine purity/statelessness guarantees | Informative; not enforced. |

# Part 5 - The Twelve Mandatory Test Types

Every UCI component ships with the applicable subset of these twelve test types; distributed components ship all twelve. Conformance verdicts are invariant/property based, never golden-output based. The suite is the operational meaning of the "conformant" and "testable" properties.

| # | Test type | What it proves | Verdict basis | Ties to |
| --- | --- | --- | --- | --- |
| 1 | Unit | Single unit behaves per contract in isolation. | Assertions on outputs/effects | Testable |
| 2 | Integration | Composed units honour contracts across seams. | Cross-seam assertions | Replaceable, observable |
| 3 | Architecture | Dependency graph is downward-only; no lateral/upward edges; single ownership. | LAYER-001, OWN-001 checks | Independent, versioned |
| 4 | Invariant | Registered invariants hold under all exercised paths. | ORCH/OWN/SHARD guards | All registered invariants |
| 5 | Distributed | Correctness across nodes/shards, leaders/followers. | Linearizability + invariant checks | IDR-001..004 |
| 6 | Replay | State rebuilt from decision trace equals live state; no recomputation. | Trace-vs-state equivalence | ORCH-003, IDR-005 |
| 7 | Property | Properties hold over generated inputs (idempotency, determinism). | Property predicates | ORCH-004, determinism |
| 8 | Stress | Behaviour under load/high-volume streaming stays correct + bounded. | SLO + invariant under load | Observable, high-volume axis |
| 9 | Failure-injection | Faults (crash, partition, split-brain, slow disk) do not violate truth. | No partial truth; invariants hold | Amendment-006, IDR |
| 10 | Recovery | After fault, node/shard recovers from WAL + snapshot to consistent truth. | Post-recovery invariant + replay | IDR-002, IDR-005 |
| 11 | Conformance | Reference scenarios pass for the target certification level. | PASS/PARTIAL/FAIL (invariant/property) | Conformance suite |
| 12 | Certification | End-to-end certification level achieved (L1..L4, Certified Product). | Level probe verdicts | Certification levels |

## 5.1 Applicability matrix by component kind

| Component kind | Required test types |
| --- | --- |
| Pure Engine (stateless) | 1,2,3,4,7,11 (+ replay 6 for effect-proposal determinism) |
| Kernel / commit path | All 1-12 (mandatory) |
| Query (read-only) | 1,2,3,4,7,8,11 (write-path invariant tests assert zero writes) |
| LCW (working memory) | 1,2,3,4,6,9,10,11 |
| Capability Fabric | 1,2,3,4,7,11 |
| Distributed subsystem | All 1-12 (mandatory) |

## 5.2 Verdict rule (normative for conformance)

- Conformance and certification verdicts are PASS / PARTIAL / FAIL.

- Verdicts are invariant/property based; comparing to a frozen golden output is prohibited.

- CCP-GATE: no behaviour may be ratified without an accompanying conformance scenario.

# Part 6 - Distributed Implementation Grounding (IDR-001..005)

All distributed mechanics in UCI are grounded in the reference IDRs below. These are non-normative reference-implementation decisions: an Independent Runtime may choose other mechanisms so long as it satisfies the same contracts and conformance scenarios.

| IDR | Decision | Key mechanics |
| --- | --- | --- |
| IDR-001 | Kernel is the Control-of-Truth via per-shard Raft. | One Raft group per tenant/workspace shard; replicate committed OUTCOMES not engine invocations; engines run anywhere, only the commit goes through the shard leader; Raft log = WAL = decision trace; no cross-shard atomic commit in v1 (use sagas). |
| IDR-002 | Replication topology. | Leader -> followers replication + snapshots + WAL. |
| IDR-003 | Membership changes. | Raft joint consensus. |
| IDR-004 | Leadership. | Per-shard leader election. |
| IDR-005 | Durability format. | Append-only WAL + snapshots. |

## 6.1 Truth is CP; observability is AP

In the CAP sense, the truth (commit) path is CP: it favours consistency and will refuse to commit rather than fork truth. Observability (metrics, logs, tracing, presence) is AP: it stays available and eventually consistent and is never a source of truth. Implementers must not read observability data as if it were committed state.

## 6.2 Read consistency tiers

| Tier | Served from | Guarantee | Use when |
| --- | --- | --- | --- |
| Linearizable | Through the shard leader | Freshest, ordered, authoritative | Truth-critical reads, decisions |
| Bounded-staleness | A follower | Fresh within a bounded lag | Dashboards needing near-real-time |
| Eventual | A replica | Eventually consistent | Cheap, tolerant analytics |

## 6.3 No cross-shard atomic commit in v1

- Cross-shard workflows use sagas, not a distributed atomic commit.

- Committed effects that must be undone are compensated saga-style (Amendment-006).

- Partition key is immutable (SHARD-001); a workflow never migrates a shard key mid-flight.

# Part 7 - Failure, Cancellation, and Recovery Model

## 7.1 Failure model (Amendment-006)

- Partial execution is rolled back by simply NOT committing; uncommitted work has no truth effect.

- Committed effects are compensated saga-style, never mutated in place to fake atomicity.

- Node crash, network partition, and split-brain are handled by the IDR Raft mechanisms.

## 7.2 Cancellation model (Amendment-005)

- Cancellation is cooperative and idempotent; a cancel may arrive more than once.

- Cancellation produces no partial truth; either an outcome is committed or it is not.

- Priority and preemption use checkpoint -> replay, consistent with ORCH-003.

## 7.3 Recovery obligations

| Fault | Detection | Recovery path | Test type |
| --- | --- | --- | --- |
| Node crash | Leader election / heartbeat loss | Rejoin, replay WAL from snapshot | Recovery, failure-injection |
| Partition | Quorum loss on minority side | Minority refuses commits; heals on rejoin | Distributed, failure-injection |
| Split-brain risk | Raft term/quorum rules | Only quorum leader commits; stale leader steps down | Distributed, invariant |
| Corrupt/slow disk | WAL checksum / latency SLO | Snapshot restore + WAL replay | Recovery, stress |

# Part 8 - The IDR Process (Implementation Decision Record)

An IDR records a reference-implementation choice that is NOT normative and does NOT change the specification. IDRs are how UCI documents "how we chose to build it" without contaminating the standard. If a decision would change meaning, contract, or behaviour, it is NOT an IDR; it is a CCP.

## 8.1 When to write an IDR vs escalate to a CCP

| Situation | Instrument |
| --- | --- |
| Choosing a replication algorithm, storage engine, or wire format | IDR |
| A build-time trade-off that any Independent Runtime could decide differently | IDR |
| A change that alters a frozen contract, behaviour, or invariant meaning | CCP (STOP; do not implement) |
| Adding/removing a layer, invariant, or milestone | CCP or Amendment (never inline) |
| Promoting a PROPOSED invariant to normative | CCP (only path) |

## 8.2 IDR template

- IDR id and title (e.g., IDR-006 - Snapshot cadence).

- Status: Proposed / Accepted / Superseded (and by which IDR).

- Context: the forces and constraints; which layer/plane it touches.

- Decision: the chosen mechanism, stated precisely.

- Non-normativity statement: confirms it does not change spec/contract/behaviour.

- Alternatives considered and why rejected.

- Invariants respected: list the REGISTERED invariants upheld (ORCH/OWN/LAYER/SHARD).

- Conformance impact: which scenarios still pass; none weakened.

- Independent-runtime note: how another runtime could decide differently and still conform.

- Consequences: operational, testing, and observability implications.

## 8.3 Worked example (illustrative, non-normative)

IDR-001 records that the Kernel realizes control-of-truth via per-shard Raft, replicating committed outcomes and treating the Raft log as WAL and decision trace. It respects ORCH-001 (only Kernel owns truth), ORCH-003 (replay from trace), and SHARD-001 (immutable partition key). An Independent Runtime B could substitute a different consensus protocol and remain conformant because the contract, not the algorithm, is normative.

# Part 9 - The CCP Process (Cognitive Change Proposal)

The CCP is the primary change instrument for anything that touches meaning: theory, specification, contract, behaviour, ontology, invariant status, layers, or milestones. During the Implementation Era the specification is frozen, so any such change MUST go through a CCP and MUST NOT be made inline while implementing.

## 9.1 CCP template

- CCP id and title.

- Status/maturity target: Draft -> Candidate -> Ratified -> Frozen (and later Deprecated/Superseded).

- Motivation: the friction discovered during implementation or use.

- Affected artifacts: theory/spec/contract/behaviour/invariant/layer/milestone.

- Dependency-chain analysis: what upstream stages are impacted and why the change is safe.

- Invariant impact: which REGISTERED invariants are affected; promotion of any PROPOSED invariant is stated explicitly.

- Proposed conformance scenario(s): MANDATORY - the CCP-GATE requirement.

- Certification impact: which levels (L1..L4) are affected and re-run needs.

- Migration/versioning plan: new version numbers; backward-compat notes.

- Rollback plan and decision record.

## 9.2 CCP-GATE (normative)

No behaviour may be ratified without an accompanying conformance scenario. A CCP that changes or adds behaviour but ships no conformance scenario is automatically blocked at the gate. This gate is what keeps "conformant" meaningful across Independent Runtimes.

| Gate check | Pass condition |
| --- | --- |
| Conformance scenario present | At least one reference scenario covers the new/changed behaviour. |
| Verdict style | Scenario verdict is invariant/property based, not golden-output. |
| Invariant consistency | No REGISTERED invariant is violated; any promotion is explicit. |
| Dependency direction | Change does not reverse Theory->...->Implementation. |
| Independent-runtime feasibility | Two independent runtimes could both satisfy it. |

## 9.3 Reference Lifecycle and maturity

The full reference lifecycle has 13 stages: Idea -> Research -> Theory -> Formalization -> Specification -> Ontology -> Contract -> Reference Behaviour -> Conformance -> Reference Runtime -> Certification -> Reference Product -> Reference Ecosystem. Maturity for any artifact moves Draft -> Candidate -> Ratified -> Frozen -> Deprecated -> Superseded.

# Part 10 - Conformance and Certification in Practice

Conformance flows from twelve axes to reference scenarios to node probes to a verdict. Certification composes conformance results into levels.

## 10.1 The twelve conformance axes

| # | Axis | Stresses |
| --- | --- | --- |
| 1 | Information-intensive | Large knowledge/state handling. |
| 2 | Event-driven | Reactive event processing. |
| 3 | Human-collaboration | Human-in-the-loop flows. |
| 4 | Multi-step planning | Plan generation and execution. |
| 5 | Long-running | Durable, resumable work. |
| 6 | Physical-world | Actuation and real-world effects. |
| 7 | Safety-critical | No partial truth under fault. |
| 8 | High-volume streaming | Throughput and backpressure. |
| 9 | Multi-agent | Coordination across agents. |
| 10 | Policy-heavy | Rich policy/authorization. |
| 11 | Autonomous | Self-directed operation. |
| 12 | Recovery/replay | Rebuild from decision trace. |

## 10.2 Conformance pipeline and verdict

- Axes -> reference scenarios -> node probes -> verdict (PASS / PARTIAL / FAIL).

- Verdicts are invariant/property based, never golden-output based.

- A PARTIAL verdict names the exact scenario/probe that did not fully pass.

## 10.3 Certification levels

| Level | Name | Focus |
| --- | --- | --- |
| L1 | Core Runtime | Kernel truth, WAL, replay, single-shard correctness. |
| L2 | Cognitive Control | Control Plane planning, engine invocation, capability binding. |
| L3 | Distributed | Multi-shard, replication, membership, failure/recovery. |
| L4 | Multi-Agent | Multi-agent coordination and shared conformance. |
| Certified Product | Certified Product | A real product built on ARVES without modifying the standard. |

## 10.4 Ecosystem goals (build targets, not new architecture)

- Production distributed runtime and a complete conformance suite.

- Independent Runtime A and Independent Runtime B both pass certification.

- Third-party certification, enterprise runtime, SDKs, marketplace, and cloud.

- Real products built on ARVES without modifying the standard.

# Part 11 - Milestones I1..I6: Scope and Success Criteria

The frozen Baseline (Part 5) defines exactly six implementation milestones. These names are used verbatim; no other milestone names exist. Each milestone maps to certification levels and to the test types that must pass.

| Milestone | Name | Primary scope | Certification target | Grounded in |
| --- | --- | --- | --- | --- |
| I1 | Distributed Runtime | Node/process runtime, WAL, single-shard Raft, replay foundation. | Toward L1 | IDR-001, IDR-005, ORCH-003 |
| I2 | Cluster Kernel | Multi-node Kernel: replication, membership, leader election, snapshots. | L1 -> L3 foundation | IDR-002, IDR-003, IDR-004 |
| I3 | Distributed Query | Read-only distributed query across shards with consistency tiers. | L3 | QUERY read-only, SHARD-001 |
| I4 | Capability Scheduling | Capability registry/bindings + scheduling of idempotent invocations. | L2 | ORCH-004, Capability Fabric |
| I5 | Multi-Agent Runtime | Multi-agent coordination on the distributed runtime. | L4 | ORCH-001..004, multi-agent axis |
| I6 | Reference Products | Certified reference products built without modifying the standard. | Certified Product | Ecosystem goals |

## 11.1 Per-milestone success criteria

## I1 - Distributed Runtime

- Kernel is the sole committer of truth; commit path exposed nowhere else (ORCH-001).

- Append-only WAL + snapshots implemented; Raft log == WAL == decision trace (IDR-005, IDR-001).

- Replay reconstructs state from the decision trace, not by recomputation (ORCH-003).

- Invariant tests for ORCH-001/003/004, OWN-001, SHARD-001 pass; architecture test green (LAYER-001).

- Unit, integration, architecture, invariant, replay, property tests pass.

## I2 - Cluster Kernel

- Leader -> follower replication with snapshots operational (IDR-002).

- Membership changes via Raft joint consensus (IDR-003); per-shard leader election (IDR-004).

- Failure-injection (crash, partition, split-brain) yields no partial truth (Amendment-006).

- Recovery from WAL + snapshot restores consistent truth; recovery + distributed tests pass.

- Only the quorum leader commits; stale leaders step down (no split-brain truth fork).

## I3 - Distributed Query

- Query writes nothing; write-path invariant tests assert zero writes (READ-ONLY).

- Linearizable, bounded-staleness, and eventual read tiers implemented and selectable.

- Cross-shard reads respect immutable partition keys (SHARD-001).

- Stress tests hold correctness and bounded latency under high-volume streaming.

- Conformance scenarios for information-intensive and high-volume axes pass.

## I4 - Capability Scheduling

- Capability Fabric owns registry + bindings; Control Plane owns the plan/engine graph only.

- Every engine/capability invocation is idempotent + content-addressable (ORCH-004).

- Engines hold no persistent state; writes are proposed effects only the Kernel commits.

- Property tests prove repeat invocation -> same outcome with no duplicated effect.

- Conformance for multi-step planning axis passes at L2.

## I5 - Multi-Agent Runtime

- Multi-agent coordination runs on the distributed runtime without the Kernel becoming CP.

- Control Plane owns plans, never truth or persistent state (ORCH-001, ORCH-002).

- Cancellation is cooperative + idempotent; preemption uses checkpoint -> replay (Amendment-005).

- Cross-shard flows use sagas; committed effects compensated saga-style (Amendment-006).

- Conformance for multi-agent and autonomous axes passes at L4.

## I6 - Reference Products

- At least one reference product certified as a Certified Product.

- Independent Runtime A and Independent Runtime B both pass certification.

- No product modifies the standard; all changes to meaning went through CCPs.

- Complete conformance suite runs green across the twelve axes.

- SDKs / marketplace / cloud packaging available as ecosystem deliverables.

# Part 12 - Definition of DONE per Milestone

A milestone is DONE only when ALL of the following hold. "Feature works on my machine" is never DONE. DONE is defined by properties, invariants, tests, conformance, and clean change hygiene.

## 12.1 Universal DONE checklist (applies to every milestone)

- All nine implementation properties demonstrably hold for the milestone components.

- All applicable test types (from Part 5) pass in CI, including invariant and architecture tests.

- No REGISTERED invariant is violated on any exercised path.

- No PROPOSED invariant was silently promoted; any promotion has a ratified CCP.

- No spec/contract/behaviour was edited during implementation; frictions captured as IDRs or CCPs.

- Relevant conformance scenarios return PASS (or documented PARTIAL with an accepted plan).

- Observability (metrics/logs/tracing/presence) present on the AP path and never used as truth.

- IDRs for all reference-implementation choices are written and accepted.

- Replay verified: state rebuilt from decision trace equals live state.

## 12.2 DONE mapping per milestone

| Milestone | DONE gate (in addition to universal checklist) |
| --- | --- |
| I1 | Replay from WAL exact; single-shard invariant + architecture green; L1 foundation probes pass. |
| I2 | Failure-injection + recovery pass; no split-brain truth; L3 replication/membership probes pass. |
| I3 | Zero-write proof for Query; three read tiers verified; high-volume conformance PASS. |
| I4 | Idempotency + content-addressability proven; L2 planning conformance PASS. |
| I5 | Multi-agent + autonomous conformance PASS at L4; saga compensation verified. |
| I6 | Certified Product achieved; Runtime A and B certified; full suite green; standard unmodified. |

# Part 13 - Implementation Workflow and Anti-Patterns

## 13.1 The per-change workflow

- Locate the frozen artifact the change proves (contract/behaviour/scenario).

- Confirm the plane (Control vs Data) and the owning layer for any state touched.

- Write or extend the applicable test types FIRST where practical (invariant + property).

- Implement behind the contract; keep engines pure and the commit path in the Kernel.

- If friction with the spec appears: STOP; write an IDR, or escalate to a CCP; never redesign inline.

- Run architecture + invariant tests locally; ensure downward-only deps.

- Attach/refresh the conformance scenario if behaviour changed (CCP-GATE).

## 13.2 Anti-patterns (automatic rejection)

| Anti-pattern | Why it fails | Correct action |
| --- | --- | --- |
| Editing a frozen behaviour to make a test pass | Reverses the dependency chain | Fix the implementation or raise a CCP |
| Kernel starts orchestrating/planning | Kernel becomes Control Plane | Keep planning in Control Plane |
| Control Plane persists state | Violates ORCH-002 | Pass plans to Kernel; hold no store |
| Rebuilding state by recomputation | Violates ORCH-003 | Replay from decision trace |
| Non-idempotent invocation | Violates ORCH-004 | Content-address + dedup at commit |
| Query writing state | Violates read-only role | Route mutations through Kernel |
| Treating a PROPOSED invariant as normative | Skips the CCP | Label proposed; do not gate on it |
| Golden-output conformance check | Wrong verdict basis | Use invariant/property verdicts |
| Cross-shard atomic commit in v1 | Not supported | Use sagas + compensation |
| Mutating a partition key | Violates SHARD-001 | Keys are immutable at creation |

# Part 14 - Traceability, Evidence, and Audit

Auditability is a first-class property. Every committed effect must trace to a provenance-bearing decision in the WAL, and every milestone must produce an evidence bundle that a certifier can independently verify.

## 14.1 Evidence bundle contents (per milestone)

- Test reports for all applicable test types with pass/fail and coverage of exercised paths.

- Invariant guard results for ORCH-001..004, OWN-001, LAYER-001, SHARD-001.

- Conformance verdicts (PASS/PARTIAL/FAIL) per relevant axis and scenario.

- Replay evidence: decision-trace-rebuilt state equals live state.

- Accepted IDRs and any ratified CCPs used during the milestone.

- Observability samples proving AP-path emission and non-use as truth.

## 14.2 Traceability matrix (illustrative)

| Artifact | Traces up to | Verified by |
| --- | --- | --- |
| Commit path code | Kernel truth contract; ORCH-001 | Invariant + replay tests |
| Replication code | IDR-002/003/004 | Distributed + recovery tests |
| Query read tiers | Read consistency tiers | Integration + conformance |
| Capability scheduling | ORCH-004; Capability Fabric | Property + conformance |
| Multi-agent coordination | ORCH-001/002; multi-agent axis | Conformance at L4 |

# Part 15 - Quick-Reference Cards

## 15.1 STOP conditions (never proceed; raise an instrument)

- You need to change a frozen contract, behaviour, invariant, layer, or milestone.

- You need to promote a PROPOSED invariant to normative.

- You need a cross-shard atomic commit (not in v1).

- You need the Kernel to plan/orchestrate or the Control Plane to persist truth.

## 15.2 Normative-weight card

- Normative: ORCH-001..004, OWN-001, LAYER-001, SHARD-001; the CCP-GATE; certification verdict rules.

- Definitional principles (not runtime invariants): O-001..007.

- Non-normative reference decisions: IDR-001..005.

- Informative, pending CCP (never gate): G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005.

## 15.3 Milestone-to-level card

| Milestone | Level target |
| --- | --- |
| I1 Distributed Runtime | L1 foundation |
| I2 Cluster Kernel | L1 -> L3 |
| I3 Distributed Query | L3 |
| I4 Capability Scheduling | L2 |
| I5 Multi-Agent Runtime | L4 |
| I6 Reference Products | Certified Product |

*Final Definition  Implementation proves the frozen specification and never changes it: build deterministic, replayable, replaceable, observable, auditable, conformant, versioned, testable, and independent runtimes across milestones I1 through I6, enforce the registered invariants as executable guards, ground all distribution in IDR-001..005, and route every change of meaning through a CCP that clears the CCP-GATE.*

# Reconciliation Note - Milestone-to-Level Mapping

Any milestone-to-certification-level pairing in this volume is illustrative only. The CANONICAL milestone-to-level mapping is defined in Volume 6 (Certification & Review Manual), Part 8; on any conflict, Volume 6 governs.

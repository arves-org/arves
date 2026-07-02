> **Rendered from `ARVES_OS_Volume_6_Certification_Review_Manual_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES OS Volume 6 - Certification & Review Manual v1.0

STATUS: CERTIFICATION CONSTITUTION (INDEPENDENT VERDICT ON CORRECTNESS AND READINESS)

SCOPE: Scenario Conformance Framework in depth; certification levels L1-L4 + Certified Product; the Independent Architecture Review process; conformance / performance / enterprise-readiness checklists; Independent Runtime A/B goals; certified ecosystem and product goals; versioning of conformance suites against spec versions.

SPECIFICATION ERA: FROZEN as of 2026-07-01. IMPLEMENTATION ERA: IN PROGRESS. This volume certifies implementations against the frozen standard; it never modifies the standard.

# Part 1 - Purpose and Position in the Dependency Chain

Volume 6 is the certification and review authority of the ARVES corpus. Where earlier volumes DEFINE (Theory, Specification, Contracts, Behaviour) and the Scenario Conformance Framework DECLARES what correctness means, this volume answers a single question: has a given implementation earned the right to call itself an ARVES runtime? It does so through two independent instruments - the Scenario Conformance Framework (mechanical, property-based verdicts) and the Independent Architecture Review (adversarial, human-judged verdicts).

The dependency chain is never reversed:

- Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation.

Certification sits at the Conformance -> Implementation boundary. An implementation PROVES the specification; it never changes it. A failed certification is a defect in the implementation (or, at most, a discovered gap that must be routed through a CCP), never a licence to edit the frozen spec.

ARVES is Universal Cognitive Infrastructure. UCS (Universal Cognitive Standard) is the standard; UCI is its reference implementation. This manual certifies UCI and any Independent Runtime against UCS.

| Instrument | Nature | Verdict Space | Authority |
| --- | --- | --- | --- |
| Scenario Conformance Framework | Mechanical, executable, property-based | PASS / PARTIAL / FAIL per scenario | Automated harness + probes |
| Independent Architecture Review | Adversarial, human-judged | PASS / PARTIAL / FAIL per dimension | Review board (arms-length) |
| Certification Levels L1-L4 | Composite gate over both | Level granted / withheld | Certification authority |
| Certified Product | Product-grade attestation | Granted / withheld | Certification authority |

# Part 2 - Principles of ARVES Certification

Certification is governed by principles that make the verdict independent, reproducible, and honest.

- Property-based, not golden-output. A verdict is a statement about invariants and properties holding across a trace, NOT a byte-comparison against a recorded expected output. Cognitive systems are non-deterministic at the inference layer; correctness is defined at the invariant layer.

- Independence. The reviewer evaluates a submission as if a rival company built it. Familiarity with the reference implementation must not soften the verdict.

- Reproducibility. Every verdict is replayable from a recorded decision trace (see ORCH-003), not by recomputing engine inference.

- Traceability. Every scenario maps to at least one axis and every node probe maps to a contract clause.

- No behaviour without a scenario. CCP-GATE forbids ratifying any behaviour that lacks a conformance scenario.

- Certification never edits the spec. Discovered gaps route through CCP, Amendment, IDR, or next major version.

- Two planes are respected in every verdict. Truth verdicts are evaluated against the Control Plane / Kernel boundary; observability data (metrics, logs, tracing, presence) is AP and never treated as truth.

# Part 3 - The Scenario Conformance Framework - Three-Layer Model

Conformance is defined in three separated layers plus a verdict. This separation lets a single axis be exercised by many scenarios, and a single scenario to be observed by many node probes.

- AXIS - a capability dimension the architecture is stressed on (12 defined, see Part 4).

- REFERENCE SCENARIO - a concrete, versioned instantiation combining several axes into an executable narrative.

- NODE PROBE - an observation point bound to a contract clause; it records what an engine/capability/kernel node did, so an invariant or property can be evaluated.

- VERDICT - PASS / PARTIAL / FAIL, computed from invariant and property checks over the recorded trace.

| Layer | Answers | Bound To | Versioned Against |
| --- | --- | --- | --- |
| Axis | Which dimension of capability is stressed | Architecture stress model | UCS major version |
| Reference Scenario | What concrete workload is run | One or more axes | Conformance suite version |
| Node Probe | What a specific node actually did | A contract clause | Contract version |
| Verdict | Did the invariants/properties hold | Registered invariants + properties | Both suite and spec version |

# Part 4 - The Twelve Conformance Axes

The twelve axes are the fixed capability dimensions along which any ARVES runtime is stressed. They are frozen with the specification; new axes require a new UCS major version.

| # | Axis | What It Stresses | Primary Invariant Focus |
| --- | --- | --- | --- |
| 1 | Information-intensive | Large ontology graphs, dense entity/observation load | OWN-001, provenance (O-003) |
| 2 | Event-driven | Reactive flows over the Event Fabric | LAYER-001 cross-cutting via Event Fabric |
| 3 | Human-collaboration | Human-in-the-loop decisions, approvals, handoff | ORCH-001 (truth only in Kernel) |
| 4 | Multi-step planning | Plan/Engine Graph over many steps | ORCH-002 (plans, not persistent state) |
| 5 | Long-running | Durable state across time, checkpoints | ORCH-003 (replay from trace) |
| 6 | Physical-world | Actuation, sensing, irreversible effects | ORCH-004 (idempotent + content-addressable) |
| 7 | Safety-critical | Fail-closed behaviour, no partial truth | Amendment-006 failure model |
| 8 | High-volume streaming | Sustained throughput, backpressure | SHARD-001 partitioning |
| 9 | Multi-agent | Many agents coordinating over shared truth | ORCH-001..004 under concurrency |
| 10 | Policy-heavy | Dense governance/authorization constraints | ORCH-001, OWN-001 |
| 11 | Autonomous | Self-directed goal pursuit, minimal human input | ORCH-002, ORCH-003 |
| 12 | Recovery/replay | Crash, partition, split-brain recovery | ORCH-003, IDR-001..005 |

Note on invariants: ORCH-001..004, OWN-001, LAYER-001, SHARD-001 are REGISTERED (normative). Ontology principles O-001..007 are DESIGN PRINCIPLES (definitional, not runtime-provable) and are used to shape scenarios, not to pass/fail a node. Proposed invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-*, ENG-*) are INFORMATIVE only and marked proposed (pending CCP) wherever they appear.

# Part 5 - Reference Scenarios

A reference scenario is a concrete, versioned workload combining several axes. Each scenario declares its axis coverage, its entry conditions, the node probes it activates, and the invariants/properties its verdict evaluates. Scenarios are illustrative reference workloads, not an exhaustive catalog; the suite grows through CCP-gated additions.

| Scenario | Axes Combined | Verdict Properties (examples) |
| --- | --- | --- |
| Ingest-and-Derive | 1 Information-intensive, 2 Event-driven | Every observation carries provenance; only Kernel commits truth (ORCH-001); derivation != inheritance |
| Plan-and-Act | 4 Multi-step planning, 6 Physical-world | Plans are not persistent state (ORCH-002); every capability invocation idempotent + content-addressable (ORCH-004) |
| Human-Gated Approval | 3 Human-collaboration, 10 Policy-heavy | No effect committed without gate; single owner per state (OWN-001) |
| Long-Run Saga | 5 Long-running, 7 Safety-critical | Partial execution rolled back by NOT committing; committed effects compensated saga-style (Amendment-006) |
| Stream-Under-Load | 8 High-volume streaming, 2 Event-driven | Partition key immutable (SHARD-001); bounded-staleness reads remain within bound |
| Swarm-Coordinate | 9 Multi-agent, 11 Autonomous | Concurrent agents never produce conflicting committed truth; execution replayable (ORCH-003) |
| Crash-and-Replay | 12 Recovery/replay, 5 Long-running | Replay reconstructs from recorded decision trace, NOT recomputation (ORCH-003); WAL = decision trace (IDR-001) |

# Part 6 - Node Probes and Verdict Computation

A node probe is an observation point bound to a contract clause. It records structured evidence about a node - an engine invocation, a capability binding, a Kernel commit, a Query read - so a property can be evaluated after the fact. Probes are passive: they observe, they do not steer.

Verdict computation proceeds in three stages:

- Collect. Run the scenario; probes append evidence to the recorded decision trace (the WAL / Raft log under IDR-001).

- Evaluate. For each declared invariant and property, evaluate a predicate over the collected trace. Predicates are property-based (e.g. "for all committed effects, exactly one owner") not golden-output comparisons.

- Aggregate. Combine per-property results into a scenario verdict.

| Verdict | Meaning | Gate Effect |
| --- | --- | --- |
| PASS | All declared invariants and properties held across the trace | Counts toward level certification |
| PARTIAL | Core invariants held; one or more non-blocking properties failed or were unproven | Level withheld until resolved; documented as known limitation |
| FAIL | A registered invariant was violated | Blocks certification; defect in implementation |

A registered-invariant violation is always FAIL. A proposed-invariant expectation (G-001/QUERY-001/LCW-001/PERSIST-001/CAP-*/ENG-*, all proposed pending CCP) can at most produce PARTIAL, never FAIL, because it is not yet normative.

# Part 7 - The Conformance Artifact

Every certification run emits a single immutable conformance artifact. It is the evidence package a third party can independently replay. Because ORCH-003 requires replay from the recorded decision trace and not recomputation, the artifact is self-sufficient: given the artifact, a verifier reconstructs the verdict without re-running engines.

| Field | Content | Source |
| --- | --- | --- |
| Suite version | Conformance suite version identifier | Suite registry |
| Spec version | UCS version under test (frozen 2026-07-01 for v1) | Specification Freeze Record |
| Runtime identity | Implementation name + build (UCI or Independent Runtime A/B) | Submission |
| Axis coverage | Which of the 12 axes were exercised | Scenario declarations |
| Scenario results | Per-scenario PASS/PARTIAL/FAIL | Verdict computation |
| Decision trace | Recorded WAL / Raft log enabling replay (ORCH-003, IDR-001/005) | Runtime under test |
| Invariant matrix | Per registered invariant: held / violated | Property evaluation |
| Proposed-invariant notes | Informative results for proposed invariants, flagged pending CCP | Property evaluation |
| Level attestation | Which of L1-L4 / Certified Product is supported | Aggregation |

# Part 8 - Certification Levels L1-L4 and Certified Product

Certification levels are cumulative. A runtime must hold every lower level before a higher level is granted. Levels map to the frozen milestones I1..I6 and to the layer/plane responsibilities.

| Level | Name | What It Certifies | Milestone Alignment |
| --- | --- | --- | --- |
| L1 | Core Runtime | Single-node truth: Kernel owns TRUTH and commits; Engine pure/stateless produces only proposed effects; Query READ-ONLY; OWN-001 holds | I1 Distributed Runtime (single-node baseline) |
| L2 | Cognitive Control | Control Plane owns Plan/Engine Graph; ORCH-001/002 hold (no truth, no persistent state in CP); LCW owns working memory not truth | I1 -> I2 |
| L3 | Distributed | Cluster Kernel: per-shard Raft (IDR-001), replicate committed OUTCOMES not invocations, WAL = decision trace, sagas for cross-shard; SHARD-001 immutable partition key; consistency tiers honored | I2 Cluster Kernel, I3 Distributed Query, I4 Capability Scheduling |
| L4 | Multi-Agent | Multi-agent runtime: many agents coordinate over shared committed truth without conflict; ORCH-003/004 under concurrency; recovery/replay axis passes | I5 Multi-Agent Runtime |
| Certified Product | Product-grade | Enterprise-readiness met; real product built on ARVES without modifying the standard; all lower levels held | I6 Reference Products |

Kernel-never-Control-Plane rule: at every level, the Kernel commits truth but never becomes the Control Plane. A submission where the Kernel orchestrates, plans, or executes FAILS regardless of other results.

# Part 9 - The Independent Architecture Review Process

The Independent Architecture Review is the human, adversarial counterpart to the mechanical conformance suite. The reviewer evaluates the submission as if a competing company had built and submitted it - no benefit of the doubt, no reliance on shared authorship with the reference implementation.

Each dimension receives an independent verdict of PASS, PARTIAL, or FAIL.

| Review Dimension | Question Asked | FAIL Trigger |
| --- | --- | --- |
| Layering | Are dependencies downward-only with no lateral coupling (LAYER-001)? | Any lateral or upward dependency |
| Ownership | Does every state have exactly one owner (OWN-001)? | Any shared or ownerless state |
| Plane separation | Is Control Plane strictly decide, Data Plane strictly carry? | Kernel acting as Control Plane; CP holding truth (ORCH-001) |
| Truth discipline | Does only the Kernel commit truth; are engine writes proposed effects only? | Engine or Query writing committed truth |
| Orchestration | Plans not persistent state (ORCH-002); replay from trace (ORCH-003); idempotent + content-addressable (ORCH-004)? | Replay by recomputation; non-idempotent invocation |
| Distribution | Does it follow IDR-001..005 (per-shard Raft, replicate outcomes, sagas, no cross-shard atomic commit in v1)? | Cross-shard atomic commit claimed in v1 |
| Consistency | Are linearizable / bounded-staleness / eventual tiers correctly implemented? | Stale read presented as linearizable |
| Failure handling | Partial rollback by NOT committing; saga compensation; cooperative idempotent cancellation (Amendments 005/006)? | Partial truth left committed after failure |
| Ontology fidelity | Do types honor O-001..007 design principles (identity, provenance, versioned+registered)? | Unversioned/unregistered types; derivation used as inheritance |
| Conformance integrity | Do scenarios and probes trace to axes and contract clauses; CCP-GATE respected? | Behaviour claimed without a conformance scenario |

Review procedure:

- Intake - receive submission, runtime identity, and conformance artifact.

- Blind pass - reviewer reconstructs the architecture from artifacts alone, without designer narration.

- Adversarial probing - reviewer attempts to construct a scenario that violates a registered invariant.

- Dimension scoring - assign PASS/PARTIAL/FAIL per dimension with cited evidence.

- Disposition - overall PASS requires all dimensions PASS; any FAIL blocks; PARTIALs are documented as conditions.

- Routing - genuine spec gaps discovered during review are routed to CCP / Amendment / IDR, never patched into the spec.

# Part 10 - Conformance Checklist

Use this checklist to gate a runtime before submitting for level certification. Every item must be demonstrable from the conformance artifact.

- All 12 axes have at least one reference scenario exercised for the target level.

- Every scenario verdict is PASS or an explicitly documented PARTIAL; no unexplained FAIL.

- OWN-001: every state in the trace resolves to exactly one owner.

- ORCH-001: no truth committed outside the Kernel; Control Plane holds no truth.

- ORCH-002: no persistent state produced by the Control Plane; only plans.

- ORCH-003: verdict replayable from recorded decision trace, not recomputation.

- ORCH-004: every engine/capability invocation idempotent and content-addressable.

- LAYER-001: dependency graph is downward-only; cross-cutting via Control Plane / Event Fabric only.

- SHARD-001: partition by tenant/workspace; partition key immutable across the trace.

- Query nodes wrote nothing (READ-ONLY).

- Engine nodes are pure/stateless; all writes appear as proposed effects the Kernel later commits.

- Proposed invariants (G-001/QUERY-001/LCW-001/PERSIST-001/CAP-*/ENG-*) reported as informative only, flagged pending CCP.

- Conformance artifact is complete, immutable, and independently replayable.

- Suite version and spec version recorded and compatible (see Part 14).

# Part 11 - Performance and Benchmark Checklist

Performance is measured as properties over the same recorded traces, never as truth. Benchmarks stress the high-volume streaming, long-running, and distributed axes.

| Metric | Axis Stressed | Property Checked |
| --- | --- | --- |
| Commit latency (leader) | Safety-critical, Distributed | Linearizable commit through shard leader within target |
| Bounded-staleness read | High-volume streaming | Follower reads within declared staleness bound |
| Eventual read convergence | Event-driven | Replica converges within target window |
| Throughput under backpressure | High-volume streaming | No dropped commits; backpressure applied, not silent loss |
| Replay time | Recovery/replay | Trace replays without re-running engines (ORCH-003) |
| Recovery after crash | Recovery/replay | Leader re-election + WAL replay restores truth (IDR-001/004/005) |
| Saga compensation time | Long-running, Safety-critical | Committed effects compensated within target (Amendment-006) |

Benchmark checklist:

- Observability data (metrics/logs/tracing/presence) is treated as AP, never as committed truth.

- Every benchmark run emits its own decision trace so results are replayable.

- Load is partitioned by immutable partition key (SHARD-001); cross-shard work uses sagas, not atomic commit.

- Latency is reported per consistency tier (linearizable / bounded-staleness / eventual), never conflated.

- No benchmark relies on golden-output comparison; all pass/fail is property-based.

# Part 12 - Enterprise-Readiness Checklist

Enterprise readiness is a precondition for the Certified Product attestation. It certifies that a runtime is operable, governable, and safe in production, on top of holding L1-L4.

- Multi-tenant isolation enforced by immutable partition key (SHARD-001); no cross-tenant truth leakage.

- Read consistency tiers exposed and documented (linearizable through leader, bounded-staleness follower, eventual replica).

- Failure model implemented: partial execution rolled back by NOT committing; committed effects compensated saga-style (Amendment-006).

- Cancellation is cooperative and idempotent; no partial truth; priority/preemption via checkpoint -> replay (Amendment-005).

- Cluster membership managed via Raft joint consensus (IDR-003); per-shard leader election (IDR-004).

- Append-only WAL + snapshots for durability and replay (IDR-005); WAL doubles as decision trace.

- Governance/policy constraints enforced at the Control Plane decision boundary, never bypassed by the Data Plane.

- Every type is versioned and registered (O-006); ontology defines meaning, not storage (O-007).

- Observability stack (AP) deployed and separated from truth (CP).

- Upgrade path preserves conformance: new build re-passes the compatible conformance suite before promotion.

- SDK and API surface do not permit clients to write committed truth directly (only through Kernel commit path).

# Part 13 - Independent Runtime A/B Goals

A core certification goal is that ARVES is not a single-vendor artifact. Two independent runtimes - Independent Runtime A and Independent Runtime B - must each pass certification against the same frozen UCS, proving the standard is implementable by more than its authors.

| Runtime | Goal | Success Criterion |
| --- | --- | --- |
| UCI (reference) | Prove the specification is implementable | Holds L1-L4 + Certified Product |
| Independent Runtime A | Prove standard is vendor-independent | Passes certification independently against same spec version |
| Independent Runtime B | Prove interoperability of the standard | Passes certification; produces artifacts replayable by A and UCI verifiers |

A/B parity properties:

- Both runtimes accept the same conformance suite version against the same UCS version.

- Both emit conformance artifacts that are cross-verifiable (a verifier built for one can replay the other via the recorded trace).

- Neither runtime requires modification of the frozen standard to pass; any gap routes through CCP.

- Distributed behaviour of both grounds in IDR-001..005 (per-shard Raft, replicate outcomes, sagas), even if engine internals differ.

# Part 14 - Versioning Conformance Suites Against Spec Versions

A verdict is only meaningful when the suite version and the spec version are compatible. The Specification Era is frozen as of 2026-07-01; the conformance suite evolves under CCP-GATE without ever mutating the frozen spec.

| Artifact | Maturity Lifecycle | Change Instrument |
| --- | --- | --- |
| UCS specification | Draft -> Candidate -> Ratified -> Frozen -> Deprecated -> Superseded | CCP / Amendment / IDR / next major version |
| Conformance suite | Draft -> Candidate -> Ratified -> Frozen | CCP (CCP-GATE: no behaviour ratified without a scenario) |
| Contract (probe binding) | Draft -> Candidate -> Ratified -> Frozen | CCP / Amendment |
| Certification artifact | Immutable once emitted | Re-run produces a new artifact |

Versioning rules:

- A conformance suite declares the exact UCS version it certifies against; certifying across incompatible versions is invalid.

- Adding a scenario or probe is a suite minor version; it never changes the spec.

- Adding or removing an axis requires a new UCS major version (the 12 axes are frozen for v1).

- Promoting a proposed invariant (G-001/QUERY-001/LCW-001/PERSIST-001/CAP-*/ENG-*) to normative requires a ratified CCP; only then may a related expectation escalate from PARTIAL to FAIL.

- Reference Lifecycle context: Conformance (stage) precedes Reference Runtime, Certification, Reference Product, and Reference Ecosystem; certification cannot outrun the lifecycle stage it depends on.

# Part 15 - Certified Ecosystem and Product Goals

The terminal goal of certification is a living ecosystem: multiple certified runtimes, real products, and a marketplace, all built on ARVES without modifying the standard. These are the frozen ecosystem goals.

- Production distributed runtime certified at L3+.

- Complete conformance suite covering all 12 axes.

- Independent Runtime A and Independent Runtime B both pass certification.

- Third-party certification available (arms-length review board, not the reference authors).

- Enterprise runtime meeting the enterprise-readiness checklist and Certified Product bar.

- SDKs that cannot bypass the Kernel commit path.

- Marketplace of certified capabilities and products.

- Cloud offering exposing documented consistency tiers.

- Real products built on ARVES without modifying the standard (I6 Reference Products).

These goals align to the frozen milestones I1 Distributed Runtime, I2 Cluster Kernel, I3 Distributed Query, I4 Capability Scheduling, I5 Multi-Agent Runtime, I6 Reference Products - and to no others.

# Part 16 - Certification Decision Matrix and Sign-Off

The final certification decision composes the mechanical conformance verdict with the Independent Architecture Review verdict. Both must be satisfied for a level to be granted.

| Conformance Result | Architecture Review | Decision |
| --- | --- | --- |
| All target-level scenarios PASS | All dimensions PASS | Level GRANTED |
| Some PARTIAL (documented, non-blocking) | All dimensions PASS | Level GRANTED with documented conditions |
| Any registered-invariant FAIL | Any | Level WITHHELD (implementation defect) |
| All PASS | Any dimension FAIL | Level WITHHELD (architecture defect) |
| Any | Spec gap discovered | Route to CCP; do NOT edit spec; re-certify after resolution |

Sign-off record fields:

- Runtime identity and build; UCS version; conformance suite version.

- Level attested (L1 / L2 / L3 / L4 / Certified Product).

- Conformance artifact reference (immutable, replayable).

- Architecture review dimension scores with cited evidence.

- Documented PARTIAL conditions and their remediation owners.

- Any CCP routed as a result of the review.

*Final Definition  Certification is the independent, replayable proof that an implementation earns the ARVES name - PASS/PARTIAL/FAIL verdicts drawn from invariants and properties, never golden output - and it proves the frozen standard without ever changing it.*

# Reconciliation Note - Canonical Milestone-to-Level Mapping

The milestone-to-certification-level mapping in this volume (Part 8) is the CANONICAL source. Volumes 2, 4 and 5 defer to it; any milestone/level table elsewhere is illustrative and, on conflict, this volume governs.

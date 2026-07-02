> **Rendered from `ARVES_Engineering_Handbook_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engineering Handbook v1.0

STATUS: INFORMATIVE (ECOSYSTEM HANDBOOK) - NON-NORMATIVE

This handbook explains how to contribute to ARVES. It is a practical guide for a new engineer or AI agent joining the ecosystem. It is NOT part of the specification and carries no normative authority. Every rule, invariant, contract, layer boundary and milestone referenced here is owned by the frozen ARVES corpus (UCS + UCI) and by AEOS (the ARVES Engineering Operating System). Where this handbook and the frozen spec appear to disagree, the frozen spec and AEOS win, always. Read this to learn how to move; read the spec to learn what is true.

Ground truth reminder (frozen): the Specification Era froze on 2026-07-01; the Implementation Era is in progress. The chain Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation is never reversed. An implementation proves the spec; it never changes it. Two planes exist: the Control Plane decides and the Data Plane carries; the Kernel never becomes the Control Plane. Layers are downward-only (LAYER-001).

# Part 1 - 30-Minute Onboarding (Understand ARVES Fast)

Goal: in thirty minutes a new engineer or AI understands what ARVES is, where truth lives, and where to look next. Do this in order. Do not read all eighteen volumes first.

- Minutes 0-5: Read the Specification Freeze Record and the Baseline. Learn that UCS is the standard, UCI is the reference implementation, and the spec is frozen.

- Minutes 5-15: Ask AEOS. AEOS is the operating system for engineering ARVES; it is the authoritative entry point for rules, invariants and process. Treat AEOS answers as the map; treat the frozen volumes as the territory.

- Minutes 15-25: Open the Documentation Index (ARVES_00_Documentation_Index). It routes you to the correct volume, atlas or catalog for any topic without reading everything.

- Minutes 25-30: Internalize the two structural facts you will use daily: the six-link chain (Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation) and the downward-only layer stack.

The layer stack (LAYER-001), top to bottom, with Control Plane cross-cutting:

| Layer | Role |
| --- | --- |
| Reality | The world the system observes and acts upon |
| Information Platform | Ingest and normalize reality into information |
| Kernel | Owns TRUTH; commits outcomes; never becomes Control Plane |
| Persistence | Durable state beneath the Kernel's authority |
| LCW | Owns Working Memory; uses-only by Engine and Control Plane |
| Query | Read-only access to committed state |
| Engine | Runs anywhere; commits only through the shard leader |
| Capability | Capability Fabric: registry and bindings |
| Execution | Carries out bound work |
| Control Plane (cross-cutting) | Decides; owns Plan and Engine Graph; not truth |

If you remember only one sentence: the Kernel owns truth, the Control Plane owns decisions, and calls only ever flow downward.

# Part 2 - How to Debug the Runtime

Debugging ARVES means locating a symptom on the plane/layer map before touching code. Truth is CP (consistent, per-shard Raft); observability is AP. Never treat an AP observation as ground truth for a CP question.

- Classify the symptom: is it a TRUTH problem (a committed outcome is wrong) or a DECISION problem (the Control Plane planned badly)? These live in different owners.

- For truth problems, go to the shard. Per IDR-001..005 the Kernel Control Plane is per-shard Raft; the Raft log is the WAL and the decision trace. Read the append-only WAL for the affected shard to see the exact committed outcomes in order.

- Remember only committed OUTCOMES are replicated, not invocations. If an invocation seemed to run but no outcome is in the log, it never committed.

- For cross-shard anomalies, recall there is NO cross-shard atomic commit; work spans shards via sagas. A partial state across shards is expected mid-saga, not a bug by itself.

- For engine placement confusion, remember engines run anywhere but commit only through the shard leader. A stale-leader engine cannot commit; check leader election and membership (joint consensus) first.

- Use observability (AP) to form a hypothesis, then confirm against the WAL (CP). Divergence between a dashboard and the log is a signal, not a contradiction.

Escalate to an IDR only when the debugging reveals a distribution decision that the spec does not yet pin down. Escalate to a CCP only when behaviour itself is wrong.

# Part 3 - How to Run Conformance

Conformance is how ARVES proves an implementation obeys the spec. The hard rule from the Reference Lifecycle (CCP-GATE): no behaviour is ratified without a conformance scenario. Running conformance is therefore the primary way you demonstrate that your change is legitimate.

- Pin the spec version. A conformance result is always stated as a pass against a specific frozen UCS version. An unpinned result is meaningless.

- Select the scenarios that cover the invariants your change touches (for example ORCH-001..004, OWN-001, LAYER-001, SHARD-001).

- Run the scenario suite against your UCI build; each scenario asserts a testable property (isolation held, plan replayable, no truth reversal, outcomes-only replication).

- Record the result as 'UCI vX passes UCS vY scenario set Z'. Attach it to your change.

- If no scenario exists for the behaviour you are adding, you cannot ratify it; you must author the scenario first (see the Scenario Conformance Framework).

# Part 4 - How to Write an IDR

An IDR (Implementation Decision Record) captures a distribution/implementation decision made during the Implementation Era. IDRs implement the frozen spec; they never change it. IDR-001..005 already fixed the Kernel distribution model (per-shard Raft, outcomes-only replication, sagas, joint consensus, append-only WAL).

- State the decision as one sentence, in the imperative (for example: 'Replicate committed outcomes, not invocations').

- Cite the frozen spec clauses and invariants the decision serves; show that the decision is downstream of the spec, never upstream of it.

- Record the alternatives considered and why they were rejected on distribution grounds (consistency, availability, partition behaviour).

- State the consequences: what the decision forbids (for example, no cross-shard atomic commit) and what it enables.

- Confirm the decision preserves plane separation: the Kernel stays the truth owner and never becomes the Control Plane.

If your IDR would require changing a contract or a behaviour, stop: that is a CCP, not an IDR.

# Part 5 - When to Open a CCP

A CCP (Cognitive Change Proposal) is the only instrument that can change the frozen specification. It is the RFC/KEP analogue for ARVES. Because the spec is frozen, opening a CCP is a deliberate, heavyweight act.

Choose the correct change instrument:

| Instrument | Use when |
| --- | --- |
| CCP | You must change or add ratified behaviour, a contract, or an invariant |
| Amendment | You must correct or clarify frozen text without new behaviour |
| IDR | You are deciding how to implement already-frozen behaviour |
| Next-major | The change is large enough to warrant a new major spec version |

- Open a CCP when behaviour is wrong, missing, or must evolve; not when implementation is merely inconvenient.

- Every CCP that adds behaviour MUST arrive with a conformance scenario (CCP-GATE). No scenario, no ratification.

- Proposed invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005) are informative and pending CCP; promoting one to registered is exactly a CCP action.

- Design principles (Ontology O-001..007) are not runtime invariants; do not file a CCP to 'enforce' them as if they were.

# Part 6 - How to Add a New Capability

Capabilities live in the Capability Fabric, which owns the registry and bindings. Adding a capability is a Capability-layer act and must respect downward-only layering.

- Define the capability contract first: inputs, outputs, and the property a conformance scenario will assert.

- Register it in the Capability Fabric registry; the Fabric owns the registry and the bindings, so registration is the single source of truth for existence.

- Bind the capability to its execution without letting it reach upward: a capability may use layers below it but must never call the Control Plane or mutate Kernel truth directly.

- Attach the proposed capability invariants (CAP-001..009 are informative, pending CCP); if the capability introduces new ratified behaviour, file the CCP with its scenario.

- Add a conformance scenario proving the binding behaves; run it (Part 3).

# Part 7 - How to Add a New Engine

Engines run anywhere and commit only through the shard leader. An engine is a Data-Plane worker driven by Control-Plane decisions; it must never assume Control-Plane authority and must never write truth except by committing an outcome through the leader.

- Place the engine correctly on the map: it sits at the Engine layer, uses LCW Working Memory (uses-only), reads via Query (read-only), and commits outcomes through the Kernel shard leader.

- Ensure the engine is stateless with respect to truth: its authoritative result is the committed outcome in the shard WAL, not any local state.

- Handle non-leadership gracefully: if the engine is not talking to the current shard leader, it cannot commit; it must retry against the leader, not force a write.

- Register the engine in the Engine Graph, which the Control Plane owns; the Control Plane owns the Plan and the Engine Graph but never owns truth or persistent state.

- Cover it with proposed engine invariants (ENG-001..005, informative, pending CCP) and a conformance scenario; ratify new behaviour via CCP.

# Part 8 - How to Write a New Connector

A connector bridges Reality and the Information Platform: it turns external systems into normalized information entering ARVES from the top of the stack. A connector is not a back door into the Kernel.

- Enter at the top: connectors feed the Information Platform, which normalizes reality into information; they never write Kernel truth directly.

- Normalize, do not decide: a connector carries and shapes data (Data Plane); decisions belong to the Control Plane. Keep the connector free of planning logic.

- Make ingestion idempotent and replay-safe, since downstream commits are outcomes recorded in an append-only WAL.

- Map the external schema to the canonical ontology (see the Canonical Ontology and Data Catalog) so the rest of the stack sees uniform information.

- Add a conformance scenario for the connector's contract and run it before proposing the connector for use.

# Part 9 - How to Analyze a Distributed Failure

Distributed failures in ARVES are analyzed through the lens of IDR-001..005 and the CP/AP split: truth is CP, observability is AP. Most 'impossible' failures are actually expected consequences of the frozen distribution model.

- Start at the shard boundary: identify which shard(s) the failure touches. Each shard has its own Raft group, its own leader election, and its own append-only WAL.

- Distinguish partition from corruption: under partition a minority shard cannot elect a leader and cannot commit; that is correct refusal, not data loss.

- For cross-shard inconsistency, look for an in-flight or failed saga; there is no cross-shard atomic commit, so compensations are the mechanism, not two-phase locking.

- Rebuild the timeline from the WAL: because the Raft log is the WAL and the decision trace, the ordered committed outcomes are the authoritative history of what happened.

- Check membership: joint consensus during reconfiguration can explain transient leadership and commit behaviour; confirm the membership state before blaming application logic.

- Only after the CP timeline is clear, correlate AP observability to explain latency or visibility gaps; never let an AP metric override the WAL.

Output of the analysis is either an IDR (a new distribution decision within the frozen spec) or, if behaviour itself is wrong, a CCP with a conformance scenario.

# Part 10 - Non-Normative Standing and Deference

This handbook is informative only. It describes practice; it does not define ARVES. It must not be cited as authority in a CCP, an IDR, a conformance result, or an amendment. For every rule it defers, in this order, to: the frozen UCS/UCI specification, the registered invariants (ORCH-001..004, OWN-001, LAYER-001, SHARD-001), the ratified instruments (CCP, Amendment, IDR), and AEOS as the engineering operating system and routing authority. If this document ever drifts from those sources, those sources are correct and this document is stale.

*Final Definition  ARVES Engineering Handbook = the non-normative, practical on-ramp for contributing to ARVES, which explains how to move within the ecosystem while deferring every rule to the frozen specification and AEOS.*

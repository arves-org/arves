> **Rendered from `ARVES_Engine_Graph_Specification_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engine Graph Specification v1.0

STATUS: EXECUTION STANDARD (ENGINE ABI + GRAPH MODEL)

# Part 1 - Purpose

Define the Engine ABI: how an engine is described so that ANY conformant ARVES runtime can execute it, without knowing the engine internals. The question is not "how do engines flow" but "how is an engine defined for portable execution". Precedent: OCI Image Specification and Kubernetes Pod Spec.

# Part 2 - Position & Normative Dependencies

This standard consumes the Cognitive Control Plane invariants (Vol 9, ORCH-001..004) and is VALIDATED BY the Scenario Conformance Framework (its reference scenarios are the acceptance criteria). It has one hard normative dependency:

- Universal Cognitive Ontology (Type Registry) - the type system for Reads/Produces/Writes. An ABI is only as well-defined as its types; this registry is a prerequisite and must be frozen for engine manifests to be interoperable.

# Part 3 - The Engine Node Contract (the ABI)

Every engine node declares a manifest with the following normative fields.

| Field | Meaning | Group |
| --- | --- | --- |
| Name | Stable engine identifier | Identity |
| Version | Semantic version of the engine contract | Identity |
| Inputs | Declared input parameters | Type contract |
| Preconditions | Conditions that must hold before invocation | Type contract |
| Reads | Ontology types consumed from state (Kernel/LCW/Query) | Type contract |
| Writes | PROPOSED state effects to be committed by Kernel (never direct truth) | Type contract |
| Produces | Inference/ontology artifacts emitted as output | Type contract |
| Capabilities Required | Capabilities the runtime must bind | Type contract |
| Determinism | Deterministic \| Seeded \| Nondeterministic | Execution |
| Idempotency Key | Content-addressable key for safe retry (ORCH-004) | Execution |
| Failure Policy | Behaviour on failure (fail, degrade, escalate) | Execution |
| Retry Policy | Retry count, backoff, recovery | Execution |
| Timeout | Max execution bound | Execution |
| Confidence | Declared/estimated output confidence | Planning metadata |
| Cost | Declared/estimated cost (token/compute) | Planning metadata |
| Latency | Declared/estimated latency | Planning metadata |

# Part 4 - Engine Purity (consistency with Vol 9)

An engine is Data Plane pure compute. It READS state and PRODUCES inference; its WRITES are PROPOSED effects that only the Kernel may commit as truth. No engine mutates cognitive truth directly (ORCH-001). This keeps engines reusable and side-effect-honest.

# Part 5 - Engine Graph Model

A goal produces a non-linear (DAG) Engine Graph of engine nodes. Edges are data/ordering dependencies. The graph is dynamically expanded (engines may emit sub-goals) under a bounded termination policy (Vol 9 Part 6-7). Join nodes perform arbitration; arbitration output is a plan artifact, never truth.

# Part 6 - Determinism & Replay

The Determinism field drives replay semantics. Deterministic/Seeded engines can be recomputed; Nondeterministic engines are REPLAYED from the recorded decision trace, not recomputed (ORCH-003). The Runtime Fingerprint (engine versions, model routing, bindings) is captured per run.

# Part 7 - Failure, Retry & Idempotency

Failure Policy and Retry Policy are declared per node. Because every invocation carries an Idempotency Key and is content-addressable (ORCH-004), retries and future distribution are safe by construction.

# Part 8 - Planning Metadata

Confidence, Cost and Latency are machine-readable, not prose. The Control Plane uses them for engine selection, scheduling and arbitration (e.g. confidence-weighted merge at join nodes).

# Part 9 - Engine Manifest (serialized form)

The manifest is the portable, serializable descriptor - the analogue of an OCI image manifest. A runtime needs only the manifest (not the source) to schedule and execute a node. Manifests are content-addressable and versioned.

# Part 10 - Runtime Contract (portability guarantee)

A conformant runtime executing an engine node MUST:

- Verify Preconditions before invocation and provide the declared Reads.

- Capture Produces and route proposed Writes to the Kernel for commitment (never commit truth inside the engine).

- Enforce Idempotency Key, Retry Policy, Failure Policy and Timeout.

- Record the invocation into the decision trace with correlation_id and Runtime Fingerprint.

- Honour Capabilities Required via the Capability Fabric.

# Part 11 - Versioning & Compatibility

Engine contracts are semantically versioned. Backward-compatible changes bump minor; breaking changes bump major and require a new manifest. A graph pins engine versions so runs remain replayable and conformance results stay meaningful across spec versions.

# Part 12 - Conformance

This specification is accepted only when the Scenario Conformance Framework reference scenarios (e.g. Warehouse Robot Dispatch: safety gate blocks unsafe plan, Engine Graph produced, execution idempotent) PASS against a runtime executing manifests defined here. Scenarios are the acceptance criteria; the spec is accountable to them.

# Part 13 - Independent-Implementability Test

Acceptance bar: an engineer outside the ARVES team, given only this specification and the Ontology Type Registry, can build a runtime that executes any conformant engine manifest and produces conformant behaviour. If not, the document is still descriptive and must be made more normative.

# Part 14 - Open Dependency Callout

The Reads/Writes/Produces type vocabulary is UNRESOLVED until the Universal Cognitive Ontology Specification is written and frozen. Until then, this ABI references ontology types by name only. Recommended immediate next document: Universal Cognitive Ontology Specification.

# Part 15 - Success Criteria

An engine becomes a portable, versioned, replayable, side-effect-honest unit that any conformant runtime can execute from its manifest alone.

*Final Definition  Engine Graph Specification = The Portable Execution ABI of ARVES Cognition.*

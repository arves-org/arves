> **Rendered from `ARVES_Volume_9_Cognitive_Control_Plane_v2.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Volume-9: Cognitive Control Plane v2.0

STATUS: SYSTEM CONSTITUTION (CONTROL PLANE) - SUPERSEDES Runtime & Event Fabric v1 for cognitive orchestration

# Part 1 - Purpose

Define the control plane that sits between thinking (engines) and action (execution). The Cognitive Control Plane owns coordination and planning of cognition; it never owns cognitive truth. If this layer is correct, distributed runtime, multi-agent, certification, products and marketplace all rest naturally on top of it.

# Part 2 - The Two-Plane Model

ARVES is read on two axes, not one stack. The Data Plane carries information and state; the Control Plane decides. The Kernel never becomes the Control Plane.

Control Plane (decides): Goal Management, Cognitive Orchestration, Engine Coordination, Capability Selection, Execution Planning, Policy Coordination, Human Collaboration, Observation Feedback.

Data Plane (carries): Reality, Information Platform, Kernel, Living Cognitive World, Persistence, Query, Engine Fabric (engines as pure compute), Capability Fabric, Execution, mechanical Runtime (event bus, task, workflow).

# Part 3 - Engine and Capability Placement (critical rule)

The engine ITSELF (Engine Fabric, M7) is Data Plane: a pure, idempotent function that reads state and produces inference. The Engine Graph (which engines run, in what order) is Control Plane. Likewise Capability Fabric is Data Plane; Capability Planner is Control Plane. Engines never own their own orchestration - otherwise reusability is lost.

# Part 4 - Control Plane Components

| Component | Responsibility |
| --- | --- |
| Goal Manager | Receives, decomposes and tracks goals; owns goal lifecycle. |
| Task Graph | Mechanical decomposition of a goal into schedulable tasks. |
| Engine Graph | Non-linear graph of engine invocations produced to satisfy a goal. |
| Capability Planner | Selects and binds capabilities required by the plan. |
| Execution Planner | Turns the resolved plan into an executable execution plan. |
| Policy Coordination | Enforces policy decisions owned by Governance; does not own policy. |
| Human Collaboration | Approval, hand-off and human-in-the-loop checkpoints. |
| Observation Feedback | Feeds outcomes back to Goal Manager for adaptation and replan. |

# Part 5 - Control Plane Invariants

These invariants are the constitutional core and the foundation of the future distributed runtime.

| ID | Invariant | Rationale |
| --- | --- | --- |
| ORCH-001 | The Control Plane owns no truth. Only the Kernel owns cognitive truth. | Prevents a second state owner; keeps Kernel authoritative. |
| ORCH-002 | The Control Plane produces plans, never persistent state. | Makes the Control Plane restartable and stateless over Kernel/Persistence. |
| ORCH-003 | Every execution is REPLAYABLE from the same Goal, State, Policies, Capabilities and Runtime Fingerprint - via a recorded decision trace, not by recomputation. | Cognitive engines (LLMs) are non-deterministic; reproducibility means deterministic replay from recorded outcomes, not identical re-computation. |
| ORCH-004 | Every engine and capability invocation is idempotent and content-addressable. | Enables safe retry and, later, distribution; the true bridge to M9. |

# Part 6 - Engine Graph Model

The engine flow is no longer linear. A goal produces a non-linear (DAG) Engine Graph.

Example: Goal -> Engine Graph -> { Planning || Simulation || Search } -> Evaluation (join/arbitration) -> Decision.

- Nodes are engine invocations (pure, idempotent - see ORCH-004). Edges are data/ordering dependencies.

- Join nodes perform ARBITRATION: they merge conflicting branch outputs by policy (confidence-weighting, tie-break). Arbitration consumes truth but its output is a PLAN artifact, never truth (ORCH-001).

- The graph is DYNAMICALLY EXPANDED, not fully static: engines may emit sub-goals that grow the graph (continuation style).

- Expansion is bounded by a termination policy (max depth / budget / no-new-subgoal) to prevent infinite meta-planning.

# Part 7 - The Planning Recursion (design resolution)

Planning is both a node IN the graph and the PRODUCER of the graph. Resolution: a bootstrap meta-planning step produces an initial graph; domain engines execute within it and may trigger bounded re-planning. The Orchestrator expands the graph incrementally as engines return, under the Part 6 termination policy.

# Part 8 - Control Plane Execution Flow

Goal -> Orchestrator -> Engine Graph -> Capability Graph -> Execution Plan -> Execution (Data Plane) -> Observation Feedback -> Goal Manager.

# Part 9 - Reproducibility & Replay

Each run records a decision trace: the expanded Engine Graph, engine outputs, arbitration choices, policy evaluations and the Runtime Fingerprint (engine versions, model routing, capability bindings, policy set). Replay re-reads recorded outcomes deterministically (ORCH-003). Recomputation is explicitly NOT guaranteed for non-deterministic engines.

# Part 10 - Policy, Approval and Human-in-the-Loop

Policy and governance decisions are owned by Security & Governance (Vol 17) and Agent Governance (Vol 14 / Vol 2). The Control Plane ENFORCES and sequences them (approval gates, HITL checkpoints) but never becomes their owner - single-ownership per Vol 1.

# Part 11 - Distribution Readiness (M8 constraints for M9)

M8 is built distribution-ready even though M9 performs the actual distribution.

- Plans are serializable and location-transparent (no in-process shared-memory assumptions).

- Engine invocations are addressable, idempotent and carry correlation_id (already in the Event Envelope).

- The Control Plane is stateless over Kernel/Persistence (ORCH-002), so it can be replicated.

- What is distributed later is the Orchestrator/plan, not the Engine.

# Part 12 - Relationship to Existing Volumes (reclassification)

The mechanical Runtime & Event Fabric described in Vol 9 v1 (event bus, task, workflow, scheduler, retry) is RECLASSIFIED as Data Plane mechanical runtime and is NOT removed. This volume (v2) elevates the cognitive control concerns on top of it. Vol 1 principles (Single Ownership, Explicit Contracts, Observability) govern.

# Part 13 - Open Design Decisions

| Decision | Options | Recommendation |
| --- | --- | --- |
| Graph shape | Static plan vs dynamically-expanded graph | Dynamic, bounded (Part 6-7) |
| Reproducibility | Recompute vs replay-from-trace | Replay-from-trace (ORCH-003) |
| Vol 9 identity | Overwrite v1 vs new v2 preserving v1 | New v2; v1 reclassified (Part 12) |
| Arbitration owner | Engine vs Control Plane | Control Plane join node (Part 6) |

# Part 14 - Conformance Hooks

Every run emits a conformance artifact (expanded Engine Graph, engine order, arbitration, policy gates). This is the input to the Scenario Conformance Framework (12 axes) and doubles as the regression suite that earns the M1-M7 completion marks.

# Part 15 - Success Criteria

Thinking is coordinated into action through a stateless, replayable, distribution-ready control plane that owns no truth.

*Final Definition  Cognitive Control Plane = The Reasoning-to-Action Control Layer of ARVES.*

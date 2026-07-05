# I5 — Multi-Agent Runtime — Engineering Design Package

```
=====================================================================
 STATUS: DESIGN PACKAGE (Ch4 PREP MODE) — NO CODE
 Build gate (G2) CLOSED — I2..I6 implementation remains gated.
 Prepared 2026-07-05 under maintainer prep-mode ruling.
=====================================================================
```

**Milestone:** I5 Multi-Agent Runtime — *"Agent spawning, delegation, coordination at
scale"* (frozen `ARVES_00_Baseline_v1`, Part 5).
**One-line intent:** many agents reasoning over **ONE shared truth base** — agent
identity, shared-truth concurrency, decision/compliance truth flows, cross-agent
consistency, and the Control Plane's ORCH-001..004 enforced at scale.
**Predecessors (required, not yet built):** I2 Cluster Kernel · I3 Distributed Query ·
I4 Capability Scheduling (Baseline Part 5 ordering). This package designs against
their *frozen decisions* (IDR-001..005), not against any assumed implementation.
**Exercised reference semantics (G1):** `products/arves-enterprise-os/src/enterprise-os.mjs`
— the P5 product that already demonstrates, on the frozen Runtime v1.0, multi-agent
shared truth, policy-as-truth, a compliance ledger, and cross-department decision
conflict detection. I5 is the runtime-grade generalization of exactly those semantics.
*G1 scope caveat (per the product's own header):* in G1 the policy engine, dedup and
conflict check run over IN-MEMORY per-process maps that are NOT read back from the
Kernel and are lost on restart — only the commits are truth; committing checks against
recovered Kernel truth is precisely what I5 elevates to runtime grade (OQ-3).
**Frozen surfaces touched:** NONE. `runtime/`, `standard/`, `spec-markdown/`, `corpus/`
are not modified by this package; every implementation-era change they would need is
enumerated in §6 as an RCR/CCP instrument.

---

## 1. Scope and Definitions

I5 is **not** a new architectural layer. Per the frozen corpus, an "agent" is composed
entirely of already-owned constructs:

| Agent element | Frozen owner | Source |
| --- | --- | --- |
| Agent identity | Tenant/Identity model — Agent is a first-class core entity | Vol 2 Tenant & Identity Constitution v2, Parts 4 & 8; Vol 14 Agent Architecture Atlas, Part 5 |
| Agent working memory (live) | **LCW** (agents *use* Working Memory; LCW owns it) | Amendments CCP Batch 1, A-001; Layer Matrix (A-003, LCW row) |
| Agent goals / plans | **Control Plane** (Goal Manager owns goal lifecycle; Plan/Engine Graph is a Control Plane artifact) | Vol 9 CCP v2, Part 4; Amendment-002 |
| Agent decisions-as-truth | **Kernel** (sole owner of cognitive truth) | Vol 9 CCP v2, Part 5 (ORCH-001); Layer Matrix (Kernel row) |
| Agent long-term/strategic memory (committed) | Kernel (truth) + Persistence (durable record) | Layer Matrix (A-003); Vol 14 Part 6 lists the memory types, ownership resolved by A-001/A-003 |
| Agent capabilities | Capability Fabric (registry/bindings); selection is Control Plane | Layer Matrix (Capability row); Vol 9 Part 3 |
| Agent runtime state (Idle/Running/…) | Execution layer in-flight state | Vol 14 Part 11; Layer Matrix (Execution row) |
| Agent governance (owner, policies, budgets, audit) | Governance (Vol 2/Vol 17/Vol 14); Control Plane *enforces*, never owns | Vol 2 Part 23; Vol 9 Part 10 |

**Definition used throughout:** *A multi-agent runtime is N concurrently-planning
Control Plane goal contexts, each bound to a governed Agent identity (Vol 2 Part 8),
all of whose state effects become truth only through the one Kernel commit gateway of
their shard.* This is a composition of frozen constructs, not an invention (RULE 2/3).

Multi-agent roles in scope: Coordinator, Worker, Specialist, Supervisor patterns
(Vol 14 Part 13; ARVES-23 "Multi-Agent Roles"). Delegation: agents create and delegate
work to sub-agents (Vol 14 Part 12) — realized as bounded dynamic Engine-Graph
expansion (Vol 9 Parts 6–7), never as a second orchestration authority.

---

## 2. BEFORE-WRITING-CODE — the ten constitutional answers

**Q1 — Which UCI node is affected?**
Primarily the **Control Plane** node (Vol 9 CCP v2) — multi-agent coordination is,
by Part 2's two-plane model, a *Control Plane* concern (Goal Management, Cognitive
Orchestration, Human Collaboration). Secondarily, as **consumers under existing
contracts**: Kernel (commit gateway), LCW (working-memory views per agent), Query
(tenant-scoped reads), Engine Fabric (pure invocations), Capability Fabric (bindings),
Execution (idempotent actions). In the runtime workspace this maps to
`arves-control-plane` and `arves-lcw` — both **CONTRACT-ONLY** today per
`runtime/RUNTIME_FREEZE_v1.0.md` (v1.1 backlog item #4 / RCR-001 status list). No new
layer is created (Non-Negotiable Rule 3); the Scenario Conformance Framework Part 7
node pipeline is unchanged.

**Q2 — Which documents govern it?**
- `ARVES_00_Baseline_v1` Part 5 — I5 scope ("Agent spawning, delegation, coordination at scale").
- `ARVES_Volume_9_Cognitive_Control_Plane_v2` — Parts 2–11; Part 5 defines ORCH-001..004, the ONLY registered orchestration invariants.
- `ARVES_Volume_14_Agent_Architecture_Atlas_v1` — agent model, lifecycle, delegation, multi-agent patterns, governance.
- `ARVES_Volume_2_Tenant_Identity_Constitution_v2` — Agent as core entity (Parts 4/8), tenant/workspace isolation (Part 21), agent governance (Part 23), audit (Part 20), roles (Part 13).
- `ARVES_23_Agent_Catalog_v1` — canonical agent types, definition template, lifecycle.
- `ARVES_00_Amendments_CCP_Batch_1_v1` — A-001 (Working Memory → LCW), A-002 (Plan → Control Plane), A-003 (LAYER-001 + Layer Matrix), A-004 (SHARD-001 + shard identity), A-005 (cancellation/priority), A-006 (failure taxonomy/compensation).
- `ARVES_IDR_Batch_1_Kernel_Distribution_v1` — IDR-001..005 (binding).
- `ARVES_Scenario_Conformance_Framework_v1` — Axis 9 (Multi-agent Coordination), Level L4, Part 7 node probes, Part 8 verdict semantics.
- `ARVES_00_Invariant_Registry_v1` — registered vs proposed invariant standing.
- `runtime/RUNTIME_FREEZE_v1.0.md` — frozen Runtime v1.0 stability contract; RCR process; contract-only status of `arves-control-plane`/`arves-lcw`.
- `ARVES_Reference_Lifecycle_v1` Part 6 — CCP process + CCP-GATE (no behaviour ratified without a conformance scenario).

**Q3 — Which contracts apply?**
- The **Kernel commit contract**: sole commit gateway, idempotent, content-addressed (RUNTIME_FREEZE "Truth" guarantee: OWN-001 + ORCH-004; plus RCR-005 content-integrity rejection).
- The **Runtime API** products bind to: SDK (content addressing + canonical encode) and Bridge (`commit`/`invoke` line protocol), per RUNTIME_FREEZE "The Runtime API".
- The **Layer Responsibility Matrix** (Amendment-003) — each layer's Owns/Reads/Writes/Cannot columns are the inter-node contracts I5 composes.
- The **contract-only crate interfaces** of `arves-control-plane` and `arves-lcw` (frozen v1.0 surfaces; extending them is an RCR).
- The **Event Envelope** routing contract: routing key = `tenant_id + correlation_id` (Amendment-004); engine/capability invocations carry `correlation_id` (Vol 9 Part 11).
- The **Engine ABI** (plan-proposal production, cancellation semantics + priority field per Amendment-005).

**Q4 — Which invariants apply?**
Registered (normative, enforceable): **OWN-001, LAYER-001, SHARD-001, ORCH-001,
ORCH-002, ORCH-003, ORCH-004** — all seven bite on I5; the full mapping with the
executable proof each needs is §4. Proposed (informative, cited only as design intent,
each marked): LCW-001, G-001, QUERY-001, CAP-002/003/007/008, ENG-001..004 — all
**(PROPOSED — CCP-GATE required)**; per `ARVES_OS_Volume_2_UltraCode_Workflow_v1`
(PROPOSED-invariant rule) they are never presented as registered and the pending-CCP
dependency is recorded in §4 (PROPOSED block), §3.22 and §6.2.

**Q5 — Which ownership rules apply?**
Amendment-003's matrix, applied to agents: an agent's *live* reasoning state is LCW's
(A-001); an agent's *plan* is the Control Plane's (A-002); an agent's *committed
decision* is the Kernel's (ORCH-001); an agent's *durable trace* is Persistence's; an
agent's *identity/governance* is the Tenant/Identity model's (Vol 2 Parts 8/17/23 —
"Every … agent must have a defined owner"). No component in I5 acquires a second copy
of any of these (OWN-001). The multi-agent coordinator is the existing Control Plane —
not a new "agent manager" owner.

**Q6 — Which IDRs apply?**
All of IDR-001..005, unmodified:
- **IDR-001 (CP kernel, per-shard Raft):** all agents of one tenant/workspace commit through that shard's single Raft leader — this *is* the shared-truth serialization point. Agent compute runs anywhere (engines are pure); only commits serialize.
- **IDR-002 (leader→followers, snapshots, WAL):** followers apply committed agent-decision outcomes; they never re-run agents.
- **IDR-003 (joint consensus membership):** agent load growth changes cluster membership only via joint consensus.
- **IDR-004 (per-shard leader election):** on leader loss, in-flight uncommitted agent work is discarded — no partial truth (with Amendments A-005/A-006).
- **IDR-005 (append-only WAL, deterministic replay):** the WAL is the multi-agent decision trace; replay of an N-agent run is replay of the one committed log (ORCH-003).
- **IDR-006** (Product Program): products remain customers of the frozen runtime; the enterprise-os reference semantics inform I5 but no product edit substitutes for the runtime milestone.

**Q7 — Does this create architectural drift?**
No, by construction: (a) no new layer — agents decompose onto the existing ten
Layer-Matrix rows (§1); (b) no lateral coupling — agent-to-agent communication is
"events, commands and messages" (Vol 14 Part 14) traversing the Control Plane / Event
Fabric, exactly the LAYER-001 cross-cutting channel; (c) no second truth owner —
coordination artifacts (assignments, delegations, arbitrations) are plan artifacts
(ORCH-001/002, Vol 9 Part 6 "arbitration output is a PLAN artifact, never truth");
(d) the conformance pipeline (SCF Part 7) is unchanged. Residual drift risks are
listed honestly in §7.21.

**Q8 — Does this require CCP / Amendment / a new IDR?**
Yes — three classes, all enumerated in §6: (i) **IDR Batch (new)** for multi-agent
*mechanisms* the frozen spec deliberately left to the Implementation Era (scheduler
policy, agent-registry storage layout, budget accounting) — mechanisms belong in IDRs
per Amendment-006's "mechanisms → IDR" precedent; (ii) **CCP(s)** to register the
decision/compliance ontology types exercised by G1 (`uci.fact`, `uci.policy`,
`uci.approval`, `uci.decision`, `uci.compliance`) as versioned ontology types (O-006:
"Every type is versioned and registered" — today they exist only as product-level
payloads); and, if any PROPOSED invariant (e.g. LCW-001) is to be *enforced* by I5
gates, its ratifying CCP with a conformance scenario (CCP-GATE, Reference Lifecycle
Part 6); (iii) **RCRs** for every runtime-crate change (contract-only
`arves-control-plane`/`arves-lcw` → implemented), since Runtime v1.0 is frozen and
"only a ratified RCR changes the runtime" (RUNTIME_FREEZE).

**Q9 — Can another independent implementation reproduce this behaviour?**
Yes, provided §6's instruments land first: the behaviour is defined entirely by
(a) frozen invariants ORCH-001..004/OWN-001/LAYER-001/SHARD-001, (b) the IDR-001..005
distribution decisions, (c) the content-addressing/commit contract already certified
across two independent runtimes (Rust + Python, per FOUNDATION program noted in
CLAUDE.md), and (d) the L4 conformance scenarios of §5. Nothing in this design depends
on private state of the reference implementation; conformance is structural/property/
invariant-based, not golden-output (SCF Part 8), which is what makes non-deterministic
multi-agent runs independently certifiable at all.

**Q10 — Would this implementation still pass conformance five years from now?**
The design binds only to frozen, versioned surfaces: the v1.0 Baseline scope, ORCH
invariants, IDR decisions, and a versioned conformance suite ("N% at Level Lx against
Framework vA / Spec vB", SCF Part 11). Replay is from recorded decision traces, not
recomputation (ORCH-003), so model/engine drift over five years does not invalidate
recorded conformance artifacts. The Runtime Fingerprint (Vol 9 Part 9) pins engine
versions, model routing, capability bindings and policy set per run. Risk that would
break this — unversioned agent-type definitions — is closed by requiring the agent
registry to be versioned (Vol 14 Part 20: "definitions, versions") and by the O-006
CCP in §6.

---

## 3. ENGINEERING DESIGN

> Prep-mode note: every subsection is design-only. Where the frozen corpus fixes the
> answer, it is cited. Where it does not, the item appears in §3.22 Open Questions —
> never as a silent assumption.

### 3.1 Responsibilities

I5 makes the following true, and nothing more:

1. **Agent identity & registry** — every agent is a governed, tenant-scoped identity
   (Vol 2 Parts 8/23) with a versioned definition (Vol 14 Part 20; ARVES-23 template:
   Name/Type/Owner/Purpose/Capabilities/Goals/Memory/Tools/Events/Policies/Dependencies).
   Spawning an agent = registering + activating a definition (lifecycle: Registered →
   Activated → Assigned → Running → Paused → Completed → Archived, ARVES-23).
2. **Shared-truth concurrency** — N agents concurrently propose writes; exactly one
   content-addressed truth base per shard results (ORCH-004 + IDR-001). Duplicate
   proposals converge (idempotent commit); conflicting proposals are detected against
   committed truth and recorded as compliance events, not silently overwritten
   (G1 reference: `enterprise-os.mjs` `proposeDecision` conflict path — noting that
   in G1 this check runs over in-process maps per the product's own header caveat,
   not against recovered Kernel truth; the runtime-grade check against committed
   truth is exactly the I5 target, OQ-3).
3. **Decision/compliance truth flows** — fact, policy, approval, decision and
   compliance-event flows in which policy checks read *committed* policy truths and
   approvals are *separate committed truths* (proposer ≠ approver), generalizing the
   G1 E1-hardened flow to runtime grade.
4. **Delegation & coordination** — Coordinator/Worker/Specialist/Supervisor patterns
   (Vol 14 Part 13) realized as bounded dynamic Engine-Graph expansion with join-node
   arbitration owned by the Control Plane (Vol 9 Parts 6–7); sub-goal emission = 
   delegation (Vol 14 Part 12).
5. **ORCH-001..004 at scale** — the four control-plane invariants hold not per-run
   but across N concurrent agent runs sharing one truth base, with executable proofs
   (§4).
6. **Governance enforcement** — budgets, permissions, policies, risk limits per agent
   (Vol 14 Parts 16/18; Vol 2 Part 23) *enforced* by the Control Plane, *owned* by
   Governance (Vol 9 Part 10 — single-ownership preserved).

Out of responsibility: everything in §6 NON-GOALS.

### 3.2 Inputs

- **Goals** submitted to agents (Vol 9 Part 8 flow entry), carrying tenant/workspace
  context (SHARD-001 partition key, Amendment-004).
- **Agent definitions** from the versioned agent registry (Vol 14 Part 20).
- **Committed truth** read via Query at a declared consistency tier (IDR-001 read
  tiers: linearizable / bounded-staleness / eventual).
- **Policies** as committed truths + the Governance policy set (Vol 9 Part 10).
- **Events/commands/messages** between agents via the Event Fabric (Vol 14 Part 14;
  Vol 9 Part 12 mechanical runtime), enveloped with `tenant_id + correlation_id`
  (Amendment-004).
- **Human approvals** at HITL checkpoints (Vol 9 Part 10), as separate committed
  approval truths (G1 E1 semantics).

### 3.3 Outputs

- **Plan artifacts**: expanded Engine Graphs, task assignments, delegation records,
  arbitration choices — Control Plane-owned, never truth (ORCH-001/002; A-002).
- **Proposed writes → Kernel commits**: agent decisions, compliance events, approvals
  — the only path by which agent work becomes state (ORCH-001; Layer Matrix).
- **Decision traces**: per-run recorded traces (expanded graph, engine outputs,
  arbitration, policy evaluations, Runtime Fingerprint) enabling replay (Vol 9 Part 9).
- **Conformance artifacts** per run (Vol 9 Part 14; SCF Part 9) — at L4, including
  multi-agent evidence: which agent proposed, which policy gates fired, which
  conflicts were detected.
- **Observability** (AP plane): per-agent tasks, actions, failures, costs, latency
  (Vol 14 Part 17) — explicitly eventually-consistent per IDR-001 non-goals.

### 3.4 Dependencies

Downward only (LAYER-001): Kernel (I2 cluster kernel: per-shard Raft commit),
Query (I3: routed, tenant-scoped reads), Capability scheduling (I4: cluster-wide
binding), Persistence (WAL/snapshots, IDR-005), LCW (working-memory views), Engine
Fabric (pure invocations), Event Fabric (mechanical runtime, Vol 9 Part 12).
I5 depends on I2/I3/I4 landing first (Baseline Part 5 ordering); none of them exist
yet — this package therefore binds to their *decisions* (IDRs), and §3.22 records the
resulting unknowns. No dependency on any product; `enterprise-os.mjs` is cited as
evidence of exercised semantics, not consumed as a component (IDR-006 direction of
dependency: products depend on runtime, never the reverse).

### 3.5 Lifecycle

Agent lifecycle (frozen): Created → Activated → Assigned → Executing → Paused →
Resumed → Completed → Archived (Vol 14 Part 4); registry view: Registered → …
→ Archived (ARVES-23). Design mapping:

| Transition | Mechanism | Constraint |
| --- | --- | --- |
| Create/Register | Agent definition committed as versioned registry state | Owner mandatory (Vol 2 Part 17); tenant-scoped (SHARD-001) |
| Activate | Control Plane admits the agent's goal context | Policy/budget gate first (Vol 9 Part 10; Vol 14 Part 18) |
| Assign/Execute | Goal → Engine Graph per Vol 9 Part 8 | Graph expansion bounded (Vol 9 Part 6 termination policy) |
| Pause/Resume | Preemption checkpoint to decision trace | Replayable (Amendment-005: preempted work checkpointed, ORCH-003) |
| Cancel | Cooperative, idempotent; cancellation event emitted | No partial truth: uncommitted proposals discarded (Amendment-005) |
| Complete/Archive | Outcome committed; trace sealed; agent context released | Control Plane retains nothing persistent (ORCH-002) |

Delegation lifecycle: a parent agent's engine emits a sub-goal → the Orchestrator
expands the graph (continuation style) and may bind the sub-goal to a child agent
context; expansion is bounded by max depth / budget / no-new-subgoal (Vol 9 Parts 6–7).
Delegated authority can never exceed the parent's governed permissions (Vol 2
Parts 13–15 policy model — "who can do what on which resource under which conditions").

### 3.6 State Model

Four distinct state classes; each has exactly one owner (OWN-001):

1. **Committed truth** (facts, policies, approvals, decisions, compliance events) —
   Kernel-owned, content-addressed, per-shard (Layer Matrix; ORCH-004; SHARD-001).
2. **Agent working memory** — LCW-owned mutable live state, *never truth, never
   authoritative* (Amendment-001; LCW Matrix row; **LCW-001 (PROPOSED — CCP-GATE
   required)** states this as an invariant but is not yet normative).
3. **Plan state** (goal contexts, engine graphs, assignments, delegation tree) —
   Control Plane-owned, non-persistent, reconstructible (ORCH-002); serializable and
   location-transparent (Vol 9 Part 11).
4. **In-flight execution state** — Execution-layer owned (Layer Matrix Execution row);
   Idle/Running/Waiting/Paused/Failed/Completed per Vol 14 Part 11.

Agent memory types (Vol 14 Part 6): Working Memory → class 2; Session Memory →
class 2 or class 1 once committed; Long-Term and Strategic Memory → class 1 only
(truth via Kernel commit), durably recorded by Persistence. No fifth state class is
introduced.

### 3.7 Distributed Behaviour

- **One serialization point per shard:** all agents of a tenant/workspace commit
  through that shard's Raft leader (IDR-001). Shared-truth concurrency is therefore
  *linearizable at the commit gateway* — the design's central simplification: agent
  count scales compute (engines run anywhere, pure), not consensus width.
- **Replicate outcomes, never agent runs:** followers apply committed agent decisions;
  they never re-execute agents (IDR-002; ORCH-003 rationale — LLM-backed agents are
  non-deterministic).
- **Cross-shard (cross-tenant/workspace) coordination:** no atomic commit in v1
  (IDR-001 refinements); a multi-agent workflow spanning shards is a saga with
  explicit compensations recorded in the decision trace (Amendment-006).
- **Control Plane replication:** stateless over Kernel/Persistence (ORCH-002), so the
  orchestrator replicates/restarts freely; what is distributed is the plan, not the
  engine (Vol 9 Part 11).
- **Agent messaging:** events/commands/messages (Vol 14 Part 14) over the mechanical
  Event Fabric, routed by `tenant_id + correlation_id` (Amendment-004) — cross-cutting
  traversal per LAYER-001, no lateral layer calls.

### 3.8 Concurrency

The concurrency model for N agents over one truth base:

1. **Write-write, identical content:** both commits address to the same ContentId;
   commit is idempotent → one truth, safe convergence (ORCH-004; RUNTIME_FREEZE
   "Truth" guarantee).
2. **Write-write, same subject, conflicting content:** the second proposal is checked
   against committed truth; conflict → rejected as decision-truth and recorded as a
   committed compliance event (`outcome: conflict`, prior decision cited by ContentId)
   — the G1 `proposeDecision` semantics elevated to the runtime check. Ordering of
   "first" is defined by the shard leader's log order (IDR-001/IDR-005), which is
   total per shard.
3. **Same-content re-proposal with different payload binding:** rejected by the
   Kernel's content-integrity gate (RCR-005, already in Runtime v1.1).
4. **Read-write races:** an agent that must decide on latest truth reads at the
   linearizable tier through the leader (IDR-001 read tiers); agents tolerant of
   staleness read from followers. The tier is declared per read — consistency is
   explicit, not ambient.
5. **Check-then-commit races (policy/conflict TOCTOU):** two agents may concurrently
   pass a policy/conflict check and race to commit. Design position: the
   authoritative check is re-evaluated at the serialization point (leader-side,
   against the log-ordered committed state), so the loser's commit becomes a recorded
   conflict/violation, never a second truth. *How much of this check runs inside the
   commit gateway without violating the Kernel's Cannot row — "Orchestrate, plan or
   execute" (Amendment-003 Layer Matrix) — and the plane rule that only the Control
   Plane decides ("The Kernel never becomes the Control Plane", Vol 9 Part 2) is a
   real design tension — carried as Open Question OQ-3, resolved only by an
   IDR (§6), not silently.*
6. **Cancellation/priority under contention:** cooperative, idempotent cancellation;
   preemption checkpoints to the decision trace (Amendment-005).

### 3.9 Failure Modes

Per the frozen failure taxonomy (Amendment-006 placement table):

| Failure | Multi-agent manifestation | Home |
| --- | --- | --- |
| Agent crash mid-plan | Goal context lost; uncommitted proposals discarded | ORCH-002 (Control Plane restartable) |
| Control-plane crash | All in-flight agent plans lost, truth intact | Spec (ORCH-002) |
| Duplicate replay / retry storm | N agents retry the same commit | Spec (ORCH-004 idempotency) |
| Partial multi-step decision flow | Some steps committed, rest failed | Amendment-006 compensation (saga), trace-recorded |
| Human cancellation of an agent | Cancellation event; no partial truth | Amendment-005 |
| Shard leader loss | In-flight uncommitted agent work discarded; re-election | IDR-004 |
| Node crash / partition / split brain | Per-shard Raft handles; CP: minority side cannot commit | IDR-001/003/004 |
| Runaway delegation (agent spawns agents…) | Graph expansion explosion | Vol 9 Part 6 termination policy (max depth / budget / no-new-subgoal) + Vol 14 Part 18 budgets |
| Policy race (two agents, one budget) | Double-approve attempt | §3.8(5) serialization-point re-check → second becomes compliance event |
| Rogue/compromised agent identity | Ungoverned commits | Vol 2 Part 23 governance; NOTE: authenticated commit is v2.0 debt (RUNTIME_FREEZE #8) — honest limit, see §3.16 |

### 3.10 Recovery

- **Truth**: recovered by WAL replay + snapshots (IDR-005; I1's proven deterministic
  recovery — RUNTIME_FREEZE "Persistence · Replay · Audit" guarantee).
- **Plans**: NOT recovered — reconstructed. The Control Plane is stateless over
  Kernel/Persistence (ORCH-002); after a crash, unfinished goals are re-planned or
  resumed from their recorded decision-trace checkpoints (Amendment-005 preemption
  checkpointing).
- **Agent registry/governance state**: it is committed truth, so it recovers with the
  truth base — an agent's identity/permissions never depend on orchestrator memory.
- **Working memory (LCW)**: rebuilt from Kernel truth + events (Layer Matrix LCW row:
  reads "Kernel truth, events") — lossy by design; it was never authoritative.
- **In-flight executions**: outcome unknown at crash → resolved by idempotent re-issue
  (ORCH-004) or compensation (Amendment-006); never by guessing.

### 3.11 Replay

ORCH-003 applied to N agents: *a multi-agent run is replayable from the same Goal,
State, Policies, Capabilities and Runtime Fingerprint via the recorded decision
trace, not recomputation.* Design consequences:

- The **shard WAL is the interleaving**: because every truth mutation from every agent
  serializes through one per-shard log (IDR-001/005 "Raft log IS the WAL IS the
  decision trace"), the nondeterministic concurrency of N agents collapses at commit
  into one totally-ordered, replayable record. Replay of the shard = replay of the
  whole multi-agent history for that tenant/workspace.
- **Per-run decision traces** (Vol 9 Part 9) record each agent's expanded graph,
  engine outputs, arbitration choices, policy evaluations and Runtime Fingerprint;
  replay re-reads recorded outcomes deterministically. Recomputation is explicitly
  NOT guaranteed for non-deterministic engines (Vol 9 Part 9).
- **Cross-agent causality**: decisions cite the ContentIds of the truths they read
  (approvals cited by `approvedBy` ContentIds in the G1 flow) — replay can verify that
  a decision's cited evidence existed at its log position. *Whether citation of read
  evidence becomes mandatory for all decision types is OQ-4.*
- Executable proof shape (design-level, for §4): replay the shard WAL into a fresh
  node → byte-identical truth (I1/I2 precedent); replay a recorded L4 scenario trace →
  identical conformance artifact.

### 3.12 Consistency

- **Within a shard**: linearizable commits (IDR-001 consequences: "writes are
  linearizable"); reads at declared tiers (linearizable / bounded-staleness /
  eventual). Cross-agent consistency inside one tenant/workspace is therefore strong:
  there is one committed history, and any agent may pay for a linearizable read of it.
- **Across shards**: no consistency guarantee is claimed (IDR-001: no cross-shard
  atomic commit; tenant isolation means no cross-tenant consistency is *required*).
  Cross-workspace multi-agent workflows use sagas/compensation (Amendment-006).
- **Truth vs observability**: truth is CP; metrics/presence/agent statistics are AP
  (IDR-001 non-goals; CLAUDE.md IDR table "Truth CP · Observability AP"). An agent
  dashboard may lag; the truth base may not.
- **Working memory**: intentionally weaker — a live view, never consulted as
  authority (Amendment-001).

### 3.13 Availability

- Truth availability is bounded by CP consensus: a shard without a quorum rejects
  commits (IDR-001 CP choice — consistency over availability for truth).
- Agent *compute* availability is independent: engines are pure and run anywhere
  (IDR-001 refinement), so agents keep reasoning during a leader election; only their
  commits wait.
- The Control Plane is replicable because it is stateless (ORCH-002; Vol 9 Part 11) —
  orchestrator failover does not lose truth, only in-flight plans (recoverable, §3.10).
- Read availability degrades gracefully via the tier ladder (leader → follower →
  replica, IDR-001).

### 3.14 Scalability

- **Agent-count scaling** = compute scaling: pure engines distribute across nodes;
  consensus load grows with *commit rate per shard*, not with agent count per se
  (IDR-001 "Compute scaling is separated from consensus").
- **Tenant scaling** = shard scaling: one Raft group per tenant/workspace (IDR-001);
  agent populations of different tenants share nothing (SHARD-001; Vol 2 Part 21).
- **Known ceiling (honest):** a single very-hot shard — thousands of agents of ONE
  workspace committing at high rate — serializes at one leader. The frozen corpus
  provides no intra-shard scaling instrument; Kernel batch-commit is recorded v1.1
  debt (RUNTIME_FREEZE backlog #3) and would need an RCR. Carried as OQ-5, with
  "High-volume Streaming" (SCF Axis 8) as its measurement axis.
- **Delegation scaling**: bounded expansion (Vol 9 Part 6) prevents planning-graph
  blowup; budgets (Vol 14 Part 18) prevent economic blowup.

### 3.15 Performance

Design targets are stated as *measurement obligations*, not promised numbers (no
benchmarks exist for I2..I4 substrate yet — Correctness over Speed, Constitution
philosophy):

- Commit latency per agent decision = Raft round within shard quorum + WAL append
  (IDR-001/005 path); measured, not assumed.
- Policy check on the hot path reads committed policy truths — cacheable in LCW as a
  non-authoritative view with leader-side authoritative re-check (§3.8(5)).
- Engine invocations dominate wall-clock (LLM latency); the runtime's contribution is
  bounded to plan/commit overhead and must be characterized per L4 scenario run.
- No optimization work is in scope before the L4 correctness proofs pass
  (Non-Negotiable Rule 7).

### 3.16 Security

- **Isolation**: tenant and workspace isolation are absolute boundaries (Vol 2
  Part 21; SHARD-001 — "no cross-tenant data in a single shard"). Agents never share
  truth across tenants; the two-tenant isolation proof exists at the gateway today
  (RCR-007 `behaviour_8_two_tenant_isolation`) and must be re-proven under multi-agent
  load (§4).
- **Agent governance**: every agent has an owner, policies, permissions, capabilities
  and an audit trail (Vol 2 Part 23); roles include `Agent` (Part 13); permissions are
  Read/Write/Execute/Manage/Administer (Part 14); RBAC+ABAC (Part 19); least
  privilege/zero trust as principles (Part 21).
- **HONEST LIMIT (load-bearing):** Runtime v1.0 has **no principal/authN/authZ on
  `Kernel::commit`** — its threat model is a trusted single host; authenticated commit
  + signatures are recorded v2.0 debt (RUNTIME_FREEZE backlog #8, partially addressed
  by RCR-002's hash-chain digest). The G1 product records the same residual (the
  `role:'legal'` tag is not cryptographically bound — `enterprise-os.mjs` `approve()`
  comment). Therefore: **I5's agent-identity enforcement is structural (separate
  committed approval truths, proposer ≠ approver) until the authenticated-commit RCR
  lands; I5 MUST NOT claim cryptographic agent authentication under v1.x.** This is a
  design-blocking dependency for any untrusted-host deployment — recorded in §4
  (ORCH-004 note), §6 (RCR list) and OQ-1.
- **Tamper-evidence**: WAL hash-chain digest (RCR-002) makes the multi-agent
  compliance ledger tamper-evident at the store; signatures/anchoring remain v2.0.

### 3.17 Observability

- Per-agent tracking of tasks, actions, failures, costs and latency (Vol 14 Part 17).
- All observability is the AP plane (IDR-001 non-goals): eventually-consistent
  metrics/logs/traces/presence, explicitly never a truth source.
- Every engine/capability invocation carries `correlation_id` (Vol 9 Part 11;
  Amendment-004 routing key) — cross-agent workflows are traceable end-to-end by
  correlation, and per-tenant by `tenant_id`.
- Decision traces (Vol 9 Part 9) are the *forensic* observability: what did agent A
  know, decide, and cite — replayable, not merely logged.

### 3.18 Metrics

Candidate metric set (design-level; AP plane; names illustrative, not a contract):
per-shard commit rate and commit latency distribution; per-agent proposal count /
accepted / blocked-by-policy / conflict-detected counts; policy-gate firing counts and
evaluation latency; approval-truth latency (proposal → approval → committed decision);
delegation depth and expansion-bound hits; budget consumption per agent (Vol 14
Part 18); leader-election frequency per shard; replay-verification pass rate;
conformance-artifact emission rate. KPI alignment: these feed the Evidence Ledger
(Standard Validation Era KPI = Evidence Increased, CLAUDE.md Project Status).

### 3.19 Auditability

- Every action records Who, When, What, Why (Vol 2 Part 20) — for agents: the agent
  identity (Who), the log position/timestamp (When), the committed decision (What),
  and the cited goal/policy/approval ContentIds (Why).
- The compliance ledger is committed truth: blocked decisions and conflicts are
  *recorded as truths*, not just refused (G1 semantics: `uci.compliance` events with
  `outcome: blocked | conflict`) — the audit trail of what was *attempted* is itself
  replayable.
- The WAL is the audit log (RUNTIME_FREEZE "Persistence · Replay · Audit"; IDR-005),
  tamper-evident per RCR-002.
- Approvals are separate, addressable truths cited by the decisions they authorize
  (G1 E1 fix) — auditors can verify authorization without trusting the proposer.

### 3.20 Trade-offs

| Chosen | Over | Because |
| --- | --- | --- |
| One commit gateway per shard (CP) | Per-agent truth stores / CRDT merge | Truth would be "eventually one", contradicting single-source-of-truth (IDR-001 rejected-alternatives) |
| Conflict detected → compliance event | Last-writer-wins overwrite | Cross-agent consistency must be auditable, not silent (G1 semantics; Vol 2 Part 20) |
| Replay-from-trace | Recompute agents | Agents are LLM-backed, nondeterministic (ORCH-003 rationale) |
| Agents as governed identities on existing layers | A new "agent layer" | Rules 2/3; Vol 9 Part 2 already places coordination in the Control Plane |
| Structural approval separation now | Waiting for cryptographic authN | v1.x freeze reality; the structural half is achievable and already exercised (G1 E1) — the crypto half is an explicit v2.0 RCR, not a fake claim |
| Bounded delegation | Unbounded agent autonomy | Termination policy is frozen (Vol 9 Part 6); infinite meta-planning is a known failure mode |
| Per-shard linearizability, cross-shard sagas | Global transactions | IDR-001 explicitly rejects cross-shard atomic commit in v1 |

### 3.21 Risks

1. **Hot-shard serialization** (§3.14) — a single workspace's agent swarm bottlenecks
   at one leader; mitigation instruments (batch-commit RCR) are debt, not designs.
2. **TOCTOU policy races** (§3.8(5)) — placing the authoritative re-check without
   giving the Kernel a "decide" responsibility (Layer Matrix: Kernel "Cannot
   orchestrate, plan or execute") is genuinely delicate; a wrong placement is an
   ORCH-001/LAYER-001 violation. Must be resolved by IDR before any code (OQ-3).
3. **Identity spoofing under v1.x** — no authenticated commit (§3.16); any in-process
   caller can wear any agent identity. Deployments beyond a trusted host are unsafe
   until the v2.0 RCR; the design must keep saying so out loud.
4. **Control-plane statelessness erosion** — multi-agent coordination invites caching
   assignment/registry state in the orchestrator; any such cache becoming
   load-bearing violates ORCH-002. Countermeasure: the §4 ORCH-002 kill-restart proof.
5. **Prerequisite drift** — I2/I3/I4 do not exist; if their implementations diverge
   from IDR assumptions, this design inherits the divergence. Countermeasure: this
   package binds only to IDR text, and §5 re-validates at L3 before L4.
6. **Ontology under-specification** — decision/compliance types are product-level
   today; certifying L4 against unregistered types would violate O-006 discipline.
   Countermeasure: the §6 CCP is a hard prerequisite for L4 certification.
7. **Scope creep toward Aspirational** — recursive self-improvement / autonomous
   strategic evolution are explicitly NOT v1.0 (Baseline Part 4); delegation depth
   bounds are the guard.

### 3.22 Open Questions

- **OQ-1 (agent authN):** What is the v2.0 authenticated-commit design (principal on
  `Kernel::commit`), and can I5 ship a useful trusted-host L4 before it? Instrument:
  RCR (breaking → v2.0 per RUNTIME_FREEZE #8). Until answered, all agent-identity
  claims are structural only.
- **OQ-2 (agent registry placement):** Is the versioned agent registry (Vol 14
  Part 20) committed truth in the Kernel (like policies), or Capability-Fabric-style
  registry state? The Layer Matrix has no "agent registry" row; both readings uphold
  OWN-001 differently. Needs an IDR; this package *leans* truth-side (recovery
  argument, §3.10) without deciding.
- **OQ-3 (authoritative check placement):** Where exactly does the serialization-point
  policy/conflict re-check run — a Control Plane admission stage ahead of the leader
  commit, or a Kernel-adjacent gate? Constraint set: ORCH-001 (no CP truth), the
  Kernel Cannot row "Orchestrate, plan or execute" (Amendment-003 Layer Matrix) plus
  the plane rule that only the Control Plane decides ("The Kernel never becomes the
  Control Plane", Vol 9 Part 2), no lateral coupling (LAYER-001). Needs an IDR with
  a conformance scenario.
- **OQ-4 (evidence citation):** Must every committed decision cite the ContentIds of
  the truths it read (as G1 approvals do), making read-sets replay-verifiable? Strong
  for audit (Vol 2 Part 20 "Why"), but the frozen corpus does not mandate it. CCP
  candidate.
- **OQ-5 (intra-shard throughput):** What commit rate must one shard sustain for the
  L4 scale scenarios, and does that force the batch-commit RCR (RUNTIME_FREEZE
  backlog #3) into I5's critical path? Unknown until I2 exists and is measured.
- **OQ-6 (agent-to-agent message semantics):** Vol 14 Part 14 names events, commands
  and messages but fixes no delivery/ordering contract. Which guarantees (at-least-once
  + idempotent consumption per ORCH-004 seems forced, but ordering across agents is
  open) — IDR needed.
- **OQ-7 (LCW multi-agent views):** Does each agent get an isolated LCW working-memory
  view, or is working memory shared per workspace? Amendment-001 fixes the owner
  (LCW), not the sharing topology. Depends on the LCW crate leaving contract-only
  status (RCR) — and touches **LCW-001 (PROPOSED — CCP-GATE required)**.
- **OQ-8 (arbitration policy content):** Join-node arbitration merges conflicting
  branch/agent outputs "by policy (confidence-weighting, tie-break)" (Vol 9 Part 6);
  the concrete arbitration policy language is unspecified. CCP or IDR depending on
  whether it needs new ontology types.

---

## 4. Invariant Mapping — registered invariants I5 must uphold

All seven registered invariants bite. Proofs are named per the RCR-006 PropertyCheck
discipline (invariant → executable proof catalog); every proof below is NEW work I5
would need (G2-gated), except where an existing test is cited as the single-agent
precedent to be extended.

| Invariant (registered) | I5 obligation | Executable proof needed |
| --- | --- | --- |
| **OWN-001** — every state has exactly one owner (Amendments Batch 1, registry table) | Agent memory/plan/decision/registry state each keep one owner (§3.6); no coordinator-side shadow copies become load-bearing | Ownership audit test: enumerate every state class touched in an L4 run and assert its single frozen owner; extend the existing architecture-gate OWN-001 check (RCR-006) to the multi-agent surfaces |
| **LAYER-001** — dependencies downward only; cross-cutting via Control Plane/Event Fabric (Amendment-003) | Agent-to-agent communication traverses Event Fabric/Control Plane only; no Engine→Engine or Capability→Capability lateral calls in any agent path | Static layer check (existing executable architecture gate) extended over the I5 crates + a runtime probe asserting no lateral call edges appear in a recorded L4 trace |
| **SHARD-001** — partition by tenant/workspace; immutable partition key (Amendment-004) | N agents of tenant A and N of tenant B share zero truth; agent identity is shard-bound for life | Multi-agent extension of `behaviour_8_two_tenant_isolation` (RCR-007): concurrent agent populations in two tenants; assert no cross-tenant read/commit/snapshot leak and immutability of an agent's partition key |
| **ORCH-001** — Control Plane owns no truth (Vol 9 Part 5) | Coordination artifacts (assignments, delegations, arbitrations) are plan artifacts; only Kernel commits create state | Probe: diff persisted state before/after a coordination-heavy L4 run with commits disabled → zero persisted deltas from the Control Plane; every state delta in a normal run maps to a Kernel commit record |
| **ORCH-002** — Control Plane produces plans, never persistent state (Vol 9 Part 5) | Orchestrator is kill-restartable mid-multi-agent-run with no truth loss | Kill-restart test: SIGKILL the orchestrator during an N-agent scenario, then assert **single-run properties** (no cross-run byte comparison — LLM-backed runs never reproduce identical outputs, §5; SCF Part 8 mandates property-based, not golden-output, conformance): (a) the WAL/truth base is an intact, uncorrupted committed prefix — RCR-002 hash-chain digest verifies end to end; (b) zero persisted state deltas originate from the Control Plane (same probe as the ORCH-001 row); (c) unfinished goals resume/replan from recorded decision-trace checkpoints (Amendment-005). A byte-identity variant (killed vs non-killed run) is admissible ONLY in a deterministic scripted-engine harness run and must be labeled as such |
| **ORCH-003** — every execution replayable from Goal/State/Policies/Capabilities/Runtime Fingerprint via recorded trace (Vol 9 Part 5) | The interleaved N-agent history replays deterministically from the shard WAL + per-run decision traces | Replay test: replay shard WAL into a fresh node → identical truth hash (extends I1 recovery + I2 design's follower-apply proof); replay a recorded L4 scenario trace → identical conformance artifact, no engine recomputation |
| **ORCH-004** — every engine/capability invocation idempotent and content-addressable (Vol 9 Part 5) | Concurrent duplicate proposals from many agents converge to one truth; retries are safe under contention | Contention test: k agents concurrently commit identical content → exactly one truth, all receive the same ContentId; same-hash/different-payload rejected (extends RCR-005 `ContentIntegrity`); NOTE: content-addressability is proven, but *principal* binding is NOT claimable under v1.x (§3.16 / OQ-1) |

**PROPOSED invariants referenced by this design** — informative only, never enforced,
per Invariant Registry Parts 4–5 and `ARVES_OS_Volume_6` (a proposed-invariant
expectation can at most yield PARTIAL, never FAIL):
LCW-001 (PROPOSED — CCP-GATE required; §3.6/OQ-7) · G-001 (PROPOSED — CCP-GATE
required; the "sole commit gateway" phrasing in §3.7 rests on registered
ORCH-001/OWN-001 + IDR-001, not on G-001) · QUERY-001 (PROPOSED — CCP-GATE required;
agent reads, §3.2) · CAP-002/003/007/008 (PROPOSED — CCP-GATE required; capability
selection/idempotency/correlation/cancellation in agent plans) · ENG-001..004
(PROPOSED — CCP-GATE required; engine purity/replay in agent graphs). Any of these
that I5 wishes to *gate on* must first ratify via CCP with a conformance scenario
(Reference Lifecycle Part 6, CCP-GATE).

---

## 5. Conformance Plan

**Framework:** `ARVES_Scenario_Conformance_Framework_v1` (SCF). Conformance is
structural/property/invariant-based, never golden-output (SCF Part 8) — essential
here, since N LLM-backed agents never reproduce identical outputs.

### 5.1 Axes instantiated (SCF Part 5)

| Axis | Why I5 must instantiate it |
| --- | --- |
| **9 — Multi-agent Coordination** (primary) | "Delegation, arbitration across agents" — the milestone's defining axis |
| 3 — Human Collaboration | Approval gates in the decision flow (Vol 9 Part 10; G1 approval truths) |
| 10 — Policy-heavy Governance | Policy-as-truth enforcement + dense audit (G1 `checkPolicy` semantics) |
| 11 — Autonomous Decision | Unattended agent decisions within risk/confidence limits (SCF Axis 11 "risk/confidence limits"; ARVES-23 Agent Governance "risk limits"; Vol 14 Part 16 governance of budgets/risks) |
| 12 — Recovery & Replay | Deterministic replay of the multi-agent trace (ORCH-003) |
| 8 — High-volume Streaming | "Tenant isolation at scale" — the hot-shard measurement axis (OQ-5) |

Reference-scenario anchor (SCF Part 6): **Enterprise Knowledge Query** (axes 1+8+9)
is the frozen scenario that already asserts "Tenant isolation held … control plane
owns no truth (ORCH-001)" under multi-agent load; I5 instantiates it plus new
axis-9-centric scenarios (a Coordinator/Worker delegation scenario and a
conflicting-decision arbitration scenario — new scenarios enter via the suite's
versioned governance, SCF Part 11).

### 5.2 Level

**L4 Multi-Agent** — "Conformance preserved under multi-agent coordination" (SCF
Part 10). L4 presupposes L3 Distributed (I2/I3/I4 substrate) which presupposes
L2 Cognitive Control; today the reference runtime holds **L1-grade live evidence for
two/three nodes only** (RCR-008/009/010: Kernel, Information Platform, Query probes at
G0/G1). The honest ladder for I5: L2 probes (Control Plane node emitting ORCH-001..004
evidence, SCF Part 7 table) → L3 under distribution → L4 under agent concurrency.
Results are always reported as "N% at Level Lx against Framework vA / Spec vB"
(SCF Part 11).

### 5.3 Node probes (SCF Part 7) — multi-agent evidence

Every I5 scenario run traverses the frozen pipeline and must add, per node: Control
Plane — expanded multi-agent Engine Graph, delegation tree, arbitration records,
ORCH-001..004 upheld, no truth produced; Kernel — every agent state transition
recorded, sole-truth-owner held under N writers; LCW — consistent world view per
scenario; Query — tenant-scoped reads per declared tier; Engine — pure invocations
per agent; Capability — bindings per plan; Execution — idempotent actions with
`correlation_id`. The per-run **conformance artifact** (SCF Part 9; Vol 9 Part 14)
gains multi-agent fields: proposing agent identity per decision, policy gates fired,
conflicts detected, arbitration choices — it is both certificate and regression
record.

### 5.4 Per-milestone Success Criteria — what each concretely means for I5

| Constitution criterion | Concrete I5 meaning |
| --- | --- |
| Architecture PASS | Independent review confirms: no new layer; agent constructs decompose per §1 table; LAYER-001 static gate green over I5 code |
| Conformance PASS | All §5.1 axis scenarios PASS at L4 per SCF Part 8 verdict rules (all required invariants + critical isolation/safety properties hold; a registered-invariant violation is always FAIL, per Vol 6 OS manual) |
| Certification PASS | A conformance artifact set at L4 against pinned Framework/Spec versions, graded by the maintainer-independent harness (FOUNDATION program), certificate issued per Reference Lifecycle Part 3 |
| Independent Review PASS | The 14-facet review (Constitution) rendered as if a third party submitted I5; the §3.21 risks explicitly re-examined |
| Invariant coverage 100% | Every row of §4's registered table has its named executable proof implemented and green; zero proof-only registered invariants remain for I5's surfaces (Constitution invariant rule) |
| Replay PASS | §4 ORCH-003 proofs: WAL replay → identical truth hash; trace replay → identical conformance artifact |
| Distributed tests PASS | §4 SHARD-001/ORCH-004 contention + isolation proofs under real multi-node deployment (leader loss, partition, retry storms per §3.9 table) |
| No architecture/spec drift | Frozen corpus untouched; every I5 change to `runtime/` entered via ratified RCR; diff-audit of `spec-markdown/`/`standard/` empty |

---

## 6. NON-GOALS and Required Change Instruments

### 6.1 Explicit NON-GOALS of I5

1. **No implementation now** — this is a Ch4 PREP MODE design package; the G2 build
   gate is closed. No code, no crate edits, no tests are created by this package.
2. **No new architectural layer** ("agent layer", "agent bus", "agent kernel") —
   agents decompose onto the frozen Layer Matrix (§1); Rule 3.
3. **No cross-shard atomic commit / global transactions** — excluded by IDR-001;
   cross-shard flows are sagas (Amendment-006).
4. **No cryptographic agent authentication claim under v1.x** — authenticated commit
   is v2.0 debt (RUNTIME_FREEZE #8); I5 ships structural identity separation only
   (§3.16) and says so in every public claim.
5. **No agent marketplace / publishing / discovery** — Vol 14 Part 19 exists in the
   corpus, but Marketplace is consciously deferred to v2 (Baseline Part 3).
6. **No recursive self-improvement / autonomous strategic evolution** — Aspirational,
   explicitly NOT v1.0 (Baseline Part 4); delegation is bounded (Vol 9 Part 6).
7. **No embodied/physical agents at scale** — Baseline Part 4 (Vol 8 vision); SCF
   Axis 6 is not an I5 axis.
8. **No new consensus/replication/membership design** — IDR-001..005 are binding and
   sufficient; I5 adds zero consensus mechanisms.
9. **No specification changes** — the frozen corpus is not edited; where I5 exposes a
   spec-level need, the instrument below is filed and I5 STOPS on that surface
   (Change Management table, Constitution).
10. **No enforcement of PROPOSED invariants** — LCW-001/CAP-*/ENG-*/G-001/QUERY-001
    stay informative until their CCPs ratify (Invariant Registry Part 5).
11. **No cross-runtime agent federation** — Cross-Runtime Federation is deferred to
    v2 (Baseline Part 3).

### 6.2 Instruments any frozen-surface change would require

| Needed change | Frozen surface | Instrument |
| --- | --- | --- |
| Implement `arves-control-plane` (contract-only → orchestrator with multi-agent goal contexts) | `runtime/` (Runtime v1.0 freeze) | **RCR** (additive → v1.1-class; per RUNTIME_FREEZE process, with its own ED-006 destroy→repair→prove cycle) |
| Implement `arves-lcw` (contract-only → working-memory views, OQ-7) | `runtime/` | **RCR** |
| Kernel batch-commit for hot-shard throughput (OQ-5) | `runtime/` (recorded backlog #3) | **RCR** (already registered as v1.1 debt) |
| Authenticated commit — principal/authN on `Kernel::commit` + signatures (OQ-1) | `runtime/` (backlog #8, breaking) | **RCR → v2.0 major** |
| Bridge request-id correlation under concurrent multi-agent invocation | `runtime/` (backlog #1) | **RCR** (already registered as v1.1 debt) |
| Register decision/compliance ontology types (`uci.fact/policy/approval/decision/compliance`) as versioned `uci.*` types (O-006) | `spec-markdown/` ontology registry (frozen) | **CCP Amendment** (MINOR, backward-compatible addition per Reference Lifecycle Part 7) **with a conformance scenario** (CCP-GATE) |
| Ratify LCW-001 (or any PROPOSED invariant I5 wants to gate on) | Invariant Registry standing | **CCP** with conformance scenario (Reference Lifecycle Part 6; Invariant Registry Part 5) |
| New axis-9 reference scenarios added to the suite | Conformance suite (versioned, single owner) | Suite version bump per **SCF Part 11** governance (suite pinned to spec version) |
| Multi-agent *mechanisms*: scheduler policy, agent-registry placement (OQ-2), check placement (OQ-3), message delivery semantics (OQ-6), arbitration policy engine (OQ-8) | None (implementation-era decisions) | **New IDR batch** ("IDR Batch — Multi-Agent Mechanisms"), following the Amendment-006 "mechanisms → IDR" precedent; IDRs implement, never change, the spec |
| Any wording defect discovered in Vol 14 / Vol 2 during I5 | `spec-markdown/` | **CCP Amendment** (minor wording) — never a silent edit |

---

*Final definition — I5 Multi-Agent Runtime = N governed agent identities planning in
one stateless Control Plane, reasoning over ONE per-shard, content-addressed,
replayable truth base — proving ORCH-001..004 do not bend under concurrency.*

*Prepared under Ch4 PREP MODE. This document authorizes no implementation; the G2
gate opens only by maintainer ruling.*

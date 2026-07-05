# I4 — Capability Scheduling: Engineering Design Package

```
=====================================================================
  STATUS: DESIGN PACKAGE (Ch4 PREP MODE) — NO CODE
  Build gate (G2) CLOSED — I2..I6 implementation remains gated.
  Prepared 2026-07-05 under maintainer prep-mode ruling.
=====================================================================
```

**Milestone:** I4 — Capability Scheduling (`ARVES_00_Baseline_v1.md`, Part 5: *"I4 Capability
Scheduling — Cluster-wide capability scheduling"*).

**Scope of this package:** scheduling capability invocations across the cluster — placement,
capability-gated authorization, deterministic/idempotent execution contracts, backpressure,
and failure isolation. This is a **design document only**. No implementation code exists or
is authorized by this document; the frozen surfaces (`runtime/`, `standard/`, `spec-markdown/`,
`corpus/`) are untouched. Any future implementation of this design in `runtime/` requires a
ratified **Runtime Change Request** per `runtime/RUNTIME_FREEZE_v1.0.md` ("Runtime Change
Request (RCR) — the only way the runtime changes").

**Dependency chain honoured:** `Theory → Specification → Contracts → Behaviour → Conformance
→ Implementation` (Engineering Constitution, "The Primary Engineering Principle"). This
package sits at the *Behaviour/Conformance-planning* stage; Implementation is deliberately
absent.

---

## 1. BEFORE-WRITING-CODE Answers (Constitution, "Before Writing Code")

### 1.1 Which UCI node is affected?

Primarily the **Capability** node of the conformance pipeline (`Reality → Information
Platform → Kernel → LCW → Query → Engine → Capability → Execution → Reality`, Scenario
Conformance Framework v1, Part 7), together with:

- the **Control Plane** components *Capability Planner* and *Execution Planner*
  (Vol 9 Cognitive Control Plane v2, Part 4), because capability **selection and scheduling
  decisions are Control Plane concerns** (Vol 9 Part 3: *"Capability Fabric is Data Plane;
  Capability Planner is Control Plane"*);
- the **mechanical Data Plane runtime** (event bus, task, workflow, **scheduler**, retry),
  which Vol 9 Part 12 explicitly reclassifies as Data Plane mechanical runtime and does NOT
  remove — the dispatch machinery of I4 lives there, not in a new layer;
- the **Execution** node (idempotent, addressable action with `correlation_id` —
  Scenario Conformance Framework Part 7, Execution row).

No new UCI node is created. I4 composes existing nodes; inventing a "Scheduler layer" would
violate Non-Negotiable Rule 3 (never introduce new architectural layers).

### 1.2 Which documents govern it?

| Document | What it governs here |
| --- | --- |
| `spec-markdown/ARVES_00_Baseline_v1.md` Part 5 | I4 milestone definition ("Cluster-wide capability scheduling") |
| `spec-markdown/ARVES_Volume_9_Cognitive_Control_Plane_v2.md` Parts 2–5, 8–12 | Two-plane model; Fabric=Data / Planner=Control; ORCH-001..004; execution flow; replay; distribution readiness; mechanical runtime reclassification |
| `spec-markdown/ARVES_Engine_Graph_Specification_v1.md` Parts 3, 6–11 | Node manifest fields (Capabilities Required, Determinism, Idempotency Key, Failure/Retry Policy, Timeout, planning metadata); runtime contract; versioning |
| `spec-markdown/ARVES_IDR_Batch_1_Kernel_Distribution_v1.md` (IDR-001..005) | CP truth, per-shard Raft, compute-anywhere/commit-through-leader, replication of outcomes, membership, election, WAL |
| `spec-markdown/ARVES_00_Invariant_Registry_v1.md` Parts 2 & 4 | Registered invariants (OWN-001, LAYER-001, SHARD-001, ORCH-001..004); PROPOSED CAP-001..009 / ENG-001..005 status |
| `spec-markdown/ARVES_Scenario_Conformance_Framework_v1.md` Parts 5–10 | Axes, node probes, conformance semantics, levels (L3 Distributed) |
| `spec-markdown/ARVES_Reference_Lifecycle_v1.md` Part 6 | CCP process and CCP-GATE (no ratification without a conformance scenario) |
| `runtime/RUNTIME_FREEZE_v1.0.md` | Runtime v1.0 freeze; contract-only `arves-capability-fabric`; RCR process; v1.1 debt (engine-enforced determinism, bridge request-id correlation, Kernel batch-commit) |
| `products/arves-ecosystem-sdk/src/kit.mjs` (read-only) | The exercised G1 reference capability host: author→certify→package→install→invoke path against the frozen runtime |

### 1.3 Which contracts apply?

- **`arves-capability-fabric` (FROZEN, CONTRACT-ONLY)** — `CapabilityRegistry`
  (`register` / `bind` / `resolve`), `CapabilityBinding` (capability, shard, version,
  provider, contract), `InvocationContract` (input/output schema + `EffectClass`:
  `Pure` / `IdempotentEffect` / `ProposesWrite`), `RegistryError` (`Unbound`,
  `NonMonotonicVersion`, `UndeclaredCapability`). Per its own crate doc it *"never decides
  whether something should run (that is the Control Plane's plan, ORCH-002) and it never
  records what happened (that is the Kernel's truth, ORCH-001)"*. I4 **consumes** this
  contract unchanged.
- **Engine Node Contract / ABI** (Engine Graph Spec Part 3) — the manifest fields the
  scheduler reads: `Capabilities Required`, `Determinism`, `Idempotency Key`,
  `Failure Policy`, `Retry Policy`, `Timeout`, and planning metadata
  (`Confidence` / `Cost` / `Latency`, Part 8: *"The Control Plane uses them for engine
  selection, scheduling and arbitration"*).
- **Runtime Contract** (Engine Graph Spec Part 10) — a conformant runtime MUST enforce
  Idempotency Key, Retry Policy, Failure Policy, Timeout; record invocations into the
  decision trace with `correlation_id` and Runtime Fingerprint; and *"honour Capabilities
  Required via the Capability Fabric"*.
- **Runtime API surface** (RUNTIME_FREEZE_v1.0.md, "The Runtime API") — SDK + Bridge
  (`commit` / `invoke`); the exercised capability logic today flows through the SDK/Bridge in
  `products/` (RCR-001 guarantee-scope clarification).
- **G1 host contract (reference, product-side)** — `kit.mjs`: capability manifests
  (`uci.capability-manifest`), certification gate enforced (never attested) at install,
  content-addressed artifact signature, effects committed one-by-one through the Bridge.

### 1.4 Which invariants apply?

**Registered-normative** (Invariant Registry Part 2 — the only enforceable set):
`OWN-001`, `LAYER-001`, `SHARD-001`, `ORCH-001`, `ORCH-002`, `ORCH-003`, `ORCH-004`.
Full mapping with executable-proof obligations in §4.

**PROPOSED (informative — CCP-GATE required, NOT enforceable):** `CAP-001..009`,
`ENG-001..005` (listed in Invariant Registry Part 4; governance rule stated verbatim in
Part 5: *"No proposed invariant may be enforced as normative … until it passes the
CCP-GATE"*). This design references them only as design
guidance with explicit `(PROPOSED — CCP-GATE required)` markers.

### 1.5 Which ownership rules apply?

- The **Capability Fabric** is the single owner of binding state — and of nothing else
  (OWN-001; fabric crate doc: *"the single owner of binding state"*).
- The **Kernel** is the sole owner of committed truth (ORCH-001); the scheduler never
  commits, never persists outcomes.
- **Policy** is owned by Security & Governance (Vol 17) and Agent Governance (Vol 14/Vol 2);
  the Control Plane *enforces and sequences* policy but never owns it (Vol 9 Part 10).
- **Schedules/placements are plans** — Control Plane output, never persistent state
  (ORCH-002). The scheduler is restartable and stateless over Kernel/Persistence
  (Vol 9 Part 11).
- **Selection is the Capability Planner's** (Control Plane); **carrying bindings is the
  Fabric's** (Data Plane) (Vol 9 Part 3). I4 must not merge these two ownerships
  (Non-Negotiable Rule 8: never duplicate ownership).

### 1.6 Which IDRs apply?

| IDR | Binding consequence for I4 |
| --- | --- |
| IDR-001 (CP, per-shard Raft) | Capability/engine **compute runs anywhere; only the COMMIT goes through the shard leader** ("Engines run anywhere … Compute scaling is separated from consensus"). Scheduling is per-shard-scoped; no cross-tenant consistency required. No cross-shard atomic commit in v1 — cross-shard coordination is sagas/compensation (Amendment-006 as cited by IDR-001). |
| IDR-002 (Leader→Followers, snapshots, WAL) | Followers apply committed **OUTCOMES**, never recompute invocations. A capability retried on a different node must converge to the same single committed truth (dedupe at the Kernel via ORCH-004 content addressing). |
| IDR-003 (Joint Consensus membership) | Scheduler worker-pool membership changes ride the same safe-membership discipline; placement must tolerate nodes joining/leaving without split-brain assumptions. |
| IDR-004 (Per-shard leader election) | *"Leader loss triggers re-election; in-flight uncommitted work is discarded (no partial truth, Amendment-005/006)."* The scheduler MUST treat in-flight dispatches as discardable and safely re-dispatchable. |
| IDR-005 (Append-only WAL, deterministic replay) | Placement/authorization decisions enter the decision trace; the Raft log IS the WAL IS the decision trace (IDR-001 refinement). |
| IDR-006 (products consume frozen platform) | Product-side capability hosts (kit.mjs pattern) consume the scheduler as a frozen runtime API; a needed platform change is a Platform Change Proposal / RCR, never a product-side edit. |

Additionally binding: the **CP/AP boundary** (IDR Batch 1 summary table): *"Presence /
capability statistics — AP (CRDT)."* Scheduler load/health statistics are explicitly
allowed to be eventually consistent; truth (committed effects) is CP.

### 1.7 Does this create architectural drift?

**No — by construction, and this is checkable.** The design adds zero layers, zero
ownership, zero invariants. It instantiates: Capability Planner + Execution Planner
(Vol 9 Part 4 rows already frozen), the mechanical Data Plane scheduler (Vol 9 Part 12,
already frozen), and the Fabric contract (frozen crate). The one drift hazard identified —
a scheduler that starts recording outcomes or owning a queue-as-truth — is called out as a
named failure mode (§3.9 F-DRIFT) and blocked by the ORCH-001/ORCH-002 proof obligations
(§4).

### 1.8 Does this require CCP / Amendment / a new IDR?

**Yes — three instruments before any build (enumerated fully in §6):**

1. **A new IDR (proposed name: IDR-007 "Capability Placement & Backpressure")** — the frozen
   corpus states scheduling *inputs* (planning metadata, Part 8 Engine Graph) and
   *constraints* (compute-anywhere/commit-through-leader), but no frozen text selects a
   placement algorithm or admission-control policy. That is an engineering decision →
   IDR per the Change Management table ("Engineering decision → IDR").
2. **RCRs** for any code landing in `runtime/` (the workspace is frozen at v1.0;
   `arves-capability-fabric` is contract-only by design — giving it or a sibling crate
   scheduling behaviour is a runtime change, additive → v1.1 minor).
3. **CCP ratification** for any CAP-00n invariant this milestone wants to *enforce*
   (each needs a conformance scenario per Reference Lifecycle Part 6 CCP-GATE).

No specification change is required or proposed. The Specification Era stays frozen.

### 1.9 Can another independent implementation reproduce this behaviour?

Yes, by design, with one honest caveat. Everything the scheduler needs is declared in
portable, frozen artifacts: the engine/capability manifest (Engine Graph Part 9: *"A runtime
needs only the manifest (not the source) to schedule and execute a node"*), the fabric
binding contract, and the decision trace. Conformance is structural/property/invariant-based,
not golden-output (Scenario Conformance Framework Part 8), so an independent scheduler with a
*different placement algorithm* still passes, provided invariants and properties hold.
**Caveat:** the placement algorithm itself is intentionally NOT part of conformance
(it is an IDR-level reference-implementation choice); independence is over *behavioural
properties*, not over identical placement choices. This mirrors the Engine Graph Part 13
independent-implementability bar.

### 1.10 Would this implementation still pass conformance five years from now?

Yes, under three conditions this design enforces: (a) every scheduled invocation is pinned
to versioned, content-addressable manifests and bindings, so old traces remain replayable
against a Runtime Fingerprint (Engine Graph Part 11; Vol 9 Part 9); (b) conformance results
are stated against a suite version ("N% at Level Lx against Framework vA / Spec vB",
Scenario Conformance Framework Part 11), so the verdict does not silently rot; (c) the
scheduler asserts invariants, not outputs, so model/hardware drift in nondeterministic
capabilities does not invalidate the certificate (Part 8 central rule). What could break it:
ratification of CAP-00n via CCP would ADD obligations (e.g. CAP-008 cancellation) — tracked
as Open Question OQ-5.

---

## 2. Architecture Readiness & Gap Summary (Workflow steps 1–8, condensed)

- **Readiness:** I1 exists (Kernel/Persistence/WAL/replay implemented, workspace 87/0 per
  RUNTIME_FREEZE RCR ledger); `arves-control-plane` and `arves-consensus` are CONTRACT-ONLY
  per their RCR-001 status headers; `arves-capability-fabric` is contract-only *by design*
  (its frozen contract plus an in-memory reference registry, `MemRegistry`, with tests);
  `arves-execution` is interfaces/contracts EXCEPT the working `CancellationToken`
  (`Arc<AtomicBool>`, RCR-001 item #5) — consistent with OQ-6. I2
  (Cluster Kernel) and I3 (Distributed Query) precede I4 in the Baseline Part 5 ordering and
  are **not built**; §3.4 records the dependency, §3.22 the sequencing question (OQ-1).
- **Gap:** the frozen corpus fully constrains *what a scheduled invocation must satisfy*
  (idempotent, content-addressed, traced, capability-gated, commit-through-leader) but is
  silent on *how placement chooses a node* and *what backpressure concretely does* (axis 8
  names backpressure; no normative admission contract exists). Both gaps are engineering
  decisions → new IDR (§6), NOT spec gaps; the spec is not reopened.

---

## 3. ENGINEERING DESIGN

> Terminology. "Scheduler" below always means the composite of three frozen-spec roles, never
> a new component: **Capability Planner** (selects/binds — Control Plane, Vol 9 Part 4),
> **Execution Planner** (turns the resolved plan into an executable execution plan — Control
> Plane, Vol 9 Part 4), and the **mechanical dispatch runtime** (task/workflow/scheduler/retry
> — Data Plane, Vol 9 Part 12).

### 3.1 Responsibilities

1. **Selection & binding (Control Plane).** For each plan node's `Capabilities Required`
   (Engine Graph Part 3), resolve the active `CapabilityBinding` for the node's shard via
   `CapabilityRegistry::resolve` (frozen fabric contract). Selection policy uses the
   machine-readable planning metadata Confidence/Cost/Latency (Engine Graph Part 8).
2. **Authorization (Control Plane enforcement, Governance-owned policy).** Gate every
   invocation on (a) an active binding existing in that shard (fabric `Unbound` is a
   hard deny), (b) policy gates owned by Vol 17/Vol 14 and *enforced* here (Vol 9 Part 10),
   (c) the declared `EffectClass` — a `ProposesWrite` capability may only produce proposed
   writes routed to the Kernel commit gateway, never direct truth (fabric contract;
   ORCH-001; CAP-004 is the matching statement but is **(PROPOSED — CCP-GATE required)**).
3. **Placement (Execution Planner output).** Assign each authorized invocation to a compute
   node under the IDR-001 rule: compute anywhere, commit only through the shard's Raft
   leader. Placement decisions are serializable plan artifacts (Vol 9 Part 11), never state.
4. **Deterministic/idempotent execution contract enforcement.** Enforce Idempotency Key,
   Retry Policy, Failure Policy, Timeout per manifest (Engine Graph Part 10). Re-dispatch is
   safe *at the truth boundary* because invocations are idempotent and content-addressable
   (ORCH-004). **Honesty caveat (mirrors §3.16):** at-most-once applies *provably* only to
   committed truth. For a capability's EXTERNAL side effect, at-most-once relies on the
   provider honouring its declared `EffectClass::IdempotentEffect` — the frozen fabric crate
   "neither validates nor performs the effect; the declaration is a contract the caller must
   honour, not behaviour this crate enforces", and fabric/engine-enforced idempotency is
   recorded v1.1 debt #2 (RUNTIME_FREEZE backlog). A provider that misdeclares idempotency
   can duplicate its external effect under re-dispatch with no partial-truth violation to
   detect it.
5. **Backpressure (per-shard admission control).** Bound in-flight work per shard/tenant so
   high-volume load cannot violate tenant isolation (Scenario Conformance Framework axis 8:
   "Throughput, backpressure, tenant isolation at scale"). Overload responses follow the
   node's declared Failure Policy (fail / degrade / escalate — Engine Graph Part 3).
6. **Failure isolation.** A failing capability, provider, node, or tenant must not spill
   over: shard-scoped queues (SHARD-001), per-node Failure Policy, discard-and-redispatch on
   leader loss (IDR-004), saga/compensation for cross-shard flows (IDR-001 / Amendment-006
   as cited there).
7. **Trace emission.** Record every selection, authorization verdict, placement, dispatch,
   retry, timeout and outcome-reference into the decision trace with `correlation_id` and
   Runtime Fingerprint (Vol 9 Part 9; Engine Graph Part 10).

Explicitly NOT a responsibility: committing truth (Kernel's, ORCH-001), owning bindings
(Fabric's, OWN-001), owning policy (Governance's, Vol 9 Part 10), executing capability code
in-process with truth state (Execution node's job, product/host side per the G1 reference).

### 3.2 Inputs

| Input | Source | Frozen basis |
| --- | --- | --- |
| Execution plan / Capability Graph | Control Plane flow: Goal → Orchestrator → Engine Graph → Capability Graph → Execution Plan (Vol 9 Part 8) | Vol 9 Parts 4, 8 |
| Node manifests (Capabilities Required, Determinism, Idempotency Key, Failure/Retry Policy, Timeout, Confidence/Cost/Latency) | Content-addressed manifests | Engine Graph Parts 3, 9 |
| Active bindings per (capability, shard) | `CapabilityRegistry::resolve` | Frozen fabric crate contract |
| Policy decisions (allow/deny/approval-required) | Governance (Vol 17 / Vol 14), enforced not owned | Vol 9 Part 10 |
| Shard key (tenant, workspace) — immutable per entity lifetime | Plan context | SHARD-001 |
| Cluster membership & node health | Consensus layer (joint consensus) + AP presence statistics | IDR-003; IDR-001 CP/AP table |
| Load/queue statistics | AP (CRDT) capability statistics | IDR-001 non-goals / CP-AP boundary |

### 3.3 Outputs

| Output | Nature | Frozen basis |
| --- | --- | --- |
| Placement assignments (invocation → node) | **Plan artifact** — serializable, location-transparent, discardable | ORCH-002; Vol 9 Part 11 |
| Dispatched invocations (idempotency key, correlation_id, pinned binding version) | Idempotent, content-addressable requests | ORCH-004; Engine Graph Parts 3, 7 |
| Authorization verdicts (+ fired policy gates) | Trace records | Vol 9 Parts 9–10 |
| Proposed writes routed to the shard-leader Kernel | The only path to truth | ORCH-001; IDR-001 |
| Decision-trace entries + Runtime Fingerprint (engine versions, model routing, capability bindings, policy set) | Replay substrate | Vol 9 Part 9; ORCH-003 |
| Conformance artifact per run | Certificate + regression record | Vol 9 Part 14; Scenario Conformance Framework Part 9 |
| Backpressure signals (admission deny / degrade / escalate) | Flow control, per Failure Policy | Axis 8; Engine Graph Part 3 |

The scheduler emits **no committed state of its own** — ORCH-002.

### 3.4 Dependencies

- **Downward only (LAYER-001):** Kernel commit gateway (via the shard leader, IDR-001);
  Persistence/WAL for the trace (IDR-005); Query for read-side state the plan needs;
  Capability Fabric registry (resolve-only, side-effect-free read per the frozen trait doc);
  Consensus (membership, leader identity, IDR-003/004).
- **Milestone dependencies:** I2 Cluster Kernel (replicated commit gateway) and I3
  Distributed Query (routed reads) precede I4 in Baseline Part 5. I4's distributed
  behaviour (§3.7) assumes a per-shard replicated Kernel exists; if I4 were attempted
  against the single-node I1 kernel, "placement" degenerates to local dispatch —
  recorded as OQ-1.
- **Contract dependencies (frozen):** `arves-capability-fabric` types; the Bridge line
  protocol (Runtime API); the ACS-001/002 identity guarantees (RUNTIME_FREEZE "Identity")
  which make content-addressed idempotency keys byte-stable across nodes and languages.
- **Anti-dependencies:** no dependency on engine/capability *internals* — scheduling is
  from the manifest alone (Engine Graph Part 9).

### 3.5 Lifecycle

1. **Register/declare** — capabilities declared per shard (`register`), then bound
   (`bind`) with strictly-monotonic versions; append-only supersession (fabric contract,
   citing IDR-005 discipline).
2. **Plan intake** — Execution Planner receives the resolved Capability Graph (Vol 9 Part 8).
3. **Authorize** — policy gates + binding resolution + EffectClass check (§3.1.2).
4. **Admit** — per-shard admission control (backpressure, §3.13).
5. **Place** — node assignment under IDR-001 (compute anywhere).
6. **Dispatch** — idempotent invocation with pinned binding version + idempotency key.
7. **Execute** — provider runs (Data Plane; product/host side in the G1 reference:
   `CapabilityHost.invoke` runs product-layer code and commits each effect via the Bridge).
8. **Commit** — proposed writes flow to the shard leader; followers apply committed
   outcomes (IDR-002).
9. **Observe/feed back** — outcomes to Observation Feedback → Goal Manager (Vol 9 Parts 4, 8).
10. **Retire** — bindings superseded by version bump, never mutated; old traces stay
    replayable against their recorded fingerprint (Engine Graph Part 11).

Scheduler restart at any step is safe: all intermediate state is plan-shaped and
reconstructible (ORCH-002; Vol 9 Part 11 — "the Control Plane is stateless over
Kernel/Persistence, so it can be replicated").

### 3.6 State Model

Three state classes, with three different owners — none owned by the scheduler as truth:

| State | Owner | Durability | Basis |
| --- | --- | --- | --- |
| Bindings (capability→provider, versioned, per shard) | Capability Fabric | Configuration; append-only supersession | OWN-001; fabric crate |
| In-flight schedule (queues, placements, retry counters) | Scheduler (ephemeral) | **Discardable** — reconstructed from plan + trace on restart | ORCH-002; IDR-004 ("in-flight uncommitted work is discarded") |
| Committed outcomes + decision trace | Kernel + Persistence (WAL) | Durable, append-only, replayable | ORCH-001; IDR-005 |

The immutable shard key (tenant/workspace) partitions ALL of the above (SHARD-001).

### 3.7 Distributed Behaviour

- **Compute anywhere, commit through the leader.** Placement may choose any healthy node
  for capability compute; the resulting proposed write is routed to the shard's Raft
  leader, which is the only commit path (IDR-001 refinements). Compute scaling is thereby
  separated from consensus — the I4 scheduler scales workers without touching quorum size.
- **Outcomes replicate, invocations do not.** Followers apply committed outcomes; they
  never re-run the capability (IDR-002; ORCH-003 rationale — nondeterministic engines).
- **Leader loss:** in-flight uncommitted work is discarded; re-election occurs; the
  scheduler re-dispatches under the same idempotency key, and content-addressing makes the
  retry converge to at-most-one committed truth (IDR-004 + ORCH-004; Kernel commit is
  idempotent and content-addressed per RUNTIME_FREEZE "Truth"; RCR-005 additionally rejects
  same-hash/different-payload re-proposals).
- **Membership churn:** joint consensus (IDR-003); the scheduler treats node
  arrival/departure as AP presence information and never blocks commit on it.
- **Cross-shard flows:** no distributed transactions; sagas/compensation only (IDR-001:
  "No cross-shard atomic commit in v1"). A plan spanning shards is decomposed into
  single-shard-atomic steps with compensating steps — design detail deferred to the new IDR
  (§6, OQ-4).
- **Location transparency:** plans and dispatch envelopes carry no in-process shared-memory
  assumptions (Vol 9 Part 11).

### 3.8 Concurrency

- **Unit of ordering = the shard.** Commits serialize per shard through its Raft log
  (IDR-001); compute is embarrassingly parallel across nodes and across shards.
- **Within a shard**, the Engine/Capability Graph's edges define the only ordering
  constraints (DAG data/ordering dependencies, Engine Graph Part 5); independent nodes
  dispatch concurrently.
- **Race: duplicate dispatch** (two workers pick up the same invocation after a scheduler
  failover) — resolved by construction: same idempotency key → same content address → one
  committed truth (ORCH-004). This must be proven executable, not assumed (§4, P-ORCH-004).
- **Race: rebind during flight** — a dispatched invocation pins its `BindingVersion`; a
  concurrent `bind` (strictly monotonic, fabric contract) supersedes future dispatches only.
  The pinned version is recorded in the Runtime Fingerprint so replay uses the binding that
  actually ran (Vol 9 Part 9; CAP-009 states this directly but is **(PROPOSED — CCP-GATE
  required)** — the registered basis is ORCH-003's "same … Capabilities" clause).
- **No locks across planes:** the fabric `resolve` is a side-effect-free read (frozen trait
  doc); the scheduler never holds fabric or kernel locks across a dispatch.

### 3.9 Failure Modes

| ID | Failure | Containment (frozen basis) |
| --- | --- | --- |
| F-UNBOUND | Capability has no active binding in the shard | Hard deny at authorization; `RegistryError::Unbound` surface (fabric contract). Plan-level Failure Policy decides fail/degrade/escalate (Engine Graph Part 3). |
| F-POLICY | Policy gate denies / requires approval | Enforced HITL/approval checkpoint (Vol 9 Part 10); safety-critical axis 7 requires the gate to BLOCK (Scenario Conformance Framework Parts 5–6). |
| F-NODE | Compute node dies mid-execution | Invocation is idempotent/content-addressed → re-dispatch elsewhere converges to one committed truth (ORCH-004); no partial truth because nothing committed (ORCH-001). *At-most-once for the EXTERNAL side effect relies on the provider's declared idempotency (see §3.1.4 caveat; v1.1 debt #2).* |
| F-LEADER | Shard leader lost | IDR-004: in-flight uncommitted work discarded; re-election; re-dispatch under same key. |
| F-TIMEOUT | Capability exceeds declared Timeout | Enforced per manifest (Engine Graph Parts 3, 10); Retry Policy applies; cancellation semantics are OQ-6 (CAP-008 is PROPOSED; runtime `CancellationToken` is single-process today). |
| F-OVERLOAD | Shard/tenant floods the cluster | Per-shard admission bounds; deny/degrade per Failure Policy; other tenants unaffected (axis 8 property "tenant isolation held"). *Feedback-loop hazard: if every admission denial is an individually consensus-committed trace entry (Raft log = WAL = decision trace, IDR-001/005), the backpressure mechanism itself adds commit load on the leader it protects — see OQ-10.* |
| F-POISON | Capability fails deterministically on every retry | Retry Policy exhausts → Failure Policy (fail/degrade/escalate); poison work must not wedge the shard queue. |
| F-DRIFT | Scheduler starts persisting outcomes / owning a truth-like queue | **Architectural failure**, blocked by ORCH-001/ORCH-002 executable proofs (§4) and the LAYER-001 architecture gate. |
| F-TAMPER | Provider substituted for a bound one | Binding pins `ProviderId` + version; the G1 reference additionally shows content-addressed artifact + code-hash verification at install (`kit.mjs` `verifyArtifact`/`codeHash`/`manifestBinds`). Runtime-side equivalent is an RCR-scoped decision (§6). |

### 3.10 Recovery

- **Scheduler crash:** restart empty; reload plans from the Control Plane and
  outcome-state from Query/trace; recompute placements. Nothing is lost because nothing
  scheduler-local was authoritative (ORCH-002; Vol 9 Part 11).
- **Partial run:** completed (committed) invocations are visible as truth; incomplete ones
  re-dispatch idempotently; the run continues from the trace — exactly the WAL-recovery
  discipline already proven for I1 (RUNTIME_FREEZE "Persistence · Replay · Audit").
- **Compensations:** cross-shard partial progress unwinds via sagas (IDR-001), never via
  cross-shard rollback.

### 3.11 Replay

- Replay reproduces a run from *Goal, State, Policies, Capabilities, Runtime Fingerprint*
  via the recorded decision trace, **not** recomputation (ORCH-003 verbatim).
- Scheduling consequences: (a) placement decisions are trace entries, so replay does not
  re-place — it re-reads; (b) `Nondeterministic` capabilities replay from recorded
  outcomes; only `Deterministic | Seeded` may be recomputed (Engine Graph Part 6; ENG-004
  states this as an invariant but is **(PROPOSED — CCP-GATE required)**); (c) the
  fingerprint captures capability bindings + versions so replay binds what actually ran
  (Vol 9 Part 9).
- The Raft log = WAL = decision trace convergence (IDR-001) means I4 adds trace *content*
  (scheduling decisions), never a second log.

### 3.12 Consistency

- Committed capability effects: **CP, linearizable per shard** (IDR-001 consequences).
- Reads feeding capability inputs: the three tiers — linearizable via leader read-index /
  bounded-staleness follower read / eventual replica (IDR-001 Read Consistency Tiers).
  Which tier a plan node demands is a manifest/plan concern — OQ-3.
- Scheduler metadata (load, presence, statistics): **AP by explicit spec permission**
  (IDR-001 non-goals; CP/AP summary table row "Presence / capability statistics — AP
  (CRDT)"). Placement decisions may therefore be made on stale load data without violating
  any invariant — but a *commit* is never gated on AP data.

### 3.13 Availability

- Truth availability is bounded by per-shard quorum (CP choice, IDR-001) — during a shard's
  quorum loss, that shard's commits stall; **other shards keep scheduling** (SHARD-001
  isolation).
- The scheduler itself is replicable (stateless over Kernel/Persistence, Vol 9 Part 11);
  N schedulers may run active-active because duplicate dispatch is harmless (ORCH-004).
- Backpressure prefers **degrade** over collapse where the manifest allows it
  (Failure Policy `degrade`, Engine Graph Part 3); read-only (`Pure`) capabilities may
  continue against follower reads during leader unavailability.

### 3.14 Scalability

- **Consensus does not scale with compute:** adding capability workers adds zero Raft
  participants (IDR-001 refinement — compute separated from consensus).
- **Shard = scaling unit:** per-tenant/workspace Raft groups scale horizontally
  (IDR-001: global single-leader rejected because it "does not scale to 10,000 nodes").
- **Planning metadata drives placement cost-awareness** (Cost/Latency, Engine Graph Part 8)
  — machine-readable by spec so schedulers can bin-pack without prose interpretation.
- Bottleneck watch: per-shard commit throughput (single leader per shard) — mitigations
  (batching) are named v1.1 debt ("Kernel batch-commit", RUNTIME_FREEZE backlog #3) and are
  RCR-gated, not I4-local hacks.

### 3.15 Performance

- Targets are deliberately **not invented here** (no frozen SLO exists — recording numbers
  would be architecture-invention). Performance obligations that ARE frozen: Timeout
  enforcement per node, declared Latency as planning metadata (Engine Graph Parts 3, 8),
  and the constitution's ordering "Never optimize before correctness".
- The design keeps the hot path allocation-light by construction: resolve (read-only) →
  authorize (policy check) → dispatch (serialized envelope). Measurable budgets belong in
  the new IDR (§6) with benchmarks as evidence — OQ-7.

### 3.16 Security

- **Capability-gated by contract:** the frozen work chain is capability-gated end-to-end
  (RUNTIME_FREEZE "Cognitive work chain"); no binding → no invocation (F-UNBOUND).
- **Policy enforcement without ownership:** approval gates and HITL checkpoints sequenced
  by the Control Plane; policy owned by Vol 17 / Vol 14 (Vol 9 Part 10).
- **Supply-chain integrity (reference pattern):** the G1 host refuses tampered artifacts,
  mismatched code hashes, unbound manifests, and re-runs certification at install rather
  than trusting flags (`kit.mjs` `install`). I4's runtime-side equivalent (who verifies
  provider integrity at bind/dispatch time) is an RCR-scoped decision — OQ-8.
- **Honest limitation (frozen record):** v1.0's threat model is a *trusted single host*;
  `Kernel::commit` carries no principal/authN/authZ; cryptographic signatures +
  authenticated commit are v2.0 debt (RUNTIME_FREEZE #8, partially addressed by RCR-002
  hash-chain digest). Therefore I4's "authorization" is **capability/policy gating, not
  cryptographic principal authentication** — claiming otherwise would violate the freeze
  record's own language ("Public docs must not imply cryptographic tamper-resistance…").
  End-to-end authenticated scheduling requires the v2.0 RCR path.
- **Determinism honesty:** determinism is *declared* (manifest field) and only best-effort
  probed at certification (kit.mjs docstring: "DETERMINISM IS A BEST-EFFORT PROBE, NOT
  ENFORCEMENT"); fabric-derived/enforced idempotency keys are v1.1 debt
  ("Engine-enforced determinism", RUNTIME_FREEZE backlog #2). I4 scheduling MUST NOT
  assume purity it cannot verify — replay therefore always prefers trace over recompute
  for anything not provably `Deterministic|Seeded`.

### 3.17 Observability

- Every dispatch carries `correlation_id` (already in the Event Envelope, Vol 9 Part 11)
  and lands in the decision trace (Engine Graph Part 10).
- Metrics/logs/tracing are **AP by spec** (IDR-001 non-goals) — the observability plane may
  lag without harming truth.
- Each run emits the Vol 9 Part 14 conformance artifact (expanded graph, order,
  arbitration, policy gates) — observability and conformance share one artifact.

### 3.18 Metrics

Candidate metric set (names illustrative, not normative — the normative requirement is only
that they exist on the AP plane and never gate commits):

- per-shard: queue depth, admission denials, in-flight count, commit latency,
  leader-changes observed;
- per-capability: invocations, retries, timeouts, failure-policy activations
  (fail/degrade/escalate counts), binding version in use;
- per-node: placement count, utilization (feeds AP placement statistics, IDR-001 CP/AP
  table);
- per-run: trace completeness (every dispatch has a trace entry — feeds the ORCH-003
  proof), idempotency-dedupe hits (feeds the ORCH-004 proof).

### 3.19 Auditability

- The append-only WAL/decision trace is the audit record (IDR-005; RUNTIME_FREEZE
  "Persistence · Replay · Audit"), now enriched with authorization verdicts and fired
  policy gates (Vol 9 Part 9 lists "policy evaluations" as trace content; axis 10
  "Policy-heavy Governance: dense policy evaluation and audit").
- Tamper-evidence of the trace: SHA-256 hash-chain digest exists (RCR-002); signatures are
  v2.0 debt — audits must state this scope honestly.
- Every scheduled invocation is reconstructible: who asked (correlation), what was allowed
  (policy gates), what was bound (binding + version), where it ran (placement entry), what
  became truth (committed outcome by content address).

### 3.20 Trade-offs

| Chosen | Rejected | Why (frozen basis) |
| --- | --- | --- |
| Per-shard CP commit + anywhere-compute | Global scheduler with global lock | IDR-001 rejected global single-leader; compute/consensus separation |
| Idempotent re-dispatch (at-least-once dispatch, at-most-once truth) | Exactly-once dispatch machinery | ORCH-004 makes dedupe free at the truth boundary; exactly-once dispatch is unprovable under partition |
| Placement = discardable plan | Persistent scheduler DB | ORCH-002; IDR-004 discard rule; restartability |
| Stale-tolerant AP load data for placement | Linearizable cluster state for placement | IDR-001 CP/AP boundary — statistics are explicitly AP; commits never depend on them |
| Declared determinism honoured, trace-first replay | Trusting recompute for all capabilities | ORCH-003; Engine Graph Part 6; kit.mjs probe limits + v1.1 debt #2 |
| Backpressure as admission control per shard | Global rate limiter | SHARD-001 tenant isolation; axis 8 property |
| Selection in Planner, bindings in Fabric | A "smart fabric" that selects | Vol 9 Part 3 critical rule; CAP-002 says the same but is (PROPOSED — CCP-GATE required) |

Costs accepted: per-shard leader is a throughput ceiling (mitigation = RCR-gated batching);
AP load data can misplace under churn (correctness unaffected, only efficiency); sagas are
harder to reason about than transactions (mandated by IDR-001's no-cross-shard-atomicity).

### 3.21 Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| R1: I4 built before I2/I3 substrate exists | High | Baseline Part 5 ordering respected; G2 stays closed; OQ-1 to maintainer |
| R2: Scheduler accretes state and becomes a second truth owner (drift) | High | ORCH-001/002 executable proofs mandatory (§4); architecture gate |
| R3: CAP-00n treated as normative prematurely | Medium | Every reference in this doc carries the (PROPOSED — CCP-GATE required) marker; enforcement blocked until CCP |
| R4: "Authorization" oversold as authN (v1.0 has no principal on commit) | Medium | §3.16 honest-limitation language; v2.0 RCR path named |
| R5: Determinism assumed rather than declared → replay divergence | Medium | Trace-first replay; v1.1 debt #2 tracked; no recompute of nondeterministic nodes |
| R6: Cross-shard capability flows creep toward distributed transactions | Medium | IDR-001 hard rule; saga decomposition in the new IDR |
| R7: Distributed cancellation is undefined (token is single-process) | Medium | OQ-6; CAP-008 is PROPOSED; timeout enforcement is the v1 backstop |
| R8: Backpressure policy invented without an IDR | Low | §6 instrument list makes IDR-007 a precondition |

### 3.22 Open Questions

- **OQ-1 (sequencing):** May I4 be designed-and-built against a single-node I1 Kernel as a
  degenerate cluster, or must I2 (Cluster Kernel) land first? Baseline Part 5 orders
  I2→I3→I4; the maintainer prep-mode ruling covers design only. *Unknown — maintainer
  decision.*
- **OQ-2 (code home):** When G2 opens, does scheduling behaviour land as (a) an RCR that
  adds behaviour beside the contract-only `arves-capability-fabric` / `arves-control-plane`
  crates, or (b) a new additive crate in the workspace? Both are runtime changes → RCR;
  neither may modify the frozen contracts. *Unknown — Runtime Team triage.*
- **OQ-3 (read tier selection):** Which IDR-001 read-consistency tier feeds a capability's
  `Reads`? Per-manifest field, per-plan default, or per-shard policy? The manifest ABI has
  no tier field today; adding one is an Engine Graph minor-version question (spec-frozen —
  would need CCP/next-major analysis). *Unknown.*
- **OQ-4 (saga contract):** The concrete compensation contract for cross-shard capability
  flows (who authors compensating capabilities; are they manifest-declared?). IDR-001
  cites Amendment-006 (sagas) — the scheduling-level mechanics need the new IDR. *Unknown.*
- **OQ-5 (CAP-00n ratification set):** Which of CAP-001..009 should I4 sponsor through the
  CCP-GATE with conformance scenarios (obvious candidates: CAP-003/004/007/009, whose
  registered shadows — ORCH-004/001/003 — I4 proves anyway)? *Maintainer/CCP decision.*
- **OQ-6 (distributed cancellation):** CAP-008 (PROPOSED) wants cooperative idempotent
  cancellation; the implemented `CancellationToken` (RCR-001) is a single-process
  `Arc<AtomicBool>`. Cross-node cancellation propagation is undefined in the frozen corpus.
  v1 backstop = Timeout enforcement. *Unknown — likely IDR + CCP for CAP-008.*
- **OQ-7 (performance budgets):** No frozen SLOs exist for scheduling latency/throughput.
  Where do budgets get recorded — the new IDR with benchmark evidence? *Unknown.*
- **OQ-8 (runtime-side artifact verification):** Should the runtime-side dispatch path
  re-verify provider artifact integrity (the kit.mjs install-gate pattern) or trust the
  host boundary? Runtime-side verification touches frozen crates → RCR. *Unknown.*
- **OQ-9 (backpressure signal surface):** How does admission-denial propagate to the Goal
  Manager (Observation Feedback per Vol 9 Part 8, or a synchronous deny)? Both fit the
  frozen flow; pick in IDR-007. *Unknown.*
- **OQ-10 (trace-entry commit granularity):** IDR-001 fixes "the Raft log IS the WAL IS the
  decision trace", and §3.1.7/§3.3 put every selection, authorization verdict, placement,
  dispatch, retry, timeout and admission denial into that trace — i.e. onto the per-shard
  Raft commit path. Which scheduling decisions must be *individually* consensus-committed
  trace entries, and which may be batched/aggregated (relates to v1.1 debt #3, Kernel
  batch-commit, RUNTIME_FREEZE backlog)? Under F-OVERLOAD the answer determines whether
  backpressure itself loads the leader it protects (§3.9). The frozen corpus does not fix
  the granularity. *Unknown — route to IDR-007, with any batching mechanics via the
  existing v1.1 debt #3 RCR.*

---

## 4. Invariant Mapping (registered-normative) + Executable Proof Obligations

Per the constitution: *"No invariant may remain proof-only once its owning component is
implemented; each must gain an executable runtime proof during its milestone."* The proofs
below are OBLIGATIONS for the (G2-gated) build, named now so the design is falsifiable.

| Invariant (registered) | What I4 must uphold | Executable proof the build will need |
| --- | --- | --- |
| **OWN-001** (every state has exactly one owner) | Bindings owned solely by the Fabric; queues/placements owned solely by the scheduler as *ephemeral* state; outcomes owned solely by the Kernel | Property test: no code path outside the Fabric mutates a binding; scheduler restart loses zero authoritative state (kill-and-restart test reconstructs identical schedule from plan+trace) |
| **LAYER-001** (downward-only dependencies) | Scheduler depends downward on Fabric/Kernel/Persistence/Query/Consensus; no sideways peer calls; cross-cutting via Control Plane / Event Fabric | Extend the existing executable architecture gate (already enforcing LAYER-001/OWN-001 per CLAUDE.md Maintainer Note) to the new scheduling crate(s)' dependency edges |
| **SHARD-001** (partition by tenant/workspace; immutable key) | All queues, admissions, placements, bindings scoped by shard key; key never rewritten in flight | Two-tenant isolation test at scheduling level (pattern: RCR-007 `behaviour_8_two_tenant_isolation`): flood tenant A, assert tenant B admission/latency unaffected and no cross-shard binding resolution |
| **ORCH-001** (Control Plane owns no truth) | Scheduler/Planner never commits; only proposed writes to the shard-leader Kernel | Negative test: scheduler component has no commit capability (API-level impossibility) + trace audit shows all truth transitions carry Kernel commit provenance |
| **ORCH-002** (plans, never persistent state) | Placements/schedules serializable, discardable plan artifacts | Crash-restart test: discard all scheduler state mid-run; run completes with identical committed truth set |
| **ORCH-003** (replayable from trace, not recomputation) | Every scheduling decision (selection, authorization, placement, retry) is a trace entry; fingerprint pins binding versions | Replay test: re-run from recorded trace reproduces identical decision sequence and truth set with zero capability re-execution of nondeterministic nodes |
| **ORCH-004** (idempotent, content-addressable invocations) | Idempotency key per dispatch; duplicate/racing dispatch converges to one truth | Duplicate-dispatch test: dispatch the same invocation from two scheduler replicas / across a leader failover; assert exactly one committed effect (Kernel dedupe by content address; RCR-005 integrity guard) |

**PROPOSED invariants touched by this design — informative only, each marked:**

| ID | Relation to I4 | Status |
| --- | --- | --- |
| CAP-001 (fabric owns only registry+bindings) | Restated by the frozen fabric crate's own contract | (PROPOSED — CCP-GATE required) |
| CAP-002 (selection = Control Plane; fabric never selects) | §3.1 split follows Vol 9 Part 3 (registered basis) | (PROPOSED — CCP-GATE required) |
| CAP-003 (idempotent, content-addressable invocation) | Covered by registered ORCH-004 | (PROPOSED — CCP-GATE required) |
| CAP-004 (no direct truth mutation; proposed writes via Kernel) | Covered by registered ORCH-001 + fabric `EffectClass::ProposesWrite` | (PROPOSED — CCP-GATE required) |
| CAP-005 (manifest-declared, runtime-bound at execution time) | §3.1.1 binding resolution; basis Engine Graph Parts 3 & 10 | (PROPOSED — CCP-GATE required) |
| CAP-006 (side-effect-honest, declared + traced) | §3.19; basis Engine Graph Part 4 / Vol 9 Part 6 | (PROPOSED — CCP-GATE required) |
| CAP-007 (correlation_id + trace for replay) | Covered by registered ORCH-003 + Engine Graph Part 10 | (PROPOSED — CCP-GATE required) |
| CAP-008 (cooperative idempotent cancellation) | OQ-6; single-process token only today | (PROPOSED — CCP-GATE required) |
| CAP-009 (versioned bindings in Runtime Fingerprint) | §3.8/§3.11 pinning; basis Vol 9 Part 6/9 | (PROPOSED — CCP-GATE required) |
| ENG-003/004/005 (engine-side idempotency, replay class, manifest) | Consumed as manifest semantics; registered bases ORCH-003/004 | (PROPOSED — CCP-GATE required) |

None of the CAP-00n/ENG-00n rows may be *enforced* as conformance criteria until each passes
the CCP-GATE with its own conformance scenario (Reference Lifecycle Part 6; Invariant
Registry Part 5).

---

## 5. Conformance Plan

### 5.1 Axes instantiated (Scenario Conformance Framework Part 5)

| Axis | Why I4 must instantiate it |
| --- | --- |
| 4 — Multi-step Planning | Capability Graph produced from the plan; selection per node |
| 7 — Safety-critical | Authorization/policy gates MUST block unauthorized/unsafe invocations |
| 8 — High-volume Streaming | Backpressure + tenant isolation at scale (the I4 backpressure requirement IS this axis) |
| 10 — Policy-heavy Governance | Dense policy evaluation + audit of authorization verdicts |
| 12 — Recovery & Replay | Deterministic replay of scheduling decisions from the trace (ORCH-003) |

### 5.2 Reference scenarios exercised (Part 6)

- **Warehouse Robot Dispatch** (6+7+11+4): *"Safety gate blocks unsafe plan; Engine Graph
  produced; execution idempotent (ORCH-004)"* — I4 supplies the capability-gated dispatch
  and idempotent execution assertions.
- **Enterprise Knowledge Query** (1+8+9): *"Tenant isolation held; … control plane owns no
  truth (ORCH-001)"* — I4 supplies the per-shard backpressure/isolation assertions.
- A new **distributed scheduling scenario** (leader failover mid-dispatch; duplicate
  dispatch; shard flood) will be needed; per Part 11 the suite is versioned with a single
  owner — adding a scenario is a suite change under that governance, and any new *invariant*
  it asserts must ride a CCP (CCP-GATE).

### 5.3 Level and node probes

- Target level: **L3 Distributed** — *"Conformance preserved across distributed
  deployment"* (Part 10), building on L2 (ORCH-001..004 conformant) which the Control
  Plane/Engine nodes must already hold.
- Node probes emitted (Part 7): **Capability** — "Capability selected and bound per plan";
  **Execution** — "Idempotent, addressable action with correlation_id"; **Control Plane** —
  "ORCH-001..004 upheld; no truth produced".
- Verdict semantics (Part 8): structural/property/invariant-based; PASS requires all
  registered invariants + critical isolation/safety properties; a failed tenant-isolation
  or safety-gate property is FAIL, not PARTIAL.
- Result statement format (Part 11): "N% at Level L3 against Framework v1 / Spec v1.0".

### 5.4 Per-milestone Success Criteria, concretely (Constitution "Success Criteria")

| Criterion | Concrete meaning for I4 |
| --- | --- |
| Architecture PASS | Executable architecture gate green over the new crate(s): downward-only edges (LAYER-001), single-ownership map (OWN-001), no new layers |
| Conformance PASS | §5.1 axes green at L3 in the versioned suite; the two Part 6 scenarios above emit passing conformance artifacts (Part 9 schema) |
| Certification PASS | Verdict derived per Part 8 from a real run's artifact (live-probe pattern of RCR-008..010 extended to the Capability/Execution nodes), graded by the maintainer-independent harness (FOUNDATION posture) |
| Independent Review PASS | The §4 proof table re-reviewed as if externally submitted; verdict recorded PASS/PARTIAL/FAIL per the constitution's review rubric |
| Invariant coverage 100% | All 7 registered invariants have named, biting, executable tests (the §4 right-hand column — no structural-citation-only rows, per the RCR-007 precedent) |
| Replay PASS | ORCH-003 replay test (§4): trace-driven re-run, zero recompute of nondeterministic capability nodes, identical truth set |
| Distributed tests PASS | Leader-failover re-dispatch, duplicate-dispatch dedupe, membership churn, two-tenant flood isolation — all green |
| No architecture/spec drift | Freeze gate green (266-file frozen surface untouched); every design claim in this doc still traceable to its cited frozen source |

---

## 6. NON-GOALS and Required Change Instruments

### 6.1 Explicit NON-GOALS of I4

- **No implementation now.** G2 is closed; this package is design-only under the prep-mode
  ruling.
- **No modification of the frozen fabric contract** (`CapabilityRegistry`,
  `CapabilityBinding`, `InvocationContract`, `RegistryError`) — I4 consumes it as-is.
- **No new layers, no new invariants, no "smart fabric."** Selection stays in the Planner
  (Vol 9 Part 3); the fabric stays a lookup/validation surface.
- **No cross-shard atomic commit / distributed transactions** (IDR-001 hard rule) — sagas
  only.
- **No cryptographic authN/authZ on commit** — v2.0 debt per RUNTIME_FREEZE #8; I4's gating
  is capability/policy-level and says so.
- **No enforcement of CAP-001..009 / ENG-001..005** until CCP-ratified with conformance
  scenarios (CCP-GATE).
- **No marketplace / multi-agent scheduling** (Baseline Part 3 defers Marketplace to v2;
  multi-agent is I5, not I4).
- **No exactly-once dispatch machinery** — at-least-once dispatch + at-most-once truth via
  ORCH-004 is the model.
- **No golden-output conformance** — property/invariant assertions only (Framework Part 8).
- **No performance optimization before correctness** (constitution ordering).

### 6.2 Change instruments any frozen-surface change would require

| Needed change | Instrument | Route |
| --- | --- | --- |
| Scheduling behaviour code in `runtime/` (new crate or behaviour beside contract-only crates) | **RCR** (additive → v1.1 minor) | `runtime/RUNTIME_FREEZE_v1.0.md` RCR process, steps 1–4, with its own ED-006 destroy→repair→prove cycle |
| Placement algorithm, backpressure/admission policy, saga mechanics, read-tier defaults | **New IDR** (proposed IDR-007 "Capability Placement & Backpressure") | Change Management table: "Engineering decision → IDR"; IDRs implement, never change, the spec (IDR Batch 1 Preamble) |
| Ratifying any CAP-00n as normative (e.g. CAP-008 cancellation before distributed cancel is built) | **CCP Amendment with a conformance scenario** | Reference Lifecycle Part 6, CCP-GATE hard rule |
| New manifest field (e.g. read-consistency tier) — touches the frozen Engine Graph ABI | **CCP / next-major analysis** — spec text cannot change in v1 | Constitution Change Management: "Specification change → Next Major Version"; Engine Graph Part 11 versioning rules |
| New conformance scenario / suite addition | Suite-governance change (single owner + changelog), invariant additions via CCP | Scenario Conformance Framework Part 11 |
| Any product-discovered runtime gap during later product use of the scheduler | **RCR, never a product-side edit** | IDR-006 + RUNTIME_FREEZE ("A runtime gap found during product work is a Runtime Change Request") |
| Kernel batch-commit for shard throughput | Existing recorded v1.1 debt #3 → RCR | RUNTIME_FREEZE v1.1 backlog |
| Engine/fabric-enforced determinism (stop trusting declared purity) | Existing recorded v1.1 debt #2 → RCR | RUNTIME_FREEZE v1.1 backlog |

---

## 7. Critical Self-Review (destroy pass, summary)

Attempted refutations and their outcomes:

1. *"The scheduler is a new architectural layer"* — refuted: every role maps to a frozen
   component row (Vol 9 Part 4 Capability/Execution Planner; Part 12 mechanical scheduler).
2. *"Placement state is hidden truth"* — refuted by design (discardable plan, ORCH-002) but
   only PROVEN by the §4 crash-restart obligation; flagged as R2 until executable.
3. *"Capabilities-run-anywhere is an invention"* — partially conceded: IDR-001's refinement
   text says **engines** run anywhere. Extending the same rule to capability compute is an
   inference from identical constraints (pure compute, commit-through-leader, ORCH-004) —
   honest status: **inference, to be ratified in IDR-007**, not a frozen quote.
4. *"Backpressure semantics are invented"* — conceded and contained: axis 8 names the
   requirement; the policy is an engineering decision routed to IDR-007 (§6), not smuggled
   into this design as fact.
5. *"Authorization overclaims"* — conceded and contained: §3.16 states the trusted-host
   v1.0 threat model verbatim from the freeze record; no cryptographic claim is made.
6. *"CAP-00n leakage"* — checked: every CAP/ENG reference in this document carries the
   (PROPOSED — CCP-GATE required) marker; enforcement is nowhere assumed.

Verdict on the design at prep-mode scope: **PASS to hold as a design package**; BUILD
remains blocked on G2 + IDR-007 + RCR triage (OQ-1/OQ-2).

---

*Traceability note: every load-bearing claim above cites its frozen source inline
(document + part/section). Where the corpus is silent, the claim is listed under §3.22 Open
Questions or routed to an instrument in §6 — never assumed.*

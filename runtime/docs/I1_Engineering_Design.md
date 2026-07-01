# I1 — Distributed Runtime · ENGINEERING DESIGN

> **Artifact class:** AEOS Phase 1–9 artifact (Implementation Era). This is an **Engineering Design**, *not* a specification.
> **Status:** DRAFT for Independent Architecture Review (AEOS Phase 14).
> **Milestone:** `I1 Distributed Runtime` (frozen Baseline milestone set: **I1** → I2 Cluster Kernel → I3 Distributed Query → I4 Capability Scheduling → I5 Multi-Agent Runtime → I6 Reference Products).
> **Spec version pinned:** UCS/UCI v1.0 (FROZEN, Specification Era closed 2026-07-01).
> **Reference runtime:** Rust (`runtime/` Cargo workspace). Design is language-agnostic; Rust-specific notes are called out explicitly.
>
> **Governing rule (never violated):** Chain `Theory → Specification → Contracts → Behaviour → Conformance → Implementation`. Implementation **proves** the spec; it **never** changes it. If a spec issue is found, STOP and classify (CCP / Amendment / IDR / next-major) — do not implement around it.

---

## 0. Scope and Framing

I1 delivers the **walking skeleton** of the ARVES reference runtime: a single-process, **single-shard** distributed runtime whose commit path, ownership boundaries, layer stack, and replay model are *identical in shape* to the multi-node target, so that growing to **per-shard Raft** in I2 is an incremental change of the consensus module, not a rearchitecture.

Two planes are honored throughout (never merged):

- **Control Plane** — *decides*. Owns the Plan / Engine Graph. Owns **no truth** and **no persistent state** (ORCH-001, ORCH-002).
- **Data Plane** — *carries*. The Kernel owns truth and is the sole commit gateway; Persistence/LCW/Query/Engine/Capability/Execution carry, store, project, infer, and act.

Layer stack is **downward-only** (LAYER-001): `Reality → Information Platform → Kernel → Persistence → LCW → Query → Engine → Capability → Execution`, with the Control Plane cross-cutting.

**Explicit non-goals for I1** (deferred to later milestones, so we do not invent architecture): multi-node clustering, live leader election under real network partition, joint-consensus membership changes, cross-shard sagas at scale, distributed query fan-out (I3), capability scheduling (I4), multi-agent runtime (I5). I1 stubs their **seams** but does not implement them.

---

## 1. Architecture Readiness Check for I1

Readiness gate answered *before* design proceeds (AEOS Phase 1). Verdict per row: **READY** / **READY-WITH-SEAM** (interface exists, behaviour deferred by milestone) / **BLOCKED**.

| # | Readiness question | Finding | Verdict |
|---|---|---|---|
| 1 | Is the specification frozen and pinned? | UCS/UCI v1.0 frozen 2026-07-01; Freeze Record + Baseline in corpus. | READY |
| 2 | Are the governing invariants registered and unambiguous for the truth path? | `ORCH-001..004`, `OWN-001`, `LAYER-001`, `SHARD-001` are **registered-normative**. | READY |
| 3 | Is the distribution model decided (no open architectural ambiguity)? | `IDR-001..005` fix per-shard Raft, outcomes-only replication, sagas, joint consensus, append-only WAL. | READY |
| 4 | Are ownership boundaries single-owner and non-overlapping? | Kernel=truth, LCW=Working Memory, Persistence=durable store, Control Plane=plans (no truth). OWN-001 satisfied by design. | READY |
| 5 | Is the layer stack fixed so no new layer is introduced? | LAYER-001 layer list is closed; every crate maps 1:1 to a layer. | READY |
| 6 | Is a conformance instrument available to ratify behaviour? | Scenario Conformance Framework + `arves-conformance` crate (12 axes → scenarios → probes → verdict). CCP-GATE: no scenario, no ratification. | READY |
| 7 | Does the crate workspace exist and compile as a skeleton? | 14 crates present, `cargo check` builds the interface-only skeleton. | READY |
| 8 | Is the Kernel commit gateway well-defined enough to build? | `G-001` (Kernel sole truth + commit gateway) is **PROPOSED** (informative, pending CCP). ORCH-001 (registered) already fixes "only Kernel owns truth", which is sufficient for I1; commit-gateway *exclusivity* rides on ORCH-001 + OWN-001, not on the unratified G-001. | READY-WITH-SEAM |
| 9 | Are the per-component contracts (LCW/Query/Persist/Engine/Cap) ratified? | `LCW-001`, `QUERY-001`, `PERSIST-001`, `CAP-001..009`, `ENG-001..005` are **PROPOSED** (pending CCP-GATE). I1 builds against ORCH/OWN/LAYER/SHARD and treats the proposed ones as **design intent only** — enforced by construction, not asserted as registered invariants. | READY-WITH-SEAM |
| 10 | Is consensus needed *now* to prove the commit path? | No. Single-shard-first: a **degenerate consensus** (log-of-one, quorum=1) exercises the same commit path. Multi-node Raft is I2. | READY |
| 11 | Is replay defined as trace-driven, not recomputation? | ORCH-003 mandates replay from recorded decision trace; IDR-005 makes the WAL that trace. | READY |
| 12 | Is idempotency / content-addressing available for invocations? | ORCH-004 mandates every engine/capability invocation idempotent + content-addressable. Requires a canonical content-hash utility — build in I1. | READY |

**Readiness verdict: READY to design.** No BLOCKED rows. Two READY-WITH-SEAM rows both resolve to "build against registered invariants; keep proposed invariants as by-construction design intent, gated behind future CCP." No architectural ambiguity remains that would require a new IDR before I1 design can proceed.

---

## 2. Affected UCI Nodes

I1 touches the whole Data-Plane spine plus the Control Plane seam, but with **shallow** (interface + minimal behaviour) depth for most nodes and **deep** (real behaviour) depth for the truth path. Each UCI node maps 1:1 to a workspace crate.

| UCI node (layer) | Crate | I1 depth | Role in I1 walking skeleton |
|---|---|---|---|
| Information Platform | `arves-information-platform` | Shallow | Emits **proposed writes** to the Kernel (never direct truth). One in-process connector stub. |
| **Kernel** (truth) | `arves-kernel` | **Deep** | Sole commit gateway. Validates proposed write → produces committed **Outcome** → hands to consensus → applies on commit. |
| Persistence | `arves-persistence` | **Deep** | Append-only **WAL** + snapshot. WAL = Raft log = decision trace (IDR-005/002). |
| LCW (Working Memory) | `arves-lcw` | Medium | Single owner of Working Memory / live state; derived from committed outcomes, uses-only by Engine + Control Plane. |
| Query | `arves-query` | Medium | Strictly read-only projections; exposes the three read tiers. |
| Engine Fabric | `arves-engine-fabric` | Medium | Pure/stateless engine behind the Engine ABI; **runs anywhere**, produces inference not truth; result committed only via leader. |
| Capability Fabric | `arves-capability-fabric` | Shallow | Registry + bindings (owns bindings, never truth/plans). One demo capability. |
| Execution | `arves-execution` | Medium | Performs the action; routes **outcome as a proposed write** to the Kernel; cancellation seam (Amendment-005), saga seam (Amendment-006). |
| **Control Plane** (cross-cutting) | `arves-control-plane` | **Deep** | Orchestrator: builds a Plan / Engine Graph, drives execution, records the **decision trace**; owns no truth, no persistent state. |
| Consensus (truth path) | `arves-consensus` | **Deep (degenerate)** | Per-shard Raft interface; I1 provides a single-node/quorum-1 implementation exercising the real API. |
| Ontology (type system) | `arves-ontology` | Medium | `uci.*` type registry + mandatory aspects (Identity/Provenance/Trust/Temporal/TenantScope) used to type every Outcome. |
| Invariants (cross-cutting) | `arves-invariants` | **Deep** | Machine-checkable invariant markers + property-test scaffolding for ORCH/OWN/LAYER/SHARD. |
| Conformance (test) | `arves-conformance` | Medium | Scenario harness; I1 ships the scenarios that ratify the I1 commit/replay behaviour. |
| Runtime binary | `arves-runtime` | **Deep** | Wires one shard end-to-end (the I1 entry point). |

**Reality** is an external UCI node (the observed/acted-upon world); in I1 it is represented only by the Information Platform connector stub and the Execution action sink. Nodes not touched at all in I1: none structurally, but **Distributed Query fan-out**, **Capability Scheduling**, and **Multi-Agent** *behaviours* are out of scope.

---

## 3. Specification / Contract / Invariant / Ownership / IDR Mapping

### 3.1 Specification mapping (frozen sources)

| I1 concern | Governing frozen source |
|---|---|
| Two planes; Control Plane decides, owns no truth/state | Vol 9 Cognitive Control Plane v2, Part 5 (ORCH-001..004) |
| Downward-only layer stack | Amendments CCP Batch 1 (LAYER-001) |
| Partition by tenant/workspace, immutable key | Amendments CCP Batch 1 (SHARD-001) |
| Single owner per state | Amendments / OWN-001 |
| Kernel distribution (Raft, outcomes-only, sagas, joint consensus, WAL) | ARVES_IDR_Batch_1_Kernel_Distribution_v1 (IDR-001..005) |
| Cognitive type system + mandatory aspects | Ontology Specification, Parts 3–9 (O-001..007 design principles) |
| Engine ABI (pure/stateless engines) | Engine Graph Specification |
| Conformance ratification | Scenario Conformance Framework; Reference Lifecycle Part 6 (CCP-GATE) |
| Read consistency tiers | IDR-001 (linearizable / bounded-staleness / eventual) |

### 3.2 Contract mapping (per node — what I1 must honor)

| Node | Contract asserted in I1 |
|---|---|
| Kernel | *Only* entry that mutates truth is `commit(ProposedWrite) → Committed<Outcome>`; rejects any direct-state mutation attempt. Deterministic given the same committed log. |
| Persistence | Append-only; no in-place edit or delete of committed records; snapshot is a pure function of a WAL prefix. |
| LCW | Working Memory is derived state; never a truth source; readers get it uses-only. |
| Query | Every query is side-effect-free; returns `(value, read_tier, commit_index)`. |
| Engine | `run(inputs) → Inference`; pure w.r.t. truth; identical inputs → identical *content hash* of the request (ORCH-004); engine nondeterminism (LLMs) is allowed but only the **committed outcome** is authoritative and replicated. |
| Control Plane | `plan(intent) → Plan(EngineGraph)`; emits a **decision trace**; holds no state across restart except what it re-derives from the committed log. |
| Execution | `execute(boundWork) → Outcome`; outcome routed as a proposed write; cancellable; failures compensated via saga seam. |
| Consensus | `propose(entry) → committed_index`; leader-only commit; followers apply, never recompute. |

### 3.3 Invariant mapping

**Registered-normative (enforced with executable proof in I1 for the nodes I1 implements):**

| Invariant | Statement (as registered) | How I1 upholds it | I1 proof obligation |
|---|---|---|---|
| ORCH-001 | Control Plane owns no truth; only the Kernel owns truth. | Control Plane has no write path to state; all mutation is `Kernel::commit`. | Property test: no state delta exists without a corresponding Kernel commit entry. |
| ORCH-002 | Control Plane produces plans, never persistent state. | Control Plane holds only in-memory Plan; nothing of the Control Plane is written to the WAL. | Test: after Control-Plane restart, no CP-owned bytes survive except re-derivation from the log. |
| ORCH-003 | Replay from recorded decision trace, not recomputation. | Replay reads the WAL (= decision trace) and re-applies committed outcomes; engines are **not** re-run. | Replay test: rebuild-from-WAL state == live state, byte-for-byte, with engines disabled. |
| ORCH-004 | Every engine/capability invocation idempotent + content-addressable. | Each invocation carries a canonical content hash; duplicate proposals with equal hash are de-duplicated at the commit gateway. | Property test: replaying a duplicate invocation produces zero additional committed outcomes. |
| OWN-001 | One owner per state. | Static ownership table; each state category has exactly one owning crate. | Architecture test: no two crates expose a mutator for the same state. |
| LAYER-001 | Layers are downward-only. | Dependency graph forbids upward edges; Execution/Engine never call Control Plane; Kernel never becomes Control Plane. | Architecture test over the crate dependency graph (no upward/lateral truth call). |
| SHARD-001 | Partition by tenant/workspace; shard key immutable. | Every Outcome carries an immutable `ShardKey{tenant, workspace}`; one Raft group per shard. | Property test: shard key never mutates post-creation; no cross-shard atomic commit path exists. |

**Proposed (informative, pending CCP-GATE) — treated as *design intent, enforced by construction*, NOT asserted as registered:** `G-001` (Kernel sole truth + commit gateway — subsumed by ORCH-001+OWN-001 for I1), `QUERY-001` (query read-only), `LCW-001` (LCW owns Working Memory), `PERSIST-001` (persistence durable store only), `CAP-001..009`, `ENG-001..005`. I1 builds so these *would* pass, and ships candidate scenarios, but does not count them toward "Invariant Coverage 100%" until each passes CCP-GATE.

**Design principles (NOT runtime-provable):** Ontology `O-001..007`. Used to shape `uci.*` types; not subjected to executable-proof obligation.

### 3.4 Ownership analysis (OWN-001)

| State category | Sole owner | Never owned by |
|---|---|---|
| Cognitive **truth** (committed outcomes) | Kernel | Control Plane, Engine, Execution, LCW, Query |
| Durable **WAL + snapshots** | Persistence (under Kernel authority) | anyone else |
| **Working Memory** / live derived state | LCW | Kernel (truth ≠ working memory), Control Plane |
| **Plan / Engine Graph** | Control Plane (in-memory only) | Kernel (never), Persistence (never) |
| Capability **bindings/registry** | Capability Fabric | Kernel, Control Plane |
| **Consensus log position** per shard | Consensus (Raft group) | Query, LCW |

No state has two owners. The Kernel owns truth but **not** Working Memory (LCW) and **not** plans (Control Plane) — this separation is what keeps the Kernel from silently becoming the Control Plane.

### 3.5 IDR mapping

| IDR | Decision | I1 realization |
|---|---|---|
| IDR-001 | Kernel is CP, per-shard Raft (one group per tenant/workspace). | Single shard, quorum=1 Raft group exercising the real `Consensus` API. Read tiers exposed (linearizable via leader read-index; bounded-staleness + eventual degenerate to linearizable in single node). |
| IDR-002 | Leader→followers replication; snapshots + append-only WAL; followers apply outcomes, never recompute. | WAL = Raft log; apply loop re-derives LCW from committed outcomes. Follower path present but 0 followers in I1. |
| IDR-003 | Joint-consensus membership. | Interface seam only; single static member in I1. |
| IDR-004 | One leader per shard; leader loss discards in-flight uncommitted work. | Single node is always leader; in-flight/uncommitted proposals are dropped on restart (no partial truth). |
| IDR-005 | Append-only WAL = ordered record of committed truth = single replay source. | Implemented as the durable spine; ORCH-003 replay reads it. |
| — | No cross-shard atomic commit; sagas/compensation (Amendment-006). | Single shard ⇒ no cross-shard path exists to violate; saga seam stubbed in Execution. |

---

## 4. Gap Analysis

Distance between the frozen spec/IDRs and the current `runtime/` skeleton (interfaces-only, `cargo check` passes).

| # | Gap | Current state | Required for I1 | Severity | Resolution |
|---|---|---|---|---|---|
| G1 | No commit gateway behaviour | `arves-kernel` header only | `commit(ProposedWrite) → Committed<Outcome>` with validation, dedupe (ORCH-004), single-writer discipline | High | Implement Kernel commit path (Section 6) |
| G2 | No WAL / decision trace | `arves-persistence` header only | Append-only WAL + snapshot; = decision trace (IDR-005, ORCH-003) | High | Implement WAL with fsync + CRC; snapshot as pure fn of WAL prefix |
| G3 | No consensus API exercised | `arves-consensus` header only | `propose/commit/apply` API, degenerate quorum=1 group per shard | High | Implement single-node Raft-shaped module; real API, trivial quorum |
| G4 | No replay | none | Deterministic replay from WAL == live state | High | Implement replay engine; property test ORCH-003 |
| G5 | No content-addressing / idempotency utility | none | Canonical hash of invocation for ORCH-004 dedupe | High | Add stable canonical-encoding + hash in ontology/invariants |
| G6 | Control Plane emits no decision trace | `arves-control-plane` header only | Plan → drive → record decisions into the committed log | High | Implement orchestrator; ensure no CP persistent state (ORCH-002) |
| G7 | LCW not derived from outcomes | `arves-lcw` header only | Working Memory rebuilt from committed apply loop | Medium | Implement apply→LCW projection |
| G8 | Query tiers absent | `arves-query` header only | Read-only projections tagged with tier + commit index | Medium | Implement read paths (tiers collapse in single node) |
| G9 | Engine ABI not defined | `arves-engine-fabric` header only | `run(inputs) → Inference`, engine-anywhere, commit-via-leader | Medium | Define ABI trait; provide a deterministic demo engine |
| G10 | Ontology aspects not typed | `arves-ontology` header only | `uci.*` core types + 5 mandatory aspects on every Outcome | Medium | Implement type registry + aspect enforcement |
| G11 | Invariant proofs are `pending` | markers only | Executable proofs for ORCH-001..004/OWN/LAYER/SHARD-001 | High | Implement property/architecture tests (Section 7 DoD) |
| G12 | No conformance scenarios for I1 | harness header only | Scenarios ratifying commit + replay + isolation | High | Author scenarios; run against I1 build (CCP-GATE) |
| G13 | Runtime binary is a stub | prints banner | Wire one shard end-to-end | High | Implement wiring in `arves-runtime` |
| G14 | Multi-node / real Raft / joint consensus | none | **Deferred to I2** | N/A (out of scope) | Keep the seam; do not implement |
| G15 | Cross-shard sagas at scale | seam only | **Deferred (Amendment-006 behaviour, later milestone)** | N/A | Keep the seam |

No gap requires a spec change. G1–G13 are pure implementation of already-frozen behaviour. G14–G15 are intentionally out of I1 scope per the frozen milestone ordering, so leaving them as seams is *not* drift.

---

## 5. Engineering Design (standard headers)

Single design for the runtime, presented across the mandatory AEOS design headers. Where a header behaves differently for the **single-shard-first walking skeleton (I1 now)** versus the **per-shard Raft target (I2+)**, both are stated so the growth path is explicit and no rearchitecture is implied.

### 5.1 Responsibilities

- **Kernel:** be the *sole* mutator of cognitive truth; validate proposed writes; assign commit order via consensus; dedupe by content hash (ORCH-004); apply committed outcomes deterministically.
- **Consensus (per shard):** order and durably commit outcomes; expose `propose → committed_index`; leader-only commit; followers apply.
- **Persistence:** hold the append-only WAL (= Raft log = decision trace) and snapshots; guarantee durability of committed entries.
- **LCW:** own Working Memory as a projection of committed outcomes; serve Engine/Control Plane uses-only.
- **Query:** provide read-only, tier-tagged views; never mutate.
- **Engine Fabric:** run pure/stateless inference anywhere; never write truth directly.
- **Capability Fabric:** own the registry/bindings; resolve a capability to bound work.
- **Execution:** perform actions; route outcomes back as proposed writes; support cancellation/compensation seams.
- **Control Plane:** decide — build the Plan/Engine Graph, drive execution, record decisions — while owning **no** truth and **no** persistent state.
- **Runtime binary:** wire exactly one shard end-to-end.

**Non-responsibilities (guard rails):** the Control Plane must never commit truth; the Kernel must never plan; Query must never write; Engines must never assume Control-Plane authority.

### 5.2 Inputs / Outputs

| Node | Inputs | Outputs |
|---|---|---|
| Information Platform | Raw observations from Reality | `ProposedWrite` (canonicalized, typed by ontology) → Kernel |
| Control Plane | Intent / trigger | `Plan(EngineGraph)`; execution directives; **decision-trace entries** |
| Engine | Typed inputs (from LCW/Query) | `Inference` (content-addressed request) |
| Execution | Bound work | Action effect on Reality + `Outcome` (proposed write) → Kernel |
| Kernel | `ProposedWrite` | `Committed<Outcome>` (ordered, durable) or typed rejection |
| Consensus | `Entry` | `committed_index` |
| Persistence | `Committed<Outcome>` | Durable WAL append; snapshot |
| LCW | Committed apply stream | Working-Memory reads (uses-only) |
| Query | Read request + desired tier | `(value, read_tier, commit_index)` |

**System input boundary:** the *only* way anything becomes truth is a `ProposedWrite` reaching `Kernel::commit`. **System output boundary:** committed outcomes (via Query/WAL) and Reality-facing actions (via Execution).

### 5.3 State Model

Three state kinds, strictly separated by owner:

1. **Truth** (Kernel, durable): the ordered set of committed outcomes = `snapshot(base) + WAL[base..commit_index]`. Immutable, append-only, content-addressed.
2. **Working Memory** (LCW, ephemeral-derivable): a pure projection of truth; can be dropped and rebuilt by replay.
3. **Decision** (Control Plane, in-memory only): the Plan/Engine Graph and in-flight orchestration. Never persisted; on restart re-derived from the committed decision trace (ORCH-002/003).

State transition: `ProposedWrite → (validate) → Entry → (consensus commit at index i) → WAL.append(i) → apply(i) → LCW.project`. Every committed outcome carries the 5 mandatory ontology aspects (Identity, Provenance, Trust, Temporal, TenantScope) and an immutable `ShardKey` (SHARD-001).

### 5.4 Distributed Behaviour

- **I1 (single-shard-first):** one shard, one node, one Raft group with quorum=1. The node is permanent leader. The commit path, WAL, apply loop, and read-index are *real*; the network is *absent*. This proves the *shape* of distribution deterministically.
- **I2+ (per-shard Raft):** N nodes per shard; leader-only commit; followers replicate the log and apply committed outcomes (never recompute — ORCH-003/IDR-002); joint-consensus membership (IDR-003); per-shard leader election (IDR-004). Shards are independent Raft groups (SHARD-001); **no cross-shard atomic commit** — cross-shard work uses sagas/compensation (Amendment-006).
- **Engine placement:** engines run anywhere (any node); only the **commit** goes through the shard leader. A stale-leader engine cannot commit. This holds trivially in I1 and by design in I2.

### 5.5 Concurrency

- **Single-writer commit:** per shard, the leader serializes all commits through the consensus log — there is exactly one linearization point (the assignment of `committed_index`). This eliminates truth-level write races by construction.
- **Concurrent proposals:** allowed; they queue at the leader and are ordered by the log. Duplicate proposals (equal content hash) are de-duplicated at commit (ORCH-004), so at-least-once delivery is safe.
- **Reads vs. writes:** reads never block the write path; linearizable reads use a read-index against the committed log; bounded-staleness/eventual reads may lag. Query is read-only, so no read can create a write race.
- **Apply loop:** single-threaded per shard; applies committed entries in index order → deterministic LCW projection. (Rust: the apply loop owns LCW state exclusively; other tasks get `&`-shared read snapshots.)

### 5.6 Failure Modes

| Failure | I1 behaviour | I2+ behaviour |
|---|---|---|
| Proposed write invalid | Kernel rejects with typed error; no WAL entry; no truth change. | same |
| Duplicate invocation (retry) | Deduped by content hash; idempotent no-op (ORCH-004). | same |
| Crash after propose, before commit | Uncommitted entry discarded on restart; no partial truth (IDR-004). | Leader loss → re-election; in-flight uncommitted work discarded. |
| Crash after commit, before apply | Recovered: replay re-applies from WAL; LCW rebuilt. | same, per follower/leader. |
| WAL corruption (bad CRC) | Truncate at last valid entry; refuse to serve stale-beyond-truncation reads; alarm. | same; can re-sync from peers. |
| Engine nondeterminism / failure | Engine result is not truth until committed; a failed engine yields no outcome. | Retry/route to another node; commit only via leader. |
| Network partition | N/A (single node). | Minority side cannot commit (CP: unavailable > inconsistent). Reads may serve bounded-staleness/eventual per tier. |
| Cross-shard partial state | N/A (single shard). | Expected mid-saga; compensation resolves it (Amendment-006). |

**CP posture (IDR-001):** truth chooses **Consistency over Availability**. When consensus cannot be reached, the write path is unavailable rather than divergent. Observability (metrics/logs/tracing/presence) is **AP** and may be eventually consistent — never treated as ground truth for a truth question.

### 5.7 Recovery

Recovery is **replay from durable state**, never recomputation:

1. Load latest snapshot (a pure function of a WAL prefix at index `b`).
2. Replay WAL entries `(b, committed_index]`, applying each committed outcome to LCW in order.
3. Resume consensus at `committed_index`; discard any uncommitted tail (IDR-004 — no partial truth).
4. Control Plane re-derives in-flight plans from the decision trace (ORCH-002/003); it persists nothing of its own to recover.

Recovery is **deterministic** and **idempotent**: replaying the same WAL any number of times yields the identical LCW state.

### 5.8 Replay (ORCH-003)

- The **WAL is the decision trace** (IDR-005). Replay re-applies **committed outcomes**, it does **not** re-run engines. This is the crux of correctness given engine nondeterminism (LLMs): we recorded *what was decided/committed*, not *how it was computed*.
- Replay determinism proof (I1 DoD): run the system live, snapshot LCW; wipe LCW; rebuild solely from WAL **with engines disabled**; assert byte-for-byte equality.
- Duplicate/idempotent invocations replay to a single committed outcome (ORCH-004), so replay is stable under at-least-once retries.

### 5.9 Consistency

- **Truth:** linearizable within a shard (single log, single commit point). Cross-shard: no atomic guarantee; only per-shard linearizability + saga-level eventual consistency (I2+).
- **Read tiers (IDR-001):**
  - *Linearizable* — latest committed truth, via leader read-index.
  - *Bounded-staleness* — recent within a bound, via follower read.
  - *Eventual* — best-effort, via read/geo replica.
  - In I1 (single node) all three collapse to linearizable, but the **API returns the requested tier tag** so callers are written correctly for I2.
- **Observability:** AP / eventually consistent; explicitly *not* a truth source.

### 5.10 Availability

- **I1:** availability == single-process liveness. If the process is up, the shard is available for reads and writes.
- **I2+:** per-shard availability requires a quorum for writes; reads at bounded-staleness/eventual tiers can remain available on a minority (with staleness), while linearizable writes/reads require the leader + quorum (CP). Shards fail independently (SHARD-001) — one tenant's outage does not affect another.

### 5.11 Scalability

- **Compute scales separately from consensus** (IDR-001 refinement): engines run anywhere; only the commit funnels through the leader. Adding engine capacity does not add consensus load.
- **Horizontal scale = more shards.** Because state is partitioned by tenant/workspace (SHARD-001) and shards share nothing, throughput scales with shard count. Target scale (per IDR-001 rationale): ~10,000 nodes via many independent per-shard Raft groups — *not* one global leader.
- **I1 proves the unit** (one shard). I2 multiplies the unit. No global bottleneck is introduced in I1.

### 5.12 Performance

- **Commit latency (I1):** dominated by WAL fsync; no network. Target: single-digit ms per commit locally.
- **Commit latency (I2+):** one consensus round-trip to quorum + fsync. Batching multiple proposals per log append amortizes fsync.
- **Read latency:** linearizable = read-index (cheap in I1); bounded-staleness/eventual = local follower/replica read.
- **Perf discipline:** correctness before optimization (constitution rule 7). I1 sets **baseline benchmarks** (commit/s, replay time for N entries) as regression guards for I2, but does not micro-optimize.

### 5.13 Security

- **Tenant isolation (SHARD-001 + TenantScope aspect):** every Outcome is scoped to a tenant/workspace; no cross-tenant read/write path exists. This is the primary security boundary and is enforced at the type + commit-gateway level.
- **Provenance + Trust aspects:** every committed outcome records its origin and trust level (ontology mandatory aspects), giving an auditable chain of custody.
- **Commit gateway as choke point:** because the Kernel is the sole mutator, authorization/validation for truth is enforced in exactly one place (ORCH-001), not scattered.
- **Least authority upward:** LAYER-001 downward-only calling forbids a lower layer (e.g., Execution) from reaching the Control Plane, closing a privilege-escalation vector by construction.
- **I1 scope:** in-process, single trust domain; transport auth / mTLS between nodes is an **I2 concern** (seam noted, not built).

### 5.14 Observability

- **Truth vs. observability separation:** WAL/commit index (CP) is authoritative; metrics/logs/traces (AP) are advisory. Dashboards are hypotheses; the WAL is the confirmation (per Handbook debugging discipline).
- **Signals emitted in I1:** commit rate, reject rate (by reason), WAL append/fsync latency, replay duration, LCW apply lag, per-shard commit index, dedupe hit count (ORCH-004 evidence).
- **Auditability:** the append-only WAL *is* the audit log — every truth mutation is an ordered, immutable, content-addressed record with provenance.
- **Tracing:** each `ProposedWrite → Committed<Outcome>` carries a correlation id linking the Control-Plane decision, engine invocation, execution outcome, and commit index.

### 5.15 Trade-offs

| Decision | Chosen | Rejected | Why (spec-downstream) |
|---|---|---|---|
| Truth model | CP (Raft) | AP/CRDT for truth | Single-source-of-truth (ORCH-001) forbids "eventually one" truth. |
| Consensus scope | Per-shard | Global single leader | Global leader can't scale to ~10k nodes; tenant isolation makes per-shard sufficient. |
| Replicate | Committed **outcomes** | Engine invocations | Engines are nondeterministic (LLMs); recompute would diverge — ORCH-003. |
| Cross-shard | Sagas / compensation | Distributed 2PC | 2PC across shards reintroduces a global coordinator + availability coupling. |
| I1 shape | Single-shard-first walking skeleton | Full multi-node now | Prove the commit/replay *shape* deterministically before adding network non-determinism; avoids inventing architecture ahead of I2. |
| Read tiers in API from day 1 | Yes (even if collapsed) | Add later | Keeps callers correct for I2; avoids an API break. |

### 5.16 Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Hidden coupling** Control Plane ↔ truth | Would violate ORCH-001/002 | Architecture test forbids any CP write path; no CP bytes in WAL. |
| **Layer violation** (upward call) | Violates LAYER-001 | Crate dependency-graph test rejects upward/lateral truth edges. |
| **Replay non-determinism** (engine re-run leaks in) | Violates ORCH-003 | Replay path physically disables engines; asserts byte-equality. |
| **Ownership drift** (two mutators for one state) | Violates OWN-001 | Ownership table + test that only one crate mutates each state category. |
| **I1 shortcuts that don't generalize to I2** | Rearchitecture cost | Consensus behind a real API from day 1; single-node is a *quorum-1 configuration*, not a different code path. |
| **Proposed invariants treated as registered** | Certification confusion | Explicitly excluded from "100% coverage"; gated behind CCP-GATE. |
| **WAL durability bug** (unfsynced "commit") | Truth loss | Commit ack only after fsync; CRC per entry; crash tests in DoD. |

### 5.17 Open Questions

1. **G-001 promotion:** should "Kernel = sole truth + commit gateway" be ratified (CCP) as a registered invariant, or does ORCH-001+OWN-001 suffice permanently? (I1 does not depend on the answer.)
2. **Snapshot cadence policy:** frequency/threshold is an engineering knob — record as an IDR before I2 to keep replay time bounded at scale.
3. **Content-hash canonicalization spec:** the exact canonical encoding for ORCH-004 hashing should be pinned (candidate IDR) so independent runtimes hash identically for cross-impl conformance.
4. **Read-index vs. lease reads** for the linearizable tier in I2 — defer to I2 consensus design.
5. **Decision-trace granularity:** how much Control-Plane decision context is written to the log for ORCH-003 re-derivation without leaking CP into truth (ORCH-002)? Needs a scenario in the conformance suite.

*None of the open questions block I1; each is routed to its correct instrument (CCP/IDR/scenario) rather than resolved by implementation fiat.*

---

## 6. The Commit Path (engine-anywhere → shard leader commit → follower apply)

The central mechanism I1 must get exactly right. Same shape at every scale.

```
Reality / Trigger
   │
   ▼
[Control Plane]  decide → Plan(EngineGraph)                 (owns no truth, ORCH-001/002)
   │  drives
   ▼
[Engine Fabric]  run(inputs) → Inference                    (pure/stateless; runs ANYWHERE)
   │  (content-addressed request; ORCH-004)
   ▼
[Execution]      execute(boundWork) → Outcome               (acts on Reality)
   │  routes outcome as ProposedWrite
   ▼
[Kernel] commit(ProposedWrite):                             (SOLE commit gateway, ORCH-001)
   │   1. validate (types + mandatory aspects + ShardKey immutable, SHARD-001)
   │   2. content-hash dedupe (idempotent, ORCH-004)
   │   3. hand Entry to consensus for THIS shard
   ▼
[Consensus / shard LEADER] propose(Entry):
   │   4. append to Raft log  ── log == WAL == decision trace (IDR-005)
   │   5. replicate to followers, await quorum               (I1: quorum=1)
   │   6. mark committed at index i                          (linearization point)
   ▼
[Persistence]    WAL.append(i) + fsync                       (durable; ack only after fsync)
   │
   ├─────────────► [LEADER apply]  apply(i) → LCW.project     (Working Memory, OWN-001)
   │
   └─────────────► [FOLLOWER apply] apply(i) → LCW.project    (I2+: followers APPLY the
                                                               committed OUTCOME; they do
                                                               NOT recompute — ORCH-003/IDR-002)
   ▼
[Query] read(tier): linearizable → leader read-index @ i
                    bounded-staleness → follower read
                    eventual → replica read
```

**Invariants enforced along the path:**

- Truth changes **only** at step 6 (commit), and **only** inside the Kernel via consensus (ORCH-001, OWN-001).
- The Control Plane appears only at the top (decide) and never touches steps 4–6 (ORCH-002).
- Engines (step 2–3) may run on any node; only the **commit** funnels through the leader — compute scaling is decoupled from consensus (IDR-001).
- Followers **apply** committed outcomes; they never re-run engines (ORCH-003, IDR-002) — this is why nondeterministic engines are safe.
- Everything committed is content-addressed and idempotent (ORCH-004), so retries and at-least-once delivery converge to one outcome.
- Everything is shard-scoped with an immutable key (SHARD-001); there is **no** cross-shard atomic commit path — the diagram has exactly one shard's log.

**I1 vs I2 delta on this path:** only steps 5 (quorum size) and the FOLLOWER apply branch differ. In I1, quorum=1 and there are no followers; the code path is identical, exercised through the real consensus API. Growing to I2 changes configuration and populates followers — not the architecture.

---

## 7. Definition of Done for I1

I1 is **DONE** only when every item below is true and evidenced. (Mirrors the constitution's Success Criteria; scoped to I1.)

### 7.1 Functional
- [ ] One shard wired end-to-end in `arves-runtime`: Trigger → Control Plane → Engine → Execution → Kernel commit → WAL → LCW → Query returns the committed value.
- [ ] `Kernel::commit` is the *only* truth mutator; validates types + 5 mandatory ontology aspects + immutable ShardKey; rejects invalid/direct-mutation attempts with typed errors.
- [ ] Consensus exercised through its real API at quorum=1 (single-node Raft-shaped group per shard); commit produces a monotonic `committed_index`.
- [ ] Append-only WAL with per-entry CRC + fsync-before-ack; snapshot is a pure function of a WAL prefix.
- [ ] LCW is a pure projection of committed outcomes; Query serves read-only, tier-tagged results (tiers may collapse but are correctly labeled).
- [ ] ORCH-004 dedupe: duplicate content-addressed invocations commit exactly once.

### 7.2 Invariant proofs (executable — no longer `pending`)
- [ ] **ORCH-001**: property test — no state delta without a Kernel commit entry.
- [ ] **ORCH-002**: test — no Control-Plane bytes in the WAL; CP state re-derived on restart.
- [ ] **ORCH-003**: replay test — rebuild-from-WAL (engines disabled) == live LCW, byte-for-byte.
- [ ] **ORCH-004**: property test — replaying duplicates yields zero extra committed outcomes.
- [ ] **OWN-001**: architecture test — exactly one mutator per state category.
- [ ] **LAYER-001**: architecture test — crate dependency graph has no upward/lateral truth edge; Kernel never calls Control Plane.
- [ ] **SHARD-001**: property test — ShardKey immutable post-creation; no cross-shard atomic commit path exists.
- [ ] Registered-invariant coverage for I1-implemented nodes = **100%**. (Proposed invariants explicitly *excluded* until CCP-GATE.)

### 7.3 Tests (mandatory classes, per constitution)
- [ ] Unit, Integration, Architecture, Invariant, Replay, Property, Recovery, Failure-Injection (crash-after-propose, crash-after-commit-before-apply, WAL truncation) — all passing.
- [ ] Baseline benchmarks recorded (commit/s, replay time for N entries) as I2 regression guards.

### 7.4 Conformance (CCP-GATE)
- [ ] I1 conformance scenarios authored in `arves-conformance` covering: commit-through-Kernel-only, outcomes-only replay, tenant isolation, idempotent invocation, no-truth-in-Control-Plane.
- [ ] Result recorded as **"UCI (I1) passes UCS v1.0 scenario set {I1}"**, pinned to the frozen spec version.
- [ ] No behaviour ratified without a scenario (no scenario ⇒ not done).

### 7.5 Governance / drift
- [ ] No specification drift: no frozen document modified.
- [ ] No architecture drift: no new layer; layer stack unchanged; every crate maps 1:1 to a UCI node/layer.
- [ ] No ownership drift: ownership table holds; OWN-001 test green.
- [ ] Any distribution decision made during I1 that the spec did not pin is recorded as an **IDR** (e.g., snapshot cadence, content-hash canonicalization) — never a silent change.
- [ ] Each crate module header cites its governing invariants (already present in the skeleton; kept accurate).

### 7.6 Review gates (AEOS Phases 10, 14, 15)
- [ ] Critical Self-Review passed (attempted to disprove: drift, hidden coupling, layer/ownership/spec violations, replay bugs, races, determinism violations).
- [ ] Independent Architecture Review verdict = **PASS** across Architecture · Specification · Contracts · Invariants · Ownership · Distributed Behaviour · Replay · Concurrency · Security · Maintainability · Future Evolution · Certification Readiness.
- [ ] Certification Verdict = **PASS** for the I1 scope.

**Exit criterion:** with all boxes checked, I1 has proven — with executable evidence — that the frozen ARVES commit/replay model runs correctly and deterministically for one shard, and that growing to per-shard Raft (I2) is a configuration/replication change on an unchanged architecture.

---

*End of I1 Distributed Runtime Engineering Design. This artifact implements and proves the frozen UCS/UCI v1.0 specification; it changes nothing in it.*

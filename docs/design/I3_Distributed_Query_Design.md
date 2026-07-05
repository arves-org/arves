# I3 — Distributed Query: Engineering Design Package

```
=====================================================================
  STATUS: DESIGN PACKAGE (Ch4 PREP MODE) — NO CODE
  Build gate (G2) CLOSED · prepared 2026-07-05
  under maintainer prep-mode ruling
=====================================================================
```

- **Milestone:** I3 — Distributed Query (`ARVES_00_Baseline_v1.md`, Part 5: *"I3 Distributed Query | Query routing; LCW partitioning"*).
- **Nature:** engineering design only. Every statement below is preparation for a future ratified build phase; nothing here authorizes implementation. The I2..I6 build gate (G2) stays closed; the Standard Validation Era KPI (Evidence Increased) is unaffected.
- **Frozen surfaces touched:** NONE. `runtime/`, `standard/`, `spec-markdown/`, `corpus/` are read-only inputs (Runtime v1.0 FROZEN, `runtime/RUNTIME_FREEZE_v1.0.md`). This document is a new living file under `docs/design/`.
- **Single-node reference semantics:** the RCR-010 `QueryProjection` in `runtime/crates/arves-conformance/src/live.rs` (read-only WAL-replay projection with tenant-scoped reads, `Verdict::Pass` in the live L1 Information→Kernel→Query artifact). I3 distributes exactly those semantics; it invents no new ones.
- **Contract of record:** the CONTRACT-ONLY crate `runtime/crates/arves-query/src/lib.rs` (`Query` trait, `ReadTier`, `ReadScope`, `StalenessBound`, `Projection`, `QueryError`) — *"Distributed query execution is an I3 concern … this crate only fixes the read-only shape that I3 must honor"* (crate doc, "Scope of this skeleton").

---

## 1. BEFORE-WRITING-CODE — the ten constitutional answers

### 1.1 Which UCI node is affected?

The **Query** node of the conformance pipeline — `Reality → Information Platform → Kernel → LCW → Query → Engine → Capability → Execution → Reality` (Scenario Conformance Framework v1, Part 7). Its required evidence is fixed by the frozen framework: *"Query — Correct, tenant-scoped read of state"* (same table).

Secondary read-side surfaces are *touched as dependencies, never modified*:

- **Persistence** — the WAL is read (replayed) to build projections (Layer Matrix: Query *Reads* "Kernel, LCW, Persistence"; Amendments CCP Batch 1, A-003).
- **Kernel / Consensus** — a linearizable read must confirm currency with the per-shard Raft leader ("read-index", IDR-001 Read Consistency Tiers). The Kernel itself gains no read API in this design (the RCR-010 reference deliberately reads via WAL replay, *"no Kernel read hook (ORCH-001/OWN-001)"*, `live.rs`).
- **LCW** — Baseline Part 5 names "LCW partitioning" as an I3 focus; this design fixes the partitioning *rule* (same shard key, SHARD-001) while noting honestly that the LCW crate is CONTRACT-ONLY (RUNTIME_FREEZE item #4 / RCR-001) — see Open Questions OQ-6.

### 1.2 Which documents govern it?

| Frozen source | What it governs here |
|---|---|
| `ARVES_00_Baseline_v1.md` Part 5 | I3 scope: "Query routing; LCW partitioning" |
| `ARVES_00_Amendments_CCP_Batch_1_v1.md` A-003 (Layer Matrix) | Query row: Owns "Read projections/views" · Reads "Kernel, LCW, Persistence" · Writes "NOTHING (read-only)" · Cannot "Mutate any state" |
| `ARVES_00_Amendments_CCP_Batch_1_v1.md` A-004 | Shard key = tenant_id + workspace_id, immutable; routing key = tenant_id + correlation_id; SHARD-001 |
| `ARVES_IDR_Batch_1_Kernel_Distribution_v1.md` IDR-001..005 | CP truth on per-shard Raft; Read Consistency Tiers (Linearizable / Bounded-staleness / Eventual); follower reads (IDR-002); WAL as single replay source (IDR-005); "no cross-shard atomic commit in v1"; CP/AP boundary (truth CP, observability AP) |
| `ARVES_Volume_9_Cognitive_Control_Plane_v2.md` Part 5 | ORCH-001..004 (registered) |
| `ARVES_Volume_9_Runtime_Event_Fabric_Bible_v1.md` Part 9 | Query Model: "Queries request information from the system" |
| `ARVES_Volume_3_Information_Core_Bible_v1.md` Part 24 | Unified Query Layer: "Single query interface across all knowledge assets" |
| `ARVES_Volume_2_Tenant_Identity_Constitution_v2.md` Parts 1/2 & isolation principles | Tenant = intelligence boundary; Tenant Isolation / Workspace Isolation / Least Privilege / Zero Trust |
| `ARVES_Scenario_Conformance_Framework_v1.md` Parts 5–10 | Axes, "Enterprise Knowledge Query" reference scenario, Query node evidence, verdict semantics, L1/L3 levels |
| `ARVES_00_Invariant_Registry_v1.md` Parts 2 & 4 | Registered vs PROPOSED invariant standing (QUERY-001 is PROPOSED) |
| `ARVES_Reference_Lifecycle_v1.md` Part 6 | CCP-GATE: no behaviour ratified without a conformance scenario |
| `runtime/RUNTIME_FREEZE_v1.0.md` | `arves-query` is contract-only; every runtime change is an RCR; RCR-010 reference semantics |

### 1.3 Which contracts apply?

1. **Layer Matrix Query row** (A-003) — the normative read-only contract.
2. **`arves-query` crate contract** (frozen at runtime-v1.0): `Query` trait — every method `&self`, no method returns a commit handle; `ReadScope { shard, tier, bound }`; `ReadTier::{Linearizable, BoundedStaleness, Eventual}`; `Projection { id, observed_at, served_tier, value }` with `served_tier` never stronger than requested; `QueryError::{NotFound, UnknownShard, LeaderUnavailable, StalenessBoundExceeded, MalformedScope}`.
3. **IDR-001 Read Consistency Tiers table** — Linearizable → through leader (read-index); Bounded-staleness → follower read; Eventual → read/geo replica.
4. **Persistence WAL replay contract** (IDR-005; exercised by RCR-010): the WAL is *"the ordered record of committed truth mutations and … the single source for deterministic replay (ORCH-003)"*.
5. **Scenario Conformance Part 7 Query evidence contract**: "Correct, tenant-scoped read of state".

### 1.4 Which invariants apply?

Registered (normative — Invariant Registry Part 2): **OWN-001, LAYER-001, SHARD-001, ORCH-001, ORCH-002, ORCH-003, ORCH-004**. Full mapping with executable proofs in §4.

Proposed (informative — Invariant Registry Part 4; each marked **(PROPOSED — CCP-GATE required)** wherever referenced): **QUERY-001** (Query strictly read-only), **PERSIST-001** (Persistence never interprets), **LCW-001** (LCW owns Working Memory), **G-001** (Kernel sole commit gateway). None is enforced as normative by this design; the registered set alone is sufficient to force the read-only shape (Layer Matrix "Writes: NOTHING" is normative via A-003 even while QUERY-001 the *invariant* awaits ratification).

### 1.5 Which ownership rules apply?

- Query owns exactly one thing: **read projections/views** (A-003). Projections are *disposable derived state*, never a second source of record — the source of record stays the Kernel-committed WAL (OWN-001; Kernel row: Owns "TRUTH").
- Query never becomes an owner of what it reads (`arves-query` doc: "Query observes owners, it does not become one").
- Persistence owns the durable store; LCW owns Working Memory (A-001); the Kernel owns truth (ORCH-001). I3 changes none of these ownerships.

### 1.6 Which IDRs apply?

- **IDR-001** — CP kernel, per-shard Raft, read tiers, no cross-shard atomic operations, truth-CP/observability-AP. This is the backbone of the whole design.
- **IDR-002** — leader→followers replication, snapshots + WAL; *"Follower reads enabled (bounded-staleness); read/geo replicas allowed"* — the explicit license for the replica read path.
- **IDR-003** — joint-consensus membership: the shard directory the router consults must be correct across membership changes.
- **IDR-004** — per-shard leader election; leader loss makes linearizable reads temporarily unavailable for that shard (CP behaviour), surfaced as `QueryError::LeaderUnavailable`.
- **IDR-005** — append-only WAL + snapshots as the single replay source: the projection-build mechanism.
- **IDR-006** — products consume the frozen platform; any I3 runtime code lands via RCR, never a product-side edit.

### 1.7 Does this create architectural drift?

**No.** Checks performed:

- No new layer: Query already exists in the frozen Layer Matrix (A-003) and the pipeline (Scenario Conformance Part 7). The "Query Router" and "Projection Replica" of §3 are *internal components of the Query node*, not new layers.
- No reversed dependency: the read path depends only downward (Persistence WAL, consensus read-index, LCW views) per LAYER-001 and the `arves-query` crate's stated layer position ("it never reaches upward and never writes downward").
- No spec change: the design realizes Baseline Part 5's named I3 scope with the contract crate's frozen shape. Scatter-gather is constrained to what IDR-001 allows (non-atomic, per-shard; §3.7).
- The frozen `.docx` corpus, `runtime/`, `standard/` are untouched.

### 1.8 Does this require CCP / Amendment / a new IDR?

- **The design itself: none.** It is prep-mode documentation.
- **Before any build:** (a) an **RCR** to add implementation code under `runtime/` (the freeze admits additive v1.x change only via RCR — RUNTIME_FREEZE "Runtime Change Request" section); (b) a **CCP** is *recommended* to ratify **QUERY-001 (PROPOSED — CCP-GATE required)** using the conformance scenario in §5 as the CCP-GATE scenario (Reference Lifecycle Part 6: "No behaviour is ratified without a conformance scenario"); (c) a **new IDR** for the bounded-staleness attestation mechanism if the leader-lease option of OQ-2 is chosen (it is an engineering decision of IDR grade, like IDR-001..005).
- Cross-shard *transactional* reads would require a spec change (next major) — therefore they are a NON-GOAL (§6), not a design element.

### 1.9 Can another independent implementation reproduce this behaviour?

**Yes, by construction — for truth-derived projections.** The observable behaviour of every projection derived from Kernel-committed truth is defined as a deterministic function of frozen artifacts: `Projection(shard, v) = fold(apply, ∅, WAL[shard][0..v])` (§3.11), where the WAL record format and commit semantics are the frozen v1.0 persistence/kernel contracts and the read-tier semantics are the IDR-001 table. LCW-backed working-memory read views are **outside** this reproducibility argument: LCW writes "Mutable live state (not truth)" (Layer Matrix, A-003), which is not recorded in the WAL, so those views are not a WAL fold and their cross-runtime reproducibility is an open question (OQ-8). Conformance is structural/property/invariant-based, not golden-output (Scenario Conformance Part 8), so an independent runtime is judged by the same scenario artifacts (§5), exactly as the 2-runtime (Rust + Python) certification already works for the ACS surface (FOUNDATION program, project status).

### 1.10 Would this implementation still pass conformance five years from now?

**Yes, if built as designed**, because: (a) conformance pins the suite to the spec version (Scenario Conformance Part 11 — "N% at Level Lx against Framework vA / Spec vB") and ARVES v1.0 is permanently frozen; (b) the design binds to invariants and the contract crate's type-level shape, not to storage formats or transport details, which remain replaceable; (c) determinism/replayability (ORCH-003, IDR-005) makes the pass criterion time-independent — the same WAL prefix must always yield the same projection. The residual risk is the PROPOSED invariants changing wording during CCP ratification; §4 therefore anchors every obligation to a *registered* invariant first.

---

## 2. Specification / gap summary (workflow phases 1–8, condensed)

- **Architecture Readiness:** ARR is at PASS with 0 GAP / 0 CONFLICT (Amendments CCP Batch 1, "Post-Amendment ARR Status"); the open F1 mechanisms were resolved by IDR-001..005.
- **Gap being closed by I3:** the Query node is behaviour-live at **L1 single-node** only (RCR-010: `QueryProjection` over `MemWalStore`, one process). The gap to **L3 Distributed** ("Conformance preserved across distributed deployment", Scenario Conformance Part 10) is: no query routing, no replica reads, no read-tier enforcement, no multi-shard fan-out, no distributed isolation proof. Baseline Part 5 assigns exactly this to I3.
- **What already exists and is reused, unmodified:** the `arves-query` contract (shape), the persistence WAL + replay cursor (mechanism), RCR-010 `QueryProjection`/`QueryProbe` (reference semantics + probe pattern), the live conformance harness (`ConformanceArtifact`/`VerdictEngine`, RCR-008), the SHARD-001 two-tenant isolation test pattern (RCR-007).

---

## 3. ENGINEERING DESIGN

### 3.1 Responsibilities

The I3 Distributed Query node is responsible for exactly four things:

1. **Query routing** — resolve a `ReadScope.shard` (tenant_id + workspace_id, A-004) to the serving per-shard Raft group and select a target replica according to `ReadTier` (IDR-001 tiers table). Baseline Part 5: "Query routing".
2. **Projection serving** — answer `Query::read / exists / latest_version` (contract crate) from a **projection replica**: a read model built exclusively by deterministic WAL replay (IDR-005; RCR-010 semantics), optionally bootstrapped from a snapshot (IDR-002).
3. **Tenant-scoped isolation** — enforce that every read is scoped to exactly one shard and that no result ever contains another tenant's data (SHARD-001; Vol 2 Tenant/Workspace Isolation; Scenario Conformance Part 7 Query evidence).
4. **Scatter-gather (bounded)** — fan a *tenant-internal, multi-workspace* read out to the relevant shards and merge results as an explicitly **non-atomic union** with per-shard versions (constrained by IDR-001 "no cross-shard atomic commit in v1"; see §3.7 and OQ-3).

Explicitly NOT responsible for: writing anything (Layer Matrix: "Writes: NOTHING (read-only)"), owning truth (ORCH-001), planning/orchestration (Kernel row "Cannot: Orchestrate, plan or execute" — mirrored for Query by its own "Cannot: Mutate any state"), interpreting meaning (Persistence row), cross-tenant federation (Baseline Part 3, deferred to v2).

### 3.2 Inputs

- A read request: `ReadScope { shard: ShardKey, tier: ReadTier, bound: Option<StalenessBound> }` + `ProjectionId` (contract crate). Malformed tier/bound combinations are rejected as `QueryError::MalformedScope` before any I/O.
- The **shard directory**: shard → Raft-group membership + current leader hint (produced by I2 Cluster Kernel under IDR-003/004; a read-only input here — see Dependencies).
- The **per-shard WAL stream** (committed records only) and **snapshots** from Persistence (IDR-002/005).
- Replica lag metadata (applied index / commit index, heartbeat timestamps) for bounded-staleness admission (OQ-2).
- LCW read views for working-memory projections, partitioned by the same shard key (Layer Matrix Query row "Reads: Kernel, LCW, Persistence"). **Caveat:** these views are outside the WAL-fold determinism guarantee of §3.11 — LCW writes mutable live state that is not truth and is not in the WAL — and their consistency/staleness/replay semantics are undefined (partitioning-depth caveat in OQ-6; semantics gap in OQ-8).

### 3.3 Outputs

- `Projection<T> { id, observed_at: Version, served_tier, value }` — with the frozen guarantee `served_tier` ≤ requested tier (contract crate doc).
- `QueryError` for every failure (the frozen error set; no new variants without an RCR — the enum is part of the frozen crate).
- For scatter-gather: a merged result carrying a **per-shard version vector** (one `Version` per contributing shard), never a fabricated global version (§3.7). **Contract honesty:** the frozen `arves-query` contract (single-shard `ReadScope`, single-`Version` `Projection<T>`, only `read`/`exists`/`latest_version`) can represent NO merged multi-shard result — the entire scatter-gather surface (fan-out API + merged-result/version-vector type) is an additive contract requiring an RCR, regardless of how OQ-4 resolves (§6.2).
- Conformance evidence: `NodeEvidence` for the Query node per probe run (Scenario Conformance Parts 7/9; RCR-010 `QueryProbe` pattern).
- AP observability signals (§3.17) — explicitly eventual-consistent per IDR-001 Non-goals.

### 3.4 Dependencies

Strictly downward (LAYER-001; crate doc "it never reaches upward and never writes downward"):

| Depends on | For | Direction check |
|---|---|---|
| Persistence (WAL + snapshots) | projection build/catch-up (IDR-005, IDR-002) | Query → Persistence: downward ✓ |
| Consensus (per-shard Raft) | read-index attestation for Linearizable; leader identity (IDR-001/004) | Query → Consensus (Kernel substrate): downward ✓ |
| LCW read views | working-memory projections (Layer Matrix) — outside the §3.11 WAL-fold guarantee; semantics unknown (OQ-8) | Query → LCW: downward ✓ |
| I2 Cluster Kernel shard directory | routing (IDR-003 membership) | Query → Kernel substrate: downward ✓ |

**No dependency on** Engine, Capability, Execution, or the Control Plane (those sit above Query; they *consume* it — Layer Matrix Engine row "Reads: State via Query"). **Milestone dependency:** I3 assumes I2 (Cluster Kernel: replicated per-shard Raft groups) exists; without I2 only the single-node degenerate form (== RCR-010) is buildable. This ordering is the Baseline Part 5 milestone order itself.

### 3.5 Lifecycle

Per replica, per shard:

1. **Bootstrap** — install latest snapshot (IDR-002), then replay the WAL suffix to the snapshot's successor offset (IDR-005). This is exactly `QueryProjection::from_store` generalized from "replay everything" to "snapshot + suffix".
2. **Catch-up / steady state** — tail the committed WAL stream; apply records in log order; advance `applied_version`.
3. **Serving** — admit reads per tier: Eventual always; BoundedStaleness iff lag proof ≤ bound, else `StalenessBoundExceeded`; Linearizable only after read-index confirmation with the shard leader, else `LeaderUnavailable`.
4. **Shed / rebuild** — a projection is disposable: on suspicion of corruption or schema evolution it is discarded and rebuilt from snapshot + WAL. Rebuild equality is a conformance property (§4, ORCH-003 row). No repair-in-place — repair *is* replay.
5. **Decommission** — on membership change (IDR-003) a replica drains, stops advertising the shard in the directory, and is removed. In-flight reads complete or fail with `UnknownShard` after removal.

### 3.6 State Model

- **Projection state (derived, disposable):** `Proj(shard) = fold(apply, ∅, WAL[shard][0..applied_version])`. It is a pure function of the committed log prefix — never independently mutated (OWN-001: the one owner of the underlying state remains the Kernel; the projection is Query's owned *view*, per Layer Matrix "Owns: Read projections/views").
- **Routing state (soft):** shard directory cache. Stale routing is safe: a wrongly-targeted replica answers `UnknownShard`; a stale leader hint fails read-index and surfaces `LeaderUnavailable`; the router retries against refreshed membership. Correctness never depends on routing-cache freshness (it affects availability/latency only).
- **No durable Query-owned state of record.** Snapshots consumed are Persistence-owned; projections are rebuildable; caches are evictable. This keeps the entire node restart-safe with zero recovery obligations of its own (§3.10).
- The partition key inside every projection row is immutable for the entity lifetime (SHARD-001, A-004) — a projection never re-homes an entity across shards.

### 3.7 Distributed Behaviour

- **Single-shard read (the default and the recommendation):** route by `ShardKey` per A-004. Linearizable → shard leader (Raft read-index: leader confirms leadership + commit index, replica serves at ≥ that index — IDR-001 tier table "Through leader (read-index)"). BoundedStaleness → any follower proving lag ≤ bound (IDR-002 "Follower reads enabled (bounded-staleness)"). Eventual → any replica, including geo replicas (IDR-001/002).
- **Scatter-gather (tenant-internal only):** a query spanning several workspaces of ONE tenant fans out one single-shard sub-read per shard and merges. The merge is an explicitly labeled **non-atomic union**: each partial carries its own `Version`; there is no cross-shard snapshot point, because v1 has no cross-shard atomicity (IDR-001: "operations are single-shard atomic; cross-shard coordination uses sagas/compensation (Amendment-006), not distributed transactions"). The unified interface (Vol 3 Part 24 "Single query interface across all knowledge assets") is honored at the API surface; the consistency unit stays the shard. **None of this fan-out surface exists in the frozen `Query` trait:** its methods take one `ReadScope` (one shard) and return one `Projection<T>` (one `Version`) — so the fan-out entry point *and* the merged-result type are both additive contract surface, landing only via RCR (§6.2), not just the partial-failure shape of OQ-4. Whether tenant-internal fan-out is even required for v1.x is OQ-3.
- **Cross-tenant scatter: forbidden, structurally.** The fan-out planner takes a single tenant_id; there is no API accepting a tenant set (Vol 2 Tenant Isolation; SHARD-001 "no cross-tenant data in a single shard", A-004).
- **Partition behaviour (CP/AP split, IDR-001):** under a partition, Linearizable reads on minority-side shards fail (`LeaderUnavailable`) — truth is CP. Eventual reads keep serving — the observability surface is AP. BoundedStaleness degrades to failure when the lag proof can no longer be established within the bound. This is the IDR-001 CP/AP boundary expressed at read time.

### 3.8 Concurrency

- Reads are wait-free with respect to the commit path: replicas apply the WAL single-threaded per shard (log order is total per shard — IDR-005), while readers observe an immutable published version of the projection (e.g., versioned snapshot pointer swap; the exact structure is an implementation choice, not contract). Readers never take locks that the Raft/commit path can contend on — the Query node cannot slow truth down.
- Per-shard apply is sequential by construction (the log is the order); cross-shard applies are independent (no ordering exists across shards — the RCR-010 note about HashMap inter-shard interleave documents precisely this: *"the per-shard log IS deterministic; only the inter-shard interleave … is unordered"*, `live.rs`).
- Scatter-gather sub-reads run concurrently and join with a deadline; the join never blocks on a failed shard beyond the deadline (§3.9).
- `Query` trait methods are `&self` (frozen contract) — implementations must be internally synchronized for shared use; no method can require exclusive access.

### 3.9 Failure Modes

| Failure | Behaviour | Contract surface |
|---|---|---|
| Shard leader unreachable / election in progress (IDR-004) | Linearizable reads fail fast | `QueryError::LeaderUnavailable` |
| Replica lag exceeds caller bound | Bounded read refused, lag reported | `QueryError::StalenessBoundExceeded { requested, observed_lag }` |
| Unknown / re-homed shard, stale directory | Read refused; router refreshes membership (IDR-003) and may retry | `QueryError::UnknownShard` |
| Projection target absent | Clean miss | `QueryError::NotFound` (`exists` → `Ok(false)`) |
| Tier/bound mismatch in request | Rejected before I/O | `QueryError::MalformedScope` |
| Partial scatter-gather failure | **Never a silent partial result.** Either fail the merged read or return an explicitly partiality-labeled result — which of the two is OQ-4 (the frozen error enum has no `Partial` variant; adding one is an RCR) | see OQ-4 |
| Projection corruption suspected | Shed + rebuild from snapshot/WAL (§3.5); serve `Eventual` from peers meanwhile if policy allows | none (internal) |
| Network partition | CP for Linearizable, AP for Eventual (§3.7) | `LeaderUnavailable` on minority side |

Failure NEVER converts a read into a write, and no failure path leaves partial truth — there is no truth here to leave (ORCH-001; Amendment-005's "no partial truth" discipline applies a fortiori to a node that owns no truth).

### 3.10 Recovery

- **Replica recovery = bootstrap** (§3.5): snapshot install + WAL suffix replay (IDR-002/005). There is no Query-private recovery log, checkpoint format, or fsck — by design the node has nothing durable of its own to recover (§3.6).
- **Recovery proof obligation:** after any crash/rebuild, the recovered projection must equal the pre-crash projection at the same `Version` (bit-equal fold result). This mirrors the Kernel's own recovery proof in `KernelProbe` (`MemKernel::recover` → `truth_hash` equality, `live.rs`) and is a mandatory test (§5).
- **Router recovery:** cold start reads the shard directory; no state carries over. Joint-consensus transitions (IDR-003) during recovery are handled by the same stale-routing safety argument as §3.6.

### 3.11 Replay

The heart of the milestone — "WAL-replay-consistent reads":

- **Definition (scope: truth-derived projections):** every projection **derived from Kernel-committed truth** is a deterministic function of a committed WAL prefix: `Proj(shard, v) = fold(apply, ∅, WAL[shard][0..v])`. Two replicas (or the same replica across a crash, or an independent runtime) replaying the same prefix MUST produce identical projections. Source: IDR-005 (WAL = "single source for deterministic replay (ORCH-003)"); reference behaviour: RCR-010 `QueryProjection::from_store` (replay from `wal.earliest()`).
- **Explicit exclusion — LCW-backed read views.** The Layer Matrix lists LCW among Query's read sources (A-003), but LCW writes "Mutable live state (not truth)" (A-003 LCW row) and working memory is not recorded in the WAL. Therefore an LCW-backed working-memory read view **cannot be a WAL fold**: it is outside the WAL-replay determinism guarantee of this section and outside the ORCH-003 proof rows of §4, and `ReadTier`/`StalenessBound` semantics (lag relative to *committed truth*) are **undefined** for such views in the frozen corpus. What consistency, staleness and replay semantics — if any — apply to working-memory reads is honestly unknown: OQ-8. Until that is resolved, every determinism/replay/consistency claim in §3.11–§3.12 and every §4 ORCH-003 proof obligation binds to truth-derived (Kernel-committed WAL / Persistence) projections only.
- **`Projection.observed_at: Version`** identifies the point in the recorded outcome trace the read reflects (contract crate: "a version identifies a point in that trace", citing ORCH-003) — so any served read is *reproducible after the fact* by replaying to that version.
- **Replay uses recorded outcomes, never recomputation** (ORCH-003; IDR-001: "Replicate committed OUTCOMES, not engine invocations … followers apply it, they do NOT recompute (ORCH-003)"; IDR-002: "Followers apply committed OUTCOMES (never recompute engines)"). The Query node inherits this for free: it only ever folds committed records; it invokes no engines.
- **Snapshot equivalence obligation:** snapshot-then-suffix must equal full replay (`snapshot(v0) ⊕ WAL[v0..v] ≡ fold(∅, WAL[0..v])`) — a mandatory property test, since IDR-002 snapshots are an optimization that must be semantics-free.

### 3.12 Consistency

- Tiers exactly per IDR-001 (and only those three — the frozen `ReadTier` enum): **Linearizable** = latest committed truth via leader read-index; **Bounded-staleness** = follower read within a caller bound; **Eventual** = best-effort replica read that "may be arbitrarily stale but never *incorrect* for the version returned" (contract crate module doc); "never *wrong* for the `Version` it reports; it may simply be old" (contract crate, `ReadTier::Eventual` variant doc).
- **Monotonicity within a session** (read-your-observed-version): the idea is that a caller passes its last `observed_at` forward and a replica serves only at ≥ that version, giving cheap monotonic reads without leader contact. **Contract honesty:** the frozen `Query` trait methods take only `(scope, id)` — `ReadScope { shard, tier, bound }` has no slot for a minimum version, so this mechanism has **no carrier in the frozen contract**. It could exist only as (a) an additive API surface via RCR, or (b) an implementation-internal replica-selection heuristic that never surfaces in the API. The version *stamp* (`observed_at`) is in the frozen contract; the pass-it-forward channel is not — flagged as OQ-5.
- **No cross-shard consistency of any kind is promised** (IDR-001). The version vector in a scatter-gather result makes that visible instead of hiding it.
- Consistency of truth is the Kernel/Raft's job; Query adds no consistency and subtracts none — it faithfully *labels* what it serves (`served_tier`, `observed_at`).

### 3.13 Availability

- Eventual-tier reads remain available under partition and under total leader loss (AP observability, IDR-001 CP/AP table).
- Bounded-staleness availability degrades gracefully with replication lag; the caller chose the bound and gets an honest refusal rather than silent staleness.
- Linearizable availability is bounded by Raft leader availability per shard (CP; IDR-004 re-election window). This is a *deliberate* unavailability — the spec's choice, not a defect.
- Read/geo replicas (IDR-002 "read/geo replicas allowed") scale read availability horizontally without touching the consensus quorum.

### 3.14 Scalability

- **Reads scale independently of consensus** — the same separation IDR-001 makes for engine compute ("Compute scaling is separated from consensus") applies to the read path: adding projection replicas adds read throughput without enlarging any Raft group.
- **Per-shard parallelism:** shards are independent Raft groups (IDR-001), so projection maintenance parallelizes perfectly across shards; the High-volume Streaming axis (Scenario Conformance Part 5, axis 8: "Throughput, backpressure, tenant isolation at scale") is stressed per shard.
- **Backpressure:** a replica that cannot keep up serves Eventual only (its bounded admissions fail honestly); it never throttles the WAL producer.
- **Hot-shard mitigation** is adding replicas for that shard — never re-partitioning an entity (the shard key is immutable, SHARD-001).

### 3.15 Performance

- Target hierarchy (correctness over speed — Engineering Philosophy): Eventual reads are local-replica memory/disk reads; BoundedStaleness adds a lag check; Linearizable adds one leader round-trip (read-index) but *not* a log write (Raft read-index avoids appending no-op entries — the standard mechanism the IDR-001 tier table names).
- No performance number is asserted here — no measurement exists; concrete SLOs are deferred to the build phase and will be recorded as evidence, not promised (honest-language rule). Budget *shape*: p99 Linearizable ≈ p99 Eventual + one intra-cluster RTT.

### 3.16 Security

- **Isolation is the security model:** shard scope is mandatory in every request; the router refuses scopeless reads; result payloads are provably single-shard (SHARD-001; Vol 2: "Tenant Isolation, Workspace Isolation, Least Privilege, Zero Trust and Defense in Depth"). The executable form is the two-tenant read-isolation test lineage (RCR-007 at the gateway, RCR-010 `QueryProbe` at the read path), extended to multi-node in §5.
- **Honest threat-model statement:** Runtime v1.0's threat model is a **trusted single host**; `Kernel::commit` carries no principal/authN (RUNTIME_FREEZE item #8). A *distributed* query fabric widens the surface (replicas hold every tenant's projections; the wire carries payloads). Therefore: authN/authZ on the read path, encrypted transport, and per-tenant replica placement policies are REQUIRED for any non-trusted deployment and are **not designed here** — they depend on the open RCR-#8 v2.0 work (signatures, authenticated commit). Recorded as OQ-1, the most important open question of this package.
- Query can never escalate to a write: no code path from the read fabric reaches `Kernel::commit` (structural, LAYER-001 + the trait shape).

### 3.17 Observability

- Per IDR-001 Non-goals, all Query observability is **AP** (eventual): metrics, traces, logs never ride the consensus path.
- Every read is traceable: tier requested/served, shard, `observed_at` version, replica identity, outcome — correlated by the A-004 routing key (tenant_id + correlation_id) and Vol 9 (Runtime Event Fabric) Part 22 ("Every execution generates traces, metrics, logs and audits").
- Replica health surface: applied index, lag estimate, last snapshot, rebuild count.

### 3.18 Metrics

Minimum set (all AP): reads/s per tier per shard · tier-refusal counts (`LeaderUnavailable`, `StalenessBoundExceeded`) · replica lag (index + time estimate) · projection rebuilds · scatter-gather fan-out width and partial-failure count · isolation-violation counter (**must be identically zero**; any nonzero value is a conformance FAIL, not a metric to trend — Scenario Conformance Part 8: isolation is a critical property).

### 3.19 Auditability

- A served read is reconstructible: `(shard, ProjectionId, observed_at, served_tier)` + the frozen WAL suffices to re-derive exactly what the caller saw (§3.11). The WAL is already the audit log of truth (IDR-005 / RUNTIME_FREEZE "append-only WAL = decision trace"); Query adds read-audit records to the AP observability plane, not to the WAL (Query writes NOTHING — A-003).
- Tamper-evidence of the underlying trace comes from RCR-002's SHA-256 hash-chain digest (`FileWal::integrity_digest`) — a projection rebuild can verify the chain digest before trusting a WAL it replays (consumption of an existing facility, no new mechanism).

### 3.20 Trade-offs

| Chosen | Over | Because |
|---|---|---|
| WAL-replay projections | a Kernel read API | ORCH-001/OWN-001: the Kernel stays "a gateway, not a database" (RCR-010 note); reads can never contend with commits |
| Three fixed tiers | per-query custom consistency | IDR-001 defines exactly three; simple over clever |
| Non-atomic labeled scatter-gather | cross-shard snapshot reads | IDR-001 forbids cross-shard atomicity in v1; explicit over implicit |
| Disposable projections, rebuild-as-repair | durable query-side indexes with private recovery | Replayability over convenience; zero Query-owned recovery surface |
| Honest refusals (`StalenessBoundExceeded`, `LeaderUnavailable`) | silent degradation | Contracts over assumptions; the caller chose the tier |
| Costs accepted | — | Linearizable reads pay a leader RTT; bounded reads can be refused under lag; scatter-gather exposes version skew to callers instead of hiding it |

### 3.21 Risks

1. **I2 dependency risk** — I3 is unbuildable beyond the single-node form until I2 (Cluster Kernel) delivers per-shard Raft + shard directory. Mitigation: the conformance plan (§5) has a single-node-degenerate stage that reuses RCR-010 as-is.
2. **Staleness-bound semantics risk** — `StalenessBound` is milliseconds (frozen contract) but distributed lag is naturally measured in log indexes; a wrong time↔index mapping silently violates the bound. Mitigation: OQ-2 must be resolved by IDR before build; conformance includes an adversarial lag-injection test.
3. **Scatter-gather scope creep** — a merge layer can silently grow into a query planner/optimizer (new architecture — forbidden, Non-Negotiable Rule 2). Mitigation: §6 NON-GOALS pins the merge to union+filter; anything more is a next-major spec discussion.
4. **Isolation regression at scale** — multi-tenant replicas are a single process holding many tenants' projections; a filter bug is a cross-tenant leak. Mitigation: isolation is a critical conformance property (FAIL, not PARTIAL — Scenario Conformance Part 8) with a mandatory negative test per §5; plus the zero-tolerance metric of §3.18.
5. **Frozen-contract friction** — the frozen `QueryError`/`ReadTier` enums may prove too narrow (e.g., OQ-4 partial results). Mitigation: the RCR process exists for exactly this; no silent widening.
6. **PROPOSED-invariant drift** — building against QUERY-001's current wording before CCP ratification risks rework. Mitigation: every obligation in §4 is anchored to a *registered* invariant; QUERY-001 ratification (with §5's scenario) is sequenced before implementation sign-off.

### 3.22 Open Questions

- **OQ-1 (Security).** What is the authN/authZ model for distributed reads, and does a multi-tenant replica need per-tenant encryption at rest? Depends on RCR-#8's open v2.0 half (signatures, authenticated commit — RUNTIME_FREEZE item #8). *Unknown; NOT assumed solved. A non-trusted-host deployment of I3 is blocked on this.*
- **OQ-2 (Staleness attestation).** How does a follower *prove* lag ≤ `max_lag` milliseconds without trusted clocks — leader-lease timestamps, commit-index heartbeat deltas, or hybrid? The frozen contract fixes the unit (Millis); the safe mechanism is an engineering decision requiring a **new IDR** before build.
- **OQ-3 (Scatter-gather necessity).** Does any v1.x consumer actually need tenant-internal multi-workspace fan-out, or can I3 ship single-shard-only (with fan-out deferred)? The frozen corpus requires a *single query interface* (Vol 3 Part 24) but nowhere mandates multi-shard reads. Leaner is safer.
- **OQ-4 (Partial scatter-gather results).** Fail the whole merged read on any shard failure, or return an explicitly-labeled partial union? The frozen `QueryError` has no partial-result shape; either answer is representable only via (a) whole-read failure today or (b) an RCR adding an additive result type.
- **OQ-5 (Session monotonicity).** Should read-your-observed-version monotonicity (§3.12) be a *stated* v1.x guarantee (then it needs a conformance scenario per CCP-GATE) or remain an unadvertised implementation property? Note that even the *unadvertised mechanism* has no carrier in the frozen `Query` trait signatures (`read`/`exists`/`latest_version` take only `(scope, id)`; `ReadScope` has no min-version field): promising OR merely exposing pass-version-forward requires an additive contract surface via RCR, unless it stays a purely internal replica-selection heuristic invisible to callers.
- **OQ-6 (LCW partitioning depth).** Baseline Part 5 pairs I3 with "LCW partitioning", but the LCW crate is CONTRACT-ONLY (RCR-001 status headers). Does I3 own (a) only the partitioning *rule* for LCW read views (this design's position: same immutable shard key, SHARD-001), or (b) a live partitioned LCW implementation? (b) would require its own RCR and probably its own design package. *This package designs (a) and defers (b).*
- **OQ-7 (Projection payload shape).** `Query::View` is an associated type (frozen contract); what canonical payload encoding should the reference implementation bind — raw committed payload bytes (RCR-010 does this) or decoded ACS-002 dCBOR values? Raw bytes are the safe default (no interpretation — Persistence row's "never interprets meaning" discipline extended to the read path); decoding adds value but needs a decision.
- **OQ-8 (LCW read-view semantics).** The Layer Matrix (A-003) makes LCW a Query read source, yet LCW writes "Mutable live state (not truth)" — working memory is not recorded in the WAL, so LCW-backed read views cannot be WAL folds (§3.11 exclusion). What consistency, staleness, and replay semantics apply to working-memory reads? Do any of the three IDR-001 read tiers apply at all, given that `StalenessBound` measures lag relative to *committed truth* and working memory has no committed-truth version to lag behind? Are such reads reproducible across replicas or across an independent runtime in any sense? *Unknown; NOT assumed. The frozen corpus does not define these semantics, the §4 ORCH-003 proof rows do not cover this input class, and no determinism claim is made for it. Resolution requires at minimum an engineering decision (possibly IDR-grade) and, if new guarantees are promised, a CCP-GATE conformance scenario.*

---

## 4. Invariant mapping (registered set) + executable proofs required

Per the constitution, no invariant may remain proof-only once its owning component is implemented. If I3 is ever built, each row's proof is mandatory before the milestone can claim completeness.

| Invariant (registered) | Statement (source) | What I3 must uphold | Executable proof required at I3 |
|---|---|---|---|
| **OWN-001** | Every state has exactly one owner (Amendments CCP Batch 1, A-001 + registry) | Query owns only "Read projections/views" (A-003); projections are derived, never a second store of record | Structural: architecture gate proves the query fabric exposes no commit path and holds no non-derived durable state. Behavioural: delete a projection, rebuild from WAL, byte-equal result (ownership of record provably elsewhere) |
| **LAYER-001** | Dependencies point downward only; no lateral peer calls; cross-cutting via Control Plane/Event Fabric (A-003) | Query fabric depends only on Persistence/Consensus/LCW/shard-directory; nothing on Engine/Capability/Execution/Control-Plane | Extend the executable architecture gate (already enforcing LAYER-001 per Maintainer Note) to the I3 crates' dependency graph; CI-fail on any upward edge |
| **SHARD-001** | Partition by tenant/workspace; partition key immutable for entity lifetime (A-004) | Every read single-shard-scoped; scatter never crosses tenants; no entity re-homing in projections | Multi-node generalization of RCR-007/RCR-010: commit tenants A and B on different nodes; prove A-scoped reads on every replica and every tier never contain B's payload; adversarial probe with forged `ShardKey` |
| **ORCH-001** | The Control Plane owns no truth; only the Kernel owns cognitive truth (Vol 9 CCP v2, Part 5) | Reads reconstruct *committed* truth only; Query mints no truth; no Kernel read hook added (RCR-010 discipline) | Negative test: a staged-but-uncommitted proposal is never visible at any tier on any replica; positive: every served payload maps to a committed WAL record (chain-digest-verified via RCR-002 facility) |
| **ORCH-002** | The Control Plane produces plans, never persistent state (Vol 9 CCP v2, Part 5) | Owner is the Control Plane, but the read path must not tempt it: Query results feed planning without becoming Control-Plane persistent state | Scenario assertion inherited from the harness (Scenario Conformance Part 8 asserts ORCH-002 on every run); no I3-specific mechanism beyond providing the evidence hook |
| **ORCH-003** | Every execution replayable from recorded decision trace, not recomputation (Vol 9 CCP v2, Part 5) | WAL-replay-consistent reads: `Proj(shard,v)` is a pure fold of the trace; snapshot⊕suffix ≡ full replay; recovered replica ≡ pre-crash replica. Scope: truth-derived projections only — LCW-backed read views are outside these proofs (§3.11 exclusion, OQ-8) | Property tests: (a) two independent replicas at version v are byte-equal; (b) crash/rebuild equality (Kernel-probe pattern, `live.rs`); (c) snapshot-equivalence (§3.11) |
| **ORCH-004** | Every engine and capability invocation is idempotent and content-addressable (Vol 9 CCP v2, Part 5) | Reads are trivially idempotent; `ProjectionId` is content-addressable (contract crate: "a projection is named by *what* it is") | Test: N identical reads at pinned version return identical `Projection` (id, observed_at, value); read retries under fault injection produce no state change anywhere (WAL length invariant before/after) |

**PROPOSED invariants referenced (informative only — each requires CCP-GATE ratification before enforcement):**

- **QUERY-001 (PROPOSED — CCP-GATE required):** "The Query layer is strictly read-only …" (Invariant Registry Part 4). This milestone is its natural ratification vehicle: §5's scenario is the CCP-GATE conformance scenario. Until ratified, the same obligation is enforced via the *registered* A-003 Layer-Matrix row (normative) — so the design does not depend on the ratification for its correctness discipline.
- **PERSIST-001, LCW-001, G-001 (each PROPOSED — CCP-GATE required):** cited only descriptively (§1.4, §3.22); nothing in this design enforces them as normative.

---

## 5. Conformance plan

### 5.1 Scenario and axes (frozen framework)

- **Reference scenario instantiated:** **"Enterprise Knowledge Query"** — axis combination **1 + 8 + 9**, key assertions *"Tenant isolation held; provenance/trust attached; control plane owns no truth (ORCH-001)"* (Scenario Conformance Framework, Part 6). This is the frozen scenario whose center of gravity is the Query node.
- **Additional axis:** **12 Recovery & Replay** ("Deterministic replay from decision trace (ORCH-003)", Part 5) — mandatory because WAL-replay-consistency IS the milestone. Axis 9 (Multi-agent Coordination) participates only as concurrent-reader load, honestly scoped: full multi-agent semantics are I5.
- **Level targeted:** **L3 Distributed** — "Conformance preserved across distributed deployment" (Part 10). The Query node is L1-live today (RCR-010); I3's claim is precisely the L1→L3 raise for this node. Result reporting follows Part 11: "N% at L3 against Framework v1.0 / Spec v1.0".

### 5.2 Staged artifacts (evidence ladder)

1. **Stage 0 (exists):** single-node `l1-information-kernel-query` artifact, `Verdict::Pass` (RCR-010) — the degenerate baseline every distributed configuration must still pass.
2. **Stage 1:** single-shard, multi-replica — follower/eventual reads + lag-refusal + rebuild-equality probes.
3. **Stage 2:** multi-shard, multi-tenant — distributed isolation (SHARD-001 row of §4), scatter-gather union semantics (if OQ-3 resolves to "yes").
4. **Stage 3:** fault-injected — leader kill (IDR-004), partition (CP/AP behaviour of §3.7), membership change under load (IDR-003), crash-rebuild during serving.

Each stage emits machine-readable `ConformanceArtifact`s (Part 9) via the existing live-harness pattern (`NodeProbe`/`VerdictEngine`, RCR-008), with worst-wins verdict semantics (Part 8): any registered-invariant violation or critical isolation failure → FAIL.

### 5.3 What the constitutional Success Criteria concretely mean for I3

| Criterion | Concrete I3 meaning |
|---|---|
| **Architecture PASS** | Executable architecture gate green over the I3 dependency graph (LAYER-001 §4 row); no new layer; independent review confirms the design ↔ frozen-spec trace table of §1.2 |
| **Conformance PASS** | Stages 1–3 artifacts all `Verdict::Pass`; the Enterprise Knowledge Query scenario passes at L3 against Framework v1.0 / Spec v1.0 |
| **Certification PASS** | The maintainer-independent certification harness (FOUNDATION: certifies ANY runtime from `standard/` alone) grades the distributed read surface; the Rust reference passes with zero maintainer intervention |
| **Independent Review PASS** | PASS verdict across the constitution's 14 review dimensions, written as if a third party submitted the work |
| **100% invariant coverage** | All seven §4 rows have named, biting, CI-executed proofs (PropertyCheck catalog extension, RCR-006 pattern); zero rows left `pending` for the Query node |
| **Replay PASS** | ORCH-003 triple proof (§4): replica-equality, crash-rebuild-equality, snapshot-equivalence — all green under fault injection |
| **Distributed tests PASS** | Stage 3 suite green: leader loss, partition (linearizable fails / eventual serves), joint-consensus membership change, lag-bound adversarial test |
| **No architecture / spec drift** | Frozen corpus byte-identical (266-file freeze gate); `runtime/` changes exist only inside the ratified I3 RCR set; zero silent edits |

Additionally, per the graded-independence ladder of the Standard Validation Era, an I3 build raises evidence, not independence: a stranger-built distributed runtime passing this scenario is the era exit gate, not this milestone.

### 5.4 CCP-GATE linkage

The Stage-2 isolation + read-only scenario is submitted as the conformance scenario required to ratify **QUERY-001 (PROPOSED)** via CCP (Reference Lifecycle Part 6: "No behaviour is ratified without a conformance scenario"). One milestone, one ratification, one scenario — no invariant enters by prose.

---

## 6. NON-GOALS and change instruments

### 6.1 Explicit NON-GOALS of I3

1. **No writes, ever, in any failure mode** — the Query node writes NOTHING (A-003). Not a deferred feature; a permanent property.
2. **No Kernel read API** — reads come from WAL replay, preserving "a gateway, not a database" (RCR-010, `live.rs`). Adding Kernel reads would be an RCR with ORCH-001 justification, and this design recommends against it.
3. **No cross-shard atomic / transactional / snapshot-consistent reads** — IDR-001 excludes cross-shard atomicity in v1; offering read-side atomicity the write side cannot match would be invented architecture.
4. **No cross-tenant queries or federation** — Vol 2 isolation; Baseline Part 3 defers Cross-Runtime Federation and Federated Kernel to v2.
5. **No query language, planner, or optimizer standardization** — the frozen corpus defines a Query *Model* ("Queries request information", Vol 9 REF Part 9) and a unified *interface* (Vol 3 Part 24), not a query language. Standardizing one would be new architecture (Non-Negotiable Rule 2) and a next-major discussion.
6. **No caching tiers with independent invalidation protocols** — the only "cache" is the projection itself, repaired by replay (§3.5). A distributed cache-coherence protocol is precisely the hidden-coupling risk the self-review must destroy.
7. **No new invariants, no enforcement of PROPOSED invariants** — QUERY-001 et al. remain informative until CCP-ratified (§5.4).
8. **No performance promises** — SLOs are measured during a build, never asserted in a design (honest language).
9. **No live LCW implementation** — I3 fixes the LCW partitioning *rule* only (OQ-6); a live partitioned LCW is its own RCR/design.
10. **No multi-agent read semantics** — axis 9 appears as load only; coordination semantics are I5 (Baseline Part 5).

### 6.2 Instruments any frozen-surface change would require

| Needed change | Instrument (per CLAUDE.md Change Management + RUNTIME_FREEZE) |
|---|---|
| Any code under `runtime/` (new I3 crates, additive `arves-query` impls, harness probes) | **RCR** — triaged by the Runtime Team into v1.1 (additive) with its own destroy→repair→prove cycle (ED-006) and freeze-baseline re-advance |
| Breaking change to the frozen `Query` trait / `ReadTier` / `QueryError` shapes | **RCR escalated to v2.0 major** (RUNTIME_FREEZE: "a breaking change requires a new major") |
| Ratifying QUERY-001 (or any PROPOSED invariant) as registered-normative | **CCP Amendment** with the §5 conformance scenario (CCP-GATE, Reference Lifecycle Part 6) |
| Staleness-attestation mechanism decision (OQ-2) | **New IDR** (engineering decision of IDR grade; joins IDR Batch 1's register) |
| Entire scatter-gather surface — fan-out API + merged-result/version-vector type (§3.3/§3.7; not representable in the frozen single-shard `Query` trait) | **RCR** (additive v1.x) — required regardless of how OQ-4 resolves |
| Partial-scatter-gather result shape (OQ-4, additive type) | **RCR** (additive v1.x) |
| Session-monotonicity carrier — a min-version parameter has no slot in the frozen `Query` trait signatures (OQ-5) | **RCR** (additive v1.x) if surfaced in the API; none if kept as an internal replica-selection heuristic |
| Any wording fix discovered in the frozen corpus during build | **CCP Amendment / next major** — STOP first; never implement around it |
| Opening the actual I3 build | **Maintainer ruling reopening G2** — outside this document's authority |

---

*Prepared under the ARVES Engineering Constitution v1.0. Design serves Specification; Specification never serves Design. If any statement above conflicts with the frozen corpus, the corpus wins and this document must be corrected.*

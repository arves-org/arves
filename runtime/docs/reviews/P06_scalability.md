# ARVES Independent Chief-Architect Review — Prompt 6: Scalability

**Reviewer role:** Independent, ISO/IEEE-grade Chief Architect.
**Objective:** Maximize the 20-year adoptability of ARVES as a cognitive-infrastructure standard, viewed strictly through the **scalability** lens.
**Assumed target scale:** 100M entities · 1B observations/day · 10,000 nodes · 5,000 tenants.
**Hard rule honored:** No finding proposes changing the frozen specification. Every proposal is an **IDR**, **CCP Amendment**, **Runtime**, **Verification**, **Certification**, **Ecosystem**, or **Product** instrument.

---

## Executive Summary

ARVES's frozen architecture makes exactly one scaling decision that dominates everything else: **state is partitioned by an immutable `(tenant, workspace)` key (SHARD-001), and there is exactly one Raft group per shard (IDR-001).** This is a genuinely good decision — it makes horizontal scale a *routing* problem, not a *consensus* problem, and it is the single most important reason ARVES *can* reach 10,000 nodes. But the frozen spec deliberately left the *how* to the Implementation Era ("IDRs implement, but never change, the frozen specification"), and the entire distributed scaling apparatus is currently **undefined** in normative terms and **absent** in the reference runtime. The runtime today is a correct single-process, single-mutex, per-append-fsync, full-state-snapshot skeleton. Everything that makes 1B/day work — sub-shard partitioning, hot-shard splitting, incremental snapshots, parallel replay, back-pressure, a saga coordinator, and a shard-placement/metadata plane — is not yet decided.

The critical risk is not that the reference runtime is a skeleton (that is expected at I1). The critical risk is that **SHARD-001's immutability clause and IDR-001's one-group-per-shard clause, taken literally, forbid the two techniques every planet-scale system relies on: sub-shard resharding and shard splitting.** If ARVES is standardized by ISO/IEEE tomorrow, an independent implementer reading SHARD-001 will build a system that *cannot* rebalance a hot tenant, and will conclude the standard does not scale. The fix is not a spec change — it is an **IDR that defines re-sharding as migration-to-a-new-shard-key** (which the spec explicitly permits) plus a **capacity/conformance requirement** that forces every certified runtime to demonstrate rebalancing. That reconciliation is the highest-value scalability work available.

The nine findings below are ranked by severity. Rough capacity math accompanies each so the numbers, not opinions, drive the decisions.

### Baseline capacity math (used throughout)

| Quantity | Assumption | Derived |
|---|---|---|
| Observations/day | 1e9 | **11,574 obs/s** average; assume **3× peak = ~35k obs/s** |
| Entities | 1e8 | at 5,000 tenants → **20,000 entities/tenant** average (power-law: top tenants 100×) |
| Nodes | 1e4 | if a node hosts ~1 vCPU-equiv Raft duty per few hundred groups |
| Tenants | 5e3 | workspaces/tenant unknown in corpus; assume **10–1,000**, power-law |
| Shards (= Raft groups) | tenant×workspace | **50k–5M groups** depending on workspace fan-out |
| Per-obs WAL record | ~256 B–1 KB canonical | **~0.25–1 TB/day** raw WAL before compaction |
| Raft replication factor | 3–5 (from IDR-001 CP) | **3–5× WAL write amplification** = 0.75–5 TB/day physical |

These numbers are the yardstick. 35k commits/s across 5M Raft groups is trivially shardable; 35k commits/s into *one hot tenant's one workspace shard* is not. That asymmetry is the whole review.

### Severity-ranked findings

| # | Severity | Title | Instrument | Complexity |
|---|---|---|---|---|
| 1 | **Critical** | SHARD-001 immutability + one-group-per-shard forbids resharding / hot-shard split | IDR + Certification | high |
| 2 | **Critical** | Per-append `fsync` and global commit mutex cap kernel throughput far below 1B/day | Runtime + IDR | high |
| 3 | **High** | Full-state snapshot & serial single-threaded replay are O(shard-size); recovery time unbounded | Runtime + IDR | high |
| 4 | **High** | No cross-shard saga coordinator, no back-pressure, no admission control anywhere | IDR + Runtime | very-high |
| 5 | **High** | No shard-placement / metadata / routing plane defined; 5M-group directory is itself a scaling problem | IDR + Runtime | high |
| 6 | **Medium** | Read-tier scaling: follower/replica fan-out and cross-shard query aggregation undefined | IDR + Runtime | high |
| 7 | **Medium** | Capability scheduling (I4) has no load model; hot capability/provider contention unaddressed | IDR + Runtime | medium |
| 8 | **Medium** | AP observability plane (metrics/CRDT presence) has no cardinality budget at 5M shards × 10k nodes | IDR + Verification | medium |
| 9 | **Low** | No scale conformance scenarios; certification cannot distinguish a toy runtime from a planet-scale one | Certification + Verification | medium |

---

## Finding 1 — SHARD-001 immutability + one-Raft-group-per-shard forbids resharding and hot-shard splitting (CRITICAL)

**Where:** `Amendments_CCP_Batch_1` A-004 / SHARD-001 ("partition key is immutable for an entity lifetime"); `IDR_Batch_1` IDR-001 ("one independent Raft group PER SHARD (tenant/workspace)"); reference: `arves-consensus/src/lib.rs` `ShardId` ("Re-partitioning is a migration ... never a mutation"), `arves-kernel/src/lib.rs` `ShardKey`.

**Why it matters.** The shard = `(tenant, workspace)` is the *only* partition axis in the frozen corpus, and it is fixed for the entity lifetime. That makes two facts true at once:

1. A single hot tenant/workspace (e.g. an enterprise tenant ingesting 30k obs/s into one workspace) lands on **one Raft group with one leader**. A single Raft leader tops out at roughly **10k–50k commits/s** with per-entry consensus, and *cannot be split*, because the shard key is immutable and there is exactly one group per shard. The hottest 1% of 5,000 tenants under a power law will each demand 10–100× the mean — precisely the tenants that break the model.
2. Conversely, 5M tiny workspaces each get their own Raft group (each needing 3–5 replicas, heartbeats, election timers, a WAL directory). At 5M groups × 3 replicas / 10k nodes = **1,500 Raft groups per node** minimum, each with independent tick/heartbeat timers — a well-known "too many Raft groups" scaling wall (the problem CockroachDB/TiKV solve with Multi-Raft + range splitting, and Kafka solved by moving off ZooKeeper-per-partition).

The spec's own escape hatch is buried in the reference code comment: *"Re-partitioning is a migration (a new shard + data movement), never a mutation of an existing key."* This is correct and it is the entire answer — but it is **not normatively defined anywhere**, has **no conformance scenario**, and no reference implementation. An independent ISO/IEEE implementer will read SHARD-001 literally, conclude the partition is permanently fixed, and build an unscalable system.

**Risks.** (a) Hot-shard write starvation with no legal remedy. (b) Raft-group explosion (heartbeat storms, election flapping) for the long tail of small workspaces. (c) Divergent third-party implementations: one team invents range-splitting, another invents workspace-sub-sharding, and their runtimes become mutually incompatible — the death of a standard.

**Long-term consequences.** Left unresolved, SHARD-001 becomes the clause that critics cite to say "ARVES doesn't scale." Resolved well (as a migration IDR + Multi-Raft grouping IDR), it becomes the clause that proves it does.

**Alternative designs.**
- *(A) Sub-shard ranges under a stable shard key.* Keep `(tenant, workspace)` as the logical shard key (SHARD-001 satisfied — the key never mutates), but define an internal, splittable **range/hash sub-partition** below it, each range its own Raft group. This is the CockroachDB model and is fully spec-compatible: the *entity's* partition key is immutable; the physical range it lives in is an implementation detail.
- *(B) Coalesced Multi-Raft.* Batch many cold shards into a shared Raft group with logically isolated state (Kafka KRaft / TiKV "region" coalescing), splitting out a shard to its own group only when it grows hot. Solves the 5M-group explosion.
- *(C) Live migration to a new shard key* for the rare case where the *logical* key itself must change (workspace merge/split at the product level) — modeled as a saga (create new shard, replay-copy, cut over, retire old), which the spec already permits.

Recommend **A + B together**: A handles hot-shard splitting; B handles cold-shard coalescing. Both preserve SHARD-001 verbatim because neither mutates an entity's key — they change only physical placement.

**Recommendation.** File **IDR-006 "Sub-shard Ranges and Coalesced Multi-Raft"**: a logical shard `(tenant, workspace)` maps to one-or-more physical Raft *ranges*; ranges split on a size/throughput threshold and coalesce when cold; entity→range routing is by a stable hash of the immutable entity key, so SHARD-001 holds. Pair with **IDR-007 "Logical Re-Sharding as Saga"** for the rare logical-key change. Add a **Certification requirement**: a runtime cannot certify at "distributed scale" level without demonstrating an automatic hot-shard split under sustained overload.

**Implementation complexity:** high. **Scientific impact:** high — reconciling an immutable-key invariant with dynamic rebalancing is a citable formal result. **Ecosystem impact:** decisive — this is the difference between "adoptable planet-scale standard" and "elegant toy."

---

## Finding 2 — Per-append `fsync` and a global commit mutex cap kernel throughput orders of magnitude below 1B/day (CRITICAL)

**Where:** `arves-persistence/src/lib.rs` `FileWal::append` — `inner.current.sync_all()` on *every* record (line ~948); `arves-kernel/src/lib.rs` `RefKernel { state: Mutex<KernelState> }` and `commit()` taking `state.lock()` for the *entire* commit path across *all* shards in the process (lines ~445, ~654).

**Why it matters.** Two independent ceilings, both far below target:

1. **Per-append fsync.** A single `fsync` costs ~0.1–10 ms depending on media (NVMe with power-loss protection ~50–100 µs; commodity SSD/EBS ~1–10 ms). At 1 ms/fsync, a shard's WAL does **~1,000 appends/s**. Target average is 11,574 obs/s and peak ~35k/s. Even spread perfectly across shards this is survivable, but any hot shard (Finding 1) that must absorb thousands of obs/s will be fsync-bound at ~1k/s — a **10–35× shortfall** on the hot path. The reference explicitly chose "correctness over speed" (a correct I1 choice), but there is no IDR defining the production durability strategy.
2. **Global commit mutex.** `RefKernel` holds *one* `Mutex<KernelState>` guarding a single `Vec<(TruthRef, Vec<u8>)>` and `HashMap` index for *every shard in the process*. `commit()` locks it, opens the WAL, appends+fsyncs *while holding the lock*, then mutates the in-memory truth set. This serializes all commits process-wide — even commits to unrelated shards on unrelated Raft groups. At scale, a node hosting 1,500 groups (Finding 1) would funnel all of them through one mutex and one fsync at a time. This is a per-node throughput cliff.

**Risks.** (a) Missing the average ingest rate the moment shards co-locate on a node. (b) Head-of-line blocking: a slow-disk shard stalls every other shard's commits sharing the process. (c) Tail-latency amplification under the CP model (a commit already waits for Raft quorum *and* now waits behind an unrelated shard's fsync).

**Long-term consequences.** These are reference-runtime defects, not spec defects — but if the reference runtime is what third parties benchmark against, ARVES will be dismissed as slow. The reference implementation is the standard's shop window.

**Alternative designs.**
- *Group commit / batched fsync* (the classic WAL technique): coalesce N appends arriving within a small window into one fsync, amortizing the sync across the batch. Raises per-shard throughput from ~1k/s to ~50–200k/s. This is a Runtime change with an IDR to record the durability semantics (a commit is durable once its batch fsyncs; quorum ack follows).
- *Per-shard state + per-shard lock* (or lock-free per-shard append): replace the single `Mutex<KernelState>` with a shard-keyed map of independently-locked shard states, so unrelated shards commit concurrently. Pure Runtime refactor; the `Kernel` trait's `commit(&self, ...)` signature already permits interior per-shard locking.
- *Pipelined Raft* (propose next entry before the previous is fully durable, commit in log order) — the standard Raft throughput technique — recorded as an IDR because it interacts with the "no partial truth" guarantee.

**Recommendation.** (1) **Runtime**: shard-partitioned kernel state with per-shard locking; group-commit batched fsync in `FileWal`. (2) **IDR-008 "Durability & Group Commit Semantics"**: define when a commit is considered durable (batch fsync) vs. committed (quorum), preserving CP truth and ORCH-003 replay. (3) **Verification**: a throughput micro-benchmark gate in CI (commits/s per shard, and aggregate commits/s per node with K co-located shards).

**Implementation complexity:** high. **Scientific impact:** medium (well-trodden techniques, but formalizing durability-vs-commit under ARVES's CP truth is worth writing down). **Ecosystem impact:** high — the reference runtime's benchmark numbers are what the market judges.

---

## Finding 3 — Full-state snapshots and serial single-threaded replay are O(shard size); recovery time is unbounded at scale (HIGH)

**Where:** `arves-kernel/src/lib.rs` `snapshot_shard` (materializes the *entire* shard truth set into one `Vec<u8>` under the global lock, lines ~576–586), `checkpoint` (iterates *all* shards serially, lines ~613–629), `try_replay` (single-threaded, per-shard sequential, lines ~506–571); `arves-persistence` `FileWal::replay_from` reads *all* retained segments and rebuilds one `Vec<WalRecord>` (lines ~1086–1146).

**Why it matters.** Three compounding O(N) costs:

1. **Snapshot cost.** `checkpoint()` writes a *full* materialized-state blob per shard (`encode_shard_blob` over every committed truth). For a hot shard with millions of entities this is a multi-GB write that (a) holds the global kernel mutex while collecting entries (`snapshot_shard` locks `state`), blocking all commits, and (b) rewrites the entire state every checkpoint even if 1% changed. At 100M entities and periodic checkpointing, full-state snapshots are the dominant I/O cost and a stop-the-world pause.
2. **Replay/recovery time.** Recovery replays the entire retained tail into an in-memory `Vec`, one record at a time, one shard at a time. A node hosting 1,500 groups recovers them **sequentially**. If each shard has even 100k tail records, that is 150M record decodes single-threaded on startup — minutes to hours of downtime per node restart, violating any reasonable SLO. `replay_from` also builds a full `Vec<WalRecord>` in memory before returning — O(tail) memory per shard.
3. **Segment scan.** `replay_from` and `load_snapshot` call `list_segments`/`read_dir` and `fs::read` the whole file every time; recovery cost scales with total retained bytes, not with delta.

**Risks.** (a) Checkpoint pauses stall ingest on hot shards. (b) Node restart / failover recovery time grows without bound as shards grow → cascading failover storms (a node takes too long to recover, its Raft groups re-elect elsewhere, load concentrates, more nodes fall over). (c) OOM on recovery of a large shard (full `Vec<WalRecord>` in memory).

**Long-term consequences.** Recovery time is the hidden killer of "10,000 nodes" — MTTR dominates availability at fleet scale. A standard that cannot bound recovery time cannot promise an SLA.

**Alternative designs.**
- *Incremental / delta snapshots* (LSM-style or checkpoint-of-changes-since-last): snapshot only mutated state; keeps snapshot cost proportional to churn, not to shard size. Requires an IDR because it changes what "snapshot" means for ORCH-003 replay (snapshot + delta chain).
- *Streaming, bounded-memory replay*: iterate the cursor and fold into state without materializing a full `Vec` (the cursor abstraction already supports this; the current code eagerly collects). Pure Runtime fix.
- *Parallel per-shard recovery*: recover the node's N groups concurrently (they are independent by SHARD-001). Pure Runtime fix; bounded by disk/CPU parallelism.
- *Learner-based fast catch-up*: a recovering node streams a recent snapshot from a healthy peer (Raft `InstallSnapshot`) rather than replaying local WAL from zero — the standard Raft answer, already implied by IDR-002/003.

**Recommendation.** (1) **IDR-009 "Incremental Snapshots + Snapshot Chains"** defining ORCH-003-preserving delta snapshots. (2) **Runtime**: streaming bounded-memory replay; parallel per-shard recovery; snapshot production off the commit-critical mutex (snapshot a copy-on-write handle, not under the write lock). (3) **Verification**: recovery-time and snapshot-pause benchmarks with an explicit budget (e.g. recover 1,500 groups in < 60 s; checkpoint pause < 50 ms).

**Implementation complexity:** high. **Scientific impact:** medium. **Ecosystem impact:** high — bounded MTTR is a certification-grade promise.

---

## Finding 4 — No cross-shard saga coordinator, no back-pressure, no admission control anywhere in spec or runtime (HIGH)

**Where:** `IDR_Batch_1` line 17 ("cross-shard coordination uses sagas/compensation (Amendment-006), not distributed transactions"); `Amendments_CCP_Batch_1` A-006 (compensation model, "mechanisms → IDR"); `Vol_9_Control_Plane` (Orchestrator, Task Graph) — none define a saga *runtime*; no crate implements sagas, queues, or back-pressure; `arves-control-plane` `Orchestrator` has `dispatch`/`resolve` but no flow control.

**Why it matters.** The frozen spec *correctly* rejects distributed transactions and mandates sagas — but a saga is only as good as its **coordinator**, and none exists. At 1B obs/day, cross-shard cognition is not rare: entity resolution links entities across workspaces; multi-agent runs (I5) span tenants; a plan that reads shard A and writes shard B is a two-shard saga. Every one of these needs a durable coordinator with timeout, retry, compensation, and idempotent replay. Without it, cross-shard operations are non-atomic *and* non-recoverable.

Worse, there is **no back-pressure model** anywhere. The Kernel `commit` returns `NotReplicated` on quorum failure but there is no signal for "leader is overloaded, slow down." The Information Platform proposes writes; the Control Plane dispatches; the Kernel commits — but nothing tells an upstream producer to stop when a shard leader is saturated (Finding 2). At 3× peak ingest into a hot shard, the system has no defined behavior except unbounded queue growth → OOM → cascading failure. This is the classic missing-flow-control failure that takes down event-driven systems.

**Risks.** (a) Partial cross-shard effects with no compensation driver → silent inconsistency across shards (the one thing the CP model was supposed to prevent). (b) Unbounded queue growth under overload → memory exhaustion → correlated multi-node failure. (c) Retry storms: without admission control, a failing shard's retries amplify load (metastable failure).

**Long-term consequences.** Sagas + back-pressure are where distributed systems live or die operationally. A standard that specifies "use sagas" without specifying the coordinator and the flow-control contract is descriptive, not normative — it will fail the Reference Lifecycle's own independent-implementability test.

**Alternative designs.**
- *Saga coordinator as a Control-Plane plan* (fits ORCH-002: the saga is a plan, its steps are single-shard commits, its state lives in the decision trace, not as new truth). Each step is idempotent + content-addressed (ORCH-004), so replay drives compensation. This is spec-aligned and needs an IDR to pin the coordinator's durability (the coordinator's own state is a per-shard log entry, not a second truth owner).
- *Credit-based / token-bucket back-pressure* on the commit path: the shard leader advertises commit credits; producers block/shed when credits are exhausted. Recorded as an IDR because it defines an observable contract (a new `CommitError::Overloaded` / retry-after) without changing truth ownership.
- *Load shedding tiers*: shed eventual-tier work first, protect linearizable truth commits last.

**Recommendation.** (1) **IDR-010 "Saga Coordinator & Compensation Runtime"** — coordinator-as-plan, durable in the decision trace, idempotent compensation, timeout/retry. (2) **IDR-011 "Back-Pressure & Admission Control"** — credit-based flow control on the commit path + load-shedding order by read tier. (3) **Runtime**: implement both, plus a bounded queue with explicit overflow policy. (4) **Verification**: failure-injection tests (overload a hot shard, kill a coordinator mid-saga, assert no partial truth and bounded memory).

**Implementation complexity:** very-high. **Scientific impact:** high (formal saga-as-plan + back-pressure under CP truth). **Ecosystem impact:** high — operational credibility.

---

## Finding 5 — No shard-placement / metadata / routing plane defined; the 50k–5M-group directory is itself a scaling problem (HIGH)

**Where:** `arves-persistence` `FileWalStore::shards()` (scans the root directory and returns *all* shards; lines ~1282–1313) and `dir_for` (one directory per shard); nothing in the corpus defines where a shard *lives*, how a client *finds* the current leader, or how the shard→node map is stored and scaled.

**Why it matters.** With up to 5M Raft groups over 10k nodes, the **placement/metadata plane** — "which nodes host shard S, who is its current leader, where do I route this commit?" — is a first-class distributed system in its own right, and it is entirely undefined. The reference `shards()` enumerates every shard by reading a directory: fine for a single node, catastrophic as a mental model for 5M shards (you cannot `read_dir` a 5M-entry directory, and you certainly cannot enumerate all shards to route one commit). Every serious system has a dedicated metadata service (Kafka controller / KRaft, HDFS NameNode, CockroachDB meta-ranges, TiKV PD). ARVES has none, and IDR-001..005 explicitly scope themselves to *per-shard* consensus, leaving the *cross-shard placement* consensus unmentioned.

The routing plane must itself be CP (a stale leader map causes commits to the wrong node), but it must not become a global bottleneck (routing 35k commits/s through one metadata leader recreates the "global single-leader" that IDR-001 rejected). This is a genuine architecture gap, not just missing code.

**Risks.** (a) No defined way to find a shard's leader → every implementer invents their own → incompatible clients. (b) Metadata hot spot: a naive central directory becomes the throughput ceiling. (c) Rebalancing (Finding 1) is impossible without a placement authority to move ranges.

**Long-term consequences.** The metadata plane is the connective tissue of the whole fleet. Undefined, it is where independent implementations will diverge most and interoperate least.

**Alternative designs.**
- *Hierarchical meta-ranges* (CockroachDB model): a small, itself-sharded, CP index that maps entity-key ranges → shard ranges → nodes, cached aggressively at clients with lease epochs for staleness detection. Scales because the meta index is tiny relative to data and is itself range-split.
- *Gossip + lease-based leader discovery*: leaders publish lease epochs over an AP gossip layer; clients cache and detect staleness via epoch mismatch on `NotLeader`. Fits "truth CP, observability AP" — placement *facts* are CP, leader *hints* are AP.
- *Placement Driver service* (TiKV PD analogue) that owns rebalancing decisions, consuming the AP load stats (Finding 8) and issuing range moves.

**Recommendation.** **IDR-012 "Shard Placement, Metadata & Routing Plane"**: a hierarchical CP meta-index (entity → range → shard → leader), client-side caching with lease epochs, and `NotLeader`-driven cache invalidation (the reference `CommitError::NotLeader` already carries the shard, and should be extended in the runtime to carry a leader hint). Add a **Placement Driver** component (Runtime) that consumes load telemetry and drives Finding 1's splits/coalesces. Keep the reference `shards()` as a single-node convenience but document that production routing MUST go through the meta-index, never a directory scan.

**Implementation complexity:** high. **Scientific impact:** medium. **Ecosystem impact:** high — routing is a hard interoperability boundary.

---

## Finding 6 — Read-tier scaling: follower fan-out and cross-shard query aggregation are undefined (MEDIUM)

**Where:** `IDR_Batch_1` Read Consistency Tiers (linearizable/bounded-staleness/eventual); `arves-query/src/lib.rs` (read-only projections, per-shard `ReadScope`, "no cross-shard atomic read here"); milestone I3 Distributed Query.

**Why it matters.** The read model is elegantly specified at the *contract* level (three tiers, per-shard scope) but the *scaling* mechanics are open. Two concrete gaps at 100M entities / 1B obs/day:

1. **Linearizable read amplification.** `ReadTier::Linearizable` "requires confirming currency with the per-shard Raft leader." If dashboards, agents, and the Experience layer default to linearizable reads, every read hits a leader — recreating the leader hot spot on the *read* path. Read-heavy cognitive workloads (search, graph exploration) will overwhelm leaders. The standard read-index optimization (leader confirms commit index without a log append) helps but is undefined; follower-served bounded-staleness reads need a defined staleness-verification protocol.
2. **Cross-shard queries.** The Query layer explicitly forbids cross-shard atomic reads (correct for consistency), but real queries — "all documents mentioning entity X across my workspaces," knowledge-graph traversal — are inherently cross-shard. There is no defined scatter-gather / aggregation model, no fan-out limit, no partial-result semantics. At 5M shards a naive fan-out is impossible; queries must be routed by a secondary index, which does not exist.

**Risks.** (a) Read load collapses onto leaders. (b) Cross-shard queries either don't work or are reinvented incompatibly. (c) Knowledge-graph traversal (a core ARVES value prop per Vol 3) has no scaling story.

**Long-term consequences.** ARVES sells "Information → Intelligence." If intelligence queries don't scale, the product thesis doesn't hold.

**Alternative designs.** Read-index + lease-read for linearizable without log append; follower reads with leader-issued read leases for bounded staleness; a scatter-gather query coordinator with fan-out caps and partial-result + staleness annotations for eventual-tier cross-shard queries; a separate, AP-tier secondary/inverted index (materialized by the Query layer as read-only projections) to avoid full fan-out.

**Recommendation.** **IDR-013 "Read-Index, Follower Reads & Cross-Shard Query Fan-Out"** (defines the linearizable read-index protocol, bounded-staleness lease reads, and a scatter-gather aggregation contract with fan-out limits). **Runtime**: implement follower reads and a query coordinator in I3. **Verification**: read-throughput benchmark that proves reads scale with replica count, not leader count.

**Implementation complexity:** high. **Scientific impact:** medium. **Ecosystem impact:** high (product-critical).

---

## Finding 7 — Capability scheduling (I4) has no load model; hot capability/provider contention is unaddressed (MEDIUM)

**Where:** `arves-capability-fabric/src/lib.rs` (registry/bindings only, "one active binding per capability per shard", `resolve` is a pure read); milestone I4 Capability Scheduling; `Vol_9` Capability Planner (Control Plane). No queueing, rate-limit, or fair-share model exists.

**Why it matters.** Capabilities bind to concrete providers (LLM endpoints, tools, execution adapters). At scale the scarce resource is not the shard — it's the **provider** (a rate-limited LLM API, a GPU pool, a robot). CAP-002's "one active binding per capability per shard" means 5M shards may all bind the same physical LLM provider. Without a scheduling/fair-share/rate-limit model, one tenant's burst starves 4,999 others of the shared provider, and there is no back-pressure from provider → planner (compounding Finding 4). The Engine Graph manifest carries `Cost`/`Latency`/`Retry` metadata, but nothing consumes it for admission or fair scheduling.

**Risks.** (a) Noisy-neighbor: one tenant exhausts a shared provider's quota. (b) Thundering herd of retries against a throttled provider (metastable failure). (c) No SLO differentiation between tenants (enterprise vs. free tier).

**Long-term consequences.** Multi-tenant fairness is a hard requirement for any tenant-aware platform (Vol 2 makes tenancy foundational). Undefined, it becomes a support/abuse nightmare and blocks enterprise adoption.

**Alternative designs.** Hierarchical weighted fair-share scheduling (tenant → workspace → plan) over shared providers; per-provider token buckets with tenant quotas; priority-aware preemption (Amendment-005 already defines priority/preemption — I4 should *consume* it); circuit breakers per provider to stop retry storms.

**Recommendation.** **IDR-014 "Capability Scheduling: Fair-Share, Quotas & Provider Back-Pressure"** for I4 — hierarchical fair-share over providers, per-tenant quotas, circuit breakers, and consumption of the manifest's Cost/Latency/Priority metadata. **Runtime**: a scheduler that sits between the Capability Planner and the Fabric. **Verification**: noisy-neighbor and provider-throttle failure-injection scenarios.

**Implementation complexity:** medium. **Scientific impact:** medium. **Ecosystem impact:** high (enterprise multi-tenancy).

---

## Finding 8 — AP observability plane has no cardinality budget at 5M shards × 10k nodes (MEDIUM)

**Where:** `IDR_Batch_1` "Metrics/logs/tracing = AP (eventual); Presence/capability statistics = AP (CRDT)"; `Vol_9_Runtime_Event_Fabric` Parts 22–23 (Observability, Telemetry). No cardinality/aggregation model.

**Why it matters.** The spec wisely puts observability on an AP tier so it never blocks truth. But at 5M shards × 10k nodes, *observability itself* is a scaling problem: per-shard leader/lag/throughput metrics = tens of millions of time series; CRDT presence/capability-stats gossip across 10k nodes has O(N²) fan-out risk. Prometheus-style pull collapses at this cardinality. The AP designation prevents observability from harming truth, but does not prevent observability from harming *itself* (and the Placement Driver in Finding 5 *depends* on this telemetry to make rebalancing decisions — so if observability drowns, rebalancing goes blind).

**Risks.** (a) Metric cardinality explosion → monitoring stack OOM. (b) CRDT gossip storm. (c) Placement Driver starved of the load signal it needs → poor rebalancing → back to Finding 1.

**Long-term consequences.** "Observable" is a Vol 18 success criterion. Unbudgeted, it silently caps the fleet size.

**Alternative designs.** Pre-aggregation at the node (roll up per-shard series into per-node/per-tenant series before export); hierarchical CRDT gossip with fan-in trees, not full mesh; sampled tracing with tail-based sampling; a bounded, versioned load-summary CRDT specifically for the Placement Driver rather than raw metrics.

**Recommendation.** **IDR-015 "Observability Cardinality Budget & Load-Summary CRDT"** — define node-side pre-aggregation, hierarchical gossip topology, and a compact load-summary structure the Placement Driver consumes. **Verification**: a cardinality budget test (series count as a function of shard/node count must be sub-quadratic).

**Implementation complexity:** medium. **Scientific impact:** low-medium. **Ecosystem impact:** medium.

---

## Finding 9 — No scale conformance scenarios; certification cannot distinguish a toy from a planet-scale runtime (LOW severity, HIGH leverage)

**Where:** `Scenario_Conformance_Framework` (12 axes, reference scenarios like Warehouse Robot Dispatch); `Reference_Lifecycle` CCP-GATE ("No behaviour is ratified without a conformance scenario"); `Certification` program (deferred to Implementation Era). The 12 axes and reference scenarios are correctness/behavior-oriented; none assert *scale* properties.

**Why it matters.** ARVES's greatest structural strength is that behavior is ratified only with a conformance scenario. But there are **no scalability conformance scenarios**. A runtime that passes every current scenario on a single node with one shard is "conformant" — yet may fall over at Finding 1/2/3. Certification is the mechanism that makes the standard *mean* something to an adopter; without scale gates, a certificate says nothing about whether the runtime scales. Since every Finding above proposes an IDR, and CCP-GATE requires a scenario per behavior, the *natural home* for enforcing Findings 1–8 is a set of scale conformance scenarios.

**Risks.** (a) Certified-but-unscalable runtimes erode trust in the certificate. (b) IDRs 6–15 get written but never verified because there's no scenario obligating them. (c) Third-party runtimes claim scale without proof.

**Long-term consequences.** The certificate is the standard's currency. Scale scenarios are what back that currency for infrastructure buyers.

**Alternative designs.** Add a **Scale Conformance Suite** (pinned to spec version per Lifecycle Part 7): hot-shard split under overload (Finding 1); sustained ingest at target rate with bounded latency (Finding 2); bounded recovery time for K co-located shards (Finding 3); saga durability + no-partial-truth under coordinator kill and bounded memory under overload (Finding 4); routing correctness under leader change (Finding 5); read-throughput-scales-with-replicas (Finding 6); noisy-neighbor fairness (Finding 7); sub-quadratic observability cardinality (Finding 8). Define **certification tiers**: "Single-Node," "Cluster," "Planet-Scale," each gated by a subset.

**Recommendation.** **Certification + Verification**: create the Scale Conformance Suite and tiered certification levels; make every IDR from Findings 1–8 cite its scale scenario (satisfying CCP-GATE). This is low code complexity but the highest-leverage governance move — it turns the other eight findings from "good ideas" into "certification obligations."

**Implementation complexity:** medium. **Scientific impact:** medium. **Ecosystem impact:** decisive — this is how a standard *enforces* scalability across independent implementations.

---

## If ARVES were standardized by ISO/IEEE tomorrow — what is still missing (scalability lens)

1. **A normative reconciliation of SHARD-001 immutability with dynamic rebalancing.** The single most important gap: without an IDR defining sub-shard ranges + coalesced Multi-Raft + logical re-sharding as migration, an independent implementer builds an unscalable system *while fully conforming to the frozen spec*. (Finding 1)
2. **A defined placement/metadata/routing plane.** IDR-001..005 specify per-shard consensus but leave the cross-shard "where does this shard live, who leads it" plane undefined — the connective tissue of any fleet. (Finding 5)
3. **A saga coordinator and a back-pressure contract.** The spec mandates sagas and rejects distributed transactions, but never defines the coordinator or flow control — so cross-shard atomicity and overload behavior are undefined. (Finding 4)
4. **Scale conformance scenarios and tiered certification.** Behavior is ratified with scenarios; scale is not, so a certificate carries no scale guarantee. Adding scale scenarios is what makes every other fix enforceable across the ecosystem. (Finding 9)
5. **Reference-runtime scaling techniques** (group-commit fsync, per-shard locking, incremental snapshots, streaming/parallel replay, follower reads). These are Runtime, not spec — but the reference runtime *is* the standard's proof of scalability, so its current single-mutex/per-append-fsync/full-snapshot posture must evolve before "1B/day" is credible. (Findings 2, 3, 6)

None of these require touching the frozen corpus. All fit the existing change instruments (IDR, CCP Amendment, Runtime, Verification, Certification). The frozen SHARD-001/IDR-001 decisions are, in fact, the right foundation — they just need the Implementation Era to supply the rebalancing, routing, saga, flow-control, and conformance machinery the Specification Era deliberately deferred.

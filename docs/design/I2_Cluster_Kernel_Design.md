# I2 — Cluster Kernel · ENGINEERING DESIGN PACKAGE

```
=====================================================================
 STATUS: DESIGN PACKAGE (Ch4 PREP MODE) — NO CODE
 Build gate (G2) CLOSED. Prepared 2026-07-05 under the maintainer
 prep-mode ruling: design documents only; no implementation may
 begin until G2 opens and the required RCRs are ratified.
=====================================================================
```

**Milestone:** I2 — Cluster Kernel (frozen Baseline, `spec-markdown/ARVES_00_Baseline_v1.md`
Part 5: *"I2 Cluster Kernel — Kernel replication; consensus if required"*).
**Intent:** the single-node I1 Kernel becomes a sharded, Raft-replicated cluster kernel —
one Raft group per `(tenant, workspace)` shard, leader-only commit, follower apply of
committed outcomes, joint-consensus membership, append-only WAL as the single replicated
decision trace (IDR-001..005).
**Predecessor artifacts (read-only inputs):**
`runtime/docs/I1_Engineering_Design.md` (I1 complete; single-shard walking skeleton),
`runtime/docs/I2.0_Engineering_Design.md` (earlier I2.0 replication-first staging note),
`runtime/crates/arves-consensus/src/lib.rs` (CONTRACT-ONLY consensus surface, frozen v1.0),
`runtime/crates/arves-kernel/src/lib.rs`, `runtime/crates/arves-persistence/src/lib.rs`
(IMPLEMENTED I1 kernel/WAL), `runtime/RUNTIME_FREEZE_v1.0.md` (freeze + RCR process +
v1.1 debt).
**Hard constraint:** `runtime/` and `standard/` are FROZEN (Runtime v1.0, tag
`runtime-v1.0`). Every line of I2 implementation that would land under `runtime/` requires
a ratified **Runtime Change Request** first (`runtime/RUNTIME_FREEZE_v1.0.md`, "Runtime
Change Request (RCR) — the only way the runtime changes"). This package designs; it does
not change any frozen surface.

---

## 0. Architecture Readiness Review (constitution workflow step 1)

*(Mirrors the precedent of I1 design §1 "Architecture Readiness Check" and I2.0 §1
"Architecture Readiness Review".)*

| Readiness input | State | Evidence |
|---|---|---|
| Specification frozen and ARR at PASS | YES | `spec-markdown/ARVES_00_Architecture_Readiness_Review_v1.md` (ARR PASS); Freeze Record; Baseline v1 |
| Predecessor milestone complete | YES | I1 COMPLETE (git tag `I1-complete`; `runtime/docs/I1_Engineering_Design.md` §7 Definition of Done) |
| Runtime substrate stable | YES | Runtime v1.0 FROZEN (tag `runtime-v1.0`); the RCR instrument exists for every required runtime change (`runtime/RUNTIME_FREEZE_v1.0.md`) |
| Distribution decisions recorded | YES | IDR-001..005 (`spec-markdown/ARVES_IDR_Batch_1_Kernel_Distribution_v1.md`) fully specify I2's architecture |
| Contract surface for I2 exists | YES | `arves-consensus` CONTRACT-ONLY crate (frozen v1.0) declares every type and trait I2 implements |
| Invariant registry authoritative | YES | `ARVES_00_Invariant_Registry_v1.md` — registered vs PROPOSED separation applied throughout (§4) |

I1 proved a single node holds durable, replayable, recoverable truth; the frozen corpus
(IDR Batch 1) already specifies exactly how that node becomes a sharded, Raft-replicated
cluster; the frozen contract crate already names every I2 surface. Nothing architectural
remains to invent — only to implement under the RCR/IDR instruments listed in §6.

**Verdict: READY** — for *design* (this package). Build readiness is separately gated:
G2 remains CLOSED until the §6.2 gate order completes.

---

## 1. BEFORE-WRITING-CODE — the ten constitutional answers

*(Engineering Constitution, "BEFORE WRITING CODE". All ten answered in full before any
implementation may be scheduled.)*

### 1.1 Which UCI node is affected?

Primary: the **Kernel** node (truth owner and sole commit gateway — Vol 9 v2 Part 5,
ORCH-001; Amendments Batch 1 layer matrix, Kernel row) and the **consensus/replication
substrate beneath it** (the `arves-consensus` contract crate — IDR-001..005). Secondary,
touched but not redesigned: **Persistence** (the WAL becomes the replicated Raft log it
was already shaped as — IDR-005) and **Query** read-tier semantics (IDR-001 read tiers
become real once followers exist). NOT affected: LCW, Engine Fabric, Capability Fabric,
Execution, Control Plane, Information Platform — their contracts are unchanged; engines
already "run anywhere" by design (IDR-001 Engineering Refinements).

No new architectural layer is introduced (Non-Negotiable Rule 3): consensus is the
replication mechanism *under* the Kernel's commit gateway, already positioned in the
frozen corpus (IDR-001; `arves-consensus` crate header: "cross-cutting (truth path;
serves the Kernel's commit gateway)").

### 1.2 Which documents govern it?

| Document | What it governs here |
|---|---|
| `ARVES_00_Baseline_v1.md` Part 5 | Milestone scope: "I2 Cluster Kernel — Kernel replication; consensus if required" |
| `ARVES_IDR_Batch_1_Kernel_Distribution_v1.md` (IDR-001..005) | CP posture, per-shard Raft, outcome replication, joint consensus, election, WAL storage, read tiers, CP/AP boundary |
| `ARVES_Volume_9_Cognitive_Control_Plane_v2.md` Part 5 | ORCH-001..004 (the constitutional core; "the foundation of the future distributed runtime") |
| `ARVES_Volume_9_Cognitive_Control_Plane_v2.md` Parts 9, 11 | Replay-from-trace; distribution-readiness constraints (plans serializable; control plane stateless/replicable) |
| `ARVES_00_Amendments_CCP_Batch_1_v1.md` A-003/A-004/A-005/A-006 | LAYER-001 + layer matrix; SHARD-001 shard identity; cancellation leaves no partial truth; saga/compensation failure model (no cross-shard 2PC) |
| `ARVES_00_Invariant_Registry_v1.md` | Which invariants are registered-normative vs PROPOSED |
| `ARVES_Scenario_Conformance_Framework_v1.md` Parts 5, 8, 10 | Conformance axes, property/invariant semantics, L3 "Distributed" level |
| `ARVES_Reference_Lifecycle_v1.md` Part 6 | CCP-GATE: "No behaviour is ratified without a conformance scenario" |
| `runtime/RUNTIME_FREEZE_v1.0.md` | Freeze boundary; RCR instrument; v1.1 debt (incl. Kernel batch-commit) |

### 1.3 Which contracts apply?

- **`arves_consensus::ShardConsensus`** (frozen contract-only crate) — the central I2
  contract: `propose(shard, Outcome) -> LogIndex`, `await_commit`, `leader`, `role`,
  `read_index(shard, ReadTier)`, `change_membership(shard, Membership)`. I2 is, at its
  core, the first implementation of this trait. Supporting types: `ShardId(tenant,
  workspace)` (SHARD-001), `Term`/`LogIndex`/`LogEntry` (IDR-005), `Outcome` ("committed
  OUTCOMES, not invocations" — IDR-002), `Membership::{Stable,Joint}` (IDR-003),
  `Leadership`/`Role` (IDR-004), `ReadTier` (IDR-001), `ConsensusError` (incl.
  `NotLeader`, `QuorumUnavailable`).
- **`arves_kernel::Kernel`** — `commit(ProposedWrite) -> TruthRef`, idempotent and
  content-addressed (ORCH-004), sole commit gateway (ORCH-001/OWN-001). I2 routes the
  commit through the shard leader's consensus group; the Kernel trait surface itself is
  not changed (additive wiring only, or RCR if a signature must change).
- **`arves_persistence::Wal` / `WalStore`** — append-only, `replay_from(offset)`,
  fsync-durable `FileWal` with hash-chain `integrity_digest` (RCR-002). Per IDR-005 the
  Raft log IS this WAL IS the decision trace: I2 must not create a second log artifact.
- **ACS-001/ACS-002** (`standard/`) — content addressing (`0x12 0x20 || SHA-256(domain‖body)`)
  and canonical dCBOR bytes (`RUNTIME_FREEZE_v1.0.md`, "What v1.0 guarantees"). Replicated
  outcome digests must be ACS-001 addresses so cross-node equality is byte-testable.

### 1.4 Which invariants apply?

Registered (normative): **OWN-001, LAYER-001, SHARD-001, ORCH-001, ORCH-002, ORCH-003,
ORCH-004** — full mapping with executable proofs in §4. PROPOSED (informative, CCP-GATE
required before enforcement): G-001, QUERY-001, PERSIST-001 are *referenced* by the IDR
Batch 1 text and by crate headers; this design cites them only as (PROPOSED) context and
derives no conformance obligation from them (`ARVES_00_Invariant_Registry_v1.md` Parts
4–5).

### 1.5 Which ownership rules apply?

- The **Kernel remains the single owner of cognitive truth** (ORCH-001; OWN-001). The
  consensus layer "provides the replication mechanism the Kernel uses… it does not decide
  what is true" (`arves-consensus` header, "Ownership boundaries").
- **Per shard, the leader is the sole writer** (IDR-004/IDR-005): followers apply
  committed outcomes; they never accept client commits and never recompute engines
  (IDR-002). One owner per state is preserved because a follower's copy is derived
  state — a faithful mirror of the leader's log, not a second decision-maker.
- **Persistence owns durability only** (layer matrix, Amendments A-003: "Durable store of
  committed state/events … Cannot interpret meaning or decide").
- **No component gains a new ownership** — I2 adds replicas of existing state, not new
  state owners (Non-Negotiable Rule 8: never duplicate ownership).

### 1.6 Which IDRs apply?

All five, bindingly:

| IDR | I2 obligation |
|---|---|
| IDR-001 Consensus | CP system; Raft; **one independent Raft group per (tenant, workspace) shard**; no global leader; engines run anywhere, only COMMIT goes through the shard leader; read tiers Linearizable / Bounded-staleness / Eventual |
| IDR-002 Replication | Leader → Followers via the Raft log; replicate **committed OUTCOMES, never invocations**; followers apply, never recompute; periodic snapshots + append-only WAL |
| IDR-003 Membership | Raft **joint consensus** (C_old,new → C_new); no split-brain; **no cross-shard atomic commit** (sagas per Amendment-006) |
| IDR-004 Leader Election | Raft election, one leader per shard; on leader loss, in-flight uncommitted work is **discarded** (no partial truth, Amendments A-005/A-006) |
| IDR-005 Storage | Append-only WAL + snapshots; **Raft log = WAL = decision trace**; single ordered source for deterministic replay (ORCH-003) |

IDR-006 (products consume the frozen platform) applies indirectly: I2 work is Runtime
Team work under RCR, never product-side edits.

### 1.7 Does this create architectural drift?

**No — by construction, and this is checkable.** I2 implements exactly the distribution
the frozen corpus already specified: IDR Batch 1 closes with "implement per-shard Raft
groups, the commit path (engine-anywhere → leader commit → follower apply), WAL/snapshot
storage, and the read-tier query paths. Code, not specification." The I1 design (§5.4,
§6) explicitly reserved the I2 delta: "Growing to I2 changes configuration and populates
followers — not the architecture." (I1 §6, verbatim). The `arves-consensus` crate already declares every type and trait I2 will
implement. Zero new layers, zero new invariants, zero contract redesigns are proposed.
Drift risks that remain (transport coupling, second-log drift, follower write paths) are
enumerated in §3.21 Risks with detection tests.

### 1.8 Does this require CCP / Amendment / a new IDR?

- **No specification change** is required: the frozen corpus fully specifies I2's
  architecture (IDR-001..005 + ORCH-001..004 + SHARD-001).
- **RCRs are mandatory** for implementation, because `runtime/` is frozen (see §6 for the
  exact instrument list: consensus implementation, kernel wiring, batch-commit debt).
- **New IDRs are expected** for engineering knobs the spec deliberately left open (each
  listed as an Open Question in §3.22 and MUST be recorded as an IDR before code):
  snapshot cadence, election timing/heartbeat parameters, transport + wire format,
  read-index vs lease reads, shard→node placement. This follows I1's own precedent
  (`I1_Engineering_Design.md` §7.5: distribution decisions the spec did not pin are
  recorded as IDRs, never silent changes).
- **CCP-GATE**: every new I2 behaviour ships with a conformance scenario
  (`ARVES_Reference_Lifecycle_v1.md` Part 6: "No behaviour is ratified without a
  conformance scenario") — plan in §5.

### 1.9 Can another independent implementation reproduce this behaviour?

Yes, and that is the design's acceptance bar. Everything an independent implementor
needs is in frozen, public artifacts: the `ShardConsensus` contract (types + semantics),
IDR-001..005 (the algorithmic decisions: Raft, per-shard groups, outcome replication,
joint consensus), ORCH-003/004 (replay + idempotency semantics), SHARD-001 (partitioning),
and ACS-001/002 (byte-level identity, already reproduced by three independent-language
runtimes per `RUNTIME_FREEZE_v1.0.md`). Raft itself is a published, independently
implemented algorithm. The conformance plan (§5) defines behaviour by
invariants/properties, not by golden outputs (`ARVES_Scenario_Conformance_Framework_v1.md`
Part 8), so an independent runtime is judged by the same scenario set, not by matching
this implementation's internals. Items that would today block a bit-perfect independent
rebuild are declared as Open Questions (wire format, timing parameters), each routed to
an IDR so they become reproducible decisions rather than folklore.

### 1.10 Would this implementation still pass conformance five years from now?

Yes, under the framework's own versioning rule: conformance results are pinned as "N% at
Level Lx against Framework vA / Spec vB" (`ARVES_Scenario_Conformance_Framework_v1.md`
Part 11), and the spec is permanently frozen, so the target cannot move. The design
avoids the classic five-year rot vectors: no dependence on wall-clock behaviour in the
truth path (deterministic fold over the log), no golden-output assertions (Part 8
property semantics), content addressing pinned to ACS-001 (byte-stable forever per the
freeze record), and every timing/format knob externalized into IDRs where it is versioned
rather than implicit. The one honest caveat: transport-layer technology choices (RPC
library) will age; the design therefore requires the consensus core to be
transport-agnostic (§3.7) so a transport swap is a non-architectural change.

---

## 2. Specification / Gap summary (what exists → what I2 adds)

| Have (I1, frozen v1.0) | Need (I2) | Frozen source authorizing it |
|---|---|---|
| Single node commits durable truth; `RefKernel`/`FileKernel`, fsync WAL, CRC + hash-chain digest | N replicas per shard agreeing on one log | IDR-001/002; Baseline Part 5 (I2) |
| WAL already Raft-log-shaped (offset = index, term present; `arves-persistence`) | The WAL *used as* the Raft log — no second artifact | IDR-005 ("The Raft log IS the WAL IS the decision trace") |
| `replay_from(offset)` streams the committed trace | Same trace consumed as the replication feed to followers | IDR-002; ORCH-003 |
| Recovery = deterministic local replay (I1.7) | Follower catch-up = the same replay, remote-origin; snapshot install for far-behind followers | IDR-002 (snapshots); ORCH-003 |
| Consensus exercised at quorum=1 through a real API shape (I1 §5.4) | Real quorum: propose → replicate → majority ack → commit | IDR-001 (commit quorum per shard) |
| `ShardConsensus` contract, types only (CONTRACT-ONLY crate) | First implementation of the contract | IDR Batch 1, "Next — I1 Distributed Runtime" |
| Fixed single node (permanent leader) | Per-shard leader election; joint-consensus membership | IDR-004; IDR-003 |
| Read tiers tagged but collapsed (I1 §5.9) | Tiers become real: leader read-index / follower bounded-staleness / replica eventual | IDR-001 Read Consistency Tiers |

Staging note: the earlier `runtime/docs/I2.0_Engineering_Design.md` proposed a
property-first ladder (I2.1 replication semantics with a fixed leader → … → I2.7
election last). This package **adopts that ladder** as the implementation sequencing
(§3.5 Lifecycle) — it proves replication before consensus, consistent with ED-002
one-property discipline — while extending it to full milestone scope.

---

## 3. ENGINEERING DESIGN

*(All 22 constitutional headers, in order.)*

### 3.1 Responsibilities

- **Cluster Kernel (Kernel + consensus, per shard):** accept `ProposedWrite`s only at the
  shard leader; validate + content-address + dedupe (ORCH-004); propose the resulting
  **Outcome** to the shard's Raft group; ack the client only after quorum commit +
  fsync; apply committed entries in log order (ORCH-001/OWN-001; IDR-001/002).
- **Consensus substrate (`ShardConsensus` impl):** per-shard log replication, leader
  election (IDR-004), joint-consensus membership (IDR-003), commit-index advancement,
  read-index service (IDR-001 tiers). Owns *no* truth semantics — payloads are opaque
  (`arves-consensus` header).
- **Follower replicas:** durably append the leader's committed entries
  (offset/term/content preserved), apply them to local truth, reject client commits,
  serve bounded-staleness reads (IDR-002; IDR-001 tiers).
- **Persistence:** remain the single durable artifact (log = WAL = decision trace,
  IDR-005); provide snapshots as pure functions of a log prefix for follower catch-up
  and replay bounding (IDR-005; IDR-002).
- **Explicit non-responsibilities:** consensus never interprets an Outcome payload
  (ORCH-001); followers never run engines (IDR-002); nothing in I2 gives the Control
  Plane a write path (ORCH-002); no cross-shard atomic commit exists anywhere (IDR-003).

### 3.2 Inputs

| Input | Producer | Notes |
|---|---|---|
| `ProposedWrite` (content-addressed, 5 ontology aspects, immutable `ShardKey`) | Information Platform / Execution outcomes via Kernel gateway | unchanged from I1 (`arves-kernel`) |
| `Outcome { digest, payload }` | Kernel (post-validation, pre-replication) | IDR-002: already-decided outcome, never an invocation |
| Raft RPC traffic (AppendEntries/Vote/Snapshot-equivalents) | peer replicas | wire format = Open Question OQ-3 → IDR |
| `Membership` change requests | operator / placement layer | joint consensus only (IDR-003) |
| Read requests + `ReadTier` | Query layer | IDR-001 tiers |
| Snapshots (log-prefix state blobs) | leader → far-behind follower | IDR-002/005 |

### 3.3 Outputs

| Output | Consumer | Notes |
|---|---|---|
| `Committed<Outcome>` + `LogIndex` (post-quorum, post-fsync) | Kernel caller (TruthRef), LCW projection, Query | linearization point per shard |
| Replicated, byte-identical per-shard WAL on every replica | recovery, replay, audit | IDR-005; equality provable via `truth_hash` / RCR-002 `integrity_digest` |
| `Leadership` / `Role` observations | routing, operators | IDR-004 |
| `read_index(tier)` guarantees | Query | IDR-001 |
| Typed rejections: `NotLeader{leader}`, `QuorumUnavailable`, `ElectionInProgress`, `MembershipRejected`, `UnknownShard` | callers | CP posture: unavailable > divergent |
| Conformance artifacts per scenario run | conformance suite | Framework Part 9 |

### 3.4 Dependencies

Downward-only (LAYER-001): consensus impl depends on `arves-persistence` (log storage)
and is depended on by the Kernel's commit path; nothing in consensus calls upward into
Kernel/Control-Plane semantics. New *external* dependencies (transport, timers) are
confined to a transport adapter behind the `ShardConsensus` contract so the core remains
deterministic and testable in-process (mirrors I1.5's isolation of durability from
distribution, `runtime/docs/I2.0_Engineering_Design.md` §D R2, substance in §9). No product code may
depend on consensus internals (IDR-006).

### 3.5 Lifecycle

Milestone staging (adopted from the ratification-pending I2.0 ladder; each step = design
note + behaviour proof + adversarial hunt + scenario, per ED-006 and CCP-GATE):

| Step | Deliverable | Property proven |
|---|---|---|
| I2.1 | Replication model (fixed leader, in-process transport) | follower applying committed outcomes reconstructs byte-identical truth |
| I2.2 | Replication behaviour | append → ack → apply flow |
| I2.3 | Consistency | same log ⇒ same replay hash across nodes |
| I2.4 | Leader semantics | leader-only commit; follower rejects client commits |
| I2.5 | Raft log formalization | term / index / commitIndex over the existing WAL |
| I2.6 | Quorum | majority acknowledgement before commit |
| I2.7 | Leader election + failover | IDR-004; uncommitted in-flight work discarded |
| I2.8 | Joint-consensus membership | IDR-003; add/remove nodes without split-brain |
| I2.9 | Read tiers | IDR-001 linearizable / bounded / eventual made real |

Runtime lifecycle of a shard group: `create(ShardId) → elect leader → serve commits →
snapshot periodically → membership changes via Joint → decommission (migration, never
key mutation — SHARD-001)`.

### 3.6 State Model

Per replica, per shard, three strictly-owned state kinds (unchanged taxonomy from I1
§5.3, extended with consensus metadata):

1. **Truth** (Kernel, durable, CP): `snapshot(base) + log[base..commitIndex]` —
   append-only, content-addressed, identical bytes on every replica at equal
   commitIndex (IDR-002/005).
2. **Consensus metadata** (consensus substrate, durable where Raft requires):
   `currentTerm`, `votedFor`, `commitIndex`, `Membership` phase. This is *mechanism*
   state, not cognitive truth — it decides ordering, never meaning (ORCH-001 boundary,
   `arves-consensus` header).
3. **Derived state** (LCW projection; follower truth mirrors): droppable, rebuilt by
   replay (ORCH-003).

Transition: `ProposedWrite → validate → Outcome → propose(leader) → append(term,index) →
replicate → quorum → commitIndex advance (linearization point) → fsync-durable →
apply in index order → project`. The shard key inside every record is immutable for the
entity lifetime (SHARD-001, Amendments A-004).

### 3.7 Distributed Behaviour

- **Topology:** one Raft group per `(tenant, workspace)` shard (IDR-001); a physical node
  hosts many groups (`arves-consensus::NodeId` doc); shards share nothing; no global
  consensus, no global leader.
- **Commit path:** engine/capability work runs anywhere and completes *before* proposal;
  only the committed outcome is replicated (IDR-001 Engineering Refinements; IDR-002).
  Followers apply, never recompute (ORCH-003) — this is what makes nondeterministic
  engines (LLMs) safe under replication.
- **Partitions:** CP posture (IDR-001): the minority side refuses writes
  (`QuorumUnavailable`) rather than diverging; bounded-staleness/eventual reads may
  continue on the minority with honest tier labeling. Observability remains AP and is
  never a truth source (IDR-001 Non-goals; CP/AP boundary table).
- **Cross-shard:** no atomic multi-shard commit exists; cross-shard workflows are
  sagas/compensations recorded in the decision trace (IDR-001; Amendments A-006).
- **Transport-agnostic core:** the Raft state machine is a deterministic function of
  (messages, timers-as-events); the network adapter is a shell around it. I2.1..I2.6
  run entirely in-process; sockets arrive only at the step where they are the property
  under test.

### 3.8 Concurrency

- **Single-writer per shard:** the leader serializes commits through the log — exactly
  one linearization point (commitIndex advance), eliminating truth-level write races by
  construction (I1 §5.5, unchanged; now enforced across nodes by `NotLeader` rejection).
- **Concurrent proposals** queue at the leader, ordered by the log; equal-content
  duplicates dedupe at commit (ORCH-004), so at-least-once client retry is safe.
- **Apply loop:** single-threaded per shard, index order — deterministic projection.
  Many shards apply in parallel (shared-nothing, SHARD-001).
- **Election concurrency:** term numbers totally order leadership; a stale leader's
  proposals are rejected by term (Raft safety; `arves-consensus::Term` doc). A stale
  leader can never commit (IDR-004).
- **Membership concurrency:** overlapping joint transitions are rejected
  (`MembershipRejected`) — one reconfiguration in flight per shard (IDR-003).

### 3.9 Failure Modes

| Failure | Behaviour | Source |
|---|---|---|
| Leader crash | Election (IDR-004); in-flight uncommitted work **discarded** — no partial truth | IDR-004; Amendments A-005/A-006 |
| Follower crash | Recover from local WAL; catch up from leader log/snapshot | IDR-002/005 |
| Network partition, minority side | Writes unavailable (`QuorumUnavailable`); labeled stale reads only | IDR-001 (CP) |
| Split-brain risk during membership change | Prevented by joint consensus (quorums of both configs required) | IDR-003 |
| Duplicate delivery / client retry | Idempotent no-op via content address | ORCH-004 |
| Gap / out-of-order record at a follower | Rejected LOUDLY (`Gap`), never silently applied | I1.7 discipline; I2.0 §9 |
| Stale leader proposes | Term check rejects; redirect via `NotLeader{leader}` | IDR-004; consensus contract |
| WAL corruption on a replica | CRC + hash-chain digest detect (RCR-002); truncate to last valid; re-sync from peers | IDR-005; `RUNTIME_FREEZE_v1.0.md` #8 |
| Follower behind leader's snapshot horizon | Snapshot install, then log tail | IDR-002/005 |
| Cross-shard partial state mid-workflow | Expected; compensated by saga actions recorded in the trace | Amendments A-006 |
| Client commit sent to a follower | Rejected — replicas are not writers | OWN-001; I2.0 §D R3 |

### 3.10 Recovery

Recovery is replay from durable state, never recomputation (ORCH-003) — the I1
procedure, now per replica: load latest snapshot (pure function of a log prefix), replay
`(base, commitIndex]`, discard any uncommitted log tail (IDR-004 — no partial truth),
resume consensus at `commitIndex`. A rejoining replica is just a recovering replica whose
remaining trace arrives from peers instead of local disk — deliberately the *same code
path shape* as local recovery, so recovery correctness proven in I1 carries over.
Deterministic and idempotent: replaying the same log N times yields identical state
(I1 §5.7).

### 3.11 Replay

- The replicated log is the decision trace (IDR-005); replay re-applies committed
  outcomes with engines physically disabled and asserts equality — the I1 proof,
  extended cross-node: **for every shard, every replica's replay of the same log
  produces the same `truth_hash` (u64) plus the same RCR-002 SHA-256 `integrity_digest`
  hash-chain**. Honest evidentiary strength: `truth_hash()` is a u64 introspection hash
  (`arves-kernel/src/lib.rs`), so its equality is strong evidence, not a byte-equality
  proof of the projected truth state; the SHA-256 chain attests **WAL bytes**, not the
  projected truth. A genuine byte-level truth-state comparison in the cross-node replay
  test is an explicit I2 proof obligation (OQ-11 — e.g. upgrading `truth_hash` to an
  ACS-001 digest under the OQ-2 IDR).
- Nondeterministic engines are never re-run during replay or follower apply
  (ORCH-003; Vol 9 Part 9: "Recomputation is explicitly NOT guaranteed").
- Replay across snapshots: `replay(snapshot(b) + log(b..i]) == replay(log(0..i])` —
  snapshot transparency is an executable property (IDR-005).

### 3.12 Consistency

- **Within a shard:** linearizable writes (single log, single commit point) — IDR-001
  "Writes are linearizable; replay is deterministic from the committed log."
- **Read tiers (IDR-001 table):** Linearizable via leader read-index;
  Bounded-staleness via follower reads with a staleness bound; Eventual via
  read/geo replicas. I1 collapsed the tiers behind an honest tag; I2 makes each path
  real and each guarantee testable.
- **Cross-shard:** per-shard linearizability only; saga-level eventual consistency
  (IDR-001 "No cross-shard atomic commit in v1").
- **Observability:** AP, eventually consistent, never authoritative (IDR-001 Non-goals).

### 3.13 Availability

- Writes: available iff the shard has a leader + quorum (CP: consistency over
  availability, IDR-001). Election windows are write-unavailability windows by design.
- Reads: linearizable reads share the write availability profile; bounded/eventual
  tiers remain available on any live replica with honest staleness labels.
- Blast-radius: shards fail independently (SHARD-001) — one tenant's quorum loss cannot
  affect another tenant's shard (tenant isolation — IDR-001 Engineering Refinements
  (Vol 2) + Amendments A-004).

### 3.14 Scalability

- **Compute scales separately from consensus** (IDR-001 Engineering Refinements):
  engines run anywhere; adding engine capacity adds zero consensus load.
- **Horizontal truth scale = more shards** (shared-nothing per-shard groups); the
  IDR-001 rationale targets ~10,000 nodes via many independent groups, explicitly
  rejecting a global single leader.
- **Multi-Raft density** (many groups per node) requires heartbeat/tick coalescing to
  avoid O(groups) network chatter — an engineering knob, recorded as OQ-5 → IDR, not an
  architecture change.
- Query/LCW partitioning is **I3 scope** (Baseline Part 5) — deliberately out of I2.

### 3.15 Performance

- Commit latency: one consensus round-trip to quorum + fsync (I1 §5.12 anticipated
  this). Proposal batching per log append amortizes fsync and network cost — note the
  adjacency to the frozen v1.1 debt item **Kernel batch-commit**
  (`RUNTIME_FREEZE_v1.0.md` backlog #3): I2's log batching is a transport/durability
  optimization and must NOT be conflated with atomic multi-effect commit semantics,
  which remain deferred debt with their own RCR.
- Correctness before optimization (Non-Negotiable Rule 7): I1's baseline benchmarks
  (commit/s, replay time) are the regression floor; I2 adds distributed baselines
  (commit latency vs replica count, failover time, catch-up throughput) as guards, not
  targets.

### 3.16 Security

- **Tenant isolation is the primary boundary** (SHARD-001 + TenantScope aspect): a Raft
  group carries exactly one tenant/workspace's truth; no cross-tenant replication
  stream exists by construction (Amendments A-004: "no cross-tenant data in a single
  shard").
- **Commit gateway as choke point** unchanged (ORCH-001); replication does not add a
  second door — followers accept entries only from the current-term leader.
- **New surface, honestly stated:** inter-node transport creates the first network
  attack surface in the truth path. v1.0's threat model is a trusted single host
  (`RUNTIME_FREEZE_v1.0.md` #8); a multi-node cluster **extends the trust boundary** and
  requires at minimum mutually-authenticated transport (mTLS or equivalent) between
  replicas. Authenticated commit (principal on `Kernel::commit`) and signed truth
  stores remain the recorded v2.0 debt (#8 STILL OPEN) — I2 must not silently claim to
  close them. Threat-model delta = Open Question OQ-7 → IDR + security review.
- RCR-002's hash-chain `integrity_digest` gains new value: replicas can compare chain
  digests to detect a tampered or forked replica.

### 3.17 Observability

- Truth vs observability separation holds (CP/AP boundary, IDR-001): dashboards are
  hypotheses; the replicated WAL is the confirmation.
- New signals per shard: role, term, commitIndex per replica, replication lag
  (leader commitIndex − follower applied index), election count/duration, quorum
  availability, snapshot install count, membership phase, `NotLeader` redirect rate,
  dedupe hits (ORCH-004 evidence).
- All observability channels are AP/eventual and may use CRDTs (IDR-001 Non-goals);
  none may be read to answer a truth question.

### 3.18 Metrics

Milestone-level KPIs (regression-guarded, per the constitution's test mandate):

| Metric | Meaning | Gate |
|---|---|---|
| Cross-replica truth equality | `truth_hash` equality (u64) plus SHA-256 WAL hash-chain (`integrity_digest`) equality at equal commitIndex, all shards; byte-level truth-state comparison per OQ-11 | MUST be 100% in every test run |
| Commit latency p50/p99 vs replica count | consensus round-trip + fsync cost | recorded baseline; no silent regression |
| Failover time (leader kill → new leader serving) | IDR-004 election liveness | bounded per the timing IDR (OQ-4) |
| Follower catch-up throughput (log + snapshot paths) | IDR-002 | recorded baseline |
| Uncommitted-work leakage after failover | entries acked-but-lost or unacked-but-committed-visible-early | MUST be zero (no partial truth) |
| Replay determinism time for N entries | ORCH-003 at scale | vs I1 baseline |
| Invariant coverage | §4 proofs executable and green | 100% of applicable registered invariants |

### 3.19 Auditability

The append-only replicated log IS the audit log (IDR-005): every truth mutation is an
ordered, immutable, content-addressed record with provenance (ontology mandatory
aspects), now attested identically on N replicas — replication *strengthens* audit
because a single-host tamper is detectable by digest divergence across replicas
(RCR-002 chain). Membership changes and leadership terms are themselves log entries
(`EntryKind::Membership`; `Term` on every entry), so the cluster's own reconfiguration
history is replayable evidence. Every conformance run emits the machine-readable
artifact of Framework Part 9.

### 3.20 Trade-offs

| Decision | Chosen | Rejected | Why (spec-downstream) |
|---|---|---|---|
| Truth model | CP Raft per shard | AP/CRDT truth; global leader | IDR-001 (verbatim: rejected alternatives) |
| Replication unit | Committed `WalRecord`/`Outcome` | invocations / client requests | IDR-002; engines nondeterministic (ORCH-003) |
| Log artifact | Reuse the I1 WAL as the Raft log | separate Raft log + WAL | IDR-005 "log IS the WAL IS the decision trace"; two artifacts = drift risk |
| Sequencing | Replication first, election last (I2.1→I2.7) | full Raft big-bang | ED-002 one-property; isolates failure causes |
| Transport in early steps | In-process pull | sockets from day one | couples replication correctness to networking prematurely (I2.0 §D R2, substance in §9) |
| Consensus home | `arves-consensus` impl beneath Kernel | bake Raft into `arves-kernel` | keeps truth owner decoupled from transport (I2.0 §D R4; LAYER-001) |
| Cross-shard writes | none (sagas) | 2PC/distributed transactions | IDR-001/003; Amendments A-006 |
| Availability under partition | refuse minority writes | serve-and-reconcile | CP posture (IDR-001); truth must never be "eventually one" |

### 3.21 Risks

| Risk | Impact | Mitigation / detection |
|---|---|---|
| Second log artifact creeps in (Raft log ≠ WAL) | violates IDR-005; replay forks | architecture test: exactly one append-only artifact per shard; digest equality across the "two views" |
| Follower gains a write path | violates OWN-001/IDR-002 | replica-mode kernel rejects commits; property test fires commits at followers |
| Transport concerns leak into the consensus core | untestable nondeterminism | deterministic state-machine core; simulation tests with scripted message schedules |
| Election livelock / disruptive rejoin | availability collapse | standard Raft mitigations (randomized timeouts, pre-vote) — parameters via timing IDR (OQ-4); chaos tests |
| Acked-but-lost commit (fsync/quorum ordering bug) | truth loss — worst-case severity | failure-injection: kill leader at every commit-path step; assert zero leakage (§3.18) |
| Multi-Raft resource exhaustion at high shard counts | scalability wall | coalesced ticks (OQ-5); load tests at representative group counts |
| Snapshot/catch-up divergence (follower installs snapshot ≠ log-replay state) | silent truth fork | snapshot-transparency property (§3.11) executed cross-node |
| Trust-boundary expansion understated | security posture drift | OQ-7 threat-model IDR + security review gate before any multi-host deployment claim |
| Treating PROPOSED invariants as registered | certification confusion | §4 markers; coverage counts registered-only (I1 precedent, §7.2) |
| Editing frozen surfaces without RCR | freeze violation (266-file gate) | §6 instrument list is a precondition; no code in prep mode |

### 3.22 Open Questions

Honest unknowns — none resolved by assumption; each routed to its instrument. **All are
G2-blocking for their respective step**, none blocks this design package.

| # | Question | Instrument |
|---|---|---|
| OQ-1 | Snapshot cadence/threshold policy (bounds replay + follower catch-up) — carried over from I1 OQ-2 | IDR before I2 code |
| OQ-2 | Unify `arves_consensus::ContentHash(String)` with the ACS-001 `ContentId` byte form? The frozen contract says the concrete hash is deferred; ACS-001 is the natural answer but binding it is a decision | IDR (+ RCR if the frozen crate type must change — likely additive alias) |
| OQ-3 | Wire format + RPC framing for Raft traffic (dCBOR/ACS-002-canonical is the natural candidate for replicated payload bytes) | IDR |
| OQ-4 | Election timeout / heartbeat / pre-vote parameters and their determinism story in tests | IDR |
| OQ-5 | Multi-Raft tick coalescing and per-node group density limits | IDR |
| OQ-6 | Read-index vs lease reads for the linearizable tier — carried over from I1 OQ-4 | IDR (decide in I2.9) |
| OQ-7 | Threat-model delta for multi-node: mTLS mandatory? where does replica identity live? relation to the open v2.0 authenticated-commit debt | IDR + security review; possibly v2.0 RCR |
| OQ-8 | Shard→node placement and rebalancing ownership (who creates/moves groups?) — the spec pins the shard *key* (SHARD-001) but not placement; full scheduling is I4 scope | IDR (minimal static placement for I2); revisit at I4 |
| OQ-9 | Bounded-staleness: how is the bound expressed/enforced (index lag vs time)? | IDR (I2.9) |
| OQ-10 | Does the follower's replica-mode Kernel need a distinct trait, or configuration of `RefKernel`? (I2.0 §9 assumed configuration; confirm under RCR review) | RCR design review |
| OQ-11 | Genuine byte-level truth-state comparison for cross-node replay: `truth_hash()` is a u64 introspection hash and the RCR-002 `integrity_digest` attests WAL bytes, not projected truth state. Upgrade `truth_hash` to an ACS-001 digest of the committed truth set (natural companion to OQ-2)? | IDR (with OQ-2); RCR if the frozen kernel helper changes |

---

## 4. Invariant Mapping (registered → executable proof)

Registered invariants only carry conformance weight (`ARVES_00_Invariant_Registry_v1.md`
Parts 2, 5). Every proof below is an executable test I2 must ship (constitution:
"No invariant may remain proof-only once its owning component is implemented").

| Invariant (registered) | I2 obligation | Executable proof required |
|---|---|---|
| **OWN-001** (Amendments A-001 + registry) | One owner per state under replication: leader is sole writer per shard; followers are derived replicas; consensus metadata owned solely by the consensus substrate | Property test: client commits against every non-leader replica are rejected; architecture test: exactly one mutator per state category incl. consensus metadata (extends I1 OWN-001 gate) |
| **LAYER-001** (Amendments A-003) | Consensus sits beneath the Kernel commit path; no upward call from consensus into Kernel/Control-Plane semantics; transport confined to the adapter | Crate dependency-graph gate extended to the consensus impl (no upward/lateral truth edge) — extends the existing executable architecture gate |
| **SHARD-001** (Amendments A-004) | One Raft group per immutable `(tenant, workspace)` key; no cross-tenant bytes in any group's log; no cross-shard atomic commit path exists | Property test: two-tenant isolation across *replicated* shards (extends RCR-007's single-node `behaviour_8_two_tenant_isolation`); negative test: no API exists to commit atomically to two shards |
| **ORCH-001** (Vol 9 v2 Part 5) | Consensus owns no truth (opaque payloads); Control Plane still owns none; only the Kernel-through-leader path creates truth | Test: no state delta on any replica without a corresponding committed log entry; consensus code provably never deserializes payload meaning (API-level: no accessor exists) |
| **ORCH-002** (Vol 9 v2 Part 5) | No Control-Plane bytes enter the replicated log; CP remains restartable/replicable (Vol 9 Part 11) | Test: WAL entry taxonomy contains only Outcome/Membership kinds (already fixed by `EntryKind`); restart-CP-mid-replication test |
| **ORCH-003** (Vol 9 v2 Part 5) | Follower apply and recovery replay from the recorded trace; never recompute; replay deterministic across replicas | Cross-node replay test: engines disabled, rebuild every replica from log, assert `truth_hash` equality (u64) plus `integrity_digest` chain equality per shard (byte-level truth-state comparison per OQ-11); snapshot-transparency property |
| **ORCH-004** (Vol 9 v2 Part 5) | Idempotent, content-addressable commit survives distribution: duplicate delivery, client retry after `NotLeader`, and replay all converge to one committed outcome | Property test: at-least-once delivery storms (dup/reorder within contiguity rules) yield zero extra committed outcomes on any replica |

**PROPOSED invariants touched by I2 sources — referenced only, NEVER enforced:**

| ID | Where it appears near I2 | Standing |
|---|---|---|
| G-001 | cited by IDR-001 ("The Kernel is the single owner … G-001, OWN-001") and the kernel crate header | (PROPOSED — CCP-GATE required; registry Part 4). I2 derives all needed force from ORCH-001+OWN-001 |
| QUERY-001 | cited by IDR-001 ("QUERY-001 (read-only)") for the read tiers | (PROPOSED — CCP-GATE required). Read-tier tests assert IDR-001 behaviour, not QUERY-001 as an invariant |
| PERSIST-001 | persistence crate header context | (PROPOSED — CCP-GATE required) |
| CAP-*/ENG-*/LCW-001 | not in I2 scope (fabrics/LCW unchanged) | (PROPOSED — CCP-GATE required); untouched |

If the maintainer wishes any PROPOSED invariant (G-001 is the natural candidate for a
cluster kernel) to become enforceable during I2, it must first pass the CCP-GATE with a
conformance scenario (`ARVES_Reference_Lifecycle_v1.md` Part 6) — that is a CCP, not an
I2 deliverable.

---

## 5. Conformance Plan

### 5.1 Level and axes (Scenario Conformance Framework)

- **Target level: L3 Distributed** — "Conformance preserved across distributed
  deployment" (Framework Part 10). Honest scoping: v1.0's live conformance reaches L1
  (Information → Kernel → Query, RCR-008..010) while Control-Plane/Engine crates are
  contract-only; therefore I2's claim is precisely **"L1 node-set conformance preserved
  under distributed deployment"** — the L3 property applied to the implemented node
  set. Full L2 (ORCH invariants proven at the live Control Plane) remains a separate,
  explicitly-tracked gap; I2 must not claim blanket "L3" without this qualifier.
- **Axes instantiated (Part 5):**
  - **Axis 12 — Recovery & Replay** (primary): deterministic replay from the decision
    trace, now cross-node; leader failover recovery.
  - **Axis 8 — High-volume Streaming / tenant isolation at scale** (secondary): two
    tenants on independent replicated shards; isolation held under replication and
    failover.
  - **Axis 2 — Event-driven** (supporting): event → Kernel state transition through the
    leader commit path (reuses the L1 pipeline under distribution).
- **Reference-scenario reuse (Part 6):** the "Enterprise Knowledge Query" assertions
  (tenant isolation held; control plane owns no truth) and the "Incident Response"
  replay assertion re-run against a 3-replica shard instead of a single node.

### 5.2 Scenario set (new, per CCP-GATE — no scenario ⇒ not done)

| Scenario | Asserts (Part 8 semantics: invariants + properties, no golden output) |
|---|---|
| S-I2-1 Replicated commit | commit at leader → quorum → all replicas converge to equal `truth_hash`/`integrity_digest`; ORCH-001/004 held |
| S-I2-2 Follower apply purity | follower reconstructs truth with engines disabled (ORCH-003/IDR-002); zero recomputation |
| S-I2-3 Leader failover | kill leader mid-load; new leader elected; no acked commit lost; no unacked partial truth (IDR-004; A-005/006) |
| S-I2-4 Partition (CP) | minority refuses writes; majority proceeds; heal → convergence; no divergent truth ever observable (IDR-001) |
| S-I2-5 Membership change | joint-consensus add/remove under load; no split-brain window (IDR-003) |
| S-I2-6 Tenant isolation, distributed | two tenants, two groups, interleaved failovers; zero cross-tenant leakage (SHARD-001) |
| S-I2-7 Read tiers | linearizable read reflects the latest commit via leader read-index; bounded/eventual labeled honestly (IDR-001) |
| S-I2-8 Snapshot catch-up | far-behind follower: snapshot install + log tail == pure log replay (IDR-002/005) |

Each run emits the Part 9 conformance artifact; results are pinned "at Level L3(scoped)
against Framework v1.0 / Spec v1.0" (Part 11).

### 5.3 Per-milestone Success Criteria — what they concretely mean for I2

| Constitutional criterion | Concrete I2 meaning |
|---|---|
| Architecture PASS | dependency-graph gate green with the consensus impl included; no new layer; log = WAL single-artifact check; independent review finds no drift from IDR-001..005 |
| Conformance PASS | S-I2-1..8 all PASS under Part 8 verdict rules (any invariant failure ⇒ FAIL) |
| Certification PASS | the certification harness (maintainer-independent, per FOUNDATION posture) grades the clustered runtime from `standard/` + scenario artifacts alone |
| Independent Review PASS | review-as-if-submitted-by-another-company across all 14 constitution dimensions, with Distributed Behaviour/Replay/Concurrency now first-class |
| Invariant coverage 100% | every §4 registered-invariant proof executable and green; PROPOSED explicitly excluded from the denominator (I1 precedent) |
| Replay PASS | cross-node deterministic replay: every replica, every shard, truth-hash equality (u64) plus SHA-256 WAL hash-chain equality (byte-level truth-state comparison per OQ-11); snapshot-transparency held |
| Distributed tests PASS | failure-injection suite: leader kill at every commit-path step, partitions, dup/reorder storms, membership churn, crash-recover-rejoin — zero truth loss, zero divergence, zero partial truth |
| No architecture / specification drift | zero frozen-file diffs (266-file freeze gate green); all engineering knobs recorded as IDRs; all runtime changes carried by ratified RCRs |

Mandatory test classes (constitution): Unit · Integration · Architecture · Invariant ·
**Distributed** · **Replay** · Property · Stress · **Failure-Injection** · Recovery ·
Conformance · Certification — the three bolded classes are I2's center of gravity.

---

## 6. NON-GOALS and Required Instruments

### 6.1 Explicit NON-GOALS (deferred, with their owning scope)

| Non-goal | Why out of I2 | Where it lives |
|---|---|---|
| Distributed Query / LCW partitioning | separate milestone | I3 (Baseline Part 5) |
| Cluster-wide capability scheduling; shard placement/rebalancing beyond static assignment | separate milestone | I4 (Baseline Part 5); OQ-8 minimal IDR |
| Multi-agent coordination | separate milestone | I5 (Baseline Part 5) |
| Cross-shard atomic commit / distributed transactions | forbidden, not deferred | IDR-001/003 (sagas per A-006) |
| AP/CRDT treatment of truth | forbidden | IDR-001 |
| Kernel **batch-commit** (atomic multi-effect) | recorded v1.1 debt with its own RCR; log batching in I2 is not it | `RUNTIME_FREEZE_v1.0.md` backlog #3 |
| Bridge request-id correlation; engine-enforced determinism | recorded v1.1 debt, orthogonal to I2 | `RUNTIME_FREEZE_v1.0.md` backlog #1/#2 |
| Signed truth store / authenticated commit (principal on `Kernel::commit`) | v2.0 debt; I2 only *extends the transport* trust analysis (OQ-7) | `RUNTIME_FREEZE_v1.0.md` #8 |
| Geo-replication topology, multi-region placement | no frozen decision exists; would be invented architecture | future IDR if ever needed |
| Federated Kernel / cross-runtime federation / cloud runtime | consciously deferred to v2 of the standard | Baseline Part 3 |
| Ratifying any PROPOSED invariant | CCP business, not milestone business | Registry Part 5; Lifecycle Part 6 |
| Product features of any kind | I2 proves Replication; ships no product value | ED-002; IDR-006 |

### 6.2 Instruments required before any I2 code exists

`runtime/` is frozen; **every** implementation step below G2 requires ratified paperwork
first (`RUNTIME_FREEZE_v1.0.md` RCR process; Change Management table in the
constitution):

| Planned change | Instrument | Nature |
|---|---|---|
| First implementation bodies for `ShardConsensus` (in or beside `arves-consensus`) | **RCR** (v1.1 additive if purely new code behind the frozen contract; v2.0 if any frozen signature must break) | runtime change |
| Kernel commit-path wiring through consensus (replica-mode configuration, OQ-10) | **RCR** | runtime change |
| Replication surface (`ReplicationSource`/`ReplicaApply` sketch from I2.0 §4) | **RCR** + RT-001 interface-evolution review | runtime change (activates reserved IDR-002 semantics; no frozen-spec change) |
| Snapshot cadence, wire format, election timing, tick coalescing, placement, staleness bound, read-index-vs-lease, ContentHash↔ACS-001 binding (OQ-1..6, 8, 9) | **IDR** each (new IDR batch: "Kernel Distribution — Batch 2") | engineering decisions; spec untouched |
| Multi-node threat-model delta (OQ-7) | **IDR** + security review; signature work escalates to **v2.0 RCR** | engineering decision / major |
| Any wish to promote G-001 (or another PROPOSED invariant) to registered | **CCP Amendment** with a conformance scenario (CCP-GATE) | spec-side instrument, optional |
| Any discovered ambiguity in IDR-001..005 during implementation | **STOP** → Architecture Review / CCP per the Change Management table — never implement around it | constitutional |
| Frozen `.docx`/mirror corrections, if any inconsistency is found | CCP / regeneration only — never a hand edit | constitutional |

**Gate order:** (1) maintainer ratifies this design package → (2) IDR Batch 2 recorded →
(3) RCRs filed and ratified → (4) **G2 build gate opens** → (5) I2.1 begins under the
§3.5 ladder. Until step 4, this remains paper.

---

*End of I2 Cluster Kernel design package. This document designs against the frozen
UCS/UCI v1.0 specification and changes nothing in it. Every load-bearing claim above
cites its frozen source; every unknown is an Open Question routed to CCP/IDR/RCR —
never an assumption.*

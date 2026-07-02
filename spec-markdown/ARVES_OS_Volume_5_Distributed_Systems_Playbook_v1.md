> **Rendered from `ARVES_OS_Volume_5_Distributed_Systems_Playbook_v1.docx`** — a faithful Markdown mirror of the FROZEN specification corpus (authoritative source of record: the `.docx`). Format conversion only; content unchanged. **Do not hand-edit** — regenerate via `python tools/docx_to_markdown.py`. Spec changes go through CCP/Amendment.

# ARVES Engineering Operating Manual Volume 5: Distributed Systems Playbook v1.0

**STATUS: ENGINEERING OPERATING MANUAL (IMPLEMENTATION-ERA PLAYBOOK; GROUNDED IN IDR-001..005; NON-NORMATIVE WHERE MARKED)**

Specification Era is FROZEN as of 2026-07-01. This volume is Implementation-Era guidance. It does not change the specification. Implementation proves the spec and never alters it. Dependency chain (never reversed): Theory -> Specification -> Contracts -> Behaviour -> Conformance -> Implementation.

Normative authority in this volume comes ONLY from REGISTERED invariants (ORCH-001..004, OWN-001, LAYER-001, SHARD-001). All IDR-001..005 material is a reference implementation decision (non-normative). Any invariant marked "proposed (pending CCP)" is INFORMATIVE only and MUST NOT be treated as ratified.

# Part 1 - Purpose, Scope, and Reading Order

This playbook tells ARVES engineers how to build, operate, and scale the distributed runtime that carries ARVES truth across a cluster of up to ~10,000 nodes, without ever contradicting the frozen Universal Cognitive Standard (UCS). UCI is the reference implementation of UCS.

Scope is deliberately narrow: distributed engineering grounded strictly in the reference implementation decisions IDR-001 through IDR-005, plus the failure model (Amendment-006) and cancellation/preemption model (Amendment-005). Everything here serves one purpose: keep TRUTH correct and single-owned while the system is physically distributed.

## 1.1 What this volume covers

- Per-shard Raft: one Raft group per tenant/workspace shard (IDR-001, SHARD-001).

- The commit path: engine-anywhere -> shard leader commit -> follower apply of committed outcomes (never recompute).

- Replication: leader/followers/snapshot/WAL (IDR-002, IDR-005).

- Membership changes via Raft joint consensus (IDR-003).

- Per-shard leader election (IDR-004).

- Storage: append-only WAL as the decision trace (IDR-005, ORCH-003).

- The CP/AP boundary: truth is CP, observability is AP.

- Read-consistency tiers: linearizable, bounded-staleness, eventual.

- Failure taxonomy and saga compensation (Amendment-006).

- Cancellation, priority, and preemption (Amendment-005).

- Distribution-readiness rules (ORCH-002, ORCH-004, SHARD-001) and scaling to ~10,000 nodes.

## 1.2 What this volume must NOT do

- It must not invent new layers, invariants, milestones, or architecture.

- It must not present proposed invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-*, ENG-*) as normative.

- It must not introduce a cross-shard atomic commit; v1 has none, sagas are used instead.

- It must not let the Control Plane own truth or persistent state (ORCH-001, ORCH-002).

## 1.3 Reading order

| Reader | Start at | Then |
| --- | --- | --- |
| Runtime engineer | Parts 2-6 | Parts 9-12 |
| SRE / operator | Parts 7-8, 13 | Parts 14-16 |
| Reviewer / architect | Parts 2-3, 17 | Part 18 checklists |
| Conformance engineer | Parts 16-18 | Milestones map Part 15 |

# Part 2 - Architectural Ground Truth for Distribution

Distribution changes physics, not ownership. The layer model (LAYER-001, downward-only dependencies) and the Layer Responsibility Matrix are unchanged when the runtime is spread across many nodes.

## 2.1 Two planes

Control Plane decides; Data Plane carries. The Kernel never becomes the Control Plane. In the reference implementation the Kernel is the component that runs the Raft consensus for commits (IDR-001), but consensus is a mechanism for owning TRUTH, not for planning or orchestration.

| Plane | Owns | Consistency class | Distribution mechanism |
| --- | --- | --- | --- |
| Control Plane | Plans / Engine Graph (never truth, never persistent state) | Not truth | Stateless, replan-safe |
| Kernel (Data/Truth) | TRUTH (commits) | CP (per-shard Raft) | Leader commit + follower apply |
| Observability | Metrics, logs, tracing, presence | AP | Gossip / eventual |

## 2.2 Layer Responsibility Matrix under distribution

| Layer | Owns | Reads | Writes | Cannot |
| --- | --- | --- | --- | --- |
| Kernel | TRUTH (commits) | Committed state, WAL | WAL entries (committed outcomes) | Orchestrate / plan / execute |
| LCW | Working Memory / live state | Kernel truth | Working memory (not truth) | Own truth |
| Query | Nothing | Committed + working state | Nothing (READ-ONLY) | Mutate any state |
| Engine | Nothing persistent | Inputs (content-addressed) | Proposed effects only | Commit; hold durable state |
| Capability Fabric | Registry + bindings | Registry | Bindings | Own truth |
| Control Plane | Plan / Engine Graph | Everything to plan | Plans (non-persistent) | Own truth or persistent state |

Registered invariants enforced here: ORCH-001 (only Kernel owns truth), OWN-001 (every state has exactly one owner), LAYER-001 (downward-only deps).

## 2.3 The one-owner rule survives sharding

OWN-001 requires every state to have exactly one owner. SHARD-001 partitions state by tenant/workspace with an immutable partition key. Therefore each unit of truth has exactly one owning shard, and within that shard exactly one Raft leader may commit it. Sharding refines ownership; it never splits ownership of a single state across shards.

# Part 3 - IDR-001..005: The Reference Implementation Decisions

The following table restates the reference implementation decisions verbatim in intent. They are non-normative: they are the reference way to satisfy the registered invariants, not new invariants themselves.

| IDR | Decision | Registered invariants it serves |
| --- | --- | --- |
| IDR-001 | Kernel is the Control-Plane-of-commit using per-shard Raft: one Raft group per tenant/workspace shard; replicate committed OUTCOMES not engine invocations; engines run anywhere, only commit goes through the shard leader; Raft log = WAL = decision trace; no cross-shard atomic commit in v1 (use sagas). | ORCH-001, ORCH-003, OWN-001, SHARD-001 |
| IDR-002 | Replication: leader -> followers + snapshots + WAL. | ORCH-003 |
| IDR-003 | Membership via Raft joint consensus. | OWN-001, SHARD-001 |
| IDR-004 | Per-shard leader election. | OWN-001 |
| IDR-005 | Append-only WAL + snapshots. Truth = CP; observability = AP. | ORCH-003 |

Wording note: "Kernel is CP" in IDR-001 refers to the CAP-theorem consistency class (Consistent + Partition-tolerant) of the truth path, not the ARVES Control Plane. The ARVES Control Plane still owns no truth (ORCH-001).

# Part 4 - Sharding Model: One Raft Group per Tenant/Workspace

SHARD-001 (registered) partitions by tenant/workspace and fixes the partition key as immutable. IDR-001 maps each shard to exactly one Raft group. This is the backbone of horizontal scale.

## 4.1 Shard identity rules

- The partition key is (tenant, workspace) and is immutable for the life of the data (SHARD-001).

- Exactly one Raft group serves one shard; one shard is served by exactly one Raft group.

- A commit for shard S is ordered only by the Raft leader of S (IDR-004).

- No operation atomically spans two shards (no cross-shard atomic commit, IDR-001).

## 4.2 Shard sizing and placement

| Concern | Reference guidance | Rationale |
| --- | --- | --- |
| Group size | 3 or 5 voting replicas per shard | Tolerate 1 or 2 replica failures |
| Placement | Replicas across fault domains (host/rack/zone) | Avoid correlated loss of quorum |
| Leadership spread | Balance leaders across nodes | Avoid hot leader nodes |
| Shard count | Grow shards with tenants/workspaces | Scale-out unit is the shard |

## 4.3 Shard sizing checklist

- Confirm partition key is immutable and derivable at ingress (SHARD-001).

- Confirm each shard maps to exactly one Raft group (IDR-001).

- Confirm replica set spans independent fault domains.

- Confirm no request path assumes two shards commit atomically.

- Confirm leader placement is balanced and observable.

# Part 5 - The Commit Path: Engine-Anywhere -> Leader Commit -> Follower Apply

This is the most important flow in the volume. It is the mechanism by which ORCH-003 (execution replayable from recorded decision trace, NOT recomputation) and ORCH-001 (only Kernel owns truth) are honored under distribution.

## 5.1 The canonical sequence

| Step | Where | What happens | Invariant |
| --- | --- | --- | --- |
| 1. Plan | Control Plane (any node) | Produces a plan / engine graph; owns no truth | ORCH-001, ORCH-002 |
| 2. Invoke engine | Any node (engine-anywhere) | Engine runs pure/stateless; content-addressable + idempotent | ORCH-004 |
| 3. Produce outcome | Any node | Engine emits a PROPOSED effect (an outcome), not a commit | ORCH-001 |
| 4. Route to leader | Client / router | Outcome routed to the leader of the owning shard | SHARD-001, IDR-004 |
| 5. Commit | Shard leader | Leader appends committed OUTCOME to Raft log (WAL) | IDR-001, IDR-005 |
| 6. Replicate | Leader -> followers | Followers receive the committed outcome | IDR-002 |
| 7. Apply | Followers | Followers APPLY the committed outcome; they never recompute it | ORCH-003, IDR-001 |

## 5.2 Why followers apply, never recompute

The Raft log stores committed OUTCOMES, not engine invocations (IDR-001). A follower deterministically applies the recorded outcome. This is exactly ORCH-003: execution is replayable from the recorded decision trace, not by recomputation. Recomputing on a follower would (a) risk divergence if the engine is nondeterministic, and (b) violate the decision-trace model.

## 5.3 Engine-anywhere constraints (ORCH-004)

- Every engine/capability invocation MUST be idempotent (ORCH-004).

- Every invocation MUST be content-addressable so the same inputs map to the same identity (ORCH-004).

- Engines are pure/stateless and own nothing persistent; they emit proposed effects only.

- Only the shard leader may turn a proposed effect into a committed outcome.

- A retried invocation on a different node must not create a second truth (idempotency).

## 5.4 Commit-path review checklist

- Does the write go to the leader of the OWNING shard (partition key) and nowhere else?

- Is what gets replicated the OUTCOME, not the invocation?

- Do followers APPLY (deterministic) rather than RECOMPUTE?

- Is the engine invocation idempotent and content-addressable (ORCH-004)?

- If two shards are touched, is it a saga, not an atomic commit?

- Is the WAL entry sufficient to replay the decision (ORCH-003)?

# Part 6 - Replication: Leader, Followers, Snapshots, and WAL (IDR-002, IDR-005)

Replication makes truth durable and available within a shard. It does not create new truth; it copies committed outcomes.

## 6.1 Replication components

| Component | Role | Notes |
| --- | --- | --- |
| Leader | Orders and commits outcomes for the shard | One per shard (IDR-004) |
| Followers | Receive and apply committed outcomes | Serve bounded-staleness reads |
| WAL (append-only) | The Raft log = decision trace | Never mutated in place (IDR-005) |
| Snapshot | Compacted state at a log index | Bounds WAL growth, speeds recovery |

## 6.2 WAL as decision trace

IDR-005 makes the WAL append-only; IDR-001 equates the Raft log with the WAL and the decision trace. This single artifact serves consensus ordering, durability, and ORCH-003 replay. It is never edited; corrections are new appended outcomes or saga compensations, never in-place mutation.

## 6.3 Snapshotting rules

- A snapshot captures applied state up to a committed log index.

- WAL entries at or below a durable snapshot index may be truncated (compaction), preserving replayability from the snapshot forward.

- A follower far behind may be caught up via snapshot install rather than full log replay.

- Snapshots are per shard; there is no global snapshot (no cross-shard atomicity).

## 6.4 Replication health checklist

- Quorum is reachable for each shard (majority of voters live).

- Follower apply lag is within the bounded-staleness budget.

- Snapshot cadence bounds WAL size and recovery time.

- WAL is append-only in storage config (no in-place rewrite).

- Snapshot + tail-WAL together fully reconstruct shard state.

# Part 7 - Leader Election per Shard (IDR-004)

Each shard elects its own leader independently. There is no global leader; a cluster of 10,000 nodes has many shard leaders spread across nodes.

## 7.1 Election essentials

- Election is per shard (IDR-004); losing one shard leader does not affect other shards.

- A term increments on each election; a leader commits only entries of its own term until it has committed one entry of the current term.

- A candidate needs a quorum (majority of voters) to win.

- During an election the shard cannot commit new outcomes (write unavailability window); reads at eventual/bounded tiers may still be served by followers.

## 7.2 Election and the CP boundary

Because truth is CP, a shard that cannot form a quorum refuses new commits rather than accepting divergent truth. This is the deliberate CP choice from IDR-005: consistency over availability for the truth path.

## 7.3 Election tuning table

| Parameter | Effect if too low | Effect if too high |
| --- | --- | --- |
| Election timeout | Spurious elections, leader churn | Slow failover, longer write outage |
| Heartbeat interval | Network overhead | Late failure detection |
| Pre-vote | N/A (recommended on) | Avoids disruptive term inflation |

# Part 8 - Membership Changes via Joint Consensus (IDR-003)

Adding, removing, or replacing replicas of a shard changes the voting set. IDR-003 uses Raft joint consensus so that the change itself is committed through the log and never creates two disjoint majorities.

## 8.1 Joint consensus procedure

| Phase | Configuration active | Quorum requirement |
| --- | --- | --- |
| Start | C(old) | Majority of C(old) |
| Joint | C(old) + C(new) | Majority of BOTH C(old) and C(new) |
| Finish | C(new) | Majority of C(new) |

Because the joint phase requires a majority in both old and new configurations, no split-brain configuration can commit conflicting truth during reconfiguration.

## 8.2 Membership change checklist

- Change one membership step at a time; let each commit before the next.

- Add a new replica as a non-voting learner first; promote after it catches up.

- Never remove a voter that would drop the remaining set below quorum.

- Verify the new replica set still spans independent fault domains.

- Confirm the reconfiguration entry itself is committed in the WAL.

# Part 9 - Storage: Append-Only WAL and Snapshots (IDR-005)

Storage is where the decision trace lives. The append-only WAL plus periodic snapshots is the reference persistence model for the truth path.

## 9.1 Storage layering

| Artifact | Mutability | Purpose | Owner |
| --- | --- | --- | --- |
| WAL | Append-only | Ordered committed outcomes = decision trace | Kernel |
| Snapshot | Immutable once written | Compacted applied state | Kernel |
| Working memory (LCW) | Mutable, non-truth | Live state | LCW |
| Query indexes | Derived, rebuildable | Read acceleration | Query (read-only) |

Note: PERSIST-001 and LCW-001 are proposed (pending CCP) and are NOT relied upon as normative here; the durability guarantee used in this volume rests on IDR-005 and ORCH-003.

## 9.2 Storage durability checklist

- A commit is acknowledged only after the outcome is durable on a quorum of replicas.

- WAL fsync policy matches the required durability class.

- Snapshots are checksummed and self-describing (include the log index they cover).

- Recovery replays: load latest snapshot, then apply WAL tail.

- No process ever rewrites a committed WAL entry in place.

# Part 10 - The CP/AP Boundary: Truth is CP, Observability is AP

IDR-005 draws a hard line: the truth path is CP (consistent, partition-tolerant, sacrificing availability under partition), while observability (metrics, logs, tracing, presence) is AP (available, partition-tolerant, eventually consistent).

## 10.1 Classifying a piece of state

| State | Class | Behavior under partition |
| --- | --- | --- |
| Committed truth | CP | Refuse commit without quorum |
| Working memory / live state | Non-truth | May be stale; reconciles from truth |
| Metrics / logs / traces | AP | Continue collecting; converge later |
| Presence / liveness | AP | Best-effort, eventually consistent |

## 10.2 CP/AP boundary rules

- Never promote AP observability data into a truth decision without committing through the shard leader.

- Never block the truth path waiting on AP subsystems (metrics must not gate commits).

- A partitioned minority may still emit observability but MUST NOT commit truth.

- Do not read presence/liveness as if it were committed truth.

# Part 11 - Read Consistency Tiers

Reads choose a tier by need. The Query layer is READ-ONLY (writes nothing); tier selection changes where and how fresh the read is, never whether it can mutate.

| Tier | Served by | Guarantee | Use when |
| --- | --- | --- | --- |
| Linearizable | Shard leader | Sees latest committed outcome | Read-after-write, safety-critical checks |
| Bounded-staleness | Follower | At most a bounded lag behind leader | Dashboards with freshness SLA |
| Eventual | Any replica | Converges eventually | High-volume, tolerant reads |

## 11.1 Read-tier selection checklist

- Does the caller require read-after-its-own-write? -> linearizable (leader).

- Can the caller tolerate a known lag bound? -> bounded-staleness (follower).

- Is throughput more important than freshness? -> eventual (replica).

- Never issue a write through any read tier (Query is READ-ONLY).

- Never assume eventual reads reflect the latest commit.

# Part 12 - No Cross-Shard Atomic Commit: Sagas Instead

IDR-001 states there is NO cross-shard atomic commit in v1. Any workflow spanning multiple shards is expressed as a saga: a sequence of per-shard committed steps, each with a compensating action, coordinated by the (stateless) Control Plane.

## 12.1 Saga structure

| Element | Meaning | Ownership |
| --- | --- | --- |
| Forward step | A single-shard committed outcome | Owning shard leader |
| Compensation | An action that offsets a committed forward step | Owning shard leader |
| Coordinator | Sequences steps and triggers compensation | Control Plane (no truth) |
| Saga log | Progress record for replay | Per-shard WAL entries |

## 12.2 Saga design rules

- Each forward step commits atomically within ONE shard only.

- Every forward step MUST have a defined, idempotent compensation (ORCH-004).

- Compensations run in reverse order of committed forward steps.

- The coordinator holds no truth and no persistent state (ORCH-002); its progress is reconstructable from per-shard WAL entries (ORCH-003).

- There is no global lock and no two-phase atomic commit across shards.

## 12.3 Saga checklist

- Is every cross-shard workflow modeled as independent single-shard commits?

- Does each step have a tested compensating step?

- Are steps and compensations idempotent and content-addressable?

- Can the saga be replayed from recorded decision traces (ORCH-003)?

- Is partial progress safe if the coordinator crashes and restarts elsewhere?

# Part 13 - Failure Taxonomy and Compensation (Amendment-006)

Amendment-006 defines the failure model. The core principle: partial execution is rolled back by NOT committing; anything already committed is compensated saga-style. Node crash, partition, and split-brain are handled by the IDR mechanisms above.

## 13.1 Failure taxonomy

| Failure | Reference response | Mechanism |
| --- | --- | --- |
| Partial execution (pre-commit) | Roll back by NOT committing | Nothing enters WAL |
| Committed effect must be undone | Saga compensation | Amendment-006 + Part 12 |
| Node crash | Follower catches up via WAL/snapshot; re-elect if leader | IDR-002, IDR-004 |
| Network partition | Minority refuses commits (CP); majority continues | IDR-005 CP boundary |
| Split-brain attempt | Prevented by quorum + joint consensus | IDR-003, IDR-004 |
| Slow / laggy follower | Snapshot install; excluded from quorum until caught up | IDR-002, IDR-005 |

## 13.2 Key rule: no partial truth

There is never a half-committed truth. Either an outcome is committed (durable on a quorum, in the WAL) or it does not exist as truth. This is what makes "roll back by not committing" correct.

## 13.3 Failure-handling checklist

- Confirm pre-commit failures leave NO WAL entry (nothing to undo).

- Confirm every committed cross-shard effect has a compensation path.

- Confirm a partitioned minority cannot commit.

- Confirm a crashed leader triggers a clean per-shard re-election.

- Confirm recovery never recomputes committed outcomes (ORCH-003).

# Part 14 - Cancellation, Priority, and Preemption (Amendment-005)

Amendment-005 specifies cancellation as cooperative and idempotent, producing no partial truth, and priority/preemption via checkpoint -> replay.

## 14.1 Cancellation model

- Cancellation is cooperative: work observes a cancel signal at safe points.

- Cancellation is idempotent: repeating a cancel has the same effect (ORCH-004 alignment).

- Cancellation produces no partial truth: uncommitted work simply is not committed.

- Already-committed effects are removed via saga compensation, not by editing the WAL.

## 14.2 Priority and preemption

| Concept | Behavior | Truth impact |
| --- | --- | --- |
| Priority | Higher-priority work may preempt lower | None until commit |
| Checkpoint | Capture resumable progress (non-truth) | Not a commit |
| Replay | Resume from checkpoint / decision trace | Deterministic (ORCH-003) |
| Preemption | Pause lower-priority work, resume later | No partial truth left behind |

## 14.3 Cancellation/preemption checklist

- Does long-running work poll for cancellation at safe checkpoints?

- Is cancellation idempotent and side-effect-safe?

- Does preemption checkpoint resumable (non-truth) state only?

- Does resume rely on replay of the decision trace, not recomputation of committed truth?

- Is any committed effect undone only via compensation?

# Part 15 - Distribution-Readiness Rules and Milestone Map

Distribution readiness is governed by the registered invariants ORCH-002, ORCH-004, and SHARD-001. A component is distribution-ready only if it satisfies all three.

## 15.1 Distribution-readiness rules

| Rule | Registered invariant | Test |
| --- | --- | --- |
| Control Plane holds no persistent state | ORCH-002 | Can any Control Plane node be killed with no truth loss? |
| Invocations idempotent + content-addressable | ORCH-004 | Does a replayed invocation create a second truth? (must be no) |
| Partitioned by immutable key | SHARD-001 | Is the partition key fixed and derivable at ingress? |
| Replayable from decision trace | ORCH-003 | Can state be rebuilt from WAL + snapshot without recompute? |
| Single owner per state | OWN-001 | Does exactly one shard/leader own each datum? |

## 15.2 Milestone map (frozen names I1..I6)

| Milestone | Scope in this playbook | Primary IDR/invariant coverage |
| --- | --- | --- |
| I1 Distributed Runtime | Engine-anywhere, commit routing, WAL basics | IDR-001, IDR-005, ORCH-004 |
| I2 Cluster Kernel | Per-shard Raft, replication, election, membership | IDR-001..004, OWN-001, SHARD-001 |
| I3 Distributed Query | Read tiers across leader/followers/replicas | Read tiers, Query READ-ONLY |
| I4 Capability Scheduling | Scheduling idempotent capability invocations | ORCH-004, ORCH-002 |
| I5 Multi-Agent Runtime | Multi-agent workflows via sagas across shards | IDR-001 (no cross-shard atomic), Amendment-005/006 |
| I6 Reference Products | Products on the distributed runtime, spec unchanged | All registered invariants |

These six milestone names are used EXACTLY as frozen in Baseline Part 5. No other milestone names are introduced.

# Part 16 - Scaling to ~10,000 Nodes

Scale in ARVES comes from many independent shards, not from one large consensus group. The scaling unit is the shard (SHARD-001); the consensus cost is bounded per shard, not per cluster.

## 16.1 Why per-shard Raft scales

- Consensus cost is O(replicas per shard) - typically 3 or 5 - not O(cluster size).

- Shards are independent: a 10,000-node cluster runs thousands of small Raft groups.

- Leaders are spread across nodes; no single node orders all truth.

- Adding tenants/workspaces adds shards, which adds parallel commit capacity.

## 16.2 Scaling limits and mitigations

| Limit | Symptom | Mitigation |
| --- | --- | --- |
| Hot shard (one big tenant) | One Raft group saturates | Sub-partition within the tenant key policy (still immutable per SHARD-001) |
| Leader hotspot | One node hosts too many leaders | Leadership rebalancing |
| WAL growth | Disk pressure | Snapshot + compaction (IDR-005) |
| Cross-shard workflows | Saga latency | Minimize spans; parallelize independent steps |
| Membership churn | Reconfiguration storms | Learners + one-at-a-time joint consensus (IDR-003) |

## 16.3 Scaling readiness checklist

- Is the scale unit the shard, not a global group?

- Are Raft groups small (3 or 5 voters) and independent?

- Are leaders balanced across nodes at target node count?

- Is snapshot/compaction keeping WAL bounded per shard?

- Are cross-shard sagas rare and short relative to single-shard commits?

# Part 17 - Anti-Patterns and Guardrails

These are the ways engineers accidentally break the model. Each maps to a registered invariant or an explicit IDR prohibition.

| Anti-pattern | Why it is wrong | Rule violated |
| --- | --- | --- |
| Follower recomputes the outcome | Risks divergence; breaks decision-trace model | ORCH-003, IDR-001 |
| Control Plane stores durable state | Control Plane must be stateless | ORCH-002 |
| Two shards committed atomically | No cross-shard atomic commit in v1 | IDR-001 |
| Writing through a read tier | Query is READ-ONLY | Layer matrix (Query) |
| Editing a committed WAL entry | WAL is append-only | IDR-005 |
| Mutable partition key | Ownership would move | SHARD-001 |
| Non-idempotent engine call | Retry creates duplicate truth | ORCH-004 |
| Minority commits during partition | Split-brain truth | IDR-004/005 CP boundary |
| Treating proposed invariants as law | They are pending CCP | Governance (CCP-GATE) |

## 17.1 Proposed-invariant guardrail

G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, and ENG-001..005 are PROPOSED (pending CCP) and INFORMATIVE only. No behaviour may be ratified against them without a conformance scenario (CCP-GATE). This volume never leans on them as normative.

# Part 18 - Consolidated Engineering Checklists

## 18.1 New-shard bring-up checklist

- Immutable partition key defined and derivable (SHARD-001).

- One Raft group provisioned for the shard (IDR-001).

- 3 or 5 voters placed across fault domains.

- Leader elected; WAL and snapshot storage healthy (IDR-004, IDR-005).

- Read tiers wired (leader/follower/replica).

## 18.2 Commit-path certification checklist

- Writes route to the owning shard leader only.

- Only OUTCOMES are replicated; followers apply, never recompute (ORCH-003).

- Engine invocations idempotent + content-addressable (ORCH-004).

- No cross-shard atomic commit; multi-shard = saga (IDR-001).

## 18.3 Resilience checklist

- Quorum loss => refuse commits, not divergent truth (CP).

- Crash recovery => snapshot + WAL tail, no recompute.

- Membership changes => joint consensus, one step at a time (IDR-003).

- Cross-shard undo => saga compensation (Amendment-006).

- Cancellation => cooperative, idempotent, no partial truth (Amendment-005).

## 18.4 Governance checklist

- Only registered invariants treated as normative.

- IDR-001..005 treated as reference (non-normative) decisions.

- Proposed invariants flagged proposed (pending CCP).

- Milestone names used exactly as I1..I6.

- No new layers, invariants, milestones, or architecture introduced.

*Final Definition  A distributed ARVES runtime is many small per-shard Raft groups that commit outcomes through one shard leader and apply them everywhere, so that TRUTH stays single-owned and CP, observability stays AP, and scale to ~10,000 nodes comes from independent shards and sagas - never from cross-shard atomic commits.*

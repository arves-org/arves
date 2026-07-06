# TLC Run Record — captured mechanical model-check of `ARVES_ClusterLive.tla` (LIVENESS + DURABILITY)

> **Status:** captured verification evidence (living, `verification/`). Extends the cluster
> formal program **beyond safety**. `ARVES_Cluster.tla` (see `TLC_CLUSTER_RUN.md`) checks the
> per-shard Raft **safety** invariants only. This module — `ARVES_ClusterLive.tla` — adds the two
> properties safety cannot express:
>
> 1. **LIVENESS** — under weak fairness on the protocol steps and the partial-synchrony
>    assumption Raft's liveness genuinely requires, **a leader is eventually elected** and **a
>    client entry is eventually committed**. The cluster makes progress; it does not stall.
> 2. **DURABILITY / crash-recovery** — a node may **crash** (lose all volatile state) and
>    **restart from its persistent log**; **committed truth survives every crash** — it stays
>    quorum-durable and never diverges.
>
> **Honest framing:** this is a model-check of the **protocol design**, not of the Rust code
> (`arves-consensus/src/raft.rs`, `arves-kernel/src/cluster.rs`). It is one honest input to the
> Formal GA-gate condition, not a proof that the Rust matches the model line-for-line (the Rust
> test suite + the deterministic in-process sim harness are that evidence).

---

## 1. LIVENESS — verdict

```
Model checking completed. No error has been found.
```

**All three temporal properties hold** over the exhaustive state graph of the liveness instance
(`Server = {n1,n2,n3}`, `MaxTerm = 3`, `MaxLogLen = 2`, `MaxCrash = 0`, `SoleCandidate = n1`):

| Property | Statement | Result |
|---|---|---|
| `LeaderEventuallyElected` | `<>(\E i : state[i] = Leader)` | **HOLDS** |
| `ProgressToCommit` | `<>(\E i : commitIndex[i] > 0)` — the cluster eventually commits (no stall) | **HOLDS** |
| `EntryEventuallyCommitted` | `(\E i : Leader ∧ Len(log[i]) ≥ 1) ~> (\E i : commitIndex[i] ≥ 1)` — a held entry is eventually committed | **HOLDS** |

`642 distinct states, depth 15`, 3 temporal branches checked, 0 states left on queue.

### Verbatim TLC output (2026-07-06)

```
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Running breadth-first search Model-Checking with fp 123 and seed -5511656279677340859 with 16 workers on 16 cores with 7191MB heap and 64MB offheap memory [pid: 28160] (Windows 11 10.0 amd64, Eclipse Adoptium 21.0.11 x86_64, MSBDiskFPSet, DiskStateQueue).
Starting... (2026-07-06 10:08:12)
Implied-temporal checking--satisfiability problem has 3 branches.
Computing initial states...
Finished computing initial states: 1 distinct state generated at 2026-07-06 10:08:13.
Progress(15) at 2026-07-06 10:08:13: 1,084 states generated, 642 distinct states found, 0 states left on queue.
Checking 3 branches of temporal properties for the complete state space with 1926 total distinct states at (2026-07-06 10:08:13)
Finished checking temporal properties in 00s at 2026-07-06 10:08:13
Model checking completed. No error has been found.
1084 states generated, 642 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 15.
```

### Why the partial-synchrony assumption (`SoleCandidate = n1`) — and why it is honest, not a trick

Raft **cannot** guarantee liveness under fully asynchronous, perpetually dueling elections: split
votes can recur forever, and at a bounded `MaxTerm` three candidates can split the vote 1-1-1 and
deadlock with no term left to break the tie. Real Raft breaks the tie with **randomized election
timeouts** — eventually one server's timeout fires *uncontested*. The constant `SoleCandidate`
models exactly that outcome: with `SoleCandidate = n1`, only `n1` may time out, i.e. "eventually
one server elects uncontested" — the precise partial-synchrony condition under which Raft liveness
is provable. It is declared up front in the module header and the cfg, not hidden.

**Falsifiability — the liveness claim has teeth (captured).** Set `SoleCandidate = Nil` (full
dueling elections) in the liveness cfg and TLC **finds a counter-example** — Raft is *not* live
under unbounded contention, exactly as theory predicts:

```
Error: Temporal properties were violated.
Error: The following behavior constitutes a counter-example:
State 2: Timeout — n1 -> Candidate, currentTerm 1
State 3: Timeout — n1 -> Candidate, currentTerm 2   (re-times-out; never consummates)
State 4: Vote    — n3 grants n1 at term 2 ...
   (36,592 distinct states explored before the fair split/stutter cycle is reported)
```

This is why the assumption is *declared*: without it the property is genuinely false, and the
model says so rather than papering over it.

---

## 2. DURABILITY / crash-recovery — verdict

```
Model checking completed. No error has been found.
```

**`DurableSafetyInv` holds in every reachable state — including every post-crash state** — of the
durability instance (`Server = {n1,n2,n3}`, `MaxTerm = 3`, `MaxLogLen = 2`, **`MaxCrash = 2`**,
`SoleCandidate = Nil` — full nondeterministic election). It conjoins the five Raft safety
invariants **plus** the durability invariant:

| Invariant | Meaning under crash/recovery | Result |
|---|---|---|
| `ElectionSafety`, `LogMatching`, `StateMachineSafety`, `LeaderCompleteness`, `LinearizableCommit` | committed truth never diverges even as nodes crash and rejoin | **HOLD** |
| `DurableOnQuorum` | every committed entry stays held, at its `(idx,term)`, by a **quorum** of persistent logs — so it survives any crash set and no later leader can be elected without it | **HOLDS** |

Exhaustive breadth-first search: **5,873,227 states generated, 1,585,424 distinct, depth 24**,
0 states left on queue.

### Crash model (the faithful Raft persistence boundary)

`Crash(i)` drops **volatile** state — `state -> Follower`, `commitIndex -> 0`, `votesGranted -> {}`
— and **keeps persistent** state `{currentTerm, votedFor, log}`. The append-only `log` is the
durable WAL (IDR-005); a restart replays it (ORCH-003). The `committed` history — the god's-eye
record of what was acked to a caller — is **unchanged** by a crash: the whole durability question
is *"do those acked facts survive?"*, so the model must not "help" by editing that history.

### Verbatim TLC output (2026-07-06)

```
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Running breadth-first search Model-Checking with fp 110 and seed -7277950643004026354 with 16 workers on 16 cores with 7191MB heap and 64MB offheap memory [pid: 36424] (Windows 11 10.0 amd64, Eclipse Adoptium 21.0.11 x86_64, MSBDiskFPSet, DiskStateQueue).
Starting... (2026-07-06 10:17:21)
Computing initial states...
Finished computing initial states: 1 distinct state generated at 2026-07-06 10:17:22.
Progress(17) at 2026-07-06 10:17:25: 709,746 states generated (709,746 s/min), 239,631 distinct states found (239,631 ds/min), 101,515 states left on queue.
Progress(22) at 2026-07-06 10:18:25: 4,227,000 states generated (3,517,254 s/min), 1,225,100 distinct states found (985,469 ds/min), 234,758 states left on queue.
Model checking completed. No error has been found.
5873227 states generated, 1585424 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 24.
Finished in 01min 30s at (2026-07-06 10:18:52)
```

A lighter, sub-minute variant (`MaxCrash = 1`, a single crash/restart) is also exhaustively clean:
`2,380,993 states generated, 764,459 distinct, depth 23, No error` (42 s).

### Falsifiability — the durability invariant has teeth (captured)

`DurableOnQuorum` is only meaningful if a **broken** persistence boundary makes it fail. Probe:
change `Crash(i)` so it **wipes the crashed node's log** (a node with a *non-durable* WAL — it
loses its log on restart) and run the same instance. TLC reports the violation immediately:

```
Error: Invariant DurableSafetyInv is violated.
  ...
  n1 commits index 1 on quorum {n1, n2}   ->  committed = {[idx |-> 1, term |-> 1, cterm |-> 1]}
  n1 CRASHES and loses its log            ->  log[n1] = <<>>
  Now only n2 holds the committed entry (1 of 3, NOT a quorum)  ->  DurableOnQuorum VIOLATED.
```

Restoring the durable log (`UNCHANGED log` in `Crash`) restores `No error has been found`. So the
persistence boundary — *keep the log across a crash* — is **load-bearing**, and the durability
invariant is not satisfied vacuously.

---

## Environment

- **TLC2 Version 2.19** of 08 August 2024 (rev 5a47802), `tla2tools.jar`
  sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`
  (the jar is **not committed** — a 2.3 MB binary; download it with the command below).
- **Java:** Eclipse Temurin 21.0.11 x86_64 (portable JRE zip — no admin install needed).
- Windows 11, 16 workers. Liveness ≈ 2 s; durability (`MaxCrash = 2`) ≈ 90 s.

## Reproduce

```sh
cd verification/formal
curl -sL -o tla2tools.jar https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar

# LIVENESS (partial synchrony, fairness) — expect: No error has been found (3 properties)
java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC -deadlock \
     -config ARVES_Cluster_Liveness_MC.cfg ARVES_ClusterLive.tla -workers auto

# DURABILITY (crash/recovery, exhaustive safety) — expect: No error has been found
java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC -deadlock \
     -config ARVES_ClusterLive_MC.cfg ARVES_ClusterLive.tla -workers auto
```

To reproduce the two teeth probes: (liveness) set `SoleCandidate = Nil` in
`ARVES_Cluster_Liveness_MC.cfg` → temporal violation; (durability) edit `Crash(i)` to also reset
`log[i]` to `<< >>` → `DurableSafetyInv` violation. Revert after confirming.

> **`-deadlock`** disables TLC's deadlock check: these are *bounded* instances, so a terminal
> state where every action is disabled (terms/log exhausted) is expected and is not a bug.

## Honest scope — what THESE runs do and do NOT cover

**Cover (mechanically checked):**

- **Liveness**, under the *explicitly declared* partial-synchrony assumption (`SoleCandidate = n1`)
  + weak fairness on every protocol step: a leader is eventually elected, and a held client entry
  is eventually committed — the cluster makes progress. The assumption is falsifiable-checked:
  removing it (full dueling) breaks the property.
- **Durability across crash/recovery**: with full nondeterministic election and up to **two**
  crash/restart events, committed truth stays quorum-durable and never diverges. The persistence
  boundary (durable log, volatile role/commitIndex/votes) is falsifiable-checked: a non-durable
  log breaks it.

**Still abstracted away (NOT covered here — as in `ARVES_Cluster.tla`):**

- **Message layer.** Vote/replicate are atomic actions; commit reads the replicated logs directly
  (the standard, sound match-index abstraction for Raft *safety*). No socket framing / reordering /
  duplication / loss is modeled, and **no network fault-tolerance is claimed**.
- **Liveness is under partial synchrony, not full asynchrony** — which is correct (Raft liveness
  is impossible under full asynchrony, FLP). We do not claim asynchronous liveness.
- **Joint-consensus membership change** (IDR-003 / RCR-020): fixed voter set.
- **Snapshot install / log compaction / wire format**: runtime stages, out of scope of the model.
- **Bounds.** Small finite instance (`MaxTerm = 3`, `MaxLogLen = 2`, `MaxCrash ≤ 2`). Deeper
  Figure-8 / longer-crash-sequence behaviours beyond these bounds are not explored; the bound is
  stated, not glossed.

**Evidence level:** the I2 cluster protocol's **liveness** (progress under partial synchrony) and
**durability** (committed truth survives crash/recovery) are raised from *no formal model* to
**captured, reproducible, exhaustive mechanical checks** of a small finite instance, each with a
captured falsifiability probe. Together with the safety run (`TLC_CLUSTER_RUN.md`) the distributed
protocol is now formally modeled for safety **and** liveness **and** durability — three honest
inputs to the Formal GA-gate condition, still model-checks of the design, not proofs of the Rust.

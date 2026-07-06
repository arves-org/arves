# TLC Run Record — captured mechanical model-check of `ARVES_Cluster.tla`

> **Status:** captured verification evidence (living, `verification/`). Extends the formal
> program from the single-node kernel commit gateway (`ARVES_Kernel.tla`, see `TLC_RUN.md`)
> to the **I2 distributed cluster commit/leader protocol** — per-term leader election, log
> replication, and commit-on-quorum, with the Raft SAFETY invariants.
>
> **Honest framing:** this is a model-check of the **protocol**, not of the Rust code. It
> mechanically verifies that the *design* the reference runtime implements
> (`arves-consensus/src/raft.rs`, `arves-kernel/src/cluster.rs`) upholds the safety
> properties; it is not a proof that the Rust matches the model line-for-line (the Rust test
> suite + the deterministic in-process sim harness are that evidence). It contributes to — and
> does not by itself discharge — the Formal GA-gate condition.

## Verdict

```
Model checking completed. No error has been found.
```

**The safety conjunction `SafetyInv` holds in every reachable state** of the finite instance
(`Server = {n1,n2,n3}`, `MaxTerm = 3`, `MaxLogLen = 3`):

| Invariant | Raft name (Figure 3) | ARVES grounding | Result |
|---|---|---|---|
| `ElectionSafety` | Election Safety | ≤ 1 leader per term (IDR-004) | **HOLDS** |
| `LogMatching` | Log Matching | append-only log, prev-term check (IDR-005) | **HOLDS** |
| `StateMachineSafety` | State Machine Safety | committed truth never diverges (ClusterKernel apply loop, ORCH-003) | **HOLDS** |
| `LeaderCompleteness` | Leader Completeness | a later-term leader holds every committed entry (Sec.5.4.1 vote restriction) | **HOLDS** |
| `LinearizableCommit` | (ARVES) | committed prefixes identical across replicas — the `ClusterKernel` ack-after-quorum guarantee | **HOLDS** |
| `TypeOK` | — | state well-typedness | **HOLDS** |

Exhaustive breadth-first search: **2,756,461 states generated, 1,070,962 distinct, depth 26**,
0 states left on queue. Fingerprint-collision probability 2.0E-7 (based on actual fingerprints).

## Verbatim TLC output (2026-07-06)

```
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Running breadth-first search Model-Checking with fp 107 and seed 2131158924464337304 with 16 workers on 16 cores with 7191MB heap and 64MB offheap memory [pid: 15500] (Windows 11 10.0 amd64, Eclipse Adoptium 21.0.11 x86_64, MSBDiskFPSet, DiskStateQueue).
Starting... (2026-07-06 08:13:11)
Computing initial states...
Finished computing initial states: 1 distinct state generated at 2026-07-06 08:13:11.
Progress(19) at 2026-07-06 08:13:14: 680,130 states generated (680,130 s/min), 279,452 distinct states found (279,452 ds/min), 93,169 states left on queue.
Model checking completed. No error has been found.
  Estimates of the probability that TLC did not check all reachable states
  because two distinct states had the same fingerprint:
  calculated (optimistic):  val = 9.8E-8
  based on the actual fingerprints:  val = 2.0E-7
2756461 states generated, 1070962 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 26.
The average outdegree of the complete state graph is 1 (minimum is 0, the maximum 7 and the 95th percentile is 3).
Finished in 52s at (2026-07-06 08:14:02)
```

## Environment

- **TLC2 Version 2.19** of 08 August 2024 (rev 5a47802), `tla2tools.jar`
  sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`
  (the jar is **not committed** — a 2.3 MB binary; download it with the command below).
- **Java:** Eclipse Temurin 21.0.11 x86_64 (portable JRE zip — no admin install needed).
- Windows 11, 16 workers. Wall time 52 s.

## Reproduce

```sh
cd verification/formal
curl -sL -o tla2tools.jar https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar
java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC -deadlock -config ARVES_Cluster_MC.cfg ARVES_Cluster.tla -workers auto
# expect: "Model checking completed. No error has been found."
```

For sharper diagnostics, split `INVARIANT SafetyInv` in the `.cfg` into the six lines
`INVARIANT TypeOK`, `INVARIANT ElectionSafety`, `INVARIANT LogMatching`,
`INVARIANT StateMachineSafety`, `INVARIANT LeaderCompleteness`, `INVARIANT LinearizableCommit`.

## Falsifiability — the model has teeth (captured)

The "prove it wrong" discipline requires showing the model *rejects* a broken protocol. Two
probes are captured below. **Probe 1** (Sec.5.4.1 up-to-date vote restriction) has teeth at the
committed `MaxTerm = 3` instance; **Probe 2** (Sec.5.4.2 current-term commit guard, the Raft
"Figure 8" rule) is *non-load-bearing at `MaxTerm = 3`* and only shows its teeth at a deeper
`MaxTerm = 4` instance — an honest, verified fact recorded exactly, not glossed.

### Probe 1 — Sec.5.4.1 up-to-date vote restriction

Weakening the election restriction — `LogUpToDate(cand, v) == TRUE` — makes TLC
report an **`SafetyInv` violation at depth 10** captured here with **`-workers 1`**: 16,739 states
generated, 7,679 distinct. (Run with `-workers 1` for these *exact* reproducible numbers. Under
`-workers auto` TLC returns the first violation *any* worker reaches, so the reported depth and
state counts vary run-to-run — a re-runner who sees, e.g., depth 13 / ~22k states is seeing the
same *class* of violation, an empty-log node elected across terms while a committed entry
diverges, not a discrepancy.) The minimal counterexample is the exact reason the restriction
exists:

```
State  5: n1 wins term 1 (empty logs), becomes Leader.
State  6: n1 (leader, term 1) appends entry [term |-> 1] at index 1  (log[n1] = <<[1]>>).
State  7: n1 replicates that entry to n3                              (log[n3] = <<[1]>>).
State  8: n2 (EMPTY log) collects votes {n2, n3} at term 2 — GRANTED only because the
          up-to-date check is disabled; the real check would REFUSE n3's vote (n2's log is
          behind n3's).
State  9: n2 becomes Leader at term 2 with an empty log.
State 10: n1 commits index 1 on quorum {n1, n3}  ->  committed = {[idx |-> 1, term |-> 1, cterm |-> 1]}
          But n2 is a Leader with currentTerm 2 > 1 and Len(log[n2]) = 0  ->
          LeaderCompleteness is VIOLATED (a committed entry is absent from a future leader,
          about to be overwritten — divergent committed truth).
```

Restoring the real `LogUpToDate` restores `No error has been found`.

### Probe 2 — Sec.5.4.2 current-term commit guard (Raft "Figure 8")

This is the guard `log[i][n].term = currentTerm[i]` in `AdvanceCommit` — the rule that a leader
may only advance `commitIndex` onto an entry **of its own term**, never count a replicated
prior-term entry toward commit. It is the exact mechanism that lets ARVES omit the Raft election
no-op entry (RCR-019 DR-2 — the frozen `EntryKind` carries only `Outcome | Membership`). So the
question "does the guard actually have teeth?" is the question "is the no-op-omission safe?".

**Honest verified finding — the guard is NOT load-bearing at the committed `MaxTerm = 3` instance.**
Deleting the guard and running TLC to **full exhaustion** at `Server = {n1,n2,n3}`, `MaxTerm = 3`,
`MaxLogLen = 3` finds **no violation**:

```
7,390,624 states generated, 2,699,023 distinct, depth 27, 0 states left on queue.
Model checking completed. No error has been found.   (2 min 23 s, -workers auto)
```

The search *did* complete; the Figure-8 counterexample simply is **not reachable** at
`MaxTerm = 3` (three terms are too few to elect the divergent later-term leader that overwrites a
prior-term commit). At this bound safety holds *with or without* the guard, so `MaxTerm = 3` alone
does **not** demonstrate the guard's necessity — a fact this record states plainly rather than
attributing the clean result to an unfinished budget.

**The guard's teeth appear at `MaxTerm = 4` — captured.** Removing the guard at
`Server = {n1,n2,n3}`, `MaxTerm = 4`, `MaxLogLen = 3` produces a genuine **`LeaderCompleteness`
violation** — the textbook Raft Figure 8:

```
State 20: AdvanceCommit — n2 (Leader, currentTerm 3) commits a PRIOR-term entry [term |-> 1] at
          index 1 on quorum {n2, n3}.   (only possible because the current-term guard is gone)
          committed = { [idx |-> 1, term |-> 1, cterm |-> 3] }
State 21: Vote — n1 (log = <<[term |-> 2]>>, a DIVERGENT entry at index 1) wins term 4 with {n1,n2}.
State 22: BecomeLeader — n1 is Leader at currentTerm 4 > cterm 3, but log[n1][1].term = 2 # 1.
          -> a committed entry is absent from a later-term leader, about to be overwritten:
          LeaderCompleteness is VIOLATED  (divergent committed truth — the Figure-8 bug).
```

Restoring the guard closes it: the **guarded** model at the **same** `MaxTerm = 4` bound is
**exhaustive and clean** —

```
14,409,961 states generated, 4,839,798 distinct, depth 33, 0 states left on queue.
Model checking completed. No error has been found.   (32 s, -workers auto)
```

So at `MaxTerm = 4` the guard is **load-bearing**: removing it breaks `LeaderCompleteness`,
restoring it makes the whole state space safe. *This* is the demonstration that earns the
RCR-019 DR-2 no-op-omission claim — the current-term commit rule is a real, falsifiably-checked
substitute for the election no-op. (The counterexample's exact depth and state count vary under
`-workers auto` — a parallel run may report the violation at depth 22 or 24 with a different
state total; the *class* of violation and the guarded/unguarded verdicts are stable.)

## Non-vacuity — the safety invariants are actually exercised (captured)

`StateMachineSafety` and `LeaderCompleteness` are only meaningful if the reachable space actually
contains states with a **non-empty `committed` set AND a leader of a strictly later term** — if
those states were unreachable, the invariants would pass *vacuously* and prove nothing. A cheap
witness confirms they are reached. Add the negated reachability predicate

```
NonVacuityWitness ==
    ~ ( \E e \in committed : \E i \in Server : state[i] = Leader /\ currentTerm[i] > e.cterm )
```

as an `INVARIANT` and run the **main (guarded)** model. TLC reports it **violated** — i.e. the
interesting state IS reachable — captured with `-workers 1` (reproducible): **violation at depth
10**, 15,561 states generated, 6,881 distinct. The witnessing state:

```
committed = { [idx |-> 1, term |-> 1, cterm |-> 1] }   (a committed entry exists)
n2 = Leader, currentTerm 2  >  cterm 1                  (a strictly-later-term leader coexists)
```

So `LeaderCompleteness` (which then *requires* that later-term leader to hold the committed
entry) and `StateMachineSafety` are checked against real cross-term-with-committed states, not
satisfied vacuously. (This `NonVacuityWitness` is a sanity probe — it is **not** part of the
committed `SafetyInv`; it is meant to be added, observed to fail, and removed.)

## Honest scope — what THIS model does and does NOT cover

**Covers (mechanically checked):** per-term leader election with the Sec.5.4.1 up-to-date vote
restriction and per-term single vote (Election Safety); single-entry log replication with the
prev-term consistency check and follower truncation (Log Matching); commit-on-quorum under the
Sec.5.4.2 current-term rule with **no election no-op** (matching ARVES RCR-019 DR-2); and the
cross-replica agreement of committed prefixes (State Machine Safety / Leader Completeness /
Linearizable-commit — the `ClusterKernel` "ack only after quorum, apply in log order on every
replica" guarantee).

**Abstracted away (NOT covered here):**

- **Message layer.** Vote/replicate are folded into atomic actions and commit reads the
  replicated logs directly (the standard, sound match-index abstraction for Raft *safety*).
  Real socket framing/reordering/duplication/loss is out of scope — it is the runtime's Rust +
  sim-harness concern, and no network fault-tolerance is claimed by this model.
- **Joint-consensus membership change** (IDR-003 / RCR-020): the voter set is fixed here. The
  dynamic reconfiguration safety argument is a separate, larger obligation.
- **Liveness.** This is a safety-only spec (no fairness, no temporal property). The lesson from
  `TLC_RUN.md` — symmetry during liveness checking is unsound — is sidestepped entirely: there
  is no liveness property, and **no symmetry** is declared (the full state graph is checked).
- **Crash/recovery durability, snapshot install, log compaction, wire format** — runtime stages
  (the kernel's fault-injection recovery tests are that evidence).

**Evidence level:** the I2 cluster commit/leader protocol is raised from *no formal model* to a
**captured, reproducible, exhaustive mechanical safety check** of a small finite instance. It is
the distributed successor `ARVES_Kernel_Formal_Spec.md` §6 called out as "a later, larger
`ARVES_Consensus.tla`", and one honest input to the Formal GA-gate condition — a model-check of
the protocol design, not a proof of the Rust implementation.

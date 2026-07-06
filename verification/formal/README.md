# `verification/formal/` — TLA+, temporal logic, state-machine models, semantics, proofs

Formal-methods verification evidence for ARVES. **Not** normative specification —
these artifacts *model* behaviour the frozen corpus already prescribes and
mechanically check it. The frozen `.docx` corpus is untouched (ED-001).

## Contents

| File | What it is |
|---|---|
| `ARVES_Kernel.tla` | TLC-checkable TLA+ module: single-shard commit gateway + append-only log + truth set + replay. |
| `ARVES_Kernel_MC.cfg` | TLC instance (`Content = {c1, c2}`), the invariants, and the liveness property to check. |
| `ARVES_Kernel_Formal_Spec.md` | The write-up: what each property proves, what is abstracted away, and the `AKF-CS-1` conformance scenario (byte-exact bridge to the reference kernel). |
| `ARVES_Cluster.tla` | TLC-checkable TLA+ module: the **I2 distributed** per-shard Raft protocol — per-term leader election, log replication, commit-on-quorum — with the Raft safety invariants. |
| `ARVES_Cluster_MC.cfg` | TLC instance (`Server = {n1,n2,n3}`, `MaxTerm = 3`, `MaxLogLen = 3`), safety-only, no symmetry. |
| `TLC_CLUSTER_RUN.md` | Captured cluster run: verbatim `No error has been found` over the exhaustive state space, plus two falsifiability probes (incl. the `MaxTerm = 4` Figure-8 current-term-guard teeth demo) and a non-vacuity witness. |

## Two models, two scopes (honest)

- `ARVES_Kernel.tla` models the **single-shard commit gateway** *as if the log were already
  the agreed, replicated log* — the state-machine layer above consensus (OWN-001, ORCH-003/004,
  liveness under fairness).
- `ARVES_Cluster.tla` models the **distributed protocol underneath it** — N-node per-term
  election, replication, and commit-on-quorum — and checks the classic Raft **safety**
  invariants (Election Safety, Log Matching, State Machine Safety, Leader Completeness) plus a
  **Linearizable-commit** invariant matching the `ClusterKernel` "ack only after quorum, apply
  in log order on every replica" guarantee. Both are model-checks of the **protocol design**,
  not proofs of the Rust code (`arves-consensus`, `arves-kernel::cluster`); see
  `TLC_CLUSTER_RUN.md` for the exact scope and abstractions (message layer, joint-consensus
  membership, liveness, and durability are out of scope of the cluster model).

## How to run TLC (reproducible recipe)

> **Kernel model — first captured run 2026-07-05 — `Model checking completed. No error has been found.`**
> Both `SafetyInv` and `EventuallyCommitted` hold over the exhaustive state space. Verbatim
> output, environment pins (TLC 2.19, Temurin 21, jar sha256), and the symmetry-unsoundness
> finding the run surfaced-and-fixed are recorded in [`TLC_RUN.md`](TLC_RUN.md).
>
> **Cluster model — captured run 2026-07-06 — `Model checking completed. No error has been found.`**
> `SafetyInv` (Election Safety, Log Matching, State Machine Safety, Leader Completeness,
> Linearizable-commit) holds over **1,070,962 distinct states** (depth 26). Verbatim output, two
> captured falsifiability probes (incl. the `MaxTerm = 4` Figure-8 current-term-guard teeth demo,
> with the guarded model exhaustively clean at that same bound), and a non-vacuity witness are in
> [`TLC_CLUSTER_RUN.md`](TLC_CLUSTER_RUN.md). Run it with `-config ARVES_Cluster_MC.cfg ARVES_Cluster.tla`.

TLC is **not** vendored in this repo. You need Java and a TLA+ tool
(`tla2tools.jar`, bundled with the TLA+ Toolbox or downloadable standalone).

```sh
# From this directory (verification/formal/). Put tla2tools.jar here or point -cp at it.
java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC \
     -config ARVES_Kernel_MC.cfg ARVES_Kernel.tla
```

Expected on a correct model: TLC reports **no errors** — every reachable state
satisfies `SafetyInv` and the temporal property `EventuallyCommitted` holds. The
state space for `Content = {c1, c2}` is tiny and completes in well under a second.
Full details, the falsifiability demonstration, the Apalache alternative, and the
`AKF-CS-1` byte-exact vectors are in `ARVES_Kernel_Formal_Spec.md` (§5, §7).

## Honest status — captured runs exist; jar/states uncommitted; CI unwired

**Two TLC runs ARE now captured with verbatim tool output committed** — the kernel
model in [`TLC_RUN.md`](TLC_RUN.md) (`SafetyInv` + `EventuallyCommitted`, exhaustive,
`No error has been found`) and the cluster model in
[`TLC_CLUSTER_RUN.md`](TLC_CLUSTER_RUN.md) (`SafetyInv`, exhaustive over 1,070,962
distinct states, plus two captured falsifiability probes and a non-vacuity witness).
Each records the TLC version, the `tla2tools.jar` sha256, the Java/OS environment, and
a reproduce command.

What is **still** honest to flag:

- **The `tla2tools.jar` and the generated `states/` are not committed** (a 2.3 MB binary
  and machine-specific fingerprint files; both `.gitignore`d). A re-runner downloads the
  pinned jar and regenerates states from the committed `.tla`/`.cfg`.
- **No CI host currently executes these models.** The `.github/workflows/` gate *defines*
  the check; no runner is wired to run it on every push yet. The captured logs are evidence
  the check passed *when run by hand*, not a standing gate.
- The `AKF-CS-1` byte-exact conformance scenario has **not** yet passed CCP-GATE.

Any "expected result" text elsewhere remains the claim under test — a certifier should
re-run TLC (recipe above) and confirm the captured verdicts reproduce.

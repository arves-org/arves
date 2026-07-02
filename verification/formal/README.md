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

## How to run TLC (reproducible recipe)

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

## Honest status — NO CAPTURED RUN

**No TLC/Apalache run is checked into this repository.** There is no committed
tool log, and no CI host currently executes this model (the `.github/workflows/`
gate defines the check; no CI host is wired yet). The commands above are the
*recipe a certifier runs*, not evidence that the check has already passed here.
Any "expected result" text is the claim under test — confirm it by running TLC
yourself. This artifact stays **DRAFT evidence** until a run is captured and the
`AKF-CS-1` scenario passes CCP-GATE.

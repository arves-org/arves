# TLC Run Record — first captured mechanical model-check of `ARVES_Kernel.tla`

> **Status:** captured verification evidence (living, `verification/`). Closes the open-debt
> item *"TLA+ kernel spec not mechanically model-checked (L0, no captured TLC run)"* —
> strategic-program **rank 5**, previously blocked on tooling (no Java in the environment).

## Verdict

```
Model checking completed. No error has been found.
```

**Both checked properties hold over the exhaustive state space:**

| Property | Kind | Result |
|---|---|---|
| `SafetyInv` | state invariant (truth/log consistency, idempotent commit) | **HOLDS** in every reachable state |
| `EventuallyCommitted` | liveness — `(pc[c] = "proposed") ~> (c ∈ truth)` under `WF_vars(Commit(c))` | **HOLDS** (2 temporal branches checked) |

Exhaustive search: **13 states generated, 10 distinct, depth 5**, 20 total distinct states in
the temporal check; fingerprint-collision probability 1.6E-18 (i.e. effectively exact).

## Environment (2026-07-05)

- **TLC2 Version 2.19** of 08 August 2024 (rev 5a47802), `tla2tools.jar`
  sha256 `936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`
  (from `https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar`;
  the jar is **not committed** — a 2.3 MB binary; download it with the command below).
- **Java:** Eclipse Temurin 21.0.11 x86_64 (portable JRE zip — no admin install needed:
  `https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jre/hotspot/normal/eclipse`).
- Windows 11, 16 workers.

## Reproduce

```sh
cd verification/formal
curl -sL -o tla2tools.jar https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar
java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC -deadlock -config ARVES_Kernel_MC.cfg ARVES_Kernel.tla -workers auto
# expect: "Model checking completed. No error has been found."
```

## Finding fixed by this run — SYMMETRY + liveness was unsound (spurious counterexample)

The shipped `ARVES_Kernel_MC.cfg` declared `SYMMETRY Perms` **while checking a liveness
PROPERTY** — a combination TLC itself warns about (*"Declaring symmetry during liveness checking
is dangerous"*). The very first mechanical run reproduced exactly why: under symmetry, TLC
reported a **spurious** `EventuallyCommitted` violation whose trace ends with `c2` still `"open"`
— but `EventuallyCommitted` is `(pc[c]="proposed") ~> (c ∈ truth)`, and a never-proposed content
satisfies the leads-to trivially. Removing the (unnecessary — the full space is 20 states)
symmetry declaration checks the **complete** state graph and both properties hold.

This is precisely what the "prove it wrong" discipline is for: the un-run config carried a latent
unsoundness that only an actual mechanical run could surface. The fix is recorded in the cfg
header; this file is the captured evidence.

## Honest scope

This checks the **single-shard commit-gateway abstraction** (2 content tokens: first commit,
idempotent re-commit, interleaving, out-of-order) — the model documented in
`ARVES_Kernel_Formal_Spec.md`, including its `AKF-CS-1` byte-exact bridge to the reference
kernel. It does **not** model the distributed I2..I6 behaviour (per-shard Raft etc.), which is
design-stage (Ch4 prep mode). Evidence level for the kernel-commit model: raised from
**L0 (written, never run)** to a **captured, reproducible mechanical check**.

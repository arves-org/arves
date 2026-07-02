# @arves/agent-runtime — Agent Runtime (P3)

**An agent whose every thought is replayable, audited truth in a real Kernel.**

P3 runs the full cognitive loop and commits *every step* as content-addressed truth in
the **real ARVES reference Kernel** (through the P2 bridge):

```
Memory → Reasoning → Planning → Capability Selection → Execution → Truth Update
```

## Could this be built without ARVES? No.

Ordinary agent frameworks give you a chain of prompts and side effects you cannot
reproduce or audit. ARVES gives you an agent whose entire decision trace — what it knew,
what it concluded, what it planned, which capability it chose, what it did — is truth in
a real Kernel: **deterministic, content-addressed, idempotent, and replayable**. Run it
twice with the same inputs and you get byte-identical trace ids and the Kernel reports
`already-committed`. That is the "impossible before ARVES" the platform exists for.

Capabilities proven: **Reasoning · Planning · Capability selection · Execution · Memory ·
Truth Update** — and it runs on the *real* Kernel, not a stand-in.

## Run

```
cargo build -p arves-bridge --bin arves-bridge   # once: the platform bridge (P2)
node examples/agent-run.mjs                       # the agent commits its trace to the real Kernel
```

The demo (proves all of the above, exits 0):
- three systems collapse to one truth (Cognitive Memory, P1);
- the agent reasons, plans 3 steps, selects a capability per step, executes each;
- **every step is committed as truth in the real Rust Kernel** with one-world identity
  (SDK-local id == Kernel id);
- a second run yields identical ids and the Kernel reports `already-committed` (replay /
  ORCH-004 idempotency).

## Architecture (IDR-006 — consumes the platform, modifies nothing)

```
@arves/agent-runtime (P3)
  → @arves/cognitive-memory (P1)   truths / memory
  → @arves/sdk (P0)                content addressing
  → arves-bridge (P2)              → real Kernel  (Truth Update, idempotent)
```

Reasoning, planning, and capability selection are product-layer agent logic; the
capabilities are deterministic (so execution replays). The *truth* — every committed
step — lives in the real Kernel under its ACS-001 address. Next: extend the bridge to the
**Engine / Capability-fabric** layers so capability execution runs inside the runtime,
and P4 Personal AI, which turns this loop loose on Mail/Calendar/Slack/… data.

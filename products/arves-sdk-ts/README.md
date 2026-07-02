# @arves/sdk — ARVES Developer SDK (TypeScript)

**Build deterministic, content-addressed, replayable cognitive apps — in a few lines.**
This is **P1** of the ARVES Product Program: the developer's entry point to the platform.

> IDR-006: this SDK is a *product*. It consumes the ARVES standard
> (`arves-standard-kit 0.2.0`) and modifies no platform file. It is itself verified
> ACS-conformant (`npm run check` reproduces the Kit's published golden ContentIds).

## Why it matters

One primitive — the **content address** — gives you four properties an ordinary
datastore/framework does not, for free:

| Property | What you get | Demo |
|----------|--------------|------|
| **Deterministic identity** | the same value → the same address, regardless of key order, machine, or language | `equal? true` |
| **Idempotency** | committing the "same" fact twice deduplicates automatically | `store size: 1` |
| **Integrity** | any change → a different address; tampering is self-evident | `differs? true` |
| **Exact truth** | 64-bit-exact integers; a nanosecond timestamp never silently corrupts | float path `…768` ≠ exact `…789` |
| **Replay** | a decision trace is content-addressed; recomputation is reproducible | `identical? true` |

## Quick start

```js
import { Arves, FactStore } from '@arves/sdk';

const arves = new Arves();
const store = new FactStore();

// Field order doesn't matter — identity is the content address.
const id = store.commit({
  type: 'uci.fact',
  claim: 'sky-is-blue',
  confidence: arves.float(0.5),   // floats are explicit (distinct ACS kind)
  observed_at: 1730000000000000000n,  // integers are BigInt — exact, never lossy
});
// id === '12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e'
// Commit it again with keys reordered → same id → store.size stays 1.

arves.verify(fact, id);      // integrity check
arves.traceRoot([id, id2]);  // reproducible decision-trace root
```

Two deliberate ergonomics enforce ACS correctness so you can't create silent bugs:
integers **must** be `BigInt` and floats **must** be `arves.float(x)` — a bare `number`
throws, because it is ambiguous (int vs float) and lossy beyond 2⁵³ (ACS-002 §5.2/§5.3).

## Run

```
node src/check.mjs          # standard-conformance self-check (4/4)
node examples/fact-store.mjs # the "why ARVES matters" demo
```

Zero third-party dependencies; SHA-256 via Node's built-in `node:crypto`; the dCBOR
profile is implemented directly from ACS-002. This is the smallest slab of the platform
a product needs — the higher products (Agent Runtime, Personal AI, Enterprise AI) build
on this same content-addressed substrate.

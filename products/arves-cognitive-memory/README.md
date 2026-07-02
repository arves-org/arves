# @arves/cognitive-memory — Cognitive Memory (P2)

**Three systems. One truth. Full audit. Exact replay. Provable reasoning.**

Cognitive Memory is the flagship of the ARVES Cognitive Application Platform and the
**common core** of everything above it — Personal AI, Enterprise AI, and every industry
solution build on this. It answers the only question that matters now: *what can you do
with ARVES that was impossible before?*

## Impossible before → Possible with ARVES

Wire three systems that describe the same reality in three different schemas — an email
invite, a calendar event, a CRM activity — into Cognitive Memory. Because **identity is
the content address**, they collapse to a single truth automatically. You get, for free:

| Capability | What you see in the demo |
|-----------|--------------------------|
| **Identity** | three schemas → one identical `ContentId` |
| **Deduplication** | three sources → one truth in memory |
| **Evidence** | that truth is attested by 3 independent systems (provenance) |
| **Truth** | the canonical fact is exact — a nanosecond timestamp never drifts |
| **Audit** | an append-only, tamper-evident chain (alter any past event → the head changes) |
| **Replay** | the whole memory state is a deterministic content address — re-ingest → identical |
| **Reasoning** | a conclusion is content-addressed over its supporting truths — reproducible and defensible |

It also **surfaces conflict** instead of hiding it: if the CRM disagrees on the time, you
get two distinct truths, not a silent fuzzy merge.

> **Scope caveat (honest).** This product is an **in-memory reference substrate**: a JS `Map`
> plus a co-located hash chain. It does not use the Kernel bridge and has **no WAL, no durable
> persistence, and no crash recovery** — all state is lost on process exit.
> - **"Replay"** here means re-ingesting the same observations recomputes the same content
>   address (deterministic recomputation), not replay from a durable log.
> - **"Tamper-evident"** requires an **externally-trusted head.** `verifyChain()` detects a
>   change to any *past* entry only relative to a head the verifier already trusts; an attacker
>   who rewrites the whole log *and* its head produces a chain that verifies clean. The head is
>   stored next to the log here, so this is integrity, not attestation. Real tamper-evidence
>   needs the head anchored outside this process — e.g. committed to the real Kernel through
>   `arves-sdk-ts/src/bridge.mjs`, or to another append-only authority.
>
> For durable, cross-process, WAL-backed truth, commit through the bridge to the real Kernel.

## Quick start

> **Repo-local preview.** This package is `private` and unpublished (`0.1.0-preview`, no npm
> registry, no `exports` map), so a bare `@arves/cognitive-memory` specifier does **not**
> resolve (`ERR_MODULE_NOT_FOUND`). Until it is published, import from the **relative source
> paths** below — exactly as `examples/three-systems.mjs` does. `@arves/cognitive-memory` is
> the intended published name, not a working import today.

```js
import { CognitiveMemory } from './src/memory.mjs';
import { allSources } from './src/connectors.mjs';

const memory = new CognitiveMemory();
for (const observation of allSources()) memory.ingest(observation);

memory.truths();                       // one deduplicated truth, with its 3 sources
memory.root();                         // deterministic address of the whole state (replay)
memory.reason('Ada attended Q3 Review', [truthId]); // reproducible, evidence-backed conclusion
```

```
node examples/three-systems.mjs   # the flagship demo (proves all six capabilities, exits 0)
```

## How it is built (the five product rules)

- **Platform-first:** consumes `@arves/sdk` (P1) → `arves-standard-kit 0.2.0`; modifies no
  platform file (IDR-006).
- **Value-first:** a working demo, runnable today.
- **Evidence-first:** the demo *asserts* each of the six capabilities — the product is
  simultaneously a proof of ARVES.
- **Developer-first:** ingest + reason in a handful of lines.
- **Production-first:** connectors carry real observation shapes (only the transport is
  stubbed); the memory, audit chain, and reasoning are written to become the real product.

Next: **P3 Agent Runtime** turns these truths into reasoning, planning, and actions.

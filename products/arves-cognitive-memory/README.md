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

## Quick start

```js
import { CognitiveMemory } from '@arves/cognitive-memory';
import { allSources } from '@arves/cognitive-memory/connectors';

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

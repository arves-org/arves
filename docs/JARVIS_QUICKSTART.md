# JARVIS Quickstart — run YOUR assistant on YOUR machine (A8)

> **Who this is for:** the maintainer (or anyone with this repo). In ~10 minutes you run
> the JARVIS phase-1 assistant on your own shard and WAL directory, exercise every
> acceptance criterion A1–A7 yourself, and — if you want the intelligence — plug your
> own LLM into the Reasoner slot **without adding a single line to this repo**.
>
> **Honesty up front:** everything the repo ships is deterministic and offline. The
> in-repo reasoner is a keyword-table STUB (NOT AI) and the sub-agents are rule-based
> actors. The intelligence is *your* LLM, attached at *your* runtime. Scope is v1.0:
> single host, no authN on commit (v2.0 debt #8) — right-sized for a personal assistant
> on your own machine, stated, not hidden.

## 0. Prerequisites

- Node.js >= 18 (no third-party packages — the product has zero dependencies)
- Rust toolchain (one-time, to build the frozen reference bridge)

## 1. Build the bridge once (the frozen Runtime v1.0 Kernel endpoint)

```sh
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml
```

That produces `runtime/target/debug/arves-bridge(.exe)` — the reference Kernel behind
the line protocol (`id=` RCR-011 · `shard=` RCR-014 · `--wal-dir` RCR-015 · `bind`
RCR-016). The product only ever talks to this; it never links runtime code (IDR-006).

## 2. Prove A1–A7 to yourself (offline, deterministic)

```sh
node products/arves-assistant/assistant.test.mjs        # stage 1: memory core (7/7)
node products/arves-assistant/skills.test.mjs           # stage 2: skills/reasoner/guardrails (6/6)
node products/arves-assistant/agents.test.mjs           # stage 3: agents/why (6/6)
node products/arves-assistant/examples/jarvis-day.mjs   # THE CAPSTONE: A1–A7 in one day (17/17)
```

The capstone runs a full assistant day — 3-source observe → two sub-agents (with a
first-committed-wins conflict) → stub-reasoner think → guardrail block → separate
approval → certified skills act → **kill the Kernel, restart over the same WAL** →
memory intact → `why()` explains the spend decision end to end, byte-identically to the
pre-restart explanation.

## 3. Run the assistant on YOUR OWN shard + WAL dir

Your truths live where you say they live and survive every restart. Minimal script
(save anywhere, e.g. `my-jarvis.mjs` next to the repo):

```js
import { Assistant } from './products/arves-assistant/src/assistant.mjs';
import { registerSkill, defineCapability } from './products/arves-assistant/src/skills.mjs';
import { StubReasoner } from './products/arves-assistant/src/reasoner.mjs';
import { why, renderWhy } from './products/arves-assistant/src/why.mjs';

const assistant = new Assistant({
  tenant: 'hakan',              // your shard (RCR-014): tenant/workspace
  workspace: 'jarvis',
  walDir: 'C:/arves-jarvis-wal', // your durable memory (RCR-015) — pick any dir you own
});
try {
  assistant.useReasoner(new StubReasoner());   // swap for your LLM reasoner (step 4)

  // Observe your world (source name = evidence, never identity):
  await assistant.observe('me', { entity: 'urn:you', event: 'dentist-appointment', at: 1_752_000_000_000n });

  // Register a skill (certification is RE-RUN here — forged flags are refused):
  await registerSkill(assistant, defineCapability({
    name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
    execute: (input) => [{
      target: 'uci.assistant.briefing',
      value: { type: 'uci.assistant.briefing', count: BigInt(input.events.length), events: [...input.events].sort() },
    }],
  }), [{ type: 'uci.assistant.skill-input', events: ['a'] }]);

  // Guardrail: spend-class actions need YOUR separate committed approval truth:
  await assistant.guardrails.setPolicy({ name: 'spend-needs-me', appliesTo: ['spend', 'irreversible'], approverRole: 'user' });

  const result = await assistant.think('summarize my day');
  console.log(result.acted ? renderWhy(why(assistant, result.invocation.truths[0].id)) : result);
} finally {
  assistant.close(); // ALWAYS — the bridge is a child process
}
```

Restart the script tomorrow with the SAME `walDir`: the fresh process honestly remembers
nothing until you re-run your deterministic day (`assistant.rebuild({...})` or the same
script) — then every body answers `already-committed`, which IS the Kernel's proof your
memory survived. (The bridge has no WAL-scan verb yet; that is a recorded RCR candidate,
see `products/arves-assistant/README.md`.)

## 4. Plug YOUR LLM into the Reasoner slot (the only line that changes)

The Reasoner interface is the exact contract (`products/arves-assistant/src/reasoner.mjs`
documents it inline):

```
interface Reasoner {
  name:    string                 // committed into every proposal truth (attribution)
  version: string
  reason(context) -> proposal     // sync or async; MUST NOT commit truth itself
}
context  = { goal, truths, decisions, skills }   // read-only projections of committed truth
proposal = { action: 'invoke-skill', skill, input, subject, actionClass, because }
         | { action: 'none', because }
```

Wiring example — this file lives in **your** project, with **your** API key in **your**
environment, never in this repo:

```js
// my-llm-reasoner.mjs  (YOURS — outside the repo)
import Anthropic from '@anthropic-ai/sdk';                    // or any model SDK you choose
const client = new Anthropic();                                // key from YOUR env

export class LlmReasoner {
  name = 'my-llm-reasoner'; version = '1.0.0';
  async reason(context) {
    const msg = await client.messages.create({
      model: '<your-model-id>', max_tokens: 512,   // your model choice (brief OQ-3: irrelevant to the repo)
      messages: [{
        role: 'user',
        content: `Goal: ${context.goal}\nKnown truths: ${JSON.stringify(context.truths.map(t => t.fact.event))}\n`
          + `Registered skills: ${context.skills.join(', ')}\n`
          + `Answer ONLY with JSON: {"action":"invoke-skill","skill":...,"input":...,"subject":...,"actionClass":...,"because":...} `
          + `or {"action":"none","because":...}. Only name a registered skill.`,
      }],
    });
    return JSON.parse(msg.content[0].text);   // add your own validation/repair as you like
  }
}
```

```js
// in your runner script — the ONLY line that changes:
import { LlmReasoner } from './my-llm-reasoner.mjs';
assistant.useReasoner(new LlmReasoner());
```

**What stays true with a real LLM attached — this is the product's claim:**

- Governance is OUTSIDE the reasoner. Whatever your model proposes, the proposal is
  committed as attributed truth, guardrail policies are checked BEFORE any skill runs,
  only certified+bound skills can act, and refusals are committed compliance truths. A
  hallucinating model cannot bypass a policy or run uncertified code.
- Your LLM is naturally non-deterministic — fine: its proposal is committed ONCE as
  content-addressed truth and replay reads the record; it never re-calls the model (the
  recorded-truth doctrine, `products/arves-ecosystem-sdk/REASONING.md`).
- `why()` still explains every decision from committed truth — including which reasoner
  (yours, by name/version) proposed it.

## 5. Exercise A1–A7 on your own data

| Criterion | Do this | You should see |
|---|---|---|
| A1 durable memory | run your script, kill it, run again with the same `walDir` | every re-proved body `already-committed`; contradiction checks cite the same prior ids |
| A2 multi-source one-truth | `observe('email', fact)` + `observe('calendar', sameFact)` | one ContentId, two sources in the evidence set |
| A3 certified skills | staple `certified: true` on an uncertified capability and register it | refused — certification is re-run, the flag is never consulted |
| A4 reasoner slot | swap `StubReasoner` for your `LlmReasoner` | same pipeline; proposals now carry your reasoner's name |
| A5 sub-agents | run `ResearcherAgent`/`SchedulerAgent` over one assistant; make them disagree on a subject | first-committed-wins; the loser's resolution truth references the winner |
| A6 guardrails | `think('order …')` without an approval | blocked + committed compliance truth; `approve('user', subject)` unlocks |
| A7 explain yourself | `renderWhy(why(assistant, effectIdOrSubject))` | the full decision path, every station a checkable ContentId |

## What stays honest (read before you demo this to anyone)

- **Stub vs real LLM:** the repo's reasoner and agents are deterministic rule tables so
  tests replay byte-identically. Intelligence = your model, your key, your machine.
- **Attribution is product-level:** agent/reasoner tags live IN the committed bodies.
  The runtime's Rust I5 attribution is not yet exposed over the bridge (recorded RCR
  candidate) — and with no authN in v1.0, tags are structural, not cryptographic.
- **Single host, no authN, no TLS:** this is a personal assistant on your machine, not
  a deployed service. GA remains gated on the four conditions (IDR-006) — this is a
  **G1 preview** and says so.
- **A probe is a probe:** skill certification is a re-run check over your test inputs,
  not a purity proof (engine-enforced determinism is recorded v1.1 RCR debt).

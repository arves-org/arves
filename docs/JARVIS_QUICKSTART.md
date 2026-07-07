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
node products/arves-assistant/agents.test.mjs           # stage 3: agents/why (7/7)
node products/arves-assistant/cli.test.mjs              # the CLI/REPL, incl. cross-process durability (3/3)
node products/arves-assistant/examples/jarvis-day.mjs   # THE CAPSTONE: A1–A7 in one day (22/22)
```

(Or just `npm --prefix products/arves-assistant test` for the four test files.)

The capstone runs a full assistant day — 3-source observe → two sub-agents (with a
first-committed-wins conflict) → stub-reasoner think → guardrail block → separate
approval → certified skills act → **kill the Kernel, restart over the same WAL** →
memory intact → `why()` explains the spend decision end to end, byte-identically to the
pre-restart explanation.

## 2a. See it — open the JARVIS console in your browser (no CLI)

If you would rather **see** JARVIS than type at it, run the local console. One command,
zero extra dependencies (Node's built-in `http` server), the same real bridge and the same
governed pipeline — just visual:

```sh
JW=./my-jarvis-wal                                            # your durable memory (any dir you own)
node products/arves-assistant/ui/server.mjs --wal-dir $JW     # then open http://localhost:7777
#   …or:  npm --prefix products/arves-assistant run ui        # (uses ./jarvis-wal)
#   …or a different port:  node products/arves-assistant/ui/server.mjs --wal-dir $JW --port 8080
```

Open **http://localhost:7777** and you have the whole assistant in one place:

- **Console** — ask JARVIS a goal and watch the trace unfold *warm thought → gate
  (pass/block) → cool committed truth*, every commit a clickable ContentId.
- **Memory** — the deduplicated truths, each with its evidence sources. Click **Observe**
  to pull a connector (`calendar`, `ical`, `email`, …) into truth.
- **Guardrails** — your policies as committed truths, and any action **awaiting approval**;
  click *Approve as \<role\>* to commit the separate approval truth that clears the gate.
- **Skills & Agents** — the certified capabilities and the deterministic sub-agents.
- **Why** — reconstruct any decision's path from committed truth.
- **Settings** — the reasoner pill tells the truth: it shows **Stub (deterministic)** until
  you attach a model. The console can never claim intelligence it doesn't have.

Everything you see is a **read projection of committed truth** in your WAL — stop the
server, start it again over the same `--wal-dir`, and every truth, decision, policy and
block comes back (RCR-033 recovery). The browser is only a window; the truth is in the Kernel.

**Plug a real model — the fastest path is OpenAI, already shipped.** Set your key in the
environment (never in the repo) and the server auto-attaches the built-in OpenAI reasoner:

```sh
OPENAI_API_KEY=sk-...  node products/arves-assistant/ui/server.mjs --wal-dir $JW
#   choose the model:   OPENAI_API_KEY=sk-...  OPENAI_MODEL=gpt-4o  node .../ui/server.mjs --wal-dir $JW
```

The Settings pill flips to `openai:<model>` (isStub=false) and every proposal it makes is
committed as attributed, governed, replayable truth. Or point the server at **any** reasoner
module (same slot as §4) — its default export must implement the `Reasoner` interface:

```sh
JARVIS_REASONER=./my-llm-reasoner.mjs node products/arves-assistant/ui/server.mjs --wal-dir $JW
```

Governance, memory and audit are unchanged — only the intelligence became yours. And it is
honestly governed BOTH ways: if the model proposes an unregistered skill it is refused; if it
supplies input a skill can't run on, that failure is committed as truth and shown plainly —
never a crash.

> **Honesty (same as everywhere):** single host, `localhost` only, no authN/TLS on the
> commit path (v1.0 scope — a personal assistant on your own machine, not a deployed
> service). This is the **G1 preview**; the four-condition GA gate (IDR-006) is unmet and
> the UI says so.

## 2b. Drive it from the CLI (interactive REPL or scripted)

The fastest way to *use* JARVIS is the CLI. It runs every command over a real
`KernelBridge` on your own shard + WAL directory — one-shot or as an interactive REPL:

```sh
JW=./my-jarvis-wal                                        # your durable memory (any dir you own)

# one-shot commands:
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW import ical
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW recall
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW ask summarize my day
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW status

# …or an interactive REPL (also scriptable via piped stdin):
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW
```

Commands: `observe <source> <entity> <event> <iso-utc>` · `import <connector> [file]` ·
`ask <goal>` · `recall [entity]` · `why <subject|id>` · `approve <role> <subject>` ·
`policy <name> <role> <class…>` · `skills` · `decisions` · `status` · `help`. Flags:
`--tenant` / `--workspace` (your shard, RCR-014) · `--wal-dir` (durable memory, RCR-015).

A scripted **govern-a-spend** session (pipe it straight into the REPL):

```sh
printf '%s\n' \
  'import ical' \
  'ask order flowers' \
  'approve user spend:order-flowers' \
  'ask order flowers' \
  'why spend:order-flowers' \
  'exit' | node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW
```

You will see the spend **BLOCKED** (a committed compliance truth), the separate `approve`
truth, then the action **ACTED** citing that approval, and finally `why()` reconstructing
the whole path — every station a checkable ContentId.

**Durability is real and cross-process.** Because each fresh CLI process rebuilds its
memory read-only from the WAL (the RCR-033 `scan` verb, `Assistant.recoverFromWal()`), a
*brand-new* process explains a decision an earlier one made:

```sh
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW recall                 # sees prior truth
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW why spend:order-flowers # explains it
```

The approval you granted earlier is rehydrated from the WAL too, so the gate stays open
across restarts — durable governance, not session state. (Honest residual, same as the
capstone: a *fresh-scan* `why` on an effect-bearing subject rebuilds every self-describing
station but not the effect→skill edge, which is process metadata until a native
attributed-invoke verb lands — a recorded RCR candidate. A live session keeps that edge.)

## 2c. Point the connectors at YOUR real files

`import <connector>` reads offline, deterministic fixtures by default, but the real-format
connectors parse formats you already keep — pass your own file:

```sh
# an Obsidian / Logseq daily note (headings set the date; `- HH:MM <event> [@entity]` bullets become facts):
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW import journal /path/to/2026-07-06.md

# a Google/Apple Calendar export (each VEVENT's SUMMARY + UTC DTSTART; optional X-ARVES-ENTITY):
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW import ical /path/to/calendar.ics

# an email message TEMPLATE (.eml — From→entity, Subject→event, Date→instant):
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW import email /path/to/message.eml

# a generic three-column CSV, or JSON Lines (one object per line):
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW import csv   /path/to/events.csv
node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW import jsonl /path/to/events.jsonl
```

The **same real-world event from different sources collapses to ONE truth** whose evidence
set names every source that saw it (A2) — the dentist appointment in your markdown journal,
your `.ics`, and a hand-typed `observe` all address to the *same* ContentId; the source name
is evidence, never identity. Connectors are pure functions of their file — no clock, no
network — so the same file always yields the same truths. To wire a **new** real source,
author a function returning `[{ source, fact: { entity, event, at } }]` (`at` = BigInt ms
UTC) exactly like `markdownJournalConnector` / `icalConnector` in
`products/arves-assistant/src/connectors.mjs`, and register it in that module's `CONNECTORS`
map. Scope stays honest, and honesty here means **failing loud rather than guessing**:

- The instant-based connectors (`journal`, `ical`, `csv`, `jsonl`) accept only UTC instants
  ending in `Z`; a floating/timezone time is rejected, not silently reinterpreted.
- The `email` connector's RFC 5322 `Date` header **must carry an explicit zone/offset**
  (`+0000`, `GMT`, `Z`, …). A *zoneless* date would be parsed as your machine's LOCAL time —
  the same `.eml` would then commit a different instant on a different-timezone host — so a
  zoneless date fails loud instead of skewing. RFC-compliant mail always carries a zone.
- The `csv` connector tolerates ONE leading header row, but only when line 1's first cell
  isn't itself a date-ish-but-invalid timestamp; a single-character typo in a line-1 ISO
  instant fails loud rather than being silently dropped as a "header".

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

Restart the script tomorrow with the SAME `walDir`: a fresh process rebuilds its memory
**read-only from the WAL** — `await assistant.recoverFromWal()` enumerates the shard's
committed truth via the RCR-033 bridge `scan` verb (the Kernel replays its WAL through the
Query layer) and rebuilds the memory + decision journal with **zero re-commits**. (The
older `assistant.rebuild({…})` membership-proof path — re-deriving candidate bodies and
reading every `already-committed` answer as proof — is retained too; it also re-establishes
the one thing a pure scan cannot, the effect→skill journal edge. Both are documented in
`products/arves-assistant/src/assistant.mjs`.) The CLI (§2b) uses `recoverFromWal()` on
every command, which is why a fresh `recall`/`why` process just works.

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

**Don't want to write the adapter? OpenAI is shipped.** `src/openai-reasoner.mjs` is a
complete, **zero-dependency** (global `fetch`, no SDK) OpenAI-backed reasoner that reads the
key from `process.env.OPENAI_API_KEY` — the key never touches the repo. It reuses the same
governed `parseProposal` (hallucinated skills refused), so nothing about governance changes:

```js
import OpenAiReasoner from './products/arves-assistant/src/openai-reasoner.mjs';
assistant.useReasoner(new OpenAiReasoner());   // model from OPENAI_MODEL (default gpt-4o-mini)
```

```sh
# run it (the CLI and the UI both auto-attach it when OPENAI_API_KEY is present):
OPENAI_API_KEY=sk-...  node products/arves-assistant/bin/jarvis.mjs --wal-dir $JW ask summarize my day
OPENAI_API_KEY=sk-...  node products/arves-assistant/openai-reasoner.test.mjs   # a live governed-pipeline proof
```

The live test **self-skips with no key**, so CI stays offline-hermetic; it makes a real call
only when you provide a key. (Your OpenAI key is billed by OpenAI, not ARVES.)

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
- **Attribution is product-level:** agent/reasoner tags live IN the committed bodies. The
  runtime's I5 attribution IS reachable over the bridge (the `commit-as` verb, RCR-034), but
  this product still uses product-level in-body tags; either way, with no authN in v1.0, a
  tag is structural, not cryptographic — any local caller could wear any tag.
- **Single host, no authN, no TLS:** this is a personal assistant on your machine, not
  a deployed service. GA remains gated on the four conditions (IDR-006) — this is a
  **G1 preview** and says so.
- **A probe is a probe:** skill certification is a re-run check over your test inputs,
  not a purity proof (engine-enforced determinism is recorded v1.1 RCR debt).

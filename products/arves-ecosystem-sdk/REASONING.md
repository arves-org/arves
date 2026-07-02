# ARVES AI Capability SDK — reasoning capabilities

> The AI-Operating-System thesis, made concrete: **the LLM is a swappable Capability/Provider.
> The runtime never changes. The model's output becomes recorded, replayable truth.**

`src/reasoning.mjs` is the AI layer of the Ecosystem Kit. It lets a third party author a
capability whose effects are produced by a *reasoning provider* (a rule engine, an on-device
model, or a remote LLM) and have that capability flow through the **unchanged** ARVES trust
boundary — `certify → package → install → invoke → commit` — exactly like any other
capability. A reasoning capability is not a special citizen: it returns `{ manifest, execute }`
with the same shape as `defineCapability()`, so the frozen Runtime v1.0 commits its bytes
without ever knowing a language model produced them.

---

## The abstraction

```js
import { Providers, defineReasoningCapability } from './src/reasoning.mjs';

const cap = defineReasoningCapability({
  name: 'reasoning.sentiment',
  version: '1.0.0',
  produces: ['uci.reasoning.verdict'],
  provider: Providers.reference,          // swap this line to swap the "model"
  reason: (input) => Providers.reference.reason(input.text), // optional explicit reasoner
});
// cap === { manifest, execute } — identical shape to defineCapability()
```

- `defineReasoningCapability({ name, version, produces, provider, reason })` returns a value
  with the **same shape** as `defineCapability()` (it is built on top of it).
- `execute(input)` calls `(reason || provider.reason)(input)` and normalizes the result into
  an array of effects `[{ target, value }]`. The reasoner may return a bare ARVES value (wrapped
  as one effect on `produces[0]`), an array of values, an effect, or an array of effects.
- `value` is any **ARVES value** — `null · boolean · BigInt · float(x) · string · Uint8Array ·
  Array · plain object` (bare JS numbers are rejected; see the SDK value model).
- When the provider is deterministic, the capability is **certifiable by the existing
  `certifyCapability()`** with no changes to the certifier.

## The provider table

A provider is `{ name, reason(input) }`. `reason` returns an ARVES value (or effects).

| Provider   | Kind                     | Offline? | Deterministic? | Certifies / replays in-repo? |
|------------|--------------------------|:--------:|:--------------:|:----------------------------:|
| `reference`| rule-based reference reasoner | yes | **yes** (pure) | **yes** |
| `local`    | on-device / self-hosted stand-in | yes | **yes** (pure) | **yes** |
| `claude`   | remote LLM adapter **stub** | — | — | integration point |
| `gpt`      | remote LLM adapter **stub** | — | — | integration point |
| `gemini`   | remote LLM adapter **stub** | — | — | integration point |

- **`reference` / `local`** are pure functions of their input — no clock, no RNG, no I/O — so a
  capability built on them produces the **same effect address for the same input**. That is the
  property `certifyCapability()`'s `deterministic` check **probes** for (a best-effort run-twice
  comparison over the supplied inputs, not full enforcement — see the SDK README), which is why
  reasoning capabilities certify and replay **fully offline** (no network, no API key).
- **`claude` / `gpt` / `gemini`** are documented **integration stubs**. Each is the single,
  named place an operator wires an API adapter + key, **out of repo**. Until then, calling
  `.reason(input)` throws:

  ```
  provider "claude" requires integration: supply an API adapter + key
  ```

  This is deliberate: **nothing in this repository performs a network call or holds a secret.**
  Wiring a remote provider is an operator action at deployment time, not a repo change.

---

## The doctrine — recorded truth, not re-calling (ORCH-003 · ACS-005 GL-012)

This is the whole moat, and it is one sentence:

> **A provider's output is committed exactly once as content-addressed truth; replay reads
> the recorded trace by its ContentId — it NEVER re-calls the provider.**

```
author time :  provider.reason(input)  →  value  →  commit  →  ContentId   (ONCE)
replay time :  ContentId               →  recorded value                   (NEVER re-calls)
```

- At **author / invoke time**, the reasoner runs once. Its output value is committed as an ACS
  fact and becomes a **ContentId** in the ledger. From that instant, the reasoning *is* an
  address — immutable, auditable, deduplicated by content (`ORCH-003`).
- At **replay time**, the system reads the recorded value by its ContentId. The provider is
  **not** invoked again. A non-deterministic LLM therefore produces a **deterministic ARVES
  fact**: the record is stable even though a second call to the model would not be
  (`ACS-005 GL-012`).

### Why this beats a wrapper

A "chatbot wrapper" calls the model on **every** run and hopes the answer is stable — it is
not. So a wrapper cannot:

- **replay** — re-running yields a different answer, so history is not reproducible;
- **audit** — there is no fixed artifact to point an auditor at;
- **deduplicate** — the same question produces different bytes each time;
- **swap models safely** — changing the model silently changes past behavior.

ARVES records the reasoning **once** and addresses it by content. Swapping `Providers.reference`
for `Providers.claude` changes **who reasons next** — it never changes the runtime, the trust
boundary, or a single fact already committed. That is what makes the LLM a swappable component
of an operating system rather than the system itself.

---

## Wire a real provider (`claude` / `gpt` / `gemini`) — the worked path to a certified capability

`arves create <name> --provider claude` scaffolds a capability whose provider **throws until
you integrate it** — that is the honest default (no repo secret, no network). This section is
the missing how-to: the *worked* path from a live model adapter to a **certified** capability,
using only the existing `defineReasoningCapability` + `certifyCapability` — **no new framework.**

The trick is the doctrine above: a provider is called **once**, and its output becomes a
recorded ARVES fact. So certification does not run your live model — it runs a `reason(input)`
that returns the **already-obtained** completion for each pinned test input. You supply the
adapter at deploy time; you supply the recorded completions to the certifier.

**Step 1 — an adapter (out of repo; your key, your HTTP client).** This is the only place a
network call lives. It is NOT imported by anything in this repo.

```js
// my-claude-adapter.mjs  (operator-owned; not committed; needs your key)
export const claude = {
  name: 'claude',
  async complete(prompt) {
    // ... your real call to the Claude Messages API (POST /v1/messages) here, returning a
    // string ... (This repo performs no network I/O and holds no secret; this file is yours.)
  },
};
```

> Model/API reference (model ids, params, the Messages API request shape) is out of scope for
> this repo's docs. Use the current Anthropic SDK (`@anthropic-ai/sdk` for Node) when you write
> `complete()`. The ARVES contract only cares about the **recorded output**, never how you
> obtained it.

**Step 2 — record the completion once (author time).** Call the adapter for each representative
input and pin the result. This is the "commit ONCE" of the doctrine, done at authoring:

```js
import { claude } from './my-claude-adapter.mjs';

export const testInputs = [{ prompt: 'summarize the incident: disk full at 02:14 UTC' }];
// Obtain each completion ONCE, then pin it. (Run this once; save `recorded` to a file/const.)
const recorded = new Map();
for (const inp of testInputs) recorded.set(inp.prompt, await claude.complete(inp.prompt));
```

**Step 3 — a DETERMINISTIC `reason` that returns the recorded value.** Certification requires
determinism (same input → same effect address). A live LLM is not deterministic, so you do
**not** hand the live adapter to the certifier — you hand it the recorded completion:

```js
import { defineReasoningCapability } from './src/reasoning.mjs';

export const capability = defineReasoningCapability({
  name: 'incident.summary', version: '1.0.0', produces: ['uci.reasoning.verdict'],
  // Deterministic: a pure lookup of the completion recorded in step 2. The model ran ONCE;
  // this fact is now content-addressed truth and will certify + replay offline forever.
  reason: (input) => ({
    type: 'uci.reasoning.verdict', provider: 'claude', summary: String(recorded.get(input.prompt)),
  }),
});
```

**Step 4 — certify + package + install through the UNCHANGED path.**

```
node bin/arves.mjs certify incident.summary.capability.mjs   # → CERTIFIED (deterministic)
node bin/arves.mjs package incident.summary.capability.mjs   # → signed artifact id
```

Because the reasoner is now a pure function of the input, it passes `certifyCapability`'s
`deterministic` check and flows through `package → install → invoke → commit` exactly like the
`reference` provider — the runtime never learns a language model was involved. To refresh the
answer with a newer model, re-run step 2 and re-`package`: a **new** ContentId records the new
reasoning; the old fact is untouched (safe model swap, full audit of what each model said).

> **Scope caveat.** The `deterministic` check is a **best-effort run-twice probe over your
> pinned author inputs** (see the Ecosystem SDK README) — it proves *these* recorded
> completions are stable, not that a live adapter is. If you hand the certifier the live
> `provider.claude` instead of a recorded `reason`, it throws (integration stub) — which is the
> point: certification records reasoning, it does not call models.

## Try it (offline, no keys)

```
# certify the example reasoning capability (uses Providers.reference — deterministic)
node bin/arves.mjs certify examples/sentiment.reasoning.capability.mjs
#   certify reasoning.sentiment@1.0.0: CERTIFIED
#     ✓ manifest-valid  ✓ has-test-inputs  ✓ effects-declared
#     ✓ effects-acs-canonical  ✓ deterministic

# package it into a signed, content-addressed artifact
node bin/arves.mjs package examples/sentiment.reasoning.capability.mjs

# confirm a remote provider is an un-wired integration point (throws, by design)
node -e 'import("./src/reasoning.mjs").then(({Providers})=>Providers.claude.reason({text:"hi"}))'
#   Error: provider "claude" requires integration: supply an API adapter + key
```

The `reference` and `local` providers keep the whole loop green with zero external
dependencies. The `claude` / `gpt` / `gemini` stubs mark exactly where — and only where — an
operator adds a real model.

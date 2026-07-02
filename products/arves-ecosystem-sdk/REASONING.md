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
  capability built on them produces the **same effect address for the same input**. That is
  exactly the property `certifyCapability()`'s `deterministic` check enforces, which is why
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

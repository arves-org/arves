# Building Products on ARVES

### The Product Development Guide

> This is the canonical guide for engineers building products **on top of** ARVES.
> It is not a runtime specification, an SDK reference, or an API manual. It teaches
> the **ARVES way** of building an AI product: the mental models first, the code second.
>
> Read this once before you write a line of product code. It should still be true years from now.

---

## How to read this guide

Every section answers one question, and only this question:

> **“If I build my product on ARVES, what do I build — and what does ARVES already give me?”**

If you ever lose that thread, stop and come back to it. That question *is* the ARVES way.

We use one legend everywhere:

```
🟦  YOU build it        (your product — the part that makes it yours)
⬛  ARVES provides it   (the frozen runtime — the part every AI product needs)
```

We build one fictional product from zero across the whole guide: **Ava, a Personal AI.**
Ava is made up; every ARVES call is real. The shipping reference implementation you can
read alongside is **JARVIS** (`products/arves-assistant/`).

---

# Part I — Why ARVES Exists

## 1. You want to build an AI product

Let’s start where you actually start. Not with a Kernel. Not with “Truth.” With you, on day one:

> *“I want to build a Personal AI. It should know my world, do things for me, and I should be able to trust it.”*

Swap “Personal AI” for whatever you’re building — a **Company AI**, a **Medical AI**, a
**Travel AI**, a **Financial AI**, a **Developer AI**. The product is different. The
underneath is always the same.

You start strong. You wire an LLM to your data, add a few tools, and it demos beautifully.
Then reality arrives.

## 2. The tax every AI product eventually pays

Every serious AI product, no matter the domain, ends up needing the same infrastructure.
Nobody plans for it on day one. Everybody pays for it by month six.

```
The AI Product Tax
──────────────────────────────────────────────────────────────────────
Memory            It must remember your world — durably, across restarts.
Consistency       The same fact from two sources must be one fact, not two.
Governance        Some actions (spend money, send mail, delete data) need rules.
Policies          “This class of action requires approval” — as data, not code.
Audit             Every decision must be recorded. Regulators and users will ask.
Explainability     “Why did it do that?” must have a real, reconstructable answer.
Replay            You must be able to re-run history and get the same result.
Determinism       Same inputs → same outputs, or you can’t debug or certify anything.
Capability mgmt   Tools must be versioned, verified, and safe to run.
Certification     You must be able to prove a tool does what it claims.
──────────────────────────────────────────────────────────────────────
```

Here is the uncomfortable truth: **your users don’t buy any of these.** They buy your
product’s experience and its intelligence. But you can’t ship the experience without the
tax underneath it — so you build it. Badly, usually, because it isn’t your expertise, and
under deadline. Then you rebuild it. Every AI company on earth is quietly rebuilding this
same layer right now.

## 3. What ARVES is, in one sentence

> **ARVES is a frozen Cognitive Runtime that pays the entire AI Product Tax once, correctly, so your product never has to.**

You keep the part that’s yours — the experience, the intelligence, the domain. ARVES keeps
the part that’s everyone’s — memory, truth, governance, audit, replay, certification.

And the word **frozen** is not decoration. It’s the whole deal:

```
⬛  runtime/  +  standard/     →  FROZEN.  Byte-stable. You may NEVER modify them.
🟦  products/                 →  LIVING.  This is where your product lives.
```

ARVES is a runtime you *consume*, exactly like an operating system or a database engine.
You don’t patch the Linux kernel to ship an app; you don’t fork Postgres to store a row.
Same here. If the runtime is missing something you need, you don’t work around it inside
your product — you file a **Runtime Change Request (RCR)**, and the runtime evolves through
its own governed process (→ v1.1 minor, → v2.0 major). This separation is the single most
important idea in ARVES. The rest of the guide builds on it.

Why does frozen help *you*? Because a moving foundation is a tax of its own. When the ground
under your product never shifts, your product keeps working, keeps replaying, and keeps
certifying — for years. Stability is a feature you get for free by not being allowed to break it.

---

# Part II — The Mental Model

## 4. Four layers, one direction

Hold this picture in your head. Everything in ARVES is a consequence of it.

```
        ┌─────────────────────────────────────────┐
        │                 USER                     │   a person, an org, another system
        └─────────────────────┬───────────────────┘
                              │  intent  ("summarize my day", "pay this invoice")
        ┌─────────────────────▼───────────────────┐
   🟦   │              YOUR PRODUCT                 │   UI · skills · reasoner · connectors
        │                 (Ava)                    │   — the part that makes it *yours*
        └─────────────────────┬───────────────────┘
                              │  the Runtime API  (SDK + Bridge line protocol)
        ┌─────────────────────▼───────────────────┐
   ⬛   │             ARVES RUNTIME                 │   Kernel · Persistence · Engine ·
        │              (v1.0, frozen)              │   Capability · Governance · Replay
        └─────────────────────┬───────────────────┘
                              │  commits
        ┌─────────────────────▼───────────────────┐
   ⬛   │                 TRUTH                     │   content-addressed, append-only,
        │        (the durable record of reality)   │   deterministically replayable
        └─────────────────────────────────────────┘
```

Notice the direction. Intent flows **down**; truth settles at the **bottom**. Your product
never reaches past the Runtime API into the Kernel, and it never writes to Truth directly —
it *asks the runtime to commit*, and the runtime decides. That one-way street is what makes
everything above auditable and everything below trustworthy.

## 5. The great divide: what you own vs. what ARVES owns

This is the table to tape to your wall. When you’re unsure whether to build something, find
it here first.

```
🟦  YOUR PRODUCT OWNS                      ⬛  ARVES PROVIDES
─────────────────────────────────────     ─────────────────────────────────────
Product experience (UI / CLI)             Truth (content-addressed, immutable)
Business logic & domain rules             Memory (durable, restart-surviving)
Skills (what actions exist)               Policy-as-truth primitives — durable
Reasoners (how to decide / an LLM)          policies, approvals & compliance events
Connectors (how your data gets in)        Capability admission (certify + bind)
The governance gate — you compose it      Audit trail (every decision is truth)
  from ARVES primitives (see Stage 5)     Replay & determinism
Integrations, copy, design, voice         Content addressing & identity
                                          The truth substrate that why() reads
```

Read the divide as a rule: **if the thing on the right shows up in your product’s code, you
are rebuilding the runtime.** Stop. It’s already there. (The one nuance — the governance
*gate* on the left — you don’t rebuild either: you *compose* it from ARVES primitives, as
Stage 5 shows.)

The line is not arbitrary. It follows one principle:

> **Business logic belongs to the product. Infrastructure belongs to ARVES.**
> Your product should be *small*. The runtime should stay *generic*.

A Medical AI and a Travel AI share zero business logic — but they share 100% of the tax.
That shared 100% is exactly what ARVES froze so nobody rebuilds it.

## 6. The one concept to truly internalize: Truth

If you understand Truth, everything else in ARVES clicks. Four properties, and why each one
saves you:

**Truth is content-addressed.** Every fact gets an identity computed from its *content* — a
68-character `ContentId`. Commit the same fact twice and you get the same id and the answer
`already-committed`. You never worry about duplicate rows or primary keys again; identity is
a property of the data itself.

**Source is evidence, not identity.** This is the quiet genius. If your calendar *and* your
email both say “dentist, Tuesday 3pm,” that is **one** truth with **two** sources of evidence
— not two records to reconcile later.

```
Memory Flow — the same event, seen twice, is one truth

  calendar ──"dentist 3pm"──┐
                            ├──►  ⬛ ARVES  ──►  ┌─────────────────────────┐
  email ─────"dentist 3pm"──┘                   │  Truth  #a3f9…           │
                                                │  entity: you            │
                                                │  event:  dentist-3pm    │
                                                │  evidence: [calendar,   │
                                                │             email]      │
                                                └─────────────────────────┘
                                                   one fact · two witnesses
```

You did not write dedup logic. You did not pick a merge strategy. You called `observe()`
twice and ARVES gave you one fact with a growing evidence set. That’s the tax, paid.

**Truth is immutable and append-only.** You never *edit* a fact. Reality changed? You commit
a *new* truth; the old one stays, forever, as what was true then. This is why audit and
replay are possible at all — history is never overwritten.

**Truth is replayable.** Because every truth is content-addressed and the log is append-only,
ARVES can replay the entire history and arrive at the exact same state. Determinism isn’t a
promise you make; it’s a property you inherit.

> Mental shift: you are not building a database of mutable rows. You are **accumulating an
> immutable record of reality**, and ARVES is the ledger.

## 7. The cognitive flow (from the product’s point of view)

Here is the complete life of a single request, the way your product experiences it. Memorize
this shape — every ARVES product runs this exact loop.

```
   USER intent
      │
      ▼
 🟦 OBSERVE ───────►  ⬛ TRUTH        your connectors feed reality in; ARVES stores &
      │                              dedups it into durable, evidence-backed memory
      ▼
 🟦 REASON ─────────►  a PROPOSAL     your reasoner (stub or LLM) proposes ONE action,
      │                              or proposes nothing. It cannot act — only propose.
      ▼
 ⬛ POLICY (the gate) ──┬── block ──► ⬛ approval needed  (recorded as truth)
      │                └── pass
      ▼
 🟦 SKILL ──────────►  runs your code   only a certified + bound skill can reach this line
      │
      ▼
 ⬛ EFFECT ─────────►  ⬛ TRUTH        the result is committed as content-addressed truth
      │
      ▼
 ⬛ MEMORY            the effect is now part of the record, ready to be observed again
      │
      ▼
 ⬛ why() ───────────► the full path, reconstructed from truth, on demand
```

Read what the colors are telling you. The blue steps — **observe, reason, skill** — are
*yours*: your data, your intelligence, your actions. Everything black — **truth, the policy
gate, effect, memory, why** — is ARVES, working the same way for every product ever built on
it. You wrote three steps. You got a governed, audited, replayable, explainable cognitive
loop.

That’s the whole game. The rest of this guide is just building this loop, once, for Ava.

---

# Part III — Building Ava, from Zero

We’ll walk the product lifecycle end to end. Each stage extends the *same* product, and each
stage is framed the same way: **🟦 What you build · ⬛ What ARVES provides · Why the split.**

```
The Product Lifecycle

  Create ──► Connect ──► Observe ──► Create ──► Register ──► Govern ──►
  product     sources     reality    skills     skills       actions

        ──► Attach ──► Run the ──► Build the ──► Ship ──► Maintain ──► Evolve
            reasoner    loop        experience
```

## Stage 0 — Create the product

**🟦 What you build.** A new package under `products/`. That’s the entire footprint of “an
ARVES product.” Copy the reference layout from `products/arves-assistant/` and rename:

```
products/ava/
├── package.json        @arves/ava · private · "type":"module" · node >=18
│                       dependencies:{}  ·  platform.modifiesRuntime:false
├── src/                your logic (.mjs) — reaches the runtime ONLY via the bridge
├── bin/                a thin CLI entry
├── ui/                 (optional) a self-contained console + server
├── fixtures/           deterministic sample inputs
├── examples/           runnable *-day.mjs demos
├── *.test.mjs          test suites, at the package root
└── README.md
```

**⬛ What ARVES provides.** The entire runtime, pre-built and frozen, reachable over one
stable API. You depend on it like a pinned external package: `arves-standard-kit 0.3.1` plus
the reference runtime at tag `runtime-v1.0`. Your `package.json` literally declares
`platform.modifiesRuntime:false` — a promise checked in CI.

**Why the split.** Your product is a thin package of *your* decisions. The heavy, correctness-
critical machinery is a dependency, not your code. That’s why an ARVES product can be small
enough for one engineer to hold in their head.

> One-time setup: the runtime is real Rust. Build the bridge once so your product can talk to
> it: `cargo build --locked -p arves-bridge --manifest-path runtime/Cargo.toml`.

## Stage 1 — Connect your data sources

Ava is useless until she knows your world. A **connector** is your adapter from a real source
(a calendar, an inbox, a notes file, a Slack export) into ARVES observations.

**🟦 What you build.** A deterministic function that reads a source and returns observations
in ARVES’s shape:

```js
// A connector returns: [{ source, fact: { entity, event, at } }]
//   source = a label ("calendar-file")   — this becomes EVIDENCE, not identity
//   entity = who/what it's about          ("you", "person:sara", "proj:acme")
//   event  = what happened                ("dentist-appointment")
//   at     = when, as a BigInt of ms      (1751468400000n)  — never a bare number

export function calendarConnector(file) {
  return readSource(file, 'calendar-file');   // reuse the line-format reader
}
// register it: add `calendar: calendarConnector` to your CONNECTORS map
```

**⬛ What ARVES provides.** Nothing yet — connectors are pure product code. But ARVES defines
the *shape* they must produce, because that shape is what makes dedup, evidence, and replay
work downstream. The discipline is the gift.

**Why the split.** Only *you* know what a “meeting” means in your domain. ARVES doesn’t want
to know — it wants a clean, deterministic stream of `{entity, event, at}`. Keep domain quirks
in the connector; keep the runtime generic.

> **Determinism rule for connectors:** no `Date.now()`, no randomness, no host-local time.
> Timestamps must be explicit UTC instants. Why? Because a connector that reads the clock
> would produce a different truth on every run — and replay would break. Determinism starts
> at the front door.

## Stage 2 — Observe reality

Now feed the world in. This is where product code meets the runtime for the first time.

**🟦 What you build.** A loop that calls `observe`:

```js
import { Assistant } from './src/assistant.mjs';

const ava = new Assistant({ walDir: './ava-wal' });  // durable, WAL-backed memory
for (const o of calendarConnector('./fixtures/calendar.txt')) {
  const r = await ava.observe(o.source, o.fact);
  // r = { id, status, deduped, sources, ... }
}
```

**⬛ What ARVES provides.** Everything that makes that one call trustworthy:

```
observe(source, fact)
      │
      ▼  ⬛ canonicalize → content-address → commit to the real Kernel
      │  ⬛ verify SDK id == Kernel id  (the "one-world" check — else it throws)
      │  ⬛ if this fact already exists: dedup, and ADD source to its evidence set
      │  ⬛ append to the WAL  → durable across restarts, replayable forever
      ▼
   { id, status: 'committed' | 'already-committed', deduped, sources: [...] }
```

You wrote a `for` loop. You received durable, deduplicated, evidence-backed, replayable
memory. That is the AI Product Tax’s biggest line item — *memory* and *consistency* — gone in
one call.

**Why the split.** Memory is not your differentiator; it’s table stakes that’s brutally hard
to get right (dedup, durability, crash recovery, cross-source identity). ARVES did it once.
Your job was only to *point at the data*.

## Stage 3 — Create skills (what Ava can *do*)

Observing is knowing. Skills are *doing*. A **skill** is a capability: a named, versioned unit
of action with declared outputs and a risk class.

**🟦 What you build.** The action itself, with `defineCapability`:

```js
import { defineCapability } from './src/skills.mjs';

const pay = defineCapability({
  name: 'ava.pay', version: '1.0.0',
  produces: ['ava.payment'],        // every effect must target one of these
  actionClass: 'spend',             // ← risk class. Sits OUTSIDE the manifest on purpose.
  execute: (input) => [{            // your logic: input → effects
    target: 'ava.payment',
    value: { type: 'ava.payment', to: input.vendor, amount: input.amount, state: 'placed' },
  }],
});
```

**⬛ What ARVES provides.** The contract that makes a skill *safe to run later*: the manifest
is content-addressed, the `actionClass` is the authoritative risk label the governance gate
will key on, and the effect shape (`{target, value}`) is checked to be canonical truth.

**Why the split.** *What Ava can do* is the essence of your product — that’s 100% yours. *How
an action is verified, gated, and recorded* is infrastructure — that’s ARVES. You describe the
action; ARVES guarantees the guardrails around it.

> Why is `actionClass` outside the manifest? Because a capability’s content-addressed id is
> hashed from its manifest (name / version / produces) **plus** a `codeHash` of your `execute`
> source **plus** the test-inputs hash — and `actionClass` is deliberately kept out of all
> three (it sits *beside* the manifest, not inside it; `codeHash` covers only `execute`). So
> declaring or changing a risk class changes neither the tool’s id nor its certification, while
> a real code, name, `produces`, or version change correctly mints a new id. And critically:
> **risk is decided by the skill, never by whoever asked to run it** — so an untrusted reasoner
> (or a jailbroken LLM) cannot relabel its own `spend` action as `normal` to slip past the gate.

## Stage 4 — Register skills (certify + bind)

Defining a skill doesn’t make it runnable. Admission does. This is ARVES’s trust boundary, and
it’s deliberately strict.

**🟦 What you build.** One call, with representative test inputs:

```js
import { registerSkill } from './src/skills.mjs';

await registerSkill(ava, pay, [
  { vendor: 'acme', amount: 1200n },   // ≥1 representative input is required
]);
```

**⬛ What ARVES provides.** A four-step admission gate, none of which trusts you:

```
registerSkill(assistant, cap, testInputs)
   1. ⬛ RE-RUN certification over testInputs   (a forged "certified:true" means nothing)
   2. ⬛ commit an admission truth              (the tool's entry into memory, auditable)
   3. ⬛ bind the name in the runtime           (now "invoke ava.pay" resolves — not before)
   4. ⬛ attach — and re-run certification AGAIN (defense in depth)
```

Certification runs five checks: the manifest is valid, there’s at least one test input, every
effect targets a declared output, every effect value is canonical truth, and the skill is
deterministic across a repeat run. Only a skill that passes **and** is bound can ever reach
its `execute`.

**Why the split.** In a world where an LLM might propose running *any* tool, “which tools are
allowed to run, and have they been proven to behave?” is a safety question — pure
infrastructure. You author the tool and its test cases; ARVES decides, on every registration,
whether it’s fit to exist.

## Stage 5 — Govern actions (policy-as-truth)

Ava can now pay a vendor. Should she be allowed to, unsupervised? No. That’s governance.

**🟦 What you build.** Your rules, expressed as policy — plus who may approve:

```js
// "spend"-class actions now require a committed approval from role 'user'
await ava.guardrails.setPolicy({ name: 'spending', appliesTo: ['spend'], approverRole: 'user' });

// later, an approval is a SEPARATE truth — the requester can't approve itself
await ava.guardrails.approve('user', subject);
```

**⬛ What ARVES provides.** The *primitives* that make governance impossible to fake — policies,
approvals, and compliance events committed as durable, content-addressed, replayable **truth** —
plus the capability-admission gate (only a certified + bound skill can be invoked at all).

**🟦 What you build.** The enforcement gate itself: one central `enforce()` call before every
skill invocation, exactly as the reference `guardrails.mjs` does. It’s small, and you write it
once — built *on* the ARVES primitives, not from scratch:

```
Approval Flow  (your gate, built on ARVES primitives)

  think("pay acme $1200")
        │
        ▼  🟦 resolve risk from the REGISTERED skill  → 'spend'   (never from the request)
        ▼  🟦 ask the gate: is 'spend' governed, and is there an approval truth?
        │
        ├── no approval ──►  ⬛ BLOCK  →  commit a compliance truth  →  "needs approval"
        │                                (the block itself is recorded — never a silent drop)
        │
        └── approval exists ──►  🟦 PASS  →  invoke the skill
```

Policies and approvals are themselves *truth* — content-addressed, durable, replayable. After
a restart, your policies still gate, because they were never config in your product; they were
facts in the runtime, and you rebuild the gate’s view from them.

**Why the split.** ARVES gives you governance you *can’t fake*: every policy, approval, and
block is permanent truth, so the record can never lie about what was allowed. What ARVES v1.0
does **not** do is structurally stop an un-gated invoke — the runtime enforces capability
binding and identity, not your policies. So composing the gate, and always routing actions
through it, is the product’s job. Do it in **one** place (as `guardrails.mjs` does), never as
scattered `if (approved)` checks, and derive risk from the skill, not the caller.

> **Honest residual (v1.0):** a committed approval’s `role` is structural, not cryptographically
> authenticated — there is no authN on commit yet (a recorded v2.0 item). The gate’s strength is
> that everything it does is permanent truth; its limit is that *you* must route every action
> through it. Stated, not hidden.

## Stage 6 — Attach a reasoner (the intelligence)

Now the fun part — the brain. And here’s the ARVES surprise: **the LLM is a plug-in, not the
center.**

**🟦 What you build (or plug in).** A reasoner: anything that turns a goal + context into *one
proposed action, or none*. In tests, use the deterministic stub. In production, plug your model
— one line changes:

```js
// Deterministic, offline, replayable — perfect for tests and demos:
import { StubReasoner } from './src/reasoner.mjs';
ava.useReasoner(new StubReasoner());

// Real intelligence — the ONLY line that changes:
import OpenAiReasoner from './src/openai-reasoner.mjs';
ava.useReasoner(new OpenAiReasoner());   // key read from env at call time, never stored
```

A reasoner’s entire contract is `{ name, version, reason(context) → proposal }`, where a
proposal is either `{ action: 'invoke-skill', skill, input, … }` or `{ action: 'none' }`.

**⬛ What ARVES provides.** The safety rails that make it OK to hand the wheel to a probabilistic
model:

- The reasoner can **only propose** — it cannot commit truth or run a skill. The runtime does
  that, after the gate.
- A proposal naming a skill that isn’t registered is **refused** — a hallucinated tool never
  reaches the runtime.
- Every proposal is committed as **attributed truth** (`openai:gpt-4o-mini` and its version),
  so you always know which brain decided what.
- **Replay never re-calls the model.** A proposal is recorded once as truth; replaying history
  reads the record. Your non-deterministic LLM lives inside a deterministic system.

**Why the split.** Your intelligence is your edge — swap models freely, it’s yours. But an LLM
is untrusted by construction: it can be wrong, jailbroken, or hallucinate. ARVES treats it
exactly like that — a suggestion engine wrapped in governance. *Governance lives outside the
reasoner*, so a hostile or buggy brain still can’t bypass a policy or run uncertified code.

> This reframes what “an AI product” even is. The LLM isn’t your product; it’s one replaceable
> component. Your product is the *governed system* around it.

## Stage 7 — Run the loop, and explain it

Everything is wired. Now Ava thinks and acts:

```js
const r = await ava.think('pay the acme invoice');
```

**🟦 What you build.** The loop that drives it — call `ava.think(intent)` for each incoming
goal, branch on the typed result, and call `why()` when someone asks for an explanation. That’s
your harness; the governed pipeline underneath is ARVES (with your Stage-5 gate).

**⬛ What ARVES provides.** The governed pipeline from a Stage-6 proposal to a committed effect,
and an honest, typed result telling you exactly what happened:

```
r.acted   === true                       ✅ ran; r.invocation.truths[] are the effects
r.blocked === true                       🚧 policy blocked it; approve, then ask again
r.failed, stage:'proposal-rejected'      ⚠️ rejected before the gate (e.g. bad LLM input)
r.failed, stage:'skill-execution'        ⚠️ gate passed, but the skill threw
r.reason  === 'no-action-proposed'       💬 nothing to do (e.g. "hi" maps to no skill)
```

And then, at any time, the question every AI product must answer. Here `why()` is *your* code —
a thin projection (like the reference `why.mjs`) reading an ARVES truth substrate:

```
why(ava, subject)   →   the decision path, reconstructed from committed truth:

    observed  →  reasoned  →  proposed  →  gated  →  approved  →  acted
       │            │            │           │          │          │
       └──  ⬛ each station is a committed truth  ·  🟦 why() walks them in order  ──┘
```

You didn’t build the *audit substrate* — every station was already truth, and ARVES gives you a
read-only WAL scan to enumerate it. You wrote the thin projection that walks it. Explainability
falls out of a truth-based runtime; you just render it.

> **Honest residual:** reconstructed purely from a cold WAL scan, the path recovers every
> station **except** the final effect→skill causal edge — that link is process metadata, not yet
> in the committed body (a recorded RCR candidate). The live same-process record has it; a
> from-scratch replay re-derives it. Stated, not hidden.

**Why the split.** “Why did the AI do that?” is the question that ends AI products in
regulated domains. In ARVES it has a real answer because *every step was already truth*. Your
product gets accountability for free — because it never had a choice about recording.

## Stage 8 — Build the experience

Only now — after memory, skills, governance, and reasoning are real — do you build what the
user actually touches.

**🟦 What you build.** A CLI (`bin/ava.mjs`) or a web console (`ui/server.mjs` + a self-
contained page), like JARVIS. This is 100% yours: design, copy, personality, flow. Make it
calm, make it yours.

**⬛ What ARVES provides.** Nothing new here — and that’s the point. Your UI is a thin client
over the same `observe / think / why` surface. Because all the hard guarantees live below the
UI, you can redesign the entire experience without touching correctness.

**Why the split.** The experience is where products win or lose, and it changes constantly.
Keeping it a thin layer over a stable core means you can iterate on delight without ever
risking truth, governance, or audit.

## Stage 9 — Ship, maintain, evolve

**Ship.** Keep the gates green. Every commit: `freeze_check.py check` (the frozen platform is
untouched — `281 files, 0 drift`) and your product’s tests pass. Products ship as **previews**
pinned to a platform version until the GA gate is met (more in §13).

**Maintain.** Your product evolves fast and freely — new skills, new connectors, new UI — all
under `products/`, all LIVING. The floor beneath you never moves.

**Evolve — and the one rule that matters most.** Sooner or later you’ll want something the
runtime doesn’t offer. Here is the entire decision:

```
Runtime is missing something I need
        │
        ├─►  Can I build it in MY product, using the runtime as-is?   ──► 🟦 do it.
        │
        └─►  Do I need to change runtime/ or standard/ behavior?
                        │
                        ▼
                 ⬛ STOP. Do NOT edit the runtime.
                 ⬛ File a Runtime Change Request (RCR).
                 ⬛ It's triaged → v1.1 (additive) or v2.0 (breaking), on its own cycle.
                 ⬛ You keep running on your pinned version until you choose to adopt.
```

This is the discipline that keeps ARVES trustworthy for everyone. The moment products are
allowed to patch the runtime “just this once,” the runtime stops being frozen, replay stops
being guaranteed, and the whole promise unravels. So: never work around the runtime inside
your product. Escalate instead. Frozen is a gift you protect by respecting it.

---

# Part IV — Doing It Well

## 10. How to structure a product

The reference layout again, now that you know what each piece is *for*:

```
products/ava/
├── package.json      identity + the promise (modifiesRuntime:false) + test taxonomy
├── src/
│   ├── assistant.mjs   your product core (observe / think / why over the bridge)
│   ├── skills.mjs      your capabilities + the admission path
│   ├── connectors.mjs  your adapters from the real world
│   ├── reasoner.mjs    the stub; your LLM plugs in beside it
│   └── guardrails.mjs  your policies
├── bin/ava.mjs       a thin CLI
├── ui/               an optional console (server + one self-contained page)
├── fixtures/         deterministic inputs, so demos & tests replay identically
├── examples/         runnable *-day.mjs stories that prove the product end-to-end
├── *.test.mjs        the mandatory test taxonomy, at the root
└── README.md         run-it-yourself + honest scope
```

Everything reaches the runtime through the bridge. Nothing imports runtime internals. If you
can delete a file and the *runtime* still builds and freezes clean, you’ve drawn the boundary
correctly — which, by construction, you always can.

## 11. Best practices — the ARVES way

**Treat ARVES as operating-system infrastructure.** You don’t reimplement the filesystem to
save a file. Same reflex here.

- **Never bypass the runtime.** Every fact and effect goes through `commit`/`observe`/`invoke`.
  A side-channel is a fact ARVES doesn’t know about — invisible to audit, replay, and `why()`.
- **Never build your own memory.** No shadow database of “what the AI knows.” Observe it as
  truth; recall it from truth. Two sources of memory always diverge.
- **Never build your own audit log.** The truth log *is* the audit log. A second one will lie.
- **Never duplicate governance.** No `if (approved)` in your skills. Set a policy; trust the gate.
- **Never mutate a truth.** Reality changed? Commit a *new* truth. The old one is history, and
  history is the point.
- **Never call the LLM directly for actions.** Route it through the reasoner slot, so its
  decisions are attributed, gated, and replayable. A raw `client.chat()` that then does
  something is an ungoverned action — the exact thing ARVES exists to prevent.
- **Keep connectors and skills deterministic.** No clocks, no randomness inside them. Determinism
  is what lets you replay and certify.
- **Keep the product small.** If a module feels like “platform,” it probably belongs in the
  platform — as an RCR, not in your `src/`.

## 12. Common mistakes (and why they’re wrong)

These are the mistakes a strong engineer makes *precisely because* they’re experienced. Old
instincts, wrong runtime.

```
❌  "I'll store what the AI knows in my own DB / vector store."
    → You just forked memory. It will diverge from truth, break replay, and go stale.
      ✅ observe() it as truth; recall from truth. ARVES already deduped and durably stored it.

❌  "I'll call the LLM directly and let it use my tools."
    → Ungoverned action. No attribution, no policy gate, no replay, no why().
      ✅ Plug the LLM into the reasoner slot. It proposes; the runtime gates and acts.

❌  "This action is low-risk, I'll skip the policy."
    → Risk changes; unbounded actions are how AI products cause incidents.
      ✅ Classify it on the skill; let the gate decide. An ungated 'normal' action is a choice, too.

❌  "I'll add audit logging to my product."
    → A second source of truth that will contradict the first one under pressure.
      ✅ The truth log is your audit log. Use why() and the journal.

❌  "The runtime is missing X, I'll just patch runtime/ quickly."
    → You broke frozen. Replay and certification are now void for everyone.
      ✅ File an RCR. Build around it in your product if you can; escalate if you can't.

❌  "I'll edit a truth to fix a mistake."
    → Truth is immutable by design; there is no edit. Mutating it would erase history.
      ✅ Commit a new, corrective truth. The record shows both what was believed and the correction.
```

Every one of these is the same error in disguise: **rebuilding, inside your product, something
the runtime already owns.** The cure is always the responsibility divide in §5.

## 13. Design philosophy

Everything in this guide reduces to four sentences. If you remember nothing else, remember these:

> **Products should be small.**
> **The runtime should stay generic.**
> **Business logic belongs in products. Infrastructure belongs in ARVES.**
> **The boundary between them is sacred — it changes only through an RCR.**

Why hold the line this hard? Because the value of ARVES is *cumulative and shared*. A generic,
frozen runtime means every product — yours, and a thousand you’ll never meet — inherits the
same correct memory, governance, audit, and replay. The instant one product bends the runtime
to its convenience, that shared foundation cracks for all of them. Discipline here isn’t
bureaucracy; it’s what makes the platform worth building on at all.

And it’s why an ARVES product can aim to still be running, replaying, and certifying years from
now: it was never allowed to entangle itself with a moving foundation.

**The GA gate.** Building on ARVES starts today — development is open. But *general
availability* (a production release that carries platform-stability guarantees) is held to a
higher bar: an independent runtime, an external team, certification, and formal verification —
all passing. Until then, honest products ship as **previews** pinned to a platform version.
(JARVIS itself is honestly a preview under this gate today.) This isn’t a limit on your
ambition; it’s the difference between “it works on my machine” and “you can bet your company on
it.”

---

## The mantra

When you’re deep in the code and unsure, don’t ask *“can I make this work?”* You almost always
can. Ask instead:

> **“Does this preserve the standard? Am I building my product — or rebuilding the runtime?”**

Build your product. Let ARVES be the runtime. That’s the whole way.

---

*This guide is a companion to `products/README.md` (the product charter and rules),
`runtime/RUNTIME_FREEZE_v1.0.md` (the freeze record and RCR process), and `CLAUDE.md` (the
Engineering Constitution). The shipping reference product is `products/arves-assistant/`.
When in doubt, the source is the authority.*

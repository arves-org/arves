# Why ARVES Could Fail

This document is not fear. It is focus. [SUCCESS.md](SUCCESS.md) is the compass — where we're
going. This is the hazard map — what sinks the ship. ARVES v1.0 is **no longer an engineering
project; it is a platform awaiting independent validation**, and at this stage every serious
risk is an *adoption* risk, not a technical one. None of them is fixed by writing more core code.

Each failure mode below has three parts: the **risk**, an **early-warning signal you can
measure**, and the **lever** that answers it. Watch the signals; pull the levers; never pull
the "add more runtime" lever unless a signal specifically demands it.

---

### 1. No supply — nobody writes capabilities
**Risk:** the marketplace stays empty because the metric that matters is *capability authors*,
and there are none but us.
**Signal:** independent-author count flat at zero after publish; the only capabilities are the
reference ones we shipped.
**Lever:** DX and the AI Developer Assistant (`arves create`), templates, and a ruthlessly
short Time-to-First-Capability. Supply is a developer-experience problem, not a runtime one.

### 2. No network effect — publishes without installs
**Risk:** a few capabilities get published but nobody installs anyone else's; no flywheel.
**Signal:** publish count rises while cross-party install count stays ~0.
**Lever:** seed genuinely useful capabilities; make discovery and install one command; reward
the first vendors. A marketplace with no *demand* side is a directory, not an economy.

### 3. Conceptual overhead too high — "too complex to bother"
**Risk:** content-addressing, truth, replay, RCR, ACS — the mental model is heavier than a dev
will pay for a first project.
**Signal:** Time-to-First-Success climbing; docs-self-sufficiency falling; the same concept
questioned again and again in the Developer Journey re-runs.
**Lever:** the **edges** — docs, examples, playground — never the core. If the runtime is the
thing they trip on, that is an RCR; if the *explanation* is, that is a docs fix. Almost always
it's the explanation.

### 4. Docs fail real (non-founder) users
**Risk:** the docs work for us because we already know the answers; a stranger stalls.
**Signal:** the rate of people asking *us* questions is not falling over time.
**Lever:** when the first independent developer emails for help, **do not just answer** — first
ask *"where did the documentation fail?"* The goal is not to give support; it is to make
support unnecessary. Every question answered privately is a docs bug shipped publicly.

### 5. The AI landscape outruns the provider abstraction
**Risk:** models evolve faster than the `ReasoningProvider` contract — tools, streaming,
multimodal, agentic loops — and every new model needs a runtime change to fit.
**Signal:** new provider adapters keep forcing RCRs into the frozen core.
**Lever:** hold the line that a provider's output is *recorded truth*, and evolve the provider
**contract at the edge** (the SDK), not the Kernel. If the abstraction leaks into the core, the
"the model can change, the truth cannot" moat is gone.

### 6. The ecosystem stays single-vendor (only us)
**Risk:** no genuine third party ever builds a runtime, publishes a capability, or governs —
**G2 never happens**, and ARVES is a well-built product, not a standard.
**Signal:** certified *independent* runtimes/vendors = 0 many months after publish.
**Lever:** the runtime-vendor on-ramp, a genuinely neutral Foundation, and the discipline to
*not hoard* — publish the standard, invite competitors, celebrate the first runtime that isn't
ours. A standard owned by one team is not a standard.

### 7. The honesty discipline erodes under adoption pressure
**Risk:** to look good at launch, claims quietly outrun evidence (the exact thing every audit
this program ran was built to prevent).
**Signal:** a public doc claims something not backed by a runnable command or graded proof;
a "G1" result gets described as "proven."
**Lever:** keep the grading (G1/G2), keep enforcement over documentation (bind and verify, don't
just note the caveat), and keep re-running the audits. The credibility that earns adoption is
the honesty — losing it loses everything.

### 8. Death by not-publishing
**Risk:** the safest-feeling failure — keep polishing the inside, never ship, and the
marginal value of each new line approaches zero while the opportunity cost rises.
**Signal:** weeks pass; the repo is still private; the "five events" are all still zero.
**Lever:** **publish.** The inside is done. The only information left is in real users, and you
cannot get it from here.

---

## The one-line reframe

> The proofs that remain — *did a stranger succeed? did an outside team certify a runtime? did
> someone earn money? did someone keep building ARVES without us?* — **cannot be produced inside
> this repository.** ARVES will now be validated by user behavior, not by code.

Read this file alongside [SUCCESS.md](SUCCESS.md) at the start of each Growth cycle: one names
what we're steering toward, the other names the rocks. If a quarter goes by and none of the
Success events moved while one of these Failure signals is flashing — that is where the next
work is, and it is almost never in the runtime.

# @arves/personal-os — Personal Cognitive OS (P4)

**Not an assistant. Not a chatbot. Not automation.** The first operating system for a
person's cognition: a persistent, content-addressed world model of your reality that
produces reasoning which is **reproducible, evidence-backed, auditable, replayable, and
aware of your prior decisions** — built entirely on the **frozen ARVES Runtime v1.0**.

```
Email · Calendar · Slack · GitHub · Finance · Health · …
        → Truth → Memory → Reasoning → Planning → Execution → Audit → Replay →
Today · Contradictions · Recommendations · Actions
```

## The success metric is not usability — it is proving ARVES is necessary

Every feature must fail this filter to ship: *could it be built with ChatGPT, Claude,
LangGraph, n8n, AutoGen, or a simple AI wrapper?* If yes, it's rejected. Here is why the
core features **cannot**:

| Feature | Why existing AI can't do it | Runtime API | Truth created | Evidence | Replay |
|---|---|---|---|---|---|
| **One truth from many systems** | a wrapper has no cross-source identity — it double-counts the same meeting from calendar+email+slack | SDK address · Bridge commit | one `uci.fact` per real event | the set of attesting sources | re-ingest → same truth |
| **Reproducible daily briefing** | an LLM answers differently every run, with no audit and no proof | Bridge commit (real Kernel) | one `uci.briefing` truth | the truth ids it reasoned over | re-run → identical id, Kernel idempotent |
| **Contradiction with a prior decision** | a chatbot has no persistent, addressable, evidence-backed decision history | Bridge commit | `uci.decision` truths | the prior decision's ContentId | the contradiction is provable, not vibes |
| **Evidence-backed recommendation** | a wrapper cannot cite *which stored truths* a recommendation rests on | SDK address | — | truth ids per recommendation | defensible after the fact |

## Run

```
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml   # once: the frozen Runtime API
node examples/my-day.mjs                          # your day as reproducible cognition (exits 0)
```

## Platform boundary (Runtime v1.0 is FROZEN)

This product **consumes** the Runtime v1.0 API (SDK + Bridge → real Kernel/Engine/
Capability) and **edits no runtime file**. If it needs something the runtime lacks, that
is a **Runtime Change Request** (→ v1.1), never a runtime edit — see
`runtime/RUNTIME_FREEZE_v1.0.md`. P4 is the runtime's first real customer, and the proof
that v1.0 can carry a rich product unchanged.

Next products build on the same API, product-layer only: Enterprise Cognitive OS (P5),
Developer Studio (P6), Marketplace (P7), Cloud (P8), Industry (P9).

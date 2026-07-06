# Product Brief — "JARVIS" (the maintainer's product, phase 1)

> **Status:** LIVING · the maintainer's product definition under Ruling 002 ("system ready" =
> this product runs on ARVES honestly). Phase 1 scope, captured 2026-07-06 from the maintainer:
> *"Ürün JARVIS gibi olacak ilk etapta"* — a JARVIS-like personal AI assistant. This brief maps
> the product's needs onto the runtime surfaces ARVES has ALREADY built, names what phase 1
> must prove, and drives the I6 Reference Products acceptance criteria.

## What JARVIS-like means here (phase 1)

A **personal cognitive assistant** that:
1. **observes the user's world** (multiple sources → one deduplicated truth base),
2. **remembers everything durably** (decisions, facts, context — survives restarts),
3. **reasons and acts through skills** (certified capabilities; an LLM as the reasoner),
4. **orchestrates sub-agents** (research / schedule / monitor working over ONE shared truth),
5. **obeys guardrails** (user policies enforced as truth; separate approval truths),
6. **can always explain itself** ("why did you do that?" → replay the decision trace).

## Why ARVES is the right substrate — the 1:1 mapping

| JARVIS need | ARVES surface (BUILT, tested) |
|---|---|
| Durable memory across restarts | `--wal-dir` file-backed Kernel (RCR-015): same ContentId after restart, deterministic recovery (ORCH-003) |
| One truth from many sources | Personal-OS connector pattern (P4): multi-source → dedup by content address → evidence sets |
| Skills = safe, certified actions | Ecosystem SDK: `defineCapability`/`certifyCapability` (re-run at install/publish — unforgeable); dynamic `bind` (RCR-016) |
| LLM as the reasoner | `defineReasoningCapability` (reasonerHash-bound) + `Determinism::Nondeterministic` engine class — the recorded inference is authoritative on replay (RCR-012 fabric enforcement); **the LLM's words become addressable truth** |
| Sub-agents over shared truth | **I5 Multi-Agent Runtime** (in flight): agent identity + attribution (RCR-029), ORCH-004 convergence across agents, deterministic conflict resolution (RCR-030) |
| Guardrails / policies | Enterprise-OS pattern: policy-as-truth; violations blocked + committed as compliance events; approvals are SEPARATE committed truths (E1) |
| "Explain yourself" | Append-only WAL = the decision trace; full replay incl. attribution (RCR-031 target); audit chain |
| Personal + private | Single-tenant first via `shard=` (RCR-014); the runtime is local (trusted single host, honest v1.0 threat model — no cloud dependency) |
| Exactly-once actions | I4 scheduler: idempotent dispatch (an action is never double-executed under retries) |

## Phase-1 acceptance criteria (drives I6)

The I6 reference product ("assistant skeleton") MUST prove, end-to-end and honestly:

- **A1 — Durable memory:** observe facts → restart the whole stack → the assistant still knows
  them (same ContentIds; contradiction detection works across restarts — closes the old P4 X1
  caveat via WAL-replay rebuild of the decision index).
- **A2 — Multi-source one-truth:** ≥3 source connectors; the same real-world event from
  different sources = ONE truth with an evidence set.
- **A3 — Certified skills:** a skill only runs if certification passes (forged flags refused);
  skills are bound dynamically (`bind`) and invoked through the full gated chain
  (capability → enforced engine → Kernel commit).
- **A4 — Reasoner slot:** a pluggable reasoner interface where the maintainer can attach a real
  LLM; the repo ships a DETERMINISTIC STUB reasoner (tests/demos stay offline+reproducible) —
  honest note that real-LLM wiring is the maintainer's runtime configuration, not repo code.
- **A5 — Sub-agent orchestration:** ≥2 agents (e.g. researcher + scheduler) working over one
  truth base with attribution; conflicting proposals resolve deterministically.
- **A6 — Guardrails:** at least one policy-as-truth (e.g. "spend/irreversible actions need a
  separate approval truth") demonstrably blocking + auditing.
- **A7 — Explain-yourself:** a `why(truthId)` flow that reconstructs the decision path from the
  WAL (what was observed, which agent proposed, which policy checked, what committed).
- **A8 — Maintainer test path:** a QUICKSTART for the maintainer to run the assistant locally
  (`--wal-dir` + their own shard), attach their LLM key OUTSIDE the repo, and exercise A1-A7.

## Open questions for the maintainer (answer anytime; defaults chosen honestly)

- **OQ-1 — First sources?** default: filesystem-notes + calendar-file + tasks-file connectors
  (offline, deterministic); real email/calendar APIs are maintainer-side wiring.
- **OQ-2 — Interface?** default: local CLI/REPL first (TS SDK); P8-style local HTTP gateway
  optional second.
- **OQ-3 — Which LLM?** irrelevant to the repo (A4 keeps it pluggable); maintainer's choice at
  test time.
- **OQ-4 — Voice/UI?** out of phase 1 (recorded as phase-2 candidates).

## Honesty boundaries (stated up front)

- Agents in repo tests are deterministic actors; the INTELLIGENCE comes from the maintainer's
  LLM at their test time. ARVES's claim is not "smart" — it is **remembered, governed,
  attributed, replayable** cognition.
- v1.0 trust model: single host, no authN on commit (v2.0 debt #8) — fine for a personal
  assistant on the maintainer's machine; stated, not hidden.

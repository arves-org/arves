# @arves/assistant — the JARVIS-like Assistant (phase 1) · stages 1–3: MEMORY + GOVERNED SKILLS + AGENTS + WHY

The maintainer's product (`docs/PRODUCT_BRIEF_JARVIS.md`, Ruling 002): a personal
cognitive assistant whose claim is not "smart" — it is **remembered, governed,
attributed, replayable** cognition. Stage 1 ships the **memory core** (A1 durable
memory · A2 multi-source one-truth); stage 2 ships the **governed think→act pipeline**
(A3 certified skills · A4 pluggable reasoner slot · A6 guardrails); stage 3 ships
**sub-agents + explain-yourself** (A5 multi-agent over one truth base · A7 `why()`) and
the **maintainer run path** (A8, `docs/JARVIS_QUICKSTART.md`) — all on the **frozen
ARVES Runtime v1.0**, honestly and offline.

## What stages 1–3 prove (run it yourself)

```
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml  # once
node products/arves-assistant/assistant.test.mjs      # stage 1: 7/7 tests, exit 0
node products/arves-assistant/examples/assistant-day.mjs  # stage 1 scripted day, exit 0
node products/arves-assistant/skills.test.mjs         # stage 2: 6/6 tests, exit 0
node products/arves-assistant/examples/assistant-skills-day.mjs  # stage 2 governed day, 8/8, exit 0
node products/arves-assistant/agents.test.mjs         # stage 3: 6/6 tests, exit 0
node products/arves-assistant/examples/jarvis-day.mjs # THE CAPSTONE: A1–A7 in one day, exit 0
# GA-hardening (daily driver, honest scope — see the section below):
node products/arves-assistant/connectors.test.mjs         # email/csv/jsonl + cross-format dedup + hostile inputs
node products/arves-assistant/reasoner-adapter.test.mjs   # real-LLM adapter shape (fake client) + governed pipeline
node products/arves-assistant/robustness.test.mjs         # hostile CLI + bridge-down + config + report
node products/arves-assistant/examples/jarvis-hardened-day.mjs  # the hardened day, 8/8, exit 0
```

- **A2 — one truth from many sources:** 3 deterministic offline connectors
  (`notes-file`, `calendar-file`, `tasks-file`) map raw items to canonical facts; the
  source is **evidence, never identity**, so the dentist appointment seen by notes AND
  calendar is ONE truth (`already-committed` from the Kernel itself) with both sources
  in its evidence set. Evidence is truth too: each (source, fact) pair is a committed
  `uci.assistant.attestation`, so attribution survives restarts by proof, not by trust.
- **A1 — memory survives restarts:** facts and decisions are committed through the
  bridge to the WAL-backed Rust Kernel (`--wal-dir`, RCR-015). The scripted day kills
  the stack, opens a NEW Assistant over the SAME walDir, and proves: identical
  ContentIds, every body `already-committed`, and **contradiction detection still
  works, citing the same prior decision id** — closing P4's X1 caveat (in-process-only
  contradiction index) at the product level.

## Stage 2 — the governed think→act pipeline (A3 · A4 · A6)

- **A3 — skills are CERTIFIED capabilities (`src/skills.mjs`):** the trust boundary is
  the Ecosystem SDK kit, **reused, not reimplemented** (`defineCapability` /
  `certifyCapability`). `registerSkill()` RE-RUNS certification at registration — a
  forged `certified: true` flag on the capability object is never consulted and cannot
  bypass the gate (proven in `skills.test.mjs`). A certified skill is dynamically
  **bound** in the assistant's shard (RCR-016) and invoked through the FULL runtime
  chain: Capability (resolve binding) → Engine (fabric-enforced) → Kernel (committed
  effect truth). The admission itself is a committed `uci.assistant.skill` truth carrying
  the code's hash. Honest limits inherited from the kit: the determinism check is a
  best-effort run-twice probe, not a purity proof (engine-enforced determinism is v1.1
  RCR debt).
- **A4 — pluggable reasoner slot (`src/reasoner.mjs`):** `think(goal)` runs
  reasoner proposal (**committed as truth**) → guardrail gate → certified skill →
  committed effect truth. The repo ships exactly ONE reasoner, **StubReasoner — a
  deterministic keyword→action table that is NOT AI**: it cannot understand or
  generalize, and answers `action:'none'` for goals outside its table rather than guess.
  The maintainer plugs a real LLM by implementing the Reasoner interface **outside the
  repo** (contract below). Governance is outside the reasoner by design: whatever any
  reasoner proposes, policies are consulted BEFORE invocation and only certified+bound
  skills can act.
- **A6 — guardrails, policy-as-truth (`src/guardrails.mjs`, the enterprise-os pattern):**
  policies are committed truths (`setPolicy`); gated action classes (e.g. `spend`,
  `irreversible`) require a **SEPARATE committed approval truth** (`approve(role,
  subject)`) — the proposer can never self-clear its own gate; a wrong-role approval
  does not unlock. Violations are **blocked AND committed** as
  `uci.assistant.compliance` truths — the ledger records what was refused, not only what
  happened.

### The Reasoner interface contract (plug your LLM here, outside the repo)

```
interface Reasoner {
  name:    string                 // committed into every proposal truth
  version: string
  reason(context) -> proposal     // sync or async; MUST NOT commit truth itself
}
context  = { goal, truths, decisions, skills }           // read-only projections
proposal = { action:'invoke-skill', skill, input, subject, actionClass, because }
         | { action:'none', because }
```

Example wiring (maintainer-side file, never in this repo): implement `reason(context)`
with your model SDK, return the proposal shape, then `assistant.useReasoner(new
LlmReasoner())` — the only line that changes. The full annotated snippet is in
`src/reasoner.mjs`. An LLM-backed reasoner is naturally non-deterministic; its proposal
is committed ONCE as content-addressed truth and replay reads the record — it never
re-calls the model (the recorded-truth doctrine, ecosystem-sdk `REASONING.md`).

## Stage 3 — sub-agents + explain-yourself (A5 · A7) and the maintainer path (A8)

- **A5 — sub-agents over ONE shared truth base (`src/agents.mjs`):** `ResearcherAgent`
  (gathers facts into agent-tagged finding truths citing fact ContentIds) and
  `SchedulerAgent` (proposes plan items from the truth base) are **deterministic
  actors, NOT AI** — rule tables over committed truth. Attribution is **product-level
  and in-body**: every contributed truth carries `agent`/`agentVersion` fields, so
  attribution is content-addressed and survives restarts. Conflicting proposals on one
  subject resolve **FIRST-COMMITTED-WINS** (the I5/RCR-030 runtime semantics at product
  level); the losing proposal stays committed truth and the loser commits a
  `uci.assistant.resolution` truth **referencing the winner**. Honest residual: with no
  authN in v1.0 the tag is structural, not cryptographic; and the runtime's Rust-level
  I5 attribution is not exposed over the bridge — RCR candidate #2 below.
- **A7 — explain yourself (`src/why.mjs`):** `why(assistant, truthIdOrSubject)`
  reconstructs the decision path from committed truths — what was **observed** (facts +
  evidence sources), what agents **researched**, who **proposed** (reasoner- or
  agent-attributed), how conflicts **resolved**, which **policy** was checked, what was
  **blocked**, what **approval** existed, what **committed** (with the admitted skill's
  codeHash) — as a structured trace where every station is a ContentId + canonical
  body, checkable against the ledger. `renderWhy(trace)` prints it. **Honest
  mechanism:** the feed is the assistant's product-side **decision journal** (every
  truth this process committed/re-proved, in commit order) — a projection rebuilt after
  a restart by re-running the deterministic day (already-committed = the proof); the
  capstone proves the trace is **byte-identical across a restart**. A native WAL-scan
  verb is RCR candidate #1 below.
- **A8 — the maintainer path (`docs/JARVIS_QUICKSTART.md`):** build the bridge once,
  run the assistant on your own `shard=` + `--wal-dir`, plug your real LLM via the
  Reasoner interface (exact contract + wiring example, key outside the repo), and
  exercise A1–A7 on your own data — with the honesty boundaries restated.

## The rebuild mechanism — stated loudly

The bridge line protocol has **no verb to enumerate or scan committed truth**. So
`rebuild()` uses the one thing the frozen Kernel guarantees: **idempotent,
content-addressed commit**. It re-derives candidate bodies deterministically (connectors
re-read their fixtures; the caller re-supplies its decision journal) and re-commits
them; the Kernel answering `already-committed` **is the membership proof** that this
exact body was truth before the process existed. The candidate list is an untrusted
hint; only the Kernel's answer counts. Honest side effect: an unknown candidate becomes
newly committed truth (there is no read-only probe) — the rebuild report separates
`recovered` from `fresh` so nothing is smuggled in silently.

## GA-hardening — the daily driver (honest scope)

Beyond stages 1–3, JARVIS is hardened and completed enough to be the maintainer's daily
driver — **without** claiming the four-condition GA gate (that gate is EXTERNAL — see the
platform boundary). Run `node examples/jarvis-hardened-day.mjs` (8/8 properties) and
`node connectors.test.mjs && node reasoner-adapter.test.mjs && node robustness.test.mjs`.

- **Hostile-input hardening (`robustness.test.mjs`, `src/connectors.mjs`):** every connector
  reads through a guard that refuses a missing/irregular file with a CLEAN error and caps
  file size (16 MiB default, `JARVIS_MAX_SOURCE_BYTES` override) so a giant file fails loud
  instead of exhausting memory. Malformed journal/iCal/CSV/JSON-lines/email lines fail with
  the offending line number. Every CLI command runs through a try/catch that returns
  `ok:false` + a loud line — a bad command, an unknown connector, a bad instant, or a
  **bridge-down mid-session** never crashes the REPL (all asserted).
- **More real-source connector TEMPLATES (`src/connectors.mjs`):** `email` (`.eml` — RFC 5322
  headers: From→entity, Subject→event, Date→UTC instant, body ignored), `csv`
  (`iso,entity,event`), and `jsonl` (JSON Lines) join the notes/calendar/tasks/journal/ical
  readers. Still 100% offline and deterministic (fixed instants, no clock/RNG), but they
  parse formats a user ALREADY keeps — point them at your own files with ZERO code changes
  (`jarvis import csv /path/to/your.csv`). The SAME dedup rule holds across formats: the
  dentist-appointment seen by csv AND jsonl collapses to ONE committed truth whose evidence
  set names both sources (A2) — proven across separate processes over a durable WAL.
- **Persistent config (`src/config.mjs`, `jarvis config …`):** a `~/.jarvisrc.json` (or
  `--config <path>`) holds session defaults (`tenant`/`workspace`/`walDir`/`exe`/`reasoner`)
  so the maintainer stops re-typing flags. Precedence is explicit and honest — CLI flag >
  config file > built-in default — and a malformed config fails loud (an explicit `--config`)
  or is warned-and-ignored (the default path, so the bin stays startable). Only the in-repo
  `stub` reasoner is selectable; anything else is refused (honest scope).
- **Export/report the day (`src/report.mjs`, `jarvis report [json]`):** a deterministic,
  grouped-by-entity export built PURELY from committed truth (instants rendered from the
  committed nanoseconds — no clock), so `report json` replays byte-identically across a
  restart, exactly like `why()`.
- **REASONER ADAPTER example (`src/llm-reasoner.example.mjs`):** a COMPLETE,
  interface-conformant `Reasoner` whose only missing piece is your model call, injected as a
  `client.complete(prompt) -> string` — the file marks **`>>> PUT YOUR API CALL HERE <<<`**
  and imports NO network SDK and NO key. It builds a prompt from the read-only truth context,
  calls your client, and parses the reply into the governed proposal shape; a **hallucinated
  skill** (one not in the registered, certified+bound set) is REFUSED (`action:'none'`) rather
  than reaching the runtime, and non-JSON output degrades to `none`. It is unit-tested against
  a fake client AND driven end-to-end through the real in-memory Kernel
  (`reasoner-adapter.test.mjs`), proving it plugs into the SAME governed pipeline as the stub.

## Honest scope (v1.0, phase-1 stages 1–3)

- **No intelligence here.** The A4 reasoner slot ships ONLY the deterministic
  StubReasoner (a keyword table, loudly NOT AI) and the A5 agents are rule-based
  deterministic actors — the intelligence arrives when the maintainer plugs their LLM
  in, outside the repo. No network code or keys live here.
- **Attribution is product-level.** Agent/reasoner identity tags live IN the committed
  truth bodies (durable, content-addressed) — not in the runtime's Rust I5 attribution
  layer, which the bridge does not expose (RCR candidate #2). No authN ⇒ structural,
  not cryptographic.
- **The decision journal is a projection.** `why()` explains only what this process has
  committed/re-proved; a fresh process explains nothing until the deterministic day is
  re-run (RCR candidate #1 would make this total).
- **Skill code runs product-side.** The frozen bridge hosts exactly ONE reference engine
  (`engine:derive.fact@1.0.0`); `bind` (RCR-016) attaches skill NAMES to that engine
  identity — the runtime never loads product JS. Each effect VALUE flows through the
  real Capability→Engine→Kernel chain and is committed as ACS-addressed truth.
- **Approval roles are structural, not authenticated.** v1.0 has no authN on commit
  (v2.0 debt #8): `approve('user', …)` separates approver truth from proposer truth on a
  trusted single host; a cryptographically bound approver identity is the v2.0
  authenticated-commit RCR.
- **Single host, no authN on commit** (v2.0 debt #8) — right-sized for a personal
  assistant on the maintainer's machine; stated, not hidden.
- **Connectors are fixture readers.** Live email/calendar/task APIs are maintainer-side
  wiring (brief OQ-1); the repo stays offline and reproducible.
- **The in-process index is a projection.** A fresh process honestly remembers nothing
  until `rebuild()` proves its memory back from committed truth (the demo asserts this).
- **Windows note:** after `close()`, give the bridge process a beat before deleting a
  temp walDir (the demo/tests sleep 400 ms).

## RCR candidates found (IDR-006: recorded, never implemented product-side)

1. **[bridge: replay/scan verb]** — a read-only verb to enumerate (or probe membership
   of) committed truth for a shard over the line protocol. Today `rebuild()` must
   re-supply candidate bodies and probe via idempotent re-commit, which (a) cannot
   recover truths whose bodies the product can no longer re-derive, and (b) commits
   unknown candidates as a side effect of probing. A native WAL-replay/scan verb (the
   Kernel already replays its WAL internally, RCR-015/ORCH-003) would make rebuild
   total and side-effect-free — and would make stage 3's `why()` decision journal a
   TOTAL reconstruction instead of a re-run-the-day projection. → v1.1 additive
   candidate.
2. **[bridge: attributed-commit verb]** — CONFIRMED by the sub-agent stage (A5):
   runtime I5 landed agent identity/attribution in Rust (`arves-control-plane`,
   RCR-029..031: identity as committed truth, scheduler-borne attributed proposals,
   attribution surviving full-cluster replay), but none of it is exposed over the
   bridge line protocol. Stage 3 therefore implements attribution HONESTLY at the
   product level — the agent/reasoner tag is carried IN every committed truth body
   (`agent`/`agentVersion`, `reasoner`/`reasonerVersion`) — never faked as runtime
   access. An `attributed-commit` (or `commit ... agent=<id>`) verb exposing the
   RCR-029 identity chain would upgrade attribution from product convention to
   runtime-enforced record. → v1.1 additive candidate.
3. **[bridge/engine: product engine hosting]** — the bridge hosts exactly one reference
   engine, so a skill's transformation logic runs product-side and only its effect
   VALUES cross the runtime chain. Runtime-hosted, certified product engine code (so the
   fabric itself enforces a skill's determinism and effect declarations, closing the
   probe-only certification gap) would be a platform change → v1.1/v2.0 RCR candidate.

## Platform boundary (IDR-006)

This product **consumes** Runtime v1.0 (tag `runtime-v1.0`) via the TS SDK codec +
Kernel Bridge line protocol (id= RCR-011 · shard= RCR-014 · --wal-dir RCR-015 · bind
RCR-016) and **modifies no file** under `runtime/`, `standard/`, `spec-markdown/`, or
`corpus/`. A platform gap is an RCR candidate above — never a product-side edit. This
ships as a **G1 preview**; the four-condition GA gate (Independent Runtime · External
Team · Certification · Formal) remains honestly **UNMET**.

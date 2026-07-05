# ARVES Completion Map — the chapters to *truly done*

> **Status:** LIVING · NON-NORMATIVE. This document answers one question — *"In how many
> chapters is ARVES fully, truly complete, and what are they?"* — grounded entirely in the
> frozen corpus and the governing docs. It **changes nothing normative**: it does not add,
> reinterpret, or reorder the frozen Standard or the Engineering Constitution; it only *maps*
> what those documents already declare. Where this map and any governing doc disagree, the
> governing doc wins and the disagreement is a docs bug to report. Derived 2026-07-05 from a
> whole-corpus review (9 document clusters → 214 completion units → 3 independent lenses →
> adversarial reconciliation).

## The answer: **5 chapters — one per declared Era**

ARVES is fully complete in exactly **five chapters**, because the ARVES Engineering
Constitution (`CLAUDE.md`) and the Master Roadmap both name exactly **five Eras** as the
coherent major phases from inception to true completion. The milestones **I1..I6**, the ten
Long-Term Objectives, the product ladder P0..P10, the Foundation KPIs, the 20-item Strategic
Program, and the certification ladder L1→L4 are **not** separate chapters — each folds inside
the one Era that owns it. (Three independent lenses proposed 10 / 5 / 6; they agreed on
substance and diverged only on granularity. The corpus literally declares *five* Eras, an
adversarial completeness critic reviewed the 5-chapter map as **sound**, so five is the count.)

| # | Chapter (Era) | Status | Closes by | The single condition that closes it |
|---|---|---|---|---|
| **1** | **Specification** — author & freeze the Standard | ✅ **DONE** | maintainer | The corpus is COMPLETE / FROZEN and permanently immutable; later change only via CCP / Amendment / IDR / next major. |
| **2** | **Implementation** — build the reference implementation that proves the spec (Build Program, **SEALED**) | ✅ **DONE** | internal | Reference impl exists and the Build Program is SEALED (16-pillar adversarial audit → CLOSE), Runtime v1.0 FROZEN. All graded **G1**. |
| **3** | **Standard Validation** — *prove ARVES wrong*; the G2 third-party runtime is the exit gate | 🟡 **CURRENT** | **external** | A genuinely unrelated party builds a runtime from `standard/` alone (no help, no reference access) and earns full-surface `SOUND-CERTIFIED` — the first real **G2**. |
| **4** | **Industrialization** — run ARVES at scale (I2..I6) | 🟢 **UN-GATED — in build** *(superseded by Ruling 002, 2026-07-05, `docs/MAINTAINER_RULINGS.md`)* | mixed | I2..I6 built and each passes its full Success Criteria; distributed runtime at scale; I6 → GA under the four-condition gate. ~~May not begin until Ch3's G2 fires~~ *(superseded by Ruling 002: build proceeds now from the `docs/design/` packages via RCR; Ch3 continues in parallel)*. |
| **5** | **Growth / Foundation** — prove *why* ARVES matters; survive its makers | 🟡 **in-progress** | **external** | All ten Long-Term Objectives hold **and** the North Star is real: a stranger downloads → builds → publishes → gets paid with no maker contact. |

## Where we are — the honest split

- **2 of 5 chapters are fully DONE** (Ch1, Ch2) — both were internally closable, and they closed.
- **3 remain, and only ONE of them is internal engineering:**
  - **Ch4** (Industrialization: I2..I6, distributed runtime at scale, v1.1/v2.0 RCR debt, L4
    fault-injection/replay/performance evidence) is fully **internal-buildable** — ~~*but it is
    hard-gated* and must not begin until Ch3's external event fires~~ *(superseded by Ruling 002,
    2026-07-05: I2..I6 is un-gated and in build; Ch3's external event remains the only thing that
    lifts independence past G1)*.
  - **Ch3** and **Ch5** close **only on external events that cannot be manufactured internally**:
    Ch3 on a stranger's **G2** certification, Ch5 on real external **adoption** (the North Star).
- Therefore **no freeze-clean or maintainer-gated (CCP/RCR) work advances independence past G1.**
  The engineering arm has done nearly everything it can. Independence today is honestly **G1**
  (Rust + Python + TypeScript, all in-program); every external proof is **open and unmet**.

## Position inside the current chapter (Ch3)

The 20-item Strategic Program's **r1–r15, r19-A, r20** and the live-conformance triple are
**DONE**. What remains *inside* Ch3 is only the publish-enablers, all of which are
maintainer-gated or external, not more internal proof:

- **r16 PUBLISH** — technical pre-conditions (r1/r2/r3/r15) MET; publishing itself needs the
  maintainer's explicit go + a chosen public org/namespace, and is **irreversible**.
- **r17 CI-host** provisioning · **r18 external-G2 funnel** (distribute CHALLENGE + recognition path).
- **r19-B** (optional RCR) — promote the reference Connector/Query into the frozen
  `arves-information-platform`/`arves-query` crates. Non-blocking; changes those crates'
  documented "contract-only" shape, so it is a deliberate maintainer decision.

The event that **closes** Ch3 — G2 — is external by construction.

## How the ten Long-Term Objectives fold across the chapters

The `CLAUDE.md` "the project is complete only when…" list (LTO 1–10) is not a parallel chapter
scheme; each objective is a *closes-when* test inside the Era that owns it:

| LTO | Objective | Chapter | State |
|---|---|---|---|
| 7 | Developer SDKs exist | Ch2 | ✅ done (Standard Kit / Developer SDK, G1) |
| 2 | Complete conformance suite exists | Ch3 | 🟡 core done; deeper L1–L4 axes fold to Ch4 |
| 3 | Independent Runtime A passes certification | Ch3 | 🟡 met at **G1** (Rust) |
| 4 | Independent Runtime B passes certification | Ch3 | 🟡 met at **G1** (Python; TypeScript codec too) |
| 5 | Third-party certification exists | Ch3 | ❌ **the G2 exit gate — open** |
| 1 | Production-grade distributed runtime exists | Ch4 | 🟢 un-gated, in build (I2..I6; superseded by Ruling 002) |
| 6 | Enterprise runtime exists | Ch4 | 🟢 un-gated (superseded by Ruling 002) |
| 8 | Marketplace exists | Ch5 | 🟡 mechanism built (G1); at-scale = Ch5 |
| 9 | Cloud platform exists | Ch5 | ❌ not started |
| 10 | Real products built entirely on ARVES without modifying the standard | Ch5 | 🟡 in-program demos exist (G1); real external orgs = open |

## Why not 10, or 6

- **Not 10** (one chapter per I1..I6 milestone + eras): the source completion-units explicitly
  state the six Baseline Part-5 milestones are the **work-breakdown inside** the Implementation
  and Industrialization Eras, not top-level phases. Promoting them double-counts the eras.
- **Not 6** (splitting Growth into a Products chapter + a Foundation chapter): the two-arm pivot
  (IDR-006) runs the Product arm and the Standard/Foundation arm **in parallel** consuming the
  same frozen platform, so they are one Era, not two sequential chapters.
- **5** is the count the corpus itself declares, and it double-counts nothing: the single G2
  event is scored once (Ch3's exit; it reappears in Ch5 only as the same event seen through the
  adoption lens), the ten objectives fold as closes-when tests, and the product ladder / KPIs /
  RCR-freeze debt / certification ladder each attach to the one Era that owns them.

## Sources

`CLAUDE.md` (Eras, Milestones I1..I6, Long-Term Objectives, Success Criteria) ·
`runtime/docs/ARVES_Master_Roadmap.md` (Five eras, Era-3 exit gate) ·
`ARVES_BUILD_PROGRAM_CLOSURE.md` (Ch2 SEAL) · `runtime/RUNTIME_FREEZE_v1.0.md` (v1.1/v2.0 debt) ·
`CHALLENGE.md` + `verification/evidence/CERTIFICATION_PROGRAM.md` + `EVIDENCE_LEDGER.md` (G0/G1/G2) ·
`verification/OPEN_DEBT_REGISTER.md` (open items by instrument) ·
`verification/evidence/STRATEGIC_PROGRAM_2026-07.md` (the 20-item program) ·
`FOUNDATION.md` + `SUCCESS.md` + `FAILURE.md` + `products/README.md` (survivability, North Star, ladder).

*Bottom line: ARVES is fully complete in **5 chapters**. Two are done. Of the three that remain,
one (Ch4) is internal engineering — now un-gated and in build *(superseded by Ruling 002)* — and
two (Ch3, Ch5) close only when someone outside the
project shows up — a G2 runtime, then real adoption.*

# ARVES — Master Roadmap

**Status:** authoritative living-repo roadmap (ED-001: progress lives here + git
tags, never in the frozen corpus). Supersedes ad-hoc milestone notes for
cross-era planning.

---

## Five eras (maintainer-set, 2026-07)

| Era | Question | Status |
|-----|----------|--------|
| **1. Specification** | *What is ARVES?* | ✅ COMPLETE (frozen corpus + AEOS) |
| **2. Implementation** | *Can we build a reference + executable standard + Kit?* | ✅ COMPLETE (I1 runtime core · ACS-001..005 · Conformance Platform · Standard Kit) |
| **3. Standard Validation** | *Is ARVES scientifically sound, and can a THIRD PARTY implement it — with no help — and pass?* | 🟢 **CURRENT ERA** |
| **4. Industrialization** | *Can ARVES run on millions of nodes?* | ⬜ gated behind Era 3 (I2–I6, Kernel Integration, enterprise, scale) |
| **5. Product** | *Can real products be built on the certified platform?* | ⬜ separate org; forbidden until the platform is certified |

**The pivot (Era 2 → 3):** we stop *building* ARVES and start trying to *prove it
wrong*. The KPI is no longer "Tests Passed" — it is **Evidence Increased**. Every
milestone updates the **Evidence Ledger** (`verification/evidence/`), and no property
is ever "Done" — it only earns an Evidence Level. Philosophy:

> *You are no longer building ARVES. You are trying to prove ARVES wrong. If you
> fail, only then ARVES becomes stronger.*

**Era-3 exit gate (the decisive proof):** a stranger downloads ONLY the Standard Kit
from a public source, implements a runtime with **no help and no access to any
reference source**, and PASSES certification. Until that happens, independence is
graded honestly (self → same-process-independent → third-party-independent), never
laundered into "independent."

The "destroy-offices" that run this era: **Scientific Review · Security · Performance/
Robustness · Academic (SOSP/OSDI/PLDI) · Standards (IETF WG) · Independent Runtime**.
See `verification/evidence/CERTIFICATION_PROGRAM.md`.

## Roadmap

Two arms run in PARALLEL from here (maintainer pivot, 2026-07 — motto: *"Stop proving
that ARVES can exist. Start proving why ARVES matters."*). The platform is ~90% mature
(3 independent G1 runtimes agree); further evidence is diminishing returns, so product
work begins now — as a **consumer of the frozen, versioned platform** (see IDR-006).

```
                         ARVES
                           │
        ┌──────────────────┴───────────────────┐
   STANDARD PROGRAM                        PRODUCT PROGRAM
   (integrity: keep proving)              (value: prove why it matters)
        │                                       │
   Program 1 BUILD ............ ✅         P1 Developer Platform / SDK .. 🟢 starting
   Program 2 EXTERNAL VALID ... 🟢 current P2 Visual Cognitive Studio
     G2 (external team PASS)               P3 Agent Runtime
     Certification                         P4 Personal AI
   Program 3 INDUSTRIALIZATION  ⬜ post-G2  P5 Enterprise AI
     I2..I6 · Kernel Integration           P6 Marketplace · P7 Cloud
                                           P8 Industry Solutions
```

**STANDARD PROGRAM** = everything that keeps ARVES *correct and independently
implementable*: Program 1 (done), Program 2 (G2 Readiness — current), Program 3
(Industrialization, post-G2). The exit criterion still stands: *can a completely
unknown team build a conformant runtime from the Kit alone, asking nothing?*

**PRODUCT PROGRAM** = everything that proves *why ARVES matters*, built ON TOP of the
platform, never inside it. Product ladder P1→P8 (Developer Platform → Visual Cognitive
Studio → Agent Runtime → Personal AI → Enterprise AI → Marketplace → Cloud → Industry).
Governance: **IDR-006** — products consume `arves-standard-kit 0.2.0` + the reference
runtime as a **frozen external dependency**; no product modifies `runtime/` or
`standard/`; a needed platform change is a **Platform Change Proposal**. The original
four-condition gate (Independent Runtime + External Team + Certification + Formal all
PASS) is **retained for GA / production release**, not for development start.

## ARVES Core Standards (ACS)

These are promoted from "CCP amendments" to **first-class, independently-living
standards** — the documents ARVES will be cited by (its "TCP/IP RFCs"). Each is
still ratified through CCP-GATE (draft + ≥1 conformance scenario), lives in
`runtime/docs/standards/`, and never edits the frozen corpus.

| ACS | Title | Role |
|-----|-------|------|
| ACS-001 | Universal Content Identity | immutable content address (multihash) |
| ACS-002 | Canonical Serialization | the deterministic byte form (keystone) |
| ACS-003 | Canonical Envelope | the interchange envelope |
| ACS-004 | Universal Type Registry | executable ontology (type + schema) |
| ACS-005 | Normative Language | RFC 2119 convention + Terms & Definitions glossary |

## Program 2 — the G2 Readiness Program (current)

The next milestone is **G2 — Independent Standard Validation**, not I2. It is a
scientific-validation program, not a feature program: the largest remaining gap is
not code, it is **independent evidence** (today's honest state is *"reproduced within
one program"* — grade G1). Five tracks:

| Track | Name | Contents |
|-------|------|----------|
| 1 | **Kit Publication** | registry/IANA policy · packaging · versioning · public release of `standard/` |
| 2 | **Formal** | TLA+ · model checking · property proofs |
| 3 | **Academic** | ablation · measurements · quantitative evaluation vs baselines |
| 4 | **Independent Runtime** | a runtime in another language (TypeScript / C# available here; Go/Java absent) |
| 5 | **External Challenge** | public Kit → unknown team → conformant runtime → PASS (the exit gate) |

**Milestone DoD (one line):** a third-party implementation passes conformance with
**no intervention, no clarification, no hidden knowledge**. Until that is "yes,"
nothing is complete — claims carry an Evidence Level and an independence grade
(G0/G1/G2), never "Done". The internal cold-build (fresh context, Kit-only, in a new
language, logging every question it wanted to ask) is the G1 rehearsal that measures
readiness; each logged question is a Kit defect to close before the real G2 attempt.

## Organization (re-weighted for the two-arm phase, 2026-07)

The platform is mature, so the org shifts from "mostly validation" to **product-led with
a standing validation lab**: **Products 50 · Platform/Standard 30 · Verification 20**
(≈100). Within the 30+20, the destroy-lab discipline persists (more agents break the
platform than extend it — the platform is near-frozen). Realized as wave-batched
workflows; see `verification/evidence/CERTIFICATION_PROGRAM.md` and
`products/README.md`.

## Product Program (P1→P8) and the six-month tracks

Runs in parallel with the Standard Program under **IDR-006** (products consume the
frozen versioned platform; never modify `runtime/` or `standard/`; platform changes via
Platform Change Proposal). Charter + ladder: `products/README.md`. Next-six-months
tracks: **A** External G2 · **B** Certification · **C** Developer SDK · **D** Visual
Designer · **E** Personal AI · **F** Enterprise Runtime · **G** Marketplace. (A+B are
Standard Program; C–G are Product Program — they proceed concurrently.)

## Product release gate — retained (all four MUST hold for GA)

Development of products starts now (IDR-006), but **general availability / a production
release carrying platform-stability guarantees** still requires all four:
**Independent Runtime PASS + External Team (G2) PASS + Certification PASS + Formal
Verification PASS**. Until then, products ship as previews pinned to a platform version,
and a product needing a platform change files a Platform Change Proposal — it never
edits the platform. The new question the whole org answers: *"What products can we now
build that were impossible before?"*

## verification/ layout (evidence, not spec — refined)

```
verification/
  formal/        TLA+ · temporal logic · state-machine models
  runtime/       architecture gates · behaviour · replay · invariants
  certification/ scenarios · conformance · divergence detection
  independent/   Runtime A · Runtime B · Runtime C (multi-impl convergence)
```

## See also
- Milestone Definition of Done: `ENGINEERING_DOCTRINE.md` **ED-004 (Scientifically Proven)**.
- Evidence base: `runtime/docs/reviews/00_ARVES_v2_Global_Readiness_Report.md`.
- Program charter: `runtime/docs/standards/ARVES_v1.1_Standardization_Program.md`.

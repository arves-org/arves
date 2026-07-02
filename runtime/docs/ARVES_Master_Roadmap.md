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

```
PROGRAM 1 — BUILD ........................... ✅ COMPLETE
  Theory · Specification (frozen; ED-001) · Reference Runtime (I1, tag I1-complete)
  · Executable Standards (ACS-001..005) · Conformance Platform · Standard Kit
  · Internal Validation · Evidence OS.
  ACS codec: Evidence Level L3 (reproduced) at independence grade G1.

PROGRAM 2 — EXTERNAL VALIDATION ............. 🟢 CURRENT  (the "G2 Readiness Program")
  Exit criterion (the whole program, one sentence):
    "Can a completely unknown engineering team, using ONLY the ARVES Standard Kit,
     build a conformant runtime without asking us a single question?"
  Milestone: G2 — Independent Standard Validation (this is the next milestone; it is
    NOT "I2"). DoD: third-party implementation → PASS → no intervention, no
    clarification, no hidden knowledge.
  Tracks: 1 Kit Publication · 2 Formal · 3 Academic · 4 Independent Runtime ·
          5 External Challenge  (see below).

PROGRAM 3 — INDUSTRIALIZATION ............... ⬜ starts only after G2 PASS
  I2 Cluster Kernel · I3 Distributed Query · I4 Capability Scheduling
  · I5 Multi-Agent Runtime · Kernel Integration · scale / enterprise

PROGRAM 4 — PRODUCT ......................... ⬜ separate org; gated (4 conditions below)
  SDK · Marketplace · Cloud · Visual Designer · Management Console · Products
```

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

## Research-lab organization (Program 2)

The org is re-shaped from a software team into a validation lab — **more agents break
ARVES than build it**: Destroy 30 · Verification 20 · Independent Runtime 15 · Academic
Review 15 · Certification 10 · Engineering 10 (≈100). Realized as wave-batched
workflows (the "destroy-offices"); see `verification/evidence/CERTIFICATION_PROGRAM.md`.

## Program 4 — Product gate (all four MUST hold, simultaneously)

Products (Program 4) may begin only when: **Independent Runtime PASS + External Team
PASS + Certification PASS + Formal Verification PASS**. At that point the platform is
frozen for consumers; products become customers of the platform (a product needing a
platform change files a Platform Change Proposal — it never edits the platform).

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

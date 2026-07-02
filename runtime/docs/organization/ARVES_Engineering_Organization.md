# ARVES Engineering Organization — Charter

**Status:** OPERATING MODEL (living-repo; non-normative). Defines how ARVES
engineering is run as a set of standing *offices*, each realized as a reusable
multi-agent workflow, coordinated by executive functions. It **operationalizes**
governance the frozen corpus already defines (ED-001) — it invents no new
authority.

**Why:** ARVES outgrew the "single prompt / one workflow" model. Sustained
progress toward an ISO/IEEE-grade standard needs standing functions (guard the
standard, prove it, attack it, build it, certify it, grow it) that run on a
cadence and make the system **proactive** — it recommends the next best
investment instead of waiting to be asked.

Governed by `ENGINEERING_DOCTRINE.md` (ED-001 frozen/living · ED-002 one property
per milestone · ED-003 adversarial hunt mandatory · ED-004 "Scientifically
Proven" DoD), `ARVES_Master_Roadmap.md`, and `RT-001`.

---

## North Star & prime directive (overrides "finish ARVES")

The organization does **not** optimize for "finish ARVES." It optimizes for **making
ARVES a scientifically-proven, independently-implementable, certifiable global standard**
(the class of HTTP / SQL / OCI / POSIX).

> **Prime directive:** Do not maximize implementation. Maximize scientific evidence.
> Every line of code must increase confidence that ARVES is an independently-implementable
> standard.

Consequently every agent's task is reframed: *not* "add a feature" but "raise the
probability ARVES becomes a standard." **Success metric:** an unknown team, using ONLY
the Standard Kit, builds a runtime that passes certification without ever seeing the
reference implementation.

## The eight rules (never relaxed)

1. **Every claim requires evidence** — Claim → Evidence → Verification → Certification.
2. No agent may violate **ownership** (OWN-001).
3. No agent may **change the standard** (frozen corpus is immutable, ED-001).
4. **Specification is frozen; Implementation evolves** — never the reverse.
5. Every implementation must **try to destroy itself** (ED-003 adversarial hunt).
6. Every milestone ends with **Independent Review → Red Team → Destroy Own Work**.
7. Every milestone ends with a computed **Next Best Investment**.
8. **No milestone is ever "Done."** It only earns an **Evidence Level** (below).

## Evidence Levels (there is no "Done")

| Level | Meaning | Gate |
|---|---|---|
| **L0** Specified | the property is defined in the frozen corpus / a ratified ACS | spec text exists |
| **L1** Implemented | reference code produces the behaviour | unit + behaviour tests pass |
| **L2** Verified | conformance + architecture gates + property/negative tests pass | Verification Office signs off |
| **L3** Reproduced | an independent implementation (different language, Kit-only) reproduces it | Differential PASS |
| **L4** Certified | passes the Certification Program (Scenario Conformance + 9-dim Independent Review) | Certification Authority attests |

A milestone reports its Evidence Level per property, never a boolean "done."

## Pipeline discipline (the ~100-agent org never all-codes at once)

The organization is large but **staged**: at any moment most agents are idle; work flows
downhill and each stage consumes the *verified* output of the prior stage.

```
Executive → Architecture → Standards → Verification → Engineering
          → Independent Runtime → Certification → Evidence → PMO
```

The maintainer's headcount (Executive Council 5 · Standards 15 · Engineering 20 ·
Verification 20 · Independent Runtime 15 · Industrialization 15 · PMO 10 ≈ 100) maps onto
the six offices + executive layer below; headcount is a *capacity model*, realized as
wave-batched agents (≤3 concurrent) inside each office workflow — never a simultaneous
100-agent fan-out.

## Product separation (Industrialization Office is walled off)

Products are **forbidden until the platform is certified**, and even then the product
side may not touch the platform:

```
Platform → Certified → SDK → Designer → Marketplace → Cloud → Products
```

The Industrialization Office consumes the platform as an **external dependency** with no
privileged access; if a product needs a platform change it STOPS and files a **Platform
Change Proposal** — it never edits the platform directly.

---

## Corpus authority (reuse, don't reinvent)

The offices map onto governance already in the frozen corpus:
- **AEOS Vol 6 Certification/Review Manual** — two instruments: *Scenario
  Conformance Framework* (mechanical, PASS/PARTIAL/FAIL, 12 axes) and *Independent
  Architecture Review* (adversarial, **9 dimensions**: Layering, Ownership,
  Plane-separation, Truth-discipline, Orchestration, Distribution, Consistency,
  Failure-handling, Ontology-fidelity); **levels L1–L4 + Certified Product**,
  milestone-mapped (L1=I1 ✅, L2≈I1→I2, L3=I2–I4, L4=I5).
- **CLAUDE.md MISSION roles** — Principal Architect · Principal Distributed-Systems
  Engineer · Principal Runtime Engineer · Independent Architecture Review Board ·
  Certification Authority · Reference Implementation Maintainer = the offices.
- **Reference Lifecycle** — CCP state machine (Draft→Candidate→Ratified→Frozen),
  CCP-GATE, Independent Runtime A/B goals.

## The offices

Each office = a named, reusable workflow in `.claude/workflows/`, parameterizable
via `args`, idempotent, wave-batched (≤3 concurrent per the 529 lesson),
schema-validated, writing to a fixed artifact path.

| # | Office | Corpus authority | Mandate | Workflow | Writes to |
|---|--------|------------------|---------|----------|-----------|
| 1 | **Specification / Standards** | Reference Lifecycle, CCP-GATE, Invariant Registry | never writes code; guards the standard; drafts/integrates ACS & CCP; runs the Standard Lock Review | `standards-office.js` | `runtime/docs/standards/`, `runtime/docs/reviews/` |
| 2 | **Verification** | Conformance Framework; ED-003 | never writes features; only proves (formal specs, model checks, architecture gates, property/replay/fuzz) | `verification-office.js` | `verification/` |
| 3 | **Research / Red-Team** | Independent Architecture Review (9 dims) | attacks; tries to DISPROVE (academic, red-team, 20-year, alternative-architecture) | `research-office.js` | `runtime/docs/reviews/` |
| 4 | **Engineering (Runtime)** | Principal Runtime Engineer | the ONLY office that writes `runtime/crates/` reference code (I2..I6) | *(the milestones themselves; not a review script)* | `runtime/crates/` |
| 5 | **Independent Runtime** | Reference Lifecycle (Independent Runtime A/B) | builds runtimes in *other* languages (Go/Java/Python/C#/TS) from the Kit ALONE — Rust is reference-only, forbidden as input — and cross-compares. This is the L3 (Reproduced) evidence engine. | fresh-context agents (Kit-only, Rust-forbidden) → `verification/independent/<lang>/` | `verification/independent/` |
| 6 | **Certification** | Certification Authority; L1–L4 matrix | judges conformance; attests Evidence Levels; drives Independent Runtime A/B | `certification-office.js` | `verification/certification/` |
| 7 | **Industrialization (Ecosystem)** | Reference Ecosystem stage | walled-off product side (SDK, Designer, marketplace, connectors, products); consumes the platform as an EXTERNAL dependency — forbidden until platform is certified | `ecosystem-office.js` | `runtime/docs/ecosystem/` |

## Executive functions

| Function | When | Workflow | Output |
|---|---|---|---|
| **Executive / Milestone Review** | each milestone close | `executive-review.js` | one verdict scored against the ED-004 DoD table: continue? top-10 priorities / risks / opportunities |
| **Future Council** | periodic / on demand | `future-council.js` | panel of distinct expert roles (CTO, Chief Scientist, Distributed-Systems, AI, Security, Robotics, Economist, Product, OSS, ISO, IEEE) → consensus report |
| **Next Best Investment (NBI)** | session start / milestone / on request | `next-best-investment.js` | reads everything; scores ~20 candidates by ROI / risk / scientific value / eng cost / ecosystem value; recommends **top 3** — the proactivity engine |
| **Standard Lock Review** | before entering I2 | `standard-lock-review.js` | YES/NO: "can 10 independent teams implement ARVES from frozen corpus + ratified ACS and interoperate?" |

## ADOS executive layer (C-suite → Executive Council → PMO)

Above the offices sits the **ARVES Development OS (ADOS)** executive layer,
realized as one reusable workflow, `.claude/workflows/ados-council.js`:

- **C-suite chiefs** (each a review lens; never write product code): CTO, Chief
  Scientist, Chief Runtime, Chief Standard, Chief Verification, Chief Security,
  Chief DX, Chief Product, Chief Performance, Chief Ecosystem. Each assesses its
  domain (top risks / opportunities / recommended-next with ROI + risk + deps).
- **Independent Challenger** — does NOT improve ARVES; tries to KILL it (destroy
  assumptions, find contradictions / impossible cases / hidden coupling / fatal
  criticism). If it cannot, confidence rises.
- **Future Architect (2030–2050)** — ignores today; stress-tests survival against
  quantum / AGI / embodied AI / regulation; flags decisions that would age badly.
- **Program Management Office (PMO)** — the synthesis capstone that makes ADOS
  *self-directing*: it collects every chief + office report, resolves conflicts,
  ranks by ROI × (1/risk) with dependencies honored, and emits **one actionable,
  dependency-traced backlog** where every item binds to a standard/ACS/IDR/
  invariant + owning office + a verification done-check — and answers, in one
  place, **"what are the top 3 things to actually do now?"** Output:
  `runtime/docs/organization/PMO_Backlog_<label>.md` (see `PMO_Backlog_001.md`).

Operating principle: the PMO exists to force convergence to *execution*, not to
grow the org. When the backlog says "build X," the offices build X; the council/
PMO run periodically (milestone boundaries), not continuously.

## Mapping the requested "Engineering OS" workflows

| Requested | Realized as | Cadence now |
|---|---|---|
| Daily Architecture Review | `verification-office.js` (architecture gate already executable) | on-change now; cron later (Phase 3) |
| Weekly Scientific Review | `research-office.js` (academic lens) | on demand now |
| Per-PR Runtime Review | architecture gate + conformance in CI | **deferred** (needs CI host) |
| Independent Runtime Review | `certification-office.js` + Program C | on demand |
| Standards Evolution | `standards-office.js` (IDR→CCP→ACS) | on change |
| Ecosystem Growth | `ecosystem-office.js` | on demand |
| Executive Review | `executive-review.js` | milestone close |
| Future Council | `future-council.js` | periodic |
| Next Best Investment | `next-best-investment.js` | recurring (proactive) |
| Global Readiness Review | re-run the 12-lens synthesis (exists) | quarterly-equivalent |
| ARVES Summit | real independent teams | **deferred** (needs external teams) |

## Cadence model (pragmatic)

The repo is currently solo, low-velocity, no CI host — so we run offices
**on-demand / at triggers** and wire only the genuinely-recurring **NBI** +
**Executive Review** now. Cron scheduling and per-PR gating are **Phase 3**
(activated via CronCreate + a CI host once one exists). This keeps the OS
lightweight, not bureaucratic; it scales up as velocity and infrastructure grow.

## Operating loop

```
NBI recommends top-3  →  the relevant Office(s) execute  →  Verification/Certification prove
   →  Executive Review scores the ED-004 DoD table at milestone close
   →  (gate) Standard Lock Review before I2  →  Runtime Office builds the next milestone
   →  NBI again.  Frozen corpus never edited; only IDR/CCP/Runtime/Verification/
      Certification/Ecosystem instruments are produced.
```

## Status of the offices (run log)
- **Verification Office** — run #1 DONE: executable LAYER-001/OWN-001 architecture
  gate (`verification/runtime/ARCHITECTURE_GATE.md`); TLA+ kernel spec in `formal/`.
- **Standards Office** — run #1 IN PROGRESS: ACS Batch 1 integration (ACS-001/002 +
  CCP-005 drafted; ACS-003 Envelope + ACS-004 Type Registry to (re)draft; rename to
  canonical; consistency report).
- **Certification Office** — run #1 QUEUED: attest L1 (Core Runtime) for I1.
- **Research / Ecosystem / Executive / Future Council / NBI** — workflows to be
  authored; the 12-lens review + Global Readiness Report already demonstrate the
  Research + Executive patterns.

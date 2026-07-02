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
SPECIFICATION ERA .......................... ✅ COMPLETE  (frozen; ED-001)
  Foundation · UCI · UCS · Reference Architecture · Reference Runtime Model
  · Certification · AEOS · Freeze

REFERENCE IMPLEMENTATION
  I1 Runtime Core .......................... ✅ COMPLETE  (tag I1-complete)

STANDARDIZATION PROGRAM (v1.1) ............. 🟡 CURRENT
  ARVES Core Standards (ACS) — CCP Batch 1
    ACS-001 Universal Content Identity ..... ✅ draft (real vectors)
    ACS-002 Canonical Serialization ........ 🟡 drafting
    ACS-003 Canonical Envelope ............. 🟡 drafting
    ACS-004 Universal Type Registry ........ 🟡 drafting
    ACS-005 Normative Language + Glossary .. 🟡 drafting
    Integration Review ..................... ⬜ (must PASS)
  Verification Program
    TLA+ formal spec ....................... 🟡 drafting
    Architecture Gates (LAYER/OWN, CI) ..... ⬜ next (cheapest, highest-leverage)
    Model checking ......................... ⬜
    Conformance population ................. ⬜
    Formal semantics (Truth, Entity) ....... ⬜
  ARVES v1.0 Standard Lock Review .......... ⬜ GATE before I2 (see below)

IMPLEMENTATION ERA
  I2 Cluster Kernel · I3 Distributed Query · I4 Capability Scheduling
  · I5 Multi-Agent Runtime · I6 Reference Products

CERTIFICATION
  Independent Runtime · Certification Program · Third-party Runtime

ECOSYSTEM
  Marketplace · SDK · Products
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

## Pre-I2 gate (sequence — do NOT enter I2 before this passes)

```
ACS-001 → ACS-002 → ACS-003 → ACS-004  →  Integration PASS
                                        →  TLA+ + Architecture Gates PASS
                                        →  ARVES v1.0 Standard Lock Review PASS
                                        →  I2 Cluster Kernel
```

Rationale: building I2 replication before the byte-exact interop surface is
locked would force a costly I2 redo. Lock the standards, prove them, THEN build.

## ARVES v1.0 Standard Lock Review (new gate)

After CCP Batch 1 + Verification kickoff, run ONE final independent audit that
answers a single question:

> **"Starting today, can 10 independent teams implement ARVES — from the frozen
> corpus + the ratified ACS set alone — and interoperate / cross-certify?"**

Method: an adversarial, spec-only re-run of the Independent Runtime Challenge
(P01) + Independent Implementation (P04), now measured against the ratified ACS
set and the differential-conformance tier. A "yes" unlocks I2 on solid ground; a
"no" returns specific ACS gaps to close first. (ED-003 adversarial-hunt applies.)

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

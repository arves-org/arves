# ARVES — Master Roadmap

**Status:** authoritative living-repo roadmap (ED-001: progress lives here + git
tags, never in the frozen corpus). Supersedes ad-hoc milestone notes for
cross-era planning.

---

## Three eras

| Era | Question | Status |
|-----|----------|--------|
| **1. Foundation** | *What is ARVES?* | ✅ COMPLETE (Specification Era frozen + AEOS) |
| **2. Standardization** | *Can ARVES be implemented universally & identically by independent teams?* | 🟡 **CURRENT — the most critical era** |
| **3. Industrialization** | *Can ARVES run on millions of nodes?* | ⬜ later (Implementation Era I2–I6, enterprise, scale) |

The decisions taken in Era 2 (content identity, serialization, envelope, type
registry, formal verification) bind every implementation for the next 10–20
years. This is the highest-leverage era of the whole project.

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

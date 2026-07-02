# ARVES v1.1 Standardization Program — Charter

**Status:** OPEN (living-repo program charter; non-normative until its outputs are
ratified through CCP-GATE). **Opened:** at the close of the 12-lens review.
**Trigger:** `runtime/docs/reviews/00_ARVES_v2_Global_Readiness_Report.md`
(synthesis of 12 independent reviews P01–P12).
**Governing doctrines:** ED-001 (frozen corpus immutable), ED-002 (one property
per milestone), ED-003 (adversarial hunt mandatory), RT-001 (interface evolution).

---

## Why this program exists

Twelve independent architect lenses **converged** on the same ~8–10 gaps (not a
long tail): content-addressing (7/12), canonical serialization (5), type schemas
(4), formal specification (4), decision-trace schema (4), conformance population
(3), normative language/glossary (3). Independent convergence is the strongest
possible signal: these are not opinions, they are **the** gaps.

**Verdict:** ARVES is *NOT-READY* for independent implementation / ISO-IEEE
submission today — **but every gap is additive and closes without reopening the
freeze** (via CCP Amendments that activate already-reserved semantics + a
Verification Program). The architecture is sound; the evidence and the byte-exact
interoperability surface are missing. This is a **formalization** program, not a
redesign.

**Roadmap change:** deep I2 (and I3+) is **paused** until the interop surface is
locked. Doing I2 replication before serialization/content-addressing are
normative would force a costly redo of I2 later. New order:

```
I2 design (done) → [ARVES v1.1 Standardization Program] → resume I2 implementation → I3 …
```

## Scope — four goals (sequenced; interoperability first)

Two runtimes that do not speak the same bytes cannot interoperate, cannot be
cross-certified, and make every later proof meaningless. Hence Goal 1 first.

### Goal 1 — Universal Interoperability  *(FIRST; closes R-01, R-02, R-03, R-04)*
Every conformant implementation must speak the **same bytes**. Delivered as the
**ARVES Core Standards (ACS)** series — the "TCP/IP RFCs" of ARVES — each a
CCP-gated normative addition (amendment + ≥1 conformance scenario per CCP-GATE):

| ACS | Title | Closes | Note |
|-----|-------|--------|------|
| **ACS-001** | Content Addressing (pre-image, canonicalization, digest) | R-01 / ORCH-004 | **draft first = CCP-001**; the common root of Git/OCI/Docker/Nix/IPFS/Bazel |
| ACS-002 | Canonical Serialization | R-02 | ARVES's "Protocol-Buffer moment" |
| ACS-003 | Wire Envelope | R-02 | reconcile Vol 9 v1 vs ARVES-21 envelopes |
| ACS-004 | Type Registry | R-03 | executable form of the ontology |
| ACS-005 | Schema Registry | R-03 | `uci.*` schemas + versioning |
| ACS-006 | Deterministic Fingerprint | R-04 | Runtime Fingerprint field set |
| ACS-007 | Canonical Hash | R-01 | mandatory algorithm(s), multihash-style |
| ACS-008 | Reference Encoding | R-02 | worked example payloads |
| ACS-009 | Replay Format | R-04 | decision-trace record schema + ordering |
| ACS-010 | Compatibility Rules | governance | version negotiation / evolution |

### Goal 2 — Formal Foundation  *(closes R-05; what Academic review requires to PASS)*
Formal semantics + machine-checked proofs, as **evidence** (lives in `verification/`,
never in the frozen corpus). Targets the three academic gaps: **Truth** semantics
(Truth Algebra / Lattice / Transition), **Cognitive-Entity** model theory
(category theory / coalgebra / state-transition), and **distributed proofs**
(no-split-brain, linearizability, deterministic replay). First, lowest-cost,
highest-leverage win: a **build-time architecture gate** proving LAYER-001
(acyclic downward-only crate graph) and OWN-001 (single write-path to truth) —
statically, in CI, today.

### Goal 3 — Executable Conformance  *(closes the divergence-blindness gap)*
Populate the Scenario Conformance Framework with real scenarios; add a
**differential-conformance tier** that pins byte-exact digests/envelopes/traces so
two independent runtimes must AGREE (depends on Goal 1). Reference outputs +
divergence detection. This is what makes "PASS" a real measurement.

### Goal 4 — Certification & Governance  *(closes R-06 + governance vacuum)*
RFC 2119 normative-language convention; a Terms & Definitions glossary; per-clause
requirement IDs + traceability; the conformity-assessment scheme (ISO/IEC 17065
style: suite/runtime authorship separation, accreditation, appeals,
decertification); IDR/CCP change-control traceability.

## Sequencing

1. **Now:** open **CCP-001 / ACS-001 Content Addressing** (this program's first
   output; unanimous #1; gates ACS-002/004 and the second runtime). Ship the
   **build-time LAYER/OWN architecture gate** in parallel (cheapest proof; Goal 2).
2. **Interop batch:** ACS-002 Serialization → ACS-003 Envelope → ACS-004/005 Type
   & Schema Registry → ACS-006/009 Fingerprint & Replay Format. Each via CCP-GATE.
3. **Formal batch (Goal 2):** TLA+ formal-spec companion for the 7 registered
   invariants (safety + liveness under fairness); begin Truth/Entity semantics.
4. **Conformance batch (Goal 3):** populate one full vertical slice end-to-end
   (e.g. `Person`) as the template, then differential conformance.
5. **Certification & Governance (Goal 4):** normative-language + glossary + scheme.
6. **Resume I2** (Replication) on the locked interop surface; then I3+.

The five parallel *execution* programs (A Reference Runtime, B Verification, C
Independent Runtime, D Certification, E Ecosystem) remain the delivery vehicles;
the four goals above are what v1.1 must complete before deep I2/I3.

## What this program will NOT do
- Not edit any frozen `.docx` (ED-001). ACS outputs are **new** CCP-gated
  documents; evidence outputs live in `verification/`.
- Not add features. Every output is interoperability, formalization, conformance,
  or governance.
- Not skip CCP-GATE: no ACS/CCP is "ratified" without at least one conformance
  scenario.

## Directory layout (living repo)
- `runtime/docs/standards/` — ACS draft standards + CCP amendment proposals.
- `verification/` — evidence (mathematics/ semantics/ proofs/ model-checking/
  runtime-verification/ certification/ benchmarks/ reproducibility/
  independent-implementations/). *Evidence, not specification.*
- `runtime/docs/reviews/` — the 12 lens reports + the Global Readiness Report.

## Definition of Done (v1.1)
An independent team can build a runtime from the frozen corpus **+ the ratified
ACS/CCP set** that **byte-level interoperates** with the reference runtime and
**passes the same populated conformance suite**, with the core invariants stated
as machine-checked formal properties. That is the day ARVES becomes a real
standard.

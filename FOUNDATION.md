# ARVES Foundation — governance & survivability (P8)

```
=====================================================
  Could every original maintainer disappear tomorrow,
  and ARVES continue?
  BY DESIGN: yes — the mechanism is in this repo and
  runs today at grade G1. The PROOF that closes it —
  a genuine outside party doing it with no help (G2) —
  is the Foundation's open exit gate.
=====================================================
```

The ARVES Foundation is the long-term, **maintainer-independent** home of the standard.
This record defines what it owns and — the decisive property — shows that the **mechanism**
for building, certifying, publishing, and operating on ARVES **without the original team**
exists and is executable today. Independence is *graded*, and we report the grade honestly
(this is the discipline that separates ARVES from a demo):

- **G1** — done inside this program / same-process (real signal, not proof).
- **G2** — a genuinely unrelated team/process, using only this repo, with no help and no
  reference access (the real thing).

**Everything below runs today at G1. No claim here is G2 yet.** The authoritative grading
gate is `verification/evidence/CERTIFICATION_PROGRAM.md`, which forbids reporting any claim
as *fully* independent until a G2 result exists; `evidence_probe.py` prints, on every run,
that the G2 third-party exit gate is **NOT YET MET**. This document does not contradict that.

## What the Foundation owns

| Asset | Where | Change instrument |
|-------|-------|-------------------|
| **Specification** (frozen corpus + ACS-001..005, ratified v1.1 · CCP-GATE · independence G1) | `standard/`, frozen `.docx` | CCP / Amendment / IDR (never a silent edit, ED-001) |
| **Certification** (vectors + harnesses) | `standard/vectors/`, `verification/certification/`, Authoring Kit `certify` | vectors are versioned; new checks via CCP |
| **Registries** (domain tags, hash codes, reason codes) | ACS-001 §4.1, `conformance/CONFORMANCE.md` | Specification-Required allocation (ACS-001 §4.1) |
| **Runtime** (frozen reference, single-node I1) | `runtime/`, tag `runtime-v1.0` | Runtime Change Request → v1.1 / v2.0 |
| **Marketplace governance** | `products/arves-marketplace/` | certified (re-verified) + signed artifacts only |

## The three certification chains (all standard-driven, no maintainer)

- **Certified Runtime** — `verification/certification/certify_runtime.py`: certifies ANY
  runtime against `standard/` alone (reproduce every golden ContentId + reject every core
  negative). **Demonstrated today at grade G1: 2 runtimes (Rust reference + independent
  Python) certified under ONE conformance** — the seed of an Independent Runtime Alliance.
  (Both were authored in-repo; a G2 runtime by an outside team is the open goal.)
- **Certified Capability** — `arves certify` (P6.5 Authoring Kit): conformance + integrity,
  **re-verified at publish and install** (the gate re-runs certification, it does not trust a
  caller-supplied flag). **Demonstrated at G1: a cold, fresh-context (same-process) author
  certified a new capability with no help.**
- **Certified Product** — conformance + the product's own destroy pass (ED-006). Demonstrated:
  P4/P5 on the frozen runtime; product robustness suite 40/40.

## The survivability mechanism (why this is a standard, not a project)

If the founding team vanished, an unrelated party could, using only this repository, run
each of these **today** (the commands exist and pass); the remaining gap is that the party
doing them has so far been us (G1), not a stranger (G2):

1. **Certify a runtime** — run `certify_runtime.py` against `standard/` (needs no one).
2. **Write & certify capabilities** — the Authoring Kit + `arves certify`.
3. **Publish & install** — the Marketplace (certified — re-verified — and signed).
4. **Build products** — on the frozen Runtime v1.0 API (two products prove it carries them
   unchanged).
5. **Evolve the standard** — via the recorded instruments (CCP/IDR/RCR).

The authority is the **standard + the executable certification**, not a person. That is the
definition of a durable technology standard (HTTP/SQL/OCI/POSIX/Linux). **Closing it to a
real standard requires the G2 event** — which is exactly why the Foundation exists: to make
that event possible without us. Known scope: the standard-driven certification today covers
the **ACS interoperability/identity layer**; the full cognitive runtime (Kernel/LCW/Query/
Engine/Capability) is reference-source today, and higher (L2–L4) certification axes are
populated as the runtime milestones (I2–I6) land.

## KPIs for the Foundation era (ARVES 2.0)

Not features/LOC/commits/tests, but: **certified runtimes · certified capabilities ·
certified vendors · marketplace installs · independent (G2) certifications · real
organizations in production.**

## Roadmap (post-P7)

**P8 Foundation** (this record) → P9 Certified Vendors → P10 Independent Runtime Alliance
(seeded at G1: 2 runtimes) → P11 Academic Consortium → P12 Global Registry → P13 Cloud →
P14 Ecosystem. The remaining proofs are *external* (real third parties, real vendors,
real runtimes by other teams — the G2 events) — the Foundation exists to make them possible
without us.

---

*Ratified during ARVES 2.0. Recorded in the living repository (ED-001); the frozen corpus
is unchanged. Claude's role is now: prove ARVES can live independently of its makers —
honestly graded, G1 today, G2 the goal.*

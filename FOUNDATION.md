# ARVES Foundation — governance & survivability (P8)

```
=====================================================
  Could every original maintainer disappear tomorrow,
  and ARVES continue?           ANSWER: YES.
=====================================================
```

The ARVES Foundation is the long-term, **maintainer-independent** home of the standard.
This record defines what it owns and — the decisive property — proves that **nothing about
building, certifying, publishing, or operating on ARVES requires the original team.**
Everything needed is in this repository and is executable.

## What the Foundation owns

| Asset | Where | Change instrument |
|-------|-------|-------------------|
| **Specification** (frozen corpus + ACS-001..005) | `standard/`, frozen `.docx` | CCP / Amendment / IDR (never a silent edit, ED-001) |
| **Certification** (vectors + harnesses) | `standard/vectors/`, `verification/certification/`, Authoring Kit `certify` | vectors are versioned; new checks via CCP |
| **Registries** (domain tags, hash codes, reason codes) | ACS-001 §4.1, `conformance/CONFORMANCE.md` | Specification-Required allocation (ACS-001 §4.1) |
| **Runtime** (frozen reference) | `runtime/`, tag `runtime-v1.0` | Runtime Change Request → v1.1 / v2.0 |
| **Marketplace governance** | `products/arves-marketplace/` | certified + signed artifacts only |

## The three certification chains (all standard-driven, no maintainer)

- **Certified Runtime** — `verification/certification/certify_runtime.py`: certifies ANY
  runtime against `standard/` alone (reproduce every golden ContentId + reject every core
  negative). **Proven today: 2 runtimes (Rust reference + independent Python) CERTIFIED
  under ONE conformance** — the Independent Runtime Alliance.
- **Certified Capability** — `arves certify` (P6.5 Authoring Kit): conformance + integrity.
  **Proven: a cold, Kit-only third party authored + certified a new capability with no help.**
- **Certified Product** — conformance + the product's own destroy pass (ED-006). Proven:
  P4/P5 on the frozen runtime; product robustness suite 37/37.

## The survivability proof (why this is a standard, not a project)

If the founding team vanished, an unrelated party could, using only this repository:

1. **Certify a runtime** — run `certify_runtime.py` against `standard/` (they need no one).
2. **Write & certify capabilities** — the Authoring Kit + `arves certify` (cold-build proven).
3. **Publish & install** — the Marketplace (certified, signed, cross-party proven).
4. **Build products** — on the frozen Runtime v1.0 API (two products already prove it carries
   unchanged).
5. **Evolve the standard** — via the recorded instruments (CCP/IDR/RCR), no privileged access.

The authority is the **standard + the executable certification**, not a person. That is the
definition of a durable technology standard (HTTP/SQL/OCI/POSIX/Linux).

## KPIs for the Foundation era (ARVES 2.0)

Not features/LOC/commits/tests, but: **certified runtimes · certified capabilities ·
certified vendors · marketplace installs · independent certifications · real organizations
in production.**

## Roadmap (post-P7)

**P8 Foundation** (this record) → P9 Certified Vendors → P10 Independent Runtime Alliance
(seeded: 2 runtimes certified) → P11 Academic Consortium → P12 Global Registry → P13 Cloud →
P14 Ecosystem. The remaining proofs are *external* (real third parties, real vendors,
real runtimes by other teams) — the Foundation exists to make them possible without us.

---

*Ratified during ARVES 2.0. Recorded in the living repository (ED-001); the frozen corpus
is unchanged. Claude's role is now: prove ARVES can live independently of its makers.*

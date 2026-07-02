# ARVES Certification (process)

How a runtime — reference, independent, or third-party — becomes a **certified**
ARVES implementation. Defined by the AEOS Certification/Review Manual; summarized
here so the Kit is self-contained.

## Two instruments
1. **Scenario Conformance** (mechanical): pass the conformance procedure
   (`../conformance/CONFORMANCE.md`) — ACS golden vectors — plus the 12 Scenario
   axes as they are populated. Verdict per vector/scenario: PASS / PARTIAL / FAIL.
2. **Independent Architecture Review** (adversarial, arms-length — NOT the
   implementer): the 9 dimensions — Layering, Ownership, Plane-separation,
   Truth-discipline, Orchestration, Distribution, Consistency, Failure-handling,
   Ontology-fidelity.

## Levels (cumulative)
- **L1 Core Runtime** — single-node: Kernel owns truth, Engine pure, Query
  read-only, OWN-001 holds. *(The reference runtime holds an L1
  GRANTED-with-conditions attestation on record with the ARVES Certification
  Authority; a third party does not need it — you certify against this Kit alone.)*
- **L2 Cognitive Control** · **L3 Distributed** · **L4 Multi-Agent** ·
  **Certified Product**.

## Decision matrix
- All target-level dimensions PASS + no registered-invariant FAIL → **GRANTED**.
- Some documented, non-blocking PARTIAL → **GRANTED-with-conditions**.
- Any registered-invariant FAIL or any dimension FAIL → **WITHHELD**.
- Spec gap discovered → route to CCP; re-certify after resolution.

## Third-party path
Download this Kit → implement → pass the conformance procedure → submit for an
arms-length architecture review → receive a level attestation. Certification is
against the frozen spec + this Kit, **never** by comparison to reference source —
that independence is the whole point.

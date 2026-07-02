# ARVES Reference Runtime — Engineering Doctrine

Implementation-era doctrines that govern *how* the reference runtime is built.
They live in the **Living Engineering Repository** (git), never in the frozen
specification corpus. Companion rule: `RT-001_Reference_Runtime_Interface_Evolution.md`.

---

## ED-001 — Frozen Specification vs Living Engineering Repository

> The frozen specification is **immutable**. The engineering repository is
> **mutable**. The two are never mixed.

| Frozen (immutable, `.docx` corpus) | Living (mutable, git repo) |
|---|---|
| Specification · Standards · Ontology | Source code |
| Baseline (milestone *scope*) | `IMPLEMENTATION_PROGRESS.md` (milestone *progress*) |
| Freeze Record · IDRs · Amendments | git tags · git history · CHANGELOG |
| Certification / Conformance definitions | Retrospectives · design docs · this doctrine |

**Consequences:**
- Milestone **completion is recorded only in the living repository** (git tag +
  progress tracker + retrospective), NEVER by editing a frozen document.
- The frozen Baseline defines *what the milestones are* (scope); it does not
  track *how far we've got* (progress).
- Changing the frozen corpus requires a Change-Management instrument (CCP /
  Amendment / IDR / next major version) — never a silent edit. Runtime interface
  maturation is governed separately by RT-001 and touches no frozen document.

*Ratified by the maintainer at the close of I1.*

---

## ED-002 — One Fundamental Property per Milestone

> A milestone does not ship features. It **proves one fundamental
> computer-science property**, independently and executably.

| Milestone | Property proven |
|---|---|
| I1 Distributed Runtime | **Persistence** (durable, replayable, recoverable truth) |
| I2 Cluster Kernel | **Replication** (a follower reconstructs identical truth) |
| I3 Distributed Query | **Consistency** (read semantics over replicated truth) |
| I4 Capability Scheduling | **Scheduling** |
| I5 Multi-Agent Runtime | **Distributed Cognition** |
| I6 Reference Products | (composition — real products on the proven base) |

**Consequences:**
- Each milestone's Definition of Done is a *property proof*, not a feature list.
- Later layers build on an already-proven foundation, which keeps the
  architecture legible and makes debugging/verification tractable.
- Guard against scope creep: if a task doesn't advance the milestone's one
  property, it doesn't belong in the milestone.

*Ratified by the maintainer at the close of I1.*

---

## ED-003 — Adversarial Fault Hunt is Mandatory

> Before a milestone is declared complete, its behaviour must be **attacked**,
> not just tested. Break it, then fix it.

- Run a multi-agent adversarial hunt: each agent attacks from a distinct fault
  lens; every finding is independently verified (or refuted) against the code.
- Fix every **confirmed** defect and add a fault-injection regression proof;
  record refuted claims as "considered and shown safe".
- "Tests pass" is necessary but not sufficient — happy-path green can hide silent
  failure modes (I1.7's silent partial-truth recovery was found this way, not by
  the passing happy-path suite).

*Ratified by the maintainer at the close of I1 (first applied in I1.7).*

---

## ED-004 — Milestone Definition of Done is "Scientifically Proven", not "Complete"

> A milestone is done when its property is *scientifically proven and
> independently reproducible*, not merely when the code compiles and tests pass.

Every milestone (and the standardization program) closes with this table
answered — it IS the Definition of Done:

| Question | Bar |
|---|---|
| Specification complete | the governing spec/ACS exists and is frozen or ratified |
| Runtime complete | the reference runtime implements it |
| Behaviour proven | executable behaviour proofs pass |
| Architecture gates pass | LAYER/OWN (and peers) enforced at build time, not just documented |
| Formal model exists | the core properties have a machine-checkable formal statement |
| Conformance pass | populated conformance scenarios pass |
| Independent implementation possible | a different team could build it from the spec + ACS alone and interoperate |

The last row is the ultimate bar: **the standard is real only when independent
implementations converge.** Ratified at the opening of the Standardization era.

---

## ED-005 — Prove the architecture; the single KPI is Independent Runtime

> ARVES no longer lacks knowledge; it lacks **evidence**. The motto is
> **"Prove the architecture,"** not "Define the architecture."

- **Single KPI — Independent Runtime.** Judge every decision by one question:
  *does this make a second, fully-independent implementation (from the frozen spec
  + ratified ACS alone) that produces the SAME conformance results easier?* If not,
  it is probably not a priority.
- **NO NEW DOCUMENTS** unless implementation, verification, or certification
  requires them. Produce code, tests, proofs, and byte-exact vectors — not more
  prose or org. (Codifying a binding rule in this doctrine is permitted; new
  elaborate documents are not.)
- **Evidence-first.** Each milestone ends with a claim→evidence table
  (claim | evidence kind | ✅/🟡/❌); the PMO manages evidence, not backlog volume.
- **Maturity model:** Specified → Implemented → **Verified** → **Reproduced** →
  **Independent** → Standard. Verified (mechanized proof) and Reproduced (a second
  runtime matches byte-for-byte) are the hard, currently-missing levels.

*Ratified after the L1 attestation + Standard Lock Review (Lock = CONDITIONAL):
the gap to a standard is evidence and a second implementation, not more design.*

---

## ED-006 — Destroy-First, Robustness-Gated Development (the permanent cycle)

> Once the architecture exists, the most valuable act is not adding a feature — it is
> trying to **break** the system. **Don't build new features. Try to prove that every
> existing feature is wrong. If you fail to break it, produce evidence that it is
> correct. Only then allow the next feature to exist.** (The path of Linux, PostgreSQL,
> Kubernetes, AWS — longevity comes from robustness, not feature count.)

**The cycle (no feature skips a stage):**

```
Idea → Implement → 100-Agent Destroy →  ┌ Broken  → Repair ┐
                                        └ Survived → Evidence ┘
   → Regression Tests → Conformance → Performance → Security
   → Independent Review → FREEZE → Next Milestone
```

**Robustness offices (the destroy lenses):** Red Team (break it, no other goal) ·
Security (memory corruption, replay, DoS, overflow, injection, race, deadlock, resource
exhaustion, privilege escalation, supply chain) · Reliability (power loss, crash,
restart, disk full/slow, network loss, clock skew, corrupt WAL/snapshot) · Scalability
(1 → 1M) · Correctness (replay, idempotency, ordering, consistency, determinism, truth,
evidence) · Performance (latency, CPU, memory, GC, cache, allocation).

**Testing disciplines (escalating):** chaos engineering (kill -9, disk/mem/cpu pressure,
latency, bit-flips, dropped/duplicate/out-of-order packets, clock jumps, leader/follower
crash, partition) · property-based testing (1M random cognitive states → replay
deterministic?) · differential testing (Rust ↔ Python ↔ TypeScript ↔ Go ↔ Java ↔ C# all
agree) · fuzzing (random CBOR/envelope/truth/identity/capability/query/plan) · mutation
testing (remove an `if` → does a test fail?) · formal verification (replay always
deterministic?) · long-running (30-day continuous) · memory-leak (1B ops, memory
stable?) · independent team (a team builds ARVES from the Standard Kit alone, no help).

**KPIs change** — from *features completed* to: **bugs found · bugs fixed · regression
tests added · evidence generated · confidence.** A feature that survives destroy with
evidence + regression tests + conformance/perf/security/independent-review + freeze is
worth more than three unbroken-because-untested features.

*Ratified by the maintainer during the product era: robustness, not feature count, is
what makes a cognitive platform outlive its competitors. Applied first in the
whole-system destroy pass before P4.*

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

# RT-001 — Reference Runtime Interface Evolution

**Type:** Reference Runtime governance rule (implementation-era).
**Status:** RATIFIED (maintainer decision, ratified during I1.6 design review).
**Scope:** The Rust reference runtime (`runtime/`) ONLY. Does not touch the frozen
UCS/UCI corpus.

---

## Rule

> **RT-001.** Reference Runtime interfaces may evolve **only** if they activate
> previously reserved semantics and do **not** invalidate any frozen normative
> specification.

## Rationale

The frozen ARVES specification (the `.docx` UCS/UCI corpus) is permanently frozen
and is never modified by implementation (Engineering Constitution, Non-Negotiable
Rule #1). But the **Rust traits** in the reference runtime are *implementation
contracts*, not the frozen specification. They are allowed to mature — provided
the change only turns a **dormant, already-reserved** contract into an **active**
one, and provided no frozen normative statement is invalidated.

This is the difference between:

- **Specification Change** — altering the frozen UCS/UCI. **Forbidden** (use
  CCP / Amendment / IDR / next major version).
- **Interface Evolution (RT-001)** — activating semantics the frozen spec already
  reserved, at the Rust trait level. **Permitted**, and recorded here.

## Test for admissibility

An interface change qualifies under RT-001 iff ALL hold:

1. The semantics were **already reserved** in the frozen corpus or the runtime
   type surface (i.e., not newly invented).
2. **No frozen normative specification** statement is invalidated.
3. **No architectural layer** is added and **no ownership boundary** moves
   (OWN-001, ORCH-001, LAYER-001 preserved).
4. The change is **recorded** (traceability) with the milestone that made it.

If any fails → it is a Specification Change and must STOP and route through Change
Management, not proceed as Interface Evolution.

## First application — I1.6 Checkpoint Semantics

The `Wal` trait's `snapshot()` marker is replaced by
`install_snapshot()` / `load_snapshot()` / `compact()`, and `earliest()` becomes
meaningful. Admissible under RT-001 because `SnapshotMeta`,
`RecordKind::SnapshotMarker`, `Wal::snapshot`, and `earliest()` were **already
present** in the runtime type surface (reserved for exactly this) — a
**dormant → active** transition, not an invention. Frozen UCS/UCI unchanged;
Kernel remains sole truth owner; Persistence still stores opaque bytes only.

See `I1.6_Checkpoint_Semantics_Design.md` §D (Decision Register).

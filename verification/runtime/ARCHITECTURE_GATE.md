# Architecture Gate — executable LAYER-001 / OWN-001 evidence

**Office:** Verification Office (run #1). **Type:** runtime evidence (executable).
**Artifact:** `runtime/crates/arves-conformance/tests/architecture_gate.rs`.
**Closes:** Global Readiness R-05 / P05 ("LAYER-001 & OWN-001 are cheaply,
statically provable — highest-leverage, lowest-cost proof") and P11 (architectural
purity, enforced not just documented).

## What it turns from documentation into an enforced rule

| Invariant | Documented as | Now enforced as |
|---|---|---|
| **LAYER-001** (downward-only dependencies) | prose in the frozen spec | a `cargo test` that reads every crate's `Cargo.toml`, builds the internal edge set, and FAILS on any edge to an equal/higher layer rank (which also forbids cycles) or any unranked crate |
| **OWN-001** (one owner of truth) | prose | a `cargo test` asserting exactly ONE crate (`arves-kernel`) defines the truth-commit gateway (`pub trait Kernel`) |

The layer ranks encode the frozen ARVES layer stack (foundation < persistence <
consensus < kernel < lcw < query/engine < capability/execution < information-
platform < control-plane < runtime < conformance-harness), spaced for future
insertion. The gate reads files at test time — no external crates, no compilation
of the crate under inspection — so it is fast, deterministic, and dependency-free.

## Result (current workspace)

```
test layer_001_dependencies_are_downward_only ... ok
test own_001_single_truth_commit_gateway ....... ok
test gate_detects_upward_edge .................. ok   (synthetic: proves it bites)
test gate_detects_unranked_crate ............... ok   (synthetic: proves it bites)
test parser_handles_both_dependency_forms ...... ok
```

Real edges verified downward-only: `arves-kernel → arves-persistence`;
`arves-runtime → {arves-kernel, arves-persistence}`; the other 11 crates are
leaves. Sole truth-commit gateway: `arves-kernel`.

## How to run
```
cargo test --manifest-path runtime/Cargo.toml -p arves-conformance --test architecture_gate
```

## Bite proof (why this is real, not decorative)
`gate_detects_upward_edge` feeds the pure checker a synthetic
`arves-persistence → arves-kernel` edge and asserts it is flagged as a LAYER-001
violation; `gate_detects_unranked_crate` asserts a new/unclassified crate fails
the gate. So adding an upward dependency, a dependency cycle, an unclassified
crate, or a second truth-commit gateway will break CI.

## Roadmap / next
- Wire this into CI once a host exists (Phase 3): run on every PR (per-PR Runtime
  Review gate).
- Extend with a SHARD-001 structural check and a Plane-separation check (CP vs DP)
  as those layers gain code in I2+.

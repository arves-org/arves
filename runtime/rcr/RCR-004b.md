# RCR-004b — `acs_validate` line-protocol bin: expose the native Rust semantic validators (Runtime v1.1)

**Status:** Applied (v1.1) · **Type:** additive (new bin + one `pub` schema builder; no existing
behavior/ABI change) · **Gate:** `cargo test --workspace` → **81 passed / 0 failed** (unchanged).
Freeze baseline re-advanced via `freeze_check.py update`.

## Motivation

RCR-004 landed the native Rust ACS-003/004/005 semantic validators in
`arves-conformance::semantic` (they reject all 19 frozen semantic vectors in-crate), but **no bin
drove them**, so the certification harness could not grade the Rust reference over the semantic
tiers — a certify/sound run still reported the Rust semantic arm as *deferred*. That left the
flagship gate attesting only the ACS-002 byte layer (the rank-1 defect: the SOUND-CERTIFIED gate
grades `tier=="core"` only). RCR-004b is the harness-exposure follow-up RCR-004 tracked, and the
prerequisite for the full-surface gate.

## Change (what shipped)

- **New bin `acs_validate`** (`runtime/crates/arves-conformance/src/bin/acs_validate.rs`): the
  semantic analogue of `acs_decode`. Line protocol: stdin `<tier>\t<hex-body>` where `tier ∈
  {envelope, instance, language}`; stdout `ACCEPT` | `REJECT\t<registered-kebab-code>` |
  `ERR\tbad-hex|bad-tier|non-canonical`. It decodes envelope/instance bodies with the frozen
  `decode_canonical`, then runs `validate_envelope` / `validate_instance` (against the frozen
  `uci.fact@1.0` schema) / `check_term_set`, emitting the CCP-006 reason code. The grader still owns
  the truth and compares the code itself, so a hollow adapter cannot game it.
- **`semantic.rs`:** the `uci.fact@1.0` schema builder moved from the test module to a `pub
  uci_fact_schema()` so the bin + the test share one definition (all `instance`-tier vectors are
  `uci.fact` instances). No validator logic changed.

## Scope (honest)

- **IS:** the Rust reference now exposes its full ACS-003/004/005 reject surface over the same
  line-protocol shape as `acs_decode`, so the certification harness (rank 1) can drive it
  inputs-only over all four reject tiers. Additive; no existing public API changed; the ACS-002
  bins/generators are byte-identical.
- **IS NOT:** the gate change itself — extending `verify_runtime_sound.py` / `certify_runtime.py` to
  drive this bin and require all four tiers is the next step (rank 1, a freeze-clean living_fix).
- **Independence unchanged: G1.**

## Verification

- `cargo build -p arves-conformance --bin acs_validate` → OK; `cargo test --workspace` → **81/0**.
- Smoke: piping all 19 frozen semantic vectors as `<tier>\t<hex>` → **19/19 `REJECT\t<exact code>`**;
  a valid `GL-001..GL-014` term-set → `ACCEPT`.
- `freeze_check.py check` reported drift on **exactly** the sanctioned files (`semantic.rs`,
  the new `acs_validate.rs`, this record, the RUNTIME_FREEZE applied-RCR line); baseline re-advanced
  via `freeze_check.py update`. No `standard/` byte touched.

# Contributing to ARVES

ARVES is a **standard with a frozen runtime**, not an ordinary app — so contribution is
governed. The rules below are what let an ecosystem grow without the platform drifting.

## The golden rule: know which layer you're touching

| You want to… | You are a… | Rule |
|--------------|-----------|------|
| Build a product / app | **Product author** | build under `products/`; consume the Runtime v1.0 API; **never edit `runtime/` or `standard/`** |
| Publish a capability/engine/connector | **Ecosystem author** | use the Authoring Kit (`products/arves-ecosystem-sdk/`); certify + sign; publish to the Marketplace |
| Implement a new runtime (Go/Java/…) | **Runtime vendor** | implement `standard/` only; certify with `verification/certification/certify_runtime.py` |
| Change the runtime | **Platform maintainer** | file a **Runtime Change Request** → v1.1 (additive) / v2.0 (breaking); never a silent edit |
| Change the specification | **Standards contributor** | CCP / Amendment / IDR (the frozen corpus is immutable, ED-001) |

If your change would edit `runtime/crates` or `standard/` and you are not doing platform/
standards work through the instruments above, **stop** — it belongs on the product side or
in an RCR.

## Every change is destroy-first (ED-006)

`Implement → Destroy (adversarial) → Repair → Regression test → Conformance → Freeze.` A
feature that survives a destroy pass with a regression test is worth more than three
untested ones. Add your regression to `products/robustness.test.mjs` (products) or a crate
test (runtime), and keep it green.

## Verify before you send

```bash
cargo test --manifest-path runtime/Cargo.toml --workspace   # runtime + gates + conformance
node products/robustness.test.mjs                           # product robustness suite
python verification/evidence/evidence_probe.py              # the evidence ledger (machine rows)
```

The architecture gate (`arves-conformance::architecture_gate`) enforces LAYER-001/OWN-001 at
build time — an upward dependency or a second truth-owner fails `cargo test`. Respect it.

## Publish a capability (the common path)

1. `defineCapability({ name, version, produces, execute })` — see the ecosystem-sdk README
   for the ARVES value model an effect `value` must satisfy.
2. `arves certify <file>` (needs ≥1 representative test input; must be deterministic).
3. `arves package <file>` — a content-addressed, signed artifact.
4. Publish to the Marketplace; any consumer can install + run it on the frozen runtime.

## Governance & ownership

The Foundation owns the spec, certification, and registries ([FOUNDATION.md](FOUNDATION.md)).
Registry allocations (domain tags, hash codes, reason codes) are Specification-Required
(ACS-001 §4.1). No single maintainer is required to certify or extend — that is the point.

## Before public release (maintainer action)

- **License:** the repo is released under the **Apache License 2.0** — see [LICENSE](LICENSE).
- Still outstanding: a code of conduct and a security-reporting policy.
- These are outward-facing decisions for the maintainers, not code.

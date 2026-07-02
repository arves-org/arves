# @arves/marketplace — Marketplace (P7)

The **distribution layer** of the ARVES ecosystem: one party **publishes** a certified,
signed capability; any other party **discovers, installs, and runs** it on the frozen
Runtime v1.0 — producing truth. Publisher and consumer never coordinate. The marketplace
holds no truth and runs nothing; it only accepts artifacts that are **certified** (P6.5
conformance) and whose **content-addressed signature verifies**, and it refuses
uncertified, tampered, or duplicate-version publishes.

```
Publisher → author → certify → sign → PUBLISH →  Marketplace  → INSTALL → run → Truth
Consumer  ────────────────────────── discover ──┘                      (frozen Runtime v1.0)
```

## Run

```
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml
node examples/publish-install.mjs      # publish (Acme) → install+run (a different org) — exits 0
```

## Why it matters

This is the loop that makes ARVES self-sustaining: a company publishes a Risk engine, a
hospital installs a Radiology capability, a developer ships a connector — all certified,
signed, and running on the same frozen runtime, with no coordination and no runtime change.
Combined with P6.5 (authoring) it answers the platform KPI — *how much value do others
create on ARVES?* Next: the **ARVES Foundation** (governance + registry + multiple certified
runtimes behind one conformance), so the ecosystem outlives any single maintainer.

## Boundary

Consumes P6.5 + the frozen Runtime v1.0; edits no runtime or spec file. It is pure
distribution — certification and integrity are enforced, execution happens in the
consumer's host against the frozen runtime.

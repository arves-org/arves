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

## What "signed" means here (honest caveat)

The "signature" is an **unkeyed ACS content address** of the artifact (`address({manifest,
codeHash, testInputsHash})`) — a re-derivable content hash, **not** a cryptographic signature
and **not** bound to any publisher identity. It proves **integrity** (the artifact's bytes match
its id, and its code/test-inputs match the signed hashes), so a *silent* mutation is detected.
It does **not** prove **authenticity**: there is no PKI, no key, no identity. Anyone who tampers
with the code can simply re-derive the new content address and present a fresh, internally
consistent "signed" artifact — the `publisher` field is an unverified self-claim. The gate that
actually bites is **re-run certification** at publish/install (conformance is enforced, not
attested); the content address only detects tampering *relative to a trusted id*. Do not read
"signed, certified" as "authenticated by a known author." Identity binding (keyed signatures /
a publisher trust root) is future work, not a property of this layer today.

## Boundary

Consumes P6.5 + the frozen Runtime v1.0; edits no runtime or spec file. It is pure
distribution — certification and integrity are enforced, execution happens in the
consumer's host against the frozen runtime.

# @arves/ecosystem-sdk — Ecosystem SDK & Authoring Kit (P6.5)

**Make ARVES unnecessary to build yourself.** This is the layer that lets a *third party* —
a company, a developer, a university — author a capability/engine/connector and publish it,
**without ever touching the ARVES runtime**. It is the prerequisite for the Marketplace
(P7) and the real proof of platform-hood: *how much code did someone else write?*

```
Author (Author SDK) → Certify (conformance) → Package (sign) → Install (cert-gated)
   → Invoke → Truth (FROZEN Runtime v1.0 Kernel)
```

The runtime never changes: a third-party capability's **code** is unknown to it; only the
**ACS truth** it produces crosses the boundary. So third-party code runs on ARVES with **no
Runtime Change Request**.

## What's in the Kit

- **Capability Author SDK** — `defineCapability({ name, version, produces, execute })`.
  `execute(input)` returns effects `{ target, value }` where `value` is an **ARVES value**
  (see the value model below).
- **Certification** — `certifyCapability(cap, testInputs)`: manifest valid · **≥1 test
  input** (certification will NOT pass vacuously — an empty input set is rejected) · effects
  target declared produces · effects ACS-canonical · **deterministic** (same input → same
  effect addresses). Failing checks report a `detail` reason. Uncertified ⇒ cannot install.
- **Packaging / signing** — `packageCapability(cap, testInputs)`: a versioned artifact whose
  **content address IS its signature**, taken over the manifest, the actual `execute` code
  (`codeHash`), **and the representative test inputs** (`testInputsHash`). Tamper with the
  manifest, the code, *or the test inputs* ⇒ different id ⇒ `verifyArtifact` fails. The
  certification gate is **enforced, not attested**: `publish` and `CapabilityHost.install`
  **re-run certification** against those signature-bound test inputs (never a caller-supplied
  flag) and refuse code that doesn't match the signed artifact. No PKI — identity is integrity.

## The ARVES value model (what an effect `value` may be)

An effect's `value` MUST be one of these, or certification's `effects-acs-canonical` check
fails (with a `detail` telling you why):

`null` · `boolean` · **`BigInt`** integer in `[-2^64, 2^64-1]` · **`float(x)`** (import
`float` from this Kit) · `string` (UTF-8, NFC) · `Uint8Array` (bytes) · `Array` · plain
object (a map; keys must be `string` or `BigInt`). A **bare JS `number` is rejected**
(ambiguous int/float, lossy beyond 2^53) — use `BigInt` for integers and `float(x)` for
floats. Timestamps are integers (e.g. epoch-ms × `1_000_000n` for nanoseconds).

> These rules were pinned by an independent cold-build: a fresh developer built + certified
> a capability from this Kit alone; the gaps their question-log surfaced (undocumented value
> model, vacuous certification, weak signature) are the fixes above.
- **Host** — `CapabilityHost`: installs certified, signature-verified capabilities and
  invokes them, committing effects as truth via the frozen runtime.
- **CLI** — `arves init <name>` (scaffold a green, certifiable capability) · `arves doctor
  <file>` (conformance assistant: every violation with its exact fix) · `arves certify <file>`
  · `arves package <file>`.

## Author + publish a capability (the 10-minute path)

```js
import { defineCapability } from '@arves/ecosystem-sdk';
export const capability = defineCapability({
  name: 'invoice.ocr', version: '1.0.0', produces: ['uci.fact'],
  execute: (inv) => [{ target: 'uci.fact',
    value: { type: 'uci.fact', entity: `invoice:${inv.vendor}`, event: `amount-usd-${inv.amountUsd}`, at: BigInt(inv.date) * 1_000_000n } }],
});
export const testInputs = [{ vendor: 'acme', amountUsd: 1234n, date: 1751468400000 }];
export const source = 'invoice.ocr@1.0.0';
export default { capability, testInputs, source };
```

```
node bin/arves.mjs init my.capability                    # scaffold a green capability file
node bin/arves.mjs doctor my.capability.capability.mjs   # HEALTHY, or every fix you need
node bin/arves.mjs certify examples/invoice-ocr.capability.mjs   # → CERTIFIED
node bin/arves.mjs package examples/invoice-ocr.capability.mjs   # → signed artifact id
node examples/third-party-capability.mjs                         # author→…→truth (exits 0)
```

## Why this matters

When a company can publish a Medical capability, a bank a Risk engine, a university a
Research engine — all certified, signed, and running on the same frozen runtime — ARVES is
no longer *your* system; it is a platform others build businesses on. Next: **P7
Marketplace** (distribute these artifacts) and, longer term, an **ARVES Foundation** with
multiple certified runtimes (Rust/Go/Java) behind one conformance.

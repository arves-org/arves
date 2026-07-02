# ARVES Quickstart — your first cognitive app in ~10 minutes

Everything below is verified. **Two paths — pick one:**
- **(A) Author a capability — Node ≥18 only, no Rust, ~5 min.** Jump straight to **step 4**.
- **(B) Run the full runtime + demos — also needs Rust ([rustup.rs](https://rustup.rs)), ~10 min.** Start at **step 1**.

No network or third-party packages are required either way.

## 1. Build the Runtime API (once, ~1–2 min)

```bash
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml
```

This builds the bridge — the seam through which products commit truth to the real Kernel.

## 2. See ARVES do something impossible for a chatbot (~1 min)

> These demos use the Runtime API from step 1. If you see `arves-bridge unavailable`, run the
> `cargo build` above first. (Authoring a capability in step 4 needs only Node — no runtime build.)

```bash
node products/arves-personal-os/examples/my-day.mjs
```

Six systems (email/calendar/slack/github/finance/health) collapse into **one truth base**;
the daily briefing is **byte-reproducible and audited**, cites its evidence, and **catches a
contradiction with a prior decision**. Run it twice — identical.

```bash
node products/arves-enterprise-os/examples/enterprise-day.mjs
```

Policy enforced *as truth*: a $150k spend is blocked, allowed after legal approval, then a
cross-department cancel is blocked — every step a replayable compliance event.

## 3. Use the SDK directly (~2 min)

```js
// hello-arves.mjs
import { Arves, FactStore } from './products/arves-sdk-ts/src/arves.mjs';
const arves = new Arves();
const store = new FactStore();

const id = store.commit({ type: 'uci.fact', claim: 'sky-is-blue',
  confidence: arves.float(0.5), observed_at: 1730000000000000000n });
// Commit the same fact with keys reordered → same id → store stays size 1 (idempotent).
console.log(id, store.size);
```

```bash
node hello-arves.mjs
```

Integers are `BigInt`, floats are `arves.float(x)` — ARVES refuses ambiguous/lossy numbers
so a content address can never silently drift.

## 4. Author + certify your own capability (~3 min)

Copy `products/arves-ecosystem-sdk/examples/invoice-ocr.capability.mjs`, change the logic,
then:

```bash
cd products/arves-ecosystem-sdk
node bin/arves.mjs certify examples/invoice-ocr.capability.mjs   # → CERTIFIED
node bin/arves.mjs package examples/invoice-ocr.capability.mjs   # → a signed artifact id
node examples/third-party-capability.mjs                         # author → … → truth
```

Rules for a capability `value` are the ARVES value model (see the ecosystem-sdk README).

## 5. Certify a runtime (~1 min)

```bash
python verification/certification/certify_runtime.py
```

Certifies runtimes against `standard/` alone — no maintainer required. Two runtimes
(Rust + Python) pass under one conformance — at grade **G1** (same-process); a genuine
third-party (G2) runtime is the open goal.

## Where next

- Publish to the Marketplace → `products/arves-marketplace/`
- The full contract → `standard/` · Why it's a platform → [FOUNDATION.md](FOUNDATION.md)
- Contribute → [CONTRIBUTING.md](CONTRIBUTING.md)

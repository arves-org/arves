# ARVES — a cognitive computing platform

**ARVES is to cognitive applications what the Linux kernel is to operating systems, or
Kubernetes is to containers:** a frozen, certified runtime that turns AI reasoning into
*truth* — deterministic, content-addressed, replayable, and auditable — with an ecosystem
of products and third-party capabilities built on top of it.

> **Status: BUILD PROGRAM COMPLETE → LAUNCH.** The technical core is built, proven, and
> frozen (`runtime-v1.0`), with a Foundation that lets it outlive its makers
> (`foundation-v1.0`). What remains is adoption. See [FOUNDATION.md](FOUNDATION.md).

## Why ARVES? (why not just a ChatGPT / LangGraph / n8n / AutoGen wrapper)

A wrapper gives you a chain of prompts and side effects you **cannot reproduce, audit, or
defend**. ARVES gives you cognition as *truth*:

| You get… | Because… | A wrapper can't |
|----------|----------|-----------------|
| **One identity across systems** | identity is a content address (ACS-001) | double-counts the same fact from 3 sources |
| **Reproducible reasoning** | every step is content-addressed truth in a Kernel | answers differently each run |
| **Full audit + replay** | append-only WAL = decision trace (ORCH-003) | no tamper-evident record |
| **Decision-aware memory** | prior decisions are addressable truths | no persistent, provable history |
| **Governed multi-agent** | policy enforced as truth; compliance ledger | no shared enforced truth |

If a product could be built cheaply with a wrapper, it doesn't need ARVES. ARVES is for the
products that **can't** be — see the Personal & Enterprise Cognitive OS demos below.

## 10 minutes to your first ARVES app

```bash
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml   # the frozen Runtime API (once)

# A personal cognitive OS: 6 systems → one truth → a reproducible, audited daily briefing
node products/arves-personal-os/examples/my-day.mjs

# An enterprise cognitive OS: policy enforced as truth, compliance ledger, multi-agent
node products/arves-enterprise-os/examples/enterprise-day.mjs

# Author + certify your OWN capability (no runtime source needed)
node products/arves-ecosystem-sdk/bin/arves.mjs certify \
  products/arves-ecosystem-sdk/examples/invoice-ocr.capability.mjs
```

Full path: **[QUICKSTART.md](QUICKSTART.md)**.

## Architecture

```
Products      Personal OS · Enterprise OS · (your product)          products/
Ecosystem     Authoring Kit · Marketplace · Certification           products/arves-{ecosystem-sdk,marketplace}
Runtime v1.0  SDK · Bridge · Engine · Capability · Kernel · Truth    runtime/  (FROZEN)
Standard      ACS-001..005 · conformance vectors                     standard/ (the contract)
Foundation    spec · certification · registry · governance          FOUNDATION.md
```

The **runtime is frozen**: products are *customers* of a stable API, never co-authors. A
needed runtime change is a Runtime Change Request, not an edit
([runtime/RUNTIME_FREEZE_v1.0.md](runtime/RUNTIME_FREEZE_v1.0.md)).

## What's proven (evidence, not claims)

- **Interoperability** — 3 independent codec implementations (Rust/Python/TypeScript) agree
  byte-for-byte; differentially fuzzed (13,807 inputs, 0 divergences).
- **Independent runtimes** — 2 runtimes certified against `standard/` alone by a
  maintainer-independent harness (`python verification/certification/certify_runtime.py`).
- **Robustness** — a whole-system destroy pass found + fixed 21 blocker/major defects;
  product robustness suite is **37/37**.
- **Ecosystem** — a cold, fresh-context third party authored + certified a capability from
  the Authoring Kit alone.
- The living evidence ledger: [verification/evidence/EVIDENCE_LEDGER.md](verification/evidence/EVIDENCE_LEDGER.md).

## Build on ARVES

- **Use it** → [QUICKSTART.md](QUICKSTART.md)
- **Publish a capability** → [products/arves-ecosystem-sdk/](products/arves-ecosystem-sdk/)
- **Contribute / extend** → [CONTRIBUTING.md](CONTRIBUTING.md)
- **Govern / certify** → [FOUNDATION.md](FOUNDATION.md)

## License

Intended to be published under a permissive, foundation-friendly open-source license
(Apache-2.0 recommended). *Finalizing the license is a maintainer decision required before
public release* — see CONTRIBUTING.

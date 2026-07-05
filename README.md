# ARVES — a cognitive computing platform

```
ARVES · Version 1.0 · Status: BUILD COMPLETE (SEALED)
Governance: Foundation · Development Model: Growth Program
Runtime / Specification / Standard: FROZEN · changes: RCR only
```

**ARVES is to cognitive applications what the Linux kernel is to operating systems, or
Kubernetes is to containers:** a frozen, certified runtime that turns AI reasoning into
*truth* — deterministic, content-addressed, replayable, and auditable — with an ecosystem
of products and third-party capabilities built on top of it.

> **Status: BUILD PROGRAM SEALED → GROWTH.** The core is built, **independently audited**
> (a 16-pillar adversarial closure audit, 3 rounds), frozen (`runtime-v1.0` = **single-node
> I1**), and sealed — see [ARVES_BUILD_PROGRAM_CLOSURE.md](ARVES_BUILD_PROGRAM_CLOSURE.md).
> The distributed milestones (I2–I6) and a genuine **third-party** runtime certification
> (independence grade **G2**) are the Growth Program, not done yet. What remains is adoption.
> See [FOUNDATION.md](FOUNDATION.md).

## Why ARVES? (why not just a ChatGPT / LangGraph / n8n / AutoGen wrapper)

A wrapper gives you a chain of prompts and side effects you **cannot reproduce, audit, or
defend**. ARVES gives you cognition as *truth*:

| You get… | Because… | A wrapper can't |
|----------|----------|-----------------|
| **One identity across systems** | identity is a content address (ACS-001) | double-counts the same fact from 3 sources |
| **Reproducible reasoning** | every step is content-addressed truth in a Kernel | answers differently each run |
| **Full audit + replay** | append-only, content-addressed WAL = a deterministic decision trace (ORCH-003) | no reproducible, replayable record |
| **Decision-aware memory** | prior decisions are addressable truths | no persistent, provable history |
| **Governed multi-agent** | policy enforced as truth; compliance ledger | no shared enforced truth |

If a product could be built cheaply with a wrapper, it doesn't need ARVES. ARVES is for the
products that **can't** be — see the Personal & Enterprise Cognitive OS demos below.

## Your first capability in ~5 minutes (Node ≥18 — no Rust)

Authoring runs entirely on Node — you do **not** need the Rust runtime build for this path:

```bash
node products/arves-ecosystem-sdk/bin/arves.mjs init hospital.incident
node products/arves-ecosystem-sdk/bin/arves.mjs doctor hospital.incident.capability.mjs   # HEALTHY, or the exact fix
node products/arves-ecosystem-sdk/bin/arves.mjs certify hospital.incident.capability.mjs  # → CERTIFIED
```

### Also want the runtime demos? (needs Rust — [rustup.rs](https://rustup.rs))

```bash
cargo build -p arves-bridge --bin arves-bridge --manifest-path runtime/Cargo.toml   # the frozen Runtime API (once)
node products/arves-personal-os/examples/my-day.mjs            # 6 systems → one reproducible, audited truth
node products/arves-enterprise-os/examples/enterprise-day.mjs  # policy-as-truth + a compliance ledger
```

Full path: **[QUICKSTART.md](QUICKSTART.md)**.

## Architecture

```
Products      Personal OS · Enterprise OS · (your product)          products/
Ecosystem     Authoring Kit · Marketplace · Certification           products/arves-{ecosystem-sdk,marketplace}
Runtime v1.0  SDK · Bridge · Engine · Capability · Kernel · Truth    runtime/  (FROZEN · single-node I1)
Standard      ACS-001..005 · conformance vectors                     standard/ (the contract)
Foundation    spec · certification · registry · governance          FOUNDATION.md
```

The **runtime is frozen**: products are *customers* of a stable API, never co-authors. A
needed runtime change is a Runtime Change Request, not an edit
([runtime/RUNTIME_FREEZE_v1.0.md](runtime/RUNTIME_FREEZE_v1.0.md)). v1.0 is the **single-node
I1** runtime; the distributed path (I2–I6, per-shard Raft per IDR-001..005) is future work.

## What's proven (evidence, not claims)

Every bullet below is backed by a command you can run; independence is **graded** (G1 =
same-process/in-program, G2 = a genuine outside party with no help) and stated honestly.

- **Interoperability** — 3 independent codec implementations (Rust / Python / TypeScript)
  agree on the golden conformance vectors and are **3-way differentially fuzzed** (13,807
  inputs, 0 divergences: identical accept/reject across all three, byte-identical re-encode
  on every accept) at the ACS-002 byte layer; the ACS-003/004/005 **semantic** reject surface
  is differential too (Rust-native vs Python, 62/62 cases agree).
- **Independent runtimes** — 2 runtimes (Rust + Python) certified against `standard/` alone over
  the **full ACS-001..005 surface** — `SOUND-CERTIFIED (full surface)`, incl. the ACS-003/004/005
  semantic reject tiers — by a maintainer-independent, non-gameable harness
  (`python verification/certification/verify_runtime_sound.py`), at grade **G1 (same-process)**.
  (The **TypeScript** codec is a 3rd independent implementation that agrees on every vector — a 3rd
  *codec*, not a 3rd certified *runtime*: `certify_runtime.py`/`verify_runtime_sound.py` drive the
  Rust + Python runtime adapters.) A genuine third-party (G2) runtime is the open exit gate.
- **Robustness** — a whole-system destroy pass hardened the stack; the product robustness
  suite is **49/49**, and the Rust workspace is **87/87** green.
- **Ecosystem** — a cold, fresh-context developer (grade **G1**, same-process) authored +
  certified a capability from the Authoring Kit alone; a genuine external third party (G2)
  is pending.
- The living evidence ledger: [verification/evidence/EVIDENCE_LEDGER.md](verification/evidence/EVIDENCE_LEDGER.md).

## Build on ARVES

- **Understand why** → [WHY_ARVES.md](WHY_ARVES.md) — the manifesto (start here)
- **Use it** → [QUICKSTART.md](QUICKSTART.md)
- **Build & certify your OWN runtime — the open G2 challenge** → [IMPLEMENTING_ARVES.md](IMPLEMENTING_ARVES.md) (cold-start packet) · [CHALLENGE.md](CHALLENGE.md) (the invitation + how to submit) · [CERTIFY_YOUR_RUNTIME.md](verification/certification/CERTIFY_YOUR_RUNTIME.md) (the copy-paste last mile + `--self-test`)
- **Publish a capability** → [products/arves-ecosystem-sdk/](products/arves-ecosystem-sdk/)
- **Contribute / extend** → [CONTRIBUTING.md](CONTRIBUTING.md)
- **Govern / certify** → [FOUNDATION.md](FOUNDATION.md)

## License

ARVES is released under the **Apache License 2.0** — see [LICENSE](LICENSE).

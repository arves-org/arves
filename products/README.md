# ARVES Cognitive Application Platform — the Apollo Program

**Mission:** *Transform ARVES from a proven cognitive standard into the world's best
cognitive application platform.*
**Motto:** *Stop proving that ARVES can exist. Start proving why ARVES matters.*

The Standard Program asks *"is ARVES correct?"*. This program asks the question that
creates value: **"what products can we now build that were impossible before?"** It runs
in **parallel** with the Standard Program (G2 external validation + certification), on
top of a platform that is ~90% mature (three independent G1 runtimes agree byte-for-byte).

## The five product rules (every product obeys all five)

1. **Platform-first.** The frozen specification is unchangeable; products only *consume*
   the platform (IDR-006). No product modifies `runtime/` or `standard/`.
2. **Value-first.** Every sprint ends with a *working demo*, not a document.
3. **Evidence-first.** Every product concretely demonstrates ≥1 core ARVES capability
   (identity, evidence, replay, audit, dedup, reasoning, …) — so products *are* proof.
4. **Developer-first.** Every capability is usable from the SDK in a few lines of code.
5. **Production-first.** Demo code is not thrown away — it is written to become a real
   product.

**Product KPI (replaces tests/conformance for this arm):** *Impossible before ARVES →
Possible with ARVES.* North-star metric: **a developer downloads the SDK and ships a
first cognitive app in ~10 minutes.**

## IDR-006 — Parallel Product Program (governance decision of record)

The Engineering Constitution previously read *"Products are forbidden until the platform
is certified."* This IDR reconciles that with the two-arm pivot — deliberately, through
the constitution's own instrument for an engineering decision (not a silent edit).

- **Decision.** Products may be **developed now**, in parallel with the Standard Program.
- **As a frozen, versioned dependency.** A product consumes `arves-standard-kit 0.2.0`
  (`standard/`) and the reference runtime at its current tag. It treats them like an
  external package with a pinned version.
- **Hard guardrail (non-negotiable).** No product may modify `runtime/` or `standard/`.
  If a product needs a platform change, it STOPS and files a **Platform Change Proposal**
  (PCP); the platform changes only through the standard's CCP/IDR process. This preserves
  exactly what the original gate protected: the standard never bends to product convenience.
- **The gate is retained for GA.** General availability / any production release that
  carries platform-stability guarantees still requires all four conditions:
  **Independent Runtime PASS + External Team (G2) PASS + Certification PASS + Formal
  Verification PASS.** Until then, products ship as **previews pinned to a platform version**.
- **Residual risk (owned by the maintainer).** A product built on a pre-G2 platform may
  need migration if G2/certification forces a breaking platform change; pinning the Kit
  version bounds the blast radius.

## The product ladder (P0 → P10) — each product PROVES a capability

| # | Product | Proves (ARVES capability) | Builds on |
|---|---------|---------------------------|-----------|
| **P0** | **Developer SDK** ✅ | ergonomic content-addressing | Standard Kit |
| **P1** | **Cognitive Memory** ✅ | Identity · Evidence · Replay · Truth · Audit · Deduplication | P0 |
| **P2** | **Runtime Bridge** ✅ | one-world identity **and the full cognitive work chain**: SDK → Capability (resolve/gate) → Engine (pure invoke) → Kernel (commit as ACS truth) | P0, Kernel/Engine/Capability |
| **P3** | **Agent Runtime** ✅ | Reasoning · Planning · Capability selection · Execution · Truth update — **on the real Kernel** | P0–P2 |
| P4 | Personal AI | Autonomy · Learning · Preferences · Scheduling · Decision support | P1–P3 |
| P5 | Enterprise AI | Multi-Agent · Governance · Policy · Compliance · Security | P1–P3 |
| P6 | Visual Cognitive Studio | visual authoring of cognitive graphs | P1–P3 |
| P7 | Marketplace | engines/capabilities/agents/connectors | P0–P5 |
| P8 | Cloud Platform | hosted ARVES | P0–P7 |
| P9 | Industry Solutions | Healthcare · Manufacturing · Government · Finance | P0–P8 |
| P10 | ARVES OS | the cognitive operating system | P0–P9 |

**Kernel Bridge (P2) was brought forward on purpose:** it prevents an "SDK world" and a
"runtime world" from diverging — the largest architectural risk of the product era. The
target chain is `Products → SDK → Kernel Bridge → Kernel → LCW → Capability → Engine`,
all sharing one ACS-001 identity. P3 Agent Runtime therefore runs on the *real* Kernel,
not a stand-in.

## Program 4 — the IMPOSSIBLE PRODUCTS filter

Every product must pass one gate: **"Could this be built without ARVES?"** If **YES →
reject it** — the product is wrong (it doesn't need the platform). We only build products
that ARVES makes uniquely possible. The KPI is no longer PASS, it is **WOW** — a person
seeing the demo asks *"how did you do this?"*. The question the whole org now answers:
*Can ARVES create products that nobody else can build?*

New validation ladder (beyond conformance/certification):
`Product Validation → Developer Adoption → Commercial Adoption`.

## Six-month tracks (parallel)

Standard arm: **A** External G2 · **B** Certification.
Product arm: **C** Developer SDK · **D** Visual Designer · **E** Personal AI ·
**F** Enterprise Runtime · **G** Marketplace.

## Organization (re-weighted for the Apollo phase)

**Products 35 · Platform 20 · Verification 15 · Research 15 · Developer Experience 15**
(≈100). Success is no longer "how many standards did we write?" but **"how many
developers actually built a product with ARVES?"** The platform is the *supplier*;
products are its *customers*; Developer Experience is now a first-class function.

## Layout

```
products/
  README.md                       this charter
  arves-sdk-ts/                   P0 — TypeScript Developer SDK ✅
    src/bridge.mjs                P2 client — talks to the real Kernel
  arves-cognitive-memory/         P1 — Cognitive Memory ✅ (flagship)
  arves-agent-runtime/            P3 — Agent Runtime ✅ (reasons on the real Kernel)
runtime/crates/arves-bridge/      P2 — Runtime Bridge (PLATFORM): Capability→Engine→Kernel, ACS-addressed
runtime/crates/arves-engine-fabric/     concrete reference Engine (PureEngine)
runtime/crates/arves-capability-fabric/ concrete reference CapabilityRegistry (MemRegistry)
```

> **Chain status:** `Products → SDK → Bridge → Capability → Engine → Kernel` is now real
> end-to-end (`node products/arves-sdk-ts/examples/engine-invoke.mjs`). P4 Personal AI
> runs on this full chain, not just SDK→Kernel.

Every product directory states, at its top, the platform version it pins and affirms it
modifies no platform file (IDR-006).

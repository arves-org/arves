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

## The product ladder (P0 → P9) — each product PROVES a capability

| # | Product | Proves (ARVES capability) | Builds on |
|---|---------|---------------------------|-----------|
| **P0** | Developer Experience | 10-minutes-to-first-app | — |
| **P1** | **Developer SDK** ✅ | ergonomic content-addressing | Standard Kit |
| **P2** | **Cognitive Memory** 🟢 | **Identity · Evidence · Replay · Truth · Audit · Deduplication** | P1 |
| P3 | Agent Runtime | Reasoning · Planning · Memory · Capability · Execution | P1, P2, Kernel |
| P4 | Personal AI | multi-source cognition → reasoning → actions | P2, P3 |
| P5 | Enterprise AI | Multi-Agent · Governance · Policy · Security · Workflow | P2, P3 |
| P6 | Visual Cognitive Studio | visual authoring of cognitive graphs | P2, P3 |
| P7 | Marketplace | engines/capabilities/agents/connectors | P1–P5 |
| P8 | Cloud Platform | hosted ARVES | P1–P7 |
| P9 | Industry Solutions | Healthcare · Manufacturing · Government · Finance | P1–P8 |

Cognitive Memory (P2) is deliberately one step *below* Personal AI: it is the common
core of Personal AI, Enterprise AI, and every industry solution. Because each product
proves a capability, the product ladder is simultaneously a second, product-level proof
of ARVES — value and evidence become the same artifact.

## Six-month tracks (parallel)

Standard arm: **A** External G2 · **B** Certification.
Product arm: **C** Developer SDK · **D** Visual Designer · **E** Personal AI ·
**F** Enterprise Runtime · **G** Marketplace.

## Organization (re-weighted for the Apollo phase)

**Products 50 · Platform 20 · Evidence 15 · Research 15** (≈100). The platform is now the
*supplier*; products are its *customers*. Platform + Evidence keep the destroy-lab
discipline on a near-frozen base; Research explores what the platform makes newly possible.

## Layout

```
products/
  README.md               this charter
  arves-sdk-ts/           P1 — TypeScript Developer SDK ✅ (on the platform)
  arves-cognitive-memory/ P2 — Cognitive Memory 🟢 (flagship; on the SDK)
```

Every product directory states, at its top, the platform version it pins and affirms it
modifies no platform file (IDR-006).

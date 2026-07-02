# ARVES Product Program

**Motto:** *Stop proving that ARVES can exist. Start proving why ARVES matters.*

The Standard Program asks *"is ARVES correct?"*. This program asks the question that
creates value: **"what products can we now build that were impossible before?"** It runs
in **parallel** with the Standard Program (G2 external validation + certification), on
top of a platform that is ~90% mature (three independent G1 runtimes agree byte-for-byte).

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

## The product ladder (P1 → P8)

| # | Product | What it is | Builds on |
|---|---------|-----------|-----------|
| **P1** | **Developer Platform / SDK** | ergonomic libraries to build on ARVES without touching byte-level ACS | Standard Kit (ACS codec) |
| P2 | Visual Cognitive Studio | visual builder for cognitive graphs / engines | P1 |
| P3 | Agent Runtime | run multi-agent cognitive apps | P1, Kernel |
| P4 | Personal AI | Email/Calendar/WhatsApp/Slack/GitHub/Bank/Docs/Photos/Health/Voice → Universal Cognitive Model → Reasoning → Actions | P1–P3 |
| P5 | Enterprise AI | SAP/Salesforce/Oracle/Slack/Jira/Teams/GitHub/Kafka/Snowflake → Information Platform → Kernel → Capability → Engine → Business Agents | P1–P3 |
| P6 | Marketplace | share/sell engines, capabilities, agents, connectors | P1–P5 |
| P7 | Cloud | hosted ARVES platform | P1–P6 |
| P8 | Industry Solutions | Healthcare (clinical reasoning), Manufacturing (industrial intelligence), Government, Finance | P1–P7 |

## Six-month tracks (parallel)

Standard arm: **A** External G2 · **B** Certification.
Product arm: **C** Developer SDK · **D** Visual Designer · **E** Personal AI ·
**F** Enterprise Runtime · **G** Marketplace.

## Organization (re-weighted)

**Products 50 · Platform/Standard 30 · Verification 20** (≈100). The platform side keeps
the destroy-lab discipline (more agents break the near-frozen platform than extend it);
the product side is where the growth now goes.

## Layout

```
products/
  README.md            this charter
  arves-sdk-ts/        P1 — the TypeScript Developer SDK (first product; on the platform)
```

Every product directory states, at its top, the platform version it pins and affirms it
modifies no platform file (IDR-006).

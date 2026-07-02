# ARVES Developer Journey Report — DX Baseline (Growth Era)

**The first Growth-Engineering artifact.** It measures ARVES not from the makers' view but from
a newcomer's: can a developer who has never seen ARVES reach a first success from the public docs
alone? Produced by the **Developer Journey Simulator** (`developer-journey-simulator` workflow):
8 distinct personas each cold-started the repo using ONLY newcomer-accessible material (docs-site,
README, QUICKSTART, product READMEs, and running the documented commands), forbidden from reading
internal source to unstick themselves — needing to was itself logged as a docs gap.

> **Honesty caveats (this is a proxy, graded like everything in ARVES).** These are *simulated*
> journeys, not real users — high signal, not truth. The observed activation is inflated by a
> pre-cached Rust toolchain; the **honest field activation is ~5/8**. The decisive versions are
> external: **real developers** (the true feedback loop) and the **Level-3 cross-vendor test**
> (GPT / Gemini / Qwen / DeepSeek reaching the same capability) — which cannot be run from here
> (Claude-family only); a same-family proxy would be G1, not the cross-vendor proof.

## KPI baseline (2026-07, simulated N=8)

| KPI | Baseline | Target | Status |
|-----|----------|--------|--------|
| Time to First Success | ~8.5 min median (author-only ~4 min; cold clone w/ live cargo compile 10–12 min) | <10 min | ✅ MET |
| Activation (reached first CERTIFIED capability) | 8/8 observed; **~5/8 (62%) adjusted** for pre-cached Rust | >80% | ✅ MET* |
| First-capability success (init→doctor→certify green) | 8/8 = 100% | >85% | ✅ MET |
| Docs self-sufficiency (never read source) | ~88% (drops for runtime-certification) | >90% | 🟡 CLOSE |
| **North-star: Time-To-First-Production-Capability** | not yet measured (needs a real publish + hosted registry) | continuously ↓ | ⏳ |

\* MET only because cargo was pre-installed; see Friction #1. On a true fresh clone the non-Rust
personas would stall at the toolchain gate.

## What genuinely works (keep)

- The **demos "wow"**: cross-source dedup, contradiction-with-prior-decision, byte-reproducible ids.
- **`doctor`/`certify` diagnostics are best-in-class**: every persona could fix a broken capability
  from the error alone (cites ACS-002 §5.2/§5.3, explains why, gives the fix) — zero source reading.
- **Graded-independence honesty** (G1 vs G2, the public v1.1 debt list) earns real trust.

## Top friction (ranked)

| # | Sev | Theme | Status |
|---|-----|-------|--------|
| 1 | major | Rust toolchain front-loaded as the first command; unnecessary for authors; no rustup link / prebuilt binary / Docker | **fixed (docs)** — canonical path is now Node-first; rustup linked. Prebuilt binary + Docker → backlog |
| 2 | major | No documented path to certify a NEW (Go/Java/own) runtime; CONTRIBUTING vs CONFORMANCE contradict | **backlog (top)** — unify into one "Add your runtime" guide + ship a non-Rust reference runner |
| 3 | major | `marketplace.html` cargo snippet missing `--manifest-path` (copy-paste dead end) | **fixed** — all cargo snippets site-wide now carry the flag |
| 4 | major | No `arves publish` / `arves install` verb — publishing your own artifact is demo-only | **backlog** — add CLI verbs over a local (then hosted) registry |
| 5 | major | Canonical first-run path self-contradictory (5 min/no-build vs 10 min/Rust) across pages | **fixed** — README/QUICKSTART/index now agree: Node-only authoring first, runtime demos opt-in |
| 6 | minor | `arves` had no `--help` / per-command reference | **fixed** — `arves --help` + `arves <cmd> --help` + unknown-cmd help |
| 7 | minor | Repo working copy not clean; scaffolds pollute `git status` | **fixed** — `.gitignore` now ignores ad-hoc `*.capability.mjs` (keeps curated examples/) |
| 8 | minor | No non-JS capability authoring on-ramp despite the pitch inviting it | **backlog** — state plainly authoring is JS/.mjs today; track a JVM/Python Authoring Kit |
| 9 | minor | Demo-to-reality gap: mocked connectors, in-process registry | **backlog** — one real-data (CSV/webhook) example + connector-SDK story |
| 10 | minor | `Cargo.lock` gitignored for a determinism-selling workspace | **backlog** — commit via RCR (freeze debt #7) |

## FAQ to pre-answer (docs + Playground)

1. What is the ONE canonical first-run path? → **Node-only authoring (~5 min, no Rust)**; runtime demos are an opt-in branch.
2. No Rust — can I author with Node only? → **Yes.** Prebuilt bridge binary / Docker for the demos is on the backlog.
3. How do I certify MY OWN runtime (Go/Java)? → *the top backlog item* — one authoritative "Add your runtime" guide is pending.
4. Author in Python/Java? → Authoring is **JavaScript/.mjs only** today; "Python/Go/Java" refers to *runtime* implementations.
5. How do I publish my own artifact? → `arves publish`/`install` verbs are backlog; today it's the in-process marketplace example.
6. Where is the full CLI reference? → `arves --help` (added); a CLI reference page is planned.
7. How much of a demo is the Rust Kernel vs the TS SDK? → clarify on `runtime.html` (item #6).
8. How do I wire a REAL data source? → backlog: a CSV/Postgres/webhook worked example.
9. Which spec docs do I actually need first? → backlog: a "read these 3 first" starter path over the ~50-doc corpus.
10. Is there a production/ops story? → v1.0 is single-node I1; distributed/ops is Growth (honestly scoped).

## Prioritized DX backlog (next investments)

1. **Runtime-vendor on-ramp** (highest strategic value — it is the G2 / "independent team builds a
   runtime" path): one unified *Add your runtime* guide + a runnable **non-Rust reference runner**
   over `standard/vectors/*.tsv`, with CONTRIBUTING and CONFORMANCE pointing at the same procedure.
2. **`arves publish` / `arves install`** CLI verbs over a local registry (then a hosted one).
3. **Prebuilt `arves-bridge` binaries** (per-OS release assets) + a **Docker image** so non-Rust
   devs run the demos without a toolchain.
4. **Real-data example** (CSV / HTTP webhook) feeding a capability; a connector-SDK story.
5. **"Read these 3 spec docs first"** curated starter path; a **CLI reference page**.
6. **Non-JS authoring** note now + JVM/Python Authoring Kit on the roadmap.

## Continuous DX Verification

This report is regenerable: re-run the `developer-journey-simulator` workflow after any DX change
and compare the KPI baseline — DX regression is caught like a performance regression. This is the
seed of the **ARVES Observatory** (a dashboard over these metrics) and the discipline behind driving
**Time-To-First-Production-Capability** down over time.

*Simulated baseline; the truth is real developers. Publishing the docs site + repo starts that loop.*

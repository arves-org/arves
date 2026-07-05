# Releasing ARVES — the Growth Program's start protocol

This file is **not** part of the sealed Build Program. It is the opening protocol of the
**Growth Program**: how ARVES goes from a proven-in-program artifact to a living, externally
adopted platform. The compass for all of it is [SUCCESS.md](SUCCESS.md).

## Pre-publish checklist

Verify green, then publish. Every command below runs offline, no keys:

- [x] **LICENSE** — Apache-2.0 at the repo root.
- [x] **Front door** — [README.md](README.md) · [WHY_ARVES.md](WHY_ARVES.md) ·
  [QUICKSTART.md](QUICKSTART.md) · [CONTRIBUTING.md](CONTRIBUTING.md) · `docs-site/` (90 pages —
  40 primary + 50 spec — GitHub-Pages-ready). "Links clean" is not asserted by hand: running
  `node tools/build_docs_site.mjs` regenerates the site and runs a **build-time link-gate** that
  fails (non-zero exit) on any broken in-site relative link; the checklist is green only when that
  build exits 0 (`link-gate: OK — 0 broken links`).
- [x] **Green:** `node products/robustness.test.mjs` (43/43) ·
  `cargo test --manifest-path runtime/Cargo.toml --workspace` (81/0) ·
  `python verification/certification/certify_runtime.py` (2/2) ·
  `python verification/evidence/evidence_probe.py` (9/9) ·
  `python verification/certification/verify_runtime_sound.py` (2/2 SOUND-CERTIFIED full surface) ·
  `python verification/freeze/freeze_check.py check` (0 drift).
- [ ] **`CODE_OF_CONDUCT.md` + `SECURITY.md`** — standard community/security-policy files
  (a maintainer action; outward-facing policy, not code).
- [ ] **Pick the public GitHub org/name.**

## Publish

1. Push to a public GitHub repository.
2. Enable **GitHub Pages** serving `docs-site/` — it is static with a `.nojekyll`, so there is
   no build step; regenerate any time with `node tools/build_docs_site.mjs` (which also runs the
   link-gate and refuses to emit a site with a broken in-site link).
3. Cut a release from the sealed line (tags `runtime-v1.0`, `arves-build-v1.0`,
   `growth-program-v1` already exist).
4. Announce with the [WHY_ARVES](WHY_ARVES.md) manifesto as the lead — *why* before *how*.

## The Growth loop (engineering discipline, applied to adoption)

```
Publish → Observe → Measure → Interview → Prioritize → Build → Publish
```

From here, **the backlog is written by real users, not by us.** Do not guess the next feature;
watch where real developers struggle and let that set priority.

## The first six months — freeze the center, sharpen the edges

**Do not touch the Runtime** except for a genuine security issue or a critical bug. Everything
else improves at the **edges**:

- Documentation · Examples · Playground · AI Assistant · Marketplace · SDK ergonomics.

Observe where real developers get stuck. **Only when many independent users hit the *same*
runtime gap** do you open a **Runtime Change Request** — so the runtime evolves to real need,
never to speculation. This is how "Core is stable, value grows at the edges" is enforced in
practice.

## Measure continuously

- Re-run the **Developer Journey Simulator** after each DX change and compare the baseline in
  [verification/dx/DEVELOPER_JOURNEY_REPORT.md](verification/dx/DEVELOPER_JOURNEY_REPORT.md):
  Time-to-First-Success, activation, docs self-sufficiency. DX regression is caught like a
  performance regression.
- Track the **five events** in [SUCCESS.md](SUCCESS.md).

## The line

> **The platform is no longer waiting for engineers; it is waiting for its first independent
> users.**

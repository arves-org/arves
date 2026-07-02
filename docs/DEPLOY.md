# ARVES — Deployment on-ramp

The #1 friction for trying ARVES is the toolchain gate: the product demos and the
`arves` capability CLI are Node-only, but the reference **runtime** (the `arves-bridge`
and `acs_decode` binaries the demos and the certification harness call) is **Rust**, and
the harness is **Python**. This image bundles all three so a developer with only Docker
can run the entire [QUICKSTART](../QUICKSTART.md) end to end — no local Rust, Node, or
Python install required.

> **Status: built and verified in-repo (2026-07).** `docker build -t arves-onramp:v1.0 .`
> succeeds, and the image was checked end-to-end **inside the container**: the `arves` CLI
> runs, `arves certify` returns `CERTIFIED`, the Personal OS demo runs on the freshly-built
> `arves-bridge` binary (exit 0), and `certify_runtime.py` reports **2/2 runtimes** under one
> conformance. Image size ≈ **371 MB**. (Publishing the image to a container registry is still
> an external CI step — see "Prebuilt release binaries" at the bottom of this page.)

---

## What you need

- **Docker** (Engine 20.10+ or Desktop). Nothing else.
- Network access **only** to pull the two base images and the crates Cargo resolves during
  the build (pinned by the committed `runtime/Cargo.lock`). After the image is built it runs
  fully offline — the demos and CLI have **zero** third-party npm dependencies and the
  certification harness is Python-stdlib-only.

## Build

From the repository root (the directory containing this repo's `Dockerfile`):

```bash
docker build -t arves-onramp .
```

Multi-stage build:

1. **`builder`** (`rust:1.82-bookworm`) compiles the two reference-runtime binaries with
   `cargo build --locked` — `arves-bridge` (the SDK↔Kernel seam) and `acs_decode` (the
   negative-vector decoder the certification harness drives).
2. **`runtime`** (`node:20-bookworm-slim` + `python3`) receives the source tree and the
   compiled binaries placed at `runtime/target/debug/` — the exact path the SDK
   (`products/arves-sdk-ts/src/bridge.mjs`) and the harness
   (`verification/certification/certify_runtime.py`) resolve.

A `.dockerignore` keeps a large host `runtime/target/` (can be hundreds of MB) and `.git`
out of the build context; the builder always compiles fresh binaries.

## Run

Print the quickstart (default command):

```bash
docker run --rm arves-onramp
```

Run the product demos (byte-reproducible; run twice for identical output):

```bash
docker run --rm arves-onramp node products/arves-personal-os/examples/my-day.mjs
docker run --rm arves-onramp node products/arves-enterprise-os/examples/enterprise-day.mjs
```

Author / certify / package a capability with the `arves` CLI (on `PATH` in the image):

```bash
docker run --rm arves-onramp arves certify products/arves-ecosystem-sdk/examples/invoice-ocr.capability.mjs
docker run --rm arves-onramp arves package products/arves-ecosystem-sdk/examples/invoice-ocr.capability.mjs
```

Certify the runtimes against the frozen Standard (Rust + Python under one conformance):

```bash
docker run --rm arves-onramp python3 verification/certification/certify_runtime.py
```

Interactive shell to explore:

```bash
docker run --rm -it arves-onramp bash
```

---

## Local install (no Docker)

If you would rather install the toolchains directly, the QUICKSTART splits into two paths:

- **Author a capability only** — needs **Node ≥ 18** and nothing else. Copy an example from
  `products/arves-ecosystem-sdk/examples/` and run the `arves` CLI.
- **Full runtime + demos** — additionally needs **Rust** (install via
  [https://rustup.rs](https://rustup.rs)) and **Python 3** for the certification harness.
  Build the bridge once with the command in [QUICKSTART](../QUICKSTART.md) step 1.

## Prebuilt release binaries — external / CI

This repo does **not** ship prebuilt per-OS binary release assets, and this image does not
create them. Publishing signed, per-platform `arves-bridge` / `acs_decode` binaries (and any
container image to a registry) is a **maintainer / CI responsibility** performed outside this
repository — e.g. a release workflow that runs `cargo build --release` on a Linux/macOS/Windows
matrix and attaches the artifacts to a tagged release, and/or `docker build` + `docker push`.
Those steps require network access and signing credentials and are therefore **out of scope**
for the offline, in-repo on-ramp described here. Until such a release exists, building this
image (or the local install above) is the supported path.

# syntax=docker/dockerfile:1
# =============================================================================
# ARVES — deployment on-ramp image
# -----------------------------------------------------------------------------
# One image that gives a non-Rust developer everything the QUICKSTART needs:
#   * the compiled `arves-bridge` (+ `acs_decode`) reference-runtime binaries,
#   * Node 20 to run the product demos and the `arves` capability CLI,
#   * Python 3 to run the runtime certification harness.
#
# Design notes (why this shape):
#   * Multi-stage: a heavy Rust toolchain builds the binaries in `builder`,
#     then ONLY the resulting binaries + the source tree land in the slim
#     Node runtime image. The Rust toolchain never ships.
#   * Layout preservation: the SDK resolves the bridge at
#       runtime/target/debug/arves-bridge   (see products/arves-sdk-ts/src/bridge.mjs
#       line 17, a HARDCODED debug path) and the Python harness resolves the same
#     path (verification/certification/certify_runtime.py). We therefore place the
#     built binaries at EXACTLY runtime/target/debug/ inside the image and copy the
#     whole repo, so every relative path in QUICKSTART.md works unchanged.
#   * HONEST NOTE — this image ships a DEBUG build, on purpose. `cargo build`
#     without `--release` writes to target/debug/, which is exactly the path the
#     frozen SDK expects. Building with `--release` would emit target/release/
#     binaries the SDK would NOT find (the resolver path is frozen under
#     runtime/, changeable only via an RCR), so a release switch would break the
#     on-ramp. Consequence: these are UNOPTIMIZED binaries — correct and
#     deterministic (what the on-ramp needs), but not perf-tuned. A production
#     deployment that wants optimized binaries must land an RCR that makes the
#     SDK/harness resolve target/release/ (or a configurable path) first; until
#     then, debug is the honest, working artifact. This is a deployment-image
#     choice only — it does not touch or reinterpret anything under runtime/.
#   * IDR-006 / freeze: this Dockerfile lives at the repo root (a Living file);
#     it only *builds and invokes* the frozen runtime — it never edits runtime/
#     or standard/.
#   * Offline after base images: no `npm install` (products declare zero
#     dependencies) and no `pip install` (harness is stdlib-only). The only
#     network fetches are the two pinned base images and the crates cargo
#     resolves from Cargo.lock during the build. If you need a fully air-gapped
#     build, prime a local registry / vendored crates first (see docs/DEPLOY.md).
#
# Base images are pinned by tag + digest-friendly version for reproducibility.
# =============================================================================

# ----------------------------------------------------------------------------
# Stage 1 — Rust builder: compile the reference-runtime binaries.
# edition = "2021" across the workspace; 1.82 is a comfortable stable floor.
# ----------------------------------------------------------------------------
FROM rust:1.82-bookworm AS builder

WORKDIR /arves

# Copy the runtime workspace (and its committed Cargo.lock) so the build is
# reproducible. Only runtime/ is needed to compile the binaries.
COPY runtime/ ./runtime/

# Build the two binaries the demos and the certification harness invoke:
#   * arves-bridge  — the SDK<->Kernel seam the product demos speak to.
#   * acs_decode    — the negative-vector decoder the cert harness drives.
# Building these two packages pulls in the whole dependency subtree they need.
# --locked enforces the committed Cargo.lock (no silent dependency drift).
#
# DEBUG on purpose: no `--release` here. The frozen SDK/harness resolve the
# bridge at runtime/target/debug/arves-bridge (hardcoded), so we must emit to
# target/debug/. These binaries are UNOPTIMIZED. Switching to `--release` would
# put them in target/release/ where the SDK cannot find them and would break the
# on-ramp; changing that resolver path is an RCR, not a Dockerfile edit.
RUN cargo build --locked \
        --manifest-path runtime/Cargo.toml \
        -p arves-bridge   --bin arves-bridge \
        -p arves-conformance --bin acs_decode

# ----------------------------------------------------------------------------
# Stage 2 — runtime image: Node 20 (product demos + arves CLI) + Python 3.
# node:20-bookworm-slim ships Python3 is NOT guaranteed, so install it.
# ----------------------------------------------------------------------------
FROM node:20-bookworm-slim AS runtime

# Python 3 for the certification harness (stdlib only, no pip packages).
# Clean apt lists to keep the image small; no other network use at runtime.
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /arves

# Copy the full source tree so every relative path in QUICKSTART.md resolves:
# products/, verification/, standard/, docs/, tools/, etc.
COPY . .

# Drop in the compiled binaries at the exact path the SDK and harness expect.
# (`.dockerignore` excludes any host `runtime/target/` from the `COPY .` above,
#  so these freshly built binaries are authoritative — nothing to overwrite.)
COPY --from=builder /arves/runtime/target/debug/arves-bridge \
                    /arves/runtime/target/debug/acs_decode \
                    /arves/runtime/target/debug/

# Sanity: fail the build early if the binaries did not land where expected.
RUN test -x /arves/runtime/target/debug/arves-bridge \
    && test -x /arves/runtime/target/debug/acs_decode

# The ecosystem-sdk exposes the `arves` CLI (bin/arves.mjs). Wrap it as a
# `node`-invoking shim on PATH so `docker run ... arves certify <cap>` works
# regardless of whether arves.mjs carries an executable bit / shebang.
RUN printf '#!/bin/sh\nexec node /arves/products/arves-ecosystem-sdk/bin/arves.mjs "$@"\n' \
        > /usr/local/bin/arves \
    && chmod +x /usr/local/bin/arves

# Default: print the quickstart so a fresh `docker run` is self-documenting.
# Override the command to run a specific demo, e.g.:
#   docker run --rm arves-onramp node products/arves-personal-os/examples/my-day.mjs
CMD ["cat", "QUICKSTART.md"]

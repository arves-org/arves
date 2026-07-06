#!/usr/bin/env bash
# ARVES :: verification/industrial — L4 INDUSTRIAL EVIDENCE runner.
#
# Runs all three L4 tiers and regenerates the performance report. This crate is
# a STANDALONE cargo workspace that path-depends on the FROZEN runtime crates
# (IDR-006); it never modifies runtime/ and adds zero third-party crates.
#
# Usage:  ./run.sh            # FULL headline sweep (release) — the documented counts
#         ./run.sh quick      # fast smoke: small seed bands + small perf loads
#
# The full sweep is the L4_REPORT.md / README.md headline evidence
# (512/128/256/192 seeds, ~2-3 min). `quick` sets ARVES_L4_SMOKE so the
# fault-injection & replay sweeps run a fast prefix band instead — the same
# gate CI runs on every push (see .github/workflows/ci.yml `industrial`). You
# can also set ARVES_L4_SMOKE=<n> yourself for an exact band size.
set -euo pipefail
cd "$(dirname "$0")"

if [ "${1:-}" = "quick" ]; then
  export ARVES_L4_SMOKE="${ARVES_L4_SMOKE:-24}"
  echo "== SMOKE mode: ARVES_L4_SMOKE=$ARVES_L4_SMOKE (prefix seed band; full band on a bare ./run.sh) =="
fi

echo "== L4 tier 1+2: fault-injection & replay-equivalence (cargo test, release) =="
cargo test --release -- --nocapture

echo
echo "== L4 tier 3: performance harness (real fsync-durable Kernel) =="
if [ "${1:-}" = "quick" ]; then
  cargo run --release --bin l4_report -- 200 500 1000
else
  cargo run --release --bin l4_report
fi

echo
echo "== done. Report: verification/industrial/L4_REPORT.md =="

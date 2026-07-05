//! `cargo run -p arves-conformance --bin conformance_live` — run the live L1 conformance
//! scenario over the real codec + Kernel and print the emitted ConformanceArtifact + its
//! verdict. Exits non-zero unless the verdict is Pass (RCR-008/009).
//!
//! It runs the growing L1 pipeline scenario (Information → Kernel today; Query is the next node).
//! The final `LIVE-L1: PASS|FAIL` line is the stable machine marker the evidence probe greps.

use std::process::exit;

use arves_conformance::live::{render, run_l1_full_scenario};
use arves_conformance::Verdict;

fn main() {
    let artifact = run_l1_full_scenario();
    print!("{}", render(&artifact));
    let pass = artifact.verdict == Verdict::Pass;
    println!("  LIVE-L1: {}", if pass { "PASS" } else { "FAIL" });
    if !pass {
        exit(1);
    }
}

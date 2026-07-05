//! `cargo run -p arves-conformance --bin conformance_live` — run the live L1 Core-Runtime
//! conformance scenario over the real Kernel and print the emitted ConformanceArtifact +
//! its verdict. Exits non-zero unless the verdict is Pass (RCR-008).

use std::process::exit;

use arves_conformance::live::{render, run_core_runtime_scenario};
use arves_conformance::Verdict;

fn main() {
    let artifact = run_core_runtime_scenario();
    print!("{}", render(&artifact));
    if artifact.verdict == Verdict::Pass {
        println!("  live L1 core-runtime conformance: PASS");
    } else {
        println!("  live L1 core-runtime conformance: NOT PASS ({:?})", artifact.verdict);
        exit(1);
    }
}

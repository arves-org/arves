//! `cargo run -p arves-conformance --bin conformance_live` — run the live L1/L2 conformance
//! scenarios over the real codec + Kernel + engine fabric and print the emitted
//! ConformanceArtifacts + their verdicts. Exits non-zero unless every verdict is Pass
//! (RCR-008/009; Engine node added RCR-037).
//!
//! It runs the L1 pipeline scenario (Information → Kernel → Query) and the L2 pipeline
//! scenario (Information → Kernel → Query → Engine) — the Engine node closes the last
//! open pipeline-node probe (RCR-031's recorded honesty gap). The `LIVE-L1: PASS|FAIL`
//! line is the stable machine marker the evidence probe greps; `LIVE-ENGINE: PASS|FAIL`
//! is the Engine-node marker.

use std::process::exit;

use arves_conformance::live::{render, run_l1_full_scenario, run_l2_engine_pipeline_scenario};
use arves_conformance::Verdict;

fn main() {
    let l1 = run_l1_full_scenario();
    print!("{}", render(&l1));
    let l1_pass = l1.verdict == Verdict::Pass;
    println!("  LIVE-L1: {}", if l1_pass { "PASS" } else { "FAIL" });

    let l2 = run_l2_engine_pipeline_scenario();
    print!("{}", render(&l2));
    // The Engine node is the fourth node of the L2 pipeline artifact.
    let engine_pass = l2.verdict == Verdict::Pass
        && l2.node_evidence.iter().any(|ev| {
            ev.node == arves_conformance::PipelineNode::Engine
                && ev.invariant_checks.iter().all(|(_, o)| *o == arves_conformance::CheckOutcome::Held)
        });
    println!("  LIVE-ENGINE: {}", if engine_pass { "PASS" } else { "FAIL" });

    if !(l1_pass && engine_pass) {
        exit(1);
    }
}

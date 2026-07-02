//! `cargo run -p arves-conformance --bin conformance` — prints the ARVES
//! Conformance Report for the ACS layer and exits non-zero if any vector fails.
//! This is the entry a runtime (Rust today, Go/Java/Python later) invokes to prove
//! conformance against the executable standard.

use std::process::exit;

fn main() {
    let report = arves_conformance::acs::run();
    print!("{}", arves_conformance::acs::render(&report));

    let neg = arves_conformance::acs::run_negative();
    let neg_ok = arves_conformance::acs::negative_core_pass(&neg);

    if report.all_pass() && neg_ok {
        println!("\nVERDICT: CONFORMANT (ACS layer — positive + negative).");
    } else {
        println!("\nVERDICT: NON-CONFORMANT.");
        for r in report.results.iter().filter(|r| !r.pass) {
            eprintln!("  FAIL {} [{}]: got {} expected {}", r.standard, r.vector, r.content_id, r.expected);
        }
        for n in neg.iter().filter(|n| n.tier == "core" && !n.pass) {
            eprintln!("  FAIL neg {} [{}]: {}", n.standard, n.case, n.outcome);
        }
        exit(1);
    }
}

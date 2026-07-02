//! `cargo run -p arves-conformance --bin conformance` — prints the ARVES
//! Conformance Report for the ACS layer and exits non-zero if any vector fails.
//! This is the entry a runtime (Rust today, Go/Java/Python later) invokes to prove
//! conformance against the executable standard.

use std::process::exit;

fn main() {
    let report = arves_conformance::acs::run();
    print!("{}", arves_conformance::acs::render(&report));
    if report.all_pass() {
        println!("\nVERDICT: CONFORMANT (ACS layer).");
    } else {
        println!("\nVERDICT: NON-CONFORMANT.");
        for r in report.results.iter().filter(|r| !r.pass) {
            eprintln!("  FAIL {} [{}]: got {} expected {}", r.standard, r.vector, r.content_id, r.expected);
        }
        exit(1);
    }
}

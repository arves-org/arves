//! `cargo run -p arves-conformance --bin acs_negative_vectors` — emit the
//! machine-readable ACS-002 negative (rejection) vector corpus (TSV) for the
//! ARVES Standard Kit. Each row is a non-canonical byte string that a conformant
//! decoder MUST reject, with the normative reason. A second-language runtime proves
//! rejection conformance by rejecting every "core" row.

fn main() {
    let neg = arves_conformance::acs::run_negative();
    print!("{}", arves_conformance::acs::negative_tsv(&neg));
}

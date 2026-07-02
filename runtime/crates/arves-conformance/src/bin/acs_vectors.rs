//! `cargo run -p arves-conformance --bin acs_vectors` — emit the machine-readable,
//! language-neutral ACS golden-vector corpus (TSV) for the ARVES Standard Kit.
//! Generated from the reference codec; a second-language runtime targets these bytes.

fn main() {
    print!("{}", arves_conformance::acs::run().tsv());
}

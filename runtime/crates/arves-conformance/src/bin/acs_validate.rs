//! `cargo run -p arves-conformance --bin acs_validate` — the semantic (ACS-003/004/005)
//! reject harness (RCR-004b). Reads `<tier>\t<hex-body>` per line from stdin and prints one
//! verdict per line:
//!   `ACCEPT`                 — the semantic validator accepts the (already-canonical) body
//!   `REJECT\t<reason_code>`  — rejected, with the registered CCP-006 kebab reason code
//!   `ERR\tbad-hex`           — the hex was malformed
//!   `ERR\tbad-tier`          — tier not in {envelope, instance, language}
//!   `ERR\tnon-canonical`     — envelope/instance body is not canonical dCBOR (fails decode)
//!
//! `tier` is one of `envelope` | `instance` | `language`. This exposes the NATIVE Rust
//! ACS-003/004/005 validators (`arves_conformance::semantic`, RCR-004) over the same
//! line-protocol shape as `acs_decode`, so the certification harness can drive the Rust
//! reference over the FULL ACS surface inputs-only — not just the ACS-002 byte layer. The
//! grader still owns the truth (it feeds inputs and compares the reason code itself), so a
//! hollow adapter cannot game it.

use std::io::{self, BufRead, Write};

use arves_acs::cbor::{decode_canonical, encode};
use arves_conformance::semantic::{
    check_term_set, uci_fact_schema, validate_envelope, validate_instance,
};

fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    let b = s.as_bytes();
    let val = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut i = 0;
    while i < b.len() {
        out.push((val(b[i])? << 4) | val(b[i + 1])?);
        i += 2;
    }
    Some(out)
}

fn main() {
    // The frozen ACS-004 uci.fact@1.0 schema, decoded once (all instance vectors are uci.fact).
    let schema =
        decode_canonical(&encode(&uci_fact_schema())).expect("uci.fact schema is canonical");

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let tier = parts.next().unwrap_or("");
        let hexs = parts.next().unwrap_or("");
        let bytes = match from_hex(hexs) {
            Some(b) => b,
            None => {
                let _ = writeln!(out, "ERR\tbad-hex");
                continue;
            }
        };
        let verdict = match tier {
            "language" => match check_term_set(&bytes) {
                Ok(()) => "ACCEPT".to_string(),
                Err(e) => format!("REJECT\t{}", e.code()),
            },
            "envelope" | "instance" => match decode_canonical(&bytes) {
                Err(_) => "ERR\tnon-canonical".to_string(),
                Ok(v) => {
                    let res = if tier == "envelope" {
                        validate_envelope(&v)
                    } else {
                        validate_instance(&v, &schema)
                    };
                    match res {
                        Ok(()) => "ACCEPT".to_string(),
                        Err(e) => format!("REJECT\t{}", e.code()),
                    }
                }
            },
            _ => "ERR\tbad-tier".to_string(),
        };
        let _ = writeln!(out, "{verdict}");
    }
}

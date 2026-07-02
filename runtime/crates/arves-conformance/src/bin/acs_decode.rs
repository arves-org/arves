//! `cargo run -p arves-conformance --bin acs_decode` — the differential decode
//! harness. Reads one hex-encoded candidate body per line from stdin and prints one
//! verdict per line:
//!   `ACCEPT\t<canonical_reencoded_hex>`  — decoded and is canonical (round-trip)
//!   `REJECT\t<reason_code>`              — non-canonical, with the ACS-002 reason
//!   `ERR\tbad-hex`                        — the line was not valid hex
//! Any implementation exposes the same line protocol so a differential fuzzer can
//! feed identical inputs to two decoders and diff their verdicts byte-for-byte.

use std::io::{self, BufRead, Write};

use arves_acs::cbor::{decode_canonical, encode};
use arves_acs::hex;

fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let b = s.as_bytes();
    let val = |c: u8| -> Option<u8> {
        match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        }
    };
    let mut i = 0;
    while i < b.len() {
        let hi = val(b[i])?;
        let lo = val(b[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Some(out)
}

fn main() {
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
        match from_hex(line) {
            None => {
                let _ = writeln!(out, "ERR\tbad-hex");
            }
            Some(bytes) => match decode_canonical(&bytes) {
                Ok(v) => {
                    let _ = writeln!(out, "ACCEPT\t{}", hex(&encode(&v)));
                }
                Err(e) => {
                    let _ = writeln!(out, "REJECT\t{}", e.code());
                }
            },
        }
    }
}

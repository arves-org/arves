//! `cargo run -p arves-bridge --bin arves-bridge` — the SDK<->Kernel bridge server.
//!
//! Line protocol (so any language can drive the real reference Kernel):
//!   stdin:  `<domain_hex> <body_hex>`   e.g. `01 a46474797065...`
//!   stdout: `<content_id_hex> <status> <index>`  status = committed | already-committed
//!           or `ERR <reason>`
//! One in-memory Kernel persists across the session, so idempotency (ORCH-004) is
//! observable: re-sending the same body returns `already-committed`. The ContentId is
//! the ACS-001 address, so it equals what the SDK computes locally — one world.

use std::io::{self, BufRead, Write};

use arves_acs::hex;
use arves_bridge::commit_body;
use arves_kernel::{CommitError, MemKernel, ShardKey};
use arves_persistence::MemWalStore;

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let b = s.as_bytes();
    let v = |c: u8| -> Option<u8> {
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
        out.push((v(b[i])? << 4) | v(b[i + 1])?);
        i += 2;
    }
    Some(out)
}

fn main() {
    let kernel = MemKernel::new(MemWalStore::new());
    let shard = ShardKey { tenant: "t1".into(), workspace: "w1".into() };
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
        let mut it = line.split_whitespace();
        let dom = it.next().and_then(|s| u8::from_str_radix(s, 16).ok());
        let body = it.next().and_then(from_hex);
        match (dom, body) {
            (Some(d), Some(b)) => match commit_body(&kernel, shard.clone(), d, &b) {
                Ok(tr) => {
                    let _ = writeln!(out, "{} committed {}", hex(&tr.content.0), tr.index.0);
                }
                Err(CommitError::AlreadyCommitted(tr)) => {
                    let _ = writeln!(out, "{} already-committed {}", hex(&tr.content.0), tr.index.0);
                }
                Err(e) => {
                    let _ = writeln!(out, "ERR {e:?}");
                }
            },
            _ => {
                let _ = writeln!(out, "ERR bad-request");
            }
        }
        let _ = out.flush();
    }
}

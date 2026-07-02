//! `cargo run -p arves-bridge --bin arves-bridge` — the SDK↔Kernel/Engine bridge server.
//!
//! Line protocol (any language can drive the real reference runtime):
//!   `<domain_hex> <body_hex>`               → commit a body directly as ACS truth
//!   `invoke <capability> <domain_hex> <body_hex>` → run the FULL cognitive chain:
//!       Capability (resolve binding) → Engine (pure invoke) → Kernel (commit effect)
//! Response: `<content_id_hex> <status> <index>`  (status = committed | already-committed)
//!           or `ERR <reason>` (e.g. `ERR unbound`).
//! One in-memory Kernel + Capability registry + reference engine persist across the
//! session, so idempotency (ORCH-004) is observable. ContentIds are ACS-001 addresses —
//! identical to what the SDK computes locally (one world).

use std::io::{self, BufRead, Write};

use arves_acs::hex;
use arves_bridge::{commit_body, invoke, InvokeError};
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as CapShardKey,
};
use arves_engine_fabric::PureEngine;
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

    // Capability registry with one reference capability bound to a reference engine.
    let mut registry = MemRegistry::new();
    let cap_shard = CapShardKey { tenant: "t1".into(), workspace: "w1".into() };
    registry.register(&cap_shard, CapabilityId("derive.fact".into())).ok();
    registry
        .bind(CapabilityBinding {
            capability: CapabilityId("derive.fact".into()),
            shard: cap_shard,
            version: BindingVersion(1),
            provider: ProviderId("engine:derive.fact@1.0.0".into()),
            contract: InvocationContract {
                input_schema: "acs:uci.fact".into(),
                output_schema: "acs:uci.fact".into(),
                effect: EffectClass::ProposesWrite,
            },
        })
        .ok();
    // A pure reference engine: admits its input as a proposed fact (the Kernel commits it).
    let engine = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());

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
        let tok: Vec<&str> = line.split_whitespace().collect();
        let resp = if tok.first() == Some(&"invoke") {
            // invoke <capability> <domain_hex> <body_hex>
            match (tok.get(1), tok.get(2).and_then(|s| u8::from_str_radix(s, 16).ok()), tok.get(3).and_then(|s| from_hex(s))) {
                (Some(cap), Some(dom), Some(body)) => {
                    match invoke(&kernel, &registry, shard.clone(), cap, &engine, body, dom) {
                        Ok(o) => match o.effects.first() {
                            Some(e) => format!(
                                "{} {} {}",
                                hex(&e.truth.content.0),
                                if e.fresh { "committed" } else { "already-committed" },
                                e.truth.index.0
                            ),
                            None => "ERR no-effect".to_string(),
                        },
                        Err(InvokeError::Unbound(_)) => "ERR unbound".to_string(),
                        Err(InvokeError::Commit(e)) => format!("ERR {e:?}"),
                    }
                }
                _ => "ERR bad-request".to_string(),
            }
        } else {
            // <domain_hex> <body_hex>  (direct commit)
            match (tok.first().and_then(|s| u8::from_str_radix(s, 16).ok()), tok.get(1).and_then(|s| from_hex(s))) {
                (Some(dom), Some(body)) => match commit_body(&kernel, shard.clone(), dom, &body) {
                    Ok(tr) => format!("{} committed {}", hex(&tr.content.0), tr.index.0),
                    Err(CommitError::AlreadyCommitted(tr)) => format!("{} already-committed {}", hex(&tr.content.0), tr.index.0),
                    Err(e) => format!("ERR {e:?}"),
                },
                _ => "ERR bad-request".to_string(),
            }
        };
        let _ = writeln!(out, "{resp}");
        let _ = out.flush();
    }
}

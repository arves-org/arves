//! `cargo run -p arves-bridge --bin arves-bridge` — the SDK↔Kernel/Engine bridge server.
//!
//! Line protocol (any language can drive the real reference runtime):
//!   `<domain_hex> <body_hex>`               → commit a body directly as ACS truth
//!   `invoke <capability> <domain_hex> <body_hex>` → run the FULL cognitive chain:
//!       Capability (resolve binding) → Engine (pure invoke) → Kernel (commit effect)
//! Response: `<content_id_hex> <status> <index>`  (status = committed | already-committed)
//!           or `ERR <reason>` (e.g. `ERR unbound`).
//!
//! **Request-id correlation (RCR-011, additive):** a request line MAY carry an explicit
//! id as its first token — `id=<token>` (1..=64 non-whitespace bytes). The token is
//! echoed verbatim as the first token of that request's response line, so a client can
//! match responses **by id instead of by position**: a dropped, injected, or reordered
//! line can no longer shift every later response onto the wrong caller (the v1.1
//! positional-FIFO debt in `RUNTIME_FREEZE_v1.0.md`). Lines without the prefix behave
//! exactly as before — the extension is backward compatible. One honest boundary:
//! `ERR too-large` is emitted for an over-long line *before* any parsing, so it never
//! carries an id — clients keep a FIFO fallback for id-less response lines.
//!
//! One in-memory Kernel + Capability registry + reference engine persist across the
//! session, so idempotency (ORCH-004) is observable. ContentIds are ACS-001 addresses —
//! identical to what the SDK computes locally (one world).

use std::io::{self, BufRead, Write};

/// Hard cap on a single request line (1 MiB): a hostile unbounded line cannot exhaust
/// memory — it is answered with `ERR too-large` and the rest of the line discarded.
const MAX_LINE: usize = 1 << 20;

/// Read one '\n'-terminated line, bounded to MAX_LINE bytes. Returns `(line, truncated)`
/// or `None` at clean EOF. A truncated line's remainder (up to the newline) is discarded.
fn read_line_bounded<R: BufRead>(r: &mut R) -> Option<(String, bool)> {
    let mut buf: Vec<u8> = Vec::new();
    let (mut truncated, mut any) = (false, false);
    let mut byte = [0u8; 1];
    loop {
        match r.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                any = true;
                if byte[0] == b'\n' {
                    break;
                }
                if buf.len() < MAX_LINE {
                    buf.push(byte[0]);
                } else {
                    truncated = true;
                }
            }
        }
    }
    if !any {
        return None;
    }
    Some((String::from_utf8_lossy(&buf).into_owned(), truncated))
}

use arves_acs::hex;
use arves_bridge::{commit_body, invoke, InvokeError};
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as CapShardKey,
};
use arves_engine_fabric::{Engine, PureEngine};
use arves_kernel::{CommitError, Kernel, MemKernel, ShardKey};
use arves_persistence::MemWalStore;

/// Max accepted request-id token length (RCR-011). A longer id is refused as `ERR bad-id`.
const MAX_ID: usize = 64;

/// RCR-011: split an optional leading `id=<token>` off a (trimmed) request line.
/// `Ok((Some(id), rest))` when a well-formed id prefix is present; `Ok((None, line))`
/// when absent; `Err(())` for a malformed id (`id=` empty, or longer than `MAX_ID`) —
/// answered as `ERR bad-id` WITHOUT an echo (an untrusted, malformed id is not
/// reflected back to the stream).
fn split_request_id(line: &str) -> Result<(Option<&str>, &str), ()> {
    match line.strip_prefix("id=") {
        None => Ok((None, line)),
        Some(rest) => {
            let (id, tail) = match rest.find(char::is_whitespace) {
                Some(i) => (&rest[..i], rest[i..].trim_start()),
                None => (rest, ""),
            };
            if id.is_empty() || id.len() > MAX_ID {
                return Err(());
            }
            Ok((Some(id), tail))
        }
    }
}

/// Handle one id-stripped request against the session runtime; returns the response
/// payload (no id echo — `respond` adds it). Extracted from `main` so the protocol
/// logic is unit-testable (RCR-011).
fn handle_request<K, R, E>(line: &str, kernel: &K, registry: &R, shard: &ShardKey, engine: &E) -> String
where
    K: Kernel,
    R: CapabilityRegistry,
    E: Engine<Input = Vec<u8>>,
{
    let tok: Vec<&str> = line.split_whitespace().collect();
    if tok.first() == Some(&"invoke") {
        // invoke <capability> <domain_hex> <body_hex>
        match (tok.get(1), tok.get(2).and_then(|s| u8::from_str_radix(s, 16).ok()), tok.get(3).and_then(|s| from_hex(s))) {
            (Some(cap), Some(dom), Some(body)) => {
                match invoke(kernel, registry, shard.clone(), cap, engine, body, dom) {
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
                    Err(e) => format!("ERR {e:?}"),
                }
            }
            _ => "ERR bad-request".to_string(),
        }
    } else {
        // <domain_hex> <body_hex>  (direct commit)
        match (tok.first().and_then(|s| u8::from_str_radix(s, 16).ok()), tok.get(1).and_then(|s| from_hex(s))) {
            (Some(dom), Some(body)) => match commit_body(kernel, shard.clone(), dom, &body) {
                Ok(tr) => format!("{} committed {}", hex(&tr.content.0), tr.index.0),
                Err(CommitError::AlreadyCommitted(tr)) => format!("{} already-committed {}", hex(&tr.content.0), tr.index.0),
                Err(e) => format!("ERR {e:?}"),
            },
            _ => "ERR bad-request".to_string(),
        }
    }
}

/// Full per-line behaviour: id split → handle → id echo. Used by `main` and by the
/// protocol tests, so what is tested is exactly what the server does.
fn respond<K, R, E>(line: &str, kernel: &K, registry: &R, shard: &ShardKey, engine: &E) -> String
where
    K: Kernel,
    R: CapabilityRegistry,
    E: Engine<Input = Vec<u8>>,
{
    match split_request_id(line) {
        Err(()) => "ERR bad-id".to_string(),
        Ok((rid, req)) => {
            let payload = if req.is_empty() {
                "ERR bad-request".to_string()
            } else {
                handle_request(req, kernel, registry, shard, engine)
            };
            match rid {
                Some(id) => format!("{id} {payload}"),
                None => payload,
            }
        }
    }
}

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
    let mut reader = stdin.lock();
    while let Some((line, truncated)) = read_line_bounded(&mut reader) {
        if truncated {
            let _ = writeln!(out, "ERR too-large");
            let _ = out.flush();
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let resp = respond(line, &kernel, &registry, &shard, &engine);
        let _ = writeln!(out, "{resp}");
        let _ = out.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A live protocol session (one Kernel + registry + engine, like `main`), so
    /// idempotency across requests is exercised exactly as a client would see it.
    struct Session {
        kernel: MemKernel,
        registry: MemRegistry,
        shard: ShardKey,
        engine: PureEngine<fn(&[u8]) -> Vec<u8>>,
    }

    impl Session {
        fn new() -> Self {
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
            fn echo(b: &[u8]) -> Vec<u8> {
                b.to_vec()
            }
            Session {
                kernel: MemKernel::new(MemWalStore::new()),
                registry,
                shard: ShardKey { tenant: "t1".into(), workspace: "w1".into() },
                engine: PureEngine::new("derive.fact", "uci.fact", echo as fn(&[u8]) -> Vec<u8>),
            }
        }

        fn req(&self, line: &str) -> String {
            respond(line, &self.kernel, &self.registry, &self.shard, &self.engine)
        }
    }

    /// The hello-truth golden ContentId (domain 0x01) — pinned by the ACS-001 vectors.
    const HELLO_CID: &str = "122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e";

    // Un-prefixed lines behave exactly as before RCR-011 (backward compatibility).
    #[test]
    fn rcr011_plain_lines_unchanged() {
        let s = Session::new();
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} already-committed 0"));
        assert_eq!(s.req("zz"), "ERR bad-request");
    }

    // A well-formed id is echoed verbatim as the first token — on success AND on error —
    // so a client can match responses by id instead of position.
    #[test]
    fn rcr011_id_is_echoed_on_success_and_error() {
        let s = Session::new();
        assert_eq!(s.req("id=r7 01 68656c6c6f2d7472757468"), format!("r7 {HELLO_CID} committed 0"));
        assert_eq!(s.req("id=r8 01 68656c6c6f2d7472757468"), format!("r8 {HELLO_CID} already-committed 0"));
        assert_eq!(s.req("id=r9 zz"), "r9 ERR bad-request");
        assert_eq!(s.req("id=only"), "only ERR bad-request"); // id but no request body
    }

    // The full cognitive chain works under an id prefix too.
    #[test]
    fn rcr011_invoke_with_id() {
        let s = Session::new();
        assert_eq!(
            s.req("id=a1 invoke derive.fact 01 68656c6c6f2d7472757468"),
            format!("a1 {HELLO_CID} committed 0")
        );
        assert_eq!(s.req("id=a2 invoke nope 01 6161"), "a2 ERR unbound");
    }

    // A malformed id (empty, or over MAX_ID) is refused as `ERR bad-id` and NOT echoed:
    // an untrusted token is never reflected back into the response stream.
    #[test]
    fn rcr011_malformed_id_refused_without_echo() {
        let s = Session::new();
        assert_eq!(s.req("id= 01 6161"), "ERR bad-id");
        let long = format!("id={} 01 6161", "x".repeat(MAX_ID + 1));
        assert_eq!(s.req(&long), "ERR bad-id");
        // Boundary: exactly MAX_ID is accepted.
        let max = format!("id={} 01 68656c6c6f2d7472757468", "y".repeat(MAX_ID));
        assert!(s.req(&max).starts_with(&format!("{} ", "y".repeat(MAX_ID))));
    }
}

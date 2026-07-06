//! `cargo run -p arves-bridge --bin arves-bridge` — the SDK↔Kernel/Engine bridge server.
//!
//! Line protocol (any language can drive the real reference runtime):
//!   `<domain_hex> <body_hex>`               → commit a body directly as ACS truth
//!   `invoke <capability> <domain_hex> <body_hex>` → run the FULL cognitive chain:
//!       Capability (resolve binding) → Engine (pure invoke) → Kernel (commit effect)
//!   `bind <capability>`                     → register+bind a capability (RCR-016)
//!   `scan` | `scan bodies`                  → READ-ONLY enumerate the shard's committed
//!       truth by WAL replay (RCR-033) — content-ids, optionally with bodies
//!   `commit-as <agent_hex> <domain_hex> <body_hex>` → commit ATTRIBUTED truth (RCR-034):
//!       the agent id rides inside the committed payload (queryable via `scan`)
//! Response: `<content_id_hex> <status> <index>`  (status = committed | already-committed)
//!           or `bound <capability>` (for `bind`) or `scan <count> …` (for `scan`)
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
//! **Per-request shard selection (RCR-014, additive):** after the optional `id=` token,
//! a request line MAY carry `shard=<tenant>/<workspace>` (each part non-empty,
//! whitespace-free, ≤ 64 bytes) — the commit/invoke runs in THAT shard instead of the
//! default `t1/w1`. Same body in two shards is two distinct truths with distinct
//! indexes and idempotency scopes (SHARD-001) — one process can now serve many tenants.
//! Token order is fixed: `id=` first (if present), then `shard=` (shard-only is lawful).
//! A malformed shard spec is answered `ERR bad-shard` WITHOUT reflecting the untrusted
//! spec back (a *valid* id prefix is still echoed, for correlation — mirroring the
//! RCR-011 bad-id discipline). Lines without the token behave byte-identically to
//! before. Documented choice: `invoke` in a non-default shard where the capability was
//! never bound is honestly refused `ERR unbound` — there is NO implicit auto-bind;
//! use the `bind` verb (RCR-016) to bind it first.
//!
//! **Dynamic capability bind (RCR-016, additive):** `bind <capability>` (composable with
//! the `id=` / `shard=` prefixes) registers and binds `<capability>` in the target shard.
//! Response: `bound <capability>`; rebinding a name already bound in that shard is
//! IDEMPOTENT — it answers `bound` again and changes nothing. A missing / extra-token /
//! over-long (> 64 bytes) capability name is refused `ERR bad-request`. HONEST SCOPE:
//! this bin hosts exactly ONE engine — the reference `engine:derive.fact@1.0.0` — and
//! `bind` binds capability NAMES to that one engine identity; it does NOT load or
//! execute arbitrary engine code. Dynamic engine loading is not a bridge feature.
//! Confusability note: capability names are fully OPAQUE tokens (same treatment as on
//! `invoke`), so a name may visually mimic a protocol token — `bind shard=x/y` lawfully
//! binds the capability literally named `shard=x/y` and answers `bound shard=x/y`.
//! There is no ambiguity: the `id=` / `shard=` prefixes are recognized POSITIONALLY
//! (leading tokens of a request line only), never by the content of a name position.
//!
//! **Durable truth via `--wal-dir <path>` (RCR-015, additive):** by default the bin
//! hosts an in-memory Kernel — truth dies with the process. With `--wal-dir <path>` it
//! hosts the file-backed `FileKernel` (I1.5) over that directory: every commit is
//! fsync-durable, and on startup the Kernel RECOVERS the directory's committed truth by
//! deterministic replay (ORCH-003, lossless-or-loud), so re-proposing an already-committed
//! body after a process kill+restart answers `already-committed` with the SAME ContentId
//! and index. No flag → MemKernel, byte-identical to before. HONEST SCOPE: this is
//! **single-host durability** — per-frame CRC32 error-detection plus the RCR-002
//! SHA-256 hash-chain integrity digest (tamper-evident, not tamper-proof); there is NO
//! authentication/authorization on commit (v2.0 debt #8 in `RUNTIME_FREEZE_v1.0.md`) and
//! no replication (per-shard Raft is I2+). An unknown flag or an unrecoverable WAL is
//! refused loudly at startup — the bin never silently falls back to volatile memory.
//!
//! **Read-only WAL scan (RCR-033, additive):** `scan` (composable with the `id=` /
//! `shard=` prefixes) enumerates the target shard's committed truth by replaying the
//! Kernel's WAL through the Query layer's `ShardProjection` (OWN-001: this is the
//! read-only projection surface — NO write path, no `Kernel::commit` reachable from the
//! scan). Response: `scan <count>` followed by each committed truth's `<content_id_hex>`
//! in deterministic commit order; `scan bodies` appends `=<payload_hex>` to each id so a
//! caller can reconstruct the shard's committed set TOTALLY (JARVIS `why()`/`rebuild()`
//! read committed truth instead of re-supplying candidate bodies). Tenant-isolated
//! (SHARD-001): a scan of shard A never returns B's records. An empty / never-committed
//! shard answers `scan 0`; a shard whose retained log EXISTS but cannot be replayed
//! (compacted prefix, no query-side snapshot — RCR-023 DR-7) is refused LOUDLY as
//! `ERR scan-fault` rather than collapsed to `scan 0`, so truth-loss is never masked as
//! emptiness. Any extra token beyond `bodies` is refused `ERR bad-request`.
//!
//! **Attributed commit (RCR-034, additive):** `commit-as <agent_hex> <domain_hex>
//! <body_hex>` commits `body` wrapped in the I5 agent-attribution envelope, so the "Who"
//! rides INSIDE the committed payload (the WAL audit trail, IDR-005) and is recoverable
//! by a `scan` + the runtime attribution decoder. An attributed commit is a DISTINCT
//! truth from a plain commit of the same body (the envelope has a different ACS address);
//! plain `<domain> <body>` commits are byte-identical to before (backward compatible).
//! HONEST SCOPE: the agent id is a CLAIMED identity carried into the trail — this verb
//! does not verify the agent is registered truth, nor that the caller IS the agent (v2.0
//! debt #8). A malformed agent hex / domain / body is refused `ERR bad-request`.
//!
//! One Kernel + Capability registry + reference engine persist across the
//! session, so idempotency (ORCH-004) is observable. ContentIds are ACS-001 addresses —
//! identical to what the SDK computes locally (one world). The `scan` verb reads through
//! a SEPARATE, Arc-shared handle on the SAME durable store the Kernel commits to, so it
//! observes every committed truth without ever holding a write handle.

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
use arves_bridge::{commit_attributed, commit_body, invoke, scan_shard, InvokeError};
use arves_capability_fabric::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, EffectClass,
    InvocationContract, MemRegistry, ProviderId, ShardKey as CapShardKey,
};
use arves_control_plane::agents::AgentId;
use arves_engine_fabric::{Engine, PureEngine};
use arves_kernel::{CommitError, FileKernel, Kernel, MemKernel, ShardKey};
use arves_persistence::{FileWalStore, MemWalStore, WalStore};

/// Max accepted request-id token length (RCR-011). A longer id is refused as `ERR bad-id`.
const MAX_ID: usize = 64;

/// The ONE engine identity this bin hosts (RCR-016 binds capability names to it).
const REF_ENGINE: &str = "engine:derive.fact@1.0.0";

/// Max accepted capability-name token length for `bind` (RCR-016). Longer names are
/// refused as `ERR bad-request` (hygiene, mirroring `MAX_ID`).
const MAX_CAP: usize = 64;

/// RCR-016: register + bind `cap` in `shard` to the reference engine identity, under the
/// reference invocation contract. Used by `main` (default `derive.fact` in `t1/w1`) and
/// by the `bind` verb, so a dynamically bound name behaves exactly like the built-in one.
fn bind_reference_capability<R: CapabilityRegistry>(registry: &mut R, shard: &ShardKey, cap: &str) -> Result<(), ()> {
    // RCR-017: identical construction rules on both opaque keys make this conversion total.
    let cap_shard = CapShardKey::new(shard.tenant(), shard.workspace())
        .expect("a valid kernel ShardKey always converts to a capability ShardKey");
    registry.register(&cap_shard, CapabilityId(cap.into())).map_err(|_| ())?;
    registry
        .bind(CapabilityBinding {
            capability: CapabilityId(cap.into()),
            shard: cap_shard,
            version: BindingVersion(1),
            provider: ProviderId(REF_ENGINE.into()),
            contract: InvocationContract {
                input_schema: "acs:uci.fact".into(),
                output_schema: "acs:uci.fact".into(),
                effect: EffectClass::ProposesWrite,
            },
        })
        .map(|_| ())
        .map_err(|_| ())
}

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

/// Max accepted length of each shard part — tenant, workspace — (RCR-014).
const MAX_SHARD_PART: usize = 64;

/// The default shard every un-prefixed request runs in (byte-identical to pre-RCR-014).
fn default_shard() -> ShardKey {
    ShardKey::new("t1", "w1").expect("valid default shard")
}

/// RCR-014: split an optional leading `shard=<tenant>/<workspace>` off an (already
/// id-stripped) request line. Grammar: exactly one `/`, both parts non-empty and
/// ≤ `MAX_SHARD_PART` bytes (whitespace-freeness is guaranteed by tokenization).
/// `Err(())` for a malformed spec — answered `ERR bad-shard` WITHOUT reflecting the
/// untrusted spec back (RCR-011 discipline).
fn split_shard(line: &str) -> Result<(Option<ShardKey>, &str), ()> {
    match line.strip_prefix("shard=") {
        None => Ok((None, line)),
        Some(rest) => {
            let (spec, tail) = match rest.find(char::is_whitespace) {
                Some(i) => (&rest[..i], rest[i..].trim_start()),
                None => (rest, ""),
            };
            let mut parts = spec.split('/');
            match (parts.next(), parts.next(), parts.next()) {
                (Some(t), Some(w), None)
                    if !t.is_empty()
                        && !w.is_empty()
                        && t.len() <= MAX_SHARD_PART
                        && w.len() <= MAX_SHARD_PART =>
                {
                    // RCR-017: the opaque constructor re-checks (non-empty, ≤256B);
                    // this grammar (≤64B) is strictly tighter, so it cannot refuse —
                    // a construction failure is still mapped to `ERR bad-shard`.
                    ShardKey::new(t, w).map(|k| (Some(k), tail)).map_err(|_| ())
                }
                _ => Err(()),
            }
        }
    }
}

/// Handle one id/shard-stripped request against the session runtime; returns the
/// response payload (no id echo — `respond` adds it). Extracted from `main` so the
/// protocol logic is unit-testable (RCR-011). `registry` is `&mut` for the `bind`
/// verb (RCR-016); commit/invoke never mutate it. `store` is the READ-ONLY handle the
/// `scan` verb (RCR-033) replays — it is never used to write.
fn handle_request<K, R, E, S>(
    line: &str,
    kernel: &K,
    store: &S,
    registry: &mut R,
    shard: &ShardKey,
    engine: &E,
) -> String
where
    K: Kernel,
    R: CapabilityRegistry,
    E: Engine<Input = Vec<u8>>,
    S: WalStore,
{
    let tok: Vec<&str> = line.split_whitespace().collect();
    if tok.first() == Some(&"scan") {
        // scan | scan bodies  (RCR-033): READ-ONLY enumeration of the shard's
        // committed truth via WAL replay. `scan` streams content-ids only;
        // `scan bodies` appends `=<payload_hex>` to each. Deterministic commit
        // order, tenant-isolated (the projection never holds a foreign shard's
        // record). Any other token shape is refused — no untrusted echo.
        let with_bodies = match (tok.get(1), tok.len()) {
            (None, 1) => false,
            (Some(&"bodies"), 2) => true,
            _ => return "ERR bad-request".to_string(),
        };
        // A never-committed shard scans as an honest empty; a shard whose retained
        // log EXISTS but cannot be replayed is a ScanFault (RCR-023 DR-7) — refuse it
        // LOUDLY as `ERR scan-fault` rather than let truth-loss read as `scan 0`.
        let truths = match scan_shard(store, shard) {
            Ok(t) => t,
            Err(_) => return "ERR scan-fault".to_string(),
        };
        let mut out = format!("scan {}", truths.len());
        for (content, payload, _idx) in &truths {
            out.push(' ');
            out.push_str(&hex(content));
            if with_bodies {
                out.push('=');
                out.push_str(&hex(payload));
            }
        }
        return out;
    }
    if tok.first() == Some(&"commit-as") {
        // commit-as <agent_hex> <domain_hex> <body_hex>  (RCR-034): commit the
        // body wrapped in the I5 attribution envelope, so the Who rides inside
        // committed truth. A malformed agent hex / domain / body is refused.
        return match (
            tok.get(1).and_then(|s| AgentId::from_hex(s)),
            tok.get(2).and_then(|s| u8::from_str_radix(s, 16).ok()),
            tok.get(3).and_then(|s| from_hex(s)),
            tok.len(),
        ) {
            (Some(agent), Some(dom), Some(body), 4) => {
                match commit_attributed(kernel, shard.clone(), dom, &agent, &body) {
                    Ok(tr) => format!("{} committed {}", hex(&tr.content.0), tr.index.0),
                    Err(CommitError::AlreadyCommitted(tr)) => {
                        format!("{} already-committed {}", hex(&tr.content.0), tr.index.0)
                    }
                    Err(e) => format!("ERR {e:?}"),
                }
            }
            _ => "ERR bad-request".to_string(),
        };
    }
    if tok.first() == Some(&"bind") {
        // bind <capability>  (RCR-016): register+bind the name to the ONE reference
        // engine identity in the target shard. Idempotent: an already-bound name
        // answers `bound` again (every binding in this bin points at REF_ENGINE, so
        // there is nothing to supersede).
        return match (tok.get(1), tok.len()) {
            (Some(cap), 2) if cap.len() <= MAX_CAP => {
                // RCR-017: identical construction rules on both opaque keys — total conversion.
                let cap_shard = CapShardKey::new(shard.tenant(), shard.workspace())
                    .expect("a valid kernel ShardKey always converts to a capability ShardKey");
                if registry.resolve(&cap_shard, &CapabilityId((*cap).into())).is_ok() {
                    format!("bound {cap}")
                } else {
                    match bind_reference_capability(registry, shard, cap) {
                        Ok(()) => format!("bound {cap}"),
                        Err(()) => "ERR bind-failed".to_string(),
                    }
                }
            }
            _ => "ERR bad-request".to_string(),
        };
    }
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

/// Full per-line behaviour: id split → shard split (RCR-014) → handle → id echo. Used
/// by `main` and by the protocol tests, so what is tested is exactly what the server does.
fn respond<K, R, E, S>(line: &str, kernel: &K, store: &S, registry: &mut R, engine: &E) -> String
where
    K: Kernel,
    R: CapabilityRegistry,
    E: Engine<Input = Vec<u8>>,
    S: WalStore,
{
    match split_request_id(line) {
        Err(()) => "ERR bad-id".to_string(),
        Ok((rid, rest)) => {
            let payload = match split_shard(rest) {
                Err(()) => "ERR bad-shard".to_string(),
                Ok((shard, req)) => {
                    let shard = shard.unwrap_or_else(default_shard);
                    if req.is_empty() {
                        "ERR bad-request".to_string()
                    } else {
                        handle_request(req, kernel, store, registry, &shard, engine)
                    }
                }
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

/// The session loop over ONE Kernel (in-memory or file-backed, RCR-015): registry +
/// reference engine + line protocol until EOF. Generic so the durable and volatile
/// arms run the IDENTICAL protocol logic — `--wal-dir` changes durability, never behaviour.
fn serve<K: Kernel, S: WalStore>(kernel: &K, store: &S) {
    // Capability registry with one reference capability bound to the reference engine
    // in the default shard (further names/shards bind via the `bind` verb, RCR-016).
    let mut registry = MemRegistry::new();
    bind_reference_capability(&mut registry, &default_shard(), "derive.fact").expect("default binding");
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
        let resp = respond(line, kernel, store, &mut registry, &engine);
        let _ = writeln!(out, "{resp}");
        let _ = out.flush();
    }
}

fn main() {
    // RCR-015: `--wal-dir <path>` selects the durable file-backed Kernel. Anything else
    // on the command line is refused LOUDLY (a mistyped `--waldir` silently falling back
    // to volatile memory would be a durability trap). No args → MemKernel, byte-identical.
    let mut wal_dir: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--wal-dir" => match args.next() {
                Some(dir) => wal_dir = Some(dir),
                None => {
                    eprintln!("arves-bridge: --wal-dir requires a path; usage: arves-bridge [--wal-dir <path>]");
                    std::process::exit(64);
                }
            },
            other => {
                eprintln!("arves-bridge: unknown argument '{other}'; usage: arves-bridge [--wal-dir <path>]");
                std::process::exit(64);
            }
        }
    }
    match wal_dir {
        Some(dir) => {
            // Durable arm: fsync-durable on-disk WAL + deterministic recovery replay
            // (ORCH-003, lossless-or-loud — an unrecoverable directory refuses startup
            // rather than serving partial truth).
            let store = FileWalStore::open_root(&dir).unwrap_or_else(|e| {
                eprintln!("arves-bridge: cannot open --wal-dir '{dir}': {e:?}");
                std::process::exit(65);
            });
            // RCR-033: keep an Arc-shared read handle on the same store for the
            // read-only `scan` verb; the Kernel takes the other and owns the write path.
            let read_store = store.clone();
            let kernel = FileKernel::try_recover(store).unwrap_or_else(|e| {
                eprintln!("arves-bridge: unrecoverable durable state in '{dir}': {e}");
                std::process::exit(66);
            });
            serve(&kernel, &read_store);
        }
        None => {
            let store = MemWalStore::new();
            let read_store = store.clone();
            serve(&MemKernel::new(store), &read_store);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A live protocol session (one Kernel + registry + engine, like `main`), so
    /// idempotency across requests is exercised exactly as a client would see it.
    /// `store` is the Arc-shared read handle the `scan` verb (RCR-033) replays —
    /// the SAME log the Kernel commits to, exactly as `main` wires it.
    struct Session {
        kernel: MemKernel,
        store: MemWalStore,
        registry: MemRegistry,
        engine: PureEngine<fn(&[u8]) -> Vec<u8>>,
    }

    impl Session {
        fn new() -> Self {
            let mut registry = MemRegistry::new();
            bind_reference_capability(&mut registry, &default_shard(), "derive.fact").expect("default binding");
            fn echo(b: &[u8]) -> Vec<u8> {
                b.to_vec()
            }
            let store = MemWalStore::new();
            Session {
                kernel: MemKernel::new(store.clone()),
                store,
                registry,
                engine: PureEngine::new("derive.fact", "uci.fact", echo as fn(&[u8]) -> Vec<u8>),
            }
        }

        fn req(&mut self, line: &str) -> String {
            respond(line, &self.kernel, &self.store, &mut self.registry, &self.engine)
        }
    }

    /// The hello-truth golden ContentId (domain 0x01) — pinned by the ACS-001 vectors.
    const HELLO_CID: &str = "122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e";

    // Un-prefixed lines behave exactly as before RCR-011 (backward compatibility).
    #[test]
    fn rcr011_plain_lines_unchanged() {
        let mut s = Session::new();
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} already-committed 0"));
        assert_eq!(s.req("zz"), "ERR bad-request");
    }

    // A well-formed id is echoed verbatim as the first token — on success AND on error —
    // so a client can match responses by id instead of position.
    #[test]
    fn rcr011_id_is_echoed_on_success_and_error() {
        let mut s = Session::new();
        assert_eq!(s.req("id=r7 01 68656c6c6f2d7472757468"), format!("r7 {HELLO_CID} committed 0"));
        assert_eq!(s.req("id=r8 01 68656c6c6f2d7472757468"), format!("r8 {HELLO_CID} already-committed 0"));
        assert_eq!(s.req("id=r9 zz"), "r9 ERR bad-request");
        assert_eq!(s.req("id=only"), "only ERR bad-request"); // id but no request body
    }

    // The full cognitive chain works under an id prefix too.
    #[test]
    fn rcr011_invoke_with_id() {
        let mut s = Session::new();
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
        let mut s = Session::new();
        assert_eq!(s.req("id= 01 6161"), "ERR bad-id");
        let long = format!("id={} 01 6161", "x".repeat(MAX_ID + 1));
        assert_eq!(s.req(&long), "ERR bad-id");
        // Boundary: exactly MAX_ID is accepted.
        let max = format!("id={} 01 68656c6c6f2d7472757468", "y".repeat(MAX_ID));
        assert!(s.req(&max).starts_with(&format!("{} ", "y".repeat(MAX_ID))));
    }

    // RCR-014: without a shard token every request runs in the default t1/w1 shard,
    // byte-identical to before; SAME body under `shard=` is a DISTINCT truth with its
    // own index sequence and idempotency scope (SHARD-001 isolation, observable).
    #[test]
    fn rcr014_two_shards_two_distinct_truths() {
        let mut s = Session::new();
        // Default shard: commit + idempotent recommit (unchanged behaviour).
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} already-committed 0"));
        // Same body in a second shard: FRESH truth (not already-committed) — distinct
        // idempotency scope. If the shard token were ignored, this would bite.
        assert_eq!(s.req("shard=t2/w2 01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        assert_eq!(s.req("shard=t2/w2 01 68656c6c6f2d7472757468"), format!("{HELLO_CID} already-committed 0"));
        // Distinct index sequences: a second body lands at index 1 in t1/w1 but the
        // second shard's log is independent.
        assert!(s.req("01 6161").ends_with(" committed 1"));
        assert!(s.req("shard=t2/w2 01 6262").ends_with(" committed 1"));
        assert!(s.req("shard=t3/w3 01 6161").ends_with(" committed 0"));
    }

    // RCR-014: token order is id first, then shard; shard-only is lawful; a shard
    // token in the wrong position is not silently honoured.
    #[test]
    fn rcr014_token_ordering() {
        let mut s = Session::new();
        assert_eq!(
            s.req("id=q1 shard=t2/w2 01 68656c6c6f2d7472757468"),
            format!("q1 {HELLO_CID} committed 0")
        );
        // Shard-only (no id) is lawful.
        assert_eq!(s.req("shard=t2/w2 01 68656c6c6f2d7472757468"), format!("{HELLO_CID} already-committed 0"));
        // Wrong order (shard before id) — the id= token is NOT parsed as an id and the
        // request body is malformed: refused, never mis-routed.
        assert_eq!(s.req("shard=t2/w2 id=q2 01 6161"), "ERR bad-request");
    }

    // RCR-014: a malformed shard spec is refused as `ERR bad-shard` and the untrusted
    // spec is never reflected back; a VALID id prefix is still echoed for correlation.
    #[test]
    fn rcr014_malformed_shard_refused() {
        let mut s = Session::new();
        assert_eq!(s.req("shard= 01 6161"), "ERR bad-shard"); // empty spec
        assert_eq!(s.req("shard=t2 01 6161"), "ERR bad-shard"); // no '/'
        assert_eq!(s.req("shard=/w2 01 6161"), "ERR bad-shard"); // empty tenant
        assert_eq!(s.req("shard=t2/ 01 6161"), "ERR bad-shard"); // empty workspace
        assert_eq!(s.req("shard=t2/w2/x 01 6161"), "ERR bad-shard"); // extra '/'
        let long = format!("shard=t2/{} 01 6161", "z".repeat(MAX_SHARD_PART + 1));
        assert_eq!(s.req(&long), "ERR bad-shard"); // over-long part
        // Boundary: exactly MAX_SHARD_PART is accepted.
        let max = format!("shard={}/w 01 6161", "y".repeat(MAX_SHARD_PART));
        assert!(max.len() < MAX_LINE && s.req(&max).ends_with(" committed 0"));
        // A valid id still correlates the refusal.
        assert_eq!(s.req("id=e1 shard=broken 01 6161"), "e1 ERR bad-shard");
    }

    // RCR-014 documented choice: invoke in a non-default shard where the capability
    // was never bound is honestly refused ERR unbound — no implicit auto-bind.
    #[test]
    fn rcr014_invoke_in_unbound_shard_refused() {
        let mut s = Session::new();
        assert_eq!(s.req("shard=t2/w2 invoke derive.fact 01 6161"), "ERR unbound");
        // The same invoke in the default shard (bound at startup) runs the full chain.
        assert_eq!(
            s.req("invoke derive.fact 01 68656c6c6f2d7472757468"),
            format!("{HELLO_CID} committed 0")
        );
    }

    // RCR-016: bind → invoke in a fresh shard turns ERR unbound into a committed truth;
    // a name that was never bound in that shard remains honestly ERR unbound.
    #[test]
    fn rcr016_bind_then_invoke_in_fresh_shard() {
        let mut s = Session::new();
        assert_eq!(s.req("shard=t9/w9 invoke summarize.doc 01 6161"), "ERR unbound");
        assert_eq!(s.req("shard=t9/w9 bind summarize.doc"), "bound summarize.doc");
        assert_eq!(
            s.req("shard=t9/w9 invoke summarize.doc 01 68656c6c6f2d7472757468"),
            format!("{HELLO_CID} committed 0")
        );
        // A different, never-bound name in the same shard is still refused.
        assert_eq!(s.req("shard=t9/w9 invoke other.cap 01 6161"), "ERR unbound");
        // ...and the bind is shard-scoped: the same name in ANOTHER shard stays unbound.
        assert_eq!(s.req("shard=t8/w8 invoke summarize.doc 01 6161"), "ERR unbound");
    }

    // RCR-016: rebinding the same name in the same shard is IDEMPOTENT (`bound` again,
    // no error, nothing superseded) — including the built-in default binding.
    #[test]
    fn rcr016_rebind_is_idempotent() {
        let mut s = Session::new();
        assert_eq!(s.req("shard=t9/w9 bind summarize.doc"), "bound summarize.doc");
        assert_eq!(s.req("shard=t9/w9 bind summarize.doc"), "bound summarize.doc");
        assert_eq!(s.req("bind derive.fact"), "bound derive.fact"); // pre-bound default
        // The binding still works after the idempotent rebinds.
        assert_eq!(
            s.req("shard=t9/w9 invoke summarize.doc 01 68656c6c6f2d7472757468"),
            format!("{HELLO_CID} committed 0")
        );
    }

    // RCR-016: `bind` composes with the id= and shard= prefixes (id echoed, shard honoured).
    #[test]
    fn rcr016_bind_composes_with_id_and_shard() {
        let mut s = Session::new();
        assert_eq!(s.req("id=b1 shard=t9/w9 bind summarize.doc"), "b1 bound summarize.doc");
        assert_eq!(
            s.req("id=b2 shard=t9/w9 invoke summarize.doc 01 68656c6c6f2d7472757468"),
            format!("b2 {HELLO_CID} committed 0")
        );
    }

    // RCR-015: truth committed through a FILE-backed Kernel survives the Kernel's death.
    // Session 1 commits over a --wal-dir directory and is dropped; session 2 RECOVERS the
    // same directory (deterministic replay, ORCH-003) and the SAME body answers
    // `already-committed` with the SAME ContentId and index. This is the in-process half
    // of the durability proof; the real kill-the-process/restart half runs over the real
    // exe in `products/robustness.test.mjs`.
    #[test]
    fn rcr015_durable_wal_survives_kernel_death() {
        let dir = std::env::temp_dir().join(format!("arves-rcr015-bin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        {
            // Session 1: durable commit, then the Kernel dies (dropped).
            let store = FileWalStore::open_root(&dir).expect("open wal dir");
            let read = store.clone();
            let kernel = FileKernel::new(store);
            let mut registry = MemRegistry::new();
            bind_reference_capability(&mut registry, &default_shard(), "derive.fact").expect("bind");
            let engine = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
            assert_eq!(
                respond("01 68656c6c6f2d7472757468", &kernel, &read, &mut registry, &engine),
                format!("{HELLO_CID} committed 0")
            );
        }
        {
            // Session 2: a FRESH Kernel recovers the directory — same body, same identity.
            let store = FileWalStore::open_root(&dir).expect("reopen wal dir");
            let read = store.clone();
            let kernel = FileKernel::try_recover(store).expect("recover durable truth");
            let mut registry = MemRegistry::new();
            bind_reference_capability(&mut registry, &default_shard(), "derive.fact").expect("bind");
            let engine = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
            assert_eq!(
                respond("01 68656c6c6f2d7472757468", &kernel, &read, &mut registry, &engine),
                format!("{HELLO_CID} already-committed 0"),
                "recovered truth must answer already-committed with the SAME ContentId + index"
            );
            // A NEW body in the recovered session lands at the next index — the recovered
            // log position is exact, not merely non-empty.
            assert!(respond("01 6161", &kernel, &read, &mut registry, &engine).ends_with(" committed 1"));
            // RCR-033: a scan over the recovered durable store sees BOTH committed truths
            // in commit order — total reconstruction survives the Kernel's death.
            let scan = respond("scan", &kernel, &read, &mut registry, &engine);
            assert!(scan.starts_with("scan 2 ") && scan.contains(HELLO_CID));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // RCR-015: without --wal-dir the bin constructs MemKernel — the volatile arm's
    // protocol behaviour is what every pre-existing test in this module exercises
    // (byte-identical); here we pin that a FILE-backed session behaves identically on
    // the protocol surface for a fresh directory (first commit fresh at index 0).
    #[test]
    fn rcr015_file_kernel_protocol_parity_on_fresh_dir() {
        let dir = std::env::temp_dir().join(format!("arves-rcr015-parity-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = FileWalStore::open_root(&dir).expect("open wal dir");
        let read = store.clone();
        let kernel = FileKernel::new(store);
        let mut registry = MemRegistry::new();
        bind_reference_capability(&mut registry, &default_shard(), "derive.fact").expect("bind");
        let engine = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());
        assert_eq!(
            respond("id=d1 invoke derive.fact 01 68656c6c6f2d7472757468", &kernel, &read, &mut registry, &engine),
            format!("d1 {HELLO_CID} committed 0"),
            "the full cognitive chain runs identically over the durable Kernel"
        );
        assert_eq!(
            respond("01 68656c6c6f2d7472757468", &kernel, &read, &mut registry, &engine),
            format!("{HELLO_CID} already-committed 0")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // RCR-016: a malformed bind (no name, extra tokens, over-long name) is ERR bad-request.
    #[test]
    fn rcr016_malformed_bind_refused() {
        let mut s = Session::new();
        assert_eq!(s.req("bind"), "ERR bad-request");
        assert_eq!(s.req("bind a b"), "ERR bad-request");
        let long = format!("bind {}", "c".repeat(MAX_CAP + 1));
        assert_eq!(s.req(&long), "ERR bad-request");
        // Boundary: exactly MAX_CAP is accepted.
        let max_name = "d".repeat(MAX_CAP);
        assert_eq!(s.req(&format!("bind {max_name}")), format!("bound {max_name}"));
    }

    // RCR-033: `scan` returns exactly the shard's committed set, in commit order.
    // A fresh session scans empty; after commits it streams the content-ids;
    // `scan bodies` appends each payload; malformed scan is refused.
    #[test]
    fn rcr033_scan_enumerates_committed_set_in_order() {
        let mut s = Session::new();
        // Empty shard: honest `scan 0`.
        assert_eq!(s.req("scan"), "scan 0");
        assert_eq!(s.req("scan bodies"), "scan 0");
        // Commit hello-truth then "aa" — two truths, commit order 0,1.
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        let aa = s.req("01 6161");
        let aa_cid = aa.split(' ').next().unwrap().to_string();
        // scan streams both ids in commit order.
        assert_eq!(s.req("scan"), format!("scan 2 {HELLO_CID} {aa_cid}"));
        // scan bodies appends `=<payload_hex>` to each (hello-truth body, then "aa").
        assert_eq!(
            s.req("scan bodies"),
            format!("scan 2 {HELLO_CID}=68656c6c6f2d7472757468 {aa_cid}=6161")
        );
        // A malformed scan (extra token) is refused without echoing it.
        assert_eq!(s.req("scan garbage"), "ERR bad-request");
        assert_eq!(s.req("scan bodies extra"), "ERR bad-request");
        // Composes with id= (echoed) and is idempotent as a read.
        assert_eq!(s.req("id=z1 scan"), format!("z1 scan 2 {HELLO_CID} {aa_cid}"));
    }

    // RCR-033: `scan` is tenant-isolated — a scan of shard A never returns B's records.
    #[test]
    fn rcr033_scan_is_tenant_isolated() {
        let mut s = Session::new();
        // Commit hello-truth in the default shard and "bb" in t2/w2.
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        assert!(s.req("shard=t2/w2 01 6262").ends_with(" committed 0"));
        // Default shard sees only hello-truth; t2/w2 sees only "bb" — never each other's.
        assert_eq!(s.req("scan"), format!("scan 1 {HELLO_CID}"));
        let bb = s.req("shard=t2/w2 scan");
        assert!(bb.starts_with("scan 1 ") && !bb.contains(HELLO_CID));
        // A never-committed shard scans empty.
        assert_eq!(s.req("shard=t9/w9 scan"), "scan 0");
    }

    // RCR-033 (adversarial finding): a shard whose retained log EXISTS but cannot be
    // replayed (compacted prefix, no query-side snapshot — RCR-023 DR-7) is refused
    // LOUDLY as `ERR scan-fault`, never collapsed to `scan 0`. Truth-loss must not
    // masquerade as emptiness on the wire; a never-committed shard still scans empty.
    #[test]
    fn rcr033_unreadable_log_is_scan_fault_not_empty() {
        use arves_persistence::{ShardKey as PShardKey, Wal};
        let mut s = Session::new();
        // Commit two truths in the default shard, then compact its log prefix away.
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        assert!(s.req("01 6161").ends_with(" committed 1"));
        let pshard = PShardKey { tenant: "t1".into(), workspace: "w1".into() };
        let mut wal = s.store.open(&pshard).unwrap();
        wal.compact(0).unwrap(); // drop offset 0: the head fold is no longer reproducible
        // The unreadable log is refused loudly, NOT reported as `scan 0`.
        assert_eq!(s.req("scan"), "ERR scan-fault");
        // A valid id still correlates the fault; a truly fresh shard still scans empty.
        assert_eq!(s.req("id=f1 scan"), "f1 ERR scan-fault");
        assert_eq!(s.req("shard=t9/w9 scan"), "scan 0");
    }

    // RCR-034: `commit-as` records the Who inside committed truth; the attribution
    // round-trips (queryable via scan + the runtime decoder), it is a DISTINCT truth
    // from a plain commit of the same body, and unattributed commit still works.
    #[test]
    fn rcr034_attributed_commit_round_trips_and_is_queryable() {
        use arves_control_plane::agents::decode_attributed;
        let mut s = Session::new();
        // Plain commit of the body (backward compatible).
        assert_eq!(s.req("01 68656c6c6f2d7472757468"), format!("{HELLO_CID} committed 0"));
        // Attributed commit of the SAME body: a DISTINCT truth (different ContentId).
        let attr = s.req("commit-as 1220abcd 01 68656c6c6f2d7472757468");
        let attr_cid = attr.split(' ').next().unwrap().to_string();
        assert!(attr.ends_with(" committed 1"));
        assert_ne!(attr_cid, HELLO_CID, "attribution changes identity");
        // Queryable: a scan-with-bodies yields the envelope, which decodes to (who, what).
        let scan = s.req("scan bodies");
        let env_hex = scan
            .split(' ')
            .find_map(|t| t.strip_prefix(&format!("{attr_cid}=")))
            .expect("attributed truth is in the scan");
        let env = from_hex(env_hex).expect("hex body");
        let (who, what) = decode_attributed(&env).expect("is an attribution envelope");
        assert_eq!(who.hex(), "1220abcd");
        assert_eq!(what, b"hello-truth".to_vec());
        // The plain truth is NOT an attribution envelope.
        let plain_hex = scan.split(' ').find_map(|t| t.strip_prefix(&format!("{HELLO_CID}="))).unwrap();
        assert_eq!(decode_attributed(&from_hex(plain_hex).unwrap()), None);
        // Re-attributing the same (agent, body) is idempotent (ORCH-004).
        assert_eq!(s.req("commit-as 1220abcd 01 68656c6c6f2d7472757468"), format!("{attr_cid} already-committed 1"));
    }

    // RCR-034: a malformed commit-as (bad agent hex, missing/extra tokens, bad body) is
    // refused ERR bad-request; it composes with id= (echoed) and shard=.
    #[test]
    fn rcr034_malformed_commit_as_refused() {
        let mut s = Session::new();
        assert_eq!(s.req("commit-as"), "ERR bad-request");
        assert_eq!(s.req("commit-as 1220abcd 01"), "ERR bad-request"); // no body
        assert_eq!(s.req("commit-as zz 01 6161"), "ERR bad-request"); // bad agent hex
        assert_eq!(s.req("commit-as 1220ab 01 zz"), "ERR bad-request"); // bad body hex
        assert_eq!(s.req("commit-as 1220ab 01 6161 extra"), "ERR bad-request"); // extra token
        // A valid id still correlates the refusal.
        assert_eq!(s.req("id=e1 commit-as zz 01 6161"), "e1 ERR bad-request");
        // Composes with shard= : attributed truth lands in the named shard.
        assert!(s.req("shard=t2/w2 commit-as 1220ab 01 6161").ends_with(" committed 0"));
    }
}

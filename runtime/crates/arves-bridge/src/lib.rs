//! ARVES Bridge — the single seam between the product/SDK world and the reference
//! runtime (Kernel / Engine).
//!
//! The reference Kernel addresses truth by an **opaque, caller-supplied**
//! `ContentHash` (arves-kernel), which drives ORCH-004 idempotency. Nothing in the
//! Kernel forces that address to be the ACS-001 ContentId — so a naive product could
//! commit truth under a different identity than the standard defines, creating two
//! divergent worlds (the largest architectural risk of the product era).
//!
//! The bridge closes that seam: it computes the ACS-001 address of the ACS-002
//! canonical body via `arves-acs` and commits it as the Kernel's `ContentHash`. The
//! `TruthRef` the Kernel returns is therefore addressed by the *same* ContentId the
//! SDK (Rust / Python / TypeScript) computes locally — one world, one identity. This
//! is where the Kernel CONSUMES the standard.

use arves_acs::{cbor, content_id};
use arves_capability_fabric::{CapabilityId, CapabilityRegistry, ShardKey as CapShardKey};
use arves_control_plane::agents::{encode_attributed, AgentId};
use arves_engine_fabric::{invoke_enforced, Engine, FabricViolation};
use arves_kernel::{CommitError, ContentHash, Kernel, ProposedWrite, ShardKey, TruthRef};
use arves_persistence::{ShardKey as PShardKey, WalStore};
use arves_query::projection::ShardProjection;

/// RCR-033 — read-only enumeration of a shard's committed truth by WAL replay.
///
/// Builds the Query layer's [`ShardProjection`] at the store's current head and
/// returns every committed truth as `(content-id bytes, payload bytes, commit
/// index)` in **deterministic commit order**. This is the seam behind the bridge
/// `scan` verb: it lets a product reconstruct a shard's committed set TOTALLY
/// from the WAL, instead of re-supplying candidate bodies.
///
/// Read-only by construction (OWN-001): the projection is the Query layer's
/// "Read projections/views" surface — it holds the WAL only behind `&`, so no
/// write method (`append`/`install_snapshot`/`compact`) is reachable, and there
/// is no path from here to `Kernel::commit`. Tenant/workspace isolation is
/// structural (SHARD-001): the fold NEVER contains a foreign shard's record.
///
/// Two outcomes are distinguished HONESTLY (adversarial finding): a shard the
/// store has never seen (no durable log) enumerates as an empty `Ok(vec![])` —
/// "nothing committed here", the honest answer for a fresh shard — whereas a
/// shard whose retained log EXISTS but cannot be faithfully replayed (open fault,
/// or a compacted prefix with no query-side snapshot bootstrap, RCR-023 DR-7)
/// returns `Err(ScanFault)`. Collapsing the latter to an empty scan would let
/// truth-loss masquerade as emptiness at the reconstruction surface — a
/// `recoverFromWal` consumer would read "nothing was ever committed" when the
/// real fact is "the log could not be read". The fault is surfaced, never hidden.
pub fn scan_shard<S: WalStore>(
    store: &S,
    shard: &ShardKey,
) -> Result<Vec<(Vec<u8>, Vec<u8>, u64)>, ScanFault> {
    let wshard = PShardKey {
        tenant: shard.tenant().to_string(),
        workspace: shard.workspace().to_string(),
    };
    match ShardProjection::at_head(store, &wshard) {
        Ok(proj) => Ok(proj
            .committed()
            .into_iter()
            .map(|(content, payload, at)| (content.to_vec(), payload.to_vec(), at))
            .collect()),
        // The projection could not be built. Split the two causes HONESTLY:
        //   * a durable log for this shard EXISTS, yet the fold failed (open/replay
        //     fault, or a compacted prefix — RCR-023 DR-7): truth MAY have been
        //     lost. Surface a ScanFault; NEVER report `scan 0`.
        //   * the store holds NO log for this shard: it was legitimately never
        //     committed to — an empty scan is the honest answer for a fresh shard.
        // Either way the write path is untouched (OWN-001): this is read-only.
        Err(_) if store.shards().contains(&wshard) => Err(ScanFault {
            shard: format!("{}/{}", wshard.tenant, wshard.workspace),
        }),
        Err(_) => Ok(Vec::new()),
    }
}

/// A shard's retained log EXISTS but could not be faithfully read by [`scan_shard`]
/// (a durable open/replay fault, or a compacted prefix with no query-side snapshot
/// bootstrap — RCR-023 DR-7). DISTINCT from a never-committed shard, which scans as
/// a legitimate empty result: surfacing the fault keeps a reconstruction consumer
/// from misreading unreadable truth as "nothing was ever committed".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanFault {
    /// The `tenant/workspace` scope whose retained log could not be read.
    pub shard: String,
}

impl std::fmt::Display for ScanFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "scan-fault: retained log for shard '{}' could not be read", self.shard)
    }
}

impl std::error::Error for ScanFault {}

/// RCR-034 — commit `body` as truth ATTRIBUTED to `agent`, the runtime way.
///
/// Wraps `body` in the I5 attribution envelope ([`encode_attributed`], the SAME
/// self-describing envelope the Control-Plane agent flow uses) and commits the
/// envelope as ACS-001-addressed truth through the frozen gateway. The "Who"
/// therefore rides INSIDE the committed payload — in the WAL, the audit log
/// (IDR-005) — never in side metadata that could drift, and it is recoverable
/// by [`arves_control_plane::agents::decode_attributed`] over a `scan`.
///
/// An attributed commit is a DISTINCT truth from a plain commit of the same
/// `body` (the envelope has a different ACS address) — attribution changes
/// identity, honestly. HONEST SCOPE (mirrors the Control-Plane decoders): the
/// `agent` is a CLAIMED identity carried into the trail; this verb does not, by
/// itself, verify the agent is registered committed truth (that structural gate
/// is `propose_attributed`'s) nor that the caller IS the agent (v2.0 debt #8).
pub fn commit_attributed(
    kernel: &impl Kernel,
    shard: ShardKey,
    domain_tag: u8,
    agent: &AgentId,
    body: &[u8],
) -> Result<TruthRef, CommitError> {
    let envelope = encode_attributed(agent, body);
    commit_body(kernel, shard, domain_tag, &envelope)
}

/// Commit a canonical ACS body as truth, addressed by its ACS-001 ContentId.
/// `TruthRef.content` will be `0x12 0x20 || SHA-256(domain_tag || body)` — the exact
/// address any conformant ACS implementation computes for the same body.
pub fn commit_body(
    kernel: &impl Kernel,
    shard: ShardKey,
    domain_tag: u8,
    body: &[u8],
) -> Result<TruthRef, CommitError> {
    let cid = content_id(domain_tag, body);
    kernel.commit(ProposedWrite { shard, content: ContentHash(cid), payload: body.to_vec() })
}

/// Encode an ACS value (ACS-002 dCBOR) and commit it ACS-addressed. A rich value goes
/// in; ACS-identified truth comes out.
pub fn commit_value(
    kernel: &impl Kernel,
    shard: ShardKey,
    domain_tag: u8,
    value: &cbor::Value,
) -> Result<TruthRef, CommitError> {
    commit_body(kernel, shard, domain_tag, &cbor::encode(value))
}

/// The ACS-001 ContentId `commit_body` will use for `(domain_tag, body)`. A caller can
/// predict identity locally and assert the Kernel agrees — the "one world" check.
pub fn address(domain_tag: u8, body: &[u8]) -> Vec<u8> {
    content_id(domain_tag, body)
}

/// One committed proposed-effect: the ACS-addressed truth and whether it was newly
/// committed (`fresh`) or resolved to already-existing truth (ORCH-004 idempotency).
pub struct CommittedEffect {
    pub truth: TruthRef,
    pub fresh: bool,
}

/// The result of running the full cognitive work chain for one invocation.
pub struct InvokeOutcome {
    /// The resolved capability id.
    pub capability: String,
    /// The provider the capability was bound to (CAP-002).
    pub provider: String,
    /// The engine's inference output (opaque).
    pub engine_output: Vec<u8>,
    /// The proposed effects, each committed as ACS-001-addressed truth by the Kernel.
    pub effects: Vec<CommittedEffect>,
}

/// Why the cognitive work chain did not run to a commit.
#[derive(Debug)]
pub enum InvokeError {
    /// The capability had no active binding in the shard — execution is refused
    /// (CAP-005). The Capability layer gates the chain: an unbound capability cannot run.
    Unbound(String),
    /// The resolved binding names a different provider than the engine presented — the
    /// gate binds to engine IDENTITY, not just the capability name, so a caller cannot
    /// smuggle an arbitrary engine past a name-only check (CAP-002).
    ProviderMismatch { expected: String, bound: String },
    /// The engine proposed an effect targeting a resource it did not declare in its
    /// manifest `produces` set (ENG-004) — refused rather than committed.
    UndeclaredEffect(String),
    /// The engine violated the fabric's enforced invocation contract (RCR-012):
    /// a mis-keyed `Inference` (ORCH-004) or a false `Determinism::Deterministic`
    /// declaration caught by the double-invoke probe — refused BEFORE any commit.
    Fabric(FabricViolation),
    /// The Kernel refused to commit a proposed effect for a reason other than idempotency.
    Commit(CommitError),
}

/// Run the REAL cognitive work chain for one capability invocation:
///
/// `Capability (resolve authoritative binding) → Engine (pure invoke) → Kernel (commit
/// each ProposedEffect as ACS-001-addressed truth)`.
///
/// The Capability layer gates execution (an unbound capability is refused, CAP-005); the
/// Engine is pure and only *proposes* effects (ENG-003); those proposals become truth
/// ONLY through the Kernel commit gateway (ORCH-001), addressed by ACS-001 so the truth's
/// identity is the same one the SDK computes. This is the seam the product era needs:
/// SDK → Bridge → Capability → Engine → Kernel, one world.
pub fn invoke<E>(
    kernel: &impl Kernel,
    registry: &impl CapabilityRegistry,
    shard: ShardKey,
    capability: &str,
    engine: &E,
    input: Vec<u8>,
    domain_tag: u8,
) -> Result<InvokeOutcome, InvokeError>
where
    E: Engine<Input = Vec<u8>>,
{
    // 1. Capability layer: resolve the authoritative binding (unbound → refuse).
    // RCR-017: both ShardKeys are opaque with IDENTICAL construction rules (non-empty,
    // ≤256 bytes/part), so converting a validated kernel key is total — the expect is
    // an invariant statement, not a reachable failure.
    let cap_shard = CapShardKey::new(shard.tenant(), shard.workspace())
        .expect("a valid kernel ShardKey always converts to a capability ShardKey");
    let binding = registry
        .resolve(&cap_shard, &CapabilityId(capability.to_string()))
        .map_err(|_| InvokeError::Unbound(capability.to_string()))?;

    // 1b. Bind the gate to engine IDENTITY, not just the capability name: the resolved
    // binding's provider MUST name this exact engine (name@version). A name-only gate
    // would let any engine run under an authorized capability (CAP-002).
    let manifest = engine.manifest();
    let expected_provider = format!("engine:{}@{}", manifest.name, manifest.version);
    if binding.provider.0 != expected_provider {
        return Err(InvokeError::ProviderMismatch { expected: expected_provider, bound: binding.provider.0 });
    }

    // 2. Engine layer: FABRIC-ENFORCED invocation (RCR-012) → Inference (proposals
    // only, ENG-003). The fabric derives the ORCH-004 key itself and verifies the
    // engine's returned key; a self-declared Deterministic engine is double-invoked
    // and compared — a mis-keyed or falsely-deterministic engine is refused here,
    // BEFORE any effect can reach the Kernel.
    let inference = invoke_enforced(engine, input).map_err(InvokeError::Fabric)?;

    // 3. Kernel: commit each proposed effect as ACS-001-addressed truth. An effect MUST
    // target a resource the engine declared (ENG-004); undeclared effects are refused
    // BEFORE any commit, so a single-effect invocation is all-or-nothing. (Cross-effect
    // atomicity for multi-effect, multi-shard invocations needs a Kernel batch-commit
    // primitive — tracked; the reference engine emits one effect, so no partial truth.)
    for effect in &inference.proposed_effects {
        if !manifest.produces.contains(&effect.target) {
            return Err(InvokeError::UndeclaredEffect(effect.target.clone()));
        }
    }
    let mut effects = Vec::new();
    for effect in &inference.proposed_effects {
        match commit_body(kernel, shard.clone(), domain_tag, &effect.payload) {
            Ok(truth) => effects.push(CommittedEffect { truth, fresh: true }),
            Err(CommitError::AlreadyCommitted(truth)) => effects.push(CommittedEffect { truth, fresh: false }),
            Err(e) => return Err(InvokeError::Commit(e)),
        }
    }

    Ok(InvokeOutcome {
        capability: capability.to_string(),
        provider: binding.provider.0,
        engine_output: inference.output,
        effects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arves_acs::cbor::Value::*;
    use arves_acs::{domain, hex};
    use arves_kernel::MemKernel;
    use arves_persistence::MemWalStore;

    fn shard() -> ShardKey {
        ShardKey::new("t1", "w1").expect("valid test shard")
    }

    // The Kernel commits truth under the ACS-001 address (the golden V1 fact ContentId).
    // SDK-computed identity == Kernel-committed identity: one world.
    #[test]
    fn commit_is_acs_addressed() {
        let k = MemKernel::new(MemWalStore::new());
        let fact = Map(vec![
            (Text("type".into()), Text("uci.fact".into())),
            (Text("claim".into()), Text("sky-is-blue".into())),
            (Text("confidence".into()), Float(0.5)),
            (Text("observed_at".into()), Int(1730000000000000000)),
        ]);
        let tr = commit_value(&k, shard(), domain::COMMIT_CONTENT, &fact).expect("commit ok");
        assert_eq!(
            hex(&tr.content.0),
            "12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e",
            "Kernel truth is addressed by the ACS-001 ContentId"
        );
        // And the address is exactly what a caller predicts locally.
        assert_eq!(tr.content.0, address(domain::COMMIT_CONTENT, &cbor::encode(&fact)));
    }

    // The full cognitive work chain: Capability (resolve) -> Engine (invoke) -> Kernel
    // (commit the proposed effect as ACS-addressed truth). Unbound capability is refused;
    // re-invocation is idempotent through the whole chain.
    #[test]
    fn invoke_runs_capability_engine_kernel_chain() {
        use arves_capability_fabric::{
            BindingVersion, CapabilityBinding, CapabilityId, EffectClass, InvocationContract,
            MemRegistry, ProviderId, ShardKey as CapShard,
        };
        use arves_engine_fabric::PureEngine;

        let k = MemKernel::new(MemWalStore::new());
        let mut reg = MemRegistry::new();
        let cshard = CapShard::new("t1", "w1").expect("valid test shard");
        let cap = CapabilityId("derive.fact".into());
        reg.register(&cshard, cap.clone()).unwrap();
        reg.bind(CapabilityBinding {
            capability: cap,
            shard: cshard,
            version: BindingVersion(1),
            provider: ProviderId("engine:derive.fact@1.0.0".into()),
            contract: InvocationContract {
                input_schema: "acs:uci.fact".into(),
                output_schema: "acs:uci.fact".into(),
                effect: EffectClass::ProposesWrite,
            },
        })
        .unwrap();

        // A pure engine that admits its input as a proposed fact.
        let engine = PureEngine::new("derive.fact", "uci.fact", |b: &[u8]| b.to_vec());

        let out = invoke(&k, &reg, shard(), "derive.fact", &engine, b"hello-truth".to_vec(), domain::COMMIT_CONTENT)
            .expect("chain runs");
        assert_eq!(out.provider, "engine:derive.fact@1.0.0");
        assert_eq!(out.effects.len(), 1);
        assert!(out.effects[0].fresh);
        // The proposed effect became ACS-001-addressed truth (the hello-truth golden id).
        assert_eq!(
            hex(&out.effects[0].truth.content.0),
            "122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e"
        );

        // Re-invoke -> same truth, not fresh (ORCH-004 idempotency through the chain).
        let again = invoke(&k, &reg, shard(), "derive.fact", &engine, b"hello-truth".to_vec(), domain::COMMIT_CONTENT).unwrap();
        assert!(!again.effects[0].fresh);

        // An unbound capability is refused — the Capability layer gates execution.
        assert!(matches!(
            invoke(&k, &reg, shard(), "nope", &engine, b"x".to_vec(), domain::COMMIT_CONTENT),
            Err(InvokeError::Unbound(_))
        ));

        // Regression (destroy finding): the gate binds ENGINE IDENTITY, not just the
        // capability name — an impostor engine under an authorized capability is refused.
        let impostor = PureEngine::new("evil.engine", "uci.fact", |b: &[u8]| b.to_vec());
        assert!(matches!(
            invoke(&k, &reg, shard(), "derive.fact", &impostor, b"x".to_vec(), domain::COMMIT_CONTENT),
            Err(InvokeError::ProviderMismatch { .. })
        ));

        // RCR-012: a bound engine whose Determinism declaration is FALSE (per-invocation
        // counter in its output) is refused by the fabric BEFORE any commit — the bridge
        // invokes engines only through invoke_enforced, so the promise is enforced on
        // the real path, not trusted.
        use arves_engine_fabric::{invocation_key, EngineManifest, Inference};
        use std::cell::Cell;
        struct Liar {
            ctr: Cell<u64>,
        }
        impl arves_engine_fabric::Engine for Liar {
            type Input = Vec<u8>;
            fn manifest(&self) -> EngineManifest {
                EngineManifest {
                    name: "derive.fact".into(),
                    version: "1.0.0".into(),
                    produces: vec!["uci.fact".into()],
                    ..Default::default() // Determinism::Deterministic — the false promise
                }
            }
            fn invoke(&self, input: Vec<u8>) -> Inference {
                let n = self.ctr.get();
                self.ctr.set(n + 1);
                let mut payload = input.clone();
                payload.extend_from_slice(&n.to_be_bytes());
                Inference {
                    key: invocation_key(&self.manifest(), &input),
                    output: payload.clone(),
                    proposed_effects: vec![arves_engine_fabric::ProposedEffect { target: "uci.fact".into(), payload }],
                }
            }
        }
        let before = k.snapshot_shard(&shard()).len();
        assert!(matches!(
            invoke(&k, &reg, shard(), "derive.fact", &Liar { ctr: Cell::new(0) }, b"x".to_vec(), domain::COMMIT_CONTENT),
            Err(InvokeError::Fabric(arves_engine_fabric::FabricViolation::NondeterministicOutput))
        ));
        // Nothing reached the Kernel: refusal happened BEFORE any commit.
        assert_eq!(k.snapshot_shard(&shard()).len(), before);
    }

    // RCR-033: scan_shard enumerates exactly the shard's committed set, in commit
    // order, read-only, and is tenant-isolated (shard A never sees B).
    #[test]
    fn scan_shard_enumerates_committed_set_tenant_isolated() {
        use arves_acs::hex;
        let store = MemWalStore::new();
        let k = MemKernel::new(store.clone());
        let a = ShardKey::new("t1", "w1").unwrap();
        let b = ShardKey::new("t2", "w2").unwrap();
        // A fresh shard has nothing committed: the honest answer is empty (Ok).
        assert!(scan_shard(&store, &a).unwrap().is_empty());
        // Commit three truths in A and one (same body as one of A's) in B.
        let a0 = commit_body(&k, a.clone(), domain::COMMIT_CONTENT, b"alpha").unwrap();
        let a1 = commit_body(&k, a.clone(), domain::COMMIT_CONTENT, b"beta").unwrap();
        let a2 = commit_body(&k, a.clone(), domain::COMMIT_CONTENT, b"gamma").unwrap();
        let _b0 = commit_body(&k, b.clone(), domain::COMMIT_CONTENT, b"alpha").unwrap();

        let scan_a = scan_shard(&store, &a).unwrap();
        // Exactly A's three truths, in commit order, with A's ContentIds + payloads.
        assert_eq!(scan_a.len(), 3);
        assert_eq!(
            scan_a.iter().map(|(c, p, i)| (hex(c), p.clone(), *i)).collect::<Vec<_>>(),
            vec![
                (hex(&a0.content.0), b"alpha".to_vec(), 0),
                (hex(&a1.content.0), b"beta".to_vec(), 1),
                (hex(&a2.content.0), b"gamma".to_vec(), 2),
            ]
        );
        // Tenant isolation: B's scan holds ONLY B's single truth (same body,
        // fresh index 0 in its own log — SHARD-001), never A's records.
        let scan_b = scan_shard(&store, &b).unwrap();
        assert_eq!(scan_b.len(), 1);
        assert_eq!(scan_b[0].1, b"alpha".to_vec());
        assert_eq!(scan_b[0].2, 0);
        // Determinism: a second scan over the same committed prefix is identical.
        assert_eq!(scan_shard(&store, &a).unwrap(), scan_a);
    }

    // Adversarial finding: an unreadable retained log must NOT collapse to `scan 0`.
    // A never-committed shard scans as a legitimate empty Ok; a shard whose durable
    // log EXISTS but cannot be replayed (compacted prefix, no query-side snapshot —
    // RCR-023 DR-7) surfaces a ScanFault, so truth-loss never masquerades as emptiness.
    #[test]
    fn scan_shard_distinguishes_fresh_shard_from_unreadable_log() {
        use arves_persistence::Wal;
        let store = MemWalStore::new();
        let k = MemKernel::new(store.clone());
        let sh = ShardKey::new("t1", "w1").unwrap();

        // Never committed to -> a legitimate empty scan, NOT a fault.
        assert_eq!(scan_shard(&store, &sh), Ok(Vec::new()));

        // Commit two truths, then compact the log's prefix away with no query-side
        // snapshot bootstrap: the retained log can no longer reproduce the head fold.
        commit_body(&k, sh.clone(), domain::COMMIT_CONTENT, b"alpha").unwrap();
        commit_body(&k, sh.clone(), domain::COMMIT_CONTENT, b"beta").unwrap();
        let pshard = PShardKey { tenant: "t1".into(), workspace: "w1".into() };
        let mut wal = store.open(&pshard).unwrap();
        wal.compact(0).unwrap(); // drop offset 0; earliest advances above the fold's start

        // The store STILL lists the shard (a durable log exists) but the fold fails:
        // the honest answer is a ScanFault, never `scan 0`.
        assert!(store.shards().contains(&pshard));
        assert_eq!(
            scan_shard(&store, &sh),
            Err(ScanFault { shard: "t1/w1".to_string() })
        );
    }

    // RCR-034: an attributed commit records the Who INSIDE committed truth; a
    // scan recovers (who, what) via the runtime decoder. It is a DISTINCT truth
    // from a plain commit of the same body, and plain commits still work.
    #[test]
    fn attributed_commit_round_trips_and_is_queryable() {
        use arves_control_plane::agents::{decode_attributed, AgentId};
        let store = MemWalStore::new();
        let k = MemKernel::new(store.clone());
        let sh = ShardKey::new("t1", "w1").unwrap();
        let agent = AgentId::from_hex("1220").expect("valid hex id");

        let attributed = commit_attributed(&k, sh.clone(), domain::COMMIT_CONTENT, &agent, b"did-the-thing").unwrap();
        let plain = commit_body(&k, sh.clone(), domain::COMMIT_CONTENT, b"did-the-thing").unwrap();
        // Attribution changes identity: the two truths are distinct.
        assert_ne!(attributed.content.0, plain.content.0);

        // The attribution is queryable from a scan: the enveloped payload decodes
        // back to exactly (agent, effect).
        let scanned = scan_shard(&store, &sh).unwrap();
        let env = scanned
            .iter()
            .find(|(c, _, _)| *c == attributed.content.0)
            .map(|(_, p, _)| p.clone())
            .expect("attributed truth is in the scan");
        assert_eq!(decode_attributed(&env), Some((agent, b"did-the-thing".to_vec())));
        // The plain truth is NOT an attribution envelope (backward compatible).
        let plain_body = scanned.iter().find(|(c, _, _)| *c == plain.content.0).map(|(_, p, _)| p.clone()).unwrap();
        assert_eq!(decode_attributed(&plain_body), None);
    }

    // ORCH-004 idempotency is now keyed on the ACS address: same body -> AlreadyCommitted.
    #[test]
    fn commit_is_idempotent_on_acs_address() {
        let k = MemKernel::new(MemWalStore::new());
        let body = b"hello-truth";
        let first = commit_body(&k, shard(), domain::COMMIT_CONTENT, body).expect("first ok");
        match commit_body(&k, shard(), domain::COMMIT_CONTENT, body) {
            Err(CommitError::AlreadyCommitted(existing)) => assert_eq!(existing, first),
            other => panic!("expected AlreadyCommitted, got {other:?}"),
        }
    }
}

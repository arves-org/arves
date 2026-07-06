//! RCR-029 (I5 Stage 1) — AGENT IDENTITY as content-addressed truth +
//! ATTRIBUTION of agent-proposed effects into the committed truth trail.
//!
//! Design basis: `docs/design/I5_MultiAgent_Runtime_Design.md` — §3.1.1
//! ("every agent is a governed, tenant-scoped identity with a versioned
//! definition; spawning an agent = registering + activating a definition"),
//! §3.10 ("agent registry/governance state is committed truth, so it recovers
//! with the truth base — an agent's identity never depends on orchestrator
//! memory"; OQ-2 leaned truth-side, resolved here as RCR-029 DR-6), §3.19
//! (Who/When/What/Why audit — the agent identity is the Who, carried INTO the
//! committed record), §1 table (agent identity = Tenant/Identity model, Vol 2
//! Parts 4/8; the Control Plane REGISTERS and ATTRIBUTES, it never OWNS the
//! identity truth — ORCH-001).
//!
//! # What an agent IS here — honest language (design §3.16 / NON-GOAL 4)
//!
//! An agent identity is an **addressable registration the runtime can
//! attribute actions to**: a versioned agent definition (ARVES-23 template
//! subset: Name/Type/Owner/Purpose), canonically encoded, committed as
//! content-addressed truth through the one frozen `Kernel::commit` gateway of
//! its shard. The agents exercised by the tests are **deterministic test
//! actors, NOT AI models**.
//!
//! **HONEST LIMIT (load-bearing, v2.0 debt #8 / design OQ-1):** this is NOT
//! cryptographic authentication. Runtime v1.x has no principal/authN/authZ on
//! `Kernel::commit`; any in-process caller can wear any REGISTERED agent
//! identity (pinned by test, not hidden). What IS enforced, structurally:
//! - an identity must exist as committed truth in the TARGET shard before an
//!   effect can be attributed to it ([`propose_attributed`] refuses otherwise);
//! - the attribution travels INSIDE the committed payload, so the WAL — the
//!   audit log (IDR-005) — carries Who for every attributed effect, on every
//!   replica, tamper-evident per the store's hash chain (RCR-002);
//! - identity is content-addressed (ORCH-004): registration is idempotent,
//!   and the id can never denote two different definitions (RCR-005).
//!
//! # Sharding (SHARD-001)
//!
//! The registration shard's `(tenant, workspace)` is part of the identity's
//! canonical body, so an [`AgentId`] is **shard-bound for life** (design §4
//! SHARD-001 row): the same definition registered in two shards yields two
//! DISTINCT identities, and an identity is addressable only through its own
//! shard's world view. No cross-tenant identity exists.
//!
//! # Layering (LAYER-001)
//!
//! Reads of committed truth go through the LCW shared-truth surface
//! (`arves_lcw::world::WorldView`) — a DOWNWARD edge (control-plane 90 →
//! lcw 50; ranks checked in `arves-conformance/src/property_check.rs` BEFORE
//! adding). Writes go only through the frozen `Kernel` trait (90 → 40).
//! Nothing here holds registry state of its own: drop every value in this
//! module and the identities survive in the truth base (ORCH-002 posture,
//! same as the RCR-027 scheduler).
//!
//! # Encoding honesty
//!
//! The canonical definition/attribution encodings below are RUNTIME-INTERNAL
//! reference encodings (length-prefixed little-endian, the house codec
//! discipline of the Kernel snapshot blob / RCR-021 outcome codec). They are
//! NOT registered `uci.*` ontology types — that registration is the design
//! §6.2 CCP instrument (O-006) and is NOT claimed here.

use arves_kernel::{
    CommitError, ContentHash, Kernel, ProposedWrite, ShardKey as KernelShardKey, ShardKeyError,
    TruthRef,
};
use arves_lcw::world::WorldView;
use core::fmt;

// ---------------------------------------------------------------------------
// Canonical codec (house discipline: u32-LE length-prefixed parts, magic'd)
// ---------------------------------------------------------------------------

/// Self-describing prefix of a canonical agent-definition body (versioned;
/// runtime-internal reference encoding — see module doc "Encoding honesty").
const AGENT_DEF_MAGIC: &[u8] = b"ARVES.AGENT.DEF.v1";
/// Self-describing prefix of an attribution envelope.
const ATTR_MAGIC: &[u8] = b"ARVES.AGENT.ATTR.v1";

// Shared with the RCR-030 `multi_agent` flow codecs (same house discipline,
// same crate — crate-internal, never a public API surface).
pub(crate) fn put_part(buf: &mut Vec<u8>, part: &[u8]) {
    buf.extend_from_slice(&(part.len() as u32).to_le_bytes());
    buf.extend_from_slice(part);
}

pub(crate) fn take<'a>(b: &'a [u8], pos: &mut usize, n: usize) -> Option<&'a [u8]> {
    let end = pos.checked_add(n)?;
    let s = b.get(*pos..end)?;
    *pos = end;
    Some(s)
}

pub(crate) fn take_part<'a>(b: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
    let len = u32::from_le_bytes(take(b, pos, 4)?.try_into().ok()?) as usize;
    take(b, pos, len)
}

pub(crate) fn take_string(b: &[u8], pos: &mut usize) -> Option<String> {
    String::from_utf8(take_part(b, pos)?.to_vec()).ok()
}

// ---------------------------------------------------------------------------
// AgentDefinition — the versioned registry record (ARVES-23 template subset)
// ---------------------------------------------------------------------------

/// A versioned agent definition — the ARVES-23 template subset Stage 1 needs
/// (Name/Type/Owner/Purpose + version; Capabilities/Goals/Policies/… bind in
/// later I5 stages through their own frozen owners, never here — OWN-001).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentDefinition {
    /// Agent name (non-empty). ARVES-23 "Name".
    pub name: String,
    /// Agent role/type — e.g. Coordinator / Worker / Specialist / Supervisor
    /// (Vol 14 Part 13). Non-empty; free-form at this stage (the canonical
    /// agent-type registry is ARVES-23's; enforcing its closed set is CCP
    /// surface, not runtime surface).
    pub agent_type: String,
    /// MANDATORY owner (Vol 2 Part 17: "Every … agent must have a defined
    /// owner"). Non-empty.
    pub owner: String,
    /// Purpose statement. ARVES-23 "Purpose"; may be empty.
    pub purpose: String,
    /// Definition version (Vol 14 Part 20: agent definitions are versioned).
    /// A new version is a NEW identity — identities are immutable truths.
    pub definition_version: u32,
}

impl AgentDefinition {
    /// Validate the definition's mandatory fields (governance minima).
    fn validate(&self) -> Result<(), AgentError> {
        if self.name.is_empty() {
            return Err(AgentError::InvalidDefinition("agent name must be non-empty".into()));
        }
        if self.agent_type.is_empty() {
            return Err(AgentError::InvalidDefinition("agent type must be non-empty".into()));
        }
        if self.owner.is_empty() {
            // Vol 2 Part 17 / Part 23: no ungoverned (owner-less) agents.
            return Err(AgentError::InvalidDefinition(
                "agent owner is mandatory (Vol 2 Part 17)".into(),
            ));
        }
        Ok(())
    }

    /// Canonical body of this definition AS REGISTERED IN `shard`: the shard's
    /// `(tenant, workspace)` is part of the identity (SHARD-001: shard-bound
    /// for life), then the template fields in fixed order, then the version.
    /// Deterministic byte-for-byte; an independent runtime can reproduce it
    /// from this doc alone.
    pub fn canonical_bytes(&self, shard: &KernelShardKey) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(AGENT_DEF_MAGIC);
        put_part(&mut b, shard.tenant().as_bytes());
        put_part(&mut b, shard.workspace().as_bytes());
        put_part(&mut b, self.name.as_bytes());
        put_part(&mut b, self.agent_type.as_bytes());
        put_part(&mut b, self.owner.as_bytes());
        put_part(&mut b, self.purpose.as_bytes());
        b.extend_from_slice(&self.definition_version.to_le_bytes());
        b
    }
}

/// Decode a committed registry payload back into `(tenant, workspace,
/// definition)`. `None` if the payload is not a canonical agent definition
/// (wrong magic, malformed parts, trailing garbage).
pub fn decode_definition(payload: &[u8]) -> Option<(String, String, AgentDefinition)> {
    let rest = payload.strip_prefix(AGENT_DEF_MAGIC)?;
    let mut pos = 0usize;
    let tenant = take_string(rest, &mut pos)?;
    let workspace = take_string(rest, &mut pos)?;
    let name = take_string(rest, &mut pos)?;
    let agent_type = take_string(rest, &mut pos)?;
    let owner = take_string(rest, &mut pos)?;
    let purpose = take_string(rest, &mut pos)?;
    let version = u32::from_le_bytes(take(rest, &mut pos, 4)?.try_into().ok()?);
    if pos != rest.len() {
        return None; // trailing garbage refused (house codec discipline)
    }
    Some((tenant, workspace, AgentDefinition {
        name,
        agent_type,
        owner,
        purpose,
        definition_version: version,
    }))
}

// ---------------------------------------------------------------------------
// AgentId — content-addressed identity (ACS-001; ORCH-004)
// ---------------------------------------------------------------------------

/// A content-addressed agent identity: the ACS-001 multihash of the canonical
/// definition body under the COMMIT_CONTENT domain tag (the identity IS the
/// content address of its registration truth — no new domain tag is minted;
/// the ACS-001 tag registry is frozen `standard/` surface, RCR-029 DR-7).
///
/// Properties inherited from content addressing:
/// - deterministic: same shard + same definition ⇒ same id, on any runtime;
/// - collision-honest: the id can never denote two definitions (RCR-005
///   content-integrity at the gateway);
/// - shard-bound: the shard is inside the hashed body (SHARD-001).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgentId(Vec<u8>);

impl AgentId {
    /// Crate-internal: rebuild an id from raw multihash bytes (used by the
    /// RCR-030 flow decoders; a decoded Who is a CLAIMED identity until
    /// cross-checked with [`is_registered`] — RCR-029 amendment A2 honesty).
    pub(crate) fn from_raw(bytes: Vec<u8>) -> AgentId {
        AgentId(bytes)
    }

    /// The identity of `def` as registered in `shard`.
    pub fn of(shard: &KernelShardKey, def: &AgentDefinition) -> AgentId {
        AgentId(arves_acs::content_id(
            arves_acs::domain::COMMIT_CONTENT,
            &def.canonical_bytes(shard),
        ))
    }

    /// Raw ACS-001 multihash bytes (34 bytes).
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }

    /// Lowercase-hex text form — the addressable key under which the
    /// registration truth appears in the LCW world view.
    pub fn hex(&self) -> String {
        arves_acs::hex(&self.0)
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hex())
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why an identity operation did not produce/attribute truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentError {
    /// The definition failed its governance minima (empty name/type/owner).
    InvalidDefinition(String),
    /// The agent is not registered committed truth in the target shard's
    /// world view — attribution refused (the structural gate; module doc).
    NotRegistered {
        /// Hex id of the unregistered agent.
        agent: String,
    },
    /// The world view's shard could not name a well-formed kernel shard.
    BadShard(ShardKeyError),
    /// The frozen commit gateway refused the write (leadership, quorum,
    /// content-integrity, …) — surfaced verbatim, never swallowed.
    Commit(CommitError),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::InvalidDefinition(d) => write!(f, "invalid agent definition: {d}"),
            AgentError::NotRegistered { agent } => {
                write!(f, "agent {agent} is not registered truth in the target shard")
            }
            AgentError::BadShard(e) => write!(f, "world view names a malformed shard: {e}"),
            AgentError::Commit(e) => write!(f, "commit gateway refused: {e}"),
        }
    }
}

impl std::error::Error for AgentError {}

// ---------------------------------------------------------------------------
// Registration (identity = committed truth; RCR-029 DR-6 resolves OQ-2
// truth-side, the design's own lean)
// ---------------------------------------------------------------------------

/// Outcome of [`register_agent`]: the identity, its registration truth, and
/// whether this call created it (`fresh`) or resolved idempotently to the
/// existing registration (ORCH-004).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentRegistration {
    /// The content-addressed identity.
    pub id: AgentId,
    /// The committed registration truth (shard + content + commit index).
    pub truth: TruthRef,
    /// `true` iff this call created the registration truth.
    pub fresh: bool,
}

/// Register `def` as a governed agent identity in `shard`: commit its
/// canonical body as content-addressed truth through the frozen gateway.
/// Idempotent — re-registering the identical definition resolves to the SAME
/// truth (`fresh: false`), never a fork (ORCH-004; RCR-005).
///
/// The registrar (this Control-Plane module) owns NOTHING afterwards: the
/// identity lives in the truth base and recovers with it (design §3.10).
pub fn register_agent<K: Kernel>(
    kernel: &K,
    shard: &KernelShardKey,
    def: &AgentDefinition,
) -> Result<AgentRegistration, AgentError> {
    def.validate()?;
    let payload = def.canonical_bytes(shard);
    let id = AgentId::of(shard, def);
    let proposed = ProposedWrite {
        shard: shard.clone(),
        content: ContentHash(id.bytes().to_vec()),
        payload,
    };
    match kernel.commit(proposed) {
        Ok(truth) => Ok(AgentRegistration { id, truth, fresh: true }),
        Err(CommitError::AlreadyCommitted(truth)) => {
            Ok(AgentRegistration { id, truth, fresh: false })
        }
        Err(e) => Err(AgentError::Commit(e)),
    }
}

/// Whether `id` is a registered agent identity in the committed truth `world`
/// reflects: the truth must exist under the id, decode as a canonical
/// definition whose DECODED SHARD IS THIS WORLD'S SHARD, and recompute to `id`
/// (defense in depth over the content-address guarantee; a payload can never
/// lie about its id).
///
/// The shard-equality check is load-bearing (SHARD-001; RCR-029 amendment A1):
/// the frozen `Kernel::commit` gateway does not verify `content ==
/// ACS-hash(payload)` (RCR-005 admission only rejects same-address/
/// different-payload forks), so a caller CAN lawfully commit a shard-B-bodied
/// definition into shard A's WAL. Without this check such a smuggled record
/// would make a shard-B-bound identity pass shard A's gate. With it, an
/// identity is addressable ONLY through its own shard's world — pinned by
/// `smuggled_foreign_shard_definition_is_refused_shard001`.
pub fn is_registered(world: &WorldView, id: &AgentId) -> bool {
    match world.get(&id.hex()) {
        Some((payload, _at)) => match decode_definition(payload) {
            Some((tenant, workspace, def)) => match KernelShardKey::new(tenant, workspace) {
                Ok(shard) => {
                    shard.tenant() == world.shard().tenant
                        && shard.workspace() == world.shard().workspace
                        && AgentId::of(&shard, &def) == *id
                }
                Err(_) => false,
            },
            None => false,
        },
        None => false,
    }
}

/// Read a registered definition back from the shared world (addressability).
pub fn find_agent(world: &WorldView, id: &AgentId) -> Option<AgentDefinition> {
    if !is_registered(world, id) {
        return None;
    }
    world.get(&id.hex()).and_then(|(payload, _)| decode_definition(payload)).map(|(_, _, d)| d)
}

// ---------------------------------------------------------------------------
// Attribution (the Who inside the committed What — design §3.19)
// ---------------------------------------------------------------------------

/// Wrap `effect` in the attribution envelope carrying `agent`'s identity.
/// The envelope IS the committed payload, so the identity rides inside the
/// truth trail (WAL) itself — never in side metadata that could drift.
pub fn encode_attributed(agent: &AgentId, effect: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(ATTR_MAGIC);
    put_part(&mut b, agent.bytes());
    put_part(&mut b, effect);
    b
}

/// Decode a committed payload as an attributed effect: `(who, what)`.
/// `None` if the payload is not an attribution envelope.
pub fn decode_attributed(payload: &[u8]) -> Option<(AgentId, Vec<u8>)> {
    let rest = payload.strip_prefix(ATTR_MAGIC)?;
    let mut pos = 0usize;
    let agent = take_part(rest, &mut pos)?.to_vec();
    let effect = take_part(rest, &mut pos)?.to_vec();
    if pos != rest.len() {
        return None; // trailing garbage refused
    }
    Some((AgentId(agent), effect))
}

/// Outcome of [`propose_attributed`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributedCommit {
    /// The identity the effect is attributed to (the audit "Who").
    pub agent: AgentId,
    /// The committed truth carrying the attribution envelope (the "What",
    /// with "When" = its commit index in the shard trace).
    pub truth: TruthRef,
    /// `true` iff this call created the truth; `false` = idempotent resolve
    /// of a duplicate proposal (ORCH-004 convergence).
    pub fresh: bool,
}

/// Propose `effect` as truth ATTRIBUTED to `agent`, into the shard whose
/// committed truth `world` reflects.
///
/// Structural gate (what v1.x CAN enforce, module doc): the agent must be
/// registered committed truth in that world — checking against COMMITTED
/// truth, not orchestrator memory, is exactly the runtime-grade check the
/// design elevates from the G1 product (whose maps were in-process only).
/// The `world`'s version is the caller's declared read basis; a staler view
/// can only make the gate MORE restrictive (an identity, once committed, is
/// immutable truth and never un-registers in v1.x — no revocation exists yet,
/// honest limit).
///
/// NOT enforced (v2.0 debt #8, said out loud): that the CALLER is the agent.
pub fn propose_attributed<K: Kernel>(
    kernel: &K,
    world: &WorldView,
    agent: &AgentId,
    effect: &[u8],
) -> Result<AttributedCommit, AgentError> {
    if !is_registered(world, agent) {
        return Err(AgentError::NotRegistered { agent: agent.hex() });
    }
    let shard = KernelShardKey::new(
        world.shard().tenant.clone(),
        world.shard().workspace.clone(),
    )
    .map_err(AgentError::BadShard)?;
    let envelope = encode_attributed(agent, effect);
    let content = arves_acs::content_id(arves_acs::domain::COMMIT_CONTENT, &envelope);
    let proposed = ProposedWrite {
        shard,
        content: ContentHash(content),
        payload: envelope,
    };
    match kernel.commit(proposed) {
        Ok(truth) => Ok(AttributedCommit { agent: agent.clone(), truth, fresh: true }),
        Err(CommitError::AlreadyCommitted(truth)) => {
            Ok(AttributedCommit { agent: agent.clone(), truth, fresh: false })
        }
        Err(e) => Err(AgentError::Commit(e)),
    }
}

/// Read the attribution of one committed truth back from the shared world:
/// `(who, what)` for the truth at content id `truth_id_hex`, or `None` if it
/// is not an attributed effect.
///
/// **HONEST LIMIT (RCR-029 amendment A2):** this reader reports the CLAIMED
/// Who as decoded from the envelope — it does NOT cross-check
/// [`is_registered`] and does not validate the id's 34-byte ACS-001 shape.
/// The registered-identity guarantee holds only for envelopes written via
/// [`propose_attributed`]; a hand-crafted envelope committed directly through
/// the raw frozen gateway can carry an arbitrary Who. An audit consumer that
/// needs registered-Who assurance must re-check `is_registered(world, &who)`.
pub fn attribution_of(world: &WorldView, truth_id_hex: &str) -> Option<(AgentId, Vec<u8>)> {
    world.get(truth_id_hex).and_then(|(payload, _)| decode_attributed(payload))
}

/// Enumerate the whole attributed truth trail visible in `world`, in commit
/// order: `(who, what, committed_at)` — the audit walk of design §3.19.
///
/// Same honest limit as [`attribution_of`] (RCR-029 amendment A2): each row's
/// Who is the CLAIMED identity in the envelope, not a verified registration.
pub fn attributed_effects(world: &WorldView) -> Vec<(AgentId, Vec<u8>, u64)> {
    let mut rows: Vec<(AgentId, Vec<u8>, u64)> = world
        .iter()
        .filter_map(|(_, payload, at)| decode_attributed(payload).map(|(who, what)| (who, what, at)))
        .collect();
    rows.sort_by_key(|(_, _, at)| *at);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> AgentDefinition {
        AgentDefinition {
            name: "ledger-worker".into(),
            agent_type: "Worker".into(),
            owner: "ops@acme".into(),
            purpose: "test actor".into(),
            definition_version: 1,
        }
    }

    #[test]
    fn definition_codec_round_trips_and_rejects_garbage() {
        let shard = KernelShardKey::new("acme", "research").unwrap();
        let bytes = def().canonical_bytes(&shard);
        assert_eq!(
            decode_definition(&bytes),
            Some(("acme".into(), "research".into(), def()))
        );
        let mut garbage = bytes.clone();
        garbage.push(0);
        assert_eq!(decode_definition(&garbage), None, "trailing garbage refused");
        assert_eq!(decode_definition(b"not-a-definition"), None);
    }

    #[test]
    fn attribution_codec_round_trips_and_rejects_garbage() {
        let shard = KernelShardKey::new("acme", "research").unwrap();
        let id = AgentId::of(&shard, &def());
        let env = encode_attributed(&id, b"effect-bytes");
        assert_eq!(decode_attributed(&env), Some((id, b"effect-bytes".to_vec())));
        let mut garbage = env.clone();
        garbage.push(0);
        assert_eq!(decode_attributed(&garbage), None, "trailing garbage refused");
        assert_eq!(decode_attributed(b"ARVES.AGENT.DEF.v1junk"), None);
    }

    #[test]
    fn identity_is_deterministic_shard_bound_and_version_sensitive() {
        let a = KernelShardKey::new("acme", "research").unwrap();
        let b = KernelShardKey::new("globex", "research").unwrap();
        // Deterministic: same shard + same definition => same id.
        assert_eq!(AgentId::of(&a, &def()), AgentId::of(&a, &def()));
        // Shard-bound (SHARD-001): same definition, different shard => different id.
        assert_ne!(AgentId::of(&a, &def()), AgentId::of(&b, &def()));
        // Versioned (Vol 14 Part 20): a new definition version is a NEW identity.
        let mut v2 = def();
        v2.definition_version = 2;
        assert_ne!(AgentId::of(&a, &def()), AgentId::of(&a, &v2));
    }

    #[test]
    fn governance_minima_are_enforced_before_any_commit() {
        let mut ownerless = def();
        ownerless.owner.clear();
        assert!(ownerless.validate().is_err(), "owner is mandatory (Vol 2 Part 17)");
        let mut nameless = def();
        nameless.name.clear();
        assert!(nameless.validate().is_err());
        let mut typeless = def();
        typeless.agent_type.clear();
        assert!(typeless.validate().is_err());
    }
}

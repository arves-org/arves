//! RCR-030 (I5 Stage 2) — MULTI-AGENT ORCHESTRATION over ONE shared truth
//! base: scheduler-borne agent proposals + the decision/compliance truth flow
//! with deterministic first-committed-wins conflict resolution.
//!
//! Design basis: `docs/design/I5_MultiAgent_Runtime_Design.md` — §3.1.2
//! ("N agents concurrently propose writes; exactly one content-addressed truth
//! base per shard results (ORCH-004 + IDR-001). Duplicate proposals converge;
//! conflicting proposals are detected against committed truth and recorded as
//! compliance events, not silently overwritten"), §3.1.3 (decision/compliance
//! truth flows: "policy checks read *committed* policy truths and approvals
//! are *separate committed truths* (proposer ≠ approver)"), §3.8 (the
//! concurrency model rows 1/2/4/5), §3.19 (auditability: blocked decisions and
//! conflicts are *recorded as truths*, not just refused). The exercised G1
//! reference semantics are `products/arves-enterprise-os/src/enterprise-os.mjs`
//! `proposeDecision`/`checkPolicy`/`approve` — whose checks ran over IN-MEMORY
//! per-process maps (the product's own header caveat); THIS module is the
//! runtime-grade elevation: every check reads COMMITTED truth through the LCW
//! [`WorldView`], and every refusal that matters is itself committed truth.
//!
//! # HONEST LANGUAGE (load-bearing)
//!
//! The "agents" orchestrated here are **deterministic test actors, NOT AI
//! models**: registered agent identities (RCR-029) on whose behalf callers
//! propose effects and decisions. Nothing here is enforced cryptographically —
//! any in-process caller can wear any REGISTERED identity (v2.0 debt #8 /
//! design OQ-1, pinned by RCR-029 test). What IS enforced, structurally:
//!
//! - **Agents never commit** (ORCH-001): every path to truth in this module is
//!   the frozen `Kernel::commit` gateway — either directly (decision flow,
//!   `kernel` is caller-supplied and in the cluster tests is the shard
//!   leader's `ClusterKernel`) or via the I4 scheduler's commit routing
//!   ([`submit_attributed_effect`]). Refusals commit nothing; this module owns
//!   no truth and holds no state at all (every function is free-standing over
//!   caller-owned values — ORCH-002 posture, drop everything and truth stays).
//! - **The gate reads committed truth** at the caller's DECLARED basis (a
//!   versioned [`WorldView`]) — the runtime-grade elevation of G1's in-memory
//!   maps (RCR-029 DR-9 pattern).
//! - **One truth per subject** (ORCH-004 across agents): duplicate content
//!   converges at the Kernel; agreeing decisions converge at the flow
//!   ([`ProposalOutcome::Converged`]); CONFLICTING decisions on one subject
//!   resolve deterministically by **first-committed-wins in shard log order**
//!   (total per shard — IDR-001/IDR-005), with the loser receiving the
//!   winner's identity and the conflict recorded as committed compliance truth
//!   ([`ProposalOutcome::Conflict`]) — the enterprise-os reference semantics
//!   at runtime level.
//!
//! # The serialization-point rule (design §3.8(5), OQ-3 scoped honestly)
//!
//! Two agents may race a check-then-commit: both read a basis with no decision
//! on subject S, both commit. The frozen Kernel MUST NOT decide (Layer Matrix
//! Kernel "Cannot: orchestrate, plan or execute"; "The Kernel never becomes
//! the Control Plane", Vol 9 Part 2), so no kernel-side gate is added. Instead
//! the CONTROL PLANE resolves the race deterministically AFTER the fact:
//!
//! 1. The derivation rule [`decision_of`] defines the subject's decision truth
//!    as the FIRST decision record on that subject in shard log order — a pure
//!    fold of the committed trace, identical on every replica (ORCH-003).
//!    A later conflicting record can exist in the WAL (it is an honest,
//!    replayable trace of what was attempted — design §3.19) but it NEVER
//!    derives as the subject's decision: no silent overwrite is possible.
//! 2. [`propose_decision`] reconciles at the serialization point: after its
//!    commit it re-reads the world AT HEAD and, if its own record is not the
//!    first on the subject, reports the loss loudly — the loser receives the
//!    winner's identity and a compliance event citing the winner is committed.
//!
//! The full OQ-3 instrument (a leader-side admission stage that refuses the
//! losing record BEFORE it enters the log) remains a later-stage IDR
//! obligation and is NOT claimed here. Honest consequence, pinned by test: the
//! POLICY gate reads the declared basis only — a policy committed between the
//! caller's basis and its commit does not retro-block in Stage 2 (recorded
//! debt, same OQ-3 class).
//!
//! # Encoding honesty
//!
//! The policy/approval/decision/compliance encodings are RUNTIME-INTERNAL
//! reference encodings (length-prefixed little-endian, the RCR-029 house
//! codec), hashed under the existing `COMMIT_CONTENT` ACS-001 domain tag
//! (RCR-029 DR-7). They are NOT registered `uci.*` ontology types — that
//! registration is the design §6.2 O-006 CCP instrument, not pre-empted here.

use crate::agents::{
    encode_attributed, is_registered, put_part, take, take_part, take_string, AgentId,
};
use crate::scheduler::{ClusterScheduler, DispatchEnv, InvocationSpec, SubmitOutcome};
use arves_capability_fabric::gate::PolicyVerdict;
use arves_capability_fabric::{CapabilityId, CapabilityRegistry};
use arves_consensus::{ShardId, TenantId, WorkspaceId};
use arves_kernel::{
    CommitError, ContentHash, Kernel, ProposedWrite, ShardKey as KernelShardKey, ShardKeyError,
    TruthRef,
};
use arves_lcw::world::{WorldError, WorldView};
use arves_lcw::ShardKey as LcwShardKey;
use arves_persistence::WalStore;
use core::fmt;

// ---------------------------------------------------------------------------
// Canonical flow codecs (house discipline; runtime-internal — module doc)
// ---------------------------------------------------------------------------

/// Self-describing prefix of a committed policy truth.
const POLICY_MAGIC: &[u8] = b"ARVES.FLOW.POLICY.v1";
/// Self-describing prefix of a committed approval truth.
const APPROVAL_MAGIC: &[u8] = b"ARVES.FLOW.APPROVAL.v1";
/// Self-describing prefix of a committed agent-decision truth.
const DECISION_MAGIC: &[u8] = b"ARVES.FLOW.DECISION.v1";
/// Self-describing prefix of a committed compliance-event truth.
const COMPLIANCE_MAGIC: &[u8] = b"ARVES.FLOW.COMPLIANCE.v1";

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a flow operation did not produce/derive truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlowError {
    /// The acting agent is not registered committed truth in the declared
    /// basis world (the structural gate — RCR-029; refusals commit nothing).
    NotRegistered {
        /// Hex id of the unregistered agent.
        agent: String,
    },
    /// A record failed its validation minima (empty subject/action/name/scope).
    InvalidRecord(String),
    /// The world's shard could not name a well-formed kernel shard.
    BadShard(ShardKeyError),
    /// The reconciliation world could not be built (read fault, surfaced).
    World(WorldError),
    /// The reconciliation store has NOT applied this flow's own just-acked
    /// commit — it is behind the truth it must arbitrate over. Refused loudly
    /// (lossless-or-loud): reconcile against a store that has applied the
    /// commit (the leader's, or any caught-up replica), then re-propose — the
    /// re-proposal converges idempotently, never forks (ORCH-004).
    ReconcileBehind {
        /// Hex content id of the committed-but-unreconciled decision record.
        decision: String,
    },
    /// The frozen commit gateway refused — surfaced verbatim, never swallowed.
    Commit(CommitError),
}

impl fmt::Display for FlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlowError::NotRegistered { agent } => {
                write!(f, "agent {agent} is not registered truth in the declared basis")
            }
            FlowError::InvalidRecord(d) => write!(f, "invalid flow record: {d}"),
            FlowError::BadShard(e) => write!(f, "world names a malformed shard: {e}"),
            FlowError::World(e) => write!(f, "reconciliation world unreadable: {e}"),
            FlowError::ReconcileBehind { decision } => write!(
                f,
                "reconciliation store has not applied decision {decision}: it cannot arbitrate"
            ),
            FlowError::Commit(e) => write!(f, "commit gateway refused: {e}"),
        }
    }
}

impl std::error::Error for FlowError {}

// ---------------------------------------------------------------------------
// Record types + codecs
// ---------------------------------------------------------------------------

/// A committed POLICY truth (Governance-owned content, Control-Plane-enforced
/// — Vol 9 Part 10; the Control Plane never owns it, ORCH-001). Semantics in
/// Stage 2 (the G1 `checkPolicy` subset, minimal on purpose — the arbitration
/// policy LANGUAGE is design OQ-8): a decision whose subject starts with
/// `scope` requires a separate committed approval truth for that exact
/// subject by a REGISTERED agent other than the proposer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyRecord {
    /// Policy name (non-empty; audit label).
    pub name: String,
    /// Subject prefix this policy applies to (non-empty).
    pub scope: String,
    /// Policy version (versioned like every registry record, Vol 14 Part 20
    /// discipline; a new version is a NEW policy truth).
    pub version: u32,
}

impl PolicyRecord {
    fn validate(&self) -> Result<(), FlowError> {
        if self.name.is_empty() {
            return Err(FlowError::InvalidRecord("policy name must be non-empty".into()));
        }
        if self.scope.is_empty() {
            return Err(FlowError::InvalidRecord("policy scope must be non-empty".into()));
        }
        Ok(())
    }

    /// Canonical committed body (deterministic byte-for-byte).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(POLICY_MAGIC);
        put_part(&mut b, self.name.as_bytes());
        put_part(&mut b, self.scope.as_bytes());
        b.extend_from_slice(&self.version.to_le_bytes());
        b
    }

    /// Whether this policy applies to `subject` (prefix rule; deterministic).
    pub fn applies_to(&self, subject: &str) -> bool {
        subject.starts_with(&self.scope)
    }
}

/// Decode a committed payload as a policy truth (wrong magic/malformed → None).
pub fn decode_policy(payload: &[u8]) -> Option<PolicyRecord> {
    let rest = payload.strip_prefix(POLICY_MAGIC)?;
    let mut pos = 0usize;
    let name = take_string(rest, &mut pos)?;
    let scope = take_string(rest, &mut pos)?;
    let version = u32::from_le_bytes(take(rest, &mut pos, 4)?.try_into().ok()?);
    if pos != rest.len() {
        return None; // trailing garbage refused (house codec discipline)
    }
    Some(PolicyRecord { name, scope, version })
}

/// A committed APPROVAL truth — a SEPARATE truth from the decision it later
/// authorizes (the G1 E1 fix, design §3.19: "approvals are separate,
/// addressable truths cited by the decisions they authorize — auditors can
/// verify authorization without trusting the proposer").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalRecord {
    /// The exact subject this approval authorizes.
    pub subject: String,
    /// The approving agent (the structural proposer ≠ approver half; the
    /// cryptographic half is v2.0 debt #8, said out loud).
    pub approver: AgentId,
}

impl ApprovalRecord {
    /// Canonical committed body.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(APPROVAL_MAGIC);
        put_part(&mut b, self.subject.as_bytes());
        put_part(&mut b, self.approver.bytes());
        b
    }
}

/// Decode a committed payload as an approval truth.
pub fn decode_approval(payload: &[u8]) -> Option<ApprovalRecord> {
    let rest = payload.strip_prefix(APPROVAL_MAGIC)?;
    let mut pos = 0usize;
    let subject = take_string(rest, &mut pos)?;
    let approver = take_part(rest, &mut pos)?.to_vec();
    if pos != rest.len() {
        return None;
    }
    Some(ApprovalRecord { subject, approver: AgentId::from_raw(approver) })
}

/// A committed AGENT-DECISION truth: the Who (agent), the What (subject +
/// action) and the Why (cited approval content ids — Vol 2 Part 20 audit).
/// Distinct from the frozen contract-only [`crate::DecisionRecord`] (the
/// orchestrator's plan-trace record): this is a FLOW truth payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentDecisionRecord {
    /// The decision subject (the contention unit of the conflict rule).
    pub subject: String,
    /// The decided action.
    pub action: String,
    /// The proposing agent (the audit Who, inside the addressed content —
    /// RCR-029: attribution can never be silently rewritten).
    pub agent: AgentId,
    /// Hex content ids of the QUALIFYING approval truths this decision cites
    /// (sorted; the audit Why). Qualifying = exact subject, registered
    /// approver, approver ≠ proposer. Citation of the full read-set is design
    /// OQ-4 and NOT claimed here.
    pub cites: Vec<String>,
}

impl AgentDecisionRecord {
    /// Canonical committed body.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(DECISION_MAGIC);
        put_part(&mut b, self.subject.as_bytes());
        put_part(&mut b, self.action.as_bytes());
        put_part(&mut b, self.agent.bytes());
        b.extend_from_slice(&(self.cites.len() as u32).to_le_bytes());
        for cite in &self.cites {
            put_part(&mut b, cite.as_bytes());
        }
        b
    }
}

/// Decode a committed payload as an agent-decision truth.
pub fn decode_decision(payload: &[u8]) -> Option<AgentDecisionRecord> {
    let rest = payload.strip_prefix(DECISION_MAGIC)?;
    let mut pos = 0usize;
    let subject = take_string(rest, &mut pos)?;
    let action = take_string(rest, &mut pos)?;
    let agent = take_part(rest, &mut pos)?.to_vec();
    let n = u32::from_le_bytes(take(rest, &mut pos, 4)?.try_into().ok()?) as usize;
    let mut cites = Vec::with_capacity(n);
    for _ in 0..n {
        cites.push(take_string(rest, &mut pos)?);
    }
    if pos != rest.len() {
        return None;
    }
    Some(AgentDecisionRecord { subject, action, agent: AgentId::from_raw(agent), cites })
}

/// What a committed compliance event records (G1 `uci.compliance` semantics:
/// `outcome: blocked | conflict`, elevated to runtime payloads).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComplianceOutcome {
    /// A policy gate refused the decision; the refusal cites the policy truth.
    Blocked {
        /// Name of the violated policy (audit label).
        policy: String,
        /// Hex content id of the committed policy truth that fired.
        policy_id: String,
    },
    /// A conflicting prior decision won the subject; the loser cites it.
    Conflict {
        /// Hex content id of the WINNING (first-committed) decision truth.
        prior_id: String,
        /// The winner's action.
        prior_action: String,
        /// The refused/superseded proposal's action.
        proposed_action: String,
    },
}

/// A committed COMPLIANCE-EVENT truth: the replayable audit trail of what was
/// *attempted* (design §3.19 — "blocked decisions and conflicts are recorded
/// as truths, not just refused").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComplianceEvent {
    /// The contested subject.
    pub subject: String,
    /// The agent whose attempt was blocked / lost the conflict (audit Who).
    pub agent: AgentId,
    /// What happened.
    pub outcome: ComplianceOutcome,
}

impl ComplianceEvent {
    /// Canonical committed body.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(COMPLIANCE_MAGIC);
        put_part(&mut b, self.subject.as_bytes());
        put_part(&mut b, self.agent.bytes());
        match &self.outcome {
            ComplianceOutcome::Blocked { policy, policy_id } => {
                b.push(1);
                put_part(&mut b, policy.as_bytes());
                put_part(&mut b, policy_id.as_bytes());
            }
            ComplianceOutcome::Conflict { prior_id, prior_action, proposed_action } => {
                b.push(2);
                put_part(&mut b, prior_id.as_bytes());
                put_part(&mut b, prior_action.as_bytes());
                put_part(&mut b, proposed_action.as_bytes());
            }
        }
        b
    }
}

/// Decode a committed payload as a compliance-event truth.
pub fn decode_compliance(payload: &[u8]) -> Option<ComplianceEvent> {
    let rest = payload.strip_prefix(COMPLIANCE_MAGIC)?;
    let mut pos = 0usize;
    let subject = take_string(rest, &mut pos)?;
    let agent = take_part(rest, &mut pos)?.to_vec();
    let tag = *take(rest, &mut pos, 1)?.first()?;
    let outcome = match tag {
        1 => ComplianceOutcome::Blocked {
            policy: take_string(rest, &mut pos)?,
            policy_id: take_string(rest, &mut pos)?,
        },
        2 => ComplianceOutcome::Conflict {
            prior_id: take_string(rest, &mut pos)?,
            prior_action: take_string(rest, &mut pos)?,
            proposed_action: take_string(rest, &mut pos)?,
        },
        _ => return None,
    };
    if pos != rest.len() {
        return None;
    }
    Some(ComplianceEvent { subject, agent: AgentId::from_raw(agent), outcome })
}

// ---------------------------------------------------------------------------
// Derived readers (pure folds of the committed world — deterministic on every
// replica; ORCH-003)
// ---------------------------------------------------------------------------

/// One decision truth as derived from the committed world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecidedTruth {
    /// Hex content id of the decision record.
    pub id_hex: String,
    /// The decoded record.
    pub record: AgentDecisionRecord,
    /// Shard log offset at which it committed (the total order of the
    /// first-committed-wins rule — IDR-001/IDR-005).
    pub committed_at: u64,
}

/// THE deterministic conflict rule (module doc): the decision truth of
/// `subject` is the FIRST **governed** decision record on that subject in
/// shard log order. A pure function of the committed trace — identical across
/// re-reads and replicas at the same version (ORCH-003); later conflicting
/// records exist only as superseded attempts and never derive.
///
/// **Governed-only derivation (RCR-030 amendment A1, the RCR-029-A1 analog):**
/// the frozen `Kernel::commit` gateway does not verify `content ==
/// ACS-hash(payload)` and has no principal, so ANY caller can lawfully commit
/// a hand-crafted decision record wearing an arbitrary Who into the shard's
/// WAL. A record whose claimed agent is NOT registered committed truth in the
/// SAME world therefore NEVER derives — it stays visible as raw trace but
/// cannot win, block, or supersede a governed decision ("no ungoverned
/// agents", Vol 2 Part 23; pinned by
/// `smuggled_ungoverned_decision_never_derives`). HONEST LIMIT, said out
/// loud: a smuggled record wearing a REGISTERED identity still derives — any
/// in-process caller can wear any registered identity under v1.x
/// (trusted-single-host model, v2.0 debt #8 / design OQ-1) — and this
/// raw-gateway path also bypasses the POLICY/approval gate entirely: on a
/// policy-scoped subject such a record derives with NO approval and with
/// unverifiable `cites`, because derivation checks registration ONLY, never
/// cites/policy (only [`propose_decision`] is governed-gated; same v1.x
/// debt-#8 class, pinned by the same test).
pub fn decision_of(world: &WorldView, subject: &str) -> Option<DecidedTruth> {
    decisions_on(world, subject).into_iter().next()
}

/// Every **governed** decision record on `subject` visible in `world` (claimed
/// agent registered as committed truth in this same world — see
/// [`decision_of`] amendment A1), in commit order: index 0 is the derived
/// winner ([`decision_of`]); the rest are superseded attempts kept as honest,
/// replayable trace (design §3.19). Ungoverned (unregistered-Who) records are
/// excluded from derivation but remain addressable raw truth in the world.
pub fn decisions_on(world: &WorldView, subject: &str) -> Vec<DecidedTruth> {
    let mut rows: Vec<DecidedTruth> = world
        .iter()
        .filter_map(|(id, payload, at)| {
            decode_decision(payload)
                .filter(|r| r.subject == subject && is_registered(world, &r.agent))
                .map(|record| DecidedTruth { id_hex: id.to_string(), record, committed_at: at })
        })
        .collect();
    rows.sort_by_key(|d| d.committed_at);
    rows
}

/// Every committed policy truth visible in `world`, in commit order:
/// `(record, id_hex, committed_at)`.
pub fn policies_in(world: &WorldView) -> Vec<(PolicyRecord, String, u64)> {
    let mut rows: Vec<(PolicyRecord, String, u64)> = world
        .iter()
        .filter_map(|(id, payload, at)| decode_policy(payload).map(|p| (p, id.to_string(), at)))
        .collect();
    rows.sort_by_key(|(_, _, at)| *at);
    rows
}

/// Every committed approval truth for exactly `subject`, in commit order:
/// `(record, id_hex, committed_at)`. NOTE (RCR-029 A2 honesty): the approver
/// in each row is the CLAIMED identity in the payload; qualification against
/// registration happens at the policy gate ([`propose_decision`]).
pub fn approvals_on(world: &WorldView, subject: &str) -> Vec<(ApprovalRecord, String, u64)> {
    let mut rows: Vec<(ApprovalRecord, String, u64)> = world
        .iter()
        .filter_map(|(id, payload, at)| {
            decode_approval(payload)
                .filter(|a| a.subject == subject)
                .map(|a| (a, id.to_string(), at))
        })
        .collect();
    rows.sort_by_key(|(_, _, at)| *at);
    rows
}

/// Every committed compliance event for `subject`, in commit order:
/// `(event, id_hex, committed_at)` — the audit walk of what was attempted.
pub fn compliance_on(world: &WorldView, subject: &str) -> Vec<(ComplianceEvent, String, u64)> {
    let mut rows: Vec<(ComplianceEvent, String, u64)> = world
        .iter()
        .filter_map(|(id, payload, at)| {
            decode_compliance(payload)
                .filter(|c| c.subject == subject)
                .map(|c| (c, id.to_string(), at))
        })
        .collect();
    rows.sort_by_key(|(_, _, at)| *at);
    rows
}

// ---------------------------------------------------------------------------
// Commit helpers (the ONLY path to truth: the frozen gateway — ORCH-001)
// ---------------------------------------------------------------------------

/// A record committed (or idempotently resolved) through the frozen gateway.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedRecord {
    /// Hex content id of the committed payload (its address in the world).
    pub id_hex: String,
    /// The committed truth.
    pub truth: TruthRef,
    /// `true` iff this call created the truth (ORCH-004: a duplicate resolves
    /// to the SAME truth with `fresh: false`, never a fork).
    pub fresh: bool,
}

fn kernel_shard_of(shard: &LcwShardKey) -> Result<KernelShardKey, FlowError> {
    KernelShardKey::new(shard.tenant.clone(), shard.workspace.clone()).map_err(FlowError::BadShard)
}

fn commit_payload<K: Kernel>(
    kernel: &K,
    shard: &KernelShardKey,
    payload: Vec<u8>,
) -> Result<CommittedRecord, FlowError> {
    let content = arves_acs::content_id(arves_acs::domain::COMMIT_CONTENT, &payload);
    let id_hex = arves_acs::hex(&content);
    let proposed = ProposedWrite { shard: shard.clone(), content: ContentHash(content), payload };
    match kernel.commit(proposed) {
        Ok(truth) => Ok(CommittedRecord { id_hex, truth, fresh: true }),
        Err(CommitError::AlreadyCommitted(truth)) => {
            Ok(CommittedRecord { id_hex, truth, fresh: false })
        }
        Err(e) => Err(FlowError::Commit(e)),
    }
}

/// Commit `policy` as Governance policy truth into `shard`. Idempotent
/// (ORCH-004). Policies are governance content the Control Plane ENFORCES,
/// never owns (Vol 9 Part 10) — this function holds nothing afterwards.
pub fn commit_policy<K: Kernel>(
    kernel: &K,
    shard: &KernelShardKey,
    policy: &PolicyRecord,
) -> Result<CommittedRecord, FlowError> {
    policy.validate()?;
    commit_payload(kernel, shard, policy.canonical_bytes())
}

/// Commit an approval truth for `subject` by `approver` into the shard whose
/// committed truth `basis` reflects. Structural gate: the approver must be
/// registered committed truth in that basis (refusal commits nothing).
/// Idempotent (ORCH-004).
pub fn commit_approval<K: Kernel>(
    kernel: &K,
    basis: &WorldView,
    approver: &AgentId,
    subject: &str,
) -> Result<CommittedRecord, FlowError> {
    if subject.is_empty() {
        return Err(FlowError::InvalidRecord("approval subject must be non-empty".into()));
    }
    if !is_registered(basis, approver) {
        return Err(FlowError::NotRegistered { agent: approver.hex() });
    }
    let shard = kernel_shard_of(basis.shard())?;
    let record = ApprovalRecord { subject: subject.into(), approver: approver.clone() };
    commit_payload(kernel, &shard, record.canonical_bytes())
}

// ---------------------------------------------------------------------------
// (a) Scheduler-borne agent proposals — through the I4 dispatch pipeline
// ---------------------------------------------------------------------------

/// Submit an agent-attributed effect THROUGH the I4 cluster scheduler
/// (design §3.1.2: "concurrent agent proposals through the scheduler into the
/// cluster"). The agent never commits: the scheduler's dispatch pipeline
/// (gate → engine → proposed effects) routes every commit through the shard
/// leader's Kernel gateway (ORCH-001), and the schedule itself stays a
/// discardable plan artifact (ORCH-002 — proven at the I4 surface, re-proven
/// under multi-agent load in this stage's tests).
///
/// Structural gate (the RCR-029 elevation): `agent` must be registered
/// committed truth in the DECLARED basis `world`; refusal queues nothing and
/// commits nothing. The attribution envelope (RCR-029) is the invocation
/// INPUT, so the engine's proposed effect — and therefore the committed truth
/// — carries the Who inside the addressed content. The bound capability's
/// engine must propose the envelope verbatim for the attribution to land
/// (the deterministic echo actor of the tests); the fabric-derived ORCH-004
/// key then makes duplicate proposals of one agent collapse at the scheduler
/// ledger AND duplicate commits converge at the Kernel.
///
/// `policy` is the Governance verdict input of the I4 gate (enforced, never
/// computed here); the decision-flow policy gate ([`propose_decision`]) is a
/// separate, committed-truth-based check.
#[allow(clippy::too_many_arguments)] // explicit inputs over ambient state (house style)
pub fn submit_attributed_effect<R: CapabilityRegistry>(
    scheduler: &mut ClusterScheduler,
    tick: u64,
    world: &WorldView,
    agent: &AgentId,
    capability: CapabilityId,
    policy: PolicyVerdict,
    effect: &[u8],
    env: &DispatchEnv<'_, R>,
) -> Result<SubmitOutcome, FlowError> {
    if !is_registered(world, agent) {
        return Err(FlowError::NotRegistered { agent: agent.hex() });
    }
    let shard = ShardId::new(
        TenantId(world.shard().tenant.clone()),
        WorkspaceId(world.shard().workspace.clone()),
    );
    let input = encode_attributed(agent, effect);
    Ok(scheduler.submit(tick, InvocationSpec { shard, capability, policy, input }, env))
}

// ---------------------------------------------------------------------------
// (b)+(d) The decision flow — propose, gate, converge or conflict
// ---------------------------------------------------------------------------

/// Outcome of one [`propose_decision`] call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProposalOutcome {
    /// This proposal's record IS the subject's derived decision truth
    /// (confirmed at the serialization point against the at-head world).
    Committed {
        /// The committed decision truth.
        truth: TruthRef,
        /// The derived decision (== this proposal's record).
        decision: DecidedTruth,
    },
    /// A decision with the SAME action already holds the subject — the
    /// proposal converges onto that ONE truth (ORCH-004 across agents at the
    /// decision level; mandate (b): same content converges). Nothing
    /// conflicting was recorded. `superseded_attempt` is `Some` only when the
    /// agreement was discovered at the serialization point (both raced their
    /// commits in): our own record exists in the trace but never derives.
    Converged {
        /// The one decision truth of the subject.
        winner: DecidedTruth,
        /// Our own committed-but-non-deriving record, if the race committed it.
        superseded_attempt: Option<TruthRef>,
    },
    /// Policy violation: refused as decision-truth, recorded as committed
    /// compliance truth (never a silent refusal — design §3.19).
    Blocked {
        /// Name of the violated policy.
        policy: String,
        /// The committed compliance event (`outcome: blocked`).
        compliance: TruthRef,
    },
    /// Conflict: a FIRST-COMMITTED decision with a DIFFERENT action holds the
    /// subject. The loser receives the winner's identity (the enterprise-os
    /// reference rule at runtime level) and the conflict is committed
    /// compliance truth citing the winner. `superseded_attempt` is `Some` when
    /// the loss was discovered at the serialization point (our record was
    /// committed but does not derive), `None` when the declared basis already
    /// showed the winner (nothing of ours was committed except the event).
    Conflict {
        /// The winning (first-committed) decision truth.
        winner: DecidedTruth,
        /// The committed compliance event (`outcome: conflict`).
        compliance: TruthRef,
        /// Our own committed-but-non-deriving record, if the race committed it.
        superseded_attempt: Option<TruthRef>,
    },
}

/// The earliest-committed policy in `basis` that applies to `subject` and is
/// NOT satisfied by a qualifying approval (qualifying = exact subject,
/// registered approver, approver ≠ proposer). Deterministic report order.
fn first_violated_policy(
    basis: &WorldView,
    proposer: &AgentId,
    subject: &str,
) -> Option<(PolicyRecord, String)> {
    let satisfied = !qualifying_approvals(basis, proposer, subject).is_empty();
    policies_in(basis)
        .into_iter()
        .find(|(p, _, _)| p.applies_to(subject) && !satisfied)
        .map(|(p, id, _)| (p, id))
}

/// Hex ids of the qualifying approvals for `subject` in `basis`, sorted
/// (the decision's `cites` — the audit Why).
fn qualifying_approvals(basis: &WorldView, proposer: &AgentId, subject: &str) -> Vec<String> {
    let mut ids: Vec<String> = approvals_on(basis, subject)
        .into_iter()
        .filter(|(a, _, _)| a.approver != *proposer && is_registered(basis, &a.approver))
        .map(|(_, id, _)| id)
        .collect();
    ids.sort();
    ids
}

/// Propose `action` on `subject` as a decision of `proposer`, into the shard
/// whose committed truth `basis` reflects. The full flow (module doc):
///
/// 1. **Structural gate** — proposer registered in the declared `basis`
///    (committed truth, not orchestrator memory).
/// 2. **Policy gate (declared basis)** — every committed policy truth whose
///    scope prefixes `subject` must be satisfied by a separate committed
///    approval truth (registered approver ≠ proposer). Violation → the refusal
///    is COMMITTED as a compliance event and reported [`ProposalOutcome::Blocked`].
///    Honest limit (OQ-3 class, pinned by test): a policy committed after
///    `basis` is not seen by this proposal.
/// 3. **Conflict pre-check (declared basis)** — an existing decision on the
///    subject converges (same action) or conflicts (different action, the
///    conflict committed as compliance truth citing the winner) WITHOUT
///    committing any decision record.
/// 4. **Commit** — the decision record (citing its qualifying approvals) goes
///    through the frozen gateway. Idempotent re-proposal resolves to the same
///    truth (ORCH-004).
/// 5. **Serialization-point reconciliation** — the world is re-read AT HEAD
///    from `reconcile_store` (which MUST have applied our own commit — the
///    leader's store, or any caught-up replica; otherwise
///    [`FlowError::ReconcileBehind`], loudly). If a racing proposal committed
///    FIRST in shard log order, ours does not derive: the loser receives the
///    winner ([`ProposalOutcome::Conflict`]/[`ProposalOutcome::Converged`]
///    with `superseded_attempt`), and a conflicting loss is committed as
///    compliance truth. First-committed-wins is total and replica-identical
///    (IDR-001/IDR-005 log order).
pub fn propose_decision<K: Kernel, S: WalStore>(
    kernel: &K,
    reconcile_store: &S,
    basis: &WorldView,
    proposer: &AgentId,
    subject: &str,
    action: &str,
) -> Result<ProposalOutcome, FlowError> {
    if subject.is_empty() {
        return Err(FlowError::InvalidRecord("decision subject must be non-empty".into()));
    }
    if action.is_empty() {
        return Err(FlowError::InvalidRecord("decision action must be non-empty".into()));
    }
    // (1) structural gate against committed truth at the declared basis.
    if !is_registered(basis, proposer) {
        return Err(FlowError::NotRegistered { agent: proposer.hex() });
    }
    let shard = kernel_shard_of(basis.shard())?;
    // (2) policy gate: reads COMMITTED policy truths + COMMITTED approvals.
    if let Some((policy, policy_id)) = first_violated_policy(basis, proposer, subject) {
        let event = ComplianceEvent {
            subject: subject.into(),
            agent: proposer.clone(),
            outcome: ComplianceOutcome::Blocked { policy: policy.name.clone(), policy_id },
        };
        let compliance = commit_payload(kernel, &shard, event.canonical_bytes())?;
        return Ok(ProposalOutcome::Blocked { policy: policy.name, compliance: compliance.truth });
    }
    // (3) conflict pre-check against the declared basis.
    if let Some(prior) = decision_of(basis, subject) {
        if prior.record.action == action {
            return Ok(ProposalOutcome::Converged { winner: prior, superseded_attempt: None });
        }
        let event = ComplianceEvent {
            subject: subject.into(),
            agent: proposer.clone(),
            outcome: ComplianceOutcome::Conflict {
                prior_id: prior.id_hex.clone(),
                prior_action: prior.record.action.clone(),
                proposed_action: action.into(),
            },
        };
        let compliance = commit_payload(kernel, &shard, event.canonical_bytes())?;
        return Ok(ProposalOutcome::Conflict {
            winner: prior,
            compliance: compliance.truth,
            superseded_attempt: None,
        });
    }
    // (4) commit the decision record through the frozen gateway.
    let record = AgentDecisionRecord {
        subject: subject.into(),
        action: action.into(),
        agent: proposer.clone(),
        cites: qualifying_approvals(basis, proposer, subject),
    };
    let ours = commit_payload(kernel, &shard, record.canonical_bytes())?;
    // (5) serialization-point reconciliation at head.
    let head = WorldView::at_head(reconcile_store, basis.shard()).map_err(FlowError::World)?;
    if !head.contains(&ours.id_hex) {
        return Err(FlowError::ReconcileBehind { decision: ours.id_hex });
    }
    let winner = decision_of(&head, subject)
        .expect("head world contains at least our own decision record");
    if winner.id_hex == ours.id_hex {
        return Ok(ProposalOutcome::Committed { truth: ours.truth, decision: winner });
    }
    if winner.record.action == record.action {
        // Lost the race to an AGREEING decision: one truth, no conflict.
        return Ok(ProposalOutcome::Converged {
            winner,
            superseded_attempt: Some(ours.truth),
        });
    }
    let event = ComplianceEvent {
        subject: subject.into(),
        agent: proposer.clone(),
        outcome: ComplianceOutcome::Conflict {
            prior_id: winner.id_hex.clone(),
            prior_action: winner.record.action.clone(),
            proposed_action: action.into(),
        },
    };
    let compliance = commit_payload(kernel, &shard, event.canonical_bytes())?;
    Ok(ProposalOutcome::Conflict {
        winner,
        compliance: compliance.truth,
        superseded_attempt: Some(ours.truth),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(tag: u8) -> AgentId {
        AgentId::from_raw(vec![0x12, 0x20, tag])
    }

    #[test]
    fn policy_codec_round_trips_and_rejects_garbage() {
        let p = PolicyRecord { name: "legal-review".into(), scope: "contract/".into(), version: 1 };
        assert_eq!(decode_policy(&p.canonical_bytes()), Some(p.clone()));
        let mut garbage = p.canonical_bytes();
        garbage.push(0);
        assert_eq!(decode_policy(&garbage), None, "trailing garbage refused");
        assert_eq!(decode_policy(b"not-a-policy"), None);
        // Cross-magic confusion refused: a policy never decodes as a decision.
        assert_eq!(decode_decision(&p.canonical_bytes()), None);
    }

    #[test]
    fn approval_decision_compliance_codecs_round_trip_and_reject_garbage() {
        let a = ApprovalRecord { subject: "contract/msa".into(), approver: agent(1) };
        assert_eq!(decode_approval(&a.canonical_bytes()), Some(a.clone()));
        let d = AgentDecisionRecord {
            subject: "contract/msa".into(),
            action: "sign".into(),
            agent: agent(2),
            cites: vec!["aa".into(), "bb".into()],
        };
        assert_eq!(decode_decision(&d.canonical_bytes()), Some(d.clone()));
        let blocked = ComplianceEvent {
            subject: "s".into(),
            agent: agent(3),
            outcome: ComplianceOutcome::Blocked { policy: "p".into(), policy_id: "cc".into() },
        };
        assert_eq!(decode_compliance(&blocked.canonical_bytes()), Some(blocked.clone()));
        let conflict = ComplianceEvent {
            subject: "s".into(),
            agent: agent(4),
            outcome: ComplianceOutcome::Conflict {
                prior_id: "dd".into(),
                prior_action: "approve".into(),
                proposed_action: "reject".into(),
            },
        };
        assert_eq!(decode_compliance(&conflict.canonical_bytes()), Some(conflict.clone()));
        for bytes in [a.canonical_bytes(), d.canonical_bytes(), conflict.canonical_bytes()] {
            let mut garbage = bytes.clone();
            garbage.push(0);
            assert!(
                decode_approval(&garbage).is_none()
                    && decode_decision(&garbage).is_none()
                    && decode_compliance(&garbage).is_none(),
                "trailing garbage refused on every decoder"
            );
        }
        // An unknown compliance tag is refused, never guessed.
        let mut bad_tag = ComplianceEvent {
            subject: "s".into(),
            agent: agent(5),
            outcome: ComplianceOutcome::Blocked { policy: "p".into(), policy_id: "e".into() },
        }
        .canonical_bytes();
        let tag_pos = COMPLIANCE_MAGIC.len() + 4 + 1 + 4 + 3; // subject "s" + agent 3 bytes
        bad_tag[tag_pos] = 9;
        assert_eq!(decode_compliance(&bad_tag), None);
    }

    #[test]
    fn policy_scope_is_a_deterministic_prefix_rule() {
        let p = PolicyRecord { name: "n".into(), scope: "budget/".into(), version: 1 };
        assert!(p.applies_to("budget/q3"));
        assert!(p.applies_to("budget/"));
        assert!(!p.applies_to("plan/q3"));
        assert!(!p.applies_to("budget")); // strict prefix, no fuzzy match
    }

    #[test]
    fn record_validation_minima_are_enforced_before_any_commit() {
        assert!(PolicyRecord { name: String::new(), scope: "s/".into(), version: 1 }
            .validate()
            .is_err());
        assert!(PolicyRecord { name: "n".into(), scope: String::new(), version: 1 }
            .validate()
            .is_err());
    }
}

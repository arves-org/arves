//! ARVES :: arves-execution
//!
//! Purpose: Execution layer: performs actions; routes outcomes as proposed writes
//!          to the Kernel. This crate is the Data-Plane actuator that *carries out*
//!          a plan produced by the Control Plane and turns the result into a
//!          [`ProposedWrite`] destined for the Kernel's commit gateway.
//!
//! Governing invariants (cited inline throughout):
//!   * ORCH-001  Control Plane owns no truth; only the Kernel owns truth. Execution
//!               therefore never commits: it *proposes*. All effects on cognitive
//!               state are expressed as [`ProposedWrite`]s routed to the Kernel.
//!   * ORCH-002  Control Plane produces plans, never persistent state. The [`Plan`]
//!               this layer receives is transient; Execution holds no durable state.
//!   * ORCH-003  Replay is from the recorded decision trace, not recomputation. Every
//!               [`Outcome`] carries a [`DecisionTraceRef`] so a committed outcome can
//!               be replayed without re-running the action.
//!   * ORCH-004  Every engine/capability invocation is idempotent + content-addressable.
//!               See [`InvocationId`] (content address) and [`Executor::execute`]'s
//!               idempotency contract.
//!   * OWN-001   One owner per state. Execution owns only its in-flight [`ExecutionId`]
//!               bookkeeping; it owns no cognitive truth (that is the Kernel's).
//!   * LAYER-001 Layers are downward-only. Execution sits below Capability and above
//!               (i.e. it depends on) the Kernel's commit gateway; it never calls
//!               upward into the Control Plane.
//!   * SHARD-001 Partition by tenant/workspace; the shard key is immutable. Every
//!               [`ProposedWrite`] carries an immutable [`ShardKey`] so the Kernel can
//!               route it to the correct per-shard Raft group.
//!
//! Amendments (frozen):
//!   * Amendment-005 (cancellation): cancellation is *cooperative* and *idempotent*.
//!                    See [`CancellationToken`] and [`Cancellation`].
//!   * Amendment-006 (failure/saga): there is no cross-shard atomic commit (IDR:
//!                    sagas). Failure of a multi-step plan is handled by recording a
//!                    [`Failure`] and driving compensating actions, never by a
//!                    distributed rollback. See [`Compensation`] and [`Outcome::Failed`].
//!
//! Layer: Data Plane (the Control Plane decides; the Data Plane carries).
//!
//! STATUS: I1 (Distributed Runtime) — interfaces/contracts. The Executor,
//! OutcomeRouter and Cancellation *traits* are contract-only (no method bodies;
//! signatures are the contract). The concrete, exercised code in this crate is
//! [`CancellationToken`], which is a working cooperative-cancellation primitive
//! (shared atomic flag; see the unit tests). The frozen specification governs;
//! this crate *implements* the spec and never changes it (Theory -> Spec ->
//! Contracts -> Behaviour -> Conformance -> Implementation).
//!
//! This crate is std-only: it declares no dependencies and imports no sibling
//! crate. The Kernel/Plan/Shard types referenced here are modelled as opaque,
//! self-contained newtypes so the contract is expressible without coupling.

#![forbid(unsafe_code)]

// ---------------------------------------------------------------------------
// Identity & content-addressing (ORCH-004, SHARD-001)
// ---------------------------------------------------------------------------

/// Immutable partition key: a tenant/workspace boundary.
///
/// Cites SHARD-001: state is partitioned by tenant/workspace and the shard key is
/// immutable for the life of the state it addresses. Execution never rewrites a
/// [`ShardKey`]; it only reads the one attached to the [`Plan`] and copies it onto
/// each emitted [`ProposedWrite`] so the Kernel can route to the correct per-shard
/// Raft group (IDR-001).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShardKey {
    /// Tenant identity component. Immutable once assigned.
    pub tenant: String,
    /// Workspace identity component within the tenant. Immutable once assigned.
    pub workspace: String,
}

/// Content address of a single engine/capability invocation.
///
/// Cites ORCH-004: every invocation is content-addressable. The identifier is
/// derived from the invocation's *content* (capability + inputs + shard), so two
/// invocations with identical content collide by construction. This is the hook
/// that makes [`Executor::execute`] idempotent: re-executing the same content
/// yields the same [`InvocationId`] and therefore the same de-duplicated effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InvocationId(pub String);

/// Opaque, unique handle for one in-flight [`Plan`] execution.
///
/// Cites OWN-001 + ORCH-002: this is the *only* state Execution owns, and it is
/// transient bookkeeping, not persistent truth. It is used to correlate
/// cancellation ([`Cancellation`]) and outcomes for a running execution.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExecutionId(pub String);

/// Reference into the Kernel's recorded decision trace (Raft log = WAL = trace).
///
/// Cites ORCH-003 + IDR: replay is driven from the recorded decision trace, never
/// from recomputation. An [`Outcome`] carries this so that, once the resulting
/// [`ProposedWrite`] is committed, the outcome can be reconstructed by replaying
/// the trace rather than re-running the (possibly non-deterministic) action.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DecisionTraceRef(pub String);

// ---------------------------------------------------------------------------
// Plan input (ORCH-002) — what the Control Plane hands the Data Plane
// ---------------------------------------------------------------------------

/// A single actionable step within a [`Plan`].
///
/// Cites ORCH-004: each step names a capability/engine and carries the exact,
/// content-addressed inputs it will be invoked with, so the step's effect is
/// idempotent and reproducible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanStep {
    /// Content address of this step's invocation (ORCH-004).
    pub invocation: InvocationId,
    /// Logical capability/engine to invoke (binding is resolved by the Capability
    /// Fabric upstream; Execution treats it as an opaque name).
    pub capability: String,
    /// Opaque, already-canonicalized inputs. Kept as bytes so this crate stays
    /// self-contained (std-only) and does not model the ontology.
    pub input: Vec<u8>,
}

/// A transient, Control-Plane-produced execution plan.
///
/// Cites ORCH-002: the Control Plane produces plans, never persistent state; and
/// ORCH-001: a plan is a *decision*, it carries no truth. Execution consumes a
/// [`Plan`] and produces [`Outcome`]s; it never persists the plan itself.
///
/// Cites SHARD-001: the plan is scoped to exactly one immutable [`ShardKey`]. There
/// is no cross-shard atomic commit (Amendment-006 / IDR); multi-shard work is
/// decomposed upstream into per-shard plans and stitched by sagas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Immutable partition this plan executes within (SHARD-001).
    pub shard: ShardKey,
    /// Ordered steps to perform.
    pub steps: Vec<PlanStep>,
}

// ---------------------------------------------------------------------------
// Proposed writes (ORCH-001, G-001 proposed) — the only way to touch truth
// ---------------------------------------------------------------------------

/// A write *proposed* to the Kernel — never a committed write.
///
/// Cites ORCH-001 (and G-001, proposed): the Kernel is the sole owner of truth and
/// the sole commit gateway. Execution's entire externally-visible effect is a
/// stream of [`ProposedWrite`]s; whether they become truth is the Kernel's
/// decision, reached via the per-shard Raft group (IDR-002: replicate committed
/// *outcomes*, not invocations).
///
/// Cites SHARD-001: the proposed write carries the immutable [`ShardKey`] copied
/// from the originating [`Plan`], so the Kernel routes it to the right shard leader
/// (IDR-003: engines run anywhere, but commit only via the shard leader).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedWrite {
    /// Immutable destination partition (SHARD-001).
    pub shard: ShardKey,
    /// The invocation that produced this proposal (ORCH-004 content address).
    pub origin: InvocationId,
    /// Opaque, canonicalized payload to be committed by the Kernel if accepted.
    pub payload: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Outcomes (ORCH-003, Amendment-006) — the result of executing a plan
// ---------------------------------------------------------------------------

/// Structured description of a step/plan failure (Amendment-006).
///
/// Cites Amendment-006: failure is data, not an exception to unwind across shards.
/// A [`Failure`] records *what* failed and whether the effect is retryable, so the
/// Control Plane can decide whether to retry, compensate (see [`Compensation`]), or
/// abandon. There is no distributed rollback (IDR: no cross-shard atomic commit).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// The invocation that failed (ORCH-004 content address).
    pub invocation: InvocationId,
    /// Human-readable, non-authoritative diagnostic. Not truth; observability only.
    pub reason: String,
    /// Whether re-executing the same [`InvocationId`] is safe/expected to succeed.
    /// Because execution is idempotent (ORCH-004), retry never double-applies.
    pub retryable: bool,
}

/// The result of executing a [`Plan`].
///
/// Cites ORCH-003: every non-cancelled outcome carries a [`DecisionTraceRef`] and
/// the set of [`ProposedWrite`]s it wants committed, so the outcome is replayable
/// from the recorded trace rather than by recomputation.
///
/// Cites Amendment-005: [`Outcome::Cancelled`] is a first-class, terminal result of
/// cooperative cancellation — not an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The plan ran to completion. The contained writes are *proposed* to the
    /// Kernel (ORCH-001); commitment is the Kernel's decision.
    Completed {
        /// Handle for the completed execution (OWN-001 transient bookkeeping).
        execution: ExecutionId,
        /// Writes proposed to the Kernel commit gateway (ORCH-001).
        writes: Vec<ProposedWrite>,
        /// Trace anchor for replay (ORCH-003).
        trace: DecisionTraceRef,
    },
    /// The plan failed. Cites Amendment-006: no cross-shard rollback; the Control
    /// Plane drives retry/compensation using [`Compensation`].
    Failed {
        /// Handle for the failed execution.
        execution: ExecutionId,
        /// What failed.
        failure: Failure,
        /// Any writes already proposed before the failure (may be empty). These are
        /// *proposals*, so partial progress never violates ORCH-001.
        partial_writes: Vec<ProposedWrite>,
        /// Trace anchor for replay/diagnosis (ORCH-003).
        trace: DecisionTraceRef,
    },
    /// The plan was cancelled cooperatively before completion (Amendment-005). A
    /// terminal, non-error outcome; idempotent to observe.
    Cancelled {
        /// Handle for the cancelled execution.
        execution: ExecutionId,
        /// Trace anchor recording the cancellation point (ORCH-003).
        trace: DecisionTraceRef,
    },
}

// ---------------------------------------------------------------------------
// Cancellation (Amendment-005) — cooperative + idempotent
// ---------------------------------------------------------------------------

/// A cooperatively-checked, clonable cancellation signal.
///
/// Cites Amendment-005: cancellation is *cooperative* — an [`Executor`] polls
/// [`CancellationToken::is_cancelled`] at safe checkpoints between steps and stops
/// of its own accord; nothing is forcibly killed. It is *idempotent* — requesting
/// or observing cancellation any number of times has the same effect.
///
/// The token is intentionally a plain shared flag (std-only). A cloned token shares
/// the same underlying signal so a single [`CancellationToken::cancel`] (or a
/// [`Cancellation::cancel`] wired to it) is observed by every holder.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    // Shared cancellation flag. `Arc<AtomicBool>` so that cloning a token shares
    // one underlying signal across holders/threads (Amendment-005: a single
    // cancel is observed by every cooperative poller). std-only; no deps.
    flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancellationToken {
    /// Create a fresh, not-yet-cancelled token.
    pub fn new() -> Self {
        Self {
            flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Request cancellation on this token (and every clone that shares its signal).
    ///
    /// Cites Amendment-005: cancellation is *idempotent* — calling this any number
    /// of times has the same effect. It only flips a cooperative flag; it never
    /// touches cognitive truth (that is the Kernel's, ORCH-001).
    pub fn cancel(&self) {
        self.flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Cooperative checkpoint. `true` once cancellation has been requested on this
    /// token or any clone sharing its signal.
    ///
    /// Cites Amendment-005: safe to call repeatedly; observing cancellation is
    /// idempotent and must never mutate cognitive truth.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// The requester side of cooperative cancellation (Amendment-005).
///
/// Cites Amendment-005: `cancel` is idempotent — calling it more than once, or
/// after the execution has already finished, is a no-op with no additional effect.
/// Because Execution proposes rather than commits (ORCH-001), cancelling can never
/// leave partially-committed truth: at worst some [`ProposedWrite`]s were already
/// handed to the Kernel, which decides their fate independently.
pub trait Cancellation {
    /// Request cancellation of the identified execution. Idempotent.
    fn cancel(&self, execution: &ExecutionId);

    /// Obtain a token wired to the identified execution for cooperative polling.
    fn token(&self, execution: &ExecutionId) -> CancellationToken;
}

// ---------------------------------------------------------------------------
// Compensation (Amendment-006) — the saga alternative to distributed rollback
// ---------------------------------------------------------------------------

/// A saga-style compensating action for an already-proposed effect (Amendment-006).
///
/// Cites Amendment-006 + IDR (no cross-shard atomic commit): because there is no
/// distributed rollback, undoing a step means *forward* execution of a compensating
/// action, itself expressed as a [`Plan`] and routed like any other. Compensation
/// therefore reuses the same [`Executor`]/[`ProposedWrite`] machinery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compensation {
    /// The original invocation whose effect is being compensated (ORCH-004).
    pub compensates: InvocationId,
    /// The forward plan that performs the compensation, in the same shard
    /// (SHARD-001 — compensation is always intra-shard; cross-shard is a saga).
    pub plan: Plan,
}

// ---------------------------------------------------------------------------
// The Executor contract (ORCH-001, ORCH-003, ORCH-004, Amendment-005/006)
// ---------------------------------------------------------------------------

/// Sink that routes execution results toward the Kernel commit gateway.
///
/// Cites ORCH-001: Execution never commits. It hands [`ProposedWrite`]s to this
/// router, which forwards them to the shard leader (IDR-003) where the Kernel
/// decides commitment. Cites IDR-002: it is committed *outcomes* that replicate,
/// not the raw invocations.
pub trait OutcomeRouter {
    /// Route the writes carried by an [`Outcome`] to the Kernel commit gateway.
    ///
    /// Returns the trace reference under which the routing decision is recorded
    /// (ORCH-003). Routing a [`Outcome::Cancelled`] is valid and proposes nothing.
    fn route(&self, outcome: &Outcome) -> DecisionTraceRef;
}

/// The core Data-Plane actuator: performs a [`Plan`] and yields an [`Outcome`].
///
/// Contract (all cited):
///   * ORCH-001 — `execute` proposes, never commits. The returned [`Outcome`]'s
///     writes are [`ProposedWrite`]s; the Kernel alone commits truth.
///   * ORCH-002 — the input [`Plan`] is transient; the executor persists nothing.
///   * ORCH-003 — the returned [`Outcome`] carries a [`DecisionTraceRef`] enabling
///     replay from the recorded trace rather than recomputation.
///   * ORCH-004 — `execute` is *idempotent* and *content-addressable*: invoking it
///     twice with a [`Plan`] whose steps share the same [`InvocationId`]s must not
///     double-apply effects; the second call reconciles to the first outcome.
///   * Amendment-005 — `execute` honours the supplied [`CancellationToken`],
///     polling it cooperatively at step boundaries and returning
///     [`Outcome::Cancelled`] promptly.
///   * Amendment-006 — on failure it returns [`Outcome::Failed`] (never panics /
///     unwinds across shards); recovery is via [`Compensation`], not rollback.
///
/// This is a *contract only* (I1 skeleton): no method bodies are provided.
pub trait Executor {
    /// Perform `plan`, cooperatively observing `cancel`, and return its [`Outcome`].
    ///
    /// The outcome's writes are proposed to the Kernel via an [`OutcomeRouter`]
    /// (ORCH-001); this method itself performs no commit.
    fn execute(&self, plan: &Plan, cancel: &CancellationToken) -> Outcome;

    /// Build the compensating action for a completed/partial invocation, per the
    /// saga model (Amendment-006). Returns `None` when the effect is inherently
    /// non-compensable (e.g. an already-idempotent read).
    fn compensate(&self, invocation: &InvocationId) -> Option<Compensation> {
        // Trivial, always-compiling default: no compensation known at this layer.
        let _ = invocation;
        None
    }
}

// ---------------------------------------------------------------------------
// Tests (Amendment-005 cooperative cancellation)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_token_is_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        // Default is equivalent to `new` (Amendment-005: starts not-cancelled).
        assert!(!CancellationToken::default().is_cancelled());
    }

    #[test]
    fn cancel_flips_the_flag() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_is_idempotent() {
        let token = CancellationToken::new();
        token.cancel();
        token.cancel();
        token.cancel();
        // Observing repeatedly is stable (Amendment-005: idempotent).
        assert!(token.is_cancelled());
        assert!(token.is_cancelled());
    }

    #[test]
    fn clone_shares_the_same_signal() {
        // A cloned token shares the underlying flag, so cancelling one is
        // observed by every holder (Amendment-005).
        let a = CancellationToken::new();
        let b = a.clone();
        assert!(!a.is_cancelled());
        assert!(!b.is_cancelled());
        b.cancel();
        assert!(a.is_cancelled());
        assert!(b.is_cancelled());
    }
}

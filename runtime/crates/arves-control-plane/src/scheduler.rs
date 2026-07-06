//! The CLUSTER CAPABILITY SCHEDULER (RCR-027, I4 Stage 2).
//!
//! Design basis: `docs/design/I4_Capability_Scheduling_Design.md` — §3.1.3 (placement),
//! §3.1.5 (per-shard admission control / backpressure), §3.1.6 (failure isolation),
//! §3.1.4 + §3.8 (deterministic/idempotent dispatch under the fabric-derived ORCH-004
//! key), §3.5 steps 2–8 (plan intake → authorize → admit → place → dispatch → execute
//! → commit), §3.6 (three state classes — the scheduler's own state is EPHEMERAL and
//! discardable), §3.7 (compute anywhere, commit through the shard leader; leader loss
//! discards in-flight work, re-dispatch converges by content addressing).
//!
//! Per the design's Terminology note, this "scheduler" is a composite of three
//! frozen-spec roles and NEVER a new component or layer: the **Capability Planner**
//! (selection input — here the caller's plan supplies the capability id; the module
//! selects nothing beyond binding resolution), the **Execution Planner** (placement,
//! [`PlacementBasis`]) and the **mechanical Data-Plane dispatch runtime** (queueing,
//! retry, admission — Vol 9 Part 12). It lives in `arves-control-plane` because the
//! DECISIONS (admission, placement, retry, quarantine) are Control-Plane concerns
//! (Vol 9 Parts 3–4); the design's OQ-2 code-home question is resolved to option (a)
//! — behaviour lands additively beside the contract-only crate (RCR-027 DR-1).
//!
//! ## Determinism (the Stage-2 hard rule)
//!
//! Every scheduling DECISION is a pure function of `(recorded state, seed, tick)` plus
//! the environment observations the scheduler is HANDED and RECORDS (leader identity,
//! node presence, gateway/kernel verdicts). There are no wall clocks, no OS randomness,
//! no ambient inputs: time is the caller-injected logical tick, placement spread is a
//! seeded FNV-1a fold ([`ClusterScheduler`] seed), and every decision is appended to an
//! in-memory decision log so two identically-scripted runs produce bit-identical
//! transcripts (proven in `tests/cluster_scheduling.rs`).
//!
//! ## Invariant posture (registered-normative only)
//!
//! - **ORCH-001** — the scheduler owns NO truth. Its only path to truth is routing a
//!   `ProposedWrite` through the frozen `Kernel::commit` gateway bound to the shard
//!   leader (`ClusterKernel`, RCR-021). Nothing scheduler-local is authoritative;
//!   discarding the scheduler loses zero committed truth (proven).
//! - **ORCH-002** — queues, the key ledger and the decision log are EPHEMERAL plan
//!   bookkeeping: discardable at any moment and reconstructible by re-submitting the
//!   plan (completed work converges by Kernel dedupe, never forks — proven by the
//!   crash-rebuild test).
//! - **ORCH-003** — a retried invocation is re-dispatched from its RECORDED
//!   [`Inference`] (`WorkState::Computed`), never recomputed: replay from record, not
//!   recomputation. The decision log records every selection/placement/retry verdict.
//!   (WAL-trace emission of scheduling decisions is OQ-10 and NOT built — the log here
//!   is in-memory observability, honestly not the durable decision trace.)
//! - **ORCH-004** — the unit of identity at the SCHEDULING surface is the
//!   fabric-derived content-addressable key (RCR-012 [`invocation_key`]) QUALIFIED by
//!   the capability id and structurally PARTITIONED by the immutable shard (DR-13):
//!   duplicate submission collapses at the ledger ONLY for a true duplicate (same
//!   shard, same capability, same manifest+input content); duplicate/racing dispatch
//!   converges to at-most-one committed truth at the Kernel (`AlreadyCommitted` /
//!   RCR-005 fork refusal), whose truth-level identity remains pure per-shard
//!   content addressing (the frozen `InvocationKey` contract is untouched).
//! - **SHARD-001** — queues, admission bounds, the work LEDGER and placement are
//!   scoped per immutable `ShardId`; flooding one shard never consumes another
//!   shard's admission budget, and dedupe can never let one shard's record silently
//!   suppress another shard's work.
//! - **OWN-001 / LAYER-001** — the scheduler owns only its ephemeral schedule; all five
//!   dependency edges point downward (control-plane 90 → capability 70 / engine 60 /
//!   kernel 40 / consensus 30 / acs 15; architecture gate green).
//!
//! ## IDR posture
//!
//! - **IDR-001** — compute is placed anywhere ([`PlacementBasis::ComputeAnywhere`] for
//!   `Pure` invocations); every commit is routed through the target shard's Raft
//!   leader. Commit-bearing invocations use **shard-leader affinity**
//!   ([`PlacementBasis::LeaderAffinity`]) as the Stage-2 REFERENCE placement policy —
//!   an engineering choice this stage records (DR-2), explicitly non-normative until
//!   the design's IDR-007 instrument ratifies a placement/backpressure policy.
//!   Node-presence input is AP by spec (possibly stale) and never gates a commit.
//! - **IDR-002** — the scheduler replicates nothing itself; committed OUTCOMES flow to
//!   followers through the I2 cluster kernel.
//! - **IDR-004** — in-flight uncommitted work is discardable: quorum loss / leadership
//!   loss surfaces as a retriable verdict, the item re-queues, and re-dispatch under
//!   the same key converges (content addressing dedupes at the Kernel).
//! - **IDR-005** — the Raft log = WAL = decision trace remains the Kernel/consensus
//!   substrate's; the scheduler adds no second durable log (its decision log is
//!   in-memory and disposable; durable trace granularity is OQ-10).
//!
//! ## HONEST SCOPE
//!
//! In-process simulation over the I2 `ClusterSim` vehicle: no network, no remote
//! execution — "placement" assigns a deterministic node LABEL and the engine compute
//! runs in-process on that node's behalf (every node hosts the same content-addressed
//! artifact set via [`EngineHost`]). At-least-once compute / at-most-once TRUTH is the
//! model (design §6.1: no exactly-once dispatch machinery); a capability's EXTERNAL
//! side effect still relies on its declared `EffectClass` honesty (design §3.1.4
//! caveat; v1.1 debt #2). No saga/cross-shard flows (OQ-4), no HITL sequencing, no
//! distributed cancellation (OQ-6), no durable trace emission (OQ-10), no
//! authN/authZ (v1.0 trusted single host). CAP-001..009 remain PROPOSED and are not
//! enforced.

use arves_capability_fabric::gate::{self, PolicyVerdict};
use arves_capability_fabric::{
    CapabilityId, CapabilityRegistry, EffectClass, ProviderId, ShardKey as FabricShardKey,
};
use arves_consensus::{NodeId, ShardId};
use arves_engine_fabric::{invocation_key, Engine, EngineManifest, IdempotencyKey, Inference};
use arves_kernel::cluster::{ClusterKernel, ClusterSim};
use arves_kernel::{CommitError, ContentHash, Kernel, ProposedWrite, ShardKey as KernelShardKey};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Shard-key bridges (SHARD-001: one immutable (tenant, workspace) identity in
// three frozen crates' shapes — same discipline as arves-kernel::cluster).
// ---------------------------------------------------------------------------

fn fabric_shard(s: &ShardId) -> Result<FabricShardKey, String> {
    FabricShardKey::new(s.tenant.0.clone(), s.workspace.0.clone()).map_err(|e| e.to_string())
}

fn kernel_shard(s: &ShardId) -> Result<KernelShardKey, String> {
    KernelShardKey::new(s.tenant.0.clone(), s.workspace.0.clone()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Engine hosting (the in-process stand-in for "every node runs the same
// content-addressed artifact set", Engine Graph Part 9).
// ---------------------------------------------------------------------------

/// The provider → engine table the dispatch path executes against. In this
/// in-process Stage-2 harness ONE host table stands for the artifact set every
/// cluster node carries (Engine Graph Part 9: a runtime needs only the manifest
/// to schedule; the artifact is content-addressed and identical everywhere) —
/// the placement decision records WHICH node label the compute is attributed
/// to. The host owns engine VALUES, never bindings (OWN-001: bindings stay the
/// registry's) and never truth.
#[derive(Default)]
pub struct EngineHost {
    engines: BTreeMap<ProviderId, Box<dyn Engine<Input = Vec<u8>>>>,
}

impl EngineHost {
    /// Empty host.
    pub fn new() -> Self {
        Self::default()
    }

    /// Host an engine under its manifest-derived provider identity
    /// (`engine:{name}@{version}` — the single fabric scheme, RCR-026).
    /// Returns the [`ProviderId`] a binding must name for the gate's
    /// engine-identity check to pass.
    pub fn host(&mut self, engine: Box<dyn Engine<Input = Vec<u8>>>) -> ProviderId {
        let pid = gate::engine_provider_id(&engine.manifest());
        self.engines.insert(pid.clone(), engine);
        pid
    }

    /// Look up the hosted engine for a bound provider (read-only).
    pub fn engine(&self, provider: &ProviderId) -> Option<&dyn Engine<Input = Vec<u8>>> {
        self.engines.get(provider).map(|b| b.as_ref())
    }
}

/// Adapter so a `&dyn Engine` can flow through the generic Stage-1 gate
/// (`gate::invoke_gated` takes `E: Engine<Input = Vec<u8>>` by value-bound
/// generic; this wrapper is that `E`).
struct HostedEngine<'a>(&'a dyn Engine<Input = Vec<u8>>);

impl Engine for HostedEngine<'_> {
    type Input = Vec<u8>;
    fn manifest(&self) -> EngineManifest {
        self.0.manifest()
    }
    fn invoke(&self, input: Vec<u8>) -> Inference {
        self.0.invoke(input)
    }
}

// ---------------------------------------------------------------------------
// Scheduler configuration, inputs and recorded decisions.
// ---------------------------------------------------------------------------

/// Deterministic scheduler bounds. All limits are logical (counts, never time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// Max invocations queued per shard — the per-shard admission bound
    /// (backpressure, design §3.1.5; axis-8 tenant isolation). Admission beyond
    /// it is DENIED, never silently dropped (F-OVERLOAD).
    pub shard_capacity: usize,
    /// Max retriable commit failures (leader loss / quorum loss) per invocation
    /// before quarantine (F-POISON boundary at the commit seam).
    pub retry_budget: u32,
    /// Max queue heads dispatched per shard per tick (fairness bound; shards
    /// are always serviced independently of each other — SHARD-001).
    pub dispatch_per_tick: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { shard_capacity: 8, retry_budget: 3, dispatch_per_tick: 1 }
    }
}

/// One capability invocation as the caller's PLAN supplies it (selection of
/// WHICH capability is the Capability Planner's input — the scheduler never
/// invents it; Vol 9 Part 3). The policy verdict is a Governance-owned input,
/// enforced-not-owned (Vol 9 Part 10; RCR-026 DR-5).
#[derive(Clone, Debug)]
pub struct InvocationSpec {
    /// The (immutable) shard this invocation is confined to (SHARD-001).
    pub shard: ShardId,
    /// The capability to invoke (resolved via the fabric registry).
    pub capability: CapabilityId,
    /// Governance policy verdict for this invocation (enforced, never computed).
    pub policy: PolicyVerdict,
    /// Canonical input bytes (the ORCH-004 key is derived from these + the
    /// bound engine's manifest identity, RCR-012).
    pub input: Vec<u8>,
}

/// Everything the scheduler OBSERVES, handed in explicitly so no ambient input
/// exists. `down` is scripted AP presence information (IDR-001 CP/AP table:
/// capability/presence statistics are AP) — it may be stale and NEVER gates a
/// commit; a stale entry only costs a deferral/retry.
pub struct DispatchEnv<'a, R: CapabilityRegistry> {
    /// The I2 cluster (RCR-021 `ClusterSim`): shard directory, leader identity,
    /// and the leader-only commit gateway.
    pub cluster: &'a Rc<RefCell<ClusterSim>>,
    /// The capability registry (Stage-1 fabric core or the frozen `MemRegistry`).
    pub registry: &'a R,
    /// The hosted engine artifact set (identical on every node — honest scope).
    pub host: &'a EngineHost,
    /// Scripted node-presence input: nodes currently believed down (AP).
    pub down: &'a BTreeSet<NodeId>,
}

/// Why a node was chosen for an invocation's compute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementBasis {
    /// Commit-bearing invocation placed on the shard's Raft leader (Stage-2
    /// reference policy, DR-2): the proposed writes commit through that leader
    /// anyway (IDR-001), so affinity saves a routing hop. NON-NORMATIVE until
    /// IDR-007 ratifies a placement policy.
    LeaderAffinity,
    /// `Pure` invocation placed on any healthy node by the seeded deterministic
    /// spread (IDR-001: compute anywhere; consensus untouched).
    ComputeAnywhere,
}

/// One recorded scheduling decision. The Vec of these IS the scheduler's
/// deterministic transcript: a pure function of (recorded state, seed, tick,
/// recorded observations). In-memory observability only — durable decision-
/// trace emission stays OQ-10 (honest scope).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchedulingDecision {
    /// A submission was refused before queueing (malformed/unrouted shard,
    /// F-UNBOUND hard deny, unhosted provider).
    SubmitDenied { tick: u64, shard: ShardId, capability: String, reason: String },
    /// Admitted into the shard queue under its ORCH-004 key.
    Admitted { tick: u64, shard: ShardId, key: IdempotencyKey },
    /// The same scheduler-level identity (shard-partitioned, capability-
    /// qualified ORCH-004 key — DR-13) is already known IN THIS SHARD — the
    /// duplicate collapses onto the existing work (idempotent submission;
    /// the engine is NOT re-invoked). Retriable-class quarantined work
    /// re-admits instead of collapsing (DR-4).
    Deduplicated { tick: u64, shard: ShardId, key: IdempotencyKey },
    /// Per-shard admission bound hit — backpressure denial (F-OVERLOAD); other
    /// shards are unaffected (SHARD-001).
    AdmissionDenied { tick: u64, shard: ShardId, key: IdempotencyKey, depth: usize },
    /// Compute placed on a node (the placement plan artifact — ORCH-002:
    /// serializable, discardable).
    Placed { tick: u64, shard: ShardId, key: IdempotencyKey, node: NodeId, basis: PlacementBasis },
    /// No dispatch possible this tick (no reachable leader for a commit-bearing
    /// invocation / no healthy compute node) — the item stays queued; deferral
    /// is not a retry and spends no retry budget.
    Deferred { tick: u64, shard: ShardId, key: IdempotencyKey, reason: String },
    /// The Stage-1 authorization gate refused (deterministic denial → quarantine).
    GateDenied { tick: u64, shard: ShardId, key: IdempotencyKey, denial: String },
    /// The engine computed an [`Inference`] under fabric enforcement (RCR-012);
    /// its effects are recorded as PROPOSALS only.
    Computed { tick: u64, shard: ShardId, key: IdempotencyKey, node: NodeId, proposed_effects: usize },
    /// A retried invocation resumed from its RECORDED inference — replay from
    /// record, not recomputation (ORCH-003): the engine was NOT re-invoked.
    ReplayedFromRecord { tick: u64, shard: ShardId, key: IdempotencyKey },
    /// One proposed effect became (or idempotently resolved to) committed truth
    /// through the shard leader's Kernel gateway. `deduped` = the Kernel
    /// answered `AlreadyCommitted` (ORCH-004 convergence, never a fork).
    Committed {
        tick: u64,
        shard: ShardId,
        key: IdempotencyKey,
        effect_index: usize,
        commit_index: u64,
        deduped: bool,
    },
    /// A retriable commit failure (leadership/quorum loss — IDR-004: in-flight
    /// work is discardable); the item re-queued under the SAME key.
    CommitUnavailable { tick: u64, shard: ShardId, key: IdempotencyKey, error: String, retries: u32 },
    /// Refusal: deterministic denial (terminal — dedupe collapses onto it) or
    /// exhausted retry budget (retriable-class — a later re-submission
    /// re-admits with a fresh budget, DR-4). Either way the item leaves the
    /// queue so it can never wedge the shard (F-POISON).
    Quarantined { tick: u64, shard: ShardId, key: IdempotencyKey, reason: String },
    /// All proposed effects landed (or none existed): the invocation is done.
    Done { tick: u64, shard: ShardId, key: IdempotencyKey, commits: usize },
}

/// Outcome of a [`ClusterScheduler::submit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// Queued under its ORCH-004 key.
    Admitted { key: IdempotencyKey },
    /// Collapsed onto already-known work with the same shard-scoped,
    /// capability-qualified key (ORCH-004 identity, DR-13). The collapsed-
    /// onto work may be live OR a DETERMINISTIC refusal (inspect
    /// [`ClusterScheduler::state_of`] to distinguish); retriable-class
    /// quarantined work re-admits instead of deduplicating (DR-4).
    Deduplicated { key: IdempotencyKey },
    /// Refused by the per-shard admission bound (backpressure; retriable later).
    AdmissionDenied { key: IdempotencyKey },
    /// Refused before queueing (see [`SchedulingDecision::SubmitDenied`]).
    Denied { reason: String },
}

/// Per-key recorded work state — the scheduler's EPHEMERAL ledger (ORCH-002:
/// discardable; rebuilt by re-submitting the plan, converging at the Kernel).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkState {
    /// Admitted, not yet computed.
    Queued,
    /// Computed under fabric enforcement; `effects_committed` proposals have
    /// already landed. Retry resumes HERE (replay from record, ORCH-003).
    Computed { inference: Inference, effects_committed: usize },
    /// Terminal: all effects committed (or none proposed).
    Done { commits: usize },
    /// Refused. `retriable: false` = DETERMINISTIC refusal (gate denial,
    /// content-integrity, unbound/unhosted at dispatch): re-running a pure
    /// function of the same state reproduces it, so dedupe lawfully collapses
    /// later duplicates onto the refusal — terminal within this scheduler.
    /// PRECISION (RCR-028 adversarial revision): "the same state" spans the
    /// gate's inputs INCLUDING the caller-supplied Governance `PolicyVerdict`,
    /// which is NOT encoded in the ORCH-004 dedupe key — so a later
    /// re-submission of the same (shard, capability, input) under a FLIPPED
    /// verdict (`Deny` → `Allow`) still collapses onto the recorded refusal
    /// for THIS scheduler's lifetime (the refusal stays visible via
    /// [`ClusterScheduler::state_of`]). A policy flip takes effect through a
    /// fresh scheduler (ORCH-002: schedules are discardable plan artifacts),
    /// which re-admits and completes the work. Pinned by
    /// `policy_flip_resubmission_collapses_in_scheduler_lifetime_fresh_scheduler_readmits`;
    /// re-admit-on-differing-verdict is deferred to the IDR-007 instrument.
    /// `retriable: true` = the commit RETRY BUDGET was exhausted under
    /// quorum/leadership loss (DR-4): a later re-submission of the same
    /// invocation RE-ADMITS it with a fresh budget instead of collapsing —
    /// a transient outage never becomes a permanent refusal.
    Quarantined { reason: String, retriable: bool },
}

struct QueuedWork {
    key: IdempotencyKey,
    capability: CapabilityId,
    policy: PolicyVerdict,
    input: Vec<u8>,
    retries: u32,
}

// ---------------------------------------------------------------------------
// The deterministic placement spread (pure function; no OS randomness).
// ---------------------------------------------------------------------------

/// Seeded FNV-1a fold over the invocation key, mixed with the logical tick —
/// a pure function of `(seed, tick, key, n)`, so placement is replayable and a
/// deferred item may lawfully re-place on a later tick (escaping a down node)
/// while remaining deterministic.
/// The SCHEDULER-LEVEL identity of one invocation (DR-13): the fabric-derived
/// content-addressable key (RCR-012 — engine manifest identity + canonical
/// input) QUALIFIED by the capability id, length-prefixed so the encoding is
/// injective (no crafted capability name can alias another pair). The bare
/// fabric key is manifest+input only, so without this qualification two
/// DIFFERENT capabilities bound to the same provider would collapse onto one
/// work item — skipping the second one's policy verdict. The SHARD partition
/// is structural, not encoded here: the ledger key is `(ShardId, key)`
/// (design §3.6 — the immutable shard key partitions everything). The
/// KERNEL's truth-level identity remains pure per-shard content addressing;
/// this qualification exists only at the scheduling surface.
fn scheduler_key(capability: &CapabilityId, fabric: &IdempotencyKey) -> IdempotencyKey {
    IdempotencyKey(format!("{}:{}::{}", capability.0.len(), capability.0, fabric.0))
}

fn placement_index(seed: u64, tick: u64, key: &IdempotencyKey, n: usize) -> usize {
    debug_assert!(n > 0);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ seed ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for &b in key.0.as_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    (h % n as u64) as usize
}

// ---------------------------------------------------------------------------
// The scheduler.
// ---------------------------------------------------------------------------

/// The Stage-2 cluster capability scheduler. See the module doc for the exact
/// posture; in one line: **decides deterministically, executes through the
/// Stage-1 gate, commits only through the shard leader's Kernel, owns nothing
/// durable.**
pub struct ClusterScheduler {
    config: SchedulerConfig,
    seed: u64,
    queues: BTreeMap<ShardId, VecDeque<QueuedWork>>,
    /// The EPHEMERAL work ledger, structurally partitioned by the immutable
    /// `ShardId` (SHARD-001 — dedupe can never collapse work across shards)
    /// and keyed within a shard by the capability-qualified fabric key
    /// ([`scheduler_key`], DR-13).
    ledger: BTreeMap<(ShardId, IdempotencyKey), WorkState>,
    decisions: Vec<SchedulingDecision>,
}

impl ClusterScheduler {
    /// A fresh scheduler. `seed` is the entire randomness budget (recorded ⇒
    /// replayable); `config` fixes the logical bounds.
    pub fn new(seed: u64, config: SchedulerConfig) -> Self {
        Self {
            config,
            seed,
            queues: BTreeMap::new(),
            ledger: BTreeMap::new(),
            decisions: Vec::new(),
        }
    }

    // -- read-only introspection ---------------------------------------------

    /// The full recorded decision log, in append order.
    pub fn decisions(&self) -> &[SchedulingDecision] {
        &self.decisions
    }

    /// The transcript: one deterministic line per decision (the bit-identity
    /// instrument of the determinism proof — RCR-025 DR-6 pattern).
    pub fn transcript(&self) -> Vec<String> {
        self.decisions.iter().map(|d| format!("{d:?}")).collect()
    }

    /// Recorded state of one invocation key WITHIN one shard, if known (the
    /// ledger is shard-partitioned — DR-13; the same key string in another
    /// shard is independent work).
    pub fn state_of(&self, shard: &ShardId, key: &IdempotencyKey) -> Option<&WorkState> {
        self.ledger.get(&(shard.clone(), key.clone()))
    }

    /// Current queue depth of one shard.
    pub fn queue_depth(&self, shard: &ShardId) -> usize {
        self.queues.get(shard).map_or(0, |q| q.len())
    }

    /// True when no work is queued anywhere.
    pub fn is_idle(&self) -> bool {
        self.queues.values().all(|q| q.is_empty())
    }

    // -- submission (plan intake → authorize-lite → admit; design §3.5 2–4) ---

    /// Submit one invocation. Fixed, documented check order (deterministic):
    ///
    /// 1. shard key well-formed in both frozen shapes + routed (a registered
    ///    Raft group exists — SHARD-001: no route, no fallback);
    /// 2. an active binding resolves in the shard (F-UNBOUND hard deny BEFORE
    ///    queueing — the full gate re-authorizes at dispatch);
    /// 3. the bound provider is hosted;
    /// 4. the SCHEDULER-LEVEL identity — the fabric-derived ORCH-004 key
    ///    ([`invocation_key`], RCR-012) qualified by the capability id and
    ///    partitioned by the immutable shard ([`scheduler_key`], DR-13) —
    ///    dedupes against the SHARD's recorded work: a true duplicate
    ///    collapses (the engine is not re-invoked), while identical input
    ///    under another shard or another capability is INDEPENDENT work;
    ///    a RETRIABLE-class quarantined key (commit retry budget exhausted,
    ///    DR-4) RE-ADMITS with a fresh budget instead of collapsing onto
    ///    the refusal;
    /// 5. the per-shard admission bound admits or DENIES (backpressure,
    ///    F-OVERLOAD; denial is visible and retriable, never a silent drop).
    pub fn submit<R: CapabilityRegistry>(
        &mut self,
        tick: u64,
        spec: InvocationSpec,
        env: &DispatchEnv<'_, R>,
    ) -> SubmitOutcome {
        // (1) shard well-formed + routed.
        let fshard = match fabric_shard(&spec.shard) {
            Ok(s) => s,
            Err(e) => return self.deny_submit(tick, &spec, format!("malformed shard key: {e}")),
        };
        if let Err(e) = kernel_shard(&spec.shard) {
            return self.deny_submit(tick, &spec, format!("malformed shard key: {e}"));
        }
        if !env.cluster.borrow().shards().contains(&spec.shard) {
            return self.deny_submit(tick, &spec, "unrouted shard: no Raft group registered".into());
        }
        // (2) F-UNBOUND before queueing.
        let binding = match env.registry.resolve(&fshard, &spec.capability) {
            Ok(b) => b,
            Err(e) => return self.deny_submit(tick, &spec, format!("unbound: {e:?}")),
        };
        // (3) the bound provider must be executable here.
        let engine = match env.host.engine(&binding.provider) {
            Some(e) => e,
            None => {
                return self
                    .deny_submit(tick, &spec, format!("provider not hosted: {:?}", binding.provider))
            }
        };
        // (4) ORCH-004 identity + dedupe, scoped to (shard, capability) —
        // DR-13: the ledger key is structurally partitioned by the immutable
        // ShardId and the fabric key is capability-qualified, so dedupe
        // collapses ONLY true duplicates within one shard. A retriable-class
        // quarantine (budget exhausted under quorum loss, DR-4) re-admits
        // with a fresh budget; deterministic refusals stay collapsed
        // (retrying a pure function of the same state reproduces them).
        let key = scheduler_key(&spec.capability, &invocation_key(&engine.manifest(), &spec.input));
        let lkey = (spec.shard.clone(), key.clone());
        match self.ledger.get(&lkey) {
            Some(WorkState::Quarantined { retriable: true, .. }) => {
                // Fall through to admission: re-queue with a fresh budget
                // (the quarantined ledger entry is replaced on admission).
            }
            Some(_) => {
                self.decisions.push(SchedulingDecision::Deduplicated {
                    tick,
                    shard: spec.shard.clone(),
                    key: key.clone(),
                });
                return SubmitOutcome::Deduplicated { key };
            }
            None => {}
        }
        // (5) per-shard admission bound (backpressure).
        let depth = self.queue_depth(&spec.shard);
        if depth >= self.config.shard_capacity {
            self.decisions.push(SchedulingDecision::AdmissionDenied {
                tick,
                shard: spec.shard.clone(),
                key: key.clone(),
                depth,
            });
            return SubmitOutcome::AdmissionDenied { key };
        }
        self.queues.entry(spec.shard.clone()).or_default().push_back(QueuedWork {
            key: key.clone(),
            capability: spec.capability.clone(),
            policy: spec.policy,
            input: spec.input.clone(),
            retries: 0,
        });
        self.ledger.insert(lkey, WorkState::Queued);
        self.decisions.push(SchedulingDecision::Admitted {
            tick,
            shard: spec.shard.clone(),
            key: key.clone(),
        });
        SubmitOutcome::Admitted { key }
    }

    fn deny_submit(&mut self, tick: u64, spec: &InvocationSpec, reason: String) -> SubmitOutcome {
        self.decisions.push(SchedulingDecision::SubmitDenied {
            tick,
            shard: spec.shard.clone(),
            capability: spec.capability.0.clone(),
            reason: reason.clone(),
        });
        SubmitOutcome::Denied { reason }
    }

    // -- dispatch (place → gate → commit; design §3.5 steps 5–8) --------------

    /// Run one logical scheduling tick: for every shard (deterministic order),
    /// dispatch up to `dispatch_per_tick` queue heads. A deferred/retrying head
    /// re-queues at the FRONT and ends that shard's tick (per-shard FIFO is the
    /// Stage-2 ordering discipline — DR-5); shards never block each other
    /// (SHARD-001 failure isolation).
    pub fn dispatch_tick<R: CapabilityRegistry>(&mut self, tick: u64, env: &DispatchEnv<'_, R>) {
        let shards: Vec<ShardId> = self.queues.keys().cloned().collect();
        for shard in shards {
            let mut dispatched = 0usize;
            while dispatched < self.config.dispatch_per_tick {
                let work = match self.queues.get_mut(&shard).and_then(|q| q.pop_front()) {
                    Some(w) => w,
                    None => break,
                };
                dispatched += 1;
                if let Some(back) = self.dispatch_one(tick, &shard, work, env) {
                    self.queues.get_mut(&shard).expect("queue exists").push_front(back);
                    break; // head blocked this tick — retry next tick, FIFO kept
                }
            }
        }
    }

    /// Dispatch one work item. Returns `Some(work)` iff it must re-queue at the
    /// shard's front (deferral or retriable commit failure).
    fn dispatch_one<R: CapabilityRegistry>(
        &mut self,
        tick: u64,
        shard: &ShardId,
        mut work: QueuedWork,
        env: &DispatchEnv<'_, R>,
    ) -> Option<QueuedWork> {
        let key = work.key.clone();
        let lkey = (shard.clone(), key.clone());
        let already_computed = matches!(self.ledger.get(&lkey), Some(WorkState::Computed { .. }));

        if !already_computed {
            match self.ledger.get(&lkey) {
                Some(WorkState::Queued) => {}
                // A popped queue item MUST be `Queued` (or `Computed`, handled
                // above): `submit` is the only enqueue path and it records
                // `Queued`; terminal states are never re-queued (dedupe
                // collapses them; quarantine removes the item from the queue).
                // Silently dropping here would be exactly the class the
                // backpressure ACCOUNTING proof declares impossible, so this
                // defensive arm fails LOUDLY instead of dropping (RCR-028
                // adversarial revision — no decision-less queue removal exists).
                state => unreachable!(
                    "popped queue item {key:?} has non-dispatchable ledger state \
                     {state:?} — enqueue/ledger accounting violated"
                ),
            }
            // -- compute phase: place, then execute through the Stage-1 gate --
            let fshard = fabric_shard(shard).expect("shard validated at submit");
            // Re-resolve at dispatch: a rebind/revoke lawfully bites every
            // SUBSEQUENT dispatch (RCR-026 DR-10); the gate pins the version.
            let binding = match env.registry.resolve(&fshard, &work.capability) {
                Ok(b) => b,
                Err(e) => {
                    self.quarantine(tick, shard, &key, format!("unbound at dispatch: {e:?}"), false);
                    return None;
                }
            };
            let engine = match env.host.engine(&binding.provider) {
                Some(e) => e,
                None => {
                    self.quarantine(
                        tick,
                        shard,
                        &key,
                        format!("provider not hosted: {:?}", binding.provider),
                        false,
                    );
                    return None;
                }
            };
            let commit_bearing = binding.contract.effect != EffectClass::Pure;
            // Observation snapshot (leader = routing input; down = AP presence).
            let (leader, healthy) = {
                let c = env.cluster.borrow();
                let leader = c.leader_of(shard);
                let healthy: Vec<NodeId> = c
                    .node_ids()
                    .into_iter()
                    .filter(|n| !env.down.contains(n))
                    .collect();
                (leader, healthy)
            };
            let (node, basis) = if commit_bearing {
                match leader {
                    Some(l) if !env.down.contains(&l) => (l, PlacementBasis::LeaderAffinity),
                    _ => {
                        self.decisions.push(SchedulingDecision::Deferred {
                            tick,
                            shard: shard.clone(),
                            key,
                            reason: "no reachable shard leader for a commit-bearing invocation"
                                .into(),
                        });
                        return Some(work);
                    }
                }
            } else {
                if healthy.is_empty() {
                    self.decisions.push(SchedulingDecision::Deferred {
                        tick,
                        shard: shard.clone(),
                        key,
                        reason: "no healthy compute node".into(),
                    });
                    return Some(work);
                }
                let i = placement_index(self.seed, tick, &key, healthy.len());
                (healthy[i].clone(), PlacementBasis::ComputeAnywhere)
            };
            self.decisions.push(SchedulingDecision::Placed {
                tick,
                shard: shard.clone(),
                key: key.clone(),
                node: node.clone(),
                basis,
            });
            // Execute through the Stage-1 gate: authorize (binding + identity +
            // capabilities-required + policy) → invoke_enforced (RCR-012 key +
            // determinism probe) → EffectClass validation. Effects come back as
            // PROPOSALS only (ORCH-001).
            let gated = match gate::invoke_gated(
                env.registry,
                &fshard,
                &work.capability,
                work.policy,
                &HostedEngine(engine),
                work.input.clone(),
            ) {
                Ok(g) => g,
                Err(denial) => {
                    self.decisions.push(SchedulingDecision::GateDenied {
                        tick,
                        shard: shard.clone(),
                        key: key.clone(),
                        denial: format!("{denial:?}"),
                    });
                    self.quarantine(tick, shard, &key, "gate denial (deterministic)".into(), false);
                    return None;
                }
            };
            self.decisions.push(SchedulingDecision::Computed {
                tick,
                shard: shard.clone(),
                key: key.clone(),
                node,
                proposed_effects: gated.inference.proposed_effects.len(),
            });
            self.ledger.insert(
                lkey.clone(),
                WorkState::Computed { inference: gated.inference, effects_committed: 0 },
            );
        } else {
            // ORCH-003: the retry resumes from the RECORDED inference — the
            // engine is NOT re-invoked by a retry within this scheduler's
            // recorded state.
            self.decisions.push(SchedulingDecision::ReplayedFromRecord {
                tick,
                shard: shard.clone(),
                key: key.clone(),
            });
        }

        // -- commit phase: route every proposal through the shard leader ------
        let (inference, mut committed) = match self.ledger.get(&lkey) {
            Some(WorkState::Computed { inference, effects_committed }) => {
                (inference.clone(), *effects_committed)
            }
            _ => unreachable!("commit phase requires a Computed record"),
        };
        let total = inference.proposed_effects.len();
        if total == 0 {
            self.ledger.insert(lkey, WorkState::Done { commits: 0 });
            self.decisions.push(SchedulingDecision::Done {
                tick,
                shard: shard.clone(),
                key,
                commits: 0,
            });
            return None;
        }
        let kshard = kernel_shard(shard).expect("shard validated at submit");
        while committed < total {
            // The commit ALWAYS goes through the shard's current leader
            // (IDR-001), re-read per attempt so a failover is followed.
            let leader = {
                let c = env.cluster.borrow();
                c.leader_of(shard)
            };
            let leader = match leader {
                Some(l) if !env.down.contains(&l) => l,
                _ => {
                    self.decisions.push(SchedulingDecision::Deferred {
                        tick,
                        shard: shard.clone(),
                        key,
                        reason: "no reachable shard leader for commit routing".into(),
                    });
                    return Some(work);
                }
            };
            let payload = inference.proposed_effects[committed].payload.clone();
            let proposal = ProposedWrite {
                shard: kshard.clone(),
                content: ContentHash(arves_acs::content_id(
                    arves_acs::domain::COMMIT_CONTENT,
                    &payload,
                )),
                payload,
            };
            let verdict = ClusterKernel::new(leader, Rc::clone(env.cluster)).commit(proposal);
            match verdict {
                Ok(truth) => {
                    self.decisions.push(SchedulingDecision::Committed {
                        tick,
                        shard: shard.clone(),
                        key: key.clone(),
                        effect_index: committed,
                        commit_index: truth.index.0,
                        deduped: false,
                    });
                    committed += 1;
                    self.record_progress(&lkey, committed);
                }
                // ORCH-004 convergence: a duplicate/racing dispatch resolves to
                // the EXISTING truth — never a second execution's fork.
                Err(CommitError::AlreadyCommitted(truth)) => {
                    self.decisions.push(SchedulingDecision::Committed {
                        tick,
                        shard: shard.clone(),
                        key: key.clone(),
                        effect_index: committed,
                        commit_index: truth.index.0,
                        deduped: true,
                    });
                    committed += 1;
                    self.record_progress(&lkey, committed);
                }
                // IDR-004: leadership/quorum loss — in-flight work is
                // discardable and re-dispatchable under the same key.
                Err(e @ (CommitError::NotLeader { .. } | CommitError::NotReplicated)) => {
                    work.retries += 1;
                    if work.retries > self.config.retry_budget {
                        // RETRIABLE-class: exhaustion under quorum loss is a
                        // transient-world refusal — re-submission re-admits
                        // with a fresh budget (DR-4), never a permanent
                        // refusal inside a long-lived scheduler.
                        self.quarantine(
                            tick,
                            shard,
                            &key,
                            format!("commit retry budget exhausted after: {e}"),
                            true,
                        );
                        return None;
                    }
                    self.decisions.push(SchedulingDecision::CommitUnavailable {
                        tick,
                        shard: shard.clone(),
                        key,
                        error: format!("{e}"),
                        retries: work.retries,
                    });
                    return Some(work);
                }
                // Deterministic refusals (content-integrity fork, rejection,
                // unrouted shard) never retry — retrying reproduces them.
                Err(e) => {
                    self.quarantine(tick, shard, &key, format!("commit refused: {e}"), false);
                    return None;
                }
            }
        }
        self.ledger.insert(lkey, WorkState::Done { commits: total });
        self.decisions.push(SchedulingDecision::Done {
            tick,
            shard: shard.clone(),
            key,
            commits: total,
        });
        None
    }

    fn record_progress(&mut self, lkey: &(ShardId, IdempotencyKey), committed: usize) {
        if let Some(WorkState::Computed { effects_committed, .. }) = self.ledger.get_mut(lkey) {
            *effects_committed = committed;
        }
    }

    /// `retriable: true` ONLY for commit-retry-budget exhaustion under
    /// quorum/leadership loss (DR-4 — re-submission re-admits with a fresh
    /// budget); every deterministic refusal is `retriable: false`.
    fn quarantine(
        &mut self,
        tick: u64,
        shard: &ShardId,
        key: &IdempotencyKey,
        reason: String,
        retriable: bool,
    ) {
        self.ledger.insert(
            (shard.clone(), key.clone()),
            WorkState::Quarantined { reason: reason.clone(), retriable },
        );
        self.decisions.push(SchedulingDecision::Quarantined {
            tick,
            shard: shard.clone(),
            key: key.clone(),
            reason,
        });
    }
}

#[cfg(test)]
mod scheduler_unit_tests {
    use super::*;

    /// Placement is a pure function of (seed, tick, key, n): stable across
    /// calls, always in range, and sensitive to each input (no OS randomness,
    /// no clocks — the Stage-2 determinism rule).
    #[test]
    fn placement_index_is_a_pure_in_range_function_of_seed_tick_key() {
        let key = IdempotencyKey("acs-002/1:cap@1.0.0:abcd".into());
        for n in 1..=7usize {
            for tick in 0..5u64 {
                let a = placement_index(42, tick, &key, n);
                let b = placement_index(42, tick, &key, n);
                assert_eq!(a, b, "pure: same inputs, same placement");
                assert!(a < n, "in range");
            }
        }
        // Sensitivity (not a distribution claim): some input change moves the
        // placement for n large enough to observe it.
        let n = 5;
        let base = placement_index(42, 1, &key, n);
        let moved = (0..64u64).any(|t| placement_index(42, t, &key, n) != base)
            || (0..64u64).any(|s| placement_index(s, 1, &key, n) != base);
        assert!(moved, "placement depends on seed/tick, not a constant");
    }

    /// The shard-key bridges enforce the SAME well-formedness rules as the
    /// frozen kernel/fabric constructors (SHARD-001: degenerate keys refused).
    #[test]
    fn shard_key_bridges_refuse_degenerate_keys() {
        use arves_consensus::{TenantId, WorkspaceId};
        let bad = ShardId::new(TenantId(String::new()), WorkspaceId("w".into()));
        assert!(fabric_shard(&bad).is_err());
        assert!(kernel_shard(&bad).is_err());
        let good = ShardId::new(TenantId("t".into()), WorkspaceId("w".into()));
        assert!(fabric_shard(&good).is_ok());
        assert!(kernel_shard(&good).is_ok());
    }
}

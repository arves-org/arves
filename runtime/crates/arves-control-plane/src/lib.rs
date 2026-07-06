//! ARVES :: arves-control-plane
//!
//! Purpose: Orchestrator: owns the Plan/Engine Graph; owns no truth, no persistent state.
//! Governing: ORCH-001..004; Amendment-002 (plan ownership).
//! Layer: Control Plane
//!
//! STATUS: CONTRACT-ONLY (by design). Defines the Control Plane Plan / Engine
//! Graph interfaces and types; carries no orchestration execution logic (that is
//! deferred to the Multi-Agent Runtime, I5). Any `fn` bodies present are trivial
//! placeholders so the contract compiles. Frozen specification governs; this
//! crate implements, never changes it.
//!
//! STATUS since RCR-027 (I4 Stage 2, per `docs/design/I4_Capability_Scheduling_Design.md`):
//! the "CONTRACT-ONLY" wording above — and the "depends on nothing in this
//! workspace: it is a pure contract surface (std-only)" clause of the Layering
//! paragraph below — are superseded for the SCHEDULING surface only. This crate
//! now ALSO carries the I4 cluster capability scheduler ([`scheduler`]): placement
//! (shard-leader affinity for commit-bearing invocations, deterministic
//! compute-anywhere spread for `Pure` ones — IDR-001), per-shard admission
//! control/backpressure and failure isolation (SHARD-001), idempotent dispatch
//! under the fabric-derived ORCH-004 key (RCR-012) — shard-partitioned and
//! capability-qualified at the scheduling surface (RCR-027 DR-13; the frozen
//! [`InvocationKey`] contract below is untouched) — and commit routing exclusively
//! through the shard leader's Kernel gateway (`arves-kernel::cluster`, RCR-021).
//! Its five dependency edges are all DOWNWARD (LAYER-001: control-plane 90 →
//! capability 70 / engine 60 / kernel 40 / consensus 30 / acs 15; architecture
//! gate green). Every frozen v1.0 type and trait signature in this file is
//! byte-unchanged; the [`Orchestrator`] plan-graph contract remains contract-only
//! (I5), and the scheduler still owns no truth and no persistent state
//! (ORCH-001/002 — now executably proven at this surface, see
//! `tests/cluster_scheduling.rs`).
//!
//! # Position in the ARVES architecture
//!
//! The Control Plane *decides*; the Data Plane *carries*. This crate is the
//! decision-making half. It hosts the [`Orchestrator`]: the component that
//! compiles a request into a [`Plan`], materialises that plan as an
//! [`EngineGraph`], drives dynamic expansion of the graph, and joins the
//! results of competing branches via arbitration.
//!
//! # The two hard boundaries (ORCH-001, ORCH-002)
//!
//! Everything in this crate is designed around two prohibitions that the
//! type signatures are meant to make hard to violate:
//!
//! * **ORCH-001 - the Control Plane owns NO truth.** Only the Kernel owns
//!   truth (see `arves-kernel`; proposed G-001 names the Kernel the sole
//!   truth + commit gateway). The Orchestrator reads facts and writes
//!   *decisions*; it never becomes the authority for any cognitive state.
//!   OWN-001 (one owner per state) is what makes this unambiguous: the
//!   Orchestrator is simply not the owner of anything truth-bearing.
//! * **ORCH-002 - the Control Plane produces plans, never persistent
//!   state.** A [`Plan`] and its [`EngineGraph`] are values: they are derived
//!   from inputs and are discarded (or re-derived) at will. Nothing the
//!   Orchestrator holds is a durable store. Durability lives below, in
//!   Persistence (proposed PERSIST-001), reached only through the Kernel.
//!
//! Because the Orchestrator holds no persistent state, its recovery story is
//! **replay from a recorded decision trace, not recomputation** (ORCH-003).
//! The Raft log doubles as the WAL and the decision trace
//! (IDR-001 (Engineering Refinement)), so a
//! resumed Orchestrator reconstructs where it was by reading committed
//! [`DecisionRecord`]s rather than re-running engines. See [`DecisionTrace`].
//!
//! Finally, every step the Orchestrator schedules must be safe to retry:
//! **every engine/capability invocation is idempotent + content-addressable**
//! (ORCH-004). That is expressed by [`InvocationKey`], the content address of
//! a node's inputs, which lets a replayed or duplicated invocation collapse
//! onto the same [`NodeId`]/outcome.
//!
//! # Layering (LAYER-001)
//!
//! The Control Plane sits alongside the downward-only layer stack
//! (Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query
//! -> Engine -> Capability -> Execution). This crate depends on nothing in
//! this workspace: it is a pure contract surface (std-only) so that the
//! dependency direction is never inverted. Concrete engine/capability/kernel
//! handles are injected through the trait parameters and associated types
//! defined here, satisfied by lower crates at composition time.
//!
//! # Amendment-002 (plan ownership)
//!
//! Amendment-002 fixes *where the plan lives*: the Orchestrator owns the
//! Plan/Engine Graph as a transient artefact of a single request/episode.
//! Ownership of the plan is NOT ownership of truth (ORCH-001) and is NOT
//! persistence (ORCH-002); the plan is the Orchestrator's private working
//! artefact, replayable from the trace but never itself authoritative.

#![forbid(unsafe_code)]

pub mod scheduler;

use std::collections::BTreeMap;
use std::fmt;

// ===========================================================================
// Identity & sharding
// ===========================================================================

/// Tenant/workspace shard key.
///
/// Governing: SHARD-001 (partition by tenant/workspace; key immutable),
/// IDR-001 (one Raft group per tenant/workspace). Every plan the Orchestrator
/// builds is scoped to exactly one shard, because commits happen only via that
/// shard's leader (IDR-001 (Engineering Refinement)) and there is no cross-shard
/// atomic commit - multi-shard work is a saga, not a transaction
/// (IDR-001 (Engineering Refinement)).
///
/// The key is immutable once assigned (SHARD-001); this type is therefore
/// treated as an opaque, comparable value with no mutators.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShardKey(pub String);

impl fmt::Display for ShardKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shard:{}", self.0)
    }
}

/// Content address of a node's inputs (engine/capability, arguments, and the
/// versions of the facts it reads).
///
/// Governing: ORCH-004 (every engine/capability invocation is idempotent +
/// content-addressable). Two invocations with the same [`InvocationKey`] are
/// *the same invocation*: a replay (ORCH-003) or a duplicate delivery must
/// collapse onto one [`NodeId`] and one outcome. The key is expected to be a
/// cryptographic digest of the canonical encoding of the inputs; this skeleton
/// only fixes the shape.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvocationKey(pub [u8; 32]);

/// Stable identity of a node within a single [`EngineGraph`].
///
/// Derived from the node's [`InvocationKey`] so that content-addressing
/// (ORCH-004) makes node identity deterministic across replay (ORCH-003):
/// re-deriving the plan yields the same `NodeId`s for the same inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Monotonic index into the decision trace / Raft log.
///
/// Governing: ORCH-003, IDR-001 (Engineering Refinement) (Raft log = WAL =
/// decision trace). This is the
/// position at which a [`DecisionRecord`] was appended; replay proceeds in
/// ascending `TraceIndex` order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceIndex(pub u64);

// ===========================================================================
// Plan (Amendment-002; ORCH-002)
// ===========================================================================

/// A compiled, transient plan for a single request/episode.
///
/// Governing: Amendment-002 (the Orchestrator owns the plan), ORCH-002 (plans,
/// never persistent state). A `Plan` is a *value*: it names the shard it runs
/// against and the [`EngineGraph`] that realises it. It is never persisted by
/// the Control Plane; it can always be re-derived from inputs or replayed from
/// the [`DecisionTrace`] (ORCH-003).
#[derive(Clone, Debug)]
pub struct Plan {
    /// The shard this plan is confined to (SHARD-001,
    /// IDR-001 (Engineering Refinement): no cross-shard atomic commit).
    pub shard: ShardKey,
    /// The graph of engine/capability invocations that this plan expands into.
    pub graph: EngineGraph,
    /// The trace index this plan was (re)constructed from, if resumed. `None`
    /// for a freshly compiled plan. Used by ORCH-003 replay to know where to
    /// continue rather than recompute.
    pub resumed_from: Option<TraceIndex>,
}

// ===========================================================================
// Engine Graph
// ===========================================================================

/// The kind of work a graph node schedules.
///
/// The Orchestrator schedules two flavours of invocation, both governed by
/// ORCH-004 (idempotent + content-addressable):
/// * [`NodeKind::Engine`] - a pure, stateless engine (see `arves-engine-fabric`;
///   proposed ENG-001..005) that produces *inference*, not truth.
/// * [`NodeKind::Capability`] - an effectful capability (see
///   `arves-capability-fabric`; proposed CAP-001..009).
///
/// Neither kind may own truth (ORCH-001); results flow back to the Kernel,
/// which is the sole commit gateway (IDR-001 (Engineering Refinement): engines
/// run anywhere, commit only via the shard leader).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// A pure engine invocation. Stateless, replay-safe, produces inference.
    Engine {
        /// Opaque engine identifier resolved against the Engine ABI at
        /// composition time.
        engine: String,
    },
    /// An effectful capability invocation.
    Capability {
        /// Opaque capability identifier resolved against the Capability ABI.
        capability: String,
    },
}

/// Lifecycle state of a node during graph execution.
///
/// This is *ephemeral* execution bookkeeping (ORCH-002: not persistent state);
/// the authoritative record of what happened is the committed [`DecisionRecord`]
/// in the [`DecisionTrace`] (ORCH-003), not this field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeStatus {
    /// Waiting on unresolved inbound edges.
    Pending,
    /// Eligible to run (all inbound edges resolved).
    Ready,
    /// Dispatched to an engine/capability, awaiting outcome.
    Running,
    /// Completed; its committed outcome is available.
    Resolved,
    /// Pruned by arbitration (a competing branch won the join).
    Pruned,
}

/// A single vertex of the [`EngineGraph`].
///
/// Governing: ORCH-004 (`key` is the content address that makes the invocation
/// idempotent). A node carries no truth (ORCH-001) - it is a scheduling record
/// pointing at an engine/capability invocation.
#[derive(Clone, Debug)]
pub struct Node {
    /// Content-addressed identity of the invocation (ORCH-004).
    pub id: NodeId,
    /// The content address of this node's inputs (ORCH-004). `id` is derived
    /// from this.
    pub key: InvocationKey,
    /// What this node schedules.
    pub kind: NodeKind,
    /// Ephemeral lifecycle status (see [`NodeStatus`]).
    pub status: NodeStatus,
    /// Arbitration group this node participates in, if any. Nodes sharing a
    /// group are competing alternatives joined by an [`ArbitrationPolicy`].
    pub arbitration_group: Option<ArbitrationGroupId>,
}

/// Why one node depends on another.
///
/// Edges are directed and carry only *ordering/data-flow* meaning; they never
/// encode ownership of state (OWN-001) - they say "node A's outcome is an input
/// to node B", nothing more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// B consumes A's produced value.
    DataFlow,
    /// B must not start until A resolves, without consuming A's value.
    Ordering,
}

/// A directed edge `from -> to` in the [`EngineGraph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Source node (must resolve first).
    pub from: NodeId,
    /// Destination node (depends on the source).
    pub to: NodeId,
    /// Nature of the dependency.
    pub kind: EdgeKind,
}

/// Identity of an arbitration group within a graph.
///
/// An arbitration group is a set of competing branches whose outcomes are
/// reconciled by an [`ArbitrationPolicy`] at a join. See
/// [`EngineGraph::arbitrate`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArbitrationGroupId(pub u64);

/// How competing branches of an arbitration group are joined into a single
/// surviving outcome.
///
/// Arbitration is a *decision*, not truth (ORCH-001): the Orchestrator selects
/// which branch's outcome proceeds. The selection itself is recorded as a
/// [`DecisionRecord`] so it replays deterministically (ORCH-003) rather than
/// being recomputed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArbitrationPolicy {
    /// First branch to resolve wins; the rest are pruned.
    FirstResolved,
    /// A quorum of agreeing branches is required before the join resolves.
    Quorum {
        /// Minimum number of agreeing branches.
        min_agree: usize,
    },
    /// Highest-scoring branch wins. Scoring is supplied by the caller at
    /// composition time; this skeleton only names the policy.
    HighestScore,
    /// All branches must resolve and agree, else the join fails.
    Unanimous,
}

/// The Plan/Engine Graph owned by the Orchestrator (Amendment-002).
///
/// Governing: ORCH-001 (owns no truth), ORCH-002 (not persistent state),
/// ORCH-004 (nodes are content-addressed). The graph is a DAG of
/// engine/capability invocations. It supports:
///
/// * **Dynamic expansion** ([`EngineGraph::expand`]): a resolved node may add
///   new nodes/edges, so the graph grows as decisions are made. Because nodes
///   are content-addressed (ORCH-004), re-inserting an equivalent invocation is
///   idempotent - it collapses onto the existing [`NodeId`].
/// * **Arbitration join** ([`EngineGraph::arbitrate`]): competing branches in
///   an [`ArbitrationGroupId`] are reconciled by an [`ArbitrationPolicy`] into a
///   single surviving outcome; losers are marked [`NodeStatus::Pruned`].
///
/// The graph holds no durable state and no truth; it is a re-derivable working
/// artefact whose authoritative history lives in the [`DecisionTrace`].
#[derive(Clone, Debug, Default)]
pub struct EngineGraph {
    /// Nodes keyed by their content-addressed identity (ORCH-004).
    pub nodes: BTreeMap<NodeId, Node>,
    /// Directed dependency edges.
    pub edges: Vec<Edge>,
    /// Arbitration policy per group; consulted at the join.
    pub arbitration: BTreeMap<ArbitrationGroupId, ArbitrationPolicy>,
}

impl EngineGraph {
    /// A new, empty graph. No truth, no state (ORCH-001/002).
    pub fn new() -> Self {
        EngineGraph {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            arbitration: BTreeMap::new(),
        }
    }

    /// Insert a node, collapsing onto the existing entry if an equivalent
    /// (same [`NodeId`]) invocation is already present.
    ///
    /// Governing: ORCH-004. Because `NodeId` is derived from the content
    /// address of the inputs, re-inserting the same invocation is idempotent -
    /// this is what makes replay (ORCH-003) and duplicate delivery safe.
    /// Returns the id of the (possibly pre-existing) node.
    pub fn insert_node(&mut self, node: Node) -> NodeId {
        let id = node.id;
        self.nodes.entry(id).or_insert(node);
        id
    }

    /// Add a dependency edge. Contract-only in this skeleton.
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }
}

// ===========================================================================
// Decisions, outcomes & the replay trace (ORCH-003)
// ===========================================================================

/// The committed outcome of a resolved node.
///
/// Governing: IDR-002 (replicate committed *outcomes*, not invocations). The
/// Orchestrator does not treat this as truth (ORCH-001); it is the Kernel's
/// committed result of an invocation, carried back so the plan can proceed. The
/// payload is opaque here (std-only skeleton).
#[derive(Clone, Debug)]
pub struct Outcome {
    /// The node this outcome resolves.
    pub node: NodeId,
    /// The content address of the invocation that produced it (ORCH-004),
    /// linking outcome to invocation for idempotent replay.
    pub key: InvocationKey,
    /// Opaque committed result bytes (shape only in the skeleton).
    pub payload: Vec<u8>,
}

/// A single decision the Orchestrator made, recorded so it can be replayed.
///
/// Governing: ORCH-003 (replay from recorded decision trace, not recomputation)
/// and IDR-001 (Engineering Refinement) (Raft log = WAL = decision trace;
/// append-only). Each variant is
/// a *decision*, never a mutation of truth (ORCH-001). Replaying the trace in
/// `TraceIndex` order reconstructs the exact plan state without re-running any
/// engine.
#[derive(Clone, Debug)]
pub enum Decision {
    /// The Orchestrator compiled a plan for a shard.
    PlanCompiled {
        /// Shard the plan targets (SHARD-001).
        shard: ShardKey,
    },
    /// A node was dynamically added to the graph (dynamic expansion).
    NodeExpanded {
        /// The (content-addressed) node that was added.
        node: NodeId,
        /// The resolved node whose outcome triggered the expansion.
        cause: NodeId,
    },
    /// A node was dispatched to an engine/capability (ORCH-004 invocation).
    NodeDispatched {
        /// The dispatched node.
        node: NodeId,
    },
    /// A node resolved with a committed outcome (IDR-002).
    NodeResolved {
        /// The resolved node.
        node: NodeId,
    },
    /// An arbitration group was joined, selecting a winner and pruning losers.
    Arbitrated {
        /// The group that was joined.
        group: ArbitrationGroupId,
        /// The surviving node.
        winner: NodeId,
    },
}

/// One append-only entry in the decision trace.
///
/// Governing: ORCH-003, IDR-001 (Engineering Refinement) (append-only WAL).
/// Pairs a monotonic
/// [`TraceIndex`] with the [`Decision`] recorded at that position.
#[derive(Clone, Debug)]
pub struct DecisionRecord {
    /// Position in the append-only log.
    pub index: TraceIndex,
    /// The decision recorded here.
    pub decision: Decision,
}

/// The append-only record of Orchestrator decisions for one shard.
///
/// Governing: ORCH-003 (replay source of truth for *decisions*, not cognitive
/// truth), IDR-001 (Engineering Refinement) (Raft log = WAL = decision trace).
/// The Control Plane owns no
/// persistent state (ORCH-002); the durable log lives below (Persistence via
/// the Kernel / the shard's Raft group, IDR-001). This type is the read/append
/// contract the Orchestrator uses; it is *observability-adjacent* but the
/// commit path is CP (linearizable), matching "truth CP, observability AP".
pub trait DecisionTrace {
    /// Error type for trace operations.
    type Error: fmt::Debug;

    /// Append a decision to the trace, returning its assigned index.
    ///
    /// Governing: ORCH-003, IDR-001 (Engineering Refinement). Append-only:
    /// entries are never mutated or
    /// deleted. In a real deployment this append is a Raft commit through the
    /// shard leader (IDR-001 (Engineering Refinement)).
    fn append(&mut self, decision: Decision) -> Result<TraceIndex, Self::Error>;

    /// Replay all records at or after `from`, in ascending index order.
    ///
    /// Governing: ORCH-003 (recover by replay, not recomputation). Feeding
    /// these records back into the Orchestrator reconstructs plan state without
    /// re-invoking any engine/capability.
    fn replay_from(&self, from: TraceIndex) -> Result<Vec<DecisionRecord>, Self::Error>;
}

// ===========================================================================
// The Orchestrator
// ===========================================================================

/// The Control Plane orchestrator: owns the Plan/Engine Graph; owns no truth
/// and no persistent state.
///
/// Governing: ORCH-001..004, Amendment-002.
///
/// # Contract
///
/// The Orchestrator is the decision-making component of the Control Plane. It:
///
/// 1. Compiles a request into a [`Plan`] scoped to one [`ShardKey`]
///    (Amendment-002; SHARD-001).
/// 2. Drives the plan's [`EngineGraph`] forward - dispatching ready nodes,
///    accepting committed [`Outcome`]s, dynamically expanding the graph, and
///    arbitrating competing branches.
/// 3. Records every decision to a [`DecisionTrace`] so it can be resumed by
///    replay (ORCH-003), never by recomputation.
///
/// # Invariants the implementer MUST uphold
///
/// * **ORCH-001** - hold no truth. Every fact consulted comes from the Kernel;
///   every result committed goes back to the Kernel (the sole commit gateway,
///   proposed G-001 / IDR-001 (Engineering Refinement)). Nothing here is
///   authoritative state.
/// * **ORCH-002** - produce plans, not persistent state. The `Plan`/`EngineGraph`
///   are transient values; the only durable artefact touched is the
///   append-only [`DecisionTrace`], which is a *log of decisions*, not owned
///   cognitive state.
/// * **ORCH-003** - resume by replaying the recorded trace, not by recomputing
///   engine outputs.
/// * **ORCH-004** - schedule only idempotent, content-addressed invocations
///   (every [`Node`] carries an [`InvocationKey`]).
///
/// The trait takes its [`DecisionTrace`] as an associated type so that no lower
/// crate is imported here (LAYER-001: downward-only; this crate stays a pure
/// contract surface). Method bodies are intentionally absent - this is a
/// skeleton (contracts, not logic).
pub trait Orchestrator {
    /// The append-only decision trace backing replay (ORCH-003).
    type Trace: DecisionTrace;

    /// Error type surfaced by orchestration operations.
    type Error: fmt::Debug;

    /// Compile a request (opaque bytes here) into a transient [`Plan`] for a
    /// shard.
    ///
    /// Governing: Amendment-002 (Orchestrator owns the plan), ORCH-002 (the
    /// plan is a value, not persisted), SHARD-001 (single-shard scope). The
    /// act of compiling is recorded as [`Decision::PlanCompiled`].
    fn compile(&mut self, shard: ShardKey, request: &[u8]) -> Result<Plan, Self::Error>;

    /// Return the nodes currently eligible to run (all inbound edges resolved).
    ///
    /// Pure read over the transient graph; owns no truth (ORCH-001).
    fn ready_nodes(&self, plan: &Plan) -> Vec<NodeId>;

    /// Dispatch a ready node's invocation.
    ///
    /// Governing: ORCH-004 (idempotent + content-addressable): dispatching the
    /// same [`InvocationKey`] twice is a no-op that resolves to the same
    /// outcome. Recorded as [`Decision::NodeDispatched`]. Engines/capabilities
    /// run anywhere but commit only via the shard leader
    /// (IDR-001 (Engineering Refinement)).
    fn dispatch(&mut self, plan: &mut Plan, node: NodeId) -> Result<(), Self::Error>;

    /// Accept a committed [`Outcome`] for a node, advancing plan state.
    ///
    /// Governing: IDR-002 (committed outcomes are what flow back, not raw
    /// invocations). Recorded as [`Decision::NodeResolved`]. The outcome is not
    /// truth owned here (ORCH-001) - it is carried from the Kernel's commit.
    fn resolve(&mut self, plan: &mut Plan, outcome: Outcome) -> Result<(), Self::Error>;

    /// Dynamically expand the graph in response to a resolved node.
    ///
    /// Amendment-002 gives the Orchestrator ownership of the plan, so it may
    /// grow the [`EngineGraph`] as decisions unfold. Content-addressing
    /// (ORCH-004) keeps expansion idempotent under replay (ORCH-003). Recorded
    /// as [`Decision::NodeExpanded`].
    ///
    /// SCOPE (DEFERRED): bounded graph-expansion - a termination policy of
    /// max-depth / budget / no-new-subgoal (Vol 9 Part 6/7) - is a documented
    /// DEFERRED policy in this contract-only skeleton; not enforced here.
    fn expand(
        &mut self,
        plan: &mut Plan,
        cause: NodeId,
        new_nodes: Vec<Node>,
        new_edges: Vec<Edge>,
    ) -> Result<(), Self::Error>;

    /// Join an arbitration group, selecting the surviving branch per its
    /// [`ArbitrationPolicy`] and pruning the losers.
    ///
    /// The selection is a *decision* (ORCH-001: not truth) recorded as
    /// [`Decision::Arbitrated`] so it replays deterministically (ORCH-003).
    /// Returns the winning [`NodeId`].
    fn arbitrate(
        &mut self,
        plan: &mut Plan,
        group: ArbitrationGroupId,
    ) -> Result<NodeId, Self::Error>;

    /// Resume orchestration by replaying the decision trace from `from`.
    ///
    /// Governing: ORCH-003 (replay, not recomputation). Reconstructs plan state
    /// from committed [`DecisionRecord`]s without re-invoking engines. Because
    /// the Control Plane holds no persistent state (ORCH-002), this is the
    /// *only* recovery mechanism.
    fn resume(&mut self, trace: &Self::Trace, from: TraceIndex) -> Result<Plan, Self::Error>;
}

//! ARVES :: arves-conformance
//!
//! Purpose: Scenario Conformance harness: 12 axes -> reference scenarios -> node probes -> verdict.
//! Governing: Scenario Conformance Framework v1.0 (the Conformance Constitution / Executable
//!            Definition of Correctness). Verdicts are STRUCTURAL, PROPERTY-BASED and
//!            INVARIANT-BASED, never golden-output.
//! Layer: cross-cutting (test / certification)
//!
//! STATUS: CONTRACT-ONLY — this crate defines the Scenario Conformance data
//! contract (axes, verdict types, probe shapes) with a small compiling smoke
//! test; the live 12-axis scenario suite is NOT yet populated (deferred; see
//! RUNTIME_FREEZE_v1.0.md and the Build Closure known-limitations). The
//! maintainer-independent runtime certification harness lives in `standard/` +
//! `verification/`. Frozen specification governs; this crate implements, never
//! changes it.
//!
//! # Position in the Chain (Scenario Conformance Framework, Part 2)
//!
//! ARVES follows the frozen chain:
//! `Theory -> Spec -> Contracts -> Behaviour -> Conformance -> Implementation`.
//! This crate is the **Conformance** stage: the fitness function that every future
//! spec (including the Engine Graph) is accountable to. It PROVES the spec; it never
//! changes it. The Engine Graph is validated BY this framework, not the reverse
//! (Part 12: `Scenario -> Engine Graph -> Conformance`).
//!
//! # The Central Rule (Part 8)
//!
//! Because cognitive engines are non-deterministic, a run does NOT assert a single
//! correct answer. It asserts that **invariants and properties held**. Conformance is:
//! - STRUCTURAL   - the pipeline traversed the expected nodes;
//! - PROPERTY-BASED - tenant isolation, provenance, gates, replay reproduces the trace;
//! - INVARIANT-BASED - the registered invariants (ORCH-001..004, ...) were upheld.
//!
//! This is why the API below has NO notion of an "expected output": there are only
//! [`Axis`]es, [`Scenario`]s, per-node evidence emitted through [`NodeProbe`], and a
//! [`Verdict`] derived from [`Invariant`] and [`Property`] checks.
//!
//! # Three-Layer Model (Part 4)
//!
//! - [`Axis`]            - a capability dimension the architecture is stressed on (12 defined).
//! - [`Scenario`]        - a concrete instantiation combining several axes (a point in axis-space).
//! - [`NodeProbe`]       - per-node evidence emitted along the pipeline (we test NODES, not features).
//! - [`Verdict`]         - `Pass` / `Partial` / `Fail`, derived from invariant and property checks.
//!
//! # Governing Ground-Truth Invariants (frozen)
//!
//! The verdict machinery references the registered orchestration invariants:
//! - `ORCH-001` Control Plane owns no truth; only the Kernel owns truth.
//! - `ORCH-002` Control Plane produces plans, never persistent state.
//! - `ORCH-003` Replay from the recorded decision trace, not recomputation.
//! - `ORCH-004` Every engine/capability invocation is idempotent + content-addressable.
//! - `OWN-001`  One owner per state.
//! - `LAYER-001` Layers are downward-only.
//! - `SHARD-001` Partition by tenant/workspace; key immutable.

#![forbid(unsafe_code)]

/// ARVES Conformance Platform — executable ACS golden-vector runner (populates the
/// previously-empty assertion surface for the ACS layer; the universal check any
/// implementation, in any language, must pass).
pub mod acs;

/// ACS-003/004/005 **semantic** validators (RCR-004): the reference runtime's native
/// reject surface for the CCP-006 envelope/instance/language negative tiers. Retires the
/// deferral where the Rust reference previously had no ACS-003/004/005 validators.
pub mod semantic;

// =============================================================================
// Part 5 - Conformance Axes (12)
// =============================================================================

/// A conformance **axis**: one capability dimension the architecture is stressed on.
///
/// Scenario Conformance Framework, Part 5. A [`Scenario`] declares a set of axes; a
/// [`Scenario`] is therefore a *point in axis-space*, not a feature. There are exactly
/// twelve axes and this enum is closed: it is a frozen part of the Conformance
/// Constitution and must never be extended or reordered without a spec amendment.
///
/// The discriminant of each variant equals its 1-based catalog number (`1..=12`) so it
/// can be cited stably (e.g. the reference scenario "Incident Response War-Room" is
/// axes `2 + 3 + 10 + 12`, Part 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
#[non_exhaustive] // closed by spec, but reserve room for a governed amendment
pub enum Axis {
    /// Axis 1 - Ingestion, canonicalization, provenance/trust (Knowledge Assistant).
    InformationIntensive = 1,
    /// Axis 2 - Reactive flow from event to state (Incident Response).
    EventDriven = 2,
    /// Axis 3 - Approval gates and human-in-the-loop hand-off.
    HumanCollaboration = 3,
    /// Axis 4 - Goal decomposition and Engine Graph expansion.
    MultiStepPlanning = 4,
    /// Axis 5 - Durable state, pause/resume, timeouts.
    LongRunningWorkflow = 5,
    /// Axis 6 - Robot/IoT sensing and actuation (Embodied).
    PhysicalWorld = 6,
    /// Axis 7 - Hard policy gates that must block unsafe plans.
    SafetyCritical = 7,
    /// Axis 8 - Throughput, backpressure, tenant isolation at scale.
    HighVolumeStreaming = 8,
    /// Axis 9 - Delegation, arbitration across agents.
    MultiAgentCoordination = 9,
    /// Axis 10 - Dense policy evaluation and audit.
    PolicyHeavyGovernance = 10,
    /// Axis 11 - Unattended decision within risk/confidence limits.
    AutonomousDecision = 11,
    /// Axis 12 - Deterministic replay from decision trace (cites `ORCH-003`).
    RecoveryAndReplay = 12,
}

impl Axis {
    /// The full, frozen catalog of the twelve conformance axes, in catalog order.
    ///
    /// Scenario Conformance Framework, Part 5.
    pub const ALL: [Axis; 12] = [
        Axis::InformationIntensive,
        Axis::EventDriven,
        Axis::HumanCollaboration,
        Axis::MultiStepPlanning,
        Axis::LongRunningWorkflow,
        Axis::PhysicalWorld,
        Axis::SafetyCritical,
        Axis::HighVolumeStreaming,
        Axis::MultiAgentCoordination,
        Axis::PolicyHeavyGovernance,
        Axis::AutonomousDecision,
        Axis::RecoveryAndReplay,
    ];

    /// The 1-based catalog number of this axis (`1..=12`), stable for citation.
    #[must_use]
    pub const fn number(self) -> u8 {
        self as u8
    }
}

// =============================================================================
// Part 7 - Pipeline Nodes (we test NODES, not features)
// =============================================================================

/// A **node** in the end-to-end pipeline that a scenario traverses.
///
/// Scenario Conformance Framework, Part 7: "Conformance is the sum of node proofs, end
/// to end". The data-plane traversal is a cycle back into Reality:
/// `Reality -> Information Platform -> Kernel -> LCW -> Query -> Engine -> Capability ->
/// Execution -> Reality`, with the **Control Plane** deciding alongside (it owns no truth
/// per `ORCH-001`). This ordering is a subset/traversal of the frozen layer stack
/// (`LAYER-001`, downward-only): the conformance harness observes the runtime layers, it
/// does not redefine them.
///
/// Each node must emit the evidence noted in its doc-comment (Part 7 table); that
/// evidence is delivered as a [`NodeEvidence`] via a [`NodeProbe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum PipelineNode {
    /// Reality boundary - the external source and, at cycle-end, the actuated effect.
    Reality,
    /// Information Platform - source normalized to the canonical model with provenance.
    InformationPlatform,
    /// Kernel - state transition recorded; Kernel is the sole truth owner (`OWN-001`, `ORCH-001`).
    Kernel,
    /// Living Cognitive World (LCW) - consistent world/state view for the scenario.
    LivingCognitiveWorld,
    /// Query - correct, tenant-scoped read of state (read-only; `SHARD-001` isolation).
    Query,
    /// Engine Fabric - pure invocation; output is inference, NOT persisted truth.
    Engine,
    /// Control Plane - Engine Graph expanded; `ORCH-001..004` upheld; produces no truth.
    ControlPlane,
    /// Capability Fabric - capability selected and bound per plan.
    Capability,
    /// Execution - idempotent, content-addressable action carrying a `correlation_id` (`ORCH-004`).
    Execution,
}

// =============================================================================
// Part 8 - Invariants & Properties asserted
// =============================================================================

/// A registered **invariant** asserted by the conformance suite (Part 8).
///
/// Invariants are *hard*: any invariant failure yields [`Verdict::Fail`]. These are the
/// registered orchestration invariants from the frozen ground truth. The enum is closed
/// to the frozen registry and marked `#[non_exhaustive]` only so a governed amendment may
/// add future invariants without breaking downstream matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Invariant {
    /// `ORCH-001` - Control Plane owns no truth; only the Kernel owns truth.
    Orch001ControlPlaneOwnsNoTruth,
    /// `ORCH-002` - Control Plane produces plans, never persistent state.
    Orch002NoPersistentStateInControlPlane,
    /// `ORCH-003` - Replay from the recorded decision trace, not recomputation.
    Orch003ReplayableFromTrace,
    /// `ORCH-004` - Every engine/capability invocation is idempotent + content-addressable.
    Orch004IdempotentAddressable,
    /// `OWN-001` - One owner per state.
    Own001OneOwnerPerState,
    /// `LAYER-001` - Layers are downward-only.
    Layer001DownwardOnly,
    /// `SHARD-001` - Partition by tenant/workspace; the shard key is immutable.
    Shard001TenantWorkspacePartition,
}

impl Invariant {
    /// The stable registry identifier (e.g. `"ORCH-001"`) for artifacts and reports.
    ///
    /// Scenario Conformance Framework, Part 9 (the artifact lists "invariants checked").
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Invariant::Orch001ControlPlaneOwnsNoTruth => "ORCH-001",
            Invariant::Orch002NoPersistentStateInControlPlane => "ORCH-002",
            Invariant::Orch003ReplayableFromTrace => "ORCH-003",
            Invariant::Orch004IdempotentAddressable => "ORCH-004",
            Invariant::Own001OneOwnerPerState => "OWN-001",
            Invariant::Layer001DownwardOnly => "LAYER-001",
            Invariant::Shard001TenantWorkspacePartition => "SHARD-001",
        }
    }
}

/// A **property** asserted by the conformance suite (Part 8).
///
/// Properties are the property-based half of the central rule. Some are *critical*
/// (isolation, safety) and behave like invariants for verdict purposes; the rest are
/// non-critical and their failure yields [`Verdict::Partial`] rather than
/// [`Verdict::Fail`]. See [`Property::is_critical`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Property {
    /// Tenant/workspace isolation held (CRITICAL; relates to `SHARD-001`).
    TenantWorkspaceIsolation,
    /// Provenance/trust present on canonicalized information.
    ProvenanceTrustPresent,
    /// Policy gates fired when required.
    PolicyGatesFired,
    /// Safety gates blocked unsafe plans (CRITICAL).
    SafetyGatesBlockedUnsafePlans,
    /// Plan replay reproduces the decision trace (relates to `ORCH-003`).
    ReplayReproducesTrace,
}

impl Property {
    /// Whether this property is **critical**: a critical property failure is treated like
    /// an invariant failure and forces [`Verdict::Fail`] (Part 8 verdict rule). A
    /// non-critical property failure yields [`Verdict::Partial`].
    #[must_use]
    pub const fn is_critical(self) -> bool {
        matches!(
            self,
            Property::TenantWorkspaceIsolation | Property::SafetyGatesBlockedUnsafePlans
        )
    }
}

/// The outcome of checking a single [`Invariant`] or [`Property`] against emitted evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckOutcome {
    /// The assertion held.
    Held,
    /// The assertion was violated.
    Violated,
    /// The assertion could not be evaluated (evidence absent). Treated conservatively by
    /// the verdict rule: an unevaluated *required* check cannot count as `Held`.
    NotEvaluated,
}

// =============================================================================
// Part 4 - The Verdict
// =============================================================================

/// The conformance **verdict** for a scenario run.
///
/// Scenario Conformance Framework, Part 8 (verdict semantics). Ordering is meaningful:
/// `Fail < Partial < Pass` is deliberately NOT used; instead the aggregation rule in
/// [`Verdict::combine`] is worst-wins (`Fail` dominates `Partial` dominates `Pass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// All required invariants AND properties held.
    Pass,
    /// A non-critical property failed; all invariants and all critical properties held.
    Partial,
    /// Any invariant failed, OR any critical safety/isolation property failed.
    Fail,
}

impl Verdict {
    /// Worst-wins aggregation across per-check verdicts: `Fail` dominates `Partial`,
    /// which dominates `Pass`. Mirrors the Part 8 rule that any invariant/critical
    /// failure sinks the whole run.
    #[must_use]
    pub fn combine(self, other: Verdict) -> Verdict {
        match (self, other) {
            (Verdict::Fail, _) | (_, Verdict::Fail) => Verdict::Fail,
            (Verdict::Partial, _) | (_, Verdict::Partial) => Verdict::Partial,
            (Verdict::Pass, Verdict::Pass) => Verdict::Pass,
        }
    }
}

// =============================================================================
// Part 6 - Reference Scenarios (axis combinations)
// =============================================================================

/// A stable identifier for a scenario within a suite version (Part 9 artifact field).
pub type ScenarioId = &'static str;

/// A tenant/workspace shard key. The conformance harness carries it opaquely so that
/// isolation ([`Property::TenantWorkspaceIsolation`]) can be asserted end-to-end.
///
/// `SHARD-001`: partition by tenant/workspace; the key is immutable.
pub type ShardKey = String;

/// A correlation identifier threaded through execution so an action is content-addressable
/// and idempotent (`ORCH-004`; Part 7 Execution evidence).
pub type CorrelationId = String;

/// A **reference scenario**: a concrete instantiation combining several axes, plus the
/// invariants and properties it asserts.
///
/// Scenario Conformance Framework, Part 6. A scenario "declares its axes and key
/// assertions"; it is a point in axis-space, not a feature. Example rows from Part 6:
/// - Incident Response War-Room - axes `2 + 3 + 10 + 12`;
/// - Warehouse Robot Dispatch   - axes `6 + 7 + 11 + 4`;
/// - Enterprise Knowledge Query - axes `1 + 8 + 9`;
/// - Long Compliance Review     - axes `5 + 10 + 3`.
///
/// This is a data contract (skeleton), not the populated suite; per Part 3 the FRAMEWORK
/// is defined now and the populated assertion suite grows as node contracts sharpen.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Scenario {
    /// Stable id within a suite version (Part 9 / Part 11 versioning).
    pub id: ScenarioId,
    /// Human-readable name (e.g. "Incident Response War-Room").
    pub name: &'static str,
    /// The axes this scenario stresses (its coordinates in axis-space, Part 6).
    pub axes: Vec<Axis>,
    /// The pipeline nodes this scenario is expected to traverse (Part 7). We test nodes,
    /// so the expected traversal is part of the scenario contract.
    pub expected_path: Vec<PipelineNode>,
    /// Invariants this scenario asserts (hard; failure -> [`Verdict::Fail`]).
    pub required_invariants: Vec<Invariant>,
    /// Properties this scenario asserts (critical ones behave like invariants).
    pub required_properties: Vec<Property>,
}

impl Scenario {
    /// True if this scenario declares the given axis (Part 6: scenarios are axis sets).
    #[must_use]
    pub fn stresses(&self, axis: Axis) -> bool {
        self.axes.contains(&axis)
    }
}

// =============================================================================
// Part 7 / Part 9 - Node evidence and the machine-readable artifact
// =============================================================================

/// Per-node **evidence** emitted while a scenario traverses the pipeline.
///
/// Scenario Conformance Framework, Part 7 ("every node emits evidence") and Part 9 (the
/// artifact records per-node evidence). This is the observation a [`NodeProbe`] produces;
/// the verdict machinery consumes a bag of these to run its invariant/property checks.
///
/// It is deliberately *structural*: it records that the node did its job and what it
/// asserted, NOT a golden output value (Part 8).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NodeEvidence {
    /// Which node emitted this evidence.
    pub node: PipelineNode,
    /// Free-form structural description of what the node did (e.g. "canonicalized with
    /// provenance"). The suite matches on the checks below, not on this text.
    pub summary: String,
    /// Correlation id tying execution/actuation together (`ORCH-004`), when applicable.
    pub correlation_id: Option<CorrelationId>,
    /// Per-invariant check outcomes this node contributes.
    pub invariant_checks: Vec<(Invariant, CheckOutcome)>,
    /// Per-property check outcomes this node contributes.
    pub property_checks: Vec<(Property, CheckOutcome)>,
}

/// The machine-readable **conformance artifact** for a single scenario run.
///
/// Scenario Conformance Framework, Part 9 (the "Vol 9 Part 14 hook"): "both the
/// certificate and the regression record". Records scenario id + axes, the expanded
/// Engine Graph reference, per-node evidence, invariants checked, arbitration choices,
/// policy gates, the Runtime Fingerprint, and the [`Verdict`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ConformanceArtifact {
    /// Scenario id and axes exercised (Part 9).
    pub scenario_id: ScenarioId,
    /// Axes exercised by the run.
    pub axes: Vec<Axis>,
    /// Opaque handle to the expanded Engine Graph produced by the Control Plane. The
    /// Engine Graph is validated BY this framework (Part 12); we only reference it here.
    pub engine_graph_ref: Option<String>,
    /// Per-node evidence collected end-to-end (Part 7).
    pub node_evidence: Vec<NodeEvidence>,
    /// Arbitration choices recorded during multi-agent coordination (axis 9), if any.
    pub arbitration_choices: Vec<String>,
    /// Policy gates encountered and whether they fired (Part 8 property).
    pub policy_gates: Vec<(String, CheckOutcome)>,
    /// The Runtime Fingerprint: which spec + suite version this run was tested against
    /// (Part 11: "N% at Level Lx against Framework vA / Spec vB").
    pub runtime_fingerprint: RuntimeFingerprint,
    /// The derived verdict for this run.
    pub verdict: Verdict,
}

/// Identifies exactly which runtime/spec/suite a result pertains to (Part 11).
///
/// A bare percentage is meaningless without a level and a version; the fingerprint pins
/// the spec version a runtime was tested against and thereby resolves corpus version
/// drift.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RuntimeFingerprint {
    /// Frozen spec version this run was checked against (e.g. Spec Era FROZEN tag).
    pub spec_version: String,
    /// Conformance suite (framework) version (e.g. "Scenario Conformance Framework v1.0").
    pub suite_version: String,
    /// Opaque runtime build/identity string.
    pub runtime_id: String,
}

// =============================================================================
// Part 10 - Scoring, Levels & Profiles
// =============================================================================

/// A conformance **level** a runtime is reported against (Part 10).
///
/// Levels map onto the frozen milestone ladder (I1..I6): a result is always stated as a
/// level against a suite version, never a bare percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ConformanceLevel {
    /// L1 - Core Runtime: Information -> Kernel -> Query nodes conformant.
    L1CoreRuntime,
    /// L2 - Cognitive Control: Engine Fabric + Control Plane invariants (`ORCH-001..004`).
    L2CognitiveControl,
    /// L3 - Distributed: conformance preserved across distributed deployment.
    L3Distributed,
    /// L4 - Multi-Agent: conformance preserved under multi-agent coordination.
    L4MultiAgent,
    /// Certified Product: a product built on a certified runtime passing its scenario set.
    CertifiedProduct,
}

// =============================================================================
// NodeProbe trait - the per-node evidence emitter (Part 4 / Part 7)
// =============================================================================

/// A **node probe**: the per-node observer that emits [`NodeEvidence`] as a scenario
/// traverses the pipeline. "We test nodes, not features" (Part 4 / Part 7).
///
/// Each concrete runtime layer supplies a `NodeProbe` implementation that reports what its
/// node did and which invariant/property checks it contributes. The harness composes the
/// probes along the pipeline; the sum of node proofs is the conformance result (Part 7).
///
/// Method bodies are intentionally omitted (skeleton): this crate defines the CONTRACT,
/// not the logic (Part 3 bootstrapping). Implementations live in the runtime layer crates
/// and PROVE the spec, never change it.
pub trait NodeProbe {
    /// Which pipeline node this probe observes.
    fn node(&self) -> PipelineNode;

    /// Emit the structural evidence for this node given the scenario under test. Returns
    /// the evidence the node must produce per the Part 7 table.
    fn observe(&self, scenario: &Scenario) -> NodeEvidence;
}

/// A **verdict engine**: derives a [`Verdict`] from collected [`NodeEvidence`] per the
/// Part 8 central rule (invariant + property based, worst-wins via [`Verdict::combine`]).
///
/// Separated from [`NodeProbe`] because probes *observe* while the engine *judges*; both
/// are trait contracts only in this skeleton.
pub trait VerdictEngine {
    /// Judge a scenario against its collected node evidence, producing the machine-readable
    /// [`ConformanceArtifact`] (Part 9). The verdict follows Part 8: `Fail` on any
    /// invariant or critical property violation, `Partial` on a non-critical property
    /// failure, else `Pass`.
    fn judge(
        &self,
        scenario: &Scenario,
        evidence: &[NodeEvidence],
        fingerprint: RuntimeFingerprint,
    ) -> ConformanceArtifact;
}

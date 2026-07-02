//! ARVES :: arves-capability-fabric
//!
//! Purpose: The Capability Fabric owns the registry of declared *capabilities* and the
//! *bindings* that map a logical capability to a concrete, invokable provider. It answers
//! exactly one question for the layers above it: "given this capability, what may be
//! invoked, and under what contract?" It never decides *whether* something should run
//! (that is the Control Plane's plan, ORCH-002) and it never records *what happened*
//! (that is the Kernel's truth, ORCH-001).
//!
//! Governing: CAP-001..009 (proposed); Vol 9 Part 3. Cross-cut: ORCH-001, ORCH-002,
//! ORCH-004, OWN-001, LAYER-001, SHARD-001.
//!
//! Layer: Capability (Data Plane). Per LAYER-001 the layering is downward-only:
//! `Reality -> Information Platform -> Kernel -> Persistence -> LCW -> Query -> Engine ->
//! `**Capability** -> Execution (+ Control Plane orthogonal). This crate sits above the
//! Engine layer and below Execution; it never reaches sideways or upward into
//! truth-owning layers, and it depends on no sibling crate (std-only skeleton).
//!
//! STATUS: I1 (Distributed Runtime) CONTRACT-ONLY (by design, not unfinished). This crate
//! defines the Capability Fabric interfaces/types; it carries no capability-gating logic.
//! The exercised capability logic in the reference runtime flows through the SDK/Bridge in
//! `products/` (see RUNTIME_FREEZE_v1.0.md, guarantee alignment). Frozen specification
//! governs; this crate *implements* the spec and never changes it (Theory -> Spec ->
//! Contracts -> Behaviour -> Conformance -> Implementation). Any `fn` bodies present are
//! trivial placeholders that exist only so the contract compiles; they encode no logic. The identifiers CAP-001..009 are PROPOSED (informative, pending
//! CCP-GATE) and MUST NOT be enforced as registered invariants until ratified; they are
//! cited here to anchor intent, not to bind.
//!
//! # What this crate owns (and, emphatically, does not)
//!
//! - **Owns**: the set of [`CapabilityBinding`]s -- the declarative map from a
//!   [`CapabilityId`] to an invokable provider plus its contract. This is the *single
//!   owner* of binding state (OWN-001).
//! - **Does NOT own truth**: bindings are configuration, not cognitive truth. Only the
//!   Kernel owns truth and is the sole commit gateway (ORCH-001; G-001 proposed). This
//!   crate never commits, never persists outcomes, never mutates world state.
//! - **Does NOT own plans**: which capability to invoke, in what order, and why, is a
//!   *plan* produced by the Control Plane (ORCH-002). The fabric is a lookup/validation
//!   surface consulted by planning and execution; it emits no plans and holds no
//!   persistent decision state.
//! - **Idempotency / content-addressing (ORCH-004)**: the fabric exposes the metadata
//!   (via [`InvocationContract`]) that lets callers construct idempotent,
//!   content-addressable invocations, but it does not perform the invocation itself.
//!
//! # Design-principle citations (CAP-001..009, proposed)
//!
//! Grounded in the frozen corpus but pending ratification; each `CAP-00n` tag below marks
//! the interface element it motivates:
//! - CAP-001: capabilities are *declared* under a stable, immutable logical identity.
//! - CAP-002: a capability resolves to at most one active provider per shard (owner-per-state).
//! - CAP-003: bindings are versioned; rebinding supersedes, never mutates in place.
//! - CAP-004: every binding carries an explicit [`InvocationContract`] (inputs/outputs/effects).
//! - CAP-005: resolution is a *read* over owned bindings and is side-effect free.
//! - CAP-006: bindings are partitioned by shard key (tenant/workspace) per SHARD-001.
//! - CAP-007: the fabric holds no truth and issues no commits (ORCH-001).
//! - CAP-008: the fabric produces no plans and no persistent outcome state (ORCH-002).
//! - CAP-009: capability invocations MUST be idempotent + content-addressable (ORCH-004).

#![forbid(unsafe_code)]

// =============================================================================
// Identity & partitioning
// =============================================================================

/// Stable, immutable logical identity of a declared capability.
///
/// CAP-001: a capability is named by identity, not by its current provider. The string is
/// a namespaced logical name (e.g. `"arves.text.summarize"`), never a physical address.
/// Callers treat it as opaque; equality and hashing are by exact bytes. The identity is
/// immutable once declared -- rebinding a capability (CAP-003) never changes its
/// [`CapabilityId`], only the [`CapabilityBinding`] it resolves to.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CapabilityId(pub String);

/// Monotonic supersession version of a binding for a given [`CapabilityId`].
///
/// CAP-003: rebinding produces a strictly higher `BindingVersion`; existing versions are
/// never mutated in place. This mirrors the append-only, supersession discipline of the
/// WAL (IDR-005) even though the fabric itself persists nothing -- versions are what make
/// a binding safely content-addressable for the invocations built atop it (ORCH-004 /
/// CAP-009).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingVersion(pub u64);

/// Immutable shard key partitioning bindings by tenant/workspace.
///
/// SHARD-001 / CAP-006: bindings are partitioned by an immutable shard key. A binding
/// resolved in one shard is never visible from another; there is no cross-shard binding
/// namespace (mirroring "no cross-shard atomic commit", IDR-004). The key is immutable once
/// assigned to the state it addresses.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShardKey {
    /// Tenant identifier component of the partition key (immutable, SHARD-001).
    pub tenant: String,
    /// Workspace identifier component of the partition key (immutable, SHARD-001).
    pub workspace: String,
}

/// Identity of a concrete provider that can service a capability.
///
/// CAP-002: this names *what* is invoked (an engine, an execution adapter, an external tool
/// endpoint) without describing *how*. The fabric stores the reference; it never
/// dereferences or invokes it -- invocation belongs to the Execution layer (LAYER-001).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProviderId(pub String);

// =============================================================================
// Invocation contract (metadata only -- no execution)
// =============================================================================

/// Declared effect class of invoking a capability.
///
/// CAP-004 / CAP-009: the fabric records the *declared* effect so callers (the Control
/// Plane when planning, Execution when acting) can uphold idempotency and content-addressing
/// (ORCH-004). The fabric neither validates nor performs the effect; the declaration is a
/// contract the caller must honour, not behaviour this crate enforces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EffectClass {
    /// No observable side effects; safe to elide, cache, and replay freely (read-only).
    Pure,
    /// Side-effecting but idempotent under a content-addressable key (ORCH-004 / CAP-009):
    /// re-invoking with identical content yields the same effect exactly once.
    IdempotentEffect,
    /// May propose a write toward the Kernel commit gateway; the commit is the Kernel's,
    /// never the fabric's (ORCH-001; G-001 proposed). The fabric only records that the
    /// capability *may* propose writes -- it issues none itself.
    ProposesWrite,
}

/// The contract a binding advertises for invoking its provider.
///
/// CAP-004: every [`CapabilityBinding`] carries an explicit contract describing the shape of
/// inputs, outputs, and the effect class. This is descriptive metadata only -- it is the
/// interface the fabric *publishes*, not logic it *runs*. Callers use it to build idempotent,
/// content-addressable invocations (CAP-009 / ORCH-004). Schema references are opaque
/// (e.g. content-addressed schema ids) so the skeleton stays std-only and does not model the
/// ontology; richer typing arrives once `arves-ontology` is wired in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvocationContract {
    /// Opaque schema reference for accepted inputs.
    pub input_schema: String,
    /// Opaque schema reference for produced outputs.
    pub output_schema: String,
    /// Declared effect class governing replay/idempotency expectations (CAP-009).
    pub effect: EffectClass,
}

// =============================================================================
// Binding (the owned state)
// =============================================================================

/// A resolved mapping from a logical capability to a concrete provider plus its contract.
///
/// This struct is the *only* state the Capability Fabric owns (OWN-001, CAP-002). It is
/// pure configuration: it carries no truth (ORCH-001) and no plan (ORCH-002). A binding is
/// immutable once created; supersession is expressed by issuing a new binding at a higher
/// [`BindingVersion`] (CAP-003), never by editing an existing one.
///
/// Because a binding is fully described by `(capability, shard, version, provider,
/// contract)`, it is content-addressable, which is what lets the invocations layered atop it
/// satisfy CAP-009 / ORCH-004.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityBinding {
    /// Logical capability this binding resolves (CAP-001).
    pub capability: CapabilityId,
    /// Partition this binding lives in; never crosses shards (SHARD-001 / CAP-006).
    pub shard: ShardKey,
    /// Supersession version; strictly increases on rebind (CAP-003).
    pub version: BindingVersion,
    /// Concrete provider to invoke (CAP-002).
    pub provider: ProviderId,
    /// Published invocation contract (CAP-004).
    pub contract: InvocationContract,
}

// =============================================================================
// Errors
// =============================================================================

/// Failure modes surfaced by the registry.
///
/// These describe registry-shaped failures only. They never encode execution failures or
/// truth conflicts -- those belong to the Execution layer and the Kernel respectively
/// (ORCH-001). Keeping the error surface narrow reinforces that the fabric is a lookup and
/// validation surface, not an actuator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// The requested capability has no active binding in the given shard (CAP-005).
    Unbound {
        /// Capability that failed to resolve.
        capability: CapabilityId,
        /// Shard the lookup was scoped to (SHARD-001).
        shard: ShardKey,
    },
    /// A rebind was rejected because its version did not strictly supersede the current one
    /// (CAP-003): monotonicity is required so supersession stays well-ordered and
    /// content-addressable (ORCH-004).
    NonMonotonicVersion {
        /// Version currently held for the capability in this shard.
        current: BindingVersion,
        /// Version offered by the rejected rebind.
        offered: BindingVersion,
    },
    /// The capability was bound before being declared, or its declaration is unknown in this
    /// shard (CAP-001).
    UndeclaredCapability(CapabilityId),
}

// =============================================================================
// Registry trait (the interface layers above consult)
// =============================================================================

/// The contract for owning and resolving capability bindings.
///
/// CAP-002 / CAP-005 / CAP-007 / CAP-008: an implementor owns bindings and resolves them as
/// side-effect-free reads. It MUST NOT own truth (ORCH-001), MUST NOT persist outcomes or
/// emit plans (ORCH-002), and MUST honour one active binding per capability per shard
/// (OWN-001 / SHARD-001 / CAP-002).
///
/// Method bodies are intentionally absent in this skeleton -- the signatures *are* the
/// contract.
pub trait CapabilityRegistry {
    /// Declare a capability so it may later be bound (CAP-001).
    ///
    /// Declaration establishes identity only within `shard`; it selects no provider and has
    /// no side effects beyond recording the (immutable) identity. Binding an undeclared
    /// capability is rejected with [`RegistryError::UndeclaredCapability`].
    fn register(
        &mut self,
        shard: &ShardKey,
        capability: CapabilityId,
    ) -> Result<(), RegistryError>;

    /// Bind (or rebind) a declared capability to a concrete provider under a contract.
    ///
    /// CAP-002 / CAP-003: establishes the single active binding for the capability in
    /// `binding.shard`. If a binding already exists, `binding.version` MUST strictly exceed
    /// the current version (else [`RegistryError::NonMonotonicVersion`]); the prior binding
    /// is superseded, never mutated (append-only supersession, cf. IDR-005). Returns the
    /// now-active binding.
    ///
    /// This records configuration only: it commits no truth (ORCH-001; G-001 proposed) and
    /// produces no plan or persistent outcome (ORCH-002).
    fn bind(
        &mut self,
        binding: CapabilityBinding,
    ) -> Result<CapabilityBinding, RegistryError>;

    /// Resolve the currently active binding for a capability in a shard.
    ///
    /// CAP-005: a pure read over owned state. It MUST be side-effect free and MUST NOT invoke
    /// the provider. Returns [`RegistryError::Unbound`] if no active binding exists in
    /// `shard` (SHARD-001 scopes the lookup; other shards are never consulted).
    fn resolve(
        &self,
        shard: &ShardKey,
        capability: &CapabilityId,
    ) -> Result<CapabilityBinding, RegistryError>;
}

// =============================================================================
// Reference implementation: an in-memory registry.
// =============================================================================

use std::collections::{HashMap, HashSet};

fn key(shard: &ShardKey, cap: &CapabilityId) -> (String, String, String) {
    (shard.tenant.clone(), shard.workspace.clone(), cap.0.clone())
}

/// A concrete in-memory [`CapabilityRegistry`] reference implementation. It owns only
/// bindings (CAP-002), enforces declare-before-bind (CAP-001), one active binding per
/// `(capability, shard)`, and strictly-monotonic supersession (CAP-003). It is pure
/// configuration: it commits no truth (ORCH-001) and resolves as a side-effect-free read.
#[derive(Default)]
pub struct MemRegistry {
    declared: HashSet<(String, String, String)>,
    active: HashMap<(String, String, String), CapabilityBinding>,
}

impl MemRegistry {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CapabilityRegistry for MemRegistry {
    fn register(&mut self, shard: &ShardKey, capability: CapabilityId) -> Result<(), RegistryError> {
        self.declared.insert(key(shard, &capability));
        Ok(())
    }

    fn bind(&mut self, binding: CapabilityBinding) -> Result<CapabilityBinding, RegistryError> {
        let k = key(&binding.shard, &binding.capability);
        if !self.declared.contains(&k) {
            return Err(RegistryError::UndeclaredCapability(binding.capability.clone()));
        }
        if let Some(current) = self.active.get(&k) {
            if binding.version.0 <= current.version.0 {
                return Err(RegistryError::NonMonotonicVersion {
                    current: current.version,
                    offered: binding.version,
                });
            }
        }
        self.active.insert(k, binding.clone());
        Ok(binding)
    }

    fn resolve(&self, shard: &ShardKey, capability: &CapabilityId) -> Result<CapabilityBinding, RegistryError> {
        self.active
            .get(&key(shard, capability))
            .cloned()
            .ok_or_else(|| RegistryError::Unbound { capability: capability.clone(), shard: shard.clone() })
    }
}

#[cfg(test)]
mod mem_registry_tests {
    use super::*;

    fn shard() -> ShardKey {
        ShardKey { tenant: "t1".into(), workspace: "w1".into() }
    }

    fn binding(v: u64) -> CapabilityBinding {
        CapabilityBinding {
            capability: CapabilityId("derive.fact".into()),
            shard: shard(),
            version: BindingVersion(v),
            provider: ProviderId("engine:derive.fact@1.0.0".into()),
            contract: InvocationContract {
                input_schema: "acs:uci.fact".into(),
                output_schema: "acs:uci.fact".into(),
                effect: EffectClass::ProposesWrite,
            },
        }
    }

    #[test]
    fn declare_bind_resolve_roundtrip() {
        let mut r = MemRegistry::new();
        // Binding before declaring is rejected (CAP-001).
        assert!(matches!(r.bind(binding(1)), Err(RegistryError::UndeclaredCapability(_))));
        r.register(&shard(), CapabilityId("derive.fact".into())).unwrap();
        r.bind(binding(1)).unwrap();
        let got = r.resolve(&shard(), &CapabilityId("derive.fact".into())).unwrap();
        assert_eq!(got.provider, ProviderId("engine:derive.fact@1.0.0".into()));
    }

    #[test]
    fn rebind_must_be_monotonic_and_unbound_reports() {
        let mut r = MemRegistry::new();
        r.register(&shard(), CapabilityId("derive.fact".into())).unwrap();
        r.bind(binding(2)).unwrap();
        assert!(matches!(r.bind(binding(2)), Err(RegistryError::NonMonotonicVersion { .. })));
        r.bind(binding(3)).unwrap(); // strictly higher supersedes (CAP-003)
        assert!(matches!(
            r.resolve(&shard(), &CapabilityId("unbound".into())),
            Err(RegistryError::Unbound { .. })
        ));
    }
}

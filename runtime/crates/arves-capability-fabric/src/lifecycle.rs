//! Binding LIFECYCLE with append-only supersession history (RCR-026, I4 Stage 1).
//!
//! Design basis: `docs/design/I4_Capability_Scheduling_Design.md` §3.5 lifecycle
//! step 1 ("capabilities declared per shard (`register`), then bound (`bind`) with
//! strictly-monotonic versions; append-only supersession") and step 10 ("Retire —
//! bindings superseded by version bump, never mutated; old traces stay replayable").
//!
//! [`LifecycleRegistry`] is a second concrete implementation of the frozen
//! [`CapabilityRegistry`] contract (the frozen trait, types and [`MemRegistry`]
//! are byte-unchanged). Where [`MemRegistry`] keeps only the ACTIVE binding, this
//! registry keeps the full **append-only supersession chain** per
//! `(shard, capability)` — "durable semantics": nothing is ever mutated or deleted,
//! every accepted lifecycle transition is a new appended event (the IDR-005
//! append-only discipline applied to configuration; the fabric still PERSISTS
//! nothing to disk — bindings are configuration, not truth, ORCH-001).
//!
//! It adds, additively (no frozen signature changed):
//!
//! - **Revocation** ([`LifecycleRegistry::revoke`]): the frozen contract has no
//!   unbind/revoke; the design retires bindings by supersession only. Simplest
//!   option consistent with that (recorded as RCR-026 DR-3): a revocation is a
//!   **tombstone event appended at a strictly higher version** — never a deletion.
//!   After a revoke, `resolve` is a hard [`RegistryError::Unbound`]; a later rebind
//!   must strictly supersede the tombstone version (replaying an old version can
//!   never resurrect a revoked capability — revocation bites).
//! - **Pinned replay resolution** ([`LifecycleRegistry::resolve_pinned`]): a
//!   dispatched invocation pins its `BindingVersion` (design §3.8 rebind-during-
//!   flight; registered basis ORCH-003 — replay uses the binding that actually ran;
//!   CAP-009 states this directly but is (PROPOSED — CCP-GATE required)). Superseded
//!   and even revoked-era versions therefore stay READABLE for replay/audit — but
//!   are never *served as active* by `resolve`.
//! - **Audit history** ([`LifecycleRegistry::history`]): the full supersession
//!   chain, in order (design §3.19 — every binding transition reconstructible).
//!
//! Errors reuse the frozen [`RegistryError`] enum unchanged (adding a variant would
//! change the frozen type): revoking an undeclared capability is
//! `UndeclaredCapability`, revoking with no active binding is `Unbound`, and a
//! non-superseding revoke/rebind version is `NonMonotonicVersion`.
//!
//! Registered invariants upheld here: OWN-001 (this registry is the single owner of
//! its binding state and owns nothing else), SHARD-001 (every chain is keyed by the
//! immutable shard key; no cross-shard visibility), ORCH-001 (no truth, no commit —
//! this module has no path to any kernel), ORCH-002 (no plans, no persistent
//! decision state), LAYER-001 (std + this crate only). Determinism: state is a pure
//! function of the recorded operation sequence (`BTreeMap`/`BTreeSet`, no clocks,
//! no randomness).

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BindingVersion, CapabilityBinding, CapabilityId, CapabilityRegistry, RegistryError, ShardKey,
};

/// One append-only lifecycle event in a `(shard, capability)` supersession chain.
///
/// Events are immutable once appended; the chain is strictly ordered by
/// [`LifecycleEvent::version`] (each accepted event strictly supersedes the last).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// A binding became active at its version (initial bind or rebind).
    Bound(CapabilityBinding),
    /// The capability was revoked: a tombstone at a strictly higher version.
    /// While a tombstone is the latest event, `resolve` returns
    /// [`RegistryError::Unbound`].
    Revoked {
        /// The tombstone's supersession version (strictly above the version it revokes).
        version: BindingVersion,
    },
}

impl LifecycleEvent {
    /// The supersession version this event occupies in the chain.
    pub fn version(&self) -> BindingVersion {
        match self {
            LifecycleEvent::Bound(b) => b.version,
            LifecycleEvent::Revoked { version } => *version,
        }
    }
}

type ChainKey = (ShardKey, CapabilityId);

/// A [`CapabilityRegistry`] with full append-only lifecycle semantics
/// (register → bind → rebind → revoke), supersession history and pinned replay
/// resolution. See the module docs for contract and invariants.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct LifecycleRegistry {
    /// Declared capability identities per shard (declare-before-bind).
    declared: BTreeSet<ChainKey>,
    /// The append-only supersession chain per `(shard, capability)`.
    chains: BTreeMap<ChainKey, Vec<LifecycleEvent>>,
}

impl LifecycleRegistry {
    /// An empty registry (no declarations, no chains).
    pub fn new() -> Self {
        Self::default()
    }

    fn chain(&self, shard: &ShardKey, capability: &CapabilityId) -> Option<&Vec<LifecycleEvent>> {
        self.chains.get(&(shard.clone(), capability.clone()))
    }

    /// The version occupied by the latest event in the chain (bound OR tombstone),
    /// i.e. the version any next lifecycle event must strictly exceed.
    fn current_version(&self, shard: &ShardKey, capability: &CapabilityId) -> Option<BindingVersion> {
        self.chain(shard, capability).and_then(|c| c.last()).map(LifecycleEvent::version)
    }

    /// Revoke the capability's active binding by appending a tombstone at `offered`
    /// (RCR-026 DR-3 — supersession, never deletion).
    ///
    /// Refusals (frozen error vocabulary, unchanged):
    /// - [`RegistryError::UndeclaredCapability`] — the capability was never declared
    ///   in `shard`;
    /// - [`RegistryError::Unbound`] — there is no ACTIVE binding to revoke (never
    ///   bound, or already revoked — a double revoke is refused, keeping the chain
    ///   meaningful);
    /// - [`RegistryError::NonMonotonicVersion`] — `offered` does not strictly exceed
    ///   the current chain version (supersession must stay strictly monotonic — the
    ///   frozen [`RegistryError::NonMonotonicVersion`] contract; design §3.5 step 1).
    ///
    /// After a successful revoke, [`CapabilityRegistry::resolve`] returns `Unbound`
    /// until a rebind strictly supersedes the tombstone.
    pub fn revoke(
        &mut self,
        shard: &ShardKey,
        capability: &CapabilityId,
        offered: BindingVersion,
    ) -> Result<(), RegistryError> {
        let k: ChainKey = (shard.clone(), capability.clone());
        if !self.declared.contains(&k) {
            return Err(RegistryError::UndeclaredCapability(capability.clone()));
        }
        match self.chains.get(&k).and_then(|c| c.last()) {
            Some(LifecycleEvent::Bound(current)) => {
                if offered <= current.version {
                    return Err(RegistryError::NonMonotonicVersion {
                        current: current.version,
                        offered,
                    });
                }
            }
            // Never bound, or the latest event is already a tombstone: nothing
            // active to revoke.
            _ => {
                return Err(RegistryError::Unbound { capability: capability.clone(), shard: shard.clone() })
            }
        }
        self.chains.entry(k).or_default().push(LifecycleEvent::Revoked { version: offered });
        Ok(())
    }

    /// Resolve the EXACT historical binding at `version` — the replay/audit surface
    /// (ORCH-003: replay uses the binding that actually ran; design §3.8/§3.11).
    ///
    /// Superseded and revoked-era `Bound` versions remain readable here forever;
    /// a tombstone version or an unknown version is [`RegistryError::Unbound`].
    /// This is a pure read and NEVER an authorization surface: only
    /// [`CapabilityRegistry::resolve`] answers "what may be invoked NOW".
    pub fn resolve_pinned(
        &self,
        shard: &ShardKey,
        capability: &CapabilityId,
        version: BindingVersion,
    ) -> Result<CapabilityBinding, RegistryError> {
        self.chain(shard, capability)
            .into_iter()
            .flatten()
            .find_map(|e| match e {
                LifecycleEvent::Bound(b) if b.version == version => Some(b.clone()),
                _ => None,
            })
            .ok_or_else(|| RegistryError::Unbound { capability: capability.clone(), shard: shard.clone() })
    }

    /// The full append-only supersession chain for `(shard, capability)`, in event
    /// order (empty if none). Audit surface (design §3.19); pure read.
    pub fn history(&self, shard: &ShardKey, capability: &CapabilityId) -> &[LifecycleEvent] {
        self.chain(shard, capability).map(Vec::as_slice).unwrap_or(&[])
    }
}

impl CapabilityRegistry for LifecycleRegistry {
    fn register(&mut self, shard: &ShardKey, capability: CapabilityId) -> Result<(), RegistryError> {
        // Declaration establishes identity only; re-declaration is a no-op (identity
        // is immutable, so declaring the same identity twice changes nothing).
        self.declared.insert((shard.clone(), capability));
        Ok(())
    }

    fn bind(&mut self, binding: CapabilityBinding) -> Result<CapabilityBinding, RegistryError> {
        let k: ChainKey = (binding.shard.clone(), binding.capability.clone());
        if !self.declared.contains(&k) {
            return Err(RegistryError::UndeclaredCapability(binding.capability.clone()));
        }
        // The new version must strictly supersede the LATEST chain event — including
        // a revocation tombstone, so a stale (pre-revoke) version can never rebind.
        if let Some(current) = self.current_version(&binding.shard, &binding.capability) {
            if binding.version <= current {
                return Err(RegistryError::NonMonotonicVersion { current, offered: binding.version });
            }
        }
        self.chains.entry(k).or_default().push(LifecycleEvent::Bound(binding.clone()));
        Ok(binding)
    }

    fn resolve(
        &self,
        shard: &ShardKey,
        capability: &CapabilityId,
    ) -> Result<CapabilityBinding, RegistryError> {
        // Authoritative resolve: ONLY the latest chain event answers. A superseded
        // binding is never served (stale-binding test), and a tombstone is a hard
        // Unbound (revocation bites).
        match self.chain(shard, capability).and_then(|c| c.last()) {
            Some(LifecycleEvent::Bound(b)) => Ok(b.clone()),
            _ => Err(RegistryError::Unbound { capability: capability.clone(), shard: shard.clone() }),
        }
    }
}

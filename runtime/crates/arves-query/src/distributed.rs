//! RCR-024 (I3 Stage 2) — DISTRIBUTED READS over the I2 cluster substrate.
//!
//! Approved design: `docs/design/I3_Distributed_Query_Design.md`. This module
//! raises the RCR-023 single-node query core to the design's distributed read
//! path over the I2 `ClusterSim` (RCR-021): shard-aware routing (§3.1 #1), the
//! IDR-001 consistency ladder served honestly (§3.7, §3.12), bounded
//! tenant-internal scatter-gather (§3.1 #4, §3.7), and a read-your-writes
//! floor (§3.12 / OQ-5 — additive carrier). Every frozen v1.0 type and trait
//! signature in `lib.rs` remains byte-unchanged (the RCR-008/019/023 additive
//! pattern).
//!
//! # The consistency ladder (IDR-001 · IDR-005: truth is CP, observability is AP)
//!
//! A [`ClusterQuery`] is a read handle bound to ONE serving replica (the same
//! client-to-replica shape as `ClusterKernel`). Per requested [`ReadTier`]:
//!
//! - **`Linearizable` — leader-consistent (CP truth).** Read-index form: the
//!   router consults the shard directory for the current highest-term live
//!   leader and takes that leader's raft *commit index* as the read-index —
//!   but ONLY after the Raft §6.4 read-index PRECONDITION holds: the leader
//!   must have a committed entry of its CURRENT term (RCR-024 DR-8). Without
//!   it, a freshly elected leader's commit index may EXCLUDE prior-term
//!   quorum-committed (acked!) entries — the §5.4.2 term guard refuses to
//!   count them, and RCR-019 DR-2 appends no election no-op — so serving on
//!   that index could silently MISS an acked write. With the precondition
//!   satisfied, the read is served ONLY if the bound replica has *applied*
//!   at least the read-index — then the replica's WAL fold provably contains
//!   every quorum-committed outcome (and possibly newer ones, which is still
//!   linearizable w.r.t. committed truth). Every other case fails
//!   [`QueryError::LeaderUnavailable`] — "could not confirm currency with the
//!   per-shard Raft leader" (frozen contract wording): a partitioned/lagging
//!   replica, a deposed minority leader, a leaderless group, or a new leader
//!   that has not yet committed in its own term all refuse rather than serve
//!   possibly-stale data as linearizable. CP behaviour, deliberately
//!   unavailable under partition and briefly after an election (design §3.7).
//! - **`BoundedStaleness` — follower read, admitted only on PROVABLE lag.**
//!   The frozen bound unit is milliseconds, but the safe time↔index
//!   attestation mechanism is design OQ-2 and awaits its own IDR. This stage
//!   therefore attests exactly one lag value: **zero** — the serving replica
//!   has applied everything the current leader has committed, so its lag is
//!   0 ms ≤ any bound (no clock, no time↔index mapping invented). The zero
//!   proof is only as good as the index it is measured against, so the same
//!   DR-8 current-term-commit precondition gates it: a freshly elected
//!   leader's not-yet-valid commit index attests nothing. When zero lag
//!   cannot be proven (replica behind, no leader to attest against, or the
//!   leader without a current-term commit),
//!   the read is refused with [`QueryError::StalenessBoundExceeded`] carrying
//!   [`LAG_UNATTESTABLE`]: the true lag in milliseconds is *unknown*, and an
//!   unknown lag must be treated as exceeding every finite bound. Honest
//!   refusal over silent staleness (design §3.20).
//! - **`Eventual` — AP observability.** Serves the bound replica's local WAL
//!   fold as-is, with no leader contact: always available (partition
//!   included), possibly stale, and *labeled* — `served_tier: Eventual` and
//!   `observed_at` say exactly what the caller got; the data is never wrong
//!   for the version it reports (frozen contract wording).
//!
//! # HONEST SCOPE — what is simulated vs real
//!
//! - **In-process simulation only** (the I2 vehicle, RCR-021): the "cluster"
//!   is the deterministic `ClusterSim` — no network, no wire, no concurrent
//!   readers racing appliers. The shard directory the router consults is the
//!   sim-omniscient membership view (`ClusterSim::shards` / `leader_of`), so
//!   the raft read-index *leadership-confirmation heartbeat round* is NOT
//!   modeled: the omniscient directory already never names a stale
//!   lower-term leader, and the applied-vs-commit check is exact because the
//!   whole attestation happens under one `&` borrow of the sim (nothing can
//!   advance mid-read). The OTHER read-index precondition — the leader has a
//!   committed entry of its current term (Raft §6.4) — IS enforced (DR-8);
//!   the directory does not close that hazard, because a legitimately
//!   elected new leader can hold a commit index that predates acked entries.
//!   A networked read-index is a later stage.
//! - **Reads reconstruct committed truth by WAL replay ONLY** (ORCH-001 /
//!   OWN-001): this module consumes exactly three routing inputs from the
//!   cluster (`shards`, `leader_of`+`commit_index_of`/`applied_of`,
//!   `wal_store_of`) and NO Kernel truth accessor; the served bytes come from
//!   folding the replica's durable WAL ([`ShardProjection`], RCR-023). No
//!   code path from here reaches `Kernel::commit`, and every read takes only
//!   an immutable borrow of the sim — queries structurally never write.
//! - **Scatter-gather is a deterministic sequential fan-out** in this stage
//!   (design §3.8 names concurrent sub-reads; no concurrency is claimed
//!   here) and an explicitly **non-atomic union** with a per-shard version
//!   vector — never a fabricated global version (IDR-001: no cross-shard
//!   atomicity in v1). The fan-out API and [`GatheredRead`] are additive
//!   contract surface via this RCR (design §3.3/§6.2: the frozen single-shard
//!   `Query` trait cannot represent a merged result). The fan-out API takes
//!   ONE tenant and every sub-read routes on the TYPED `ShardId` from the
//!   directory — never re-parsed from the ambiguous `"tenant/workspace"`
//!   text form (RCR-023 DR-2's `/`-in-part ambiguity; RCR-024 DR-9) — so a
//!   gather labeled for tenant T carries only shards whose typed tenant
//!   equals T. The frozen text-scoped entry points keep RCR-023 DR-2's
//!   documented first-`/` parse (trusted single host, as at v1.0).
//! - **Read-your-writes** ([`ClusterQuery::read_at_least`], [`floor_of`]) is
//!   the OQ-5 additive carrier: the frozen trait signatures are untouched; the
//!   floor rides a new method and a new error type, never a new variant of the
//!   frozen `QueryError` enum (which would be a breaking v2.0 change).
//! - **No authN/authZ** (OQ-1 — trusted single host, as at v1.0); **no LCW
//!   read views** (OQ-8 unresolved; nothing here touches LCW).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use arves_consensus::{NodeId, ShardId, TenantId, WorkspaceId};
use arves_kernel::cluster::ClusterSim;
use arves_kernel::TruthRef;
use arves_persistence::{ShardKey as WalShardKey, WalStore};

use crate::projection::{parse_shard_text, ShardProjection};
use crate::{
    Millis, Projection, ProjectionId, Query, QueryError, QueryResult, ReadScope, ReadTier,
    ShardKey, StalenessBound, Version,
};

/// Sentinel `observed_lag` reported when a bounded-staleness read is refused
/// because the replica's lag **cannot be attested in milliseconds at all**:
/// no safe time↔index mapping exists yet (design OQ-2 — its own IDR is
/// pending), so the only honest statement is "unknown, treat as unbounded".
/// This is deliberately NOT a measured value.
pub const LAG_UNATTESTABLE: Millis = Millis::MAX;

/// Tier/bound consistency rule shared by every entry point (frozen
/// `QueryError::MalformedScope` doc): `BoundedStaleness` requires a bound;
/// the other tiers must not carry one. Checked BEFORE any routing or I/O
/// (design §3.2).
fn tier_bound_consistent(tier: ReadTier, bound: Option<StalenessBound>) -> bool {
    match tier {
        ReadTier::BoundedStaleness => bound.is_some(),
        ReadTier::Linearizable | ReadTier::Eventual => bound.is_none(),
    }
}

/// The first trace position that INCLUDES the commit named by `truth`: a fold
/// with `applied() >= floor_of(truth)` provably reflects that write (RCR-024
/// read-your-writes vocabulary; `TruthRef.index` is the record's dense WAL
/// offset, so the position after it is `index + 1` — RCR-023 DR-4).
pub fn floor_of(truth: &TruthRef) -> Version {
    truth.index.0 + 1
}

/// Why a [`ClusterQuery::read_at_least`] (read-your-writes) request failed.
///
/// Additive error TYPE (design OQ-5: the floor carrier is additive contract
/// surface). Deliberately not a new variant of the frozen [`QueryError`] enum
/// — widening that enum would break exhaustive matches (a v2.0 change).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FloorReadError {
    /// The serving replica's fold has not reached the requested floor: the
    /// caller's write is quorum-committed truth, but THIS replica has not
    /// applied it yet. Retry here later, or read at a current replica.
    BelowFloor {
        /// The trace position the caller required (see [`floor_of`]).
        floor: Version,
        /// The trace position the serving replica's fold actually reached.
        applied: Version,
    },
    /// The underlying single-shard read failed before the floor was checked.
    Query(QueryError),
}

/// Merged result of a tenant-internal scatter-gather read: an explicitly
/// **non-atomic union** (design §3.7). Each contributing shard reports its own
/// fold version; there is deliberately NO global version field, because no
/// cross-shard snapshot point exists in v1 (IDR-001: single-shard atomicity
/// only). Additive contract surface via RCR-024 (design §3.3/§6.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GatheredRead {
    /// The single tenant every part belongs to: the fan-out planner takes one
    /// tenant and every sub-read routes on the typed `ShardId` (never
    /// re-parsed text — RCR-024 DR-9), so each part's shard has exactly this
    /// tenant as its typed identity.
    pub tenant: String,
    /// The tier every sub-read was admitted at (never stronger than requested).
    pub served_tier: ReadTier,
    /// Per-shard version vector: the fold trace position of EVERY shard the
    /// fan-out scanned (canonical `"tenant/workspace"` text keys, in
    /// deterministic ascending order). This makes cross-shard version skew
    /// visible instead of hiding it (design §3.12).
    pub versions: BTreeMap<ShardKey, Version>,
    /// The shards where the projection id was found, with each shard's own
    /// [`Projection`]. A shard where the id is absent contributes to
    /// [`GatheredRead::versions`] only — absence in one workspace is not a
    /// failure of the union.
    pub parts: BTreeMap<ShardKey, Projection<Vec<u8>>>,
}

/// A distributed read handle bound to ONE serving replica of the I2 cluster
/// (client-to-replica semantics, mirroring `ClusterKernel`). Implements the
/// frozen [`Query`] trait with the distributed tier semantics described in the
/// module doc, plus the additive read-your-writes and scatter-gather surfaces.
///
/// Read-only by construction: every method takes `&self` and only ever takes
/// an IMMUTABLE borrow of the shared [`ClusterSim`] — a query cannot tick the
/// cluster, cannot reach any commit path, and cannot mutate consensus state
/// (Layer Matrix Query row: "Writes: NOTHING").
pub struct ClusterQuery {
    node: NodeId,
    cluster: Rc<RefCell<ClusterSim>>,
}

impl ClusterQuery {
    /// Bind a read handle to replica `node` of `cluster`.
    ///
    /// # Panics
    /// If `node` is not a replica of the cluster (harness programming error).
    pub fn new(node: NodeId, cluster: Rc<RefCell<ClusterSim>>) -> Self {
        assert!(
            cluster.borrow().node_ids().contains(&node),
            "ClusterQuery must bind an existing replica"
        );
        Self { node, cluster }
    }

    /// Route, admit, fold, serve — the one shared read path.
    ///
    /// Order (design §3.2/§3.7): (1) scope validation before any routing or
    /// I/O; (2) resolve exactly ONE shard from the canonical text form and
    /// check the shard directory (SHARD-001); (3) tier admission against the
    /// consensus metadata (module-doc ladder); (4) fold the bound replica's
    /// durable WAL and serve the closure over it.
    fn serve<R>(
        &self,
        scope: &ReadScope,
        f: impl FnOnce(&ShardProjection) -> QueryResult<R>,
    ) -> QueryResult<R> {
        if !tier_bound_consistent(scope.tier, scope.bound) {
            return Err(QueryError::MalformedScope);
        }
        let wshard = parse_shard_text(&scope.shard)
            .ok_or_else(|| QueryError::UnknownShard { shard: scope.shard.clone() })?;
        let sid = ShardId::new(
            TenantId(wshard.tenant.clone()),
            WorkspaceId(wshard.workspace.clone()),
        );
        self.serve_typed(&sid, &wshard, scope.tier, scope.bound, f)
    }

    /// Typed-key core of [`Self::serve`] (RCR-024 DR-9): routing happens on
    /// the exact `(tenant, workspace)` identity. Text parsing exists ONLY at
    /// the frozen `ReadScope` entry points ([`Self::serve`]); an internal
    /// caller that already holds the typed `ShardId` (the gather fan-out)
    /// enters here directly, so the ambiguous `"tenant/workspace"` text form
    /// (RCR-023 DR-2: `/` is legal inside a part) can never misdirect a
    /// sub-read to another tenant's shard.
    fn serve_typed<R>(
        &self,
        sid: &ShardId,
        wshard: &WalShardKey,
        tier: ReadTier,
        bound: Option<StalenessBound>,
        f: impl FnOnce(&ShardProjection) -> QueryResult<R>,
    ) -> QueryResult<R> {
        let sim = self.cluster.borrow();
        if !sim.shards().contains(sid) {
            // SHARD-001: no registered group, no route — never a fallback.
            return Err(QueryError::UnknownShard {
                shard: format!("{}/{}", wshard.tenant, wshard.workspace),
            });
        }
        self.admit(&sim, sid, tier, bound)?;
        let proj = self.local_fold(&sim, wshard)?;
        f(&proj)
    }

    /// Tier admission (the module-doc consistency ladder, exactly).
    fn admit(
        &self,
        sim: &ClusterSim,
        sid: &ShardId,
        tier: ReadTier,
        bound: Option<StalenessBound>,
    ) -> QueryResult<()> {
        match tier {
            // AP observability: always admitted, staleness labeled (IDR-005).
            ReadTier::Eventual => Ok(()),
            // CP truth: read-index against the current leader, or refuse.
            ReadTier::Linearizable => {
                let leader = sim.leader_of(sid).ok_or(QueryError::LeaderUnavailable)?;
                if !sim.has_committed_in_current_term(&leader, sid) {
                    // Raft §6.4 read-index PRECONDITION (RCR-024 DR-8): a
                    // freshly elected leader's commit index may still EXCLUDE
                    // prior-term quorum-committed (acked) entries — the
                    // §5.4.2 term guard refuses to count them and RCR-019
                    // DR-2 appends no election no-op — so it is NOT yet a
                    // valid read-index. Refuse rather than silently miss an
                    // acked write (CP: availability degrades, honesty never).
                    return Err(QueryError::LeaderUnavailable);
                }
                let read_index = sim.commit_index_of(&leader, sid);
                if sim.applied_of(&self.node, sid) >= read_index {
                    Ok(())
                } else {
                    // This replica cannot confirm currency with the leader —
                    // the frozen contract's exact meaning for this refusal.
                    Err(QueryError::LeaderUnavailable)
                }
            }
            // Follower read, admitted ONLY on provably-zero lag (OQ-2 pending).
            ReadTier::BoundedStaleness => {
                let requested = bound.expect("validated: bounded scope carries a bound");
                // The zero-lag proof is measured against the leader's commit
                // index, so the SAME DR-8 precondition gates it: an invalid
                // read-index proves no lag bound at all.
                let zero_lag_proven = sim
                    .leader_of(sid)
                    .filter(|l| sim.has_committed_in_current_term(l, sid))
                    .map(|l| sim.applied_of(&self.node, sid) >= sim.commit_index_of(&l, sid))
                    .unwrap_or(false);
                if zero_lag_proven {
                    // lag = 0 entries = 0 ms ≤ any bound: no clock needed.
                    Ok(())
                } else {
                    Err(QueryError::StalenessBoundExceeded {
                        requested,
                        observed_lag: LAG_UNATTESTABLE,
                    })
                }
            }
        }
    }

    /// Fold the bound replica's durable WAL for `wshard` (WAL replay only —
    /// ORCH-001). A shard that is registered in the directory but has no WAL
    /// at this replica yet simply has zero committed truth here: the honest
    /// result is the empty fold at position 0, not an error (RCR-024 DR-5).
    fn local_fold(&self, sim: &ClusterSim, wshard: &WalShardKey) -> QueryResult<ShardProjection> {
        let store = sim.wal_store_of(&self.node);
        if store.shards().contains(wshard) {
            ShardProjection::at_head(&store, wshard)
        } else {
            Ok(ShardProjection::empty(wshard.clone()))
        }
    }

    /// Single-shard read that additionally requires the serving fold to have
    /// reached `floor` — the read-your-writes carrier (design §3.12 / OQ-5):
    /// pass [`floor_of`] of the `TruthRef` your commit returned and the result
    /// provably reflects that write.
    ///
    /// The floor is checked BEFORE presence: a lagging replica that has not
    /// reached the floor answers [`FloorReadError::BelowFloor`] (retry/route),
    /// never a false `NotFound` for a write that exists as committed truth.
    ///
    /// # Errors
    /// [`FloorReadError::BelowFloor`] when the replica's fold is behind
    /// `floor`; otherwise [`FloorReadError::Query`] with the same errors as
    /// [`Query::read`].
    pub fn read_at_least(
        &self,
        scope: &ReadScope,
        id: &ProjectionId,
        floor: Version,
    ) -> Result<Projection<Vec<u8>>, FloorReadError> {
        let (applied, value) = self
            .serve(scope, |proj| {
                Ok((proj.applied(), proj.get(id).map(|(v, _)| v.to_vec())))
            })
            .map_err(FloorReadError::Query)?;
        if applied < floor {
            return Err(FloorReadError::BelowFloor { floor, applied });
        }
        match value {
            Some(value) => Ok(Projection {
                id: id.clone(),
                observed_at: applied,
                served_tier: scope.tier,
                value,
            }),
            None => Err(FloorReadError::Query(QueryError::NotFound { id: id.clone() })),
        }
    }

    /// Tenant-internal scatter-gather (design §3.1 #4, §3.7): fan one
    /// single-shard sub-read per registered shard of ONE tenant, in
    /// deterministic ascending shard order, and merge into the explicitly
    /// non-atomic union [`GatheredRead`].
    ///
    /// - **Single-tenant by construction, typed routing (DR-9)**: the planner
    ///   takes a single `tenant` (no API accepts a tenant set) and every
    ///   sub-read routes on the typed `ShardId` selected from the directory —
    ///   never re-parsed from the ambiguous text form (RCR-023 DR-2: `/` is
    ///   legal inside a part) — so every part served under this tenant label
    ///   comes from a shard whose typed tenant is exactly `tenant`. The text
    ///   keys in the result are display labels only, never routing inputs.
    /// - **Partial failure fails the WHOLE read** (RCR-024 DR-3, resolving
    ///   design OQ-4 to the option representable without widening the frozen
    ///   error enum): the first failing sub-read's `QueryError` is returned
    ///   and no partial union is ever emitted silently. A per-shard `NotFound`
    ///   is NOT a failure — that shard contributes its version only.
    /// - **Deterministic sequential fan-out**: sub-reads run in shard order
    ///   under one borrow of the sim; no concurrency is claimed (module doc).
    ///
    /// # Errors
    /// [`QueryError::MalformedScope`] for an inconsistent tier/bound;
    /// [`QueryError::UnknownShard`] (carrying the bare tenant text) when the
    /// directory holds no shard for `tenant`; otherwise the first sub-read
    /// failure, verbatim.
    pub fn gather_read(
        &self,
        tenant: &str,
        tier: ReadTier,
        bound: Option<StalenessBound>,
        id: &ProjectionId,
    ) -> QueryResult<GatheredRead> {
        if !tier_bound_consistent(tier, bound) {
            return Err(QueryError::MalformedScope);
        }
        let sids: Vec<ShardId> = self
            .cluster
            .borrow()
            .shards()
            .into_iter()
            .filter(|s| s.tenant.0 == tenant)
            .collect(); // directory order (BTreeMap keys): deterministic
        if sids.is_empty() {
            // A tenant with no registered shard is not routable; the bare
            // tenant text rides the least-wrong frozen variant (RCR-024 DR-4).
            return Err(QueryError::UnknownShard { shard: tenant.to_string() });
        }
        let mut versions = BTreeMap::new();
        let mut parts = BTreeMap::new();
        for sid in sids {
            let text: ShardKey = format!("{}/{}", sid.tenant.0, sid.workspace.0);
            // DR-9: route on the TYPED identity we already hold — never
            // round-trip through the text form, whose first-`/` parse could
            // resolve a `/`-bearing tenant to ANOTHER tenant's shard.
            let wshard = WalShardKey {
                tenant: sid.tenant.0.clone(),
                workspace: sid.workspace.0.clone(),
            };
            // Any sub-read failure fails the whole gather (DR-3 / OQ-4).
            let (applied, value) = self.serve_typed(&sid, &wshard, tier, bound, |proj| {
                Ok((proj.applied(), proj.get(id).map(|(v, _)| v.to_vec())))
            })?;
            versions.insert(text.clone(), applied);
            if let Some(value) = value {
                parts.insert(
                    text,
                    Projection {
                        id: id.clone(),
                        observed_at: applied,
                        served_tier: tier,
                        value,
                    },
                );
            }
        }
        Ok(GatheredRead { tenant: tenant.to_string(), served_tier: tier, versions, parts })
    }
}

impl Query for ClusterQuery {
    /// Raw committed payload bytes, verbatim (RCR-023 DR-3 / design OQ-7 safe
    /// default: the read path never interprets meaning).
    type View = Vec<u8>;

    fn read(
        &self,
        scope: &ReadScope,
        id: &ProjectionId,
    ) -> QueryResult<Projection<Self::View>> {
        self.serve(scope, |proj| match proj.get(id) {
            Some((value, _committed_at)) => Ok(Projection {
                id: id.clone(),
                // The trace position of the serving replica's fold: the whole
                // served state is reproducible by replaying to it (§3.11).
                observed_at: proj.applied(),
                // Exactly the requested tier — never stronger (frozen
                // guarantee; this fabric never re-labels).
                served_tier: scope.tier,
                value: value.to_vec(),
            }),
            None => Err(QueryError::NotFound { id: id.clone() }),
        })
    }

    fn exists(&self, scope: &ReadScope, id: &ProjectionId) -> QueryResult<bool> {
        self.serve(scope, |proj| Ok(proj.get(id).is_some()))
    }

    fn latest_version(
        &self,
        scope: &ReadScope,
        id: &ProjectionId,
    ) -> QueryResult<Version> {
        self.serve(scope, |proj| {
            proj.latest(id).ok_or_else(|| QueryError::NotFound { id: id.clone() })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arves_kernel::{CommitIndex, ContentHash, ShardKey as KShardKey};

    #[test]
    fn floor_of_names_the_first_position_including_the_commit() {
        let tr = TruthRef {
            shard: KShardKey::new("t", "w").expect("shard"),
            content: ContentHash(vec![1]),
            index: CommitIndex(0),
        };
        // Offset 0 is reflected by any fold with applied >= 1 (RCR-023 DR-4).
        assert_eq!(floor_of(&tr), 1);
    }

    #[test]
    fn tier_bound_rule_matches_the_frozen_malformed_scope_contract() {
        let b = Some(StalenessBound::new(5));
        assert!(tier_bound_consistent(ReadTier::BoundedStaleness, b));
        assert!(!tier_bound_consistent(ReadTier::BoundedStaleness, None));
        assert!(tier_bound_consistent(ReadTier::Linearizable, None));
        assert!(!tier_bound_consistent(ReadTier::Linearizable, b));
        assert!(tier_bound_consistent(ReadTier::Eventual, None));
        assert!(!tier_bound_consistent(ReadTier::Eventual, b));
    }
}

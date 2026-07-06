//! RCR-023 (I3 Stage 1) — the single-node QUERY CORE behind the frozen contract.
//!
//! Approved design: `docs/design/I3_Distributed_Query_Design.md`. This module
//! implements the design's single-node read path (§3.5 lifecycle, §3.6 state
//! model, §3.11 replay) behind the frozen v1.0 `arves-query` contract types —
//! every frozen type/trait in `lib.rs` is byte-unchanged (the RCR-008/019
//! additive pattern).
//!
//! # What this is
//!
//! - [`ShardProjection`] — a **read-only, disposable, derived** fold of ONE
//!   shard's committed WAL prefix: `Proj(shard, v) = fold(apply, ∅, WAL[shard][0..v])`
//!   (design §3.6). It is a pure function of the committed decision trace
//!   (IDR-005, ORCH-003) and is rebuilt — never repaired — on any doubt
//!   (design §3.5 step 4).
//! - [`WalQuery`] — the first implementation of the frozen [`Query`] trait: a
//!   single-node projection replica serving `read` / `exists` / `latest_version`
//!   over per-shard folds, with tenant/workspace scoping (SHARD-001) and the
//!   three IDR-001 read tiers in their **single-node degenerate** form (see
//!   "Honest scope" below).
//!
//! Reference semantics: RCR-010's `QueryProjection` in `arves-conformance`
//! (which stays there, unmodified, as a conformance probe per design §2); this
//! module is the additive `arves-query` implementation the design's §6.2 row 1
//! prescribes. Like RCR-010 it reads by WAL replay only — there is NO Kernel
//! read hook (ORCH-001 / OWN-001, design §6.1 #2) and no code path from here
//! can reach `Kernel::commit` (this crate does not even depend on the kernel).
//!
//! # Read-only by type (OWN-001 / Layer-Matrix "Writes: NOTHING")
//!
//! The only persistence operations this module invokes are the shared-reference
//! ones: `WalStore::open(&self)`, `Wal::replay_from(&self)`, `Wal::head(&self)`,
//! `Wal::earliest(&self)`. Every WAL handle is consumed behind `&` (see
//! [`fold_range`]), so the `&mut self` write surface (`append`,
//! `install_snapshot`, `compact`) is uncallable from this module by
//! construction. No public item here returns a write handle, commit token, or
//! `&mut` view of anything durable.
//!
//! # Honest scope (single-node; nothing distributed is claimed)
//!
//! This is the design's **single-node degenerate form** (design §3.4
//! "Milestone dependency"): ONE process reading ITS OWN store. There is no
//! query routing fabric, no replica set, no network, no scatter-gather, no LCW
//! read views (OQ-8 unresolved). Tier semantics are accordingly degenerate:
//!
//! - **Linearizable** — catch the fold up to the local WAL head, then serve.
//!   On a single node the local committed log IS the commit index, so
//!   catching up to it is the degenerate of the IDR-001 read-index protocol
//!   ("replica serves at ≥ the leader-confirmed index" — here the sole replica
//!   and the leader-confirmed log coincide). The real leader round-trip is a
//!   later I3 stage.
//! - **BoundedStaleness** — catch up to head, then serve: the single-node core
//!   proves `lag = 0ms ≤ bound` by *being current*. Consequently
//!   [`QueryError::StalenessBoundExceeded`] is **unreachable** in this core by
//!   construction; the general follower lag-attestation mechanism (time↔index
//!   mapping) is design OQ-2 and awaits its own IDR before any distributed
//!   stage.
//! - **Eventual** — serve the standing fold WITHOUT refreshing it: observably
//!   stale relative to the store if commits happened since, but never *wrong*
//!   for the [`Projection::observed_at`] it reports (frozen contract wording).
//!
//! `served_tier` is always exactly the requested tier — never stronger
//! (frozen `Projection` guarantee).

use std::collections::BTreeMap;
use std::sync::Mutex;

use arves_persistence::{
    ContentId, RecordKind, ReplayCursor, ShardKey as WalShardKey, Wal, WalRecord, WalStore,
};

use crate::{
    Projection, ProjectionId, Query, QueryError, QueryResult, ReadScope, ReadTier, ShardKey,
    Version,
};

// ---------------------------------------------------------------------------
// Shard-key text form (contract `ShardKey` = opaque String)
// ---------------------------------------------------------------------------

/// Canonical text form of a `(tenant, workspace)` shard key for the frozen
/// contract's opaque [`ShardKey`] string: `"tenant/workspace"` (the A-004
/// shard key, SHARD-001).
///
/// RCR-023 DR-2: the form is parsed at the FIRST `/`, so a tenant containing
/// `/` is not routable through this text form in the Stage-1 core — and worse
/// than unroutable: `arves-kernel::ShardKey::new` permits `/` inside parts, so
/// two DISTINCT committed shards (tenant=`a/b`, ws=`c`) and (tenant=`a`,
/// ws=`b/c`) share the text `a/b/c`, which this parser always resolves to
/// `(a, b/c)` — a caller intending tenant `a/b` is silently served tenant
/// `a`'s shard if it exists (a **misdirected read**, not a cross-shard leak:
/// WAL keys stay distinct and the fold never mixes records; v1.0's
/// trusted-single-host model already lets any caller name any shard). A typed
/// contract key, or rejecting `/`-bearing parts in `ShardKey::new`, is future
/// RCR surface.
pub fn shard_scope_text(shard: &WalShardKey) -> ShardKey {
    format!("{}/{}", shard.tenant, shard.workspace)
}

/// Parse the canonical `"tenant/workspace"` text form back into a persistence
/// shard key. Returns `None` when the text cannot name a shard (no `/`, or an
/// empty part) — the caller surfaces that as [`QueryError::UnknownShard`].
pub(crate) fn parse_shard_text(text: &str) -> Option<WalShardKey> {
    let (tenant, workspace) = text.split_once('/')?;
    if tenant.is_empty() || workspace.is_empty() {
        return None;
    }
    Some(WalShardKey { tenant: tenant.to_string(), workspace: workspace.to_string() })
}

/// The [`QueryError`] for a shard this replica cannot serve — unknown,
/// unparseable, or (RCR-023 DR-7) not faithfully reconstructible from the
/// retained log (compacted prefix without a query-side snapshot facility, or a
/// durable-layer replay fault). `UnknownShard` ("not served here") is the
/// least-wrong member of the frozen error enum for all of these; a richer
/// read-fault variant is future RCR surface.
fn unservable(shard: &WalShardKey) -> QueryError {
    QueryError::UnknownShard { shard: shard_scope_text(shard) }
}

// ---------------------------------------------------------------------------
// ProjectionId (content-addressable, ORCH-004)
// ---------------------------------------------------------------------------

/// The content-addressable [`ProjectionId`] of a committed record: the
/// lowercase-hex encoding of its [`ContentId`] bytes (RCR-023 DR-3, resolving
/// design OQ-7 to the named safe default — "a projection is named by *what* it
/// is", frozen contract / ORCH-004).
pub fn projection_id_for(content: &ContentId) -> ProjectionId {
    let mut s = String::with_capacity(content.0.len() * 2);
    for b in &content.0 {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// ---------------------------------------------------------------------------
// ShardProjection — the deterministic per-shard fold
// ---------------------------------------------------------------------------

/// One materialized entry of a shard projection: the raw committed payload
/// bytes (never interpreted — design OQ-7 safe default) plus the commit offset
/// at which this value entered the trace.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    /// Commit offset of the record that produced this value (the Kernel's
    /// `CommitIndex` for the same record).
    committed_at: Version,
    /// Raw content-address bytes (kept so the fold digest can hash the same
    /// tuple basis as the Kernel's `truth_hash`).
    content: Vec<u8>,
    /// Raw committed payload bytes, verbatim from the WAL record.
    value: Vec<u8>,
}

/// A read-only projection of ONE shard, built exclusively by deterministic
/// WAL replay: `fold(apply, ∅, WAL[shard][0..applied])` (design §3.6/§3.11;
/// IDR-005, ORCH-003).
///
/// The projection is **derived, disposable state** (Layer Matrix: Query owns
/// "Read projections/views" only): it is a pure function of the committed
/// prefix, holds nothing durable of its own, and is repaired by being rebuilt
/// (design §3.5 step 4). Two builds over the same prefix are equal
/// (`PartialEq`), which is exactly the ORCH-003 replica-equality property.
///
/// Version vocabulary (RCR-023 DR-4):
/// - [`ShardProjection::applied`] is the **trace position** — the number of
///   offsets folded in; the projection reflects `WAL[0..applied)`.
/// - Each entry's version ([`ShardProjection::latest`]) is the **commit
///   offset** of the record that produced its current value, always
///   `< applied`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShardProjection {
    shard: WalShardKey,
    /// Trace position: offsets `[0, applied)` have been folded in.
    applied: Version,
    entries: BTreeMap<ProjectionId, Entry>,
}

impl ShardProjection {
    /// The empty fold (`∅`) for `shard`: position 0, no entries.
    pub fn empty(shard: WalShardKey) -> Self {
        Self { shard, applied: 0, entries: BTreeMap::new() }
    }

    /// The shard this projection is scoped to (SHARD-001; immutable).
    pub fn shard(&self) -> &WalShardKey {
        &self.shard
    }

    /// Trace position the fold has reached: the projection reflects exactly
    /// `WAL[shard][0..applied)`. This is the [`Projection::observed_at`] a
    /// read served from this fold reports (RCR-023 DR-4).
    pub fn applied(&self) -> Version {
        self.applied
    }

    /// Number of materialized entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the projection holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One deterministic fold step (RCR-023 DR-5).
    ///
    /// - A record of a FOREIGN shard never enters the fold and never advances
    ///   it — structural SHARD-001 isolation at the lowest level.
    /// - Only [`RecordKind::Outcome`] materializes an entry;
    ///   `Membership` / `SnapshotMarker` / `Barrier` advance the position only
    ///   (they carry no truth).
    /// - Records must arrive in offset order (the WAL's total per-shard order,
    ///   IDR-005); feeding the fold out of order is a caller bug.
    ///
    /// # Panics
    /// If an own-shard record arrives out of offset order. This is a hard
    /// assert (release builds included): a silently wrong fold on the
    /// determinism-critical step would be partial truth — lossless-or-loud
    /// (RCR-023 DR-7 house style).
    pub fn apply(&mut self, rec: &WalRecord) {
        if rec.shard != self.shard {
            return; // SHARD-001: a foreign-shard record NEVER enters this fold.
        }
        assert_eq!(
            rec.offset, self.applied,
            "ShardProjection::apply: fold must consume the trace in offset order"
        );
        self.applied = rec.offset + 1;
        if rec.kind == RecordKind::Outcome {
            self.entries.insert(
                projection_id_for(&rec.content),
                Entry {
                    committed_at: rec.offset,
                    content: rec.content.0.clone(),
                    value: rec.payload.clone(),
                },
            );
        }
    }

    /// Deterministic **snapshot-at-index** build: fold `WAL[shard][0..version)`
    /// from the store. Two calls with the same arguments over the same
    /// committed prefix return equal projections (ORCH-003), and any served
    /// read is reproducible after the fact by rebuilding at its `observed_at`
    /// (design §3.11).
    ///
    /// # Errors
    /// [`QueryError::UnknownShard`] if the store holds no WAL for `shard`, or
    /// if the retained log cannot faithfully reproduce the fold (compacted
    /// prefix — Stage 1 has no query-side snapshot bootstrap, RCR-023 DR-7).
    ///
    /// # Panics
    /// If `version` exceeds the committed head: the trace does not reach that
    /// point, and fabricating a shorter fold silently would be partial truth —
    /// this fails loudly instead (lossless-or-loud house style, RCR-023 DR-7).
    pub fn at_version<S: WalStore>(
        store: &S,
        shard: &WalShardKey,
        version: Version,
    ) -> QueryResult<Self> {
        ensure_known(store, shard)?;
        let wal = store.open(shard).map_err(|_| unservable(shard))?;
        assert!(
            version <= wal.head(),
            "ShardProjection::at_version: version {version} beyond committed head {} — \
             refusing to fabricate a trace point that does not exist",
            wal.head()
        );
        let mut p = Self::empty(shard.clone());
        fold_range(&mut p, &wal, version)?;
        Ok(p)
    }

    /// Build the projection at the store's current committed head (the
    /// bootstrap of design §3.5 step 1, replay-only form — RCR-010 semantics).
    ///
    /// # Errors
    /// As for [`ShardProjection::at_version`].
    pub fn at_head<S: WalStore>(store: &S, shard: &WalShardKey) -> QueryResult<Self> {
        ensure_known(store, shard)?;
        let wal = store.open(shard).map_err(|_| unservable(shard))?;
        let head = wal.head();
        let mut p = Self::empty(shard.clone());
        fold_range(&mut p, &wal, head)?;
        Ok(p)
    }

    /// Catch the fold up to the store's current committed head by applying the
    /// WAL suffix `[applied..head)` (design §3.5 step 2). Checkpoint ⊕ suffix
    /// ≡ full replay: a projection built at version `v` and caught up equals a
    /// fresh build at head (the §3.11 snapshot-equivalence obligation in its
    /// Stage-1 form; proven by test).
    ///
    /// # Errors
    /// [`QueryError::UnknownShard`] if the shard is absent or the retained log
    /// no longer covers `[applied..)` (RCR-023 DR-7).
    pub fn catch_up<S: WalStore>(&mut self, store: &S) -> QueryResult<()> {
        ensure_known(store, &self.shard)?;
        let wal = store.open(&self.shard).map_err(|_| unservable(&self.shard))?;
        let head = wal.head();
        fold_range(self, &wal, head)
    }

    /// Payload bytes and commit offset of `id`'s current value, if present.
    /// Returning `&[u8]` (never `&mut`) keeps the observation read-only.
    pub fn get(&self, id: &ProjectionId) -> Option<(&[u8], Version)> {
        self.entries.get(id).map(|e| (e.value.as_slice(), e.committed_at))
    }

    /// Commit offset of `id`'s current value (the entry-level version of
    /// RCR-023 DR-4), if present.
    pub fn latest(&self, id: &ProjectionId) -> Option<Version> {
        self.entries.get(id).map(|e| e.committed_at)
    }

    /// Deterministic digest of the fold, hashing the same per-truth tuple the
    /// Kernel's `truth_hash` hashes — `(tenant, workspace, content bytes,
    /// commit offset, payload)` in commit order, FNV-1a 64 with the same seed.
    ///
    /// For a dedicated single-shard store with distinct content addresses this
    /// digest equals the committing Kernel's `truth_hash`, which is the
    /// executable "projection == kernel truth basis" replay-consistency proof
    /// (ORCH-003; proven by test, not by this doc).
    pub fn fold_digest(&self) -> u64 {
        let mut rows: Vec<&Entry> = self.entries.values().collect();
        rows.sort_by_key(|e| e.committed_at);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for e in rows {
            h = fnv1a_64(h, self.shard.tenant.as_bytes());
            h = fnv1a_64(h, self.shard.workspace.as_bytes());
            h = fnv1a_64(h, &e.content);
            h = fnv1a_64(h, &e.committed_at.to_le_bytes());
            h = fnv1a_64(h, &e.value);
        }
        h
    }
}

/// FNV-1a 64 (same constants as the Kernel's introspection hash, so the fold
/// digest and `truth_hash` share a basis).
fn fnv1a_64(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Refuse shards the store does not hold (frozen `WalStore::open` creates on
/// demand, so existence must be checked against `shards()` first).
fn ensure_known<S: WalStore>(store: &S, shard: &WalShardKey) -> QueryResult<()> {
    if store.shards().contains(shard) {
        Ok(())
    } else {
        Err(unservable(shard))
    }
}

/// Fold committed records `[p.applied .. upto)` into `p`, reading the WAL
/// strictly through its shared-reference (read-only) surface. Taking the WAL
/// as `&W` makes the `&mut self` write methods (`append`, `install_snapshot`,
/// `compact`) uncallable here — the type-level read-only argument of the
/// module doc.
fn fold_range<W: Wal>(p: &mut ShardProjection, wal: &W, upto: Version) -> QueryResult<()> {
    if upto <= p.applied {
        return Ok(()); // Nothing new; an older `upto` never rewinds a fold.
    }
    if wal.earliest() > p.applied {
        // The retained log no longer covers the next offset the fold needs
        // (compacted prefix): the full fold is not reproducible here.
        // Stage 1 has no query-side snapshot bootstrap (RCR-023 DR-7).
        return Err(unservable(&p.shard));
    }
    let mut cur = wal.replay_from(p.applied).map_err(|_| unservable(&p.shard))?;
    loop {
        match cur.next() {
            Ok(Some(rec)) => {
                if rec.offset >= upto {
                    break;
                }
                p.apply(&rec);
            }
            Ok(None) => break,
            Err(_) => return Err(unservable(&p.shard)),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// WalQuery — the frozen `Query` trait, implemented (single-node core)
// ---------------------------------------------------------------------------

/// A single-node projection replica implementing the frozen [`Query`] trait
/// over per-shard WAL-replay folds ([`ShardProjection`]).
///
/// - **Scope validation before any I/O** (design §3.2): an internally
///   inconsistent tier/bound combination is [`QueryError::MalformedScope`]
///   without touching the store.
/// - **Tenant/workspace scoping** (SHARD-001): every read resolves exactly one
///   shard from the scope's canonical `"tenant/workspace"` text form; a fold
///   structurally never contains a foreign shard's record.
/// - **Tiers** in their single-node degenerate form — see the module doc's
///   "Honest scope" for exactly what each tier means here and why
///   `StalenessBoundExceeded` is unreachable in this core.
/// - **Internally synchronized**: all methods are `&self` (frozen contract);
///   the per-shard folds live behind a `Mutex` so a shared `WalQuery` is safe
///   to use from multiple readers (design §3.8).
/// - **Bootstrap-then-lag** (RCR-023 DR-9): the first read touching a shard
///   bootstraps its fold to the then-current head; `Eventual` reads thereafter
///   serve the standing fold without refresh, so staleness is deterministic
///   and observable without any clock.
pub struct WalQuery<S: WalStore> {
    store: S,
    replicas: Mutex<BTreeMap<WalShardKey, ShardProjection>>,
}

impl<S: WalStore> WalQuery<S> {
    /// A query replica over `store`. Holds no durable state of its own
    /// (design §3.6): every fold is rebuildable from the store at any time.
    pub fn new(store: S) -> Self {
        Self { store, replicas: Mutex::new(BTreeMap::new()) }
    }

    /// Validate the scope, resolve the shard, admit per tier, and serve `f`
    /// over the (possibly refreshed) fold.
    fn serve<R>(
        &self,
        scope: &ReadScope,
        f: impl FnOnce(&ShardProjection) -> QueryResult<R>,
    ) -> QueryResult<R> {
        // 1. Scope validation BEFORE any I/O (design §3.2): BoundedStaleness
        //    requires a bound; the other tiers must not carry one (frozen
        //    `QueryError::MalformedScope` doc).
        let malformed = match scope.tier {
            ReadTier::BoundedStaleness => scope.bound.is_none(),
            ReadTier::Linearizable | ReadTier::Eventual => scope.bound.is_some(),
        };
        if malformed {
            return Err(QueryError::MalformedScope);
        }
        // 2. Resolve exactly one shard (SHARD-001).
        let shard = parse_shard_text(&scope.shard)
            .ok_or_else(|| QueryError::UnknownShard { shard: scope.shard.clone() })?;
        ensure_known(&self.store, &shard)?;
        // 3. Fetch-or-bootstrap the shard fold (RCR-023 DR-9).
        let mut replicas = self.replicas.lock().expect("replicas poisoned");
        if !replicas.contains_key(&shard) {
            let boot = ShardProjection::at_head(&self.store, &shard)?;
            replicas.insert(shard.clone(), boot);
        }
        let proj = replicas.get_mut(&shard).expect("fold just ensured");
        // 4. Tier admission (single-node degenerate semantics; module doc).
        match scope.tier {
            // Degenerate read-index: the local committed log IS the commit
            // index of the sole replica; catch up to it, then serve.
            ReadTier::Linearizable => proj.catch_up(&self.store)?,
            // Prove lag 0ms <= any bound by being current. The frozen
            // `StalenessBoundExceeded` refusal is unreachable here (OQ-2's
            // distributed attestation awaits its IDR).
            ReadTier::BoundedStaleness => proj.catch_up(&self.store)?,
            // Serve the standing fold without refresh: possibly stale, never
            // wrong for the `observed_at` it reports.
            ReadTier::Eventual => {}
        }
        f(proj)
    }
}

impl<S: WalStore> Query for WalQuery<S> {
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
                // The trace position the read reflects (RCR-023 DR-4): the
                // whole served fold is reproducible by replaying to it.
                observed_at: proj.applied(),
                // Exactly the requested tier — never stronger (frozen
                // guarantee; this core never re-labels).
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

// ---------------------------------------------------------------------------
// Unit tests (pure; no kernel, no I/O beyond MemWalStore)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn wshard(t: &str, w: &str) -> WalShardKey {
        WalShardKey { tenant: t.into(), workspace: w.into() }
    }

    fn rec(shard: &WalShardKey, offset: u64, content: &[u8], payload: &[u8]) -> WalRecord {
        WalRecord {
            shard: shard.clone(),
            offset,
            term: 1,
            kind: RecordKind::Outcome,
            content: ContentId(content.to_vec()),
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn shard_text_round_trip_and_rejects() {
        let sh = wshard("acme", "research");
        assert_eq!(shard_scope_text(&sh), "acme/research");
        assert_eq!(parse_shard_text("acme/research"), Some(sh));
        assert_eq!(parse_shard_text("noslash"), None);
        assert_eq!(parse_shard_text("/ws"), None);
        assert_eq!(parse_shard_text("tenant/"), None);
    }

    #[test]
    fn projection_id_is_lowercase_hex_of_content() {
        assert_eq!(projection_id_for(&ContentId(vec![0x00, 0xab, 0xff])), "00abff");
    }

    #[test]
    fn foreign_shard_record_never_enters_the_fold() {
        // SHARD-001 structural isolation at the fold step itself (DR-5).
        let acme = wshard("acme", "research");
        let globex = wshard("globex", "research");
        let mut p = ShardProjection::empty(acme.clone());
        p.apply(&rec(&acme, 0, b"a", b"acme-truth"));
        p.apply(&rec(&globex, 0, b"g", b"globex-truth")); // foreign: ignored
        assert_eq!(p.len(), 1);
        assert_eq!(p.applied(), 1);
        assert!(p.get(&projection_id_for(&ContentId(b"g".to_vec()))).is_none());
    }

    #[test]
    fn non_outcome_records_advance_position_but_carry_no_truth() {
        let sh = wshard("acme", "research");
        let mut p = ShardProjection::empty(sh.clone());
        let mut barrier = rec(&sh, 0, b"b", b"noise");
        barrier.kind = RecordKind::Barrier;
        p.apply(&barrier);
        assert_eq!(p.applied(), 1);
        assert!(p.is_empty());
    }
}

//! RCR-029 (I5 Stage 1) — the LCW SHARED-TRUTH SURFACE + the first
//! [`WorkingMemory`] / [`LiveWorkspace`] implementations.
//!
//! Approved design: `docs/design/I5_MultiAgent_Runtime_Design.md` — §3.1.2
//! (shared-truth concurrency: N agents over ONE per-shard committed truth
//! base), §3.6 state class 2 (agent working memory — LCW-owned, never truth),
//! §3.10 ("Working memory (LCW): rebuilt from Kernel truth + events — lossy by
//! design; it was never authoritative"), §5.3 (LCW node probe: "consistent
//! world view per scenario"). This is the design's first rung: the coherent,
//! versioned READ surface agents share, plus the working-memory cells it
//! hydrates.
//!
//! # What this module adds (additive; every frozen v1.0 type and trait
//! # signature in `lib.rs` is unchanged except one additive `#[non_exhaustive]`
//! # error variant, recorded in RCR-029)
//!
//! 1. [`MemWorkingMemory`] — the first implementation of the frozen
//!    [`WorkingMemory`] contract: deterministic in-memory cells, local
//!    revisions, optimistic conditions. Live, mutable, and **NOT truth**.
//! 2. [`MemLiveWorkspace`] — the first implementation of the frozen
//!    [`LiveWorkspace`] contract, enforcing the contract's "at most one active
//!    live view per shard" single-owner rule (OWN-001) structurally.
//! 3. [`WorldView`] — the Living-Cognitive-World shared-truth surface: a
//!    **read-only, versioned, coherent** view of ONE shard's committed truth,
//!    built exclusively by deterministic WAL replay
//!    (`view(shard, v) = fold(apply, ∅, WAL[shard][0..v))`). Two builds at the
//!    same commit index over the same committed prefix are EQUAL — across
//!    re-reads and across replicas (ORCH-003; proven in
//!    `tests/shared_world.rs`, including over the I2 cluster).
//!
//! # Ownership and layering honesty (OWN-001 / LAYER-001 / QUERY-001-proposed)
//!
//! - The [`WorldView`] is the LCW **working-memory hydration surface** over
//!   committed truth (Layer Matrix, LCW row: reads "Kernel truth, events").
//!   It is NOT a second external read API: read-only projections for external
//!   consumers remain the Query layer's (QUERY-001, PROPOSED — CCP-GATE
//!   required; cited as design intent only). The view is derived, disposable
//!   and never authoritative — exactly like Working Memory itself.
//! - A literal dependency on `arves-query` would be an UPWARD edge
//!   (LAYER-001 ranks: lcw 50 < query 60, checked in
//!   `arves-conformance/src/property_check.rs` BEFORE this module was written),
//!   so the view folds the WAL directly through `arves-persistence`
//!   (lcw 50 → persistence 20, downward) using the SAME deterministic
//!   replay-fold discipline I3's `ShardProjection` proved (RCR-023) and the
//!   same digest basis as the Kernel's `truth_hash` — "built on I3's
//!   projections" as semantics, not as a crate edge (RCR-029 DR-2).
//! - Read-only by type: every WAL handle is consumed behind `&` (the RCR-023
//!   argument) — the `&mut` write surface (`append`, `install_snapshot`,
//!   `compact`) is uncallable from this module by construction. No code path
//!   from this crate can reach `Kernel::commit` (this crate does not depend on
//!   the kernel).
//!
//! # Coherence (the Stage-1 property)
//!
//! The multi-agent shared world is coherent because it is a pure function of
//! the one per-shard committed log (IDR-001/IDR-005: the Raft log IS the WAL
//! IS the decision trace): `WorldView::at_version(store, shard, N)` is
//! identical for every replica holding the committed prefix `[0..N)` and for
//! every re-read — and it STAYS identical after further commits land (a
//! versioned view is stable at its version). N agents hydrating working
//! memory from the same `(shard, N)` see byte-identical worlds.
//!
//! # HONEST SCOPE
//!
//! In-process, deterministic; no network. No event-stream deltas (the Layer
//! Matrix "events" input is a later stage; this surface rebuilds from truth
//! alone). No cross-shard view (SHARD-001: no cross-shard live state). The
//! sharing topology per agent (isolated vs shared per workspace) is design
//! OQ-7 — this module provides the SHARED per-shard surface and lets each
//! agent hydrate its OWN [`MemWorkingMemory`]; nothing stronger is claimed.
//! LCW-001 remains PROPOSED (CCP-GATE required) and is cited as intent only.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use arves_persistence::{
    RecordKind, ReplayCursor, ShardKey as PShardKey, Wal, WalStore,
};

use crate::{
    LcwError, LcwResult, LiveState, LiveValue, LiveWorkspace, PutCondition, Revision, ShardKey,
    StateKey, WorkingMemory,
};

// ---------------------------------------------------------------------------
// Shard-key bridge (SHARD-001: one immutable (tenant, workspace) identity in
// two crates' shapes — same discipline as arves-kernel::cluster).
// ---------------------------------------------------------------------------

fn pshard(s: &ShardKey) -> PShardKey {
    PShardKey { tenant: s.tenant.clone(), workspace: s.workspace.clone() }
}

/// Lowercase hex of a byte slice (the content-address text form used as the
/// view's entry id and as the hydrated [`StateKey`] — same mapping as
/// `arves-query`'s `projection_id_for`, re-derived here to keep the layering
/// downward-only).
fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 0x0f) as usize] as char);
    }
    s
}

/// FNV-1a 64 (same constants as the Kernel's `truth_hash` and the I3 fold
/// digest, so the world digest shares their basis).
fn fnv1a_64(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ---------------------------------------------------------------------------
// MemWorkingMemory — first implementation of the frozen WorkingMemory contract
// ---------------------------------------------------------------------------

/// Deterministic in-memory [`WorkingMemory`]: the live, mutable scratch space
/// of ONE shard. **Never truth, never durable, never authoritative** — it may
/// be discarded and rebuilt from committed truth at any time (that rebuild is
/// [`WorldView::hydrate_into`]).
///
/// Revisions are LOCAL mutation ordering only (frozen [`Revision`] doc); the
/// first write of a cell lands at revision 1 (`Revision::ZERO.next()`).
#[derive(Debug)]
pub struct MemWorkingMemory {
    shard: ShardKey,
    cells: BTreeMap<String, (LiveValue, Revision)>,
}

impl MemWorkingMemory {
    /// A fresh, empty Working Memory bound to `shard` for its whole lifetime
    /// (SHARD-001: the binding is immutable; there is no rebind).
    pub fn new(shard: ShardKey) -> Self {
        Self { shard, cells: BTreeMap::new() }
    }

    /// Number of live cells currently held.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether no live cells are held.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl WorkingMemory for MemWorkingMemory {
    fn shard(&self) -> &ShardKey {
        &self.shard
    }

    fn get(&self, key: &StateKey) -> LcwResult<LiveState> {
        match self.cells.get(&key.0) {
            Some((value, revision)) => Ok(LiveState {
                key: key.clone(),
                value: value.clone(),
                revision: *revision,
            }),
            None => Err(LcwError::NotFound(key.clone())),
        }
    }

    fn put(
        &mut self,
        key: &StateKey,
        value: LiveValue,
        condition: PutCondition,
    ) -> LcwResult<Revision> {
        let current = self.cells.get(&key.0).map(|(_, r)| *r);
        match condition {
            PutCondition::Always => {}
            PutCondition::IfAbsent => {
                if let Some(rev) = current {
                    // Frozen contract: "the write is skipped for IfAbsent".
                    // RCR-029 DR-5: the skip is reported as Ok(current revision)
                    // — no mutation, no error (the cell simply already exists).
                    return Ok(rev);
                }
            }
            PutCondition::IfRevision(expected) => {
                let actual = current.unwrap_or(Revision::ZERO);
                if actual != expected {
                    return Err(LcwError::RevisionConflict {
                        key: key.clone(),
                        expected,
                        actual,
                    });
                }
            }
        }
        let next = current.unwrap_or(Revision::ZERO).next();
        self.cells.insert(key.0.clone(), (value, next));
        Ok(next)
    }

    fn evict(&mut self, key: &StateKey) -> LcwResult<bool> {
        Ok(self.cells.remove(&key.0).is_some())
    }

    fn contains(&self, key: &StateKey) -> bool {
        self.cells.contains_key(&key.0)
    }
}

// ---------------------------------------------------------------------------
// MemLiveWorkspace — first implementation of the frozen LiveWorkspace contract
// ---------------------------------------------------------------------------

/// Deterministic [`LiveWorkspace`]: hands out per-shard [`MemWorkingMemory`]
/// views and enforces the frozen contract's single-owner rule — "a given shard
/// has at most one active live view" (OWN-001, LCW-001 PROPOSED) — by tracking
/// open shards; a second `open` before `discard` is refused with
/// [`LcwError::AlreadyOpen`] (the RCR-029 additive variant).
///
/// The workspace itself retains NO live state: the moved-out view IS the live
/// state (Rust move semantics are the ownership mechanism), and `discard`
/// releases the shard's slot. Nothing here is truth or durable (ORCH-001/002).
#[derive(Debug, Default)]
pub struct MemLiveWorkspace {
    open: Mutex<BTreeSet<(String, String)>>,
}

impl MemLiveWorkspace {
    /// A workspace with no open shards.
    pub fn new() -> Self {
        Self::default()
    }
}

impl LiveWorkspace for MemLiveWorkspace {
    type Memory = MemWorkingMemory;

    fn open(&self, shard: &ShardKey) -> LcwResult<Self::Memory> {
        let key = (shard.tenant.clone(), shard.workspace.clone());
        let mut open = self.open.lock().expect("open-set poisoned");
        if !open.insert(key) {
            return Err(LcwError::AlreadyOpen { shard: shard.clone() });
        }
        Ok(MemWorkingMemory::new(shard.clone()))
    }

    fn discard(&self, shard: &ShardKey) -> LcwResult<()> {
        let key = (shard.tenant.clone(), shard.workspace.clone());
        self.open.lock().expect("open-set poisoned").remove(&key);
        // Discarding live state is ALWAYS safe w.r.t. the system of record:
        // truth is the Kernel's and the durable trace is Persistence's (frozen
        // contract doc). Discarding an un-opened shard is a lawful no-op.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WorldView — the coherent, versioned shared-truth surface
// ---------------------------------------------------------------------------

/// Why a [`WorldView`] could not be built. These are READ faults only — the
/// view has no write surface to fail on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldError {
    /// The store holds no WAL for this shard (SHARD-001: never a cross-shard
    /// fallback).
    UnknownShard(ShardKey),
    /// The requested commit index exceeds the committed head — the trace does
    /// not reach that point, and fabricating a shorter view silently would be
    /// partial truth (lossless-or-loud house style, surfaced as an error on
    /// this read-only surface).
    BeyondHead {
        /// The shard whose trace was too short.
        shard: ShardKey,
        /// The commit index the caller requested.
        requested: u64,
        /// The committed head actually available.
        head: u64,
    },
    /// The retained log cannot faithfully reproduce the fold (compacted prefix
    /// without a view-side snapshot bootstrap, or a durable-layer replay
    /// fault). Same honesty as RCR-023 DR-7.
    Unreadable {
        /// The shard whose log could not be folded.
        shard: ShardKey,
        /// Human-readable detail; not a stable API surface.
        detail: String,
    },
}

impl core::fmt::Display for WorldError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            WorldError::UnknownShard(s) => write!(f, "unknown shard {s}"),
            WorldError::BeyondHead { shard, requested, head } => write!(
                f,
                "world view for {shard} requested at commit index {requested} beyond head {head}"
            ),
            WorldError::Unreadable { shard, detail } => {
                write!(f, "world view for {shard} unreadable: {detail}")
            }
        }
    }
}

impl std::error::Error for WorldError {}

/// Convenience result alias for world-view construction.
pub type WorldResult<T> = Result<T, WorldError>;

/// One committed truth as seen by the world view.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WorldEntry {
    /// Commit offset at which this truth entered the shard's trace.
    committed_at: u64,
    /// Raw content-address bytes (kept so the digest hashes the same tuple
    /// basis as the Kernel's `truth_hash`).
    content: Vec<u8>,
    /// Raw committed payload bytes, verbatim from the WAL record.
    payload: Vec<u8>,
}

/// The **shared-truth surface**: a read-only, versioned, coherent view of ONE
/// shard's committed truth at an exact commit index, built exclusively by
/// deterministic WAL replay.
///
/// - **Coherent:** a pure function of the committed prefix `[0..observed_at)`;
///   equal (`PartialEq`) across re-reads and across replicas holding that
///   prefix (ORCH-003; IDR-005 total per-shard order).
/// - **Versioned:** `observed_at` is the exact commit index the view reflects;
///   a view never drifts — later commits are visible only through a NEW view.
/// - **Not truth:** the view is derived and disposable (OWN-001 — truth stays
///   the Kernel's; the durable trace stays Persistence's). It hydrates
///   Working-Memory cells; it does not replace the Query layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldView {
    shard: ShardKey,
    observed_at: u64,
    entries: BTreeMap<String, WorldEntry>,
}

impl WorldView {
    /// The empty world (`∅`) for `shard`: commit index 0, no truths.
    pub fn empty(shard: ShardKey) -> Self {
        Self { shard, observed_at: 0, entries: BTreeMap::new() }
    }

    /// Build the view of `shard` at exactly commit index `version`: the fold
    /// of `WAL[shard][0..version)`. Deterministic: two calls with the same
    /// arguments over the same committed prefix return EQUAL views — on any
    /// replica (the Stage-1 coherence property, proven by test).
    pub fn at_version<S: WalStore>(
        store: &S,
        shard: &ShardKey,
        version: u64,
    ) -> WorldResult<Self> {
        let ps = pshard(shard);
        if !store.shards().contains(&ps) {
            return Err(WorldError::UnknownShard(shard.clone()));
        }
        let wal = store.open(&ps).map_err(|e| WorldError::Unreadable {
            shard: shard.clone(),
            detail: format!("wal open: {e:?}"),
        })?;
        let head = wal.head();
        if version > head {
            return Err(WorldError::BeyondHead {
                shard: shard.clone(),
                requested: version,
                head,
            });
        }
        let mut view = Self::empty(shard.clone());
        if version == 0 {
            return Ok(view);
        }
        if wal.earliest() > 0 {
            // The retained log no longer covers offset 0 (compacted prefix):
            // the full fold is not reproducible here. No view-side snapshot
            // bootstrap exists in Stage 1 (RCR-029; same honesty as RCR-023
            // DR-7) — refuse loudly rather than serve partial truth.
            return Err(WorldError::Unreadable {
                shard: shard.clone(),
                detail: format!("compacted prefix [0..{}) not foldable", wal.earliest()),
            });
        }
        let mut cur = wal.replay_from(0).map_err(|e| WorldError::Unreadable {
            shard: shard.clone(),
            detail: format!("replay: {e:?}"),
        })?;
        loop {
            match cur.next() {
                Ok(Some(rec)) => {
                    if rec.offset >= version {
                        break;
                    }
                    if rec.shard != ps {
                        // SHARD-001: a foreign-shard record NEVER enters the
                        // world (defensive; a per-shard WAL should not yield one).
                        continue;
                    }
                    if rec.kind == RecordKind::Outcome {
                        view.entries.insert(
                            hex(&rec.content.0),
                            WorldEntry {
                                committed_at: rec.offset,
                                content: rec.content.0.clone(),
                                payload: rec.payload.clone(),
                            },
                        );
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(WorldError::Unreadable {
                        shard: shard.clone(),
                        detail: format!("replay step: {e:?}"),
                    })
                }
            }
        }
        view.observed_at = version;
        Ok(view)
    }

    /// Build the view at the store's current committed head.
    pub fn at_head<S: WalStore>(store: &S, shard: &ShardKey) -> WorldResult<Self> {
        let ps = pshard(shard);
        if !store.shards().contains(&ps) {
            return Err(WorldError::UnknownShard(shard.clone()));
        }
        let head = store
            .open(&ps)
            .map_err(|e| WorldError::Unreadable {
                shard: shard.clone(),
                detail: format!("wal open: {e:?}"),
            })?
            .head();
        Self::at_version(store, shard, head)
    }

    /// The shard this view is scoped to (SHARD-001; immutable).
    pub fn shard(&self) -> &ShardKey {
        &self.shard
    }

    /// The exact commit index this view reflects: the fold consumed
    /// `WAL[0..observed_at)`.
    pub fn observed_at(&self) -> u64 {
        self.observed_at
    }

    /// Number of committed truths visible in this view.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the view holds no truths.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Payload bytes and commit offset of the truth addressed by the
    /// lowercase-hex content id `id_hex`, if visible at this version.
    /// Returning `&[u8]` (never `&mut`) keeps the observation read-only.
    pub fn get(&self, id_hex: &str) -> Option<(&[u8], u64)> {
        self.entries.get(id_hex).map(|e| (e.payload.as_slice(), e.committed_at))
    }

    /// Whether a truth with content id `id_hex` is visible at this version.
    pub fn contains(&self, id_hex: &str) -> bool {
        self.entries.contains_key(id_hex)
    }

    /// Iterate `(content id hex, payload, committed_at)` in deterministic
    /// content-id order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &[u8], u64)> {
        self.entries
            .iter()
            .map(|(id, e)| (id.as_str(), e.payload.as_slice(), e.committed_at))
    }

    /// Deterministic digest of the view, hashing the SAME per-truth tuple the
    /// Kernel's `truth_hash` hashes — `(tenant, workspace, content bytes,
    /// commit offset, payload)` in commit order, FNV-1a 64 with the same seed.
    /// For a single-shard node this digest equals the Kernel's `truth_hash`:
    /// the shared world IS committed truth, basis-identical (proven by test).
    pub fn world_digest(&self) -> u64 {
        let mut rows: Vec<&WorldEntry> = self.entries.values().collect();
        rows.sort_by_key(|e| e.committed_at);
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for e in rows {
            h = fnv1a_64(h, self.shard.tenant.as_bytes());
            h = fnv1a_64(h, self.shard.workspace.as_bytes());
            h = fnv1a_64(h, &e.content);
            h = fnv1a_64(h, &e.committed_at.to_le_bytes());
            h = fnv1a_64(h, &e.payload);
        }
        h
    }

    /// Hydrate this view into a [`WorkingMemory`]: one live cell per visible
    /// truth, keyed by the truth's content-id hex, valued by its committed
    /// payload bytes (written with [`PutCondition::Always`], in commit order).
    /// Returns the number of cells written.
    ///
    /// This is the design-§3.10 rebuild path made executable: working memory
    /// is reconstructed FROM committed truth, is free to diverge afterwards,
    /// and its divergence never flows back (the view has no write surface and
    /// this crate cannot reach `Kernel::commit` — OWN-001 by construction).
    ///
    /// # Errors
    /// [`LcwError::WrongShard`] if `wm` is bound to a different shard
    /// (SHARD-001: no cross-shard live state).
    ///
    /// **Partial write on error (RCR-029 erratum E3, said out loud):** cells
    /// are written one by one in commit order, and if a `put` fails mid-walk
    /// the error is returned immediately with NO rollback — `wm` is left
    /// partially hydrated. This cannot happen with [`MemWorkingMemory`]
    /// (`PutCondition::Always` never fails there), but a foreign
    /// `WorkingMemory` whose `put` can fail must treat any `Err` from this
    /// method as "discard and re-hydrate", never as a usable world.
    pub fn hydrate_into<M: WorkingMemory>(&self, wm: &mut M) -> LcwResult<u64> {
        if *wm.shard() != self.shard {
            return Err(LcwError::WrongShard {
                expected: wm.shard().clone(),
                actual: self.shard.clone(),
            });
        }
        let mut rows: Vec<(&String, &WorldEntry)> = self.entries.iter().collect();
        rows.sort_by_key(|(_, e)| e.committed_at);
        let mut written = 0u64;
        for (id, e) in rows {
            wm.put(
                &StateKey(id.clone()),
                LiveValue::new(e.payload.clone()),
                PutCondition::Always,
            )?;
            written += 1;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_is_lowercase_and_deterministic() {
        assert_eq!(hex(&[0x00, 0xab, 0xff]), "00abff");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn empty_world_has_version_zero_and_no_truths() {
        let sh = ShardKey { tenant: "acme".into(), workspace: "research".into() };
        let w = WorldView::empty(sh.clone());
        assert_eq!(w.observed_at(), 0);
        assert!(w.is_empty());
        assert_eq!(w.shard(), &sh);
        assert_eq!(w.world_digest(), WorldView::empty(sh).world_digest());
    }
}

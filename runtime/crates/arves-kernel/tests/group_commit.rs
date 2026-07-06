//! RCR-039 — group-commit / batched-fsync at the Kernel gateway.
//!
//! Proves `RefKernel::commit_group`:
//!   * **fsync count** — a group of N fresh commits issues ONE sync, not N
//!     (asserted via a counting `WalStore` double — a real fsync cannot be
//!     counted from outside `std`);
//!   * **determinism** — grouped truth_hash equals the non-grouped path;
//!   * **durability** — a grouped commit survives drop+recover on the real
//!     fsync-durable `FileKernel` (no acked-but-lost truth);
//!   * **gateway semantics preserved** — ORCH-004 idempotency, RCR-005
//!     content-integrity, IDR-004 cross-shard refusal — identical to
//!     `commit`/`commit_batch`.

use arves_kernel::{
    BatchError, CommitError, ContentHash, FileKernel, Kernel, MemKernel, ProposedWrite, RefKernel,
    ShardKey,
};
use arves_persistence::{
    ContentId, FileWalStore, MemWalStore, Offset, PendingRecord, ReplayCursor,
    ShardKey as PShardKey, SnapshotMeta, Term, Wal, WalError, WalRecord, WalStore,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// A counting WalStore double: `append` models one fsync, `append_group` models
// exactly ONE fsync for the whole group. Lets us assert 1-vs-N at the contract.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CInner {
    records: Vec<WalRecord>,
}

#[derive(Clone)]
struct CountingWal {
    shard: PShardKey,
    log: Arc<Mutex<CInner>>,
    syncs: Arc<AtomicU64>,
}

struct CCursor {
    records: Vec<WalRecord>,
    pos: usize,
    start: Offset,
}
impl ReplayCursor for CCursor {
    fn next(&mut self) -> Result<Option<WalRecord>, WalError> {
        if self.pos < self.records.len() {
            let r = self.records[self.pos].clone();
            self.pos += 1;
            Ok(Some(r))
        } else {
            Ok(None)
        }
    }
    fn position(&self) -> Offset {
        self.start + self.pos as Offset
    }
}

impl Wal for CountingWal {
    type Cursor = CCursor;
    fn shard(&self) -> &PShardKey {
        &self.shard
    }
    fn append(&mut self, record: PendingRecord) -> Result<Offset, WalError> {
        if record.shard != self.shard {
            return Err(WalError::UnknownShard(record.shard));
        }
        let mut log = self.log.lock().unwrap();
        let offset = log.records.len() as Offset;
        log.records.push(WalRecord {
            shard: record.shard,
            offset,
            term: record.term,
            kind: record.kind,
            content: record.content,
            payload: record.payload,
        });
        // Models ONE fsync per record.
        self.syncs.fetch_add(1, Ordering::SeqCst);
        Ok(offset)
    }
    fn append_group(&mut self, records: Vec<PendingRecord>) -> Result<Vec<Offset>, WalError> {
        for r in &records {
            if r.shard != self.shard {
                return Err(WalError::UnknownShard(r.shard.clone()));
            }
        }
        let mut log = self.log.lock().unwrap();
        let mut offs = Vec::with_capacity(records.len());
        for record in records {
            let offset = log.records.len() as Offset;
            log.records.push(WalRecord {
                shard: record.shard,
                offset,
                term: record.term,
                kind: record.kind,
                content: record.content,
                payload: record.payload,
            });
            offs.push(offset);
        }
        // Models exactly ONE fsync for the whole group.
        self.syncs.fetch_add(1, Ordering::SeqCst);
        Ok(offs)
    }
    fn install_snapshot(
        &mut self,
        _u: Offset,
        _t: Term,
        _s: &[u8],
    ) -> Result<SnapshotMeta, WalError> {
        Ok(SnapshotMeta {
            shard: self.shard.clone(),
            up_to_offset: 0,
            term: 0,
            content: ContentId(vec![]),
        })
    }
    fn load_snapshot(&self) -> Result<Option<(SnapshotMeta, Vec<u8>)>, WalError> {
        Ok(None)
    }
    fn compact(&mut self, _u: Offset) -> Result<(), WalError> {
        Ok(())
    }
    fn replay_from(&self, offset: Offset) -> Result<Self::Cursor, WalError> {
        let log = self.log.lock().unwrap();
        let records = log
            .records
            .iter()
            .filter(|r| r.offset >= offset)
            .cloned()
            .collect();
        Ok(CCursor { records, pos: 0, start: offset })
    }
    fn head(&self) -> Offset {
        self.log.lock().unwrap().records.len() as Offset
    }
    fn earliest(&self) -> Offset {
        0
    }
}

#[derive(Clone, Default)]
struct CountingWalStore {
    inner: Arc<Mutex<HashMap<PShardKey, Arc<Mutex<CInner>>>>>,
    syncs: Arc<AtomicU64>,
}
impl CountingWalStore {
    fn new() -> Self {
        Self::default()
    }
    fn syncs(&self) -> u64 {
        self.syncs.load(Ordering::SeqCst)
    }
}
impl WalStore for CountingWalStore {
    type Wal = CountingWal;
    fn open(&self, shard: &PShardKey) -> Result<Self::Wal, WalError> {
        let mut map = self.inner.lock().unwrap();
        let log = map
            .entry(shard.clone())
            .or_insert_with(|| Arc::new(Mutex::new(CInner::default())))
            .clone();
        Ok(CountingWal { shard: shard.clone(), log, syncs: self.syncs.clone() })
    }
    fn shards(&self) -> Vec<PShardKey> {
        self.inner.lock().unwrap().keys().cloned().collect()
    }
}

// ---------------------------------------------------------------------------

fn sk() -> ShardKey {
    ShardKey::new("t1", "w1").unwrap()
}
fn pw(content: &str, payload: &str) -> ProposedWrite {
    ProposedWrite {
        shard: sk(),
        content: ContentHash(content.as_bytes().to_vec()),
        payload: payload.as_bytes().to_vec(),
    }
}

/// The headline: N fresh commits through `commit_group` issue ONE fsync, while N
/// separate `commit` calls issue N. Coalescing is real, durability unchanged.
#[test]
fn group_commit_does_one_fsync_not_n() {
    const N: usize = 16;

    // N separate commits -> N syncs.
    let store_seq = CountingWalStore::new();
    let k_seq: RefKernel<CountingWalStore> = RefKernel::new(store_seq.clone());
    for i in 0..N {
        k_seq.commit(pw(&format!("c{i}"), &format!("p{i}"))).unwrap();
    }
    assert_eq!(store_seq.syncs(), N as u64, "per-commit path fsyncs once per record");

    // One group of N -> ONE sync.
    let store_grp = CountingWalStore::new();
    let k_grp: RefKernel<CountingWalStore> = RefKernel::new(store_grp.clone());
    let group: Vec<ProposedWrite> = (0..N).map(|i| pw(&format!("c{i}"), &format!("p{i}"))).collect();
    let out = k_grp.commit_group(group).unwrap();
    assert_eq!(store_grp.syncs(), 1, "group-commit coalesces N records into ONE fsync");
    assert_eq!(out.len(), N);
    assert!(out.iter().all(|o| o.fresh), "all fresh");
    assert_eq!(k_grp.committed_count(), N);
}

/// Grouped truth is byte-identical to the non-grouped path: same committed
/// content, same order, same `truth_hash`, same count (determinism, HARD RULE 4).
#[test]
fn group_commit_truth_hash_equals_sequential() {
    const N: usize = 32;
    let props: Vec<ProposedWrite> = (0..N).map(|i| pw(&format!("c{i}"), &format!("p{i}"))).collect();

    let k_seq = MemKernel::new(MemWalStore::new());
    for p in &props {
        k_seq.commit(p.clone()).unwrap();
    }

    let k_grp = MemKernel::new(MemWalStore::new());
    let out = k_grp.commit_group(props.clone()).unwrap();

    assert_eq!(out.len(), N);
    assert_eq!(k_grp.committed_count(), k_seq.committed_count());
    assert_eq!(
        k_grp.truth_hash(),
        k_seq.truth_hash(),
        "grouped truth_hash MUST equal the non-grouped path"
    );
    // Offsets/indexes match position (dense, in order).
    for (i, o) in out.iter().enumerate() {
        assert_eq!(o.truth.index.0, i as u64);
    }
}

/// A grouped commit is durable on the REAL fsync-durable Kernel: drop the process
/// and recover from disk alone — every acked record is present with the identical
/// truth_hash. No acked-but-lost truth.
#[test]
fn group_commit_is_durable_on_filekernel() {
    let mut dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    dir.push("rcr039-file-durable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let committed_hash;
    {
        let store = FileWalStore::open_root(&dir).unwrap();
        let k = FileKernel::new(store);
        let group: Vec<ProposedWrite> =
            (0..20).map(|i| pw(&format!("c{i}"), &format!("p{i}"))).collect();
        let out = k.commit_group(group).unwrap();
        assert_eq!(out.len(), 20);
        committed_hash = k.truth_hash();
    } // drop == process exit; only the durable WAL remains

    let store2 = FileWalStore::open_root(&dir).unwrap();
    let recovered = FileKernel::recover(store2);
    assert_eq!(recovered.committed_count(), 20, "every grouped record recovered");
    assert_eq!(
        recovered.truth_hash(),
        committed_hash,
        "recovered truth_hash == committed (grouped) truth_hash"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// ORCH-004 inside a group: an identical duplicate collapses to ONE append; the
/// first occurrence is fresh, the rest resolve to it. And a group whose members
/// are already committed adds NO new fsync-worthy work beyond its fresh members.
#[test]
fn group_commit_idempotent_within_group_and_against_truth() {
    let store = CountingWalStore::new();
    let k: RefKernel<CountingWalStore> = RefKernel::new(store.clone());

    // Pre-commit A (one sync).
    k.commit(pw("A", "pa")).unwrap();
    assert_eq!(store.syncs(), 1);

    // Group [A (already), B, C, B(dup)] -> fresh = {B, C}, one group sync.
    let out = k
        .commit_group(vec![pw("A", "pa"), pw("B", "pb"), pw("C", "pc"), pw("B", "pb")])
        .unwrap();
    assert_eq!(store.syncs(), 2, "one pre-commit sync + one group sync");
    assert_eq!(out.len(), 4);
    assert!(!out[0].fresh, "A already committed -> resolve");
    assert!(out[1].fresh, "B fresh");
    assert!(out[2].fresh, "C fresh");
    assert!(!out[3].fresh, "B duplicate within group -> resolve");
    // A, B, C = 3 distinct truths.
    assert_eq!(k.committed_count(), 3);
    // The duplicate B entries resolve to the SAME truth.
    assert_eq!(out[1].truth, out[3].truth);
}

/// RCR-005 content-integrity: a group forking committed truth (same address,
/// different payload) is refused WHOLE — nothing appended (no sync, no truth).
#[test]
fn group_commit_content_integrity_fork_refuses_whole_group() {
    let store = CountingWalStore::new();
    let k: RefKernel<CountingWalStore> = RefKernel::new(store.clone());
    k.commit(pw("A", "original")).unwrap();
    assert_eq!(store.syncs(), 1);

    let err = k
        .commit_group(vec![pw("B", "pb"), pw("A", "TAMPERED")])
        .unwrap_err();
    match err {
        BatchError::Refused { index, cause: CommitError::ContentIntegrity { .. } } => {
            assert_eq!(index, 1)
        }
        other => panic!("expected Refused/ContentIntegrity, got {other:?}"),
    }
    assert_eq!(store.syncs(), 1, "refused group performed NO append/fsync");
    assert_eq!(k.committed_count(), 1, "only the original truth exists");
}

/// An intra-group fork (same address, two payloads INSIDE the group) is refused
/// whole, nothing applied.
#[test]
fn group_commit_intra_group_fork_refused() {
    let k = MemKernel::new(MemWalStore::new());
    let err = k
        .commit_group(vec![pw("A", "p1"), pw("A", "p2")])
        .unwrap_err();
    assert!(matches!(
        err,
        BatchError::Refused { cause: CommitError::ContentIntegrity { .. }, .. }
    ));
    assert_eq!(k.committed_count(), 0, "nothing applied");
}

/// IDR-004: no cross-shard atomic commit — a mixed-shard group is refused up front.
#[test]
fn group_commit_cross_shard_refused() {
    let k = MemKernel::new(MemWalStore::new());
    let other = ProposedWrite {
        shard: ShardKey::new("t1", "w2").unwrap(),
        content: ContentHash(b"x".to_vec()),
        payload: b"x".to_vec(),
    };
    let err = k.commit_group(vec![pw("A", "pa"), other]).unwrap_err();
    assert!(matches!(err, BatchError::CrossShard { index: 1, .. }));
    assert_eq!(k.committed_count(), 0);
}

/// An empty group is a no-op.
#[test]
fn group_commit_empty_is_noop() {
    let k = MemKernel::new(MemWalStore::new());
    assert!(k.commit_group(vec![]).unwrap().is_empty());
    assert_eq!(k.committed_count(), 0);
}

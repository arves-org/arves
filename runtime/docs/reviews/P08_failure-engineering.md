# P08 — Failure Engineering Review (Independent Chief-Architect Lens)

**Reviewer role:** Independent Failure/Resilience Architect
**Objective:** Try to destroy the ARVES reference runtime; enumerate a structured
catalogue of catastrophic failure classes; redesign recovery/guards for the worst
classes, extending the I1.7 "lossless-or-loud" discipline.
**Scope reviewed (code):** `arves-persistence/src/lib.rs` (segmented durable WAL +
checkpoint/compaction/recovery), `arves-kernel/src/lib.rs` (sole commit gateway +
`try_recover`/`try_replay`), `arves-consensus/src/lib.rs` (contracts only),
`arves-runtime/src/main.rs`, plus the recovery tests and the I1.5/I1.6/I1.7 and
I2.0 design docs.
**Governing corpus (read-only):** IDR-001..005 (Kernel Distribution), Amendments
CCP Batch 1 (`SHARD-001`), Vol 9 Cognitive Control Plane v2 (`ORCH-001..004`),
Invariant Registry, ED-001..003 doctrine, RT-001.

> **Hard rule honoured throughout:** no finding proposes modifying the frozen
> specification. Every proposal is an **IDR**, **CCP Amendment**, **Runtime**,
> **Verification**, **Certification**, **Ecosystem**, or **Product** instrument.

---

## Executive summary

The runtime's storage core is unusually disciplined for its maturity: single
`write_all` framing with a trailing CRC, fsync-before-`Ok`, atomic-rename
checkpoints, torn-tail truncation on open, a `replay_from` contiguity check, and
the I1.7 "lossless or loud" `RecoveryError` surface. The adversarial hunt that
produced I1.7 was real work and closed the *silent* single-node data-loss holes it
audited.

But "lossless or loud" today is a **single-node, single-shard, cooperative-crash**
guarantee. The runtime has never faced the failure classes that define whether a
cognitive-infrastructure standard is *internationally trustworthy*: it has no
distribution yet (so split-brain, partition, byzantine follower, duplicate replay
are entirely unmodelled), it **trusts the storage medium and the OS** (fsync lies,
`fsync` EIO / "fsyncgate", latent sector errors, misdirected/torn writes below the
frame granularity, directory-entry durability), it has **exactly one local copy**
(no redundancy → any single-block corruption of a sole snapshot is unrecoverable by
construction, only *detected*), and its "loud" behaviour is **`panic!`** with no
quarantine, no operator contract, and no fault-domain isolation.

The single most consequential structural gap: **`head` on open is derived only from
the last segment** (`arves-persistence/src/lib.rs:1234-1257`) and **only the last
segment is CRC-validated on open**. Interior-segment loss/corruption is caught only
opportunistically at `replay_from`, and a whole *missing interior segment* combined
with the absence of a durable manifest means the runtime cannot always tell "log
legitimately ends here" from "a segment silently vanished." A standard that will
run 10,000-node tenants for 20 years must treat the storage substrate as an
**adversary**, not a partner. This review supplies (§2) a ~110-class failure
taxonomy and (§3) ranked, spec-preserving redesigns; the highest-leverage ones are:
a **durable append-only segment manifest / log-truth root** (F-01), **medium
distrust via per-block checksums + double-write of critical metadata** (F-02), a
**quarantine-not-panic operator contract** (F-04), a **fault-injection filesystem +
crash-consistency model checker in the conformance suite** (F-03), and an
**anti-entropy / repair-from-peer protocol designed now** so the I2 replication work
inherits the failure contract rather than retrofitting it (F-05).

If ARVES were handed to ISO/IEEE tomorrow, the failure-engineering chapter would be
**empty**: there is no normative fault model, no durability-assumption register, no
required fault-injection conformance, and no defined recovery/quarantine state
machine. That absence — not any single code bug — is the finding that most
threatens 20-year adoptability.

---

## Severity-ranked findings table

| # | Severity | Title | Instrument | Impl. complexity |
|---|----------|-------|-----------|-----------|
| F-01 | Critical | No durable segment manifest / log-truth root → interior segment loss is undetectable, `head` over-reports | IDR + Runtime | High |
| F-02 | Critical | Storage medium is trusted (fsync lies, latent/misdirected writes, no per-block ECC, sole copy) — "loud" ≠ "recoverable" | IDR + Runtime | High |
| F-03 | Critical | No fault-injection filesystem or crash-consistency model checking in verification; correctness rests on hand-picked bit-flips | Verification + Certification | High |
| F-04 | High | "Loud" = `panic!`; no quarantine state, no operator recovery contract, no fault-domain isolation (one shard bricks the process) | IDR + Runtime | Medium |
| F-05 | High | Distributed failure model (split-brain, partition, byzantine follower, duplicate/partial replication, leader flap) is entirely undesigned; I2.0 explicitly defers it | IDR + Runtime | Very-high |
| F-06 | High | No normative Fault Model / Durability-Assumption Register in the corpus → nothing to certify against; "lossless or loud" is a code comment, not a standard | CCP-Amendment + Certification | Medium |
| F-07 | High | Checkpoint/compaction durability barrier lacks directory-fsync and manifest fencing; crash windows can resurrect deleted segments or orphan the log head | IDR + Runtime | Medium |
| F-08 | Medium | Content-addressed idempotency trusts the hash with no payload verification and an unspecified/possibly-weak digest → collision or corrupted-payload aliasing forks or poisons truth | IDR + Runtime | Medium |
| F-09 | Medium | Clock/term/monotonicity: `term` hardcoded to 0, wall-clock never used but never fenced; no epoch/generation guard against stale-writer / dual-mount | IDR + Runtime | Medium |
| F-10 | Medium | Disk-full / partial-write / ENOSPC mid-append and mid-checkpoint are not deterministically handled; `Durability(String)` is an untyped, non-recoverable dead end | IDR + Runtime | Medium |
| F-11 | Medium | Poisoned-mutex `.expect(...)` on every state/log lock turns any one panic into permanent whole-runtime unavailability | Runtime | Low |
| F-12 | Low | Multi-shard recovery is all-or-nothing and serial; one bad shard blocks recovery of all healthy shards (availability blast radius) | Runtime | Low |

---

## Section 2 — Failure taxonomy (target breadth ~110 classes, grouped)

Legend for current runtime posture:
**H** = handled/guarded today · **D** = *detected* but not recoverable single-node ·
**U** = unhandled / unmodelled · **N/A(dist)** = requires distribution (not yet built).

### G1. Process / power / crash-timing (single node)
1. Clean process exit mid-idle — **H**.
2. Crash after `write_all` before `sync_all` returns — torn tail; truncated on open — **H**.
3. Crash *during* `sync_all` (frame partially on platter) — CRC catches partial frame; truncated — **H** (assuming no fsync lie; see G6).
4. Crash between snapshot fsync and `rename` — orphan `.snap.tmp`; swept on open — **H**.
5. Crash between `rename` and directory durability (no dir fsync) — checkpoint may be lost though caller believed it durable — **U** (F-07).
6. Crash after `install_snapshot` durable, before `compact` — snapshot + full log both present; recoverable — **H** (design-verified).
7. Crash *mid-`compact`* after some `remove_file`s — ascending unlink + snapshot-first ordering keeps it safe — **H** (hunt REFUTED the hole).
8. Crash mid-`compact` after deleting a covered segment but *before* dir fsync — deleted segment may reappear (dirent not durable) — **U** (F-07).
9. Crash mid-`rotate` (new empty segment created, dirent not durable) — new segment may vanish; next append re-creates it — mostly benign but undocumented — **U**.
10. Repeated crash-loop during recovery (crash inside `try_replay`) — recovery is pure/idempotent, so re-entrant — **H**.
11. Power loss with volatile disk write cache enabled and no barrier — fsync insufficient; committed data lost — **U** (F-02/F-06: durability assumption unstated).
12. Kill -9 vs graceful shutdown — no shutdown hook; both identical path — **H** by design (crash-only).
13. OOM-killer terminates process mid-append — same as (2) — **H**.
14. Crash while `open`-time torn-tail truncation itself is mid-`set_len` — truncation not atomic; could leave a shorter-than-good-prefix file — **U** (idempotent re-truncate on next open mitigates, undocumented).

### G2. Storage-medium physical faults
15. Latent sector error / unreadable block in a *sealed interior* segment — `replay_from` reads fail → `Durability` → loud, but unrecoverable single-node — **D**.
16. Bit-rot flipping a byte inside a committed frame body — CRC32 detects at replay; loud — **D**.
17. Bit-rot in the CRC field itself — frame rejected as corrupt; loud — **D**.
18. Bit-rot inside a snapshot blob — snapshot CRC detects; falls back to older/none — **H** if an older snapshot or full log exists; **D** if sole copy (F-02).
19. Misdirected write (disk writes correct data to wrong LBA) — frame-level CRC does **not** detect if the *displaced* frame is itself CRC-valid but at the wrong offset — offset field check helps, but a whole-block move can pass — **U** (F-02).
20. Torn write *below* frame granularity (e.g. 4 KiB atomic sector, frame spans it) — partial old/new mix — CRC likely catches; not guaranteed for aligned same-length rewrite — **U**.
21. Silent data corruption reported as success by the drive firmware — no end-to-end verification beyond CRC-on-read — **D** at best.
22. Whole segment file truncated by filesystem (e.g. after fsck) — interior truncation → gap; caught at `replay_from` only — **D** (F-01: head over-reports until then).
23. Whole segment file *deleted* out from under the runtime — `list_segments` just won't see it; interior gap detected at replay, but a *tail* segment deletion silently lowers head — **U** (F-01).
24. Snapshot file deleted, log prefix already compacted — `CompactedPrefixWithoutSnapshot` loud — **D**.
25. Disk write amplification / SSD wear causing correlated multi-block failure — sole-copy = total loss — **D** (F-02).
26. Read returns stale data (drive cache incoherence) — undetectable single-node — **U**.
27. CRC32 aliasing: 1-in-4-billion undetected corruption per frame; at trillions of frames over 20 years, expected undetected corruptions > 0 — **U** (F-02: CRC32 too weak for a durability root).
28. Firmware bug returns zeros for a valid block — decodes as corruption (version byte 0 is valid `Outcome` kind but version!=1 → rejected) → loud — **D**.

### G3. Filesystem / OS semantics
29. `fsync` returns success but data not durable ("fsyncgate") — **U** (F-02/F-06).
30. `fsync` returns EIO and the kernel *clears* the dirty page (Linux pre-4.13 / cross-fs) — data silently lost, next fsync returns success — **U** (F-02) — catastrophic and famous.
31. Directory entry not durable after file create/rename without dir fsync — **U** (F-07); dir fsync is only best-effort and only after snapshot rename, never after rotate/compact.
32. Rename not atomic on the target filesystem (some network FS) — checkpoint half-visible — **U** (F-06: FS assumptions unstated).
33. `set_len` (truncate) not durable without fsync — done with fsync for truncation — **H**; but the *file-length* metadata durability depends on dir/inode fsync semantics — partial.
34. Filesystem reorders metadata vs data on crash (non-ordered mode) — append could be visible with stale length or vice-versa — **U** (F-02).
35. Case-insensitive / Unicode-normalizing FS collapses two distinct shard dir names (hex-encoded, so safe) — **H** (hex encoding avoids it).
36. Path length / component limit exceeded by long tenant/workspace after hex-doubling — `Durability` open error, loud but opaque — **U** (F-10).
37. Filesystem remounted read-only mid-operation (medium error) — append fails loudly; no quarantine, panics — **U** (F-04/F-10).
38. Two processes open the same store root (double-mount / stale container) — **no lock file**; both believe they lead; interleaved appends corrupt offsets — **U** (F-09: no single-writer fencing).
39. `O_APPEND` atomicity assumption across NFS — append offset races — **U** (F-06).
40. Clock skew affecting file mtime-based logic — none used; `list_segments` sorts by parsed offset, not mtime — **H**.

### G4. Logical / format / decoder faults
41. Frame `body_len` field corrupted to a huge value — bounds-checked; treated as torn → truncate — **H**.
42. Frame `body_len` corrupted to point past EOF but CRC region readable — `body_end+4 > len` → torn — **H**.
43. Offset field inside frame ≠ expected position — `decode_body` returns None → corruption — **H**.
44. Shard tenant/workspace inside frame ≠ directory shard — rejected → corruption — **H**.
45. Unknown `RecordKind` byte — rejected → corruption — **H**.
46. Frame version != 1 — rejected → corruption — **H** (good forward-compat fence).
47. Trailing garbage inside a framed body — `!r.done()` → None → corruption — **H**.
48. Snapshot blob decodes but `decode_shard_blob` finds trailing garbage — `unwrap_or_default()` → **silently installs EMPTY state** — **U** (see F-08 note: `install_state` swallows decode failure).
49. Snapshot file `up_to` in filename ≠ decoded `up_to` — mismatch skipped — **H**.
50. Two snapshots with same `up_to` but different content (crash-rewrite) — deterministic name means the second `rename` overwrites; no versioning/generation — **U** (F-07).
51. Content hash collision (two distinct payloads, same `ContentHash`) — second commit aliased as `AlreadyCommitted`; truth silently wrong — **U** (F-08).
52. Empty payload / empty content id — accepted; length-prefixed encoding handles it — **H**.
53. Extremely large payload (u32 length overflow) — `b.len() as u32` truncates silently for >4 GiB — **U** (F-10).
54. Non-UTF8 tenant/workspace bytes on disk (from a future encoding) — `from_utf8` fails → shard skipped by `shards()` → **silent shard disappearance** — **U** (F-01/F-12).

### G5. Compaction / snapshot lifecycle
55. Compaction requested at offset beyond head — `OffsetOutOfRange` — **H**.
56. Compaction below already-compacted base — no-op — **H**.
57. Snapshot at `head-1` with term hardcoded 0 — loses real term; harmless single-node but breaks future Raft snapshot-transfer (needs true `(term, index)`) — **U** (F-09).
58. Checkpoint of a shard that fails midway (multi-shard `checkpoint()` loop) — leaves some shards checkpointed, some not; returns `Err(String)` and aborts — **U** (partial checkpoint, untyped error) (F-10).
59. Compaction deletes a superseded snapshot `u < up_to_offset` with best-effort `let _ =` — a failed delete leaks but is harmless — **H**.
60. Snapshot install succeeds, compaction deletes covered segment, then snapshot found corrupt on next open — prefix unrecoverable — **D** (F-02: sole copy).
61. Interleave: snapshot at N, then more appends N+1..M, crash before next snapshot — tail replay from N+1 — **H**.
62. `load_snapshot` picks highest valid `up_to`, but a *lower* snapshot covers a range whose segments were already compacted by the higher one — if highest is corrupt, fallback lower snapshot may reference already-deleted segments → gap — **U** (F-07: no coupling of snapshot validity to retained-segment set).
63. Snapshot covers `up_to` but the corresponding segment was never actually compacted (compaction crash) — double coverage, replay idempotent — **H**.

### G6. Concurrency / re-entrancy (in-process)
64. Two threads commit same content concurrently — mutex serializes; second sees `AlreadyCommitted` — **H**.
65. Commit while checkpoint runs — both take the same `state`/`inner` mutex; serialized — **H** but coarse (F-11 scalability note).
66. Panic while holding `state` mutex → poisoned → every subsequent `.expect("state poisoned")` panics → whole process dead until restart — **U** (F-11).
67. Panic in one shard's WAL mutex poisons only that shard's inner lock — but the shared `open_wals`/`state` are process-global — blast radius = whole node — **U** (F-04/F-11).
68. Reentrant recovery calling `open` which mutates `open_wals` cache while iterating shards — no aliasing bug found (cache keyed per shard) — **H**.
69. Cursor holds a materialized `Vec` snapshot; concurrent append after cursor creation not seen — acceptable for replay (bounded to head at creation) — **H**.

### G7. Distributed — partition / consensus (NOT YET BUILT; all N/A(dist)/U)
70. Full network partition splitting a shard's Raft group — **U** (I2 deferred).
71. Split-brain: two leaders in same term after asymmetric partition — **U**.
72. Split-brain across a partition healing with divergent committed suffixes — **U**.
73. Leader flap / rapid re-election (herd) — **U**.
74. Lost-leader with in-flight uncommitted entries (must be discarded per IDR-004) — undesigned in code — **U**.
75. Follower far behind leader after leader compacted the log — needs snapshot transfer; I2.0 lists it as a *future* hunt lens, not designed — **U**.
76. Duplicate replay: same committed outcome delivered twice — content-idempotency helps, but *offset*-idempotency on follower is only sketched (I2.0 §9) — **U**.
77. Out-of-order / gapped replication stream — I2.0 sketches "loud gap," unbuilt — **U**.
78. Partial replication: quorum ack lost after local durable append (leader thinks not-committed, follower has it) — **U**.
79. Byzantine follower returns fabricated ack / wrong content — no signing/attestation planned — **U**.
80. Byzantine leader replicates inconsistent outcomes to different followers — no cross-follower verification — **U**.
81. Membership change (joint consensus) crash mid-transition — contract exists, no impl — **U**.
82. Cross-shard saga partial failure / orphaned compensation — sagas named in IDR, no failure model — **U**.
83. Read-index staleness violation under partition (linearizable read served by deposed leader) — **U**.
84. Quorum loss (minority survives) — availability vs safety choice undocumented for operators — **U** (F-06).
85. Node clock jump causing premature election timeout storm — **U**.
86. Replay divergence: follower `truth_hash != leader` — I2 proof target, no adversarial coverage yet — **U**.
87. Zombie leader resumes after long GC pause / VM freeze (>lease) and commits — **U** (F-09: no fencing token).
88. Log divergence at same `(term,index)` with different content — **U**.

### G8. Clock / time / ordering
89. Wall-clock never consulted for truth (good: deterministic) — **H**.
90. But `term=0` everywhere → no epoch to fence a stale writer / old process re-attaching to the same dir — **U** (F-09).
91. Monotonic offset relied on file scan; a resurrected deleted tail segment could reuse offsets — **U** (F-01/F-07).
92. NTP step backward affecting future lease/heartbeat math (I2) — **U**.
93. Leap second / TZ — no calendar time in truth path — **H**.

### G9. Resource exhaustion
94. Disk full mid-append: `write_all`/`sync_all` error → `Durability` → commit `Rejected` → caller sees error, no partial frame committed (torn tail truncated next open) — **H** for safety, **U** for graceful behaviour (F-10).
95. Disk full mid-checkpoint tmp write — errors before rename; no corruption; but shard left uncompacted, log grows — **H**-ish (F-10 for the growth/backpressure).
96. Inode exhaustion (many shards × many segments) — `create_dir_all`/open fail loudly; no bound on segment count — **U** (F-10).
97. FD exhaustion (one open append FD per shard cached forever, never closed) — leak across many shards — **U** (F-11/F-12).
98. Memory exhaustion: `replay_from` and `load_snapshot` read *entire* segments/snapshot into `Vec` in RAM; a huge shard OOMs recovery → crash-loop — **U** (F-10: recovery not streaming).
99. Unbounded WAL growth if checkpoint never called (checkpoint is caller-driven, no policy) — **U** (F-10).
100. `committed` grows unboundedly in memory (whole truth set resident) — inherent to reference design; a scale ceiling — **U** (scalability, out of failure scope but noted).

### G10. Operational / lifecycle / human
101. Operator restores a stale backup of one shard dir (offset regression) — no manifest/epoch to detect the rewind — **U** (F-01/F-09).
102. Operator copies a shard dir to a new node without changing shard key — two nodes, same identity, no fencing — **U** (F-09).
103. Partial backup (segments copied without the snapshot, or vice-versa) — may present a compacted prefix without snapshot → loud — **D**.
104. Upgrade to a new frame version reading old files — version fence rejects → loud; but no migration path defined — **U** (F-06).
105. Downgrade after writing v2 frames — old binary rejects → loud — **H** (fails safe).
106. Config change of `rotate_every` between runs — segments of mixed sizes; `list_segments` + offset math still correct — **H**.
107. Two different `rotate_every` values racing (shouldn't happen) — offsets still authoritative — **H**.
108. Silent shard disappearance if its directory name fails to parse (non-hex, bad utf8) — `shards()` skips it with no warning → recovery *succeeds* while omitting a whole tenant — **U** (F-01/F-12) — a silent-loss class the I1.7 discipline missed because it operates per-shard *after* enumeration.
109. Observability gap: no metric/event emitted on torn-tail truncation, corruption detection, or orphan sweep — silent self-healing hides real medium degradation from operators — **U** (F-04/F-06).
110. No fault-domain isolation: persistence, kernel, and (future) control plane share one process/address space — one segfault in any future engine linkage takes truth down — **U** (F-04).

**Coverage summary:** of 110 classes, roughly 35 are genuinely **H**, ~12 are
**D** (detected-but-single-node-unrecoverable, the honest I1.7 residual), and the
majority are **U** — dominated by (a) trusting the medium/OS, (b) the absence of
any second copy, (c) the entire distributed dimension, and (d) the lack of a
normative fault model to certify against.

---

## Section 3 — Findings & redesign proposals

Each finding: **Why it matters · Risks · Long-term consequences · Alternatives ·
Recommendation · Implementation complexity · Scientific impact · Ecosystem impact.**

---

### F-01 — No durable segment manifest / log-truth root: interior/tail segment loss is undetectable and `head` over-reports (Critical · IDR + Runtime)

**Evidence.** `FileWalStore::open` (`arves-persistence/src/lib.rs:1234-1257`) derives
`head` solely from `last.start + count` of the **last** segment and CRC-scans **only
that last segment**. Interior segments are read (and thus validated) only lazily in
`replay_from`. `shards()` enumerates by directory listing (`:1282-1313`) with no
authoritative record of *which* shards or *how many* segments *should* exist. There
is no on-disk statement of "this log legitimately ends at offset N" independent of
the segment files themselves.

**Why it matters.** The trace *is* the truth (IDR-005). If a tail segment is deleted
(G7-23) or an interior segment vanishes (G4-54, filesystem/fsck/operator), the
runtime has **no anchor** to distinguish "the log correctly ends here" from "the log
was silently amputated." The current contiguity check in `replay_from` catches
*interior* gaps only when a later record survives; it cannot catch a **truncated
tail** (there is nothing after the gap to be non-contiguous with), and `head` will
simply be lower than reality — silent loss that survives even the I1.7 discipline.
`shards()` silently dropping an unparseable shard directory (G10-108) is the same
class one level up.

**Risks.** False "clean recovery" after real data loss — the exact failure I1.7 was
created to prevent, reintroduced at segment/shard granularity. In a certification
setting this is a trust-collapse event.

**Long-term consequences.** Without a manifest, replication (I2) and backup/restore
inherit an unverifiable substrate; every higher layer's correctness is conditional
on an unstated "no segment silently disappeared" assumption.

**Alternative designs.** (a) **Durable append-only MANIFEST** per shard: a
CRC/hash-chained record of `(segment_start, sealed, record_count, high_offset,
term)` updated (fsync + dir fsync) on every seal/rotate/compact; recovery reconciles
segments against the manifest and fails loud on any discrepancy. (b) **Chained
segment headers**: each new segment's header stores the previous segment's tail hash
(a hash chain / Merkle spine), so a missing interior segment breaks the chain
detectably. (c) **`head` persisted explicitly** in a fenced superblock. (d) Rely on
replication to repair (insufficient — you still cannot *detect* the loss locally to
trigger repair).

**Recommendation.** Adopt **(a) manifest + (b) hash-chained segment headers**
together: the manifest gives an authoritative segment set and high-offset; the hash
chain makes interior/tail loss and reordering cryptographically detectable, and
becomes the **log-truth root** that a peer/backup can compare in O(1). Extend
`shards()` to fail loud (not skip) on an unparseable/foreign directory under the
store root. Record as a new **IDR** ("Log integrity via manifest + hash-chained
segments") since it is a reference-implementation durability decision under RT-001,
plus the Runtime work. This is the natural, spec-preserving extension of I1.7:
*losslessly detect* every amputation, then be loud.

**Implementation complexity.** High (new durable structure, migration, recovery
reconciliation, tests).

**Scientific impact.** Turns "append-only log" from a filesystem convention into a
**verifiable, tamper-evident data structure** — a publishable, standardizable
contribution (Merkle-chained WAL as the decision-trace root).

**Ecosystem impact.** The log-truth root becomes the interoperability primitive for
Independent Runtimes A/B (they can prove byte-identical traces by comparing roots)
and for third-party backup/audit tooling.

---

### F-02 — The storage medium and OS are trusted; "loud" ≠ "recoverable" with a sole copy (Critical · IDR + Runtime)

**Evidence.** Durability rests on `File::sync_all` returning `Ok` (`:948`, `:983`),
CRC32 (`:565`) as the only integrity check, and exactly **one** local copy of every
segment and snapshot. There is no per-block checksum independent of the frame, no
double-write of critical metadata, no scrubbing, and no statement of what the
runtime assumes about the medium.

**Why it matters.** Every catastrophic storage class in G2/G3 (fsync EIO page-drop
"fsyncgate" G3-30, fsync lies G3-29, misdirected writes G2-19, sub-frame torn writes
G2-20, CRC32 aliasing at 20-year scale G2-27, sole-snapshot rot G2-18/60) currently
resolves to either *undetected loss* or *detected-but-unrecoverable* loss (the honest
**D** classes). A cognitive-infrastructure standard cannot durably "own truth" (the
Kernel's entire reason to exist, ORCH-001/OWN-001) on a substrate it merely hopes is
honest.

**Risks.** The most dangerous is fsync-EIO-with-page-drop: the runtime believes a
commit is durable, the page is dropped, the next fsync succeeds, and truth is
silently gone with a `TruthRef` already handed to a caller — a correctness violation
of the commit contract itself.

**Long-term consequences.** 20-year, planet-scale operation makes rare events
certain: CRC32's ~2^-32 miss rate over trillions of frames yields expected undetected
corruptions > 0. The standard's credibility depends on making these detectable and,
where a second copy exists, recoverable.

**Alternative designs.** (a) Upgrade the integrity primitive to a **cryptographic
digest (BLAKE3/SHA-256) per frame and per snapshot**, feeding the F-01 hash chain.
(b) **Per-block (e.g. 4 KiB) checksums** to localize and detect misdirected/torn
sub-frame writes. (c) **Double-write critical metadata** (manifest/superblock A/B
with generation numbers) so a torn metadata write is always recoverable. (d)
**`fsync` error handling that treats EIO as fatal and refuses further commits** until
operator-verified (mitigates the page-drop class). (e) **Background scrubber** that
re-reads and re-verifies sealed segments/snapshots on a schedule to surface latent
sector errors *before* they meet a real read. (f) **Redundancy via replication
(I2)** as the recovery path — but detection (a–e) must exist first.

**Recommendation.** Publish a **Durability-Assumption Register** (see F-06) stating
exactly what is assumed of the medium/OS/FS, then close the gap between assumption
and reality with (a)+(c)+(d)+(e). Use a cryptographic hash as the integrity root
(dovetails with F-01). Record the medium-distrust posture as an **IDR**
("Adversarial storage medium: detect, then repair").

**Implementation complexity.** High.

**Scientific impact.** Aligns ARVES with the state of the art in trustworthy storage
(end-to-end integrity, checksummed logs) and makes its durability claims *provable*
rather than *assumed*.

**Ecosystem impact.** Enables certified deployment on commodity/cloud storage of
unknown honesty — a precondition for broad adoption.

---

### F-03 — No fault-injection filesystem or crash-consistency model checking in verification (Critical · Verification + Certification)

**Evidence.** The recovery tests (`arves-kernel/tests/recovery.rs`,
`arves-persistence/tests/recovery.rs`) inject faults by *hand-picked byte flips at
known offsets* (`flip_byte(path, 8)`) and *one* orphan-file scenario. There is no
systematic exploration of crash points, no simulated power loss with reordered
writes, no fsync-failure injection, and no model of adversarial filesystem behaviour.
The I1.7 hunt was human-agent-driven, not tooled.

**Why it matters.** The entire "lossless or loud" claim is only as strong as the
faults exercised. The taxonomy in §2 shows dozens of crash *windows* (G1) and FS
behaviours (G3) that a fixed bit-flip cannot reach. Correctness of crash-consistency
is exactly the domain where hand-written tests are known to miss the case that
matters (the reason tools like CrashMonkey, ALICE, Hypothesis/proptest, and TLA+/FDB
deterministic simulation exist).

**Risks.** Certifying "lossless or loud" on the strength of a dozen bespoke tests
invites a field failure that a systematic harness would have caught — the worst
outcome for a standard's reputation.

**Long-term consequences.** Independent Runtimes A/B (a stated LONG-TERM OBJECTIVE)
cannot be *comparably* certified without a shared, mechanized fault-injection
conformance suite; otherwise "passes conformance" means different things per vendor.

**Alternative designs.** (a) A **fault-injection `WalStore` / filesystem shim**
(behind the existing `Wal`/`WalStore` traits) that can: fail/partial-write at chosen
byte counts, reorder+drop writes on simulated crash, lie about fsync, inject EIO,
and enumerate every crash point in an operation. (b) **Property-based testing**
(proptest) generating random commit/checkpoint/compact/crash schedules and asserting
the recovery invariant (recovered `truth_hash` ∈ {prefix hashes} OR loud error) —
never silent wrong truth. (c) **Deterministic simulation** à la FoundationDB: a
single-threaded, seeded scheduler driving the runtime through millions of
fault-permuted histories. (d) **A TLA+/Alloy model** of the WAL+snapshot+compaction
state machine, model-checked for the "no silent loss" property, kept in the repo as
executable specification-of-behaviour (not a change to the frozen spec).

**Recommendation.** Build (a)+(b) now as part of the **conformance suite** (make it
a **Certification** gate: no milestone passes without the fault harness green), and
add (d) as a **Verification** artifact for the storage core. This operationalizes
ED-003 ("attack, don't just test") with tooling instead of relying on repeated
human hunts.

**Implementation complexity.** High (but the trait boundaries already exist, which
makes the shim tractable — a strong point of the current design).

**Scientific impact.** A mechanized crash-consistency conformance kit for a cognitive
standard is itself a research/standardization contribution.

**Ecosystem impact.** Becomes *the* certification instrument that makes "Independent
Runtime A/B pass certification" meaningful and vendor-neutral.

---

### F-04 — "Loud" = `panic!`; no quarantine state, operator contract, or fault-domain isolation (High · IDR + Runtime)

**Evidence.** `RefKernel::recover` panics on any `RecoveryError`
(`arves-kernel/src/lib.rs:473-476`); every lock uses `.expect("... poisoned")`
(e.g. `:388`, `:654`); `arves-runtime/src/main.rs` maps recovery failure to
`process::exit(3)`. There is one process, one address space, all shards, and truth.

**Why it matters.** "Loud" is the right *safety* choice, but a panic is a *blunt*
one: it gives operators no structured state ("shard X is quarantined pending repair,
shards Y/Z are healthy and serving"), no machine-readable event, and no isolation —
one corrupt shard bricks the entire node, and one poisoned mutex (G6-66/67) makes the
whole runtime permanently unavailable until restart, which will just re-hit the same
corruption and crash-loop.

**Risks.** Availability blast radius = 100% of a node for a fault confined to 1
shard; crash-loop instead of controlled degradation; no operator affordance to
export/repair a single quarantined shard.

**Long-term consequences.** Enterprise/cloud deployment (LONG-TERM OBJECTIVES 6–9)
requires graceful degradation and a defined operational contract, not `panic!`.

**Alternative designs.** (a) A **recovery state machine** with explicit terminal
states per shard: `Healthy`, `Quarantined{reason}`, `RepairPending`, and a runtime
that serves healthy shards while quarantining bad ones (ties to F-12). (b) Replace
poison-`expect` with a **poison-recovery / re-init policy** or move to lock types
that don't poison, or a supervisor that reconstructs the guarded state. (c)
**Fault-domain isolation**: process-per-tenant or per-shard-group so one crash is
contained (dovetails with SHARD-001's per-tenant Raft groups). (d) Structured
**operator events** (F-06) for every quarantine.

**Recommendation.** Define a **quarantine-not-panic operator contract** as an
**IDR** and implement (a)+(d) now; schedule (c) with I2. Keep `panic!` only as the
last-resort "cannot even reach a safe quarantine" backstop. This *extends* "lossless
or loud" to "lossless or **loud and contained**."

**Implementation complexity.** Medium.

**Scientific impact.** Low-moderate (systems engineering rigor).

**Ecosystem impact.** High — turns a research prototype into something an SRE can
operate; a precondition for the enterprise/cloud objectives.

---

### F-05 — The distributed failure model is entirely undesigned; I2.0 defers split-brain, partition, byzantine, and partial replication (High · IDR + Runtime)

**Evidence.** I2.0 (`docs/I2.0_Engineering_Design.md` §0, §11) explicitly lists
"leader election, quorum, membership, failover, split-brain, and networking" as
**non-goals** for the first replication deliverable; `arves-consensus` is contracts
only. Taxonomy classes G7-70..88 are all **U**.

**Why it matters.** IDR-001..005 stake ARVES's correctness on *per-shard Raft as a CP
system*. Every hard part of CP is a *failure* behaviour: what happens on partition,
on a deposed leader with in-flight entries (IDR-004 says "discarded" — unproven), on
a follower that fell behind a compacted log (I2.0 names it a *future* lens), on
duplicate/reordered/partial replication, and on byzantine peers. Proving replication
on the happy path (I2.1) without a first-class failure model risks baking in a design
that cannot later absorb these classes without rework.

**Risks.** Retrofitting fencing tokens, snapshot-transfer, and split-brain guards
after the replication data path is set is exactly how distributed systems acquire
latent safety bugs.

**Long-term consequences.** A CP standard whose partition behaviour is unspecified is
not standardizable; ISO/IEEE reviewers will ask "what does a client observe during a
partition?" and today there is no answer.

**Alternative designs.** (a) Write the **distributed fault model FIRST** (an IDR):
enumerate partition/split-brain/byzantine/duplicate/partial-replication behaviours
and the required client-observable semantics (linearizable writes, read-tier
guarantees under partition per the IDR-001 tiers). (b) Adopt **fencing tokens /
epochs** (ties to F-09) into the WAL/term now so a deposed or zombie leader (G7-87)
cannot commit. (c) Design **snapshot+log transfer** (follower-behind-compaction) as a
first-class part of the I1.7 "loud" hook the design already anticipates. (d) Plan
**verification via deterministic simulation** (F-03) and a **Jepsen-style
partition-testing** harness for I2+.

**Recommendation.** Produce a **Distributed Fault Model IDR** before I2.1 code, so
replication is built to the failure contract, not the happy path; feed it directly
into the ED-003 adversarial hunt lenses for I2.

**Implementation complexity.** Very-high (this is the core of I2–I5).

**Scientific impact.** High — a fully specified, verifiable per-shard CP failure
model is a genuine contribution.

**Ecosystem impact.** Foundational for every multi-node product and for
certification.

---

### F-06 — No normative Fault Model / Durability-Assumption Register in the corpus (High · CCP-Amendment + Certification)

**Why it matters.** "Lossless or loud" lives in a code comment and a design doc
(`I1.7_Recovery_Design.md`), not in any frozen or ratified normative artifact. There
is nothing an independent implementer or certifier can be held to: no statement of
the fault model, the durability assumptions (fsync semantics, FS ordering, atomic
write unit), the required recovery outcomes, or the observable behaviour under each
fault class. The frozen corpus (correctly, per the freeze) cannot be edited — so this
must enter through the sanctioned instruments.

**Risks.** Two "conformant" runtimes could disagree on whether silently dropping a
tenant with an unparseable directory (G10-108) is acceptable — because nothing
forbids it normatively.

**Long-term consequences.** Certification without a fault model certifies nothing
about resilience — the dimension most likely to cause real-world harm.

**Alternatives.** (a) A **CCP Amendment** adding a normative *Resilience &
Durability* annex: fault taxonomy (this §2 is a starting catalogue), required
outcome per class (recover / detect-and-repair / quarantine-loud), and the
durability-assumption register. (b) An **IDR** if it is deemed reference-only
(weaker; certifiers can't bind other vendors to an IDR). (c) A separate
**Certification** program document referencing the taxonomy.

**Recommendation.** Pursue (a): promote the "lossless or loud" discipline and this
failure taxonomy into a ratified CCP Amendment with a conformance scenario
(Reference Lifecycle CCP-GATE), so it becomes certifiable and binding on Independent
Runtimes A/B. This is the single highest-leverage *standardization* action.

**Implementation complexity.** Medium (documentation + governance, no code).

**Scientific impact.** High — a normative fault model is what elevates ARVES from a
project to a standard.

**Ecosystem impact.** Decisive for cross-vendor certification and trust.

---

### F-07 — Checkpoint/compaction durability barrier lacks directory-fsync and manifest fencing (High · IDR + Runtime)

**Evidence.** `install_snapshot` does dir fsync only best-effort *after* rename
(`:989-991`); `compact` (`:1036-1084`) unlinks segments and superseded snapshots with
**no directory fsync at all**; `rotate` (`:897-908`) creates a new segment with no
dir fsync. Snapshot files use a **deterministic name** (`snap-<up_to>.snap`,
`:801-803`) with no generation number.

**Why it matters.** Without directory-entry durability, a crash after `remove_file`
(compaction) but before the dirent update is durable can **resurrect a deleted
segment** (offset reuse, G8-91) or, for rename, **lose a checkpoint the caller
believed durable** (G1-5). Deterministic snapshot names mean a crash-interrupted
rewrite of the same `up_to` (G5-50) has no A/B fallback. `load_snapshot` choosing the
highest `up_to` can also select a snapshot whose covered segments were already
compacted while a *lower* fallback references now-deleted segments (G5-62).

**Risks.** Silent offset reuse or checkpoint loss — both violate the append-only /
durable-truth contract (IDR-005) in narrow but real crash windows.

**Long-term consequences.** These windows are rare per-crash but certain at scale;
they undermine the very ordering guarantees the log exists to provide.

**Alternatives.** (a) **fsync the shard directory** after every rotate, snapshot
rename, and compaction unlink. (b) **Generation-numbered snapshots** with A/B slots
and a fenced pointer to the active one. (c) Make the **F-01 manifest the single
fence**: compaction/rotation is only "done" once the manifest (fsync'd) says so;
recovery trusts the manifest, not the raw dirents. (d) Couple snapshot validity to
its retained-segment set so a fallback snapshot never references deleted segments.

**Recommendation.** Implement (a) immediately (cheap, closes several windows), and
fold (c)+(d) into the F-01 manifest work. Record the durability-barrier ordering as
part of the F-01/F-02 **IDR**.

**Implementation complexity.** Medium.

**Scientific impact.** Moderate (correct crash-consistent metadata protocol).

**Ecosystem impact.** Moderate — required for trustworthy on-disk format across FS
implementations.

---

### F-08 — Content-addressed idempotency trusts an unspecified hash with no payload verification (Medium · IDR + Runtime)

**Evidence.** `commit` treats a matching `ContentHash` as `AlreadyCommitted` and
returns the *existing* truth **without comparing payloads** (`arves-kernel/src/lib.rs:660-663`).
`ContentHash`/`ContentId` are opaque `Vec<u8>` with the digest function *"intentionally
unspecified"* (kernel doc `:78-80`); the runtime never verifies that a stored
payload actually hashes to its content id. `install_state` silently installs empty
state on a decode failure (`:591`, `unwrap_or_default()`).

**Why it matters.** ORCH-004 (idempotent + content-addressable) is a *safety*
invariant for truth. If the hash is weak or attacker-influenced, a collision aliases
two distinct payloads to the same truth (G4-51) — the second is silently swallowed as
"already committed," a **truth-forking / truth-suppression** bug. If a payload is
corrupted on disk but its (separate) content id survives, replay reconstructs *wrong*
truth under a *correct* reference, and the idempotency index will even suppress a
correct re-commit. `install_state`'s silent-empty fallback is a latent silent-loss
class that slipped past I1.7 (it lives in the *snapshot restore* path, not the tail
replay path the hunt focused on).

**Risks.** Silent wrong-truth and truth-suppression — the highest-consequence class
for a "truth owner."

**Alternatives.** (a) **Mandate a cryptographic digest** (BLAKE3/SHA-256) for content
addressing (ties to F-02) and *verify payload↔content on commit and on replay*,
failing loud on mismatch. (b) On an idempotent hit, **compare payload bytes** and
reject (loud) if they differ under the same hash. (c) Make `install_state`
**fallible** and loud on decode failure instead of `unwrap_or_default()`.

**Recommendation.** Do (a)+(b)+(c). Record the digest choice and payload-verification
rule as an **IDR** ("Content addressing integrity"), since the digest function is
currently an open decision.

**Implementation complexity.** Medium.

**Scientific impact.** Moderate — makes content-addressing a verifiable integrity
mechanism, not just a dedup key.

**Ecosystem impact.** Content ids become cross-runtime-portable identity — important
for I2 replication and marketplace/product objectives.

---

### F-09 — No epoch/fencing token; `term` hardcoded to 0; dual-mount and stale-writer unguarded (Medium · IDR + Runtime)

**Evidence.** Kernel commit writes `term: 0` (`:671`); checkpoint writes
`install_snapshot(head-1, 0, ...)` (`:623`). There is **no lock file** or fencing
token on the store root, so two processes (stale container, operator copy, dual
mount — G3-38, G10-102) can both open and append, interleaving offsets and corrupting
the log. There is no generation/epoch to detect a stale writer re-attaching (G8-90)
or an operator restoring an older backup (G10-101, offset rewind).

**Why it matters.** The WAL is *shaped* like a Raft log (offset=index, term present),
and I2.0 leans on that. But `term=0` throws away the epoch that will fence stale
leaders (G7-87 zombie leader), and the missing single-writer fence makes even the
*single-node* guarantee violable the moment two processes touch the same directory —
a common operational reality.

**Risks.** Log corruption from concurrent writers; undetected backup rewind; a future
zombie-leader safety hole pre-baked into the storage format.

**Alternatives.** (a) **Advisory lock file / OS file lock** on the store root,
refusing to open if held (single-writer fence). (b) A **monotonic epoch/generation**
persisted in the manifest (F-01), bumped on every open, so a stale writer or rewound
backup is detected loudly. (c) Thread the **real `term`** through commit/checkpoint
now so the WAL is genuinely Raft-log-shaped for I2 (avoids a format migration later).

**Recommendation.** Implement (a) immediately; fold (b) into the manifest; do (c)
before I2.5 (Raft log). Record as an **IDR** ("Single-writer fencing + epoch").

**Implementation complexity.** Medium.

**Scientific impact.** Moderate.

**Ecosystem impact.** Prevents a whole family of operational-mistake data corruptions;
essential for containerized/cloud deployment.

---

### F-10 — Resource-exhaustion and partial-checkpoint handling is ad-hoc; `Durability(String)` is an untyped dead end (Medium · IDR + Runtime)

**Evidence.** ENOSPC/EIO surface as `WalError::Durability(String)` (`:944`, `:949`,
`:982`) — an opaque, non-actionable variant; `commit` maps it to `Rejected{reason}`.
`checkpoint()` returns `Err(String)` mid-loop (`:613-629`), leaving some shards
compacted and others not. `replay_from`/`load_snapshot` read entire segments/snapshots
into RAM (`:1104`, `:1016`) — recovery of a huge shard OOMs and crash-loops (G9-98).
WAL growth is unbounded absent a caller-driven checkpoint policy (G9-99).
`payload.len() as u32` truncates for >4 GiB (G4-53).

**Why it matters.** Disk-full and OOM-during-recovery are among the most common
production incidents; today they degrade to opaque errors, partial state, or
crash-loops rather than defined backpressure/quarantine.

**Alternatives.** (a) A **typed error taxonomy** (`OutOfSpace`, `IoError`,
`MediumReadError`, `TooLarge`) so callers and operators can act. (b) **Streaming
replay** (bounded memory) instead of whole-file `Vec`. (c) A **checkpoint/compaction
policy** (size/age triggers) so the log can't grow unbounded. (d) **Reserve headroom**
so recovery/compaction can always make progress under near-full disk. (e) Reject
oversize payloads loudly rather than truncating the length field.

**Recommendation.** Do (a)+(b)+(e) as Runtime hardening; specify (c)+(d) as an
**IDR** ("WAL retention & backpressure policy").

**Implementation complexity.** Medium.

**Scientific/Ecosystem impact.** Moderate — operability and scale-ceiling.

---

### F-11 — Poisoned-mutex `.expect` converts one panic into permanent node unavailability (Medium→High blast radius · Runtime)

**Evidence.** Every access uses `.expect("state poisoned")` / `.expect("wal poisoned")`
(`:388`, `:407`, `:654`, etc.). Rust poisons a `Mutex` if a holder panics; every
subsequent access then panics. Combined with F-04's single address space, one panic
anywhere → the node is dead and crash-loops.

**Why it matters.** Turns a localized, possibly-transient fault into a total, sticky
outage — the opposite of graceful degradation.

**Alternatives.** (a) Use `parking_lot::Mutex` (no poisoning) or explicitly recover
from poison and rebuild guarded state. (b) A supervisor that re-initializes the shard
from durable state on poison. (c) Reduce panic sites (F-04, F-08, F-10 typed errors).

**Recommendation.** Adopt non-poisoning locks or a poison-recovery policy; couple
with the F-04 quarantine state machine.

**Implementation complexity.** Low.

**Scientific impact.** Low. **Ecosystem impact.** Moderate (availability).

---

### F-12 — Multi-shard recovery is all-or-nothing and serial (Medium · Runtime)

**Evidence.** `try_replay` (`:506-571`) iterates shards and returns on the first
`RecoveryError`; one bad shard aborts recovery of *all* shards. `shards()` silently
drops unparseable directories (tie to F-01).

**Why it matters.** Per-tenant isolation (SHARD-001, Vol 2) is a core promise; a
single corrupt tenant should not deny service to thousands of healthy tenants. This
is both a resilience and a *multitenancy fairness* issue.

**Alternatives.** (a) **Per-shard recovery outcomes**: recover healthy shards, mark
bad ones `Quarantined` (F-04), serve the healthy set. (b) **Parallel per-shard
recovery** for restart-time bounds at scale. (c) Loud-but-non-fatal aggregate report.

**Recommendation.** Make recovery per-shard-isolated with a quarantine result;
integrate with F-04's state machine. Keep "loud" at the shard level, not the node
level.

**Implementation complexity.** Low–medium.

**Scientific impact.** Low. **Ecosystem impact.** High for multitenant SLA.

---

## If ARVES were standardized by ISO/IEEE tomorrow — what's still missing (failure lens)

1. **A normative Fault Model + Durability-Assumption Register** (F-06): the standard
   currently asserts truth durability without stating what it assumes of the medium or
   what an implementation must do under each fault class. This is the single largest
   gap.
2. **A verifiable log-truth root** (F-01/F-02): "append-only" is a convention, not a
   tamper-evident, loss-detectable structure; a standard needs the latter.
3. **Mechanized fault-injection conformance** (F-03): certification must be by
   machine-checked crash/partition histories, not human hunts and hand-picked flips,
   or Independent Runtimes A/B are not comparably certified.
4. **A specified distributed failure model** (F-05): partition/split-brain/byzantine
   client-observable semantics are undefined — unacceptable for a CP standard.
5. **An operator/recovery state contract** (F-04/F-12): quarantine, repair, and
   degradation must be defined states, not `panic!`.

The runtime's storage discipline is a strong, honest foundation — the I1.7
"lossless or loud" ethic is exactly right. The work now is to **generalize it**: from
single-node to distributed, from trusted-medium to adversarial-medium, from
detected-loss to repaired-loss, and from a code comment to a certifiable normative
standard.

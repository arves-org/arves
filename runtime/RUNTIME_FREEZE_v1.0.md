# ARVES Runtime — v1.0 Freeze Record

```
=====================================================
  ARVES Runtime v1.0    ·    STATUS: FROZEN
=====================================================
```

**Ratified by the maintainer (2026-07).** From this point the ARVES **Runtime Platform**
is a *stable substrate that products consume* — not a moving target that reshapes under
each new feature. This is the boundary that turns ARVES from one large application into a
platform (the discipline of the Linux kernel, the Kubernetes API, POSIX).

Recorded per ED-001 in the living repository: the freeze is a **git tag** (`runtime-v1.0`)
+ this record, never an edit to the frozen specification corpus.

## The two platforms (separated)

| Runtime Platform (FROZEN, slow) | Product Platform (fast) |
|---|---|
| Kernel · Persistence · Consensus · Engine Fabric · Capability Fabric · Bridge · ACS codec · SDK core | Personal AI · Enterprise AI · Healthcare · Factory · Government · Studio · Marketplace |
| `runtime/`, `standard/` | `products/` |
| changes ONLY via a Runtime Change Request (below) | ships value continuously |

A product is a **customer** of the runtime, never a co-author of it (IDR-006, now
strengthened by this freeze). Optional future step: physically split into `arves-runtime`
and `arves-products` repositories — the logical boundary below is what matters today.

## What v1.0 guarantees (the stability contract)

Frozen and byte-stable; products may depend on these without fear of drift:

- **Identity** — ACS-001 content address `0x12 0x20 || SHA-256(domain‖body)`; ACS-002
  canonical dCBOR bytes. A value's ContentId is stable forever (three independent-language
  runtimes agree; differentially fuzzed; rejection-conformant).
- **Truth** — the Kernel is the sole commit gateway (OWN-001); commit is idempotent and
  content-addressed (ORCH-004).
- **Persistence · Replay · Audit** — append-only WAL = decision trace; deterministic
  replay/recovery (I1, fault-hardened).
- **Cognitive work chain** — `SDK → Bridge → Capability → Engine → Kernel`, one identity
  end-to-end; capability-gated; engine-pure (ENG-003). *Guarantee scope (RCR-001, item #6):*
  the `arves-engine-fabric` and `arves-capability-fabric` runtime crates are **contract-only**
  (interfaces/types); the exercised engine/capability *logic* flows through the SDK/Bridge in
  `products/`. The stable guarantee is the **contract + the identity-preserving Bridge path**,
  not engine/capability logic living inside those two runtime crates.
- **Robustness** — the whole stack survived a 6-lens destroy pass (21 blocker/major fixed;
  regression-locked). Encoders are depth/range-bounded; the bridge fails safe.

## The Runtime API (what products bind to)

- **SDK** (`products/arves-sdk-ts`, and any conformant re-implementation): content
  addressing + canonical encode; `Arves`, `FactStore`.
- **Bridge** (`arves-bridge`): `commit` and `invoke` over the line protocol → the real
  Kernel/Engine/Capability. ContentIds are ACS-001 (SDK-local == Kernel-committed).
- **Conformance** — `standard/` (specs + golden + negative vectors) defines the contract;
  a runtime is conformant iff it reproduces the vectors and rejects the negatives.

These surfaces are stable for v1.x. Additive, backward-compatible extension is allowed;
a breaking change requires a new major (v2.0) via the process below.

## Runtime Change Request (RCR) — the only way the runtime changes

While building products, if the runtime is found lacking:

1. **STOP** — do not edit the runtime from product work.
2. File an **RCR**: the product need, the exact runtime gap, and why no product-side
   workaround suffices.
3. The **Runtime Team** triages: reject (product-side solution exists), schedule into a
   minor (v1.1, additive/backward-compatible), or escalate to a major (v2.0, breaking).
4. Only a ratified RCR changes the runtime — with its own destroy→repair→prove cycle
   (ED-006) and a version bump. Products keep running on their pinned runtime version
   until they choose to adopt the new one.

> **Applied RCRs (v1.1), records under `runtime/rcr/`:** RCR-001 (closure-audit #6/#7/#9),
> RCR-002 (truth-store tamper-evidence, #8 partial), **RCR-003** (contract-crate citation /
> traceability corrections from `verification/evidence/CONTRACT_CRATE_AUDIT.md` — doc-only across
> 6 crates + one `arves-invariants` `Layer`-helper logic fix with tests; `cargo test --workspace`
> 71→**75/0**; freeze baseline re-advanced via `freeze_check.py update`), **RCR-004** (native Rust
> ACS-003/004/005 semantic validators in `arves-conformance::semantic` — retires the CCP-006
> semantic-tier deferral; rejects all 19 frozen envelope/instance/language vectors with the exact
> registered codes; additive, `cargo test --workspace` 75→**77/0**; harness exposure = RCR-004b),
> **RCR-004b** (new `acs_validate` line-protocol bin exposing the native Rust semantic validators —
> `<tier>\t<hex>` → `ACCEPT`|`REJECT\t<kebab-code>`; lets the certification harness grade the Rust
> reference over the full ACS-003/004/005 reject surface; additive, workspace **81/0**, smoke 19/19), **RCR-005** (Kernel commit-gateway **content-integrity** — reject a re-proposal that
> binds the same `ContentHash` to a *different* payload, `CommitError::ContentIntegrity`; closes the
> Kernel-owned half of gap #3 with no ACS coupling per RULE #9; additive, `cargo test --workspace`
> 77→**78/0**), **RCR-006** (PropertyCheck/Suite — the invariant→executable-proof catalog
> `arves-conformance::property_check`: LAYER-001/OWN-001 executed in-process, ORCH-003/004/SHARD-001
> cited to their biting tests, ORCH-001/002 honestly Pending; shared-checker refactor of the
> architecture gate; additive, `cargo test --workspace` 78→**80/0**; closes gap #18), **RCR-007**
> (SHARD-001 two-tenant isolation test at the truth gateway — `behaviour_8_two_tenant_isolation`
> proves same-content/different-shard is distinct truth with no cross-tenant snapshot leak; upgrades
> the PropertyCheck SHARD-001 citation from structural to a named biting test; closes Madde-11 audit
> finding SHARD-001-F1; additive, `cargo test --workspace` 80→**81/0**), **RCR-008** (live L1
> conformance — `arves-conformance::live` `KernelProbe` + `LiveVerdictEngine` emit the first
> executable `ConformanceArtifact` by driving the real `MemKernel`, deriving ORCH-003/004 + OWN-001
> + SHARD-001 from behaviour → `Verdict::Pass`; adds downward kernel/persistence deps; bin
> `conformance_live`; behaviour L0→L1 for the Kernel node at G0/G1; additive, `cargo test
> --workspace` 81→**83/0**), **RCR-009** (live L1 **Information Platform** node — a reference
> `Connector` canonicalizes a Source into a deterministic content-addressed `ProposedWrite` with the
> five ontology aspects; `InformationPlatformProbe` + the first two-node Information→Kernel
> `ConformanceArtifact`, `Verdict::Pass`; additive, `cargo test --workspace` 83→**85/0**), **RCR-010**
> (live L1 **Query** node — a read-only `QueryProjection` reconstructs committed truth by replaying
> the persistence WAL (no Kernel read, ORCH-001/OWN-001) with tenant-scoped isolation; completes the
> first **end-to-end** Information→Kernel→Query `ConformanceArtifact`, `Verdict::Pass`; additive,
> `cargo test --workspace` 85→**87/0**), **RCR-011** (bridge **request-id correlation** — the line
> protocol accepts an optional `id=<token>` first token echoed verbatim on the response, so clients
> match responses **by id instead of position**; a dropped/injected/reordered line can no longer
> shift every later response onto the wrong caller; malformed ids refused as `ERR bad-id` without
> echo; backward compatible — un-prefixed lines byte-identical to before; closes v1.1 backlog
> item 1; additive, `cargo test --workspace` 87→**91/0**, product regression 49→**50/50** incl. a
> biting reverse-order fake-bridge test), **RCR-012** (engine-**enforced** determinism &
> fabric-derived idempotency — `arves-engine-fabric` gains `invocation_key` (the FABRIC derives the
> ORCH-004 key: ACS-001 address of the canonical input under domain 0x04; engines no longer
> self-mint it — closes `PureEngine`'s documented NON-CONFORMANT placeholder) and `invoke_enforced`
> (key verification + a double-invoke probe that REFUSES a false `Determinism::Deterministic`
> declaration instead of trusting it); the bridge invokes engines only through it, so refusal
> happens BEFORE any effect reaches the Kernel; new LAYER-001-legal downward edge
> engine-fabric→acs; closes v1.1 backlog item 2; additive, `cargo test --workspace` 91→**97/0**),
> **RCR-013** (same-shard **atomic batch commit** — `RefKernel::commit_batch` validates the whole
> batch first (cross-shard refused per IDR-004 saga rule; content-integrity forks against
> committed truth AND intra-batch forks refuse the WHOLE batch, zero applied) then applies through
> the identical single-commit gateway under one lock; identical duplicates resolve idempotently
> (`fresh:false`), never fork; honest boundary — a mid-apply host I/O failure surfaces loudly as
> `PartialApply` (WAL-transactional apply is I2/Raft work); frozen `Kernel` trait untouched; closes
> v1.1 backlog item 3; additive, `cargo test --workspace` 97→**98/0**), **RCR-018** (doc-only,
> like RCR-003 — de-drift of the frozen `runtime/docs/ARVES_Master_Roadmap.md`: stale
> `arves-standard-kit 0.2.0` → **0.3.1** via `standard/VERSION` (closes OPEN_DEBT §F "MR-drift"),
> and every era/gating claim invalidated by maintainer **Ruling 002** (2026-07-05,
> `docs/MAINTAINER_RULINGS.md`) marked superseded in place — "gated behind Era 3" / "post-G2" /
> "not I2" / "forbidden until certified" — never silently deleted; `runtime/README.md` audited
> CLEAN, untouched; no logic, no code, `cargo test --workspace` stays **98/0**), **RCR-014**
> (bridge **per-request shard selection** — optional `shard=<tenant>/<workspace>` token after the
> optional `id=` token (each part non-empty, whitespace-free, ≤64 bytes) scopes that commit/invoke
> to that shard; absent → default `t1/w1`, byte-identical; same body in two shards = two distinct
> truths with distinct indexes/idempotency scopes (SHARD-001 observable through ONE process — ends
> forced process-per-tenant); malformed → `ERR bad-shard` without reflecting the untrusted spec;
> documented choice: invoke in a never-bound shard is honestly `ERR unbound`, no implicit
> auto-bind (use RCR-016 `bind`); SDK client gains `{tenant, workspace}` opts; additive,
> `cargo test --workspace` 98→**102/0**, product regression 50→**52/52** incl. a real-exe
> raw-protocol isolation bite), **RCR-016** (bridge **dynamic capability bind verb** — new
> `bind <capability>` (composable with `id=`/`shard=`) registers+binds the name in the target
> shard to the ONE reference engine identity `engine:derive.fact@1.0.0` → `bound <capability>`;
> rebinding the same name in the same shard is idempotent `bound`; malformed → `ERR bad-request`;
> HONEST SCOPE: binds NAMES to the one hosted reference engine, does NOT load arbitrary engine
> code; default binding and verb share one helper so dynamic names behave like the built-in by
> construction; SDK client gains `bind(capability)`; additive, `cargo test --workspace`
> 102→**106/0**, product regression 52→**53/53** incl. a real-exe bind→invoke bite; capstone
> PASS on the new exe), **RCR-015** (bridge **durable truth via `--wal-dir <path>`** — the bin
> constructs `FileKernel::try_recover(FileWalStore::open_root(path))` instead of `MemKernel`:
> fsync-durable commits + deterministic recovery replay on startup (ORCH-003, lossless-or-loud —
> an unrecoverable dir refuses startup, never partial truth); no flag → MemKernel byte-identical;
> unknown args refused loudly (no silent volatile fallback); session loop factored generic
> (`serve<K: Kernel>`) so both arms run identical protocol logic; SDK client gains `{walDir}`
> opts; HONEST SCOPE: single-host durability, CRC32 + RCR-002 hash-chain tamper-EVIDENCE, NO
> authN (v2.0 debt #8), no replication (Raft is I2+); additive, `cargo test --workspace`
> 106→**108/0**, product regression 53→**55/55** incl. a real-exe hard-kill/restart round-trip
> answering `already-committed` with the SAME ContentId+index), **RCR-017** (**opaque ShardKey**,
> closes audit finding SHARD-001-F2 — `arves-kernel::ShardKey` fields made private
> (immutable-BY-TYPE, SHARD-001) with sole constructor `ShardKey::new(tenant, workspace)` rejecting
> empty and >256-byte parts (`ShardKeyError`) + `tenant()`/`workspace()` accessors;
> `arves-capability-fabric::ShardKey` aligned with IDENTICAL rules so kernel→fabric conversion at
> the bridge seam is total; every in-workspace call site updated (kernel tests, runtime bin,
> conformance live probes/Connector, bridge lib+bin incl. the RCR-014 `shard=` parser whose ≤64B
> grammar is strictly tighter); workspace-internal breaking refactor, WIRE-COMPATIBLE — no external
> Rust consumers (IDR-006), line protocol byte-identical; biting tests
> `behaviour_10_degenerate_shard_key_unrepresentable` + fabric `rcr017_*` prove an empty tenant is
> unrepresentable; honest scope: type-surface fix, not distributed placement immutability (I2+);
> `cargo test --workspace` 108→**110/0**, product regression stays **55/55** on the rebuilt exe),
> **RCR-019** (I2 Stage 1 — deterministic per-shard **Raft CORE** inside `arves-consensus`,
> implemented additively BEHIND the frozen contract per `docs/design/I2_Cluster_Kernel_Design.md`
> under maintainer Ruling 002: new `raft` module (pure step-function state machine — terms,
> seeded-randomized election timeouts via injected logical tick, log replication, quorum commit
> with the §5.4.2 current-term guard, follower catch-up backtracking, stale-leader step-down) +
> new `sim` module (deterministic in-process MessageBus harness, drops/partitions as bus filters,
> continuous per-step checking of all four Raft safety properties, and `SimShardConsensus` — the
> first impl of the frozen `ShardConsensus` trait; follower handles refuse commits `NotLeader`,
> OWN-001); the four safety properties + failover/partition/catch-up scenarios land as
> deterministic scripted tests (no sleeps, no wall clocks, no OS randomness — SplitMix64 from
> recorded seeds only); HONEST SCOPE: in-process simulation only — no network transport, no
> WAL wiring (IDR-005 unification is the persistence-wiring stage), no joint-consensus
> membership (I2.8), no real read tiers (I2.9), no snapshots (OQ-1); no frozen signature
> changed, zero new deps (LAYER-001 gate green); additive, `cargo test --workspace`
> 110→**126/0**), **RCR-020** (I2 Stage 2 — **multi-shard consensus instance map +
> JOINT-CONSENSUS membership + leadership transfer** inside `arves-consensus`, additively
> behind the frozen contract per `docs/design/I2_Cluster_Kernel_Design.md` ladder step
> I2.8: `VoterConfig` with the IDR-003 C_old,new DUAL-majority rule gating both elections
> and commits (config effective on append, rollback on truncation — Raft §6), two-phase
> `change_membership(Stable target)` → joint entry → auto-appended C_new on joint commit →
> same-term leader step-down when excluded (`voted_for` preserved for Election Safety);
> `MsgBody::TimeoutNow` leadership transfer (target campaigns at term+1) + thesis-§4.2.3
> leadership check (removed servers cannot disrupt healthy leaders; transfer bypasses);
> `SimShardMap` — exactly ONE independent Raft group per immutable `ShardId` (IDR-001,
> SHARD-001 blast-radius isolation proven; duplicate group refused loudly); deterministic
> scripted tests prove the no-two-disjoint-majorities window (old-majority side cannot
> commit, new-majority side cannot elect, mid-transition), add/remove mid-stream, leader
> self-removal recovery, and loss-free transfer; HONEST SCOPE unchanged: in-process
> simulation only — no network transport, no WAL wiring, no read tiers (I2.9), no
> snapshots (OQ-1), no learner promotion; no frozen signature changed (no new error/entry
> variants), zero new deps (LAYER-001 gate green); additive, `cargo test --workspace`
> 126→**139/0**), **RCR-021** (I2 Stage 3 — the **CLUSTER KERNEL** inside `arves-kernel`
> per design §6.2 row 2 (the Kernel commit-path wiring RCR): `ClusterKernel` implements
> the frozen `Kernel` trait over the RCR-019/020 per-shard Raft substrate — commit
> authoritative ONLY on the shard leader (`CommitError::NotLeader{shard}` live —
> OWN-001/IDR-004), the IDENTICAL `RefKernel` gateway admission (ORCH-004 dedupe,
> RCR-005 content-integrity — `commit_inner`'s head factored into a shared `admission`,
> never forked) runs BEFORE replication, ack only after quorum, `NotReplicated` live on
> lost quorum (IDR-001 CP) with the RCR-019 DR-8 identity check; deterministic apply
> loop commits every replicated outcome through the SAME gateway on every replica —
> follower truth byte-identical (same ContentHashes/CommitIndexes/per-shard state-blob
> bytes, ORCH-003 across nodes; membership entries never enter the Kernel); Kernel
> snapshot install for a crashed/lagging follower (IDR-002 snapshot-then-log-tail: truth
> state + dense WAL continuation + cursor jump) and crash recovery by local-WAL replay
> (I1.7); ONE new dependency edge kernel(40)→consensus(30), downward-only, architecture
> gate re-verified green; scenarios S-I2-1/-3/-4/-8 land as deterministic in-process
> analogues (leader-minority partition → NotReplicated with zero partial truth → heal →
> fresh commit through the successor; crash→snapshot→catch-up with aligned offsets);
> HONEST SCOPE: in-process simulation only — no network transport; IDR-005
> raft-log/WAL unification still deferred (in-memory raft core log + per-replica durable
> WAL), no raft-state crash durability, no read tiers (I2.9), no cluster batch; no
> frozen signature changed; additive, `cargo test --workspace` 139→**147/0**), **RCR-022**
> (I2 Stage 4 — **DISTRIBUTED PROOFS + the I2 milestone record**, closing the I2 Cluster
> Kernel series RCR-019..022 per the design's conformance plan: deterministic
> duplicate/reordered-delivery mangling on the sim bus (counter-scripted, zero
> randomness, mangling trace folded into the replayable history digest) + adversarial
> cluster tests — symmetric 2/3 partition (minority `NotReplicated` with zero partial
> truth, majority commits, heal → ONE truth), old-leader-returns (stale term refused
> everywhere, deposes nobody, stale entry provably absent from truth), dup/reorder
> storms with client retries (ORCH-004 at cluster level: truth exactly-once per content
> address on every replica; consensus-level: every digest commits exactly once), and
> full-cluster rebuild-from-WAL (ORCH-003: every node rebuilt from its own log →
> identical `truth_hash`/state bytes); plus the S-I2-6 live conformance artifact
> (`l1-cluster-kernel-distributed`, `Verdict::Pass` — two tenants on two independent
> replicated Raft groups, interleaved failover, zero cross-tenant leakage on every
> replica, per-shard leadership; fingerprint pins the honest claim "L3(scoped): L1
> node-set under distributed deployment / in-process deterministic simulation, no
> network transport") and extended ORCH-003/004+SHARD-001 catalog citations (coverage
> counts unchanged — ORCH-001/002 stay honestly Pending, Control Plane still
> contract-only); ONE new downward edge conformance(110)→consensus(30), architecture
> gate green; HONEST I2 SERIES SCOPE: an in-process deterministic cluster — NO network
> exists, NO network fault-tolerance claimed; S-I2-7 read tiers NOT delivered (OQ-6 →
> IDR, I3); recorded inheritance to I3+: IDR-005 raft-log/WAL unification, transport +
> `ShardConsensus` rewiring (RCR-021 DR-14), protocol snapshots/compaction (OQ-1),
> placement (OQ-8/I4), threat model (OQ-7); no frozen signature changed; additive,
> `cargo test --workspace` 147→**155/0** — I2 series total 110→155. Record:
> `runtime/rcr/RCR-022.md`), **RCR-023** (I3 Stage 1 — the **single-node QUERY CORE**
> inside `arves-query` per `docs/design/I3_Distributed_Query_Design.md`: `ShardProjection`
> (read-only disposable per-shard fold `Proj(shard,v)=fold(apply,∅,WAL[0..v))` — IDR-005 /
> ORCH-003; deterministic snapshot-at-index builds, suffix catch-up, fold digest sharing the
> Kernel `truth_hash` tuple basis) + `WalQuery` — the FIRST implementation of the frozen
> `Query` trait (`read`/`exists`/`latest_version`), scope validation before I/O, SHARD-001
> tenant/workspace scoping, IDR-001 tiers in **single-node degenerate** form (Linearizable/
> Bounded catch up to the local head — the sole replica's committed log IS the commit index;
> Eventual serves the standing fold, observably stale, never wrong for its `observed_at`;
> `StalenessBoundExceeded` unreachable in this core — OQ-2 attestation IDR pending); reads by
> WAL replay ONLY, NO Kernel read hook (ORCH-001/OWN-001); executable proofs: two-tenant
> isolation on every tier + structural fold isolation, projection digest == kernel
> `truth_hash` incl. across recover, pinned-build equality + checkpoint⊕suffix ≡ full
> replay, reads-change-nothing (WAL head + truth_hash invariant) + idempotent identical
> results, MalformedScope-before-routing; ONE new downward edge query(60)→persistence(20)
> (LAYER-001, gate green; kernel is dev-dep only), OQ-7 resolved to raw payload bytes +
> hex-of-`ContentId` ids; HONEST SCOPE: single process, single replica — no routing fabric,
> no follower reads, no real read-index, no scatter-gather, no LCW views (OQ-8), no network;
> RCR-010's conformance `QueryProjection`/probe stays UNMODIFIED (design §2); QUERY-001
> still PROPOSED (enforced via the registered A-003 row + trait shape); no frozen signature
> changed; additive, `cargo test --workspace` 155→**166/0**. Record: `runtime/rcr/RCR-023.md`),
> **RCR-024** (I3 Stage 2 — **DISTRIBUTED READS** over the I2 cluster substrate per
> `docs/design/I3_Distributed_Query_Design.md`: `arves-query::distributed::ClusterQuery`, a
> per-replica read handle over `ClusterSim` implementing the frozen `Query` trait with
> shard-aware routing (SHARD-001 directory resolution) and the IDR-001 ladder served
> HONESTLY — Linearizable = in-process read-index (highest-term leader's commit index,
> VALID only under the Raft §6.4 precondition: the leader has a committed entry of its
> CURRENT term — DR-8, revision closing the RCR-019 DR-2 interaction where a fresh
> leader's commit index excludes prior-term acked entries; serve only at a replica
> applied ≥ a valid read-index, else `LeaderUnavailable`; CP, refuses under partition,
> at a deposed minority leader, and at a new leader without a current-term commit),
> BoundedStaleness = admitted ONLY on
> provably-ZERO lag against a valid read-index (applied ≥ leader commit ⇒ 0ms ≤ any
> bound, clock-free; same DR-8 gate), else refused
> with the `LAG_UNATTESTABLE` sentinel (OQ-2 time↔index IDR still pending — nothing
> fabricated), Eventual = the replica's local WAL fold, always available, staleness
> LABELED (`served_tier`/`observed_at` — AP observability, IDR-005 CP/AP split); plus
> additive surfaces per design §3.3/§6.2/OQ-5 (frozen trait untouched): `gather_read` →
> `GatheredRead` tenant-internal scatter-gather (non-atomic union, per-shard version
> vector, NO global version, deterministic ascending merge, fail-WHOLE on any sub-read
> failure — OQ-4 resolved without widening the frozen error enum; single-tenant fan-out
> with sub-reads routed on the TYPED `ShardId`, never re-parsed `"tenant/workspace"`
> text — DR-9, revision closing the RCR-023 DR-2 `/`-in-part ambiguity on the gather
> surface) and `read_at_least`/`floor_of`/`FloorReadError` read-your-writes
> floor (checked BEFORE presence: a lagging replica answers `BelowFloor`, never a false
> `NotFound`); reads stay WAL replay ONLY (RCR-023 `ShardProjection` reused; ORCH-001 — the
> four new read-only `ClusterSim` accessors `shards`/`commit_index_of`/`wal_store_of`/
> `has_committed_in_current_term` (+ the `SimCluster` introspection it delegates to)
> expose routing metadata + the Persistence substrate, never Kernel truth; queries take
> only immutable sim borrows — structurally write-free); executable proofs: read-index at
> leader AND current followers with identical projections (ORCH-003 across nodes),
> partitioned follower serves LABELED stale Eventual + refuses both strong tiers while the
> majority leader serves quorum truth then converges on heal, deposed-minority-leader
> refusal, read-your-writes floor at current vs lagging replicas, scatter-gather bit-equal
> across independent runs + fail-whole under lag, cluster-wide two-tenant isolation on
> every replica × tier with zero truth change, PLUS the two revision regressions (acked
> write never silently missed after a leader change — strong tiers refuse until a
> current-term commit; `/`-bearing-tenant gather serves only its own typed shard); TWO
> new downward edges query(60)→kernel(40)
> + query(60)→consensus(30) (LAYER-001 gate green; design §3.4 rows 2/4; still no LCW
> edge — OQ-8); HONEST SCOPE: in-process `ClusterSim` vehicle — no network, no read-index
> heartbeat round (omniscient directory closes the stale-leader hazard; the §6.4
> current-term-commit precondition — the hazard the directory does NOT close — is
> enforced by DR-8's refusal), no real ms lag
> attestation, sequential deterministic fan-out, no authN/authZ (OQ-1), QUERY-001 still
> PROPOSED; no frozen signature changed; additive, `cargo test --workspace` 166→**176/0**.
> Record: `runtime/rcr/RCR-024.md`), **RCR-025** (I3 Stage 3 — **ADVERSARIAL READ PROOFS
> + the I3 milestone record** per `docs/design/I3_Distributed_Query_Design.md` §4/§5:
> `arves-query/tests/adversarial_reads.rs` proves (a) **torn-read impossibility** — a
> query never observes a partially-applied RCR-013 batch: every reader-reachable
> observation point sits on a batch boundary, each batch is visible all-or-none on every
> tier, every served `observed_at` is provably a boundary, refused batches change nothing
> bit-identically (honest limits stated: `at_version` CAN pin the per-record trace
> mid-batch — audit surface only; `PartialApply` host-I/O and the CLUSTER batch form stay
> the RCR-013/021 deferred boundaries); (b) **replay equivalence** — on every replica the
> rebuilt-from-own-WAL fold equals the live-served read (position + bytes), rebuilds are
> equal across replicas, full-cluster crash/recover changes nothing, every served read is
> reproducible by a pinned rebuild at its `observed_at`; (c) **partition reads** — 5-node
> 2/3 minority: AP reads stay BIT-IDENTICAL to the pre-partition capture (labeled, old
> position), fabricate NOTHING (majority-only truth absent in every read form; the
> visible universe is exactly the old prefix), strong tiers refuse; heal converges all
> five projections to equality (post-heal marker commit validates the read-index per the
> RCR-024 DR-8 precondition — refusal, never silent staleness); (d) **query determinism
> under message storms** — with duplicate/reorder mangling ACTIVE and provably biting on
> both shard buses, two identically-scripted runs produce bit-identical query transcripts
> (mid-storm AND converged) and replicas converge to identical folds; PLUS the live
> conformance raise: `arves-conformance::live` gains the **Enterprise Knowledge Query
> under distribution** artifact (`enterprise-knowledge-query-distributed` — the design
> §5.1 frozen reference scenario; axes 1+8+12, axis 9 honestly omitted: no concurrent
> readers exist in-process; axis 8 via its tenant-isolation clause ONLY — no
> volume/throughput/backpressure exercised in-process, RCR-025 DR-3) riding the
> RCR-023/024 `arves-query` fabric over the cluster
> substrate with every check derived from behaviour (`Verdict::Pass`; fingerprint states
> "no network transport"); RCR-010's single-node `QueryProjection`/probe stays
> byte-unmodified and its L1 artifact green (design §2); the PropertyCheck catalog
> (RCR-006) cites all I3 proofs on the SHARD-001/ORCH-003/ORCH-004 rows CITATION-ONLY —
> coverage honestly unchanged at 5 proven / 2 pending (ORCH-001/002 stay Pending,
> Control Plane contract-only); ONE new downward edge conformance(110)→query(60)
> (LAYER-001 gate green); QUERY-001 still PROPOSED (its §5.4 CCP-GATE scenario now EXISTS
> as a live artifact; ratification stays maintainer-gated); no frozen signature changed;
> additive, `cargo test --workspace` 176→**181/0** — I3 series total 155→181. §5.2
> Stage-3 is discharged EXCEPT "membership change under load (IDR-003)": the kernel-layer
> `ClusterSim` exposes no membership API (partition/heal/crash only), so NO query read
> crosses a membership transition in I3 — that item's evidence maps to the I2 raft-layer
> joint-membership suite (consensus-layer, not query-layer) and the query-layer proof
> (incl. the §3.6 stale-routing `UnknownShard`/refresh story under a real transition) is
> inherited by I4+ (RCR-025 DR-7); "leader kill"/"crash-rebuild during serving" are
> discharged in their approximated deposed-leader / sequential crash-then-serve forms
> (RCR-025 DR-8). THE I3
> MILESTONE RECORD (delivered scope, honest NON-claims, I4+ inheritance: OQ-2 attestation
> IDR, networked read-index, protocol snapshot bootstrap, LCW views OQ-8, authN/authZ
> OQ-1, typed shard key, distributed batch, query reads across membership change (DR-7),
> QUERY-001 CCP) lives in
> `runtime/rcr/RCR-025.md` (v1.2, amended per adversarial review)).

## Organization (three teams, three mandates)

- **Runtime Team** — *never break.* Owns `runtime/` + `standard/`; guards the guarantees
  above; changes only via RCR. Never thinks about "how will Personal AI look?".
- **Product Team** — *ship value.* Owns `products/`; consumes the frozen Runtime API;
  never thinks about "how is the WAL written?".
- **Verification Team** — *break everything.* Owns `verification/`; runs the destroy /
  chaos / differential / fuzz / property passes against both.

## v1.1 backlog (known debt — deferred, non-blocking for products)

Recorded, important, and explicitly NOT blocking P4 (per the destroy-round report):

1. **Bridge request-id correlation** — replace positional FIFO with explicit request ids
   (today: input-sanitization + response-shape validation close the reachable desync).
   **ADDRESSED by RCR-011 (v1.1):** the protocol accepts an optional `id=<token>` echoed
   on the response; the SDK client matches by id (FIFO retained only as the fallback for
   id-less lines). Backward compatible; see `runtime/rcr/RCR-011.md`.
2. **Engine-enforced determinism** — the fabric derives/enforces the idempotency key
   rather than trusting an engine's self-declared `Determinism` (today: the reference
   `PureEngine` is pure by construction).
   **ADDRESSED by RCR-012 (v1.1):** `invocation_key` (fabric-derived, content-addressable
   ORCH-004 key) + `invoke_enforced` (key verification + a double-invoke probe refusing a
   false `Deterministic` declaration), enforced on the bridge's real invoke path. The
   probe is honestly a probe, not a proof — see `runtime/rcr/RCR-012.md`.
3. **Kernel batch-commit** — atomic multi-effect / multi-shard commit (today: single-effect
   invocations are all-or-nothing; multi-effect effects are independent idempotent truths).
   **ADDRESSED by RCR-013 (v1.1) for the same-shard half:** `RefKernel::commit_batch` is
   all-or-nothing across the validation class under one lock. The multi-SHARD half is
   deliberately NOT a commit — IDR-004 rules cross-shard intent a saga; that path is I2+
   (per-shard Raft) work. See `runtime/rcr/RCR-013.md`.

### Added by the Build Program Closure Audit (2026-07) — RCR-tracked

These are honest findings from the independent 15-pillar closure audit. They do **not**
block closing the (correctly-scoped, single-node I1) Build Program, but each is recorded here
as v1.1/v2.0 debt and must enter via an RCR — never a silent crate edit under the freeze.

4. **Runtime source doc-integrity** — ~13 crates carried stale
   `I1 skeleton — NO implementation yet` headers, yet kernel+persistence are fully
   implemented (working FileKernel/WAL/recovery/checkpoint; 65 tests). **ADDRESSED by RCR-001
   (v1.1):** every stale header corrected to state each crate's actual status — kernel /
   persistence / invariants marked IMPLEMENTED; engine-fabric / capability-fabric /
   control-plane / query / lcw / ontology / information-platform / conformance / consensus
   marked CONTRACT-ONLY (by design / deferred). Comments only, no logic change.
5. **`CancellationToken::is_cancelled()` no-op** (arves-execution) — unconditionally returned
   `false`; the Amendment-005 cooperative-cancellation capability silently did nothing.
   **ADDRESSED by RCR-001 (v1.1):** the token is now backed by a shared `Arc<AtomicBool>`;
   `is_cancelled()` reflects a real flag, `cancel()` sets it, and clones share one signal.
   Additive (new `cancel()` method; `is_cancelled()` signature unchanged) + 4 unit tests.
6. **Freeze-doc guarantee alignment** — Engine Fabric / Capability Fabric were listed under
   "What v1.0 guarantees," but the exercised engine/capability logic flows through `products/`
   (SDK/Bridge); the runtime crates are contract-only. **ADDRESSED by RCR-001 (v1.1):** the
   "Cognitive work chain" guarantee above now states the contract-only scope explicitly.
7. **Commit `Cargo.lock`** — was gitignored; for a binary-producing workspace it should be
   committed so clean clones resolve byte-identical pinned dependencies (Determinism/Replay
   value). **ADDRESSED by RCR-001 (v1.1):** the `Cargo.lock` entry removed from the root
   `.gitignore` so `runtime/Cargo.lock` can be committed. Non-breaking build hygiene.
8. **Truth-store cryptographic tamper-evidence** (v1.1/v2.0, zero-trust) — the WAL/snapshots
   use CRC32 (error-detection, forgeable) with no hash chain / Merkle root / signature, and
   `Kernel::commit` carries no principal/authN/authZ. v1.0's threat model is a **trusted single
   host**; a multi-tenant / untrusted-host deployment requires a signed, hash-chained truth
   store (independent review `runtime/docs/reviews/P07_security-zero-trust.md`). Public docs
   must not imply cryptographic tamper-resistance of the persisted store under v1.0.
   **PARTIALLY ADDRESSED by RCR-002 (v1.1):** a dependency-free SHA-256 **tamper-evident
   hash-chain digest** (`FileWal::integrity_digest`) now detects any alteration of any committed
   record — including a tamper that repairs the per-frame CRC32 (proven by a regression test:
   `rcr002_integrity`). This closes the "edit one record + fix its CRC" hole and provides the
   chain a signature scheme will sign. STILL OPEN (v2.0): cryptographic **signatures** +
   **authenticated commit** (principal/authN on `Kernel::commit`) + digest **anchoring** — a
   fully hostile host that rewrites the whole trace *and* the anchor still needs signatures to
   stop. Threat model unchanged for v1.0 (trusted single host); see `runtime/rcr/RCR-002.md`.

Each enters via an RCR into v1.1 (or v2.0 for #8's breaking parts), with regression + property tests.

---

*Freeze marker: git tag `runtime-v1.0`. Products (P4→P8) now build on this frozen base;
any runtime gap is an RCR, not a product edit.*

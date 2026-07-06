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
> `runtime/rcr/RCR-025.md` (v1.2, amended per adversarial review)), **RCR-026** (I4
> Stage 1 — the **CAPABILITY FABRIC CORE** inside `arves-capability-fabric` per
> `docs/design/I4_Capability_Scheduling_Design.md` §3.1.1/§3.1.2/§3.1.4/§3.5:
> `lifecycle::LifecycleRegistry` — a second implementation of the frozen
> `CapabilityRegistry` trait with **append-only supersession history** per
> `(shard, capability)` plus additive `revoke` (tombstone at a strictly-higher version,
> never a deletion — RCR-026 DR-3; resolve → hard `Unbound`; a stale pre-revoke version
> can never rebind), `resolve_pinned` (exact historical version for replay — ORCH-003
> basis; superseded/revoked-era bindings stay readable forever, never served as active)
> and `history` (ordered audit chain); `gate` — the AUTHORIZATION GATE formalizing
> fabric-side the exact semantics the bridge exercises: active-binding hard deny
> (F-UNBOUND), engine-IDENTITY match `engine:{name}@{version}` (Vol 9 Part 3 basis;
> CAP-002 stays PROPOSED), every manifest `capabilities_required` resolved in the SAME
> shard (Engine Graph Parts 3/10), caller-supplied Governance `PolicyVerdict`
> enforced-not-owned (`Deny`/`ApprovalRequired` BLOCK before invocation — no HITL
> surface exists yet, DR-5), the declared-`EffectClass` gate (`Pure` must propose
> NOTHING), and `invoke_gated` wiring **`arves-engine-fabric::invoke_enforced`
> (RCR-012)** so every gated invocation gets the fabric-derived ORCH-004 key check +
> determinism probe, returning the **pinned `BindingVersion`** (design §3.8) and
> effects as PROPOSALS ONLY (no kernel edge exists in the crate — commit structurally
> impossible, ORCH-001); ONE new downward edge capability(70)→engine(60) (LAYER-001
> gate green); the bridge is UNTOUCHED (unification onto the fabric gate is a named
> follow-up, DR-7); NOT built and NOT claimed: placement/backpressure (IDR-007-gated),
> selection, trace emission (OQ-10), policy engine, distributed
> registry/revocation/cancellation (OQ-6), disk-durable bindings, authN/authZ;
> CAP-001..009 stay PROPOSED; no frozen signature changed; additive, `cargo test
> --workspace` 181→**193/0**. Record: `runtime/rcr/RCR-026.md`), **RCR-027** (I4
> Stage 2 — **CLUSTER SCHEDULING** inside `arves-control-plane` per
> `docs/design/I4_Capability_Scheduling_Design.md` §3.1.3/§3.1.5/§3.1.6/§3.5/§3.7:
> `scheduler::ClusterScheduler` — capability invocations scheduled across the I2/I3
> cluster with (a) **placement**: shard-leader AFFINITY for commit-bearing
> invocations + seeded deterministic compute-anywhere spread for `Pure` ones
> (IDR-001 "engines run anywhere, commit only via shard leader"; reference policy,
> explicitly NON-NORMATIVE pending the design's IDR-007 instrument — DR-2); (b) a
> **deterministic scheduler**: per-shard FIFO queues, per-shard bounded admission
> (backpressure = visible retriable `AdmissionDenied`, never a silent drop or global
> limiter — DR-3) and quarantine-based failure isolation (poison/policy denials
> terminal, never a wedged queue; deferral ≠ retry — DR-4/5); decisions are a pure
> function of (recorded state, seed, tick) — two identically-scripted runs produce
> BIT-IDENTICAL transcripts; (c) **idempotent dispatch**: the fabric-derived ORCH-004
> key (RCR-012) is the unit of identity — duplicate submission collapses (one
> execution), racing independent schedulers converge to EXACTLY one committed truth
> (at-least-once compute / at-most-once truth, design §6.1, honest), and a retry
> after leader/quorum loss replays FROM THE RECORDED INFERENCE, never re-invoking the
> engine (ORCH-003 — DR-6); the scheduling-surface dedupe identity is the fabric key
> QUALIFIED by capability id and PARTITIONED by shard (DR-13, adversarial revision —
> cross-shard/cross-capability collapse refuted by negative tests; retriable-class
> quarantine re-admits with a fresh budget, DR-4 revised); (d) **the full
> distributed chain**: scheduled invocation
> → Stage-1 gate (`invoke_gated`, RCR-026) → RCR-012 `invoke_enforced` → proposed
> effects → shard-leader `ClusterKernel::commit` (RCR-021) → quorum → byte-identical
> truth on every replica; ORCH-002 proven by kill-mid-run + rebuild-from-plan with an
> identical committed truth set; PropertyCheck ORCH-001/ORCH-002 rows flipped
> Pending→CitedTest (EXPLICIT recorded flip, scoped to the scheduling surface; the
> I5 Orchestrator stays contract-only — DR-11), coverage now 7 proven / 0 pending;
> FIVE new downward edges control-plane(90)→capability(70)/engine(60)/kernel(40)/
> consensus(30)/acs(15) (LAYER-001 gate green); the frozen `Orchestrator` contract is
> byte-unchanged; NOT built and NOT claimed: network/remote execution (in-process
> `ClusterSim`; placement is a recorded node label), plan-DAG ordering/arbitration
> (I5), sagas (OQ-4), HITL sequencing, distributed cancellation/timeouts (OQ-6),
> durable decision-trace emission (OQ-10), Failure-Policy degrade/escalate, cluster
> batch commit (v1.1 debt #3), authN/authZ; CAP-001..009 stay PROPOSED; additive,
> `cargo test --workspace` 193→**206/0** (203 at first application + 3
> adversarial-revision proofs). Record: `runtime/rcr/RCR-027.md`), **RCR-028** (I4
> Stage 3 — **ADVERSARIAL SCHEDULING PROOFS + I4 MILESTONE CLOSE** per the design's
> §4 proof table and §5 conformance plan; additive tests + conformance extension
> ONLY, no frozen signature touched: (a) **storm/duplicate/reorder schedules** —
> two RACING schedulers submit 4 unique invocations 11× in two orderings; every
> duplicate collapses visibly, compute is honestly at-least-once (8 recorded
> executions), and each unique key lands as FRESH truth EXACTLY once across both
> decision logs (ORCH-004 at the scheduling layer); (b) **node death
> mid-invocation** — the placed leader dies between placement and quorum: retriable
> verdict, re-placement onto the elected successor, retry replays FROM THE RECORD
> (engine count pinned, ORCH-003), then SCHEDULER death on top — a fresh scheduler
> re-derives the identical content-addressed key from the plan alone and every
> re-commit resolves `deduped` (zero duplicate commits); the rejoined dead node
> converges byte-identically; (c) **backpressure honesty** — the accounting
> equation: 12 submit calls ≡ 6 Admitted + 6 AdmissionDenied decisions logged 1:1
> with returned outcomes, denials leave NO ledger half-state (stateless ⇒
> retriable), refused work re-admits after drain and completes — a silent drop is
> impossible by accounting; (d) **failure isolation** — a 3-invocation POISON storm
> interleaved ahead of healthy work quarantines terminally (probe double-invoke
> only, zero retries, zero truth) while same-shard, other-tenant AND post-storm new
> work all complete bounded; (e) **leadership change mid-schedule** — old leader
> survives/steps down/rejoins; a racing re-run under the new leader resolves to the
> old-era truth; fresh-commit key set across both schedulers is exactly one per
> invocation; (f) the live **`capability-scheduling-distributed`** conformance
> artifact (design §5.2 scenario; §5.3 node probes — the FIRST live artifact for
> the ControlPlane/Capability/Execution pipeline nodes): axes 4/7/8/10/12, every
> required invariant + property (TenantWorkspaceIsolation,
> SafetyGatesBlockedUnsafePlans, PolicyGatesFired, ReplayReproducesTrace) derived
> Held from behaviour, `Verdict::Pass`, honest fingerprint ("no network transport";
> placement "non-normative pending IDR-007"); PropertyCheck rows WIDENED with
> scheduling-layer citations (coverage stays 7 proven / 0 pending — no flip);
> THREE new downward edges conformance(110)→control-plane(90)/capability(70)/
> engine(60) (LAYER-001 gate green); L3 claimed ONLY as scoped
> ("under distributed deployment, in-process simulation" — the RCR-022/025
> language); I5 inheritance recorded (Orchestrator plan-graph + its own
> ORCH-001/002 obligations · HITL sequencing · durable trace emission OQ-10 ·
> sagas OQ-4 · distributed cancellation OQ-6 · IDR-007 ratification · CAP-00n
> CCP sponsorship OQ-5 · bridge-gate unification); CAP-001..009 stay PROPOSED;
> additive, `cargo test --workspace` 206→**213/0** (212 at first application +
> 1 adversarial-revision proof: the policy-flip collapse pin, DR-6, plus the
> loud-not-silent defensive dispatch arm, DR-7). I4 MILESTONE: ✅ DONE —
> pending maintainer integration (RCR-026..028). Record: `runtime/rcr/RCR-028.md`
> (milestone summary)), **RCR-029** (I5 Stage 1 — **AGENT IDENTITY as
> content-addressed truth + the LCW SHARED-TRUTH surface** per
> `docs/design/I5_MultiAgent_Runtime_Design.md` §3.1.1/§3.1.2/§3.19/§3.10:
> (a) `arves-lcw` gains its FIRST behaviour behind the frozen contracts — the
> first `WorkingMemory`/`LiveWorkspace` implementations (single-owner rule
> ENFORCED via the one additive `#[non_exhaustive]` variant
> `LcwError::AlreadyOpen`) and the `WorldView` shared-truth surface: a
> read-only, VERSIONED, coherent view of one shard's committed truth built
> exclusively by deterministic WAL replay, whose digest shares the Kernel
> `truth_hash` basis (equality proven); coherence proven at every commit index
> across re-reads AND across all replicas of the I2 cluster; hydration rebuilds
> working memory FROM truth and divergence never flows back (no write surface
> by construction); (b) `arves-control-plane` gains the additive `agents`
> module — agent identity = versioned ARVES-23-subset definition (owner
> MANDATORY, Vol 2 Part 17), canonically encoded with the registration shard
> inside the hashed body (SHARD-001: shard-bound for life), `AgentId` = ACS-001
> content id under the existing COMMIT_CONTENT tag (no new domain tag — DR-7),
> registration = idempotent commit through the frozen Kernel gateway (OQ-2
> resolved truth-side for this stage, DR-6: the registry recovers with the
> truth base, proven addressable from every replica), and ATTRIBUTION: every
> agent-proposed effect carries its agent identity INSIDE the committed payload
> — round-trips out of the truth trail on every replica; the structural gate
> refuses unregistered identities against COMMITTED truth (the runtime-grade
> elevation of the G1 in-process-map caveat); HONEST + PINNED BY TEST: agents
> here are deterministic test actors NOT AI models, and identity is an
> addressable registration NOT cryptographic authN (v2.0 debt #8/OQ-1 — one
> caller lawfully wears two registered identities; kept loud); TWO new edges,
> both downward, ranks checked first (lcw(50)→persistence(20),
> control-plane(90)→lcw(50); lcw→query deliberately absent — upward); the
> frozen `Orchestrator` plan-graph contract REMAINS contract-only; delegation/
> coordination/lifecycle-beyond-registration/revocation/OQ-3 re-check NOT
> built; LCW-001 et al. stay PROPOSED; PropertyCheck stays 7 proven / 0 pending
> (no flip); additive, `cargo test --workspace` 213→**234/0** (233 at first
> application + 1 adversarial-revision proof: `is_registered` gained the
> decoded-shard == world-shard check closing the SHARD-001 smuggle hole,
> amendment A1, pinned by `smuggled_foreign_shard_definition_is_refused_shard001`;
> the fabricated-Who honesty claim scoped to the `propose_attributed` path,
> amendment A2; `hydrate_into` partial-write-on-error documented, erratum E3 —
> see the RCR's Amendments section). Record:
> `runtime/rcr/RCR-029.md`), **RCR-030** (I5 Stage 2 — **MULTI-AGENT
> ORCHESTRATION over ONE shared truth base** per
> `docs/design/I5_MultiAgent_Runtime_Design.md` §3.1.2/§3.1.3/§3.8/§3.19:
> the additive `arves-control-plane::multi_agent` module — (a) concurrent
> agent proposals THROUGH the I4 scheduler (`submit_attributed_effect`:
> structural registered-gate against committed truth, attribution envelope as
> the invocation input; agents never commit — ORCH-001; the schedule stays a
> discardable plan artifact whose rebuild converges by Kernel dedupe with zero
> fresh commits — ORCH-002/004); (b) shared-truth concurrency: duplicates and
> agreeing decisions converge to ONE truth (ORCH-004 across agents, both at
> the pre-check and when raced commits land); CONFLICTING decisions on one
> subject resolve deterministically FIRST-COMMITTED-WINS in shard log order
> (total per shard, IDR-001/IDR-005) — the loser receives the WINNER's
> identity and the conflict is committed compliance truth citing it (the
> enterprise-os G1 `proposeDecision` reference semantics at runtime level);
> the Kernel decides NOTHING (no kernel-side gate — the Control Plane
> reconciles post-commit at the at-head world, DR-2; the full OQ-3
> leader-side admission re-check stays a recorded IDR obligation, and the
> policy gate reads the DECLARED basis only — pre-policy-basis honest limit
> pinned by test); (c) cross-agent consistency reads per the I3 ladder with
> LABELED guarantees: Linearizable sees prior committed truth from a follower,
> a partitioned replica refuses Linearizable/BoundedStaleness honestly and
> serves Eventual stale-but-labeled, and a stale basis can NEVER mint a second
> derived truth; (d) decision/compliance truth flows: policy checks read
> COMMITTED policy truths, approvals are SEPARATE committed truths
> (proposer ≠ approver, self-approval refused), admitted decisions CITE their
> qualifying approvals (Why precedes the decision in the one history), and
> refusals are committed compliance events (duplicate refusals converge —
> ORCH-004 on the audit ledger); derivation is GOVERNED-ONLY (amendment A1:
> an unregistered-Who record smuggled through the raw gateway never derives,
> pinned; a REGISTERED identity worn by any caller still does — v1.x
> structural limit kept loud); ALL SIX permutations of three racing agents +
> interleaved scheduler storms proven: one derived truth per subject, every
> loser loud, replicas byte-identical, same schedule ⇒ byte-identical bytes;
> HONEST: agents are deterministic test actors NOT AI models; ONE new edge,
> downward, rank-checked first (control-plane(90)→persistence(20), promoted
> from the RCR-029 dev-dep); flow encodings are runtime-internal, NOT `uci.*`
> types (O-006 CCP not pre-empted); the frozen `Orchestrator` plan-graph
> contract REMAINS contract-only; delegation/arbitration/HITL sequencing/
> durable trace emission NOT built; additive, `cargo test --workspace`
> 234→**246/0** (245 at first application + 1 adversarial-revision proof:
> the governed-only derivation smuggle pin, amendment A1). Record:
> `runtime/rcr/RCR-030.md`), **RCR-031** (I5 Stage 3 — **ADVERSARIAL
> MULTI-AGENT PROOFS + the axis-9 live conformance scenario; I5 MILESTONE
> SUMMARY** per `docs/design/I5_MultiAgent_Runtime_Design.md` §4/§5:
> (a) agent storms — 3 agents × 4 proposals + injected duplicates under SEVEN
> seeded schedule permutations ⇒ the FINAL TRUTH SET and attribution-trail
> SET identical across ALL permutations on every replica (the
> order-independence proof; log-order difference honestly pinned as
> non-vacuity, same schedule ⇒ byte-identical state); (b) lawful-API misuse —
> a replay of another agent's proposal dedupes to the ORIGINAL truth, an
> address rebind re-attributing it is refused by RCR-005 (attribution
> unforgeable at the content-addressing layer), a re-wrap is a DIFFERENT
> truth under the re-wrapper's own Who, and duplicate floods across racing
> schedulers land exactly ONE fresh commit; (c) partition mid-work —
> minority-side proposals fail honestly (`NotReplicated`/`NotLeader`,
> nothing committed), the majority keeps working, heal converges
> byte-identically, no acked attributed truth lost, the refused proposal
> retriable exactly-once; (d) full-cluster crash-recover from per-node WALs
> reproduces identical truth AND an identical attribution trail / decision
> derivation / compliance ledger (ORCH-003 including attribution); (e) the
> live `multi-agent-coordination-distributed` scenario — axis 9 instantiated
> (axes 3/10/12 joined; 11/8 omitted honestly), `Verdict::Pass`, the §5.3
> multi-agent artifact fields (`policy_gates`/`arbitration_choices`)
> populated for the FIRST time, first LivingCognitiveWorld node evidence
> (`SharedWorldLcwProbe`); PropertyCheck citations widened over the I5
> surface (coverage stays 7/7; the frozen `Orchestrator` plan-graph contract
> explicitly NOT pre-certified — it REMAINS contract-only); HONEST: agents
> are deterministic test actors NOT AI models, structural identity (v2.0
> debt #8), in-process `ClusterSim` (no network); ONE new edge, downward,
> rank-checked first (conformance(110)→lcw(50)); additive, `cargo test
> --workspace` 246→**251/0**. **I5 MILESTONE: ✅ DONE — pending maintainer
> integration (RCR-029..031).** Record: `runtime/rcr/RCR-031.md` (milestone
> summary)), **RCR-032** (REAL TRANSPORT — the single biggest reduction of the
> "in-process, no network" caveat for the Runtime: through RCR-019..031 the
> whole I2–I5 cluster ran on ONE vehicle, the in-process deterministic
> MessageBus (`sim.rs`). RCR-032 adds a **`Transport` delivery seam** in
> `arves-consensus` with TWO impls — `InProcessTransport` (the existing FIFO bus
> as a trait impl; `sim.rs` byte-unchanged) and **`LoopbackTransport`, a REAL
> `std::net` TCP transport on `127.0.0.1`** that length-frames the SAME
> serialized `Envelope`s onto real OS sockets (partial-read + one-reconnect
> handling) — and proves, via one identical driver (`TransportRound`), that a
> small cluster round (leader election + one commit) commits **byte-identical
> truth over real TCP sockets and over the in-memory bus**. DETERMINISM
> PRESERVED (HARD RULE 4): the core stays a pure function of (messages, seed,
> tick); the harness canonicalizes each drain's order so the socket only moves
> bytes — *the transport moves bytes, the protocol decides truth, the harness
> fixes the order*. HONEST SCOPE (unchanged, NOT claimed): loopback = one
> process (NOT multi-host); no TLS/mutual-auth (OQ-7, mTLS v2.0 debt); no real
> latency / message loss / partition TIMING (the socket delivers every sent
> frame; adversarial delivery stays the `sim.rs` filter/mangle model); wire
> format decision stays OQ-3 (this LE codec is internal framing). **No new
> dependency** (`std::net` only — `arves-consensus` `[dependencies]` empty, rank
> 30, LAYER-001 gate green); no frozen type/trait touched; retroactive
> scope-annotation appended to RCR-019/021/022/024/027/031 ("in-process OR
> real-loopback transport (RCR-032)"); additive, `cargo test --workspace`
> 251→**255/0** (loopback socket run re-run 5× — deterministic, no flake).
> Record: `runtime/rcr/RCR-032.md`), **RCR-033** (BRIDGE `scan` VERB — the
> read-only WAL-enumeration seam that closes JARVIS's recorded "the bridge has
> NO verb to scan/enumerate committed truth" caveat: `scan` / `scan bodies`
> (composable with `id=`/`shard=`) replays the target shard's WAL through the
> Query layer's `ShardProjection` and streams its committed set —
> content-ids, optionally with payloads — in deterministic commit order.
> OWN-001 read tier: reads are NOT on the `Kernel` trait, so this exposes the
> Query layer (RCR-010/023), never a Kernel read hook; the store is read behind
> `&` only (no write handle reachable). Tenant isolation is structural
> (SHARD-001 — a foreign record never enters the fold); a never-committed shard
> answers `scan 0`. New enumerator `ShardProjection::committed()` (read-only,
> `&[u8]` refs); new downward edges bridge(105)→query(60) and bridge(105)→
> control-plane(90); no frozen type/trait touched. Record: `runtime/rcr/
> RCR-033.md`), **RCR-034** (BRIDGE `commit-as` VERB — exposes the EXISTING I5
> attribution (RCR-029 `encode_attributed`) over the seam: `commit-as
> <agent_hex> <domain_hex> <body_hex>` wraps the body in the agent-attribution
> envelope so the Who rides INSIDE committed truth (WAL/IDR-005) and is
> recoverable by `decode_attributed` over a `scan`. An attributed commit is a
> DISTINCT truth from a plain commit of the same body (idempotent per ORCH-004);
> plain commits are byte-unchanged. HONEST: a CLAIMED Who — no registration
> gate, no caller-identity check (v2.0 debt #8); the attributed-INVOKE path is
> the recorded next candidate. New public `AgentId::from_hex`; no frozen
> type/trait touched. RCR-033+RCR-034 additive, `cargo test --workspace`
> 255→**263/0** (verified). Downstream products (freeze-clean, IDR-006):
> SDK `scan()`/`commitAs()`/`decode()`, assistant `recoverFromWal()` (total
> read-only reconstruction, ZERO re-commits), `jarvis-day` 17→18 properties.
> Record: `runtime/rcr/RCR-034.md`), **RCR-035** (BRIDGE `scan` HONESTY FIX,
> amends RCR-033 — an adversarial review found `scan_shard` swallowed EVERY
> read/replay fault as an empty result, so a corrupt/compacted retained log was
> indistinguishable from a never-committed shard: both answered `scan 0`, masking
> truth-loss as emptiness. `scan_shard` now returns `Result<…, ScanFault>`: a
> shard the store has NO log for is a legitimate empty `Ok`, but a shard whose log
> EXISTS yet cannot be replayed (open/replay fault, or a compacted prefix with no
> query-side snapshot — RCR-023 DR-7) is `Err(ScanFault)`, surfaced on the wire as
> `ERR scan-fault`, NEVER `scan 0`. Still a read (the probe is `WalStore::shards()`;
> no write path, no new dependency edge); determinism unchanged. The SDK `scan()`
> already throws on any `ERR`. Additive+corrective, `cargo test --workspace`
> 263→**265/0** (verified). Downstream (freeze-clean, IDR-006): the assistant
> effect-subject test + `jarvis-day` capstone now PROVE the disclosed
> reconstruction residual (the effect→skill edge is absent after a read-only WAL
> recovery) by a RUNNING assertion instead of prose; `jarvis-day` 18→19 properties.
> Record: `runtime/rcr/RCR-035.md`), **RCR-036** (AUTHENTICATED COMMIT — closes
> the commit-path half of v2.0 security debt #8, the largest v1.0 trust caveat:
> `Kernel::commit` had no principal/authN. `arves-kernel` gains a dependency-free
> **HMAC-SHA256** (RFC 2104 over the runtime's own `arves-acs::sha256`, verified
> against RFC 4231 vectors; ZERO new third-party crates) authenticated-commit
> path: `Principal`, `RefKernel::register_principal` (shared-key provisioning —
> key distribution OUT OF SCOPE), `commit_authenticated(proposed, principal, mac)`
> which VERIFIES the MAC (constant-time) at the gateway BEFORE admitting through
> the IDENTICAL `commit_inner` (ORCH-004/RCR-005 unchanged), and an
> `authenticated_digest()` **anchor** hash-chaining each `(principal, mac,
> content, offset)` — the principal-attributable companion to RCR-002's WAL
> `integrity_digest` (both chains proven to move together). A forged/tampered MAC
> is REJECTED with the new `CommitError::AuthenticationFailed`; the unauthenticated
> `commit` path is byte-unchanged (v1.0 trusted-host mode). HONEST SCOPE: HMAC is
> SYMMETRIC/shared-key — repudiable, NOT public-key non-repudiation (ed25519-class
> = a separate v2.0 RCR, still open); no authZ/key-management. ONE new downward
> edge `arves-kernel(40)→arves-acs(15)` (LAYER-001 legal). Additive, `cargo test
> --workspace` 265→**279/0** (verified). Record: `runtime/rcr/RCR-036.md`),
> **RCR-037** (ENGINE live-conformance node — closes the honest gap RCR-031
> recorded: `arves-conformance::live` had a live `NodeProbe` for every pipeline
> node EXCEPT `Engine`, so no live artifact composed an Engine claim. New
> `EngineProbe` drives the reference `PureEngine` through `invoke_enforced`
> (RCR-012) and derives the engine invariants from behaviour — ORCH-004 engine
> PURITY (the FABRIC derives the content-addressed key; same input+manifest → same
> key + inference, different input → different key), ORCH-003 DETERMINISM
> ENFORCEMENT (a false `Deterministic` declaration is REFUSED before any effect
> escapes; the honest engine replays bit-identically), ORCH-001/ENG-003
> PROPOSALS-NOT-COMMITS (invoking the engine commits nothing on a real `MemKernel`;
> only `Kernel::commit` promotes the proposed effect to truth). New end-to-end
> `l2-information-kernel-query-engine` scenario composes the Engine node
> (Information → Kernel → Query → **Engine**) — every data-plane pipeline node now
> has a live probe. `conformance_live` emits `LIVE-ENGINE: PASS` beside
> `LIVE-L1: PASS`. HONEST SCOPE: reference PureEngine, in-process; the determinism
> check is a PROBE not a proof; ENG-001..005 stay PROPOSED and are NOT asserted
> (every check derives from a REGISTERED invariant). Additive, NO new architectural
> edge (`arves-conformance(110)→arves-engine-fabric(60)` already present), no frozen
> type/trait touched; +2 conformance tests (`live_engine_scenario_passes`,
> `live_l2_engine_pipeline_scenario_passes`), workspace **279/0** alongside the
> concurrent RCR-036 lane. Record: `runtime/rcr/RCR-037.md`).

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
   chain a signature scheme will sign.
   **FURTHER ADDRESSED by RCR-036 (v1.1):** the **authenticated-commit** half now exists — a
   dependency-free **HMAC-SHA256** path (`RefKernel::commit_authenticated`, principal-keyed,
   built on the runtime's own `arves-acs::sha256`, RFC 4231-verified) VERIFIES a MAC at the sole
   commit gateway before admitting truth and hash-chains each MAC into a principal-attributable
   `authenticated_digest()` anchor that composes with RCR-002's WAL digest. A forged/tampered
   commit is REJECTED (`CommitError::AuthenticationFailed`), not merely evidenced. HONEST BOUND:
   HMAC is **symmetric/shared-key** (repudiable; key distribution out of scope) — this closes the
   "unauthenticated commit" hole for the **trusted-key model**, NOT zero-trust.
   STILL OPEN (v2.0): public-key **non-repudiation** (ed25519-class **signatures** — needs a
   scoped crypto dependency), key management (distribution/rotation/revocation) + authZ, and
   durable anchor **verification on recovery** — a fully hostile host that rewrites the whole
   trace *and* forges without the shared key still needs public-key signatures to stop. Threat
   model unchanged for v1.0 (trusted single host); see `runtime/rcr/RCR-002.md`,
   `runtime/rcr/RCR-036.md`.

Each enters via an RCR into v1.1 (or v2.0 for #8's breaking parts), with regression + property tests.

---

*Freeze marker: git tag `runtime-v1.0`. Products (P4→P8) now build on this frozen base;
any runtime gap is an RCR, not a product edit.*

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
> `cargo test --workspace` 108→**110/0**, product regression stays **55/55** on the rebuilt exe).

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

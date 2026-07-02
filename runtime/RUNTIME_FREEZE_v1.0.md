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
2. **Engine-enforced determinism** — the fabric derives/enforces the idempotency key
   rather than trusting an engine's self-declared `Determinism` (today: the reference
   `PureEngine` is pure by construction).
3. **Kernel batch-commit** — atomic multi-effect / multi-shard commit (today: single-effect
   invocations are all-or-nothing; multi-effect effects are independent idempotent truths).

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

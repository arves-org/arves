# RCR-004 (DRAFT DESIGN) — runtime v1.1: native semantic validators + Kernel content-integrity + PropertyCheck

**Status:** ENGINEERING DESIGN. Per CLAUDE.md ("ENGINEERING DESIGN — no code at this stage" →
"CRITICAL SELF-REVIEW" → "IMPLEMENTATION"), this is the mandated design phase. The actual
`runtime/` edits land as ratified **RCR-004 / RCR-005 / RCR-006** records under `runtime/rcr/`,
each with its own destroy→repair→prove cycle and a `freeze_check.py update`.

> **✅ RCR-004 APPLIED (2026-07-04)** — `runtime/rcr/RCR-004.md`. The native Rust ACS-003/004/005
> semantic validators shipped in `arves-conformance::semantic` and reject all **19/19** frozen
> vectors (envelope 7/7 + instance 8/8 + language 4/4) with the exact registered codes; workspace
> **75→77/0**, freeze re-baselined to 153/0. *Harness exposure* (an `acs_validate` bin +
> certify/runner display) is the tracked **RCR-004b** follow-up. RCR-005 (#3) and RCR-006 (#18)
> remain designed-not-applied below.

**Scope:** three tracked Bucket-B / v1.1 items that touch the FROZEN runtime:
- **RCR-004** — native Rust ACS-003/004/005 semantic validators (retire the CCP-006 deferral).
- **RCR-005 (#3)** — Kernel commit-gateway content-integrity (stop trusting a caller ContentId).
- **RCR-006 (#18)** — PropertyCheck/Suite: invariant-id → executable-proof catalog for the Kernel.

Independence stays **G1**; none of this manufactures G2. All three are additive/backward-compatible
(minor **v1.1**), not breaking (v2.0).

---

## RCR-004 — native Rust ACS-003/004/005 semantic validators

**Motivation.** CCP-006 shipped 19 semantic negative vectors; the frozen Rust v1.0 reference has
no ACS-003/004/005 validators, so it **declares the tiers deferred** (like `nfc`). The living
Python validators exercise them (`conformance_semantic.py`). RCR-004 makes the *reference runtime*
reject them natively, so a G2 party can diff against a Rust reference for the full ACS surface, not
just ACS-001/002.

- **Responsibilities.** Given a decoded ACS-002 `Value`, validate ACS-003 envelope semantics,
  ACS-004 instance semantics (against a schema `Value`), and ACS-005 term-set semantics; reject
  with the **registered CCP-006 reason code** (not prose) or accept.
- **Inputs.** A decoded `arves_acs::cbor::Value`; for instances, the decoded schema document
  (an ACS-004 golden vector). Raw bytes for ACS-005 term-sets.
- **Outputs.** `Result<(), RejectCode>` where `RejectCode` is one of the 11 registered kebab codes.
- **Dependencies.** `arves-acs` (Value model + decoder). New module `arves-acs::semantic` (or a new
  `arves-acs-semantic` crate) + wiring in `arves-conformance`. No new external deps (std-only).
- **Lifecycle / State.** Pure, stateless functions — a pure function of the decoded value(s).
- **Determinism / Replay.** Deterministic; no clock, no randomness; platform-independent (§6.5).
- **Failure modes.** (a) Rust reject reason diverges from the frozen vector's code → caught by a
  new `arves-conformance` test asserting each of the 19 vectors rejects with the exact code; (b)
  Rust ↔ Python semantic divergence → **extend the differential fuzzer** to the semantic tiers
  (generate valid-then-one-mutation envelopes/instances, assert both impls agree accept/reject).
- **Consistency.** The Rust codes MUST equal the frozen TSV `reject_reason`; the Python reference
  should be upgraded to emit the same codes (today it emits prose — a small living follow-up) so the
  differential check compares codes, not just accept/reject.
- **Observability.** The conformance runner reports `envelope 7/7 instance 8/8 language 4/4` for the
  Rust arm instead of `DEFERRED`; a new bin `acs_validate` (line protocol, like `acs_decode`) lets
  `certify_runtime.py` / `verify_runtime_sound.py` drive the Rust semantic tiers inputs-only.
- **Trade-offs.** +~500–700 LOC in the frozen runtime; must not break the 75 workspace tests. Buys:
  the reference runtime covers the full ACS certification surface; the CCP-006 deferral retires.
- **Risks.** Byte-exact code parity across all 19 vectors; the ACS-004 schema-binding (needs the
  golden schema document available to the runner); keeping `_MapValue`-equivalent key handling
  identical to Python.
- **Open questions.** (1) New crate vs module in `arves-acs`? (prefer a module to avoid a new crate
  in the graph). (2) Does `verify_runtime_sound.py` gain a semantic-tier grader (grader owns the
  truth)? (3) Should the reference runtime's conformance *verdict* now REQUIRE the semantic tiers,
  or keep them as a declared-optional band? (recommend: keep optional for a pure interop codec;
  REQUIRED for the full runtime — mirrors CONFORMANCE.md).

---

## RCR-005 (#3) — Kernel commit-gateway content-integrity

> **✅ RCR-005 APPLIED (2026-07-04)** — `runtime/rcr/RCR-005.md`. Implemented the **Kernel-owned,
> non-coupling** half: the gateway now rejects a re-proposal that binds the same `ContentHash` to a
> *different* payload (`CommitError::ContentIntegrity`), closing the "same address, different
> content" fork; workspace **77→78/0**. Per **NON-NEGOTIABLE RULE #9**, full ACS-001 multihash
> re-derivation at the Kernel (which needs a `domain` on `ProposedWrite` + a Kernel→`arves-acs`
> coupling) was **NOT** taken — address integrity stays at the bridge (its layered-correct owner);
> that coupling is a recorded maintainer decision, not this RCR.

**Motivation.** The Kernel's sole commit gateway trusts a **caller-supplied ContentId**; address
integrity is enforced only in the optional bridge. A buggy/malicious caller can commit a payload
under the wrong address, breaking the ACS-001 "equal address ⇒ same content" guarantee at the one
place it matters most.

- **Responsibilities.** At the commit gateway, **recompute** `ContentId = 0x12 0x20 ‖
  SHA-256(domain_tag ‖ payload)` and reject if it ≠ the claimed id.
- **Inputs.** Proposed write: `(domain_tag, payload_bytes, claimed_content_id)`.
- **Outputs.** Commit iff recomputed == claimed; else reject `content-id-mismatch` (new reason).
- **Invariants.** ORCH-004 (idempotent + content-addressable commit), ACS-001 §5 addressing,
  OWN-001 (Kernel owns truth). Strengthens ORCH-004's "equal Content Address ⇒ existing outcome".
- **Dependencies.** `arves-kernel` commit path + `arves-acs::content_id`. No new deps.
- **Failure modes / Recovery.** A wrong claimed id is rejected at the gateway (was silently
  accepted). Replay unaffected (recompute is deterministic). No WAL format change (additive check
  before append).
- **Trade-offs.** One SHA-256 per commit (already computed on the write path in practice) — cheap
  vs the correctness it buys. Backward-compatible: honest callers are unaffected.
- **Risks.** Must not break existing Kernel tests; add a biting negative test
  (`content ≠ hash(payload) → rejected`). Possible new negative vector at the Kernel layer (not an
  ACS wire vector — a runtime-behaviour vector).
- **Open questions.** Reject-reason registry: is `content-id-mismatch` an ACS reason (CCP) or a
  Kernel runtime reason (RCR-local)? (recommend: Kernel-local, documented in the Kernel crate, since
  it is a runtime-behaviour reject, not an ACS wire-format reject).

---

## RCR-006 (#18) — PropertyCheck/Suite: invariant → executable-proof catalog

**Motivation.** LAYER-001/OWN-001 are executably gated and ORCH-003/004 have biting Kernel tests,
but there is no **catalog** binding each invariant id to its executable proof (the registry lists
invariants; nothing maps `ORCH-003` → the test that proves it). The proof obligation ("no invariant
remains proof-only once its component is implemented") is met ad-hoc.

- **Responsibilities.** A `PropertyCheck` catalog: `InvariantId → fn() -> ProofResult`, run as a
  suite, so every implemented invariant has an addressable, executable proof.
- **Inputs.** The registered invariant set (OWN/LAYER/SHARD-001, ORCH-001..004).
- **Outputs.** Per-invariant PASS/FAIL + a coverage report (which invariants are proven vs pending).
- **Dependencies.** `arves-invariants` + the existing architecture gate + Kernel tests (wrap, don't
  duplicate). No new deps.
- **Trade-offs.** Formalizes existing proofs into a catalog (low risk); makes "invariant coverage
  100%" checkable rather than asserted.
- **Risks.** Scope creep — keep to the implemented invariants (ORCH-003/004 + structural
  LAYER/OWN/SHARD); do NOT attempt proofs for un-implemented milestones (I2–I6).
- **Open questions.** Is this a test-only harness or a runnable bin surfaced to certification?
  (recommend: a workspace test + a small report bin, wired into the evidence probe).

---

## Critical self-review (destroy the design)

- **Drift risk (RCR-004).** If the Rust validator's codes are hand-mapped, they can silently diverge
  from the TSV. *Mitigation:* a table-driven test that asserts each of the 19 frozen vectors rejects
  with its exact TSV `reject_reason`, plus a semantic differential-fuzz arm — the same discipline
  that caught the 2+1 ACS-002 reference bugs.
- **Freeze-scope creep.** RCR-004 is tempting to expand into "rewrite the conformance crate."
  *Mitigation:* additive module only; the ACS-002 generator + the 75 tests stay byte-identical;
  freeze `check` must show ONLY the intended runtime files.
- **#3 double-hashing.** Recomputing at the gateway may duplicate a hash the bridge already did.
  *Mitigation:* the gateway is the authority; the bridge check becomes redundant-but-harmless (or is
  removed). The gateway MUST be the enforcement point (OWN-001: the Kernel owns truth integrity).
- **#18 over-claim.** A catalog that maps invariants to *weak* tests would launder coverage.
  *Mitigation:* each entry cites the specific biting assertion; ORCH-003 = replay-equality test,
  ORCH-004 = idempotent-commit test — no placeholder proofs.
- **Independence honesty.** None of this is G2. A Rust reference that rejects the semantic vectors is
  still same-program evidence. *Keep the ledger capped at G1.*

## Sequencing recommendation

RCR-004 first (retires the most visible CCP-006 deferral, extends the differential arm), then
RCR-005 (#3, correctness-critical, small + biting), then RCR-006 (#18, formalization). Each as its
own ratified `runtime/rcr/RCR-00x.md` + `freeze_check.py update` + destroy→repair→prove. This
DESIGN draft gates none of them; it is the required design phase, recorded before any runtime byte
moves.

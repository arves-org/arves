# ARVES Open-Debt & Ambiguity Register

> **One place** that lists every still-open item across both tracking schemes
> (`SYSTEM_GAP_ANALYSIS.md #1-39` ↔ `G2_READINESS.md B1-B4`), with its **instrument**, **owner-arm**,
> and **status**. It doubles as the **CHALLENGE.md ambiguity artifact** a G2 party is asked to return:
> anything here that a stranger would hit is pre-declared. **Grade: G1 (self).** Nothing here changes a
> frozen byte; each item routes through its sanctioned instrument (CCP / RCR / freeze-clean / external).
>
> **Arms:** STD standard · RT runtime · VER verification · PRD product · GOV governance/docs.
> **Instruments:** CCP (frozen `standard/`) · RCR (frozen `runtime/`) · FC freeze-clean living_fix ·
> DOC doc-only · EXT external (needs a real outside party). Last updated 2026-07-05.

## A. The one true external gate (Bucket C — nothing internal discharges it)

| Item | Instrument | Status |
|------|-----------|--------|
| **G2** — a genuinely unrelated party certifies a runtime from `standard/` alone, no help, earning `SOUND-CERTIFIED (full ACS-001..005 surface)` | EXT | **OPEN — THE exit gate.** Independence honestly capped at **G1** everywhere until it fires. |
| Third-party / arms-length architecture review (L2–L4) | EXT | open |
| Real organizations in production on ARVES without modifying the standard | EXT | open (north-star) |

## B. Standard (frozen `standard/` → CCP)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| **B2** | Root-event `causation_id`: present-with-Null vs absent are two lawful encodings → two ContentIds on the most common envelope (ORCH-004 dedup trap a G2 team would hit) | CCP | 🟡 **DRAFT STAGED (CCP-008).** Gap demonstrated (`gen_ccp008_vector.py`: current validator ACCEPTS the absent form; the two encodings' ContentIds `fc0e…` vs `b1b7…` DIVERGE); byte-clean fix (§5 MUST present-Null) + candidate vector `envelope-root-omits-causation_id → missing-required-field`. Ratification is maintainer-gated CCP-GATE. |
| #20 | ACS-004 §5.1 urn↔type binding is normative but §6.5 doesn't enforce it; *"modulo namespace"* is underspecified for `uci.fact` (schema) vs `urn:arves:uci.core:fact@1.0` (instance) | CCP | OPEN (design decision — the exact binding rule must be fixed first; then a §6.5 clause + `instance-urn-type-mismatch` vector) |
| #21 | ACS-005 §9.1 requires a `GL-nnn` entry for "Data Plane"; glossary closes at GL-014 (defined inline only). Real fix `GL-015` changes the §9.2 golden term-set vector | CCP (profile bump) | OPEN (byte-affecting → ACS-005/2, or amend §9.1 to resolve via the §7 inline def — a normative choice) |
| #1/#2/#23 | ACS-003/004/005 negative-vector corpus | CCP | ✅ **CLOSED** (CCP-006/007: 19 semantic vectors + 11 codes; Kit 0.3.1) |
| #19/#22/#24 | int-range coverage · shortest-len clause · §9.3 glossary lint | CCP / done | ✅ **CLOSED** (CCP-007 / confirmed) |

## C. Runtime (frozen `runtime/` → RCR)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| RCR-004b | Expose the native Rust semantic validators to the harness (`acs_validate` bin) | RCR | ✅ **CLOSED** (RCR-004b, commit 154a698) |
| #3 (full) | Kernel commit-gateway recomputes the ACS-001 multihash from the payload pre-image | RCR (v1.1) | **OPEN — RULE #9 decision.** Needs a `domain` on `ProposedWrite` + a Kernel→`arves-acs` coupling. Kernel-owned half (content-hash ⇒ payload binding) is CLOSED (RCR-005); ACS-001 recompute stays at the bridge unless the maintainer rules to couple. |
| SHARD-001-F2 | Runtime `ShardKey` fields are `pub` (mutable-by-type); the opaque `arves-invariants::ShardKey` is unused | RCR | OPEN (low exploitability) |
| #18 | PropertyCheck/Suite invariant→proof catalog | RCR | ✅ **CLOSED** (RCR-006; ORCH-001/002 honestly Pending until the Control Plane is implemented) |
| SHARD-001-F1 | Kernel cross-tenant isolation test | RCR | ✅ **CLOSED** (RCR-007, `behaviour_8_two_tenant_isolation`) |
| v1.1 backlog | bridge request-id correlation (vs positional FIFO) · engine-enforced (not self-declared) determinism · Kernel batch-commit (multi-effect atomicity) | RCR | OPEN (RUNTIME_FREEZE v1.1 backlog, non-blocking) |
| v2.0 debt | truth-store cryptographic tamper-evidence / authenticated commit (zero-trust) | RCR (v2.0) | OPEN |
| I2–I6 | distributed runtime (per-shard Raft, IDR-001..005); the 11 CONTRACT-ONLY crates | RCR | **GATED behind Standard Validation** — mandate is "prove wrong," not "build." |

## D. Verification (freeze-clean `verification/`)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| B1 | Flagship SOUND gate attested only ACS-002 core (0/19 semantic) | FC | ✅ **CLOSED** (rank 1, commit 3c08ff5: full-surface, coverage-labeled, non-gameable) |
| B3/B4 | Harness gameable / crashes on Kit-only checkout | FC | ✅ **CLOSED** (sound verifier + degrade guards; `test_harness_integrity.py`) |
| Fz1 | Differential fuzz "0 hard divergences" is silent on 16 interop-safe reason-code disagreements | FC | OPEN (low — interop-safe by design; a ledger-metric qualifier would surface it) |
| Semantic differential (r12) | The ACS-003/004/005 reject surface was *self*-conformance (Python-only, `conformance_semantic.py`) | FC | ✅ **CLOSED** (rank 12: `acs_semantic_differential.py` — the Rust native validators via `acs_validate` and the Python reference AGREE **62/62**, 0 hard divergences, over a deterministic single-mutation corpus; the semantic reject surface is now **DIFFERENTIAL** above the byte layer; drift-proof probe 11/11) |
| 3-way fuzz | Add the conformant TypeScript codec as a 3rd differential-fuzz arm (currently Rust↔Python; ACS-002 layer) | FC | OPEN (rank 19) |
| Formal | TLA+ kernel spec not mechanically model-checked (L0, no captured TLC run) | FC | OPEN (rank 5 — cheapest Evidence-Level raise) |
| Live conformance | The 12 Scenario axes are typed but zero-instantiated (L0); no live-runtime `ConformanceArtifact` | RCR (not FC — the contract types are `#[non_exhaustive]`, so the live impl must live in `arves-conformance`) | ✅ **L1 CORE PIPELINE CLOSED — end-to-end live** (RCR-008 Kernel · RCR-009 Information/Connector · RCR-010 Query/WAL-replay). The single L0 behaviour row is now an executing 3-node `ConformanceArtifact` (Information→Kernel→Query, `Verdict::Pass`, 7 invariant-checks from behaviour; drift-proof probe 10/10). The **deeper L1–L4 axes** (Control Plane / Engine / Capability / multi-agent / distributed) stay typed-only, gated behind Standard Validation (I2–I6); promoting the reference Connector/Query into the `arves-information-platform`/`arves-query` crates is rank 19. |

## E. Product (freeze-clean `products/`)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| E1 | enterprise-os "requires legal approval" is proposer-self-attested (approvals is a caller array) | FC | ✅ **CLOSED** (rank 14: approval is now a SEPARATE committed `uci.approval` truth via `approve({role,subject})`; `checkPolicy` reads `#approvals` not the proposer's array — the finance agent can no longer self-clear its own gate. Biting regression: a decision self-declaring `approvals:['legal']` is now BLOCKED (49/49). **RESIDUAL → runtime #8:** Runtime v1.0 has no authN on `commit`, so the `role` tag isn't cryptographically bound — a fully authenticated approval identity is the v2.0 authenticated-commit RCR.) |
| E2 | a bare-`Number` amount crashes the ACS commit (deep opaque encoder error); spend policy applies only on exact `spend:` subject prefix | FC | ✅ **CLOSED (amount hardening)** (rank 14: `#toIntUsd` coerces an integer Number/string USD amount to canonical BigInt and rejects any non-integer EARLY with a field-named product error — no more deep dCBOR crash; regressions for both arms). The exact-`spend:`-prefix subject scope is a separate **policy-language** decision (CCP-class, tracked with #20), not an input-crash. |
| M1 | marketplace signature binds artifact bytes but NOT the advertised catalog/install identity (`cap.manifest` ≠ `artifact.manifest`) | FC | ✅ **CLOSED** (rank 11: `manifestBinds` deep-equal guard in publish + host.install + registry.install; biting regression `robustness.test.mjs` — a valid artifact for B under A's name is refused; 46/46) |
| #11 | capability determinism gate is a best-effort author-input probe, not enforcement | FC | OPEN (reworded honestly; full enforcement = v1.1 RCR debt) |
| P1 | personal-os "durable decision history" overstated the in-memory detection index | DOC | ✅ **CLOSED** (wording + ledger caveat) |
| #4 | products commit to the real Kernel (not just an in-memory Map) | FC/DOC | ✅ **CLOSED** (routed through bridge; claims scoped) |

## F. Governance / docs

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| V1 | `ARVES_00_Invariant_Registry_v1` (frozen `.docx` mirror) still reads "no runtime code exists yet" / all proofs "pending" — contradicts RCR-006's 5/7-proven catalog | CCP / regenerate | OPEN (acknowledged in CLAUDE.md; `.docx` regen, not a silent edit) |
| V2 | `ARVES_IDR_Batch_1_Kernel_Distribution_v1` cites G-001 / QUERY-001 inline without a "proposed/pending" qualifier | CCP / regenerate | OPEN |
| V3 | Milestone identifiers diverge: `ARVES_Reference_Lifecycle_v1` uses M10/M11/M12 vs I1..I6 elsewhere; no reconciliation table | CCP / regenerate | OPEN |
| MR-drift | `runtime/docs/ARVES_Master_Roadmap.md` still says `arves-standard-kit 0.2.0` (current 0.3.1); it is under `runtime/` (frozen) | RCR / regenerate | OPEN (the living onboarding docs were de-drifted in rank 3; this frozen mirror lagged) |
| #36/#37 | Documentation-Index "current" marker · a dedicated "certify YOUR runtime" quickstart snippet | DOC / FC | ✅ **CLOSED** (rank 15). **#37:** `verification/certification/certify_your_runtime.py` — a copy-paste driver that grades YOUR runtime through the non-gameable `grade_sound` (zero Python if it speaks three line protocols); `--self-test` grades the reference bins via the vendor path (SOUND-CERTIFIED, proving the driver is real) + the single page `CERTIFY_YOUR_RUNTIME.md`; wired from README/QUICKSTART/CHALLENGE/IMPLEMENTING; non-gameability regression-locked (`test_harness_integrity.py` B3-driver: hollow-via-driver REJECTED, real-via-driver CERTIFIED). **#36:** `docs/SPEC_STARTER.md` now marks `Documentation_Index_v2.2` as the current master register and v1/v2/v2.1 as superseded. |
| #16/#10 | freeze-diff gate covers `runtime/`+`standard/` but NOT the frozen spec mirror (`spec-markdown/`) — a silent edit to the Registry mirror passes CI | FC | ✅ **CLOSED** (rank 10: `FROZEN_ROOTS` now includes `spec-markdown/` + `corpus/`; gate covers **261** files, selftest bites a `.docx` tamper). The V1-V3 *content* fixes are still CCP/regen — but a silent edit to them is now caught. |
| #17 CI | `.github/workflows/ci.yml` is a gate DEFINITION only — no CI host provisioned; freeze/drift gates rest on author discipline | FC + EXT | OPEN (rank 17) |

## G. External funnel (Bucket C — makes the G2 event possible without us)

| Item | Instrument | Status |
|------|-----------|--------|
| Complete `RELEASING.md` (CODE_OF_CONDUCT / SECURITY / public org) + PUBLISH | EXT | **Technical pre-conditions now MET** — ranks 1 (sound gate full-surface), 2, 3, and **15** (certify-YOUR-runtime path) are all CLOSED, so the "never publish an under-attesting gate" bar is cleared. PUBLISH itself stays **maintainer-gated**: it needs the maintainer's explicit go + a chosen public org/namespace, and is irreversible (external distribution). Not something the engineering arm does unilaterally. |
| Distribute CHALLENGE + recognition path + leading-indicator funnel (fetches→attempts→self-check→submissions→G2) | EXT | OPEN (rank 18) |

---

*Every "CLOSED" here was verified at 0 freeze-drift with its instrument's record. Every "OPEN" names the
one instrument that discharges it. The single item that lifts independence past **G1** is **G2** (§A) —
external by construction. See `STRATEGIC_PROGRAM_2026-07.md` for the execution order.*

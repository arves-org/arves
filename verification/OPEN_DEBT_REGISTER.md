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
| **B2** | Root-event `causation_id`: present-with-Null vs absent are two lawful encodings → two ContentIds on the most common envelope (ORCH-004 dedup trap a G2 team would hit) | CCP | **OPEN — highest-leverage standard item.** Fix is byte-clean (present-Null is already golden). Rank 9 = CCP-008 DRAFT. |
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
| 3-way fuzz | Add the conformant TypeScript codec as a 3rd differential-fuzz arm (currently Rust↔Python) | FC | OPEN (rank 19) |
| Formal | TLA+ kernel spec not mechanically model-checked (L0, no captured TLC run) | FC | OPEN (rank 5 — cheapest Evidence-Level raise) |
| Live conformance | The 12 Scenario axes are typed but zero-instantiated (L0); no live-runtime `ConformanceArtifact` | FC | OPEN (ranks 6–8, 13 — the biggest untapped Evidence surface) |

## E. Product (freeze-clean `products/`)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| E1 | enterprise-os "requires legal approval" is proposer-self-attested (approvals is a caller array) | FC | OPEN (claim scoped honestly; full fix = authenticated approval truths, rank 14) |
| E2 | spend policy applies only on exact `spend:` subject prefix; a bare-`Number` amount crashes the ACS commit | FC | OPEN (rank 14) |
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
| #36/#37 | Documentation-Index "current" marker · a dedicated "certify YOUR runtime" quickstart snippet | DOC / FC | OPEN (rank 15) |
| #16/#10 | freeze-diff gate covers `runtime/`+`standard/` but NOT the frozen spec mirror (`spec-markdown/`) — a silent edit to the Registry mirror passes CI | FC | OPEN (rank 10) |
| #17 CI | `.github/workflows/ci.yml` is a gate DEFINITION only — no CI host provisioned; freeze/drift gates rest on author discipline | FC + EXT | OPEN (rank 17) |

## G. External funnel (Bucket C — makes the G2 event possible without us)

| Item | Instrument | Status |
|------|-----------|--------|
| Complete `RELEASING.md` (CODE_OF_CONDUCT / SECURITY / public org) + PUBLISH | EXT | OPEN — **HARD-gated behind ranks 1,2,3,15** (never publish an under-attesting gate) |
| Distribute CHALLENGE + recognition path + leading-indicator funnel (fetches→attempts→self-check→submissions→G2) | EXT | OPEN (rank 18) |

---

*Every "CLOSED" here was verified at 0 freeze-drift with its instrument's record. Every "OPEN" names the
one instrument that discharges it. The single item that lifts independence past **G1** is **G2** (§A) —
external by construction. See `STRATEGIC_PROGRAM_2026-07.md` for the execution order.*

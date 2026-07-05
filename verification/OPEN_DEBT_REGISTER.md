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
| #20 | ACS-004 §5.1 urn↔type binding is normative but §6.5 doesn't enforce it; *"modulo namespace"* is underspecified for `uci.fact` (schema) vs `urn:arves:uci.core:fact@1.0` (instance) | CCP | 🟡 **DRAFT STAGED (CCP-009).** Adversarially-reviewed draft + oracle (`gen_ccp009_vector.py` GREEN): CURRENT `validate_instance` ACCEPTS a type/version-mismatched urn (#20 confirmed); proposed §6.5 clause 7 (full-form parse of the Identity carrier, pinned grammar incl. no-leading-zero versions) REJECTS all **4** candidate vectors with `urn-type-mismatch`; goldens unchanged (byte-clean). 4 open questions banked for the ruling. Ratification is maintainer-gated CCP-GATE. |
| #21 | ACS-005 §9.1 requires a `GL-nnn` entry for "Data Plane"; glossary closes at GL-014 (defined inline only). Real fix `GL-015` changes the §9.2 golden term-set vector | CCP (profile bump) | 🟡 **DRAFT STAGED (CCP-010).** Adversarially-reviewed draft + oracle (`gen_ccp010_vector.py` GREEN): both options demonstrated — **A** (GL-015 → ACS-005/2 profile bump; ContentId change computed; requires a **paired RCR** for the 2 frozen `runtime/` golden pins + TS/vectors mirrors, atomically with §8's GL-001..014 sentence) vs **B** (amend §9.1 wording, byte-clean). BLOCKER surfaced honestly: Vol 9 Part 2's broad plane list (Kernel+Persistence ARE Data Plane) vs the ACS-005 §7 narrow note — the maintainer must rule the definition before Option A. Ratification is maintainer-gated CCP-GATE. |
| #1/#2/#23 | ACS-003/004/005 negative-vector corpus | CCP | ✅ **CLOSED** (CCP-006/007: 19 semantic vectors + 11 codes; Kit 0.3.1) |
| #19/#22/#24 | int-range coverage · shortest-len clause · §9.3 glossary lint | CCP / done | ✅ **CLOSED** (CCP-007 / confirmed) |

## C. Runtime (frozen `runtime/` → RCR)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| RCR-004b | Expose the native Rust semantic validators to the harness (`acs_validate` bin) | RCR | ✅ **CLOSED** (RCR-004b, commit 154a698) |
| #3 (full) | Kernel commit-gateway recomputes the ACS-001 multihash from the payload pre-image | RCR (v1.1) | **OPEN — RULE #9 decision.** Needs a `domain` on `ProposedWrite` + a Kernel→`arves-acs` coupling. Kernel-owned half (content-hash ⇒ payload binding) is CLOSED (RCR-005); ACS-001 recompute stays at the bridge unless the maintainer rules to couple. |
| SHARD-001-F2 | Runtime `ShardKey` fields are `pub` (mutable-by-type); the opaque `arves-invariants::ShardKey` is unused | RCR | ✅ **CLOSED** (RCR-017, 2026-07-05: `arves-kernel::ShardKey` fields private, `ShardKey::new` rejects empty/>256B parts + `tenant()`/`workspace()` accessors; `arves-capability-fabric::ShardKey` aligned identically; every in-workspace call site updated; biting tests `behaviour_10_degenerate_shard_key_unrepresentable` + fabric `rcr017_*`. Honest residue: `arves-invariants::ShardKey` remains a separate unused opaque type — a consolidation would be a new layering decision, not this RCR) |
| #18 | PropertyCheck/Suite invariant→proof catalog | RCR | ✅ **CLOSED** (RCR-006; ORCH-001/002 honestly Pending until the Control Plane is implemented) |
| SHARD-001-F1 | Kernel cross-tenant isolation test | RCR | ✅ **CLOSED** (RCR-007, `behaviour_8_two_tenant_isolation`) |
| v1.1 backlog | bridge request-id correlation (vs positional FIFO) · engine-enforced (not self-declared) determinism · Kernel batch-commit (multi-effect atomicity) | RCR | ✅ **CLOSED — all three (2026-07-05):** **RCR-011** (protocol `id=<token>` echo; client matches by id; biting reverse-order fake-bridge regression, robustness 50/50) · **RCR-012** (fabric-derived ORCH-004 `invocation_key` + `invoke_enforced` double-invoke probe refusing a false `Deterministic` declaration BEFORE any commit; closes PureEngine's documented NON-CONFORMANT placeholder; honest: probe-not-proof) · **RCR-013** (same-shard atomic `commit_batch`, all-or-nothing over the validation class; cross-shard refused per IDR-004 saga rule; honest `PartialApply` boundary for host I/O). Workspace 87→**98/0**; freeze re-baselined per instrument (269 files). |
| v2.0 debt | truth-store cryptographic tamper-evidence / authenticated commit (zero-trust) | RCR (v2.0) | OPEN |
| I2–I6 | distributed runtime (per-shard Raft, IDR-001..005); the 11 CONTRACT-ONLY crates | RCR | **OPEN FOR BUILD — maintainer Ruling 002 (2026-07-05, `docs/MAINTAINER_RULINGS.md`) un-gated I2–I6**; build proceeds from the reviewed design packages in milestone order (I2→I6), each via RCR + 15-step discipline; primary driver = the maintainer's own product on ARVES. **PREP MODE complete (2026-07-05, maintainer ruling):** full engineering design packages for all five milestones exist at `docs/design/I{2,3,4,5,6}_*.md` (~3,300 lines: constitution BEFORE-WRITING-CODE answers, all 22 design sections, invariant/IDR mapping, conformance plans, NON-GOALS; each adversarially self-reviewed → revised, 23 findings applied, 0 skipped). No code was written during prep; **construction now starts from those reviewed designs** (Ruling 002 superseded the prep-mode gate). |

## D. Verification (freeze-clean `verification/`)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| B1 | Flagship SOUND gate attested only ACS-002 core (0/19 semantic) | FC | ✅ **CLOSED** (rank 1, commit 3c08ff5: full-surface, coverage-labeled, non-gameable) |
| B3/B4 | Harness gameable / crashes on Kit-only checkout | FC | ✅ **CLOSED** (sound verifier + degrade guards; `test_harness_integrity.py`) |
| Fz1 | Differential fuzz "0 hard divergences" is silent on interop-safe reason-code disagreements | FC | ✅ **CLOSED** (2026-07-05: the probe-owned ledger metric now surfaces the count explicitly — "13807 inputs, 0 hard divergences (302 all-reject reason-code differs, interop-safe)"; nothing is silent) |
| Semantic differential (r12) | The ACS-003/004/005 reject surface was *self*-conformance (Python-only, `conformance_semantic.py`) | FC | ✅ **CLOSED** (rank 12: `acs_semantic_differential.py` — the Rust native validators via `acs_validate` and the Python reference AGREE **62/62**, 0 hard divergences, over a deterministic single-mutation corpus; the semantic reject surface is now **DIFFERENTIAL** above the byte layer; drift-proof probe 11/11) |
| Capstone (r20) | No single runnable artifact tied the whole stack (products → bridge → Kernel → WAL) into one end-to-end, reproducible proof | FC | ✅ **CLOSED** (rank 20: `products/organization-day.capstone.mjs` — a governed org day (Personal + Enterprise OS) on the real Kernel, **re-run on a fresh Kernel byte-identical** (determinism), + the runtime's own `conformance_live` WAL-replay reconstruction (RCR-010). 10 properties held; drift-proof probe 12/12 `capstone-organization-day`) |
| 3-way fuzz (r19-A) | Add the conformant TypeScript codec as a 3rd differential-fuzz arm (was Rust↔Python; ACS-002 layer) | FC | ✅ **CLOSED** (rank 19 Part A: `typescript/src/decode_lines.mjs` — a TS line-protocol decoder driver mirroring the Rust `acs_decode` bin; `acs002_differential_fuzz.py` now drives all three over one corpus. **13807 inputs, 0 hard divergences** — identical accept/reject across Rust/Python/TypeScript, byte-identical re-encode on all 3135 accepts; nfc deferral generalized to any REJECT==`non-nfc-text`. Probe `differential` upgraded to 3-way.) The r19 **Part B** (promote the reference Connector/Query from `arves-conformance::live` into the frozen `arves-information-platform`/`arves-query` crates) is a **runtime RCR** — see §D "Live conformance"; still OPEN. |
| Formal | TLA+ kernel spec not mechanically model-checked (L0, no captured TLC run) | FC | ✅ **CLOSED** (rank 5, 2026-07-05: tooling unblocked with a portable Temurin 21 JRE + pinned tla2tools.jar; **TLC 2.19 exhaustive run — `SafetyInv` + `EventuallyCommitted` HOLD**, 20 distinct states, no error. Bonus real finding: the shipped MC.cfg declared SYMMETRY while checking liveness — unsound per TLC's own warning; it produced a **spurious** counterexample on the first run and is now fixed. Captured record + repro: `verification/formal/TLC_RUN.md`) |
| Live conformance | The 12 Scenario axes are typed but zero-instantiated (L0); no live-runtime `ConformanceArtifact` | RCR (not FC — the contract types are `#[non_exhaustive]`, so the live impl must live in `arves-conformance`) | ✅ **L1 CORE PIPELINE CLOSED — end-to-end live** (RCR-008 Kernel · RCR-009 Information/Connector · RCR-010 Query/WAL-replay). The single L0 behaviour row is now an executing 3-node `ConformanceArtifact` (Information→Kernel→Query, `Verdict::Pass`, 7 invariant-checks from behaviour; drift-proof probe 10/10). The **deeper L1–L4 axes** (Control Plane / Engine / Capability / multi-agent / distributed) stay typed-only, gated behind Standard Validation (I2–I6); promoting the reference Connector/Query into the `arves-information-platform`/`arves-query` crates is rank 19. |

## E. Product (freeze-clean `products/`)

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| E1 | enterprise-os "requires legal approval" is proposer-self-attested (approvals is a caller array) | FC | ✅ **CLOSED** (rank 14: approval is now a SEPARATE committed `uci.approval` truth via `approve({role,subject})`; `checkPolicy` reads `#approvals` not the proposer's array — the finance agent can no longer self-clear its own gate. Biting regression: a decision self-declaring `approvals:['legal']` is now BLOCKED (49/49). **RESIDUAL → runtime #8:** Runtime v1.0 has no authN on `commit`, so the `role` tag isn't cryptographically bound — a fully authenticated approval identity is the v2.0 authenticated-commit RCR.) |
| E2 | a bare-`Number` amount crashes the ACS commit (deep opaque encoder error); spend policy applies only on exact `spend:` subject prefix | FC | ✅ **CLOSED (amount hardening)** (rank 14: `#toIntUsd` coerces an integer Number/string USD amount to canonical BigInt and rejects any non-integer EARLY with a field-named product error — no more deep dCBOR crash; regressions for both arms). The exact-`spend:`-prefix subject scope is a separate **policy-language** decision (CCP-class, tracked with #20), not an input-crash. |
| M1 | marketplace signature binds artifact bytes but NOT the advertised catalog/install identity (`cap.manifest` ≠ `artifact.manifest`) | FC | ✅ **CLOSED** (rank 11: `manifestBinds` deep-equal guard in publish + host.install + registry.install; biting regression `robustness.test.mjs` — a valid artifact for B under A's name is refused; 46/46) |
| #11 | capability determinism gate is a best-effort author-input probe, not enforcement | FC / RCR | 🟡 **PARTIALLY DISCHARGED.** The runtime half is CLOSED: **RCR-012** landed engine-layer **ENFORCED** determinism on the real invoke path (`arves-engine-fabric::invoke_enforced` — fabric-derived ORCH-004 `invocation_key` + a double-invoke probe that REFUSES a false `Deterministic` declaration BEFORE any effect reaches the Kernel; the bridge invokes only through it). The remaining half is products-side: the authoring-kit **certify** step is still a best-effort author-input probe (a probe, not a proof — it samples the author's own inputs and cannot enforce anything at runtime). |
| P1 | personal-os "durable decision history" overstated the in-memory detection index | DOC | ✅ **CLOSED** (wording + ledger caveat) |
| #4 | products commit to the real Kernel (not just an in-memory Map) | FC/DOC | ✅ **CLOSED** (routed through bridge; claims scoped) |

## F. Governance / docs

| ID | Item | Instrument | Status |
|----|------|-----------|--------|
| V1 | `ARVES_00_Invariant_Registry_v1` (frozen `.docx` mirror) still reads "no runtime code exists yet" / all proofs "pending" — contradicts RCR-006's 5/7-proven catalog | CCP / regenerate | OPEN (acknowledged in CLAUDE.md; `.docx` regen, not a silent edit). Regen-staging drafts exist at `verification/ccp-drafts/regen-staging/` (ccp lane). |
| V2 | `ARVES_IDR_Batch_1_Kernel_Distribution_v1` cites G-001 / QUERY-001 inline without a "proposed/pending" qualifier | CCP / regenerate | OPEN. Regen-staging drafts exist at `verification/ccp-drafts/regen-staging/` (ccp lane). |
| V3 | Milestone identifiers diverge: `ARVES_Reference_Lifecycle_v1` uses M10/M11/M12 vs I1..I6 elsewhere; no reconciliation table | CCP / regenerate | OPEN. Regen-staging drafts exist at `verification/ccp-drafts/regen-staging/` (ccp lane). |
| MR-drift | `runtime/docs/ARVES_Master_Roadmap.md` still says `arves-standard-kit 0.2.0` (current 0.3.1); it is under `runtime/` (frozen) | RCR / regenerate | ✅ **CLOSED (RCR-018, doc-only, 2026-07-05):** Kit version corrected to 0.3.1 (pointing at `standard/VERSION` as the single source) + every Ruling-002-invalidated gating claim in the frozen roadmap marked superseded ("gated behind Era 3" / "post-G2" / "not I2" / "forbidden until certified"); `runtime/README.md` checked — no stale claims. Record: `runtime/rcr/RCR-018.md`. |
| #36/#37 | Documentation-Index "current" marker · a dedicated "certify YOUR runtime" quickstart snippet | DOC / FC | ✅ **CLOSED** (rank 15). **#37:** `verification/certification/certify_your_runtime.py` — a copy-paste driver that grades YOUR runtime through the non-gameable `grade_sound` (zero Python if it speaks three line protocols); `--self-test` grades the reference bins via the vendor path (SOUND-CERTIFIED, proving the driver is real) + the single page `CERTIFY_YOUR_RUNTIME.md`; wired from README/QUICKSTART/CHALLENGE/IMPLEMENTING; non-gameability regression-locked (`test_harness_integrity.py` B3-driver: hollow-via-driver REJECTED, real-via-driver CERTIFIED). **#36:** `docs/SPEC_STARTER.md` now marks `Documentation_Index_v2.2` as the current master register and v1/v2/v2.1 as superseded. |
| #16/#10 | freeze-diff gate covers `runtime/`+`standard/` but NOT the frozen spec mirror (`spec-markdown/`) — a silent edit to the Registry mirror passes CI | FC | ✅ **CLOSED** (rank 10: `FROZEN_ROOTS` now includes `spec-markdown/` + `corpus/`; gate covers **261** files, selftest bites a `.docx` tamper). The V1-V3 *content* fixes are still CCP/regen — but a silent edit to them is now caught. |
| #17 CI | `.github/workflows/ci.yml` is a gate DEFINITION only — no CI host provisioned; freeze/drift gates rest on author discipline | FC + EXT | ✅ **CLOSED — CI is LIVE (2026-07-05, r17).** Repo published to `github.com/arves-org/arves`; all 5 gates GREEN on a clean Linux clone (freeze 219/0 · workspace 98/0 · robustness 50/50 · certification+integrity · evidence probe `--check` 12/12 CONSISTENT). Branch protection: the freeze job is a **Required** status check on `main`. First-publish findings fixed en route: platform-dependent freeze hashing (67 CRLF phantom drifts) and filesystem-vs-tracked manifest (50 local-artifact phantom drifts) — both now impossible by construction. |

## G. External funnel (Bucket C — makes the G2 event possible without us)

| Item | Instrument | Status |
|------|-----------|--------|
| Complete `RELEASING.md` (CODE_OF_CONDUCT / SECURITY / public org) + PUBLISH | EXT | ✅ **PUBLISHED (2026-07-05, r16).** Maintainer chose the org (`arves-org`) and authorized the push. Live: **repo** `github.com/arves-org/arves` (public, `main` + 12 tags) · **release** `arves-build-v1.0` · **docs-site** `arves-org.github.io/arves` (92 pages, link-gated deploy, HTTP 200) · **CI** 5/5 gates green on a clean clone, freeze job Required on `main`. Residual maintainer nicety: the private contact address field in COC/SECURITY (GitHub private reporting suffices meanwhile). **The G2 funnel is now physically possible — Bucket C is open for strangers.** |
| Distribute CHALLENGE + recognition path + leading-indicator funnel (fetches→attempts→self-check→submissions→G2) | EXT | OPEN (rank 18) |

---

*Every "CLOSED" here was verified at 0 freeze-drift with its instrument's record. Every "OPEN" names the
one instrument that discharges it. The single item that lifts independence past **G1** is **G2** (§A) —
external by construction. See `STRATEGIC_PROGRAM_2026-07.md` for the execution order.*

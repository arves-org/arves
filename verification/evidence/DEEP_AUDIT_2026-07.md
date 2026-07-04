# ARVES Deep Audit — un-audited surfaces (Madde 11, 2026-07-04)

> **Run by:** the maintainers (Chief Architect) via four parallel read-only auditors.
> **Grade:** **G1 (self / same-repo).** Internal audit. Changes nothing frozen by itself; each fix
> routes through its sanctioned instrument. Closes SYSTEM_GAP_ANALYSIS §5's "still-open surfaces not
> deeply audited": SHARD-001 tenant isolation, the differential fuzzer internals, the four
> not-yet-audited products, and spec-Volume / `.docx`↔`spec-markdown` fidelity.

## 0. One-paragraph verdict

**The system is substantially honest and the load-bearing machinery holds.** The differential fuzzer
genuinely cross-checks two languages deterministically (13807/0 reproduced byte-identically);
SHARD-001 isolation is really enforced at persistence (biting wrong-shard test); the four products
route commits through the real Kernel and each source file carries an honest scope caveat; spec
mirror fidelity is 1:1 (50/50, authoritative direction correct) and the substantive Volumes use only
registered invariant IDs. The confirmed gaps are **one major product-honesty defect** (enterprise-os
self-attested approval), **a Kernel-level SHARD-001 proof gap** (now fixed — RCR-007), and a cluster
of **honesty-wording / stale-frozen-doc** items. No isolation or certification claim is contradicted
by working code.

## 1. SHARD-001 tenant isolation

**Verdict: enforced at persistence; was structural-only at the Kernel (now fixed).**
- ✅ Persistence physically separates shards (per-shard `MemLog` / per-shard dir
  `<hex tenant>__<hex workspace>`) and **actively rejects** a foreign record on write
  (`WalError::UnknownShard`) and read (`decode_body` corruption), with biting tests
  (`file_wal.rs::wrong_shard_append_rejected`, `multi_shard_isolation_survives_disk`).
- ✅ Kernel exposes **no read surface**; commit keys truth by `(tenant, workspace, content)`.
- **F1 [MAJOR] → FIXED (RCR-007):** the Kernel had no cross-tenant negative test (all tests hardcoded
  `t1/w1`) and the PropertyCheck SHARD-001 entry cited *structure*, not a named test. Added
  `behaviour_8_two_tenant_isolation` (same content / different shard → distinct truths, no snapshot
  leak) and repointed the citation.
- **F2 [MINOR] → tracked (RCR):** the runtime `ShardKey` fields are `pub` (mutable-by-type); the
  opaque `arves-invariants::ShardKey` is unused in the runtime. Low exploitability (nothing re-keys).
- **F3 [MINOR] → tracked (products doc/RCR):** products are single-tenant (bridge hardcodes `t1/w1`,
  carries no tenant); `EnterpriseCognitiveOS` markets an "org OS" yet gives no tenant isolation (two
  orgs on one bridge share `t1/w1`). No "trust the caller's tenant" bug — no tenant is caller-supplied.
- **F4 [MINOR] → FIXED (doc):** `G2_READINESS.md` filed the ACS-003 envelope *presence* check (reject
  empty tenant/workspace) under "SHARD-001" — a presence check, not isolation. Wording corrected.

## 2. Differential fuzzer (`acs002_differential_fuzz.py`)

**Verdict: HONEST and reproducible — no overclaim on the core assertion.**
- ✅ Real two-language cross-check: Rust `acs_decode` bin (calls `arves_acs::cbor`) vs the independent
  Python `acs002_decode`, same bytes zipped and compared on accept/reject **and** re-encoded bytes.
- ✅ Deterministic: fixed `SEED = 20260702`; two runs byte-identical (`13807 / 0 hard divergences`).
- ✅ Covers encode + decode + a heavy reject surface; a missing Rust bin causes a **loud** failure
  (nonzero exit), never a silent false pass.
- **Fz1 [LOW] → noted:** "0 hard divergences" is silent on **16 reason-code disagreements** (both arms
  reject, different reason — the two decoders check rules in different order). Interop-safe by the
  harness's stated design (accept-vs-reject is the interop property, not the reason code) and printed
  to stdout ("reason differ 16"), but the ledger metric doesn't reflect it. A wording tightening, not
  a defect — recorded here; the reason-code corpus that *does* pin codes is the CONFORMANCE negative
  vector set.

## 3. Products (marketplace / agent-runtime / enterprise-os / personal-os)

**Verdict: all route commits through the real Kernel + carry honest file-top caveats; one MAJOR
governance-claim defect + minor honesty items.**
- **arves-agent-runtime — CLEAN.** Every trace step commits to the real Kernel; the in-memory `Arves`
  is used only for the local-id one-world check, precisely as its caveat states.
- **E1 [MAJOR] → FIXED (claim scoped):** enterprise-os's headline "requires legal approval" control is
  satisfiable by the constrained party — `approvals` is a caller-supplied array on the proposer's own
  decision (`enterprise-os.mjs:66`), and the demo has **finance** self-supply `approvals:['legal']`
  and commit (`enterprise-day.mjs:27`). No authenticated legal-agent approval truth. The claim (README
  + ledger + inline) is scoped honestly; the full fix (separate authenticated approval truths) is
  tracked as a product living_fix.
- **E2 [MINOR] → tracked:** the spend policy applies only on an exact caller-controlled `subject`
  prefix (`'spend:'`); a renamed subject silently bypasses it, and a bare-`Number` amount crashes the
  ACS commit rather than being cleanly rejected.
- **M1 [MINOR] → tracked:** the marketplace signature binds the artifact bytes but **not** the
  advertised catalog/install identity — no `cap.manifest` deep-equals `artifact.manifest`, and the
  catalog/install key comes from the live manifest, so a validly-signed artifact for capability *B*
  can be served under name *A* (cert + codeHash still bite; impact is squatting of *certified* code).
  Certification **is** re-verified at publish + install (no trusted flag) — Q2 clean.
- **P1 [MINOR] → FIXED (wording):** personal-os inline "persistent/durable decision history" overstates
  the in-memory `#decisions` map used for contradiction detection (lost on process exit; the WAL commit
  is durable, the *detection state* is per-process). Softened.
- **X1 [MINOR] → FIXED (ledger caveat):** the Evidence Ledger caveats the P6.5 determinism probe but
  asserts P4/P5 "compliance ledger" / "contradiction-with-prior-decision" without the same
  "in-memory, process-scoped" caveat the product sources carry. Caveat added to those rows.
- ✅ Runnability: all example imports are relative and resolve; the only prerequisite is the built
  `arves-bridge` bin (in every README).

## 4. Spec Volumes / mirror fidelity

**Verdict: most sub-areas CLEAN; three findings, ALL in the frozen `.docx` corpus → CCP / regeneration
(never a silent edit; the `.docx` cannot be edited from the living tree).**
- ✅ CLEAN: mirror filename correspondence (50 `.docx` ↔ 50 `.md`, 0 orphans); authoritative direction
  (`.docx` wins) stated and never reversed; the six substantive Volumes use only registered invariant
  IDs; "Data Plane" is defined at its point of use (Vol 9 Part 2); the 12-axis model + layer-matrix are
  consistent across Vol 9 / Engine Graph / Scenario Framework / Vol 6.
- **V1 [MEDIUM] → CCP/regenerate (already acknowledged):** `ARVES_00_Invariant_Registry_v1` still reads
  "no runtime code exists yet" with every proof "pending" — contradicts the built I1 runtime. CLAUDE.md's
  maintainer note already flags this; the fix is regenerating the frozen `.docx`, not a silent edit.
- **V2 [LOW-MED] → CCP/regenerate:** `ARVES_IDR_Batch_1_Kernel_Distribution_v1` lists **G-001** and
  **QUERY-001** inline with registered invariants (lines 15, 53) with no "proposed/pending CCP"
  qualifier — the exact anti-pattern the OS Volumes warn against, and the origin of the registry's
  "referenced but never defined" note.
- **V3 [LOW] → CCP/regenerate:** milestone identifiers diverge — `ARVES_Reference_Lifecycle_v1` uses
  **M10/M11/M12** for Distributed Runtime / Multi-Agent / Reference Products, while Vol 6 + Baseline +
  CLAUDE.md use **I1..I6**; no reconciliation table. Both frozen.

## 5. Disposition

| Finding | Sev | Instrument | Status |
|---|---|---|---|
| SHARD-001 F1 (no Kernel isolation test) | major | RCR | ✅ **fixed (RCR-007)** |
| enterprise-os E1 (self-attested approval) | major | living_fix (claim scoped now; full fix tracked) | ✅ **claim scoped** |
| SHARD-001 F4 (G2_READINESS wording) | minor | doc | ✅ fixed |
| personal-os P1 (durable overstatement) | minor | doc | ✅ fixed |
| ledger X1 (caveat asymmetry) | minor | doc | ✅ fixed |
| fuzzer Fz1 (16 reason-differ silent) | low | living_fix | noted (interop-safe by design) |
| SHARD-001 F2 (mutable ShardKey) | minor | RCR | tracked |
| SHARD-001 F3 (products single-tenant) | minor | products doc/RCR | tracked |
| enterprise-os E2 (prefix matcher) | minor | living_fix | tracked |
| marketplace M1 (identity binding) | minor | living_fix | tracked |
| spec V1/V2/V3 (frozen corpus) | med/low | CCP / regenerate `.docx` | recorded (maintainer-gated) |

**Independence unchanged: G1.** This audit hardens the surface a G2 party stands on; it is not a G2 event.

# V1 Regen Staging — Invariant Registry mirror: "no runtime code exists yet" / all proofs "pending"

**Status:** STAGED correction — NOT applied. Target is the frozen `.docx`
`ARVES_00_Invariant_Registry_v1.docx` and its mirror
`spec-markdown/ARVES_00_Invariant_Registry_v1.md`. Regeneration is maintainer-gated
(open-debt register V1; acknowledged in CLAUDE.md Maintainer Note).

## Defect

The frozen mirror asserts no runtime exists and every proof is pending. The I1 runtime exists
(workspace green) and RCR-006's executable catalog (`arves-conformance::property_check`,
pinned by test `property_check_suite_holds`) binds the 7 registered invariants to proofs:
**5 proven / 2 pending** (LAYER-001, OWN-001 in-process; ORCH-003, ORCH-004, SHARD-001 cited
tests; ORCH-001, ORCH-002 pending — arves-control-plane is contract-only, I2+).

## Exact current frozen text (verbatim)

Part 1, closing sentence:

> Proof status for all invariants is currently "pending" - no runtime code exists yet; each must gain an executable proof during its owning milestone.

Part 2 table, `Proof` column: all seven rows read `pending`.

## Exact corrected text

Part 1, closing sentence — replace with:

> Proof status is tracked per invariant in Part 2. The I1 reference runtime exists; RCR-006's executable catalog (arves-conformance::property_check) binds each registered invariant to its proof and pins coverage against silent drift. Invariants whose owning component is not yet implemented remain "pending" and must gain an executable proof during their owning milestone.

Part 2 table, `Proof` column — replace row values:

| ID | Proof (corrected) |
| --- | --- |
| OWN-001 | proven — in-process (property_check catalog; architecture gate: single `pub trait Kernel` owner) |
| LAYER-001 | proven — in-process (property_check catalog; downward-only edges over the real Cargo graph) |
| SHARD-001 | proven — cited tests (behaviour_8_two_tenant_isolation; file_wal wrong_shard_append_rejected / multi_shard_isolation_survives_disk; RCR-007) |
| ORCH-001 | pending — arves-control-plane contract-only (I2+) |
| ORCH-002 | pending — arves-control-plane contract-only (I2+) |
| ORCH-003 | proven — cited test (walking_skeleton replay behaviours) |
| ORCH-004 | proven — cited test (walking_skeleton idempotency/content-integrity, RCR-005) |

Part 4 table (proposed invariants): `Proof` column values stay `pending` — correct as frozen.

No other content changes. Part 5's counts (14 registered incl. principles, 23 proposed) are
unaffected.

## Regeneration instrument (maintainer-gated; never a silent .md edit)

1. Maintainer authorizes the regeneration (this is the CCP/regenerate path already reserved by
   CLAUDE.md's Maintainer Note and `verification/OPEN_DEBT_REGISTER.md` V1).
2. Edit the authoritative `ARVES_00_Invariant_Registry_v1.docx` with exactly the corrected text
   above (Part 1 sentence + Part 2 Proof column), recording the change as a
   CCP-instrument correction (status-tracking update, not a normative statement change — no
   invariant statement, source, or classification is touched).
3. Regenerate the mirror: `python tools/docx_to_markdown.py` (per the mirror's own header:
   "Do not hand-edit — regenerate via python tools/docx_to_markdown.py").
4. Re-baseline the freeze: `python freeze_check.py update` (maintainer-run), confirming drift
   is exactly the regenerated mirror + the `.docx`.
5. Close open-debt item V1 in `verification/OPEN_DEBT_REGISTER.md` and update the CLAUDE.md
   Maintainer Note parenthetical ("the frozen Invariant Registry .docx mirror still reads…")
   — both living files, same commit.

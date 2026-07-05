# V2 Regen Staging — IDR Batch 1 cites G-001 / QUERY-001 without proposed-status qualifiers

**Status:** STAGED correction — NOT applied. Target is the frozen `.docx`
`ARVES_IDR_Batch_1_Kernel_Distribution_v1.docx` and its mirror
`spec-markdown/ARVES_IDR_Batch_1_Kernel_Distribution_v1.md`. Regeneration is maintainer-gated.

## Defect

The frozen Invariant Registry (Part 4) and CLAUDE.md classify `G-001` and `QUERY-001` as
**proposed — informative, NOT ratified, no conformance weight until CCP-GATE**. IDR Batch 1
cites both alongside registered invariants (ORCH-001, OWN-001, SHARD-001) with no qualifier,
so a reader of the IDR alone would take them as registered-normative. (Registry Part 5:
"CLAUDE.md must not claim it as registered, until it passes the CCP-GATE.")

## Exact current frozen text (verbatim)

IDR-001 Context (first sentence):

> **Context: **The Kernel is the single owner of cognitive truth (ORCH-001, G-001, OWN-001). Distributed truth must not diverge. State is partitioned by tenant/workspace (SHARD-001).

IDR-001 closing line:

> **Spec invariants upheld: **ORCH-001 (truth = Kernel), G-001, OWN-001, QUERY-001 (read-only), SHARD-001 (per-tenant partition).

## Exact corrected text

IDR-001 Context — replace the first sentence with:

> **Context: **The Kernel is the single owner of cognitive truth (ORCH-001, OWN-001; also consistent with proposed invariant G-001 — informative, pending CCP-GATE per the Invariant Registry Part 4). Distributed truth must not diverge. State is partitioned by tenant/workspace (SHARD-001).

IDR-001 closing line — replace with:

> **Spec invariants upheld: **ORCH-001 (truth = Kernel), OWN-001, SHARD-001 (per-tenant partition). Additionally consistent with the proposed (informative, not yet CCP-ratified) invariants G-001 and QUERY-001 (read-only); per the Invariant Registry Part 4 these carry no conformance weight until ratified.

No other occurrence of G-001/QUERY-001 exists in this document; no decision text changes
(the IDR-001..005 decisions themselves are untouched — this is a citation-status correction
only, aligning the IDR with the Registry's own governance rule).

## Regeneration instrument (maintainer-gated; never a silent .md edit)

1. Maintainer authorizes the correction as a CCP-instrument citation fix (registered-vs-
   proposed labeling; no engineering decision altered — IDR content stays IDR content).
2. Edit the authoritative `ARVES_IDR_Batch_1_Kernel_Distribution_v1.docx` with exactly the two
   corrected passages above.
3. Regenerate the mirror: `python tools/docx_to_markdown.py`.
4. Re-baseline the freeze: `python freeze_check.py update` (maintainer-run), drift confined to
   the regenerated mirror + the `.docx`.
5. Record closure in `verification/OPEN_DEBT_REGISTER.md` (V2) in the same commit.

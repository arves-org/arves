# V3 Regen Staging — Reference Lifecycle M10/M11/M12 vs Baseline I1..I6 (no reconciliation)

**Status:** STAGED correction — NOT applied. Target is the frozen `.docx`
`ARVES_Reference_Lifecycle_v1.docx` and its mirror
`spec-markdown/ARVES_Reference_Lifecycle_v1.md`. Regeneration is maintainer-gated.

## Defect

Two frozen documents number the Implementation-Era milestones differently with no mapping.
The Baseline is the single source (CLAUDE.md: "from the frozen ARVES v1.0 Baseline, Part 5 —
single-sourced, do not diverge"):

`spec-markdown/ARVES_00_Baseline_v1.md` (Part 5 table, verbatim rows):

> | I1 Distributed Runtime | Cluster execution; distributed replay |
> | I2 Cluster Kernel | Kernel replication; consensus if required |
> | I3 Distributed Query | Query routing; LCW partitioning |
> | I4 Capability Scheduling | Cluster-wide capability scheduling |
> | I5 Multi-Agent Runtime | Agent spawning, delegation, coordination at scale |
> | I6 Reference Products | Products on certified runtime -> ARVES v1.0 GA |

The Reference Lifecycle uses an unexplained M10/M11/M12 scheme instead. A reader cannot tell
whether M10..M12 are three *different* milestones, a renumbering, or a grouping — the corpus
never says.

## Exact current frozen text (verbatim)

`spec-markdown/ARVES_Reference_Lifecycle_v1.md`, Part 10 — Era Transition, second bullet:

> - Implementation Era (next): Distributed Runtime (M10), Multi-Agent Runtime (M11), Reference Products (M12), Enterprise Runtime, Marketplace, Certification Program, ARVES v2.

## Exact corrected text

Replace the bullet with:

> - Implementation Era (next): Distributed Runtime (M10 — groups Baseline Part 5 milestones I1 Distributed Runtime, I2 Cluster Kernel, I3 Distributed Query and I4 Capability Scheduling), Multi-Agent Runtime (M11 = I5), Reference Products (M12 = I6), Enterprise Runtime, Marketplace, Certification Program, ARVES v2. Reconciliation: the Baseline Part 5 I-numbering (I1..I6) is the single-sourced milestone scheme; M10..M12 continue this document's own M1..M9 lifecycle numbering as coarser groupings of it and define no additional milestones.

Rationale for the mapping (title-level, mechanical): M11 "Multi-Agent Runtime" and M12
"Reference Products" match I5/I6 verbatim; M10 "Distributed Runtime" matches I1 by title and,
being the only remaining pre-M11 implementation milestone, necessarily spans the distributed
work the Baseline splits as I1..I4. The mapping introduces no new milestone and re-scopes
none — it makes the Baseline's authority explicit, as CLAUDE.md already requires.

## Regeneration instrument (maintainer-gated; never a silent .md edit)

1. Maintainer confirms the M10=I1..I4 / M11=I5 / M12=I6 mapping (the only judgement call is
   whether M10 groups I1..I4 or equals I1 alone; the corrected text states the grouping
   reading — if the maintainer rules M10=I1 only, substitute "(M10 = I1 Distributed Runtime;
   I2..I4 follow per Baseline Part 5)" in the text above).
2. Edit the authoritative `ARVES_Reference_Lifecycle_v1.docx` Part 10 bullet with exactly the
   corrected text (a CCP-instrument reconciliation note; no lifecycle process content changes).
3. Regenerate the mirror: `python tools/docx_to_markdown.py`.
4. Re-baseline the freeze: `python freeze_check.py update` (maintainer-run), drift confined to
   the regenerated mirror + the `.docx`.
5. Record closure in `verification/OPEN_DEBT_REGISTER.md` (V3) in the same commit. No Baseline
   edit — it is the authority being pointed at, not the document being corrected.

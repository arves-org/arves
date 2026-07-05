# CCP-010 Ruling Memo — "Data Plane": broad (Vol 9 v2) vs narrow (ACS-005 §7 inline)

**Status:** DRAFT maintainer decision memo — NOT a ruling, NOT ratified. Staged under
`verification/ccp-drafts/` only; no frozen byte is touched. This memo answers the blocking
**Open Question §1.1** of `verification/ccp-drafts/CCP-010-gl015-data-plane.md`, which CCP-010
states MUST be ruled before its Option A (issue `GL-015`) can be ratified.

## 1. The conflict, verbatim

**Broad** — frozen `spec-markdown/ARVES_Volume_9_Cognitive_Control_Plane_v2.md`, Part 2:

> ARVES is read on two axes, not one stack. The Data Plane carries information and state; the
> Control Plane decides. The Kernel never becomes the Control Plane.
>
> Data Plane (carries): Reality, Information Platform, Kernel, Living Cognitive World,
> Persistence, Query, Engine Fabric (engines as pure compute), Capability Fabric, Execution,
> mechanical Runtime (event bus, task, workflow).

**Narrow** — `standard/acs/ACS-005_Normative_Language.md`, §7 closing note (the note after GL-014):

> `Data Plane` and `Tenant`/`Workspace` appear as capitalized normative terms in the checker
> list (§9) and resolve to GL-010's owning plane and GL-004 respectively; `Data Plane` is
> defined inline as the pure-execution plane (the Engine/Capability/Execution layers) that owns
> nothing persistent.

The narrow note is internally torn within one sentence: "GL-010's owning plane" is, per Vol 9
Part 3 ("The engine ITSELF (Engine Fabric, M7) is Data Plane"), the broad carrying plane — yet
the same sentence then *defines* the plane as only Engine/Capability/Execution. Vol 9 Part 3
supports membership ("Engine Fabric is Data Plane; Capability Fabric is Data Plane"), never
exclusivity.

## 2. What GL-015 would say under each definition

CCP-010 §2 already drafts both candidate rows; this memo cites them rather than re-drafting:

- **A-broad** (Vol 9 v2 Part 2 verbatim): GL-015 = "The carrying plane of the two-plane model:
  Reality, Information Platform, Kernel, Living Cognitive World, Persistence, Query, Engine
  Fabric, Capability Fabric, Execution, and the mechanical Runtime. *Necessary:* carries
  information and state while the Control Plane decides; within it only the Kernel MAY own
  Cognitive Truth (ORCH-001). *Sufficient:* membership in the Vol 9 Part 2 carrier list."
  → supersedes ACS-005 §7's narrow inline clause.
- **A-narrow** (ACS-005 §7 note verbatim): GL-015 = "The pure-execution plane — the
  Engine/Capability/Execution layers — that owns nothing persistent. *Necessary:* comprises only
  stateless execution layers; MUST NOT own Cognitive Truth or any persistent state."
  → rules against frozen Vol 9 v2 Part 2.

Either ruling is meaning-affecting at corpus level (CCP-010 §1.1); neither changes the §8
Term-Set Address differently (only Term IDs are hashed).

## 3. Evidence: what the frozen corpus and the frozen runtime already assume

1. **Coherence.** Under NARROW, the Kernel belongs to *no* plane — yet Vol 9 Part 2 says
   verbatim "Kernel" is in the Data Plane list and "The Kernel never becomes the Control
   Plane". NARROW makes frozen Vol 9 v2 self-contradictory. Under BROAD, everything is
   coherent: the plane carries state; *within* it, ORCH-001 restricts truth ownership to the
   Kernel alone. Vol 9 Part 12 also reclassifies the mechanical Runtime "as Data Plane
   mechanical runtime" — again broad.
2. **The frozen Runtime v1.0 is uniformly broad.** Crate doc-headers self-classify (grep
   `"Data Plane"` in `runtime/crates/*/src/lib.rs`): `arves-kernel` ("Layer: Data Plane …
   Kernel"), `arves-persistence` ("Persistence (Data Plane)"), `arves-lcw`, `arves-query`,
   `arves-information-platform` ("Information Platform (Data Plane)"), `arves-ontology`,
   `arves-engine-fabric`, `arves-execution`, `arves-capability-fabric`, and
   `arves-invariants` ("a Data Plane … which carries truth"). Zero crates encode the narrow
   reading. Ruling NARROW would falsify ~10 frozen crate headers and the invariants crate's
   own rationale text — requiring an RCR whose only purpose is to make the implementation
   match a glossary note that contradicts the spec the implementation was built from.
3. **Authority ordering.** The dependency chain is Specification → Contracts → Behaviour.
   Vol 9 v2 is frozen Specification-Era corpus; the ACS-005 §7 note is one inline sentence in
   a later standards convention (CCP-005) whose *purpose* was citability, not plane
   re-definition. Where they conflict, the specification wins; the standard serves it.
4. **No protection is lost.** Everything the narrow wording tries to guarantee is already
   normatively owned elsewhere: GL-010 (Engine "owns nothing persistent"), GL-013, ORCH-001
   (only the Kernel owns truth), ORCH-004 (idempotent invocations). NARROW adds no obligation
   BROAD lacks; it only mislocates an Engine-layer property onto the whole plane.

## 4. Recommendation

**Rule BROAD** (Vol 9 v2 Part 2 is the meaning of "Data Plane"; the ACS-005 §7 inline clause
"is defined inline as the pure-execution plane … owns nothing persistent" is a drafting error
that described the Engine/Capability/Execution *sub-plane*, and is superseded for that term).
Then ratify CCP-010 **Option A with Candidate A-broad**.

Rationale: it is the only option that (a) keeps frozen Vol 9 v2 self-consistent, (b) matches
the entire frozen Runtime v1.0 byte-for-byte as it stands today (no relabeling RCR), (c)
respects the Spec→Standard authority ordering, and (d) costs nothing normatively (see §3.4).

## 5. Ratification steps, per option (all maintainer-gated)

**If BROAD is ruled (recommended):**
1. Record the ruling (e.g. `docs/MAINTAINER_RULINGS.md` entry: "Data Plane = Vol 9 v2 Part 2
   broad carrying plane; ACS-005 §7 inline narrow clause superseded").
2. Promote CCP-010 DRAFT → ratified with Candidate **A-broad** as the GL-015 row; execute
   CCP-010 §6 Option-A steps (a)–(f) exactly as drafted there: ACS-005 §7 row + closing-note
   fix (14→15; strike the narrow inline clause, keep Tenant/Workspace), §8 sentence
   `GL-001…GL-015`, §9.2 vector 1 replacement (ContentId `12204668…46aa`, oracle-proven),
   TSV + `acs_golden_vectors.md` mirror, living Python/TS checkers, **paired RCR (v1.1)** for
   the two frozen runtime golden pins (`arves-acs/tests/acs_platform.rs`,
   `arves-conformance/src/acs.rs`), `freeze_check.py update`, re-certify both runtimes.
3. No Vol 9 edit and no crate-header RCR needed — BROAD is what they already say.

**If NARROW is ruled:**
1. This overrides frozen Vol 9 v2 Part 2's member list — per the Constitution's Change
   Management table that is a **Specification change → Next Major Version**, not a CCP
   Amendment. Option A-narrow is therefore blocked until an ARVES v2 instrument exists.
2. Additionally requires an RCR relabeling ~10 runtime crate doc-headers plus
   `arves-invariants` rationale text, and a decision on which plane Kernel/Persistence/LCW/
   Query/Information Platform then belong to (a third plane = new architectural layer,
   forbidden by Non-Negotiable Rule 3 absent a spec instrument).
3. Not recommended; recorded for completeness.

**If deferred:** the corpus keeps two conflicting definitions; the reference checker stays
PASS-GATED on "Data Plane"; CCP-010 Option B (byte-clean §9.1 wording) remains the fallback
but resolves only the citability gap, not this meaning conflict.

## 6. Honesty

- This is a memo recommending a ruling; the ruling itself is the maintainer's alone.
- The runtime-usage evidence (§3.2) is descriptive input, not authority — implementation never
  changes specification; it is cited because it shows which reading the spec-derived build
  actually encoded, i.e. as *proof about the spec's operative meaning*, not as a vote.
- All quotes above are verbatim from the named frozen files; byte/ContentId claims are
  inherited from CCP-010's oracle (`gen_ccp010_vector.py`), not recomputed here.

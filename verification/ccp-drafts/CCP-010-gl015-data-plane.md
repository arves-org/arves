# CCP-010 (DRAFT) — "Data Plane" glossary resolution: GL-015 vs §9.1 amendment (closes open-debt #21)

**Status:** **DRAFT proposal — NOT ratified, NOT applied.** Freeze-clean: this document + its
oracle (`gen_ccp010_vector.py`) live under `verification/ccp-drafts/`. **Nothing here edits the
frozen `standard/`.** Ratification is a separate, maintainer-authorized step; under **Option A**
the frozen surface is: ACS-005 §7, §8 (Term-Set Address sentence), §9.2 (via **CCP-GATE**), the
golden vector TSV **and** its human-readable mirror `acs_golden_vectors.md`, both reference
checkers plus the TypeScript differential arm (living `verification/`), **and two frozen
`runtime/` files that pin the v1 golden — which require a paired Runtime Change Request (v1.1)
per `runtime/RUNTIME_FREEZE_v1.0.md`, an instrument CCP-GATE alone cannot substitute for** (§6).
Option A is additionally blocked on the §1.1 broad-vs-narrow Open Question.

**Instrument:** CCP Amendment. **Closes:** open-debt item **#21** — the one self-declared gap
inside ACS-005 itself: §9.1 demands a `GL-nnn` entry for "Data Plane" that §7 never issues.

**Unlike CCP-008, this draft does NOT recommend a single fix as settled.** The two candidate fixes
differ in *kind* (one is byte-affecting, one is byte-clean); choosing between them is a normative
ruling reserved to the maintainer. This draft presents both, proves the byte consequence of each
with an oracle, and states a recommendation.

## 1. The defect (quoted, not paraphrased)

ACS-005 (`standard/acs/ACS-005_Normative_Language.md`) contains an internal tension between §9.1
and §7.

**§9.1** (the term list the checker enforces, normative) says:

> The checker SHALL require a glossary entry for **each** of the following capitalized normative
> terms; a document that uses any of them without a resolvable `GL-nnn` entry SHALL be reported
> non-conformant:
>
> ```
> Capability, Cognitive Entity, Cognitive Truth, Commit, Conformance,
> Content Address, Control Plane, Data Plane, Decision Trace, Engine,
> Kernel, Replay, Shard, Tenant
> ```

**§7** (the glossary) closes at `GL-014` and resolves "Data Plane" only *inline*, in its closing
note:

> The glossary is deliberately closed at the 14 load-bearing terms named by R-06. Adding a term
> uses the same instruments as any other change (CCP / Amendment / IDR) and a new `GL-nnn`; a term
> MUST NOT be redefined by silent edit. `Data Plane` and `Tenant`/`Workspace` appear as capitalized
> normative terms in the checker list (§9) and resolve to GL-010's owning plane and GL-004
> respectively; `Data Plane` is defined inline as the pure-execution plane (the
> Engine/Capability/Execution layers) that owns nothing persistent.

**§9.3** then makes the tension a conformance problem:

> A checker that … reports PASS for a document in which any §9.1 term lacks a glossary entry SHALL
> be non-conformant.

So: §9.1 requires a *resolvable `GL-nnn` entry* for "Data Plane"; §7 issues no `GL-nnn` for it
(there is no `GL-015`); and a strict §9.3 reading forbids a clean PASS while that holds. Note the
asymmetry with the other two special cases: §9.1's own parenthetical explicitly covers `Workspace`
(via `Tenant`/GL-004) and `CP` (via GL-007b) —

> (14 terms; `Workspace` resolves via `Tenant`/GL-004 and `CP` is the disambiguation alias GL-007b,
> both intentionally covered by their parent entries.)

— but "Data Plane" gets **no such §9.1 coverage clause**; only §7's inline note gestures at
GL-010's "owning plane". The reference checker
(`verification/independent/python/acs005_checker.py`, §9.3 glossary-resolution lint) therefore
honestly reports the corpus as **PASS-GATED**, with "Data Plane" the single GATED term, and refuses
both an unconditional PASS and a hard FAIL while the gap stands. The gap is real, self-declared by
the standard, and currently permanent unless a CCP resolves it.

`gen_ccp010_vector.py` makes both candidate resolutions concrete (run it):

```
  CURRENT term-set (GL-001..GL-014)
    check_term_set     : ACCEPTS
    ContentId          : 1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074
    matches golden     : True
  OPTION-A term-set (GL-001..GL-015)
    check_term_set     : ACCEPTS (well-formed)
    ContentId          : 12204668482a15bd962650173dafd05b0d813bf8cf46fd1369709538facd3bef46aa
    differs from golden: YES -> BYTE-AFFECTING (ACS-005/2 profile bump)
  §9.3 GLOSSARY-RESOLUTION LINT
    current verdict            : PASS-GATED (gated: Data Plane)
    simulated Option-A verdict : PASS
```

### 1.1 A second, corpus-level defect: broad vs narrow "Data Plane" (OPEN QUESTION — blocking for Option A)

The §7 inline note is **not** the only frozen definition of "Data Plane", and it does not agree
with the older one. Frozen Vol 9 v2
(`spec-markdown/ARVES_Volume_9_Cognitive_Control_Plane_v2.md`, Part 2) defines the plane broadly,
verbatim:

> Data Plane (carries): Reality, Information Platform, Kernel, Living Cognitive World,
> Persistence, Query, Engine Fabric (engines as pure compute), Capability Fabric, Execution,
> mechanical Runtime (event bus, task, workflow).

Under Vol 9's definition the **Kernel** (the sole truth owner per ORCH-001) and **Persistence**
*are* Data Plane members — the plane "carries information and state" (Vol 9 Part 2). ACS-005 §7's
inline note instead defines a **narrow** plane: "the pure-execution plane (the
Engine/Capability/Execution layers) that owns nothing persistent" — excluding Kernel, Persistence,
Query, LCW and the Information Platform. The §7 note is even internally torn within one sentence:
"Data Plane" is said to "resolve to GL-010's owning plane" (which, per Vol 9 Part 3, is the broad
carrying plane in which engines sit) and is then *defined* as the narrow pure-execution plane.
What Vol 9 Part 3 actually supports is only the **membership** direction — "The engine ITSELF
(Engine Fabric, M7) is Data Plane … Likewise Capability Fabric is Data Plane" — i.e.
Engine/Capability are *in* the plane; it nowhere says the plane comprises *only* them.

**OPEN QUESTION (the maintainer MUST rule this BEFORE Option A can be ratified):** which meaning
does `GL-015` carry?

- **Broad** (Vol 9 v2 Part 2): the carrying plane, including Kernel and Persistence.
- **Narrow** (ACS-005 §7 inline note): the pure-execution plane that owns nothing persistent.

Promoting the **narrow** definition to a first-class, citable `GL-015` is a **MEANING-AFFECTING
choice at corpus level, not a mere promotion of expression**: frozen Vol 9 v2's own capitalized
"Data Plane" would thereafter resolve to a definition that excludes half of its listed members,
yielding "the Data Plane MUST NOT own truth or persistent state" alongside "the Kernel is Data
Plane" (Vol 9 Part 2) and "only the Kernel MAY own cognitive truth" (ORCH-001). Promoting the
**broad** definition instead overrides ACS-005 §7's own inline text. Neither choice differs at the
§8 address layer (only Term IDs are hashed — see §2), but they are **different standards**. An
earlier revision of this draft claimed the candidate row was "meaning unchanged, expression
promoted"; that claim was false at corpus level and is withdrawn.

## 2. Option A — issue `GL-015 Data Plane` (byte-affecting: ACS-005/2 profile bump)

Follow §7's own change-instrument rule ("Adding a term uses the same instruments as any other
change (CCP / Amendment / IDR) and a new `GL-nnn`") and add a first-class glossary entry. Because
of the §1.1 conflict this draft presents **two** candidate rows — one per meaning — and claims
neither is "meaning unchanged": ratifying the narrow row rules ACS-005 §7's note *over* Vol 9
Part 2; ratifying the broad row rules Vol 9 Part 2 *over* the note. Both rows produce the
**identical** §8 term-set body (`GL-001..GL-015` — the Term-Set Address hashes Term IDs only, not
definition text), so the byte consequence below holds for either.

**Candidate A-narrow** (source: ACS-005 §7 closing note):

| Term ID | Term | Definition (necessary/sufficient) | Grounded in |
|---|---|---|---|
| `GL-015` | **Data Plane** | The pure-execution plane — the Engine/Capability/Execution layers — that owns nothing persistent. *Necessary:* comprises only stateless execution layers (GL-010 Engine, GL-013 Capability); MUST NOT own Cognitive Truth (GL-002) or any persistent state; its outputs are inference/proposed effects, never a Commit (GL-005). *Sufficient:* being the plane that hosts Engine (GL-010) and Capability (GL-013) invocations under `ORCH-004-R1` idempotency. | ACS-005 §7 closing note (inline definition, verbatim source). **Conflicts with frozen Vol 9 v2 Part 2 (§1.1)** — ratifying this row is a meaning-affecting ruling against the broad definition, not an expression promotion. |

**Candidate A-broad** (source: Vol 9 v2 Part 2, verbatim list):

| Term ID | Term | Definition (necessary/sufficient) | Grounded in |
|---|---|---|---|
| `GL-015` | **Data Plane** | The carrying plane of the two-plane model: Reality, Information Platform, Kernel, Living Cognitive World, Persistence, Query, Engine Fabric (engines as pure compute), Capability Fabric, Execution, and the mechanical Runtime. *Necessary:* carries information and state while the Control Plane decides (Vol 9 Part 2); within it only the Kernel MAY own Cognitive Truth (ORCH-001). *Sufficient:* membership in the Vol 9 Part 2 carrier list. | Vol 9 v2 Part 2 (verbatim list); Vol 9 Part 3 (engine/capability membership); ORCH-001. **Supersedes ACS-005 §7's narrow inline note (§1.1)** — equally meaning-affecting. |

**Byte consequence (oracle-proven).** The §8 Term-Set Address is defined over "the glossary Term
IDs `GL-001`…`GL-014`, sorted ascending, joined by a single `\n` … under tag `0x08`", and §9.2
vector **1** pins those bytes. Adding GL-015 changes that body to `GL-001\n…\nGL-015`:

| | body | ContentId |
|---|---|---|
| current golden (§9.2 v1 / `acs_golden_vectors.tsv` row `ACS-005 term-set`) | `GL-001..GL-014` | `1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074` |
| Option-A candidate (body hex `474c…3135`, see oracle output) | `GL-001..GL-015` | `12204668482a15bd962650173dafd05b0d813bf8cf46fd1369709538facd3bef46aa` |

A golden ContentId changes, so this is a **profile bump — ACS-005/2** — routed through CCP-GATE,
never a silent edit (the same discipline CCP-008 §5 records for the hypothetical absent-canonical
ruling). Scope of the bump is bounded, and the oracle proves the bound:

- §9.2 vector **1** (term-set) — REPLACED with the GL-015 body + ContentId above;
- §8 **Term-Set Address definition** — the normative sentence "the glossary Term IDs
  `GL-001`…`GL-014`, sorted ascending, joined by a single `\n` … under tag `0x08`" MUST be
  amended to `GL-001`…`GL-015` **in the same atomic amendment**; left unamended, §8 would still
  normatively define the address over fourteen IDs while the new §9.2 vector 1 pins fifteen — a
  self-contradicting standard;
- §9.2 vector **2** (requirement clause) — UNCHANGED;
- §9.2 vector **3** (term-name list) — **oracle-anchored, unchanged by construction**: Option A
  does not edit the §9.1 name list ("Data Plane" is already in it), so vector 3's bytes cannot
  move; the oracle recomputes the frozen anchor and confirms it matches the golden (this is an
  anchor check on the untouched body, not a computation over an Option-A state);
- §7 prose — the closing note's "deliberately closed at the 14 load-bearing terms" and its
  inline-resolution sentence are superseded for "Data Plane" (the Tenant/Workspace half stays);
- §9.3 lint — flips from PASS-GATED to clean PASS (oracle-simulated).

## 3. Option B — amend §9.1's resolution wording (byte-clean)

Keep the glossary closed at GL-014 and instead extend §9.1's existing parenthetical coverage
mechanism (the one already used for `Workspace` and `CP`) so that "Data Plane" *resolves* via the
§7 inline definition. Candidate amendment to the §9.1 parenthetical:

> (14 terms; `Workspace` resolves via `Tenant`/GL-004, `CP` is the disambiguation alias GL-007b,
> and `Data Plane` resolves via the §7 inline definition anchored to GL-010's owning plane — all
> three intentionally covered without a dedicated entry.)

**Byte consequence.** None. The §9.1 *list* itself is unchanged (the fourteen names, and thus §9.2
vector 3's bytes, stay identical); no Term ID is added (vector 1 stays `GL-001..GL-014`); vector 2
is untouched. The checkers change verdict semantics only: "Data Plane" moves from GATED to an
ALIAS-class resolution, and PASS-GATED becomes PASS with zero golden edits.

**Cost.** "Data Plane" stays a second-class term forever: it has no stable `GL-nnn`, so no
document, probe, or amendment can *cite* its definition the way §5's stability rule intends — and
citability is the entire point of ACS-005 (§10: "A standard is only as strong as its ability to be
cited and checked word-for-word"). It also converts §7's "deliberately closed at the 14" from a
snapshot into a load-bearing permanent claim, and it stretches the §9.1 alias mechanism beyond its
stated design (Workspace and CP are covered *by parent entries* — GL-004 and GL-007b are real
glossary rows; an inline prose note is not a parent entry, so Option B's "alias" is weaker in kind
than the two precedents it imitates).

## 4. Trade-off table (honest)

| Axis | Option A — GL-015 | Option B — §9.1 wording |
|---|---|---|
| Golden vector bytes | **Changes vector 1** (term-set) → ACS-005/2 profile bump | **Zero byte changes** (byte-clean) |
| Blast radius | §1.1 broad-vs-narrow ruling (meaning-affecting, blocking) + frozen ACS-005 §7 + **§8 Term-Set Address sentence** + §9.2 table + TSV row + **`acs_golden_vectors.md` mirror** + both reference checkers' `GOLDEN`/term-map + **TypeScript arm `vectors.mjs` (14-loop + golden)** + **paired RCR (v1.1) for frozen `runtime/` pins: `arves-acs/tests/acs_platform.rs`, `arves-conformance/src/acs.rs`** + re-cert of 2 runtimes + evidence probe regen | §9.1 parenthetical + both checkers' lint tables (verdict semantics only) |
| Citability of "Data Plane" | First-class, stable `GL-015`, addressable per §5 stability rule | Permanently uncitable as a `GL-nnn`; inline prose only |
| Fidelity to ACS-005's own rules | Follows §7's explicit change-instrument rule verbatim ("a new `GL-nnn`") | Follows the §9.1 alias precedent, but weaker in kind (no parent *entry* exists) |
| §9.3 strict reading | Fully satisfied (every §9.1 term has a `GL-nnn`) | Satisfied only via the amended wording; the strict "lacks a glossary entry" clause must itself be re-read |
| Reference checker today | Already names this as "the ACTUAL fix … a first-class GL-015" (`acs005_checker.py` docstring + lint rationale) | Would require rewriting that self-documented expectation |
| Risk to differential conformance | A second runtime certified against v1 goldens FAILs vector 1 until it adopts /2 — versioning must be explicit | None |
| Precedent set | Profile bumps are routine, governed, and survivable | Inline definitions are an acceptable terminal state for normative terms |

## 5. Recommendation

**Recommend Option A (GL-015, ACS-005/2 profile bump)** — while stating plainly that the byte cost
is real, that the choice is the maintainer's, not this draft's, and that **Option A cannot be
ruled at all until the §1.1 broad-vs-narrow Open Question is ruled first** (the GL-015 row text
depends on it, and either ruling is meaning-affecting).

Reasons: (1) §7 itself prescribes the instrument — "Adding a term uses the same instruments as any
other change (CCP / Amendment / IDR) and a new `GL-nnn`" — so Option A is the standard obeying its
own law, whereas Option B is the standard re-reading its law to avoid a cost; (2) the reference
checker already records GL-015 as the intended fix and deliberately withholds clean PASS until it
lands — Option B would spend that honesty rather than redeem it; (3) the bump is confined to
a single vector row (vector 2 is untouched; vector 3 is unchanged by construction, since the §9.1
name list is not edited — its anchor recomputed and matching), and exercising the profile-bump machinery on a minimal case *before*
G2 is itself validation-era evidence that versioning works; (4) ACS-005's stated purpose is
word-for-word citability, which Option B permanently denies to one of its own fourteen load-bearing
terms. Option B remains the recorded fallback if the maintainer rules that pre-G2 golden-vector
stability outweighs all four.

## 6. Ratification (what actually touches frozen — GATED on the maintainer)

If **Option A** is ruled — which first requires ruling the **§1.1 broad-vs-narrow Open Question**
(the ruled meaning determines which GL-015 row text is added) — the work spans **two instruments**:
one atomic CCP amendment (CCP-GATE, frozen `standard/`) **plus one paired Runtime Change Request
(v1.1, per `runtime/RUNTIME_FREEZE_v1.0.md`, frozen `runtime/`)**. CCP-GATE alone cannot authorize
the `runtime/` edits.

*CCP amendment (frozen `standard/`):* (a) add the ruled GL-015 row to ACS-005 §7 and adjust the
closing note (14 → 15; remove the "Data Plane … inline" clause, keep Tenant/Workspace); (b) amend
the **§8 Term-Set Address sentence** from "`GL-001`…`GL-014`" to "`GL-001`…`GL-015`" and replace
§9.2 vector 1's body/pre-image/ContentId with the GL-015 values above, marking the profile
ACS-005/2 — both in the same atomic amendment; (c) update `standard/vectors/acs_golden_vectors.tsv`
row `ACS-005 term-set` **and** the human-readable mirror `standard/vectors/acs_golden_vectors.md`
(the `ACS-005-CS-1 term-set` row).

*Living checkers (`verification/`):* (d) Python arm — `acs005_checker.py` GOLDEN +
`_GLOSSARY_TERM_TO_GL` (add `Data Plane → GL-015`) + retire the `_KNOWN_GATED` entry, and
`acs_values.acs005_term_set_body` (range 1..16); TypeScript differential arm —
`verification/independent/typescript/src/vectors.mjs` (the `i <= 14` term-ID loop and its pinned
golden). The Rust semantic arm (`runtime/crates/arves-conformance/src/semantic.rs`) needs **no
change**: its `check_term_set` is grammar/structure-only (no 14-pin, no `_KNOWN_GATED`, no
`_GLOSSARY_TERM_TO_GL`, no §9.3 glossary-resolution lint), so there is no Rust "likewise" to the
Python lint retirement.

*Paired RCR (frozen `runtime/`, v1.1):* (e) update the pinned v1 goldens in
`runtime/crates/arves-acs/tests/acs_platform.rs` (the `(1..=14)` term-ID builder + asserted
ContentId `1220ced3…2074`; its 14-name-list vector is untouched, per §2) and
`runtime/crates/arves-conformance/src/acs.rs` (the `ACS-005 term-set` golden row).

*Closure:* (f) re-run `freeze_check.py update` + re-certify both runtimes + regenerate the
evidence probe.
If **Option B** is ruled: (a) amend the §9.1 parenthetical as drafted in §3; (b) move "Data Plane"
from GATED to ALIAS in both checkers; (c) `freeze_check.py update`; no vector, TSV, or
re-certification work. Each step is a frozen edit requiring explicit maintainer authorization.

## 7. Honesty

- **DRAFT, freeze-clean.** No frozen byte is touched; the oracle simulates Option A in memory only.
- **A prior honesty claim is withdrawn.** An earlier revision asserted the GL-015 candidate was
  "meaning unchanged, expression promoted" from §7's inline note. A corpus survey (§1.1) shows the
  frozen corpus holds **two conflicting definitions** of "Data Plane" (Vol 9 v2 Part 2 broad vs
  ACS-005 §7 narrow); promoting *either* to a citable GL-015 is meaning-affecting at corpus level.
  The conflict is recorded as a blocking Open Question, not resolved here.
- **Oracle proof strength, stated precisely.** Vector 1's ContentId change and the §9.3 verdict
  flip are computed (oracle-proven). Vector 3's invariance is **by construction** (Option A does
  not edit the §9.1 name list); the oracle only recomputes the untouched frozen anchor — it does
  not compute over an Option-A state.
- **The defect is self-declared, not discovered here:** ACS-005 §7's closing note names the inline
  resolution, and the reference checker has surfaced it as PASS-GATED since it was written. This
  CCP converts a known-gated exception into a ruled outcome; it does not manufacture a gap.
- **The recommendation is not a decision.** Byte-affecting vs byte-clean is a normative trade the
  Constitution reserves to Change Management; ratification (and the choice between A and B) is
  maintainer-gated at CCP-GATE.
- **In-program (G1).** Oracle and draft were authored in this program; closing #21 strengthens
  self-consistency of ACS-005, it does not constitute external (G2) validation.

# CONTRACT-ONLY Crate Fidelity Audit

**Status:** READ-ONLY audit — NOTHING in `runtime/`, `standard/`, or `spec-markdown/` was edited.
**Scope:** the 11 CONTRACT-ONLY runtime crates (trait/type skeletons declared "CONTRACT-ONLY" or "interfaces only").
**Question:** does each crate *faithfully* and *honestly* represent its FROZEN contract, so a G2 party reading `runtime/` as the reference implementation is **not misled**?
**Method:** per-crate read of `src/lib.rs` + `Cargo.toml` against the frozen corpus (`spec-markdown/`), cross-checked against the companion `ARVES_00_Invariant_Registry_v1.md` and `RUNTIME_FREEZE_v1.0.md`. Load-bearing findings re-verified directly against the frozen sources (IDR Batch 1 numbering; Registry CAP-/ENG- statements; the `arves-invariants` `Layer` enum).
**Freeze posture:** freeze gate = **0 drift** (this audit changed no frozen file). Every proposed correction below routes through a **Runtime Change Request (RCR)**, gated on the maintainer. Runtime status remains **G1**; this audit is an input to G2 readiness, not a certification.
**Auditor:** ARVES Chief Architect (Independent Architecture Review posture — reviewed as if another party submitted the code).

---

## 1. Honest Summary

**The skeletons are honestly labelled and structurally faithful — but four crates carry real invariant-/IDR-provenance drift that would mislead a G2 party doing ID-based traceability.** Every one of the 11 crates carries a truthful STATUS block: each declares CONTRACT-ONLY (or, for the two genuinely-runnable ones, IMPLEMENTED) with no overclaim, `Cargo.toml` is `version 0.0.0`/`publish=false`/std-only, and no placeholder body is dressed up as working logic. So there is **no honesty failure** — the labels tell the truth. The *engineering substance* — the traits, types, names, signatures and the mechanisms they model (CP truth on per-shard Raft, replicate-outcomes-not-invocations, proposed-writes-only-the-Kernel-commits, cooperative-idempotent cancellation, saga compensation, single-owner Working Memory, read-only projections, the ontology aspect surface) — is a clean, correct mirror of the frozen decisions in 7 of 11 crates and is never semantically contradicted in any crate. **The defect is traceability, not mechanism.** Four crates re-author or mis-number the *identifiers* they cite: `arves-consensus` inverts the frozen IDR-003/004/005 numbering and drops IDR-005; `arves-engine-fabric` and `arves-capability-fabric` re-author the numbered ENG-/CAP- statements so the same IDs mean different things than the frozen Invariant Registry (the sole frozen enumerator of those IDs) says; and `arves-invariants` — the crate *billed as the canonical LAYER-001 reference* — encodes a `Layer` ordering (`Query` above `Engine`) whose `may_depend_on` helper returns the wrong verdict for the frozen "Engine Reads: State via Query" edge. Because these are ID-level citation defects (and, in three of the four, the drifting IDs are explicitly marked *proposed / non-binding*), **no frozen normative contract is violated** — but a certifier chasing those specific citations would be misled, which is exactly the risk this audit exists to catch. Recommended remediation is per-crate (via RCR, never the frozen spec): align the citation IDs to the frozen record or drop the numeric labels and cite only the registered-normative invariants each crate actually upholds.

**Verdict tally:** 7 / 11 faithful (structure + citations both correct), 4 / 11 with confirmed material drift (`arves-consensus`, `arves-engine-fabric`, `arves-capability-fabric`, `arves-invariants`). Confirmed drift/observation findings appear in 7 crates total (the 4 above plus 3 faithful crates carrying minor doc-completeness notes). **9 confirmed drift findings** are severe enough to propose a correction (3 major-severity, 6 minor-severity); the remaining items are documentation-completeness observations that would not mislead a G2 reader.

---

## 2. Per-Crate Table

| Crate | Governing frozen contract | Fidelity | Honest label? | G2-misleading? | # drift findings |
|---|---|---|---|---|---|
| `arves-consensus` | IDR Batch 1 (IDR-001..005 Kernel Distribution); SHARD-001; ORCH-001/003/004; OWN-001 | **PARTIAL** | Yes | **Yes** | 3 (major) |
| `arves-control-plane` | ORCH-001..004 (Vol 9 Part 5); Amendment-002 Plan Ownership; Vol 9 Part 6 Engine Graph | Faithful | Yes | No | 2 (minor) |
| `arves-engine-fabric` | Engine Graph Spec (Parts 3/4/6/7); ORCH-001..004; Registry ENG-001..005 | **PARTIAL** | Yes | **Yes** | 4 (1 major, 3 minor) |
| `arves-execution` | Vol 9 Parts 2/5; Amendments A-003/005/006; IDR-001/002/003; Engine Graph; SHARD-001 | Faithful | Yes | No | 0 |
| `arves-capability-fabric` | Vol 9 Part 3/5 (Fabric = Data Plane); ORCH-001/002/004; Registry CAP-001..009 | **PARTIAL** | Yes | **Yes** | 2 (1 major, 1 minor) |
| `arves-query` | QUERY-001 (proposed); Layer Matrix Query row; IDR-001 read tiers | Faithful | Yes | No | 0 |
| `arves-lcw` | Amendment-001 Working Memory Ownership; OWN-001/ORCH-001; LCW-001 (proposed); Layer Matrix LCW row | Faithful | Yes | No | 0 |
| `arves-ontology` | Ontology Spec (Parts 3/4/6/9); ACS-004 Universal Type Registry (ratified); O-001..007 | Faithful | Yes | No | 2 (minor, doc-completeness) |
| `arves-information-platform` | Vol 3 Information Core Bible; Amendment-003 Layer Matrix; Ontology Part 4 aspects | Faithful | Yes | No | 0 |
| `arves-runtime` | Vol 9 Part 5 ORCH-001..004; LAYER-001/OWN-001 (composition/wiring contract) | Faithful | Yes | No | 1 (minor, peripheral README) |
| `arves-invariants` | ARVES_00_Invariant_Registry_v1; Amendment-003 Layer Matrix; Vol 9 Part 5 | **PARTIAL** | Yes | **Yes** | 2 (minor) |

> Note: `arves-runtime` is not a trait/type skeleton — it is a runnable wiring **binary** (`main.rs`, deps = kernel + persistence). It is included because the task named it among the 11; per `RUNTIME_FREEZE_v1.0.md` finding #4 / RCR-001 it is classified IMPLEMENTED, and its label is honest. Its single finding is a peripheral doc, not a contract skeleton defect.

**Faithful (structure + citations correct):** 7 — `control-plane`, `execution`, `query`, `lcw`, `ontology`, `information-platform`, `runtime`.
**With confirmed material drift (partial, G2-misleading):** 4 — `consensus`, `engine-fabric`, `capability-fabric`, `invariants`.

---

## 3. Confirmed Drift / Overclaim Findings

Every finding below was verified against the cited FROZEN source. None is an honesty failure (all STATUS labels are truthful); all are **traceability / citation / encoding drift** unless noted.

### F-1 — `arves-consensus`: IDR-003 mislabelled (Storage/WAL concept tagged as IDR-003) · **MAJOR**
The crate attributes "the Raft log **is** the WAL **is** the decision trace" to **IDR-003**. In the frozen spec this convergence sentence is **IDR-001's Engineering Refinement** (`IDR_Batch_1` line 29: "IDR-001 + IDR-005 + ORCH-003 converge…"), and the WAL / single-source-for-replay decision itself is **IDR-005** (lines 69–71). Frozen **IDR-003 is the Membership Strategy** (Joint Consensus, lines 61–63). Sibling `arves-persistence` cites IDR-005 for exactly this concept, confirming `arves-consensus` is the outlier, not a repo convention.
**Frozen source:** `ARVES_IDR_Batch_1_Kernel_Distribution_v1.md` IDR-005 (69–71), IDR-001 refinement (29), IDR-003 (61–63).
**Where:** `runtime/crates/arves-consensus/src/lib.rs` lines 30, 101, 109, 115, 119, 171, 179, 184.

### F-2 — `arves-consensus`: IDR-004 mislabelled (Membership tagged as IDR-004; conflated with Leader Election) · **MAJOR**
The crate tags joint-consensus **membership** changes as **IDR-004**. Frozen **IDR-004 is Leader Election** ("one leader per shard", lines 65–67); **Membership / Joint Consensus is IDR-003** (lines 61–63). The crate both mislabels membership as IDR-004 and elsewhere (correctly) labels leader election as IDR-004, conflating two distinct frozen IDRs under one number.
**Frozen source:** `ARVES_IDR_Batch_1_Kernel_Distribution_v1.md` IDR-003 Membership (61–63) vs IDR-004 Leader Election (65–67).
**Where:** `runtime/crates/arves-consensus/src/lib.rs` lines 35, 38, 193, 196, 249, 254, 261, 263, 273.

### F-3 — `arves-consensus`: IDR-005 never cited / IDR scope understated as "IDR-001..004" · **MAJOR**
The header (line 4) and core-contract heading (line 340) declare the crate is governed by **"IDR-001..004"**; **IDR-005 appears nowhere**, yet the crate directly models the storage/WAL semantics the frozen spec assigns to IDR-005. A G2 party reading this crate as reference would (a) conclude only four IDRs govern the kernel-distribution substrate, and (b) invert the meaning of IDR-003/004/005 relative to the frozen record.
**Frozen source:** `ARVES_IDR_Batch_1_Kernel_Distribution_v1.md` IDR-001..005 (13–71); IDR-005 Storage (69–71).
**Where:** `runtime/crates/arves-consensus/src/lib.rs` lines 4, 21–38, 340, 363.

### F-4 — `arves-engine-fabric`: ENG-001..005 label collision with the frozen Invariant Registry · **MAJOR**
The crate's docstrings assign ENG-001..005 meanings that do **not** match the canonical statements in the companion frozen Registry. Crate ENG-003 = "writes are proposals" vs Registry ENG-003 = "idempotent + content-addressable, carries idempotency key"; crate ENG-004 = "declared reads/produces" vs Registry ENG-004 = "nondeterministic engines replayed from trace, not recomputed"; crate ENG-005 = "declared determinism + required capabilities" vs Registry ENG-005 = "semantically versioned, content-addressable manifest". Crate ENG-002 collapses into Registry ENG-001, and the idempotency semantics the Registry files under ENG-003 are tagged only ORCH-004 in the crate. Mitigated (not resolved) by the crate consistently tagging all as "(proposed)".
**Frozen source:** `ARVES_00_Invariant_Registry_v1.md` Part 4, lines 56–60 (ENG-001..005 canonical statements) — verified verbatim.
**Where:** `runtime/crates/arves-engine-fabric/src/lib.rs:32-46` and repeated in doc comments.

### F-5 — `arves-engine-fabric`: normative Writes vs Produces manifest fields merged · **MINOR**
Engine Graph Spec Part 3 defines **Writes** (proposed state effects the Kernel commits) and **Produces** (inference/ontology artifacts emitted as output) as two distinct normative manifest fields. The crate's `EngineManifest.produces` maps to the spec's **Writes** (documented as "shapes of ProposedEffects"), and there is **no** manifest field for the spec's **Produces** (the produced artifact appears only at runtime as `Inference.output`, undeclared). Renames/merges a normative field pair.
**Frozen source:** `ARVES_Engine_Graph_Specification_v1.md` Part 3 (rows "Writes" and "Produces").
**Where:** `runtime/crates/arves-engine-fabric/src/lib.rs:180-215`.

### F-6 — `arves-engine-fabric`: several normative Part-3 manifest fields absent · **MINOR**
`EngineManifest` omits Preconditions, Failure Policy, Retry Policy, Timeout, Confidence, Cost, Latency (Inputs is the `Engine::Input` associated type, not a manifest field). Acceptable scope-narrowing for a skeleton and non-contradictory, but a G2 reader treating this crate as the full manifest ABI reference finds only a partial subset.
**Frozen source:** `ARVES_Engine_Graph_Specification_v1.md` Part 3 (Execution + Planning-metadata groups); Parts 8, 10.
**Where:** `runtime/crates/arves-engine-fabric/src/lib.rs:159-190`.

### F-7 — `arves-engine-fabric`: `PureEngine` "real cognitive work chain" vs spec-violating default key · **MINOR**
`PureEngine` is described as "the first executable Engine, used to run the real cognitive work chain" (lines 297–298), yet its `invoke()` hardcodes `Inference.key = IdempotencyKey::default()` (empty), contradicting the crate's own ABI requirement (line 283) that `Inference::key` MUST match the invocation's idempotency key. In a CONTRACT-ONLY crate this is a placeholder body, but the "real … work chain" wording plus the spec-violating default is an internal inconsistency that could read as a conformant reference invocation.
**Frozen source:** `ARVES_Engine_Graph_Specification_v1.md` Part 3/Part 7 (Idempotency Key, ORCH-004); crate self-contract at `lib.rs:283`.
**Where:** `runtime/crates/arves-engine-fabric/src/lib.rs:292-336`.

### F-8 — `arves-capability-fabric`: CAP-001..009 statements re-authored vs the frozen Registry · **MAJOR**
The crate re-authors the numbered CAP-001..009 statements so they no longer match the frozen Registry (the sole frozen source that enumerates each CAP-00n; every other frozen mention treats CAP-001..009 as an opaque range). Divergences: crate CAP-002 = "one active provider per shard" vs frozen "selection is a Control Plane concern; the Fabric never selects"; CAP-003 = "bindings are versioned/supersession" vs frozen "every invocation is idempotent + content-addressable"; CAP-005 = "resolution is a side-effect-free read" vs frozen "declared as manifest requirements, bound at execution time"; CAP-006 = "partitioned by shard key" vs frozen "side-effect-honest: external effects declared/recorded"; CAP-007 = "the fabric holds no truth" vs frozen "carries a correlation_id, recorded in the trace"; CAP-008 = "the fabric produces no plans" vs frozen "every invocation is cancellable". Only CAP-001 and CAP-009 are roughly on-topic. A G2 certifier doing ID-based traceability would find these IDs silently mean different things. **Mitigated** (not resolved): the crate loudly and repeatedly marks CAP-001..009 as **proposed / informative / non-binding / pending CCP-GATE**, matching the Registry's own non-normative standing, so no *normative* contract is violated.
**Frozen source:** `ARVES_00_Invariant_Registry_v1.md` Part 4, lines 47–55 (CAP-001..009) — verified verbatim.
**Where:** `runtime/crates/arves-capability-fabric/src/lib.rs` lines 45–57 and the CAP-00n tags at 68–72, 82–90, 100–105, 117–128, 138–146, 162–174, 189–207, 222–258.

### F-9 — `arves-invariants`: `Layer` ordering puts Query above Engine, so `may_depend_on` inverts the frozen "Engine Reads: State via Query" edge · **MINOR (but the crate is the billed LAYER-001 reference)**
The `Layer` enum sets `Query = 5` strictly **above** `Engine = 6`, and `may_depend_on(self, other)` returns `true` iff `self.rank() < other.rank()`. The frozen Layer Responsibility Matrix (Amendment-003, quoted as LAYER-001) has the Engine row **"Reads: State via Query"** — i.e. Engine legally depends **downward** on Query, so Query must be strictly *below* Engine. As encoded, `Engine.may_depend_on(Query)` = `6<5` = **false** (frozen makes it legal) and `Query.may_depend_on(Engine)` = `5<6` = **true** (illegal — Query "Reads: Kernel, LCW, Persistence" only, "Writes: NOTHING"). Because the crate is billed (`lib.rs` 5, 14, 26–27, 125–131) as the single source of invariant identifiers and the LAYER-001 reference, a G2 party citing `Layer::may_depend_on` as reference truth is misled for the Query/Engine pair. Blast radius is limited: the helper is standalone and is **not** consumed by the actual enforced gate.
**Frozen source:** `ARVES_00_Amendments_CCP_Batch_1_v1.md` Amendment-003 Layer Matrix (Engine "Reads: State via Query"; Query "Reads: Kernel, LCW, Persistence" / "Writes: NOTHING"); LAYER-001 in `ARVES_00_Invariant_Registry_v1.md` Part 2. **Verified directly in the crate source** (`Layer` enum + `may_depend_on`).
**Where:** `runtime/crates/arves-invariants/src/lib.rs:282-331`.

### F-10 — `arves-invariants`: reference `Layer` ordering is not reconciled with the runtime's own enforced gate · **MINOR**
`arves-conformance/tests/architecture_gate.rs::layer_rank` ranks `arves-query` = 60 and `arves-engine-fabric` = 60 (**equal / peers**, comment "query/engine"), whereas `arves-invariants::Layer` makes them strictly ordered. The reference helper is not consumed by the enforcer (the gate has its own crate-name-keyed rank fn), which limits blast radius but evidences that the `arves-invariants` ordering was never reconciled against the enforcer. A reader treating `arves-invariants` as the canonical LAYER-001 encoding gets an ordering the runtime itself does not enforce.
**Frozen source:** `ARVES_00_Amendments_CCP_Batch_1_v1.md` Amendment-003 (peer layers / no lateral calls).
**Where:** `runtime/crates/arves-invariants/src/lib.rs:294-300` vs `runtime/crates/arves-conformance/tests/architecture_gate.rs:26-45`.

### Minor citation/doc-completeness items in otherwise-faithful crates (not G2-misleading; listed for completeness)
- **`arves-control-plane` C-1/C-2 (MINOR):** doc-comment IDR cross-references are misnumbered — "engines commit only via the shard leader (IDR-003)", "Raft log = WAL = decision trace (IDR-004)", "no cross-shard atomic commit / saga (IDR-005)" — all of which are actually **IDR-001 Engineering Refinements** (lines 27/29/31). IDRs are non-normative and are **not** this crate's governing contract (ORCH-001..004 + Amendment-002 are), so a G2 party is not misled about the actual contract. Also a modeling omission: `Orchestrator::expand` / `EngineGraph` carry no termination/budget parameter for the bounded graph-expansion policy required by Vol 9 Part 6/7. Under-modeled in a skeleton, not a false claim. *(`lib.rs` 40, 82–83, 118, 137, 162, 353, 392, 405, 416, 418, 454, 493, 509–515.)*
- **`arves-ontology` O-1/O-2 (MINOR):** `RootType` exposes 8 composed categories where Ontology Spec Part 6 lists 18 URNs, and the Trust/Provenance aspect field sets are narrower than Part 4 prose. **Both are affirmatively resolved by the ratified normative amendment ACS-004** (§6.1 canonicalizes the 8-name RootType space; §8 binds the exact aspect fields), which even reproduces this crate's value types verbatim as "the contracts arves-ontology ships". The only gap is that the crate header doesn't *cite* ACS-004, so a reader of crate + Part 6 alone might momentarily wonder about 18→8. Documentation-completeness, not contradiction.
- **`arves-runtime` R-1 (MINOR):** the top-level `runtime/README.md:27` says "Status: I1 skeleton — interfaces only" and `:25` calls arves-runtime "wires the single-shard walking skeleton". "Interfaces only" is **stale/false** — arves-runtime is a runnable binary (verified: identical `truth_hash` pre/post recovery; 2 cross-process restart tests pass). This is the exact stale-header class RCR-001 fixed, but RCR-001's scope was crate `src/lib.rs` headers only, so the README was missed. **Understates** maturity (not an overclaim); the in-crate header and RUNTIME_FREEZE_v1.0.md are correct.

---

## 4. Proposed Runtime Change Requests (RCRs)

All corrections are to `runtime/` (crate source/docs) only — **never** the frozen `spec-markdown/`, which is correct as-is. Each RCR is **gated on the maintainer**; this audit applied none of them. Runtime remains **v1.0 FROZEN**; these are candidate v1.1 doc/label corrections (no ABI/type change except the internal `arves-invariants` `Layer` fix, which is a private-helper correction, not a public-contract change).

| RCR | Crate | Correction | Class |
|---|---|---|---|
| **RCR-CCA-01** | `arves-consensus` | Renumber IDR citations to the frozen record: WAL/log-is-decision-trace → **IDR-005 / IDR-001-refinement** (not IDR-003); membership/joint-consensus → **IDR-003** (not IDR-004); leader election → **IDR-004**; and change the header/heading from "IDR-001..004" to **"IDR-001..005"** so IDR-005 (Storage) is cited. Doc-comment + header only; no type/trait change. (F-1, F-2, F-3) | Doc/traceability |
| **RCR-CCA-02** | `arves-engine-fabric` | Either align the crate's ENG-001..005 docstrings to the frozen Registry statements (Part 4 lines 56–60), or drop the numeric ENG-00n labels and cite only the registered-normative invariants the crate upholds (ORCH-001/003/004). Keep the "(proposed)" tags. (F-4) | Doc/traceability |
| **RCR-CCA-03** | `arves-engine-fabric` | Restore the frozen Part-3 **Writes vs Produces** distinction: either add a declared `produces` (output-artifact) field distinct from proposed-effects, or rename the existing `produces` to `writes` and document the omission of Produces as an explicit scope note. (F-5) | Contract-surface |
| **RCR-CCA-04** | `arves-engine-fabric` | Add a header scope note enumerating the omitted normative manifest fields (Preconditions/Failure/Retry/Timeout/Confidence/Cost/Latency) as deferred, so the crate isn't read as the complete manifest ABI. (F-6) | Doc |
| **RCR-CCA-05** | `arves-engine-fabric` | Fix `PureEngine::invoke` to set `Inference.key` from the invocation's idempotency key (honoring the crate's own `lib.rs:283` contract), or reword away from "real cognitive work chain" and mark the body explicitly as a non-conformant placeholder. (F-7) | Code (placeholder) |
| **RCR-CCA-06** | `arves-capability-fabric` | Align the CAP-001..009 tags to the frozen Registry statements (Part 4 lines 47–55), **or** drop the numeric CAP-00n labels and cite only the registered-normative invariants the crate actually upholds (ORCH-001/002/004, OWN-001, LAYER-001, SHARD-001). Keep the "(proposed)" framing. (F-8) | Doc/traceability |
| **RCR-CCA-07** | `arves-invariants` | Correct the `Layer` ordering so `Query` is strictly **below** `Engine` (Engine legally reads Query per the frozen Matrix), **or** model Query/Engine as **peers** to match the enforced `architecture_gate.rs` rank (both 60). Reconcile the reference helper with the actual enforced gate so they cannot diverge. (F-9, F-10) | Code (internal helper) |
| **RCR-CCA-08** | `arves-control-plane` | Renumber the doc-comment IDR cross-references (commit-via-leader / log=WAL=trace / no-cross-shard-saga) to **IDR-001 Engineering Refinements**; add an optional termination/budget parameter (or a documented scope note) for bounded graph expansion per Vol 9 Part 6/7. (C-1, C-2) | Doc |
| **RCR-CCA-09** | `arves-ontology` | Add a one-line header citation to **ACS-004 §6.1 / §8** noting the 18→8 RootType reconciliation and that the aspect field sets are the ratified ACS-004 binding. Documentation nicety; no type change. (O-1, O-2) | Doc |
| **RCR-CCA-10** | `arves-runtime` | Extend RCR-001's stale-header fix to `runtime/README.md:27` (and `:25`): replace "I1 skeleton — interfaces only" with the accurate runnable-binary status matching main.rs and RUNTIME_FREEZE_v1.0.md. (R-1) | Doc |

---

## 5. Freeze & Governance Note (explicit)

- **This audit is READ-ONLY.** It edited **no** file in `runtime/`, `standard/`, or `spec-markdown/`. The **freeze gate remains at 0 drift** — nothing in the frozen corpus was touched, and the frozen IDR Batch 1, Invariant Registry, and Amendments were confirmed **correct as written** (all drift is in `runtime/` citations, never in the spec).
- **All corrections route through a Runtime Change Request (RCR)**, gated on the maintainer, per `RUNTIME_FREEZE_v1.0.md`. The audit proposes RCR-CCA-01..10; it applied none. A "runtime gap found during review is a Runtime Change Request, never a silent edit" (Constitution / Freeze Record).
- **Runtime status remains G1.** This audit is an input to G2 readiness (it hardens `runtime/` as a citable reference), not a certification verdict. G2 (third-party / external validation) is unchanged by this document.
- **No frozen normative contract is violated by any finding.** Every confirmed drift is ID-level traceability/citation drift or an internal-helper/doc encoding issue; three of the four material-drift crates mark the drifting IDs as *proposed / non-binding*, so no normative invariant is misrepresented as ratified.

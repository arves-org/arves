# ARVES — Academic Peer-Review (Prompt 2)

**Reviewing committee lens:** MIT Distributed Systems · Stanford AI · CMU Robotics · ETH Zurich Systems · Oxford Formal Methods
**Objective:** Determine what prevents ARVES from being published/accepted as a peer-reviewed *scientific reference architecture* (ISO/IEEE-grade), and how to fix each defect via the only permitted instruments (IDR / CCP Amendment / Runtime / Verification / Certification / Ecosystem / Product). **The frozen corpus is immutable; nothing here proposes editing it.**
**Date:** 2026-07-02 · **Reviewer:** Independent Chief-Architect / Academic Committee

---

## Executive Summary

ARVES is, as a *design*, unusually disciplined for a "cognitive infrastructure" effort: it separates a truth-owning Kernel from a truth-less Control Plane, it names invariants (ORCH-001..004, OWN-001, LAYER-001, SHARD-001), it defines a conformance framework that is honestly *property-based rather than golden-output* (Scenario Conformance Framework, Part 8), and it has a real change process (Reference Lifecycle). These are the right instincts and they are genuinely publishable *architecture*.

**But as a scientific artifact it is not yet defensible in front of a program committee.** Three classes of defect would each, on their own, trigger a *reject* at a top venue (SOSP/OSDI/POPL) or a *major-revision* at ISO/IEEE:

1. **Unfalsifiable and unsubstantiated foundational claims.** The Baseline (S1) claims the foundation delivered "Vision, Theory, **Mathematics, Formal Proof**, Validation." The Documentation Index v2.2 — the self-declared *complete* register — contains **no mathematics document, no formal-proof document, and no validation study**. A `grep` of the entire 50-document corpus returns zero theorems, lemmas, proofs, model-checks, or refinement mappings. The corpus's *own* Gap Analysis states it is "roughly 90% naming & structure, 10% content" and "NOT yet an implementable specification." A reviewer will read the "Formal Proof" claim as a category error and reject the paper on integrity grounds.

2. **Primitive terms are enumerated, never defined.** "Cognitive truth," "intelligence," "cognitive entity," "meaning," "thinking," "direction" are the load-bearing nouns of the entire standard, yet none is given a definition with necessary/sufficient conditions or a semantic model. O-001 ("Everything is a Cognitive Entity") is asserted as a universal but is untestable and, taken literally, vacuous. A type registry (Ontology Spec) provides *URNs and one-line glosses*, not a semantics. This is the single most common reason ontology/standards papers are rejected: "your ontology has no model theory."

3. **Distributed-systems claims are stated but not specified or verified.** "Linearizable," "deterministic replay," "CP," "idempotent," "no split-brain" appear as assertions and design intents. There is no formal state-machine spec, no consistency model with a precise predicate, no TLA+/Coq/Ivy artifact, and no proof that per-shard Raft + sagas preserves any global correctness property. The reference runtime (arves-kernel, arves-persistence) is well-commented and unit-tested, but a single-node in-memory/file WAL cannot substantiate a distributed linearizability or replay claim; the tests are examples, not proofs, and there is no property-based or model-checked evidence.

None of these are fatal to the *project* — they are precisely the artifacts the Implementation Era and a "Formalization" work-stream can produce **without touching the frozen text**, because formalization can be delivered as *companion normative-reference artifacts* (Verification proposals) and machine-checkable properties (the Reference Lifecycle Part 4 already demands exactly this: "maps every theoretical claim to at least one testable assertion; unfalsifiable claims do not advance"). The corpus set the right bar and then froze before clearing it. This review's central recommendation is a **Formalization & Verification Program** that produces the mathematics, the model-theoretic semantics, and the machine-checked distributed proofs as external, versioned, conformance-gated companions.

**Headline verdict:** *Reject as a scientific reference architecture in current form; the path to Accept is a formalization program, not a redesign.* The architecture is sound enough to formalize; the claims currently outrun the evidence.

---

## Severity-Ranked Findings

| # | Severity | Title | Instrument | Impl. Complexity |
|---|----------|-------|------------|------------------|
| F1 | Critical | "Formal Proof / Mathematics" claimed but absent — unfalsifiable foundational claim | Verification | very-high |
| F2 | Critical | Primitive terms ("cognitive truth", "intelligence", "cognitive entity") undefined — no model-theoretic semantics | Verification | high |
| F3 | Critical | Distributed correctness (linearizability, replay, no-split-brain) asserted, never specified or model-checked | Verification | very-high |
| F4 | High | ORCH-003 conflates *reproducibility* with *replay-from-trace*; the reproducibility claim is scientifically weak as stated | IDR | medium |
| F5 | High | Conformance properties are named in prose, not written as formal predicates → "PASS" is not yet a scientific measurement | Verification | high |
| F6 | High | O-001..007 "design principles" are unfalsifiable universals presented alongside provable invariants — conflates two epistemic categories | Certification | medium |
| F7 | High | No evaluation methodology / empirical validation plan — no baselines, metrics operationalization, or reproducibility package | Verification | high |
| F8 | Medium | Determinism of nondeterministic engines is under-specified: replay soundness depends on an unspecified "Runtime Fingerprint" completeness argument | IDR | medium |
| F9 | Medium | Saga-based cross-shard model has no stated global consistency/serializability guarantee | IDR | high |
| F10 | Medium | Analogies (OCI, K8s Pod, W3C, Sonobuoy) substitute for prior-art positioning and novelty claims — no related-work / differentiation | Ecosystem | low |
| F11 | Medium | "Independent-Implementability Test" is asserted as an acceptance bar but never operationalized (no protocol, no witness) | Certification | medium |
| F12 | Low | Terminology drift & self-admitted inconsistencies (glossary absent; Gap Analysis §4) undermine the "one meaning system" claim | Ecosystem | low |

---

## F1 — "Formal Proof / Mathematics" is claimed as delivered but does not exist (Critical)

**Finding.** The Baseline (`ARVES_00_Baseline_v1`, Table 0) lists the S1 Foundation deliverable as *"Vision, Theory, **Mathematics, Formal Proof**, Validation."* The Documentation Index v2.2 (`ARVES_00_Documentation_Index_v2.2`), explicitly self-described as *"The complete register... this register, not any prose, is the complete list,"* contains **no Mathematics document, no Formal Proof document, and no Validation study**. A full-corpus search for `theorem|lemma|proof|model check|TLA|refinement|liveness|safety property` returns only *incidental* uses (e.g., "Formal Proof" as a Baseline table cell; "Proof over Opinion" as a slogan; the word "proof" reused for unit-test "behaviour proofs"). No mathematical object exists anywhere in the 50 files.

**Why it matters (scientific impact).** In a peer-reviewed reference architecture, "formal proof" is a term of art (a machine- or hand-checked derivation in a defined logic). Claiming it as a delivered, frozen artifact when none exists is, to a program committee, either overclaiming or a definitional confusion — both are desk-reject triggers. Worse, the claim is *frozen*, so it cannot be softened in-text; it will be read as a permanent, load-bearing falsehood at the foundation of the standard. ISO/IEEE editors treat unsubstantiated normative claims as blocking.

**Risks / long-term consequences.** (a) Reputational: the first rigorous external reviewer will lead with this, and it taints every other (sound) claim. (b) Legal/standards: an ISO normative reference cannot cite a non-existent "Formal Proof." (c) Path-dependency: downstream volumes lean on "the foundation is proven," so the gap propagates.

**Alternative designs considered.**
- *Do nothing / rely on the freeze* — unacceptable; the claim is falsifiable and false-as-stated.
- *Reinterpret "Formal Proof" as "internal-consistency audit"* (the Documentation Index integrity check) — this is a CCP-clarification candidate, but an audit is not a proof; it only downgrades the claim, it does not satisfy it.
- **Recommended:** Deliver the missing mathematics as an *external, versioned Formalization companion* (see below), and register a **CCP Amendment (PATCH/clarification)** that binds the frozen word "Formal Proof" to that companion's specific theorem set — the Reference Lifecycle explicitly allows clarification-without-behaviour-change as PATCH (Table 3), so this closes the integrity gap without reopening the Spec Era.

**Recommendation (Verification proposal — "ARVES Formalization Companion, Vol F1: Foundations").** Produce a companion normative-reference document containing: (i) a state-transition model of the Kernel commit gateway; (ii) a precise statement + proof sketch of the core safety property ("no state is truth unless committed through the Kernel" = a machine-checkable invariant, already reified in `arves-invariants::catalog`); (iii) the theorem that per-shard commit is linearizable *within a shard* under the IDR-001 Raft assumptions (cite/reuse the existing Raft linearizability results — do not re-derive Raft); (iv) an explicit *non-theorem* section stating what is NOT proven (cross-shard serializability, engine determinism). Gate it through CCP-GATE with a conformance scenario per theorem, satisfying Reference Lifecycle Part 4.

**Implementation complexity:** very-high (requires a formal-methods work-stream; 2-4 person-quarters for a defensible v1). **Ecosystem impact:** transforms ARVES from "framework with slogans" into "standard with a proof obligation ledger" — the single biggest credibility lever.

---

## F2 — Primitive terms are enumerated, never defined; the ontology has no model theory (Critical)

**Finding.** "Cognitive truth," "intelligence," "cognitive entity," "meaning," "knowledge," "thinking," "direction" are the standard's primitives. None is *defined*. Vol 1 gives a *cycle* ("World → Information → Knowledge → Thinking → Direction → Action → Learning → World") and a mapping of "Seven Fundamental Questions" to cores, but these are motivational, not definitional. The Ontology Spec (`ARVES_Universal_Cognitive_Ontology_Specification_v1`) provides a *type registry* — URNs (`uci.fact@1`), one-line glosses ("A validated truth claim"), aspects, and relations — but **no semantics**: no interpretation function, no axioms constraining `supports`/`derived_from`/`causes`, no consistency conditions, no distinction between `uci.fact` (a "validated truth claim") and Kernel-committed "cognitive truth." O-001 asserts "Everything is a Cognitive Entity," which is either false (a raw byte is not cognitive) or vacuous (if everything qualifies, the predicate carries no information).

**Why it matters (scientific impact).** A *Universal Cognitive Ontology* is an ontology; ontologies are judged by their model theory (cf. OWL/Description Logic, RDF semantics, the entire KR literature). "One versioned, registered, provenance-aware cognitive type system that all standards derive from" (Ontology Part 12) is a *syntactic* achievement. Without a semantics, two conformant runtimes can register the same URNs and still disagree on what a `uci.fact` *is*, defeating the interoperability the document claims (Part 11 Independent-Implementability Test). Oxford/formal-methods reviewers will reject on this alone: "you have a vocabulary, not an ontology."

**Risks / long-term consequences.** Semantic drift across independent runtimes; unfalsifiable ontology principles (F6); inability to define what conformance to the *ontology* even means; the "shared meaning system" claim becomes marketing.

**Alternative designs.**
- *Adopt an existing upper ontology* (BFO/DOLCE) as the semantic backbone — high rigor, but heavy and risks contradicting frozen O-001..007; more suitable as a v2 alignment.
- *Description-Logic formalization* of the root lattice + relations (SROIQ) with a reasoner (HermiT/ELK) checking consistency — pragmatic, tool-supported, and directly testable.
- **Recommended:** A **Verification companion — "Ontology Semantics Reference"** giving (i) an interpretation domain, (ii) axioms for each relation (`supports` is irreflexive+asymmetric; `derived_from` is acyclic — a *lineage DAG*, matching O-005; `causes` is a strict partial order over `uci.event`), (iii) a decidable consistency check, and (iv) a precise distinction between `uci.fact` (an ontology instance) and *cognitive truth* (Kernel-committed state), resolving the F2/ORCH-001 ambiguity. This *interprets* the frozen registry; it does not modify it.

**Implementation complexity:** high. **Scientific impact:** turns O-001..007 from unfalsifiable slogans into a checkable theory; enables an *ontology conformance level* in certification.

---

## F3 — Distributed correctness is asserted but never specified or model-checked (Critical)

**Finding.** IDR-001 states "Writes are linearizable; replay is deterministic from the committed log," selects CP, per-shard Raft, joint consensus, and "no cross-shard atomic commit; ... sagas/compensation." These are *design decisions*, correctly recorded as IDRs. But nowhere is there: a formal consistency model (a precise definition of the linearizability the system claims and *the object it linearizes*), a state-machine specification of the commit path, an argument that "replicate committed OUTCOMES, not invocations" (IDR-001) preserves replay determinism given nondeterministic engines, or any machine-checked model (TLA+/Ivy/P) of leader election + joint-consensus + saga interplay. The reference runtime is single-node: `arves-kernel` (`RefKernel<S>` over `MemWalStore`/`FileWalStore`) and `arves-persistence` are unit- and integration-tested (kernel `tests/{checkpoint,persistent_wal,recovery,walking_skeleton}.rs`), which proves *single-node* append/replay/recovery behaviour — it says nothing about the distributed claims, which are the scientifically interesting ones.

**Why it matters (scientific impact).** SOSP/OSDI/PODC reviewers expect distributed claims to be either (a) proven, (b) model-checked, or (c) empirically stress-tested against an adversarial fault model (Jepsen-style). ARVES currently has none for its distributed layer. "CP via Raft" is not a result — Raft's guarantees are per-group; the *composition* (many shards + sagas + engine placement "engines run anywhere, commit through the leader") is where the novel correctness questions live, and those are unaddressed. The Master Blueprint even lists a "10,000 nodes" scale target (IDR-001 rejects global single-leader "does not scale to 10,000 nodes") with zero scaling analysis.

**Risks / long-term consequences.** A subtle composition bug (e.g., a saga observing a partially-committed multi-shard state, or a stale-leader engine committing after re-election) could violate the very single-truth invariant the architecture is built to protect — and there is currently no artifact that would catch it before production.

**Alternative designs.**
- *Empirical-only* (Jepsen against the future cluster) — necessary but insufficient for a *standard*; finds bugs, doesn't establish guarantees.
- *Full mechanized proof* (Verdi/IronFleet-style) — gold standard, very high cost.
- **Recommended (staged Verification program):** (1) a **TLA+ specification** of the per-shard commit + leader-election + membership-change state machine with `TLC`/`Apalache` model-checking of the safety invariant "committed truth is never lost or forked" and a bounded liveness check; (2) a **TLA+ model of the saga/compensation protocol** proving a stated *cross-shard* guarantee (see F9); (3) a **Jepsen-style conformance axis** (the framework already has Axis 12 "Recovery & Replay" and Axis 8 "tenant isolation at scale") wired to the reference runtime once I2 lands. Each artifact is external and versioned per UCS version — no frozen text changes.

**Implementation complexity:** very-high. **Ecosystem impact:** a public TLA+ model + Jepsen report is exactly what independent certifiers and enterprise adopters demand; it is also the strongest possible answer to "why should we trust ARVES over an ad-hoc agent framework."

---

## F4 — ORCH-003 conflates *reproducibility* with *replay-from-trace* (High)

**Finding.** ORCH-003 ("Every execution is REPLAYABLE from the same Goal, State, Policies, Capabilities and Runtime Fingerprint — via a recorded decision trace, not by recomputation") and its rationale ("reproducibility means deterministic replay from recorded outcomes, not identical re-computation") redefine *reproducibility* to mean *re-reading a log*. Scientifically, replaying a recorded output is *not* reproducibility — it is *deterministic playback of a recording*. Reproducibility, in the scientific sense, is the ability to obtain equivalent results by *re-running the process*. The corpus explicitly disclaims re-computation for nondeterministic engines (Vol 9 Part 9: "Recomputation is explicitly NOT guaranteed").

**Why it matters.** The claim as worded invites the critique "ARVES's 'reproducibility' is a tautology: of course replaying a log reproduces the log." That undercuts a headline selling point. The *actual* property ARVES has is stronger and more honest if named precisely: **trace-faithful deterministic replay** (given the trace, state evolution is a deterministic function). That is a legitimate, defensible property — it just isn't "reproducibility."

**Risks / long-term consequences.** Reviewers and adopters conflate the two and either over-trust (thinking they can re-derive decisions) or dismiss ARVES as circular. Either way, the credibility of the audit/replay story — a genuine strength — is damaged by imprecise naming.

**Alternative designs.**
- *Leave ORCH-003 frozen and unqualified* — perpetuates the confusion.
- **Recommended (IDR):** Author an **IDR ("Replay Semantics Clarification")** that, without altering ORCH-003's frozen text, *implements* it by defining two distinct, precisely-stated properties for the reference runtime and conformance suite: **(P1) Trace-Faithful Replay** — given a recorded decision trace and Runtime Fingerprint, the committed-state trajectory is a deterministic, bit-identical function of the trace (this is what `arves-kernel::truth_hash` already checks single-node); and **(P2) Re-derivation (optional, engine-class-gated)** — only Deterministic/Seeded engines (Engine Graph Part 6, ENG-004) may be re-run to *recompute* outcomes. Map each to a conformance assertion (Axis 12). This is pure implementation-side clarification and is exactly what an IDR is for.

**Implementation complexity:** medium. **Scientific impact:** converts a vulnerable claim into a crisp, checkable theorem statement; makes the audit story unassailable.

---

## F5 — Conformance properties are named in prose, not written as formal predicates (High)

**Finding.** The Scenario Conformance Framework (Part 8) is *methodologically excellent* — it correctly rejects golden-output testing for nondeterministic systems and asserts invariants + properties. But the properties are prose: "tenant/workspace isolation," "provenance/trust present," "safety gates blocked unsafe plans," "plan replay reproduces the decision trace." None is written as a formal predicate over a defined state/trace model. "PASS/PARTIAL/FAIL" is therefore, today, a *human judgement*, not a measurement. The framework itself admits this (Part 3, "honest scope": "Many node contracts in the corpus are still one-line ... the POPULATED assertion suite grows as node contracts are sharpened").

**Why it matters (scientific impact).** A conformance suite is a *fitness function*; to be scientific it must be a *function* — deterministic, defined over precise inputs (the conformance artifact schema, Part 9), returning a defined verdict. Until each property is a predicate, "N% at Level Lx" (Part 10) is not reproducible across evaluators, defeating the certification program's entire purpose (independent teams "judged by scenario results, not code inspection," Part 13).

**Risks / long-term consequences.** Two certifiers reach different verdicts on the same runtime; certification becomes contestable; the ecosystem cannot trust badges.

**Alternative designs.**
- *Keep prose properties + human graders* — non-reproducible, non-scientific.
- **Recommended (Verification):** Build a **machine-checkable property library** keyed to the frozen conformance artifact schema (Part 9): each of the ~5 asserted properties becomes an executable predicate over the artifact (e.g., *isolation*: `∀ read r in trace, r.tenant == scenario.tenant`; *replay*: `truth_hash(replay(trace)) == artifact.truth_hash`; *no-truth-in-control-plane*: `∀ commit c, c.origin ∈ Kernel`). The `arves-invariants::PropertyCheck`/`Suite` trait scaffolding is *already designed for exactly this* — it is currently a signatures-only skeleton (methods intentionally unimplemented). Filling it is a Verification deliverable, not a spec change.

**Implementation complexity:** high. **Ecosystem impact:** makes certification reproducible and litigation-resistant; a prerequisite for third-party certifiers (Long-Term Objective 5).

---

## F6 — O-001..007 unfalsifiable "principles" are presented beside provable invariants (High)

**Finding.** The Invariant Registry and CLAUDE.md carefully separate O-001..007 ("Ontology Design Principles — definitional, NOT runtime-provable") from registered invariants. This separation is *good*. However, at the scientific level, several O-principles are stated as universally-quantified factual claims ("Everything is a Cognitive Entity"; "Truth emerges from validated Evidence") that are unfalsifiable as worded, yet sit in the same registry table as machine-checkable invariants (OWN-001, SHARD-001). Mixing epistemic categories (unfalsifiable design axioms vs. provable runtime invariants) in one register invites the reviewer to hold the axioms to the invariant standard and find them wanting.

**Why it matters.** Popperian falsifiability is a bright line for reviewers. An unfalsifiable universal is fine *as a modeling assumption* but must be labeled as such and *not* be allowed to do normative work. O-004 ("Truth emerges from validated Evidence") in particular reads like a claim about epistemology that the system cannot enforce or test.

**Risks.** Critics generalize "some ARVES 'principles' are unfalsifiable" into "ARVES is unfalsifiable," discrediting the falsifiable core.

**Alternative designs.**
- **Recommended (Certification):** In the *certification/verification* layer (not the frozen registry), reclassify each O-principle as one of {*modeling axiom* (assumed, not tested), *definitional convention* (naming rule), *derivable property* (follows from axioms — provable)}. E.g., O-005 ("Derivation is not Inheritance") is a *derivable structural property* (the lineage graph is a DAG disjoint from the is-a lattice) and CAN be machine-checked on the type registry; promote it to a checked property. O-001 is a *modeling axiom* and should be explicitly quarantined from conformance weight (it already carries no proof obligation — make that visible in certification docs). This is a documentation/certification act, consistent with CLAUDE.md's existing treatment.

**Implementation complexity:** medium. **Scientific impact:** protects the falsifiable core by fencing the axioms.

---

## F7 — No evaluation methodology or empirical validation plan (High)

**Finding.** The Baseline claims S1 delivered "Validation," and the Master Blueprint lists "Success Metrics" (Knowledge Quality, Decision Quality, Goal Success, Agent Effectiveness, Learning Rate). None is operationalized: no metric definitions, no measurement protocol, no baselines, no datasets, no reproducibility package, no ablation plan. There is no statement of *how one would empirically demonstrate* that ARVES improves any of these over a non-ARVES baseline.

**Why it matters (scientific impact).** A reference *architecture* paper can be accepted on design + proofs, but any claim of *intelligence quality* or *learning* (which the corpus makes repeatedly) requires an evaluation methodology. "Learning Rate" as a frozen success metric with no operational definition is exactly the kind of claim a Stanford AI reviewer flags as unmeasurable.

**Risks / long-term consequences.** The Evolution Core (learning/meta-learning) and L3/L4 capabilities are unvalidatable; the "continuous improvement" thesis is untestable; adopters cannot compare ARVES runtimes.

**Alternative designs.**
- **Recommended (Verification/Ecosystem):** Produce an **Evaluation Methodology companion** that operationalizes each success metric as a measurable quantity with a protocol, defines at least one *reference benchmark task per conformance axis* (the 12 axes are a natural benchmark taxonomy), and ships a reproducibility package (fixed seeds, recorded traces, expected property-verdicts). Tie it to the certification levels so "Certified L2" carries measured meaning. For cognitive-quality claims, adopt held-out task suites and report against a stated baseline; explicitly mark L3/L4/"Learning Rate" as *aspirational, not validated* (the Baseline already fences L3/L4 as "north-star, not v1.0 commitments" — extend that honesty to the metrics).

**Implementation complexity:** high. **Ecosystem impact:** benchmarks drive adoption and comparability; without them "certified" is unfalsifiable.

---

## F8 — Replay soundness rests on an unspecified "Runtime Fingerprint" completeness argument (Medium)

**Finding.** Trace-faithful replay (ORCH-003) is sound only if the recorded trace + Runtime Fingerprint capture *every* input that influenced the committed outcome. The corpus enumerates fingerprint contents ("engine versions, model routing, capability bindings, policy set") but never argues *completeness* — i.e., that there is no hidden nondeterministic input (wall-clock, RNG seed not in the seed field, external side-effect ordering, network-dependent arbitration at join nodes) outside the fingerprint. Vol 9 Part 6 join-node arbitration ("confidence-weighting, tie-break") could depend on branch-completion order, which is a distributed timing input.

**Why it matters.** If the fingerprint is incomplete, replay is *not* deterministic and F4's P1 property is false. This is a classic determinism-audit gap.

**Alternative designs.**
- **Recommended (IDR):** An **IDR ("Determinism Boundary")** enumerating the *complete* set of nondeterminism sources and mandating that each is either (a) captured in the trace, (b) forbidden inside engines (purity, ENG-001), or (c) made deterministic (e.g., join-node arbitration MUST be a deterministic function of branch *contents*, not arrival order, with a total tie-break on content hash — content-addressability, ORCH-004, already gives the ordering). Add a conformance assertion that replays a trace under adversarially-permuted branch arrival and requires identical `truth_hash`.

**Implementation complexity:** medium. **Scientific impact:** upgrades replay from "believed deterministic" to "deterministic by construction with an enumerated boundary."

---

## F9 — Cross-shard saga model has no stated global guarantee (Medium)

**Finding.** IDR-001 forbids cross-shard atomic commit and delegates multi-shard work to "sagas/compensation (Amendment-006)." No global correctness property is stated for the saga model — no serializability class, no isolation level, no guarantee about what a reader observing two shards mid-saga may see (the Handbook even says "a partial state across shards is expected mid-saga, not a bug"). For a system whose *raison d'être* is a single, coherent notion of cognitive truth, the absence of any cross-shard consistency statement is a theoretical hole.

**Why it matters.** "Single source of cognitive truth" (G-001) is a *global* claim; sagas make truth *per-shard* with no defined cross-shard semantics. Reviewers will ask: "In what sense is truth 'single' if a multi-shard fact can be half-true?"

**Alternative designs.**
- **Recommended (IDR + Verification):** An **IDR** stating the intended cross-shard guarantee (e.g., *per-shard linearizability + causal consistency across shards via saga event ordering*, explicitly NOT serializability), plus a TLA+ model (F3) proving no saga can leave permanent orphaned truth (every committed compensation restores an invariant). Name the anomaly classes explicitly permitted (fractured reads mid-saga) so adopters design around them.

**Implementation complexity:** high. **Scientific impact:** replaces a hole with a precisely-bounded, honest guarantee.

---

## F10 — Analogies substitute for prior-art positioning and novelty claims (Medium)

**Finding.** The strongest documents lean on analogies: Engine Graph = "OCI Image / K8s Pod Spec"; Conformance = "Certified Kubernetes / Sonobuoy / W3C"; Reference Lifecycle = "W3C Process / IETF RFC 2026 / KEP / SemVer." Analogies are helpful pedagogy but they are *not* related-work analysis. Nowhere does ARVES state its *novel* contribution relative to the actual literature (agent frameworks, blackboard architectures, BDI, cognitive architectures like SOAR/ACT-R, event-sourcing/CQRS, workflow engines like Temporal, deterministic-replay systems) or *why* those are insufficient.

**Why it matters (scientific impact).** Every top venue requires a related-work section establishing novelty. Without it, ARVES reads as a well-organized synthesis of known ideas (event sourcing + Raft + DAG orchestration + an ontology), which reviewers may deem "engineering, not research." The Vol 1 "Non Goals" ("not a chatbot/RAG/agent framework/workflow engine") gestures at differentiation but asserts rather than argues it.

**Alternative designs.**
- **Recommended (Ecosystem):** A companion **"Related Work & Novelty" position paper** that (i) maps each ARVES mechanism to its nearest prior art, (ii) states precisely what is novel (candidate: the *composition* — a truth-owning consensus kernel + truth-less replayable cognitive control plane + a portable engine ABI + property-based cognitive conformance — as a *coherent standard*, not any single mechanism), and (iii) argues insufficiency of each prior baseline. This is exactly the artifact a journal submission needs and it requires no spec change.

**Implementation complexity:** low. **Scientific impact:** without it, publication is impossible regardless of technical merit.

---

## F11 — "Independent-Implementability Test" is an acceptance bar that is never operationalized (Medium)

**Finding.** Three frozen documents (Ontology Part 11, Engine Graph Part 13, Reference Lifecycle Part 11) each assert an "Independent-Implementability Test": *"an independent team, given only this specification, can build X."* This is the correct scientific bar (it is essentially a reproducibility/replicability requirement on the *specification itself*). But it is never operationalized: no protocol for running the test, no definition of a passing witness, no record of it ever being attempted. The corpus's own Gap Analysis ("NOT yet an implementable specification"; "90% naming, 10% content") strongly implies the test would *currently fail*.

**Why it matters.** An unrun acceptance test is not an acceptance test. The standard asserts a property about itself that it has not demonstrated — and internal evidence suggests it does not yet hold.

**Alternative designs.**
- **Recommended (Certification):** Operationalize the test as a **Certification protocol**: commission a genuinely independent team (or a firewalled sub-team) to implement a bounded slice (e.g., L1 Core Runtime: Information → Kernel → Query) from the frozen spec *alone*, record every point where they had to consult authors or guess, and treat each such point as a defect to be closed via IDR (clarifying implementation guidance) — never by editing the frozen spec. Publish the implementation report (the W3C model the corpus already cites). This both *tests* the claim and *produces* an independent runtime (Long-Term Objectives 3-4).

**Implementation complexity:** medium. **Ecosystem impact:** the implementation report is the canonical evidence ISO/IEEE and adopters trust.

---

## F12 — Terminology drift and self-admitted inconsistency undermine "one meaning system" (Low)

**Finding.** The Gap Analysis (§4) records: conflicting canonical models across Vol 3 / Vol 13 / ARVES-19 (later resolved by superseding, but the *frozen* history remains), no glossary despite heavy specialized vocabulary, version drift (v2 vs v1, no changelog), and a numbering gap (doc 25 missing). "CP" is overloaded (CAP-consistency-class vs. Control Plane) badly enough that Vol 5 and Vol 2 need explicit disambiguation notes. The Ontology claims to be "The Shared Meaning System" while the corpus lacks a terminology dictionary.

**Why it matters.** A standard whose central claim is *shared meaning* cannot ship without a glossary; overloaded acronyms cause real implementer errors (the corpus itself had to fix "CP" wording in a consistency pass).

**Alternative designs.**
- **Recommended (Ecosystem):** Ship an **external normative Glossary companion** (allowed — glossaries are clarification, PATCH-class) defining every primitive with a single canonical meaning and flagging every overloaded term (CP, truth, fact, plan, capability). Cross-link to the Ontology semantics (F2). This is low-cost and high-leverage for interoperability.

**Implementation complexity:** low. **Scientific impact:** modest alone, but a hard prerequisite for the F2 semantics and F5 predicates to be unambiguous.

---

## What is still missing if ARVES were standardized by ISO/IEEE tomorrow

Even accepting the architecture as-is, an ISO/IEEE editor would block on:

1. **No normative mathematics.** A standard cannot claim "Formal Proof" (Baseline S1) without a proof artifact (F1). ISO normative references must exist.
2. **No formal semantics for the primitives.** "Cognitive truth" and the ontology have no model theory (F2); "conformance to the ontology" is undefined.
3. **No machine-checked distributed spec.** The consistency/replay/no-split-brain claims have no TLA+/Coq artifact and no adversarial empirical validation (F3, F9).
4. **Conformance verdicts are not yet reproducible measurements** (F5) and the self-declared acceptance bar (Independent-Implementability) has never been run (F11).
5. **No evaluation methodology** for any cognitive-quality/learning claim (F7); success metrics are unmeasured.
6. **No related-work / novelty positioning** (F10) — required for the peer-reviewed *reference* status the objective targets.

**The good news for the 20-year horizon:** every one of these is deliverable as *external, versioned, conformance-gated companion artifacts* under the existing Reference Lifecycle (Formalization stage, Part 4; CCP-GATE) — the freeze does not block formalization, semantics, proofs, benchmarks, or a glossary. ARVES froze an *architecture* prematurely relative to its *science*; the corrective is a **Formalization & Verification Program**, not a redesign. The architecture is, in this committee's judgment, sound enough to formalize — which is the highest compliment a frozen design can receive from a review board.

---

## Consolidated Recommendation (single program, phased)

Establish an **ARVES Formalization & Verification Program** producing external normative-reference companions, each CCP-gated with conformance scenarios, none modifying the frozen corpus:

- **Phase 0 (Ecosystem, low cost):** Glossary (F12) + Related-Work/Novelty position paper (F10).
- **Phase 1 (Verification):** Ontology Semantics Reference (F2) + machine-checkable conformance predicate library filling the existing `arves-invariants` skeleton (F5) + O-principle reclassification (F6).
- **Phase 2 (IDR):** Replay-Semantics + Determinism-Boundary + Cross-Shard-Guarantee IDRs (F4, F8, F9).
- **Phase 3 (Verification, heavy):** TLA+ specs + model-checking of commit/consensus/saga (F3), Foundations proof companion binding the frozen "Formal Proof" claim (F1), Evaluation Methodology + benchmark suite (F7).
- **Phase 4 (Certification):** Operationalize and run the Independent-Implementability Test as a published implementation report (F11), yielding the first independently-certified runtime.

Outcome: ARVES becomes a standard whose every headline claim has a corresponding, versioned, machine-checkable proof obligation — the definition of an ISO/IEEE-grade, peer-reviewable reference architecture.

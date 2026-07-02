# ARVES — ISO/IEEE Working-Group Readiness Review (Prompt 3, Wave-1 Gate)

**Reviewer role:** Independent ISO/IEC/IEEE-style Working-Group editor
**Lens:** Standardizability — terminology, normative language (SHALL/SHOULD/MAY discipline), normative references, vocabulary consistency, undefined behaviour, conformance-clause completeness, interoperability, certification robustness.
**Corpus reviewed:** 50 frozen documents in `runtime/review-input/*.txt` (Master Blueprint; Vol 1–18; Universal Cognitive Ontology Spec; Engine Graph Spec; Scenario Conformance Framework; Reference Lifecycle; Baseline; Freeze Record; IDR Batch 1; Amendments CCP Batch 1; Invariant Registry; Cognitive Control Plane v2; AEOS OS Vol 1–6; Catalogs 19–26).
**Reference-runtime evidence consulted:** `runtime/crates/arves-conformance/src/lib.rs`.
**Optimization horizon:** the next 20 years, not delivery speed.
**Change discipline:** the frozen spec is never modified; every remedy below is an **IDR**, **CCP Amendment**, **Runtime**, **Verification**, or **Certification** instrument.

---

## Executive Summary

ARVES has the *skeleton* of a standardizable specification and, unusually for an early-stage corpus, it has already invented the correct **governance machinery**: a Reference Lifecycle modelled on the W3C Process / IETF RFC 2026 / Kubernetes KEP; a CCP change process with a hard "no behaviour without a conformance scenario" gate (CCP-GATE); a two-track UCS-vs-UCI (standard-vs-reference-implementation) split modelled on a language standard vs. its compiler; a versioned conformance framework modelled on Sonobuoy + the W3C test-suite/implementation-report model; and an independent-implementability acceptance bar stated in three separate normative documents. That process layer is genuinely strong and is the single best reason to believe ARVES *could* become an ISO/IEEE-grade standard.

However, judged as an ISO/IEC/IEEE **deliverable** rather than as a governance vision, the corpus is **not yet structured like a standardizable specification**. The blocking problems are structural and systemic, not local:

1. **No RFC 2119 / ISO normative-language discipline anywhere in the normative core.** Across ~4,500 lines of the entire corpus, the keywords SHALL/SHOULD/MAY/MUST/REQUIRED appear only ~12 times, and almost all of those are in the AEOS *process* playbooks — not in the standards themselves. The Ontology Spec, Conformance Framework, Reference Lifecycle, Engine Graph ABI and Vol 9 state their requirements as declarative narrative ("owns no truth", "writes NOTHING", "reads state and produces inference"). A working group cannot mechanically extract a conformance requirements list from prose whose modality is implicit.

2. **No Terms & Definitions clause and no glossary — corpus-wide.** ISO/IEC Directives Part 2 and IEEE style both make a Terms-and-Definitions clause mandatory. ARVES defines dozens of load-bearing terms (Truth, Cognitive Truth, Working Memory, Living Cognitive World, Decision Trace, Runtime Fingerprint, Plan/Engine Graph, Control Plane, Data Plane, Proposed Effect, Observation vs. Signal vs. Event) only by usage. The corpus's own Gap Analysis flags this ("No glossary / terminology dictionary despite many specialized terms") but rates it "Low" — an ISO editor would rate it blocking.

3. **No Normative References clause and inconsistent normative dependency wiring.** Precedents (Raft, W3C Process, RFC 2026, OCI Image Spec, Kubernetes Pod Spec, Sonobuoy, Semantic Versioning) are cited inline as *inspiration* with no dated normative-reference list, so an independent implementer cannot know which external artifacts are binding.

4. **The conformance clause is a framework, not a suite — by the corpus's own admission.** The Scenario Conformance Framework Part 3 states plainly that node contracts are "still one-line" and the populated assertion suite "grows as node contracts are sharpened." The reference-runtime conformance crate confirms this: it is an interface skeleton ("NO implementation yet"). A standard whose *executable definition of correctness* is unpopulated cannot yet certify interoperability, and the two-independent-runtimes goal (Independent Runtime A/B) cannot be exercised.

5. **Vocabulary is inconsistent across the very documents that must interoperate.** Relationship/relation names, identifier casing, and even the term "Control Plane" itself drift between documents (the corpus's own consistency audit had to fix a "Vol 2/Vol 3 CP-vs-Control-Plane wording" conflict). Two ontologies (ARVES-19 SCREAMING_CASE relationship types vs. the UCS Ontology Spec snake_case relations) coexist in the frozen set with only a "superseded" marker to disambiguate.

6. **Undefined / underspecified behaviour at the exact points interoperability lives.** The canonical entity schemas are literally copy-paste identical (all 16 ARVES-19 entities share one definition); there are no field types, cardinalities, error models, event payload schemas, or wire formats. The Ontology Spec's own Part 9 registry entry `{ urn, version, aspects, schema, relations }` names a `schema` field that is never populated anywhere.

None of these are fatal to the *idea* — they are the normal delta between a strong architectural vision with excellent governance and a ratifiable ISO/IEEE Draft International Standard. But they are exactly the deltas an ISO/IEEE working group exists to close, and they are large. The corpus is roughly, in its own words, "90% naming & structure, 10% content," and standardizability lives almost entirely in the missing 10%.

**Gate verdict: CONDITIONAL.** The deeper reviews are worth running — the architecture and governance are coherent enough that populating them is engineering, not redesign. But the corpus must not be presented to any standards body until at least the normative-language, terminology, normative-reference, and conformance-suite-population instruments below are executed. Until then it reads as a high-quality architecture whitepaper, not a specification.

---

## Severity-Ranked Finding Register

| # | Severity | Title | Instrument | Impl. Complexity |
|---|----------|-------|------------|------------------|
| F1 | Critical | No RFC 2119 normative-language discipline in the normative core | CCP-Amendment | high |
| F2 | Critical | No Terms & Definitions clause / glossary anywhere | CCP-Amendment | medium |
| F3 | Critical | Conformance clause is an unpopulated framework, not a suite | Verification | very-high |
| F4 | High | No Normative References clause; precedents cited only as inspiration | CCP-Amendment | low |
| F5 | High | Inconsistent vocabulary & identifier conventions across interoperating docs | CCP-Amendment | medium |
| F6 | High | Undefined behaviour at interop surfaces: no schemas, wire formats, error models | CCP-Amendment | very-high |
| F7 | High | Certification robustness: single arbiter, no test-suite-independence, no appeals/audit | Certification | high |
| F8 | Medium | Conformance profiles/levels not mapped clause-by-clause to normative requirements | Verification | high |
| F9 | Medium | No requirement identifiers / traceability IDs on individual normative statements | CCP-Amendment | medium |
| F10 | Medium | Frozen corpus retains superseded normative documents without machine-checkable precedence | CCP-Amendment | low |
| F11 | Medium | "Non-deterministic correctness" model needs a normative interoperability bound | IDR | high |
| F12 | Low | Editorial: version drift, numbering gap (#25), no changelogs, no document-status legend at point of use | CCP-Amendment | low |

---

## F1 — No RFC 2119 / ISO normative-language discipline in the normative core
**Severity: Critical · Instrument: CCP-Amendment · Implementation complexity: high**

**Finding.** The corpus expresses requirements almost entirely as declarative narrative. Empirically, across the whole `review-input` set the modal keywords SHALL / SHALL NOT / MUST / MUST NOT / SHOULD / MAY / REQUIRED / RECOMMENDED / OPTIONAL occur only ~12 times, and 8 of those are inside the AEOS engineering *playbooks* (Vol 4/5, Master Index, Handbook) — process documents, not the standard. In the documents a working group would treat as normative:
- Engine Graph Spec has exactly one MUST ("A conformant runtime executing an engine node MUST:", Part 10) — the *only* clean conformance clause in the entire standard corpus.
- IDR Batch 1 has one SHALL, but IDRs are explicitly non-normative (`STATUS: REFERENCE IMPLEMENTATION DECISIONS (NOT A NORMATIVE STANDARD)`).
- The Ontology Spec, Scenario Conformance Framework, Reference Lifecycle and Vol 9 (the source of the ORCH invariants) contain **zero** RFC 2119 keywords. Their requirements read "The Control Plane owns no truth" (Vol 9 Part 5 / ORCH-001), "Query ... Writes NOTHING (read-only)" (Amendments Table 0), "an engine reads state and produces inference; it never mutates truth."

**Why it matters.** Declarative present-tense prose is descriptive, not prescriptive. "The Control Plane owns no truth" can be read as (a) a design fact the authors assert, (b) a requirement an implementation must satisfy, or (c) an invariant a verifier must check — three different obligations. ISO/IEC Directives Part 2 §7 and IEEE editorial rules require that every requirement use a controlled modal verb precisely so an implementer and a test author extract the *same* obligation set. Without it, two independent teams reading "owns no truth" will build and test different systems and both will claim conformance — which directly defeats the corpus's own stated acceptance bar (Ontology Spec Part 11; Engine Graph Part 13; Reference Lifecycle Part 11: "an independent team, given only this specification, can build...").

**Risks / long-term consequences.** This is the single largest 20-year risk. A standard ratified in declarative prose accretes divergent implementations, each "conformant" by its own reading; interoperability erodes silently; and every dispute becomes an editorial argument with no textual anchor. Retrofitting modality *after* implementations exist is far more expensive and politically fraught (you are now breaking someone's product) than doing it before.

**Alternative designs.** (a) Adopt RFC 2119/RFC 8174 keywords wholesale with a boilerplate conformance-terminology clause. (b) Adopt ISO Directives modal verbs (shall/should/may/can/must-for-external-constraint). (c) A hybrid: RFC 2119 for the ABI/runtime contracts (engineer audience) and ISO modals for the governance/ontology layer. Recommendation prefers (a) for engineering-standard fit and tooling familiarity, but the *choice* is less important than the *discipline*.

**Recommendation.** Open a CCP Amendment "Normative Language Convention" that (1) adds a Conformance Terminology clause defining the keyword set once, referenced by every normative document; (2) rewrites each existing invariant and runtime-contract sentence into a single keyworded requirement (e.g. ORCH-001 → "A conformant runtime SHALL NOT commit cognitive truth outside the Kernel; the Control Plane SHALL hold no committed truth."). Because the *statements* do not change — only their modality is made explicit — this is an editorial amendment (PATCH/MINOR per Reference Lifecycle Table 3, "Clarification, no behaviour change"), not a spec reopening. High complexity only because of the number of sentences to convert.

**Scientific impact.** Converts an architecture theory into a falsifiable requirements set; each requirement becomes independently testable, which is the precondition for the "Formalization" stage the Reference Lifecycle Part 4 already demands.
**Ecosystem impact.** Enables third-party test authors and certification bodies to work from text alone; a hard prerequisite for Independent Runtime A/B and for any arms-length certification authority.

---

## F2 — No Terms & Definitions clause / glossary anywhere in the corpus
**Severity: Critical · Instrument: CCP-Amendment · Implementation complexity: medium**

**Finding.** No document contains a Terms-and-Definitions clause. The Gap Analysis confirms it ("No glossary / terminology dictionary despite many specialized terms", rated Low) and lists it as an open item ("Glossary / terminology dictionary ... Define all specialized terms in one place ... Corpus-wide"). Load-bearing terms defined only by usage include: *Truth* / *Cognitive Truth* / *committed truth*; *Working Memory*; *Living Cognitive World*; *Decision Trace* vs. *WAL* vs. *Raft log* (Vol 9/IDR-001 collapse these into "one ordered source" but never define the term); *Runtime Fingerprint*; *Plan* vs. *Engine Graph* vs. *Task Graph*; *Control Plane* vs. *Data Plane*; *Proposed Effect* / *proposed write*; *Observation* vs. *Signal* vs. *Event* vs. *Fact* (distinguished in the Ontology URN table but never formally defined); *Capability* vs. *Engine*; *arbitration*; *content-addressable*.

**Why it matters.** In ISO/IEC/IEEE deliverables, Terms & Definitions is a *mandatory* clause and every defined term is capitalized/marked on use and resolved to exactly one definition. ARVES's entire architecture is a set of ownership and separation claims about these terms ("only the Kernel owns Truth"), so an undefined "Truth" makes the central invariants uninterpretable to an outsider. The corpus even encodes a term-collision it had to repair — the Documentation Index integrity note records fixing a "Vol 2/Vol 3 CP-vs-Control-Plane wording" conflict — which is precisely the failure mode an absent T&D clause guarantees will recur.

**Risks / long-term consequences.** Without a single normative glossary, each new document reintroduces subtly different senses; the ontology (which *is* the type vocabulary) and the prose vocabulary diverge; and the "independent team given only this specification" test fails at the first ambiguous noun. Over 20 years, terminology drift is the classic slow death of a standard.

**Alternative designs.** (a) A standalone normative "ARVES Terminology" document (like ISO/IEC 2382 for IT vocabulary) that all others normatively reference. (b) A T&D clause inside a new "ARVES Core Standard" umbrella document. (c) Federate: each standard defines its own terms, with a corpus-level uniqueness check in the AEOS consistency verification. Recommendation: (a) — a single frozen `uci.*`-aligned vocabulary is most defensible and dovetails with the Ontology Registry, which already versions type meanings.

**Recommendation.** CCP Amendment adding a normative Terminology document; each definition carries a stable ID and a source citation; the AEOS cross-document consistency check is extended to fail if a normative document uses a capitalized term absent from the glossary. Medium complexity: the terms exist and are used consistently in *spirit*; the work is extraction, disambiguation (Control Plane!), and one-place authoring.

**Scientific impact.** A precise vocabulary is the substrate for the formal invariants; it lets the ORCH/OWN/LAYER/SHARD invariants be stated over defined nouns rather than intuited ones.
**Ecosystem impact.** SDK authors, connector authors, and certification reviewers all key off shared terms; a glossary is what lets a marketplace of independent capabilities describe themselves compatibly.

---

## F3 — The conformance clause is an unpopulated framework, not a suite
**Severity: Critical · Instrument: Verification · Implementation complexity: very-high**

**Finding.** ARVES rightly treats conformance as its "executable definition of correctness" and has a good three-layer model (12 Axes → Reference Scenarios → Node Probes → Verdict) plus property-based (not golden-output) semantics. But the Scenario Conformance Framework Part 3 openly states the suite is unpopulated: "Executable PASS/FAIL requires precise node contracts. Many node contracts in the corpus are still one-line. Therefore: the FRAMEWORK ... is defined now; the POPULATED assertion suite grows as node contracts are sharpened." Vol 6 Part 5 reinforces this ("Scenarios are illustrative reference workloads, not an exhaustive catalog"). The reference runtime confirms it: `arves-conformance/src/lib.rs` is explicitly "I1 skeleton — interfaces/contracts only, NO implementation yet"; `NodeProbe::observe` and `VerdictEngine::judge` are trait signatures with no bodies; `Scenario` is a data contract with example rows only.

**Why it matters.** For an ISO/IEEE standard, the conformance clause (or companion test suite) is what makes the standard *falsifiable and certifiable*. The corpus stakes its whole standardizability claim on conformance ("Independent teams claiming 'we built ARVES' are judged by scenario results, not by code inspection" — Framework Part 13). With four illustrative scenarios and thin, one-line node contracts, that judgement cannot actually be rendered: there is no assertion set dense enough to distinguish a conformant runtime from a plausible impostor. The Independent Runtime A/B goal (Vol 6 Part 13) — the corpus's own proof of vendor-neutrality — is currently unrunnable.

**Risks / long-term consequences.** A standard that ships a conformance *framework* but not a conformance *suite* invites premature "certified" claims against a near-empty bar, which is worse than no certification because it launders non-interoperable implementations with a passing badge. If the suite is populated only after multiple implementations exist, each vendor lobbies for a suite that its product happens to pass.

**Alternative designs.** (a) Gate certification launch (already deferred per Freeze Record Part 5 / Vol 6 header) on a minimum populated suite covering all 12 axes with N assertions each. (b) Adopt the W3C model literally: publish a test-suite repository + an implementation-report format, and require ≥2 independent passing reports per feature before a feature is marked "conformance-testable." (c) Deepen one vertical slice end-to-end first (the Gap Analysis's own recommendation) to prove the harness, then fan out.

**Recommendation.** This is a **Verification** program, not a spec change: (1) sharpen node contracts to the point of assertability under CCP (each sharpening arrives with its scenario, honoring CCP-GATE); (2) populate the `arves-conformance` crate's `VerdictEngine`/`NodeProbe` implementations; (3) define a minimum-suite bar per level (L1–L4) that certification launch is gated on; (4) require cross-verifiability (Vol 6 Part 13: a verifier built for one runtime replays another's artifact) as an actual passing test, not a stated goal. Very-high complexity: this is the core of the entire Implementation Era.

**Scientific impact.** Turns "property-based correctness for non-deterministic systems" from a slogan into a reproducible methodology — genuinely novel and publishable if done rigorously.
**Ecosystem impact.** The suite *is* the ecosystem's trust anchor; without it there is no marketplace, no third-party certification, and no meaningful "ARVES-compatible" claim.

---

## F4 — No Normative References clause; external precedents cited only as inspiration
**Severity: High · Instrument: CCP-Amendment · Implementation complexity: low**

**Finding.** No document has a Normative References clause. External standards are named inline as *precedent/inspiration*: Raft (IDR-001), W3C Process Document / IETF RFC 2026 / Kubernetes KEP / Semantic Versioning (Reference Lifecycle Part 1), OCI Image Specification / Kubernetes Pod Spec (Engine Graph Part 1/9), Certified Kubernetes (Sonobuoy) + W3C test-suite/implementation-report model (Conformance Framework Part 1). None are dated, versioned, or marked binding-vs-informative.

**Why it matters.** ISO/IEC Directives require separating **Normative references** (indispensable for application) from a **Bibliography** (informative). Raft is a live example: IDR-001 makes "per-shard Raft" the reference implementation's consensus, but the Reference Lifecycle two-track model says an Independent Runtime may pick other mechanisms — so is the Raft paper normative or not? The text is silent. An implementer cannot tell which external documents they are contractually bound to, nor which version.

**Risks / long-term consequences.** Undated references rot; "Kubernetes Pod Spec" without a version is unresolvable in 10 years. Ambiguous binding status lets disputes about whether an implementation must follow, e.g., RFC 8174 keyword semantics, be settled by opinion.

**Recommendation.** CCP Amendment adding, to each normative document, a Normative References clause (dated, versioned, indispensable-only) and a Bibliography (everything else). Explicitly classify Raft, OCI, Kubernetes specs as *informative bibliography* (since the two-track model permits alternative mechanisms) while any keyword RFC adopted under F1 becomes *normative*. Low complexity — the references already exist; they need sorting and dating.

**Scientific impact.** Low-moderate; mainly rigor.
**Ecosystem impact.** Lets implementers scope their external compliance surface precisely; removes hidden coupling to specific infrastructure choices.

---

## F5 — Inconsistent vocabulary & identifier conventions across interoperating documents
**Severity: High · Instrument: CCP-Amendment · Implementation complexity: medium**

**Finding.** Multiple, coexisting naming systems for the same concepts sit inside the frozen set:
- **Two relation vocabularies.** ARVES-19 (Canonical Ontology) uses SCREAMING_CASE relationship types: `OWNS, MEMBER_OF, WORKS_ON, SUPPORTS, USES, PART_OF, DEPENDS_ON, RELATED_TO`. The UCS Ontology Spec (which *supersedes* ARVES-19) uses snake_case relations: `supports, derived_from, belongs_to, decomposes_into, produces, causes, governs, constrains`. Both are present, and only a "Superseded" status marker (in a *different* document, the Documentation Index) tells a reader which governs. Notably the sets are not even a rename of each other — `USES`, `WORKS_ON`, `REPORTS_TO`, `LOCATED_IN` have no counterpart in the superseding relation set, so the mapping is lossy and undocumented, contradicting the Ontology Spec's Part 8 promise that "Nothing is lost."
- **Three identifier casings for wire-level names.** Event types are dot.case (`tenant.created`, `decision.created`); envelope/ABI fields are snake_case (`correlation_id`, `tenant_id`) *and* Title Case ("Idempotency Key", "Retry Policy", "Runtime Fingerprint") in the same Engine Graph table; ontology URNs are `uci.<type>@<version>`.
- **Overloaded core term.** The Documentation Index integrity note records a repaired "Vol 2/Vol 3 CP-vs-Control-Plane wording" collision — evidence the term "CP" (consistency-partition, per CAP) and "Control Plane" were conflated across documents.

**Why it matters.** A standard's vocabulary must be uniform *especially* across the documents that must interoperate at runtime (ontology types resolve engine Reads/Writes; event names cross service boundaries). Two frozen relation vocabularies with a lossy, undocumented mapping is a direct interoperability hazard: an engine manifest that Reads a `uci.fact` and expects a `supports` edge cannot consume ARVES-19's `SUPPORTS` without a translation table that the spec does not provide.

**Risks / long-term consequences.** Superseded-but-present artifacts get cited by implementers who miss the status marker; casing inconsistency produces serialization bugs at every boundary; the lossy relation mapping means knowledge modelled under ARVES-19 cannot round-trip through a UCS-conformant runtime.

**Alternative designs.** (a) Physically remove or clearly stamp superseded normative documents (see F10). (b) Publish an explicit ARVES-19→UCS relation crosswalk as a normative annex, proving the "nothing is lost" claim or honestly recording what was dropped. (c) A single Identifier Conventions clause fixing casing per artifact class (types, relations, events, fields).

**Recommendation.** CCP Amendment: (1) normative crosswalk table mapping every ARVES-19 relationship and entity to its UCS root type/relation (or explicitly deprecating it); (2) an Identifier Conventions clause; (3) extend the AEOS consistency check to flag any normative use of a superseded vocabulary. Medium complexity.

**Scientific impact.** Moderate — a proven-complete crosswalk substantiates the ontology's "one hierarchy, nothing lost" claim.
**Ecosystem impact.** High — uniform names are the precondition for connector/capability portability across runtimes.

---

## F6 — Undefined behaviour at interoperability surfaces: no schemas, wire formats, error models
**Severity: High · Instrument: CCP-Amendment · Implementation complexity: very-high**

**Finding.** The corpus defines *what owns what* but not *what crosses the wire*. Concretely:
- **Copy-paste entity definitions.** In ARVES-19 all 16 canonical entities (Person, Organization, Team, Workspace, Agent, Goal, Strategy, Task, Project, Knowledge Object, Conversation, Document, Resource, Event, Device, Location) share byte-identical Definition/Attributes/Ownership/Lifecycle/Trust text. The Gap Analysis names this: "In ARVES-19 all 16 entities share the identical definition, which cannot be correct."
- **A named-but-empty schema slot.** The Ontology Spec Part 9 registry entry is `{ urn, version, aspects, schema, relations }` — but no `schema` is ever given for any `uci.*` type. The type system that "closes the Engine Graph ABI" has no field-level types, requiredness, or cardinality.
- **No event payload schemas.** ARVES-21 defines the envelope fields but, per its own template and the Gap Analysis, "no per-event payload schema, versioning example, or sample message."
- **No API signatures or error model.** ARVES-24 lists API *names* only — no paths, verbs, status codes, or error taxonomy. There is no normative error/failure representation anywhere (Amendment-006 defines a failure *taxonomy* conceptually but no error object).
- **No canonical serialization.** The Engine Manifest is called "the analogue of an OCI image manifest" and "serializable/content-addressable," but no serialization format, canonicalization rule, or hashing algorithm is specified — so two runtimes will compute different content-addresses for the same manifest, breaking ORCH-004 idempotency/content-addressability *across* implementations.

**Why it matters.** Interoperability is defined precisely at these surfaces. Content-addressability (ORCH-004) is meaningless without a canonical byte representation and a named hash. Cross-runtime artifact replay (Vol 6 Part 13) is impossible without a defined decision-trace/artifact wire format. An engine manifest is not portable if its type references resolve to schema-less URNs.

**Risks / long-term consequences.** This is where "independently implementable" quietly becomes false: two teams will serialize, hash, and error-model differently, and their artifacts will not cross-verify — silently failing the vendor-neutrality proof that is ARVES's reason for existing.

**Alternative designs.** (a) Adopt an existing canonical-serialization + hashing regime (e.g. a JSON canonicalization scheme + a named digest) as normative, à la OCI's content-addressing. (b) Define schemas in a schema language (JSON Schema / Protobuf / CDDL) as normative annexes. (c) Vertical-slice first (Gap Analysis recommendation): fully specify one entity (Person) end-to-end — schema → data contract → events → API → wire format — as the template.

**Recommendation.** CCP Amendment series (one per surface), each arriving with a conformance scenario per CCP-GATE: (1) canonical serialization + digest algorithm for manifests, artifacts, and the decision trace; (2) per-type `schema` population in the Ontology Registry; (3) event payload schemas; (4) API signatures + a normative error model. Very-high complexity — this is most of the "missing 10% content." Sequence via the vertical-slice approach to prove the templates before scaling.

**Scientific impact.** Moderate; the novelty is in canonicalizing *non-deterministic* decision traces such that they still cross-verify.
**Ecosystem impact.** Decisive — no wire contract, no interoperating ecosystem.

---

## F7 — Certification robustness: single arbiter, no test-suite independence, no appeals/audit trail
**Severity: High · Instrument: Certification · Implementation complexity: high**

**Finding.** Vol 6 defines a two-instrument certification (mechanical conformance + adversarial Independent Architecture Review) with a decision matrix and sign-off record — a strong start. But several ISO/IEEE certification-scheme essentials are absent or under-specified:
- **Test-suite independence isn't guaranteed.** The reference runtime (UCI) and the conformance suite share authorship; Vol 6 Part 2 asks the *reviewer* to be independent ("as if a rival company built it") but does not require the *suite* to be developed/maintained independently of any runtime it certifies. A suite co-evolved with UCI can encode UCI-specific assumptions.
- **No appeals, dispute-resolution, decertification, or surveillance-audit process.** ISO/IEC 17000-series conformity-assessment schemes require these; ARVES has issuance but no revocation, re-audit cadence, or challenge path.
- **No conflict-of-interest / accreditation rule for the certification authority.** "Arms-length review board" is named (Vol 6 Part 15) but not constituted; who accredits the accreditor is undefined.
- **Reproducibility depends on trusting the submitter's own trace.** Verdicts replay from "the runtime under test"'s recorded decision trace (Vol 6 Part 6). Nothing normatively prevents a runtime from emitting a *self-serving* trace; there is no independent instrumentation or attestation requirement.

**Why it matters.** A certification mark is only worth the independence and process rigor behind it. For a 20-year, multi-vendor standard, the certification scheme must survive adversarial vendors, not just honest ones. A submitter-provided trace with no attestation is a trust hole in the very artifact that is supposed to be "self-sufficient" (Vol 6 Part 7).

**Risks / long-term consequences.** Without decertification/surveillance, a runtime that regresses after certification keeps its badge; without suite independence, "certified" degrades to "passes the authors' tests"; without appeals, the scheme cannot be trusted by competitors — killing multi-vendor adoption.

**Alternative designs.** (a) Model the scheme on ISO/IEC 17065 (product certification) with accreditation, surveillance, and appeals. (b) Require the conformance suite to be governed by a body separate from any runtime maintainer (mirror the W3C separation of spec/test-suite/implementations). (c) Add trace-attestation: signed traces, or independent probe injection the submitter cannot forge.

**Recommendation.** A **Certification** program instrument (not a spec change): define the conformity-assessment scheme — accreditation, surveillance re-audit cadence, decertification triggers, appeals, conflict-of-interest rules, and trace-attestation requirements — and require suite/runtime authorship separation before certification launch (already deferred per Freeze Record Part 5). High complexity, mostly process/governance design.

**Scientific impact.** Low; process rigor.
**Ecosystem impact.** High — a credible, independent mark is what makes "certified ARVES runtime" a currency competitors will trust.

---

## F8 — Conformance profiles/levels not mapped clause-by-clause to normative requirements
**Severity: Medium · Instrument: Verification · Implementation complexity: high**

**Finding.** Conformance is reported "as a level against a suite version" (L1–L4 + Certified Product), and Vol 6 Table 6 maps levels to milestones and to *some* invariants. But there is no clause-by-clause conformance mapping: no table where every normative requirement (once F1/F9 give them IDs) is tagged to the level(s) at which it applies and the scenario(s)/probe(s) that test it. Vol 6's checklist (Part 10) is close but is a flat list keyed to invariants, not to *all* normative statements, and it mixes registered invariants, proposed invariants, and ad-hoc properties ("Query nodes wrote nothing") without a single traceable matrix.

**Why it matters.** ISO/IEEE deliverables require a conformance clause that states, for each requirement, whether it is mandatory/optional per profile and how it is verified. Otherwise coverage is unknowable: you cannot prove L2 tests *everything* L2 requires, nor that a requirement isn't orphaned (stated but never tested).

**Risks / long-term consequences.** Orphan requirements (stated, never tested) and untested corners accumulate; "L3 certified" means different things over time as the suite drifts.

**Recommendation.** **Verification** instrument: build a requirement→level→scenario→probe traceability matrix (depends on F9 IDs), extend the AEOS traceability check to fail on any normative requirement with zero covering probe, and publish per-level coverage. High complexity; gated on F1/F9.

**Scientific impact.** Moderate — completeness proofs for conformance coverage.
**Ecosystem impact.** Moderate — buyers can trust that a level means full coverage of that level's requirements.

---

## F9 — No requirement identifiers / traceability IDs on individual normative statements
**Severity: Medium · Instrument: CCP-Amendment · Implementation complexity: medium**

**Finding.** ARVES has excellent *invariant* IDs (ORCH-001..004, OWN/LAYER/SHARD-001, O-001..007) and *finding* IDs (F1..F7 in the ARR) and *amendment* IDs (A-001..006). But the vast majority of normative sentences — the runtime-contract bullets in Engine Graph Part 10, the layer-matrix cells in the Amendments Table 0, the property list in Conformance Framework Part 8, the enterprise-readiness bullets in Vol 6 Part 12 — have no stable identifiers. They cannot be cited, tested, or traced individually.

**Why it matters.** Traceability (theory→spec→contract→behaviour→conformance) is a stated ARVES principle (Vol 6 Part 2), but it operates at document granularity, not requirement granularity. A conformance probe "bound to a contract clause" (Framework Part 7) needs the clause to *have an address*.

**Risks / long-term consequences.** Without per-requirement IDs, F8's coverage matrix cannot be built, disputes cannot cite text precisely, and amendments cannot surgically target a single requirement.

**Recommendation.** CCP Amendment establishing a requirement-numbering convention (e.g. `EGS-R-010.1` for Engine Graph Spec Part 10 bullet 1) applied across normative documents; probes reference requirement IDs. Medium complexity — mechanical but corpus-wide.

**Scientific impact.** Low; infrastructure for rigor.
**Ecosystem impact.** Moderate — precise citation is what lets a distributed community argue productively about a standard.

---

## F10 — Frozen corpus retains superseded normative documents without machine-checkable precedence
**Severity: Medium · Instrument: CCP-Amendment · Implementation complexity: low**

**Finding.** The frozen set includes documents that are superseded but still shipped as normative-in-appearance: ARVES-19 (superseded by the UCS Ontology Spec), Vol 9 Runtime & Event Fabric Bible v1 (superseded by Vol 9 v2), and the entity/relationship lists in Vol 3/13. Precedence lives only in the Documentation Index status column and in prose ("SUPERSEDES the entity and relationship lists in Volume 3..."), not in the documents themselves. A reader opening `ARVES_19_Canonical_Ontology_v1.txt` sees "STATUS: CANONICAL ONTOLOGY CONSTITUTION (AUTHORITATIVE KNOWLEDGE MODEL)" with no in-document supersession banner.

**Why it matters.** In a frozen standard, a superseded-but-authoritative-looking document is a landmine: implementers cite it, and its vocabulary (F5) leaks back in. ISO handles this with explicit withdrawal/replacement metadata on the document itself.

**Recommendation.** CCP Amendment (editorial): stamp each superseded document with an in-document supersession header (`SUPERSEDED BY <doc> — NON-NORMATIVE`), and add a machine-readable precedence field the AEOS consistency check enforces. Low complexity.

**Scientific impact.** Low.
**Ecosystem impact.** Moderate — prevents accidental implementation against retired models.

---

## F11 — "Non-deterministic correctness" needs a normative interoperability bound
**Severity: Medium · Instrument: IDR (feeding a future CCP) · Implementation complexity: high**

**Finding.** ARVES's signature move — property-based, not golden-output, correctness because "cognitive engines are non-deterministic" (Framework Part 8; Vol 9 Part 9; ORCH-003) — is architecturally sound but leaves an interoperability gap unclosed: two conformant runtimes can produce *different decisions* for the same Goal/State/Policies and both PASS. That is acceptable for a single runtime's replay, but the standard never bounds *how much* two conformant runtimes may diverge, nor defines any notion of semantic equivalence for outcomes. "Same types, same relations, same aspects" (Ontology Part 11) is a *structural* interoperability claim; there is no *behavioural* interoperability claim.

**Why it matters.** A user who moves a workload from Runtime A to Runtime B has no standardized guarantee about behavioural compatibility — only that both honor the invariants. For safety-critical axes (7) and autonomous axes (11), "both are conformant yet decided oppositely" is a governance problem a 20-year standard must at least *name* and bound.

**Risks / long-term consequences.** Without a defined divergence bound or equivalence class, "portable" means only "structurally portable," and safety-critical certification claims are weaker than they appear.

**Alternative designs.** (a) Define behavioural conformance classes: Deterministic/Seeded engines must reproduce exactly; Nondeterministic engines must only satisfy invariant+safety properties (this is implicit today — make it normative and explicit, per ENG-004). (b) Introduce an outcome-equivalence relation (same decision *class* under policy) as an optional profile. (c) Require, for safety-critical scenarios, that the *gate outcomes* (not the reasoning) be deterministic across runtimes.

**Recommendation.** An **IDR** exploring behavioural-equivalence bounds, feeding a future CCP that adds a normative "behavioural conformance" clause distinguishing structural from behavioural interoperability, with a hard determinism requirement on safety gates. High complexity — genuinely hard theory.

**Scientific impact.** High — bounding interoperability of non-deterministic cognitive systems is a real research contribution.
**Ecosystem impact.** High for safety/regulated domains; determines whether ARVES can enter certified safety contexts.

---

## F12 — Editorial: version drift, numbering gap, missing changelogs and point-of-use status
**Severity: Low · Instrument: CCP-Amendment · Implementation complexity: low**

**Finding.** The Gap Analysis records: version drift (Blueprint/Vol1/Vol2 = v2, rest v1, "no changelog"); a numbering gap (document 25 missing, "24 jumps to 26"); no per-document changelogs. Additionally, document-status ("Frozen/Ratified/Superseded/Normative/Informative") lives centrally in the Documentation Index but is applied inconsistently in individual document headers.

**Why it matters.** ISO/IEEE documents carry self-describing status, version, and change history. Central-only status (F10) plus no changelogs makes it hard to audit what changed between v1 and v2 of a constitution — a problem for a standard meant to be amended over decades.

**Recommendation.** CCP Amendment (editorial): add a standard front-matter block (version, status, supersedes, changelog) to every normative document; resolve or formally reserve #25 (the Index already notes it as "reserved" — make that a real entry). Low complexity.

**Scientific impact.** Negligible.
**Ecosystem impact.** Low-moderate — smoother long-term maintenance and auditability.

---

## Gate Verdict — CONDITIONAL

**Is the corpus structured like a standardizable specification?** Partly. It has the *governance* structure of one (arguably better than many real standards at the same age): a formal lifecycle, a CCP change process with a no-behaviour-without-a-scenario gate, a UCS/UCI two-track split, versioned conformance, and an explicit independent-implementability acceptance bar repeated across documents. That is the hard, rare part and it is present and coherent.

But it lacks the *specification* structure of one: no normative-language discipline (F1), no Terms & Definitions (F2), an unpopulated conformance suite (F3), no Normative References clause (F4), inconsistent interoperating vocabulary (F5), and undefined behaviour at every wire surface (F6). These are the six load-bearing clauses an ISO/IEC/IEEE working group would demand before accepting a Working Draft, and they are exactly the "missing 10% content" the corpus's own Gap Analysis identifies.

Verdict is **CONDITIONAL, not NOT-READY**, because none of the gaps require redesigning the frozen architecture — every remedy is an additive CCP Amendment, an IDR, or a Verification/Certification program that *proves* the frozen spec rather than changing it. The dependency chain is intact; the invariants are coherent; the governance can carry the load. The deeper Wave-2 reviews are therefore worth running.

**Hard conditions before any standards-body submission (each an instrument, not a spec change):**
1. Execute F1 (normative-language convention) and F2 (Terminology clause) — the two clauses without which the corpus is unreadable as a specification.
2. Execute F4 (Normative References) and F9 (requirement IDs) — cheap, unblock everything downstream.
3. Begin F3/F6/F8 as the core Verification program: populate the conformance suite and specify wire contracts via the vertical-slice-first sequence, each CCP arriving with its scenario per CCP-GATE.
4. Constitute F7 (independent certification scheme with attestation, surveillance, appeals) before certification launch, which the Freeze Record already defers.

**If ISO/IEEE standardized ARVES tomorrow, the single biggest missing thing:** there is no way to mechanically extract a testable requirements set from the text — because requirements are written as declarative prose with no controlled modal verbs (F1), no defined terms (F2), and no populated conformance suite (F3). Two honest independent teams reading the frozen corpus would build two non-interoperable "conformant" runtimes and the standard could not adjudicate between them — which is the precise failure the corpus's own independent-implementability bar exists to prevent.

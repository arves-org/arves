# ARVES Independent Chief-Architect Review — Prompt 9: Ecosystem

**Reviewer role:** Independent, arms-length Chief Architect optimizing for ARVES becoming an
internationally adoptable, ISO/IEEE-grade cognitive-infrastructure standard over a 20-year horizon.
**Lens:** Ecosystem — SDKs, marketplace, connector ecosystem, independent certification labs,
training/curriculum, developer experience, governance (standard ownership + CCP adjudication),
community, and version evolution/compatibility policy.
**Corpus status:** UCS/UCI v1.0 FROZEN (2026-07-01). This review NEVER proposes editing the frozen
spec. Every proposal is one of: IDR · CCP Amendment · Runtime · Verification · Certification ·
Ecosystem · Product.
**Date:** 2026-07-02

---

## Executive Summary

ARVES v1.0 has done something most aspiring standards never do: it froze a coherent normative core
(Ontology, Engine Graph ABI, Scenario Conformance Framework, Reference Lifecycle, Certification
Manual, Control Plane invariants) with an explicit, W3C/IETF/KEP-grade *process* for change
(Reference Lifecycle Parts 6–8, CCP-GATE). The intellectual scaffolding for an ecosystem is
unusually strong: two-track UCS/UCI versioning, property-based (not golden-output) conformance,
Independent Runtime A/B as a first-class goal, and an independent-implementability acceptance bar
attached to every normative document.

**But the ecosystem itself is almost entirely deferred and, more importantly, un-instrumented.**
The Baseline (Part 3) and Freeze Record (Part 4) consciously defer Marketplace, Community/Ecosystem,
Cloud Runtime, Enterprise Governance, and Cross-Runtime Federation to v2. That is a legitimate
scoping choice — but it means that **on the day ARVES goes public there is no legal owner of the
standard, no accredited certification lab program, no machine-readable contract artifact from which
an SDK or connector could be generated, no compatibility test suite, and no licensing/trademark/IP
regime.** An ISO/IEEE standard is not the prose; it is the prose *plus* the maintenance
organization, the conformance test suite as a shippable artifact, the IPR policy, and the naming
authority. ARVES today has world-class prose and a near-empty ecosystem-instrument shelf.

The single highest-leverage gap is **the absence of machine-readable contract artifacts.** The
Ontology (`uci.<type>@<version>`), the Engine Manifest, and the ARVES-20/21/22/24 catalogs are all
defined in *prose and one-line inventories*. An SDK generator, a connector scaffolder, a conformance
harness, and a marketplace validator all need the *same* thing: a frozen, machine-readable schema
registry (IDL) that is the canonical source these tools consume. Without it, every SDK is
hand-written, every connector re-interprets the ontology, and "independent implementation" degrades
into "independent guessing." This is the ecosystem equivalent of shipping a language standard with
no reference grammar file.

The second highest-leverage gap is **governance of the standard as an institution.** The Reference
Lifecycle repeatedly asserts a "single owner + changelog" for the ontology registry and the
conformance suite, and the Certification Manual demands an "arms-length review board." But *who* that
owner is, how the board is constituted, how a CCP is adjudicated when the owner is also a commercial
vendor, and what the IP/trademark/patent-grant regime is — none of this exists. ISO/IEEE adoption is
impossible without a named custodian, a documented decision procedure, and a RAND or royalty-free
IPR commitment. This must be established *before* the ecosystem forms, because retrofitting
governance onto an existing community is one of the most reliable ways to fork a standard.

The reference runtime confirms the gap from the code side: all crates are `version = "0.0.0"`,
`publish = false`, and are path-linked skeletons. There is no published SDK surface, no stable API
crate, and the conformance crate (`arves-conformance/src/lib.rs`) is explicitly "I1 skeleton -
interfaces/contracts only, NO implementation yet." The ecosystem cannot be seeded from artifacts
that do not yet ship.

**Overall ecosystem verdict: PARTIAL — foundations excellent, instruments absent.** The
recommendations below are sequenced so that the *enabling substrate* (machine-readable contracts +
governance charter) is built before the *visible surface* (SDKs, marketplace, labs), because every
downstream instrument depends on those two.

---

## Severity-Ranked Findings

| # | Severity | Title | Type | Complexity |
|---|----------|-------|------|-----------|
| E1 | Critical | No machine-readable contract artifacts (IDL) to generate SDKs/connectors/harness from | Ecosystem | high |
| E2 | Critical | No standard-ownership charter, CCP adjudication authority, or IPR/trademark regime | Ecosystem | high |
| E3 | High | Certification defined but not operationalized as an accredited independent-lab program | Certification | high |
| E4 | High | Version evolution policy lacks executable compatibility suites, LTS, and migration tooling | Verification | high |
| E5 | High | No SDK strategy: languages, generation source, and the "cannot bypass Kernel" guarantee | Ecosystem | high |
| E6 | Medium | Connector ecosystem has a framework but no signing/provenance/certification instrument | Ecosystem | medium |
| E7 | Medium | Marketplace deferred with no trust substrate (capability/engine signing, revocation) | Ecosystem | high |
| E8 | Medium | Developer experience: no distributable dev runtime, conformance-as-a-service, or golden path | Product | medium |
| E9 | Medium | No training/curriculum/certification-of-humans program to build the practitioner base | Ecosystem | low |
| E10 | Low | Community process (contribution, RFC forum, working groups) undefined beyond the CCP shell | Ecosystem | low |

---

## E1 — No machine-readable contract artifacts (IDL) to generate SDKs, connectors, and the conformance harness from

**Severity:** Critical · **Type:** Ecosystem · **Complexity:** high

**Finding.** Every ecosystem instrument — SDKs, connectors, the conformance harness, a marketplace
validator, cross-runtime interop tests — consumes *contracts*. In ARVES v1.0 the contracts exist
only as prose:

- The Universal Cognitive Ontology Specification defines `uci.<type>@<version>` and a registry entry
  shape `{ urn, version, aspects, schema, relations }` (Part 9) — but there is **no frozen file
  format** for that schema, no JSON Schema / Protobuf / CDDL / RDF serialization, and no published
  registry document enumerating the 18 root types with their field-level schemas.
- The Engine Graph Specification defines the manifest *fields* (TABLE 0) and says the manifest is
  "content-addressable and versioned" and "the analogue of an OCI image manifest" (Parts 9–10) — but
  gives **no serialized form, no schema, no content-addressing algorithm** (which hash? over what
  canonical byte form?).
- ARVES-20/21/22/24 (Service/Event/Data/API catalogs) are **one-line inventories** ("Knowledge API,
  Ontology API, …"), not machine-readable OpenAPI/AsyncAPI documents. The Event Catalog even defines
  a "Canonical Event Envelope" (`event_id, event_type, tenant_id, …`) in prose but not as a schema.

**Why it matters.** An SDK generated by hand drifts from the spec the moment a maintainer is tired;
an SDK generated from a frozen IDL cannot drift. The entire ARVES value proposition — "an independent
team, given only this specification, can build a runtime that interoperates" (Ontology Part 11,
Engine Graph Part 13) — is only *mechanically* true if the type vocabulary is a file, not a
paragraph. Certified Kubernetes works because Sonobuoy runs a *test binary*, not because vendors read
prose; the OCI ecosystem works because the image manifest is a *JSON schema with a digest algorithm*.
ARVES explicitly cites OCI and Sonobuoy as precedent (Engine Graph Part 1; Conformance Framework
Part 1) but has not produced their load-bearing artifact: the machine-readable contract.

**Risks / long-term consequences.** Without this, three failure modes are near-certain over 20 years:
(1) *dialects* — each SDK/runtime encodes a slightly different interpretation of `uci.fact@1`,
producing silent interop breaks that conformance cannot catch because conformance itself has no
canonical schema to check against; (2) *bit-rot of "independent implementability"* — the acceptance
bar becomes aspirational because no one can actually regenerate the type system from a file; (3) *ISO
rejection* — a standards body will ask "where is the normative machine-readable schema?" and there is
no answer.

**Alternative designs.**
- *A. Single IDL, multiple projections.* Author one canonical schema language (JSON Schema 2020-12 +
  a small ARVES profile, or CDDL, or Protobuf) as the frozen artifact; generate OpenAPI, AsyncAPI,
  and language bindings from it. Pro: one source of truth. Con: picks a serialization winner early.
- *B. Layered IDL.* Ontology types in a schema-neutral core (RDF/SHACL or CDDL) + transport bindings
  (OpenAPI/AsyncAPI/gRPC) generated per protocol. Pro: transport-agnostic, most ISO-friendly. Con:
  more tooling.
- *C. Keep prose, add a "reference schema" companion (non-normative).* Cheapest, but re-creates the
  drift problem the ontology registry was invented to kill.

**Recommendation.** Pursue **B**, delivered as a *Runtime + Verification* artifact that does **not**
alter the frozen spec: publish `arves-ontology-registry-v1` (the 18 `uci.*` types + 5 aspects + 8
relations, serialized as SHACL/CDDL) and `arves-engine-manifest-v1.schema` (with a specified
content-addressing algorithm, e.g. SHA-256 over RFC 8785 JCS canonical JSON) as *derived, verifiable
projections* of the frozen prose, published under RT-001's "activate reserved semantics" rule and
pinned to UCS v1.0. Route any genuine ambiguity (e.g. exact field of `uci.observation`) through a
**CCP-with-scenario**, never a silent schema decision. Sequence this **first** — E5/E6/E7 all depend
on it.

**Implementation complexity:** high (the schemas are small, but freezing the *canonicalization and
content-addressing* rules correctly is subtle and must be conformance-tested).

**Scientific impact.** High. A machine-checkable cognitive-type ontology with provenance/trust
aspects, versioned and content-addressed, is a genuine research artifact — it makes ARVES the first
cognitive-infrastructure standard whose *semantics* are executable, not just its runtime.

**Ecosystem impact.** Foundational. This is the keystone; SDKs (E5), connectors (E6), marketplace
(E7), and cross-runtime interop (E4) are all generated from or validated against it.

---

## E2 — No standard-ownership charter, CCP adjudication authority, or IPR/trademark regime

**Severity:** Critical · **Type:** Ecosystem · **Complexity:** high

**Finding.** The corpus repeatedly presumes an owner without ever constituting one:

- Ontology Part 9: "Governance: single owner, changelog." Conformance Framework Part 11: "The suite
  has a single owner and changelog." Reference Lifecycle Part 6: "All changes flow through a CCP …
  Owner review; scope agreed" (TABLE 2, `Accepted` state).
- Certification Manual Part 15 requires "third-party certification available (arms-length review
  board, not the reference authors)" and Part 13 wants vendor-independence — yet no board exists and
  none is chartered.
- The Baseline (Part 3) and Freeze Record (Part 4) **defer** "Community/Ecosystem programs" and
  "Enterprise Governance" to v2.

Nowhere in the corpus is there: a named legal custodian (foundation/consortium/SDO), a membership or
seat model, a documented CCP *adjudication* procedure (who breaks a tie? what is quorum? what is the
appeal path?), an IPR policy (RAND vs royalty-free), a patent non-assertion covenant, a trademark
policy for the "ARVES"/"Certified ARVES" marks, or an antitrust-safe process for competing vendors to
co-govern.

**Why it matters.** A standard's long-run survival is a *governance* property, not a technical one.
IETF, W3C, OASIS, Linux Foundation, and ISO all exist primarily to answer "who decides, and under
what IP terms?" ARVES explicitly models itself on W3C Process / IETF RFC 2026 / KEP (Reference
Lifecycle Part 1) — but adopted their *process shape* without their *institution*. The moment ARVES
is public and has two independent runtimes (a stated goal), the first hard CCP with commercial stakes
will have no legitimate adjudicator, and the standard forks. The freeze record having a single
"signature page" but no signing *body* is precisely the ISO/IEEE-blocking gap.

**Risks / long-term consequences.** (1) *Vendor capture* — if the reference-implementation authors
are also the de-facto CCP owner, "arms-length review" is a fiction and adopters treat ARVES as a
single-vendor product, not a standard. (2) *Fork on first dispute.* (3) *IP landmine* — without a
patent covenant, an implementer can be sued for practicing the standard, which is fatal to adoption.
(4) *Trademark dilution* — "ARVES-certified" becomes meaningless if anyone can claim it.

**Alternative designs.**
- *A. Foundation model (Linux Foundation / CNCF style).* Neutral non-profit holds the trademark and
  IP, technical steering committee (TSC) adjudicates CCPs, royalty-free patent grant. Best fit for a
  runtime-centric ecosystem; matches the Kubernetes precedent ARVES already cites.
- *B. Formal SDO track (ISO/IEC JTC1 or IEEE-SA).* Highest external legitimacy; slowest; requires a
  submitting national body or IEEE working group. Best pursued *after* a foundation seasons the spec.
- *C. Benevolent-dictator + charter (Python/PEP style).* Fast early, but the corpus's own
  independent-implementability ethos (Reference Lifecycle Part 11) argues against a single human
  authority.

**Recommendation.** Charter **A now, B later.** Draft an **ARVES Foundation Charter** (an *Ecosystem*
instrument, non-normative to UCS, so it does not touch the freeze) that: (1) names the legal
custodian and transfers the "ARVES"/"UCS"/"UCI" trademarks to it; (2) constitutes a Technical
Steering Committee as the *CCP owner* referenced throughout the Reference Lifecycle, with explicit
quorum, tie-break, conflict-of-interest (reference-authors recuse on their own submissions), and
appeal rules; (3) adopts a royalty-free patent non-assertion covenant for conformant implementations;
(4) defines the "Certified ARVES" certification mark and who may use it. Then, once two independent
runtimes are certified (a corpus goal), open an **IEEE-SA study group / ISO liaison** using the
foundation as the submitting entity. Nothing here amends the frozen spec; it *instantiates* the owner
the spec already assumes.

**Implementation complexity:** high (legal + organizational, not technical, but genuinely hard).

**Scientific impact.** Medium — governance is not research, but a credible neutral custodian is the
precondition for academic and national-lab participation.

**Ecosystem impact.** Foundational and blocking. Every CCP, every certification, every trademark use,
and any ISO/IEEE path routes through this.

---

## E3 — Certification is well-defined but not operationalized as an accredited independent-lab program

**Severity:** High · **Type:** Certification · **Complexity:** high

**Finding.** Volume 6 (Certification & Review Manual) is genuinely strong: L1–L4 + Certified Product,
property-based verdicts, the immutable replayable conformance artifact (Part 7), the Independent
Architecture Review procedure (Part 9), and the A/B parity goals (Part 13). But it defines *how to
judge* a submission, not *how a certification-issuing institution operates*. Missing:

- **Lab accreditation.** Who accredits an "arms-length review board"? What are the requirements to
  *be* a certification lab (independence tests, reproducibility audit, insurance)? ISO/IEC 17025-style
  lab accreditation has no analogue here.
- **Self-cert vs audited tiers.** Certified Kubernetes allows vendor self-certification against a
  public test suite; ARVES demands a human Independent Architecture Review (Part 9) for *every* level,
  which does not scale and is not defined as a paid/staffed service.
- **The conformance suite as a shippable, versioned binary.** The suite is defined (12 axes,
  reference scenarios) and the crate is a skeleton, but there is no distributable "run this to get
  your artifact" tool, no public results registry, and no certification-mark issuance workflow.
- **Cross-verifier requirement is stated but not built.** Part 13 requires that "a verifier built for
  one [runtime] can replay the other" — an excellent interop test — but there is no reference verifier
  and no cross-verification conformance scenario.

**Why it matters.** Certification is the *revenue and trust engine* of a standards ecosystem and the
mechanism that keeps implementations honest. Deferring the "Certification Program launch" (Freeze
Record TABLE 0) is defensible, but the *operational design* of the program (accreditation, tiers,
suite-as-artifact, results registry) is itself an Implementation-Era deliverable that gates I-series
milestone sign-off (every milestone "Definition of Done" requires "Certification PASS," AEOS Part 3).

**Risks / long-term consequences.** (1) *Certification bottleneck* — mandatory human review per level
means throughput is bounded by a tiny reviewer pool; the ecosystem cannot scale past a handful of
runtimes. (2) *Reviewer inconsistency* — without accredited, calibrated labs, two reviewers reach
different verdicts and the mark loses meaning. (3) *No public trust signal* — adopters cannot verify a
"Certified ARVES" claim without a public results registry.

**Alternative designs.**
- *A. Two-tier: automated self-cert (mechanical suite) + audited (adds Independent Architecture
  Review).* Mirrors CNCF. Scales; L1/L2 self-cert, L3/L4/Product audited.
- *B. Fully audited only.* Highest assurance, does not scale.
- *C. Fully automated only.* Scales, but drops the adversarial architecture review the corpus rightly
  prizes.

**Recommendation.** Build **A** as a *Certification* + *Verification* program: (i) ship the
conformance suite as a versioned, replayable binary (`arves-conformance` fleshed out) that emits the
Part-7 artifact and pins UCS/suite versions per Reference Lifecycle Part 8; (ii) publish a
**public conformance-results registry** keyed by runtime identity + artifact digest; (iii) define an
**ARVES Lab Accreditation** scheme (independence, reproducibility re-run of a submitted artifact,
recusal rules) owned by the E2 Foundation TSC; (iv) build the **reference cross-verifier** and add a
"Cross-Verify-A-and-B" conformance scenario (via CCP-GATE) to make Part-13 parity executable. Keep
human Independent Architecture Review mandatory only for L3+/Product; allow mechanical self-cert for
L1/L2 with random audit.

**Implementation complexity:** high (suite implementation + registry + accreditation policy + legal).

**Scientific impact.** High — a public, replayable, property-based certification corpus for cognitive
runtimes would be a first-of-kind reproducibility asset.

**Ecosystem impact.** High — this is how "Independent Runtime A/B" and "third-party certification"
(corpus goals) actually happen instead of remaining aspirational.

---

## E4 — Version evolution policy lacks executable compatibility suites, LTS, and migration tooling

**Severity:** High · **Type:** Verification · **Complexity:** high

**Finding.** The versioning *policy* is well-articulated: SemVer at the standard level (Reference
Lifecycle Part 7), two-track UCS/UCI lines (Part 8), the change-type→version-effect table (TABLE 3:
breaking→MAJOR, additive→MINOR amendment, clarify→PATCH, removal→deprecate-one-major-then-MAJOR), and
suite-pinned results ("N% at Level Lx against Spec vB / Suite vA"). Engine contracts are SemVer'd and
"a graph pins engine versions" (Engine Graph Part 11). This is a good *policy*. What is missing is the
*machinery* that makes compatibility a verifiable property rather than a promise:

- No **backward-compatibility test suite** that, given UCS vN and vN+1, mechanically proves a
  vN-conformant runtime still passes the vN scenario set — the corpus asserts "upgrade path preserves
  conformance" (Cert Manual Part 12 checklist) but provides no executable check.
- No **LTS / support-window policy** — how long is a frozen major supported? Kubernetes, Java, and
  Node all fail without this; ISO standards have review cycles.
- No **migration tooling or codemod story** for the deprecate-one-major-then-MAJOR path (TABLE 3).
- No **registry-namespace authority** for `uci.*` versions — who assigns `uci.fact@2`, and how are
  collisions prevented across vendors extending the ontology?

**Why it matters.** A 20-year standard lives or dies on compatibility discipline. The policy is
correct but unenforced; the difference between "we promise backward compatibility" and "here is the
suite that fails your build if you break it" is the difference between SemVer-in-name and
SemVer-in-fact. ISO/IEEE reviewers will specifically probe the maintenance and review cycle.

**Risks / long-term consequences.** (1) *Silent breakage* — a MINOR amendment quietly breaks a
runtime because nothing tests the claim; (2) *ontology namespace chaos* — vendors mint conflicting
`uci.*@n` types once extension is allowed; (3) *upgrade paralysis* — with no LTS, adopters freeze on
an unsupported version and fork.

**Alternative designs.**
- *A. Compatibility as conformance.* Add cross-version scenarios to the suite (vN artifact must
  replay under vN+1 verifier) — reuses the ORCH-003 replay machinery ARVES already has.
- *B. Separate compat-test tool.* Standalone, more work, less integrated.
- *C. Policy-only + manual review.* Status quo; does not scale.

**Recommendation.** Pursue **A** plus an **IDR + Ecosystem** package: (i) an IDR fixing the LTS
window and deprecation timeline for UCI (implementation-era decision, does not touch UCS); (ii) a
**CCP-gated compatibility scenario class** ("vN artifact replays under vN+1") that turns the Cert
Manual Part-12 upgrade checklist into an executable test; (iii) a **`uci.*` namespace registry**
operated by the E2 Foundation, with a reserved vendor sub-namespace (`uci.x.<vendor>.*`) so ecosystem
extensions never collide with core types; (iv) publish migration guides and codemods as ecosystem
deliverables per major bump. Leverages the existing replay/decision-trace substrate (IDR-003, WAL as
decision trace) rather than inventing new machinery.

**Implementation complexity:** high (cross-version harness is non-trivial; namespace registry is
organizational).

**Scientific impact.** Medium — executable backward-compatibility for a cognitive standard is novel.

**Ecosystem impact.** High — this is what lets the ecosystem *evolve* without shattering, the literal
20-year question.

---

## E5 — No SDK strategy: which languages, generated from what, and the "cannot bypass the Kernel" guarantee

**Severity:** High · **Type:** Ecosystem · **Complexity:** high

**Finding.** The corpus references SDKs only obliquely: the Certification Manual Part 12 checklist
demands "SDK and API surface do not permit clients to write committed truth directly (only through
the Kernel commit path)" and Part 15 lists "SDKs that cannot bypass the Kernel commit path" as an
ecosystem goal; Vol 3 Part 25 mentions a "Provider SDK." There is **no SDK strategy**: no target
language set, no statement of what the SDK is generated *from*, no design for how the "cannot bypass
the Kernel" invariant is *enforced* in a client library, and no published crate (all runtime crates
are `publish = false`, `version = "0.0.0"`).

**Why it matters.** For most developers, *the SDK is the standard* — they will never read the
Ontology spec; they will `import arves`. If SDKs are hand-written they drift from the frozen contracts
(E1) and the "cannot write truth directly" guarantee becomes a code-review convention rather than a
structural property. The corpus's own architecture (ORCH-001: only the Kernel commits truth; Engine
purity; Query read-only) is exactly what an SDK must *encode by construction*: a well-designed SDK
should make it *impossible to express* a direct-truth-write, mirroring the runtime's commit-gateway.

**Risks / long-term consequences.** (1) *Invariant erosion at the edge* — an SDK that exposes a raw
write path silently violates ORCH-001 in every app built on it, and conformance (which tests
*runtimes*, not client apps) never sees it. (2) *Drift* — hand-written SDKs across N languages become
N dialects. (3) *Adoption drag* — no SDK means every adopter reimplements the client, and ARVES stays
a spec-on-paper.

**Alternative designs.**
- *A. Generated-from-IDL SDKs (depends on E1).* Types generated from the ontology registry; API
  clients generated from OpenAPI/AsyncAPI; a thin hand-written "safety layer" that only exposes the
  Kernel commit path and read-only Query. Pro: no type drift; the safety layer is small and auditable.
- *B. Fully hand-written idiomatic SDKs.* Best DX, worst drift; only viable for 1–2 flagship
  languages with heavy investment.
- *C. Single Rust SDK + FFI.* Guarantees one truth surface, but poor idiomatic DX per language.

**Recommendation.** **A**, sequenced after E1. Prioritize languages by ecosystem gravity:
**TypeScript** (agent/app developers), **Python** (AI/ML + data), **Rust** (matches the reference
runtime, systems integrators), then **Go/Java** (enterprise). Generate ontology types and API/event
clients from the E1 IDL; hand-write only the *narrow* commit/query safety layer whose types make a
direct-truth-write *unrepresentable* (e.g. the only write API returns a `ProposedEffect`, never a
committed `Fact`, mirroring the kernel's commit-gateway design). Add a conformance-style **SDK
contract test** (Verification) that asserts no SDK exposes a truth-write path — turning the Cert
Manual Part-12 line item into an executable check. Publish crates/packages under the Foundation's
namespace with the two-track version (SDK targets UCS vX, as the corpus says "a UCI runtime declares
which UCS version it implements," Reference Lifecycle Part 8).

**Implementation complexity:** high (generation pipeline + per-language safety layer + contract
tests).

**Scientific impact.** Medium — "invariant-preserving-by-construction client libraries" is a
publishable idea.

**Ecosystem impact.** Very high — the SDK is the primary adoption surface and the primary risk of
invariant erosion.

---

## E6 — Connector ecosystem: a framework exists, but no signing/provenance/certification instrument

**Severity:** Medium · **Type:** Ecosystem · **Complexity:** medium

**Finding.** ARVES has an unusually good *conceptual* connector story: Vol 3 (Information Core) defines
Provider Model, Provider Registry, Connector Framework (Pull/Push/Streaming/Hybrid), Schema
Intelligence, and a Provider SDK (Parts 5–9, 25); the Engineering Handbook Part 8 correctly frames a
connector as a top-of-stack Data-Plane element that "is not a back door into the Kernel," must be
idempotent/replay-safe, and must map to the canonical ontology. What is missing is the *ecosystem
instrument*: connectors are the highest-risk third-party artifact (they ingest untrusted external
data and can poison downstream truth), yet there is no connector **signing, provenance, versioning,
certification, or revocation** scheme, and no connector conformance scenario beyond the generic
"add a conformance scenario for the connector's contract" (Handbook Part 8, which is non-normative).

**Why it matters.** Connectors are where the outside world touches ARVES. The ontology's provenance
and trust aspects (Ontology TABLE 1) are precisely designed to carry connector trust — but nothing
binds a *specific connector build* to a *provenance identity* or lets a tenant *revoke* a compromised
connector. In a public ecosystem this is a supply-chain attack surface (cf. dependency-confusion,
malicious npm packages).

**Risks / long-term consequences.** (1) *Truth poisoning* — a malicious/buggy connector injects
observations that, once validated, become committed facts; (2) *no revocation* — a compromised
connector cannot be pulled ecosystem-wide; (3) *provenance gaps* — the ontology promises provenance
but nothing enforces that a connector *populates* it correctly.

**Alternative designs.**
- *A. Signed, content-addressed connector manifests (mirror the Engine Manifest / OCI model).*
  Connector = signed manifest declaring source, canonical-ontology mappings, and required scopes;
  registry supports revocation. Pro: reuses E1 content-addressing + E7 signing substrate.
- *B. Curated-only connectors (first-party).* Safe, but kills the ecosystem's breadth.
- *C. Sandbox-only, unsigned.* Isolation without identity; provenance still weak.

**Recommendation.** **A**, layered on E1 + E7: define a **Connector Manifest** (content-addressed,
signed, declaring provider identity, canonical `uci.*` output mappings, and requested tenant scopes)
in the Foundation's connector registry, with a **revocation list** and a **mandatory
provenance-population conformance scenario** (via CCP-GATE) that FAILS a connector which emits
observations lacking the Provenance aspect. This makes the ontology's provenance promise
*enforceable at the ingest boundary* — the exact place the Handbook says connectors must not become a
Kernel back door.

**Implementation complexity:** medium (registry + signing reuse E7; the provenance scenario is small).

**Scientific impact.** Medium — verifiable-provenance-at-ingest for a cognitive system is a strong
safety story.

**Ecosystem impact.** High — connectors are the breadth of the ecosystem; safe connectors are what
make ARVES usable against real enterprise data.

---

## E7 — Marketplace deferred with no trust substrate (capability/engine signing, revocation, entitlement)

**Severity:** Medium · **Type:** Ecosystem · **Complexity:** high

**Finding.** Marketplace is explicitly deferred to v2 (Baseline Part 3, Freeze Record Part 4), yet the
corpus already assumes marketplace surfaces exist: "Agent Marketplace API/Service" (ARVES-24/20),
"Marketplace of certified capabilities and products" (Cert Manual Part 15), Long-Term Objective #8
("Marketplace exists," CLAUDE.md). The substrate for a *trustworthy* marketplace partly exists —
engine manifests are "content-addressable and versioned" (Engine Graph Part 9), capabilities are
registered with contracts (Capability Atlas Parts 16–21) — but the marketplace-specific instruments
do not: no artifact **signing/attestation**, no **certification-gated listing** (only certified
capabilities/products may be listed), no **revocation/recall**, no **entitlement/licensing/billing**
model, and no **tenant-scoped trust policy** (which marketplace artifacts a tenant permits).

**Why it matters.** A marketplace without a trust substrate is a malware distribution channel. The
value of "certified capabilities" (Cert Manual Part 15) is only realizable if the marketplace
*enforces* certification as a listing gate and can *revoke* a listing when a defect/CVE emerges.

**Risks / long-term consequences.** (1) *Supply-chain compromise* at ecosystem scale; (2)
*certification theater* — "certified" listings that were never actually gated; (3) *no recall path*
for a vulnerable capability already bound in production plans.

**Alternative designs.**
- *A. Sigstore-style keyless signing + transparency log over content-addressed manifests.* Modern,
  auditable, reuses E1 content-addressing. *B. Classic PKI signing.* Simpler, key-management burden.
  *C. Curated closed marketplace first.* Safe bootstrap, limited breadth.

**Recommendation.** Since Marketplace is a v2 scope item, treat this as a **v2 Ecosystem design
started now**: specify a **marketplace trust substrate** (signed, content-addressed capability/engine
manifests + transparency log + revocation + certification-gated listing + tenant trust policy) as a
foundation for the deferred Marketplace, built on E1 (content-addressing), E3 (certification gate),
and E6 (connector precedent). Bootstrap with a curated/closed marketplace (C) at GA and open it once
signing + revocation are proven. Nothing here reopens the freeze; it prepares the deferred v2 item so
that when it lands it is safe by construction.

**Implementation complexity:** high.

**Scientific impact.** Low-medium (applied supply-chain security).

**Ecosystem impact.** High — the marketplace is a headline Long-Term Objective; without trust it is a
liability rather than an asset.

---

## E8 — Developer experience: no distributable dev runtime, conformance-as-a-service, or golden path

**Severity:** Medium · **Type:** Product · **Complexity:** medium

**Finding.** There is no "get started in 10 minutes" path. The reference runtime crates are
`publish = false`, `version = "0.0.0"`, path-linked, with only kernel/persistence/conformance
partially fleshed and the rest skeletons (`wc -l` shows ~250–700 lines per crate, conformance marked
"NO implementation yet"). There is no single-binary local dev runtime, no container image, no CLI
(`arves run scenario …`), no local conformance runner a developer can invoke, and the Deployment
Atlas (Vol 18) lists "Single Node … Edge" deployment models in prose only.

**Why it matters.** Adoption is dominated by time-to-first-success. Kubernetes exploded partly because
`minikube`/`kind` gave a local cluster in one command and Sonobuoy gave conformance in one command.
ARVES's own strongest asset — property-based conformance — is inaccessible without a runnable local
harness. The Engineering Handbook Part 3 ("How to Run Conformance") describes the *concept* but there
is no tool to actually run.

**Risks / long-term consequences.** (1) *Adoption drag*; (2) *conformance stays theoretical* because
no one can run it locally; (3) *the "independent implementability" acceptance bar (repeated in every
spec)* is never exercised because no reference dev runtime demonstrates it.

**Alternative designs.**
- *A. Single-binary dev runtime + CLI + container image + local conformance runner.* Best DX.
- *B. Hosted-only sandbox.* Lower friction to *try*, but no offline/CI story. *C. Docs-only golden
  path.* Cheapest, weakest.

**Recommendation.** **A**, as a *Product* deliverable riding the I-series: ship `arves-dev` (a
single-node runtime binary + OCI image), an `arves` CLI (`arves scenario run`, `arves conformance
report`, `arves ontology validate <manifest>`), and **conformance-as-a-service** (a hosted endpoint
that ingests a conformance artifact and returns a verdict + registry entry, tying into E3). Provide a
documented **golden path**: install → run a reference scenario → read the Part-7 artifact →
understand a verdict. This makes the frozen theory *tangible* for the first time.

**Implementation complexity:** medium (mostly packaging + CLI over the runtime that must exist anyway
for the I-series).

**Scientific impact.** Low.

**Ecosystem impact.** High — DX is the top-of-funnel for every other ecosystem instrument.

---

## E9 — No training/curriculum or human-certification program to build the practitioner base

**Severity:** Medium · **Type:** Ecosystem · **Complexity:** low

**Finding.** The corpus has excellent *internal* onboarding (Engineering Handbook Part 1's 30-minute
onboarding; AEOS as the routing OS) but nothing *external*: no curriculum, no "ARVES Certified
Architect/Engineer" human-certification, no reference courseware, no worked example catalog beyond the
abstract reference scenarios. ARVES's conceptual model (Kernel-owns-truth, Control-Plane-decides,
downward-only layers, property-based conformance) is *unusual* and has a real learning curve; without
teaching material, the practitioner pool stays at the size of the original authors.

**Why it matters.** Standards spread through people. ISO/IEEE, AWS, Kubernetes (CKA/CKAD), and every
durable ecosystem invest heavily in human certification and curriculum precisely because a certified
practitioner base is what makes enterprises comfortable adopting. The unusual ARVES mental model
*increases* this need.

**Risks / long-term consequences.** (1) *Bus-factor* — knowledge stays with the authors; (2)
*mis-implementation* — practitioners violate ORCH-001/LAYER-001 because no one taught them; (3) *slow
enterprise adoption* — no hireable "ARVES engineers."

**Alternative designs.** *A. Foundation-owned curriculum + human cert exams (CKA model).* *B.
Community-authored, un-curated.* *C. Vendor-specific training only* (risks single-vendor framing).

**Recommendation.** **A**, low-cost and high-leverage: the E2 Foundation publishes a **layered
curriculum** (Concepts → Building on ARVES → Implementing a Runtime → Certifying) reusing the
Handbook's onboarding and the reference scenarios as labs, plus an **"ARVES Certified" human exam**
whose questions are drawn from the frozen invariants and the conformance axes. Because the material
already exists in the corpus, this is mostly repackaging.

**Implementation complexity:** low.

**Scientific impact.** Low.

**Ecosystem impact.** Medium-high over the long run — the practitioner base is the ecosystem's compounding asset.

---

## E10 — Community process (contribution, RFC forum, working groups) undefined beyond the CCP shell

**Severity:** Low · **Type:** Ecosystem · **Complexity:** low

**Finding.** The CCP is a solid *change-proposal state machine* (Reference Lifecycle TABLE 2:
Proposed→Accepted→Formalized→Conformance-defined→Ratified→Frozen) and the Engineering Handbook Part 5
tells an insider *when* to open one. But the *community mechanics* around it are undefined: no public
CCP forum/repository, no working-group structure, no code of conduct, no contributor onboarding for
*external* (non-author) participants, no roadmap-input process, and no defined path for a community
member to *become* a maintainer. "Community/Ecosystem programs" are deferred to v2 (Baseline Part 3).

**Why it matters.** The Reference Lifecycle's own acceptance bar (Part 11) is that "an independent
team, given only this document, can propose, formalize, conformance-test, ratify and certify a change
… without consulting the original authors." That is impossible without a public venue and a
documented contributor path. The process is described; the *place to do it* does not exist.

**Risks / long-term consequences.** (1) *Closed-shop perception* — CCPs happen in private, killing
the neutral-standard story; (2) *no succession* — no path from contributor to maintainer; (3)
*roadmap opacity* — the community cannot influence direction, so it forks.

**Alternative designs.** *A. Public CCP repo + working groups + CoC + maintainer ladder (CNCF/W3C
model).* *B. Mailing-list/RFC (IETF model).* *C. Keep private until v2.*

**Recommendation.** **A**, cheap and immediate under the E2 Foundation: a **public CCP repository**
(the RFC/KEP venue the Reference Lifecycle already implies), **topic working groups** (ontology,
runtime, conformance, connectors), a **code of conduct**, and a documented **maintainer ladder**.
This operationalizes the independent-implementability acceptance bar the corpus already commits to,
without touching the frozen spec.

**Implementation complexity:** low (process + repo, not code).

**Scientific impact.** Low.

**Ecosystem impact.** Medium — this is what converts "a spec with a change process" into "a living
standard with a community."

---

## Sequencing (the 20-year build order)

The instruments have a strict dependency order; building visible surface before substrate guarantees
drift and forks.

1. **E2 — Foundation Charter + IPR/trademark + CCP adjudication authority.** *Nothing legitimate
   happens without an owner.* Blocks E3, E4-namespace, E7-signing, E9, E10.
2. **E1 — Machine-readable contract registry (ontology IDL + engine manifest schema +
   content-addressing).** The keystone artifact. Blocks E5, E6, E7.
3. **E3 — Certification program: suite-as-binary + results registry + lab accreditation + cross-verifier.**
   Turns the frozen Certification Manual into a running program; enables Independent Runtime A/B.
4. **E8 — Dev runtime + CLI + conformance-as-a-service (golden path).** Makes everything above
   tangible; top-of-funnel for adoption.
5. **E5 — Generated SDKs (TS, Python, Rust, then Go/Java) with the by-construction Kernel-safety layer.**
6. **E6 — Connector manifests + signing + provenance conformance + revocation.**
7. **E4 — Executable compatibility suites + LTS IDR + `uci.*` namespace registry + migration tooling.**
8. **E7 — Marketplace trust substrate (v2 design started now; curated bootstrap at GA).**
9. **E9 / E10 — Curriculum + human certification; public CCP venue + working groups + maintainer ladder.**

---

## If ARVES were standardized by ISO/IEEE tomorrow, what is still missing (ecosystem lens)

An ISO/IEC or IEEE-SA reviewer would accept the *normative prose* as unusually mature but would
**block ratification** on ecosystem grounds for four reasons, none of which require touching the
frozen spec: **(1)** there is no named custodian/SDO owner with a documented decision procedure and an
IPR/patent policy (E2); **(2)** the normative type system and ABI exist only as prose, with no
machine-readable schema artifact that implementers and the conformance suite consume (E1) — an SDO
requires the normative machine-readable annex; **(3)** the conformance suite is not a shippable,
versioned, publicly runnable test artifact with a results/accreditation regime, so "conformance" is
not yet independently verifiable in practice (E3); and **(4)** there is no executable
backward-compatibility regime, LTS/maintenance-cycle policy, or namespace authority, so the 20-year
evolution story is a promise rather than a mechanism (E4). Everything ARVES needs to clear these bars
is *instrumentation of decisions the corpus already made* — the freeze is an asset here, not an
obstacle.

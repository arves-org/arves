# ARVES Gate Review — Prompt 1: Independent Runtime Challenge

**Lens:** Independent Runtime Challenge — can a different company, given ONLY the frozen specs (no source access), build a *conformant, interoperable* ARVES runtime?
**Reviewer role:** Independent Chief-Architect-level GATE reviewer (ISO/IEEE-grade standardization readiness).
**Scope reviewed:** 50 frozen specification documents (`runtime/review-input/*.txt`) + the reference runtime (`runtime/crates/*/src/*.rs`) for implementation-leak evidence.
**Date:** 2026-07-02
**Constraint honored:** No finding proposes modifying the frozen specification. Every remedy is an IDR, CCP Amendment, Runtime work item, Verification work item, or Certification-program change.

---

## Executive Summary

ARVES is, today, a **vision-and-governance corpus**, not an implementable engineering standard. The authors already know this: their own `ARVES_00_Gap_Analysis_v1` states the corpus is *"roughly 90% naming & structure, 10% content ... NOT yet an implementable specification"* and that *"not a single JSON event, API request/response body, or example record exists."* This review confirms that assessment from the specific angle that matters most for a real standard — **could a rival team reproduce a runtime that interoperates with ARVES from the documents alone?** The answer is **no**, and the reasons are structural, not cosmetic.

The core problem is a **contract-depth inversion**. ARVES has invested enormous rigor in the *meta-layer* — the change process (Reference Lifecycle), the conformance harness shape (Scenario Conformance Framework), the invariant registry, the era/freeze discipline, the certification review manual. All of that is genuinely good and is the scaffolding a 20-year standard needs. But the *object-layer* — the actual bytes on the wire, the type schemas, the hash and canonicalization algorithms, the API request/response shapes, the event payloads, the manifest serialization — is almost entirely absent. A standard is only as interoperable as its narrowest under-specified contract, and ARVES's narrowest contracts are one-line prose.

Three facts make this a **GATE-blocking** condition for independent implementability:

1. **The load-bearing invariant, ORCH-004 ("idempotent and content-addressable"), has no defined content-addressing.** The corpus repeats "content-addressable" 40+ times but never once specifies the hash function, the pre-image, or the canonicalization that produces it. The reference `arves-kernel` and `arves-engine-fabric` crates openly admit this: *"The byte layout of the hash is intentionally unspecified,"* *"payload shape is intentionally left opaque."* Two independent teams will content-address differently, so their idempotency keys, dedupe behavior, and decision traces will never agree.

2. **Conformance is defined so that it cannot detect divergence.** The Certification Manual (Vol 6 Part 2) makes verdicts *"property-based, not golden-output"* and evaluates only 7 structural invariants (ownership, layering, replay-from-trace, tenant partition). Two runtimes with incompatible wire formats, incompatible types, and incompatible traces can **both PASS**. Conformance proves internal discipline, not cross-implementation interoperability — the exact property a standard exists to guarantee.

3. **The type system that "closes the ABI loop" is empty.** The Ontology Spec declares that `uci.*` URNs resolve Reads/Produces/Writes and that a registry entry is `{ urn, version, aspects, schema, relations }` — but the `schema` field is never populated for any type. ARVES-19 gives all 16+ entities the *identical* seven-attribute stub. There is no type an independent runtime could actually serialize, validate, or exchange.

The corpus is also **internally honest about its own descriptiveness**: three separate frozen specs (Engine Graph Part 13, Ontology Part 11, Reference Lifecycle Part 11) each contain an "Independent-Implementability Test" that says, in effect, *"if an outside team cannot build this from the document alone, the document is still descriptive and must be made more normative."* By the corpus's own bar, they fail their own test.

**Gate verdict: NOT-READY.** This is not a rejection of the architecture — the two-plane model, single-ownership, CP-truth/AP-observability split, and replay-from-trace are sound and defensible design choices. It is a statement that the specification, as frozen, is **not yet a specification an independent party can implement to interoperate**. The remedies below are all achievable *without reopening the freeze* — they are CCP Amendments (MINOR, backward-compatible additions of the missing normative detail) and IDR/Runtime/Verification/Certification work. The Reference Lifecycle explicitly provides for exactly this. But until the object-layer contracts exist, deeper reviews (performance, security, scale) would be reviewing prose, and the Wave-1 gate should not clear to standard-ready.

---

## Severity-Ranked Finding Table

| # | Severity | Title | Type | Impl. Complexity |
|---|----------|-------|------|------------------|
| F1 | Critical | ORCH-004 content-addressing is undefined (no hash, no pre-image, no canonicalization) | CCP-Amendment | high |
| F2 | Critical | No wire format / serialization for any contract; wire format explicitly delegated to "IDR" (i.e. out of the standard) | CCP-Amendment | high |
| F3 | Critical | The `uci.*` type registry has no schemas — the ABI "type loop" closes on empty types | CCP-Amendment | high |
| F4 | Critical | Conformance cannot detect cross-implementation divergence (property-based, 7 structural invariants only) | Certification | very-high |
| F5 | High | "Replayable decision trace" (ORCH-003) has no defined trace format or replay determinism contract | CCP-Amendment | high |
| F6 | High | Event Envelope & event payloads have field names but no types, encoding, or versioning rules | CCP-Amendment | medium |
| F7 | High | API Catalog lists names only — no paths, verbs, request/response schemas, status/error models | CCP-Amendment | medium |
| F8 | High | Engine Manifest "serialized form" is named but never serialized; portability guarantee is unbacked | CCP-Amendment | high |
| F9 | Medium | Arbitration / join-node semantics ("confidence-weighted merge, tie-break") are undefined and non-deterministic | CCP-Amendment | high |
| F10 | Medium | Graph termination policy ("max depth / budget / no-new-subgoal") has no normative bounds or algorithm | IDR |  medium |
| F11 | Medium | Canonicalization ("Any Information → Canonical") is the platform's central promise and is entirely unspecified | CCP-Amendment | very-high |
| F12 | Medium | Read-consistency tiers named but their observable guarantees (staleness bound, read-index) are unspecified | IDR | medium |
| F13 | Medium | Proposed invariants (CAP-*, ENG-*, G/QUERY/LCW/PERSIST-001) are what an implementer needs, yet carry no conformance weight | CCP-Amendment | medium |
| F14 | Low | Corpus version drift + a superseded-but-unreplaced canonical model (ARVES-19 vs Ontology Spec) leave the authoritative source ambiguous | CCP-Amendment | low |

---

## Findings

### F1 — Critical — ORCH-004 content-addressing is undefined (CCP-Amendment)

**Statement.** ORCH-004 ("every engine and capability invocation is idempotent and content-addressable") is the single most-cited invariant in the corpus (Vol 9 Part 5; Invariant Registry; Engine Graph Part 7; OS Volumes 2–6 reference it 40+ times). It is described as "the true bridge to M9" (distribution). Yet **nowhere in any frozen document is the content-addressing mechanism defined**: no hash function, no digest length, no pre-image definition (what exactly is hashed), and no canonicalization of that pre-image. Engine Graph Table 0 lists "Idempotency Key | Content-addressable key for safe retry" with no derivation. The reference code confirms the gap is deliberate and unresolved: `arves-kernel` says *"The byte layout of the hash is intentionally unspecified at the skeleton stage"* (lib.rs, `ContentHash`), and `arves-engine-fabric` derives `IdempotencyKey(String)` from *"(manifest identity, canonicalized input, read-snapshot)"* — prose, not an algorithm.

**Why it matters.** Content-addressing is the interoperability primitive. If Team A hashes canonical JSON with SHA-256 and Team B hashes CBOR with BLAKE3, the same logical invocation yields different keys. Then: (a) idempotent dedupe at the Kernel commit gateway silently forks truth across implementations; (b) a decision trace produced by A cannot be verified by B; (c) the certification artifact (which claims to be "independently replayable") is only replayable by the runtime that produced it. Every downstream guarantee — replay, distribution, cross-vendor certification — rests on this undefined primitive.

**Risks.** Silent truth-forking between "conformant" runtimes; certification artifacts that are not portable; a future v2 that cannot change the hash without breaking every stored trace (because the choice was never versioned as a standard).

**Long-term consequences.** Twenty years out, this is the difference between "ARVES trace" being a portable artifact (like an OCI image digest or a Git commit hash) and being a vendor-private blob. OCI's success rests entirely on a *specified* digest (`sha256:...`) with defined canonical serialization. ARVES currently has the OCI ambition (Engine Graph Part 1 cites OCI as precedent) without the OCI digest spec.

**Alternative designs.** (a) Mandate one algorithm (e.g. SHA-256 over a defined canonical form) — simplest, most interoperable, least flexible. (b) Define a small negotiated algorithm set with a self-describing prefix (`sha256:`, `blake3:`) — the OCI/multihash approach; more flexible, still interoperable. (c) Leave it per-runtime — the current state; **not interoperable and should be rejected.**

**Recommendation.** Open a **CCP Amendment (MINOR)** — "ORCH-004 Content-Addressing Contract" — defining: (1) the exact pre-image for engine, capability, and commit content hashes; (2) a canonical byte form for that pre-image (tie to F11); (3) a self-describing digest encoding (multihash-style) with at least one mandatory algorithm; (4) a conformance scenario per CCP-GATE that runs the same logical invocation on two independent code paths and asserts identical keys. This is MINOR because it *adds* normative detail the invariant already implies; it does not change behavior the spec ever defined.

**Implementation complexity:** high. **Scientific impact:** high — turns "content-addressable" from an aspiration into a checkable property. **Ecosystem impact:** high — this is the precondition for portable traces and third-party certification.

---

### F2 — Critical — No wire format; serialization explicitly delegated out of the standard (CCP-Amendment)

**Statement.** No frozen document defines how any ARVES artifact is serialized on the wire or on disk. The Event Envelope (Vol 9 v1 Part 6; ARVES-21) lists fields but no encoding. The Engine Manifest is called "the serialized form" (Engine Graph Part 9) but no serialization is given. Decisive evidence that this is *by design and outside the standard*: `ARVES_OS_Volume_4_Implementation_Playbook_v1` line 306 classifies *"Choosing a replication algorithm, storage engine, or wire format"* as an **IDR** — i.e. a reference-implementation decision, not a normative standard clause. IDRs are explicitly *"NOT a normative standard"* (IDR Batch 1 header) and an independent team reading only the frozen specs never sees them.

**Why it matters.** A standard whose wire format is a reference-implementation detail is, definitionally, not a wire standard. Two independent runtimes cannot exchange a single event, manifest, or trace. This defeats the stated goal (Scenario Conformance Part 14: *"a standard that independent teams can implement, test and certify"*) and the long-term objectives in CLAUDE.md (Independent Runtime A and B, third-party certification, marketplace).

**Risks.** Every integration becomes a bilateral adapter; "the ARVES ecosystem" fragments into per-vendor dialects; the marketplace objective (LONG-TERM OBJECTIVE #8) is unreachable because agents/engines cannot be exchanged as portable manifests.

**Long-term consequences.** Standards that leave the wire to implementations (early SOAP, many "reference" specs) never achieve interop; standards that pin it (HTTP, Protocol Buffers, OCI, CloudEvents) do. ARVES should decide now, because a wire format retrofitted after multiple runtimes exist is a MAJOR-version break.

**Alternative designs.** (a) One mandatory canonical encoding (e.g. canonical CBOR or JSON Canonicalization Scheme, RFC 8785) for hashing + a permitted set for transport — recommended. (b) A CloudEvents-style envelope binding with multiple content-type bindings. (c) Leave to IDR — reject for a standard.

**Recommendation.** **CCP Amendment (MINOR)** — "ARVES Serialization & Envelope Binding" — promoting the wire format from IDR to normative: define the canonical encoding used for hashing (tie to F1), the transport encoding(s), and the envelope binding, each with a worked example payload (the Gap Analysis's top-priority missing artifact). Keep runtime-internal storage layout as an IDR; only the *exchanged* forms need to be normative.

**Implementation complexity:** high. **Scientific impact:** medium. **Ecosystem impact:** critical — no wire format, no ecosystem.

---

### F3 — Critical — The `uci.*` type registry has no schemas (CCP-Amendment)

**Statement.** The Ontology Spec is declared the *root of UCS* and the *"normative dependency that closes the Engine Graph ABI (Reads/Writes/Produces resolve to types defined here)"* (Ontology Part 2, Part 10). It defines URNs (`uci.fact@1`, etc.) and says a registry entry is `{ urn, version, aspects, schema, relations }` (Part 9). But **the `schema` is never given for any type.** Table 2 lists 18 URNs with one-line meanings; there is no field list, no types, no cardinality, no required/optional, no constraints. The predecessor ARVES-19 (which the Ontology Spec supersedes) is worse: all 16+ canonical entities share the *identical* definition and the identical attribute list "Identifier, Name, Description, Status, CreatedAt, UpdatedAt, Metadata" — the Gap Analysis flags this exactly: *"In ARVES-19 all 16 entities share the identical definition, which cannot be correct."* The reference `arves-ontology` crate encodes the *aspects* as traits but leaves per-type schema entirely open (`TypeRegistration` carries no field schema; *"Population of the registry is out of scope for this skeleton"*).

**Why it matters.** The Engine ABI's entire portability claim (Engine Graph Part 13) is that "a runtime needs only the manifest and the Ontology Type Registry" to execute any engine. If the registry has no schemas, the manifest's Reads/Writes/Produces reference *names with no shape*. An independent runtime cannot validate an engine's inputs, cannot serialize its outputs interoperably, and cannot check preconditions. The ABI loop is declared "closed" (Part 10) but closes onto a vacuum.

**Risks.** Every runtime invents its own field set for `uci.fact`, `uci.goal`, etc.; engines are not portable; the Ontology's own Independent-Implementability Test (Part 11: "same types, same relations, same aspects") fails.

**Long-term consequences.** The type registry is the semantic constitution; if it ships empty and runtimes fill it privately, harmonizing them later is a MAJOR break affecting every engine manifest and every stored fact.

**Alternative designs.** (a) Deepen one vertical slice first (the Gap Analysis's own recommendation: fully schema `uci.entity`/Person end-to-end) then template the rest — pragmatic, proves the registry format works. (b) Adopt an existing schema language (JSON Schema / CUE / Protobuf) as the normative schema encoding rather than inventing one. (c) Ship URNs-only — the current state; not implementable.

**Recommendation.** **CCP Amendment (MINOR)** — "uci.* Type Schemas v1" — choosing a normative schema encoding and populating at least the root types (Table 2) and the aspects (Table 1) with real field schemas, cardinalities, and one example instance each. MINOR because it *adds* the `schema` field the registry format already reserves.

**Implementation complexity:** high. **Scientific impact:** high. **Ecosystem impact:** critical.

---

### F4 — Critical — Conformance cannot detect cross-implementation divergence (Certification)

**Statement.** Conformance is defined as *"STRUCTURAL, PROPERTY-BASED and INVARIANT-BASED — NOT golden-output"* (Scenario Conformance Part 8; Vol 6 Part 2). Verdicts evaluate only the 7 registered invariants (ORCH-001..004, OWN-001, LAYER-001, SHARD-001) plus a handful of properties (isolation, provenance-present, gates-fired, safety-blocked, replay-reproduces-trace). Proposed invariants "can at most produce PARTIAL, never FAIL" (Vol 6 Part 6). The reference `arves-conformance` crate faithfully encodes this: the artifact records structural evidence and a `Verdict`, with *"NO notion of an expected output."*

**Why it matters.** The purpose of a conformance suite in a standard is to guarantee that two independent PASS-ing implementations *interoperate*. ARVES's suite is designed to prove a single implementation is internally disciplined (it does not fork truth, it layers downward, it replays its own trace). It is structurally incapable of catching: divergent types (F3), divergent wire formats (F2), divergent content-addressing (F1), divergent arbitration (F9), or divergent canonicalization (F11). Two runtimes that agree on none of those can both earn L1–L4 and "Certified Product." The non-determinism argument ("cognitive engines are non-deterministic, so we can't compare outputs") is valid *for engine inference* but has been over-applied to the entire artifact, including the parts that must be byte-identical for interop (types, envelopes, digests, trace structure).

**Risks.** "Certified ARVES" becomes a badge that guarantees discipline but not compatibility — the worst outcome for a standard, because it manufactures false confidence in interop. Independent Runtime A and B (LONG-TERM OBJECTIVES #3–4) could both certify and be mutually unintelligible.

**Long-term consequences.** This is the finding most likely to be *invisible* until two real runtimes try to exchange data in year 3 and discover certification never guaranteed it. Fixing it later means re-certifying the whole ecosystem.

**Alternative designs.** (a) Add an **interoperability conformance tier**: golden-*structure* tests (not golden inference) that assert byte-identical envelopes, digests, type serializations, and trace framing across implementations — a cross-runtime differential harness. (b) A W3C-style "two independent implementations must produce identical artifacts for the deterministic layer" exit criterion (the Reference Lifecycle already gestures at this in the Reference Ecosystem stage). (c) Keep property-only — reject; it under-defines interop.

**Recommendation.** This is primarily a **Certification** work item (extend the program), backed by a **Verification** harness: define a deterministic-layer differential-conformance suite that pins the byte-exact contracts (once F1/F2/F3/F5 land) and requires two independent implementations to agree. This depends on F1–F3 and F5 existing first.

**Implementation complexity:** very-high. **Scientific impact:** high — this is the formal boundary between "non-deterministic inference" and "deterministic interop surface," a genuinely interesting contribution if drawn correctly. **Ecosystem impact:** critical.

---

### F5 — High — "Replayable decision trace" (ORCH-003) has no trace format or determinism contract (CCP-Amendment)

**Statement.** ORCH-003 requires every execution to be *"replayable from the same Goal, State, Policies, Capabilities and Runtime Fingerprint — via a recorded decision trace, not by recomputation."* The "decision trace" is invoked everywhere (Vol 9 Part 9; Engine Graph Part 6; Conformance Part 9; Certification Part 7 says the artifact is *"self-sufficient ... a verifier reconstructs the verdict without re-running engines"*). But the **trace has no defined schema**: what records it contains, in what order, with what fields, under what serialization. Nor is "Runtime Fingerprint" schematized beyond a prose list ("engine versions, model routing, capability bindings, policy set"). The reference `arves-persistence` defines a `WalRecord` shape for the *reference* runtime, but that is an IDR-level artifact, not a normative trace standard, and its `payload` is *"opaque bytes only."*

**Why it matters.** Certification's central promise — an artifact a *third party* replays to reconstruct the verdict — is only achievable if the trace format is standardized. Otherwise only the producing runtime can replay it, and "independent verification" collapses into "trust the vendor's own replayer."

**Risks.** Non-portable certification artifacts; replay that silently diverges because the fingerprint under-captures a determinism-relevant input (e.g. model routing seed, policy version); an inability to arbitrate certification disputes with the artifact alone.

**Long-term consequences.** The decision trace is ARVES's equivalent of a build's provenance/attestation. If it is not a standardized, portable format, the "attested conformance" objective (Reference Lifecycle Certification stage) is unverifiable across parties.

**Alternative designs.** (a) A normative trace schema + canonical serialization (reuse F2's encoding) with a defined record taxonomy (commit, invocation, arbitration, gate, membership). (b) Adopt an existing provenance format (in-toto / SLSA-style attestation) as the trace envelope. (c) Leave per-runtime — reject.

**Recommendation.** **CCP Amendment (MINOR)** — "Decision Trace & Runtime Fingerprint Schema" — defining the normative trace record types, ordering guarantees, the fingerprint's complete field set, and a worked replay example; plus a conformance scenario where runtime A's trace is replayed by runtime B to the same verdict.

**Implementation complexity:** high. **Scientific impact:** high. **Ecosystem impact:** high.

---

### F6 — High — Event Envelope & payloads: names without types, encoding, or versioning rules (CCP-Amendment)

**Statement.** The Event Envelope is given twice with slightly different field lists — Vol 9 v1 Part 6 (`event_id, tenant_id, workspace_id, event_type, timestamp, correlation_id, payload`) vs ARVES-21 (`event_id, event_type, tenant_id, workspace_id, correlation_id, timestamp, source, payload, version`). Neither gives field *types*, id formats (UUID? ULID?), timestamp representation (epoch ns? RFC 3339?), or the payload schema per event. ARVES-21 lists ~50 event names and an "Event Contract Template" (Payload Schema, Version, Idempotency Rules) that is **never filled in for a single event**. The Gap Analysis flags this precisely (*"ARVES-21 lists envelope fields but no per-event payload schema, versioning example, or sample message"*).

**Why it matters.** Events are the cross-cutting communication substrate (LAYER-001: cross-cutting traverses the Event Fabric). Two runtimes that disagree on the envelope cannot interoperate at the most basic level, and the two divergent envelope definitions mean even the *reference* is ambiguous about which is canonical.

**Risks.** `correlation_id` drives routing (Amendment-004) and trace correlation (CAP-007) — if its format/semantics are unspecified, routing and replay correlation diverge. The two envelope variants are a latent conflict an independent reader cannot resolve.

**Recommendation.** **CCP Amendment (MINOR)** — reconcile the two envelopes into one normative schema with field types, id/timestamp formats, and versioning rules; populate the ARVES-21 contract template for at least the events touched by the four reference scenarios; provide example messages. Consider a CloudEvents binding to inherit a battle-tested envelope rather than inventing one.

**Implementation complexity:** medium. **Scientific impact:** low. **Ecosystem impact:** high.

---

### F7 — High — API Catalog: names without paths, verbs, schemas, or error models (CCP-Amendment)

**Statement.** ARVES-24 lists ~40 API names ("Goal API, Planning API, ...") and a "Canonical REST Pattern (GET/POST/PUT/DELETE under /api/v1)" and an API Contract Template — but **no endpoint has a path, verb binding, request/response schema, status codes, or error model.** The Gap Analysis: *"ARVES-24 lists API names only — no paths, HTTP verbs, parameters, status codes, or error models."*

**Why it matters.** Any product built "entirely on ARVES without modifying the standard" (LONG-TERM OBJECTIVE #10) needs a stable API contract. Without it, every SDK (OBJECTIVE #7) is written against a specific runtime, not the standard, so runtimes are not substitutable.

**Risks.** SDKs and products couple to a concrete runtime; the "Certified Product" level certifies against a moving target; error handling is unportable (each runtime invents its own error model).

**Recommendation.** **CCP Amendment (MINOR)** — publish a normative machine-readable API description (OpenAPI or equivalent) for the core control-surface APIs (Goal, Planning, Query, Execution, Conformance-artifact retrieval), with request/response schemas (reusing F3 types), status codes, and a standard error model. Deepen one vertical slice first per the Gap Analysis order.

**Implementation complexity:** medium. **Scientific impact:** low. **Ecosystem impact:** high.

---

### F8 — High — Engine Manifest "serialized form" is named but never serialized (CCP-Amendment)

**Statement.** The Engine Graph Spec's whole thesis is that the manifest is *"the portable, serializable descriptor — the analogue of an OCI image manifest ... A runtime needs only the manifest (not the source) to schedule and execute a node"* (Part 9). Table 0 lists the manifest fields (Name, Version, Inputs, Preconditions, Reads, Writes, Produces, Capabilities Required, Determinism, Idempotency Key, Failure Policy, Retry Policy, Timeout, Confidence, Cost, Latency). But there is **no serialization, no field type for any entry, no enumerations** (what are the legal values of Failure Policy — the prose says "fail, degrade, escalate," but Retry Policy, Determinism encoding, Timeout units, Cost units are undefined), and **no example manifest**. The reference `arves-engine-fabric` types `reads/produces/capabilities_required` as `Vec<String>` and `idempotency_key` as an opaque `String` scheme tag — deliberately unspecified.

**Why it matters.** The portability guarantee (Part 10) and the Independent-Implementability Test (Part 13) both depend on the manifest being a *concrete, serializable, typed* artifact. As written, "portable manifest" is an assertion with no artifact behind it. OCI succeeded because the image manifest is a byte-exact JSON schema with a digest; ARVES cites OCI as precedent but has not produced the manifest schema.

**Risks.** Engines are not portable across runtimes; the marketplace (OBJECTIVE #8) — which trades engine/agent manifests — has nothing to trade; `Cost`/`Latency`/`Confidence` are "machine-readable, not prose" (Part 8) yet have no machine-readable units or ranges.

**Recommendation.** **CCP Amendment (MINOR)** — "Engine Manifest Schema v1" — a normative, typed, serialized manifest schema (reusing F2 encoding and F1 idempotency contract), enumerations for Determinism/Failure/Retry policies, units for Timeout/Cost/Latency, ranges for Confidence, and at least one complete example manifest that a runtime can execute.

**Implementation complexity:** high. **Scientific impact:** medium. **Ecosystem impact:** high.

---

### F9 — Medium — Arbitration / join-node semantics are undefined and non-deterministic (CCP-Amendment)

**Statement.** Join nodes "perform ARBITRATION: they merge conflicting branch outputs by policy (confidence-weighting, tie-break)" (Vol 9 Part 6; Engine Graph Part 5, Part 8: "confidence-weighted merge at join nodes"). But the merge function, the confidence-weighting formula, and the tie-break rule are never defined. Arbitration output is a plan artifact that feeds decisions, so its determinism directly affects whether two runtimes (or two replays) reach the same plan.

**Why it matters.** ORCH-003 replay determinism and cross-runtime plan agreement both require arbitration to be a defined function of its inputs. "By policy" with no policy definition means each runtime arbitrates differently; the same branch outputs can yield different plans, and even a single runtime's replay is only deterministic if the arbitration function is pinned (the trace records the *choice*, but conformance F4 can't check the choice was correct because "correct" is undefined).

**Risks.** Divergent plans across conformant runtimes; safety-critical scenarios (Warehouse Robot Dispatch) where arbitration decides whether an unsafe branch is selected — undefined tie-break here is a safety hazard, not a nicety.

**Recommendation.** **CCP Amendment (MINOR)** — define the arbitration contract: the confidence-weighting function (or a small set of named, selectable policies), a deterministic tie-break (e.g. lowest content-address wins), and its recording in the trace. Add a conformance scenario asserting replay-stable arbitration.

**Implementation complexity:** high. **Scientific impact:** medium. **Ecosystem impact:** medium.

---

### F10 — Medium — Graph termination policy has no normative bounds or algorithm (IDR)

**Statement.** Dynamic graph expansion is "bounded by a termination policy (max depth / budget / no-new-subgoal) to prevent infinite meta-planning" (Vol 9 Parts 6–7; Engine Graph Part 5). The *existence* of a bound is normative; the *bound itself* — default max depth, budget units, the "no-new-subgoal" detection rule — is undefined.

**Why it matters.** Termination behavior affects liveness and cost and is scenario-observable (a long-running planning scenario either terminates or does not). Two runtimes with different defaults produce different behavior on the same goal. This is genuinely an implementation *mechanism* decision, so it belongs in an IDR — but the IDR must exist and be conformance-linked so behavior is at least declared per runtime and captured in the fingerprint.

**Recommendation.** Open an **IDR** defining the reference runtime's termination policy and its parameters, require the parameters to be captured in the Runtime Fingerprint (so replay is faithful), and add a conformance property that expansion terminates within the declared bound.

**Implementation complexity:** medium. **Scientific impact:** low. **Ecosystem impact:** medium.

---

### F11 — Medium — Canonicalization is the central promise and is entirely unspecified (CCP-Amendment)

**Statement.** ARVES's mission is literally *"Any Information → Canonical Knowledge"* (Vol 1 §3; Blueprint). The Information Platform "canonicalizes" (Amendment-004 Layer Matrix: *"Provider/connector registry, canonicalization"*; Conformance node evidence: *"Source normalized to canonical model with provenance"*). Yet there is no definition of *what canonical means*: no canonical data model schema (blocked by F3), no canonicalization rules, no worked "raw → canonical" example. This also underlies F1: content-addressing requires a canonical pre-image, which requires canonicalization to be defined.

**Why it matters.** If "canonical" is undefined, two runtimes canonicalize the same source differently, so their downstream facts, hashes, and traces diverge from the very first pipeline node. Conformance can only assert "provenance present," not "canonicalized *correctly and identically*."

**Risks.** Divergence at the pipeline entry point propagates through the entire system; the platform's headline value proposition is unverifiable.

**Recommendation.** **CCP Amendment (MINOR)** — define canonicalization as a normative transform: the canonical model (depends on F3), the normalization rules (unicode/number/date/ordering — reuse RFC 8785-style rules), and worked examples for the reference-scenario sources. Add a conformance property that the same source canonicalizes identically across runtimes.

**Implementation complexity:** very-high (semantically deep). **Scientific impact:** high. **Ecosystem impact:** medium.

---

### F12 — Medium — Read-consistency tiers named but their observable guarantees unspecified (IDR)

**Statement.** IDR-001 defines three read tiers — Linearizable (through leader, read-index), Bounded-staleness (follower read), Eventual (replica). The tier *names* and *paths* are given, but the observable contract is not: what staleness bound (a number) bounds "bounded-staleness"; what a client must send to get a read-index; what monotonicity/session guarantees each tier offers.

**Why it matters.** Query is a conformance node ("correct, tenant-scoped read"). "Correct" at a tier requires the tier's guarantee to be defined; otherwise an independent runtime's "bounded-staleness" could be arbitrarily stale and still claim conformance.

**Recommendation.** Extend the relevant **IDR** to specify each tier's observable guarantee (concrete staleness bound or bound-negotiation protocol, session/monotonic-read semantics) and add conformance properties for at least linearizable and bounded-staleness. IDR is the right instrument since consistency mechanism is an implementation decision, but the *observable guarantee* should be tightened toward normative.

**Implementation complexity:** medium. **Scientific impact:** medium. **Ecosystem impact:** medium.

---

### F13 — Medium — The invariants an implementer needs carry no conformance weight (CCP-Amendment)

**Statement.** The invariants that actually constrain an *implementation* — G-001 (sole commit gateway), QUERY-001 (query read-only), LCW-001, PERSIST-001, CAP-001..009, ENG-001..005 — are all classified **Proposed / informative**, and Vol 6 Part 6 says a proposed-invariant expectation "can at most produce PARTIAL, never FAIL." Only the 7 abstract ORCH/OWN/LAYER/SHARD invariants are enforceable. So the concrete engineering contracts (engine purity, proposed-writes-only, read-only query, versioned capability bindings) are exactly the ones a conformance run cannot fail on.

**Why it matters.** An independent implementer reading the corpus finds the useful, checkable engineering rules (ENG-*, CAP-*) sitting in a "grounded but never ratified, verified 0 contradictions" limbo. They are the best-specified behavioral contracts in the corpus yet cannot be relied on for a pass/fail. This inverts the value: the vague invariants are enforced; the precise ones are advisory.

**Recommendation.** For each proposed invariant that is genuinely a runtime contract, run it through the **CCP-GATE** (CCP Amendment/IDR + one conformance scenario, per Reference Lifecycle Part 6) to ratify it as registered-normative. Prioritize G-001, QUERY-001, ENG-002/003, CAP-003/004 — the ones the reference crates already treat as load-bearing.

**Implementation complexity:** medium. **Scientific impact:** medium. **Ecosystem impact:** medium.

---

### F14 — Low — Version drift and an unresolved "superseded" canonical model (CCP-Amendment)

**Statement.** The Ontology Spec (Part 2) declares it *supersedes* Vol 3, Vol 13, and ARVES-19 — "which become non-normative once this registry is frozen." But ARVES-19 is still in the frozen corpus with its (contradictory, all-identical) entity definitions, and the Ontology Spec that replaces it is itself schema-empty (F3). An independent reader cannot tell which entity model is authoritative in practice, because the authoritative one is empty and the deprecated one is populated-but-wrong. Compounding: Blueprint/Vol1/Vol2 are v2 while most volumes are v1 with no changelog (Gap Analysis), and there are multiple `Documentation_Index` versions (v1, v2, v2.1, v2.2).

**Why it matters.** Ambiguous authority is a slow-acting interoperability hazard: different teams anchor to different "canonical" models. Low severity only because the *governance* to fix it (supersession, versioning) already exists in the Reference Lifecycle maturity model — it just has not been applied to close these out.

**Recommendation.** **CCP Amendment (PATCH/MINOR)** — issue a clarifying supersession record that (a) formally marks ARVES-19 / Vol 3 / Vol 13 entity models Superseded per the Lifecycle status model, (b) points to the Ontology Spec as sole authority, and (c) is enforced by making the Ontology schemas real (F3). Consolidate the Documentation Index to one authoritative version with a changelog.

**Implementation complexity:** low. **Scientific impact:** low. **Ecosystem impact:** low.

---

## Gate Verdict

**NOT-READY** for ISO/IEEE-grade, independently-implementable standardization.

**Justification.** The independent-runtime lens asks one question: *given only the frozen specs, can a rival team build a conformant, interoperable ARVES?* The evidence is decisive and largely self-reported by the corpus:

- The authors' own Gap Analysis rates the corpus *"90% naming & structure, 10% content ... NOT yet an implementable specification."*
- The three frozen "Independent-Implementability Tests" (Engine Graph Part 13, Ontology Part 11, Reference Lifecycle Part 11) set the bar — *an outside team must be able to build it from the document alone* — and the corpus does not meet its own bar.
- The single most-cited invariant (ORCH-004 content-addressing) is undefined; the wire format is explicitly pushed to IDR (outside the standard); the type registry that "closes the ABI" is schema-empty; and conformance is architecturally unable to catch the resulting divergence.

Crucially, **this is a not-ready verdict, not a not-viable one.** The architecture is coherent and the *governance machinery to fix every finding already exists* — the Reference Lifecycle's CCP Amendment process is designed to add exactly this missing normative detail as MINOR, backward-compatible changes *without reopening the freeze*. None of the 14 findings requires changing a decision the spec actually made; they require *making decisions the spec deferred or left implicit*. The corpus froze the meta-layer correctly; it now needs to populate the object-layer through the process it built for precisely this purpose.

**Recommended path to clear the gate:** land F1 (content-addressing), F2 (wire format), F3 (type schemas), and F5 (trace format) as CCP Amendments — these four unlock everything else — then build the F4 differential-conformance tier on top. Until at least F1–F3 exist as normative, byte-level contracts with worked examples, deeper Wave-2 reviews (performance, scale, security) would be evaluating prose, and the gate should remain closed.

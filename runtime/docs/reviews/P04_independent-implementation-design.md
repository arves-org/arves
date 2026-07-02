# P04 — Independent Implementation (spec-only, Rust)

**Lens:** Design a completely independent Rust implementation of ARVES using ONLY the frozen specifications. Document every ambiguity that had to be resolved to proceed, and what normative text (IDR/CCP) would have removed the guesswork. Output the ambiguity ledger + a proposed module architecture.

**Reviewer role:** Independent Chief-Architect-level reviewer, optimizing for ISO/IEEE-grade international adoption over a 20-year horizon.

**Method:** I read the frozen corpus as an outside engineer with no access to the ARVES team, then attempted to derive a buildable Rust workspace from the text alone. Every point where the text under-determined a decision I had to make became an ambiguity-ledger entry. I consulted the existing reference runtime (`arves-kernel`, `arves-persistence`) *only* to confirm that it, too, had to invent the same undocumented choices — which is the strongest possible evidence that the specification, not the implementer, is the source of the gap. **I never propose changing the frozen corpus**; every remedy is an IDR, a CCP Amendment, a Runtime/Verification/Certification improvement, or Ecosystem/Product work.

---

## Executive Summary

ARVES's frozen corpus is unusually strong on *architecture* (ownership, layering, planes, CP/AP boundary, invariants) and unusually weak on *bytes*. The specification tells an independent implementer **which component owns what** and **what must be true**, but almost never tells them **what a message looks like on the wire, how an identifier is spelled, how a hash is computed, or how a type is validated**. The corpus itself is candid about this: the Engine Graph Specification (Part 13) and the Ontology Specification (Part 11) both stake their credibility on an "Independent-Implementability Test" — *"an engineer outside the ARVES team, given only this specification, can build a runtime that produces conformant behaviour."* **Today that test fails**, and it fails for a concrete, fixable reason: the one hard normative dependency the whole ABI rests on — the Universal Cognitive Ontology "Type Registry" — is published as a table of 18 URN names with one-line English glosses and **zero field-level schemas** (Ontology Spec, Table 2; Part 9 says the registry entry is `{ urn, version, aspects, schema, relations }` but the `schema` is never given for any type).

The consequence is systemic: two independent teams can each build a runtime that passes every ORCH-001..004 invariant check the Scenario Conformance Framework defines, yet be **wire-incompatible and semantically incompatible** with each other. They will disagree on how `uci.fact@1` serializes, how a `ContentHash` is computed (so ORCH-004 idempotency keys will *not* match across runtimes), how the Event Envelope frames on the wire, and what "the same Goal, State, Policies, Capabilities and Runtime Fingerprint" (ORCH-003) means as a concrete byte structure. The existing reference runtime confirms this precisely: to make *any* progress it had to invent a URN grammar (`urn:arves:<ns>:<type>@<maj>.<min>:<id>`), an FNV-1a truth hash, a CRC32-IEEE frame format, a hex shard-directory naming scheme, a bespoke snapshot blob codec, and a placeholder `term = 0` — **none of which appear in the frozen text**, and all of which a second team would guess differently.

The good news: **none of this requires reopening the Specification Era.** The corpus's own change machinery (IDR for engineering decisions, CCP Amendment for backward-compatible normative additions with a conformance scenario per CCP-GATE) is exactly the right instrument. The single highest-leverage action for ISO/IEEE adoption is to publish, via CCP, a **frozen wire-and-type profile**: canonical serialization, a canonical content-addressing function, per-type JSON/CBOR schemas for every `uci.*` URN, and an interoperability test-vector corpus. Until that exists, ARVES is a well-architected *reference implementation*, not yet an *implementable standard*.

Below: a severity-ranked finding table, then one section per finding (with why-it-matters, risks, long-term consequences, alternatives, recommendation, complexity, scientific impact, ecosystem impact), then the **Ambiguity Ledger** (the concrete decisions I had to invent), then the **proposed module architecture**.

---

## Severity-Ranked Findings

| # | Severity | Title | Proposal type | Impl. complexity |
|---|----------|-------|---------------|------------------|
| F1 | Critical | Ontology "Type Registry" has no field-level schemas — the ABI's hard dependency is unresolved | CCP-Amendment | very-high |
| F2 | Critical | No canonical serialization or content-addressing function → ORCH-004 idempotency is not cross-runtime reproducible | CCP-Amendment | high |
| F3 | High | ORCH-003 "decision trace" and "Runtime Fingerprint" have no normative byte/field structure | CCP-Amendment | high |
| F4 | High | Engine Manifest is a field *list*, not a schema — not machine-parseable across runtimes | CCP-Amendment | medium |
| F5 | High | Raft/WAL record & framing format is undefined; reference runtime invented CRC32 frames, `term=0`, hex dirs | IDR | medium |
| F6 | High | Conformance artifact schema is named but not specified → certification cannot be reproduced independently | Certification | medium |
| F7 | Medium | Shard-key derivation, entity→shard binding, and cross-shard entity identity underspecified | IDR | medium |
| F8 | Medium | Event Envelope / Command / Query contracts are field lists without types, ordering, or transport binding | CCP-Amendment | medium |
| F9 | Medium | No formal state-machine for engine/task lifecycle or leader-loss "discard in-flight" semantics | IDR | medium |
| F10 | Medium | Policy / safety-gate evaluation contract is undefined, yet safety gates gate certification (Warehouse Robot) | CCP-Amendment | high |
| F11 | Low | No reference interoperability test-vector corpus to bind two runtimes together | Verification | medium |
| F12 | Low | Aspect value types (Trust score range, Temporal semantics, Provenance origin enum) unspecified | CCP-Amendment | low |

---

## F1 — Ontology "Type Registry" has no field-level schemas (Critical)

**Finding.** The Universal Cognitive Ontology Specification declares itself "the ROOT of UCS" and "the normative dependency that closes the Engine Graph ABI (Reads/Writes/Produces resolve to types defined here)" (Ontology Spec, Part 2). The Engine Graph Spec Part 14 is explicit that "the Reads/Writes/Produces type vocabulary is UNRESOLVED until the Universal Cognitive Ontology Specification is written and frozen." The Ontology Spec is now frozen (Freeze Record, Part 3) — but Part 9 defines a registry entry as `{ urn, version, aspects, schema, relations }` and then **never publishes a single `schema`**. Table 2 gives 18 URNs (`uci.fact`, `uci.observation`, `uci.goal`, …) with one-line meanings ("A validated truth claim"). There is no field list, no type for any field, no required/optional marking, no cardinality, no encoding.

**Why it matters.** This is the keystone. Every downstream contract dereferences these types: engines declare `Reads: [uci.fact@1]`, the Kernel commits payloads that are supposed to *be* `uci.*` instances, Query projects them, Conformance asserts "provenance/trust present." An independent implementer cannot construct a `uci.fact@1` value, cannot validate one received from another runtime, and cannot compute a content hash over one (F2) because they do not know its canonical fields. I had to *invent the entire schema for every type* to write even a toy Kernel payload. The existing reference runtime sidestepped this by making the Kernel payload an **opaque `Vec<u8>`** (`arves-kernel` `ProposedWrite.payload: Vec<u8>`, commented "typed later via arves-ontology") — i.e. it deferred the keystone rather than resolving it, which is honest but means the reference runtime is not yet exercising the type system it certifies against.

**Risks.** (1) Two certified runtimes are mutually unintelligible — the worst outcome for a *standard*. (2) Conformance can pass while interoperability fails, because the Scenario Conformance Framework (Part 8) asserts *structural/property* invariants ("provenance present"), not *bit-level type conformance*. (3) ISO/IEEE reviewers will reject a "type system" with no types as descriptive prose, exactly the failure the spec's own Part 11 warns against.

**Long-term consequences.** If schemas are inferred independently by each vendor and only later reconciled, the reconciliation is a **breaking MAJOR** (Reference Lifecycle, Table 3: "Breaking change to types/contracts/ABI → MAJOR"). Publishing schemas *now* as a MINOR/CCP addition (backward-compatible: it adds the missing `schema` that was always promised) avoids a v2 fork.

**Alternative designs.**
- *(A) JSON Schema per URN* — human-readable, tool-rich, weak on canonical bytes.
- *(B) CBOR + CDDL (RFC 8610)* — deterministic encoding (RFC 8949 §4.2), compact, ISO-adjacent; strongest for content-addressing (feeds F2).
- *(C) Protobuf/FlatBuffers* — great tooling, but field-number governance and canonical-bytes caveats (protobuf is not canonical by default).
- *(D) A Rust-source master registry generating schemas* — couples the standard to one language; rejected (violates "Standards over Frameworks").

**Recommendation.** File a **CCP Amendment** ("Ontology Type Schemas v1.0") that, for each frozen `uci.*` URN, publishes the missing `schema` (fields, types, required/optional, cardinality) referencing the five mandatory aspects (Identity/Provenance/Trust/Temporal/TenantScope) exactly once as mixins (as Part 4 already mandates). Adopt **CDDL over CBOR** as the canonical schema+encoding, with a JSON projection for ergonomics. Ship one conformance scenario per type (satisfies CCP-GATE). This is additive, not a redefinition — it fills the `schema` slot the frozen text already reserved.

**Implementation complexity:** very-high (18 root types + domain subtypes + aspect composition + test vectors).
**Scientific impact:** Converts ARVES's ontology from a taxonomy into a *machine-checkable semantic contract* — the difference between an ontology paper and RDF/OWL. Enables formal statements like "every committed truth is a well-typed `uci.fact@n`."
**Ecosystem impact:** Unblocks every third-party runtime, SDK codegen, and marketplace type-checking. Without it, there is no ecosystem — only isolated forks.

---

## F2 — No canonical serialization or content-addressing function (Critical)

**Finding.** ORCH-004 ("every engine and capability invocation is idempotent and content-addressable") and the whole IDR-001 commit model ("re-committing an identical proposal must resolve to the same TruthRef rather than forking truth") depend on a **content address** — a hash of a canonical byte representation. The corpus never defines (a) the canonicalization (field order, encoding, number/string normalization) or (b) the hash function (SHA-256? BLAKE3? truncated?). The reference runtime had to invent both: `ContentHash(pub Vec<u8>)` is caller-supplied and unvalidated in `arves-kernel`, and the internal truth-set fingerprint uses a hand-rolled **FNV-1a-64** (`fnv1a_64`, chosen explicitly "without relying on any hasher whose seed could vary"), while the WAL frame uses **CRC32-IEEE**. None of FNV/CRC32/SHA appears in the frozen text.

**Why it matters.** Content-addressing is the *bridge to distribution* — the Distributed Systems Playbook, Table 15, literally tests "Does a replayed invocation create a second truth? (must be no)." If two runtimes hash differently, the same logical proposal produces different addresses, so idempotent dedup fails *across* runtimes and even across two implementations of the same runtime. ORCH-004 becomes unprovable as an interoperability property; it is only provable *within* one implementation's private hash.

**Risks.** Silent truth-forking across a federated/multi-vendor deployment (a deferred-to-v2 goal, but the design must not preclude it). Idempotency keys in the Engine Manifest (`IdempotencyKey`) are `String` with an undefined derivation ("derived from (manifest identity, canonicalized input, read-snapshot)" — canonicalization unspecified), so even a single vendor's two engine builds may diverge.

**Long-term consequences.** Content addresses leak into WAL records, snapshots, and decision traces that must be stable "five years from now" (CLAUDE.md success criterion). A later change to the hash is a MAJOR break of every stored trace. Freeze it once, early, via CCP.

**Alternative designs.** *(A)* SHA-256 over CBOR canonical form (deterministic, ISO-registered hash, boring-is-good). *(B)* BLAKE3 (faster, keyed/streaming, less "standards-committee-familiar"). *(C)* Multihash/multiformats (self-describing, future-proof algorithm agility) — recommended wrapper so the address *names its own algorithm*, avoiding a future MAJOR when the hash is upgraded.

**Recommendation.** **CCP Amendment** "Canonical Encoding & Content Addressing v1.0": canonical CBOR (RFC 8949 §4.2 deterministic) as the addressing pre-image; `content-address = multihash(sha2-256, canonical-cbor(value))`; define the idempotency-key derivation as `content-address(manifest-id ⊕ canonical-input ⊕ read-snapshot-digest)`. One conformance scenario proving cross-implementation address equality on shared vectors (ties to F11).

**Implementation complexity:** high (canonicalization edge cases: floats, map ordering, string normalization NFC).
**Scientific impact:** Makes ORCH-003/004 *formally verifiable* interoperability properties rather than intra-runtime conveniences.
**Ecosystem impact:** Prerequisite for a marketplace of engines/capabilities that different runtimes can share by address.

---

## F3 — ORCH-003 "decision trace" and "Runtime Fingerprint" have no byte/field structure (High)

**Finding.** ORCH-003 requires replay "from the same Goal, State, Policies, Capabilities and Runtime Fingerprint — via a recorded decision trace." Vol 9 Part 9 lists what the trace *records* (expanded Engine Graph, engine outputs, arbitration choices, policy evaluations, Runtime Fingerprint = "engine versions, model routing, capability bindings, policy set"). This is a **content outline, not a contract**: no field types, no ordering, no serialization, no versioning of the trace format itself.

**Why it matters.** Replay is the marquee guarantee and Axis 12 of conformance. An independent replayer must read another runtime's trace byte-for-byte to reproduce a decision. Today it cannot, because the trace is prose. The reference runtime equates "Raft log = WAL = decision trace" (IDR-005) and stores opaque outcome payloads — but the *structured* trace (graph, arbitration, fingerprint) that ORCH-003 names is nowhere serialized in `arves-persistence` (records are `{shard, offset, term, kind, content, payload}` with `payload: Vec<u8>` opaque).

**Risks.** "Replayable" degrades to "replayable by the same codebase," which is not a standard. Certification's Recovery & Replay axis becomes self-referential.

**Long-term consequences.** The decision trace is the audit artifact regulators will demand (Vol 2 Part 20 "Who/When/What/Why"; Vol 17 Security & Governance). An unspecified audit format cannot be independently verified in court/compliance — fatal for enterprise adoption.

**Alternatives.** Reuse the F1/F2 CBOR+multihash substrate for a `DecisionTrace` type: a typed, versioned, append-structured record referencing engine invocations by content address. Alternatively adopt an existing provenance model (W3C PROV-O) as the semantic layer over the byte layer.

**Recommendation.** **CCP Amendment** "Decision Trace & Runtime Fingerprint Schema v1.0" defining both as `uci.*`-typed, versioned, content-addressed structures. Conformance scenario: a trace produced by runtime A replays on runtime B to identical committed truth (ORCH-003 across implementations).

**Complexity:** high. **Scientific impact:** turns "replay" into a reproducibility guarantee comparable to reproducible-build / deterministic-simulation research. **Ecosystem impact:** cross-vendor audit, third-party certification labs, forensic replay.

---

## F4 — Engine Manifest is a field list, not a schema (High)

**Finding.** Engine Graph Spec Table 0 enumerates manifest fields (Name, Version, Inputs, Preconditions, Reads, Writes, Produces, Capabilities Required, Determinism, Idempotency Key, Failure Policy, Retry Policy, Timeout, Confidence, Cost, Latency) with English "Meaning" cells. Part 9 says it is "the analogue of an OCI image manifest" and "serializable" — but no serialization, no types, no enum value sets (what are the legal `Failure Policy` values? Part 4/Table say "fail, degrade, escalate" once, informally), no `Preconditions` expression language. The reference `EngineManifest` (Rust struct) made concrete choices — `reads/produces: Vec<String>`, `Determinism` enum with `Deterministic` default, `IdempotencyKey(String)` — that the spec does not mandate and a second team would spell differently.

**Why it matters.** Part 13's own acceptance bar is that an outside engineer "can build a runtime that executes any conformant engine manifest." Without a manifest schema, manifests are not portable — defeating the OCI analogy the spec invokes.

**Risks/consequences.** Every runtime invents its own manifest dialect; engines are non-portable; the "marketplace" (long-term objective) cannot exist. `Preconditions` with no expression language means safety-relevant gating is unspecified (compounds F10).

**Alternatives.** *(A)* CDDL schema for the manifest (consistent with F1/F2). *(B)* Adopt OCI-style JSON manifest + JSON Schema. *(C)* Define a small, total, side-effect-free precondition expression language (e.g. CEL-like) with a frozen grammar.

**Recommendation.** **CCP Amendment** "Engine Manifest Schema v1.0": typed schema, closed enum sets for Determinism/Failure Policy/Retry Policy, a frozen precondition mini-language, and manifest content-addressing (F2). Ships with the Warehouse Robot scenario as its conformance witness (already the ABI's stated acceptance test, Engine Graph Part 12).

**Complexity:** medium. **Scientific impact:** a portable cognitive-execution unit (the paper-worthy "OCI for cognition" claim becomes real). **Ecosystem impact:** engine marketplace, SDK codegen, cross-runtime scheduling.

---

## F5 — Raft/WAL record & framing format undefined (High)

**Finding.** IDR-005 fixes the *model* ("append-only WAL + snapshots; Raft log = WAL = decision trace") but no *format*. The reference runtime had to invent, with no spec basis: a frame `= [u32 body_len][body][u32 crc32_ieee(body)]`; a body field order `version|kind|term|offset|tenant|workspace|content|payload`; `WAL_FRAME_VERSION = 1`; `RecordKind ∈ {Outcome, Membership, SnapshotMarker, Barrier}`; a snapshot file codec; a `term = 0` placeholder (the commit path hard-codes `term: 0` because single-node has no election yet); and a shard-directory name `hex(tenant)__hex(workspace)`. These are all reasonable, but a second Raft implementer (or a follower from a *different* vendor's runtime) cannot join the same shard group.

**Why it matters.** IDR-003 (joint-consensus membership) and IDR-002 (leader→follower replication) require the *replicated log wire format* to be identical across peers. If ARVES ever wants heterogeneous-runtime clusters (deferred to v2, but distribution-readiness is a v1 constraint per Vol 9 Part 11), the log format must be normative. Even within one vendor, an undocumented on-disk format is an operational-forensics liability.

**Risks/consequences.** Vendor lock-in at the cluster layer; inability to do rolling upgrades across runtime versions; no third-party log inspector/repair tool.

**Alternatives.** *(A)* Adopt an existing Raft log framing (etcd/raft, tikv/raft-rs wire) as the reference and document it as an IDR. *(B)* Define an ARVES-native frame (CRC + length-prefixed CBOR body) as the reference runtime already sketched — promote it from code to an IDR. *(C)* Keep the *on-disk* format a private IDR (single-vendor) but make the *replication RPC* wire format a CCP normative contract, since only the RPC needs cross-peer agreement in v1.

**Recommendation.** **IDR "Reference WAL & Raft Log Format v1"** documenting the reference runtime's existing frame (CRC32-IEEE, versioned body, field order, kind enum, segment/snapshot file naming, `term` semantics) as an explicit engineering decision (it is non-normative per Reference Lifecycle Part 8, which is correct for an implementation detail). Separately note that heterogeneous clustering requires a *CCP* replication-RPC contract before v2. This is the one finding correctly homed in an IDR, not the spec — the corpus already anticipated it (IDR Batch 1 preamble: "IDRs implement, but never change, the frozen specification").

**Complexity:** medium (mostly documentation of existing choices + `term` de-hardcoding when I2 lands). **Scientific impact:** low-moderate. **Ecosystem impact:** log tooling, cross-version upgrades, operability.

---

## F6 — Conformance artifact schema named but not specified (High)

**Finding.** The Scenario Conformance Framework (Part 9) says "every run emits a machine-readable artifact … scenario id + axes, expanded Engine Graph, per-node evidence, invariants checked, arbitration choices, policy gates, Runtime Fingerprint, and the verdict. This artifact is both the certificate and the regression record." Vol 9 Part 14 calls it a "conformance artifact." **No schema is given.** The verdict rules (PASS/PARTIAL/FAIL, Part 8) are prose. Levels L1–L4 (Table 3) are prose.

**Why it matters.** This is the *certificate*. Reference Lifecycle Part 13: "Independent teams claiming 'we built ARVES' are judged by scenario results." A certificate with no schema cannot be verified by a third party, cannot be compared across runtimes, and cannot be machine-audited — which is the entire point of "Certified Kubernetes / Sonobuoy" that the framework cites as precedent (Sonobuoy emits a *precisely-specified* results bundle).

**Risks/consequences.** Certification becomes a subjective sign-off, not a reproducible attestation — disqualifying for ISO/IEEE and for the "third-party certification exists" long-term objective. Node-probe evidence ("Kernel: State transition recorded; sole truth owner") has no assertion format, so PASS is unfalsifiable.

**Alternatives.** *(A)* Define a `ConformanceArtifact` `uci.*` type (reuse F1/F2/F3 substrate). *(B)* Adopt an existing test-report standard (JUnit XML is too weak; SARIF or a purpose-built JSON bundle like Sonobuoy). *(C)* Define per-node-probe assertion predicates as machine-checkable expressions.

**Recommendation.** **Certification improvement**: publish the Conformance Artifact schema + per-axis machine-checkable predicates + a signed-attestation envelope, versioned against Suite/Spec versions (Part 11 already requires "N% at Level Lx against Framework vA / Spec vB" — make that a field, not a sentence). Pair with a reference verifier tool.

**Complexity:** medium. **Scientific impact:** executable, reproducible correctness definition (the framework's own stated goal, Part 14). **Ecosystem impact:** independent certification labs, comparable public conformance reports, buyer trust.

---

## F7 — Shard-key derivation & cross-shard entity identity underspecified (Medium)

**Finding.** Amendment-004 fixes Shard Key = `tenant_id (primary) + workspace_id (secondary)` and SHARD-001 makes it immutable. But: (a) *how* is `tenant_id/workspace_id` derived from an inbound observation at ingress? (Playbook 4.3 requires it be "derivable at ingress" but gives no rule.) (b) The Entity Key is "Ontology urn + id" (Amendment-004) — but an `EntityUrn` in the reference runtime *embeds* type+version+local_id and is separate from the shard key; the spec never says whether an entity's identity includes its shard, nor what happens to identity if a workspace is merged/split (a real tenant-lifecycle event). (c) Hot-shard sub-partitioning "within the tenant key policy (still immutable)" (Playbook Table 17) is contradictory-sounding and unspecified.

**Why it matters.** Routing correctness and OWN-001 depend on a deterministic, total ingress→shard function. Two runtimes that derive shards differently route the same entity to different Raft groups → truth splits.

**Alternatives.** *(A)* IDR specifying `shard = (tenant_id, workspace_id)` extracted from the Event Envelope's mandatory fields, with a total default (`workspace_id = "_tenant_default"` when absent). *(B)* Allow a documented, immutable *sub-shard salt* for hot tenants that is fixed at entity creation and captured in identity.

**Recommendation.** **IDR "Shard Derivation & Entity Binding v1"**: total ingress function, entity-creation-time shard binding recorded in provenance, explicit "no re-sharding in v1; workspace merge/split is a saga producing new entities" rule. (Keeps it out of the frozen spec; it is an implementation decision consistent with SHARD-001.)

**Complexity:** medium. **Scientific impact:** low. **Ecosystem impact:** consistent multi-tenant routing across runtimes.

---

## F8 — Event Envelope / Command / Query contracts are typeless field lists (Medium)

**Finding.** The Event Envelope appears in three places with **inconsistent field sets**: Vol 9 v1 Part 6 (`event_id, tenant_id, workspace_id, event_type, timestamp, correlation_id, payload`) vs ARVES-21 (`event_id, event_type, tenant_id, workspace_id, correlation_id, timestamp, source, payload, version`) — the latter adds `source` and `version`. Neither gives types, timestamp format, id format, or transport binding. Command/Query "contracts" (ARVES-21 Parts 8–9, ARVES-24) are template names only.

**Why it matters.** The envelope carries the routing key (`tenant_id + correlation_id`, Amendment-004) and the correlation id that threads the decision trace (ORCH-003, CAP-007). Divergent envelopes break routing and replay across components/runtimes. The two-version drift is itself a latent conformance ambiguity.

**Alternatives.** *(A)* CCP Amendment freezing the ARVES-21 nine-field envelope (the superset) as the canonical `uci.event` transport form with CDDL types (id = UUIDv7 or ULID; timestamp = RFC 3339 / epoch-nanos; version = semver). *(B)* Adopt CloudEvents (CNCF) as the envelope, mapping ARVES fields onto its attributes — instant tooling + a recognized standard, strong ISO-adjacency.

**Recommendation.** **CCP Amendment** "Event/Command/Query Wire Contracts v1.0" reconciling the two envelope variants (declare ARVES-21 authoritative), typing every field, and (recommended) profiling **CloudEvents**. Conformance scenario: Incident Response War-Room (already Axis 2+3+10+12) validates envelope round-trip + correlation-id trace linkage.

**Complexity:** medium. **Scientific impact:** low-moderate. **Ecosystem impact:** high — every connector, SDK, and event bus depends on it.

---

## F9 — No formal state machine for engine/task lifecycle or leader-loss semantics (Medium)

**Finding.** Vol 9 v1 Part 15 lists states (`Created, Queued, Running, Paused, Completed, Failed, Cancelled`) with no transition table, guards, or terminal rules. Amendment-005 says leader loss "discards in-flight uncommitted work" and cancellation is "cooperative and idempotent," but no state machine defines *when* a "safe cancellation point" occurs, how `Paused`→`Running` preserves ORCH-004 keys, or how preemption checkpoints map to the trace.

**Why it matters.** Long-running Workflow (Axis 5) and Recovery & Replay (Axis 12) certification need deterministic lifecycle semantics. "Cooperative cancellation at safe points" is untestable without defining the points.

**Alternatives.** *(A)* IDR with an explicit lifecycle FSM (states, events, guards, invariants each transition preserves). *(B)* Model it as a `uci.execution` typed state field validated by a transition predicate.

**Recommendation.** **IDR "Execution Lifecycle State Machine v1"** (engineering decision; the states are frozen, the *transitions* are implementation detail). Conformance hook: Long Compliance Review scenario exercises pause/resume/checkpoint/replay.

**Complexity:** medium. **Scientific impact:** low. **Ecosystem impact:** predictable long-running/agent workloads.

---

## F10 — Policy / safety-gate evaluation contract undefined (Medium, high complexity)

**Finding.** The Warehouse Robot Dispatch reference scenario (Conformance Framework Table 1) asserts "Safety gate blocks unsafe plan" — a *critical* property whose failure is a FAIL verdict (Part 8). Yet there is no contract for how a policy/safety gate is expressed, evaluated, or proven to have "fired." Vol 9 Part 10 says the Control Plane "enforces and sequences" policy but "never owns" it (owned by Vol 17 Security & Governance / Vol 2), and `uci.policy`/`uci.constraint` exist as URNs — but no evaluation semantics, no decision object, no obligation format.

**Why it matters.** This is the safety-critical axis; it is the property most likely to matter for regulated/embodied deployments and the one an ISO reviewer will scrutinize hardest. "The gate fired" must be a machine-checkable, replayable predicate over a typed policy-decision object recorded in the trace.

**Alternatives.** *(A)* Adopt a recognized policy model (XACML decision structure, or OPA/Rego as the reference evaluator) mapped onto `uci.policy`. *(B)* Define an ARVES-native policy-decision `uci.*` type: `{policy_ref, inputs_digest, effect ∈ {permit, deny, obligate}, obligations, evidence}` recorded in the decision trace.

**Recommendation.** **CCP Amendment** "Policy Decision & Safety-Gate Contract v1.0": a typed, content-addressed policy-decision object (reusing F1–F3), a "gate fired" predicate for conformance, and an explicit statement that the *evaluator* is pluggable (IDR-level) but the *decision object and its trace record* are normative. Conformance witness: Warehouse Robot (safety gate) + Policy-heavy Governance axis.

**Complexity:** high. **Scientific impact:** ties ARVES to formal policy/verification literature; enables provable safety claims. **Ecosystem impact:** required for regulated/enterprise/embodied products (Vol 8, Vol 17).

---

## F11 — No reference interoperability test-vector corpus (Low, foundational)

**Finding.** There is no published corpus of canonical inputs → expected content-addresses, serialized `uci.*` instances, sample envelopes, sample decision traces, and sample conformance artifacts that two runtimes can both be tested against.

**Why it matters.** W3C's model (which the corpus cites) is "test-suite + implementation-report": interop is proven by *shared vectors*, not by prose. Without vectors, "independent implementability" (the spec's own repeated acceptance bar) can never be demonstrated — only asserted.

**Recommendation.** **Verification improvement**: build an `arves-testvectors` corpus (versioned with the spec) generated once from the reference runtime *after* F1/F2/F3 land, then frozen. Every runtime's CI runs it. This is the concrete artifact that would let two Rust teams — or a Rust and a Go team — prove they built the same standard.

**Complexity:** medium. **Scientific impact:** reproducibility infrastructure. **Ecosystem impact:** the actual mechanism by which "≥1 independent certified runtime" (Reference Lifecycle final stage) is achieved.

---

## F12 — Aspect value types unspecified (Low)

**Finding.** The five mandatory aspects (Ontology Part 4) are named but untyped: Trust = "Trust score and verification status" (range? enum?), Temporal = "Valid From/To, Observed At" (format? open intervals?), Provenance = "Source, collector, transformation, timestamps" (structure?). The reference runtime invented `Confidence(f64)` in `[0,1]`, `Timestamp(i64)` epoch-nanos, `BiTemporal{valid_from, recorded_at}`, and an `Origin ∈ {Observed, Derived{invocation_id}, Asserted}` enum — all defensible, all unspecified.

**Recommendation.** Fold aspect value-type schemas into the **F1 CCP Amendment** (they are part of the type registry's `schema`). Trivial once F1's substrate is chosen.

**Complexity:** low. **Scientific/ecosystem impact:** completes the type system; small but necessary.

---

## Ambiguity Ledger

Every row is a decision I **had to invent** to write a spec-only Rust implementation, because the frozen text under-determines it. "Reference runtime chose" documents where the existing code silently made the same guess (evidence the gap is in the spec, not the reader). "Removed by" names the normative instrument that would eliminate the guesswork.

| # | Subsystem | Ambiguity (what the spec did not fix) | What I had to invent | Reference runtime chose | Removed by |
|---|-----------|----------------------------------------|----------------------|-------------------------|-----------|
| A1 | Ontology | Field schema of every `uci.*` type | Full CBOR/CDDL schema per URN | Opaque `payload: Vec<u8>` (deferred) | CCP (F1) |
| A2 | Ontology | Aspect value types (Trust range, Temporal format, Provenance shape) | `Confidence∈[0,1]`, epoch-ns, bitemporal, Origin enum | Exactly those, undocumented | CCP (F12/F1) |
| A3 | Identity | URN grammar for entities/types | `urn:arves:<ns>:<type>@<maj>.<min>:<local>` | Same string form (`EntityUrn`, `SCHEME="urn:arves"`) | CCP (F1) |
| A4 | Kernel | Canonical serialization for hashing | Deterministic CBOR pre-image | none (hash is caller-supplied) | CCP (F2) |
| A5 | Kernel | Content-address hash function | `multihash(sha2-256, …)` | FNV-1a-64 (fingerprint) + CRC32 (frame) | CCP (F2) |
| A6 | Engine | Idempotency-key derivation | `addr(manifest ⊕ input ⊕ read-digest)` | `IdempotencyKey(String)`, undefined derivation | CCP (F2/F4) |
| A7 | Engine | Manifest serialization + enum value sets | CDDL manifest; closed Determinism/Failure/Retry enums | `Vec<String>` reads/produces; `Determinism` enum, `Deterministic` default | CCP (F4) |
| A8 | Engine | `Preconditions` expression language | Total, side-effect-free mini-language | (not implemented) | CCP (F4) |
| A9 | Control Plane | Decision-trace byte/field structure | Typed, versioned, content-addressed `DecisionTrace` | Opaque WAL `payload` only | CCP (F3) |
| A10 | Control Plane | Runtime Fingerprint structure | Typed `{engine_versions, model_routing, bindings, policy_set}` | (not implemented) | CCP (F3) |
| A11 | Control Plane | Arbitration/join merge policy encoding | Confidence-weighted merge parameters as typed policy | (not implemented) | CCP (F3/F10) |
| A12 | Persistence | WAL frame format | `[len][body][crc32]`, versioned body, field order | Exactly that (`WAL_FRAME_VERSION=1`, CRC32-IEEE) | IDR (F5) |
| A13 | Persistence | Snapshot blob codec | length-prefixed LE tuples | Exactly that (`encode_shard_blob`) | IDR (F5) |
| A14 | Persistence | Shard-directory naming on disk | `hex(tenant)__hex(workspace)` | Exactly that (`dir_for`) | IDR (F5) |
| A15 | Persistence | Segment rotation threshold | 1024 records/segment | `DEFAULT_ROTATE_EVERY=1024` | IDR (F5) |
| A16 | Consensus | Raft `term` semantics pre-election | single-node `term` handling | hard-coded `term: 0` placeholder | IDR (F5/F9) |
| A17 | Consensus | Replication RPC wire format | (deferred; needs cross-peer contract) | (not implemented) | CCP (F5, v2) |
| A18 | Kernel/Persist | Recovery contract on corruption | "lossless or loud" (fail, don't serve gaps) | Exactly "lossless or loud" (`RecoveryError`) | IDR (matches Playbook 9.2, could be CCP predicate) |
| A19 | Routing | Ingress → `(tenant, workspace)` shard function | Total function from Event Envelope fields | (implicit; caller supplies `ShardKey`) | IDR (F7) |
| A20 | Routing | Entity re-sharding on workspace merge/split | "no re-shard in v1; saga produces new entities" | (unaddressed) | IDR (F7) |
| A21 | Runtime | Event Envelope field set (v1 7-field vs ARVES-21 9-field) | Chose ARVES-21 superset | (not implemented) | CCP (F8) |
| A22 | Runtime | Envelope id/timestamp/version formats | ULID id; epoch-ns; semver | (not implemented) | CCP (F8) |
| A23 | Runtime | Command/Query request/response schemas | Per-op CDDL | (not implemented) | CCP (F8) |
| A24 | Runtime | Execution lifecycle transition table | Explicit FSM with guards | (not implemented) | IDR (F9) |
| A25 | Runtime | "Safe cancellation point" definition | Checkpoint boundaries in FSM | (not implemented) | IDR (F9) / Amendment-005 |
| A26 | Governance | Policy-decision object + "gate fired" predicate | Typed `{effect, obligations, evidence}` | (not implemented) | CCP (F10) |
| A27 | Query | Read-tier freshness bound units + staleness metric | ns-lag bound; leader read-index protocol | (not implemented; I3) | IDR |
| A28 | Conformance | Conformance-artifact schema + per-node assertion predicates | Typed artifact + predicates | (not implemented) | Certification (F6) |
| A29 | Conformance | L1–L4 level pass thresholds (numeric) | Explicit % per level | (not implemented) | Certification (F6) |
| A30 | Capability | Capability manifest + binding descriptor schema | CDDL binding descriptor | (skeleton only) | CCP (F4-adjacent) |

**Reading the ledger:** ~30 forced inventions to stand up a spec-only implementation. The clustering is stark — **serialization/typing/addressing (A1–A11, A21–A23, A26, A28, A30)** dominate and are all resolvable by the *same* CBOR/CDDL/multihash substrate, filed as a small batch of CCP Amendments. **On-disk/format details (A12–A16, A18)** are correctly IDR-homed (implementation, non-normative). Only **A17** genuinely belongs to the deferred-to-v2 federation work.

---

## Proposed Module Architecture (spec-only Rust workspace)

Derived strictly from the frozen Layer Responsibility Matrix (Amendments Table 0) and LAYER-001 (downward-only). Crate boundaries mirror **ownership** (OWN-001), so no crate can own two states. This is the workspace I would build from the specs alone; it happens to align with the existing reference crate split (good sign), but I make the **type/serialization substrate a first-class foundational crate** the existing runtime lacks as a populated dependency.

```
arves/                                (Cargo workspace)
├── foundation/
│   ├── arves-codec        # F2: canonical CBOR + multihash content-addressing.
│   │                      #     THE dependency every other crate hashes/serializes through.
│   │                      #     No other crate defines a hash or a wire format. (fixes A4,A5,A6)
│   ├── arves-ontology     # F1/F12: uci.* Type Registry — typed schemas + 5 aspects
│   │                      #     (Identity/Provenance/Trust/Temporal/TenantScope) as mixins.
│   │                      #     Depends: arves-codec. Owns: MEANING, never storage (O-007).
│   └── arves-contracts    # F8: Event Envelope, Command, Query wire contracts (CloudEvents profile).
│                          #     Shard key + routing key derivation types (F7). Depends: ontology, codec.
│
├── truth-path/            # Data Plane, CP tier (IDR-001..005)
│   ├── arves-persistence  # F5: append-only WAL (frame format = IDR), snapshots, replay.
│   │                      #     Owns DURABILITY only (PERSIST-001). Depends: codec, contracts.
│   ├── arves-consensus    # per-shard Raft: election, replication, joint-consensus membership.
│   │                      #     Log = WAL (IDR-005). Depends: persistence, contracts.
│   └── arves-kernel       # SOLE commit gateway; owns TRUTH (ORCH-001/OWN-001/G-001).
│                          #     Typed ProposedWrite<T: CognitiveEntity> (NOT opaque bytes).
│                          #     Depends: consensus, persistence, ontology, codec.
│
├── state-and-read/        # Data Plane
│   ├── arves-lcw          # Working Memory (LCW-001); reads Kernel truth, holds live non-truth state.
│   └── arves-query        # READ-ONLY projections (QUERY-001); read tiers (linearizable/bounded/eventual).
│                          #     Writes NOTHING. Depends: kernel(read), lcw, persistence.
│
├── compute/               # Data Plane
│   ├── arves-engine-fabric# F4: pure engines behind the ABI; Manifest schema; ProposedEffect.
│   │                      #     Reads via Query only; emits proposals only (ENG-001..005).
│   └── arves-capability-fabric # registry + bindings only (CAP-001..009); owns no truth/plans.
│
├── control-plane/         # Control Plane — owns NO truth, NO persistent state (ORCH-001/002)
│   ├── arves-control-plane# Goal Mgr, Orchestrator, Engine Graph expansion (bounded), Capability
│   │                      #     Planner, Execution Planner, arbitration/join nodes.
│   ├── arves-trace        # F3: DecisionTrace + Runtime Fingerprint typed/versioned/addressed.
│   │                      #     Written by Control Plane, stored via Kernel/Persistence.
│   └── arves-policy       # F10: policy-decision object + safety-gate predicate (evaluator pluggable).
│
├── ingress-egress/
│   ├── arves-information-platform # canonicalizes Reality → PROPOSED writes to Kernel. Owns no truth.
│   └── arves-execution    # in-flight execution state; actions to world; outcomes → proposals to Kernel.
│
├── conformance/
│   ├── arves-invariants   # executable ORCH-001..004, OWN/LAYER/SHARD-001 checkers (arch tests).
│   ├── arves-conformance  # F6: harness, 12 axes, ConformanceArtifact schema, verdict engine.
│   └── arves-testvectors  # F11: frozen interop corpus (inputs → addresses → traces → artifacts).
│
└── arves-runtime          # single binary wiring one node; subcommands for the walking-skeleton proofs.
```

**Dependency rule (mechanically enforceable, LAYER-001):** a CI arch-test in `arves-invariants` fails the build if any crate depends *upward* or *laterally* across the plane boundary. The Control Plane crates may depend on Data Plane *read* surfaces but must not depend on Persistence write paths; `arves-query` must expose no `&mut` / write API (QUERY-001 enforced by the *type system*, not convention).

**Two decisions where I diverge from the existing runtime (and why):**
1. **`arves-codec` and a populated `arves-ontology` are foundational, not deferred.** The existing runtime keeps Kernel payloads opaque and leaves the ontology a skeleton; that defers the keystone (F1/F2). A spec-only implementer *cannot* defer it, because the ABI's Reads/Writes/Produces are undefined without it. Making the codec the crate everything hashes through is what makes ORCH-004 an interoperability property (F2) rather than a per-crate convenience.
2. **`ProposedWrite` is generic over `T: CognitiveEntity`, not `Vec<u8>`.** Typed commits let the Kernel enforce "every committed truth is a well-typed, provenance-and-trust-bearing `uci.*` value" — turning O-002/O-003/O-004 into a *compile-time* + commit-time guarantee instead of an untested claim. The opaque-bytes choice was pragmatic for a skeleton but cannot exercise the type system it must certify.

**What ties it together for certification:** `arves-testvectors` (F11) is generated once from `arves-runtime` after F1–F3/F6 land, then frozen and versioned with the spec. A *second, independent* Rust team (or a Go/TypeScript team) that builds against the same CCP-published schemas must reproduce every vector — which is the operational definition of "independently implementable" that the corpus repeatedly demands but does not yet make achievable.

---

## If ARVES were standardized by ISO/IEEE tomorrow — what is still missing (this lens)

An ISO/IEEE working group would accept the **architecture** (planes, ownership, invariants, CP/AP boundary, lifecycle process) largely as-is — it is genuinely strong and precedent-aware. It would **reject the document set as a normative standard** on the grounds that the mandatory Independent-Implementability Tests (Ontology Part 11, Engine Graph Part 13, Reference Lifecycle Part 11) cannot currently be passed: there are **no type schemas, no canonical encoding, no content-addressing function, no wire formats, no decision-trace/fingerprint/conformance-artifact schemas, and no interoperability test vectors.** Every one of these is fixable *without reopening the freeze*, via the corpus's own IDR/CCP machinery, and all cluster onto a single CBOR/CDDL/multihash substrate. The gap between ARVES and an implementable international standard is not architectural — it is **one batch of "bytes-level" CCP Amendments plus an IDR and a test-vector corpus.** Until those exist, ARVES is a superb reference *implementation* of an idea; after them, it becomes a *standard two independent teams can prove they both built.*

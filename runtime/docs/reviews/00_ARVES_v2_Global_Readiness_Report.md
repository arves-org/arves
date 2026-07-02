# ARVES v2 — Global Readiness Report (Wave-1 Gate Synthesis)

**Author role:** Synthesis author, ARVES Global Readiness Report — Independent Validation Era.
**Inputs:** Twelve independent architect reviews (P01–P12) covering Independent-Runtime Challenge,
Academic Peer-Review, ISO/IEEE Readiness, Independent-Implementation Design, Formal Verification,
Scalability, Security (Zero-Trust), Failure Engineering, Ecosystem, Product Strategy, Reference-
Implementation Audit, and the Twenty-Year Retrospective.
**Governing constraint:** The frozen `.docx` specification corpus is IMMUTABLE (doctrine ED-001).
**No recommendation in this report edits the frozen spec.** Every remedy is one of the sanctioned
instruments: **IDR** (engineering decision) · **CCP Amendment** (activates already-reserved
semantics; must pass CCP-GATE = amendment + one conformance scenario) · **Runtime** ·
**Verification** · **Certification** · **Ecosystem** · **Product**.
**Doctrines honored:** ED-001 (freeze immutable; evidence lives in a NEW `verification/` tree),
ED-002 (one fundamental property proven per milestone), ED-003 (adversarial hunt mandatory),
RT-001 (runtime interfaces evolve only by activating reserved semantics).
**Date:** 2026-07-02

---

## 1. Executive Verdict

**Is ARVES ISO/IEEE-ready / independently-implementable today?**

**NO — but fixable without reopening the freeze.**

Gate result (composite of the three structured Wave-1 verdicts): **NOT-READY-but-fixable-without-
reopening-the-freeze.** The strongest single lens (P01 Independent Runtime Challenge) returned
**NOT-READY**; the two conditional lenses (P03 ISO/IEEE Readiness = **CONDITIONAL**, P05 Formal
Verification = **CONDITIONAL**) both cleared their gate only on the explicit condition that the
missing artifacts be produced first. Taken together the twelve lenses converge on one gate posture:
a rival team given only the frozen corpus **cannot** build a conformant, interoperable ARVES runtime
today, and ARVES **cannot** be submitted to a standards body today — yet **not one** of the blocking
findings requires changing a decision the specification actually made. Every remedy is additive
(CCP Amendment, IDR, Verification, Certification, Ecosystem, Product) and lands inside the corpus's
own change machinery.

**The single convergent thesis across all twelve lenses:** *ARVES has a world-class meta-layer and
an empty object-layer.* The change process (Reference Lifecycle), the conformance framework shape,
the invariant registry, the two-plane ownership algebra (ORCH-001/OWN-001/LAYER-001/SHARD-001), the
era/freeze discipline, and the "replay-from-trace not recompute" bet (ORCH-003) are genuinely
excellent and are the scaffolding a 20-year standard needs — but the actual bytes on the wire (the
content-addressing hash and canonical pre-image, the serialization/envelope, the `uci.*` type
schemas, the decision-trace and fingerprint schema), the formal statements of the invariants, the
populated conformance suite, and the defined semantics of the load-bearing primitives are almost
entirely absent. The corpus's own Gap Analysis calls itself *"90% naming & structure, 10% content."*
The academic anchor states it exactly: **"the architecture is sound enough to formalize; the claims
currently outrun the evidence."** This is a FORMALIZATION signal, not a redesign signal — ARVES
froze the architecture prematurely relative to its science, and the corrective is a Formalization &
Verification program plus an object-layer CCP batch, executed through the freeze-preserving
instruments the corpus already built.

---

## 2. Critical Cross-Cutting Gaps

The findings that recur across multiple lenses are, by construction, the ones that block
standardization and independent implementation. Nine are surfaced below, each with the lenses that
raised it, why it blocks, and the closing instrument. (The single de-duplicated register is §3.)

### CCG-1 — The interoperability surface is undefined: content-addressing (the epicenter)
**Raised by:** P01 (F1, "load-bearing, GATE-blocking"), P04 (F2), P05 (P05-4), P07 (S3), P08 (F-08),
P11 (F3), P12 (F1). Structured anchor: **P01 = NOT-READY** names ORCH-004 content-addressing as the
biggest gap; **P05 = CONDITIONAL** names it unverifiable as written.
**Why it blocks:** ORCH-004 ("idempotent and content-addressable") is the most-cited invariant in
the corpus and the "bridge to distribution," yet no document defines the hash function, the pre-image,
or the canonicalization. Two independent runtimes will content-address differently, so their
idempotency keys, dedup, and decision traces never agree — truth silently forks across "conformant"
runtimes. The reference runtime confirms the gap is deliberate and unresolved (five crates use three
incompatible shapes; snapshots use CRC-32, a torn-write detector, in the content-address role — a
collision/second-preimage hole per P07/P08/P11/P12).
**Instrument:** CCP Amendment "ORCH-004 Content-Addressing Contract" — self-describing (multihash-
style) tagged, versioned digest over a defined canonical pre-image; SHA-256 baseline; forbid CRC-class
in the address role; conformance scenario asserting two independent code paths derive identical
addresses for the same canonical payload.

### CCG-2 — The interoperability surface is undefined: canonical serialization / wire format
**Raised by:** P01 (F2, explicitly delegated to IDR = outside the standard), P03 (F6), P04 (F2/F8),
P12 (F3, "opaque payload → runtimes clone the reference, not the spec").
**Why it blocks:** No frozen document defines how any artifact serializes on the wire or on disk;
the wire format is explicitly classified as an IDR (a reference-implementation decision an independent
reader never sees). A standard whose wire format is an implementation detail is, definitionally, not a
wire standard — two runtimes cannot exchange a single event, manifest, or trace, and the reference
runtime's incidental fixed-order-little-endian blob became the de-facto format (P12). Content-
addressing (CCG-1) is meaningless without a canonical pre-image, so this and CCG-1 must land together.
**Instrument:** CCP Amendment "ARVES Serialization & Envelope Binding" — promote the exchanged-form
encoding from IDR to normative (deterministic CBOR / RFC 8785 JCS as the addressing pre-image; a
transport binding; worked example payloads). Runtime-internal storage layout remains an IDR.

### CCG-3 — The interoperability surface is undefined: `uci.*` type schemas are empty
**Raised by:** P01 (F3, "the ABI type loop closes on empty types"), P03 (F6), P04 (F1, "the keystone"),
P09 (E1, "no IDL to generate from"), P12 (F8, no schema-evolution rules).
**Why it blocks:** The Ontology Spec is declared the root of UCS and the dependency that "closes the
Engine Graph ABI," and defines a registry entry as `{ urn, version, aspects, schema, relations }` —
but the `schema` is never populated for any type. Engines declare `Reads: [uci.fact@1]` against names
with no shape; an independent runtime cannot validate inputs, serialize outputs interoperably, or check
preconditions. The predecessor ARVES-19 gives all 16 entities the identical seven-attribute stub.
Every SDK, connector, harness, and marketplace validator needs this one machine-readable artifact.
**Instrument:** CCP Amendment "uci.* Type Schemas v1" — choose a normative schema encoding
(CDDL/JSON-Schema), populate the 18 root types + 5 aspects + relations with real field schemas,
cardinalities, and one example instance each; add additive-only, field-preserving schema-evolution
rules (P12-F8).

### CCG-4 — The interoperability surface is undefined: decision-trace & runtime-fingerprint schema
**Raised by:** P01 (F5), P04 (F3), P05 (P05-2, ORCH-003 conflates three obligations).
**Why it blocks:** ORCH-003 requires replay "from a recorded decision trace, not by recomputation,"
and Certification's central promise is a third party replaying the artifact to reconstruct a verdict —
but the trace has no defined schema and "Runtime Fingerprint" is a prose list. Only the producing
runtime can replay it, so "independent verification" collapses into "trust the vendor's replayer."
P05 further shows ORCH-003 bundles three distinct provable obligations (replay-determinism, replay-
equivalence, trace-completeness) requiring different proof techniques and never separated.
**Instrument:** CCP Amendment "Decision Trace & Runtime Fingerprint Schema" (typed, versioned,
content-addressed; reuses CCG-1/CCG-2 substrate) + a Verification decomposition of ORCH-003 into
003a/b/c with a sealed-replay harness proving "not by recomputation."

### CCG-5 — No formal / normative-language discipline (the claims outrun the evidence)
**Raised by:** P02 (F1 "Formal Proof claimed but absent," F3 distributed correctness never model-
checked), P03 (F1 no RFC 2119, F2 no Terms & Definitions), P05 (P05-1 no formal spec of any property,
P05-6 no liveness stated anywhere), P12 (F2 invariants named but never machine-checked). Structured
anchors: **P03 = CONDITIONAL** (no normative language, no glossary); **P05 = CONDITIONAL** (no formal
spec; 18 invariants cited-but-undefined; LAYER-001/OWN-001 are the cheapest, highest-leverage proofs).
**Why it blocks:** The Baseline claims "Mathematics, Formal Proof" as delivered; a corpus-wide search
finds zero theorems, lemmas, model-checks, or refinement mappings. Requirements are declarative prose
with ~12 RFC-2119 keywords corpus-wide, no Terms & Definitions clause, and no formal object to diff
against — so a working group cannot mechanically extract a testable requirements set, and two honest
teams build two non-interoperable "conformant" runtimes. Safety and liveness are never separated.
**Instrument:** IDR "Formal Specification companion (TLA+) + build-time LAYER-001/OWN-001 architecture
gate" (highest leverage / lowest cost first), plus CCP Amendment "Normative Language Convention (RFC
2119) + Terms & Definitions." The formal companion + glossary are the object every other proof cites.

### CCG-6 — Conformance is unpopulated and structurally cannot detect cross-implementation divergence
**Raised by:** P01 (F4 "both can PASS and be mutually unintelligible"), P03 (F3 "framework, not a
suite — by the corpus's own admission"), P05 (P05-5 zero verification tooling, conformance carries
zero populated assertions), P06 (F9 no scale scenarios), P07 (S9 no security axis).
**Why it blocks:** Conformance is property-based (correct for non-deterministic inference) but the
non-determinism argument has been over-applied to the whole artifact, including the parts that must be
byte-identical for interop (types, envelopes, digests, trace framing). Two runtimes agreeing on none
of those can both certify — the worst outcome for a standard, manufacturing false confidence in
interop. The `arves-conformance` crate is an interface skeleton with no populated assertions; there is
no property-based testing, model checking, or golden-vector corpus.
**Instrument:** Verification (populate the predicate library keyed to the artifact schema; a
differential/interop tier that pins the byte-exact deterministic surface once CCG-1..4 land) +
Certification (a cross-verifier and a second independent runtime as an actual passing test). Depends on
CCG-1..4 existing first.

### CCG-7 — Undefined primitive semantics: the ontology has no model theory
**Raised by:** P02 (F2 "you have a vocabulary, not an ontology"; F6 unfalsifiable O-principles beside
provable invariants), P03 (F2 load-bearing terms defined only by usage).
**Why it blocks:** "Cognitive truth," "intelligence," "cognitive entity," "meaning" are the load-
bearing nouns; none is defined with necessary/sufficient conditions or an interpretation function.
Two conformant runtimes can register the same URNs and disagree on what a `uci.fact` *is*. The central
invariants ("only the Kernel owns Truth") are uninterpretable to an outsider when "Truth" is undefined.
**Instrument:** Verification companion "Ontology Semantics Reference" (interpretation domain, relation
axioms, a decidable consistency check, and the `uci.fact`-vs-Kernel-committed-truth distinction) +
CCP Amendment glossary (CCG-5) as its substrate. This *interprets* the frozen registry; it does not
modify it.

### CCG-8 — Cross-shard saga correctness has no property, axis, or proof
**Raised by:** P05 (P05-9 "the one place ARVES abandons single-shard atomicity is the one place with
no property to verify"), P06 (F4 no saga coordinator / back-pressure), P08 (F-05 distributed failure
model undesigned), P12 (F7 "the easy 80% specified and tested; cross-shard is the incident-prone
surface").
**Why it blocks:** IDR-001 correctly forbids cross-shard atomic commit and mandates sagas/compensation
— but no correctness criterion is stated (atomicity-modulo-compensation, compensation idempotency,
saga isolation, saga liveness), no coordinator exists, no back-pressure/admission control exists, and
there is no conformance axis. "Single source of cognitive truth" (G-001) is a global claim while sagas
make truth per-shard with undefined cross-shard semantics — the least-tested, most bug-prone surface.
**Instrument:** IDR "Saga Correctness & Coordinator" (atomicity-modulo-compensation + idempotent
compensation via ORCH-004 + declared isolation level + saga liveness) + Verification (TLA+ saga module
with failure injection) + a CCP-gated cross-shard conformance scenario (proposed invariant SAGA-001).

### CCG-9 — Governance vacuum + reference-implementation as un-canonical de-facto standard
**Raised by:** P09 (E2 no ownership charter / CCP adjudicator / IPR-trademark regime), P11 (F1 IDR
citation drift — the same concept cited as three different IDR numbers across crates; F4 `RT-001`
invented in code), P12 (F9 deferred certification launch with no neutral steward → monoculture).
**Why it blocks:** The corpus presumes a "single owner" and an "arms-length review board" everywhere
but constitutes neither; there is no legal custodian, CCP tie-break/appeal procedure, or IPR/patent
covenant, so the first commercially-loaded CCP forks the standard. Simultaneously the reference
implementation's traceability — its primary value as the tie-breaker — is self-contradictory (IDR
citations disagree with the frozen source and with each other), and the runtime minted its own
governance token (`RT-001`) outside the four sanctioned instruments. Deferring certification launch
with no interim neutral steward cements the reference runtime as the de-facto standard before any
independent certification counterbalances it.
**Instrument:** Ecosystem (charter a neutral steward / foundation holding the suite + CCP registry +
trademark + royalty-free patent covenant) + Verification (compiler-checked IDR citation catalog in
`arves-invariants`, CI lint failing on string-literal IDR citations) + CCP/IDR to re-home `RT-001`.

---

## 3. De-Duplicated, Severity-Ranked Proposal Register

Overlapping findings merged across all twelve lenses (many independently flagged content-addressing,
wire format, glossary, and formal properties — these are collapsed into single proposals). Ranked
critical-first. Instrument, severity, and implementation-complexity are the merged consensus of the
contributing lenses.

| ID | Title | Instrument | Severity | Impl-Complexity | Lenses that raised it | One-line recommendation |
|----|-------|-----------|----------|-----------------|-----------------------|-------------------------|
| **R-01** | ORCH-004 Content-Addressing Contract (tagged/versioned digest over a canonical pre-image; ban CRC in the address role) | CCP-Amendment | Critical | high | P01·P04·P05·P07·P08·P11·P12 | Freeze a multihash-style SHA-256 digest + canonical pre-image; scenario proves two runtimes derive identical addresses. |
| **R-02** | ARVES Serialization & Envelope Binding (promote the exchanged wire form from IDR to normative) | CCP-Amendment | Critical | high | P01·P03·P04·P12 | Freeze deterministic CBOR/RFC 8785 as the addressing/exchange encoding with worked example payloads. |
| **R-03** | uci.* Type Schemas v1 (populate the reserved `schema` slot for all 18 root types + 5 aspects + relations) + additive schema-evolution rules | CCP-Amendment | Critical | very-high | P01·P03·P04·P09·P12 | Choose CDDL/JSON-Schema; ship real field schemas + one example instance per type; field-preserving evolution. |
| **R-04** | Decision Trace & Runtime Fingerprint Schema; decompose ORCH-003 into determinism/equivalence/completeness | CCP-Amendment + Verification | Critical | high | P01·P04·P05 | Typed, versioned, content-addressed trace; sealed-replay harness proves "not by recomputation." |
| **R-05** | Formal Specification companion (TLA+ invariant core + liveness under fairness) + build-time LAYER-001/OWN-001 architecture gate | IDR + Verification | Critical | very-high (gate: low) | P02·P03·P05·P11·P12 | Model-check the 7 registered invariants; ship the cheap static layer/ownership gate first. |
| **R-06** | Normative Language Convention (RFC 2119/8174) + Terms & Definitions glossary + requirement IDs | CCP-Amendment | Critical | high | P02·P03 | Keyword every invariant/contract sentence; author one normative glossary; give each requirement a stable ID. |
| **R-07** | Populate the conformance suite + build a differential/interop conformance tier (byte-exact deterministic surface) | Verification + Certification | Critical | very-high | P01·P03·P05·P06·P07 | Fill the predicate library; add a cross-runtime tier that pins types/envelopes/digests/trace once R-01..R-04 land. |
| **R-08** | Ratify the 18 cited-but-undefined invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009, ENG-001..005) via CCP-GATE, each with a formal property | CCP-Amendment | High | medium | P01(F13)·P05(P05-3) | Ratify Batch-1 (G/QUERY/LCW/PERSIST-001) first, each with a TLA+ statement + a conformance scenario. |
| **R-09** | Cryptographic tamper-evidence for the decision trace/WAL (hash-chained + signed segment heads); replace CRC32-as-integrity-root | IDR + Runtime + Certification | High | high | P07(S1)·P08(F-01/F-02)·P12(F11) | Hash-chain the append-only log, sign the head, make truth_hash cryptographic; scenario: tamper → recovery fails loud. |
| **R-10** | Authenticated/authorized/tenant-scoped commit gateway (deny-by-default) | IDR + Runtime | High | high | P07(S2·S8) | Commit accepts a signed shard-scoped authorization; add Unauthenticated/Unauthorized/ScopeViolation errors. |
| **R-11** | Ontology Semantics Reference (model theory: interpretation, relation axioms, decidable consistency; fact-vs-truth distinction) | Verification | High | high | P02(F2·F6)·P03(F2) | Give the type registry a model theory so "conformance to the ontology" is definable and O-005 becomes checkable. |
| **R-12** | Saga Correctness & Coordinator + cross-shard conformance axis (proposed SAGA-001) + back-pressure/admission control | IDR + Verification | High | very-high | P05(P05-9)·P06(F4)·P08(F-05)·P12(F7) | Define atomicity-modulo-compensation + idempotent compensation + isolation level; TLA+ + failure-injection; scenario. |
| **R-13** | Neutral governance steward / Foundation charter (CCP adjudicator, IPR/royalty-free patent covenant, "ARVES" trademark) | Ecosystem | High | high | P09(E2)·P12(F9) | Constitute the "single owner" the corpus assumes as a vendor-neutral body before the first commercial CCP. |
| **R-14** | Machine-readable contract registry (IDL) + generated SDKs with by-construction "cannot bypass Kernel" safety layer | Ecosystem | High | high | P09(E1·E5)·P04(module arch) | Publish the ontology IDL + manifest schema as verifiable projections; generate SDKs; hand-write only the narrow commit/read safety layer. |
| **R-15** | Canonical IDR citation catalog + shard-key type; enforce single normative type per concept | IDR + Verification | High | high | P11(F1·F5)·P04 | Add a compiler-checked `idr`/`ShardKey` catalog in `arves-invariants`; CI fails on string-literal IDR/duplicate shard-key. |
| **R-16** | Engine Manifest Schema v1 (typed, serialized, enum'd, precondition grammar, content-addressed) | CCP-Amendment | High | high | P01(F8)·P04(F4)·P12(F6) | Freeze a typed manifest schema with closed policy enums + a total precondition mini-language + one runnable example. |
| **R-17** | Reconcile the Event Envelope (two frozen variants) + populate per-event payload schemas; consider CloudEvents binding | CCP-Amendment | High | medium | P01(F6)·P03(F5)·P04(F8) | Declare the ARVES-21 nine-field envelope authoritative; type every field; populate the contract template. |
| **R-18** | Fault Model + Durability-Assumption Register (medium-distrust, manifest/segment integrity, quarantine-not-panic) | CCP-Amendment + Runtime | High | high | P08(F-01·F-02·F-04·F-06) | Promote "lossless or loud" to a normative fault model; add durable segment manifest + per-shard quarantine states. |
| **R-19** | Fault-injection filesystem + crash-consistency model checking in the conformance suite | Verification + Certification | High | high | P08(F-03) | Add a deterministic fault-injection WalStore + proptest crash schedules as a certification gate. |
| **R-20** | Reconcile the two "decision trace" owners (Wal vs DecisionTrace) into one ordered source per shard | IDR | Medium | medium | P11(F6) | Make control-plane DecisionTrace a typed projection over the one WAL, sharing the offset space (IDR-005). |
| **R-21** | API Catalog → normative machine-readable API + error model (OpenAPI for core control surfaces) | CCP-Amendment | Medium | medium | P01(F7)·P03(F6) | Publish OpenAPI for Goal/Planning/Query/Execution/Conformance-retrieval with request/response + a standard error model. |
| **R-22** | Arbitration/join-node determinism + graph-expansion termination (ranking function) | CCP-Amendment + Verification | Medium | high | P01(F9·F10)·P05(P05-8/P05-10)·P02(F8) | Define arbitration as a deterministic function of branch contents with content-hash tie-break; prove expansion terminates. |
| **R-23** | Read-consistency tier guarantees + read-index/follower-read/cross-shard fan-out for scale | IDR + Runtime | Medium | high | P01(F12)·P06(F6) | Specify staleness bounds + read-index; add follower reads and a scatter-gather query coordinator with fan-out limits. |
| **R-24** | Scalability reconciliation: SHARD-001 immutability vs resharding (sub-shard ranges + coalesced Multi-Raft) + placement/metadata plane | IDR + Certification | Medium | high | P06(F1·F2·F3·F5) | Sub-shard ranges under the immutable key + a hierarchical CP meta-index; group-commit fsync + per-shard locking; scale scenarios. |
| **R-25** | Capability/engine sandbox + signed supply-chain (SLSA/sigstore) for manifests and the runtime | IDR + Certification + Ecosystem | Medium | very-high | P07(S6·S7) | Deny-by-default WASM sandbox enforcing declared effect class; verify signed, provenance-bearing manifests at bind time. |
| **R-26** | Ingress trust adjudication + evidence gate (operationalize O-004 against poisoning/prompt-injection) | IDR + Runtime + Certification | Medium | high | P07(S5) | Clamp connector-asserted trust to a signed source registry; refuse Observation→Fact without validated Evidence. |
| **R-27** | Crypto-agility / post-quantum envelope for identity, audit signatures, attestation | IDR + CCP-Amendment | Medium | high | P12(F11)·P07 | Algorithm-tagged, versioned signature/identity envelope with a frozen baseline + reserved PQ successor tags. |
| **R-28** | ARVES Application Bundle (AAB) + Certified-Product profile ("built on ARVES" becomes verifiable) | IDR + Certification | Medium | high | P10(F1·F3)·P09 | Define a content-addressed, versioned bundle composing frozen artifacts by reference; certify the product, not just the runtime. |
| **R-29** | Conformance-first lighthouse product line from the 4 reference scenarios + design-partner/independent-runtime program | Product + Ecosystem | Medium | medium | P10(F2·F5·F10)·P02(F11) | Build Knowledge Fabric → Incident Command → Compliance Copilot → Robotics Dispatch; each release is a conformance run + ISO evidence. |
| **R-30** | Evaluation methodology + reproducibility package + related-work/novelty positioning | Verification + Ecosystem | Low | high | P02(F7·F10)·P12 | Operationalize the success metrics with baselines/benchmarks; publish the novelty argument for peer-reviewed status. |
| **R-31** | Long-term WAL readability / migration + UCS↔UCI compatibility matrix & support window | IDR + CCP-Amendment | Low | medium | P12(F4·F12)·P08 | Mandate forward-compatible decoders + a "Decade Replay" scenario; publish the compatibility matrix and sunset policy. |
| **R-32** | Version-drift / supersession hygiene (stamp superseded docs; consolidate the Documentation Index) | CCP-Amendment | Low | low | P01(F14)·P03(F10·F12) | Machine-checkable supersession headers + one authoritative Documentation Index with a changelog. |
| **R-33** | Learned-state provenance & fingerprinting (make Vol 7 continual learning replayable/auditable) | IDR | Low | high | P12(F10) | Commit learned state through the Kernel as versioned, provenance-bearing truth pinned in the Runtime Fingerprint. |
| **R-34** | Connector/marketplace trust substrate (signed content-addressed manifests, revocation, provenance-population conformance) | Ecosystem | Low | high | P09(E6·E7)·P10(F4·F9) | Signed connector/engine manifests + revocation list + a scenario failing any connector that omits the Provenance aspect. |

**Reading the register:** R-01..R-07 are the critical spine — the object-layer interoperability
surface (R-01..R-04), the formal/normative discipline (R-05..R-06), and the conformance that binds
them (R-07). R-01..R-04 unlock nearly everything else; R-07 depends on them existing first. R-08..R-19
are the high-severity body (invariant ratification, security, saga correctness, governance, SDK/IDL,
reference-implementation purity, durability). R-20..R-34 are medium/low — real, but sequenced after
the spine because they compose on top of it.

---

## 4. Recommended Sequencing — Five Parallel Programs

The maintainer's structure. Each critical gap is mapped to the program that closes it. All evidence
lives in a new `verification/` tree (ED-001: the frozen `.docx` corpus is never touched).

### Program A — Reference Runtime (I2..I6, ongoing)
**Owns:** the milestone chain. **Proves next:** I2 Cluster Kernel → **Replication** (per ED-002, one
fundamental property per milestone; Program A's next proof obligation is replication, not the object-
layer). **Closes:** R-09/R-10 (authenticated, tamper-evident commit as replication lands), R-18/R-19
(fault model + fault-injection, which P08 argues must be designed *before* I2.1 code so replication
inherits the failure contract rather than retrofitting it), R-20 (single-owner decision trace),
R-23/R-24 (read tiers, resharding/placement — I3 onward), R-15 (IDR citation + shard-key canonical
type, a mechanical purity fix that should precede more distributed code). Runtime-side scaling work
(group-commit fsync, per-shard locking, streaming/parallel recovery) rides here.

### Program B — Verification Program (targets the Truth/Entity semantics + distributed proofs)
- **V1 Formal Semantics:** the TLA+ invariant-core companion (R-05) + Ontology Semantics Reference
  (R-11) + RFC-2119/glossary substrate (R-06). Closes CCG-5, CCG-7.
- **V2 Mathematical Proofs:** decompose ORCH-003 (R-04 Verification half); ratified-invariant proofs
  (R-08); arbitration/termination ranking functions (R-22).
- **V3 Model Checking:** TLC/Apalache/`stateright` on per-shard Raft (IDR-001..004 safety + liveness),
  the saga module (R-12), tenant non-interference; Kani/`loom` on the kernel. Closes CCG-8's proof half.
- **V4 Runtime Verification:** populate the conformance predicate library + fault-injection harness
  (R-07, R-19); the sealed-replay harness (R-04). The cheapest, highest-leverage early win is the
  build-time LAYER-001/OWN-001 architecture gate (R-05 gate half) — low cost, covers all executions.
- **V5 Third-party Certification:** the differential/interop conformance tier + cross-verifier + the
  second independent runtime as a passing test (R-07 Certification half). Hands off to Program D.

### Program C — Independent Runtime (second team, different language, spec-only)
**Goal:** convergence / differential conformance. A firewalled team builds a bounded slice (e.g. L1
Core: Information → Kernel → Query) from the frozen spec **plus the R-01..R-04 CCP artifacts alone**,
recording every forced guess (P02-F11, P04's ambiguity ledger, P12-F3). Every guess is a defect closed
by IDR/CCP — never by consulting authors, never by cloning the reference runtime. This is the
operational test of CCG-1..4: two implementations must reproduce every interop test vector byte-for-
byte on the content-addressed surface. Closes the P01 NOT-READY verdict by construction. Blocked until
R-01..R-04 exist; that is why the object-layer CCP batch is the gating prerequisite for this program.

### Program D — Certification (third-party conformity assessment, ISO/IEC 17065 style)
**Closes:** CCG-6 (as an operationalized program), R-28 (Certified-Product profile / AAB), R-25 (signed
supply-chain admission), the "Regulated Autonomy" attestation profile (P12-F5), lab accreditation /
surveillance / appeals / decertification (P03-F7), and the neutral-steward-owned suite (R-13, shared
with Program E). Consumes Program B's V4/V5 artifacts; issues the portable, replayable conformance
certificate.

### Program E — Developer Ecosystem (SDKs, marketplace, products)
**Closes:** R-13/R-14 (Foundation charter + IDL + generated SDKs), R-34 (connector/marketplace trust
substrate), R-29 (conformance-first lighthouse products + design-partner program), R-30 (novelty
positioning), R-31/R-32 (compatibility matrix, doc hygiene). Strictly depends on R-01..R-03 (the IDL
is a projection of the object-layer CCPs) and R-13 (nothing legitimate happens without a neutral owner).

**Proposed `verification/` directory tree** (new tree, ED-001-compliant — evidence lives here, not in
the spec):

```
verification/
├── mathematics/              # theorem statements, hand proofs, proof-obligation ledger
├── semantics/                # Ontology Semantics Reference; RFC-2119 glossary; term model theory
├── proofs/                   # TLA+ invariant-core companion; ORCH-003a/b/c decomposition; saga proofs
├── model-checking/           # TLC/Apalache/stateright Raft + saga; Kani/loom kernel harnesses
├── runtime-verification/     # proptest suites; sealed-replay harness; fault-injection WalStore; LAYER/OWN gate
├── certification/            # conformance predicate library; differential/interop tier; cross-verifier; profiles
├── benchmarks/               # scale + throughput + recovery-time budgets; evaluation methodology
├── reproducibility/          # frozen interop test-vector corpus; golden traces; expected verdicts
└── independent-implementations/   # Program C second-runtime slice + its ambiguity ledger
```

---

## 5. Top Candidate Instruments to Draft Next

### Top 3 Candidate CCP Amendments

**CCP-A — "ORCH-004 Content-Addressing Contract" (R-01)**
- *Why:* the single most-cited invariant, named the biggest gap by the NOT-READY lens (P01), and
  unverifiable-as-written by the CONDITIONAL formal-verification lens (P05). It is the interoperability
  epicenter — every downstream guarantee (replay, dedup, distribution, cross-vendor certification)
  rests on it. Seven of twelve lenses raised it.
- *Risk:* choosing a fixed hash with no agility repeats SHA-1's history; leaving it opaque perpetuates
  the reference-runtime monoculture (P12) and the CRC-32-as-address soundness hole (P07/P08/P11).
- *Recommendation:* draft first. Self-describing (multihash-style) tagged, versioned digest; SHA-256
  baseline with reserved successor tags; forbid CRC-class in the address role; CCP-GATE scenario: two
  independent code paths derive identical addresses for the same canonical payload. Land jointly with
  CCP-B (an address needs a canonical pre-image).

**CCP-B — "ARVES Serialization & Envelope Binding" (R-02)**
- *Why:* a standard whose wire format is delegated to IDR is not a wire standard (P01-F2). No two
  runtimes can exchange a single artifact; the reference runtime's incidental encoding became the
  de-facto format (P12-F3). This is the canonical pre-image CCP-A hashes over.
- *Risk:* picks a serialization winner early; mitigate by making only the *exchanged* form normative
  (deterministic CBOR / RFC 8785) and keeping runtime-internal storage an IDR.
- *Recommendation:* draft second, paired with CCP-A. Ship worked example payloads (the Gap Analysis's
  top-priority missing artifact); scenario: round-trip + content-address stability across encodings.

**CCP-C — "uci.* Type Schemas v1" (R-03)**
- *Why:* the keystone (P04-F1). The ABI's portability claim closes onto empty types; no SDK, connector,
  harness, or marketplace validator can exist without this one machine-readable artifact (P09-E1).
- *Risk:* very-high effort (18 root types + aspects + subtypes + evolution rules); mitigate by
  deepening one vertical slice end-to-end first (Gap Analysis recommendation), then templating.
- *Recommendation:* draft third; choose CDDL/JSON-Schema; populate the reserved `schema` slot (this is
  *activating already-reserved semantics*, squarely RT-001/CCP-appropriate) with additive, field-
  preserving evolution rules (P12-F8) and one example instance per type.

*(Runner-up CCP: "Normative Language Convention (RFC 2119) + Terms & Definitions" (R-06) — cheap,
unblocks mechanical requirement extraction and every downstream predicate; draft alongside the batch.)*

### Top 3 Candidate IDRs

**IDR-α — "Formal Specification companion (TLA+) + build-time LAYER-001/OWN-001 architecture gate" (R-05)**
- *Why:* P05 names this the highest-leverage next step (eight of nine formal findings reference the
  formal object it creates); P05-8/P11 note LAYER-001/OWN-001 are provable *statically* at build time —
  the cheapest proof in the whole program, covering all executions at once. P02/P12 make it the
  credibility lever that turns "claims that outrun evidence" into a proof-obligation ledger.
- *Risk:* very-high for the full TLA+ companion (new skill, CI integration); but the static
  architecture gate is *low* cost and can ship immediately.
- *Recommendation:* draft the IDR now; deliver the build-time layer/ownership gate first (immediate
  win), then the TLA+ invariant-core companion under Program B/V1. Aligns with ED-002 (I1's provable
  property) and ED-003 (adversarial).

**IDR-β — "Consensus-context injection + canonical IDR citation & shard-key catalog" (R-15, P11-F2/F5)**
- *Why:* the reference implementation is the tie-breaker for ambiguous prose, yet its IDR citations are
  self-contradictory (same concept → three IDR numbers) and SHARD-001 has ~7 incompatible types; the
  Kernel also mints a consensus-owned `term: 0`. These corrupt traceability and pre-bake an I2 rework.
- *Risk:* touches every crate's doc comments + on-disk dir naming; but it is mechanical (comments +
  one constants module + a CI lint), not behavioral.
- *Recommendation:* draft before more distributed code lands. Add a compiler-checked `idr`/`ShardKey`
  catalog in `arves-invariants`; forbid the Kernel from originating a `Term` (inject a `LeaderContext`).

**IDR-γ — "Saga Correctness & Coordinator" (R-12, feeding a CCP-gated SAGA-001)**
- *Why:* the one place ARVES abandons single-shard atomicity is the one place with no property to verify
  (P05-9), no coordinator/back-pressure (P06-F4), no failure model (P08-F-05), and no conformance axis
  (P12-F7) — the most bug-prone surface in the ecosystem.
- *Risk:* saga verification is genuinely hard (very-high); the isolation level and compensation algebra
  must be stated precisely or the standard under-defines its hardest surface.
- *Recommendation:* draft as I2/I3 distributed work matures. Define atomicity-modulo-compensation +
  idempotent compensation (via ORCH-004) + declared isolation + saga liveness; verify with a TLA+ saga
  module + failure injection; ratify SAGA-001 through CCP-GATE with a cross-tenant-handoff scenario.

---

## 6. When To Do What — Before I2 vs During I2..I6 vs Post-1.0

Program A proves **Replication** next (I2 Cluster Kernel). Sequencing respects that: the object-layer
CCP batch is *concurrent* Program B/E work that must not stall Program A's replication proof, but a few
items are prerequisites *for* correct replication and must land before I2.1 code.

### Before resuming I2 (prerequisites for correct replication + the gating batch for Program C)
- **IDR-β (R-15):** consensus-context injection + canonical IDR/shard-key catalog. The Kernel must
  stop minting `term: 0` and there must be one shard-key type *before* replication wires `term` and
  cross-node shard identity into durable, hard-to-migrate records (P11-F2/F5).
- **R-18 Fault Model + R-19 fault-injection harness (design, at least):** P08 is explicit — write the
  distributed fault model and stand up the crash/partition harness *before* I2.1 code, so replication
  is built to the failure contract, not the happy path.
- **The build-time LAYER-001/OWN-001 architecture gate (R-05, gate half):** cheapest proof, guards
  against structural drift as distributed code grows; ship immediately.
- **Kick off the object-layer CCP batch (CCP-A/B/C = R-01/R-02/R-03) + RFC-2119/glossary (R-06):**
  these run in parallel (Program B/E) and are the gating prerequisite for Program C (the second runtime
  cannot start until the interop surface is normative). They do not block I2 replication, but they are
  on the critical path to clearing the P01 NOT-READY verdict, so start them now.

### During I2..I6 (co-evolve with the milestone that owns each property, per ED-002)
- **I2 (Cluster Kernel / Replication):** R-09 tamper-evident + R-10 authenticated commit (replication
  is exactly when unauthenticated followers applying leader outcomes becomes dangerous — P07-S1/S2);
  V3 model-checking of per-shard Raft safety + liveness (R-05/R-08); R-20 single decision-trace owner.
- **I3 (Distributed Query):** R-23 read-tier guarantees + follower/cross-shard reads; R-24 resharding +
  placement/metadata plane; R-04 trace/fingerprint schema exercised end-to-end.
- **I4 (Capability Scheduling):** ratify ENG-*/CAP-* invariants (R-08 Batch 2/3); R-25 capability
  sandbox + signed supply-chain; capability fair-share/quotas/back-pressure (P06-F7).
- **I5 (Multi-Agent Runtime):** R-12/IDR-γ saga correctness + coordinator (cross-shard/multi-agent is
  where it bites); R-22 sub-planner-node convention + arbitration determinism; R-33 learned-state
  provenance.
- **I6 (Reference Products):** R-28 AAB + Certified-Product profile; R-29 conformance-first lighthouse
  products + design-partner program producing the ISO evidence package.
- **Continuous across I2..I6:** grow R-07 (populate conformance + differential/interop tier) as node
  contracts sharpen; each CCP arrives with its scenario (CCP-GATE); each milestone discharges its
  invariant's proof (ED-002) via an adversarial hunt (ED-003).

### Post-1.0 (after the reference runtime + populated conformance exist)
- **Program C convergence at full breadth:** certify a second independent runtime (and ideally a
  third-party lab, Program D/V5) — the operational proof that ARVES is a standard, not a product.
- **Program D certification launch:** lab accreditation, surveillance, appeals, decertification; the
  "Regulated Autonomy" attestation profile (P12-F5); ISO/IEC-17065-style conformity assessment; and the
  IEEE-SA study-group / ISO liaison, submitted by the neutral steward (R-13).
- **Program E scale-out:** marketplace trust substrate (R-34), full SDK matrix (R-14), curriculum, the
  novelty/related-work paper (R-30), crypto-agility/PQ migration rehearsal (R-27) and long-term WAL
  readability (R-31) — the 20-year durability items P12 flags as the difference between ratification
  and ossification.

---

*Report path: `c:/Users/hkuzudisli/Desktop/Arves-Foundation-Docs/runtime/docs/reviews/00_ARVES_v2_Global_Readiness_Report.md`*
*Synthesized from P01–P12. No frozen-spec modification proposed; every remedy is an IDR, CCP Amendment,
Runtime, Verification, Certification, Ecosystem, or Product instrument. Evidence lives in a new
`verification/` tree (ED-001).*

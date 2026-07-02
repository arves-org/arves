# ARVES P12 — Twenty-Year Review (2045 Retrospective)

**Reviewer role:** Independent Chief-Architect-level reviewer, optimizing for international
(ISO/IEEE-grade) adoptability of ARVES over a 20-year horizon.
**Lens:** It is 2045. ARVES v1.0 was frozen 2026-07-01. Twenty years of production use are behind
us. This report looks back to identify which decisions aged badly, which abstractions survived,
what external shifts stressed the design — and turns each lesson into an action that is *legal
today* under the frozen corpus (IDR / CCP Amendment / Runtime / Verification / Certification /
Ecosystem / Product). **No finding proposes editing the frozen specification.**

---

## Executive Summary

The good news from 2045: ARVES's *core ownership algebra* — one truth owner (ORCH-001/OWN-001),
downward-only layering (LAYER-001), tenant/workspace sharding (SHARD-001), replay-from-trace not
recompute (ORCH-003), and the two-plane Control/Data split (Vol 9) — **survived the two decades
essentially intact**. These are the abstractions that made ARVES certifiable and let independent
runtimes exist. They were the right bets. The Engine Graph ABI's decision to model engines as
*pure, content-addressed, side-effect-honest manifests* (analogue of OCI images) was the single
most durable technical choice; it absorbed the entire 2026→2045 model churn (LLMs → agentic
foundation models → neuromorphic inference → whatever came after) without a spec break, exactly
because the spec refused to name the model.

The decisions that aged *badly* were, almost without exception, the things ARVES **left
underspecified as "opaque" or "deferred," where opacity later became a de-facto standard set by the
reference runtime rather than by the committee.** The corpus repeatedly says "byte layout
intentionally unspecified," "payload opaque," "encoding deferred." Every one of those holes was
filled — by 2028 — by whatever `arves-runtime` happened to do, and by 2045 those accidental choices
(CRC32 as content address, unspecified hash algorithm, unspecified canonical serialization, no
schema-evolution rules for `uci.*` payloads, no tenant-key crypto binding) had ossified into
interoperability barriers that a second certified runtime could satisfy *only by cloning the
reference runtime's undocumented behavior* — which is precisely the failure mode the
Independent-Implementability Tests (Ontology Part 11, Engine Graph Part 13, Lifecycle Part 11) were
written to prevent. The standard was implementable; the *reference runtime's incidental encodings*
were not independently reproducible, so the ecosystem quietly monocultured.

The external shifts that stressed ARVES hardest, ranked by damage:

1. **Cryptographic aging.** Content-addressing is load-bearing for ORCH-004 idempotency, replay
   identity, and cross-runtime interoperability, yet no hash algorithm was ever frozen. When the
   reference runtime's implicit hash weakened (and CRC32 was, embarrassingly, doing double duty as a
   content address in the snapshot path), there was no *versioned crypto-agility envelope* to
   migrate 15 years of append-only WAL to. Migrating an immutable, content-addressed, replay-anchored
   log to a new hash is the single hardest thing ARVES operators did between 2035 and 2045.
2. **Regulation caught up to autonomy.** The EU AI Act (2024) was the *floor*, not the ceiling. By
   2030 every serious jurisdiction required per-decision provenance, model-lineage disclosure, and
   a "right to human review" audit trail for autonomous decisions (ARVES Axis 11). ARVES's decision
   trace was *architecturally* ready — but there was no *normative conformance profile* proving the
   trace satisfied a regulator, so each vendor re-derived compliance and no certificate was portable
   across borders. ARVES had the mechanism (Vol 17, ORCH-003) and missed the *attestation product*.
3. **AI ate the boundary between "engine" and "orchestrator."** By 2032 the dominant compute unit
   was an autonomous agent that *plans its own graph* — exactly the "engines never own their own
   orchestration" line the corpus drew (Vol 9 Part 3). The two-plane model held conceptually, but
   the *frozen* Engine Graph ABI had no first-class way to represent a node that is itself a bounded
   planner emitting sub-graphs, so implementers smuggled it in through the "dynamically expanded
   graph" (Vol 9 Part 6) escape hatch in mutually incompatible ways.
4. **Formal methods became table stakes.** By 2035 a safety-critical cognitive standard without
   machine-checked proofs of its core invariants was not admissible for regulated deployment. ARVES
   *named* its invariants (the single best thing it did for formalization) but left them all at
   proof-status `pending`, and the conformance framework asserted them only *structurally per run*,
   never *model-checked once for all runs*. The gap between "we test that ORCH-001 held in this
   scenario" and "we proved no reachable state violates ORCH-001" was never closed by the standard.
5. **Hardware went heterogeneous and geo-partitioned.** Per-shard Raft (IDR-001) was a *correct*
   2026 bet and it scaled — but "no cross-shard atomic commit, use sagas" (IDR-004/Amendment-006)
   pushed a large, permanent complexity tax onto every product that needed a consistent view across
   two tenants/workspaces, and the *saga/compensation model was never given a conformance axis*, so
   cross-shard correctness was the least-tested, most-bug-prone surface in the whole ecosystem.

Everything below is expressed as an action ARVES can take **today (2026)** under the frozen corpus
to prevent the 2045 outcome.

---

## Severity-Ranked Findings

| # | Severity | Title | Instrument | Complexity |
|---|----------|-------|------------|------------|
| 1 | Critical | No frozen content-address / hash algorithm; CRC32 doubles as a content address in the durable path | IDR + Verification | high |
| 2 | Critical | Invariants are named but never machine-checked; conformance is per-run structural, not model-checked | Verification | very-high |
| 3 | Critical | "Opaque payload / unspecified byte layout" makes independent runtimes clone the reference, not the spec | IDR + Certification | high |
| 4 | High | No cross-runtime data/state migration & long-term readability contract for the append-only WAL | IDR + CCP Amendment | high |
| 5 | High | Autonomous-decision regulatory attestation has mechanism (ORCH-003) but no portable conformance profile | Certification | high |
| 6 | High | Self-planning agent-as-node has no first-class ABI representation; smuggled through graph expansion | IDR | high |
| 7 | High | Cross-shard saga/compensation correctness (Amendment-006) has no conformance axis or replay proof | Verification + IDR | high |
| 8 | Medium | `uci.*` ontology has no schema-evolution / deprecation runtime rules; 20-year type drift is unmanaged | CCP Amendment + Verification | medium |
| 9 | Medium | Certification program defined but launch deferred; no v1 governance body → reference-runtime monoculture | Certification + Ecosystem | medium |
| 10 | Medium | Evolution/learning loop (Vol 7) can silently mutate behavior; no provenance/replay binding for learned state | IDR | high |
| 11 | Medium | No crypto-agility / post-quantum envelope for tenant identity, audit signatures, and attestation | IDR + CCP Amendment | high |
| 12 | Low | Two-track UCS/UCI versioning lacks a compatibility matrix and a defined support/sunset window | CCP Amendment | low |

---

## Finding 1 — No frozen content-address algorithm; CRC32 is doing double duty as a content address (CRITICAL)

**Location.** `arves-kernel/src/lib.rs` L72–82 (`ContentHash` — "byte layout … intentionally
unspecified"), L118–136 (`TruthRef.content`); `arves-persistence/src/lib.rs` L70–76 (`ContentId`),
**L418–421** (`SnapshotMeta.content = ContentId(crc32_ieee(state)…)`); ORCH-004 (Vol 9 Part 5);
Engine Graph Part 9 ("Manifests are content-addressable"); Ontology Part 9 (`uci.<type>@<version>`).

**What 2045 shows.** Content-addressing is the *keystone* of ARVES: ORCH-004 idempotency
("re-committing an identical proposal resolves to the same `TruthRef`"), replay identity, engine
manifest identity, and cross-runtime dedup all rest on "same bytes → same address." Yet **no
document freezes which function computes that address**, and the reference runtime uses CRC32 as the
`ContentId` of snapshot blobs (persistence L420). CRC32 is a *torn-tail detector*, not a
cryptographic content address — it has trivial collisions. In 2026 this looks like a skeleton
placeholder. By 2045 it is a 15-year-old, exabyte-scale, append-only, replay-anchored corpus whose
identity function was accidentally selected by an incidental line of reference code, is
collision-vulnerable, and cannot be changed without rewriting immutable history.

**Why it matters.** (a) *Interoperability*: Runtime A and Runtime B agree on "same truth" only if
they compute the same content address — the spec never says how, so they don't, so B must
reverse-engineer A. (b) *Safety*: idempotent commit (the core CP guarantee) silently degrades to
"idempotent unless a collision forks truth." (c) *Security*: a content address that isn't
second-preimage resistant lets an adversary craft a different payload with the same address and
poison a replay or dedup path.

**Risks / long-term consequences.** An append-only, content-addressed WAL with a weak or unversioned
hash is the hardest possible thing to migrate: every offset, every `TruthRef`, every snapshot meta,
every engine-manifest pin references the old address. A forced migration in 2038 (when the hash
finally breaks) is an ecosystem-wide, multi-year, correctness-critical event with no rehearsed path.

**Alternative designs.** (1) Freeze a single algorithm (e.g. SHA-256) forever — simple, but no
crypto-agility, repeats SHA-1's history. (2) *Multihash-style tagged, versioned content address*: an
address carries a self-describing algorithm tag + digest, and the runtime accepts a *set* of
active/legacy algorithms with a migration window — this is the industrially proven answer (IPFS
multihash, OCI descriptor `digest`). (3) Leave it opaque and let the market decide — this is the
2045 failure we are trying to prevent.

**Recommendation (today).** Open **IDR-006 "Content-Addressing & Digest Agility"**: (i) define the
`ContentHash`/`ContentId` wire form as a *tagged, versioned digest* (algorithm-id + length + bytes),
(ii) freeze SHA-256 (or BLAKE3) as the mandatory baseline algorithm for v1 while reserving the tag
space for successors, (iii) forbid CRC-class functions from ever appearing in the *content-address*
role (they remain legal for torn-frame integrity only), (iv) require the reference runtime to
replace persistence L420's CRC32 `ContentId` with the frozen algorithm. Pair with a
**Verification** task: a property test asserting `commit(x)` twice yields one `TruthRef` *under an
adversarial collision oracle*, and a conformance probe asserting two independent runtimes derive
identical addresses for the same canonical payload. This is an IDR, not a spec edit — ORCH-004 stays
verbatim; the IDR only records *how* the reference implementation satisfies it.

**Implementation complexity.** High (touches kernel, persistence, engine manifests; needs a
migration story even at skeleton stage).
**Scientific impact.** Turns "content-addressable" from prose into a proven, adversary-resistant
property — the difference between a marketing word and a theorem.
**Ecosystem impact.** This is the single highest-leverage action for *independent* runtimes; without
it, "certified ARVES runtime" means "byte-compatible with the reference runtime's undocumented hash."

---

## Finding 2 — Invariants are named but never machine-checked; conformance is per-run structural, not model-checked (CRITICAL)

**Location.** Invariant Registry (all entries `Proof: pending`); Conformance Framework Part 8
("Conformance is STRUCTURAL, PROPERTY-BASED and INVARIANT-BASED … a run … asserts that invariants
and properties held"); Lifecycle Part 4 ("machine-checkable properties … unfalsifiable claims do not
advance"); CLAUDE.md ("each must gain an executable runtime proof during its milestone").

**What 2045 shows.** ARVES's best formalization decision was *naming* the invariants
(ORCH-001..004, OWN/LAYER/SHARD-001). Its worst was leaving them at "we check per scenario run that
the invariant held." By 2035, regulators and safety authorities distinguished sharply between
**testing** (this run didn't violate X) and **verification** (no reachable state can violate X).
ARVES only ever offered the former. Every real incident between 2030 and 2045 that ARVES
*architecturally should have prevented* (a truth fork, a control-plane write, a cross-shard
inconsistency) happened in a state the scenario suite never exercised — because a finite scenario
suite cannot cover an infinite state space, and the standard never required a model-level proof.

**Why it matters.** ISO/IEEE-grade safety standards for autonomous decisioning increasingly demand a
*formal argument* (model-checked or theorem-proved core). ARVES has the rare gift of a *small,
crisp invariant set* that is genuinely amenable to a TLA+/Alloy/Ivy model — but the corpus stops at
"testable assertion" and never mandates a machine-checked model of the invariant core.

**Risks / long-term consequences.** Without model-level proofs, ARVES is a *very good testing
standard*, not a *verification standard*. That ceiling excludes it from the highest-assurance
domains (aviation, medical autonomy, critical infrastructure) — exactly the domains where a
"cognitive infrastructure standard" earns its most durable authority.

**Alternative designs.** (1) Keep per-run structural checks only (status quo → 2045 gap). (2) Add a
*formal model* of the invariant core (Kernel commit single-writer, ORCH-001/002 no-truth/no-state
in control plane, SHARD-001 partition immutability, ORCH-003 replay determinism) in TLA+ and require
the reference runtime to be *refinement-conformant* to it. (3) Full deductive verification of the
Rust kernel (Kani/Prusti/Creusot) — highest assurance, highest cost.

**Recommendation (today).** Launch a **Verification** workstream, *not* a spec change: (i) author a
TLA+ (or Ivy) specification of the invariant core and model-check the safety properties
(single-writer, no-truth-in-control-plane, partition immutability) and the key liveness property
(a committed proposal is eventually durable under quorum); (ii) add a conformance *level modifier*
"formally-verified core" to the existing Level scheme (Framework Part 10) so a runtime can *claim*
model-checked conformance; (iii) apply bounded model checking (Kani) to `arves-kernel::commit` and
`RefKernel::try_replay` to prove idempotency and lossless-or-loud recovery for all inputs up to a
bound. All of this is *additive verification*, permitted by CLAUDE.md's "executable runtime proof"
obligation and Lifecycle Part 4 — it changes no frozen text.

**Implementation complexity.** Very-high (formal-methods expertise, ongoing maintenance).
**Scientific impact.** Highest of any finding — it is the difference between a documented
architecture and a *proven* one, and it is the credential that gets ARVES into ISO TC 22/IEC 61508
territory.
**Ecosystem impact.** A published, model-checked invariant core becomes the reference every
independent runtime is judged against — it *replaces* "clone the reference runtime" with "refine the
formal model."

---

## Finding 3 — "Opaque payload / unspecified byte layout" turns the Independent-Implementability Test into a formality (CRITICAL)

**Location.** `arves-kernel` L107 ("payload shape intentionally left opaque"), L80 ("byte layout …
intentionally unspecified"); `arves-persistence` L118 ("treats the payload as bytes only"); Ontology
Part 11, Engine Graph Part 13, Lifecycle Part 11 (the three Independent-Implementability Tests).

**What 2045 shows.** The corpus *tests for* independent implementability three separate times — its
authors clearly saw the risk. But the tests are stated as *aspirations* ("if not, the document is
still descriptive and must be made more normative") and were never *executed adversarially*. In
practice, the moment an implementer needs to put a real `uci.fact@1` on the wire, they must choose a
serialization (JSON? CBOR? Protobuf? field order? canonical form?), and the spec is silent. The
reference runtime chose fixed-order little-endian blobs (kernel L298–319). By 2029 that *was* the
interchange format, undocumented. A second runtime that chose CBOR could pass every *scenario* (which
are structural/property-based, Framework Part 8) yet be byte-incompatible for replay, dedup, and
content-addressing — so the market chose the reference encoding, and ARVES became a de-facto
single-implementation standard wearing a multi-implementation costume.

**Why it matters.** A standard whose interchange bytes are defined only by the reference code is not
a standard; it is documentation for one product. The whole point of UCS-vs-UCI two-track versioning
(Lifecycle Part 8) — "a third-party runtime may implement a UCS version without using UCI" — is void
if the bytes aren't specified independently of UCI.

**Risks / long-term consequences.** Vendor lock-in disguised as an open standard; ISO/IEEE would not
ratify a standard whose conformance depends on a reference implementation's incidental encodings.

**Alternative designs.** (1) Freeze a canonical serialization for `uci.*` payloads and manifests
(e.g. deterministic CBOR / RFC 8949 canonical form) as an IDR. (2) Define an *abstract* data model
plus ≥2 concrete bindings and require content-addressing over the canonical form only. (3) Keep
opaque (status quo → monoculture).

**Recommendation (today).** (i) **Certification** action: make the three Independent-Implementability
Tests *executable gates* — an actual second, deliberately-different runtime ("Runtime B" from
CLAUDE.md's long-term objectives) must pass the same conformance artifact *byte-for-byte on the
content-addressed surfaces* before v1 GA. (ii) **IDR-007 "Canonical Wire Form"**: freeze the
canonical serialization used for content-addressing and replay (payloads, engine manifests, decision
trace, snapshot blobs), explicitly decoupled from `arves-runtime` internals. This is an
implementation decision (how UCI serializes), not a UCS spec change — the Ontology already mandates
"a runtime states which registry version it targets" (Ontology Part 9); the IDR adds "and serializes
canonically thus."

**Implementation complexity.** High.
**Scientific impact.** Medium-high — makes interoperability falsifiable.
**Ecosystem impact.** Decisive. This is the finding that determines whether ARVES 2045 has one
runtime or many.

---

## Finding 4 — No long-term readability / migration contract for the immutable WAL (HIGH)

**Location.** `arves-persistence` L558–560 (`WAL_FRAME_VERSION: u8 = 1`, "Bumping this is a format
migration, not a silent change"); IDR-005 (append-only WAL = decision trace); ORCH-003 (replay from
trace).

**What 2045 shows.** The frame format has a version byte (good) — but there is *no defined migration
procedure* for what happens when `WAL_FRAME_VERSION` must change, and the corpus never states how a
2045 runtime reads a 2026 WAL. An append-only decision trace is, by construction, *permanent
history*; ORCH-003 replay must work against 20-year-old records. Yet the only migration primitive is
"bump the version byte; decoders reject other versions as corruption" (L706–708) — which means a v2
decoder *rejects every v1 record as corrupt*. In 2045, "corruption" and "old format" are
indistinguishable, and the lossless-or-loud recovery path (kernel L482–571) will loudly refuse to
start on perfectly good historical truth.

**Why it matters.** Digital preservation over decades is a first-order requirement for an
infrastructure standard. Audit and regulatory retention (Vol 17: GDPR/SOC2/retention) frequently
mandate 7–30 year record availability. A format that can't be read forward is a compliance landmine.

**Risks / long-term consequences.** Either the format never evolves (ossification) or it evolves and
strands history (data loss). Both are unacceptable for the append-only truth of a cognitive OS.

**Alternative designs.** (1) Multi-version decoders required to read all prior frame versions
(forward-compatibility mandate). (2) Offline re-encoding migration tool that rewrites sealed segments
to the new version, preserving content addresses (requires Finding 1's stable address). (3) Version
negotiation + per-segment format tag.

**Recommendation (today).** (i) **IDR-008 "WAL Longevity & Migration"**: mandate that any conformant
decoder reads *all* prior `WAL_FRAME_VERSION`s (forward-compatible read), and define the sealed-
segment re-encode migration that preserves content addresses and offsets. (ii) **CCP Amendment**
adding a *conformance scenario* "Decade Replay" to the Recovery & Replay axis (Framework Axis 12): a
runtime must replay a fixture WAL written by an older frame version and reproduce an identical
`truth_hash`. This uses the existing amendment path (backward-compatible addition = MINOR, Lifecycle
Table 3) and the CCP-GATE requirement that behavior ships with a scenario.

**Implementation complexity.** High.
**Scientific impact.** Medium.
**Ecosystem impact.** High — long-term readability is a procurement requirement for governments and
enterprises, the exact adopters ISO/IEEE grade targets.

---

## Finding 5 — Autonomous-decision regulatory attestation: mechanism exists, portable profile does not (HIGH)

**Location.** Conformance Axis 11 (Autonomous Decision), Axis 7 (Safety-critical); ORCH-003
(decision trace); Vol 17 (GDPR/ISO27001/SOC2, "Track Who, What, When and Why"); Framework Part 9
(conformance artifact "is both the certificate and the regression record").

**What 2045 shows.** ARVES had the *right substrate* — every autonomous decision produces a replayable
decision trace with provenance, policy gates, and a Runtime Fingerprint. This is *exactly* what the
2028–2035 wave of AI-autonomy regulation demanded. But ARVES never packaged it as a **portable
regulatory attestation profile**, so every deployment re-argued compliance from scratch, no
regulator recognized the ARVES artifact as sufficient, and the artifact's *evidentiary* properties
(tamper-evidence, signing, chain-of-custody) were never specified. The mechanism was 90% of a
world-class compliance story; the missing 10% (a named, signed, regulator-facing profile) meant it
delivered a fraction of its potential authority.

**Why it matters.** The durable, defensible moat for a cognitive-infrastructure standard is being
*the* thing regulators point at. ARVES was one certification profile away from that and didn't ship
it.

**Risks / long-term consequences.** Competing, lighter-weight audit formats become the regulatory
lingua franca; ARVES's superior trace is relegated to an internal detail.

**Alternative designs.** (1) A "Regulated Autonomy" conformance *profile* (Framework Part 10 already
supports profiles) that adds signing + tamper-evidence + a fixed schema for the decision-trace
attestation. (2) Map the conformance artifact fields to specific regulatory clauses (a crosswalk
document, non-normative). (3) Third-party attestation service that countersigns ARVES artifacts.

**Recommendation (today).** **Certification** action: define a **"Regulated Autonomy" conformance
profile** on top of Axes 7 + 11 + 12 that (i) requires the decision-trace artifact to be signed and
tamper-evident (depends on Findings 1 & 11 for the crypto), (ii) freezes the artifact's regulatory
schema so it is *portable across vendors and borders*, (iii) ships a non-normative crosswalk to
GDPR/EU-AI-Act/ISO-42001 clauses. This is squarely inside the deferred Certification Program the
Freeze Record already authorizes launching in the Implementation Era — no spec edit.

**Implementation complexity.** High.
**Scientific impact.** Medium (mostly integration/standards-liaison work, but high strategic value).
**Ecosystem impact.** Very high — this is the "why enterprises and governments choose ARVES" story.

---

## Finding 6 — Self-planning agent-as-node has no first-class ABI representation (HIGH)

**Location.** Engine Graph Spec Parts 3, 5 (manifest fields; DAG of engine nodes); Vol 9 Part 3
("engines never own their own orchestration"), Part 6 ("dynamically expanded … engines may emit
sub-goals"), Part 7 (planning recursion); Baseline Part 4 (L3/L4 autonomous capabilities aspirational).

**What 2045 shows.** The dominant unit of cognition by ~2032 was an *autonomous agent* that receives
a goal and plans-and-executes its own sub-graph. The corpus anticipated this tension (Vol 9 Part 7,
"planning is both a node and the producer of the graph") and resolved it *conceptually* with bounded
dynamic expansion. But the *frozen* Engine Graph ABI (the manifest fields, Table 0) has **no
first-class field to declare a node as a bounded sub-planner** — its expansion budget, its
sub-goal-emission contract, its arbitration authority. So every runtime implemented "agent-as-node"
through the informal "engines may emit sub-goals" escape hatch, in incompatible ways, and multi-agent
graphs (Axis 9, Level L4) did not port across runtimes.

**Why it matters.** The single biggest external shift (AI going agentic) hit the one ABI surface that
lacked an explicit contract for it. The two-plane model *held*, but the ABI's expressiveness didn't
keep the resulting graphs interoperable.

**Risks / long-term consequences.** Multi-agent conformance (the L4 endgame, Baseline I5) fragments;
the most valuable/differentiated ARVES products (agent studios, autonomous orgs) become
runtime-specific.

**Alternative designs.** (1) IDR that *interprets* the existing manifest fields to encode a
sub-planner node (declare `Produces: uci.goal` sub-goals + an explicit expansion-budget field carried
in Planning Metadata) — additive, within frozen ABI semantics. (2) A new engine *determinism* class
"Bounded-Planner" recorded via IDR. (3) Defer to ARVES v2 (loses 15 years of interoperability).

**Recommendation (today).** **IDR-009 "Sub-Planner Node Convention"**: without adding ABI fields
(they're frozen), define the *normative convention* by which a node declares itself a bounded
sub-planner using existing fields — `Produces` a `uci.goal` set, `Determinism = Nondeterministic`
(so it replays from trace, ORCH-003), expansion bounds recorded in the decision trace's termination
policy (Vol 9 Part 6), arbitration handled by a Control-Plane join node (Vol 9 Part 6, never the
node itself). Add a conformance scenario under Axis 9 proving two runtimes expand the same
sub-planner node to the same *trace shape*. This records how to use the frozen ABI for agents; it
adds nothing to the ABI.

**Implementation complexity.** High.
**Scientific impact.** High — pins down the recursive-planning semantics that are the frontier of
cognitive architectures.
**Ecosystem impact.** High — protects the multi-agent product tier.

---

## Finding 7 — Cross-shard saga/compensation correctness has no conformance axis or replay proof (HIGH)

**Location.** IDR-001/IDR-004 ("no cross-shard atomic commit in v1 … sagas/compensation");
Amendment-006 (compensation model, saga-style, recorded in decision trace); Conformance Framework
Axes (12 axes — *none* is "cross-shard consistency / saga recovery"); kernel L160–164
(`UnknownShard` — "no cross-shard atomic commit; express as a saga").

**What 2045 shows.** Per-shard Raft scaled beautifully for *within*-shard truth. But real products
constantly need effects that span tenants/workspaces (an org-wide policy change, a cross-team
handoff, a marketplace transaction). ARVES pushed all of this onto *sagas with compensation*
(Amendment-006) — a correct choice — but then **never gave sagas a conformance axis, a replay
guarantee, or an invariant**. The result by 2045: cross-shard flows were the least-specified,
least-tested, most incident-prone surface in the ecosystem. Compensation logic is notoriously the
hardest code to get right, and ARVES certified runtimes could pass every scenario while shipping
subtly broken cross-shard compensation, because nothing tested it.

**Why it matters.** The corpus's most defensible distributed decision (per-shard CP, IDR-001) created
a matching liability (everything cross-shard is a hand-rolled saga) and left the liability
unconformance-tested. This is the classic "we specified the easy 80% and tested the easy 80%."

**Risks / long-term consequences.** Cross-shard data corruption / orphaned compensations in
production; loss of trust in the "consistency-first" promise precisely where it's hardest to keep.

**Alternative designs.** (1) Add a saga conformance axis + a proposed invariant (SAGA-001:
"a cross-shard saga either completes or is fully compensated; both paths are recorded in the decision
trace and are replayable") ratified through CCP with a scenario. (2) A reference saga
coordinator in the runtime with property tests (partial-failure injection). (3) Leave to products
(status quo → the 2045 mess).

**Recommendation (today).** (i) **Verification + IDR-010 "Saga Conformance"**: define the saga
execution model concretely as an IDR (coordinator placement, compensation ordering, idempotent
compensation via ORCH-004), and add property/failure-injection tests (crash between forward action
and compensation; compensation replay). (ii) **CCP-GATE** a proposed invariant SAGA-001 with a new
reference scenario "Cross-Tenant Handoff with Compensation" combining the existing Long-running +
Recovery axes. The proposed-invariant path (Invariant Registry Part 4, "must enter via a CCP
Amendment/IDR with a conformance scenario") is exactly the sanctioned mechanism.

**Implementation complexity.** High.
**Scientific impact.** Medium-high (distributed-saga correctness is a live research area).
**Ecosystem impact.** High — this is where multi-tenant products actually break.

---

## Finding 8 — `uci.*` ontology has no runtime schema-evolution / deprecation rules (MEDIUM)

**Location.** Ontology Part 9 (versioning: "backward-compatible = minor; breaking = major + new urn
version"), O-006 (every type versioned/registered); Lifecycle Table 3 (removal → deprecate one major
cycle → MAJOR).

**What 2045 shows.** The ontology got the *versioning grammar* right (`uci.fact@1`, minor/major).
What it lacks is *runtime schema-evolution semantics*: when a payload written as `uci.fact@1` is read
by a runtime that knows `uci.fact@3`, what happens? Are unknown fields preserved on rewrite? Is there
a canonical up-conversion? Over 20 years, `uci.*` types accreted dozens of minor versions, and
because the read/write compatibility rules were never made *executable*, different runtimes handled
old-versioned truth differently — the same append-only-migration problem as Finding 4, but at the
*type* layer instead of the *frame* layer.

**Why it matters.** An immutable, decades-long truth store *guarantees* it will hold every historical
type version. Reading old truth correctly is not optional; it's the definition of a durable ontology.

**Risks / long-term consequences.** Semantic drift — the same historical fact means slightly
different things to different runtime versions — which is corrosive to a *shared meaning system*
(the Ontology's entire purpose).

**Alternative designs.** (1) Mandate field-preserving, additive-only minor evolution + a canonical
up-converter per major transition (CCP Amendment + conformance scenario). (2) Store the exact
registry version with every payload (partially implied) and require readers to resolve through the
registered schema. (3) Freeze the type set entirely (kills evolution).

**Recommendation (today).** **CCP Amendment** (backward-compatible = MINOR) adding *normative
schema-evolution rules* to the type registry: minor versions are additive and field-preserving on
round-trip; a reader encountering an unknown minor must preserve unknown fields; each major bump
ships a canonical up-conversion function and a conformance scenario ("read `uci.fact@1`, project
through `@3`, round-trip preserves provenance and trust aspects"). This is exactly the amendment path
the Ontology's own governance clause invites.

**Implementation complexity.** Medium.
**Scientific impact.** Medium — schema evolution is well-trodden (Protobuf/Avro) but rarely made
*conformance-checked* in a cognitive ontology.
**Ecosystem impact.** Medium-high — protects semantic interoperability over decades.

---

## Finding 9 — Certification program deferred with no interim governance body → reference-runtime monoculture (MEDIUM)

**Location.** Freeze Record Table 0 ("Certification Program | Defined; program launch deferred to
Implementation Era"); Baseline Part 3 (Marketplace, Community/Ecosystem, Certification deferred to
v2); Lifecycle Part 11 (independent-implementability); CLAUDE.md long-term objectives 3–5 (Runtime A,
Runtime B, third-party certification).

**What 2045 shows.** Deferring the *program launch* was reasonable in 2026. But deferring it with **no
interim neutral governance body** meant that for the years between freeze and program launch, the
*only* arbiter of "what ARVES does" was the reference runtime's maintainer. Combined with Findings 1
& 3 (opaque encodings), this cemented the reference implementation as the de-facto standard *before*
any independent certification existed to counterbalance it. By the time the formal program launched,
the monoculture was entrenched. ISO/IEEE-grade standards succeed because a *neutral body* holds the
conformance suite and the change process from day one.

**Why it matters.** Governance neutrality is not a v2 nicety; it is the thing that makes a standard a
standard rather than a product spec. The Lifecycle Part 11 test ("an independent team … *without
consulting the original authors*") is unmeetable if the authors are the only certification authority.

**Risks / long-term consequences.** Standard capture; the "independent Runtime B" objective becomes
theoretically-open-but-practically-impossible; ISO/IEEE liaison stalls because there is no vendor-
neutral steward to liaise.

**Alternative designs.** (1) Stand up a minimal neutral *conformance-suite steward* now (owns the
suite + changelog + the CCP registry) even before full certification launch. (2) Publish the
conformance suite + fixtures publicly under an open governance charter immediately. (3) Keep deferred
(status quo → capture).

**Recommendation (today).** **Certification + Ecosystem** action: while respecting the deferred
*program launch*, establish now the *minimum viable neutral steward* the Lifecycle already
presupposes — a single, published owner for the conformance suite and CCP registry (Framework Part
11 and Lifecycle Part 6 both require "a single owner and changelog"; this action simply makes that
owner *neutral and public* rather than implicitly the reference-runtime team). Commit publicly to
certifying an independent Runtime B *before* v1 GA (already a stated long-term objective). No spec
edit; this operationalizes governance the corpus already assumes.

**Implementation complexity.** Medium (organizational more than technical).
**Scientific impact.** Low directly; high indirectly (neutral stewardship is the precondition for
ISO/IEEE submission).
**Ecosystem impact.** Critical for adoption; this is the difference between an open standard and a
single vendor's product.

---

## Finding 10 — The Evolution/learning loop can silently mutate behavior with no provenance/replay binding (MEDIUM)

**Location.** Vol 7 Evolution Core (Reflection → Learning → Behavior Improvement; Part 10 Behavior
Optimization; Part 16 Identity Continuity); Baseline Part 4 (L3 recursive self-improvement,
aspirational); ORCH-003 (replay from trace); Ontology aspects (Provenance, Trust).

**What 2045 shows.** ARVES's Evolution layer is designed to *continuously change behavior* from
experience (Vol 7 Parts 3, 10, 11 meta-learning). But the frozen corpus never binds *learned/adapted
state* to the same provenance + replay discipline as cognitive truth. Over 20 years this became the
quiet safety problem: a runtime's decisions in 2044 depended on 18 years of accumulated learned
preferences/calibrations whose lineage was not fully in the decision trace, so a 2044 decision was
**not replayable from first principles** — replaying the trace reproduced the *recorded outcome*
(ORCH-003 is satisfied narrowly) but could not reconstruct *why the learned state was what it was*.
For a regulated autonomous system, "we can replay the decision but not audit the learning that shaped
it" is a serious accountability gap.

**Why it matters.** The corpus's Identity-Continuity goal (Vol 7 Part 16) and its replay guarantee
(ORCH-003) are in latent tension with continuous behavior mutation. The tension was never resolved by
a normative rule about how learned state enters the Runtime Fingerprint.

**Risks / long-term consequences.** Unauditable behavioral drift; "the system learned to do X and no
one can reconstruct when or from what evidence." This is the AI-safety failure mode that regulation
targets hardest.

**Alternative designs.** (1) IDR requiring learned/adapted state to be treated as *versioned,
provenance-carrying truth committed through the Kernel* and captured in the Runtime Fingerprint (Vol
9 Part 9) — so replay includes the learned-state version. (2) A separate, append-only "learning
trace" cross-referenced from the decision trace. (3) Forbid online learning in certified profiles
(safe but defeats Vol 7).

**Recommendation (today).** **IDR-011 "Learned-State Provenance & Fingerprinting"**: record the
normative rule that any learned/adapted state influencing a decision is (i) committed through the
Kernel as versioned truth with full provenance/trust aspects (Ontology Part 4), and (ii) pinned in
the Runtime Fingerprint so ORCH-003 replay is *reproducible against the exact learned-state version*.
This upholds ORCH-001/003 and Vol 7 without changing them — it decides *how* the reference runtime
makes Vol 7's learning replayable. Pair with a Verification scenario under Axis 11+12: "replay a
decision that depended on learned state; the trace pins the learned-state version and reproduces it."

**Implementation complexity.** High.
**Scientific impact.** High — reproducible/auditable continual learning is an open frontier.
**Ecosystem impact.** Medium-high — essential for regulated autonomy (ties to Finding 5).

---

## Finding 11 — No crypto-agility / post-quantum envelope for identity, audit signatures, attestation (MEDIUM)

**Location.** Vol 2 Tenant & Identity Constitution; Vol 17 (Identity Security, Audit "Who/What/
When/Why"); Freeze Record (no crypto primitives named anywhere); Finding 1 (content-addressing);
Finding 5 (signed attestation).

**What 2045 shows.** ARVES specifies *that* there is authentication, authorization, audit, and trust
(Vol 17) but never freezes *which cryptographic primitives* underpin tenant identity, audit-trail
signatures, or attestation. Between 2030 and 2040 the industry migrated to post-quantum signatures.
Because ARVES had no *crypto-agility envelope* (an algorithm-tagged, versioned, negotiable primitive
set), every deployment hard-coded 2026-era crypto and the PQ migration was a per-vendor scramble with
no portable path — and the decades of *signed audit trails* (the evidentiary backbone of Finding 5)
were signed with algorithms that were, by 2042, no longer legally sufficient.

**Why it matters.** A cognitive-infrastructure standard whose audit trail is its trust anchor cannot
outlive its cryptography. 20 years is longer than any single signature algorithm's safe lifetime.

**Risks / long-term consequences.** Retroactive invalidation of historical audit trails; inability to
prove the integrity of decades-old decisions; PQ-migration chaos.

**Alternative designs.** (1) Define a crypto-agility envelope (algorithm-id + params + signature)
with a mandatory baseline and reserved successor tags — same pattern as Finding 1's digest agility.
(2) Freeze one algorithm (repeats the problem). (3) Leave to products (2045 scramble).

**Recommendation (today).** **IDR-012 + CCP Amendment "Crypto Agility Envelope"**: as an
implementation decision, define the algorithm-tagged, versioned envelope for signatures/identity/
attestation with a frozen baseline and reserved tags for PQ successors; as a MINOR amendment, add a
conformance property "signatures are algorithm-tagged and a runtime accepts the mandatory baseline +
negotiates successors." Co-design with Finding 1's digest agility (shared tagging scheme). No UCS
spec text changes — Vol 17 says *that* there is signing; the IDR says *how*, agilely.

**Implementation complexity.** High.
**Scientific impact.** Medium (applied crypto-agility, but critical correctness).
**Ecosystem impact.** High for regulated/government adoption; low for hobbyists.

---

## Finding 12 — Two-track UCS/UCI versioning lacks a compatibility matrix and a support/sunset window (LOW)

**Location.** Lifecycle Part 8 (two-track UCS/UCI, "a UCI runtime declares which UCS version it
implements"); Table 1 (status lifecycle incl. Deprecated "still valid for one major cycle"); Part 7
(suites pinned to spec version).

**What 2045 shows.** The two-track model is elegant and correct in principle — but by 2045 there were
several UCS versions and many more UCI versions, and the corpus never defined a **compatibility
matrix** (which UCI versions implement which UCS versions, and which combinations are certified) or a
**support/sunset window** (how long a UCS version is maintained). "Deprecated = valid for one major
cycle" (Table 1) is the only durability commitment, and it's about *types*, not *standard versions*.
Operators planning 15-year deployments had no contractual basis for how long their targeted UCS
version would be supported.

**Why it matters.** Long-lived infrastructure procurement (the ISO/IEEE-grade adopter) requires a
published support lifecycle. Its absence is a minor technical gap but a real *adoption* barrier.

**Risks / long-term consequences.** Version sprawl; uncertainty that deters long-horizon adopters;
ambiguous certification claims ("certified against ARVES" — which version, still supported?).

**Alternative designs.** (1) Publish a UCS↔UCI compatibility matrix and a support-window policy
(e.g. each UCS major supported N years / M majors) as governance artifacts. (2) Encode it in the
conformance report format (already "N% at Level Lx against Spec vB / Suite vA" — add support-status).
(3) Leave implicit.

**Recommendation (today).** **CCP Amendment (PATCH/MINOR governance clarification)**: publish a
UCS/UCI compatibility matrix and a defined support/sunset window as versioned governance artifacts,
and extend the conformance-report string to include the support status of the targeted spec version.
This clarifies process without changing any normative behavior (Lifecycle Table 3: clarification =
PATCH).

**Implementation complexity.** Low.
**Scientific impact.** Low.
**Ecosystem impact.** Medium — small effort, real de-risking for long-horizon adopters.

---

## What survived (the bets that were right, to be defended, not changed)

- **The ownership algebra** (OWN-001, ORCH-001/002, LAYER-001, SHARD-001) — the reason ARVES is
  certifiable at all. Do not dilute it; Findings 1–3 and 7 exist to *protect* it, never to relax it.
- **Replay-from-trace over recomputation** (ORCH-003) — the correct response to nondeterministic
  engines; it aged perfectly and is *more* right in 2045 than in 2026. Finding 10 only asks to
  extend its reach to learned state.
- **The Engine Graph ABI's refusal to name the model** (Engine Graph Part 4/9; Model Strategy is
  "model-agnostic routing", Blueprint L47–48) — the single most future-proof decision in the corpus.
  It absorbed 20 years of AI hardware/model revolution without a spec break.
- **Two-plane Control/Data split** (Vol 9) — held under agentic AI; Finding 6 only asks to make the
  *convention* for agent-nodes explicit within the frozen ABI.
- **Conformance-as-fitness-function and scenario-not-golden-output** (Framework Part 8) — the right
  epistemology for a nondeterministic system; Finding 2 asks to *add* model-checking above it, not
  replace it.

---

## If ARVES were standardized by ISO/IEEE tomorrow, what would still be missing?

A submission-grade package would still lack, in priority order: **(1) a frozen, agile content-address
and cryptographic primitive set** (Findings 1, 11) — ISO will not ratify identity/integrity that is
"implementation-defined"; **(2) a machine-checked formal model of the invariant core** (Finding 2) —
high-assurance domains require verification, not just testing; **(3) a canonical, implementation-
independent wire form with a demonstrated second interoperable runtime** (Findings 3, 9) — a standard
with one implementation is a product spec; **(4) a neutral governance steward holding the suite and
change process** (Finding 9) — ISO/IEEE require vendor-neutral stewardship; **(5) long-term data
readability, schema-evolution, and cross-shard/saga correctness guarantees** (Findings 4, 7, 8) — a
20-year infrastructure standard must prove it can read and reason over its own decades-old truth. The
architecture is world-class; the *encodings, proofs, and governance* around it are what remain
between ARVES and an ISO/IEEE ratification.

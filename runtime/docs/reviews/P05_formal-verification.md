# P05 — Formal Verification Lens (Wave-1 Gate Review)

**Reviewer role:** Independent Chief-Architect-level reviewer · Formal Verification lens
**Objective:** Determine whether ARVES's core guarantees are *formally statable* and *mechanically verifiable*, and whether that verifiability is a credible path to an ISO/IEEE-grade, independently-implementable, 20-year standard.
**Scope:** Enumerate every property that could be mathematically/mechanically verified (safety, liveness, determinism, replay-equivalence, single-writer ownership, tenant/shard isolation, consistency, idempotency, and others). For each: precise statement, proof technique, verification method, concrete runtime assertion. Flag every invariant the corpus references but never defines.
**Inputs read:** Invariant Registry v1; IDR Batch 1; Amendments CCP Batch 1; Vol 9 Cognitive Control Plane v2; Engine Graph Spec v1; Scenario Conformance Framework v1; Reference Lifecycle v1; Universal Cognitive Ontology Spec v1. Reference runtime: `arves-invariants`, `arves-consensus`, `arves-kernel`, `arves-persistence`, `arves-conformance` (crate sources + kernel/persistence tests).
**Constraint honoured:** No finding proposes modifying the frozen spec. Every recommendation is an IDR, CCP Amendment, Runtime, Verification, or Certification deliverable.

---

## Executive Summary

**Gate verdict: CONDITIONAL (lean pass).**

ARVES clears this gate — but conditionally, and the condition is the whole point of the lens.

The good news, and it is genuinely strong: ARVES's core guarantees are *unusually* amenable to formal statement. The corpus was authored with a formalization discipline (Reference Lifecycle Part 4 explicitly says "unfalsifiable claims do not advance"), and the seven registered invariants (`OWN-001`, `LAYER-001`, `SHARD-001`, `ORCH-001..004`) are each expressible as a precise safety property over a well-defined state space. IDR-001..005 commit the truth path to per-shard Raft with an append-only log that is simultaneously WAL and decision trace — a decision that maps cleanly onto the most heavily model-checked class of distributed algorithms in the literature (TLA+ Raft, Verdi Raft, Jepsen). The reference kernel already realizes a deterministic replay path with a `truth_hash()` designed precisely for replay-equivalence testing, and its recovery tests are genuine fault-injection proofs ("lossless or loud"). This is a codebase and a corpus that *want* to be verified.

The condition — and the reason this is not an unconditional pass — is a triad of gaps that, left unaddressed, would make the verifiability aspirational rather than real over 20 years:

1. **The corpus states properties in prose, never in a formal notation.** There is no temporal-logic specification, no state-machine definition, no distinction (in machine-checkable form) between the *safety* and *liveness* halves of the same invariant. `ORCH-003` ("every execution is replayable") is a liveness-flavoured English sentence that actually encodes at least three distinct formal obligations (determinism-of-replay, replay-equivalence, and trace-completeness) which must be verified by *different* techniques. Until each invariant is decomposed into formal safety/liveness obligations, "verifiable" is a claim, not a fact.

2. **Every invariant's proof status is `pending`, and the only verification tooling present is hand-written unit tests.** There is *no* property-based testing (no `proptest`/`quickcheck`), *no* model checking (no TLA+/Stateright), *no* bounded model checking of the Rust (no Kani), *no* refinement proof (no Stainless/Creusot). The consensus crate — where the hardest safety properties live — is a pure skeleton (trait signatures, two trivial constructor tests). The conformance framework defines a beautiful invariant-keyed verdict machinery but has *zero populated assertions*. The corpus's own falsifiability rule is therefore not yet met for a single invariant.

3. **The corpus references invariants it never defines, and defines "idempotent/content-addressable" without pinning the algebra.** Eighteen proposed invariants (`G-001`, `QUERY-001`, `LCW-001`, `PERSIST-001`, `CAP-001..009`, `ENG-001..005`) are cited throughout the normative documents (IDR-001 cites `G-001` and `QUERY-001`; the Engine Graph Spec leans on `ENG-*`/`CAP-*` semantics) yet are *informative only* per the Registry. You cannot verify what you have not defined. Separately, `ORCH-004` ("idempotent and content-addressable") is stated without a canonicalization rule or a hash function — and in the reference kernel the idempotency contract surfaces as a `CommitError::AlreadyCommitted` rather than an `Ok`, a semantic subtlety that two independent implementations will formalize differently.

None of the three requires touching the frozen text. All three are closeable via the additive instruments the Reference Lifecycle already sanctions: a **Formal Specification Companion** (CCP Amendment, new frozen normative document carrying TLA+/state-machine definitions), a milestone-aligned **Verification programme** (Runtime + Verification deliverables), and CCP ratification of the proposed invariants each *with a formal property, not just prose*.

The single most consequential thing this lens found: **ARVES has already made the two architectural decisions that make formal verification tractable — deterministic replay-from-trace instead of recomputation, and per-shard Raft instead of a bespoke consensus protocol.** Most cognitive/agent systems cannot be formally verified because they are non-deterministic all the way down. ARVES quarantined the non-determinism (engines) behind a deterministic, replicated, content-addressed truth path. That is the difference between a system that *could* carry an ISO-grade formal-verification annex and one that never could. The gate should clear so the deeper reviews run — but the standard is not "standard-ready" until the formal statements exist and at least the registered invariants have executable proofs.

**ISO/IEEE gap (single biggest):** There is no formal (machine-readable, notation-based) specification of any ARVES property. ISO/IEC/IEEE standards that make verifiability claims (e.g. the formal annexes in avionics DO-178C/DO-333, or the state-machine specs in IEEE 1588) require the property to exist in a formal notation with a stated proof obligation. ARVES has precise *prose* and a precise *implementation*, but nothing in between — no TLA+ module, no state-transition system, no temporal-logic property set — so a certifier cannot independently reproduce the verification argument.

---

## Severity-Ranked Findings Table

| # | Finding | Severity | Proposal type | Impl. complexity |
|---|---------|----------|---------------|------------------|
| P05-1 | No formal specification exists for any property; invariants live only as English prose (no TLA+/state-machine/temporal-logic form), so safety vs liveness obligations are never separated | Critical | CCP-Amendment | very-high |
| P05-2 | `ORCH-003` (replayability) conflates three distinct formal obligations — replay-determinism, replay-equivalence, and trace-completeness — that require different proof techniques and are never individually stated | Critical | Verification | high |
| P05-3 | 18 invariants (`G-001`, `QUERY-001`, `LCW-001`, `PERSIST-001`, `CAP-001..009`, `ENG-001..005`) are cited by normative documents but never defined; some are load-bearing for IDR-001 and the Engine ABI yet remain informative | High | CCP-Amendment | medium |
| P05-4 | `ORCH-004` idempotency/content-addressing is unverifiable as written: no canonicalization rule, no hash function, no idempotency algebra; reference kernel surfaces the property as `Err(AlreadyCommitted)`, not `Ok`, so the formal shape differs across implementations | High | CCP-Amendment | medium |
| P05-5 | Zero verification tooling in the reference implementation: no property-based tests, no model checking, no bounded model checking, no refinement proof; consensus (hardest safety) is a skeleton and conformance has zero populated assertions | High | Verification | high |
| P05-6 | No liveness properties are stated anywhere (leader election terminates, commit eventually completes, graph expansion terminates); safety-only verification hides deadlock/livelock/starvation classes | High | Verification | high |
| P05-7 | Tenant/shard isolation (`SHARD-001`) has no non-interference formalization; the property is stated as "no cross-tenant data in a shard" but the stronger, verifiable claim (observational non-interference across tenants) is never made | Medium | Verification | medium |
| P05-8 | `LAYER-001` and `OWN-001` are structural invariants provable statically (dependency-graph acyclicity, single-writer typing) but the corpus offers no proof obligation, and the runtime encodes them only as runtime `may_depend_on` helpers, not compile-time/architecture-test guarantees | Medium | Verification | low |
| P05-9 | Cross-shard sagas (Amendment-006) have no formal correctness criterion (atomicity-modulo-compensation / eventual saga-consistency); the one place ARVES abandons single-shard atomicity is the one place with no property to verify | Medium | IDR | high |
| P05-10 | Bounded termination of dynamic Engine-Graph expansion (Vol 9 Part 6-7) is asserted but never formalized as a ranking-function / well-founded-order termination proof obligation | Low | Verification | medium |

---

## P05-1 — No formal specification exists for any ARVES property (Critical · CCP-Amendment · very-high)

**Precise statement of the gap.** Every ARVES guarantee is expressed as a one-line English sentence in a table cell (Invariant Registry Table 0; Vol 9 Table 1; Amendments Table 2). None is expressed in a formal notation from which a proof obligation can be mechanically derived. There is no state-transition system defining the ARVES machine, no temporal-logic (LTL/CTL/TLA+) rendering of any invariant, and — critically — no separation of each invariant into its **safety** part (nothing bad happens) and its **liveness** part (something good eventually happens). For example, `ORCH-001` ("the Control Plane owns no truth") is a safety invariant (`□ ¬(∃ s ∈ State : owner(s) = ControlPlane)`) but is written as a design maxim; `ORCH-004` ("idempotent and content-addressable") is really two properties (a functional-equation safety property plus an addressing bijection) fused into a phrase.

**Why it matters.** ISO/IEC/IEEE standards that claim verifiability (DO-333 formal-methods supplement to DO-178C; the formal state machines in IEEE 1588 PTP; the Z/CSP annexes historically used in defence standards) carry the property in a formal notation *inside or alongside* the normative text, precisely so that a certifier can reproduce the verification argument without consulting the authors — which is exactly the Reference Lifecycle Part 11 "independent-implementability test" applied to *proofs*, not just implementations. Prose invariants cannot be model-checked, cannot be refined, and cannot anchor a conformance assertion that means the same thing to two teams. The corpus's own Reference Lifecycle Part 4 ("Formalization turns Theory into invariants and machine-checkable properties … not prose") sets exactly this bar and does not meet it: it *names* invariants but never renders them machine-checkable.

**Risks.** (i) Divergent formalization — two certified runtimes each "prove" `ORCH-003` against incompatible informal readings, and the conformance suite cannot detect the divergence because it too is keyed to prose. (ii) Silent weakening — over successive major versions, an invariant's meaning drifts because there is no canonical formal object to diff against. (iii) Safety blindness — because safety and liveness are never separated, the safety-critical axis (Scenario Conformance Framework Axis 7) is "verified" by tests that only ever exercise the safety half, leaving liveness failures (a safety gate that never fires because the graph deadlocks) invisible.

**Long-term consequences.** A 20-year standard whose properties exist only as prose fractures into implementation dialects (this is the classic fate of informally specified standards; cf. the pre-TLA+ history of consensus protocols, where "Paxos" meant a dozen incompatible things). Conversely, a standard that ships a formal companion becomes *citable by construction*: downstream safety cases (embodied/robotics per Vol 8, regulated enterprise per Vol 17) can reference the formal property and its proof.

**Alternative designs.**
- *(A) TLA+ specification companion (recommended primary).* One frozen normative companion — call it the **ARVES Formal Specification (AFS) v1** — that defines the ARVES abstract state machine (state = per-shard committed truth sets + in-flight proposals + control-plane plan artifacts; actions = Propose, Commit, Apply, Replay, Elect, ChangeMembership, ExpandGraph) and renders each registered invariant as a TLA+ `THEOREM` (safety as an invariant, liveness under `WF`/`SF` fairness). TLA+ is the strongest fit because IDR-001's per-shard Raft is the single most model-checked protocol in TLA+ (Lamport's own Raft/Paxos work; the Stateright ecosystem), so the consensus core comes nearly for free.
- *(B) Coq/Isabelle mechanized proof.* Maximum assurance, matches the Verdi Raft precedent, but very-high cost and over-engineered for a standard whose engines are irreducibly non-deterministic — full functional correctness of the cognitive layer is not even meaningful.
- *(C) Alloy structural model.* Excellent and cheap for the *structural* invariants (`LAYER-001` acyclicity, `OWN-001` single-writer, `SHARD-001` partition-disjointness) but weak for temporal/liveness properties. Best as a complement, not the backbone.
- *(D) Do nothing / keep prose.* Rejected: violates the corpus's own Part 4 formalization rule and forfeits the gate.

**Recommendation.** CCP Amendment (MINOR, additive — does **not** touch the frozen technical corpus): ratify a frozen **ARVES Formal Specification companion** rendering the seven registered invariants in TLA+ (state machine + safety invariants + liveness theorems under stated fairness), with Alloy for the three structural invariants as a fast pre-check. Pair it with a Verification deliverable (P05-5) that runs TLC/Apalache in CI. Sequence it *first* among the Wave-2 formal deliverables because every other finding here references a formal object this creates.

**Implementation complexity:** very-high (new formal artifact, new skill on the team, CI integration).
**Scientific impact:** High — a publicly model-checked cognitive-infrastructure standard would be a first; the TLA+ model of "deterministic replay over per-shard Raft with non-deterministic compute quarantined" is itself a contribution.
**Ecosystem impact:** High — the formal companion is the anchor every certifier, regulator, and independent implementer cites; without it, "verifiable" is marketing.

---

## P05-2 — `ORCH-003` conflates three distinct verifiable obligations (Critical · Verification · high)

**Precise statement.** `ORCH-003` reads: "Every execution is replayable from the same Goal, State, Policies, Capabilities and Runtime Fingerprint — via a recorded decision trace, not by recomputation." This single sentence bundles at least three mathematically distinct properties, each requiring a different proof technique:

1. **Replay-determinism (safety, functional).** Given the recorded trace `T` and the same Runtime Fingerprint `F`, the replay function is a pure deterministic fold: `replay(T, F) = replay(T, F)` on every invocation. Verifiable by: property-based test that replays the same trace N times and asserts bit-identical resulting state; formally, referential transparency of the apply/fold function.
2. **Replay-equivalence (safety, refinement).** The state reconstructed by replay equals the state that existed at record time: `replay(record(run)) ≈ state_at(run)` under a defined equivalence `≈`. This is a *refinement/simulation* property, not just determinism. The reference kernel already provides the exact instrument to check it — `truth_hash()` "Equal before and after replay iff identical" (`arves-kernel/src/lib.rs:638`) — and the recovery tests assert `recovered.truth_hash() == expected` (`arves-kernel/tests/recovery.rs:150`). But the corpus never *states* this as a property with a defined `≈`.
3. **Trace-completeness (safety, well-formedness).** The trace records *enough* to reconstruct: every committed outcome, arbitration choice, policy evaluation, and the Runtime Fingerprint appear in the trace (Vol 9 Part 9 enumerates them informally). Verifiable by: a schema-completeness check plus a "no read outside the trace during replay" assertion.

None of these three is separately stated, so "we verified ORCH-003" is ambiguous: the reference kernel verifies (2) via `truth_hash` equality but has no explicit check for (1) idempotent re-replay of the *same* running kernel, and (3) is untested because the trace schema is not fixed (see P05-4).

**Why it matters.** Replayability is ARVES's headline scientific claim — it is how a *non-deterministic* cognitive system earns reproducibility (Scenario Conformance Framework Part 8: "a run does not assert a single correct answer; it asserts that invariants and properties held"). If the three obligations are not separated, an implementation can pass a replay test while silently violating trace-completeness (e.g., reading a wall clock during replay), which would make its "replay" a recomputation in disguise — the exact thing `ORCH-003` forbids ("not by recomputation").

**Risks.** (i) A runtime passes conformance by re-deriving decisions that happen to match, then diverges under a model change. (ii) Cross-runtime replay portability (a Runtime-A trace replayed on Runtime-B) is never verified because the obligation isn't isolated. (iii) The `not by recomputation` clause — the scientifically novel part — is unfalsifiable without a trace-completeness property.

**Long-term consequences.** Replay is the foundation for audit (Vol 17 governance), debugging, and certification evidence. If it silently degrades to recomputation, every downstream audit claim collapses. Over 20 years and multiple engine-model generations, only trace-completeness guarantees that a 2026 run replays on a 2040 runtime.

**Alternative designs.**
- *(A) Decompose ORCH-003 into ORCH-003a/b/c as formal sub-properties (recommended)*, each with its own proof technique: 003a replay-determinism → property-based (`proptest`: replay any generated trace twice, assert equal); 003b replay-equivalence → the existing `truth_hash` refinement check, generalized to a `proptest` over random commit histories; 003c trace-completeness → a "sealed replay" harness that runs replay with all external inputs (clock, RNG, network) trapped to panic, proving replay reads *only* the trace.
- *(B) Model-check replay in TLA+* as a refinement mapping (the WAL fold refines the abstract truth machine). Strong, complements (A), belongs in the P05-1 companion.
- *(C) Golden-trace corpus* — a set of frozen (trace, expected-truth-hash) pairs that every certified runtime must reproduce. This makes replay-equivalence a *cross-runtime* conformance artifact, not just an intra-runtime test. Recommended as the Certification instrument.

**Recommendation.** Verification deliverable: build the three-part replay proof (property-based for 003a/b, sealed-replay harness for 003c) against `arves-kernel`, generalizing the existing `truth_hash` equality from fixed test vectors to `proptest`-generated histories. Add a Certification deliverable: a frozen golden-trace corpus for cross-runtime replay-equivalence. The corpus need not restate ORCH-003 — the decomposition lives in the AFS companion (P05-1) and the conformance suite.

**Implementation complexity:** high (the sealed-replay harness — trapping clock/RNG/IO during replay — is the hard part and is exactly what proves "not by recomputation").
**Scientific impact:** Very high — a formal separation of "deterministic replay of a non-deterministic system" into determinism/equivalence/completeness is a genuine methodological contribution and the intellectual core of ARVES.
**Ecosystem impact:** High — cross-runtime replay portability is the acid test of ARVES being a *standard* rather than one team's runtime.

---

## P05-3 — Load-bearing invariants are cited but never defined (High · CCP-Amendment · medium)

**Precise statement.** The Invariant Registry (Part 4) records 18 **proposed** invariants that are "grounded in the frozen corpus … but never formally ratified" and carry *no conformance weight*: `G-001`, `QUERY-001`, `LCW-001`, `PERSIST-001`, `CAP-001..009`, `ENG-001..005`. Yet several are *load-bearing for normative documents*:

- **`G-001`** ("Kernel is the single global source of committed truth and the sole commit gateway") is cited *by name* in IDR-001's Context and in the reference kernel's module doc (`arves-kernel/src/lib.rs:26`) as the very justification for the single write path — but it is only proposed.
- **`QUERY-001`** ("Query is strictly read-only") is cited in IDR-001's "Spec invariants upheld" list and shapes the Kernel's deliberate absence of read methods (`arves-kernel/src/lib.rs:31`) — but it is only proposed.
- **`ENG-003`/`ENG-004`/`CAP-003`** carry the idempotency/replay semantics that the Engine Graph Specification's Runtime Contract (Part 10) *requires a conformant runtime to enforce* — but they are only proposed.

So the frozen normative documents lean on invariants the Registry says cannot yet be enforced. This is not a contradiction the corpus is unaware of (CLAUDE.md and the Registry both flag it), but from a verification standpoint it is a hard blocker: **you cannot write a proof obligation for a property that has no ratified statement.**

**Why it matters.** The verification programme cannot proceed past the seven registered invariants without these. `G-001` in particular is the *single-writer* property that makes the whole CP-truth argument sound; verifying "the Kernel is the sole commit gateway" is arguably the most important safety property in the system, and it is formally undefined. The Reference Lifecycle CCP-GATE ("no behaviour is ratified without a conformance scenario") means each of these needs both a formal statement *and* a scenario before it counts.

**Risks.** (i) Verification stalls at 7 invariants while the architecture depends on 25. (ii) An implementer reads IDR-001, sees `G-001` cited as ground truth, and treats it as normative — exactly the "informative treated as registered" error the Registry warns against. (iii) When these are finally ratified, their formal statements may not match the informal usage already baked into the reference runtime, forcing rework.

**Long-term consequences.** The gap between "invariants the architecture assumes" (25) and "invariants that are verifiable" (7) is the true measure of how far ARVES is from its own falsifiability standard. Closing it is prerequisite to any credible ISO submission.

**Alternative designs.**
- *(A) Ratify the proposed invariants in priority batches via CCP, each with a formal property + conformance scenario (recommended).* Batch 1 (highest leverage, blocks everything): `G-001`, `QUERY-001`, `LCW-001`, `PERSIST-001` — the four Layer-Matrix single-writer/read-only invariants. Batch 2: `ENG-001..005` (engine purity/determinism/manifest). Batch 3: `CAP-001..009` (capability binding/idempotency/cancellation). Each ratification adds a formal statement to the AFS companion (P05-1) and a scenario to the conformance suite (CCP-GATE satisfied by construction).
- *(B) Leave them proposed and verify only the 7 registered.* Rejected: leaves the single-writer property (`G-001`) formally undefined, which guts the safety story.
- *(C) Fold the four Layer-Matrix invariants into refinements of the already-registered `OWN-001`/`ORCH-001`* (since `G-001` ⊂ `OWN-001` applied to truth; `QUERY-001` ⊂ `ORCH-001`/layer matrix). Elegant and reduces the count, but still requires a CCP to make the derivation normative.

**Recommendation.** CCP Amendment batch: ratify Batch 1 (`G-001`, `QUERY-001`, `LCW-001`, `PERSIST-001`) first, each accompanied by (a) a formal property in the AFS companion and (b) a conformance scenario satisfying CCP-GATE. Defer `ENG-*`/`CAP-*` to their owning milestones (I4/I5) so ratification tracks implementation. Explicitly reconcile each ratified statement against its already-baked usage in the reference kernel to avoid retro-drift.

**Implementation complexity:** medium (process-heavy — CCP + scenario per invariant — but each statement is short).
**Scientific impact:** Medium — mostly consolidation, though ratifying `G-001` as a formal single-writer theorem is significant.
**Ecosystem impact:** High — removes the "informative invariants cited as normative" trap that would confuse every independent implementer.

---

## P05-4 — `ORCH-004` idempotency/content-addressing is unverifiable as written (High · CCP-Amendment · medium)

**Precise statement.** `ORCH-004` ("every engine and capability invocation is idempotent and content-addressable") is the bridge to safe retry, replay, and distribution. But as written it is not verifiable, for two reasons:

1. **No canonicalization or hash function is fixed.** "Content-addressable" requires a total function from invocation content to an address such that identical content ⇒ identical address (across runtimes and time). The corpus never specifies canonical serialization (byte order, field ordering, floating-point normalization, unicode normalization) or a digest algorithm. The reference kernel uses opaque `ContentHash(Vec<u8>)` supplied by the caller (`arves-kernel/src/lib.rs`), the consensus crate uses `ContentHash(String)` "concrete hash function is deferred" (`arves-consensus/src/lib.rs:141`), and persistence uses CRC-32/FNV internally — three different notions, none normative. Two independent runtimes will produce *different* content addresses for the *same* logical invocation, so cross-runtime idempotency/dedup is unverifiable and, worse, silently broken.
2. **The idempotency algebra is not stated, and the reference implementation's shape diverges from the natural formal reading.** The natural formal statement is: `commit(x) ; commit(x) ≡ commit(x)` with the *same observable result*. In the reference kernel, the first `commit` returns `Ok(TruthRef)` and the second returns `Err(CommitError::AlreadyCommitted(same_ref))` (`arves-kernel/src/lib.rs:661`). This *is* idempotent (no fork; the same `TruthRef` is returned) and is a defensible design — but the property's *type-level shape* (Ok vs Err) is an implementation choice, not a spec-mandated one. A second team could equally return `Ok(existing_ref)` on the duplicate. Both are "idempotent," but a conformance assertion that checks the return *variant* would reject one; an assertion that checks only the resolved `TruthRef` accepts both. The corpus does not say which, so the property is ambiguous.

**Why it matters.** Idempotency + content-addressing is what makes at-least-once delivery, retry, and replay safe by construction (IDR-002 replicates outcomes precisely because they are content-addressed). If the address is runtime-specific, then: dedup across runtimes fails; a Runtime-A trace cannot be idempotency-checked on Runtime-B; and the Raft-log-as-decision-trace identity (IDR-003) is not portable. This is the difference between ARVES being a *standard* and being a family of incompatible runtimes that each happen to be internally idempotent.

**Risks.** (i) Cross-runtime replay (P05-2) silently duplicates or forks truth because addresses differ. (ii) A conformance assertion written against the reference kernel's `Err(AlreadyCommitted)` shape falsely fails a legitimate `Ok(existing)` implementation. (iii) Floating-point / map-ordering non-canonicalization makes "identical content" non-deterministic even within one runtime across versions.

**Long-term consequences.** Content-addressing is the substrate for the future marketplace (Vol 15/16) and multi-agent runtime (I5) — agents exchange content-addressed artifacts. A non-canonical address scheme poisons the entire ecosystem's interop 20 years out.

**Alternative designs.**
- *(A) Frozen canonicalization + digest amendment (recommended).* CCP Amendment adding a normative rule: canonical form = RFC 8785 (JSON Canonicalization Scheme) or a defined CBOR/deterministic-encoding profile; digest = SHA-256; the Idempotency Key = digest of the canonical invocation tuple `(engine_urn@ver, canonical(inputs), reads-snapshot-ref, capability-bindings, fingerprint)`. This makes the address a *total, portable, verifiable* function. (This dovetails with the manifest-schema gaps other lenses flag — do it once, in an Interchange Formats companion.)
- *(B) Define the idempotency algebra formally and leave the return-shape free.* State the property observationally: "for any two commits with equal Idempotency Key, both resolve to the same `TruthRef` and at most one entry is appended to the log," and require the conformance assertion to check the *resolved ref and log-append-count*, not the `Result` variant. This resolves the Ok/Err ambiguity without constraining implementations.
- *(C) Leave hashing to implementers (status quo).* Rejected: forfeits cross-runtime interop, the entire point of a standard.

**Recommendation.** CCP Amendment (fold into a shared *Interchange Formats* companion): (i) fix canonicalization (RFC 8785 or deterministic CBOR) + SHA-256 digest + the Idempotency-Key tuple; (ii) state `ORCH-004` observationally (same key ⇒ same `TruthRef` ⇒ ≤1 log append) so implementations may return either `Ok(existing)` or a typed "already committed" without failing conformance. Add a Verification deliverable: a cross-runtime golden-digest corpus (byte-identical canonical forms + digests) and a `proptest` that commits duplicates in random interleavings and asserts single-append + same-ref.

**Implementation complexity:** medium (canonicalization + digest is well-trodden; the algebra statement is short).
**Scientific impact:** Medium — content-addressed cognition is a nice property but the techniques are standard.
**Ecosystem impact:** Very high — this is a hard interop gate for marketplace/multi-agent; get it wrong and the ecosystem never coheres.

---

## P05-5 — Zero verification tooling in the reference implementation (High · Verification · high)

**Precise statement.** A scan of every crate's `Cargo.toml` and sources finds **no** verification tooling of any kind: no `proptest`/`quickcheck` (property-based testing), no `stateright`/TLA+ harness (model checking), no `kani` (bounded model checking of Rust), no `loom` (concurrency-interleaving testing), no `creusot`/`prusti`/Stainless (deductive verification). The only tests are hand-written `#[test]` unit/integration cases. Concretely:
- `arves-invariants` is a pure skeleton: it defines invariant *identifiers* and a `PropertyCheck` *trait signature*, with method bodies "intentionally omitted" (`arves-invariants/src/lib.rs:432`). No check is implemented.
- `arves-consensus` — where the hardest safety properties (single-leader, log-matching, state-machine-safety) live — is a skeleton with two trivial constructor tests (`arves-consensus/src/lib.rs:435`). The consensus algorithm itself is unimplemented and therefore unverified.
- `arves-conformance` defines an excellent invariant-keyed `Verdict::combine` worst-wins machinery (`arves-conformance/src/lib.rs:279`) but has **zero populated assertions** — no scenario carries real invariant checks against a running pipeline.
- The genuine verification-relevant work that *does* exist is in `arves-kernel`/`arves-persistence`: real replay, snapshot, `truth_hash`, and the "lossless or loud" fault-injection recovery tests (`arves-kernel/tests/recovery.rs`). These are good *example-based* tests, but they exercise fixed vectors, not generated state spaces.

So the Invariant Registry's "proof status: pending — no runtime code exists yet" is accurate, and the corpus's own falsifiability rule (Reference Lifecycle Part 4) is met for zero invariants today.

**Why it matters.** The hardest ARVES properties are exactly the ones no example-based test can cover: consensus safety (adversarial interleavings, partitions, leader churn), replay-equivalence over *arbitrary* histories, and isolation under concurrent multi-tenant load. These are the classic domains of model checking and property-based testing. Shipping only unit tests means the safety-critical claims (Axis 7) rest on the cases the authors happened to think of.

**Risks.** (i) A consensus safety bug (double-leader commit, log divergence) ships because no interleaving explorer ever ran. (ii) Replay-equivalence holds for the tested vectors but fails on an untested payload shape. (iii) The conformance suite reports PASS with zero real assertions, giving false certification confidence.

**Long-term consequences.** The tooling choice compounds: a codebase that grows for 20 years without property-based or model-checked coverage accretes untestable behaviour. Adding `proptest`/`kani`/`stateright` *now*, while the crates are skeletons, is dramatically cheaper than retrofitting.

**Alternative designs (a layered verification stack — recommended in full).**
- *Layer 1 — Model checking (protocol level).* TLA+/TLC or Rust `stateright` for the per-shard Raft (IDR-001..004): single-leader-per-term, log-matching, leader-completeness, state-machine-safety, plus the ARVES-specific "replicate outcomes not invocations" (IDR-002) and "log = WAL = trace" (IDR-003). This is the highest-value layer because consensus is where subtle safety bugs hide and Raft is the most model-checked protocol in existence.
- *Layer 2 — Property-based testing (implementation level).* `proptest` over `arves-kernel`: idempotent commit (P05-4), replay-determinism + replay-equivalence over generated histories (P05-2), snapshot/compaction round-trip, recovery losslessness under generated corruption (generalize the fixed-vector recovery tests).
- *Layer 3 — Concurrency-interleaving (`loom`).* The kernel uses `Mutex<KernelState>`; `loom` proves the commit/replay/checkpoint paths are free of the concurrency bugs unit tests miss.
- *Layer 4 — Bounded model checking (`kani`).* Prove panic-freedom and specific assertions (no double-append for the same key; recovery never returns partial-and-Ok) directly on the Rust, bounded.
- *Layer 5 — Static/architecture tests.* For `LAYER-001`/`OWN-001` (see P05-8).

**Recommendation.** Verification programme, milestone-aligned exactly as the corpus intends ("each invariant gains an executable proof during its owning milestone"): stand up `proptest` in `arves-kernel` now (Layer 2) to discharge `ORCH-003`/`ORCH-004` for I1; stand up a `stateright`/TLA+ model of per-shard Raft in I2 (Layer 1) to discharge the IDR-001..004 safety properties; add `loom` to the kernel concurrency paths; adopt `kani` for panic-freedom on the recovery path. Populate the conformance suite's assertions (Layer 5) as node contracts sharpen. This is pure Runtime/Verification work — no spec change.

**Implementation complexity:** high (breadth of tooling + a TLA+/stateright Raft model is real work), but front-loaded and cheap relative to retrofitting.
**Scientific impact:** High — a model-checked reference implementation of a cognitive-infrastructure kernel is publishable and rare.
**Ecosystem impact:** High — a certified runtime with model-checked consensus is a credible enterprise/regulated-market entrant (Vol 17).

---

## P05-6 — No liveness properties are stated anywhere (High · Verification · high)

**Precise statement.** Every ARVES invariant is a *safety* property (nothing bad happens). Not a single *liveness* property (something good eventually happens) is stated in the corpus. The obvious and necessary liveness obligations are all missing:
- **Leader election terminates** (IDR-004): under a stable-enough network, some node eventually becomes leader for the shard (`◇ (∃ n : Leadership = Established(n))`). Absent this, `Leadership::Absent` (`arves-consensus/src/lib.rs:243`) could persist forever with no property violated.
- **Commit eventually completes** (IDR-001): a proposal on the leader, given quorum, is eventually committed (`propose ⇒ ◇ committed`). The `await_commit` contract (`arves-consensus/src/lib.rs:388`) blocks indefinitely with no liveness guarantee stated.
- **Graph expansion terminates** (Vol 9 Part 6-7): the dynamically-expanded Engine Graph reaches a terminal state under the bounded termination policy (see P05-10).
- **Recovery terminates** and **replay terminates**.
- **Progress under fair scheduling** for preempted/priority work (Amendment-005: "preempted work … is replayable" — but does it eventually *run*?).

**Why it matters.** Safety-only verification is a well-known trap: a system that does nothing is trivially safe. The safety-critical axis (Scenario Conformance Framework Axis 7) demands that a safety gate *blocks* an unsafe plan — but if the graph deadlocks before reaching the gate, the plan is "blocked" only vacuously. Liveness is what distinguishes "the unsafe action didn't happen because we prevented it" from "the unsafe action didn't happen because nothing happened." For an embodied/robotic deployment (Vol 8), a livelocked planner is itself a safety hazard.

**Risks.** (i) Livelock/starvation in leader election or graph expansion goes undetected because no property forbids it. (ii) Priority inversion under preemption (Amendment-005) is unverifiable. (iii) Certification claims safety without liveness, which for a safety-critical standard is a category error.

**Long-term consequences.** Liveness bugs are the ones that surface at 3am under production load, years after certification. A standard that never states liveness cannot certify against it, ever.

**Alternative designs.**
- *(A) State liveness in TLA+ under explicit fairness (recommended).* TLA+ expresses liveness as temporal formulas under weak/strong fairness (`WF_vars`/`SF_vars`) on the enabling actions. The Raft liveness properties (election terminates under a stable leader-election timeout regime) are standard TLA+ exercises. This belongs in the AFS companion (P05-1) and is checked by TLC (with care re: liveness state-space cost) or reasoned via the standard Raft liveness argument.
- *(B) Bounded-liveness / timeout assertions in the runtime.* Where full temporal-logic liveness is impractical, encode bounded-liveness as timeouts with runtime assertions ("election completes within N election-timeouts with high probability"; "commit completes within a bounded number of rounds absent partition"). Weaker but operationally testable.
- *(C) Chaos/deadline testing.* Jepsen-style partition injection asserting eventual progress after heal. Complements, does not replace, the formal statement.

**Recommendation.** Verification deliverable + AFS companion clause: for each of the five liveness obligations, state the temporal property under explicit fairness assumptions in TLA+, and add runtime bounded-liveness assertions (election-completes-within-N, commit-completes-within-M-rounds) with metrics. Prioritize election-termination and commit-completion in I2 alongside the consensus safety model.

**Implementation complexity:** high (liveness under fairness is the subtle part of TLA+; liveness model-checking is state-space-expensive).
**Scientific impact:** Medium-High — a cognitive planner with a formal termination-under-fairness argument is uncommon.
**Ecosystem impact:** High — liveness is non-negotiable for safety-critical and enterprise SLAs.

---

## P05-7 — Tenant/shard isolation has no non-interference formalization (Medium · Verification · medium)

**Precise statement.** `SHARD-001` is stated as "partition by tenant/workspace; the partition key is immutable," and the Layer Matrix says "no cross-tenant data in a single shard." Both are *structural partition* claims (data is disjointly assigned). Neither is the property that actually matters for a multi-tenant standard: **observational non-interference** — tenant A's operations, load, or failures do not affect what tenant B observes. The corpus (and reference `ShardKey`, which enforces immutability at the type level via a private field, `arves-invariants/src/lib.rs:341`) verifies *partition disjointness* but says nothing about *interference through shared mechanism* (a shared node hosting many shards per `NodeId` doc `arves-consensus/src/lib.rs:129`; shared scheduler; shared metrics). Two tenants on the same physical node can be perfectly partitioned in data yet interfere via resource contention, timing side-channels, or a poison-pill that crashes the shared process.

**Why it matters.** Enterprise Knowledge Query (Scenario Conformance Framework, axes 1+8+9) asserts "tenant isolation held" as a *critical* property (its failure ⇒ `Fail`). But "isolation" is currently only checkable as "no B-data in A's read," not as "A cannot influence B." For a standard that will host regulated tenants side-by-side (Vol 2 tenant isolation, Vol 17 governance), the stronger non-interference property is the one regulators and security reviewers will demand.

**Risks.** (i) A conformance "isolation held" PASS that only checked data disjointness misses a timing/resource side-channel. (ii) A noisy-neighbour or poison-pill on a shared node violates isolation with no property broken. (iii) The safety story for multi-tenant embodied/agent deployments (I5) is incomplete.

**Long-term consequences.** Non-interference retrofit is expensive; stating it now shapes the scheduler/placement design before it ossifies.

**Alternative designs.**
- *(A) Formalize isolation as non-interference (recommended, staged).* Define two levels: **L-Data** (partition disjointness — already essentially provable; formalize as an Alloy/TLA+ invariant that no committed truth crosses shard keys) and **L-Observation** (a non-interference property: for any two tenants, B's observable outputs are a function only of B's inputs — a classic security-lattice / information-flow statement). Verify L-Data by model check now; treat L-Observation as a security-property obligation (coordinate with the security lens) verified by information-flow analysis and side-channel/resource-fairness testing.
- *(B) Verify only L-Data (status quo+).* Cheaper, but leaves the property the scenario actually cares about ("isolation held") under-specified.

**Recommendation.** Verification deliverable: formalize and model-check L-Data disjointness against the kernel/consensus state machine now (it is a short TLA+ invariant given `SHARD-001`); open an IDR / coordinate with the security lens to define L-Observation non-interference as the normative meaning of the conformance suite's "tenant isolation" critical property, verified by information-flow + resource-fairness tests. No spec edit; the conformance suite carries the strengthened definition.

**Implementation complexity:** medium (L-Data is easy; L-Observation is a research-grade property but can be staged).
**Scientific impact:** Medium-High — formal non-interference for a multi-tenant cognitive substrate is a strong result.
**Ecosystem impact:** High — non-interference is table-stakes for regulated multi-tenant hosting.

---

## P05-8 — Structural invariants are statically provable but left as runtime helpers (Medium · Verification · low)

**Precise statement.** `LAYER-001` (downward-only dependencies; no lateral peer calls) and `OWN-001` (exactly one owner per state) are *structural* properties provable **statically** — before any execution — because they are properties of the dependency graph and the write-capability graph, not of runtime state. The reference implementation, however, encodes `LAYER-001` only as a *runtime* helper `Layer::may_depend_on` (a numeric rank comparison, `arves-invariants/src/lib.rs:325`) and `OWN-001`/`SHARD-001` immutability only as type-level private fields. There is no compile-time or architecture-test enforcement that the *actual crate dependency graph* is acyclic and downward-only, and no static check that exactly one component holds write capability for each state class.

**Why it matters.** Structural invariants are the cheapest and strongest to verify — a static proof covers *all* executions at once, versus a runtime check that only covers exercised paths. Leaving `LAYER-001` as a runtime helper means a lateral dependency (Engine calling Capability sideways, or an upward call) could compile and ship; the helper is never consulted at build time. `OWN-001` similarly: nothing statically prevents a second component from acquiring write access to truth.

**Risks.** (i) A layer violation ships because the runtime helper is advisory, not enforced. (ii) `OWN-001` erodes silently as a new crate gains a write path to state the Kernel owns. (iii) The architecture-drift the constitution forbids (CLAUDE.md non-negotiable rules 2-4) becomes undetectable without the static gate.

**Long-term consequences.** Over 20 years, structural drift is the most insidious failure mode; a static gate is the cheapest possible guard and pays compounding dividends.

**Alternative designs.**
- *(A) Architecture tests + dependency-graph lint (recommended, low cost).* A build-time check (custom `cargo` xtask, or a tool like `cargo-deny`/`cargo-modules`/a bespoke graph analysis) that (i) asserts the crate dependency DAG is acyclic and every edge is downward-only per the `Layer::rank` order, and (ii) asserts write-capability is single-source per state class (only `arves-kernel` links the commit path). Fail the build on violation. This turns `LAYER-001`/`OWN-001` from advisory helpers into hard gates.
- *(B) Alloy model of the layer/ownership graph.* Formal and cheap; proves acyclicity/single-writer over all configurations. Good complement, belongs in the AFS companion.
- *(C) Runtime-only (status quo).* Rejected: covers only exercised paths, misses the whole point of a structural invariant.

**Recommendation.** Verification deliverable (low effort, high leverage): add an architecture-test / dependency-graph gate to CI enforcing `LAYER-001` (acyclic, downward-only crate graph aligned to `Layer::rank`) and `OWN-001` (single write-path to truth). Optionally add a tiny Alloy model in the AFS companion. Do this early — it is the cheapest proof in the whole programme.

**Implementation complexity:** low.
**Scientific impact:** Low (standard technique).
**Ecosystem impact:** Medium — a build-time architecture gate is a strong signal to independent implementers about what "conformant structure" means.

---

## P05-9 — Cross-shard sagas have no formal correctness criterion (Medium · IDR · high)

**Precise statement.** IDR-001 and Amendment-006 establish that there is **no cross-shard atomic commit**; multi-shard effects are coordinated by "sagas/compensation," and partial execution is "rolled back by NOT committing" while already-committed effects are "compensated by explicit compensation actions recorded in the decision trace." This is the *one* place ARVES deliberately abandons single-shard linearizability — and it is the one place with **no stated correctness property**. What does a correct saga guarantee? The corpus does not say. There is no statement of: atomicity-modulo-compensation (either all forward effects commit, or every committed forward effect is eventually compensated), compensation idempotency/commutativity, saga-level isolation (can a concurrent reader observe a half-applied saga?), or eventual saga-consistency.

**Why it matters.** Sagas are notoriously subtle — the failure modes (compensation-of-compensation, non-commutative compensations, observing intermediate states, compensation that itself fails) are exactly where distributed systems lose correctness. ARVES quarantined non-determinism beautifully at the engine boundary and consistency beautifully at the shard boundary, then opened a hole precisely at the cross-shard seam with no property to defend it. This is the single most dangerous *unspecified* behaviour in the truth path.

**Risks.** (i) A cross-shard workflow leaves the system in a state that is neither fully applied nor fully compensated, with no invariant violated because none exists. (ii) A reader observes a torn multi-shard write (no isolation property forbids it). (iii) Compensation failures cascade with no defined recovery.

**Long-term consequences.** As products (I6) compose multi-shard workflows, the saga layer becomes load-bearing; an unspecified saga semantics is a 20-year liability.

**Alternative designs.**
- *(A) IDR defining saga correctness formally (recommended).* An Implementation Decision Record (this is squarely an implementation decision, not a spec change) stating: **atomicity-modulo-compensation** (`◇(all-forward-committed ∨ all-committed-forward-compensated)`), **compensation must be idempotent** (reuses `ORCH-004`), **saga isolation level** (declare it: read-committed-per-shard, no cross-shard snapshot isolation in v1), and **saga liveness** (a saga eventually reaches a terminal all-or-compensated state under fairness). Verify by model-checking a saga TLA+ module against injected mid-saga failures.
- *(B) Forbid cross-shard sagas in v1 (narrow the standard).* Cleaner to verify but cripples the product layer; rejected.
- *(C) Adopt a published saga formalism* (e.g. the Garcia-Molina/Salem saga model or a formal long-running-transaction calculus) by dated normative reference. Attractive — reuses existing theory — pair with (A).

**Recommendation.** IDR (Batch 2, cross-shard coordination): formally define saga correctness (atomicity-modulo-compensation + idempotent/commutative compensation + declared isolation level + saga liveness), grounded in a dated reference to an established saga model. Verify via a TLA+ saga module with failure injection. This is an IDR, not a spec change — Amendment-006 already delegated "mechanisms → IDR."

**Implementation complexity:** high (saga verification is genuinely hard).
**Scientific impact:** Medium — formally verified sagas over a Raft-per-shard substrate is a solid systems contribution.
**Ecosystem impact:** High — multi-shard correctness underpins every non-trivial product.

---

## P05-10 — Engine-Graph expansion termination is asserted, not formalized (Low · Verification · medium)

**Precise statement.** Vol 9 Parts 6-7 state that the dynamically-expanded Engine Graph is "bounded by a termination policy (max depth / budget / no-new-subgoal) to prevent infinite meta-planning." This is a *termination* claim — the single most classic candidate for a **ranking-function / well-founded-order** proof — but it is stated as a policy, not as a proof obligation. There is no defined measure that strictly decreases on each expansion, no proof that the three cited bounds (depth, budget, no-new-subgoal) jointly guarantee termination, and no runtime assertion that the measure is monotone.

**Why it matters.** "The planner terminates" is a liveness property (P05-6) with a clean formal witness (a well-founded ranking function). It is the safety valve against a runaway meta-planning loop consuming unbounded resources — a real hazard for an autonomous-decision axis (Scenario Conformance Framework Axis 11) and a cost/DoS concern.

**Risks.** (i) A pathological engine emits sub-goals that keep the graph growing under a mis-specified policy, and nothing proves the bound actually bounds. (ii) The three bounds interact (a budget reset on re-planning) in a way that permits non-termination.

**Long-term consequences.** Modest but real: an unbounded-planning incident is a costly, hard-to-debug production event; a ranking-function proof retires the class.

**Alternative designs.**
- *(A) Ranking-function termination proof (recommended).* Define a well-founded measure `μ(graph_state)` (e.g. lexicographic `(remaining_budget, max_depth − current_depth, pending_subgoals)`) that strictly decreases on every expansion action; prove (TLA+ or by hand) that every enabled expansion decreases `μ` and that `μ` is bounded below ⇒ termination. Add a runtime assertion that `μ` is monotone non-increasing across expansions.
- *(B) Bounded-step cap only (status quo).* A hard step counter terminates but does not *explain* termination and can cut off legitimate planning; weaker.

**Recommendation.** Verification deliverable (I3/I5, with the Control-Plane/graph work): define the ranking function, prove termination of graph expansion under the Part 6-7 policy, and assert measure-monotonicity at runtime. Fold the formal statement into the AFS companion (P05-1) as the graph-termination liveness theorem (P05-6).

**Implementation complexity:** medium.
**Scientific impact:** Low-Medium — a ranking-function termination argument for a self-expanding cognitive planner is a tidy result.
**Ecosystem impact:** Medium — bounded planning is a cost/safety guarantee products rely on.

---

## Appendix A — Property → Proof-Technique → Runtime-Assertion Matrix

The consolidated enumeration the lens was asked to produce. "Runtime assertion" = a concrete check the reference runtime can execute; "Proof technique" = the mechanical/mathematical method for the class of behaviour.

| Property (formal class) | Invariant(s) | Proof technique | Verification method | Concrete runtime assertion |
|---|---|---|---|---|
| Single-writer / truth ownership (safety) | `OWN-001`, `G-001`*, `ORCH-001` | Alloy single-writer model; TLA+ invariant | Static architecture test (only kernel links commit path) + model check | Assert: exactly one code path appends to a shard's WAL; `commit` is the sole truth mutator |
| Layer downward-only (safety, structural) | `LAYER-001` | Dependency-graph acyclicity; Alloy | Build-time crate-graph lint vs `Layer::rank` | Fail build if any dependency edge points upward/lateral |
| Partition disjointness (safety) | `SHARD-001` | TLA+/Alloy invariant | Model check no truth crosses shard key | Assert every `TruthRef.shard` equals its WAL shard; shard key never re-keyed |
| Tenant non-interference (safety, info-flow) | `SHARD-001` (strengthened) | Information-flow / non-interference | Info-flow analysis + resource-fairness/side-channel test | Assert tenant B's read result is a function only of B-shard state |
| Control plane owns no state (safety) | `ORCH-001`, `ORCH-002` | TLA+ invariant | Model check + runtime probe | Assert control-plane components hold no durable handle; conformance node evidence |
| Replay-determinism (safety, functional) | `ORCH-003a` | Referential transparency; `proptest` | Property-based | Replay same trace twice ⇒ equal `truth_hash()` |
| Replay-equivalence (safety, refinement) | `ORCH-003b` | Refinement/simulation; `proptest` | Property-based over generated histories | `recover(record(run)).truth_hash() == run.truth_hash()` |
| Trace-completeness / no-recomputation (safety) | `ORCH-003c` | Sealed-replay (trap external inputs) | Fault-injection harness | Replay with clock/RNG/IO trapped-to-panic completes without touching them |
| Idempotency (safety, algebraic) | `ORCH-004`, `ENG-003`*, `CAP-003`* | Functional-equation property; `proptest` | Property-based, random interleavings | Duplicate key ⇒ ≤1 WAL append AND same `TruthRef` (variant-agnostic) |
| Content-addressing portability (safety) | `ORCH-004` | Canonicalization + digest determinism | Cross-runtime golden-digest corpus | Same canonical input ⇒ identical SHA-256 across runtimes |
| Consensus single-leader-per-term (safety) | IDR-004 | TLA+/`stateright` Raft | Model check | ≤1 leader per (shard, term) |
| Log matching / state-machine safety (safety) | IDR-001/003 | TLA+/`stateright` Raft | Model check | Committed prefix identical across replicas |
| Leader election terminates (liveness) | IDR-004 | TLA+ under fairness | Model check + chaos | Bounded: election completes within N timeouts absent partition |
| Commit completes (liveness) | IDR-001 | TLA+ under fairness | Model check + chaos | Bounded: `propose` on leader with quorum ⇒ commit within M rounds |
| Saga atomicity-modulo-compensation (safety+liveness) | Amendment-006 | TLA+ saga module + failure injection | Model check | Terminal state = all-forward-committed OR all-compensated |
| Recovery losslessness ("lossless or loud") (safety) | IDR-005, `PERSIST-001`* | `proptest` fault injection; `kani` | Property-based + BMC | Recovery returns full truth OR loud error — never partial-and-Ok |
| Graph-expansion termination (liveness) | Vol 9 Part 6-7 | Ranking function / well-founded order | Hand/TLA+ proof + runtime assert | Measure `μ` strictly decreases each expansion |
| Cancellation leaves no partial truth (safety) | Amendment-005, `CAP-008`* | TLA+ invariant; `proptest` | Model check + property test | Cancelled work ⇒ no uncommitted proposed write becomes truth |
| Panic-freedom (safety) | (implementation) | `kani` BMC; `#![forbid(unsafe_code)]` | Bounded model check | Recovery/commit paths do not panic on bounded adversarial input |
| Concurrency-safety (safety) | (implementation) | `loom` interleaving | Exhaustive small-model | `Mutex<KernelState>` paths free of data races/deadlock under all interleavings |

`*` = proposed invariant, not yet ratified (P05-3): the property is verifiable only once the invariant is formally defined.

---

## Gate Verdict & Justification

**GATE VERDICT: CONDITIONAL.**

**Are ARVES's core guarantees formally statable?** Yes — decisively. This is the finding that earns the pass. Every registered invariant is a precise property over a well-defined state space; the two hardest problems in the architecture (non-deterministic cognition, distributed truth) were solved by the two decisions that *make* formal verification tractable — deterministic replay-from-trace (`ORCH-003`) instead of recomputation, and per-shard Raft (IDR-001) instead of a bespoke protocol. The reference kernel already ships the exact instrument (`truth_hash`) that a replay-equivalence proof needs, and its recovery tests are real fault-injection proofs. A system whose non-determinism is quarantined behind a deterministic, content-addressed, replicated truth path is a system that *can* carry a formal-verification annex. Most cognitive/agent systems cannot make that claim at all.

**Are they formally *verifiable* today?** No — and this is why the verdict is CONDITIONAL, not STANDARD-READY. The properties exist only as prose (P05-1); the headline replayability claim is three unseparated obligations (P05-2); a third of the architecture's invariants are cited-but-undefined (P05-3); the idempotency/addressing algebra is unpinned and diverges across implementations (P05-4); and there is literally zero verification tooling — no property-based tests, no model checking, no bounded model checking — with the consensus core (hardest safety) still a skeleton and the conformance suite carrying zero assertions (P05-5). No liveness property is stated anywhere (P05-6). The corpus's *own* falsifiability rule (Reference Lifecycle Part 4: "unfalsifiable claims do not advance") is met for zero invariants today.

**Why CONDITIONAL and not NOT-READY:** none of the ten gaps is a *dead end*. Every one is closeable via the additive instruments the Reference Lifecycle already sanctions — a Formal Specification companion (CCP Amendment, no frozen-text edit), a milestone-aligned verification programme (Runtime/Verification), CCP ratification of the proposed invariants each with a formal property, and one IDR for saga correctness. The architecture does not need to change; it needs its guarantees written down formally and executed. The condition for advancing from CONDITIONAL to STANDARD-READY on this lens is concrete and gate-able:
1. **AFS companion exists** (P05-1): the 7 registered invariants in TLA+ (safety) + the 5 liveness obligations under fairness (P05-6), with `ORCH-003` decomposed (P05-2) and `ORCH-004` canonicalization/algebra fixed (P05-4).
2. **Executable proofs for the registered set** (P05-5): `proptest` discharging `ORCH-003`/`ORCH-004` on the kernel; a `stateright`/TLA+ Raft model discharging IDR-001..004 safety; the `LAYER-001`/`OWN-001` static architecture gate (P05-8, the cheap early win).
3. **Batch-1 proposed invariants ratified** with formal statements (P05-3): `G-001` (single-writer — the most important safety property in the system), `QUERY-001`, `LCW-001`, `PERSIST-001`.

**Recommendation to the orchestrator:** clear the gate — the deeper reviews are unambiguously worth running, because the formal-verifiability foundation is real and rare. But record CONDITIONAL: ARVES is not ISO/IEEE formal-verification-ready until the formal statements exist and the registered invariants have executable proofs. The single highest-leverage next step is the AFS companion (P05-1), because eight of the other nine findings reference a formal object it creates.

---

*Report path: `c:/Users/hkuzudisli/Desktop/Arves-Foundation-Docs/runtime/docs/reviews/P05_formal-verification.md`*
*Lens: Formal Verification (Prompt 5). No frozen-spec modifications proposed; all recommendations are IDR / CCP-Amendment / Runtime / Verification / Certification deliverables.*

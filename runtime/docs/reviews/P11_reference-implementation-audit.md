# P11 — Reference Implementation Audit (Architectural Purity)

**Lens:** Reference Implementation Audit — architectural purity, not bugs.
**Reviewer role:** Independent Chief-Architect-level reviewer optimizing for ISO/IEEE-grade adoptability over a 20-year horizon.
**Scope:** `runtime/crates/*` (arves-kernel, arves-persistence, arves-consensus, arves-control-plane, arves-query, arves-lcw, arves-execution, arves-information-platform, arves-invariants, arves-conformance, arves-engine-fabric, arves-capability-fabric, arves-ontology, arves-runtime) plus their tests.
**Governing corpus (read-only, immutable):** `runtime/review-input/*.txt`, especially `ARVES_IDR_Batch_1_Kernel_Distribution_v1.txt`, and the invariant set in `CLAUDE.md` / `ARVES_00_Invariant_Registry_v1`.

> **Hard rule honored:** No finding proposes to modify the frozen specification. Every remediation is an IDR, a CCP Amendment, a Runtime change, a Verification improvement, a Certification improvement, an Ecosystem move, or a Product move.

---

## Executive Summary

The reference runtime is unusually disciplined for an early implementation: it is `#![forbid(unsafe_code)]`, dependency-minimal, and its rustdoc is essentially a traceability narrative back to the invariants. Ownership prose (Kernel owns truth, Persistence owns durability, Control Plane owns no truth) is stated correctly and often. As *architecture writing*, it is strong.

As *architectural purity under audit*, it has one systemic defect and several boundary leaks that must be fixed before this codebase can plausibly anchor an international standard where **the reference implementation IS the tie-breaker for ambiguous prose**:

1. **The IDR numbering in the code does not match the frozen IDR Batch 1.** The *same* concept ("Raft log = WAL = decision trace") is cited as `IDR-003` in `arves-consensus`, `IDR-004` in `arves-control-plane`, and `IDR-005` in `arves-kernel`/`arves-persistence`. "Joint consensus" is `IDR-003` in the frozen spec but `IDR-004` in `arves-consensus` and `IDR-001` in `arves-persistence`. This is a **traceability corruption**: the primary asset of the reference implementation is its provable link to the spec, and that link is currently self-contradictory. (CRITICAL)

2. **The Kernel silently decides consensus-owned values.** `RefKernel::commit` hardcodes `term: 0` into every WAL record (`arves-kernel/src/lib.rs:671`). `Term` is a Raft leader-epoch concept owned by `arves-consensus` (IDR-004). The truth-owner is minting a consensus value it does not own. (HIGH)

3. **Persistence invents the content-addressing scheme for snapshots.** `install_snapshot` sets `content: ContentId(crc32_ieee(state)...)` (`arves-persistence/src/lib.rs:420, 996`). ORCH-004 content-addressing is a Kernel/spec concern; a durable store choosing CRC-32 as the "content address" of Kernel truth is a decision leaking into the wrong layer. (HIGH)

4. **`RT-001` is a governance instrument invented inline in code** (`arves-persistence/src/lib.rs:262-264`) with no definition anywhere in the frozen corpus or the Invariant Registry. The runtime minted its own change-control token. (HIGH)

5. **`SHARD-001` has no single canonical type.** `ShardKey`/`ShardId`/`TenantScope` are re-declared in at least seven crates with pairwise-incompatible shapes, glued by hand-written converters (`to_pshard`/`from_pshard`). The invariant that is supposed to be *the* partitioning identity has no authoritative representation. (MEDIUM)

6. **The "one decision trace" (IDR-005) is modeled by two independent owners.** `arves-persistence::Wal` and `arves-control-plane::DecisionTrace` are two separate append-only-log contracts, each claiming to be the recorded decision trace. IDR-005 says the log = WAL = decision trace is **one** artifact. (MEDIUM)

7. **The Kernel exposes read methods (`truth_hash`, `committed_count`) that are load-bearing in the conformance proof** and are the machine-readable contract of the cross-process restart test — reads on the sole-writer that blur QUERY-001. (MEDIUM)

8. **Snapshot serialization is split across Kernel (blob codec) and Persistence (file framing) with an implicit, uncited determinism contract.** (MEDIUM)

9. **The Kernel↔Persistence seam couples the truth-owner to a concrete storage crate and record model** rather than to a minimal port. (LOW-MEDIUM)

The through-line: the code's *prose* is pure, but at several seams the code *quietly makes a decision the specification should own*, and the citations that are supposed to prove purity are internally inconsistent. For an ISO/IEEE reference implementation, "the docs say the right thing" is not enough — the *symbols and the numbers* must be canonical and machine-checkable.

---

## Severity-Ranked Findings

| # | Severity | Title | Proposal type | Impl. complexity |
|---|----------|-------|---------------|------------------|
| F1 | **critical** | IDR citation drift: same concept mapped to 3 different IDR numbers | IDR + Verification | medium |
| F2 | **high** | Kernel hardcodes consensus-owned `term: 0` into truth records | IDR + Runtime | medium |
| F3 | **high** | Persistence invents the snapshot content-address (CRC-32) — an ORCH-004 decision | IDR + Runtime | medium |
| F4 | **high** | `RT-001` invented in code as a governance instrument, undefined in the corpus | CCP-Amendment | low |
| F5 | **medium** | SHARD-001 has no canonical type; `ShardKey` duplicated across 7 crates | IDR + Runtime | high |
| F6 | **medium** | Two owners of "the decision trace" (Wal vs DecisionTrace) contradict IDR-005 | IDR | medium |
| F7 | **medium** | Kernel read methods are load-bearing in conformance; blur QUERY-001 | Verification + Runtime | low |
| F8 | **medium** | Snapshot codec split Kernel/Persistence with implicit determinism contract | IDR | medium |
| F9 | **low-medium** | Kernel coupled to concrete Persistence crate + record model, not a port | Runtime | medium |
| F10 | **low** | `Barrier`/`Membership`/`SnapshotMarker` record kinds unused; dead ABI surface | Runtime + Verification | low |

---

## F1 — IDR citation drift: the reference implementation contradicts the frozen IDR numbering (CRITICAL)

**Evidence.** Frozen `ARVES_IDR_Batch_1_Kernel_Distribution_v1.txt` (authoritative):
- IDR-001 = Consensus Strategy (per-shard Raft)
- IDR-002 = Replication Strategy (leader→followers, snapshots + WAL)
- IDR-003 = **Membership** Strategy (Joint Consensus)
- IDR-004 = **Leader Election**
- IDR-005 = **Storage** (append-only WAL = ordered record = single source for deterministic replay)

The code cites otherwise, inconsistently:

- `arves-consensus/src/lib.rs:28,99,107,113,117,169` — "**IDR-003** — The Raft log **is** the WAL **is** the decision trace" and treats IDR-003 as append-only-log/storage. Frozen IDR-003 is *Membership*, and log=WAL=trace is IDR-005 (+ IDR-001 convergence).
- `arves-consensus/src/lib.rs:33,190,214,247,252` — "**IDR-004** — Membership changes use joint consensus" AND "IDR-004 (per-shard leader election)". Frozen: joint consensus = IDR-003; leader election = IDR-004. The crate collapses two IDRs into one number and drops IDR-005.
- `arves-persistence/src/lib.rs:6,10` — "**IDR-005**: Raft log = WAL = decision trace" (defensible, since IDR-005 = Storage) but line **88** cites "membership change committed via joint consensus (**IDR-001**)" — joint consensus is IDR-003, not IDR-001 — and line **14** cites "IDR-003 (embodied as ORCH-003): recovery is replay" — IDR-003 is Membership.
- `arves-control-plane/src/lib.rs:37,115,350,389,402,413` — "The Raft log doubles as the WAL and the decision trace (**IDR-004**)" and line **134** "no cross-shard atomic commit (**IDR-005**)". Neither matches the frozen text (log=WAL=trace ≈ IDR-005; no-cross-shard is stated under IDR-001's refinements).
- `arves-kernel/src/lib.rs:43,90,120,135` — "IDR-004/IDR-005 (Raft log = WAL = decision trace)" and "IDR-002/IDR-003 (shard leader)". IDR-003 is Membership, not leadership.

So the concept "log = WAL = decision trace" is simultaneously IDR-003 (consensus), IDR-004 (control-plane), and IDR-005 (kernel/persistence). "Joint consensus" is IDR-001 (persistence), IDR-003 (spec), IDR-004 (consensus). This is not a typo in one file; it is *divergent mental models* frozen into rustdoc across the workspace.

**Why it matters.** The single most valuable property of a reference implementation for a standard body is that **every behavior is traceable to a governing clause**, and reviewers/certifiers can mechanically follow the citation. When the citations disagree with each other and with the frozen source, the traceability chain `Theory → Spec → Contracts → Behaviour → Conformance → Implementation` (the PRIMARY ENGINEERING PRINCIPLE) is broken at the last hop. An ISO/IEEE reviewer who spot-checks "show me where IDR-003 is honored" will find three different answers and lose confidence in the entire corpus.

**Risks.** (a) A future engineer "fixes" code to match a wrong citation, propagating the error into behavior. (b) A CCP Amendment that references "IDR-004" is interpreted differently by different crates. (c) Certification artifacts (the conformance `RuntimeFingerprint`) claim conformance to invariants whose IDs are unstable.

**Long-term consequences.** Citation drift compounds: by I4–I6, with dozens of IDRs, an un-canonicalized citation space becomes unauditable, and the "reference implementation as tie-breaker" model collapses.

**Alternative designs.**
- *(A) Canonical ID module (recommended).* Extend `arves-invariants` with an `idr` module mirroring the existing `catalog` pattern: `pub const IDR_003: &str = "IDR-003"` plus a one-line frozen statement, and require all crates to cite via the symbol, not a string literal. A compile-time doctest/const-assert enforces that the statement text matches the frozen source. This makes every citation compiler-checked, exactly as invariant IDs already are.
- *(B) Free-text errata pass only.* Cheaper, but leaves the space stringly-typed and re-driftable.
- *(C) Re-number in the spec.* **Forbidden** — the corpus is frozen.

**Recommendation.** Open an **IDR** ("IDR citation canonicalization") that (1) declares the code's citations must match `ARVES_IDR_Batch_1` exactly, (2) adds an `arves_invariants::idr` const catalog with frozen statements, and (3) mandates symbol-based citation. Then a **Verification** improvement: a workspace lint / test that greps rustdoc for `IDR-\d{3}` string literals and fails CI (only symbol references allowed). This is a mechanical, non-behavioral fix — it changes *comments and one constants module*, not logic — and it restores the reference implementation's core value.

**Implementation complexity:** medium (touches every crate's doc comments; the enforcement harness is small).
**Scientific impact:** high — restores machine-checkable traceability, the property a standards body most needs to verify.
**Ecosystem impact:** high — independent Runtime A/Runtime B implementers rely on these citations to know *which clause* they must satisfy.

---

## F2 — The Kernel silently decides a consensus-owned value (`term: 0`) (HIGH)

**Evidence.** `arves-kernel/src/lib.rs:668-678` — `RefKernel::commit` builds a `PendingRecord { ..., term: 0, kind: RecordKind::Outcome, ... }`. The Kernel picks the Raft term.

`Term` is defined and documented as a **consensus** concept: `arves-persistence/src/lib.rs:51-55` ("Raft term ... per-shard leader election, IDR-001") and `arves-consensus/src/lib.rs:102-109` (`Term` "totally order leadership"). The Kernel is not the leader-election authority — `arves-consensus` is (frozen IDR-004).

**Why it matters.** ORCH-001/OWN-001 pin *cognitive truth* to the Kernel, but a Raft `term` is *not cognitive truth* — it is a consensus-layer fact owned by the leader-election mechanism. By writing `term: 0` the Kernel is (a) asserting a value from a layer it must not own, and (b) baking a magic constant into the durable trace that will be *wrong* the moment real Raft (I2) exists, because term 0 is the pre-election bootstrap term. When I2 wires in `arves-consensus`, either the Kernel keeps lying (`term: 0` forever, corrupting the leader-epoch record used for replay disambiguation) or the Kernel must reach *up/across* into consensus to fetch the current term — an ownership inversion.

**Risks.** Replay across leader epochs (the documented purpose of `Term` in persistence) is impossible with a constant term; a stale-leader write cannot be distinguished from a current one. This directly undermines IDR-004's "in-flight uncommitted work is discarded" guarantee.

**Long-term consequences.** The single-node skeleton's `term: 0` becomes a load-bearing assumption that I2 must unwind; worse, existing on-disk WALs written by the skeleton carry `term: 0` and are indistinguishable from a genuine term-0 record.

**Alternative designs.**
- *(A) Term is supplied to the commit path, never chosen by the Kernel (recommended).* The commit surface should accept an already-decided consensus context (leader term) from `arves-consensus`; in the single-node skeleton, inject a `LeaderContext { term }` sourced from a trivial "always-leader term 1" stub that is explicitly a consensus stand-in, not a Kernel constant.
- *(B) Sentinel term for pre-consensus era.* Reserve a documented `Term::PRE_CONSENSUS` and forbid it once I2 lands. Cheaper, but still a Kernel-owned value.
- *(C) Drop `term` from persistence records until consensus exists.* Rejected — the field is part of the durable frame format (`WAL_FRAME_VERSION`), and removing it is a format migration.

**Recommendation.** **IDR** ("Consensus context injection at the commit gateway") establishing that the Kernel never originates a `Term`; it is always passed in from the consensus substrate (a stub in I1). Plus a **Runtime** change: replace the literal `term: 0` with an injected `LeaderContext`, and mark records written by the pre-consensus skeleton with a distinct, documented sentinel so I2 can detect and migrate them.

**Implementation complexity:** medium.
**Scientific impact:** medium-high — clean layer ownership of consensus epochs is what makes deterministic replay provable.
**Ecosystem impact:** medium — a second implementation must agree on *who* owns the term, or the two WAL formats diverge.

---

## F3 — Persistence invents the snapshot content-address (CRC-32 as ORCH-004 identity) (HIGH)

**Evidence.** `arves-persistence/src/lib.rs:420, 996, 1028` — every `SnapshotMeta` sets `content: ContentId(crc32_ieee(state).to_le_bytes().to_vec())`. The store's own rustdoc says `ContentId` "Supports ORCH-004 ... content-addressable" (`:70-76`). CRC-32 is a *corruption detector*, not a content address: it is not collision-resistant and is trivially forgeable. Persistence is choosing the addressing function for Kernel-owned truth.

**Why it matters.** ORCH-004 (idempotent + content-addressable) is a cross-cutting invariant whose *content-hash function* is a specification-level decision (the Kernel keys idempotency on it in `commit`, `arves-kernel/src/lib.rs:655-663`). Persistence — which is documented as owning *no* truth and making *no* cognitive decisions (PERSIST-001, `:18-20`) — is unilaterally defining what "the content address of this truth blob" means, and defining it as a value (CRC-32) that cannot serve ORCH-004's dedup/idempotency purpose. This is a decision leaking into the layer least entitled to make it. The Kernel already produces the blob (`snapshot_shard`) and the Kernel already owns content hashing (`ContentHash`), so the addressing authority belongs above.

**Risks.** If any code ever treats `SnapshotMeta.content` as an ORCH-004 identity (dedup, cross-node snapshot fetch in I2), CRC-32 collisions/forgeries become truth-integrity holes. Two different snapshots can share a CRC-32; a malicious or corrupt peer could serve a wrong snapshot that passes the "content address" check.

**Long-term consequences.** I2 snapshot transfer between Raft peers will want a real content address; retrofitting one after the field has shipped as CRC-32 is a `SnapshotMeta` format migration and a semantics change for every stored `.snap`.

**Alternative designs.**
- *(A) Kernel supplies the content address; Persistence stores it verbatim (recommended).* Mirror the existing `ProposedWrite.content` pattern: `install_snapshot` takes the blob **and** its Kernel-computed `ContentId`; Persistence keeps a *separate* integrity CRC internally for torn-write detection but never labels it "content address." This restores ORCH-001/OWN-001/PERSIST-001 cleanly.
- *(B) Standardize a hash in the spec via IDR.* Choose one digest (e.g., a specified SHA-2/3 variant) as the ORCH-004 hash function for the reference implementation.
- *(C) Rename the field to `integrity: Crc32` in persistence.* Honest about what it is, but leaves ORCH-004 addressing unspecified.

**Recommendation.** **IDR** ("ORCH-004 content-address function and ownership") fixing the hash function and stating the *Kernel* computes it, and a **Runtime** change so `install_snapshot`/`SnapshotMeta.content` carry a Kernel-supplied address while the CRC stays an internal integrity check under a distinct name. Combine with F8.

**Implementation complexity:** medium.
**Scientific impact:** high — content-addressability is a load-bearing correctness primitive (replay, dedup, peer snapshot verification); leaving it to an accidental CRC is a soundness gap.
**Ecosystem impact:** high — independent runtimes must agree on the address function or they cannot exchange snapshots/verify each other.

---

## F4 — `RT-001` is a governance instrument invented in code, undefined in the corpus (HIGH)

**Evidence.** `arves-persistence/src/lib.rs:262-264` — `install_snapshot` doc: *"Governing: RT-001 (this activates the previously-reserved `SnapshotMeta` / `RecordKind::SnapshotMarker` surface; it is Reference Runtime interface evolution, not a specification change)."* Also referenced at `:551`. `RT-001` appears **nowhere** in the frozen corpus or `ARVES_00_Invariant_Registry_v1`. The runtime coined a new change-control token (`RT-*`, "Reference Runtime") and used it to self-authorize an interface addition.

**Why it matters.** CLAUDE.md's CHANGE MANAGEMENT table enumerates exactly four instruments: CCP Amendment, Architecture Review, IDR, Next Major Version. There is no `RT` instrument. Whether or not "activating a reserved field" is benign, *inventing a governance category inline* is precisely the "silent change to architecture" the constitution forbids ("Never silently change the architecture"). If the runtime can mint its own governance tokens, the change-control discipline that a standards body relies on is unenforceable.

**Risks.** `RT-*` proliferates as a catch-all escape hatch ("it's just runtime interface evolution") that launders real design decisions past the CCP-GATE. The proposed invariants (PERSIST-001 etc.) are supposed to enter via CCP Amendment/IDR *with a conformance scenario*; an `RT-*` side-channel bypasses that gate.

**Long-term consequences.** Two parallel governance vocabularies (official IDR/CCP vs. ad-hoc RT) make the provenance of any given interface un-auditable.

**Alternative designs.**
- *(A) Fold `RT-*` into IDR (recommended).* "Reference-runtime interface evolution" is exactly what an IDR is for. Re-issue the `install_snapshot` activation as, e.g., `IDR-006` (Batch 2) or a documented sub-decision, and delete `RT-001`.
- *(B) Formally define an `RT` instrument via CCP Amendment.* Only if the maintainers genuinely want a lighter-weight category — but this itself must go through CCP, not be asserted in a doc comment.
- *(C) Leave it, add a registry entry.* Rejected — it still originated as a silent, self-authorized token.

**Recommendation.** **CCP Amendment** (or IDR) that either (a) abolishes `RT-001` and re-homes the decision under IDR, or (b) formally defines the `RT` instrument in the Change Management table and the Registry with a conformance obligation. Until then, `RT-001` should be treated as unregistered and the interface addition re-justified under an existing instrument.

**Implementation complexity:** low (documentation + one governance entry).
**Scientific impact:** medium — governance integrity is a prerequisite for calling the corpus "frozen."
**Ecosystem impact:** medium — third-party certifiers must be able to trust that every interface has a governed origin.

---

## F5 — SHARD-001 has no canonical type; `ShardKey` is duplicated across 7 crates (MEDIUM)

**Evidence.** Distinct, pairwise-incompatible partition-key types:
- `arves-kernel`: `struct ShardKey { tenant: String, workspace: String }` (`:64`)
- `arves-persistence`: `struct ShardKey { tenant: String, workspace: String }` (`:62`) — plus `Ord`/`PartialOrd`, unlike Kernel's
- `arves-consensus`: `struct ShardId { tenant: TenantId, workspace: WorkspaceId }` with newtyped parts (`:80`)
- `arves-control-plane`: `struct ShardKey(pub String)` (`:85`) — single opaque string
- `arves-lcw`: `struct ShardKey { tenant, workspace }` (`:80`)
- `arves-query`: `pub type ShardKey = String` (`:85`)
- `arves-conformance`: `pub type ShardKey = String` (`:299`)
- `arves-information-platform`: `struct TenantScope { tenant, workspace }` (`:83`)
- `arves-execution`: `struct ShardKey { tenant, workspace }` (`:62`)
- `arves-invariants`: `struct ShardKey(String)` with a `tenant/workspace → "t/w"` composition (`:341-357`)

The Kernel↔Persistence seam is bridged by hand-written converters `to_pshard`/`from_pshard` (`arves-kernel/src/lib.rs:266-277`).

**Why it matters.** SHARD-001 is a *registered, normative* invariant: "partition by tenant/workspace; key immutable." A registered invariant with **no single authoritative representation** cannot be enforced structurally — each crate re-derives it, and they already disagree on shape (`(tenant, workspace)` struct vs. opaque `String` vs. newtyped parts vs. `"t/w"` join). The composition rule matters: `arves-invariants` joins with `/`, so a tenant named `a/b` collides with workspace boundaries — a real immutability/identity hazard that a canonical type would forbid by construction.

The crates justify duplication as "std-only, so LAYER-001 dependencies aren't inverted." But `arves-invariants` is explicitly designed as the zero-dependency, cross-cutting home for exactly these symbols (`LayerRank`, its own `ShardKey`) — the canonical type belongs there, and depending *downward* on it inverts nothing.

**Risks.** Serialization drift (persistence uses hex-encoded `tenant__workspace` dir names; a `String`-based shard key in query/conformance cannot round-trip the same identity), and inconsistent `Ord` (persistence sorts shards for deterministic cross-shard replay order, `:1309-1312`; other crates have no total order), meaning "the same shard" is not provably the same across layers.

**Long-term consequences.** By I3 (distributed query) and I5 (multi-agent), shard identity crosses every layer boundary; N incompatible encodings guarantee subtle isolation bugs (a `SHARD-001` violation is a tenant-isolation breach — the most severe class in `arves-conformance`, `Property::TenantWorkspaceIsolation` is CRITICAL).

**Alternative designs.**
- *(A) Canonical `ShardKey` in `arves-invariants` (recommended).* Define the one immutable, structured, totally-ordered, collision-free (length-prefixed, not `/`-joined) shard key in the dependency-free invariants crate; every layer depends downward on it. Keeps LAYER-001 intact and gives SHARD-001 a single enforceable type.
- *(B) A tiny `arves-types` foundation crate.* Same effect, new crate.
- *(C) Keep per-crate types but generate them from one source + add cross-crate round-trip conformance tests.* Weaker; drift still possible.

**Recommendation.** **IDR** ("Canonical shard-key type and encoding") + **Runtime** refactor to a single `ShardKey` in `arves-invariants`, with a collision-free encoding and a documented total order, replacing the converters. Add a conformance property that any two layers agree on shard identity for the same `(tenant, workspace)`.

**Implementation complexity:** high (touches every crate + the on-disk dir-naming in persistence).
**Scientific impact:** medium-high — a normative partitioning invariant needs a canonical carrier to be provable.
**Ecosystem impact:** high — cross-implementation interop requires one wire encoding of shard identity.

---

## F6 — Two independent owners of "the decision trace" contradict IDR-005 (MEDIUM)

**Evidence.** `arves-persistence` defines `trait Wal` with `append`/`replay_from` as "the durable face of the single artifact (IDR-005): Raft log = WAL = decision trace" (`:214-233`). Separately, `arves-control-plane` defines `trait DecisionTrace` with its *own* `append(decision) -> TraceIndex` and `replay_from(from) -> Vec<DecisionRecord>` (`:407-424`), also citing "IDR-004 (Raft log = WAL = decision trace; append-only)" and its own `TraceIndex`/`DecisionRecord` types.

**Why it matters.** IDR-005 (and the frozen text's "IDR-001 + IDR-005 + ORCH-003 converge on one ordered source") is emphatic that the log, the WAL, and the decision trace are **one artifact viewed from three angles** — not three artifacts, and certainly not two independently-appendable logs. `arves-control-plane`'s `DecisionTrace` is a *second* append-only-log abstraction with its own index space (`TraceIndex(u64)` vs persistence `Offset`) and its own record model (`DecisionRecord` vs `WalRecord`). OWN-001 says one owner per state; here two crates each present an "append to the trace" API, and nothing in the type system says they are the same underlying sequence.

**Risks.** An implementer can wire `DecisionTrace` to a *different* store than the Kernel's WAL, producing two divergent "decision traces" — exactly the fork IDR-005 exists to prevent. Replay (ORCH-003) then has two candidate sources with no canonical ordering between orchestrator decisions and kernel outcomes.

**Long-term consequences.** In I5 (multi-agent), orchestrator decisions and kernel outcomes must interleave in one total order to be replayable; two logs cannot be linearized after the fact.

**Alternative designs.**
- *(A) `DecisionTrace` is a typed *view/projection* over the one WAL, not a second log (recommended).* Control-plane decisions become a `RecordKind` in the single per-shard WAL (the frozen text already says the Raft log carries decisions). `DecisionTrace` becomes a read-only lens with encode/decode into `WalRecord`, sharing `Offset`.
- *(B) Keep two traces but add a spec-level ordering contract (IDR) that pins them to one Raft log with a defined merge order.* More complex; still two APIs.
- *(C) Document that `DecisionTrace` is illustrative only.* Weak; leaves an ownership ambiguity in the reference.

**Recommendation.** **IDR** ("The decision trace is the WAL; control-plane decisions are WAL records") that reconciles `DecisionTrace` as a projection over `arves-persistence::Wal`, sharing the offset space, so there is provably one ordered source per shard.

**Implementation complexity:** medium.
**Scientific impact:** medium-high — single-source replay determinism is central to ORCH-003.
**Ecosystem impact:** medium.

---

## F7 — Kernel read methods are load-bearing in conformance and blur QUERY-001 (MEDIUM)

**Evidence.** `arves-kernel/src/lib.rs:631-649` — `committed_count()` and `truth_hash()` are public methods on the truth-owner, doc-labeled "Introspection (NOT the Query layer)." Yet `arves-runtime/src/main.rs` prints them as `TRUTH_HASH=`/`COUNT=`, and `arves-runtime/tests/real_restart.rs` asserts on those exact strings as "the machine-readable contract the restart proof parses" (`main.rs:56-57`). The I1.7 recovery tests (`arves-kernel/tests/recovery.rs:135,144,150-151`) also assert on `truth_hash`/`committed_count`.

**Why it matters.** The Kernel trait deliberately has *no reads* (`:30-37, :208-212`) to protect the single-writer contract; QUERY-001 (proposed) makes reads the Query layer's exclusive concern. But the concrete `RefKernel` re-introduces reads on the truth-owner and — more importantly — those reads are now the **definition of correctness** for the milestone's headline proof. A conformance obligation that reads the truth-owner directly, bypassing Query, entrenches a read path the architecture says should not exist. When Query is real (I3), there will be two read paths to truth (Kernel introspection + Query), and the conformance harness will still depend on the wrong one.

**Risks.** The "not the Query layer" caveat is prose, not enforcement; downstream code (or a second implementer) will treat `truth_hash` as a stable read API because the tests do. This is how a skeleton convenience becomes a de-facto contract.

**Long-term consequences.** Certification artifacts that fingerprint runtime state via a Kernel-side hash will be incomparable to a runtime that (correctly) exposes state only via Query.

**Alternative designs.**
- *(A) Move the replay-equivalence proof behind a dedicated test-only capability, not a public Kernel method (recommended).* Expose `truth_hash` via a `#[cfg(test)]` or a separate `arves-kernel-testkit` seam, or compute the fingerprint through the (stub) Query layer so the conformance proof exercises the architectural read path.
- *(B) Define a spec-level "Runtime Fingerprint" read that is explicitly a Query-layer projection.* `arves-conformance::RuntimeFingerprint` already exists; route the hash through it.
- *(C) Keep as-is but mark the methods `#[doc(hidden)]` and forbid external use.* Weakest.

**Recommendation.** **Verification** improvement: the deterministic-replay proof should read truth through the architectural read surface (Query stub) rather than a Kernel getter, and **Runtime**: demote `truth_hash`/`committed_count` to a test-only introspection seam so they cannot ossify into a public read API on the sole writer.

**Implementation complexity:** low.
**Scientific impact:** medium — keeps the "Kernel is a gateway, not a store" property enforceable rather than aspirational.
**Ecosystem impact:** medium — the conformance read path is what independent runtimes must replicate.

---

## F8 — Snapshot serialization split across Kernel and Persistence with an implicit determinism contract (MEDIUM)

**Evidence.** The Kernel owns the *blob codec* (`encode_shard_blob`/`decode_shard_blob`, `arves-kernel/src/lib.rs:298-344`, sorting entries by offset, `:584`) and asserts determinism ("fixed-order little-endian ... snapshot + tail replay reproduces the same truth set as a from-zero replay", `:291-296`). Persistence owns the *file framing* (`encode_snapshot`/`decode_snapshot` with `SNAP_VERSION`, `:827-865`). Two versioned formats (`WAL_FRAME_VERSION`, `SNAP_VERSION`) plus an un-versioned Kernel blob format, with the cross-layer determinism guarantee stated only in comments.

**Why it matters.** The determinism of `snapshot + tail == from-zero replay` (an ORCH-003 obligation) depends on *both* the Kernel blob's byte-order/sort AND persistence's framing being stable, but only persistence's half is version-stamped. If the Kernel blob layout changes, an old snapshot decodes into a *different* truth set with no version guard (`decode_shard_blob` returns `None`→`unwrap_or_default()` = **silent empty state** on mismatch, `:591`). That is a silent-truth-loss path hiding inside a "purity" seam.

**Risks.** A Kernel blob format change silently zeroes recovered truth for old snapshots (defect class the I1.7 hardening explicitly set out to eliminate — "lossless or loud" — yet `install_state`'s `unwrap_or_default()` is silent-lossy).

**Alternative designs.**
- *(A) Version and CRC the Kernel blob too; make decode failure loud (recommended).* Add a `KERNEL_BLOB_VERSION`, and have `install_state` surface a `RecoveryError` on decode failure instead of `unwrap_or_default()`.
- *(B) Move all snapshot serialization to one owner.* Since the Kernel owns truth, the Kernel should own the whole snapshot format and persistence stores fully opaque bytes (it nearly does — this tightens it).
- *(C) IDR defining the snapshot format contract across the seam.*

**Recommendation.** **IDR** ("Snapshot format ownership and versioning") stating the Kernel owns a versioned, integrity-checked blob and decode failure is loud, and a **Runtime** fix replacing `unwrap_or_default()` with a `RecoveryError` (aligns with the crate's own "lossless or loud" doctrine, `:351-357`).

**Implementation complexity:** medium.
**Scientific impact:** medium — closes a silent-loss hole in the replay-equivalence claim.
**Ecosystem impact:** low-medium.

---

## F9 — Kernel coupled to the concrete Persistence crate and its record model, not a minimal port (LOW-MEDIUM)

**Evidence.** `arves-kernel/Cargo.toml` depends on `arves-persistence`, and `arves-kernel/src/lib.rs:261-264` imports concrete types `FileWalStore, MemWalStore, PendingRecord, RecordKind, ContentId, ReplayCursor, Wal, WalError, WalStore` plus persistence's own `ShardKey as PShardKey`. `RefKernel<S: WalStore>` is generic over the store *trait* (good), but the Kernel still (a) constructs `PendingRecord`/`RecordKind::Outcome` (persistence's record model) and (b) converts between two `ShardKey` types.

**Why it matters.** LAYER-001 permits Kernel→Persistence (downward). So the *dependency direction* is legal. The purity issue is narrower: the Kernel is coupled to persistence's *record vocabulary* (`RecordKind`, `PendingRecord`, `Term`) rather than to a minimal "durable outcome sink" port. This is why F2 (`term`) and F3 (CRC address) leak — the Kernel is forced to fill persistence-shaped fields it doesn't own. A thinner port (append opaque `(content, payload)`; return `offset`) would make it structurally impossible for the Kernel to set a `term` or for persistence to demand one.

**Risks.** Every future persistence-layer field (compression, encryption, membership records) becomes something the Kernel's `commit` must think about; the seam widens over time.

**Alternative designs.**
- *(A) Define the durable-sink port in `arves-kernel` (or a shared contracts crate) with the minimal `append(content, payload) -> offset` + snapshot/replay surface; persistence implements it (recommended).* Inverts nothing (persistence depends on the port), and removes `term`/`RecordKind` from the Kernel's concern.
- *(B) Keep the current generic-over-`WalStore` design but narrow `PendingRecord` to omit consensus fields until consensus exists.*

**Recommendation.** **Runtime** refactor to a minimal durable-outcome port so the Kernel cannot express consensus-owned fields (subsumes F2). Optional shared `arves-contracts` crate.

**Implementation complexity:** medium.
**Scientific impact:** low-medium.
**Ecosystem impact:** medium — a clean port is what a second persistence implementation plugs into.

---

## F10 — Dead ABI surface: unused record kinds and consensus variants (LOW)

**Evidence.** `RecordKind::{Membership, SnapshotMarker, Barrier}` (`arves-persistence/src/lib.rs:88-95`) are defined and encoded (`kind_to_u8`) but never produced by any commit/checkpoint path (only `Outcome` is written; `arves-kernel` `install_snapshot` uses a separate `.snap` file, not a `SnapshotMarker` record). Similarly `arves-consensus` `EntryKind::Membership` / `Membership::Joint` are pure skeleton.

**Why it matters.** For a *skeleton* this is expected and even good (reserving the ABI). The purity concern is that reserved-but-unexercised variants are exactly where format drift and untested corruption paths hide (a `SnapshotMarker` byte in a real WAL would decode to a record the Kernel's replay loop treats as an `Outcome`-shaped truth, since `try_replay` doesn't branch on `kind`, `:548-568`).

**Risks.** When these are activated, replay must learn to handle non-`Outcome` kinds; today `RefKernel::try_replay` blindly materializes every record as truth regardless of `kind`, so an activated `Membership`/`Barrier` record would be mis-ingested as truth.

**Recommendation.** **Runtime**: `try_replay` should explicitly match on `RecordKind` and skip non-truth kinds (defensive today, mandatory at I2), and **Verification**: a test that a `Barrier`/`Membership` record is *not* counted as truth. Document reserved variants as "reserved; MUST be rejected/ignored by I1 replay."

**Implementation complexity:** low.
**Scientific impact:** low.
**Ecosystem impact:** low.

---

## If ARVES were standardized by ISO/IEEE tomorrow — what this lens says is still missing

1. **A canonical, machine-checked citation space.** Today the reference implementation's IDR citations disagree with the frozen source and with each other (F1). An ISO/IEEE reference implementation must have *every* clause reference resolve to exactly one frozen statement, ideally compiler-enforced. The invariant IDs already do this (`arves-invariants::catalog`); IDRs and the shard-key type do not.

2. **A single source of truth for shared normative types.** SHARD-001 is normative but has ~7 incompatible representations (F5). A standard needs one canonical wire/type encoding for each normative concept (shard identity, content address, term, offset) so independent Runtime A and Runtime B can interoperate and be certified against the *same* bytes.

3. **Enforced layer/ownership boundaries, not documented ones.** The purity today is prose ("NOT the Query layer", "owns no truth"); the code still lets the Kernel mint a `term` (F2), lets Persistence define a content address (F3), lets two crates own "the trace" (F6), and lets a Kernel getter become the conformance contract (F7). A standard needs these encoded so a *conformance test can fail* when a boundary is crossed — e.g., a durable-sink port the Kernel physically cannot over-fill, and a content-address function fixed by IDR.

4. **Governed provenance for every interface.** `RT-001` (F4) shows the runtime can currently self-authorize interface change outside the four sanctioned instruments. A standard requires that no interface exists without a traceable, governed origin.

5. **"Lossless or loud" everywhere, including the Kernel blob.** The recovery hardening is excellent for the WAL, but the Kernel snapshot blob still has a silent-empty decode path (F8/F10). ISO-grade durability requires that *no* deserialization step can silently substitute empty/partial state.

Fixing F1–F4 is the minimum bar to make the "reference implementation as the authoritative tie-breaker" model credible; F5–F8 are what make it *interoperable and certifiable* across independent implementations.

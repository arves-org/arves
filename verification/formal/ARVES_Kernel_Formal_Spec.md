# ARVES Kernel Formal Specification (TLA+) — Verification Evidence

**Type:** Verification evidence (formal-methods artifact), delivered under the
`verification/` tree per the ARVES v1.1 Standardization Program, Goal 2 (Formal
Foundation). **This is NOT a normative specification.** It does not define
ARVES behaviour; it *models* the behaviour the frozen corpus already prescribes
and mechanically checks that the model upholds the frozen invariants.
**Status:** DRAFT evidence (Candidate on CCP-GATE pass of the scenario in §7).
**Closes (contributes to):** Global Readiness **R-05** (Formal Specification
companion — TLA+ invariant core + liveness under fairness). **Answers findings:**
**P05-1** (no formal spec of any property), **P05-2** (ORCH-003 conflation),
**P05-4** (idempotency algebra unpinned), and part of **P05-6** (no liveness
stated) — for the single-shard kernel core only.
**Immutability:** the frozen v1.0 `.docx` corpus is untouched (**ED-001**). This
artifact is additive evidence; it is ratified — like any behaviour — through the
Reference Lifecycle **CCP-GATE** = *this document + at least one conformance
scenario* (§7).

> Normative keywords (MUST / SHALL / SHOULD / MAY) follow RFC 2119/8174. Because
> this is *evidence*, RFC-2119 keywords here bind **the model and any tool that
> checks it**, and bind a **conformant reference kernel** to reproduce §7's
> vectors — they do **not** add or alter any normative ARVES requirement. The
> normative source of the invariants remains the frozen corpus.

**Companion files (this directory):**
- [`ARVES_Kernel.tla`](./ARVES_Kernel.tla) — the TLC-checkable module.
- [`ARVES_Kernel_MC.cfg`](./ARVES_Kernel_MC.cfg) — the TLC instance (`Content = {c1, c2}`).

---

## 1. Problem

The frozen corpus states every ARVES guarantee as one line of English prose in a
table cell (Invariant Registry Table 0; Vol 9 Cognitive Control Plane v2 Part 5).
P05-1 found the single largest formal-verification gap: **no property exists in a
machine-checkable notation**, so a certifier cannot reproduce the safety/liveness
argument without asking the authors, and two independent runtimes can each "prove"
the same prose invariant against incompatible informal readings. P05-2 found that
`ORCH-003` ("every execution is replayable") silently fuses three distinct
obligations. P05-4 found `ORCH-004` ("idempotent and content-addressable") is
stated without an idempotency algebra, and that the reference kernel surfaces
idempotency as `Err(AlreadyCommitted)` while another implementation could return
`Ok(existing)` — the formal *shape* differs across implementations.

This artifact supplies the missing formal object for the **kernel truth core**:
a small, abstract, TLC-checkable state machine that renders the three
kernel-owned invariants as machine-checked properties, plus the first stated
ARVES **liveness** property.

## 2. Scope

`ARVES_Kernel.tla` models **one shard**: a commit gateway (the single
truth-mutating action), an **append-only log** (the Raft-log-as-WAL-as-decision-
trace of IDR-003/005, here reduced to its append-only essence), and an
**in-memory truth set** the Kernel owns (ORCH-001/OWN-001), together with a
**replay** reconstruction of truth from the log. It checks four safety invariants
and one liveness property (§4).

It is **deliberately abstract and small** — a model that TLC actually exhausts in
under a second is worth more as evidence than a large aspirational one that never
runs. What is abstracted away is stated honestly in §6; the headline omission is
**distribution/Raft (leader election, replication, quorum, crash-recovery,
cross-shard sagas)**, which belongs to milestone I2+ and to a later, larger
`ARVES_Consensus.tla`.

## 3. The abstract state machine (informative summary of the module)

| Element | Model | Frozen grounding | Reference kernel |
|---|---|---|---|
| `log` | `Seq(Content)`, append-only | Raft log = WAL = decision trace (IDR-003/005) | `WalStore::append` |
| `truth` | `SUBSET Content` (set of committed content) | Kernel owns truth (ORCH-001, OWN-001) | `KernelState.committed` |
| `pc[c]` | `open → proposed → committed` | producers propose (ORCH-001), Kernel commits | `ProposedWrite` → `commit` |
| `Propose(c)` | arms `c`; mutates **no** truth | Control Plane owns no truth (ORCH-001) | proposal construction |
| `Commit(c)` | the **sole** truth mutator | single write path (OWN-001, G-001-proposed) | `RefKernel::commit` |
| replay | `Range(log)` (fold to element set) | replay reads the trace, never recomputes (ORCH-003) | `RefKernel::try_replay` |

`Commit(c)` is the only action that changes `truth` or `log`. Its idempotent
branch (`c \in truth`) appends **nothing** and returns the existing truth —
exactly the `arves-kernel` behaviour where a duplicate `ContentHash` resolves to
the existing `TruthRef` (`arves-kernel/src/lib.rs`, the `AlreadyCommitted` path)
rather than forking truth.

## 4. What each property proves

### 4.1 `OWN_001` — single writer / one owner per `(shard, content)` (safety)
```
OWN_001 == \A c \in Content : Count(log, c) <= 1
```
Within the one shard, each content address appears in the committed log **at most
once**. Because `Commit` is the *only* action that appends, and it appends only on
the first commit of a content (the `c \notin truth` branch), there is no second
write path and no fork. This is the machine-checked form of **OWN-001** ("every
state has exactly one owner") applied to cognitive truth, and of the proposed
**G-001** ("the Kernel is the sole commit gateway"). *Proves:* no duplicate/forked
truth record for a content in a shard.

### 4.2 `ORCH_004` — idempotent, content-addressable commit (safety, algebraic)
```
ORCH_004 == /\ Range(log) = truth
            /\ \A c \in Content : Count(log, c) <= 1
```
This pins the **idempotency algebra** P05-4 said was missing, and pins it
**observationally** (the design P05-4 recommended): the property speaks about the
*resolved truth* and the *log-append count*, never about the `Result` variant.
Concretely, `commit(x) ; commit(x)` yields (i) the same membership in `truth`
(at most one truth per `(shard, content)` — structural, since `truth` is a set),
and (ii) **no second log append** (`Count(log, c) <= 1`). An implementation may
surface the duplicate as `Ok(existing)` or as a typed `AlreadyCommitted`; both
satisfy this property, so a conformance check written against it cannot spuriously
reject a legitimate variant. *Proves:* re-proposal of an already-committed content
adds no log record and maps to existing truth — safe retry by construction, the
bridge to distribution (ORCH-004's stated purpose).

### 4.3 `ORCH_003_ReplayEquiv` — replay-equivalence (safety, refinement)
```
ORCH_003_ReplayEquiv == Range(log) = truth      \* Range(log) is the abstract replay fold
```
Truth reconstructed **from the log** equals the live committed truth in every
reachable state. This is the machine-checked form of **ORCH-003(b)** — the
replay-equivalence obligation that P05-2 separated out of the conflated
`ORCH-003`. In the reference kernel the concrete witness is
`truth_hash()`-before == `truth_hash()`-after-`replay()`; here the witness is set
equality of the replay fold with `truth`. *Proves:* replay is not a recomputation
that happens to match — the log is a *sufficient* and *faithful* record of truth,
which is the precondition for audit, debugging, and cross-runtime replay.

> **Scope note on ORCH-003.** P05-2 decomposes `ORCH-003` into **003a**
> replay-determinism, **003b** replay-equivalence, and **003c** trace-completeness.
> This module formalizes **003b** (and, because `truth` and `log` move in
> lockstep and `Commit` is deterministic in its effect, it also witnesses a
> degenerate form of **003a**). **003c** (a "sealed replay" that traps
> clock/RNG/IO) is a *runtime* property, not expressible in this state machine;
> it stays a `proptest`/sealed-harness deliverable in `verification/runtime-verification/`.

### 4.4 `EventuallyCommitted` — liveness under weak fairness
```
Fairness           == \A c \in Content : WF_vars(Commit(c))
EventuallyCommitted == \A c \in Content : (pc[c] = "proposed") ~> (c \in truth)
```
Under weak fairness on `Commit`, **a committed proposal is eventually reflected in
truth** — once a content is proposed and `Commit(c)` stays enabled, it eventually
fires and `c` becomes a member of `truth`. This is the **first liveness property
stated for any ARVES behaviour** (P05-6 found none existed anywhere), stated in the
standard TLA+ way — a temporal `~>` (leads-to) under an explicit `WF` fairness
assumption. It is the abstract, single-node ancestor of the distributed
"commit-completes" liveness (P05-6, IDR-001) that a later `ARVES_Consensus.tla`
must prove under quorum and stable-leader fairness. *Proves:* the gateway makes
progress — it does not deadlock/livelock a proposed write forever (safety alone
would be vacuously satisfied by a kernel that commits nothing).

## 5. How to check it (exact commands)

The module is **standalone TLA+** — it needs only TLC (or Apalache). Java and a
TLA+ tool are the only prerequisites (neither ships in this repo; install
locally). All commands are run from `verification/model-checking/`.

### 5.1 TLC (reference model checker)
Obtain `tla2tools.jar` (the TLA+ tools; the TLA+ Toolbox bundles it, or download
the standalone jar), then:

```sh
# From verification/model-checking/
java -XX:+UseParallelGC -cp tla2tools.jar tlc2.TLC \
     -config ARVES_Kernel_MC.cfg ARVES_Kernel.tla
```

Expected result: TLC reports **no errors** — every reachable state satisfies
`SafetyInv` and the temporal property `EventuallyCommitted` holds. The reachable
state space for `Content = {c1, c2}` is tiny (a few tens of states after symmetry
reduction) and completes in well under a second. To check the invariants
individually (sharper diagnostics), replace `INVARIANT SafetyInv` in the `.cfg`
with the four lines `INVARIANT TypeOK`, `INVARIANT OWN_001`, `INVARIANT ORCH_004`,
`INVARIANT ORCH_003_ReplayEquiv`.

To *demonstrate* the model has teeth (falsifiability, Reference Lifecycle Part 4),
temporarily break the idempotent branch (make `Commit` always `Append(log, c)`):
TLC then reports a violation of `OWN_001`/`ORCH_004` with a minimal
counterexample trace (propose c1, commit c1, propose … — the second append). Revert
after confirming.

### 5.2 Apalache (symbolic / SMT alternative — optional)
Apalache checks the same invariants symbolically (useful as an independent
oracle and for larger `Content`). It needs light type annotations; the safety
core translates directly:

```sh
# Safety only (Apalache does not check `~>` liveness):
apalache-mc check --inv=SafetyInv --length=10 ARVES_Kernel.tla
```

For liveness, TLC is the tool of record here.

## 6. What is abstracted away (stated honestly)

This model is **evidence for the single-shard kernel core, and nothing more**. A
reviewer must not read it as covering the distributed system. Omitted, by design:

- **Distribution / Raft.** No leader election, no replication, no followers, no
  quorum, no terms, no `NotLeader`/`NotReplicated`. The frozen truth path is
  *per-shard Raft* (IDR-001..005); the hard consensus safety properties
  (single-leader-per-term, log-matching, state-machine-safety, leader-completeness)
  and the distributed liveness (election terminates, commit completes under
  quorum) belong to a **later, larger `ARVES_Consensus.tla`** in milestone I2.
  This module models the commit gateway *as if the log were already the agreed,
  replicated log* — i.e. the state-machine layer above consensus.
- **Crash / recovery.** No modelling of snapshot + tail-replay, compaction, or the
  "lossless or loud" recovery of `arves-kernel` (`try_replay`, `RecoveryError`).
  The abstract `Range(log)` replay is the *idealized* fold; the runtime's
  fault-injection recovery tests remain the evidence for the durable path.
- **Cross-shard sagas.** Single shard only. Atomicity-modulo-compensation
  (P05-9 / R-12, proposed SAGA-001) is out of scope and unmodelled.
- **Ordered truth digest.** The runtime's `truth_hash()` is an **order-sensitive**
  FNV-1a-64 fold over commit order; this module abstracts `truth` to an **unordered
  set** (`Range(log)`), which is sufficient to prove single-writer, idempotency,
  and replay-equivalence *of membership*, but does not by itself prove the
  runtime's *order-sensitive* digest is reproduced. That order-equivalence is
  pinned instead, byte-exactly, by the runtime conformance scenario **CS-1** (§7),
  which is why CS-1 includes a reordered vector that yields a *different* digest.
- **Content-address computation.** The model treats content as opaque tokens
  `c1, c2`. The bytes those tokens stand for, and the SHA-256 that derives them,
  are ACS-001/CCP-001's concern; CS-1 (§7) binds the tokens to real ACS-001
  ContentIds so the model and the runtime share one identity notion.
- **Engines, graph expansion, arbitration, policies.** Entirely out of scope
  (Control-Plane concerns; ORCH-003c trace-completeness and graph-termination are
  separate deliverables, P05-2/P05-10).

## 7. Conformance scenario (CCP-GATE requirement) — `AKF-CS-1`

CCP-GATE requires the draft **plus at least one conformance scenario**. `AKF-CS-1`
ties the **abstract model** to the **reference kernel** through **byte-exact**
values, so "the model checks" and "the runtime behaves" are the same claim, not
two.

**Setup.** One shard `acme/research`. Two contents, whose abstract model tokens
`c1, c2` denote concrete **ACS-001/CCP-001** ContentIds (domain tag `0x01` =
commit-content; `ContentId = 0x1220 || SHA256(domain_tag || body)`). These bytes
were computed with Python `hashlib` (SHA-256) and are reproducible:

| token | body (utf-8) | pre-image (hex) | ContentId (hex) |
|---|---|---|---|
| `c1` | `truth-alpha` | `0174727574682d616c706861` | `12206623c3d81c6f9a6ecf04ee9d474ffbcb31e29bb45ce01070d6bef20506d63f10` |
| `c2` | `truth-beta`  | `0174727574682d62657461`   | `1220095e3f1504ab8cee6c3b52ad1344d46e21d3ae4cf527cf119308034bf4345a34` |

**Sequence (the trace the model and the runtime both realize):**
1. `Propose(c1)`, `Commit(c1)` → log `<<c1>>`, truth `{c1}`, `c1@index 0`.
2. `Propose(c2)`, `Commit(c2)` → log `<<c1, c2>>`, truth `{c1, c2}`, `c2@index 1`.
3. `Propose(c1)` again, `Commit(c1)` → **ORCH-004 idempotent**: log **unchanged**
   `<<c1, c2>>`, truth **unchanged** `{c1, c2}`. In the runtime, `commit()` returns
   the existing `TruthRef` (surfaced as `AlreadyCommitted`) and appends **nothing**.

**A conformant implementation SHALL exhibit all of:**

- **(OWN-001)** `Count(log, c1) = 1` and `Count(log, c2) = 1` — no duplicate record
  despite the re-proposal in step 3. `committed_count() == 2`.
- **(ORCH-004)** step 3 appends **zero** log records and resolves to the step-1
  `TruthRef` (`content = c1`, `index = 0`). The observable outcome MAY be
  `Ok(existing)` or `AlreadyCommitted(existing)`; both are conformant.
- **(ORCH-003b, replay-equivalence)** replaying the log `<<c1, c2>>` reconstructs
  truth `{c1, c2}`. The reference kernel's **order-sensitive** digest of the
  committed truth set, `truth_hash()`, over `[(acme, research, c1, 0, c1), (acme,
  research, c2, 1, c2)]` — where each payload equals the 34-byte ContentId — SHALL
  be exactly:

  ```
  truth_hash = 0x7bb9b2e30ee7427c   (decimal 8915353625633964668)
  ```

  and SHALL be **identical before and after** `replay()`, and **identical after**
  the idempotent re-commit of step 3.

- **(order matters — why the SET abstraction is not the whole story)** the *same
  two truths in the opposite commit order* `[c2@0, c1@1]` yield a **different**
  digest:

  ```
  truth_hash(reordered) = 0xed1b740f02f0b8f4
  ```

  A runtime whose replay does not preserve log order would produce this value and
  is therefore **non-conformant**. (The abstract model's set-equality invariant is
  necessary but not sufficient for this; CS-1 supplies the byte-exact order check
  the model omits — see §6.)

**Model side of CS-1.** Running TLC with `Content = {c1, c2}` (§5) explores every
interleaving of the two proposals/commits, including step 3's idempotent
re-commit, and reports `SafetyInv` holding in all reachable states and
`EventuallyCommitted` holding — i.e. the model *proves*, over all interleavings,
the properties CS-1 asserts for this one trace. An implementation that produces a
different `truth_hash` for the canonical order, or that appends a second record in
step 3, or that fails to reflect a proposed commit in truth, SHALL be
non-conformant.

> **Reproduce the vectors.** `sha256(0x01 || "truth-alpha") = 6623c3d8…`,
> `sha256(0x01 || "truth-beta") = 095e3f15…`; `truth_hash` is the FNV-1a-64 fold
> (offset basis `0xcbf29ce484222325`, prime `0x100000001b3`) over
> `tenant ‖ workspace ‖ content ‖ index.to_le_bytes(8) ‖ payload` per committed
> truth, exactly as `arves-kernel/src/lib.rs` computes it. Any language's SHA-256
> and a 15-line FNV-1a-64 reproduce all four values.

## 8. Proposal analysis

- **Why it matters.** P05-1 is the single biggest ISO/IEEE gap: ARVES had precise
  *prose* and a precise *implementation* but nothing in between — no formal object
  a certifier can diff or re-check. This artifact is the first such object for the
  truth core. It converts three headline claims ("single writer", "idempotent",
  "replayable") from unfalsifiable prose into properties a machine rejects when
  violated, and it states the first ARVES liveness property. It is the anchor eight
  of the ten P05 findings reference.
- **Risks.** (a) **Abstraction gap** — a model this small can lull reviewers into
  thinking the *distributed* system is verified; §6 states bluntly that it is not,
  and the artifact is scoped as the single-shard core only. (b) **Model/runtime
  drift** — the model could diverge from `arves-kernel` over time; CS-1's
  byte-exact vectors are the tripwire (the runtime's `truth_hash` conformance test
  fails if either side drifts). (c) **Set-vs-order abstraction** — modelling
  `truth` as a set does not capture the runtime's order-sensitive digest; mitigated
  by CS-1's explicit reordered-vector check. (d) **Liveness cost** — liveness
  checking is state-space expensive; kept tractable by the two-token model and
  symmetry reduction.
- **Long-term consequences.** A publicly model-checked kernel core is *citable by
  construction*: downstream safety cases (embodied/robotics Vol 8, regulated
  enterprise Vol 17) and any ISO submission can reference the property and its TLC
  run rather than an author's assurance. It also sets the pattern — abstract state
  machine + byte-exact conformance scenario — that the harder I2 consensus model
  and the saga model (R-12) will follow.
- **Alternative designs considered.**
  - *(A) TLA+ + TLC (chosen).* Best fit: the eventual per-shard Raft is the most
    model-checked protocol in TLA+, so this core composes upward; TLC checks both
    safety and liveness; the notation is standard and vendor-neutral. Downside:
    finite-state, so it checks *instances*, not all `Content`.
  - *(B) Coq/Isabelle mechanized proof (Verdi-Raft style).* Maximum assurance and
    all-cardinality, but very-high cost and over-engineered for a core whose only
    non-determinism is the *engines* (irreducibly non-deterministic — full
    functional correctness there is not even meaningful). Rejected for the core;
    reconsider for a consensus refinement proof later.
  - *(C) Alloy structural model.* Cheap and excellent for the *structural*
    invariants (LAYER-001 acyclicity, OWN-001 single-writer, SHARD-001
    disjointness) but weak for temporal/liveness. Recommended as a *complement*
    (P05-8's cheap static gate), not the backbone.
  - *(D) Apalache only.* Great symbolic safety oracle and all-cardinality-bounded,
    but does not check `~>` liveness; kept as the optional second oracle (§5.2).
  - *(E) Do nothing / keep prose.* Rejected — violates the corpus's own Reference
    Lifecycle Part 4 ("unfalsifiable claims do not advance") and forfeits R-05.
- **Recommendation.** Land this as the first `verification/model-checking/`
  artifact; wire the TLC run into CI as a required check; treat CS-1 as a CCP-GATE
  scenario and cross-link it to the `arves-kernel` `truth_hash` conformance test so
  model and runtime cannot drift silently. Sequence the larger `ARVES_Consensus.tla`
  (IDR-001..005 safety + commit/election liveness) next in I2, refining this
  gateway's abstract log into a replicated Raft log.
- **Implementation complexity.** The model + config: **low** (a few dozen lines,
  runs in under a second). The discipline it introduces (a TLA+ skill on the team,
  CI integration, and the eventual consensus/saga models): **medium→high**, but
  front-loaded and far cheaper than retrofitting formal methods later.
- **Scientific impact.** A publicly model-checked cognitive-infrastructure kernel
  is rare; the *pattern* — deterministic, content-addressed, replayable truth core
  formalized as a small TLA+ machine with a byte-exact conformance bridge to the
  reference runtime — is itself a reusable methodological contribution, and the
  precondition for the eventual "deterministic replay over per-shard Raft with
  non-deterministic compute quarantined" result P05 flagged as novel.
- **Ecosystem impact.** The formal core plus CS-1's byte-exact vectors is what a
  second independent runtime and any third-party certifier can check against
  without trusting the reference team — the precondition for a real certification
  market and cross-runtime interop.

## 9. Dependencies & sequence

- **Depends on:** **ACS-001/CCP-001** (Content Addressing) for the concrete
  ContentIds that CS-1's tokens denote; the reference kernel
  `arves-kernel/src/lib.rs` (`commit`, `try_replay`, `truth_hash`) for grounding.
- **Contributes to:** Global Readiness **R-05** (the TLA+ invariant-core half),
  and answers **P05-1** (formal object now exists for the truth core), **P05-2**
  (ORCH-003b isolated and checked), **P05-4** (idempotency stated observationally),
  **P05-6** (first liveness property).
- **Blocks / precedes:** `ARVES_Consensus.tla` (I2 — refines the abstract log into
  per-shard Raft; adds IDR-001..005 safety and distributed liveness); the saga
  model (R-12 / proposed SAGA-001); the `proptest` sealed-replay harness for
  **ORCH-003c** trace-completeness (`verification/runtime-verification/`); the
  static **LAYER-001/OWN-001** architecture gate (P05-8, the cheapest win).

---

*Evidence path: `verification/model-checking/ARVES_Kernel_Formal_Spec.md`
(module `ARVES_Kernel.tla`, instance `ARVES_Kernel_MC.cfg`). Ratification path
(Reference Lifecycle): DRAFT evidence → CCP-GATE (this document + `AKF-CS-1`) →
Candidate → Ratified evidence. The frozen v1.0 corpus is unchanged (ED-001); this
artifact is additive verification evidence, not normative specification.*

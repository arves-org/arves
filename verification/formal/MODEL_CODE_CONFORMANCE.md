# MODEL ↔ CODE Conformance — the running witness that links the checked TLA+ models to the reference Rust

**Type:** Verification evidence (LIVING, under `verification/`). NOT normative
spec; edits no frozen `.docx` (ED-001). **RCR:** RCR-040.
**Test:** `runtime/crates/arves-kernel/tests/model_code_conformance.rs`
(3 tests, deterministic, part of `cargo test --workspace`).

---

## 1. The gap this closes

`verification/formal/` model-checks the **design**: TLC exhausts a small
abstract state machine and proves the safety (and, for the kernel core,
liveness) invariants hold over **all interleavings**. But until now nothing tied
that proof to the **running Rust code** — the two were disconnected claims. A
certifier could re-run TLC and separately read the Rust, but no artifact asserted
*"the code exhibits, on a concrete trace, exactly what the model proves."* That
is the last thing capping the Verification dimension below its honest ceiling.

This artifact is the missing link: a Rust test that **drives the real reference
code** — through the kernel's named byte-exact scenario **AKF-CS-1**, and, for
the cluster, a concrete trace that **mirrors the model's actions** (TLC itself
exhausts all interleavings; it does not enumerate this one trace) — and asserts
the code's **observable state** matches each model invariant at each step.

## 2. HONEST SCOPE — what this IS and IS NOT

| | |
|---|---|
| **IS** | A **concrete-scenario conformance witness**. For ONE trace per model, every checked model invariant is mapped to a Rust assertion on the real code's observable state. "The model checks" and "the code behaves" become **linked** claims. |
| **IS NOT** | A **refinement proof** / **formal code proof**. It does not show the Rust refines the TLA+ over ALL states/interleavings. TLC still owns the all-interleavings argument; this test owns the one-trace bridge. |
| **Open Question (RCR-040 OQ-1)** | Full model-to-code refinement needs code-level formal tooling (Kani for the Rust, or TLA+ trace-validation / `tlc` state export compared against a code trace). Recorded, not claimed. |

The witness is therefore **necessary but not sufficient** for full refinement —
exactly as AKF-CS-1's byte-exact vectors are necessary-but-not-sufficient for
the set abstraction (ARVES_Kernel_Formal_Spec.md §6). Its value: the model and
the code can no longer **drift silently** — a change to either that breaks the
mapped relationship fails this test.

---

## 3. Part 1 — `ARVES_Kernel.tla` ↔ `RefKernel` (scenario AKF-CS-1)

Test: `akf_cs1_kernel_model_conformance_witness`. Drives the byte-exact AKF-CS-1
trace (ARVES_Kernel_Formal_Spec.md §7) on the real `RefKernel<MemWalStore>`.
ContentIds are recomputed from the runtime's OWN SHA-256 (`arves-acs::content_id`,
zero new deps) — if either the doc's vectors or the kernel's `truth_hash` fold
drifted, the test fails (the anti-drift tripwire of §8 Risk (b)).

| Model invariant (TLA+) | What it requires | Rust assertion witnessing it on the running code |
|---|---|---|
| `OWN_001` (§4.1) `\A c: Count(log,c) <= 1` | single writer; no forked/duplicate truth record | after committing c1, c2 and **re-committing c1**, `k.committed_count() == 2` (one record per content) |
| `ORCH_003_ReplayEquiv` (§4.3) `Range(log) = truth` | replay reconstructs truth faithfully from the log | `k.truth_hash()` **before** == `RefKernel::recover(store).truth_hash()` **after replay**, and both == byte-exact `0x7bb9b2e30ee7427c` |
| `ORCH_004` (§4.2) idempotent, content-addressable | re-proposal adds no log record, maps to existing truth | step-3 re-commit → `Err(AlreadyCommitted(tr1))`; `committed_count()` stays `2`; `truth_hash()` unchanged |
| §6/§7 order-sensitivity (the set abstraction's limit) | the SAME truths in opposite order ≠ same digest | reordered kernel (`c2@0, c1@1`) → `truth_hash() == 0xed1b740f02f0b8f4` ≠ canonical |

The ContentId vectors themselves are asserted equal to the §7 table
(`1220 6623c3d8…`, `1220 095e3f15…`).

---

## 4. Part 2 — `ARVES_Cluster.tla` ↔ `ClusterKernel`/`ClusterSim`

Test: `cluster_model_conformance_witness`. A 3-node cluster (`Server = {n1,n2,n3}`,
matching `ARVES_Cluster_MC.cfg`) driven through a concrete trace whose steps
mirror the model's actions:

```
elect (Timeout→Vote→BecomeLeader)  →  commit e1,e2 (ClientRequest)
→ settle (Replicate + AdvanceCommit)  →  isolate leader (minority partition)
→ commit e3 fails NotReplicated (CP)  →  heal  →  successor elected at a HIGHER term
→ commit e3 through the new leader (fresh)  →  converge
```

The witnesses read observable consensus **mechanism** metadata (terms/indices,
via RCR-040's read-only `log_terms_of` / `leaders_by_term_of` /
`committed_terms_of`) and Kernel **truth** (byte-identical `shard_state_of`,
`truth_hash_of`) — never coupling to internal Kernel state (ORCH-001).

| Model invariant (TLA+) | What it requires | Rust assertion witnessing it on the running code |
|---|---|---|
| `ElectionSafety` | ≤ 1 leader per term (IDR-004) | `assert_election_safety`: every term in `leaders_by_term_of(shard)` has ≤ 1 leader (checked after election, after commit, after failover) |
| `LogMatching` | shared `(index,term)` ⇒ shared prefix | `assert_log_matching`: pairwise over `log_terms_of`, where two logs agree at an index their whole preceding term-prefix agrees |
| `StateMachineSafety` | no two DIFFERENT entries committed at one index | `assert_state_machine_and_linearizable`: each replica's committed prefix (`log_terms_of` up to `commit_index_of`) equals the single-valued `committed_terms_of` history |
| `LeaderCompleteness` | a later-term leader holds every earlier-committed entry | after the higher-term successor emerges, its `log_terms_of` reproduces the pre-failover committed `(idx,term)` prefix captured before the fault |
| `LinearizableCommit` | committed prefixes of any two replicas agree | truth-level consequence: byte-identical `shard_state_of`, equal `truth_hash_of`, equal `committed_count_of` across **all** replicas (ORCH-003) |

Additional witnessed facts on the trace: the failover raises the term
(`new_term > old_term`) and deposes the old leader; the minority commit yields
`NotReplicated` with **zero partial truth** anywhere (IDR-001 CP / A-005/A-006);
the post-heal commit is FRESH and exactly-once (`committed_count == 3` on every
replica); and an idempotent re-proposal resolves to the same `TruthRef`
(ORCH-004 under replication — the bridge back to Part 1's kernel model).

A third test, `cluster_conformance_witness_is_deterministic`, asserts the whole
witness is a pure function of its seed (identical truth digests on every replica
across two runs) — the witness is only meaningful if replayable (ORCH-003).

---

## 5. Why the mapping is sound (not a coincidence)

- The model's `committed` history is **the same object** the runtime's own
  safety observer (`arves-consensus::sim::observe_all`) maintains and panics on;
  `committed_terms_of` exposes it read-only. StateMachineSafety at code level is
  thus not merely "the trace happened to agree" — the observer would have
  aborted the run at the first divergent commit.
- `LinearizableCommit`/`StateMachineSafety` in the model guarantee **byte-identical
  follower truth**; the runtime realizes that as identical `shard_state_of` bytes
  and `truth_hash_of` — the strongest observable form (a diverged replica fails).
- `LeaderCompleteness` is witnessed on a trace that **actually changes terms**
  (isolate → heal → higher-term successor), so the assertion has teeth: a runtime
  that erased committed truth on failover would fail the pre/post prefix check.

## 6. Falsifiability (this witness has teeth)

Each mapped assertion fails if the code regresses: break follower apply order and
`assert_state_machine_and_linearizable` fails; let a stale leader double-commit a
term and `assert_election_safety` fails; drop the §5.4.1 vote restriction so a
new leader loses committed entries and the LeaderCompleteness prefix check fails;
change the `truth_hash` fold and AKF-CS-1's byte-exact check fails. The models'
OWN falsifiability probes (ARVES_Cluster.tla PROBE 1/2, ARVES_Kernel §5) remain
the model-side teeth; this file is the code-side complement.

---

*Evidence path: `verification/formal/MODEL_CODE_CONFORMANCE.md`. Witness:
`runtime/crates/arves-kernel/tests/model_code_conformance.rs`. Models:
`ARVES_Kernel.tla` (+ `_MC.cfg`), `ARVES_Cluster.tla` (+ `_MC.cfg`). RCR-040.
The frozen v1.0 corpus is unchanged (ED-001); this is additive verification
evidence, not normative specification.*

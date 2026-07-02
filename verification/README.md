# ARVES verification/ — EVIDENCE, not specification

Machine-checkable proofs, formal semantics, model checks, executable gates,
conformance, benchmarks, and independent-implementation evidence that
substantiate the frozen ARVES specification's claims. This tree is part of the
**Living Engineering Repository** (doctrine ED-001): it *proves* the frozen spec;
it is never itself frozen spec.

Structure (four offices' evidence):

- **formal/** — TLA+ modules, temporal-logic properties, state-machine models,
  denotational/operational semantics, and hand/mechanized proofs. (Verification
  Office → Formal Foundation.)
- **runtime/** — executable evidence bound to the reference runtime: build-time
  architecture gates (LAYER-001/OWN-001), behaviour proofs, replay, runtime
  invariant checks. (Verification Office → runtime evidence.)
- **certification/** — conformance scenarios, divergence detection, and the
  L1..L4 certification kit + verdicts. (Certification Office.)
- **independent/** — independent runtime implementations (A/B/C) and
  cross-runtime convergence evidence (the ultimate "is it a standard?" proof).

Governed by `runtime/docs/ENGINEERING_DOCTRINE.md` (ED-001..004) and the
`ARVES_Master_Roadmap.md` Verification Program.

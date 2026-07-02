# PMO Backlog #002 — post-Certification / Lock-Review

**Supersedes** #001's top-3 (all executed). Produced from the `certify-and-lock`
verdict (arms-length) + the ACS batch state. **Honest gate state below.**

## Verdicts (this cycle)
- **L1 (Core Runtime): GRANTED-with-conditions.** All L1 dimensions PASS or
  documented-non-blocking PARTIAL; NO registered-invariant FAIL; Distribution
  N/A-at-L1. Independent reviewers re-ran `cargo test` (workspace + architecture
  gate) and confirmed. Record: `verification/certification/L1_Attestation_and_Standard_Lock_Review.md`.
- **Standard Lock Review: CONDITIONAL** (not YES today; not NO). Byte-exact
  identity surface is real (a source-blind team reproduced ACS-001/002/003 vectors
  28/28), but binding gaps remain → **I2 stays GATED.**

## Done this cycle
- ✅ Interop batch ACS-001..005 drafted (real vectors).
- ✅ **G2 fixed** — the ACS-005 `0x06`/`0x07` collision the consistency report
  missed (caught by the Lock Review) is resolved: ACS-005 → `0x08`/`0x09`, vectors
  recomputed; ACS-001 registry authoritative for 0x01–0x09.
- ✅ L1 attestation recorded.

## Path to Lock = YES  →  then I2 (top of backlog)

| # | Task | Gap | Instrument | ROI | Blocked by |
|---|------|-----|-----------|-----|-----------|
| 1 | **Ratify ACS-001..005 via CCP-GATE** (Draft→Candidate→Ratified) — now that all tag collisions are fixed | G1 | CCP-GATE | 10 | — |
| 2 | **Build the differential/divergence harness + golden-vector corpus** (a Rust test asserting every published ContentId/body) AND **adopt ACS in `arves-kernel`/`arves-ontology`** (dCBOR codec + multihash `SHA256(0x01‖body)` + ACS-004 validator; typed-instance→ProposedWrite) | G3+G4 | Verification + Runtime | 10 | #1 |
| 3 | **Decision-Trace / Runtime-Fingerprint standard** (ACS-006/009; R-04) for cross-runtime replay | G6 | ACS/CCP | 8 | #1 |
| 4 | Add decoder **rejection vectors** + ACS-001 varint / double-hash-wording / Unicode-version fixes | G5 | CCP | 7 | #1 |
| 5 | Populate the **full type system** (~17 remaining `uci.*` roots + aspects/relations + Event payloads); reconcile ACS-004 roots vs Ontology Table 2; EntityUrn grammar + `validate()` | G7+G8 | ACS/CCP | 7 | #1 |
| 6 | Open **IDRs for the L1 conditions before I2**: F2 (Kernel mints consensus-owned `term:0`), F3 (persistence chooses CRC-32 as content-address), F7 (Kernel-side reads outside Query), F6 (two decision-trace owners) | G9 | IDR | 8 | — |
| 7 | Harden verification depth: non-poisoning locks, fault-injection FS + crash-consistency checker, captured TLC artifact, fix TLA+ README path | — | Verification | 6 | — |
| 8 | **I2 Cluster Kernel (Replication)** | — | Runtime | 9 | Lock=YES (#1–#5) + #6 |

## TOP 3 to do now
1. **#6 (IDRs for the four L1 conditions)** — cheap, unblocks nothing else but must precede I2; do in parallel.
2. **#1 (ratify ACS via CCP-GATE)** — the batch is collision-free and vector-backed; ratification makes it bindable.
3. **#2 (differential harness + ACS adoption in the runtime)** — the single highest-leverage move: converts the paper standard into a *falsifiable, cross-testable* one and is the crux of Lock=YES.

**Note:** #2/#3/#5 are genuine multi-session engineering (a dCBOR codec, a
conformance harness, 17 type schemas). "Finish everything" for *this* cycle =
L1 attested + Lock verdict rendered honestly + the one live defect (G2) fixed +
the path enumerated. I2 is correctly gated behind Lock=YES.

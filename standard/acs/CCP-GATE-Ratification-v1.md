# CCP-GATE Ratification — ACS-001..005 (v1.1)

**Decision:** ACS-001, ACS-002, ACS-003, ACS-004, ACS-005 are **RATIFIED** as the ARVES v1.1
normative core standards, effective 2026-07, **at independence grade G1**.

**Instrument:** CCP-GATE (Reference Lifecycle, Part 6): `DRAFT → CCP-GATE (spec + conformance
scenario) → Candidate → Ratified`. This record is the governance act; it is not a silent edit.

## The gate, and the evidence it is met

CCP-GATE requires, for each standard, a complete normative specification **and** a conformance
scenario that reference implementations pass. For ACS-001..005:

| Requirement | Evidence |
|---|---|
| Complete normative spec (RFC 2119) | `standard/acs/ACS-001..005_*.md` (ACS-005 supplies the normative language) |
| Conformance scenario per standard | `ACS-001-CS-1 … ACS-005-CS-1` + golden/negative vectors (`standard/vectors/*.tsv`) |
| Reference impls pass it | **3 independent implementations** (Rust / Python / TypeScript) reproduce every golden ContentId byte-for-byte and reject every core negative with the matching reason |
| Cross-impl agreement | Rust↔Python **differential fuzz: 13,807 inputs, 0 divergences** |
| Runtime certification | `certify_runtime.py`: **2/2 runtimes certified under ONE conformance** |
| Pre-ratification defect sweep | the domain-tag collision (0x06/0x07) found + fixed during drafting (`ACS_Batch_1_Consistency_Report.md`); vectors recomputed and re-verified |

The gate is **met**: the specs are complete, each has a conformance scenario, and multiple
implementations agree. The ACS set moves from DRAFT to **Ratified**.

## Honest scope of this ratification (what it does and does NOT claim)

- **Two different axes, not conflated.** *Ratification* is the **spec-maturity** axis: the ACS
  specs are complete, conformance-backed, and stable — a byte-affecting change now requires a new
  **CCP Amendment / major version**, never a silent edit. *Independence* is a **separate** axis,
  graded G0/G1/G2.
- **Independence remains G1.** Every implementation that agrees today was produced **inside this
  program** (same-process). Ratification is the maintainer authority declaring the spec final and
  conformance-backed — it is **NOT** a claim of external/independent validation.
- **G2 is still the open exit gate** (unchanged): an unknown team, using ONLY the Kit, building a
  conformant runtime with no help. Ratifying the spec does not close G2; it defines the stable
  target that a G2 party implements against.

## Effect

- `standard/VERSION` and each ACS doc's `**Status:**` now read **RATIFIED v1.1 (G1)**.
- The golden + negative vectors are **normative** for v1.x; changes go through CCP Amendment.
- The independence ledger (`verification/evidence/`) is unchanged: **maximum G1; G2 NOT YET MET**.

*Ratified under ARVES 2.0 governance. Recorded in the living repository (ED-001); the frozen `.docx`
corpus is unchanged. Independence validation continues toward G2 — the real exit gate.*

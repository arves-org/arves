# CCP-GATE Ratification v2 — CCP-006: ACS-003/004/005 semantic reject codes + negative vectors

**Decision:** The **11 semantic reject reason codes** and **18 negative vectors** for the
ACS-003 (Canonical Envelope), ACS-004 (Universal Type Registry instance), and ACS-005
(Normative Language) reject surfaces are **RATIFIED** into the frozen Standard Kit, effective
2026-07-04, **at independence grade G1**. This is the ratification of draft **CCP-006**
(`verification/ccp-drafts/CCP-006-acs-reject-reason-codes-and-vectors.md`).

**Instrument:** CCP-GATE (Reference Lifecycle, Part 6) + ACS-001 §4.1 Reason-code registry
(*"new codes are added only via a CCP Amendment that also adds a negative conformance vector
exercising the code"*). This record is the governance act; it is **not** a silent edit.

**Authorization:** maintainer-approved (ARVES Chief Architect). Kit version **0.2.0 → 0.3.0**
(a vector-set addition — see "Scope" for why this is *not* a byte-affecting profile change).

## Closes

SYSTEM_GAP_ANALYSIS **#1 / #2 / #23** — before this ratification the certification gate's
"16/16 core-reject" corpus was **100 % ACS-002**; the ACS-003 envelope (429-line normative
spec), the ACS-004 instance-validity rules, and the ACS-005 term-set rules were normative
MUST/SHALL surfaces with **zero** negative vectors, so a runtime could skip all of them and
still certify. That was the single biggest hole under the "falsifiable differential
conformance" thesis. It is now closed at the **standard** level: the negative corpus exists,
is normative, is oracle-verified, and is exercised.

## The gate, and the evidence it is met

| Requirement (CCP-GATE + ACS-001 §4.1) | Evidence |
|---|---|
| Each new reason code ships with a negative vector exercising it | 11 codes ↔ 18 vectors, 1:1 coverage (`vectors/acs_negative_vectors.tsv`, tiers `envelope`/`instance`/`language`) |
| Every vector is a *pure semantic* defect (ACS-002 layer accepts it) | each `input_hex` **decodes cleanly** as canonical dCBOR, then the semantic validator rejects it — machine-checked |
| A reference validator that rejects them exists (implementable from the spec) | `verification/independent/python/acs003_envelope.py` · `acs004_instance.py` · `acs005_checker.py` |
| Oracle re-verification | `python verification/ccp-drafts/gen_candidate_vectors.py` → **18/18** decode-clean + rejected with the matching reason (exit 0) |
| Living exercise over the *frozen* vectors | `python verification/independent/python/conformance_semantic.py` → `envelope 7/7  instance 7/7  language 4/4 REJECTED` |
| Drift-proof | wired into `evidence_probe.py` (`--check` gate) as row `acs-semantic-reject` |
| Freeze re-baselined | `freeze_check.py update` re-manifested the changed `standard/` files; `freeze_check.py` = 0 drift |

## Scope — what this ratifies, and what it explicitly does NOT

- **Vector-set addition, not a profile change.** No ACS-001..005 *encoding* is altered. Not a
  single golden ContentId or canonical byte changes. This is purely the *negative* corpus
  growing, which is why it is a minor Kit bump (0.3.0), not a new ACS profile.
- **The frozen Rust v1.0 reference DEFERS the semantic tiers.** The reference runtime
  implements the ACS-001 addresser + ACS-002 decoder only; it has no ACS-003/004/005
  validators, so — exactly like the `nfc` tier it already defers — it **declares** the
  `envelope`/`instance`/`language` tiers deferred rather than failing them. Native Rust
  validators are tracked as **RCR-004** (a runtime v1.1 change; the Rust `arves-conformance`
  crate is byte-frozen at v1.0 and is **not** touched by this CCP).
- **Discovery folded in (corrects CCP-006 §6).** The draft said "append the 18 vectors into
  `acs_negative_vectors.tsv`" as if mechanical. In ratification we found the frozen TSV is the
  *generated output* of the frozen Rust `arves-conformance` crate, and that three living
  Kit-only runners (`conformance_negative.py`, `typescript/conformance.mjs`,
  `reference-runner/run.mjs`) consume it — and that the pure-ACS-002 decoders among them would
  mis-handle a semantic row (which decodes fine). The ratification therefore also made every
  consumer **tier-aware** (a living, freeze-clean change): pure ACS-002 decoders now DECLARE
  the semantic tiers deferred; the semantic tiers are exercised by the dedicated validators.
  No frozen `runtime/` byte changed.
- **Independence is unchanged — G1.** The reference validators that reject these vectors were
  authored inside this program. Ratifying them strengthens the standard's *self-conformance*;
  it does **not** manufacture G2. A genuine external runtime rejecting these vectors from the
  Kit alone would be the G2 evidence — this ratification makes that surface *exist to be
  tested*. **G2 remains the open exit gate.**

## Effect

- `vectors/acs_negative_vectors.tsv`: 17 → **35 rows** (adds 7 envelope + 7 instance + 4 language).
- `conformance/CONFORMANCE.md`: the semantic tiers + the 11 codes are documented and normative.
- `acs/ACS-001_Content_Addressing.md` §4.1: the reason-code registry now records both bands.
- `VERSION`: `arves-standard-kit 0.3.0`.
- The independence ledger (`verification/evidence/`) is unchanged: **maximum G1; G2 NOT YET MET**.

*Ratified under ARVES 2.0 governance. Recorded in the living repository (ED-001); the frozen
`.docx` corpus is unchanged. RCR-004 (native Rust ACS-003/004/005 validators) is the tracked
follow-on; independence validation continues toward G2 — the real exit gate.*

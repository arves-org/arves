# CCP-006 — ACS-003/004/005 reject reason codes + negative vectors

**Status:** **✅ RATIFIED (2026-07-04)** via `standard/acs/CCP-GATE-Ratification-v2.md`. The 11
codes + 18 vectors are now in the frozen Kit (`arves-standard-kit 0.3.0`); freeze re-baselined
to 150 files, 0 drift. This draft is retained as the design record. **Two corrections the
ratification made to this draft:** (1) §6 called the TSV append "mechanical" — in fact the frozen
TSV is the *generated output* of the frozen Rust `arves-conformance` crate and is consumed by
three living Kit-only runners, so ratification also made every consumer **tier-aware** (living,
freeze-clean) rather than a bare append; (2) the frozen Rust v1.0 reference cannot reject these
(they are valid ACS-002), so the semantic tiers are **declared-deferred** by the reference (like
`nfc`) and exercised by the living Python validators (`conformance_semantic.py`) — native Rust
validators are tracked as **RCR-004**. Original draft header below (historical).

---

**Status (original draft):** **DRAFT proposal — NOT ratified, NOT applied.** Freeze-clean: this
document and its oracle-verified candidate vectors live under `verification/ccp-drafts/` (living).
**Nothing here edits the frozen `standard/`.** Ratification (which *does* touch frozen material)
is a separate, maintainer-authorized **CCP-GATE** step — see §6.

**Instrument:** CCP Amendment, mandated by **ACS-001 §4.1**: *"new [reason] codes are added only
via a CCP Amendment that also adds a negative conformance vector exercising the code."* This
draft supplies both halves (codes + vectors) so the gate has something concrete to ratify.

**Closes (on ratification):** SYSTEM_GAP_ANALYSIS **#1 / #2 / #23** — the certification gate's
"16/16 core-reject" is **100% ACS-002**; the ACS-003 envelope, ACS-004 instance, and ACS-005
language **reject surfaces are normative MUST/SHALL but have ZERO negative vectors**, so a runtime
that skips all of them still certifies. This is the single biggest hole under the "falsifiable
differential conformance" thesis.

## 1. Why this is a CCP and not Kit-packaging

The frozen reason-code registry (`conformance/CONFORMANCE.md`, closed at 13 ACS-002 canonical-form
codes) has **no code** for a semantic reject (missing field, wrong type, malformed ContentId,
cardinality, provenance). A negative vector needs a stable `reject_reason`; a stable reason code
is a **normative registry addition**; ACS-001 §4.1 says that is CCP-only. (The earlier triage that
called this "Kit-packaging, no CCP" was wrong — corrected in G2_READINESS.md §2a.)

## 2. Proposed reject reason codes (registry extension)

Eleven new codes, kebab-case to match the existing ACS-002 vocabulary. `non-nfc-text` (already
registered) is reused for non-NFC ACS-005 bodies.

| Proposed code | Standard | Rule (normative source) |
|---|---|---|
| `missing-required-field` | ACS-003 §6.3, ACS-004 §6.5.2 | a REQUIRED field is absent |
| `unknown-field` | ACS-003 §5/§6.3, ACS-004 §6.5.5 | a key outside the closed field set |
| `field-type-mismatch` | ACS-003 §6.3, ACS-004 §6.3/§7.1/§5.1 | value's kind violates the field's declared type/carrier |
| `value-out-of-range` | ACS-004 §7.2 | right kind, out of range (`conf`∉[0,1], `u32`, registered `int` range) |
| `malformed-content-id` | ACS-003 §6.3 / ACS-001 §5 | `payload_cid` not a 34-byte `0x12 0x20 ‖ SHA-256` multihash |
| `empty-shard-scope` | ACS-003 §6.3 (SHARD-001) | `tenant_id`/`workspace_id` null or empty |
| `cardinality-violation` | ACS-004 §6.4/§6.5.4 | field violates its cardinality (scalar-as-array, array-as-scalar, empty `1..*`) |
| `provenance-invariant` | ACS-004 §8 | `invocation` present iff `origin == "derived"` violated |
| `terms-not-sorted` | ACS-005 §8/§11 | list entries not strictly ascending |
| `duplicate-term` | ACS-005 §8/§11/§5 | duplicate list entry |
| `malformed-term-list` | ACS-005 §8/§9.2/§11 | leading/trailing/blank LF, or an entry failing the ID/term grammar |

**Proposed `tier` values** (extending the existing `core`/`nfc`): `envelope` (ACS-003),
`instance` (ACS-004), `language` (ACS-005). The certification verdict would count these tiers
alongside `core`.

## 3. Candidate negative vectors (18, oracle-verified)

`candidate_negative_vectors.tsv` (same columns as the frozen `acs_negative_vectors.tsv`:
`standard  case  tier  input_hex  reject_reason`). **7 envelope + 7 instance + 4 language.**

Every row is machine-verified by `gen_candidate_vectors.py` against the living reference
validators (the **oracle**): each defect (a) is built from a VALID structure with exactly ONE
mutation, (b) **encodes to canonical dCBOR that `decode()` ACCEPTS** — so the ACS-002 layer passes
and the defect is purely the semantic reject the current gate never exercises — and (c) is
**REJECTED** by `acs003_envelope.py` / `acs004_instance.py` / `acs005_checker.py`. The generator
exits non-zero unless all 18 confirm, so the TSV is oracle-guaranteed, not hand-authored.

Regenerate + re-verify: `python verification/ccp-drafts/gen_candidate_vectors.py`.

## 4. Harness wiring (on ratification)

`certify_runtime.py` / `CONFORMANCE.md` would drive a runtime's **envelope-decoder /
instance-validator / term-checker** over the new-tier rows exactly as they drive the ACS-002
decoder today: for each row, the runtime MUST reject `input_hex` with the matching `reject_reason`.
The verdict line would read e.g. `envelope-reject 7/7  instance-reject 7/7  language-reject 4/4`
in addition to `core-reject 16/16`. A conformant runtime must now implement the ACS-003/004/005
validators (the reference validators show they are implementable from the spec alone).

## 5. ACS-001 addressing negatives (addendum — separate harness path)

ACS-001 rejects are **addresser** rejects, not decoder rejects, so they don't fit the
`input_hex → decode` format. Proposed (described, not in the decode TSV): `unknown-domain-tag`
— addressing with a domain tag outside the allocated `0x01–0x09` range (e.g. reserved `0x0A`) MUST
be refused (ACS-001 §4: "an implementation MUST NOT compute an address without a [valid] domain
tag"; `acs001_address.content_id` raises today). If ratified, the harness gains a small
addresser-negative path: `(domain_hex, body_hex) → refuse`.

## 6. Ratification (what actually touches frozen — GATED on the maintainer)

This draft changes nothing. **Ratification** at CCP-GATE would, as one atomic amendment:

1. Add the 11 codes (+ tier values) to the `conformance/CONFORMANCE.md` reason-code registry and
   the ACS-001 §4.1 registry note.
2. Append the 18 vectors into `standard/vectors/acs_negative_vectors.tsv` (a frozen-Kit vector-set
   change → Kit version bump).
3. Extend `certify_runtime.py` + `CONFORMANCE.md`'s procedure to drive the new tiers (§4).
4. Re-run `freeze_check.py update` as part of the amendment (the sanctioned way the frozen
   baseline advances), and re-certify both reference runtimes against the expanded set.

Each is a frozen edit requiring explicit maintainer authorization; per CCP-GATE, each code ships
with its exercising vector (supplied here).

## 7. Honesty

- **DRAFT, freeze-clean.** No frozen byte is touched by this proposal; the freeze-diff gate stays
  at 0 drift. The vectors + codes are *candidates*.
- **The oracle is in-program (G1).** The validators that verify these candidates were authored in
  this program. Ratifying + shipping them strengthens the standard's *self-conformance*; it does
  **not** manufacture G2. A genuine external runtime rejecting these vectors from the Kit alone
  would be the G2 evidence — this draft just makes that surface exist to be tested.
- **Reason-code naming is a normative-design decision** surfaced here for the maintainer; the
  granularity (e.g. `field-type-mismatch` vs `value-out-of-range`) is a ratification choice.

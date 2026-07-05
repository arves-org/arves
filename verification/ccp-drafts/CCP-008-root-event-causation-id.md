# CCP-008 (DRAFT) — root-event `causation_id` canonicalization (closes B2)

**Status:** **DRAFT proposal — NOT ratified, NOT applied.** Freeze-clean: this document + its
generator (`gen_ccp008_vector.py`) + the candidate vector live under `verification/ccp-drafts/`.
**Nothing here edits the frozen `standard/`.** Ratification (which touches frozen ACS-003 §5 + the
envelope validator + the negative corpus) is a separate, maintainer-authorized **CCP-GATE** step.

**Instrument:** CCP Amendment. **Closes:** `G2_READINESS.md` **B2** — the last of the four confirmed
G2-readiness blockers without a concrete fix.

## 1. The defect (demonstrated, not asserted)

ACS-003 §5 makes `causation_id` the **sole OPTIONAL** field (Text | Null). For a **root event**
(one with no cause) two encodings are therefore both lawful today:

- **present with the explicit Null** — the encoding the single golden envelope already uses; and
- **absent** — a lawful realization of an optional field (ACS-003 §5.7 / ACS-004 §6.4 cardinality).

They produce **different canonical bodies** → **different ContentIds** for "the same" root event.
`gen_ccp008_vector.py` makes it concrete (run it):

```
CURRENT validator      : ACCEPTS the absent form (B2 gap confirmed)
ContentId present-Null : 1220fc0ef055e4d39de1c3ab7d2597361d24f7a8b6a1a0609a91b872b85ae4896f93
ContentId absent       : 1220b1b7d68f725946aae6239b9e55cc5653b0e9edf6a70261a8faeb24373161c9ad
-> two lawful encodings, two ContentIds: DIVERGE (B2)
```

Two honest implementers therefore disagree on the address of the **most common** envelope, and the
Kernel's **ORCH-004** idempotency dedup silently **forks** (it keys on the ContentId). This is
exactly the "two lawful encodings, one identity" trap the standard exists to preclude — and it is
the one a genuine first G2 team is most likely to hit.

## 2. Proposed fix (byte-clean — no golden ContentId changes)

Add a MUST to **ACS-003 §5** (harmonized with ACS-004 §6.4):

> A root event (one with no causing event) **SHALL** encode `causation_id` **present with the
> explicit Null** value; it **SHALL NOT** omit the key. An envelope that omits `causation_id` is
> non-canonical and **MUST** be rejected `missing-required-field`.

This picks the encoding the **single golden envelope already uses** (`causation_id = Null`), so
**no golden ContentId changes** — it is a vector-set + validator addition, not a byte-affecting
profile change (the CCP-007 pattern). It removes the second lawful encoding, so `present-Null` is
the *only* canonical form and the address is total again.

## 3. The candidate negative vector (oracle-demonstrated)

`ccp008_candidate_vector.tsv` (one row, same columns as the frozen negative TSV):

| case | tier | reject_reason |
|------|------|---------------|
| `envelope-root-omits-causation_id` | `envelope` | `missing-required-field` |

`gen_ccp008_vector.py` proves, machine-checked: (1) the candidate **decodes clean** as canonical
dCBOR; (2) the **CURRENT** `acs003_envelope.validate_envelope` **ACCEPTS** it (the B2 gap is real,
not hypothetical); (3) the **PROPOSED** rule **REJECTS** it with `missing-required-field` (the fix
is implementable); (4) the present-Null and absent ContentIds **diverge**. Exit 0 iff all four hold.

## 4. Ratification (what actually touches frozen — GATED on the maintainer)

At CCP-GATE, as one atomic amendment: (a) add the §5 MUST to `ACS-003_Canonical_Envelope.md`;
(b) make `causation_id` a *required-present* field in the envelope validator (`acs003_envelope.py`
+ the native Rust `arves-conformance::semantic`), so both references reject the absent form;
(c) append the vector to `standard/vectors/acs_negative_vectors.tsv` (envelope tier 7 → 8); (d)
re-run `freeze_check.py update` + re-certify both runtimes + regenerate the evidence probe. Each is
a frozen edit requiring explicit maintainer authorization.

## 5. Honesty

- **DRAFT, freeze-clean.** No frozen byte is touched; the freeze gate stays at 0 drift.
- **Byte-clean fix.** Present-Null is already the golden choice, so ratification changes no golden
  ContentId — but if a maintainer instead rules *absence* canonical, that WOULD change the golden
  and becomes an ACS-003 **profile bump** routed through CCP-GATE, never a silent edit.
- **In-program (G1).** The oracle that demonstrates the gap was authored in this program; closing
  B2 strengthens self-consistency, it does not manufacture G2.

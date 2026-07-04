# ARVES Conformance — procedure (language-neutral)

How ANY implementation proves it conforms to the ARVES Core Standards, using only
this Kit. No reference source required.

## The check

For each row in `../vectors/acs_golden_vectors.tsv` (`standard, vector, domain,
body_hex, content_id`):

1. **Encoder conformance (ACS-002/003/004).** From the logical value described
   normatively in the relevant `acs/ACS-00x` spec, produce the canonical `body`.
   Assert `hex(body) == body_hex`. (For ACS-001/005 the body is raw bytes, given
   directly; there is no encoding step.)
2. **Addresser conformance (ACS-001).** Compute
   `ContentId = 0x12 ‖ 0x20 ‖ SHA-256(domain ‖ body)`. Assert `hex(ContentId) == content_id`.

An implementation is **ACS-conformant** iff every row passes both checks. Two
independent implementations that both pass therefore agree byte-for-byte on every
address and body — they interoperate.

## The rejection check (negative vectors)

Producing the right bytes is only half of conformance. A conformant **decoder** MUST
also *reject* every byte string that is not in canonical form — otherwise two
implementations could accept different encodings of "the same" value and disagree on
its address. For each row in `../vectors/acs_negative_vectors.tsv` (`standard, case,
tier, input_hex, reject_reason`):

- **`tier = core`** — decode `input_hex` with your canonical decoder. It MUST fail,
  and the failure reason MUST equal `reject_reason` (the stable codes below). These
  are enforced by every conformant implementation.
- **`tier = nfc`** — the input is valid UTF-8 but not NFC-normalized. An
  implementation with a Unicode NFC facility MUST reject it (`non-nfc-text`); a
  dependency-free implementation without a Unicode table MAY **defer** this one rule
  (document the deferral — do not silently accept).
- **`tier = envelope` / `instance` / `language`** — *semantic* reject surfaces above
  the ACS-002 byte layer (added by **CCP-006**, ratified via ACS-001 §4.1). Each
  `input_hex` **decodes cleanly as canonical dCBOR** — the ACS-002 layer ACCEPTS it —
  and the defect is a violation of **ACS-003** (Canonical Envelope, §5/§6.3), **ACS-004**
  (Universal Type Registry instance validation, §6.5/§7/§8), or **ACS-005** (Normative
  Language term-set, §8/§9.2/§11) respectively. A runtime that implements the
  corresponding validator MUST reject the row with the matching `reject_reason`. A pure
  ACS-002 interop codec that does not implement the ACS-003/004/005 layer MAY **defer**
  these tiers — declaring the deferral explicitly, exactly like `nfc`, and never
  silently accepting. These tiers are **REQUIRED** for any runtime that processes
  envelopes / typed instances / glossaries (the full cognitive runtime); they are the
  falsifiable negative surface for ACS-003/004/005, which before CCP-006 had golden
  vectors but **zero** negative vectors.

Reason codes (ACS-002 §5): `non-shortest-int`, `non-shortest-len`,
`indefinite-length`, `unsorted-map-keys`, `duplicate-map-keys`, `float-not-float64`,
`negative-zero-float`, `non-finite-float`, `trailing-data`, `reserved-or-unsupported`,
`truncated`, `nesting-too-deep`, `non-nfc-text`. An implementation passes the rejection
check iff it rejects every `core` row with the matching reason. (`nesting-too-deep` is
emitted when structural nesting exceeds the ACS-002 §5.10 limit `MAX_DEPTH = 128`, so a
hostile depth bomb is rejected rather than crashing the decoder. `reserved-or-unsupported` is the
reason for anything not in the §4 value model: CBOR tags, the `undefined`/simple
values, non-UTF-8 text octets, and a map key that is not a Text or Integer.)

Semantic reason codes (ACS-003/004/005 — **CCP-006**, ACS-001 §4.1). Emitted by the
`envelope` / `instance` / `language` tiers above; each ships with a negative vector that
exercises it:

- **ACS-003 envelope / ACS-004 instance (shared):** `missing-required-field`,
  `unknown-field`, `field-type-mismatch`, `value-out-of-range`, `malformed-content-id`,
  `empty-shard-scope` (SHARD-001), `cardinality-violation`, `provenance-invariant`.
- **ACS-005 language:** `terms-not-sorted`, `duplicate-term`, `malformed-term-list`.

(`non-nfc-text` is reused for a non-NFC ACS-005 body.) A runtime that implements a given
layer passes its tier iff it rejects every row of that tier with the matching reason;
a runtime that defers the layer MUST declare the deferral (it does not fail for a
deferred tier, exactly as with `nfc`).

### Verdict semantics: core vs full conformance

There are two conformance tiers, and the top-line verdict MUST name which one:

- **Core-conformant** — passes all positive vectors and all `core` rejection rows.
  This is the interoperability gate: any two core-conformant runtimes agree
  byte-for-byte on every address and reject the same non-canonical inputs. A runtime
  without a Unicode NFC facility MAY be core-conformant while **deferring** the
  `nfc` tier; it MUST report the deferral explicitly, e.g.
  `VERDICT: CONFORMANT (ACS core; nfc-tier DEFERRED)`, and MUST NOT silently treat
  the deferred input as canonical — "defer" means "declare unenforced," not "accept."
- **Fully conformant** — additionally enforces the `nfc` tier (rejects `non-nfc-text`).
  A runtime that claims full conformance MUST reject every `nfc` row.

The reference Rust runner is core-conformant with a documented nfc deferral (it has
no Unicode table); the independent Python runner is fully conformant (stdlib
`unicodedata`). Both are correct at their declared tier — the tier label is what
makes the two verdicts comparable rather than contradictory.

## The report

Group results by standard and emit:

```
ARVES Conformance Report — ACS layer
  ACS-001 PASS (n/n)
  ACS-002 PASS (n/n)
  ACS-003 PASS (n/n)
  ACS-004 PASS (n/n)
  ACS-005 PASS (n/n)
  ACS golden vectors: N/N PASS
VERDICT: CONFORMANT (ACS layer)
```

Reference implementation of this procedure (Rust):
`cargo run -p arves-conformance --bin conformance`. A Go/Java/Python runner is a
thin re-implementation of the same two checks over the same TSV.

## Notes
- SHA-256 is FIPS 180-4. dCBOR is RFC 8949 §4.2 as profiled by ACS-002 (floats
  always 64-bit; NFC text; bytewise-sorted map keys; shortest ints; definite
  lengths).
- A conformant decoder MUST also REJECT non-canonical inputs (ACS-002 §5); the
  rejection check above runs over `../vectors/acs_negative_vectors.tsv` (**36 vectors**:
  16 `core` + 1 `nfc` + 7 `envelope` + 8 `instance` + 4 `language`; the semantic tiers were
  added by **CCP-006**, and the 8th instance row — `int`-above-i64 — by **CCP-007**).
  Reference (ACS-002 layer): `cargo run -p arves-conformance --bin conformance` reports
  `ACS-002 negative vectors: 16/16 core REJECTED` and defers the semantic tiers (no
  ACS-003/004/005 validators in the frozen v1.0 reference — tracked as **RCR-004**). The
  ACS-003/004/005 semantic tiers are exercised by the living reference validators:
  `python verification/independent/python/conformance_semantic.py` reports
  `envelope 7/7  instance 8/8  language 4/4 REJECTED`.
- This procedure covers the ACS interoperability layer. Runtime-behaviour
  conformance (the 12 Scenario axes, L1..L4) is the Certification process
  (`../certification/`).

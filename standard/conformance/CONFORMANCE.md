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
- A conformant decoder MUST also REJECT non-canonical inputs (ACS-002/003 §6);
  negative/rejection vectors are a planned Kit addition.
- This procedure covers the ACS interoperability layer. Runtime-behaviour
  conformance (the 12 Scenario axes, L1..L4) is the Certification process
  (`../certification/`).

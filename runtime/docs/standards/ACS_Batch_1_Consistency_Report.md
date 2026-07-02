# ACS Batch 1 — Consistency Report (Standards Office run #1)

**Office:** Specification / Standards Office. **Input:** the five ARVES Core
Standards drafts (ACS-001..005). **Purpose:** verify the batch is internally
coherent and byte-consistent before CCP-GATE candidacy and the Standard Lock
Review. **PMO Backlog #001, top-1.**

## The batch

| ACS | Title | CCP | Status | Domain tag(s) |
|-----|-------|-----|--------|---------------|
| ACS-001 | Universal Content Identity (content addressing) | CCP-001 | draft, real vectors | registry authority (0x01–0x07) |
| ACS-002 | Canonical Serialization (deterministic CBOR) | CCP-002 | draft, real vectors | — (defines the body) |
| ACS-003 | Canonical Envelope | CCP-003 | draft, real vector (OpenSSL-verified) | **0x06** canonical-envelope |
| ACS-004 | Universal Type Registry | CCP-004 | draft, real vectors | **0x07** type-schema; instances 0x01 |
| ACS-005 | Normative Language + Glossary | CCP-005 | draft (renamed from CCP-005) | — (governance) |

## Interop chain — verified coherent

The five standards compose into one byte-exact interoperability surface:

```
value ──ACS-002 dCBOR canon()──▶ body ──ACS-001 (0x?? ‖ body)──▶ ContentId (1220‖SHA256)
                                                   │
ACS-004 type-schema ──0x07──▶ schema ContentId (type identity)
ACS-004 instance ────0x01──▶ committed-truth ContentId
ACS-003 envelope ────0x06──▶ envelope ContentId; carries payload_cid = ACS-001 ContentId of the dCBOR payload body
ACS-005 ──▶ RFC 2119 modality + glossary + requirement IDs for all of the above
```

- Every address in the batch is `1220 ‖ SHA256(domain_tag ‖ ACS-002-canonical-body)` — one addressing rule (ACS-001), one body rule (ACS-002). ✔
- `occurred_at`/timestamps are ACS-002 **Integers** (i64 ns), never binary64 — consistent across ACS-002, ACS-003, ACS-004. ✔
- The ACS-002 profile-version tag (`ACS-002/1`) is carried **outside** every body (in the ACS-003 envelope / ACS-001 registry) — consistent. ✔
- ACS-005 supplies the normative-language convention (RFC 2119) + requirement IDs that ACS-001..004 already write in. ✔

## Defect found and fixed (the reason this pass exists)

**DOMAIN-TAG COLLISION (interop-breaking).** ACS-003 and ACS-004 were drafted in
parallel and **both independently claimed ACS-001 reserved domain tag `0x06`**
(ACS-003 for the envelope, ACS-004 for the type-schema). Two independent
implementations following the unreconciled drafts would compute *different*
addresses for the same value — a silent interoperability break, exactly the class
of gap the whole standardization program exists to prevent.

**Resolution (Standards Office):**
- `0x06` = **canonical-envelope** (ACS-003) — kept.
- `0x07` = **type-schema** (ACS-004) — reassigned; ACS-004's schema `ContentId`
  recomputed under `0x07`: `1220 6b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87`
  (re-derived from the exact 430-byte schema body; the prior `0x06` value
  `1220 d795bbb3…` was verified first to confirm the body transcription, then the
  `0x07` value computed — both via Python + OpenSSL).
- **ACS-001 is now the single domain-tag registry authority**, recording
  0x01–0x07 with 0x08–0x7F reserved; ACS-003/004 "allocate from" it rather than
  each inventing tags.

This is a governance lesson captured for future batches: **parallel-drafted
standards MUST run a Standards-Office reconciliation pass before ratification;**
shared registries (domain tags, requirement IDs, URNs) are the collision surface.

## Open items (not blocking this report)
- Ratify ACS-001..005 through CCP-GATE (Draft → Candidate): each already ships ≥1
  conformance scenario with real vectors.
- Re-run all conformance vectors in one harness (ACS-001-CS-1 / ACS-002-CS-1 /
  ACS-003-CS-1 / ACS-004-CS-1) as an executable differential-conformance check
  (Verification/Certification Office).
- ACS-006 Fingerprint + ACS-009 Replay Format (decision-trace schema; R-04) —
  next batch.

## Verdict
**ACS Batch 1 is internally consistent after the domain-tag fix.** The interop
surface (identity + serialization + envelope + type registry + normative language)
is coherent and byte-exact. Ready to proceed to PMO top-2 (L1 attestation) and
top-3 (Standard Lock Review), which will test independent-implementability against
this batch.

# ACS-001 / CCP-001 — Content Addressing Contract

**Type:** ARVES Core Standard (ACS) delivered as a Cognitive Change Proposal
Amendment (CCP-001). **Status:** RATIFIED v1.1 (CCP-GATE passed 2026-07; see `CCP-GATE-Ratification-v1.md`) —
normative at independence grade G1 (G2 external validation remains the open exit gate). **Program:** ARVES v1.1 Standardization, Goal 1 (Universal
Interoperability). **Closes:** Global Readiness Report R-01 (raised by 7/12
lenses). **Governs / activates:** the frozen invariant **ORCH-004** ("every
invocation is idempotent and content-addressable"), which names the property but
does not define the algorithm. This amendment ADDS the definition; it does not
edit any frozen document (ED-001; sanctioned via the Reference Lifecycle CCP
process).

> Normative keywords (MUST / SHALL / SHOULD / MAY) are used per RFC 2119/8174.
> This document also seeds the Goal-4 Normative-Language Convention by example.

---

## 1. Problem

ORCH-004 makes content-addressing a load-bearing, interoperability-critical
primitive, but no frozen document defines **what bytes are hashed**, **how they
are canonicalized**, or **which digest function** produces the address. Seven of
twelve independent reviews flagged this as the single largest blocker: without a
byte-exact address, two independent implementations cannot dedup, replay,
cross-reference, or cross-certify — they cannot interoperate at all.

## 2. Scope

This standard defines the **content address**: a deterministic, self-describing
digest computed over a domain-tagged canonical pre-image. It defines the address
format, the mandatory digest, domain separation, and the binding to ORCH-004
idempotency. It does **not** define the canonical serialization of rich values —
that is **ACS-002** (Canonical Serialization); this standard is parametric over
it and pins byte-exact test vectors so conformance is testable today.

## 3. Definitions (normative)

- **Content Address (`ContentId`)** — the self-describing multihash of a
  pre-image, as defined in §5.
- **Pre-image** — the exact byte string that is hashed: `domain_tag || body`,
  where `body` is the canonical serialization (ACS-002) of the addressed value.
- **Domain tag** — a single byte (§4) that partitions the address space by
  meaning so that byte-identical bodies in different roles cannot collide.

## 4. Domain separation (normative)

An implementation **SHALL** prefix every pre-image with exactly one domain tag:

| Tag | Domain |
|-----|--------|
| `0x01` | commit-content (a committed truth payload) |
| `0x02` | engine-manifest |
| `0x03` | capability-manifest |
| `0x04` | invocation (idempotency key) |
| `0x05` | decision-trace record |
| `0x06` | canonical-envelope (allocated by ACS-003) |
| `0x07` | type-schema (allocated by ACS-004) |
| `0x08` | normative-glossary term-set (allocated by ACS-005) |
| `0x09` | requirement clause (allocated by ACS-005) |

Tags `0x0A`–`0x7F` are RESERVED for future ACS standards. An implementation
**MUST NOT** compute an address without a domain tag, and **MUST NOT** reuse a tag
for a different domain.

### 4.1 Registry & allocation policy (normative)

ARVES maintains three registries; this section is the allocation authority for all of
them (analogous to an RFC "IANA Considerations" section).

- **Domain-tag registry** (the table above). Range `0x01–0x09` are allocated;
  `0x0A–0x7F` are unallocated and available; `0x80–0xFF` are RESERVED (never
  allocated in v1). **Allocation policy:** *Specification Required* — a new tag is
  allocated only by a ratified ACS or a CCP Amendment that defines the domain's
  pre-image and a conformance vector; a tag, once allocated, is permanent and MUST
  NOT be reassigned. (Two historical double-allocations of `0x06` were caught and
  corrected during Batch 1; see `ACS_Batch_1_Consistency_Report.md`.)
- **Multihash hash-code registry.** `0x12` = SHA-256 (mandatory, §5). New hash codes
  (for algorithm agility, ACS-002 §11) are allocated by CCP Amendment; both the old
  and new code MAY coexist during migration. Codes follow the multicodec table.
- **Reason-code registry.** The rejection reason codes are normative and enumerated in
  `conformance/CONFORMANCE.md`; new codes are added only via a CCP Amendment that also
  adds a negative conformance vector exercising the code. Two bands are registered:
  the ACS-002 canonical-form codes (`core`/`nfc` tiers), and — via **CCP-006**
  (`acs/CCP-GATE-Ratification-v2.md`) — the ACS-003/004/005 *semantic* codes
  (`envelope`/`instance`/`language` tiers): `missing-required-field`, `unknown-field`,
  `field-type-mismatch`, `value-out-of-range`, `malformed-content-id`,
  `empty-shard-scope`, `cardinality-violation`, `provenance-invariant`,
  `terms-not-sorted`, `duplicate-term`, `malformed-term-list`. Every one ships with its
  exercising negative vector in `vectors/acs_negative_vectors.tsv`.

Each registry's designated maintainer is the ARVES Certification Authority; changes
follow the Reference-Lifecycle CCP-GATE (never a silent edit; ED-001).

## 5. Address format (normative)

The content address **SHALL** be a self-describing multihash:

```
ContentId = varint(hash_code) || varint(digest_len) || digest
```

- The digest function **SHALL** be SHA-256; every conformant implementation
  **MUST** implement SHA-256. `hash_code` for SHA-256 **SHALL** be `0x12` and
  `digest_len` **SHALL** be `0x20` (32). Thus a SHA-256 ContentId is the 34-byte
  string `0x12 0x20 || SHA256(pre-image)`.
- An implementation **MAY** additionally support other registered hash codes for
  migration, but interoperability and certification **SHALL** be evaluated on
  SHA-256. Being self-describing, the address carries its own algorithm, so a
  future algorithm upgrade does not require re-addressing existing content.
- The address **SHALL** be computed as
  `ContentId = 0x12 0x20 || SHA256(domain_tag || body)`, i.e. the multihash of a
  **single** SHA-256 taken over the pre-image `domain_tag || body`. The pre-image is
  the *data* passed to SHA-256; it is never itself pre-hashed. (There is exactly one
  hash application — the earlier "self-describing multihash of the pre-image" and the
  34-byte form `0x12 0x20 || SHA256(pre-image)` above state the same single hash.)

## 6. Idempotency binding to ORCH-004 (normative)

- Two proposals whose pre-images are byte-equal **SHALL** produce equal
  `ContentId`s, and equal `ContentId`s **SHALL** denote the same truth.
- The Kernel **SHALL** commit at most one truth per `(shard, ContentId)`; a
  re-proposal of an existing `ContentId` **SHALL** resolve to the existing
  `TruthRef` and **SHALL NOT** append a second log record. Whether this is
  surfaced as `Ok(existing)` or a typed `AlreadyCommitted` outcome is
  implementation-defined and **SHALL NOT** affect conformance (observational
  equivalence).
- `ContentId` equality is the ONLY sanctioned identity test for truth; an
  implementation **SHALL NOT** use a non-content-derived identifier as the commit
  identity.

## 7. Conformance scenario (CCP-GATE requirement) — `ACS-001-CS-1`

A conformant implementation, given the pre-image bytes below, **SHALL** produce
exactly the stated `ContentId`. Two independent implementations **SHALL** agree on
all vectors (differential conformance). Digest = SHA-256; ContentId = `1220 ||
SHA256(pre-image)`.

| # | domain | body (utf-8) | pre-image (hex) | ContentId (hex) |
|---|--------|--------------|-----------------|-----------------|
| 1 | `0x01` commit-content | `hello-truth` | `0168656c6c6f2d7472757468` | `122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e` |
| 2 | `0x02` engine-manifest | `{"engine":"summarize","version":"1"}` | `027b22656e67696e65223a2273756d6d6172697a65222c2276657273696f6e223a2231227d` | `12205c631bd808332b0889763100ad7458710c137320381e3b4ea9cce3c0640a4e54` |
| 3 | `0x04` invocation | `acme/research|c1|hello-truth` | `0461636d652f72657365617263687c63317c68656c6c6f2d7472757468` | `1220ae7a70002ef6dd81018d4715a986dae6dfdc1b7bc85acdd66698875f2fe302bc` |

An implementation that produces a different `ContentId` for any vector, or that
collides vectors 1 and 3 by omitting the domain tag, **SHALL** be non-conformant.

## 8. Proposal analysis

- **Why it matters.** Content addressing is the interoperability epicenter:
  dedup, replay cross-reference, engine/capability identity, cache keys, and
  cross-runtime certification all resolve to it. It is the common root beneath
  Git, OCI/Docker, Nix, IPFS, and Bazel — immutable identity by content.
- **Risks.** (a) Canonicalization dependence: addresses are only stable once
  ACS-002 fixes the canonical body; until then, addresses are defined over fixed
  test bytes (§7) and over already-canonical byte payloads. (b) Algorithm agility
  vs simplicity: mandating SHA-256 while allowing self-describing alternatives
  balances longevity against interop; a single mandatory algorithm keeps
  certification decidable.
- **Long-term consequences.** A self-describing multihash lets ARVES migrate hash
  functions in 20 years without re-addressing history — a decision that ages well
  (cf. systems that hard-coded SHA-1 and suffered).
- **Alternatives considered.** (i) Raw SHA-256 with no multihash prefix —
  rejected: no algorithm agility. (ii) Delegate to IDR (status quo) — rejected:
  keeps the standard's central primitive outside the standard. (iii) Per-domain
  separate hash schemes — rejected: domain *tag* + one scheme is simpler and
  collision-safe.
- **Recommendation.** Ratify via CCP-GATE with `ACS-001-CS-1`; then a Runtime task
  adopts it in `arves-kernel` (define `ContentId` construction; the caller-
  supplied opaque `ContentHash` becomes a computed multihash). Note: the WAL's
  CRC-32 is frame-integrity only and is NOT a content address; this standard does
  not change that.
- **Implementation complexity.** Standard: medium. Reference-runtime adoption:
  low–medium (SHA-256 + framing; depends on ACS-002 for rich values).
- **Scientific impact.** Turns "content-addressable" from an unfalsifiable claim
  into a testable, cross-implementation property (differential conformance).
- **Ecosystem impact.** Unblocks the second independent runtime, cross-runtime
  caches/registries, and reproducible artifacts — the precondition for a real
  certification market.

## 9. Dependencies & sequence
- **Blocks:** ACS-002 (Serialization), ACS-004/005 (Type/Schema Registry — their
  identities are content addresses), ACS-006/009 (Fingerprint/Replay), the
  differential-conformance tier (Goal 3), and Program C (second runtime).
- **Depends on:** ACS-002 for the canonical body of rich (non-byte) values; until
  ACS-002 ratifies, ACS-001 is conformance-testable via §7's fixed byte vectors.

---

*Ratification path (Reference Lifecycle): DRAFT → CCP-GATE (this doc + `ACS-001-CS-1`)
→ Candidate → Ratified. On ratification this becomes a registered normative
addition; the frozen v1.0 corpus is unchanged.*

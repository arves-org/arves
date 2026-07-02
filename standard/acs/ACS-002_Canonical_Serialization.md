# ACS-002 / CCP-002 — Canonical Serialization Contract

**Type:** ARVES Core Standard (ACS) delivered as a Cognitive Change Proposal
Amendment (CCP-002). **Status:** DRAFT (Candidate on CCP-GATE pass; not yet
Ratified). **Program:** ARVES v1.1 Standardization, Goal 1 (Universal
Interoperability). **Closes:** the ACS-001 dependency on a byte-exact canonical
body (ACS-001 §2, §9) and the Global Readiness family that flagged the absence of
a defined serialization. **Governs / activates:** the frozen invariants
**ORCH-003** ("every execution is REPLAYABLE … deterministically") and
**ORCH-004** ("every … invocation is idempotent and content-addressable") — both
name a deterministic byte form without defining one — and the **ENG-005** /
**CAP-003** notion of a *content-addressable manifest*. This amendment ADDS the
definition of the canonical byte form; it does **not** edit any frozen document
(ED-001; sanctioned via the Reference Lifecycle CCP process).

> Normative keywords (MUST / SHALL / SHOULD / MAY / MUST NOT / SHALL NOT) are used
> per RFC 2119 and RFC 8174: they carry their normative meaning only when in
> ALL-CAPS. This document continues the Goal-4 Normative-Language Convention
> seeded by ACS-001.

---

## 1. Problem

ACS-001 (Content Addressing) hashes a **pre-image** `domain_tag || body`, where
`body` is "the canonical serialization (ACS-002) of the addressed value." ACS-001
is explicitly *parametric* over that serialization and could only pin conformance
over fixed raw-byte payloads. Until the canonical body is defined, no two
independent implementations can agree on the bytes of a *rich* ARVES value — a
fact, an engine manifest, a decision-trace record — and therefore cannot agree on
its `ContentId`. Every downstream property that ACS-001 unlocks (dedup, replay
cross-reference, idempotent commit under ORCH-004, deterministic replay under
ORCH-003, cross-runtime certification) collapses the moment the value is anything
richer than an opaque byte string.

The frozen corpus makes this concrete. The Universal Cognitive Ontology defines a
value model that carries **both** exact integers (`Timestamp` = signed 64-bit
nanoseconds since the Unix epoch) **and** floating-point numbers (`Confidence` in
`[0.0, 1.0]`), alongside structured URNs, provenance records, and bitemporal
coordinates. A serialization that cannot represent a 64-bit nanosecond timestamp
*exactly* silently corrupts cognitive truth. The reference kernel today stores the
body as "opaque canonicalized payload bytes (typed later via arves-ontology)" —
i.e. the canonicalization is assumed but undefined. ACS-002 defines it.

## 2. Scope

This standard defines **ARVES Canonical Serialization**: the deterministic byte
form (`body`) of an ARVES value — the exact bytes that ACS-001 hashes and that
ACS-003 will envelope. It fixes: the value model, integer/float rules and the
canonical number form, string and Unicode normalization, map-key ordering,
null-vs-absent semantics, array ordering, definite-length framing, non-finite
handling, the round-trip and determinism obligations, and the version tag.

It does **not** define: how a concrete `uci.*` cognitive type maps onto the value
model — that is **ACS-004/005** (Type / Schema Registry); the outer envelope or
the transport of the version tag — that is **ACS-003**; nor the address format
itself — that is **ACS-001**, over which this standard is the missing complement.

## 3. Definitions (normative)

- **ARVES value** — an instance of the abstract value model in §4. It is the unit
  that is serialized; it is independent of any programming language, in-memory
  representation, or storage engine (Ontology `O-007`).
- **Canonical serialization / canonical body** — the single byte string produced
  by applying §5 to an ARVES value. For a given value it is unique.
- **Canonicalization** — the total function `value → canonical body`.
- **dCBOR** — the ARVES *deterministic CBOR* profile: the subset of RFC 8949
  Concise Binary Object Representation constrained by §5. Every canonical body is
  a well-formed, deterministically-encoded CBOR data item per RFC 8949 §4.2.
- **Non-canonical encoding** — any byte string that decodes to an ARVES value but
  is not the canonical serialization of that value (e.g. non-shortest integer,
  unsorted map keys, indefinite length, non-NFC text). Such input MUST be
  rejected by a conformant decoder (§6).

## 4. Value model (normative)

An ARVES value **SHALL** be exactly one of the following. This is the complete
base model for ACS-002/1; nothing else may appear in a canonical body.

| # | Value | Meaning | dCBOR carrier |
|---|-------|---------|---------------|
| 1 | **Null** | absence-of-value marker (distinct from a missing map entry, §5.7) | major 7, `0xf6` |
| 2 | **Bool** | `true` / `false` | major 7, `0xf5` / `0xf4` |
| 3 | **Integer** | a signed integer in the closed range `[-2^64, 2^64 − 1]` covered by CBOR majors 0/1; the ontology's `Timestamp` (i64) is a strict subset | major 0 (≥0) / major 1 (<0) |
| 4 | **Float** | an IEEE-754 binary64 **finite** number; the ontology's `Confidence` is a subset | major 7, `0xfb` |
| 5 | **Text** | a Unicode string, NFC-normalized, carried as UTF-8; the ontology's `EntityUrn` string form is a Text | major 3 |
| 6 | **Bytes** | an opaque, uninterpreted octet string | major 2 |
| 7 | **Array** | an ordered, finite sequence of ARVES values | major 4 |
| 8 | **Map** | a finite association of keys to ARVES values, where every key is a Text or an Integer | major 5 |

Integer and Float are **distinct** value kinds and **SHALL NOT** be conflated: a
quantity that is conceptually an integer (a count, a nanosecond timestamp, a
version component) **MUST** be an Integer value, and **MUST NOT** be encoded as a
Float. CBOR tags, indefinite-length items, and the CBOR "undefined" simple value
(`0xf7`) are **NOT** part of the base model and **MUST NOT** appear in a canonical
body; the tag space is RESERVED for a future minor version introduced via CCP.

## 5. Canonical encoding rules (normative)

A canonical body **SHALL** be a deterministically-encoded CBOR data item (RFC 8949
§4.2) that additionally satisfies §5.1–§5.9. There is exactly one canonical body
per ARVES value.

### 5.1 Definite lengths only
Every text string, byte string, array, and map **SHALL** be encoded with a
definite length. Indefinite-length ("streaming") encodings **MUST NOT** be
produced and **MUST** be rejected on decode.

### 5.2 Integers — shortest form
An Integer **SHALL** be encoded in the shortest additional-information form that
represents it (RFC 8949 §4.2.1): the argument **SHALL** use the fewest bytes
(inline `0..23`, then 1, 2, 4, or 8 bytes). A non-negative Integer `n` uses major
0; a negative Integer `n` uses major 1 with argument `−1 − n`. A longer-than-
necessary encoding of an Integer is non-canonical and **MUST** be rejected.

### 5.3 Floats — fixed binary64, finite only
A Float **SHALL** be encoded as a 64-bit IEEE-754 double (major 7, additional
information 27, initial byte `0xfb`), in network byte order. ACS-002 deliberately
does **NOT** apply the RFC 8949 §4.2.2 "smallest floating-point type that
preserves the value" reduction: fixing every Float to binary64 removes the last
float-width degree of freedom and is trivially implementable on every platform.
The values **NaN**, **+Infinity**, and **−Infinity** have no cognitive meaning and
no canonical bit pattern; an encoder **MUST** reject them and **MUST NOT** emit a
non-finite Float. Negative zero **SHALL** be normalized to positive zero
(`+0.0`); an encoder **MUST** emit `fb0000000000000000` for a zero Float and a
decoder **MUST** reject `fb8000000000000000`.

### 5.4 Text strings — UTF-8, NFC
A Text value **SHALL** be Unicode-normalized to **Normalization Form C (NFC)** —
**per UAX #15, Unicode 16.0.0** — and then encoded as its UTF-8 octets (major 3).
Conformant runtimes **MUST** normalize against that pinned Unicode version so two
implementations cannot disagree on whether a string is NFC; a later Unicode version
is a new profile introduced via CCP (§7). Two inputs that denote the same
abstract text under Unicode canonical equivalence (e.g. the same string supplied
decomposed, NFD, versus precomposed, NFC) **SHALL** produce identical canonical
bytes. An encoder **MUST NOT** emit non-NFC text; a decoder **MUST** reject a text
string whose octets are not already in NFC. Byte strings (§4 kind 6) are **NOT**
normalized — they are opaque octets.

### 5.5 Byte strings
A Bytes value **SHALL** be encoded as major 2 with a definite length, octets
verbatim. Bytes and Text are distinct value kinds and **SHALL NOT** be
interchanged.

### 5.6 Maps — bytewise-sorted encoded keys, no duplicates
A Map **SHALL** be encoded as major 5 with a definite length. Its entries
**SHALL** be sorted by the **bytewise lexicographic order of each key's own
canonical (dCBOR) encoding** (RFC 8949 §4.2.1). Because keys are Text or Integer
and are themselves canonically encoded, this order is total and implementation-
independent. Two entries whose canonical key encodings are byte-equal are
**duplicate keys**; an encoder **MUST** reject a Map containing duplicates and a
decoder **MUST** reject a Map whose entries are unsorted or contain duplicate
keys.

### 5.7 Null versus absent
A Map entry that is *present with a Null value* and a Map entry that is *absent*
are **distinct** and produce **distinct** canonical bodies; they **SHALL NOT** be
conflated. Canonicalization **MUST NOT** drop, insert, or default any Map entry:
the set of keys is part of the value's identity.

### 5.8 Arrays — order preserved
An Array **SHALL** be encoded as major 4 with a definite length, elements in their
given order. Array order is significant and **SHALL** be preserved verbatim; an
encoder **MUST NOT** sort, deduplicate, or reorder array elements. (Unordered
collections such as sets are out of scope for ACS-002/1 and are RESERVED for a
future ACS.)

### 5.9 No trailing data
A canonical body **SHALL** consist of exactly one top-level CBOR data item with no
leading or trailing bytes. A decoder **MUST** reject any input with trailing
octets after the top-level item.

### 5.10 Maximum nesting depth
A canonical body **SHALL NOT** nest Arrays and Maps deeper than **`MAX_DEPTH = 128`**
structural levels (the top-level item is depth 0; each Array element and each Map key
or value is one level deeper). A decoder **MUST** reject a body that exceeds this
depth with reason `nesting-too-deep`, **before** recursing into it, so that a hostile
"depth bomb" cannot exhaust the decoder's stack. The bound is a fixed constant shared
by all implementations so they agree on the canonical set; 128 is ~30× the deepest
structure any ACS type reaches. (See Security Considerations, §11.)

## 6. Determinism, round-trip, and decoder validation (normative)

Three obligations bind every conformant implementation:

1. **Determinism.** For any ARVES value `v`, canonicalization **SHALL** yield the
   same byte string on every conformant implementation, on every platform, in
   every process, at every time: `canon(v)` is a pure function of `v` alone. This
   is the property ACS-001 relies on for `ContentId` equality (ACS-001 §6).
2. **Round-trip.** For every value `v` in the model, `decode(canon(v))` **SHALL**
   equal `v`, and re-encoding a decoded canonical body **SHALL** reproduce the
   identical bytes: `canon(decode(b)) == b` for every canonical body `b`
   (canonicalization is idempotent).
3. **Validation.** A conformant decoder **MUST** reject any non-canonical
   encoding: non-shortest integers, unsorted or duplicate map keys, indefinite
   lengths, non-NFC text, non-finite or negative-zero floats, tags, the
   `undefined` simple value, and trailing data. Silent acceptance of non-canonical
   input would let two byte strings that "mean the same value" carry different
   `ContentId`s, breaking ORCH-004 idempotency; therefore rejection is mandatory,
   not optional.

Determinism binds ACS-002 to ACS-001 as follows: the ACS-001 pre-image is
`domain_tag || canon(v)`, so two implementations that both honor §5 produce the
same body, the same pre-image, and the same `ContentId` for the same value — the
precondition for differential conformance and cross-runtime certification.

## 7. Version tag and evolution (normative)

The profile defined by §4–§6 is versioned **`ACS-002/1`** (major `2`, this
standard; minor `1`, this profile). The version is a property of the *serialization
scheme*, not of any single value, and **SHALL** be carried **outside** the body —
in the ACS-003 envelope and/or the ACS-001 domain registry — so that canonical
bodies remain minimal and byte-stable and are never self-describing at the value
layer. Embedding a version field inside every body is **NON-CONFORMANT**, because
it would change the bytes (and thus every `ContentId`) of otherwise-identical
values.

A future revision (e.g. adding CBOR tags for a bignum or an unordered-set kind)
**SHALL** be introduced as a new minor or major profile via a CCP Amendment with
its own conformance scenario (Reference Lifecycle CCP-GATE), **SHALL** receive a
distinct version tag, and **SHALL NOT** silently alter the bytes produced by
`ACS-002/1`. Bodies already addressed under `ACS-002/1` remain valid and
re-derivable forever, because the address is self-describing (ACS-001 §5) and the
producing profile is pinned by the envelope.

## 8. Conformance scenario (CCP-GATE requirement) — `ACS-002-CS-1`

A conformant implementation, given each ARVES value below, **SHALL** produce
exactly the stated canonical body, and therefore (via ACS-001, `ContentId =
1220 || SHA256(domain_tag || body)`) exactly the stated `ContentId`. Two
independent implementations **SHALL** agree on all vectors (differential
conformance). All bodies are `ACS-002/1` dCBOR. The `ContentId`s below were
independently re-derived from the pre-image bytes with a second, unrelated hashing
toolchain and matched byte-for-byte.

### 8.1 Structured vectors

| # | domain | ARVES value | canonical body (hex) | ContentId (hex) |
|---|--------|-------------|----------------------|-----------------|
| V1 | `0x01` commit-content | Map `{ "type": "uci.fact", "claim": "sky-is-blue", "confidence": Float 0.5, "observed_at": Integer 1730000000000000000 }` (author key order irrelevant) | `a46474797065687563692e6661637465636c61696d6b736b792d69732d626c75656a636f6e666964656e6365fb3fe00000000000006b6f627365727665645f61741b180231d5856d0000` | `12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e` |
| V2 | `0x02` engine-manifest | Map `{ "engine": "summarize", "version": Integer 1, "deterministic": true, "reads": [ "uci.observation", "uci.fact" ], "seed": Null }` | `a56473656564f6657265616473826f7563692e6f62736572766174696f6e687563692e6661637466656e67696e656973756d6d6172697a656776657273696f6e016d64657465726d696e6973746963f5` | `1220e5aad722341bd0838fb268d73a0a28401457883b9e5e623c05dc0623f57a690d` |
| V3 | `0x05` decision-trace | Map `{ "label": "Amélie é—中", "n": Integer −1000 }` — `label` supplied in either NFC or NFD form | `a2616e3903e7656c6162656c70416dc3a96c696520c3a9e28094e4b8ad` | `12207c5367768a3cd0d90b781cac2530335f0310ffc155eac4ac82da80af71e2366a` |

**What each vector pins.**
- **V1** proves (a) map-key reordering: an author who supplies `type, confidence,
  observed_at, claim` and an author who supplies `claim, observed_at, type,
  confidence` MUST both yield the body above (keys sorted bytewise by each key's
  full encoded bytes — the length-prefixed dCBOR key, per §5.6 — giving
  `type < claim < confidence < observed_at`, i.e. `0x64… < 0x65… < 0x6a… < 0x6b…`,
  which is the order of the body above); (b) the integer/float distinction:
  `confidence` is a binary64 Float (`fb3fe0000000000000`) while `observed_at` is a
  64-bit Integer held **exactly** (`1b180231d5856d0000` = 1 730 000 000 000 000 000),
  the case a JCS/float64 scheme would corrupt.
- **V2** proves nested arrays (order preserved: `uci.observation` before
  `uci.fact`), an explicit **Null** entry (`seed: null` present, not dropped — §5.7),
  a Bool, and an Integer version component encoded as an Integer, not a Float.
- **V3** proves Unicode NFC: the decomposed input (`A m e U+0301 l i e … U+00E9→e
  U+0301 …`) and the precomposed input (`… U+00E9 …`) both normalize to NFC and
  produce the identical body `…70416dc3a9…` (é = UTF-8 `c3a9`); it also pins a
  negative Integer (`3903e7` = −1000).

### 8.2 Scalar vectors (primitive encodings, domain `0x01`)

| ARVES value | canonical body (hex) | ContentId (hex) |
|-------------|----------------------|-----------------|
| Bool `true` | `f5` | `1220673dcea23abded6dda60ed0d1ee28ec62383f1672326e4bb493742036b05d7f7` |
| Null | `f6` | `12208cb54aa16f22dace168fb0e08f5989904a6393238d44bd5ce86f0296fdb609a5` |
| Integer `0` | `00` | `122047dc540c94ceb704a23875c11273e16bb0b8a87aed84de911f2133568115f254` |
| Integer `24` | `1818` | `1220798e8eb448e14baded12b398d53377dd7601c9abd3ecfc6bed824370d9e2da85` |
| Float `1.0` | `fb3ff0000000000000` | `12200007fa2b944d411802cbe51de525247c3a196e3252760f5458f347540b0c3de6` |
| Text `"hello-truth"` | `6b68656c6c6f2d7472757468` | `122019da5fff72b64840bcc15c8b8e9a8c5279b2abb2222f1995a5290b0f89c003a3` |

> Note (intended, not a defect): the ACS-002 Text `"hello-truth"` body is
> `6b68656c6c6f2d7472757468` — the UTF-8 octets prefixed by the CBOR text-string
> header `6b` (major 3, length 11). This is **not** the raw-byte string
> `68656c6c6f2d7472757468` used in ACS-001 §7 vector 1, so the two `ContentId`s
> differ. This is correct: ACS-002 bodies are *typed CBOR values*, not raw octets;
> a raw-byte payload is an ACS-002 **Bytes** value, a string is a **Text** value,
> and the two are deliberately distinguishable (§5.5). Implementations that
> address structured values MUST use ACS-002 bodies, not ad-hoc raw bytes.

### 8.3 Non-conformance
An implementation **SHALL** be non-conformant if it: emits a Float for any Integer
value (e.g. `observed_at` or `version`); fails to sort map keys by encoded-key
bytes (V1); drops the `seed: null` entry (V2); fails to NFC-normalize (V3);
produces any indefinite-length item; emits `-0.0`, `NaN`, or an Infinity; or
diverges from any body/`ContentId` above.

## 9. Proposal analysis

- **Why it matters.** ACS-002 is the keystone of the ACS stack: it is the "body"
  under ACS-001, the payload under ACS-003, the on-the-wire form of every
  registered type (ACS-004/005), and the substrate of fingerprint/replay
  (ACS-006/009). Nothing above it can be byte-exact until it is. It converts
  ORCH-003 (replay) and ORCH-004 (idempotent, content-addressable) from properties
  a document *claims* into properties two independent runtimes can *prove*
  identical, byte for byte.
- **Why deterministic CBOR and not JCS (the decisive choice).** The primary
  alternative, RFC 8785 JSON Canonicalization Scheme, serializes numbers via the
  ECMAScript/I-JSON rule — every number is an IEEE-754 **binary64**. The ARVES
  ontology carries `Timestamp` as a **signed 64-bit nanosecond** integer, whose
  magnitude exceeds `2^53`; under JCS such a value cannot survive round-trip
  without loss. A cognitive-truth standard that silently rounds timestamps,
  counters, or version numbers is disqualifying. dCBOR (RFC 8949 §4.2) encodes
  integers and floats as **distinct** major types, exactly, to 64 bits — an exact
  fit for the frozen value model. dCBOR is additionally binary, length-prefixed,
  and typed, which shrinks the canonicalization surface (no whitespace, no
  string-escape variance, no int/float ambiguity) and therefore the number of ways
  two implementations can diverge. It also composes naturally with ACS-001's
  binary, self-describing multihash.
- **Risks.** (a) *Ecosystem familiarity* — JSON is more human-readable; dCBOR needs
  a decoder/diagnostic tool. Mitigation: RFC 8949 §8 defines a standard diagnostic
  notation, and the vectors here are auditable via any CBOR tool. (b) *NFC cost* —
  requiring NFC pushes a Unicode dependency into every implementation; mitigation:
  NFC is available in every major platform and is required exactly once, at the
  Text boundary. (c) *Float determinism* — floats are inherently perilous;
  mitigation: fixing binary64 and forbidding non-finite/-0.0 makes the Float
  encoding total and unambiguous, and the ontology uses floats only for a bounded
  `[0,1]` confidence. (d) *Two encoders, one profile* — the risk that RFC 8949's
  own §4.2.2 float-reduction differs from our fixed-width rule; mitigation: §5.3
  explicitly *overrides* §4.2.2 to a single width, closing that gap.
- **Long-term consequences.** Choosing exact 64-bit integers now avoids a class of
  silent-corruption incidents that plagued float-only formats; fixing one binary64
  width avoids the perennial "shortest float" disagreements; carrying the version
  tag *outside* the body means bodies addressed today remain re-derivable in 2046
  without re-encoding. The profile is intentionally minimal so that it can be
  formally specified and even mechanically checked.
- **Alternatives considered.** (i) **RFC 8785 JCS** — rejected on the binary64
  number model (above) and larger text-canonicalization surface. (ii) **Raw /
  application-defined JSON** — rejected: no canonical form at all, the status quo
  gap. (iii) **MessagePack** — rejected: no standardized deterministic/canonical
  profile with mandated key ordering. (iv) **Protocol Buffers / FlatBuffers** —
  rejected: schema-coupled and *not* canonical (field order and default handling
  are unspecified across implementations); ARVES needs a canonical form that is
  schema-*independent* at the value layer (types are layered on top by ACS-004/005).
  (v) **Full RFC 8949 §4.2 with the §4.2.2 float reduction** — rejected in favor of
  fixed binary64 to eliminate the final float-width ambiguity.
- **Recommendation.** Ratify via CCP-GATE with `ACS-002-CS-1`. Then a Runtime task
  adopts it in `arves-ontology` (a canonical encoder/decoder for the value model)
  and `arves-kernel` (the `ProposedWrite.content` `ContentHash` becomes
  `multihash(SHA-256, SHA256(domain || canon(value)))` from ACS-001, replacing the
  caller-supplied opaque hash). The WAL's CRC-32 remains frame-integrity only and
  is unaffected.
- **Implementation complexity.** Standard: medium. Reference-runtime adoption:
  medium — a deterministic CBOR encoder/decoder plus NFC is a well-understood,
  few-hundred-line component with abundant prior art; the strictness (validation on
  decode) is the main care point.
- **Scientific impact.** Turns "canonical serialization" and, transitively,
  "content-addressable" and "replayable" from prose claims into a
  differentially-testable, cross-implementation property with published,
  independently-recomputed vectors.
- **Ecosystem impact.** Unblocks the second independent runtime (Program C),
  cross-runtime caches and registries, reproducible engine/capability manifests
  (ENG-005/CAP-003), and the differential-conformance tier (Goal 3). It is the
  precondition for a real certification market: without one canonical body, two
  certified runtimes could not exchange a single addressed value.

## 10. Dependencies & sequence
- **Blocks:** ACS-001's addressing of *rich* (non-raw-byte) values (ACS-001 §9
  named ACS-002 as its dependency); ACS-003 (Envelope — carries the version tag);
  ACS-004/005 (Type / Schema Registry — map `uci.*` types onto this value model);
  ACS-006/009 (Fingerprint / Replay — ORCH-003 replay compares canonical bodies);
  the differential-conformance tier (Goal 3) and the second runtime (Program C).
- **Depends on:** ACS-001 for the address format and domain-tag registry (this
  standard reuses domain tags `0x01`, `0x02`, `0x05`); RFC 8949 (CBOR) and RFC 8949
  §4.2 (deterministic encoding); Unicode NFC (UAX #15); IEEE-754 binary64.
- **Grounded in (frozen, not edited):** Universal Cognitive Ontology v1 (value
  model: `Timestamp` i64, `Confidence` f64, `EntityUrn`, `Origin`); Vol 9 Cognitive
  Control Plane v2 Part 5 (ORCH-003, ORCH-004); Invariant Registry v1 (ENG-005,
  CAP-003 content-addressable manifests); Engine Graph Specification v1 (manifests
  and graphs are content-addressable).

## 11. Security Considerations (normative)

A canonical decoder is exposed to hostile, attacker-chosen bytes; the following are
part of the standard, not implementation advice.

- **Hostile-decoder threat model.** A decoder **MUST** treat every input as adversarial
  and fail safely (return a rejection) rather than abort, over-allocate, or recurse
  without bound. Specifically: (a) it **MUST** enforce the §5.10 `MAX_DEPTH` limit
  *before* recursing, so a nested "depth bomb" cannot exhaust the stack; (b) it **MUST**
  bounds-check every declared length against the remaining input *before* allocating or
  copying, so a large declared length (e.g. a header claiming `2^63` elements) cannot
  drive unbounded allocation; a length exceeding the input is `truncated`.
- **Parser-differential / canonicalization attacks.** Because identity is the ContentId,
  two implementations that disagree on whether a body is canonical would assign
  different identities to "the same" value — an attack on ORCH-004 idempotency and on
  interoperability, not merely a cosmetic difference. The rejection rules (§5, §6) and
  the shared negative-vector corpus exist to close this: a conformant decoder rejects
  every non-canonical body, so no non-canonical body is ever addressed. The one
  **residual** gap is the NFC tier (§5.4): a *core-conformant* decoder that defers NFC
  (no Unicode table) may accept a non-NFC body that a *fully-conformant* decoder rejects.
  The two therefore agree only over NFC (valid) inputs; a runtime that must be safe
  against non-NFC inputs **MUST** be fully conformant (enforce §5.4).
- **Hash reliance and agility.** ACS-001's "equal ContentId ⇒ same value" identity
  relies on the **second-preimage and collision resistance of SHA-256**. Should SHA-256
  weaken, the self-describing multihash prefix (`0x12 0x20`) is the agility mechanism: a
  new hash is introduced as a new multihash code via CCP, and both codes may coexist
  during migration. Truncating or reusing the digest is forbidden.

---

*Ratification path (Reference Lifecycle): DRAFT → CCP-GATE (this doc + `ACS-002-CS-1`)
→ Candidate → Ratified. On ratification this becomes a registered normative
addition; the frozen v1.0 corpus is unchanged (ED-001).*

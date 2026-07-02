# ACS-003 / CCP-003 — Canonical Envelope Contract

**Type:** ARVES Core Standard (ACS) delivered as a Cognitive Change Proposal
Amendment (CCP-003). **Status:** RATIFIED v1.1 (CCP-GATE passed 2026-07; see `CCP-GATE-Ratification-v1.md`) —
normative at independence grade G1 (G2 external validation remains the open exit gate). **Program:** ARVES v1.1 Standardization, Goal 1 (Universal
Interoperability). **Closes:** the two divergent "event envelope" definitions in
the frozen corpus — the Vol 9 Event Envelope (`Vol 9 Runtime & Event Fabric Bible
v1`, Part 6; carried forward by `Vol 9 Cognitive Control Plane v2`, Part 11) and
the ARVES-21 "Canonical Event Envelope" (`ARVES_21_Event_Catalog_v1`) — which the
Gap Analysis flagged as under-specified and which Amendment-004 (SHARD-001)
explicitly lists as "Affected: … Vol 9 / ARVES-21 (envelope)". **Governs /
activates:** the frozen invariant **SHARD-001** (partition by tenant/workspace;
partition key immutable) at the envelope layer, and the **ORCH-003** (replayable)
/ **ORCH-004** (idempotent, content-addressable) properties for the interchange
unit that carries every event. This amendment ADDS the single normative envelope;
it does **not** edit any frozen document (ED-001; sanctioned via the Reference
Lifecycle CCP process).

> Normative keywords (MUST / SHALL / SHOULD / MAY / MUST NOT / SHALL NOT) are used
> per RFC 2119 and RFC 8174: they carry their normative meaning only when in
> ALL-CAPS. This document continues the Goal-4 Normative-Language Convention
> seeded by ACS-001 and ACS-002.

---

## 1. Problem

The frozen corpus defines the interchange envelope **twice**, and the two
definitions disagree:

- **Vol 9 (Runtime & Event Fabric Bible v1, Part 6)** — the *Event Envelope* has
  the fields `event_id, tenant_id, workspace_id, event_type, timestamp,
  correlation_id, payload`. Vol 9 (Cognitive Control Plane v2, Part 11) then
  asserts that engine invocations "carry `correlation_id` (already in the Event
  Envelope)".
- **ARVES-21 (Event Catalog v1)** — the *Canonical Event Envelope* has the fields
  `event_id, event_type, tenant_id, workspace_id, correlation_id, timestamp,
  source, payload, version`.

The two field sets differ (ARVES-21 adds `source` and `version`; the two documents
also list the fields in different orders), and **neither** fixes the property that
interoperability actually needs: the **types**, the **timestamp representation**,
the **serialization**, the **binding of the payload to a content address**, or the
**partition/shard binding**. Two independent runtimes given "the same event"
cannot produce the same bytes, cannot compute the same address for it, and cannot
replay or dedup it. The Gap Analysis records exactly this: "ARVES-21 lists envelope
fields but no per-event payload schema, versioning example, or sample message."

ACS-001 fixed the **address**; ACS-002 fixed the **body**. Nothing yet fixes the
**envelope that carries the body across a wire, a log, or a shard boundary**.
This standard defines it, once, normatively, and reconciles the two frozen
definitions into a single schema whose bytes are pinned by a conformance vector.

## 2. Scope

This standard defines the **ARVES Canonical Envelope**: the single normative
interchange unit that wraps every ARVES event and command payload. It fixes: the
complete field set with types; the `event_id` and timestamp formats (exact signed
64-bit nanoseconds, an ACS-002 **Integer**, explicitly **NOT** a binary64 float);
the tenant/shard binding under SHARD-001; the `payload_cid` field as the ACS-001
`ContentId` of the ACS-002 dCBOR payload body; the serialization of the envelope
itself via ACS-002; and the placement of the ACS-002 version tag **inside the
envelope but outside the body**. It reserves an ACS-001 domain tag for the
envelope and pins one CCP-GATE conformance vector (`ACS-003-CS-1`).

It does **not** define: the per-event **payload schema** of any concrete
`event_type` (that is the Event Catalog populated via ACS-004/005, the Type /
Schema Registry); the transport binding (Kafka/NATS/HTTP framing) beyond the
informative CloudEvents note in §9; the address format (**ACS-001**); or the
canonical body form (**ACS-002**), over which this standard composes.

## 3. Definitions (normative)

- **Canonical Envelope** — an ACS-002 **Map** value (§4) whose keys are the fields
  of §5, canonicalized per ACS-002 and, when addressed, hashed under the envelope
  domain tag (§7). It is itself an ARVES value; it is *not* a special format.
- **Payload** — the ARVES value transported by the envelope. Its canonical body is
  an ACS-002 dCBOR item; the envelope carries **only its ACS-001 `ContentId`**, not
  the body, in the `payload_cid` field (§5, §6).
- **Envelope `ContentId`** — the ACS-001 self-describing multihash of the envelope,
  computed as `1220 || SHA256(0x06 || canon(envelope))` (§7). Two envelopes with
  equal `ContentId` denote the same event delivery (ORCH-004 idempotency).
- **Serialization version tag (`ser_version`)** — the ACS-002 profile identifier
  (`"ACS-002/1"`) that fixes the encoding of both the envelope and its payload
  body. Carried as an envelope field (§5), it is *outside* the body (ACS-002 §7).

## 4. The envelope is an ACS-002 value (normative)

The Canonical Envelope **SHALL** be an ACS-002 **Map** (value-model kind 8) all of
whose keys are **Text** and all of whose values are ACS-002 values of the kinds in
§4 of ACS-002 (Null, Bool, Integer, Float, Text, Bytes, Array, Map). It **SHALL**
be serialized by ACS-002/1 dCBOR (§5.1–§5.9 of ACS-002): definite lengths,
shortest integers, bytewise-sorted encoded keys, NFC text, no tags, no
indefinite lengths, no trailing data. Consequently the field **order authored by a
producer is irrelevant**: the canonical byte form is fixed solely by ACS-002 map-key
ordering. There is exactly one canonical envelope body per envelope value.

An implementation **MUST NOT** invent a bespoke envelope wire format; the envelope
**SHALL** reuse ACS-002 so that the envelope, its address, and its payload's
address all live in one canonical regime.

## 5. Field set (normative)

The reconciliation rule is: **the Canonical Envelope is the union of the Vol 9 and
ARVES-21 fields**, retyped and completed for interoperability, with SHARD-001 and
the ACS-001/002 additions. Every field below is a Text key of the envelope Map. A
conformant envelope **SHALL** contain exactly the REQUIRED fields; OPTIONAL fields
**MAY** be present, and when present **SHALL** use the stated type. No other keys
**SHALL** appear (unknown keys are rejected — §6).

| Key | ACS-002 type | Req. | Origin | Meaning & rules |
|-----|--------------|------|--------|-----------------|
| `ser_version` | Text | **REQUIRED** | ACS-002 §7 | The serialization profile tag, `"ACS-002/1"`. Fixes the encoding of the envelope AND the payload body. Carried here so bodies stay minimal and non-self-describing (ACS-002 §7). |
| `event_id` | Text | **REQUIRED** | Vol 9 + ARVES-21 | Globally unique event identity, as a URN Text (`urn:arves:evt:…`). It identifies a *delivery*; it is **NOT** the content address. `ContentId` equality (ACS-001 §6), not `event_id`, is the idempotency identity. |
| `event_type` | Text | **REQUIRED** | Vol 9 + ARVES-21 | The dotted event name from the Event Catalog (e.g. `information.fact.committed`). Selects the payload schema (ACS-004/005). |
| `tenant_id` | Text | **REQUIRED** | Vol 9 + ARVES-21; **SHARD-001** | Primary partition/shard key (Amendment-004). Immutable for the entity lifetime (SHARD-001). MUST NOT be null or empty. |
| `workspace_id` | Text | **REQUIRED** | Vol 9 + ARVES-21; **SHARD-001** | Secondary partition/shard key (Amendment-004). Part of the immutable partition key. MUST NOT be null. |
| `correlation_id` | Text | **REQUIRED** | Vol 9 + ARVES-21 | Correlation/routing key (Amendment-004: routing key = `tenant_id + correlation_id`). Ties an event to its cognitive run/trace for replay (ORCH-003; CAP-007). |
| `causation_id` | Text \| Null | OPTIONAL | reconciliation | The `event_id`/id of the immediate cause, or **Null** when the event is a root (present-with-Null is distinct from absent — ACS-002 §5.7). Enables causal ordering without recomputation. |
| `source` | Text | **REQUIRED** | ARVES-21 | Producer identity as a URN (`urn:arves:svc:…`). Present in ARVES-21, absent in Vol 9; the reconciliation KEEPS it and makes it REQUIRED for provenance/audit (Vol 9 Part 22). |
| `occurred_at` | Integer | **REQUIRED** | Vol 9 + ARVES-21 (`timestamp`) | The event time as **signed 64-bit nanoseconds since the Unix epoch** — the ontology `Timestamp`. It **SHALL** be an ACS-002 **Integer** (major 0/1), and **SHALL NOT** be a Float. This is the renamed, retyped `timestamp` field (see §5.1). |
| `schema_version` | Integer | **REQUIRED** | ARVES-21 (`version`) | The payload contract version for `event_type` (ARVES-21 "Version"; O-006 "every type is versioned"). An **Integer**, never a Float. Distinct from `ser_version`. |
| `payload_domain` | Integer | **REQUIRED** | ACS-001 | The ACS-001 domain tag (§4 of ACS-001) under which `payload_cid` was computed (e.g. `1` commit-content, `5` decision-trace). Makes the address self-checking without fetching the body. |
| `payload_cid` | Bytes | **REQUIRED** | ACS-001 | The ACS-001 `ContentId` of the payload's ACS-002 dCBOR body: `1220 ‖ SHA256(payload_domain ‖ canon(payload))`. Carried as **Bytes** (the 34-byte multihash verbatim), **NOT** as a hex Text, so it is byte-canonical. The body itself is **NOT** in the envelope. |

The envelope **SHALL NOT** carry the raw payload body; it carries only
`payload_cid` (+ `payload_domain`). This keeps envelopes small, makes the payload
independently dedup-able and cacheable by address, and lets a consumer verify the
payload it fetches against `payload_cid` (ACS-001 §6).

### 5.1 Timestamp reconciliation (normative)

Both frozen definitions name a field `timestamp` but neither fixes its
representation. ACS-003 fixes it as the field `occurred_at`, typed as an ACS-002
**Integer** holding **signed 64-bit nanoseconds since the Unix epoch** (the frozen
ontology `Timestamp`, i64). An implementation **MUST NOT** encode `occurred_at` as
an ACS-002 Float (binary64): a nanosecond timestamp exceeds `2^53` and a binary64
representation silently rounds it — the exact JCS failure ACS-002 §9 rejects. A
producer that emits `occurred_at` as a Float, a decimal string, or an RFC 3339
Text **SHALL** be non-conformant. (An RFC 3339 rendering, if ever needed for
humans, is a display concern outside the canonical envelope.)

### 5.2 SHARD-001 binding (normative)

Under Amendment-004 and SHARD-001, the partition/shard key is `tenant_id`
(primary) + `workspace_id` (secondary), and the routing key is `tenant_id` +
`correlation_id`. Therefore `tenant_id` and `workspace_id` are **REQUIRED** and
non-null in every envelope, and the pair **SHALL** be treated as immutable for the
lifetime of the entity the event concerns (SHARD-001). An envelope missing either,
or carrying a null/empty `tenant_id` or `workspace_id`, **SHALL** be non-conformant.
Cross-tenant data **SHALL NOT** share a shard (Amendment-004); the envelope carries
the keys that make that partitioning decidable at the edge without decoding the
payload.

### 5.3 `event_id` format (normative)

`event_id` **SHALL** be a Text URN that is globally unique. Producers **SHOULD**
use a monotonic, time-sortable identifier form (e.g. a UUIDv7 or ULID rendered as
`urn:arves:evt:<id>`) so that log order approximates causal order, but the standard
constrains only that it is a unique Text; it is opaque to conformance. `event_id`
identifies a delivery and **SHALL NOT** be used as the content-identity of the
payload — that is `payload_cid` (ACS-001 §6, §7). Two redeliveries of the same
event carry the same `event_id`; two distinct events **SHALL** carry distinct
`event_id`s.

## 6. Encoding, addressing, and validation (normative)

1. **Encode.** The envelope body **SHALL** be `canon(envelope)` under ACS-002/1
   (§4 here). Keys are sorted by their encoded-key bytes; `occurred_at` and
   `schema_version` and `payload_domain` are Integers; `payload_cid` is Bytes;
   `causation_id` is either a Text or the explicit Null value.
2. **Address.** The envelope `ContentId` **SHALL** be
   `1220 ‖ SHA256(0x06 ‖ canon(envelope))` (ACS-001 §5 with the envelope domain
   tag §7). The payload `ContentId` carried in `payload_cid` **SHALL** be
   `1220 ‖ SHA256(payload_domain ‖ canon(payload))`.
3. **Validate.** A conformant decoder **MUST** reject an envelope that: is not
   canonical ACS-002 (per ACS-002 §6 — non-shortest int, unsorted/duplicate keys,
   indefinite length, non-NFC text, non-finite/-0.0 float, tags, trailing data);
   is missing any REQUIRED field; carries an unknown key; types any field contrary
   to §5 (notably `occurred_at`/`schema_version`/`payload_domain` as non-Integer,
   or `payload_cid` as non-Bytes); carries a `payload_cid` that is not a
   well-formed 34-byte `0x12 0x20 ‖ 32-byte` SHA-256 multihash; or carries a null
   or empty `tenant_id`/`workspace_id`.
4. **Verify-on-fetch.** A consumer that fetches the payload body **SHOULD**
   recompute `1220 ‖ SHA256(payload_domain ‖ body)` and **MUST** reject the body if
   it does not equal `payload_cid` (ACS-001 §6). This makes the envelope a tamper-
   evident, content-checked reference.

Determinism obligation: for a given envelope value, `canon(envelope)` **SHALL**
yield identical bytes on every conformant implementation (ACS-002 §6.1), and hence
an identical envelope `ContentId` — the precondition for cross-runtime dedup,
replay, and certification.

## 7. Domain tag reservation (normative)

ACS-001 §4 RESERVES tags `0x06`–`0x7F` "for future ACS standards." This standard
CLAIMS tag `0x06` for the **canonical-envelope** domain and adds it to the ACS-001
domain registry via this CCP amendment:

| Tag | Domain | Defined by |
|-----|--------|-----------|
| `0x06` | canonical-envelope (an ACS-003 envelope) | ACS-003 (this document) |

An implementation **SHALL** compute an envelope `ContentId` under `0x06` and
**MUST NOT** reuse `0x06` for any other domain, per the ACS-001 domain-separation
rule. Tags `0x07`–`0x7F` remain RESERVED. This registration is additive to
ACS-001's frozen-corpus-independent registry; it does not edit ACS-001's text
(ED-001) — it extends the registry ACS-001 itself declared open.

## 8. Version tag placement (normative)

The ACS-002 serialization version is carried in the envelope field `ser_version`
(= `"ACS-002/1"`). It is **inside the envelope Map but outside the payload body**,
satisfying ACS-002 §7's rule that the version is a property of the *scheme*, not of
any value, and MUST NOT be embedded in a body. `ser_version` fixes the encoding of
**both** the envelope and its referenced payload body. A future ACS-002 minor/major
profile (a new dCBOR tag set, an unordered-set kind, etc.) **SHALL** be introduced
by its own CCP with its own tag value and a distinct `ser_version` string; bodies
and envelopes addressed under `"ACS-002/1"` remain valid and re-derivable forever
because the address is self-describing (ACS-001 §5) and the producing profile is
pinned here. An envelope **SHALL NOT** embed a serialization version inside the
payload body.

## 9. CloudEvents binding (informative)

The Canonical Envelope maps cleanly onto CNCF **CloudEvents v1.0** context
attributes for teams that must bridge to existing eventing infrastructure. This
mapping is **informative only**; the normative interchange form is the ACS-002
dCBOR envelope of §5, and a CloudEvents rendering is a *transport binding*, not the
canonical value.

| Canonical Envelope | CloudEvents v1.0 | Note |
|--------------------|------------------|------|
| `event_id` | `id` | Both are producer-unique delivery ids. |
| `source` | `source` | URN producer identity. |
| `event_type` | `type` | Dotted event name. |
| `occurred_at` (i64 ns) | `time` (RFC 3339) | **Lossy across the boundary**: RFC 3339 is millis/micros-typical; the canonical i64-ns value is authoritative and MUST be preserved as an extension attribute (e.g. `occurredatnanos`) or reconstructed from the envelope, never re-derived from `time`. |
| `payload_cid` | `dataschema` / extension `payloadcid` | The content address; CloudEvents has no native content-address slot, so carry it as an extension. |
| `tenant_id`, `workspace_id`, `correlation_id`, `causation_id`, `schema_version`, `payload_domain`, `ser_version` | extension attributes (`tenantid`, `workspaceid`, `correlationid`, `causationid`, `schemaversion`, `payloaddomain`, `serversion`) | CloudEvents extension-attribute names are lowercase-alphanumeric; the mapping strips underscores. |
| payload body (fetched by `payload_cid`) | `data` (+ `datacontenttype: application/cbor`) | The body is dCBOR; CloudEvents carries it as binary `data`. |

A binding **MUST NOT** treat the CloudEvents `time` attribute as the authoritative
timestamp: `occurred_at` (i64 ns) is authoritative, precisely because a binary64 or
RFC 3339 rendering cannot round-trip nanoseconds (§5.1, ACS-002 §9). Conformance is
evaluated on the ACS-002 dCBOR envelope, never on a CloudEvents rendering.

## 10. Conformance scenario (CCP-GATE requirement) — `ACS-003-CS-1`

A conformant implementation, given the envelope value below, **SHALL** produce
exactly the stated canonical body and exactly the stated envelope `ContentId`. Two
independent implementations **SHALL** agree on all bytes (differential
conformance). The body is `ACS-002/1` dCBOR; the envelope `ContentId` is
`1220 ‖ SHA256(0x06 ‖ body)`. The `ContentId`s below were independently
re-derived from the pre-image bytes with a second, unrelated hashing toolchain
(OpenSSL `dgst -sha256`) and matched byte-for-byte.

### 10.1 The payload (addressed first)

The payload is the ACS-002-CS-1 V1 fact value, addressed under ACS-001 domain
`0x01` (commit-content):

- **Payload value:** Map `{ "type": "uci.fact", "claim": "sky-is-blue",
  "confidence": Float 0.5, "observed_at": Integer 1730000000000000000 }`
- **Payload canonical body (hex):**
  `a46474797065687563692e6661637465636c61696d6b736b792d69732d626c75656a636f6e666964656e6365fb3fe00000000000006b6f627365727665645f61741b180231d5856d0000`
- **Payload `ContentId` (hex) = `payload_cid`:**
  `12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e`
  (OpenSSL SHA-256 of `0x01 ‖ body` = `4284f0ac…363e` ✓)

### 10.2 The canonical envelope

- **`ser_version`** = Text `"ACS-002/1"`
- **`event_id`** = Text `"urn:arves:evt:01J8ZK9M4Q2N7C3F"`
- **`event_type`** = Text `"information.fact.committed"`
- **`tenant_id`** = Text `"acme"`
- **`workspace_id`** = Text `"research"`
- **`correlation_id`** = Text `"urn:arves:corr:c1"`
- **`causation_id`** = **Null** (present with Null — root event, §5.7 of ACS-002)
- **`source`** = Text `"urn:arves:svc:information-core"`
- **`occurred_at`** = Integer `1730000000000000000` (exact i64 ns)
- **`schema_version`** = Integer `1`
- **`payload_domain`** = Integer `1`
- **`payload_cid`** = Bytes `12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e` (the 34-byte multihash from §10.1, verbatim)

**Canonical envelope body (hex), `ACS-002/1` dCBOR, 327 bytes:**

```
ac66736f75726365781e75726e3a61727665733a7376633a696e666f726d6174696f6e2d
636f7265686576656e745f6964781e75726e3a61727665733a6576743a30314a385a4b39
4d3451324e374333466974656e616e745f69646461636d656a6576656e745f7479706578
1a696e666f726d6174696f6e2e666163742e636f6d6d69747465646b6f636375727265645f
61741b180231d5856d00006b7061796c6f61645f636964582212204284f0acb42a473063
3fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e6b7365725f76657273696f6e69
4143532d3030322f316c636175736174696f6e5f6964f66c776f726b73706163655f6964
6872657365617263686e636f7272656c6174696f6e5f69647175726e3a61727665733a63
6f72723a63316e7061796c6f61645f646f6d61696e016e736368656d615f76657273696f
6e01
```

(Concatenate the lines with no separators; the whitespace above is presentational.)

**Envelope `ContentId` (hex) = `1220 ‖ SHA256(0x06 ‖ body)`:**

```
1220fc0ef055e4d39de1c3ab7d2597361d24f7a8b6a1a0609a91b872b85ae4896f93
```

(OpenSSL SHA-256 of `0x06 ‖ body` = `fc0ef055…6f93` ✓, independently confirmed.)

### 10.3 Field-by-field encoding (what each byte pins)

Keys are emitted in bytewise order of their **encoded** form (ACS-002 §5.6),
which is *not* alphabetical on the raw strings — e.g. `source` (6 chars) sorts
before `event_id` (8 chars) because the shorter Text header `66` precedes `68`:

| Sorted key | encoded key (hex) | value (hex) | pins |
|------------|-------------------|-------------|------|
| `source` | `66736f75726365` | `781e75726e3a61727665733a7376633a696e666f726d6174696f6e2d636f7265` | ARVES-21 `source` retained, REQUIRED URN |
| `event_id` | `686576656e745f6964` | `781e75726e3a61727665733a6576743a30314a385a4b394d3451324e37433346` | delivery id, not content id |
| `tenant_id` | `6974656e616e745f6964` | `6461636d65` | SHARD-001 primary key |
| `event_type` | `6a6576656e745f74797065` | `781a696e666f726d6174696f6e2e666163742e636f6d6d6974746564` | Event Catalog dotted name |
| `occurred_at` | `6b6f636375727265645f6174` | `1b180231d5856d0000` | **exact i64 ns** (`1b…` = 8-byte Integer = 1 730 000 000 000 000 000), NOT float |
| `payload_cid` | `6b7061796c6f61645f636964` | `582212204284…363e` | Bytes(34) = ACS-001 multihash of the dCBOR body |
| `ser_version` | `6b7365725f76657273696f6e` | `694143532d3030322f31` | `"ACS-002/1"` version tag, outside the body |
| `causation_id` | `6c636175736174696f6e5f6964` | `f6` | **Null present, not absent** (§5.7) |
| `workspace_id` | `6c776f726b73706163655f6964` | `687265736561726368` | SHARD-001 secondary key |
| `correlation_id` | `6e636f7272656c6174696f6e5f6964` | `7175726e3a61727665733a636f72723a6331` | routing/replay correlation |
| `payload_domain` | `6e7061796c6f61645f646f6d61696e` | `01` | ACS-001 domain of `payload_cid` |
| `schema_version` | `6e736368656d615f76657273696f6e` | `01` | payload contract version (Integer, not float) |

The leading `ac` is a definite-length Map of 12 entries (major 5, `0x0c`).

### 10.4 Non-conformance
An implementation **SHALL** be non-conformant if it: encodes `occurred_at` or
`schema_version` or `payload_domain` as a Float, decimal Text, or RFC 3339 Text;
carries `payload_cid` as a hex Text or an unprefixed 32-byte digest instead of the
34-byte Bytes multihash; drops the `causation_id: Null` entry (§5.7); reorders map
keys other than by encoded-key bytes; omits any REQUIRED field or admits an unknown
key; embeds `ser_version` inside the payload body; or diverges from the body /
`ContentId` above by a single byte. An implementation that addresses the envelope
under any domain tag other than `0x06`, or that collides the envelope address with
a bare-Map commit-content address by omitting the domain tag, **SHALL** be
non-conformant.

## 11. Proposal analysis

- **Why it matters.** The envelope is the atom of interchange: every event,
  command, and cross-shard message is an envelope. With two frozen definitions and
  no fixed types, "the same event" has no canonical bytes, so it has no address, so
  it cannot be deduped, replayed (ORCH-003), idempotently handled (ORCH-004), or
  cross-certified. ACS-003 collapses the two definitions into one and makes the
  envelope a first-class ACS-002 value with an ACS-001 address — the smallest
  change that makes the entire event fabric byte-exact and differentially testable.
- **The reconciliation, precisely.** The union of Vol 9 and ARVES-21 is taken
  (ARVES-21's `source` and `version` are KEPT — dropping provenance/versioning
  would regress auditability, Vol 9 Part 22, and O-006). `timestamp` is retyped and
  renamed `occurred_at` as an exact i64-ns Integer (§5.1); `version` is renamed
  `schema_version` (Integer) to free the word "version" from ambiguity with
  `ser_version`. SHARD-001/Amendment-004 supplies the tenant/workspace binding —
  Amendment-004 already names "Vol 9 / ARVES-21 (envelope)" as affected, so this is
  a *decision the frozen corpus deferred to a CCP*, not an invention. The ACS layer
  adds only what addressing requires: `payload_cid`, `payload_domain`, `ser_version`.
- **Risks.** (a) *Two documents disagree — which wins?* Neither: the CCP supersedes
  both with a superset, and cites Amendment-004's own "Affected" note as the
  sanctioned reconciliation hook, so no frozen text is contradicted. (b) *Envelope
  carries an address, not the body — extra fetch.* Deliberate: it keeps envelopes
  small, makes payloads independently cacheable/dedup-able, and makes tampering
  detectable (§6.4); a co-located body is a transport optimization, not a change to
  the canonical envelope. (c) *Domain-tag land grab.* Mitigated: ACS-001 already
  RESERVED `0x06`–`0x7F` for exactly this, and the claim is registered here, not
  silently. (d) *CloudEvents `time` lossiness.* Called out normatively (§9) so a
  bridge cannot quietly downgrade nanosecond truth.
- **Long-term consequences.** One envelope, addressed and self-verifying, means an
  event logged in 2026 is byte-reproducible and re-addressable in 2046 regardless of
  which runtime reads it — the ORCH-003 replay guarantee made physical. Carrying the
  address (not the body) in the envelope lets storage, cache, and registry layers
  evolve independently of the event fabric.
- **Alternatives considered.** (i) *Bless one of the two frozen envelopes as-is* —
  rejected: both lack types, timestamp representation, serialization, and an
  address; picking one still leaves the interop gap and orphans the other's fields.
  (ii) *Embed the payload body in the envelope* — rejected: bloats every message,
  defeats content-dedup, and duplicates the body/address regime. (iii) *Carry
  `payload_cid` as hex Text* — rejected: a second encoding of the same address
  invites divergence; Bytes(34) is the one canonical form. (iv) *Timestamp as RFC
  3339 Text or binary64* — rejected on the exact ACS-002 §9 grounds (nanoseconds
  exceed 2^53 / round-trip loss). (v) *Adopt CloudEvents as the normative form* —
  rejected: CloudEvents has no canonical binary form, no content-address slot, and a
  lossy time model; it is retained as an informative binding only.
- **Recommendation.** Ratify via CCP-GATE with `ACS-003-CS-1`. Then a Runtime task
  adopts it in the event fabric (`arves-runtime` / event bus): the envelope encoder
  reuses the ACS-002 codec and the ACS-001 `ContentId` construction; the Event
  Catalog (ARVES-21) is regenerated against this one schema; per-event payload
  schemas are layered by ACS-004/005. The WAL's CRC-32 remains frame-integrity only
  and is unaffected.
- **Implementation complexity.** Standard: low–medium (it is a field list plus two
  reused primitives). Reference-runtime adoption: low — given an ACS-002 codec and
  ACS-001 addressing, the envelope is a typed Map plus a validator; the strictness
  (reject unknown keys, enforce Integer timestamp, verify multihash shape) is the
  main care point.
- **Scientific impact.** Turns "canonical event envelope" from a phrase that meant
  two different things into a single, differentially-testable byte string with a
  published, independently-recomputed vector — the envelope becomes a falsifiable,
  cross-implementation property.
- **Ecosystem impact.** Unblocks the second independent runtime (Program C),
  cross-runtime event replay and dedup, an addressable event store, and the
  differential-conformance tier (Goal 3). It is the precondition for two certified
  runtimes to exchange a single event and agree, byte for byte, on what it was.

## 12. Dependencies & sequence
- **Blocks:** the populated Event Catalog (ARVES-21 per-event payload schemas via
  ACS-004/005), cross-runtime event replay/dedup (ORCH-003/004), the addressable
  event store, and the differential-conformance tier (Goal 3) for events.
- **Depends on:** ACS-001 (address format + domain registry; this standard claims
  tag `0x06`), ACS-002 (the envelope IS an ACS-002 value and carries `ser_version`),
  RFC 8949 §4.2 (deterministic CBOR), SHA-256, and Unicode NFC (via ACS-002 Text).
- **Grounded in (frozen, not edited):** Vol 9 Runtime & Event Fabric Bible v1 Part 6
  and Vol 9 Cognitive Control Plane v2 Part 11 (the Event Envelope); ARVES-21 Event
  Catalog v1 (the Canonical Event Envelope + Event Contract Template); Amendments
  CCP Batch 1 Amendment-004 / SHARD-001 (tenant/workspace partition + routing keys,
  "Affected: … Vol 9 / ARVES-21 (envelope)"); Invariant Registry v1 (ORCH-003/004,
  SHARD-001); Universal Cognitive Ontology v1 (`Timestamp` i64, `EntityUrn`, O-006
  versioning); Gap Analysis v1 (envelope under-specification).

---

*Ratification path (Reference Lifecycle): DRAFT → CCP-GATE (this doc +
`ACS-003-CS-1`) → Candidate → Ratified. On ratification this becomes a registered
normative addition and registers ACS-001 domain tag `0x06`; the frozen v1.0 corpus
is unchanged (ED-001).*

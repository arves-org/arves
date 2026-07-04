# ACS-004 / CCP-004 — Universal Type Registry

**Type:** ARVES Core Standard (ACS) delivered as a Cognitive Change Proposal
Amendment (CCP-004). **Status:** RATIFIED v1.1 (CCP-GATE passed 2026-07; see `CCP-GATE-Ratification-v1.md`) —
normative at independence grade G1 (G2 external validation remains the open exit gate). **Program:** ARVES v1.1 Standardization, Goal 1 (Universal
Interoperability). **Closes:** Global Readiness Report **R-03** (`uci.*` Type
Schemas v1 — "populate the reserved `schema` slot for all root types + aspects +
relations, plus additive schema-evolution rules"), the keystone finding P04-F1
("Ontology Type Registry has no field-level schemas") and P01-F1. **Governs /
activates:** the frozen Ontology-Spec obligation that *"the ABI loop now closes:
Engine Manifest → Input Type → Ontology Registry → Output Type"* (Universal
Cognitive Ontology Specification, Part 10) and the frozen registry-entry shape
`{ urn, version, aspects, schema, relations }` (Ontology Spec, Part 9) whose
`schema` slot was reserved but **never populated**. It activates the design
principle **`O-006`** (*every type is versioned and registered*) by giving it an
executable, byte-exact form. This amendment ADDS the type registry and the schema
of one fully-worked type; it does **not** edit any frozen document (ED-001;
sanctioned via the Reference Lifecycle CCP process, Part 6 CCP-GATE).

> Normative keywords (MUST / MUST NOT / SHALL / SHALL NOT / SHOULD / SHOULD NOT /
> MAY / REQUIRED / RECOMMENDED / OPTIONAL) are used per RFC 2119 as updated by
> RFC 8174: they carry normative force only when in ALL-CAPS. This document
> continues the Goal-4 Normative-Language Convention seeded by ACS-001/002 and
> defined by ACS-005 (CCP-005).

---

## 1. Problem

The Universal Cognitive Ontology Specification declares itself *"the ROOT of UCS"*
and *"the normative dependency that closes the Engine Graph ABI (Reads/Writes/
Produces resolve to types defined here)"* (Ontology Spec, Part 2). Part 9 defines
a registry entry as `{ urn, version, aspects, schema, relations }` and Part 10
asserts *"the ABI loop now closes: Engine Manifest → Input Type → Ontology
Registry → Output Type."* But the frozen corpus **never publishes a single
`schema`**: Table 2 lists 18 root-type URNs (`uci.fact`, `uci.observation`,
`uci.goal`, …) with one-line English meanings, and Part 4 names five mandatory
aspects (Identity / Provenance / Trust / Temporal / TenantScope) — with **no field
list, no field type, no cardinality, no required/optional marking, and no
encoding** for any of them. The "type loop" that the ABI relies on therefore
closes onto **empty `uci.*` types**.

The consequence is fatal for a *standard*: an independent implementer cannot
construct a `uci.fact@1` value, cannot validate one received from another runtime,
and cannot compute a content address over one (ACS-001) because the canonical
field set is undefined. The reference runtime today sidesteps this by carrying the
Kernel payload as opaque bytes (`arves-kernel` `ProposedWrite`, *"typed later via
`arves-ontology`"*), and `arves-ontology` ships the **contracts** — the aspect
traits `Identity`/`Provenance`/`Trust`/`Temporal`/`TenantScope`, the value types
`EntityUrn`, `TypeVersion{major,minor}`, `Confidence(f64)∈[0,1]`, `Timestamp(i64)`
ns, `Origin{Observed|Derived|Asserted}`, `ProvenanceRecord`, `BiTemporal`,
`ShardKey{tenant,workspace}`, and a `TypeRegistry` trait whose population is
*"out of scope for this skeleton."* The vocabulary exists; the **executable type
definitions do not**. ACS-004 supplies them.

ACS-001 (addressing) and ACS-002 (serialization) gave ARVES byte-exact identity
and byte-exact bodies for the abstract value model. They deliberately do **not**
say how a concrete cognitive type (`uci.fact`) *maps onto* that value model —
ACS-002 §2 explicitly defers this to "ACS-004/005 (Type / Schema Registry)". This
standard is that missing complement: it fixes, byte-for-byte, **what fields a
`uci.*` type has, of what value-model kind, with what cardinality, how the type
itself is identified, how it evolves, and how an instance serializes.** It also
resolves the one type-layer question ACS-002 left open (§4 below).

## 2. Scope

This standard defines the **ARVES Universal Type Registry**: the normative,
machine-readable, content-addressed form of the frozen ontology's type system. It
fixes:

- the **schema encoding** (§3) — one normative language, justified against
  alternatives;
- **type identity** (§5) — how a `uci.*` type is named and content-addressed via
  ACS-001;
- the **schema document** (§6) — the dCBOR shape of a registry `schema` entry, its
  field descriptors, cardinality vocabulary, and the ACS-004 type-code set;
- **integer-vs-float typing** (§7) — resolving the type-layer question ACS-002 §4
  deferred;
- the **aspect binding** (§8) — how the five mandatory aspects attach exactly once;
- **type versioning and compatibility** (§9) — additive/minor vs breaking/major;
- **instance serialization** (§10) — how a typed value becomes an ACS-002 body and
  an ACS-001 `ContentId`;
- one **fully-worked vertical slice**, `uci.fact@1` (§11), end-to-end with
  computed byte-exact vectors, as the template for the remaining types.

It does **not** define: the canonical byte form of the value model (that is
**ACS-002**, on which this standard is built); the address format (that is
**ACS-001**); the wire envelope that carries the schema-version tag on the network
(that is **ACS-003**, a forward dependency); the *population* of every remaining
`uci.*` type and the domain-relation schemas (that is the **ACS-005 Schema
Registry** work item — this standard fixes the format and one slice as its
normative template, per the R-03 mitigation "deepen one vertical slice end-to-end
first, then template"); nor storage, indexing, or query of instances (Ontology
`O-007`: *the ontology defines meaning, not storage*).

## 3. Normative schema encoding — CDDL over dCBOR (the decisive choice)

A `uci.*` type schema **SHALL** be expressed in two byte-equivalent, normative
forms with a strict precedence order:

1. **The authoritative form** is the **ACS-004 schema document** (§6): a single
   ARVES value (a Map) serialized as an **ACS-002/1 dCBOR body** and identified by
   its ACS-001 `ContentId` (§5). This form is what implementations compute over,
   exchange, and certify against; it inherits ACS-002 determinism verbatim.
2. **The human-readable projection** is **CDDL — Concise Data Definition Language,
   RFC 8610** — the IETF schema language *for* CBOR/dCBOR. Every schema document
   §6 **SHALL** have an equivalent CDDL rule, and this document publishes it for
   the worked slice (§11.1). Where the CDDL prose and the dCBOR schema document
   disagree, the **schema document is authoritative** (it is the addressed
   artifact).

A conformant registry **MUST** treat the dCBOR schema document as the source of
truth; the CDDL projection is RECOMMENDED for documentation and tooling and
**MUST NOT** be used as the content-addressed identity of a type.

**Why CDDL/dCBOR and not JSON Schema 2020-12, CUE, or Protobuf.** The task requires
one normative encoding justified against these three. The decision follows
directly from the stack ACS-004 sits on:

- **The data model is already dCBOR (ACS-002).** ACS-002 fixed the value model to
  eight kinds with an *exact 64-bit Integer distinct from binary64 Float*, NFC
  Text, bytewise-sorted map keys, and definite lengths. CDDL (RFC 8610) is the
  schema language the IETF designed for *exactly this* data model; it names Integer
  and Float, byte strings vs text strings, arrays, and maps as first-class,
  distinct constructs. Choosing CDDL means the schema layer and the byte layer
  share one type universe with **zero impedance mismatch** — the single most
  important property for a 20-year, independently-implementable standard.

- **JSON Schema 2020-12 — rejected.** JSON's value model has *one* number type
  (IEEE-754 binary64 under I-JSON / ECMAScript). ACS-002 rejected JCS for precisely
  this reason: the ontology's `Timestamp` is a signed 64-bit nanosecond integer
  whose magnitude exceeds 2⁵³ and cannot survive a binary64 round-trip. A *schema*
  language that cannot even *name* an exact int-vs-float distinction (JSON Schema's
  `type: "integer"` is a value-range assertion over the same binary64, not a
  distinct wire type) cannot be the normative schema for a format whose whole point
  is that distinction. JSON Schema is excellent tooling and human ergonomics — it
  is retained *only* as an optional, non-normative documentation projection, never
  as identity.

- **CUE — rejected as the normative encoding.** CUE is a powerful
  configuration/constraint language (unification lattice, defaults, constraints)
  and would model the ontology elegantly. But it is (a) a single-ecosystem
  implementation (one reference tool, Go) rather than an IETF-frozen, multiply-
  implemented specification — a governance risk over a 20-year horizon; (b) far
  larger in surface than the field-descriptor subset ARVES needs, expanding the
  independent-reimplementation cost the Ontology-Spec Part 11 test forbids; and (c)
  not natively a CBOR schema language — it would require its own value-model
  binding, reintroducing the impedance mismatch CDDL removes. CUE is a strong
  candidate *if* ARVES ever needs cross-field constraint logic; that is a future
  ACS layered *above* this one, not the base type registry.

- **Protobuf — rejected.** Protobuf/FlatBuffers have the richest tooling, but
  ACS-002 §9 already recorded the disqualifier: Protobuf is **not canonical** —
  field order and default/absent handling are unspecified across implementations,
  and its wire form conflates *absent* with *default-valued*, which directly
  violates ACS-002 §5.7 (Null-vs-absent is identity-bearing). It also introduces
  field-number governance (a parallel, non-content-addressed identity space) that
  duplicates the URN/ContentId identity this standard defines. A schema encoding
  whose own wire form is non-canonical cannot sit under a content-addressing
  standard.

**Summary.** CDDL-over-dCBOR is the only candidate that (i) shares ACS-002's exact
value model, (ii) is an open, IETF-frozen, multiply-implementable specification,
(iii) keeps the independent-reimplementation surface minimal, and (iv) composes
directly with ACS-001 content addressing. The other three are retained, at most,
as **non-normative projections**.

## 4. Position in the ABI type loop (normative)

The Engine Graph ABI resolves `Reads`/`Writes`/`Produces` to `uci.*` URNs, and the
Ontology Spec Part 10 asserts the loop *closes* through the registry. ACS-004
makes that closure executable:

```
Engine Manifest.Reads = [ uci.fact@1 ]
      │
      ▼  resolve URN@version  (§5)
ACS-004 Type Registry ── entry ─▶ { urn, version, aspects, schema (§6), relations }
      │                                    │
      │ schema.ContentId (ACS-001, §5)     │ field schema (§6,§7,§8)
      ▼                                    ▼
Engine Manifest.Produces = [ uci.fact@1 ] ── instance validates (§6.4) ──▶ ACS-002 body ──▶ ACS-001 ContentId (§10)
```

A conformant runtime **SHALL** resolve every `uci.*` URN referenced by an engine
manifest to exactly one ACS-004 registry entry whose schema document has a fixed
`ContentId`; two runtimes resolving the same `urn@version` **SHALL** obtain schema
documents with byte-equal `ContentId`s. A URN that resolves to no registry entry,
or to a schema document with a diverging `ContentId`, **SHALL** be a conformance
FAIL — the ABI loop does not close on an empty or divergent type.

## 5. Type identity (normative)

### 5.1 URN form

A registered type **SHALL** be named by a URN of the frozen `arves-ontology` form:

```
uci.<name>@<major>.<minor>          (short registry form, e.g. uci.fact@1.0)
urn:arves:<namespace>:<name>@<major>.<minor>:<local-id>   (full EntityUrn, §10)
```

The short form `uci.<name>@<major>.<minor>` identifies a **type**; the full
`EntityUrn` identifies an **instance** of that type (its `type-name@version`
segment MUST equal the type's short form modulo namespace). `<major>` and
`<minor>` are the `TypeVersion` components from `arves-ontology`
(`O-006`). The `@<minor>` MAY be elided in prose to mean "the highest ratified
minor of that major"; on the wire and in a registry entry it **SHALL** be explicit.

### 5.2 Content-addressed type identity

Beyond the URN (a human/logical name), every registered type **SHALL** additionally
have a **content-addressed identity**: the ACS-001 `ContentId` of its ACS-004
schema document (§6), computed under the new domain tag **`0x07` type-schema**
(§5.3). Two independently authored registries that publish the same type
**SHALL** produce byte-equal schema documents and therefore byte-equal schema
`ContentId`s. The schema `ContentId` is the sanctioned cross-runtime identity test
for "the same type at the same version"; a matching URN with a diverging schema
`ContentId` **SHALL** be treated as two different types (a governance error to be
resolved via §9, never silently merged).

### 5.3 Domain tag allocation

This standard allocates the tag `0x07` from the ACS-001 §4 RESERVED range
`0x06–0x7F`. (Tag `0x06` is allocated to the **canonical-envelope** domain by
ACS-003; the ACS-001 domain-tag registry records both allocations and is the
single authority.)

| Tag | Domain | Addressed value |
|-----|--------|-----------------|
| `0x07` | **type-schema** | an ACS-004 schema document (§6) |

An implementation **SHALL** compute a type's content-addressed identity as
`multihash(SHA-256, SHA256(0x07 ‖ schema-body))` where `schema-body` is the
ACS-002/1 dCBOR body of the schema document. It **MUST NOT** address a schema
document under any other tag, and **MUST NOT** reuse `0x07` for a non-schema value.
Tags `0x08–0x7F` remain RESERVED for future ACS standards (ACS-005 MAY allocate a
tag for a *relation* schema).

## 6. The schema document (normative)

An **ACS-004 schema document** is an ARVES value of kind **Map** (ACS-002 §4 kind
8) — the machine-readable `schema` entry the Ontology Spec Part 9 reserved. It is
serialized as an ACS-002/1 dCBOR body and addressed per §5.2. It **SHALL** contain
exactly the following top-level Text keys, and no others:

| Key | Value kind | Cardinality | Meaning |
|-----|-----------|-------------|---------|
| `urn` | Text | 1 | short type URN without version, e.g. `"uci.fact"` |
| `ver` | Map `{major:Int, minor:Int}` | 1 | the `TypeVersion` (`O-006`) |
| `root` | Text | 1 | the frozen `RootType` name (§6.1) |
| `aspects` | Array of Text | 1 | the mandatory aspects this type carries (§8) |
| `fields` | Map of Text → **field descriptor** (§6.2) | 1 | the type's fields |

A schema document **MUST NOT** contain a `relations` key in ACS-004; semantic
relations (Ontology Spec, Part 7 — `supports`, `derived_from`, …) are the province
of the ACS-005 Schema/Relation Registry and are addressed there. (The frozen
registry-entry shape `{ urn, version, aspects, schema, relations }` is honored
across the ACS-004/005 pair: ACS-004 fixes `urn`, `version`, `aspects`, `schema`
(the fields); ACS-005 fixes `relations`.)

### 6.1 Root-type vocabulary

`root` **SHALL** be exactly one member name of the frozen `RootType` space realized
in `arves-ontology`: `Observation`, `Event`, `Fact`, `Goal`, `Entity`, `Relation`,
`Inference`, `Policy`. (These are the composed, non-inherited categories of
`O-001`/`O-005`; a domain subtype, e.g. `Person`, sets `root` to its is-a root —
`Entity` — per Ontology Spec Table 4.) A `root` outside this set **SHALL** be
rejected.

### 6.2 Field descriptor

A **field descriptor** is a Map with exactly two Text keys, and no others:

| Key | Value | Meaning |
|-----|-------|---------|
| `type` | Text — an ACS-004 **type code** (§6.3) | the value-model kind of the field |
| `card` | Text — a **cardinality code** (§6.4) | required/optional and multiplicity |

Field descriptors are held in the `fields` Map keyed by **field name** (Text). Per
ACS-002 §5.6 the `fields` Map, and every descriptor Map, is emitted with keys
sorted by encoded-key bytes; the schema author's declaration order is irrelevant
(the schema document is order-independent, §11.3).

### 6.3 ACS-004 type codes (normative)

A field `type` **SHALL** be exactly one of the following codes. Each code maps to a
single ACS-002 §4 value kind (the "carrier"); the extra codes are *refinements*
that add a validation constraint but do **not** change the carrier or its bytes.

| Code | ACS-002 carrier | Constraint (validation only) |
|------|-----------------|------------------------------|
| `null` | Null | — |
| `bool` | Bool | — |
| `int` | Integer | signed, `[-2^63, 2^63-1]` (the ontology `Timestamp` i64 range) |
| `u32` | Integer | `[0, 2^32-1]` (e.g. a version component or count) |
| `float` | Float | finite binary64 (ACS-002 §5.3) |
| `conf` | Float | finite binary64 **and** `0.0 ≤ v ≤ 1.0` (ontology `Confidence`) |
| `text` | Text | NFC UTF-8 (ACS-002 §5.4) |
| `bytes` | Bytes | opaque octets |
| `urn` | Text | an `EntityUrn` string (§5.1 full form) |

A refinement code (`u32`, `conf`, `urn`) is encoded on the wire **identically** to
its carrier (`int`, `float`, `text` respectively); the constraint is enforced by
the validator (§6.5), not by a distinct byte encoding. This keeps the byte layer
exactly the ACS-002 model while giving the schema layer real semantic types. The
code set is **closed** in `ACS-004/1`; adding a code is a new minor profile via CCP
(§9, §12).

**Integer range coverage (normative note — CCP-007).** The two registered integer
type codes cover `int` = signed i64 `[-2^63, 2^63-1]` and `u32` = `[0, 2^32-1]`.
ACS-002 Integers are valid across the wider `[-2^64, 2^64-1]` range (ACS-002 §5.2),
so a value in `[2^63, 2^64-1]` (or `[-2^64, -2^63-1]`) is representable on the wire
but has **no** registered `ACS-004/1` refinement type; a schema needing the full
unsigned-64 (`u64`) or wider range is a future type-code addition via CCP (the code
set is closed, above). A conformant validator **SHALL** reject an `int`-typed value
outside `[-2^63, 2^63-1]` with reason `value-out-of-range` (negative vector
`instance-int-above-i64`, `2^63` typed `int`).

### 6.4 Cardinality codes (normative)

A field `card` **SHALL** be exactly one of:

| Code | Meaning | Instance obligation |
|------|---------|---------------------|
| `1` | required, single | the field **MUST** be present with a single value of `type` |
| `0..1` | optional, single | the field **MUST** be either absent or a single value of `type` |
| `1..*` | required, one-or-more | the field **MUST** be present as an **Array** (≥1) of `type` |
| `0..*` | optional, zero-or-more | the field **MUST** be either absent or an **Array** (≥0) of `type` |

For `1..*`/`0..*` the field value is an ACS-002 **Array** (§4 kind 7) whose every
element satisfies `type`; array order is significant and preserved (ACS-002 §5.8).
An **optional** field is realized by *absence of the map key* (ACS-002 §5.7), **not**
by a present Null — the two are distinct identities. A field descriptor whose
`type` is `null` with `card` `1` denotes a field that is *always present with the
explicit Null value* (a deliberate, addressable "known-nothing"); this is the only
sanctioned use of a present Null in an ACS-004 instance.

### 6.5 Validation obligation (normative)

Given an instance value `v` (a Map) and a schema document `S`, a conformant
validator **SHALL** accept `v` **iff** all of the following hold, and **SHALL**
reject it otherwise:

1. `v` is an ACS-002 Map with Text keys only.
2. For every field `f` in `S.fields` with `card ∈ {1, 1..*}`: `f` is present in `v`.
3. For every field `f` in `S.fields` with `card ∈ {0..1, 0..*}`: `f` is either
   absent from `v` or present.
4. For every present field `f`: its value satisfies `S.fields[f].type` (the §6.3
   constraint), and for `card ∈ {1..*, 0..*}` its value is an Array (≥1 for `1..*`,
   ≥0 for `0..*`) each element of which satisfies `type`.
5. `v` contains **no** key absent from `S.fields` (the schema is closed; unknown
   fields are rejected — this is required so that two runtimes cannot silently
   diverge on extra fields, and so that instance `ContentId`s are a total function
   of the schema).
6. The mandatory aspects (§8) required by `S.aspects` are satisfied by the presence
   and typing of their carrier fields (§8).

Validation is a pure function of `(v, S)`; it **SHALL** be deterministic and
platform-independent. An implementation **SHALL NOT** accept an instance that
fails any clause, and **SHALL NOT** reject one that passes all clauses
(observational equivalence across implementations).

## 7. Integer-vs-float typing — resolving ACS-002's deferred question (normative)

ACS-002 §4 fixed Integer and Float as *distinct value kinds* at the byte layer but
explicitly deferred "how a concrete `uci.*` cognitive type maps onto the value
model" to this standard. ACS-004 resolves it as follows and this resolution is
**normative and permanent** for `ACS-004/1`:

1. A field whose cognitive meaning is a **count, an index, a version component, an
   identifier ordinal, or a time coordinate** (nanoseconds since epoch, and any
   duration in integral units) **SHALL** be typed `int` (or `u32`) and **SHALL**
   be encoded as an ACS-002 Integer (major 0/1), held **exactly**. It **MUST NOT**
   be typed or encoded as a Float. The ontology's `Timestamp` (i64 ns) is the
   canonical case: it is `int`, never `float`, so a 1 730 000 000 000 000 000-ns
   timestamp survives round-trip byte-exact (contrast a binary64 scheme, which
   would silently corrupt it).
2. A field whose cognitive meaning is a **bounded ratio, a probability, a score, or
   a physical measurement** **SHALL** be typed `float` (or the refinement `conf`
   for `[0,1]`) and encoded as ACS-002 fixed binary64 (§5.3), finite, `-0.0`
   normalized to `+0.0`. The ontology's `Confidence` (`[0.0,1.0]`) is the canonical
   case: it is `conf`.
3. The typing of a field is a property of the **type schema**, fixed once at type
   registration, and **SHALL NOT** vary per instance. An instance that supplies a
   Float where the schema says `int`/`u32`, or an out-of-range value where the
   schema says `u32`/`conf`, **SHALL** be rejected (§6.5). This closes the
   int/float ambiguity at the type layer exactly as ACS-002 closed it at the byte
   layer, so that no two implementations can disagree on whether `observed_at` is
   an integer.

## 8. Aspect binding (normative)

**Canonical `aspects` order (normative).** The `aspects` array of a schema document
**SHALL** list the five mandatory aspects in exactly this order:
`[ "Identity", "Provenance", "Temporal", "Trust", "TenantScope" ]`. Array element order
is content-address-bearing (ACS-002 §5.8), so this order is fixed by the standard; it is
the order carried in the authoritative §11.2 schema body (ContentId `1220…6b3f99c6`). Any
other ordering is a different, non-conformant document. The binding table below is grouped
by carrier for readability and does **not** define the array order.

Ontology Spec Part 4 mandates that the five shared aspects be *"defined ONCE as
aspects and attached to every type — not copied per entity."* ACS-004 binds each
aspect to a **fixed set of carrier fields** with fixed type codes. A type whose
`aspects` array lists an aspect **SHALL** include that aspect's carrier fields with
exactly these codes and cardinalities:

| Aspect (`O-`ref) | Carrier field(s) | Type code | Card |
|------------------|------------------|-----------|------|
| **Identity** (`O-002`) | `urn` | `urn` | `1` |
| **TenantScope** (`SHARD-001`) | `tenant`, `workspace` | `text`, `text` | `1`, `1` |
| **Provenance** (`O-003`) | `origin`; `source`; `invocation` | `text`; `text`; `urn` | `1`; `1`; `0..1` |
| **Trust** (`O-004`) | `confidence` | `conf` | `1` |
| **Temporal** (supports `ORCH-003`) | `valid_from`, `recorded_at` | `int`, `int` | `1`, `1` |

Normative constraints on aspect carriers:

- `origin` **SHALL** be one of the Text values `"observed"`, `"derived"`,
  `"asserted"` (the frozen `Origin` variants). `invocation` **SHALL** be present
  (a content-addressable `EntityUrn` of the producing invocation, `ORCH-004`) **iff**
  `origin == "derived"`, and **SHALL** be absent otherwise (absence, not Null —
  §6.4). This makes the Provenance aspect a small, checkable state machine.
- `valid_from` and `recorded_at` are the two coordinates of the frozen `BiTemporal`
  value; both are `int` nanosecond timestamps (§7).
- Every carrier field is typed and encoded exactly as the corresponding
  `arves-ontology` value type, so an ACS-004 instance is a faithful, storage-
  independent projection of a `CognitiveEntity` (`O-007`).

Aspects attach **by field composition, not by inheritance** (`O-005`): the schema
lists the aspect and includes its carrier fields; there is no subtype edge. A type
that omits a mandatory aspect from `aspects` **SHALL** be rejected at registration
(all `uci.*` cognitive types carry all five; a future non-cognitive registry is
out of scope).

## 9. Type versioning and compatibility (normative)

Type evolution follows `O-006` and the Reference Lifecycle versioning policy
(semantic versioning at the type level), made byte-precise here:

1. **Minor (backward-compatible) change** — `major` unchanged, `minor` incremented.
   The ONLY permitted minor changes are: (a) **adding** an OPTIONAL field
   (`card ∈ {0..1, 0..*}`); (b) **relaxing** a required field to optional; (c)
   widening a refinement constraint within the same carrier where every prior
   value remains valid (e.g. `u32` → `int`). A minor change **MUST NOT** add a
   required field, remove a field, rename a field, change a field's carrier kind
   (`int`↔`float`, `text`↔`bytes`), tighten a constraint, or change cardinality in
   a value-invalidating direction. Because the schema document (§6) changes bytes
   whenever any field changes, a minor version has a **new schema `ContentId`**;
   existing instances of the prior minor remain valid under the new minor (field-
   preserving evolution), and their `ContentId`s are unchanged.
2. **Major (breaking) change** — `major` incremented, `minor` reset to `0`, and a
   **new type URN version** (`uci.<name>@<major+1>.0`). Any change not permitted as
   minor above is a major change and **SHALL** be published as a new major with its
   own schema `ContentId`; the prior major remains valid and re-derivable forever
   (its schema document and every instance addressed under it are immutable).
3. **A registered `(urn, major, minor)` is frozen.** Once a schema document is
   published at a version, its bytes — and thus its schema `ContentId` — **SHALL
   NOT** change. A "fix" is a new minor or major, never an in-place edit (mirroring
   the Spec-Era freeze discipline at the type level, and ED-001 at the corpus
   level). Adding, deprecating, or superseding a type **SHALL** proceed via CCP /
   Amendment / IDR — never a silent edit.
4. **Deprecation.** A type version MAY be marked deprecated in the registry's
   changelog; per the Reference Lifecycle it remains valid for at least one major
   cycle. Deprecation does not change bytes.

A conformant runtime **SHALL** state which registry version it targets (Ontology
Spec Part 9) and **SHALL** resolve an instance's `EntityUrn` `@major.minor` segment
to the exact registered schema for validation (§6.5).

## 10. Instance serialization (normative)

An **instance** of a registered type is an ARVES value of kind Map (ACS-002 §4)
whose keys are the field names of the type's schema and whose values satisfy the
field descriptors (§6.5). Its byte form and identity are fully determined by the
ACS stack:

1. The instance Map **SHALL** be serialized to its canonical body by ACS-002/1
   (deterministic CBOR, §5 of ACS-002): shortest Integers, fixed binary64 Floats,
   NFC Text, definite lengths, and **map keys sorted by encoded-key bytes**. The
   author's field order is irrelevant (§11.3).
2. The instance's `EntityUrn` (its `urn` field, Identity aspect) is the logical
   identity; its **content identity** is the ACS-001 `ContentId` of its body under
   the domain tag that matches its role. A committed `uci.fact` is committed truth,
   so its body is addressed under **`0x01` commit-content**:
   `ContentId = multihash(SHA-256, SHA256(0x01 ‖ body))`. (A type destined for a
   different role — e.g. a manifest — would use that role's tag; ACS-004 does not
   change the ACS-001 tag registry, it consumes it.)
3. Because ACS-002 canonicalization is a pure function of the value and ACS-004
   validation is a pure function of `(instance, schema)`, two independent runtimes
   that hold the same logical fact produce the **same body**, the **same
   pre-image**, and the **same `ContentId`** — the ORCH-004 idempotency and
   ORCH-003 replay precondition, now provable at the type layer.

## 11. Vertical slice — `uci.fact@1` end-to-end (normative template)

This section fully specifies one type end-to-end as the template all remaining
`uci.*` types (ACS-005) follow. `uci.fact` is *"a validated truth claim"*
(Ontology Spec Table 2, `RootType::Fact`) — the canonical committed-truth value.
All bytes below were **computed** with a from-scratch ACS-002/1 dCBOR encoder and
**independently re-verified with OpenSSL** for the SHA-256 step (§11.5).

### 11.1 CDDL projection (non-normative documentation of the §11.2 schema)

```cddl
; uci.fact@1.0 — ACS-004 field schema (projection; schema document §11.2 is authoritative)
uci-fact-1 = {
  ; Identity aspect (O-002)
  urn:         urn,
  ; TenantScope aspect (SHARD-001)
  tenant:      text,
  workspace:   text,
  ; Provenance aspect (O-003)
  origin:      "observed" / "derived" / "asserted",
  source:      text,
  ? invocation: urn,            ; present iff origin == "derived"
  ; Trust aspect (O-004)
  confidence:  conf,            ; float64 in [0.0, 1.0]
  ; Temporal aspect (supports ORCH-003)
  valid_from:  int,             ; i64 ns since Unix epoch
  recorded_at: int,             ; i64 ns since Unix epoch
  ; Fact payload
  claim:       text,
  observed_at: int,             ; i64 ns
  ? evidence:  [* urn],         ; 0+ supporting EntityUrns  (card 0..*)
}
urn  = text  .regexp "urn:arves:.*"     ; ACS-004 type code `urn`
conf = float .and (0.0..1.0)            ; ACS-004 type code `conf`
```

### 11.2 The schema document (authoritative)

The `uci.fact@1.0` schema document is the ARVES Map:

```
{
  "urn": "uci.fact",
  "ver": { "major": 1, "minor": 0 },
  "root": "Fact",
  "aspects": [ "Identity", "Provenance", "Temporal", "Trust", "TenantScope" ],
  "fields": {
    "urn":         { "type": "urn",  "card": "1"    },
    "tenant":      { "type": "text", "card": "1"    },
    "workspace":   { "type": "text", "card": "1"    },
    "origin":      { "type": "text", "card": "1"    },
    "source":      { "type": "text", "card": "1"    },
    "invocation":  { "type": "urn",  "card": "0..1" },
    "confidence":  { "type": "conf", "card": "1"    },
    "valid_from":  { "type": "int",  "card": "1"    },
    "recorded_at": { "type": "int",  "card": "1"    },
    "claim":       { "type": "text", "card": "1"    },
    "observed_at": { "type": "int",  "card": "1"    },
    "evidence":    { "type": "urn",  "card": "0..*" }
  }
}
```

Its ACS-002/1 dCBOR body, ACS-001 pre-image under tag `0x07`, SHA-256, and schema
`ContentId` (the type's content-addressed identity, §5.2) are:

```
schema body (hex, 430 bytes):
a56375726e687563692e6661637463766572a2656d616a6f7201656d696e6f720064726f6f74644661
6374666669656c6473ac6375726ea26463617264613164747970656375726e65636c61696da2646361
7264613164747970656474657874666f726967696ea2646361726461316474797065647465787466736f
75726365a264636172646131647479706564746578746674656e616e74a2646361726461316474797065
64746578746865766964656e6365a2646361726464302e2e2a64747970656375726e69776f726b737061
6365a264636172646131647479706564746578746a636f6e666964656e6365a2646361726461316474797
06564636f6e666a696e766f636174696f6ea2646361726464302e2e3164747970656375726e6a76616c69
645f66726f6da264636172646131647479706563696e746b6f627365727665645f6174a2646361726461
31647479706563696e746b7265636f726465645f6174a264636172646131647479706563696e74676173
7065637473 85684964656e746974796a50726f76656e616e63656854656d706f72616c6554727573746b
54656e616e7453636f7065

schema pre-image = 0x07 ‖ body
schema SHA-256  = 6b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87
schema ContentId = 1220 6b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87
```

(The body hex is wrapped for readability; the byte string is contiguous.)

### 11.3 Example instance → dCBOR body → ContentId

An observed `uci.fact@1.0` instance (origin `observed`, so `invocation` is
**absent** — §8; one supporting `evidence` URN):

```
{
  "urn":         "urn:arves:uci.core:fact@1.0:f-1730000000",
  "tenant":      "acme",
  "workspace":   "research",
  "origin":      "observed",
  "source":      "sensor-array-7",
  "confidence":  0.98,
  "valid_from":  1730000000000000000,
  "recorded_at": 1730000000500000000,
  "claim":       "sky-is-blue",
  "observed_at": 1730000000000000000,
  "evidence":    [ "urn:arves:uci.core:evidence@1.0:e-42" ]
}
```

Its ACS-002/1 body, ACS-001 pre-image under tag `0x01` (commit-content — a fact is
committed truth, §10), SHA-256, and instance `ContentId`:

```
instance body (hex, 264 bytes):
ab6375726e782875726e3a61727665733a7563692e636f72653a6661637440312e303a662d3137333030
303030303065636c61696d6b736b792d69732d626c7565666f726967696e686f627365727665646673
6f757263656e73656e736f722d61727261792d376674656e616e746461636d65686576696465
6e636581782475726e3a61727665733a7563692e636f72653a65766964656e636540312e303a652d3432
69776f726b73706163656872657365617263686a636f6e666964656e6365fb3fef5c28f5c28f5c6a7661
6c69645f66726f6d1b180231d5856d00006b6f627365727665645f61741b180231d5856d00006b726563
6f726465645f61741b180231d5a33a6500

instance pre-image = 0x01 ‖ body
instance SHA-256  = 6fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e
instance ContentId = 1220 6fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e
```

**What the encoding pins.**
- **Map-key ordering by encoded-key bytes (ACS-002 §5.6).** The author-declared
  order above is irrelevant; the canonical body emits keys in the order
  `urn(3) · claim(5) · origin · source · tenant(6) · evidence(8) · workspace(9) ·
  confidence · valid_from(10) · observed_at · recorded_at(11)` — shortest text
  keys first (the CBOR text header carries the length), then bytewise. Any two
  authors, in any order, MUST produce the identical 264 bytes.
- **Int-vs-float (§7).** `confidence` is a binary64 Float
  (`fb 3fef5c28f5c28f5c` = 0.98) while `valid_from`/`observed_at`/`recorded_at` are
  64-bit Integers held exactly (`1b180231d5856d0000` = 1 730 000 000 000 000 000;
  `1b180231d5a33a6500` = 1 730 000 000 500 000 000). A binary64-number scheme would
  corrupt the timestamps; ACS-004 forbids it.
- **Cardinality (§6.4).** `evidence` (`0..*`) is an Array of one `urn`
  (`81 …` = array(1)); `invocation` (`0..1`) is **absent**, not Null (§8), because
  `origin == "observed"`.

### 11.4 Derived variant (absent-vs-present, negative-space check)

The same fact with `origin == "derived"` **SHALL** additionally carry
`invocation` (§8). This is a *different value* (an added key) and therefore a
*different* `ContentId`, proving absence and presence are distinct identities
(ACS-002 §5.7). With `origin = "derived"`,
`invocation = "urn:arves:uci.core:invocation@1.0:inv-9"`, all other fields as §11.3:

```
derived-instance ContentId (tag 0x01) =
1220 0bc84b15220c19b853116d09314f91ecc9e8249e4f645eca8b236c94bfd96ef1
```

### 11.5 Independent verification (OpenSSL)

The SHA-256 of each pre-image was recomputed with OpenSSL (a second, unrelated
toolchain) and matched the encoder byte-for-byte:

```
$ openssl dgst -sha256 schema_preimage.bin
SHA2-256(...) = 6b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87   ✓
$ openssl dgst -sha256 inst_preimage.bin
SHA2-256(...) = 6fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e   ✓
```

## 12. Version tag and evolution (normative)

The registry format and type-code/cardinality vocabularies defined by §3–§10 are
versioned **`ACS-004/1`** (major `4`, this standard; minor `1`, this profile),
consistent with the ACS-002/1 tagging convention. The profile version is a
property of the *registry scheme*, not of any type or instance, and **SHALL** be
carried **outside** any schema document or instance body (in the ACS-003 envelope
and/or the ACS-001 domain registry), so bodies remain minimal and byte-stable
(ACS-002 §7). Embedding an ACS-004 profile version inside a schema document or
instance is **NON-CONFORMANT**.

A future revision (e.g. adding a type code such as `set`, a cross-field constraint
sublanguage, or a `relations` section moved from ACS-005) **SHALL** be introduced
as a new minor/major profile via a CCP Amendment with its own conformance
scenario (CCP-GATE), **SHALL** receive a distinct profile version, and **SHALL NOT**
silently alter the bytes produced by `ACS-004/1`. Type schemas and instances
addressed under `ACS-004/1` remain valid and re-derivable forever (self-describing
address, ACS-001 §5; pinned profile, this section).

## 13. Conformance scenario (CCP-GATE requirement) — `ACS-004-CS-1`

A conformant implementation **SHALL** satisfy every clause below on the pinned
`uci.fact@1.0` vectors of §11. Two independent implementations **SHALL** agree on
all bytes and all `ContentId`s (differential conformance). Digest = SHA-256;
`ContentId = 1220 ‖ SHA256(domain_tag ‖ body)`; bodies are `ACS-002/1` dCBOR.

1. **Schema identity.** Encoding the §11.2 schema document produces the body of
   §11.2 (430 bytes) and, under domain tag `0x07`, the schema `ContentId`
   `12206b3f99c64d23029f49b986e4c89152955c649274e5cf60b1b3ad581b19fa4b87`.
2. **Instance validity.** The §11.3 instance **SHALL** validate against the §11.2
   schema per §6.5 (all required fields present and correctly typed; `confidence`
   ∈ `[0,1]`; timestamps are Integers; `evidence` is an Array of `urn`;
   `invocation` correctly absent because `origin == "observed"`; no unknown keys).
3. **Instance identity.** Encoding the §11.3 instance produces the body of §11.3
   (264 bytes) and, under domain tag `0x01`, the instance `ContentId`
   `12206fce3fbcfce59860140942f7d1ca9e7b274fd936f2237011ff144552f091f07e`.
4. **Order-independence.** The instance authored in **any** field order produces
   the identical 264-byte body and identical `ContentId` (clause 3).
5. **Absent-vs-present.** The §11.4 derived variant validates (with `invocation`
   present) and yields the **distinct** `ContentId`
   `12200bc84b15220c19b853116d09314f91ecc9e8249e4f645eca8b236c94bfd96ef1`; omitting
   `invocation` when `origin == "derived"`, or including it when
   `origin == "observed"`, **SHALL** be a validation FAIL.

An implementation is **non-conformant** if it: emits a Float for any Integer field
(e.g. a timestamp) or an Integer for `confidence`; accepts `confidence > 1.0`;
fails to sort the instance map keys by encoded-key bytes; represents an optional
absent field as a present Null (or vice versa); accepts an instance with a key not
in the schema; resolves `uci.fact@1.0` to a schema document with a `ContentId`
other than clause 1; or diverges from any body or `ContentId` above.

## 14. Proposal analysis

- **Why it matters.** This is the keystone the 12-lens review named (P04-F1,
  P01-F1): the ontology calls itself "the ROOT of UCS" and asserts the ABI loop
  closes through it, yet the loop closed onto *empty types*. Until a `uci.*` type
  has a byte-exact schema, no engine manifest can be resolved, no committed fact can
  be validated, no instance can be content-addressed (ACS-001), and two "certified"
  runtimes are mutually unintelligible — the worst outcome for a standard. ACS-004
  turns the ontology from a taxonomy into a machine-checkable semantic contract and
  establishes the ORCH-003/ORCH-004 **precondition** (a well-defined, differentially
  testable content address for every typed value) at the *type* layer as well as the
  byte layer. ORCH-003 (replay) and ORCH-004 (idempotency) range over executions and
  invocations; ACS provides the deterministic-identity precondition they require — it
  does not by itself prove those runtime invariants, which are established by the
  Kernel/Control-Plane conformance layer.
- **Why CDDL/dCBOR (the decisive choice).** See §3. The data model is already
  dCBOR (ACS-002); CDDL is the IETF schema language for that exact model, so the
  schema and byte layers share one type universe. JSON Schema cannot name the
  exact int-vs-float distinction ACS-002 is built on; CUE is single-ecosystem and
  larger-surfaced; Protobuf is non-canonical and conflates absent with default —
  each disqualifying under a content-addressing standard. The three are retained
  only as optional, non-normative projections.
- **Resolving ACS-002's open question.** §7 fixes the int-vs-float mapping at the
  type layer permanently: counts/timestamps/versions are `int` (exact), ratios/
  probabilities are `float`/`conf` (binary64). This is the type-layer twin of
  ACS-002 §4 and removes the last place two implementations could disagree on a
  field's numeric kind.
- **Risks.** (a) *Closed-schema strictness* — rejecting unknown fields (§6.5.5) is
  deliberate: openness would let two runtimes diverge silently and would make an
  instance `ContentId` not a total function of its schema. Extensibility is
  provided *by versioning* (§9), not by tolerated extra fields. (b) *Aspect
  rigidity* — binding aspects to fixed carrier fields (§8) could feel constraining,
  but it is exactly the "define aspects once, attach everywhere" mandate of the
  frozen Ontology Part 4, made byte-exact. (c) *CDDL vs schema-document drift* —
  mitigated by declaring the dCBOR schema document authoritative (§3) and the CDDL
  a projection. (d) *Scale* — 18 root types + domain subtypes is large; mitigated
  by the R-03 strategy this doc executes: fully specify one slice (`uci.fact`) as
  the byte-exact template, then ACS-005 templates the rest.
- **Long-term consequences (20-year view).** Type identity is *content-addressed*
  (§5.2), so "the same type at the same version" is a byte-checkable fact across
  vendors and decades, not a coordination promise; frozen `(urn, major, minor)`
  schemas and their instances are re-derivable forever (self-describing address);
  and evolution is additive-by-construction (§9), so a v2 fork is avoided by
  publishing the missing `schema` slot *now* as the MINOR addition the registry
  format always reserved (P04-F1, P01-F1 recommendation).
- **Alternatives considered.** (i) *Ship URNs-only* (status quo) — rejected: not
  implementable, the very gap R-03 raises. (ii) *A Rust-source master registry
  generating schemas* — rejected: couples the standard to one language ("Standards
  over Frameworks", constitution). (iii) *Invent a bespoke schema language* —
  rejected: needless reimplementation surface; CDDL already fits the model. (iv)
  *Open schemas (ignore unknown fields)* — rejected (see Risks a).
- **Recommendation.** Ratify via CCP-GATE with `ACS-004-CS-1`. Then a Runtime task
  adopts it in `arves-ontology` (populate the `TypeRegistry` with the `uci.fact@1.0`
  schema document; add an ACS-004 validator and a schema/instance `ContentId`
  computation) and `arves-kernel` (the opaque `ProposedWrite` payload becomes a
  validated ACS-004 instance whose `ContentId` is computed per §10). ACS-005 then
  templates the remaining `uci.*` types and adds the `relations` section.
- **Implementation complexity.** Standard: high (the keystone). Reference-runtime
  adoption: medium — a schema-document encoder (reuses the ACS-002 dCBOR codec), a
  pure validator (§6.5), and a registry lookup; the strictness (closed schemas,
  int/float discipline, absent-vs-null) is the main care point, and it is exactly
  the discipline ACS-002 already enforces at the byte layer.
- **Scientific impact.** Converts "type system" from prose into a
  differentially-testable, cross-implementation property: two runtimes must
  resolve `uci.fact@1.0` to the same schema `ContentId` and address the same fact
  to the same instance `ContentId`. It makes formal statements like *"every
  committed truth is a well-typed `uci.fact@n`"* checkable — the difference between
  an ontology paper and a machine-verified type theory.
- **Ecosystem impact.** Unblocks every third-party runtime (Program C), SDK
  codegen (types projected from schema documents), marketplace type-checking,
  cross-runtime caches/registries keyed by schema `ContentId`, and the
  differential-conformance tier (Goal 3). Without it there is no ecosystem — only
  isolated forks with private, mutually-unintelligible types.

## 15. Dependencies & sequence

- **Depends on:** **ACS-002** (Canonical Serialization — schema documents and
  instances are ACS-002/1 dCBOR bodies; this standard *is* the type-layer complement
  ACS-002 §2 deferred); **ACS-001** (Content Addressing — type identity and instance
  identity are ACS-001 `ContentId`s; consumes the domain-tag registry and allocates
  `0x07`); RFC 8610 (CDDL, projection); RFC 8949 §4.2 (dCBOR, via ACS-002); the
  frozen `arves-ontology` value types (`EntityUrn`, `TypeVersion`, `Confidence`,
  `Timestamp`, `Origin`, `ProvenanceRecord`, `BiTemporal`, `ShardKey`, `RootType`,
  the five aspect traits).
- **Blocks:** **ACS-005** (Schema Registry — templates the remaining `uci.*` types
  from this format and adds the `relations` section); **ACS-003** (Wire Envelope —
  carries the `ACS-004/1` profile tag and the `urn@version`); ACS-006/009
  (Fingerprint / Replay — traces reference typed, content-addressed instances); the
  differential-conformance tier (Goal 3) and the second runtime (Program C); Engine
  Graph ABI resolution (Ontology Spec Part 10 — the loop closes here).
- **Grounded in (frozen, not edited):** Universal Cognitive Ontology Specification
  v1 (Part 2 ABI closure; Part 4 aspects; Part 9 registry-entry shape `{ urn,
  version, aspects, schema, relations }`; Part 10 ABI loop; `O-001..007`); ARVES-19
  Canonical Ontology (`Person`/`Fact` and the domain vocabulary Table 4 maps onto
  roots); Invariant Registry v1 (`SHARD-001`, `OWN-001`, `ORCH-001`, `ORCH-003`,
  `ORCH-004`); the reference `arves-ontology` contracts crate.

---

*Ratification path (Reference Lifecycle, Part 6): DRAFT → CCP-GATE (this doc +
`ACS-004-CS-1`) → Candidate → Ratified. On ratification this becomes a registered
normative addition; the frozen v1.0 corpus is unchanged (ED-001).*

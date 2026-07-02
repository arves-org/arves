# ARVES Specification — Read These 3 First

The ARVES specification corpus is large (~50 documents in the
[Specification Corpus](../spec-markdown/ARVES_00_Documentation_Index_v2.2.md) plus the standard
set on [The Standard](../standard/README.md) page). You do **not** need to read it all to start.
This page is a curated on-ramp: the **three load-bearing documents** that a newcomer, a capability
author, or someone building an independent runtime should read first, in order — plus where to go
next.

> The Markdown files in the `spec-markdown/` corpus are faithful mirrors of the frozen `.docx`
> corpus (the authoritative source of record). The ACS contracts are published below —
> [ACS-001](../standard/acs/ACS-001_Content_Addressing.md) through
> [ACS-005](../standard/acs/ACS-005_Normative_Language.md). Everything is frozen — changes go
> through CCP / Amendment / IDR, never a silent edit.

---

## 1. The Invariant Registry — *what must always be true*

**[`spec-markdown/ARVES_00_Invariant_Registry_v1.md`](../spec-markdown/ARVES_00_Invariant_Registry_v1.md)**

Start here. It is the single authoritative list of ARVES's invariants — the properties every
implementation must uphold — with each one's statement, source, and proof status. Crucially, it
tells you what is **registered-normative** (enforce these: `OWN-001`, `LAYER-001`, `SHARD-001`,
`ORCH-001..004`) versus what is only **proposed/informative** and carries no conformance weight
until ratified. Reading this first calibrates everything else: you learn what ARVES *promises*
before you read how it delivers those promises.

**Why first:** it is the shortest path to understanding what ARVES is accountable for, and it
prevents you from treating a draft idea as a hard requirement.

---

## 2. ACS-001 — Content Addressing — *how identity works*

**[`standard/acs/ACS-001_Content_Addressing.md`](../standard/acs/ACS-001_Content_Addressing.md)**

ARVES identity is **content-addressed**: a value's id is derived from its bytes, so the same
value always has the same id and any tamper changes the id. This is the contract behind
"byte-reproducible ids", the artifact signatures the [CLI](./CLI_REFERENCE.md) produces, and the
whole certify/package/install trust boundary. If you author capabilities, this is *why* your
effects get stable, verifiable identities.

**Why second:** once you know what must be true (doc 1), this is the first mechanism that makes
it true — deterministic identity.

---

## 3. ACS-002 — Canonical Serialization — *the exact bytes*

**[`standard/acs/ACS-002_Canonical_Serialization.md`](../standard/acs/ACS-002_Canonical_Serialization.md)**

Content addressing (doc 2) only works if every implementation agrees on the **exact byte
encoding** of a value. ACS-002 is that byte-exact canonical serialization: the ARVES value model
(null · boolean · integer · float · string · bytes · array · map) and precisely how each
serializes. This is the document the `doctor`/`certify` tooling cites when it rejects, for
example, a bare JS number (ambiguous int/float) — the errors point at ACS-002 §5.2/§5.3. If you
are building an independent runtime, this is the contract you must match byte-for-byte.

**Why third:** it is the ground truth underneath doc 2 — the exact bytes that make identity, and
therefore interoperability, deterministic across implementations.

---

## Then: the Freeze Record — *what is settled, and how it changes*

**[`spec-markdown/ARVES_00_Specification_Freeze_Record_v1.md`](../spec-markdown/ARVES_00_Specification_Freeze_Record_v1.md)**

Read this fourth. It is the "signature page" of ARVES v1.0: it declares the specification frozen
and defines the change-management instruments (CCP / Amendment / IDR / next major version). It
tells you the rules of the road — that the spec never bends to implementation, and exactly how a
genuine gap is raised rather than worked around.

---

## Where to go after the first four

- **Authoring a capability?** → [CLI_REFERENCE.md](./CLI_REFERENCE.md) and the
  [Ecosystem SDK README](../products/arves-ecosystem-sdk/README.md); then the rest of the ACS set
  — [ACS-003 Canonical Envelope](../standard/acs/ACS-003_Canonical_Envelope.md),
  [ACS-004 Universal Type Registry](../standard/acs/ACS-004_Universal_Type_Registry.md),
  [ACS-005 Normative Language](../standard/acs/ACS-005_Normative_Language.md).
- **Building a runtime?** → the conformance vectors and certification material under
  [`standard/`](../standard/) (`standard/vectors/`, `standard/conformance/`,
  `standard/certification/`).
- **Want the full map?** → the documentation index,
  [`spec-markdown/ARVES_00_Documentation_Index_v2.2.md`](../spec-markdown/ARVES_00_Documentation_Index_v2.2.md).
- **The frozen runtime contract?** → [`runtime/RUNTIME_FREEZE_v1.0.md`](../runtime/RUNTIME_FREEZE_v1.0.md).

That is the minimum spine — three documents to understand ARVES, a fourth to understand how it
stays stable, and clear next hops for whatever you are building.

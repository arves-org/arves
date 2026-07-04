# CCP-GATE Ratification v3 — CCP-007: minor ACS clarifying clauses (no byte/vector change)

**Decision:** Two **clarifying normative clauses** — one in ACS-002 §5.2, one in ACS-004 §6.3 —
plus one new instance negative vector, are **RATIFIED** into the frozen Kit, effective
2026-07-04, **at independence grade G1**. Kit **0.3.0 → 0.3.1**. Instrument: CCP-GATE +
ACS-001 §4.1. This record is the governance act; it is **not** a silent edit.

**Critically: no golden vector, no ContentId, and no canonical byte changes.** Both clauses
make the normative *text* match rules the vectors + reference implementations already enforce.

## What this ratifies (SYSTEM_GAP_ANALYSIS #22, #19)

| # | Gap | Fix (clause) | Byte/vector impact |
|---|-----|--------------|--------------------|
| **#22** | ACS-002 §5 had no explicit **shortest-form LENGTH** clause, though reason code `non-shortest-len` + vector `780161` already mandate it | ACS-002 §5.2: the shortest-argument rule now explicitly covers the **length prefix** of Text/Bytes/Array/Map, rejected as `non-shortest-len` | **none** — documents the existing `non-shortest-len` vector |
| **#19** | ACS-004 `int`/`u32` **cannot type a valid ACS-002 Integer in [2^63, 2^64-1]** — a silent coverage gap | ACS-004 §6.3: a normative **range-coverage note** (int = i64, u32; the [2^63,2^64-1] band has no v1 type code; `u64` is a future CCP) **+** a new negative vector `instance-int-above-i64` (2^63 typed `int` → `value-out-of-range`) | **+1 negative vector** (36 rows); no golden/byte change |

The new vector is oracle-verified (`gen_candidate_vectors.py`: decode-clean + rejected) and
exercised by `conformance_semantic.py` → **instance 8/8** (evidence_probe row `acs-semantic-reject`).

## Confirmed already-done (SYSTEM_GAP_ANALYSIS #24)

**#24** — the ACS-005 §9.3 glossary-resolution lint — is **implemented** in the reference
checker (`acs005_checker.py`, `glossary_resolution_lint`; done in commit `f3a7e86`). It resolves
13/14 §9.1 terms to their `GL-nnn` entry (+ the `Workspace`→GL-004 / `CP`→GL-007b aliases) and
**flags "Data Plane" as GATED** (see #21). No further tooling work; the residual is the #21 spec
decision below.

## NOT ratified — recorded as maintainer design decisions (the constitutional STOP)

Two gap-analysis items are **not** minor clarifications; each is a genuine normative decision that
would change frozen semantics or a frozen golden vector. Per CLAUDE.md Change Management ("If a
specification issue is discovered — STOP … classify it"), they are documented here for a
deliberate maintainer ruling rather than rushed into the batch:

- **#20 — ACS-004 §6.5 does not bind an instance's Identity `urn` to its schema's type.** ACS-004
  §5.1 already states the rule normatively (*"the full EntityUrn's `type-name@version` segment MUST
  equal the type's short form modulo namespace"*), but §6.5 validation does not enforce it. The
  blocker is that *"modulo namespace"* is **underspecified for the shipped example**: the schema
  carries `urn = "uci.fact"` (short form) while the instance carries
  `urn:arves:uci.core:fact@1.0:…` — so binding requires deciding exactly how `uci.fact` maps to
  `uci.core:fact@1.0` (is `uci.core` the namespace and `fact` the name? does `uci.fact` abbreviate
  `uci.core:fact`?). Until that mapping is fixed normatively, a §6.5 enforcement clause + negative
  vector would encode a guess. **Decision needed:** the exact EntityUrn↔short-form binding rule,
  then a §6.5 clause + an `instance-urn-type-mismatch` vector.
- **#21 — ACS-005 §9.1 requires a `GL-nnn` entry for "Data Plane", but the glossary closes at
  GL-001..GL-014 and defines "Data Plane" only inline (§7 note).** The reference checker already
  flags this as **GATED** (honest, not a silent pass). The "real fix" is `GL-015 Data Plane` — but
  adding a 15th glossary term **changes the §9.2 golden term-set vector** (`GL-001..GL-014` →
  `…GL-015`), a **byte-affecting** change to a frozen golden ContentId, i.e. an **ACS-005 profile
  bump (v2)**, not a minor clause. **Decision needed:** either (a) ratify `GL-015 Data Plane` as
  ACS-005/2 and recompute vector #1, or (b) amend §9.1 to resolve "Data Plane" via its §7 inline
  definition (softening the GL-entry requirement) — a normative choice with no free lunch.

## Effect

- `acs/ACS-002_Canonical_Serialization.md` §5.2 and `acs/ACS-004_Universal_Type_Registry.md` §6.3:
  the two clarifying clauses (no byte/vector change).
- `vectors/acs_negative_vectors.tsv`: 35 → **36 rows** (+`instance-int-above-i64`).
- `conformance/CONFORMANCE.md`, `VERSION` (0.3.1): counts updated.
- Independence unchanged: **G1; G2 NOT YET MET.** RCR-004 (native Rust semantic validators) and
  gaps #3/#18/#20/#21 remain open.

*Ratified under ARVES 2.0 governance. Recorded in the living repository (ED-001); the frozen
`.docx` corpus is unchanged.*

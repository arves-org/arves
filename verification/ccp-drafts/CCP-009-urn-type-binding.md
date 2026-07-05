# CCP-009 (DRAFT) — ACS-004 urn↔type binding enforcement (closes open-debt #20)

**Status:** **DRAFT proposal — NOT ratified, NOT applied.** Freeze-clean: this document + its
generator (`gen_ccp009_vector.py`) + the candidate vectors (`ccp009_candidate_vectors.tsv`) live
under `verification/ccp-drafts/`. **Nothing here edits the frozen `standard/`.** Ratification
(which touches frozen ACS-004 §6.5, the ACS-001 §4.1 reason-code registry, both reference
validators, and the negative corpus) is a separate, maintainer-authorized **CCP-GATE** step.

**Instrument:** CCP Amendment. **Closes:** `verification/OPEN_DEBT_REGISTER.md` **#20** —
*"ACS-004 §5.1 urn↔type binding is normative but §6.5 doesn't enforce it; 'modulo namespace' is
underspecified for `uci.fact` (schema) vs `urn:arves:uci.core:fact@1.0` (instance)."*

## 1. The defect (demonstrated, not asserted)

### 1.1 §5.1 makes the binding normative…

ACS-004 §5.1 (Type identity — URN form) fixes two URN forms and binds them:

> ```
> uci.<name>@<major>.<minor>          (short registry form, e.g. uci.fact@1.0)
> urn:arves:<namespace>:<name>@<major>.<minor>:<local-id>   (full EntityUrn, §10)
> ```
>
> The short form `uci.<name>@<major>.<minor>` identifies a **type**; the full
> `EntityUrn` identifies an **instance** of that type (its `type-name@version`
> segment MUST equal the type's short form modulo namespace).

That parenthetical MUST is normative (RFC 2119/8174 per the ACS-004 preamble): an instance's
`urn` is not free text — its `<name>@<major>.<minor>` segment is REQUIRED to agree with the
schema it claims to instantiate.

### 1.2 …but §6.5 does not enforce it

ACS-004 §6.5 (Validation obligation) enumerates the accept-iff clauses exhaustively — a
conformant validator "**SHALL** accept `v` **iff** all of the following hold": (1) Map with Text
keys; (2) required fields present; (3) optional fields absent-or-present; (4) present fields
satisfy their §6.3 type code and cardinality; (5) no unknown keys (closed schema); (6) aspect
carriers satisfied. **No clause references §5.1.** The only check the `urn` value receives is
its §6.3 type code — *"`urn` | Text | an `EntityUrn` string (§5.1 full form)"* — which **all
three** reference validators implement as a **prefix-only** check (`urn:arves:`;
`acs004_instance.py`, `acs004.mjs`, and the Rust `arves-conformance` semantic arm) — never a
full §5.1 parse, and never a comparison against `S.urn`/`S.ver`. The only frozen negative vector
for the code (`instance-urn-not-arves`) exercises exactly the missing prefix, so a prefixed but
non-parsing value such as `urn:arves:junk` also validates clean today. Since §6.5 also declares
observational equivalence ("**SHALL NOT** reject one that passes all clauses"), a validator that
*did* unilaterally enforce §5.1 at instance-validation time would arguably be **non-conformant
under §6.5 as written** — the two sections are in genuine tension, not merely silent.

The gap is machine-demonstrated (run `gen_ccp009_vector.py`): take the §11.3 golden `uci.fact@1.0`
instance and change only its `urn` to name a **different type** — `urn:arves:uci.core:goal@1.0:…`.
The current independent `acs004_instance.validate_instance` **ACCEPTS** it:

```
CURRENT validate_instance: ACCEPTS (#20 gap confirmed)
PROPOSED clause 7        : REJECTS -> urn-type-mismatch
ContentId (tag 0x01)     : 12201d99b0cb79affef6ab738ff7406f5f4c331d07640dd0f5f76ff201d31c10a7f8
```

The consequence: a committed, content-addressed value whose Identity aspect (§8, `O-002`) carries
a **false type identity** validates clean. Everything that trusts the `urn` — instance-urn schema
resolution (ACS-004 §9, closing paragraph: "resolve an instance's `EntityUrn` `@major.minor`
segment to the exact registered schema for validation (§6.5)"), manifest-URN ABI resolution (§4),
registry lookup, cross-runtime exchange — is routed to the wrong schema, and two honest runtimes
can disagree about *what type an accepted instance is* while agreeing on its bytes. That is a
semantic fork underneath ORCH-004-keyed dedup, invisible to the byte layer.

The blast radius is flow-dependent (an honesty sharpening, not a weakening): in the §9
resolve-from-instance-urn flow the two name/version candidates below would already fail —
`goal@1.0` resolves to a different (or unregistered) schema, `fact@2.0` to no registry entry,
a §4 conformance FAIL — so §9 *partially* mitigates. The false acceptance bites when the schema
is chosen by the **§4 manifest flow** (`Manifest.Produces = [uci.fact@1]` selects the schema
before any instance exists) or when the pair is handed directly to the pure §6.5 validator: the
instance then validates clean under the wrong identity.

### 1.3 "modulo namespace" is underspecified

§5.1 compares the instance segment to the short form "**modulo namespace**" — but the two forms
carry *different* namespace-ish material:

- the schema document's `urn` key is `"uci.fact"` (§6, §11.2) — a dotted **registry name** whose
  `uci.` prefix marks the Universal Cognitive Infrastructure type space (Ontology Spec Table 2);
- the golden instance's `urn` is `"urn:arves:uci.core:fact@1.0:f-1730000000"` (§11.3) — whose
  §5.1 `<namespace>` segment is `uci.core` and whose `<name>` segment is plain `fact`.

So "equal modulo namespace" cannot be literal string equality: `uci.fact` ≠ `fact` and
`uci.` ≠ `uci.core`. The frozen text does not say whether (a) `uci.fact`'s comparable name is
`fact` (strip the `uci.` registry prefix) with `<namespace>` fully disregarded, (b) the
`<namespace>` must additionally belong to the `uci.` family, or (c) the namespace is pinned. An
independent implementer must guess — the exact failure mode ACS-004 exists to preclude.

## 2. Lawful binding-rule options

All three options below are consistent with the frozen §5.1 text and with the pinned golden
vectors (§11.3 instance, §11.4 derived variant, both `urn:arves:uci.core:fact@1.0:…` against
schema `uci.fact` ver `1.0`).

| | Rule | Golden-clean? | Assessment |
|---|---|---|---|
| **A** (recommended) | Parse the instance `urn` per the §5.1 full form. REQUIRE `<name>` = `S.urn` stripped of its `uci.` registry prefix (`"uci.fact"` → `fact`) AND `<major>.<minor>` = `S.ver`. **Disregard `<namespace>` entirely.** | ✔ | The literal reading of "modulo namespace": the namespace segment is exactly the material excluded from the comparison. Minimal, byte-clean, vendor-namespace-friendly (a third party may mint `urn:arves:acme.custom:fact@1.0:…` and still bind to the registered `uci.fact@1.0`). |
| **B** | Option A **plus**: `<namespace>` MUST begin `uci.` for a type whose short form begins `uci.` (golden uses `uci.core`). | ✔ | Stricter than the frozen text — "modulo namespace" says the namespace does not participate, so constraining it is an *addition*, not a reading. It would also forbid vendor namespaces for registered types, which nothing in the frozen corpus requires. Lawful as a CCP, but over-reaches #20. |
| **C** | Option A **plus**: `<namespace>` pinned to exactly `uci.core` for all `uci.*` types. | ✔ | Rejected. Hard-codes one namespace the frozen corpus never declares canonical, contradicts the *spirit* of "modulo namespace", and permanently blocks multi-vendor instance minting — an ecosystem cost with no conformance benefit. |

**Recommendation: Option A.** Rationale: (i) it is the only option that *interprets* the frozen
§5.1 sentence rather than extending it — "modulo namespace" naturally means "the comparison
disregards the namespace segment"; (ii) it is fully determined by data already in the schema
document (`S.urn`, `S.ver`) — no new registry state; (iii) it is byte-clean — the oracle proves
the §11.3 golden and §11.4 derived instances both pass, so **no golden ContentId changes**;
(iv) it keeps the door open: a future CCP could still tighten toward B without invalidating any
Option-A-valid instance corpus… except vendor-namespaced ones, which is exactly the debate a
namespace-governance CCP should own (Open Question below), not this binding fix.

## 3. Proposed fix (byte-clean — no golden ContentId changes)

Add one clause to **ACS-004 §6.5** (after clause 6), harmonized with §5.1 and §8:

> 7. **URN↔type binding (§5.1).** Where `S.fields` declares the Identity carrier `urn` (§8)
>    and the present `urn` value satisfies clause 4 (a Text bearing the `urn:arves:` prefix —
>    what the reference validators enforce for the §6.3 `urn` code today), it **SHALL**
>    additionally parse as the §5.1 full form
>    `urn:arves:<namespace>:<name>@<major>.<minor>:<local-id>`, pinned exactly:
>    `<namespace>` and `<name>` are non-empty and contain neither `:` nor `@`;
>    `<major>` and `<minor>` are **canonical decimal integers** — `0`, or a digit string with
>    no leading zero (`fact@01.0` and `fact@1.00` are *not* the full form and REJECT); and
>    `<local-id>` is non-empty. A parsed urn **SHALL** further satisfy
>    `<name>` = the `<name>` of `S.urn` (for a registry name of the form `uci.<name>`,
>    the comparable name is `<name>`; the `<namespace>` segment is **disregarded** — this
>    is the normative meaning of §5.1 "modulo namespace") **and**
>    `<major>` = `S.ver.major` **and** `<minor>` = `S.ver.minor` (numeric equality; because
>    both sides are canonical decimals, string equality gives the same verdict by
>    construction — no normalization divergence is possible). **Any** clause-7 failure —
>    an Identity-carrier urn that passes clause 4 but does not parse as the pinned full
>    form, or a parsed urn violating the equality — **SHALL** be rejected
>    **`urn-type-mismatch`**. A `urn` that fails clause 4 itself (e.g. lacks the
>    `urn:arves:` prefix — the frozen `instance-urn-not-arves` vector) remains a clause-4
>    `field-type-mismatch`; clause 7 is evaluated only on clause-4-clean values, so the
>    two codes never compete on one input.

And register the new reason code per the ACS-001 §4.1 reason-code registry convention (kebab-case,
tier `instance`, added "only via a CCP Amendment that also adds a negative conformance vector
exercising the code"):

| Reason code | Governing clause | Meaning |
|---|---|---|
| `urn-type-mismatch` | ACS-004 §6.5.7 / §5.1 | the Identity-carrier `urn` passes clause 4 but is not the pinned §5.1 full form, or its `<name>@<major>.<minor>` segment does not equal the schema's short form modulo namespace |

A distinct code (not `field-type-mismatch`) is chosen because a clause-7 failure occurs on a
value that already passed the clause-4 typing check — the defect is a **binding** violation
against the Identity contract, observationally different from a typing failure and separately
diagnosable by a certification harness. CCP-006 is precedent here only for the kebab-case
semantic-code band and the registry convention ("new code only via a CCP Amendment that also
adds a negative vector"); CCP-006 itself coalesced several distinct MUST-reject rules under
shared codes (11 codes over 19 vectors) and recorded code granularity as *"a ratification
choice"* — so the distinct code is this draft's own argued choice, not a CCP-006 rule.

This is a **vector-set + validator addition, not a byte-affecting profile change** (the CCP-007/
CCP-008 pattern): the golden §11.3/§11.4 instances already satisfy clause 7, so every pinned
body, schema `ContentId` (`1220…6b3f99c6…`), and instance `ContentId` (`1220…6fce3fbc…`,
`1220…0bc84b15…`) is unchanged. The clause removes an *acceptance* the standard never intended,
resolving the §5.1↔§6.5 tension in §5.1's favor.

## 4. The candidate negative vectors (oracle-demonstrated)

`ccp009_candidate_vectors.tsv` (four rows, same columns as the frozen negative TSV):

| case | tier | reject_reason | mutation (vs the §11.3 golden instance) |
|------|------|---------------|------------------------------------------|
| `instance-urn-type-mismatch` | `instance` | `urn-type-mismatch` | `urn` = `urn:arves:uci.core:goal@1.0:f-1730000000` (type **name** `goal` ≠ schema `fact`) |
| `instance-urn-version-mismatch` | `instance` | `urn-type-mismatch` | `urn` = `urn:arves:uci.core:fact@2.0:f-1730000000` (type **version** `2.0` ≠ schema `1.0`) |
| `instance-urn-not-full-form` | `instance` | `urn-type-mismatch` | `urn` = `urn:arves:junk` (clause-4-clean **prefix**, but not the §5.1 full form — closes the malformed-urn bypass) |
| `instance-urn-version-leading-zero` | `instance` | `urn-type-mismatch` | `urn` = `urn:arves:uci.core:fact@01.0:f-1730000000` (leading-zero version segment — not a canonical decimal, so not the pinned full form; machine-pins the clause-7 grammar) |

Canonical bodies and ContentIds (tag `0x01`, commit-content — §10), computed by the oracle:

```
instance-urn-type-mismatch          ContentId = 12201d99b0cb79affef6ab738ff7406f5f4c331d07640dd0f5f76ff201d31c10a7f8
instance-urn-version-mismatch       ContentId = 12208677ea3afc761b609d4a56b9519f32565be40e8170dc2ee95539ece18013b487
instance-urn-not-full-form          ContentId = 1220840a45817a9d573ba8dc70db7f04f2a4811203dd8d37be3f0202a562373001af
instance-urn-version-leading-zero   ContentId = 1220fb1309d4448e52627e0bf8a8fa45eedacbb6a98f1e344b95741018a6d65d0ba9
```

`gen_ccp009_vector.py` proves, machine-checked: (1) each candidate **decodes clean** as canonical
dCBOR; (2) the **CURRENT** `acs004_instance.validate_instance` **ACCEPTS** all four (the #20 gap
is real, not hypothetical — including the prefixed-but-malformed urns, which today's prefix-only
§6.3 check waves through); (3) the **PROPOSED** clause-7 rule **REJECTS** all four with
`urn-type-mismatch` (the fix is implementable and un-bypassable); (4) the §11.3 golden instance
**and** the §11.4 derived variant **PASS** the proposed rule (byte-clean — no golden change).
Exit 0 iff all hold.

## 5. Ratification (what actually touches frozen — GATED on the maintainer)

At CCP-GATE, as one atomic amendment: (a) add the §6.5 clause 7 to
`ACS-004_Universal_Type_Registry.md` (and the one-line §5.1 cross-reference "enforced by §6.5.7"
if the maintainer wants the tension visibly closed at both ends); (b) register
`urn-type-mismatch` in the ACS-001 §4.1 reason-code registry band and `conformance/CONFORMANCE.md`;
(c) implement clause 7 — including its parse-failure rejection of prefixed-but-malformed
Identity urns — in all three reference validators (`acs004_instance.py`, the peer TypeScript
`acs004.mjs`, and the native Rust `arves-conformance` semantic arm; the §6.3/clause-4 prefix
check itself is untouched); (d) append the four vectors to
`standard/vectors/acs_negative_vectors.tsv` (instance tier 8 → 12); (e) re-run
`freeze_check.py update` + re-certify both runtimes + regenerate the evidence probe. Each is a
frozen edit requiring explicit maintainer authorization.

## 6. Honesty

- **DRAFT, freeze-clean.** No frozen byte is touched; the freeze gate stays at 0 drift.
- **Byte-clean fix.** Option A accepts every pinned golden, so ratification changes no golden
  ContentId. If the maintainer instead rules for Option B/C (namespace-constraining), the goldens
  *still* pass — but any existing vendor-namespaced instances would newly reject, which is a
  compatibility ruling the CCP-GATE must make explicitly, never silently.
- **The reference validators are conformant today — under the prefix-only reading of §6.3.**
  All three implement the §6.3 `urn` code as an `urn:arves:` prefix check, the only frozen
  negative vector for it (`instance-urn-not-arves`) exercises exactly that prefix, and §6.5's
  iff + observational-equivalence wording then *requires* their acceptance of all four
  candidates. Whether the frozen §6.3 row — "an `EntityUrn` string (§5.1 full form)" — already
  demanded a full parse is an ambiguity no validator may resolve unilaterally; clause 7 resolves
  it **explicitly for the Identity carrier only**, leaving §6.3/clause 4 (and every other
  `urn`-typed field — Open Question (ii)) on the prefix-only reading. The defect is in the
  standard's §5.1↔§6.5 seam; this CCP fixes the standard, and the validators follow at
  ratification.
- **Compatibility ruling — new rejections, stated explicitly.** Ratifying clause 7 newly rejects
  not only wrong-type/wrong-version urns but also Identity-carrier urns that are prefixed yet
  not the pinned full form (`urn:arves:junk`, leading-zero versions such as `fact@01.0`) — all
  of which validate clean today. Without this, the binding is trivially escaped by malforming
  the urn. The CCP-GATE accepts these rejections as part of this amendment — the same explicit
  treatment §2 gives the Option B/C namespace question, never a silent tightening.
- **In-program (G1).** The oracle demonstrating the gap was authored in this program; closing #20
  strengthens self-consistency, it does not manufacture G2 evidence.
- **Open questions (not assumed):** (i) Should vendor namespaces for registered `uci.*` types be
  *endorsed* (Option A permits them silently) or governed by a separate namespace-registry CCP?
  (ii) Does clause 7 also bind `urn`-typed fields *other than* the Identity carrier (`invocation`,
  `evidence` elements name *other* types — `invocation@1.0`, `evidence@1.0`)? This draft scopes
  clause 7 to the Identity carrier only, because §5.1 speaks of "an instance **of that type**";
  cross-field type binding (does `evidence` have to reference a registered type at all?) is a
  distinct, larger question for the ACS-005 relation layer.

---

*Ratification path (Reference Lifecycle, Part 6): DRAFT → CCP-GATE (this doc + the four vectors +
the validator rule) → Candidate → Ratified. On ratification this becomes a registered normative
addition; the frozen v1.0 corpus is unchanged (ED-001).*

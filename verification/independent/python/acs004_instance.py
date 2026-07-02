"""
ACS-004 / CCP-004 — Universal Type Registry: INDEPENDENT INSTANCE VALIDATOR (§6.5/§7/§8).

Independent implementation from the ARVES Standard Kit ONLY
(standard/acs/ACS-004_Universal_Type_Registry.md), with the ACS-002 codec
(acs002_dcbor / acs002_decode) and the spec-derived logical values (acs_values)
that already live in this living-verification arm. NO reference-runtime source was
consulted; nothing under runtime/ was read. This module proves the ACS-004
§6.5/§7/§8 REJECT rules are implementable from the standard alone and gives the
future negative-vector CCP a from-scratch oracle (KPI = Evidence Increased).

The peer TypeScript validator (verification/independent/typescript/src/acs004.mjs,
`validateInstance`) implements exactly this obligation over its own value model;
its logic is MIRRORED here, but every rule below is RE-DERIVED from the ACS-004
prose and cited to the section that fixes it.

Rejection reasons are FREE-FORM descriptive strings that cite the governing §.
Stable, machine-readable reason codes for ACS-004 rejections are deliberately NOT
invented here: per ACS-001 §4.1 a stable-reason-code registry is a future CCP, and
CONFORMANCE.md is not edited by this evidence subset. Free-form + §-citation is the
intentional, correct contract for this proof.

--------------------------------------------------------------------------------
ACS-004 REJECT RULES ENFORCED (each MUST-reject clause, with its § reference):

§6.5.1  the instance is not an ACS-002 Map, or has a non-Text key            (§6.5.1)
§6.5.5  a key of the instance is absent from S.fields (closed schema / unknown
        field rejected)                                                       (§6.5.5)
§6.5.2  a field with card ∈ {1, 1..*} is absent (required-field presence)     (§6.5.2)
§6.5.4  a present field's value fails its §6.3 type code (type conformance),
        where the closed type-code set (§6.3) + int/float discipline (§7) is:
          null   Null                                                         (§6.3)
          bool   Bool                                                         (§6.3)
          int    Integer in [-2^63, 2^63-1]  (Timestamp i64; MUST be Integer,
                 NOT Float — §7.1/§7.3)                                        (§6.3/§7)
          u32    Integer in [0, 2^32-1]                                       (§6.3/§7)
          float  finite binary64                                              (§6.3/§7.2)
          conf   finite binary64 AND 0.0 ≤ v ≤ 1.0 (Confidence)               (§6.3/§7.2)
          text   Text (NFC UTF-8)                                             (§6.3)
          bytes  Bytes                                                        (§6.3)
          urn    Text beginning "urn:arves:" (EntityUrn form §5.1)            (§6.3/§5.1)
§6.5.4  a field with card ∈ {1..*, 0..*} whose value is not an Array, or a
        1..* Array that is empty (cardinality / multiplicity)                 (§6.4/§6.5.4)
        a field with card ∈ {1, 0..1} whose value IS an Array (a scalar
        cardinality carrying a multi-value)                                   (§6.4/§6.5.4)
§8      provenance state machine: `invocation` is present IFF `origin ==
        "derived"` — reject derived-without-invocation and
        non-derived-with-invocation                                           (§8)

SCOPED OUT (documented, not a silent gap):
  - §6.5.6 aspect-completeness at *registration* (a type that omits a mandatory
    aspect from S.aspects, §8) is a SCHEMA/registration check, not an instance
    check; this validator validates INSTANCES against a given schema, so it is
    out of scope here (the schemas we validate against already carry all five
    aspects). Aspect *carriers* ARE enforced instance-side via §6.5.4 typing of
    the carrier fields (urn/tenant/workspace/origin/source/invocation/confidence/
    valid_from/recorded_at) and the §8 origin↔invocation state machine.
  - `origin` ∈ {"observed","derived","asserted"} membership (§8): checked as a
    descriptive reject, since §8 fixes the closed Origin variant set and the §8
    state machine references it.
--------------------------------------------------------------------------------
"""

import sys

# Reuse our OWN independent Python only (permitted by the independence rules).
from acs002_dcbor import AInt, AFloat, encode
from acs002_decode import decode, _MapValue
from acs_values import acs004_instance, acs004_schema_document


class InstanceInvalid(Exception):
    """A §6.5/§7/§8 validation failure carrying a free-form, §-citing reason."""
    __slots__ = ("reason",)

    def __init__(self, reason):
        self.reason = reason
        super().__init__(reason)


# ---------------------------------------------------------------------------
# Bounds fixed by the ACS-004 §6.3 type-code table (and §7 int/float discipline).
# ---------------------------------------------------------------------------
_INT_MIN = -(2 ** 63)          # §6.3 int: signed i64 lower bound
_INT_MAX = (2 ** 63) - 1       # §6.3 int: signed i64 upper bound
_U32_MIN = 0                   # §6.3 u32 lower bound
_U32_MAX = (2 ** 32) - 1       # §6.3 u32 upper bound

# §8: the closed Origin variant set (frozen `Origin` variants).
_ORIGIN_VARIANTS = ("observed", "derived", "asserted")


def _is_finite_float(af):
    """True iff `af` is an ACS-002 Float carrying a finite binary64 (§5.3/§6.3).

    Decoded Floats are AFloat with a Python float in `.v`; the ACS-002 decoder has
    already rejected NaN/±Inf at the byte layer, but we re-check finiteness here so
    the type-code contract is self-standing (a pure function of the decoded value).
    """
    if not isinstance(af, AFloat):
        return False
    x = af.v
    # finite iff it equals itself (not NaN) and is not ±Inf.
    return x == x and x != float("inf") and x != float("-inf")


def _check_type(code, v):
    """
    ACS-004 §6.3 type-code check: does the decoded value `v` satisfy type `code`?

    Returns (ok: bool, why: str). `why` is a free-form reason (empty when ok) that
    cites the § the constraint comes from. Carrier + refinement per §6.3 / §7.

    Decoded-value kinds (from acs002_decode.decode):
      Null -> None ; Bool -> bool ; Integer -> AInt ; Float -> AFloat ;
      Text -> str ; Bytes -> bytes ; Array -> list ; Map -> _MapValue.
    """
    if code == "null":
        # §6.3 null: carrier Null.
        if v is None:
            return True, ""
        return False, "expected Null for type 'null' (§6.3)"

    if code == "bool":
        # §6.3 bool: carrier Bool. (bool is checked BEFORE int-like reasoning; a
        # decoded Bool is a Python bool, never an AInt, so no int/bool confusion.)
        if isinstance(v, bool):
            return True, ""
        return False, "expected Bool for type 'bool' (§6.3)"

    if code == "int":
        # §6.3 int / §7.1: MUST be an ACS-002 Integer (never a Float), in i64 range.
        if isinstance(v, bool) or not isinstance(v, AInt):
            return False, ("expected Integer for type 'int'; a Float or non-Integer "
                           "is rejected (§6.3/§7.1)")
        if not (_INT_MIN <= v.v <= _INT_MAX):
            return False, ("Integer %d out of int range [-2^63, 2^63-1] (§6.3)"
                           % v.v)
        return True, ""

    if code == "u32":
        # §6.3 u32 / §7: MUST be an ACS-002 Integer in [0, 2^32-1].
        if isinstance(v, bool) or not isinstance(v, AInt):
            return False, ("expected Integer for type 'u32'; a Float or non-Integer "
                           "is rejected (§6.3/§7)")
        if not (_U32_MIN <= v.v <= _U32_MAX):
            return False, "Integer %d out of u32 range [0, 2^32-1] (§6.3)" % v.v
        return True, ""

    if code == "float":
        # §6.3 float / §7.2: finite binary64. An Integer where the schema says float
        # is rejected (int/float discipline, §7).
        if not _is_finite_float(v):
            return False, ("expected finite binary64 Float for type 'float'; an "
                           "Integer or non-finite value is rejected (§6.3/§7.2)")
        return True, ""

    if code == "conf":
        # §6.3 conf / §7.2: finite binary64 AND 0.0 ≤ v ≤ 1.0 (Confidence).
        if not _is_finite_float(v):
            return False, ("expected finite binary64 Float for type 'conf'; an "
                           "Integer or non-finite value is rejected (§6.3/§7.2)")
        if not (0.0 <= v.v <= 1.0):
            return False, ("Confidence %r out of range [0.0, 1.0] for type 'conf' "
                           "(§6.3/§7.2)" % v.v)
        return True, ""

    if code == "text":
        # §6.3 text: carrier Text (NFC UTF-8; NFC already enforced by the decoder).
        if isinstance(v, str):
            return True, ""
        return False, "expected Text for type 'text' (§6.3)"

    if code == "bytes":
        # §6.3 bytes: carrier Bytes (opaque octets).
        if isinstance(v, (bytes, bytearray)):
            return True, ""
        return False, "expected Bytes for type 'bytes' (§6.3)"

    if code == "urn":
        # §6.3 urn: carrier Text that is an EntityUrn string (§5.1 full form:
        # begins "urn:arves:").
        if not isinstance(v, str):
            return False, "expected Text for type 'urn' (§6.3)"
        if not v.startswith("urn:arves:"):
            return False, ("Text %r is not an EntityUrn: must begin 'urn:arves:' "
                           "(§6.3/§5.1)" % v)
        return True, ""

    # A type code outside the closed §6.3 set: the schema itself is malformed.
    return False, "unknown ACS-004 type code %r (§6.3 code set is closed)" % code


def _map_entries(map_value):
    """
    Extract {field_name: decoded_value} from a decoded ACS-002 Map (_MapValue),
    enforcing §6.5.1: every key MUST be Text. A _MapValue keys entries as
    ('T', text) or ('I', int); an Integer key ('I', ...) is a non-Text key and is
    rejected here. Raises InstanceInvalid on a non-Map or a non-Text key.
    """
    if not isinstance(map_value, _MapValue):
        raise InstanceInvalid("instance is not an ACS-002 Map (§6.5.1)")
    out = {}
    for (kind, k) in map_value.keys():
        if kind != "T":
            raise InstanceInvalid(
                "instance Map key is not Text (found Integer key %r) (§6.5.1)" % (k,))
        out[k] = map_value[(kind, k)]
    return out


def _descriptor(field_name, desc_value):
    """
    Read a §6.2 field descriptor Map -> (type_code:str, card_code:str).

    The descriptor is a decoded _MapValue with Text keys 'type' and 'card' whose
    values are Text (§6.2). Raises InstanceInvalid if the schema descriptor is
    malformed (defensive; the schema we validate against is spec-derived).
    """
    d = _map_entries(desc_value)
    if "type" not in d or "card" not in d:
        raise InstanceInvalid(
            "schema field %r descriptor missing 'type'/'card' (§6.2)" % field_name)
    type_code = d["type"]
    card_code = d["card"]
    if not isinstance(type_code, str) or not isinstance(card_code, str):
        raise InstanceInvalid(
            "schema field %r descriptor 'type'/'card' must be Text (§6.2)"
            % field_name)
    return type_code, card_code


def validate_instance(instance_value, schema_value):
    """
    Validate a DECODED instance Map against a DECODED ACS-004 schema document, per
    §6.5 (with §7 int/float discipline and the §8 provenance state machine).

    Returns (ok: bool, reason: str): (True, "") if the instance satisfies every
    clause; (False, "<free-form reason citing §>") on the first violated clause.
    A conformant validator accepts IFF all §6.5 clauses hold and rejects otherwise
    (§6.5, observational equivalence). Validation is a pure function of
    (instance, schema): deterministic and platform-independent (§6.5).
    """
    try:
        # §6.5.1: instance is an ACS-002 Map with Text keys only.
        inst = _map_entries(instance_value)
    except InstanceInvalid as e:
        return False, e.reason

    # The schema document is a Map (§6); its `fields` is a Map<Text, descriptor>.
    try:
        schema = _map_entries(schema_value)
    except InstanceInvalid:
        return False, "schema document is not an ACS-002 Map (§6)"
    if "fields" not in schema:
        return False, "schema document has no 'fields' Map (§6)"
    try:
        fields = _map_entries(schema["fields"])
    except InstanceInvalid:
        return False, "schema 'fields' is not a Map (§6)"

    # §6.5.5: closed schema — reject any instance key not present in S.fields.
    for key in inst:
        if key not in fields:
            return False, ("unknown field %r not in schema (closed schema; §6.5.5)"
                           % key)

    # Per-field: presence (§6.5.2/§6.5.3), type (§6.5.4/§6.3/§7), cardinality (§6.4).
    for fname, desc_value in fields.items():
        try:
            type_code, card_code = _descriptor(fname, desc_value)
        except InstanceInvalid as e:
            return False, e.reason

        present = fname in inst

        # §6.5.2: card ∈ {1, 1..*} MUST be present.
        if card_code in ("1", "1..*") and not present:
            return False, ("required field %r (card %s) absent (§6.5.2)"
                           % (fname, card_code))
        # §6.5.3: card ∈ {0..1, 0..*} MAY be absent -> nothing more to check.
        if not present:
            continue

        val = inst[fname]

        if card_code in ("1", "0..1"):
            # Single-valued cardinality. An Array here is a cardinality violation
            # (a scalar field carrying a multi-value) — §6.4/§6.5.4.
            if isinstance(val, list):
                return False, ("field %r has cardinality %s (single) but value is "
                               "an Array (§6.4/§6.5.4)" % (fname, card_code))
            ok, why = _check_type(type_code, val)
            if not ok:
                return False, "field %r: %s" % (fname, why)

        elif card_code in ("1..*", "0..*"):
            # Multi-valued cardinality: value MUST be an Array (§6.4/§6.5.4).
            if not isinstance(val, list):
                return False, ("field %r (card %s) must be an Array (§6.4/§6.5.4)"
                               % (fname, card_code))
            if card_code == "1..*" and len(val) < 1:
                return False, ("field %r (card 1..*) must be a non-empty Array "
                               "(§6.4/§6.5.4)" % fname)
            for i, el in enumerate(val):
                # Each element itself may not be an Array (elements are scalar of
                # `type`; no nested-array element form in §6.4).
                if isinstance(el, list):
                    return False, ("field %r element %d is an Array, not a scalar "
                                   "%s (§6.4)" % (fname, i, type_code))
                ok, why = _check_type(type_code, el)
                if not ok:
                    return False, "field %r element %d: %s" % (fname, i, why)

        else:
            # Cardinality code outside the closed §6.4 set -> malformed schema.
            return False, ("field %r has unknown cardinality code %r (§6.4 set is "
                           "closed)" % (fname, card_code))

    # §8: provenance state machine. `origin` is a closed Origin variant, and
    # `invocation` is present IFF `origin == "derived"`. `origin`/`invocation` are
    # aspect carriers already type-checked above; here we enforce the state machine.
    if "origin" in inst:
        origin = inst["origin"]
        if isinstance(origin, str):
            # §8: closed Origin variant set.
            if origin not in _ORIGIN_VARIANTS:
                return False, ("origin %r is not one of %s (§8)"
                               % (origin, _ORIGIN_VARIANTS))
            has_invocation = "invocation" in inst
            if origin == "derived" and not has_invocation:
                return False, ("origin == 'derived' requires 'invocation' to be "
                               "present (§8)")
            if origin != "derived" and has_invocation:
                return False, ("'invocation' is present but origin != 'derived' "
                               "(must be absent, not Null) (§8)")

    return True, ""


# ---------------------------------------------------------------------------
# Self-test — the proof. POSITIVE (accept) + one NEGATIVE (reject) per rule.
#
# Every case starts from the spec-derived VALID instance/schema (acs_values),
# applies EXACTLY ONE mutation, then round-trips encode -> decode so the input to
# validate_instance is a genuinely DECODED value (the same shape a real body would
# produce). The mutated bodies remain valid canonical dCBOR, so `decode` succeeds;
# the SEMANTIC validator MUST reject them — that is the whole point.
# ---------------------------------------------------------------------------

def _decode_pair(instance_dict, schema_dict):
    """encode -> decode both dicts, returning the decoded (_MapValue, _MapValue)."""
    inst_dec = decode(encode(instance_dict))
    schema_dec = decode(encode(schema_dict))
    return inst_dec, schema_dec


def run_selftest():
    schema = acs004_schema_document()          # §11.2 valid schema
    results = []                                # (name, passed, detail)

    # -- POSITIVE: the §11.3 valid instance MUST be accepted (ACS-004-CS-1 cl.2). --
    inst = acs004_instance()
    inst_dec, schema_dec = _decode_pair(inst, schema)
    ok, reason = validate_instance(inst_dec, schema_dec)
    results.append(("POSITIVE valid uci.fact@1.0 accepted (§6.5)", ok,
                    "" if ok else "unexpected reject: " + reason))

    # Helper: mutate a fresh copy of the valid instance, expect a REJECT.
    def expect_reject(name, mutate):
        m = acs004_instance()
        mutate(m)
        try:
            i_dec, s_dec = _decode_pair(m, schema)
        except Exception as e:  # pragma: no cover - a mutation that breaks bytes
            results.append((name, False,
                            "mutation did not stay valid canonical dCBOR: %r" % e))
            return
        ok2, reason2 = validate_instance(i_dec, s_dec)
        results.append((name, (not ok2),
                        ("wrongly ACCEPTED" if ok2
                         else "rejected: " + reason2)))

    # -- NEGATIVE 1: unknown field (closed schema, §6.5.5). --
    def m_unknown(m):
        m["surprise"] = "extra"                 # key absent from S.fields
    expect_reject("NEG unknown field rejected (§6.5.5)", m_unknown)

    # -- NEGATIVE 2: required field absent (§6.5.2) — drop `claim` (card 1). --
    def m_missing(m):
        del m["claim"]
    expect_reject("NEG required field 'claim' absent rejected (§6.5.2)", m_missing)

    # -- NEGATIVE 3: type failure for `conf` — confidence 1.5 out of [0,1] (§6.3/§7.2). --
    def m_conf(m):
        m["confidence"] = AFloat(1.5)
    expect_reject("NEG conf out of [0,1] rejected (§6.3/§7.2)", m_conf)

    # -- NEGATIVE 4: type failure for `int` — valid_from as Text, not Integer (§6.3/§7.1). --
    def m_int(m):
        m["valid_from"] = "not-an-integer"      # Text where schema says int
    expect_reject("NEG int field as Text rejected (§6.3/§7.1)", m_int)

    # -- NEGATIVE 5: type failure for `urn` — urn not starting 'urn:arves:' (§6.3/§5.1). --
    def m_urn(m):
        m["urn"] = "not-a-urn"                   # Text but not an EntityUrn
    expect_reject("NEG urn not 'urn:arves:' rejected (§6.3/§5.1)", m_urn)

    # -- NEGATIVE 6: u32 out of range. No u32 field exists in uci.fact@1.0, so this
    #    rule is exercised by a SYNTHETIC one-field schema + instance (the §6.3 u32
    #    code is real and tested here rather than skipped). --
    def neg_u32():
        u32_schema = {
            "urn": "uci.probe",
            "ver": {"major": AInt(1), "minor": AInt(0)},
            "root": "Fact",
            "aspects": ["Identity", "Provenance", "Temporal", "Trust", "TenantScope"],
            "fields": {"count": {"type": "u32", "card": "1"}},
        }
        bad = {"count": AInt(_U32_MAX + 1)}      # 2^32, just past the u32 ceiling
        i_dec, s_dec = _decode_pair(bad, u32_schema)
        ok3, reason3 = validate_instance(i_dec, s_dec)
        results.append(("NEG u32 out of [0,2^32-1] rejected (§6.3/§7)",
                        (not ok3),
                        ("wrongly ACCEPTED" if ok3 else "rejected: " + reason3)))
    neg_u32()

    # -- NEGATIVE 7: cardinality — `evidence` (0..*) as a non-Array scalar (§6.4/§6.5.4). --
    def m_card_scalar(m):
        m["evidence"] = "urn:arves:uci.core:evidence@1.0:e-42"  # scalar, not Array
    expect_reject("NEG 0..* field as non-Array rejected (§6.4/§6.5.4)", m_card_scalar)

    # -- NEGATIVE 7b: cardinality — single field `claim` (card 1) as an Array (§6.4/§6.5.4). --
    def m_card_array(m):
        m["claim"] = ["sky-is-blue"]             # Array where card 1 (single) is required
    expect_reject("NEG card-1 field as Array rejected (§6.4/§6.5.4)", m_card_array)

    # -- NEGATIVE 8a: §8 violation A — origin=="derived" but no invocation. --
    def m_derived_no_inv(m):
        m["origin"] = "derived"                  # but leave invocation ABSENT
    expect_reject("NEG origin=derived without invocation rejected (§8)",
                  m_derived_no_inv)

    # -- NEGATIVE 8b: §8 violation B — invocation present but origin=="observed". --
    def m_observed_with_inv(m):
        # origin stays "observed" (valid instance default); add an invocation urn.
        m["invocation"] = "urn:arves:uci.core:invocation@1.0:inv-9"
    expect_reject("NEG invocation present with origin=observed rejected (§8)",
                  m_observed_with_inv)

    # ---- Report ----
    print("ACS-004 instance validator (§6.5/§7/§8) — independent self-test")
    print("-" * 72)
    all_pass = True
    for name, passed, detail in results:
        tag = "PASS" if passed else "FAIL"
        if not passed:
            all_pass = False
        line = "  [%s] %s" % (tag, name)
        if detail:
            line += "  -- " + detail
        print(line)
    print("-" * 72)
    npos = 1
    nneg = len(results) - npos
    print("summary: %d/%d cases passed (%d positive + %d negative); overall %s"
          % (sum(1 for _, p, _ in results if p), len(results), npos, nneg,
             "PASS" if all_pass else "FAIL"))
    return 0 if all_pass else 1


if __name__ == "__main__":
    sys.exit(run_selftest())

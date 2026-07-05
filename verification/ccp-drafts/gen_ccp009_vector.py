#!/usr/bin/env python3
"""
CCP-009 (DRAFT) — open-debt #20: ACS-004 urn<->type binding. Generate + verify the candidate
negative vectors, and DEMONSTRATE the gap in the current validator.

#20 (OPEN_DEBT_REGISTER.md): ACS-004 §5.1 makes the urn<->type binding NORMATIVE — the full
EntityUrn's "`type-name@version` segment MUST equal the type's short form modulo namespace" —
but §6.5 (the validation obligation) has NO clause enforcing it, and "modulo namespace" is
underspecified for `uci.fact` (schema short name) vs `urn:arves:uci.core:fact@1.0:...`
(instance EntityUrn, whose namespace segment is `uci.core` and whose name segment is `fact`).
Consequence: an instance whose `urn` names a DIFFERENT type (`...:goal@1.0:...`) validates
clean against the `uci.fact@1.0` schema — a well-typed value carrying a false type identity.
Two honest runtimes then disagree on whether such an instance is a `uci.fact`, and every
downstream consumer that trusts the Identity aspect (ABI resolution §4, registry lookup §9)
is routed to the wrong schema.

PROPOSED FIX (byte-clean, Option A of the draft): add §6.5 clause 7 — a clause-4-clean
Identity-carrier `urn` (Text with the 'urn:arves:' prefix) MUST parse as the PINNED §5.1 full
form `urn:arves:<namespace>:<name>@<major>.<minor>:<local-id>` (<major>/<minor> canonical
decimals, no leading zeros) — a prefixed non-parsing urn ('urn:arves:junk', 'fact@01.0') is a
clause-7 REJECT, closing the malformed-urn bypass — and MUST satisfy
`<name> == S.urn stripped of its "uci." registry prefix` AND `<major>.<minor> == S.ver`,
DISREGARDING `<namespace>` (the literal meaning of §5.1's "modulo namespace"). Violation is
rejected `urn-type-mismatch` (a new ACS-001 §4.1 reason-code registration). The §11.3 golden
instance and the §11.4 derived variant BOTH satisfy the rule, so NO golden ContentId changes.

This script (freeze-clean; verification/ccp-drafts/) proves:
  1. each candidate DECODES CLEAN as canonical dCBOR;
  2. the CURRENT `acs004_instance.validate_instance` ACCEPTS it -> the #20 gap is real;
  3. the PROPOSED clause-7 rule REJECTS it with `urn-type-mismatch` -> the fix is implementable;
  4. the §11.3 golden instance (and §11.4 derived variant) PASS the proposed rule -> byte-clean.

Run:  python verification/ccp-drafts/gen_ccp009_vector.py
"""

import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
sys.path.insert(0, PYDIR)

from acs002_dcbor import encode                                   # noqa: E402
from acs002_decode import decode, _MapValue                       # noqa: E402
from acs001_address import content_id                             # noqa: E402
import acs_values as V                                            # noqa: E402
from acs004_instance import validate_instance                     # noqa: E402

OUT = os.path.join(HERE, "ccp009_candidate_vectors.tsv")

# §5.1 full EntityUrn form, PINNED (proposed §6.5 clause 7):
#   urn:arves:<namespace>:<name>@<major>.<minor>:<local-id>
# <namespace>/<name> non-empty, no ':' or '@'; <major>/<minor> canonical decimal integers
# (0, or a digit string with NO leading zero — 'fact@01.0' is NOT the full form);
# <local-id> non-empty.
_CANON_INT = r"(0|[1-9][0-9]*)"
_FULL_URN = re.compile(
    r"^urn:arves:([^:@]+):([^:@]+)@" + _CANON_INT + r"\." + _CANON_INT + r":(.+)$")


def _entries(map_value):
    """Decoded _MapValue -> {text_key: value} (Text keys only, §6.5.1)."""
    assert isinstance(map_value, _MapValue)
    out = {}
    for (kind, k) in map_value.keys():
        assert kind == "T"
        out[k] = map_value[(kind, k)]
    return out


def proposed_rule_rejects(inst_value, schema_value):
    """The CCP-009 proposed §6.5 clause 7 (Option A): urn<->type binding, modulo namespace.

    Applies only to a clause-4-clean urn (Text with the 'urn:arves:' prefix — the check the
    reference validators enforce for §6.3 today). Such a urn MUST parse as the PINNED §5.1
    full form (else REJECT urn-type-mismatch — this closes the 'urn:arves:junk' bypass) and
    MUST satisfy name == S.urn minus its 'uci.' registry prefix AND major.minor == S.ver,
    DISREGARDING the namespace segment. Returns (rejected, reason_code); the ratification
    oracle for the clause.
    """
    inst = _entries(inst_value)
    schema = _entries(schema_value)
    urn_val = inst.get("urn")
    if not isinstance(urn_val, str) or not urn_val.startswith("urn:arves:"):
        return (False, "")                      # clause-4 territory (field-type-mismatch)
    m = _FULL_URN.match(urn_val)
    if m is None:
        # clause-4-clean but NOT the pinned §5.1 full form (e.g. 'urn:arves:junk',
        # leading-zero version segments) -> clause-7 REJECT: it cannot bind to any type.
        return (True, "urn-type-mismatch")
    _namespace, name, major, minor = m.group(1), m.group(2), int(m.group(3)), int(m.group(4))
    short = schema["urn"]                       # e.g. "uci.fact" (§6, no version)
    expected_name = short[len("uci."):] if short.startswith("uci.") else short
    ver = _entries(schema["ver"])
    expected_major, expected_minor = ver["major"].v, ver["minor"].v
    if name != expected_name or major != expected_major or minor != expected_minor:
        return (True, "urn-type-mismatch")
    return (False, "")


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

    schema_dec = decode(encode(V.acs004_schema_document()))       # §11.2, authoritative

    # ---- Byte-clean check: the golden §11.3 instance MUST pass the proposed rule. ----
    golden = V.acs004_instance()
    golden_body = encode(golden)
    golden_dec = decode(golden_body)
    g_ok, g_reason = validate_instance(golden_dec, schema_dec)
    g_rej, _ = proposed_rule_rejects(golden_dec, schema_dec)
    golden_clean = g_ok and not g_rej

    # §11.4 derived variant must also stay clean under the proposed rule.
    derived = V.acs004_instance()
    derived["origin"] = "derived"
    derived["invocation"] = "urn:arves:uci.core:invocation@1.0:inv-9"
    derived_dec = decode(encode(derived))
    d_ok, _ = validate_instance(derived_dec, schema_dec)
    d_rej, _ = proposed_rule_rejects(derived_dec, schema_dec)
    derived_clean = d_ok and not d_rej

    # ---- Candidates: golden instance with ONE mutation each — a §5.1-violating urn. ----
    candidates = [
        # (case, mutated urn) — type-NAME mismatch: a goal urn on a fact instance.
        ("instance-urn-type-mismatch",
         "urn:arves:uci.core:goal@1.0:f-1730000000"),
        # type-VERSION mismatch: fact@2.0 urn validated against the fact@1.0 schema.
        ("instance-urn-version-mismatch",
         "urn:arves:uci.core:fact@2.0:f-1730000000"),
        # prefixed but NOT the §5.1 full form: passes today's prefix-only §6.3 check,
        # would escape a binding rule that deferred non-parsing urns to clause 4.
        ("instance-urn-not-full-form",
         "urn:arves:junk"),
        # leading-zero version segment: not a canonical decimal -> not the pinned full
        # form. Machine-pins the grammar so implementations cannot diverge on '01'.
        ("instance-urn-version-leading-zero",
         "urn:arves:uci.core:fact@01.0:f-1730000000"),
    ]

    print("CCP-009 candidates — ACS-004 urn<->type binding (open-debt #20)")
    print("=" * 70)
    print("  golden §11.3 passes proposed rule :", golden_clean, "(byte-clean: no golden change)")
    print("  derived §11.4 passes proposed rule:", derived_clean)

    rows = []
    all_ok = golden_clean and derived_clean
    for case, bad_urn in candidates:
        inst = V.acs004_instance()
        inst["urn"] = bad_urn
        body = encode(inst)

        # (1) decodes clean?
        try:
            value = decode(body)
            decode_ok = True
        except Exception as e:  # noqa: BLE001
            print("FAIL: %s does not decode-clean: %s" % (case, e))
            return 1

        # (2) does the CURRENT §6.5 validator accept it (proving the #20 gap)?
        cur_ok, cur_reason = validate_instance(value, schema_dec)

        # (3) does the PROPOSED clause-7 rule reject it?
        rejected, code = proposed_rule_rejects(value, schema_dec)

        cid = content_id(0x01, body)            # §10: committed fact -> tag 0x01

        print("-" * 70)
        print("  case                  :", case)
        print("  instance urn          :", bad_urn)
        print("  schema type           : uci.fact@1.0  (S.urn='uci.fact', S.ver=1.0)")
        print("  decodes-clean         :", decode_ok)
        print("  CURRENT validate_instance:",
              "ACCEPTS (#20 gap confirmed)" if cur_ok else ("rejects: " + cur_reason))
        print("  PROPOSED clause 7     :",
              ("REJECTS -> " + code) if rejected else "accepts (fix not effective!)")
        print("  canonical body (hex)  :", body.hex())
        print("  ContentId (tag 0x01)  :", cid.hex())

        ok = decode_ok and cur_ok and rejected and code == "urn-type-mismatch"
        all_ok = all_ok and ok
        rows.append((case, body.hex(), code))

    print("-" * 70)
    if not all_ok:
        print("RESULT: RED — the demonstration did not hold")
        return 1

    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("standard\tcase\ttier\tinput_hex\treject_reason\n")
        for case, hexs, code in rows:
            f.write("ACS-004\t%s\tinstance\t%s\t%s\n" % (case, hexs, code))
    print("WROTE candidates ->", os.path.relpath(OUT, ROOT).replace(os.sep, "/"))
    print("RESULT: GREEN — #20 gap demonstrated; proposed rule rejects; goldens unchanged (DRAFT).")
    print("(Ratification adds the §6.5 clause + the reason code + these vectors to the frozen Kit — CCP-GATE.)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

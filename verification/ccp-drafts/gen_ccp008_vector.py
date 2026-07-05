#!/usr/bin/env python3
"""
CCP-008 (DRAFT) — B2: root-event `causation_id` canonicalization. Generate + verify the ONE
candidate negative vector, and DEMONSTRATE the B2 gap in the current validator.

B2 (G2_READINESS.md): ACS-003 makes `causation_id` the sole OPTIONAL field (Text | Null). For a
ROOT event (no cause) two encodings are therefore lawful — *present with the explicit Null* and
*absent* — and they produce DIFFERENT canonical bodies and thus DIFFERENT ContentIds for "the same"
root event. Two honest implementers diverge on the most common envelope, and the Kernel's
ORCH-004 idempotency dedup silently forks. This is exactly the "two lawful encodings, one identity"
trap the standard exists to preclude.

PROPOSED FIX (byte-clean): ACS-003 §5 adds a MUST — a root event's `causation_id` SHALL be
*present with the explicit Null* (the encoding the single golden envelope already uses, so NO
golden ContentId changes); an envelope that OMITS `causation_id` is rejected `missing-required-field`.
This removes the second lawful encoding.

This script (freeze-clean; verification/ccp-drafts/) proves:
  1. the candidate (golden envelope with `causation_id` REMOVED) DECODES CLEAN as canonical dCBOR;
  2. the CURRENT `acs003_envelope.validate_envelope` ACCEPTS it  -> the B2 gap is real;
  3. the PROPOSED rule (causation_id MUST be present) REJECTS it -> the fix is implementable.

Run:  python verification/ccp-drafts/gen_ccp008_vector.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
sys.path.insert(0, PYDIR)

from acs002_dcbor import encode                                   # noqa: E402
from acs002_decode import decode, _MapValue                      # noqa: E402
from acs001_address import content_id                            # noqa: E402
import acs_values as V                                           # noqa: E402
from acs003_envelope import validate_envelope, EnvelopeInvalid   # noqa: E402

OUT = os.path.join(HERE, "ccp008_candidate_vector.tsv")

VALID_CID = content_id(0x01, encode(V.acs002_v1_fact()))


def proposed_rule_rejects(value):
    """The CCP-008 proposed §5 rule: `causation_id` MUST be present (root -> present-with-Null).
    Returns (rejected, reason_code). Implemented here as the ratification oracle."""
    if not isinstance(value, _MapValue):
        return (False, "")
    present = {k for (kind, k) in value.keys() if kind == "T"}
    if "causation_id" not in present:
        return (True, "missing-required-field")
    return (False, "")


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

    # Candidate: the valid §10.2 envelope with `causation_id` REMOVED (a lawful root encoding today).
    env = dict(V.acs003_envelope(VALID_CID))
    assert "causation_id" in env, "fixture must start with causation_id present (golden = Null)"
    del env["causation_id"]
    body = encode(env)

    # (1) decodes clean?
    try:
        value = decode(body)
        decode_ok = True
    except Exception as e:  # noqa: BLE001
        print("FAIL: candidate does not decode-clean:", e)
        return 1

    # (2) does the CURRENT validator accept it (proving the B2 gap)?
    try:
        validate_envelope(value)
        current_accepts = True
    except EnvelopeInvalid as e:
        current_accepts = False
        current_reason = str(e)

    # (3) does the PROPOSED rule reject it?
    rejected, code = proposed_rule_rejects(value)

    # The two lawful ContentIds that make B2 concrete: present-Null vs absent.
    cid_present_null = content_id(0x06, encode(V.acs003_envelope(VALID_CID)))  # golden (causation_id=Null)
    cid_absent = content_id(0x06, body)                                        # this candidate (absent)

    print("CCP-008 candidate — root-event causation_id (B2)")
    print("=" * 66)
    print("  decodes-clean         :", decode_ok)
    print("  CURRENT validator     :", "ACCEPTS (B2 gap confirmed)" if current_accepts
          else ("rejects: " + current_reason))
    print("  PROPOSED rule         :", ("REJECTS -> " + code) if rejected else "accepts (fix not effective!)")
    print("  ContentId present-Null:", cid_present_null.hex())
    print("  ContentId absent      :", cid_absent.hex())
    print("  -> two lawful encodings, two ContentIds:",
          "DIVERGE (B2)" if cid_present_null != cid_absent else "same")

    ok = decode_ok and current_accepts and rejected and (cid_present_null != cid_absent)
    if not ok:
        print("RESULT: RED — the demonstration did not hold")
        return 1

    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("standard\tcase\ttier\tinput_hex\treject_reason\n")
        f.write("ACS-003\tenvelope-root-omits-causation_id\tenvelope\t%s\t%s\n" % (body.hex(), code))
    print("WROTE candidate ->", os.path.relpath(OUT, ROOT).replace(os.sep, "/"))
    print("RESULT: GREEN — B2 gap demonstrated; proposed rule rejects; vector well-formed (DRAFT).")
    print("(Ratification adds the §5 MUST + the validator rule + this vector to the frozen Kit — CCP-GATE.)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

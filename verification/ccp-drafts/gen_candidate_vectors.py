#!/usr/bin/env python3
"""
CCP-006 (DRAFT) — generate + oracle-verify CANDIDATE negative vectors for the ACS-003 / ACS-004
/ ACS-005 reject surfaces (SYSTEM_GAP_ANALYSIS #1/#2/#23).

FREEZE-CLEAN: this is a DRAFT generator living under verification/ccp-drafts/. It writes ONLY
`candidate_negative_vectors.tsv` next to itself. It does NOT touch standard/ (the frozen vector
set + CONFORMANCE.md reason-code registry) — shipping these vectors + the new reason codes into
the Kit is the ratification step (CCP-GATE + ACS-001 §4.1), gated on the maintainer.

Each candidate is machine-verified against the living reference validators (the ORACLE):
  1. the defect is built from a VALID structure with exactly ONE mutation;
  2. it ENCODES to canonical dCBOR that `decode()` ACCEPTS (so the ACS-002 layer passes — the
     defect is purely semantic, exactly the surface the current gate never exercises);
  3. the reference validator REJECTS it — and we record the PROPOSED stable reason code.

If any candidate fails to decode-clean or fails to be rejected, this script exits non-zero: the
draft TSV is only emitted when every row is oracle-confirmed.

Run:  python verification/ccp-drafts/gen_candidate_vectors.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
sys.path.insert(0, PYDIR)

from acs002_dcbor import encode, AInt, AFloat            # noqa: E402
from acs002_decode import decode                          # noqa: E402
from acs001_address import content_id                     # noqa: E402
import acs_values as V                                     # noqa: E402
from acs003_envelope import validate_envelope, EnvelopeInvalid   # noqa: E402
from acs004_instance import validate_instance, InstanceInvalid   # noqa: E402
from acs005_checker import check_term_set                  # noqa: E402

OUT = os.path.join(HERE, "candidate_negative_vectors.tsv")

# Proposed stable reject reason codes (kebab-case, matching the ACS-002 style). These do NOT yet
# exist in the closed CONFORMANCE.md registry — adding them is the CCP (ACS-001 §4.1).
#   missing-required-field  unknown-field  field-type-mismatch  value-out-of-range
#   malformed-content-id  empty-shard-scope  cardinality-violation  provenance-invariant
#   terms-not-sorted  duplicate-term  malformed-term-list

VALID_CID = content_id(0x01, encode(V.acs002_v1_fact()))   # the ACS-003 payload_cid (34 bytes)


def _env(mut):
    d = dict(V.acs003_envelope(VALID_CID))
    mut(d)
    return d


def _inst(mut):
    d = dict(V.acs004_instance())
    mut(d)
    return d


# (standard, case, tier, proposed_code, kind, payload)
#   kind 'env'  : payload is a mutated envelope dict  -> encode -> decode -> validate_envelope
#   kind 'inst' : payload is a mutated instance dict  -> encode -> decode -> validate_instance
#   kind 'lang' : payload is raw bytes                -> check_term_set
def _drop(k):
    return lambda d: d.pop(k, None)


def _set(k, v):
    return lambda d: d.__setitem__(k, v)


CASES = [
    # ---- ACS-003 Canonical Envelope (§5/§6.3) ----
    ("ACS-003", "envelope-missing-required-field", "envelope", "missing-required-field", "env", _env(_drop("event_type"))),
    ("ACS-003", "envelope-unknown-field", "envelope", "unknown-field", "env", _env(_set("x_extra", "nope"))),
    ("ACS-003", "envelope-occurred_at-as-text", "envelope", "field-type-mismatch", "env", _env(_set("occurred_at", "1730000000000000000"))),
    ("ACS-003", "envelope-causation_id-wrong-type", "envelope", "field-type-mismatch", "env", _env(_set("causation_id", AInt(7)))),
    ("ACS-003", "envelope-payload_cid-33-bytes", "envelope", "malformed-content-id", "env", _env(_set("payload_cid", bytes([0x12, 0x20]) + b"\x00" * 31))),
    ("ACS-003", "envelope-payload_cid-wrong-prefix", "envelope", "malformed-content-id", "env", _env(_set("payload_cid", bytes([0x00, 0x20]) + b"\x00" * 32))),
    ("ACS-003", "envelope-empty-tenant", "envelope", "empty-shard-scope", "env", _env(_set("tenant_id", ""))),
    # ---- ACS-004 Universal Type Registry — instance validation (§6.5/§7/§8) ----
    ("ACS-004", "instance-unknown-field", "instance", "unknown-field", "inst", _inst(_set("x_extra", "nope"))),
    ("ACS-004", "instance-missing-required-field", "instance", "missing-required-field", "inst", _inst(_drop("claim"))),
    ("ACS-004", "instance-conf-out-of-range", "instance", "value-out-of-range", "inst", _inst(_set("confidence", AFloat(1.5)))),
    # CCP-007 (#19): a valid ACS-002 Integer above the `int` i64 ceiling (2^63) — decodes
    # clean (ACS-002 range is [-2^64, 2^64-1]) but is out of the ACS-004 `int` [-2^63, 2^63-1].
    ("ACS-004", "instance-int-above-i64", "instance", "value-out-of-range", "inst", _inst(_set("valid_from", AInt(2**63)))),
    ("ACS-004", "instance-int-as-text", "instance", "field-type-mismatch", "inst", _inst(_set("valid_from", "not-an-int"))),
    ("ACS-004", "instance-urn-not-arves", "instance", "field-type-mismatch", "inst", _inst(_set("urn", "not-a-urn"))),
    ("ACS-004", "instance-evidence-scalar-not-array", "instance", "cardinality-violation", "inst", _inst(_set("evidence", AInt(1)))),
    ("ACS-004", "instance-derived-without-invocation", "instance", "provenance-invariant", "inst", _inst(_set("origin", "derived"))),
]

# ACS-005 language cases (raw bytes over the term-set body).
def _lang_cases():
    base = V.acs005_term_set_body().decode("utf-8")
    lines = base.split("\n")
    unsorted = "\n".join([lines[1], lines[0]] + lines[2:])          # first two swapped
    dup = "\n".join([lines[0]] + lines)                             # duplicate first entry
    trailing = base + "\n"                                          # trailing LF
    badgram = "\n".join(lines[:-1] + ["GL-01A"])                    # malformed ID (stays sorted; fails GL-\d{3})
    return [
        ("ACS-005", "termset-not-sorted", "language", "terms-not-sorted", unsorted.encode()),
        ("ACS-005", "termset-duplicate", "language", "duplicate-term", dup.encode()),
        ("ACS-005", "termset-trailing-lf", "language", "malformed-term-list", trailing.encode()),
        ("ACS-005", "termset-bad-grammar", "language", "malformed-term-list", badgram.encode()),
    ]


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    rows = []       # (standard, case, tier, input_hex, reject_reason)
    report = []     # (case, decode_ok, rejected, oracle_reason)
    failures = []

    for (std, case, tier, code, kind, payload) in CASES:
        try:
            body = encode(payload)
        except Exception as e:  # noqa: BLE001
            failures.append("%s: encode failed: %s" % (case, e))
            continue
        # (2) MUST decode-clean (defect is purely semantic, ACS-002 layer passes)
        try:
            value = decode(body)
        except Exception as e:  # noqa: BLE001
            failures.append("%s: does NOT decode-clean (%s) — not a pure semantic defect" % (case, e))
            continue
        # (3) oracle MUST reject
        rejected = False
        oracle_reason = ""
        try:
            if kind == "env":
                validate_envelope(value)          # raises EnvelopeInvalid on reject
            elif kind == "inst":
                schema_val = decode(encode(V.acs004_schema_document()))
                ok, reason = validate_instance(value, schema_val)   # RETURNS (ok, reason)
                if not ok:
                    rejected, oracle_reason = True, reason
            else:
                raise AssertionError("unexpected kind")
        except (EnvelopeInvalid, InstanceInvalid) as e:
            rejected = True
            oracle_reason = str(e)
        if not rejected:
            failures.append("%s: oracle ACCEPTED a body that must be rejected (%s)" % (case, code))
            continue
        rows.append((std, case, tier, body.hex(), code))
        report.append((case, True, True, oracle_reason))

    for (std, case, tier, code, body) in _lang_cases():
        ok, reason = check_term_set(body)
        if ok:
            failures.append("%s: ACS-005 checker ACCEPTED a body that must be rejected (%s)" % (case, code))
            continue
        rows.append((std, case, tier, body.hex(), code))
        report.append((case, "n/a", True, reason))

    print("CCP-006 candidate negative-vector generation (oracle-verified)")
    print("=" * 70)
    for (case, dec, rej, reason) in report:
        print("  [OK] %-38s decode=%s reject=%s" % (case, dec, rej))
        print("        oracle: %s" % (reason[:110]))
    print("-" * 70)
    if failures:
        print("FAILURES (%d) — TSV NOT written:" % len(failures))
        for f in failures:
            print("  - " + f)
        return 1

    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("standard\tcase\ttier\tinput_hex\treject_reason\n")
        for (std, case, tier, hexs, code) in rows:
            f.write("%s\t%s\t%s\t%s\t%s\n" % (std, case, tier, hexs, code))
    codes = sorted(set(r[4] for r in rows))
    print("WROTE %d oracle-verified candidate vectors -> %s"
          % (len(rows), os.path.relpath(OUT, ROOT).replace(os.sep, "/")))
    print("Proposed reject reason codes (%d): %s" % (len(codes), ", ".join(codes)))
    print("(These codes + vectors enter the FROZEN Kit only via a CCP Amendment — ACS-001 §4.1.)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

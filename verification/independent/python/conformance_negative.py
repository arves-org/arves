"""
ARVES Negative Conformance runner — ACS-002 rejection check.

Follows standard/conformance/CONFORMANCE.md "The rejection check (negative
vectors)": a conformant DECODER MUST reject every byte string that is not in
canonical form, with the stable reason code the Kit lists (ACS-002 §5).

For each row in standard/vectors/acs_negative_vectors.tsv
(standard, case, tier, input_hex, reject_reason):
  - tier = core : decode(input_hex) MUST raise Rejected with .reason == reject_reason.
  - tier = nfc  : the input is valid UTF-8 but not NFC; because Python has
                  unicodedata we ENFORCE it and MUST reject with 'non-nfc-text'.

It ALSO re-verifies that the decoder ACCEPTS canonical input: it rebuilds the
ACS-002 positive bodies V1/V2/V3 (from acs_values + our own encoder) and asserts
decode(body) round-trips — reencode(decode(body)) == body (§6.2 idempotency).
This proves the decoder accepts canonical and rejects ONLY non-canonical.

No reference-runtime source was consulted; nothing under runtime/ and no Rust was
read.
"""

import os
import sys

from acs002_decode import decode, reencode, Rejected
from acs002_dcbor import encode
import acs_values as V


HERE = os.path.dirname(os.path.abspath(__file__))
NEG_TSV = os.path.normpath(os.path.join(
    HERE, "..", "..", "..", "standard", "vectors", "acs_negative_vectors.tsv"))


def load_negative_rows():
    rows = []
    with open(NEG_TSV, "r", encoding="utf-8") as f:
        header = f.readline().rstrip("\n").split("\t")
        assert header == ["standard", "case", "tier", "input_hex", "reject_reason"], \
            "unexpected negative-TSV header: %r" % (header,)
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            standard, case, tier, input_hex, reject_reason = line.split("\t")
            rows.append({
                "standard": standard,
                "case": case,
                "tier": tier,
                "input_hex": input_hex.strip().lower(),
                "reject_reason": reject_reason.strip(),
            })
    return rows


def run_rejection_check(rows):
    """Decode each input; a PASS means it was rejected with the expected reason."""
    core_total = 0
    core_rejected_ok = 0
    nfc_total = 0
    nfc_rejected_ok = 0
    mismatches = []

    print("ARVES Negative Conformance — ACS-002")
    print("  (a conformant decoder MUST reject every non-canonical body, §5/§6.3)")
    print("")

    for r in rows:
        raw = bytes.fromhex(r["input_hex"])
        want = r["reject_reason"]
        tier = r["tier"]
        if tier == "core":
            core_total += 1
        elif tier == "nfc":
            nfc_total += 1

        got_reason = None
        outcome = None
        try:
            value = decode(raw)
            # Decoder ACCEPTED a body that MUST be rejected -> FAIL.
            outcome = "FAIL"
            detail = ("decoder ACCEPTED non-canonical input (value=%r); "
                      "expected rejection %s" % (value, want))
            mismatches.append((r, "ACCEPTED(no rejection)", want))
        except Rejected as e:
            got_reason = e.reason
            if got_reason == want:
                outcome = "PASS"
                detail = "rejected: %s" % got_reason
                if tier == "core":
                    core_rejected_ok += 1
                elif tier == "nfc":
                    nfc_rejected_ok += 1
            else:
                outcome = "FAIL"
                detail = ("rejected with WRONG reason: got %s, expected %s"
                          % (got_reason, want))
                mismatches.append((r, got_reason, want))
        except Exception as e:  # any non-typed error is also a conformance failure
            outcome = "FAIL"
            detail = "unexpected %s: %s (expected rejection %s)" % (
                type(e).__name__, e, want)
            mismatches.append((r, "%s" % type(e).__name__, want))

        print("  [%s] %-24s tier=%-4s input=%-20s -> %s"
              % (outcome, r["case"], tier, r["input_hex"], detail))

    print("")
    print("  core: %d/%d REJECTED with matching reason"
          % (core_rejected_ok, core_total))
    print("  nfc : %d/%d REJECTED as non-nfc-text" % (nfc_rejected_ok, nfc_total))

    return core_total, core_rejected_ok, nfc_total, nfc_rejected_ok, mismatches


def run_positive_roundtrip():
    """
    Prove the decoder ACCEPTS canonical input and round-trips it (§6.2):
    reencode(decode(canon(v))) == canon(v) for the ACS-002 positive bodies we can
    rebuild (V1/V2/V3). Returns (ok_count, total, failures).
    """
    cases = [
        ("V1 uci.fact", V.acs002_v1_fact),
        ("V2 engine-manifest", V.acs002_v2_engine_manifest),
        ("V3 nfc+neg", V.acs002_v3_nfc_neg),
    ]
    print("")
    print("  Positive round-trip (decoder ACCEPTS canonical, §6.2 canon(decode(b))==b):")
    ok = 0
    failures = []
    for name, builder in cases:
        body = encode(builder())          # our own canonical encoder
        try:
            value = decode(body)          # MUST accept canonical input
            reencoded = reencode(value)   # MUST reproduce identical bytes
            if reencoded == body:
                ok += 1
                print("    [PASS] %-20s decode+reencode reproduces %d-byte body"
                      % (name, len(body)))
            else:
                failures.append(name)
                print("    [FAIL] %-20s round-trip byte mismatch\n"
                      "           body     =%s\n           reencoded=%s"
                      % (name, body.hex(), reencoded.hex()))
        except Exception as e:
            failures.append(name)
            print("    [FAIL] %-20s decoder REJECTED canonical body: %s: %s"
                  % (name, type(e).__name__, e))
    print("    round-trip: %d/%d PASS" % (ok, len(cases)))
    return ok, len(cases), failures


def run():
    rows = load_negative_rows()
    (core_total, core_ok, nfc_total, nfc_ok, mismatches) = run_rejection_check(rows)
    rt_ok, rt_total, rt_failures = run_positive_roundtrip()

    print("")
    all_core_ok = (core_ok == core_total)
    all_nfc_ok = (nfc_ok == nfc_total)
    all_rt_ok = (rt_ok == rt_total)

    total_rejected = core_ok + nfc_ok
    total_vectors = core_total + nfc_total
    print("  %d/%d REJECTED (%d core + %d nfc)"
          % (total_rejected, total_vectors, core_ok, nfc_ok))

    if mismatches:
        print("  MISMATCHES:")
        for (r, got, want) in mismatches:
            print("    - %s (tier=%s): got %r, expected %r"
                  % (r["case"], r["tier"], got, want))

    conformant = all_core_ok and all_nfc_ok and all_rt_ok
    verdict = "CONFORMANT (rejects non-canonical, accepts canonical)" if conformant \
        else "NON-CONFORMANT"
    print("VERDICT: %s" % verdict)

    return 0 if conformant else 1


if __name__ == "__main__":
    sys.exit(run())

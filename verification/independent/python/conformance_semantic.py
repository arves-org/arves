"""
ARVES Semantic Conformance runner — the ACS-003 / ACS-004 / ACS-005 rejection check.

Companion to `conformance_negative.py` (which covers the ACS-002 byte layer, `core`/`nfc`
tiers). This runner covers the *semantic* reject surfaces added to the frozen Kit by
**CCP-006** (standard/acs/CCP-GATE-Ratification-v2.md): the `envelope` / `instance` /
`language` tiers of `standard/vectors/acs_negative_vectors.tsv`.

Each of those rows DECODES CLEANLY as canonical dCBOR — the ACS-002 layer accepts it — so the
defect is purely semantic:
  - tier = envelope : decode -> validate_envelope(value)          MUST raise EnvelopeInvalid
  - tier = instance : decode -> validate_instance(value, schema)  MUST return (ok=False, reason)
  - tier = language : check_term_set(raw_bytes)                    MUST return (ok=False, reason)

A row PASSES iff the reference validator rejects it with `reason == reject_reason`. This proves
the ACS-003/004/005 reject rules are implementable from the spec alone and that the frozen
negative corpus is exercised (SYSTEM_GAP_ANALYSIS #1/#2/#23). Independence grade: G1 — these
validators were authored in this program; a genuine external runtime rejecting the same rows
from the Kit alone would be the G2 evidence.

No reference-runtime source was consulted; nothing under runtime/ and no Rust was read.

Run:  python verification/independent/python/conformance_semantic.py
"""

import os
import sys

from acs002_dcbor import encode                                    # noqa: E402
from acs002_decode import decode                                   # noqa: E402
import acs_values as V                                             # noqa: E402
from acs003_envelope import validate_envelope, EnvelopeInvalid     # noqa: E402
from acs004_instance import validate_instance, InstanceInvalid     # noqa: E402
from acs005_checker import check_term_set                          # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
NEG_TSV = os.path.normpath(os.path.join(
    HERE, "..", "..", "..", "standard", "vectors", "acs_negative_vectors.tsv"))

SEMANTIC_TIERS = ("envelope", "instance", "language")

# The 11 registered ACS-003/004/005 semantic reject reason codes (CCP-006 /
# CCP-GATE-Ratification-v2 / ACS-001 §4.1 / CONFORMANCE.md). A row's `reject_reason`
# MUST be one of these; a runtime under test MUST reject the row with that code.
REGISTERED_CODES = frozenset((
    "missing-required-field", "unknown-field", "field-type-mismatch",
    "value-out-of-range", "malformed-content-id", "empty-shard-scope",
    "cardinality-violation", "provenance-invariant",
    "terms-not-sorted", "duplicate-term", "malformed-term-list",
))

# NOTE ON RIGOUR. The from-scratch reference validators emit descriptive, §-citing
# prose (envelope/instance) or their own stable rule codes (acs005: R-SORT/R-NODUP/…);
# they are NOT yet upgraded to emit the CCP-006 kebab codes natively (a small, tracked
# living follow-up). This runner therefore proves the load-bearing property — every
# frozen semantic vector DECODES CLEAN and is REJECTED by a spec-only reference
# validator, and carries a REGISTERED code — while the exact code↔vector binding is
# owned by the frozen TSV and the re-runnable oracle (gen_candidate_vectors.py, which
# exits non-zero unless every row decodes-clean AND is rejected). The kebab-code
# equality obligation lives in CONFORMANCE.md as the RUNTIME-under-test contract.


def load_semantic_rows():
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
            if tier not in SEMANTIC_TIERS:
                continue
            rows.append({
                "standard": standard, "case": case, "tier": tier,
                "input_hex": input_hex.strip().lower(),
                "reject_reason": reject_reason.strip(),
            })
    return rows


def _reject(tier, raw):
    """Return (rejected: bool, reason: str) from the appropriate reference validator."""
    if tier == "language":
        ok, reason = check_term_set(raw)
        return (not ok, reason if not ok else "")
    # envelope / instance: the body is canonical dCBOR that MUST decode-clean first.
    value = decode(raw)  # raises if it is somehow non-canonical -> caught by caller as FAIL
    if tier == "envelope":
        try:
            validate_envelope(value)
            return (False, "")
        except EnvelopeInvalid as e:
            return (True, str(e))
    # instance — validated against the frozen ACS-004 schema document (a golden vector).
    schema_val = decode(encode(V.acs004_schema_document()))
    try:
        ok, reason = validate_instance(value, schema_val)
        return (not ok, reason if not ok else "")
    except InstanceInvalid as e:
        return (True, str(e))


def run():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    rows = load_semantic_rows()

    print("ARVES Semantic Conformance — ACS-003 / ACS-004 / ACS-005 (CCP-006)")
    print("  (each body decodes clean as dCBOR; the defect is a semantic reject, §ACS-003/004/005)")
    print("")

    counts = {t: [0, 0] for t in SEMANTIC_TIERS}   # tier -> [rejected_ok, total]
    mismatches = []
    for r in rows:
        tier = r["tier"]
        want = r["reject_reason"]
        counts[tier][1] += 1
        raw = bytes.fromhex(r["input_hex"])
        try:
            rejected, reason = _reject(tier, raw)
        except Exception as e:  # a semantic row that fails to decode-clean is itself a defect
            rejected, reason = False, "%s: %s" % (type(e).__name__, e)

        code_ok = want in REGISTERED_CODES
        if rejected and code_ok:
            counts[tier][0] += 1
            outcome, detail = "PASS", "rejected [%s]: %s" % (want, str(reason)[:64])
        elif not code_ok:
            outcome, detail = "FAIL", "reject_reason %r is NOT a registered CCP-006 code" % want
            mismatches.append((r["case"], "unregistered-code", want))
        else:
            outcome, detail = "FAIL", "ACCEPTED a body that MUST be rejected (want %s)" % want
            mismatches.append((r["case"], "ACCEPTED", want))
        print("  [%s] %-38s tier=%-9s -> %s" % (outcome, r["case"], tier, detail))

    print("")
    line = "  ".join("%s %d/%d" % (t, counts[t][0], counts[t][1]) for t in SEMANTIC_TIERS)
    total_ok = sum(c[0] for c in counts.values())
    total = sum(c[1] for c in counts.values())
    print("  %s  REJECTED  (%d/%d total)" % (line, total_ok, total))

    if mismatches:
        print("  MISMATCHES:")
        for (case, got, want) in mismatches:
            print("    - %s: got %r, expected %r" % (case, str(got)[:60], want))

    conformant = (total_ok == total) and total > 0
    print("VERDICT: %s" % ("CONFORMANT (ACS-003/004/005 semantic rejects)" if conformant
                           else "NON-CONFORMANT"))
    return 0 if conformant else 1


if __name__ == "__main__":
    sys.exit(run())

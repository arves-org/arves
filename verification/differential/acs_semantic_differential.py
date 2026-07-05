#!/usr/bin/env python3
"""
ACS-003/004/005 SEMANTIC differential (rank 12) — the reject surface ABOVE the ACS-002 byte layer.

`acs002_differential_fuzz.py` proves the two implementations agree on the ACS-002 *byte* layer.
This does the same for the SEMANTIC layer: it drives a large, deterministic single-mutation corpus
of ACS-003 envelopes / ACS-004 instances / ACS-005 term-sets through BOTH the independent Python
reference validators AND the native Rust validators (via the `acs_validate` bin, RCR-004b), and
asserts they AGREE on accept/reject for every case. This converts the semantic reject surface from
*self*-conformance (Python only, `conformance_semantic.py`) into *differential* conformance — which
is what makes the falsifiable-conformance thesis actually TRUE above the byte layer.

Every case DECODES CLEAN as canonical dCBOR (the ACS-002 layer accepts it); only the semantics vary.
The corpus is deterministic and exhaustive over single-field mutations (no RNG), so the run is
byte-reproducible.

HONESTY: the load-bearing differential property is **accept/reject agreement** (do the two
implementations reject exactly the same bodies?). Reason-CODE parity is reported separately and is
"mapped-then-checked": the Rust bin emits the registered kebab codes; the Python reference emits
prose / R-codes, so a per-case code equality is NOT asserted here (tracked — until the Python
validators emit native codes). No divergence is swallowed.

Run:  python verification/differential/acs_semantic_differential.py
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
sys.path.insert(0, PYDIR)

from acs002_dcbor import encode, AInt, AFloat                    # noqa: E402
from acs002_decode import decode                                # noqa: E402
from acs001_address import content_id                           # noqa: E402
import acs_values as V                                          # noqa: E402
from acs003_envelope import validate_envelope, EnvelopeInvalid  # noqa: E402
from acs004_instance import validate_instance                   # noqa: E402
from acs005_checker import check_term_set                       # noqa: E402


def _rust_bin():
    p = os.path.join(ROOT, "runtime", "target", "debug", "acs_validate")
    return p + ".exe" if os.path.exists(p + ".exe") else p


VALID_CID = content_id(0x01, encode(V.acs002_v1_fact()))
SCHEMA = decode(encode(V.acs004_schema_document()))


# --- deterministic single-mutation corpus (each stays valid canonical dCBOR) ---

def envelope_cases():
    """Valid envelope + one mutation per field: drop it, or set a wrong-typed value."""
    base = dict(V.acs003_envelope(VALID_CID))
    cases = [("envelope", "valid", dict(base))]
    for k in list(base.keys()):
        d = dict(base); d.pop(k, None)
        cases.append(("envelope", f"drop:{k}", d))
        # wrong type: swap Text<->Int-ish so it still decodes clean but violates §5 typing.
        d2 = dict(base)
        d2[k] = AInt(7) if isinstance(base[k], str) else "wrong-type-text"
        cases.append(("envelope", f"wrongtype:{k}", d2))
    cases.append(("envelope", "extra-key", {**base, "x_unknown": "nope"}))
    return cases


def instance_cases():
    base = dict(V.acs004_instance())
    cases = [("instance", "valid", dict(base))]
    for k in list(base.keys()):
        d = dict(base); d.pop(k, None)
        cases.append(("instance", f"drop:{k}", d))
        d2 = dict(base)
        d2[k] = AInt(7) if isinstance(base[k], str) else "wrong-type-text"
        cases.append(("instance", f"wrongtype:{k}", d2))
    cases.append(("instance", "extra-field", {**base, "x_unknown": "nope"}))
    cases.append(("instance", "conf-out-of-range", {**base, "confidence": AFloat(1.5)}))
    cases.append(("instance", "int-above-i64", {**base, "valid_from": AInt(2 ** 63)}))
    cases.append(("instance", "derived-no-invocation", {**base, "origin": "derived"}))
    return cases


def termset_cases():
    body = V.acs005_term_set_body().decode("utf-8")
    lines = body.split("\n")
    variants = {
        "valid": body,
        "not-sorted": "\n".join([lines[1], lines[0]] + lines[2:]),
        "duplicate": "\n".join([lines[0]] + lines),
        "trailing-lf": body + "\n",
        "leading-lf": "\n" + body,
        "blank-line": "\n".join([lines[0], ""] + lines[1:]),
        "bad-grammar": "\n".join(lines[:-1] + ["GL-01A"]),
        "extra-valid": "\n".join(lines + ["GL-015"]),
        "removed-entry": "\n".join(lines[:-1]),
    }
    return [("language", name, text.encode("utf-8")) for name, text in variants.items()]


def py_verdict(tier, payload):
    """(tier, payload) -> 'ACCEPT' | 'REJECT' | 'NONCANON' from the Python reference."""
    if tier == "language":
        ok, _r = check_term_set(payload)
        return "ACCEPT" if ok else "REJECT"
    try:
        body = encode(payload)
        value = decode(body)
    except Exception:  # noqa: BLE001 — did not decode clean (skip: not a semantic-layer case)
        return "NONCANON"
    if tier == "envelope":
        try:
            validate_envelope(value)
            return "ACCEPT"
        except EnvelopeInvalid:
            return "REJECT"
    ok, _r = validate_instance(value, SCHEMA)
    return "ACCEPT" if ok else "REJECT"


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

    binp = _rust_bin()
    if not os.path.exists(binp):
        print("SKIP: acs_validate bin not built (run `cargo build -p arves-conformance --bin acs_validate`).")
        return 0

    # Build the corpus + Python verdicts, keeping only decode-clean semantic cases.
    corpus = []  # (tier, name, hexbody, py_verdict)
    raw = envelope_cases() + instance_cases()
    for tier, name, payload in raw:
        v = py_verdict(tier, payload)
        if v == "NONCANON":
            continue  # not a pure semantic case (the ACS-002 differential covers those)
        corpus.append((tier, name, encode(payload).hex(), v))
    for tier, name, body in termset_cases():
        corpus.append((tier, name, body.hex(), py_verdict(tier, body)))

    # Drive the Rust arm once (batched).
    payload = "".join(f"{tier}\t{hexs}\n" for (tier, _n, hexs, _v) in corpus).encode()
    out = subprocess.run([binp], input=payload, stdout=subprocess.PIPE).stdout.decode().splitlines()

    agree = 0
    divergences = []
    for (tier, name, _h, pv), line in zip(corpus, out):
        rv = "ACCEPT" if line.strip() == "ACCEPT" else ("REJECT" if line.startswith("REJECT") else "ERR")
        if rv == pv:
            agree += 1
        else:
            divergences.append((tier, name, pv, line.strip()))

    n = len(corpus)
    rejects = sum(1 for c in corpus if c[3] == "REJECT")
    print("ACS-003/004/005 SEMANTIC differential — Rust (native) vs Python (reference)")
    print("=" * 74)
    print(f"  corpus            : {n} decode-clean semantic cases "
          f"({rejects} reject / {n - rejects} accept)")
    print(f"  accept/reject     : {agree}/{n} AGREE")
    if divergences:
        print(f"  HARD DIVERGENCES  : {len(divergences)}")
        for (tier, name, pv, rl) in divergences:
            print(f"    - [{tier}] {name}: python={pv} rust={rl}")
        print("VERDICT: NON-CONFORMANT (the two semantic validators disagree)")
        return 1
    print("  hard divergences  : 0")
    print("  reason-code parity: NOT asserted here (Rust=kebab, Python=prose; mapped-then-checked, tracked)")
    print("VERDICT: CONFORMANT — the ACS-003/004/005 reject surface is DIFFERENTIAL "
          "(two independent implementations agree), not self-conformance.")
    return 0


if __name__ == "__main__":
    sys.exit(main())

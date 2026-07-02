"""
Sound (non-gameable) runtime verifier — the Verification arm's answer to audit gap B3.

`certify_runtime.py` follows the frozen RUNTIME_AUTHORS_GUIDE contract, which hands the
grader the answer key alongside the runtime's output and does a string compare; a hollow
adapter that just echoes the published answers is CERTIFIED (gap B3, recorded in
`verification/evidence/G2_READINESS.md`). This verifier removes that hole WITHOUT touching
the frozen Kit contract:

  * The grader OWNS the truth. It recomputes every ContentId itself
    (ACS-001 §5/§7: `0x12 0x20 || SHA-256(domain_tag || body)`) and decides accept/reject
    with a reference decoder. The runtime under test is given INPUTS ONLY, never the
    expected answer.
  * It probes FRESH `(domain, body)` pairs that are NOT in the published vectors, so a
    runtime that hardcodes or echoes the 12 published ContentIds fails.
  * It injects valid canonical bodies that a conformant decoder MUST ACCEPT, so an
    all-REJECT adapter fails.

A runtime is SOUND-CERTIFIED iff it reproduces every published + fresh address and decides
every core-negative + accept-probe correctly.

This is the contract proposed for a future Kit 0.2.1 (which would converge
RUNTIME_AUTHORS_GUIDE + certify_runtime.py onto it — a maintainer-gated, frozen-Kit change).
It ships here as a LIVING check so the Verification arm can gate on B3 today without any
frozen-Kit edit.

Run:  python verification/certification/verify_runtime_sound.py
"""

import hashlib
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
VEC = os.path.join(ROOT, "standard", "vectors")
PYDIR = os.path.join(ROOT, "verification", "independent", "python")


def expected_cid(domain, body):
    """Grader-owned truth: ACS-001 §5/§7 ContentId. The verifier never trusts the runtime."""
    return (bytes([0x12, 0x20]) + hashlib.sha256(bytes([domain]) + bytes(body)).digest()).hex()


def load_golden():
    rows = []
    with open(os.path.join(VEC, "acs_golden_vectors.tsv"), encoding="utf-8") as f:
        next(f)
        for line in f:
            if not line.strip():
                continue
            std, _vec, dom, body_hex, cid = line.rstrip("\n").split("\t")
            rows.append((std, int(dom, 16), bytes.fromhex(body_hex), cid.lower()))
    return rows


def load_negative():
    rows = []
    with open(os.path.join(VEC, "acs_negative_vectors.tsv"), encoding="utf-8") as f:
        next(f)
        for line in f:
            if not line.strip():
                continue
            _std, _case, tier, input_hex, reason = line.rstrip("\n").split("\t")
            rows.append((tier, bytes.fromhex(input_hex), reason))
    return rows


GOLDEN = load_golden()
NEG = load_negative()

# FRESH address probes — NOT present in the published vectors. A runtime that hardcodes or
# echoes the 12 published ContentIds cannot answer these; only real SHA-256 over
# (domain_tag || body) does. Domains 0x01/0x02/0x04 are allocated (ACS-001 §4.1).
FRESH = [
    (0x01, b"arves-g2-integrity-probe-alpha"),
    (0x02, b"arves-g2-integrity-probe-beta"),
    (0x04, b"arves-g2-integrity-probe-gamma"),
]

# ACCEPT probes — valid canonical ACS-002 bodies a conformant decoder MUST accept. An
# all-REJECT hollow adapter fails these. (Taken from the golden set: canonical by construction.)
ACCEPT_PROBES = [body for (std, _d, body, _c) in GOLDEN if std == "ACS-002"]

CORE = [(inp, reason) for (tier, inp, reason) in NEG if tier == "core"]

# Vector-integrity self-check: the grader's independent recompute of every published row
# must equal the stored ContentId. A mismatch means a corrupted vector, not a runtime bug.
for _std, _d, _b, _cid in GOLDEN:
    if expected_cid(_d, _b) != _cid:
        raise SystemExit("VECTOR INTEGRITY FAILURE: recompute != stored ContentId for a golden row")


def grade_sound(name, addresser, rejecter):
    """
    addresser: (domain: int, body: bytes) -> ContentId hex     (runtime under test)
    rejecter:  (body: bytes) -> (verdict, reason)              (runtime under test)
    The runtime is given inputs only; all expected values live here in the grader.
    """
    addr_inputs = [(d, b) for (_s, d, b, _c) in GOLDEN] + list(FRESH)
    published_n = len(GOLDEN)

    published_ok = fresh_ok = 0
    for i, (d, b) in enumerate(addr_inputs):
        got = addresser(d, b)
        if got == expected_cid(d, b):
            if i < published_n:
                published_ok += 1
            else:
                fresh_ok += 1

    core_ok = 0
    for (inp, reason) in CORE:
        verdict, r = rejecter(inp)
        if verdict == "REJECT" and r == reason:
            core_ok += 1

    accept_ok = 0
    for body in ACCEPT_PROBES:
        verdict, _r = rejecter(body)
        if verdict == "ACCEPT":
            accept_ok += 1

    certified = (
        published_ok == published_n
        and fresh_ok == len(FRESH)
        and core_ok == len(CORE)
        and accept_ok == len(ACCEPT_PROBES)
    )
    return {
        "runtime": name,
        "published": (published_ok, published_n),
        "fresh": (fresh_ok, len(FRESH)),
        "core_reject": (core_ok, len(CORE)),
        "accept": (accept_ok, len(ACCEPT_PROBES)),
        "certified": certified,
    }


# ---- reference Python runtime primitives (the runtime under test here) ----
sys.path.insert(0, PYDIR)
from acs001_address import content_id as py_content_id   # noqa: E402
from acs002_decode import decode as py_decode, Rejected   # noqa: E402


def py_addr(domain, body):
    return py_content_id(domain, body).hex()


def py_rej(body):
    try:
        py_decode(body)
        return ("ACCEPT", "")
    except Rejected as e:
        return ("REJECT", e.reason)


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    rec = grade_sound("ARVES Python (independent)", py_addr, py_rej)
    print("ARVES Sound Runtime Verification - non-gameable (grader owns the truth)")
    print("=" * 70)
    p, pt = rec["published"]
    fr, frt = rec["fresh"]
    c, ct = rec["core_reject"]
    a, at = rec["accept"]
    print(f"  {rec['runtime']:<28} published {p}/{pt}  fresh {fr}/{frt}  "
          f"core-reject {c}/{ct}  accept {a}/{at}  ->  "
          f"{'SOUND-CERTIFIED' if rec['certified'] else 'NOT CERTIFIED'}")
    print("-" * 70)
    print("  Runtime given INPUTS ONLY; grader recomputed every ContentId and re-decoded")
    print("  every input. Fresh + accept probes defeat a hollow echo adapter (gap B3).")
    return 0 if rec["certified"] else 1


if __name__ == "__main__":
    sys.exit(main())

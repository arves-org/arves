"""
Regression test for the certification-harness audit gaps (verification/evidence/G2_READINESS.md):

  B3 — a gameable grader would CERTIFY a hollow adapter that only echoes published answers.
  B4 — certify_runtime.py must not crash on a Kit-only checkout (missing reference bins).

This test is the executable proof that B3 is caught by the sound verifier and B4 degrades
gracefully. Run:  python verification/certification/test_harness_integrity.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import verify_runtime_sound as S   # noqa: E402
import certify_runtime as C        # noqa: E402

failures = []


# ---- B3: the sound verifier must PASS the real runtime and REJECT a hollow echo adapter ----

real = S.grade_sound("real-python", S.py_addr, S.py_rej)
if not real["certified"]:
    failures.append(f"real Python runtime should SOUND-CERTIFY, got {real}")

# A maximally-informed hollow adapter: it read the published vectors and hardcodes every
# published answer (ContentIds + core reject reasons), but it does NO real work — so it
# cannot address the grader's FRESH inputs and cannot ACCEPT a valid body.
_published_cid = {(d, bytes(b)): cid for (_s, d, b, cid) in S.GOLDEN}
_published_reason = {bytes(inp): reason for (inp, reason) in S.CORE}


def hollow_addr(domain, body):
    # echoes the published ContentId when it recognizes the input; blind to FRESH probes.
    return _published_cid.get((domain, bytes(body)), "12" + "00" * 33)


def hollow_rej(body):
    # echoes the correct published core reason if it recognizes the input; otherwise the
    # classic exploit is to just claim REJECT — which is exactly wrong for accept-probes.
    if bytes(body) in _published_reason:
        return ("REJECT", _published_reason[bytes(body)])
    return ("REJECT", "non-shortest-int")


hollow = S.grade_sound("hollow-echo", hollow_addr, hollow_rej)
if hollow["certified"]:
    failures.append(f"SECURITY REGRESSION (B3): hollow echo adapter was SOUND-CERTIFIED: {hollow}")
# It must specifically miss the anti-gaming gates:
if hollow["fresh"][0] != 0:
    failures.append(f"(B3) hollow should reproduce 0 fresh addresses, got {hollow['fresh']}")
if hollow["accept"][0] != 0:
    failures.append(f"(B3) hollow should ACCEPT 0 valid probes, got {hollow['accept']}")
# Sanity: it CAN still echo the published surface (so the test is exercising the right hole).
if hollow["published"][0] != hollow["published"][1]:
    failures.append(f"(B3) hollow was expected to echo the published surface, got {hollow['published']}")


# ---- B4: certify_runtime.py must degrade (not crash) when a reference binary is absent ----

_saved = C.RUST_BRIDGE
try:
    C.RUST_BRIDGE = os.path.join(C.ROOT, "runtime", "target", "debug", "does-not-exist")
    rc = C.main()  # must return, not raise FileNotFoundError
    if rc not in (0, 1):
        failures.append(f"(B4) certify_runtime.main() returned unexpected code {rc}")
except FileNotFoundError as e:
    failures.append(f"(B4) certify_runtime crashed on a Kit-only checkout instead of degrading: {e}")
finally:
    C.RUST_BRIDGE = _saved


if failures:
    print("HARNESS-INTEGRITY: FAIL")
    for f in failures:
        print("  - " + f)
    sys.exit(1)
print("HARNESS-INTEGRITY OK: real=SOUND-CERTIFIED, hollow=REJECTED (B3), "
      "missing-runtime=degraded-not-crashed (B4)")

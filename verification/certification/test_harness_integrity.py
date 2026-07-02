"""
Regression test for the certification-harness audit gaps (verification/evidence/G2_READINESS.md):

  B3 — a gameable grader would CERTIFY a hollow adapter that only echoes published answers.
  B4 — certify_runtime.py must not crash on a Kit-only checkout (missing reference bins).

It also covers the sound verifier's Rust arm (both reference runtimes graded inputs-only by
ONE grader):

  B3-rust — a BYTE-BROKEN / STALE Rust-style adapter (returns wrong ContentIds) must be
            NOT SOUND-CERTIFIED under grade_sound(), exactly like a hollow Python echo.
  B4-rust — a MISSING reference bin must degrade to RustUnavailable (a SKIPPED row), never
            crash the verifier with FileNotFoundError.

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


# ---- B3-rust: a BYTE-BROKEN / STALE Rust-style adapter must be NOT SOUND-CERTIFIED ----
#
# Models a Rust bridge that drifted from the frozen bytes (e.g. wrong domain-tag framing or a
# stale build): it hands back a plausible-looking but WRONG ContentId for every address and
# mislabels decode verdicts. The grader owns the truth, so every answer mismatches and the
# runtime is rejected — the same guarantee the real Rust arm relies on to stay honest.

def broken_rust_addr(domain, body):
    # Deterministic, well-formed 34-byte multihash prefix, but NOT the SHA-256 of the input:
    # a byte-broken/stale adapter that never reproduces a real ContentId.
    return "12" + "20" + "ba" * 32


def broken_rust_rej(body):
    # Stale decoder: claims ACCEPT for genuine core-negatives (wrong) and REJECT-with-bogus-
    # reason for valid accept-probes (wrong) — inverts both gates.
    return ("ACCEPT", "") if bytes(body) in _published_reason else ("REJECT", "stale-reason")


broken_rust = S.grade_sound("broken-rust", broken_rust_addr, broken_rust_rej)
if broken_rust["certified"]:
    failures.append(f"SECURITY REGRESSION (B3-rust): byte-broken/stale Rust adapter was "
                    f"SOUND-CERTIFIED: {broken_rust}")
# It must miss every anti-gaming gate — nothing about a stale runtime can match.
if broken_rust["published"][0] != 0:
    failures.append(f"(B3-rust) stale adapter reproduced a published address it shouldn't: "
                    f"{broken_rust['published']}")
if broken_rust["fresh"][0] != 0:
    failures.append(f"(B3-rust) stale adapter reproduced a fresh address: {broken_rust['fresh']}")
if broken_rust["accept"][0] != 0:
    failures.append(f"(B3-rust) stale adapter ACCEPTed a valid probe: {broken_rust['accept']}")


# ---- B4-rust: a MISSING reference bin must degrade to RustUnavailable, not crash ----
#
# Simulate a Kit-only checkout by pointing the bridge bin at a path that does not exist. The
# adapter builder MUST raise RustUnavailable (which main() turns into a SKIPPED row), never a
# bare FileNotFoundError that would abort the whole verifier.
_saved_bridge = S.RUST_BRIDGE
try:
    S.RUST_BRIDGE = os.path.join(S.ROOT, "runtime", "target", "debug", "does-not-exist")
    try:
        S.rust_build_adapters()
        failures.append("(B4-rust) rust_build_adapters() did not degrade when the bridge bin "
                        "is absent; it must raise RustUnavailable")
    except S.RustUnavailable:
        pass  # correct: degraded, not crashed
    except FileNotFoundError as e:
        failures.append(f"(B4-rust) rust_build_adapters() crashed with FileNotFoundError "
                        f"instead of RustUnavailable: {e}")
    # And main() as a whole must still return a normal exit code (SKIPPED row, run stays green
    # on the still-available Python arm).
    rc = S.main()
    if rc not in (0, 1):
        failures.append(f"(B4-rust) verify_runtime_sound.main() returned unexpected code {rc} "
                        f"with a missing Rust bin")
finally:
    S.RUST_BRIDGE = _saved_bridge


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

# B4 (Python arm): a Kit-only checkout without the in-repo Python reference must degrade to an
# UNAVAILABLE row, not crash. Simulate the post-import "reference absent" state.
_saved_py = C.PY_AVAILABLE
try:
    C.PY_AVAILABLE = False
    rc = C.main()  # must return, not raise
    if rc not in (0, 1):
        failures.append(f"(B4-py) certify_runtime.main() returned unexpected code {rc}")
except Exception as e:  # noqa: BLE001 — any crash here is the regression
    failures.append(f"(B4-py) certify_runtime crashed when the Python reference is absent: {e}")
finally:
    C.PY_AVAILABLE = _saved_py


if failures:
    print("HARNESS-INTEGRITY: FAIL")
    for f in failures:
        print("  - " + f)
    sys.exit(1)
print("HARNESS-INTEGRITY OK: real=SOUND-CERTIFIED, hollow=REJECTED (B3), "
      "byte-broken-rust=REJECTED (B3-rust), missing-runtime=degraded-not-crashed (B4/B4-rust)")

"""
ARVES ACS-002 differential fuzzer — Rust reference vs. independent Python.

Verification Office harness (NOT part of the independent authoring — it drives both
finished decoders). It generates a large, deterministic corpus of candidate bodies
(canonical values, byte-level mutations of them, and random byte strings), feeds the
SAME bytes to:
  - the Rust reference decoder via `runtime/target/debug/acs_decode` (line protocol:
    ACCEPT<TAB>reencoded_hex | REJECT<TAB>reason | ERR<TAB>bad-hex), and
  - the independent Python decoder `acs002_decode.decode` (Kit-only authorship),
and asserts they AGREE on the interop-critical property: ACCEPT vs REJECT, and — on
ACCEPT — identical canonical re-encoding.

The single documented asymmetry is the ACS-002 nfc-tier deferral: the dependency-free
Rust reference ACCEPTS non-NFC text while the Python (stdlib unicodedata) REJECTS it
as `non-nfc-text`. That specific pair is classified NFC-DEFERRAL, not a divergence.
Any other accept/reject disagreement, or a reencode-byte disagreement, is a hard
DIVERGENCE and fails the run.

Deterministic: fixed RNG seed, so the corpus and verdict are reproducible.
Run: python verification/differential/acs002_differential_fuzz.py
"""

import os
import subprocess
import sys
import random

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
RUST_BIN = os.path.join(ROOT, "runtime", "target", "debug", "acs_decode.exe")
if not os.path.exists(RUST_BIN):
    RUST_BIN = os.path.join(ROOT, "runtime", "target", "debug", "acs_decode")

sys.path.insert(0, PYDIR)
from acs002_dcbor import AInt, AFloat, encode          # noqa: E402  (our own encoder)
from acs002_decode import decode, reencode, Rejected   # noqa: E402  (Kit-only decoder)

SEED = 20260702
N_CANONICAL = 4000
N_RANDOM = 4000


# --- corpus generation --------------------------------------------------------

_INT_POOL = [
    0, 1, 23, 24, 255, 256, 65535, 65536, 2**32 - 1, 2**32, 2**53,
    2**63 - 1, 2**63, 2**64 - 1, -1, -24, -256, -1000,
    -(2**63), -(2**63) - 1, -(2**64),
]
_FLOAT_POOL = [0.0, 1.0, -1.25, 0.5, 3.14159, 1e300, -1e-300, 2.0, 123456.789, -0.5]
# All NFC (no combining marks) so canonical generation never triggers the nfc tier.
_TEXT_POOL = ["", "a", "b", "type", "claim", "n", "x", "zz", "hello-truth",
              "uci.fact", "é", "中", "αβ"]


def gen_int(rng):
    if rng.random() < 0.5:
        return AInt(rng.choice(_INT_POOL))
    return AInt(rng.randint(-(2**64), 2**64 - 1))


def gen_value(rng, depth):
    kinds = ["null", "bool", "int", "float", "text", "bytes"]
    if depth < 3:
        kinds += ["array", "map"]
    t = rng.choice(kinds)
    if t == "null":
        return None
    if t == "bool":
        return rng.choice([True, False])
    if t == "int":
        return gen_int(rng)
    if t == "float":
        return AFloat(rng.choice(_FLOAT_POOL))
    if t == "text":
        return rng.choice(_TEXT_POOL)
    if t == "bytes":
        return bytes(rng.randint(0, 255) for _ in range(rng.randint(0, 6)))
    if t == "array":
        return [gen_value(rng, depth + 1) for _ in range(rng.randint(0, 4))]
    # map: unique Text/Integer keys
    m = {}
    seen = set()
    for _ in range(rng.randint(0, 4)):
        if rng.random() < 0.7:
            k = rng.choice(["a", "b", "c", "type", "claim", "n", "x", "zz", "é"])
            kk = ("T", k)
        else:
            k = AInt(rng.randint(-5, 30))
            kk = ("I", k.v)
        if kk in seen:
            continue
        seen.add(kk)
        m[k] = gen_value(rng, depth + 1)
    return m


def mutate(body, rng):
    out = []
    if not body:
        return out
    b = bytearray(body)
    out.append(bytes(b[:-1]))                                   # truncate last byte
    out.append(bytes(b) + bytes([rng.randint(0, 255)]))         # append trailing byte
    c = bytearray(b)                                            # flip one bit
    i = rng.randrange(len(c))
    c[i] ^= (1 << rng.randint(0, 7))
    out.append(bytes(c))
    ib = b[0]                                                   # widen head -> non-shortest
    major, ai = ib >> 5, ib & 0x1F
    if ai < 24 and major <= 5:
        out.append(bytes([(major << 5) | 24, ai]) + bytes(b[1:]))
    return out


def build_corpus():
    rng = random.Random(SEED)
    raw = []
    canon = []
    for _ in range(N_CANONICAL):
        try:
            body = encode(gen_value(rng, 0))
        except Exception:
            continue
        canon.append(body)
        raw.append(body)
    for body in canon:
        raw.extend(mutate(body, rng))
    for _ in range(N_RANDOM):
        raw.append(bytes(rng.randint(0, 255) for _ in range(rng.randint(1, 24))))
    seen = set()
    corpus = []
    for b in raw:
        if not b:                      # empty input has no line-protocol form; both
            continue                   # decoders trivially reject it as truncated
        h = b.hex()
        if h not in seen:
            seen.add(h)
            corpus.append(b)
    return corpus, len(canon)


# --- run both decoders --------------------------------------------------------

def rust_verdicts(corpus):
    payload = "\n".join(b.hex() for b in corpus) + "\n"
    p = subprocess.run([RUST_BIN], input=payload.encode("ascii"),
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    lines = p.stdout.decode("ascii").splitlines()
    if len(lines) != len(corpus):
        sys.exit("Rust harness returned %d lines for %d inputs" % (len(lines), len(corpus)))
    out = []
    for ln in lines:
        kind, _, rest = ln.partition("\t")
        out.append((kind, rest))
    return out


def py_verdict(b):
    try:
        v = decode(b)
        return ("ACCEPT", reencode(v).hex())
    except Rejected as e:
        return ("REJECT", e.reason)
    except Exception as e:  # a real parser bug would surface here
        return ("PYERR", "%s: %s" % (type(e).__name__, e))


def main():
    corpus, n_canon = build_corpus()
    rust = rust_verdicts(corpus)

    aa = rr = 0
    reason_match = reason_mismatch = 0
    reason_diff_samples = []
    nfc_deferral = 0
    divergences = []
    pyerrs = []

    for b, (rk, rv) in zip(corpus, rust):
        pk, pv = py_verdict(b)
        if pk == "PYERR":
            pyerrs.append((b.hex(), pv))
            divergences.append((b.hex(), "PYERR", "rust=%s/%s py=%s" % (rk, rv, pv)))
            continue
        if rk == "ACCEPT" and pk == "ACCEPT":
            aa += 1
            if rv != pv:
                divergences.append((b.hex(), "REENCODE", "rust=%s py=%s" % (rv, pv)))
        elif rk == "REJECT" and pk == "REJECT":
            rr += 1
            if rv == pv:
                reason_match += 1
            else:
                reason_mismatch += 1
                if len(reason_diff_samples) < 15:
                    reason_diff_samples.append((b.hex(), rv, pv))
        elif rk == "ACCEPT" and pk == "REJECT" and pv == "non-nfc-text":
            nfc_deferral += 1                       # documented ACS-002 nfc-tier deferral
        else:
            divergences.append((b.hex(), "ACCEPT/REJECT", "rust=%s/%s py=%s/%s" % (rk, rv, pk, pv)))

    total = len(corpus)
    print("ARVES ACS-002 Differential Fuzz — Rust reference vs independent Python")
    print("  seed=%d  inputs=%d (canonical bodies=%d, +mutations, +random)" % (SEED, total, n_canon))
    print("  ACCEPT/ACCEPT (reencode identical): %d" % aa)
    print("  REJECT/REJECT                     : %d  (reason match %d, reason differ %d)"
          % (rr, reason_match, reason_mismatch))
    print("  nfc-tier deferral (Rust ACCEPT / Py REJECT non-nfc-text): %d" % nfc_deferral)
    print("  hard divergences                  : %d" % len(divergences))
    if reason_diff_samples:
        print("  --- reason-differ samples (both REJECT; interop-safe) ---")
        for h, rv, pv in reason_diff_samples:
            print("   %-28s rust=%-22s py=%s" % (h, rv, pv))
    if divergences:
        print("  --- first divergences ---")
        for h, kind, detail in divergences[:20]:
            print("   [%s] %s  %s" % (kind, h, detail))
    ok = len(divergences) == 0
    print("VERDICT: %s" % ("DIFFERENTIAL PASS (no accept/reject or reencode disagreement)"
                           if ok else "DIFFERENTIAL FAIL"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

"""
ARVES ACS-002 differential fuzzer — 3-way: Rust reference vs independent Python vs independent TypeScript.

Verification Office harness (NOT part of the independent authoring — it drives three
finished decoders). It generates a large, deterministic corpus of candidate bodies
(canonical values, byte-level mutations of them, and random byte strings), feeds the
SAME bytes to all three, and asserts they AGREE on the interop-critical property:
ACCEPT vs REJECT, and — when they all ACCEPT — identical canonical re-encoding.
  - Rust reference   — `runtime/target/debug/acs_decode` (line protocol:
    ACCEPT<TAB>reencoded_hex | REJECT<TAB>reason | ERR<TAB>bad-hex),
  - independent Python — `acs002_decode.decode` (Kit-only authorship, in-process),
  - independent TypeScript — `typescript/src/decode_lines.mjs` (Kit-only authorship, same
    line protocol as the Rust bin; core mode / nfc DEFERRED).

The single documented asymmetry is the ACS-002 nfc-tier deferral: the dependency-free Rust
reference AND the TypeScript codec (core mode) ACCEPT non-NFC text, while the Python (stdlib
`unicodedata`) REJECTS it as `non-nfc-text`. A mixed verdict whose every REJECT is exactly
`non-nfc-text` is classified NFC-DEFERRAL, not a divergence. Any OTHER accept/reject
disagreement, or a reencode-byte disagreement among the accepters, is a hard DIVERGENCE and
fails the run. (Reason codes on an all-REJECT input are reported but not required to match —
multi-defect inputs can reject for different, equally-correct reasons.)

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
TS_DRIVER = os.path.join(ROOT, "verification", "independent", "typescript", "src", "decode_lines.mjs")

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


def ts_verdicts(corpus):
    payload = "\n".join(b.hex() for b in corpus) + "\n"
    p = subprocess.run(["node", TS_DRIVER], input=payload.encode("ascii"),
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    lines = p.stdout.decode("ascii").splitlines()
    if len(lines) != len(corpus):
        sys.exit("TS harness returned %d lines for %d inputs (stderr: %s)"
                 % (len(lines), len(corpus), p.stderr.decode("utf-8", "replace")[:300]))
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
    ts = ts_verdicts(corpus)

    reencode_ok = all_reject = 0
    reason_match = reason_mismatch = 0
    reason_diff_samples = []
    nfc_deferral = 0
    divergences = []

    for b, (rk, rv), (tk, tv) in zip(corpus, rust, ts):
        pk, pv = py_verdict(b)
        arms = {"rust": (rk, rv), "py": (pk, pv), "ts": (tk, tv)}
        # An arm that neither cleanly ACCEPTed nor REJECTed (PYERR / ERR / crash) is a hard finding.
        errs = [(n, k, v) for n, (k, v) in arms.items() if k not in ("ACCEPT", "REJECT")]
        if errs:
            divergences.append((b.hex(), "ARM-ERROR", "; ".join("%s=%s/%s" % e for e in errs)))
            continue
        accepts = {n: v for n, (k, v) in arms.items() if k == "ACCEPT"}
        rejects = {n: v for n, (k, v) in arms.items() if k == "REJECT"}
        if not rejects:                                   # all three ACCEPT
            if len(set(accepts.values())) == 1:
                reencode_ok += 1
            else:
                divergences.append((b.hex(), "REENCODE", " ".join("%s=%s" % kv for kv in accepts.items())))
        elif not accepts:                                 # all three REJECT
            all_reject += 1
            if len(set(rejects.values())) == 1:
                reason_match += 1
            else:
                reason_mismatch += 1
                if len(reason_diff_samples) < 15:
                    reason_diff_samples.append((b.hex(), dict(rejects)))
        else:                                             # mixed accept/reject
            if all(r == "non-nfc-text" for r in rejects.values()):
                nfc_deferral += 1                         # documented ACS-002 nfc-tier deferral
                if len(set(accepts.values())) != 1:       # the accepters must still agree on bytes
                    divergences.append((b.hex(), "REENCODE(nfc)", " ".join("%s=%s" % kv for kv in accepts.items())))
            else:
                divergences.append((b.hex(), "ACCEPT/REJECT",
                                    " ".join("%s=%s/%s" % (n, k, v) for n, (k, v) in arms.items())))

    total = len(corpus)
    print("ARVES ACS-002 Differential Fuzz — 3-way: Rust reference vs independent Python vs independent TypeScript")
    print("  seed=%d  inputs=%d (canonical bodies=%d, +mutations, +random)" % (SEED, total, n_canon))
    print("  ACCEPT (all 3, reencode identical): %d" % reencode_ok)
    print("  REJECT (all 3)                    : %d  (reason match %d, reason differ %d)"
          % (all_reject, reason_match, reason_mismatch))
    print("  nfc-tier deferral (Rust/TS ACCEPT / Py REJECT non-nfc-text): %d" % nfc_deferral)
    print("  hard divergences                  : %d" % len(divergences))
    if reason_diff_samples:
        print("  --- reason-differ samples (all REJECT; interop-safe, reasons not required equal) ---")
        for h, rd in reason_diff_samples:
            print("   %-28s %s" % (h, " ".join("%s=%s" % kv for kv in rd.items())))
    if divergences:
        print("  --- first divergences ---")
        for h, kind, detail in divergences[:20]:
            print("   [%s] %s  %s" % (kind, h, detail))
    ok = len(divergences) == 0
    print("VERDICT: %s" % ("3-WAY DIFFERENTIAL PASS (no accept/reject or reencode disagreement across Rust/Python/TypeScript)"
                           if ok else "3-WAY DIFFERENTIAL FAIL"))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

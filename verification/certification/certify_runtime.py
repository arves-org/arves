"""
ARVES Runtime Certification Harness — the Certification Authority's tool.

It certifies ANY runtime against the frozen Standard alone (standard/vectors/*.tsv) — no
reference source, no maintainer judgement, no hidden knowledge. A runtime is CERTIFIED iff
it (a) reproduces every golden ContentId from its (domain, body) and (b) rejects every core
negative vector with the matching reason. This is what makes ARVES maintainer-independent:
the STANDARD + THIS HARNESS are the authority, so anyone can certify a runtime — even if the
original team disappears (the Foundation survivability property).

Run: python verification/certification/certify_runtime.py
Certifies the Rust reference runtime and the independent Python runtime under ONE
conformance (the Independent Runtime Alliance).
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
VEC = os.path.join(ROOT, "standard", "vectors")
PYDIR = os.path.join(ROOT, "verification", "independent", "python")


def exe(name):
    p = os.path.join(ROOT, "runtime", "target", "debug", name)
    return p + ".exe" if os.path.exists(p + ".exe") else p


RUST_BRIDGE = exe("arves-bridge")   # address: "<domhex> <bodyhex>" -> "<contentId> ..."
RUST_DECODE = exe("acs_decode")     # reject:  "<bodyhex>" -> "ACCEPT.." | "REJECT<TAB>reason"


def load_golden():
    rows = []
    with open(os.path.join(VEC, "acs_golden_vectors.tsv"), encoding="utf-8") as f:
        next(f)
        for line in f:
            if not line.strip():
                continue
            _std, _vec, dom, body_hex, cid = line.rstrip("\n").split("\t")
            rows.append((int(dom, 16), body_hex.lower(), cid.lower()))
    return rows


def load_negative():
    rows = []
    with open(os.path.join(VEC, "acs_negative_vectors.tsv"), encoding="utf-8") as f:
        next(f)
        for line in f:
            if not line.strip():
                continue
            _std, _case, tier, input_hex, reason = line.rstrip("\n").split("\t")
            rows.append((tier, input_hex.lower(), reason))
    return rows


# ---- Rust reference runtime adapter (drives the shipped bins) ----

def rust_addresses(golden):
    payload = "".join(f"{dom:02x} {body}\n" for dom, body, _ in golden)
    out = subprocess.run([RUST_BRIDGE], input=payload.encode(), stdout=subprocess.PIPE).stdout.decode().splitlines()
    return [ln.split()[0] for ln in out]


def rust_rejects(neg):
    payload = "".join(f"{inp}\n" for _, inp, _ in neg)
    out = subprocess.run([RUST_DECODE], input=payload.encode(), stdout=subprocess.PIPE).stdout.decode().splitlines()
    res = []
    for ln in out:
        parts = ln.replace("\t", " ").split()
        res.append((parts[0], parts[1] if len(parts) > 1 else ""))
    return res


# ---- Independent Python runtime adapter (imported) ----

sys.path.insert(0, PYDIR)
from acs001_address import content_id as py_content_id   # noqa: E402
from acs002_decode import decode as py_decode, Rejected  # noqa: E402


def py_addresses(golden):
    return [py_content_id(dom, bytes.fromhex(body)).hex() for dom, body, _ in golden]


def py_rejects(neg):
    out = []
    for _tier, inp, _reason in neg:
        try:
            py_decode(bytes.fromhex(inp))
            out.append(("ACCEPT", ""))
        except Rejected as e:
            out.append(("REJECT", e.reason))
    return out


def certify(name, addresses, rejects, golden, neg):
    pos = sum(1 for got, (_, _, want) in zip(addresses, golden) if got == want)
    core = [(v, r) for (v, r), (tier, _, _) in zip(rejects, neg) if tier == "core"]
    core_reasons = [reason for (tier, _, reason) in neg if tier == "core"]
    core_ok = sum(1 for (verdict, reason), want in zip(core, core_reasons) if verdict == "REJECT" and reason == want)
    certified = pos == len(golden) and core_ok == len(core)
    return {
        "runtime": name,
        "positive": (pos, len(golden)),
        "negative_core": (core_ok, len(core)),
        "certified": certified,
    }


def main():
    golden = load_golden()
    neg = load_negative()

    records = [
        certify("ARVES Rust (reference)", rust_addresses(golden), rust_rejects(neg), golden, neg),
        certify("ARVES Python (independent)", py_addresses(golden), py_rejects(neg), golden, neg),
    ]

    print("ARVES Runtime Certification — against the frozen Standard alone")
    print("=" * 66)
    for r in records:
        p, pt = r["positive"]
        c, ct = r["negative_core"]
        print(f"  {r['runtime']:<28} positive {p}/{pt}  core-reject {c}/{ct}  ->  "
              f"{'CERTIFIED' if r['certified'] else 'NOT CERTIFIED'}")
    all_certified = all(r["certified"] for r in records)
    print("-" * 66)
    print(f"  Independent Runtime Alliance: {sum(r['certified'] for r in records)}/{len(records)} runtimes "
          f"certified under ONE conformance -> {'PASS' if all_certified else 'FAIL'}")
    print("  Certified by the Standard + this harness alone — no maintainer required.")
    return 0 if all_certified else 1


if __name__ == "__main__":
    sys.exit(main())

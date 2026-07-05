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
    return [ln.split()[0] if ln.split() else "" for ln in out]  # guard empty/ERR lines (B4 secondary)


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
try:  # B4: a Kit-only checkout may lack the in-repo Python reference — degrade, don't crash.
    from acs001_address import content_id as py_content_id   # noqa: E402
    from acs002_decode import decode as py_decode, Rejected  # noqa: E402
    PY_AVAILABLE = True
except ImportError:
    PY_AVAILABLE = False


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
    try:  # keep output legible on legacy Windows console codepages
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    golden = load_golden()
    neg = load_negative()

    # B4 (verification/evidence/G2_READINESS.md): on a Kit-only checkout the reference Rust
    # binaries are not built. Guard their invocation so the harness degrades to an
    # UNAVAILABLE row instead of dying with an uncaught FileNotFoundError before printing
    # ANY verdict. certify()'s signature is unchanged — the RUNTIME_AUTHORS_GUIDE contract
    # still holds; only main() is made resilient, and the record list is data-driven so a
    # vendor can run only their own runtime.
    records = []
    if os.path.exists(RUST_BRIDGE) and os.path.exists(RUST_DECODE):
        try:
            records.append(certify("ARVES Rust (reference)", rust_addresses(golden),
                                   rust_rejects(neg), golden, neg))
        except (FileNotFoundError, OSError):
            records.append({"runtime": "ARVES Rust (reference)", "unavailable": True})
    else:
        records.append({"runtime": "ARVES Rust (reference)", "unavailable": True})
    if PY_AVAILABLE:
        records.append(certify("ARVES Python (independent)", py_addresses(golden),
                               py_rejects(neg), golden, neg))
    else:
        records.append({"runtime": "ARVES Python (independent)", "unavailable": True})

    print("ARVES Runtime Certification - against the frozen Standard alone")
    print("=" * 66)
    for r in records:
        if r.get("unavailable"):
            print(f"  {r['runtime']:<28} UNAVAILABLE (reference binaries not built in this "
                  f"checkout — build them, or run only your own runtime)")
            continue
        p, pt = r["positive"]
        c, ct = r["negative_core"]
        print(f"  {r['runtime']:<28} positive {p}/{pt}  core-reject {c}/{ct}  ->  "
              f"{'CERTIFIED (ACS-002 core)' if r['certified'] else 'NOT CERTIFIED'}")
    avail = [r for r in records if not r.get("unavailable")]
    all_certified = len(avail) > 0 and all(r["certified"] for r in avail)
    print("-" * 66)
    print(f"  Independent Runtime Alliance: {sum(r['certified'] for r in avail)}/{len(avail)} "
          f"available runtime(s) certified under ONE conformance -> {'PASS' if all_certified else 'FAIL'}")
    if len(avail) < len(records):
        print(f"  ({len(records) - len(avail)} runtime(s) unavailable in this checkout — not a "
              f"certification failure; build the reference bins or add your own record.)")
    # NOTE (B3, tracked in G2_READINESS.md): certify() above follows the frozen
    # RUNTIME_AUTHORS_GUIDE contract, which receives the answer key and does not recompute;
    # a hollow echo adapter can pass it. The non-gameable check is verify_runtime_sound.py.
    print("  Certified by the Standard + this harness alone — no maintainer required.")
    print("  SCOPE: this harness grades the ACS-002 core interop layer (positive + core-reject).")
    print("  The FULL ACS-001..005 surface — incl. the ACS-003/004/005 semantic reject tiers —")
    print("  is graded NON-GAMEABLY by verify_runtime_sound.py (a full-surface SOUND-CERTIFIED is")
    print("  the CHALLENGE.md win-condition; see G2_READINESS.md B3). Run it next.")
    return 0 if all_certified else 1


if __name__ == "__main__":
    sys.exit(main())

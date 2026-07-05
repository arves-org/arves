"""
certify_your_runtime.py — the copy-paste driver a *new* runtime uses to certify itself (rank 15).

`certify_runtime.py` and `verify_runtime_sound.py` ship with the two reference runtimes
(Rust + Python) already wired in. A G2 party asked "how do I certify MY runtime?" had to
read the harness source and hand-edit `main()`. This driver removes that friction: point it
at YOUR runtime and it grades you through the SAME non-gameable grader (`grade_sound`, which
owns the truth and hands your runtime inputs only), then prints your SOUND-CERTIFIED verdict.

Your runtime never has to be written in Python. If it can speak three tiny line protocols on
stdin/stdout (the exact ones the shipped reference bins already speak), you configure three
commands and write ZERO Python:

  --addr CMD       stdin: "<domain_hex> <body_hex>\\n" per line
                   stdout: one line per input whose FIRST whitespace token is your ContentId hex
                   (0x12 0x20 || SHA-256(domain_tag || body)); "ERR ..."/blank = a wrong answer.
  --decode CMD     stdin: "<body_hex>\\n" per line
                   stdout: "ACCEPT" | "ACCEPT\\t<hex>" | "REJECT\\t<reason>" | "ERR\\t..."
                   The reason is your ACS-002 reason code, compared byte-for-byte to the vector.
  --validate CMD   (OPTIONAL — ACS-003/004/005 semantic layer; omit → semantic DEFERRED)
                   stdin: "<tier>\\t<body_hex>\\n"  (tier ∈ envelope|instance|language)
                   stdout: "ACCEPT" | "REJECT\\t<kebab-reason-code>"

Each command is run ONCE with the whole batch on stdin (like the reference bins). Env vars
ARVES_ADDR_CMD / ARVES_DECODE_CMD / ARVES_VALIDATE_CMD are honoured if the flags are absent.

The grader is non-gameable and identical to the one the reference runtimes face: it recomputes
every ContentId itself, probes FRESH (domain, body) pairs not in the published vectors, and
injects valid bodies you MUST accept — so a hollow echo of the published answers fails. Reaching
`SOUND-CERTIFIED (full ACS-001..005 surface)` here, as a genuinely unrelated party with no help
from the authors, is the CHALLENGE.md G2 win-condition.

Examples:
  # Zero-Python: your runtime exposes three stdin/stdout programs.
  python verification/certification/certify_your_runtime.py \\
      --addr "./my-runtime address" --decode "./my-runtime decode" --validate "./my-runtime validate"

  # Prove this driver itself is real by grading the reference Rust bins through the vendor path
  # (build them first: cargo build -p arves-conformance -p arves-bridge --manifest-path runtime/Cargo.toml):
  python verification/certification/certify_your_runtime.py --self-test

  # In-process instead? Import grade_sound and pass Python adapter functions directly — see
  # CERTIFY_YOUR_RUNTIME.md "Path 2". This driver covers the language-agnostic line-protocol path.

Exit: 0 = SOUND-CERTIFIED, 1 = graded but NOT certified, 2 = not configured / driver error.
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)

# The grader owns the truth. We import the input sets + the non-gameable grader/printer from the
# shipped sound verifier and reuse them verbatim, so this driver cannot be "nicer" to a vendor
# than the reference runtimes' own grader is.
from verify_runtime_sound import (  # noqa: E402
    GOLDEN, FRESH, CORE, ACCEPT_PROBES, SEMANTIC, SEMANTIC_ACCEPT,
    grade_sound, _print_record,
)


def _run(cmd, payload):
    """Run a shell command once, feeding `payload` (bytes) on stdin; return stdout lines (str)."""
    proc = subprocess.run(cmd, shell=True, input=payload, stdout=subprocess.PIPE)
    return proc.stdout.decode("utf-8", "replace").splitlines()


def build_line_protocol_adapters(addr_cmd, decode_cmd, validate_cmd=None):
    """
    Build (addresser, rejecter, semantic) closures that answer from a vendor runtime driven over
    the three line protocols. Every input is batched through ONE process per protocol; the
    closures look answers up by exact input key. The vendor sees INPUTS ONLY — grade_sound owns
    every expected value — so this path is exactly as non-gameable as the in-process path.
    """
    # --- addresser: golden + fresh, in the exact order grade_sound will ask ---
    addr_inputs = [(d, b) for (_s, d, b, _c) in GOLDEN] + list(FRESH)
    addr_payload = "".join("%02x %s\n" % (d, bytes(b).hex()) for (d, b) in addr_inputs).encode()
    addr_lines = _run(addr_cmd, addr_payload)
    addr_map = {}
    for (d, b), line in zip(addr_inputs, addr_lines):
        tok = line.split()
        addr_map[(d, bytes(b))] = tok[0].lower() if tok else ""   # ERR/blank -> non-matching

    # --- rejecter: core-negative inputs + accept-probe bodies ---
    dec_inputs = [inp for (inp, _r) in CORE] + list(ACCEPT_PROBES)
    dec_payload = "".join(bytes(b).hex() + "\n" for b in dec_inputs).encode()
    dec_lines = _run(decode_cmd, dec_payload)
    dec_map = {}
    for b, line in zip(dec_inputs, dec_lines):
        parts = line.replace("\t", " ").split()
        verdict = parts[0] if parts else "ERR"
        reason = parts[1] if len(parts) > 1 else ""
        if verdict == "ACCEPT":
            dec_map[bytes(b)] = ("ACCEPT", "")
        elif verdict == "REJECT":
            dec_map[bytes(b)] = ("REJECT", reason)
        else:
            dec_map[bytes(b)] = ("ERR", reason)

    def addresser(domain, body):
        return addr_map.get((domain, bytes(body)), "")

    def rejecter(body):
        return dec_map.get(bytes(body), ("ERR", ""))

    # --- semantic (optional): ACS-003/004/005 tiers, code-exact ---
    semantic = None
    if validate_cmd:
        sem_inputs = [(tier, inp) for (tier, inp, _r) in SEMANTIC] \
            + [(tier, body) for (tier, body) in SEMANTIC_ACCEPT]
        sem_payload = "".join("%s\t%s\n" % (tier, bytes(b).hex()) for (tier, b) in sem_inputs).encode()
        sem_lines = _run(validate_cmd, sem_payload)
        sem_map = {}
        for (tier, b), line in zip(sem_inputs, sem_lines):
            parts = line.replace("\t", " ").split()
            verdict = parts[0] if parts else "ERR"
            code = parts[1] if len(parts) > 1 else ""
            sem_map[(tier, bytes(b))] = (verdict, code) if verdict in ("ACCEPT", "REJECT") else ("ERR", "")

        def semantic(tier, body):
            return sem_map.get((tier, bytes(body)), ("ERR", ""))

    return addresser, rejecter, semantic


def _parse_args(argv):
    """Minimal --flag VALUE parser (no argparse dep churn); env vars are the fallback."""
    opts = {"name": None, "addr": os.environ.get("ARVES_ADDR_CMD"),
            "decode": os.environ.get("ARVES_DECODE_CMD"),
            "validate": os.environ.get("ARVES_VALIDATE_CMD"), "self_test": False}
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--self-test":
            opts["self_test"] = True
        elif a in ("--addr", "--decode", "--validate", "--name") and i + 1 < len(argv):
            opts[a[2:]] = argv[i + 1]
            i += 1
        else:
            raise SystemExit("unknown/incomplete argument: %s (see the module docstring)" % a)
        i += 1
    return opts


def _reference_bin(name):
    p = os.path.join(ROOT, "runtime", "target", "debug", name)
    return p + ".exe" if os.path.exists(p + ".exe") else p


HELP = """\
certify_your_runtime.py — grade YOUR runtime through the non-gameable ARVES grader.

Not configured. Point the driver at your runtime's three line-protocol programs:

  python verification/certification/certify_your_runtime.py \\
      --addr "<your address program>" \\
      --decode "<your decode program>" \\
      [--validate "<your semantic validator program>"]   # omit -> semantic tiers DEFERRED

(or set ARVES_ADDR_CMD / ARVES_DECODE_CMD / ARVES_VALIDATE_CMD). Line protocols and the exact
copy-paste path are in verification/certification/CERTIFY_YOUR_RUNTIME.md.

To see a real pass first, grade the reference Rust bins through this same vendor path:
  cargo build -p arves-conformance -p arves-bridge --manifest-path runtime/Cargo.toml
  python verification/certification/certify_your_runtime.py --self-test
"""


def main(argv):
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    opts = _parse_args(argv)

    if opts["self_test"]:
        # Honest smoke test: drive the REFERENCE Rust bins over the identical vendor line protocol.
        # This grades a real runtime through this driver — proving the driver itself is sound — and
        # shows a reader exactly what a full-surface pass looks like. It is NOT your runtime.
        addr_bin, dec_bin, val_bin = (_reference_bin("arves-bridge"),
                                      _reference_bin("acs_decode"), _reference_bin("acs_validate"))
        if not (os.path.exists(addr_bin) and os.path.exists(dec_bin)):
            print("SELF-TEST UNAVAILABLE: reference bins not built. Run:\n"
                  "  cargo build -p arves-conformance -p arves-bridge --manifest-path runtime/Cargo.toml")
            return 2
        opts["name"] = "ARVES Rust (reference, via vendor path)"
        opts["addr"], opts["decode"] = addr_bin, dec_bin
        opts["validate"] = val_bin if os.path.exists(val_bin) else None

    if not (opts["addr"] and opts["decode"]):
        sys.stdout.write(HELP)
        return 2

    name = opts["name"] or "Your Runtime (vendor)"
    print("ARVES certify-your-runtime — non-gameable grader, your runtime given INPUTS ONLY")
    print("=" * 78)
    try:
        addresser, rejecter, semantic = build_line_protocol_adapters(
            opts["addr"], opts["decode"], opts["validate"])
    except OSError as e:
        print("driver error launching a runtime command: %s" % e)
        return 2

    rec = grade_sound(name, addresser, rejecter, semantic)
    _print_record(rec)
    print("-" * 78)
    print("  Grader owned every ContentId; fresh + accept probes defeat a hollow echo (gap B3).")
    if opts["validate"] is None:
        print("  semantic: DEFERRED — pass --validate to earn the unqualified full-surface stamp.")
    verdict = "SOUND-CERTIFIED" if rec["certified"] else "NOT CERTIFIED"
    print("  %s -> %s" % (name, verdict))
    if rec["certified"] and not opts["self_test"]:
        print("  If you are a genuinely unrelated party with no contact with the authors, this is the")
        print("  G2 evidence ARVES is missing — submit via CHALLENGE.md (record every Kit ambiguity).")
    return 0 if rec["certified"] else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

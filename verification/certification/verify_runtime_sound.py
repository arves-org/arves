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

A runtime is SOUND-CERTIFIED (full ACS-001..005 surface) iff it reproduces every published +
fresh address, decides every core-negative + accept-probe correctly, AND (rank 1) rejects every
ACS-003/004/005 semantic negative (envelope/instance/language) while accepting the golden valid
bodies. The Rust arm is graded on the exact registered kebab reason code (via the acs_validate
bin); the Python reference emits prose reasons, so its semantic arm is reject-verified. A runtime
that implements only the ACS-002 layer earns the LABELED lesser verdict "SOUND-CERTIFIED (ACS core;
semantic DEFERRED)" — the word is never printed unqualified, so a stamp cannot imply the whole
standard while attesting only the byte layer (this closed the B1 over-claim the gate previously had:
it graded tier=="core" only, 0 of the 19 semantic vectors).

TWO RUNTIMES, ONE GRADER (inputs-only)
--------------------------------------
This verifier grades BOTH reference runtimes under the identical non-gameable grader:

  * ARVES Python (independent) — imported in-process (`acs001_address`, `acs002_decode`).
  * ARVES Rust (reference)     — driven inputs-only over the shipped line-protocol bins
    (`arves-bridge`, `acs_decode`). The grader hands the Rust process only INPUTS
    (`<domain_hex> <body_hex>` to address, `<body_hex>` to decode) and recomputes/compares
    every answer here. So the Rust arm is graded by the same fresh + accept + core-reject
    probes; a byte-broken or stale Rust adapter that returns wrong ContentIds is NOT
    CERTIFIED exactly like a hollow Python echo would be.

The Rust adapters are GUARDED: on a Kit-only checkout the reference bins are not built, so
the adapter probe degrades to an UNAVAILABLE / SKIPPED line (never a crash / FileNotFoundError)
— mirroring certify_runtime.py's B4 behaviour. Unavailability is NOT a certification failure;
it is reported as SKIPPED and does not turn the run red on its own.

Adapter line protocols (frozen bins, read-only — see the bin source headers):
  * arves-bridge:  stdin `<domain_hex> <body_hex>\n` -> stdout `<contentId_hex> <status> <idx>`
                   (or `ERR <reason>`). The FIRST whitespace token of each output line is the
                   ContentId hex; `ERR`/empty lines are treated as a non-matching answer.
  * acs_decode:    stdin `<body_hex>\n` -> stdout `ACCEPT\t<hex>` | `REJECT\t<reason>` |
                   `ERR\tbad-hex`. Tab/space split -> (verdict, reason). The reason column is
                   the ACS-002 reason code, compared byte-for-byte against the negative vector.

This is the contract proposed for a future Kit 0.2.1 (which would converge
RUNTIME_AUTHORS_GUIDE + certify_runtime.py onto it — a maintainer-gated, frozen-Kit change).
It ships here as a LIVING check so the Verification arm can gate on B3 today without any
frozen-Kit edit.

Run:  python verification/certification/verify_runtime_sound.py
"""

import hashlib
import os
import subprocess
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

# Semantic (ACS-003/004/005) reject vectors — the envelope/instance/language tiers CCP-006/007
# added that the `core` gate never exercised. A runtime implementing the ACS-003/004/005 layer
# MUST reject each. The Rust arm (via the acs_validate bin) emits the registered kebab reason
# code and is graded on EXACT code equality; the Python reference emits prose reasons, so its arm
# is graded on rejection ("reject-verified" — the mapped-then-checked caveat, until the Python
# validators emit native codes). A pure ACS-002 codec passes no semantic rejecter and the tiers
# are reported DEFERRED, exactly like the standard's nfc tier.
SEMANTIC = [(tier, inp, reason) for (tier, inp, reason) in NEG
            if tier in ("envelope", "instance", "language")]

# Semantic ACCEPT probes: valid bodies the runtime MUST accept at the semantic layer, so a
# reject-everything adapter fails the semantic tiers too (grader-owned golden bodies + a valid
# GL term-set). Instance golden = the uci.fact instance (domain 0x01), not the schema (0x07).
_GOLDEN_ENVELOPE = next((b for (s, _d, b, _c) in GOLDEN if s == "ACS-003"), None)
_GOLDEN_INSTANCE = next((b for (s, d, b, _c) in GOLDEN if s == "ACS-004" and d == 0x01), None)
_VALID_TERMSET = ("\n".join("GL-%03d" % i for i in range(1, 15))).encode()
SEMANTIC_ACCEPT = [t for t in (
    ("envelope", _GOLDEN_ENVELOPE),
    ("instance", _GOLDEN_INSTANCE),
    ("language", _VALID_TERMSET),
) if t[1] is not None]

# Vector-integrity self-check: the grader's independent recompute of every published row
# must equal the stored ContentId. A mismatch means a corrupted vector, not a runtime bug.
for _std, _d, _b, _cid in GOLDEN:
    if expected_cid(_d, _b) != _cid:
        raise SystemExit("VECTOR INTEGRITY FAILURE: recompute != stored ContentId for a golden row")


def grade_sound(name, addresser, rejecter, semantic=None):
    """
    addresser: (domain: int, body: bytes) -> ContentId hex               (runtime under test)
    rejecter:  (body: bytes) -> (verdict, reason)                        (runtime under test)
    semantic:  (tier: str, body: bytes) -> (verdict, kebab_code|"")      (runtime under test; optional)
    The runtime is given inputs only; all expected values live here in the grader. `semantic`
    grades the ACS-003/004/005 tiers; a runtime that does not implement that layer passes
    semantic=None and the tiers are DEFERRED (like the nfc tier), not failed.
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

    # Semantic (ACS-003/004/005) tiers — graded only if the runtime provides a semantic
    # rejecter. Each negative MUST be REJECTED (and, when the runtime emits a registered
    # kebab code, that code MUST equal the grader-owned reason); each accept-probe MUST be
    # ACCEPTED so a reject-everything adapter cannot pass the semantic tiers.
    sem_reject = {"envelope": [0, 0], "instance": [0, 0], "language": [0, 0]}
    sem_accept = 0
    sem_graded = semantic is not None
    if sem_graded:
        for (tier, inp, reason) in SEMANTIC:
            sem_reject[tier][1] += 1
            verdict, code = semantic(tier, inp)
            if verdict == "REJECT" and (code == "" or code == reason):
                sem_reject[tier][0] += 1
        for (tier, body) in SEMANTIC_ACCEPT:
            verdict, _c = semantic(tier, body)
            if verdict == "ACCEPT":
                sem_accept += 1
    sem_ok = (not sem_graded) or (
        all(sem_reject[t][0] == sem_reject[t][1] for t in sem_reject)
        and sem_accept == len(SEMANTIC_ACCEPT)
    )

    certified = (
        published_ok == published_n
        and fresh_ok == len(FRESH)
        and core_ok == len(CORE)
        and accept_ok == len(ACCEPT_PROBES)
        and sem_ok
    )
    return {
        "runtime": name,
        "published": (published_ok, published_n),
        "fresh": (fresh_ok, len(FRESH)),
        "core_reject": (core_ok, len(CORE)),
        "accept": (accept_ok, len(ACCEPT_PROBES)),
        "semantic": sem_reject,
        "semantic_accept": (sem_accept, len(SEMANTIC_ACCEPT)),
        "semantic_graded": sem_graded,
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


# Python semantic (ACS-003/004/005) validators. The Python reference emits prose/R-code
# reasons, not the registered kebab codes, so this arm returns an EMPTY code and is graded on
# rejection only (reject-verified). The Rust arm is code-exact via the acs_validate bin.
from acs003_envelope import validate_envelope as _py_env, EnvelopeInvalid as _PyEnvInvalid  # noqa: E402
from acs004_instance import validate_instance as _py_inst   # noqa: E402
from acs005_checker import check_term_set as _py_terms       # noqa: E402
from acs002_dcbor import encode as _py_encode                # noqa: E402
import acs_values as _V                                      # noqa: E402

_PY_SCHEMA = py_decode(_py_encode(_V.acs004_schema_document()))


def py_semantic(tier, body):
    """(tier, body) -> (verdict, "") — reject-verified; code intentionally empty (prose native)."""
    try:
        if tier == "language":
            ok, _r = _py_terms(bytes(body))
            return ("ACCEPT", "") if ok else ("REJECT", "")
        value = py_decode(bytes(body))
        if tier == "envelope":
            try:
                _py_env(value)
                return ("ACCEPT", "")
            except _PyEnvInvalid:
                return ("REJECT", "")
        ok, _r = _py_inst(value, _PY_SCHEMA)
        return ("ACCEPT", "") if ok else ("REJECT", "")
    except Exception:  # noqa: BLE001 — a body that does not decode-clean is a non-ACCEPT
        return ("ERR", "")


# ---- Rust reference runtime adapters (driven inputs-only over the shipped bins) ----
#
# The grader gives the Rust process INPUTS ONLY and recomputes every expected value itself,
# so the Rust arm is exactly as non-gameable as the Python arm. Missing bins degrade to an
# UNAVAILABLE probe (see RustUnavailable) rather than crashing the run (B4 parity).

class RustUnavailable(Exception):
    """The Rust reference bins are not built in this checkout; the arm is SKIPPED, not failed."""


def _rust_exe(name):
    p = os.path.join(ROOT, "runtime", "target", "debug", name)
    return p + ".exe" if os.path.exists(p + ".exe") else p


RUST_BRIDGE = _rust_exe("arves-bridge")   # address bin: first token of each line = ContentId hex
RUST_DECODE = _rust_exe("acs_decode")     # decode bin:  ACCEPT/REJECT<TAB>reason | ERR<TAB>bad-hex
RUST_VALIDATE = _rust_exe("acs_validate") # semantic bin: <tier>\t<hex> -> ACCEPT | REJECT<TAB>kebab


def _run_bin(path, payload):
    """
    Feed `payload` (bytes) to the bin at `path` on stdin, return stdout lines (str).
    Degrades to RustUnavailable if the binary is absent or cannot be launched — the
    verifier must never die with FileNotFoundError on a Kit-only checkout (B4 parity).
    """
    if not os.path.exists(path):
        raise RustUnavailable(path)
    try:
        out = subprocess.run([path], input=payload, stdout=subprocess.PIPE).stdout
    except (FileNotFoundError, OSError) as e:  # e.g. wrong-arch bin / exec bit missing
        raise RustUnavailable(str(e))
    return out.decode("utf-8", "replace").splitlines()


def rust_build_adapters():
    """
    Build (addresser, rejecter) that answer from the Rust bins' output, keyed by input.
    All inputs are batched through ONE subprocess each (state persists across a bridge
    session, which is harmless: the first output token is the ContentId whether the commit
    is `committed` or `already-committed`). Raises RustUnavailable if a bin is missing so
    main() can record a SKIPPED row instead of crashing.

    Non-gameable: the runtime still only ever sees INPUTS; grade_sound() owns every expected
    value and compares here. A stale / byte-broken bridge (wrong ContentIds) simply mismatches
    and is NOT CERTIFIED — the fresh + accept probes cannot be echoed.
    """
    # Address inputs: the same set grade_sound() will address (golden + fresh), keyed exactly.
    addr_inputs = [(d, b) for (_s, d, b, _c) in GOLDEN] + list(FRESH)
    addr_payload = "".join("%02x %s\n" % (d, bytes(b).hex()) for (d, b) in addr_inputs).encode()
    addr_lines = _run_bin(RUST_BRIDGE, addr_payload)
    addr_map = {}
    for (d, b), line in zip(addr_inputs, addr_lines):
        tok = line.split()
        addr_map[(d, bytes(b))] = tok[0].lower() if tok else ""   # ERR/empty -> non-matching

    # Decode inputs: every core-negative input and every accept-probe body, keyed exactly.
    dec_inputs = [inp for (inp, _r) in CORE] + list(ACCEPT_PROBES)
    dec_payload = "".join(bytes(b).hex() + "\n" for b in dec_inputs).encode()
    dec_lines = _run_bin(RUST_DECODE, dec_payload)
    dec_map = {}
    for b, line in zip(dec_inputs, dec_lines):
        parts = line.replace("\t", " ").split()
        verdict = parts[0] if parts else "ERR"
        reason = parts[1] if len(parts) > 1 else ""
        if verdict == "ACCEPT":
            dec_map[bytes(b)] = ("ACCEPT", "")
        elif verdict == "REJECT":
            dec_map[bytes(b)] = ("REJECT", reason)
        else:  # ERR / empty -> a non-ACCEPT, non-matching-reason answer
            dec_map[bytes(b)] = ("ERR", reason)

    def rust_addr(domain, body):
        return addr_map.get((domain, bytes(body)), "")

    def rust_rej(body):
        return dec_map.get(bytes(body), ("ERR", ""))

    # Semantic adapter (acs_validate bin) — code-exact over the ACS-003/004/005 tiers. If the
    # bin is absent (partial build) degrade to None so the Rust arm still grades core with the
    # semantic tiers DEFERRED, rather than being skipped entirely.
    rust_semantic = None
    try:
        sem_inputs = [(tier, inp) for (tier, inp, _r) in SEMANTIC] \
            + [(tier, body) for (tier, body) in SEMANTIC_ACCEPT]
        sem_payload = "".join("%s\t%s\n" % (tier, bytes(b).hex()) for (tier, b) in sem_inputs).encode()
        sem_lines = _run_bin(RUST_VALIDATE, sem_payload)
        sem_map = {}
        for (tier, b), line in zip(sem_inputs, sem_lines):
            parts = line.replace("\t", " ").split()
            verdict = parts[0] if parts else "ERR"
            code = parts[1] if len(parts) > 1 else ""
            sem_map[(tier, bytes(b))] = (verdict, code) if verdict in ("ACCEPT", "REJECT") else ("ERR", "")

        def rust_semantic(tier, body):
            return sem_map.get((tier, bytes(body)), ("ERR", ""))
    except RustUnavailable:
        rust_semantic = None

    return rust_addr, rust_rej, rust_semantic


def _print_record(rec):
    p, pt = rec["published"]
    fr, frt = rec["fresh"]
    c, ct = rec["core_reject"]
    a, at = rec["accept"]
    # Coverage-labeled verdict — the word SOUND-CERTIFIED never appears unqualified, so the
    # stamp can never imply the whole standard while attesting only the ACS-002 core (closes
    # the B1 over-claim). "full ACS-001..005 surface" = core + all 3 semantic tiers graded/pass.
    if not rec["certified"]:
        verdict = "NOT CERTIFIED"
    elif rec.get("semantic_graded"):
        verdict = "SOUND-CERTIFIED (full ACS-001..005 surface)"
    else:
        verdict = "SOUND-CERTIFIED (ACS core; semantic DEFERRED)"
    print("  %-28s published %d/%d  fresh %d/%d  core-reject %d/%d  accept %d/%d  ->  %s"
          % (rec["runtime"], p, pt, fr, frt, c, ct, a, at, verdict))
    # Semantic (ACS-003/004/005) tiers — full-surface line (rank 1). DEFERRED if the runtime
    # did not provide a semantic rejecter (a pure ACS-002 codec).
    sr = rec["semantic"]
    sa, sat = rec["semantic_accept"]
    if rec.get("semantic_graded"):
        print("  %-28s   semantic: envelope %d/%d  instance %d/%d  language %d/%d  accept %d/%d"
              % ("", sr["envelope"][0], sr["envelope"][1], sr["instance"][0], sr["instance"][1],
                 sr["language"][0], sr["language"][1], sa, sat))
    else:
        print("  %-28s   semantic: DEFERRED (no ACS-003/004/005 validator provided)" % "")


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    print("ARVES Sound Runtime Verification - non-gameable (grader owns the truth)")
    print("=" * 70)

    records = []   # graded records (each has "certified")
    skipped = []   # (name, reason) for runtimes unavailable in this checkout

    # ARVES Python (independent) — imported in-process, always available in this repo.
    records.append(grade_sound("ARVES Python (independent)", py_addr, py_rej, py_semantic))

    # ARVES Rust (reference) — driven inputs-only over the shipped bins; SKIPPED if unbuilt.
    try:
        rust_addr, rust_rej, rust_semantic = rust_build_adapters()
        records.append(grade_sound("ARVES Rust (reference)", rust_addr, rust_rej, rust_semantic))
    except RustUnavailable as e:
        skipped.append(("ARVES Rust (reference)", str(e)))

    for rec in records:
        _print_record(rec)
    for name, why in skipped:
        print("  %-28s SKIPPED / UNAVAILABLE (reference bin not built in this checkout: %s)"
              % (name, os.path.basename(str(why)) or why))

    print("-" * 70)
    print("  Runtimes given INPUTS ONLY; grader recomputed every ContentId and re-decoded")
    print("  every input. Fresh + accept probes defeat a hollow echo adapter (gap B3).")
    graded_ok = all(rec["certified"] for rec in records)
    print("  %d/%d graded runtime(s) SOUND-CERTIFIED under ONE grader%s -> %s"
          % (sum(r["certified"] for r in records), len(records),
             (" (%d SKIPPED)" % len(skipped)) if skipped else "",
             "PASS" if graded_ok else "FAIL"))
    if skipped:
        print("  (SKIPPED != FAIL: build the reference bins to grade the Rust arm too.)")
    # A run is green iff at least one runtime was graded and every graded runtime certified.
    return 0 if (records and graded_ok) else 1


if __name__ == "__main__":
    sys.exit(main())

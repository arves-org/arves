# Certify YOUR runtime — the copy-paste last mile

> **Status:** LIVING · NON-NORMATIVE. This page is the *operational* companion to the cold-start
> packet [`IMPLEMENTING_ARVES.md`](../../IMPLEMENTING_ARVES.md) and the invitation
> [`CHALLENGE.md`](../../CHALLENGE.md). It changes nothing normative — `standard/` wins on any
> disagreement, and a disagreement is a docs bug to report. It assumes you have **already built**
> an ACS-001 addresser + ACS-002 canonical codec (and, for the full stamp, the ACS-003/004/005
> validators) from `standard/` alone. This is only *how to run the graders against what you built.*

You built a runtime. Here is the shortest honest path from "it works on my machine" to the
**`SOUND-CERTIFIED (full ACS-001..005 surface)`** verdict that is the G2 win-condition — with **zero
contact with the authors**.

## Two verdicts — know which one you are chasing

| Command | What it proves | Gameable? |
|---|---|---|
| `certify_runtime.py` | `positive 12/12  core-reject 16/16 -> CERTIFIED` — the ACS-002 interop **format** | **Yes** — it trusts your adapter's output (frozen `RUNTIME_AUTHORS_GUIDE` contract). Treat it as the verdict *shape*, not the proof. |
| **`verify_runtime_sound.py`** | **`SOUND-CERTIFIED`** — the grader **owns the truth**: it recomputes every ContentId, probes **fresh** `(domain, body)` pairs not in the vectors, and injects valid bodies you **must accept** | **No.** A hollow echo of the published answers fails. **This is the real proof and the CHALLENGE win-condition.** |

Chase the second one. The first is a convenience that reports the same numbers.

---

## Path 1 — zero Python (recommended): speak three line protocols

If your runtime can read a batch on **stdin** and write one answer **per line** to **stdout**, you
write no Python at all. These are the exact protocols the reference bins already speak.

| Program | stdin (one per line) | stdout (one per line) |
|---|---|---|
| **address** | `<domain_hex> <body_hex>` | a line whose **first whitespace token** is your `ContentId` hex = `0x12 0x20 ‖ SHA-256(domain_tag ‖ body)`. `ERR …`/blank = wrong answer. |
| **decode** | `<body_hex>` | `ACCEPT` · `ACCEPT\t<hex>` · `REJECT\t<reason>` · `ERR\t…`. `<reason>` is your ACS-002 reason code, compared **byte-for-byte** to the vector. |
| **validate** *(optional)* | `<tier>\t<body_hex>` (`tier ∈ envelope\|instance\|language`) | `ACCEPT` · `REJECT\t<kebab-reason-code>`. Omit this program → the semantic tiers are **DEFERRED** (you earn the labeled core stamp, not the full one). |

Then run the driver — it grades you through the **non-gameable** grader and prints your verdict:

```bash
python verification/certification/certify_your_runtime.py \
    --addr    "./my-runtime address" \
    --decode  "./my-runtime decode" \
    --validate "./my-runtime validate"     # omit for the ACS-002-core-only stamp
```

**See a real pass first.** Build the reference bins and grade *them* through the identical vendor
path — this proves the driver is real and shows you exactly what green looks like:

```bash
cargo build -p arves-conformance -p arves-bridge --manifest-path runtime/Cargo.toml
python verification/certification/certify_your_runtime.py --self-test
#  ARVES Rust (reference, via vendor path) published 12/12  fresh 3/3  core-reject 16/16
#     accept 3/3  ->  SOUND-CERTIFIED (full ACS-001..005 surface)
#     semantic: envelope 7/7  instance 8/8  language 4/4  accept 3/3
```

Exit code: `0` = SOUND-CERTIFIED · `1` = graded but not certified · `2` = not configured.

---

## Path 2 — in-process (your runtime is importable from Python)

Import the grader and pass it three functions. The grader hands them **inputs only**; every
expected value lives inside the grader.

```python
# your_cert.py  (run from the repo root: python your_cert.py)
import sys
sys.path.insert(0, "verification/certification")
from verify_runtime_sound import grade_sound, _print_record

def addresser(domain: int, body: bytes) -> str:
    # return your ContentId hex for (domain_tag, body)
    return my_runtime.content_id(domain, body).hex()

def rejecter(body: bytes):
    # return ("ACCEPT", "") or ("REJECT", "<acs-002-reason-code>")
    try:
        my_runtime.decode(body);  return ("ACCEPT", "")
    except my_runtime.Rejected as e:
        return ("REJECT", e.reason)

def semantic(tier: str, body: bytes):        # optional; drop the arg for core-only
    # tier in {"envelope","instance","language"} -> ("ACCEPT","") | ("REJECT","<kebab-code>")
    return my_runtime.validate(tier, body)

rec = grade_sound("My Runtime (vendor)", addresser, rejecter, semantic)  # semantic=None -> DEFERRED
_print_record(rec)
sys.exit(0 if rec["certified"] else 1)
```

The convenience `certify_runtime.py` uses a slightly different (list-based, answer-key) contract —
`certify(name, addresses(golden), rejects(neg), golden, neg)`; use it only for the format line. The
`grade_sound` contract above is the one that actually proves your work.

---

## The rules (they are the point)

- **No reading `runtime/`.** Build from the normative `standard/` text. If you needed our code, the
  Kit failed — that ambiguity *is* a finding.
- **We will not help during the attempt.** A question collapses G2 → G1. Bank every ambiguity for
  your submission instead.
- **`SOUND-CERTIFIED` given inputs-only, as a genuinely unrelated party, is the G2 evidence ARVES is
  missing.** Submit it via [`CHALLENGE.md`](../../CHALLENGE.md) → *How to submit* (the
  `g2-runtime-certification` issue template): your source, the **verbatim** `verify_runtime_sound.py`
  (or `certify_your_runtime.py`) output, a no-contact confirmation, and every point the Kit forced a
  guess. The ambiguity list is worth as much as the pass — it becomes a CCP Amendment that hardens
  the Standard for the next party.

*Honest status: G2 is open and unmet. This page makes it attemptable; reaching it is your part.*

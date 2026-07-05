#!/usr/bin/env python3
"""
CCP-010 (DRAFT) — open-debt #21: ACS-005 §9.1 requires a resolvable `GL-nnn` glossary
entry for "Data Plane", but §7 closes the glossary at GL-014 and defines "Data Plane"
only INLINE in the §7 closing note. This script is the DRAFT's oracle: it demonstrates,
machine-checked, why the two candidate fixes differ in kind:

  * Option A (add a first-class `GL-015 Data Plane` term entry) is BYTE-AFFECTING:
    the §8/§9.2-v1 term-set body becomes `GL-001..GL-015` and its ContentId CHANGES
    from the frozen golden — an ACS-005/2 profile bump, never a silent edit.
  * Option B (amend the §9.1 resolution wording so "Data Plane" resolves via the §7
    inline definition, alias-style like Workspace/CP) is BYTE-CLEAN: no §9.2 vector
    body changes, so no golden ContentId changes.

WHAT THIS SCRIPT PROVES (freeze-clean; nothing under standard/ is touched):
  1. the CURRENT golden term-set body (GL-001..GL-014) PASSES `check_term_set` and
     addresses to the frozen §9.2-v1 golden ContentId (the anchor);
  2. the Option-A body (GL-001..GL-015) is itself WELL-FORMED under `check_term_set`
     (so Option A is implementable) but its ContentId DIFFERS from the golden —
     the byte-affecting proof;
  3. the §9.2-v3 term-NAME vector is unchanged by Option A BY CONSTRUCTION ("Data
     Plane" is already in the §9.1 name list, and Option A does not edit that
     list); the check below is an ANCHOR check — it recomputes the untouched
     frozen body and confirms the golden — not a computation over an Option-A
     state. The bump is confined to vector #1 (plus the §8 GL-001..GL-015
     wording amendment, which is textual);
  4. the §9.3 glossary-resolution lint verdict is PASS-GATED today (Data Plane is
     the single gated term) and flips to clean PASS under a simulated Option-A
     resolution (Data Plane -> GL-015) — the ratification oracle for Option A.
     (Option B's obligation is textual — a §9.1 wording amendment — so its proof
     here is the byte-clean anchor: every golden ContentId stays identical.)

Run:  python verification/ccp-drafts/gen_ccp010_vector.py
Exit 0 iff all four demonstrations hold.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
sys.path.insert(0, PYDIR)

from acs001_address import content_id                              # noqa: E402
import acs_values as V                                             # noqa: E402
import acs005_checker as C                                         # noqa: E402
from acs005_checker import check_term_set, glossary_resolution_lint  # noqa: E402

# Frozen goldens (standard/vectors/acs_golden_vectors.tsv, ACS-005 rows; read-only).
GOLDEN_TERM_SET = "1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074"
GOLDEN_TERM_NAMES = "12200c1c893c613d0f12976697084f05a76243589ed55a3d2cdae9dbce9d69df4751"


def option_a_term_set_body() -> bytes:
    """Option-A candidate: §8 term-set body with the new GL-015, i.e. GL-001..GL-015,
    sorted ascending, LF-joined, no trailing LF (identical §8/§11 framing)."""
    ids = ["GL-%03d" % i for i in range(1, 16)]   # GL-001 .. GL-015
    return "\n".join(ids).encode("utf-8")


def simulate_option_a_lint():
    """Simulate ratified Option A IN MEMORY ONLY (no frozen byte is touched):
    'Data Plane' gains a first-class GL-015 entry, so it leaves the gated set.
    Returns the §9.3 lint verdict under that simulation."""
    saved_gated = dict(C._KNOWN_GATED)
    saved_map = dict(C._GLOSSARY_TERM_TO_GL)
    try:
        C._KNOWN_GATED.pop("Data Plane", None)
        C._GLOSSARY_TERM_TO_GL["Data Plane"] = "GL-015"
        verdict, _rows = glossary_resolution_lint()
        return verdict
    finally:
        C._KNOWN_GATED.clear()
        C._KNOWN_GATED.update(saved_gated)
        C._GLOSSARY_TERM_TO_GL.clear()
        C._GLOSSARY_TERM_TO_GL.update(saved_map)


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

    print("CCP-010 candidate — GL-015 'Data Plane' (open-debt #21)")
    print("=" * 68)

    # (1) CURRENT golden term-set: well-formed + anchored to the frozen golden.
    cur_body = V.acs005_term_set_body()                  # GL-001..GL-014
    cur_ok, cur_reason = check_term_set(cur_body)
    cur_cid = content_id(0x08, cur_body).hex()
    print("  CURRENT term-set (GL-001..GL-014)")
    print("    check_term_set     :", "ACCEPTS" if cur_ok else ("rejects: " + cur_reason))
    print("    ContentId          :", cur_cid)
    print("    matches golden     :", cur_cid == GOLDEN_TERM_SET)

    # (2) Option-A term-set (GL-001..GL-015): well-formed, but a DIFFERENT address.
    a_body = option_a_term_set_body()
    a_ok, a_reason = check_term_set(a_body)
    a_cid = content_id(0x08, a_body).hex()
    print("  OPTION-A term-set (GL-001..GL-015)")
    print("    check_term_set     :", "ACCEPTS (well-formed)" if a_ok else ("rejects: " + a_reason))
    print("    body (hex)         :", a_body.hex())
    print("    ContentId          :", a_cid)
    print("    differs from golden:", "YES -> BYTE-AFFECTING (ACS-005/2 profile bump)"
          if a_cid != GOLDEN_TERM_SET else "no (?!)")

    # (3) The §9.2-v3 term-NAME vector is unchanged by Option A BY CONSTRUCTION
    #     (the §9.1 name list is not edited; "Data Plane" is already in it). This
    #     is an ANCHOR check on the untouched frozen body, not an Option-A state.
    tn_body = V.acs005_term_names_body()
    tn_cid = content_id(0x08, tn_body).hex()
    tn_unaffected = (tn_cid == GOLDEN_TERM_NAMES) and \
        ("Data Plane" in tn_body.decode("utf-8").split("\n"))
    print("  TERM-NAME vector (§9.2 v3) — anchor check (unchanged by construction)")
    print("    'Data Plane' already listed  :", "Data Plane" in tn_body.decode("utf-8").split("\n"))
    print("    anchor recomputed == golden  :", tn_cid == GOLDEN_TERM_NAMES)

    # (4) §9.3 lint: PASS-GATED today; clean PASS under simulated Option A.
    cur_verdict, rows = glossary_resolution_lint()
    gated = sorted(t for (t, s, _d) in rows if s == "GATED")
    sim_verdict = simulate_option_a_lint()
    print("  §9.3 GLOSSARY-RESOLUTION LINT")
    print("    current verdict            :", cur_verdict, "(gated: %s)" % ", ".join(gated))
    print("    simulated Option-A verdict :", sim_verdict)
    print("  OPTION B (amend §9.1 wording; alias-style resolution)")
    print("    byte-clean: no §9.2 body changes; both goldens above stay identical.")
    print("    (Its proof obligation is textual — maintainer rules on the wording.)")

    ok = (
        cur_ok and cur_cid == GOLDEN_TERM_SET
        and a_ok and a_cid != GOLDEN_TERM_SET
        and tn_unaffected
        and cur_verdict == "PASS-GATED" and gated == ["Data Plane"]
        and sim_verdict == "PASS"
    )
    if not ok:
        print("RESULT: RED — the demonstration did not hold")
        return 1
    print("RESULT: GREEN — gap + both options demonstrated (DRAFT; ratification is CCP-GATE,")
    print("maintainer-authorized; NO frozen byte was touched by this script).")
    return 0


if __name__ == "__main__":
    sys.exit(main())

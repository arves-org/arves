"""
ACS-005 — Normative-Language reference CHECKER (well-formedness of the §8/§9.2
addressed bodies).

Independent implementation from the ARVES Standard Kit ONLY
(standard/acs/ACS-005_Normative_Language.md). No reference-runtime source used.

WHAT THIS PROVES
----------------
ACS-005 §9.3 defines a conformant checker by its pass/fail semantics but does not
spell out the structural well-formedness of the addressed bodies inline; that
well-formedness is *pinned by* §8 (addressing framing), §9.2 (the byte-exact
vectors), and §11 (reproducibility). This module implements the §8/§9.2/§11
well-formedness rules as an executable oracle, so that:

  * every VALID body built by `acs_values` is ACCEPTED and addresses to its §9.2
    golden ContentId (positive proof, KPI = Evidence Increased); and
  * a single-rule mutation of a valid body is REJECTED with a §-citing reason
    (the negative oracle for a future negative-vector CCP).

REASON STRINGS ARE FREE-FORM (ACS-001 §4.1 defers stable reason codes to a future
CCP). Each reason cites the governing § of ACS-005. This is intentional and
correct for this subset — do NOT read stable codes into these strings.

CHECKER RULES (each cites the ACS-005 § that fixes it)
------------------------------------------------------
Structural (all three body kinds):
  R-UTF8    §9.2/§11   body MUST be valid UTF-8.
  R-NFC     §8/§9.2    body text MUST be Unicode NFC (canonical text form; the
                       ACS text-canonicalization discipline the addressed bytes
                       assume — see SCOPE note in check header).
  R-NOLEAD  §8/§11     no leading LF (bodies are a "single \n" join — §11).
  R-NOTRAIL §8/§11     no trailing LF ("no trailing newline" — §8 term-set bullet,
                       §9.2 vectors, §11).
  R-NOBLANK §8/§11     no blank line / no empty entry (a "single \n" join between
                       non-empty entries forbids "\n\n" — §11).
  R-SORT    §8/§11     entries MUST be in strictly ascending order
                       ("sorted ascending" — §8, §11; SD-003 latent-coupling note).
  R-NODUP   §8/§11/§5  no duplicate entry (ascending term/ID lists; Term/Req IDs
                       are stable & unique — §5, §7).

Grammar (per body kind):
  R-GRAM/term-set   §9.2 v1 / §7    each entry MUST match `GL-` + 3 digits (GL-nnn).
  R-GRAM/term-names §9.1            each entry MUST be a §9.1 capitalized term
                                    (Titlecase words, space-separated).
  R-GRAM/requirement §9.2 v2 / §5   the body MUST be exactly one clause of the form
                                    `<RequirementId>: <clause text>` where
                                    RequirementId matches the §5 grammar
                                    (Owner "-" 3*DIGIT "-R" 1*DIGIT) and the clause
                                    text is non-empty.

SCOPE NOTE (rules scoped out, with reason)
------------------------------------------
  * Stable reason CODES — OUT (ACS-001 §4.1 future CCP; reasons are free-form here).
  * §9.3 keyword-lint semantics (a checker MUST NOT flag lower-case must/shall) —
    OUT of this module: that is a *document* lint over prose, not a rule about the
    §8/§9.2 addressed *bodies* this checker validates. The bodies contain no
    lower-case-keyword hazard (the requirement body's keywords are authored
    ALL-CAPS by §9.2 v2). Implementing a prose lint here would exceed the subset.
  * ContentId equality to the §9.2 goldens is asserted for the positive bodies as
    an anchor (§9.2/§11), but a mismatching ContentId is not itself a structural
    "reason" — a body that is well-formed but semantically different is simply a
    *different* well-formed body; §9.3's ContentId clause is checked in self-test.
"""

import re
import sys
import unicodedata

from acs_values import (
    acs005_term_set_body,
    acs005_requirement_body,
    acs005_term_names_body,
)
from acs001_address import content_id


# --- §9.2 golden ContentIds (hex), for the positive anchor -------------------
GOLDEN = {
    "term-set":    "1220ced393907a4d27eb54ac12acea65e29c7168c2991b3ca9df4b39765e870d2074",
    "requirement": "12207f1a532d2be5061377d6664be065bbb45b6e61741bb70c1195454054e1cf0475",
    "term-names":  "12200c1c893c613d0f12976697084f05a76243589ed55a3d2cdae9dbce9d69df4751",
}
DOMAIN = {"term-set": 0x08, "requirement": 0x09, "term-names": 0x08}

# --- Grammars ----------------------------------------------------------------
# §7 / §9.2 v1: glossary Term IDs are GL- + zero-padded 3-digit sequence.
_TERM_ID = re.compile(r"^GL-[0-9]{3}$")

# §5: RequirementId = Owner "-" 3*DIGIT "-R" 1*DIGIT ; Owner = uppercase-alpha { uppercase-alpha | "-" }
_REQ_ID = re.compile(r"^[A-Z][A-Z-]*-[0-9]{3}-R[0-9]+$")

# §9.1: capitalized normative term — Titlecase words separated by single spaces.
# (Each word starts uppercase; letters only; e.g. "Cognitive Entity", "Data Plane".)
_TERM_NAME = re.compile(r"^[A-Z][a-z]+( [A-Z][a-z]+)*$")


def _structural_checks(body: bytes):
    """
    Structural well-formedness common to every §8/§9.2 body kind.
    Returns (ok, reason, entries). `entries` is the LF-split list when ok.
    """
    # R-UTF8 (§9.2/§11): body MUST be valid UTF-8.
    try:
        text = body.decode("utf-8")
    except UnicodeDecodeError as e:
        return (False, "R-UTF8 (ACS-005 §9.2/§11): body is not valid UTF-8: %s" % e, None)

    # R-NFC (§8/§9.2): body text MUST be in Unicode NFC canonical form.
    if unicodedata.normalize("NFC", text) != text:
        return (False, "R-NFC (ACS-005 §8/§9.2): body text is not Unicode NFC (canonical text form).", None)

    # R-NOTRAIL (§8/§11): no trailing LF ("no trailing newline").
    if text.endswith("\n"):
        return (False, "R-NOTRAIL (ACS-005 §8/§11): body has a trailing LF; the join has no trailing newline.", None)

    # R-NOLEAD (§8/§11): no leading LF (bodies are a single-\n join of non-empty entries).
    if text.startswith("\n"):
        return (False, "R-NOLEAD (ACS-005 §8/§11): body has a leading LF; entries are joined by a single \\n.", None)

    entries = text.split("\n")

    # R-NOBLANK (§8/§11): no blank line / empty entry ("single \n" join, no \n\n).
    if any(e == "" for e in entries):
        return (False, "R-NOBLANK (ACS-005 §8/§11): body contains a blank line / empty entry; a single-\\n join forbids \\n\\n.", None)

    # R-NODUP (§8/§11/§5): no duplicate entry (stable, unique Term/Req IDs; ascending list).
    if len(set(entries)) != len(entries):
        return (False, "R-NODUP (ACS-005 §8/§11/§5): body contains a duplicate entry; list entries MUST be unique.", None)

    # R-SORT (§8/§11): entries MUST be in strictly ascending order.
    if entries != sorted(entries):
        return (False, "R-SORT (ACS-005 §8/§11): entries are not in ascending sorted order.", None)

    return (True, "ok", entries)


def _grammar_check(entries, pattern, label, section):
    """R-GRAM (§): each entry MUST match the required grammar for this body kind."""
    for e in entries:
        if not pattern.match(e):
            return (False, "R-GRAM/%s (ACS-005 %s): entry %r does not match the required grammar." % (label, section, e))
    return (True, "ok")


def check_term_set(body: bytes):
    """
    Validate a §8 tag-0x08 glossary term-SET body (Term IDs GL-001..GL-nnn).
    Returns (ok: bool, reason: str).
    """
    ok, reason, entries = _structural_checks(body)
    if not ok:
        return (ok, reason)
    return _grammar_check(entries, _TERM_ID, "term-set", "§9.2 v1/§7")


def check_term_names(body: bytes):
    """
    Validate a §8 tag-0x08 term-NAME list body (the §9.1 capitalized terms).
    Returns (ok: bool, reason: str).
    """
    ok, reason, entries = _structural_checks(body)
    if not ok:
        return (ok, reason)
    return _grammar_check(entries, _TERM_NAME, "term-names", "§9.1")


def check_requirement(body: bytes):
    """
    Validate a §9.2 v2 tag-0x09 requirement-CLAUSE body:
    exactly one line `<RequirementId>: <clause text>` (§5 ID grammar; non-empty clause).
    Returns (ok: bool, reason: str).
    """
    ok, reason, entries = _structural_checks(body)
    if not ok:
        return (ok, reason)
    # A requirement clause is a single line (§9.2 v2 pins one clause).
    if len(entries) != 1:
        return (False, "R-GRAM/requirement (ACS-005 §9.2 v2): a requirement clause body MUST be exactly one line, found %d." % len(entries))
    line = entries[0]
    # Split on the first ": " that follows the RequirementId.
    if ": " not in line:
        return (False, "R-GRAM/requirement (ACS-005 §9.2 v2): clause MUST be `<RequirementId>: <text>`; missing `: ` separator.")
    req_id, _, clause = line.partition(": ")
    if not _REQ_ID.match(req_id):
        return (False, "R-GRAM/requirement (ACS-005 §5): RequirementId %r does not match Owner-NNN-Rn grammar." % req_id)
    if clause.strip() == "":
        return (False, "R-GRAM/requirement (ACS-005 §9.2 v2): clause text after the RequirementId MUST be non-empty.")
    return (True, "ok")


# ---------------------------------------------------------------------------
# Self-test — the proof.
# ---------------------------------------------------------------------------

def run_selftest() -> int:
    results = []  # (name, expect_ok, got_ok, reason)

    def record(name, expect_ok, got_ok, reason):
        results.append((name, expect_ok, got_ok, reason))

    # --- POSITIVE: each valid body accepted, and anchored to its golden ContentId ---
    positives = [
        ("term-set",    acs005_term_set_body(),    check_term_set),
        ("requirement", acs005_requirement_body(), check_requirement),
        ("term-names",  acs005_term_names_body(),  check_term_names),
    ]
    for name, body, checker in positives:
        ok, reason = checker(body)
        record("POS accept " + name, True, ok, reason)
        # §9.2/§11 anchor: the valid body MUST address to its golden ContentId.
        got_cid = content_id(DOMAIN[name], body).hex()
        record("POS ContentId " + name, True, got_cid == GOLDEN[name],
               "got %s want %s" % (got_cid, GOLDEN[name]))

    # --- NEGATIVES: one per rule, each a single-mutation of a valid body ---
    # Base bodies (decoded) to mutate.
    ts_text = acs005_term_set_body().decode("utf-8")     # GL-001..GL-014
    tn_text = acs005_term_names_body().decode("utf-8")   # Capability..Tenant
    rq_text = acs005_requirement_body().decode("utf-8")  # ORCH-001-R1: ...

    negatives = [
        # R-NOTRAIL: trailing LF appended.
        ("NEG trailing-LF", check_term_set, (ts_text + "\n").encode("utf-8"), "R-NOTRAIL"),
        # R-NOLEAD: leading LF prepended.
        ("NEG leading-LF", check_term_set, ("\n" + ts_text).encode("utf-8"), "R-NOLEAD"),
        # R-SORT: swap first two entries to break ascending order.
        ("NEG out-of-order", check_term_set,
         "\n".join(["GL-002", "GL-001"] + ts_text.split("\n")[2:]).encode("utf-8"), "R-SORT"),
        # R-NODUP: duplicate an entry (kept in order so only NODUP fires).
        ("NEG duplicate", check_term_set,
         "\n".join(["GL-001", "GL-001"] + ts_text.split("\n")[1:]).encode("utf-8"), "R-NODUP"),
        # R-NOBLANK: insert a blank line (empty entry) after the first.
        ("NEG blank-line", check_term_set,
         "\n".join(["GL-001", ""] + ts_text.split("\n")[1:]).encode("utf-8"), "R-NOBLANK"),
        # R-GRAM (term-set): malformed ID (non-digit in the 3-digit sequence).
        # Mutate the LAST entry GL-014 -> GL-01A so sort order & uniqueness still
        # hold (GL-01A > GL-013) and R-GRAM is the ONLY rule that can fire.
        ("NEG malformed-id", check_term_set,
         "\n".join(ts_text.split("\n")[:-1] + ["GL-01A"]).encode("utf-8"), "R-GRAM"),
        # R-GRAM (term-names): malformed term (digit not allowed in §9.1 grammar).
        # Mutate the LAST entry Tenant -> Tenant1 so ascending order still holds
        # (Tenant1 > Shard, and it remains the max) and only R-GRAM fires.
        ("NEG malformed-term", check_term_names,
         "\n".join(tn_text.split("\n")[:-1] + ["Tenant1"]).encode("utf-8"), "R-GRAM"),
        # R-GRAM (requirement): drop the `: ` separator.
        ("NEG req-no-sep", check_requirement,
         rq_text.replace(": ", " ", 1).encode("utf-8"), "R-GRAM"),
        # R-GRAM (requirement): malformed RequirementId (breaks §5 grammar).
        ("NEG req-bad-id", check_requirement,
         rq_text.replace("ORCH-001-R1", "orch-1", 1).encode("utf-8"), "R-GRAM"),
        # R-UTF8: invalid UTF-8 bytes.
        ("NEG invalid-utf8", check_term_set, b"GL-001\n\xff\xfe", "R-UTF8"),
        # R-NFC: a non-NFC (NFD) codepoint injected into a term name.
        ("NEG non-nfc", check_term_names,
         unicodedata.normalize("NFD", "Capabilité").encode("utf-8")
         + b"\n" + "\n".join(tn_text.split("\n")[1:]).encode("utf-8"), "R-NFC"),
    ]
    for name, checker, body, want_rule in negatives:
        ok, reason = checker(body)
        # Reject (ok is False) AND the reason cites the intended rule.
        rejected_for_right_reason = (not ok) and (want_rule in reason)
        record(name, False, ok, reason if not ok else "UNEXPECTEDLY ACCEPTED")
        if ok:
            # keep as failure; already recorded above
            pass
        elif want_rule not in reason:
            # rejected but for the wrong rule — flag it
            results[-1] = (name + " (WRONG RULE, wanted %s)" % want_rule, False, ok, reason)

    # --- Report ---
    all_pass = True
    for name, expect_ok, got_ok, reason in results:
        passed = (expect_ok == got_ok)
        # For negatives, the reason must also cite the right rule; that was folded
        # into the name above, so a wrong-rule case still shows here as a FAIL only
        # if got_ok != expect_ok. Re-check reason-tag for negatives:
        status = "PASS" if passed else "FAIL"
        if not passed:
            all_pass = False
        print("[%s] %-40s expect_ok=%-5s got_ok=%-5s  %s"
              % (status, name, expect_ok, got_ok, reason))

    # Additional guard: every NEG must have been rejected for its intended rule.
    # (A negative rejected for the wrong rule was renamed with "(WRONG RULE …)" and
    #  still counts as expect_ok=False/got_ok=False → passed above; catch it here.)
    wrong_rule = [r for r in results if "(WRONG RULE" in r[0]]
    if wrong_rule:
        all_pass = False
        for r in wrong_rule:
            print("[FAIL] %s rejected but not for the intended rule." % r[0])

    total = len(results)
    passed_n = sum(1 for (_, e, g, _) in results if e == g) - len(wrong_rule)
    print("-" * 72)
    print("SUMMARY: %d/%d cases passed (%d positives+anchors, %d negatives)%s"
          % (passed_n, total, len(positives) * 2, len(negatives),
             "" if all_pass else "  <-- FAILURES PRESENT"))
    return 0 if all_pass else 1


if __name__ == "__main__":
    sys.exit(run_selftest())

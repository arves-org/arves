#!/usr/bin/env python3
"""
Freeze-diff gate: mechanically detect a SILENT content edit to a frozen file.

The ARVES freeze (runtime/ + standard/ + the frozen spec mirror spec-markdown/ and source-of-record
corpus/) was enforced only by author discipline + a git tag (runtime-v1.0); nothing mechanically
caught a silent edit (verification/evidence/SYSTEM_GAP_ANALYSIS.md #10/#16).
This tool hashes every frozen SOURCE file and compares to a committed manifest, so drift becomes
a checkable gate instead of a promise. It is a LIVING tool: it READS frozen files, never edits them.

  python verification/freeze/freeze_check.py update    # (re)write the baseline manifest — do this
                                                        # ONLY as part of a sanctioned RCR / CCP.
  python verification/freeze/freeze_check.py check      # exit 1 if any frozen file drifted from it
  python verification/freeze/freeze_check.py selftest   # prove it detects a tamper and passes clean

The manifest is a ROLLING baseline: the frozen state legitimately advances via sanctioned RCR
(runtime/) and CCP (standard/) changes; when one lands, `update` is run as part of that instrument.
Any OTHER change to a frozen file is silent drift and `check` fails.
"""

import hashlib
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
# The frozen surfaces (CLAUDE.md rule #1 calls the specification the STRONGEST freeze):
#   runtime/ + standard/  — the reference runtime + the Standard Kit (changes via RCR/CCP), and
#   spec-markdown/ + corpus/ — the frozen specification MIRROR (.md) and SOURCE-OF-RECORD (.docx),
#     which change only via CCP / regeneration. Without these last two a silent edit to the
#     spec mirror (e.g. the Invariant Registry) passes CI (SYSTEM_GAP #10/#16, DEEP_AUDIT V1-V3).
FROZEN_ROOTS = ["runtime", "standard", "spec-markdown", "corpus"]
EXCLUDE_DIRS = {"target", "__pycache__", ".git", "node_modules", ".pytest_cache"}
MANIFEST = os.path.join(HERE, "freeze_manifest.tsv")


def iter_frozen_files():
    # The freeze protects the REPOSITORY's frozen content, so the authoritative file
    # list is `git ls-files` over the frozen roots — the exact set every clean clone
    # materializes. A filesystem walk (the previous behaviour) also swept LOCAL-ONLY
    # derivative artifacts into the manifest (e.g. the 50 never-committed
    # runtime/review-input/*.txt corpus conversions), which made the very first public
    # CI run report 50 phantom "REMOVED" drifts on a clean clone (first-publish
    # finding, 2026-07-05). Tracked-but-deleted files still bite: git lists them, the
    # hash read fails against the manifest as MISSING/REMOVED.
    try:
        out = subprocess.run(
            ["git", "ls-files", "-z", "--"] + FROZEN_ROOTS,
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        )
        if out.returncode == 0 and out.stdout:
            for rel in sorted(r for r in out.stdout.decode("utf-8").split("\0") if r):
                p = os.path.join(ROOT, rel)
                if os.path.exists(p):
                    yield rel.replace(os.sep, "/"), p
            return
    except OSError:
        pass
    # Fallback (no git available — e.g. a tarball checkout): the filesystem walk.
    for root in FROZEN_ROOTS:
        base = os.path.join(ROOT, root)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = sorted(d for d in dirnames if d not in EXCLUDE_DIRS)
            for fn in sorted(filenames):
                p = os.path.join(dirpath, fn)
                rel = os.path.relpath(p, ROOT).replace(os.sep, "/")
                yield rel, p


def compute():
    out = {}
    for rel, p in iter_frozen_files():
        with open(p, "rb") as f:
            # Line-ending-independent hashing: normalize CRLF -> LF before hashing so the
            # SAME content hashes identically on a Windows checkout (git autocrlf smudges
            # text files to CRLF) and a Linux CI checkout (LF). Without this, 67 frozen
            # files hashed differently across platforms and the CI freeze gate would
            # report mass phantom drift on a clean clone. The normalization is a
            # deterministic function of the bytes, applied uniformly (binaries too), so
            # both platforms always agree; a REAL content edit still changes the hash —
            # only a pure line-ending flip (exactly what git's smudge does) is invisible.
            out[rel] = hashlib.sha256(f.read().replace(b"\r\n", b"\n")).hexdigest()
    return out


def write_manifest(d):
    with open(MANIFEST, "w", encoding="utf-8", newline="\n") as f:
        f.write("# ARVES freeze manifest — sha256 of every frozen source file (runtime/ + standard/, "
                "excluding build artifacts).\n")
        f.write("# Rolling baseline: regenerate (`update`) ONLY as part of a sanctioned RCR/CCP. "
                "`check` fails on any other drift.\n")
        for rel in sorted(d):
            f.write("%s\t%s\n" % (rel, d[rel]))


def read_manifest():
    d = {}
    with open(MANIFEST, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            rel, h = line.split("\t")
            d[rel] = h
    return d


def diff(current, baseline):
    modified = sorted(r for r in current if r in baseline and current[r] != baseline[r])
    added = sorted(r for r in current if r not in baseline)
    removed = sorted(r for r in baseline if r not in current)
    return modified, added, removed


def main(argv):
    cmd = argv[1] if len(argv) > 1 else "check"

    if cmd == "update":
        d = compute()
        write_manifest(d)
        print("freeze manifest written: %d frozen files hashed (%s)"
              % (len(d), os.path.relpath(MANIFEST, ROOT).replace(os.sep, "/")))
        return 0

    if cmd == "check":
        if not os.path.exists(MANIFEST):
            print("NO MANIFEST — run `update` first (as part of a sanctioned RCR/CCP).")
            return 2
        current = compute()
        mod, add, rem = diff(current, read_manifest())
        n = len(mod) + len(add) + len(rem)
        if n == 0:
            print("FREEZE OK: %d frozen files, 0 drift." % len(current))
            return 0
        print("FREEZE DRIFT DETECTED (%d) — a frozen file changed:" % n)
        for r in mod:
            print("  MODIFIED %s" % r)
        for r in add:
            print("  ADDED    %s (new frozen file — needs RCR/CCP + `update`)" % r)
        for r in rem:
            print("  REMOVED  %s" % r)
        print("If this change is sanctioned (RCR for runtime/, CCP for standard/), re-run `update` "
              "as part of that instrument; otherwise REVERT it — the freeze forbids silent edits.")
        return 1

    if cmd == "selftest":
        base = compute()
        if not base:
            print("selftest FAIL: no frozen files found")
            return 1
        victim = sorted(base)[0]
        tampered = dict(base)
        tampered[victim] = "0" * 64
        mod, add, rem = diff(tampered, base)
        tamper_detected = (victim in mod) and not add and not rem
        cmod, cadd, crem = diff(base, base)
        clean_clean = not (cmod or cadd or crem)
        print("selftest: tamper-detected=%s (victim=%s) ; clean-run-clean=%s"
              % (tamper_detected, victim, clean_clean))
        ok = tamper_detected and clean_clean
        print("RESULT: %s" % ("GREEN (freeze gate bites and passes clean)" if ok else "FAIL"))
        return 0 if ok else 1

    print("usage: freeze_check.py [update|check|selftest]")
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))

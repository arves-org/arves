#!/usr/bin/env python3
"""
Freeze-diff gate: mechanically detect a SILENT content edit to a frozen file.

The ARVES freeze (runtime/ + standard/) was enforced only by author discipline + a git tag
(runtime-v1.0); nothing mechanically caught a silent edit (verification/evidence/SYSTEM_GAP_ANALYSIS.md).
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
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
FROZEN_ROOTS = ["runtime", "standard"]
EXCLUDE_DIRS = {"target", "__pycache__", ".git", "node_modules", ".pytest_cache"}
MANIFEST = os.path.join(HERE, "freeze_manifest.tsv")


def iter_frozen_files():
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
            out[rel] = hashlib.sha256(f.read()).hexdigest()
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

"""
Combined self-test runner for the ACS-003/004/005 reference validators.

This is the B1 "evidence subset" (verification/evidence/G2_READINESS.md §2a): it proves the
ACS-003 §6.3 envelope, ACS-004 §6.5/§7/§8 instance, and ACS-005 §9.2/§9.3 language REJECT
rules are IMPLEMENTABLE from the Kit spec alone — each with a positive-accept case and one
negative per rule. It is LIVING / freeze-clean: the validators emit free-form descriptive
reasons (NOT stable reason codes) and ship NO negative vectors. Defining stable reject reason
codes + shipping negative vectors in standard/vectors/ is a CCP Amendment per ACS-001 §4.1 —
these validators are the oracle that future CCP's vectors will be checked against.

Run:  python verification/independent/python/acs_validators_selftest.py
"""

import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
MODULES = ["acs003_envelope.py", "acs004_instance.py", "acs005_checker.py"]


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass
    results = []
    for m in MODULES:
        print("=" * 72)
        print("### %s" % m)
        rc = subprocess.run([sys.executable, os.path.join(HERE, m)], cwd=HERE).returncode
        results.append((m, rc))
    print("=" * 72)
    all_ok = all(rc == 0 for _, rc in results)
    for m, rc in results:
        print("  %-24s %s" % (m, "PASS" if rc == 0 else "FAIL"))
    print("-" * 72)
    if all_ok:
        print("  ACS-003/004/005 reject rules: PROVEN IMPLEMENTABLE from the Kit spec (B1 evidence).")
        print("  (Stable reason codes + shipped negative vectors remain a CCP — ACS-001 §4.1.)")
    else:
        print("  FAILURES PRESENT — see per-module output above.")
    return 0 if all_ok else 1


if __name__ == "__main__":
    sys.exit(main())

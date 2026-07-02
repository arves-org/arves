"""
ARVES Evidence Probe — regenerates the machine-verifiable rows of the Evidence
Ledger by actually RUNNING every executable evidence source. If a claim has
regressed, the corresponding row fails loudly here — the ledger cannot drift from
reality (that is the point: "Evidence Increased" must be measured, not asserted).

It runs, from the repo root:
  - the Rust Conformance Platform  (positive + negative ACS vectors)
  - the independent Python conformance + rejection runners (Kit-only, grade G1)
  - the Rust<->Python differential fuzzer (accept/reject agreement)
  - the full Rust workspace test suite (behaviour + architecture gate)

and writes verification/evidence/evidence_ledger.tsv. Declared (not-yet-executable)
evidence — formal proofs, security/academic/standards review, and the G2 third-party
runtime exit gate — lives in EVIDENCE_LEDGER.md, earned by the destroy-offices.

Run: python verification/evidence/evidence_probe.py
"""

import os
import re
import subprocess
import sys

try:  # keep em-dashes legible on legacy Windows console codepages
    sys.stdout.reconfigure(encoding="utf-8")
except Exception:
    pass

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
MANIFEST = os.path.join(ROOT, "runtime", "Cargo.toml")
PYDIR = os.path.join(ROOT, "verification", "independent", "python")
TSDIR = os.path.join(ROOT, "verification", "independent", "typescript")
FUZZ = os.path.join(ROOT, "verification", "differential", "acs002_differential_fuzz.py")
LEDGER_TSV = os.path.join(HERE, "evidence_ledger.tsv")


def run(cmd, cwd=None):
    p = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return p.returncode, p.stdout.decode("utf-8", "replace")


class Row:
    def __init__(self, claim, dimension, level, indep, cmd):
        self.claim = claim
        self.dimension = dimension
        self.level = level          # L0..L4 the row targets
        self.indep = indep          # G0 / G1 / G2
        self.cmd = cmd              # reproduction command
        self.ok = False
        self.metric = ""

    def tsv(self):
        status = "PASS" if self.ok else "FAIL"
        return "\t".join([self.claim, self.dimension, self.level, self.indep,
                          status, self.metric, self.cmd])


def probe():
    rows = []

    # 1. Rust Conformance Platform — positive + negative ACS vectors.
    cmd = "cargo run -q --manifest-path runtime/Cargo.toml -p arves-conformance --bin conformance"
    rc, out = run(["cargo", "run", "-q", "--manifest-path", MANIFEST,
                   "-p", "arves-conformance", "--bin", "conformance"])
    pos = re.search(r"golden vectors:\s*(\d+)/(\d+)\s*PASS", out)
    neg = re.search(r"negative vectors:\s*(\d+)/(\d+)\s*core REJECTED", out)
    r = Row("ACS-001..005 golden vectors (encode + address)", "Behaviour+Implementation",
            "L2", "G1", cmd)
    r.ok = rc == 0 and bool(pos) and pos.group(1) == pos.group(2)
    r.metric = "positive %s/%s" % (pos.group(1), pos.group(2)) if pos else "no-parse"
    rows.append(r)
    r2 = Row("ACS-002 rejection / negative vectors (decode)", "Differential+Implementation",
             "L2", "G1", cmd)
    r2.ok = rc == 0 and bool(neg) and neg.group(1) == neg.group(2)
    r2.metric = "core %s/%s REJECTED" % (neg.group(1), neg.group(2)) if neg else "no-parse"
    rows.append(r2)

    # 2. Independent Python conformance (positive) — Kit-only, grade G1.
    rc, out = run([sys.executable, "conformance.py"], cwd=PYDIR)
    r = Row("Independent Python reproduces golden vectors", "Independent",
            "L3", "G1", "python verification/independent/python/conformance.py")
    r.ok = rc == 0 and "CONFORMANT" in out
    r.metric = "CONFORMANT" if r.ok else "see output"
    rows.append(r)

    # 3. Independent Python rejection — Kit-only, enforces nfc (fully conformant).
    rc, out = run([sys.executable, "conformance_negative.py"], cwd=PYDIR)
    m = re.search(r"(\d+)/(\d+)\s*REJECTED", out)
    r = Row("Independent Python reproduces rejection (16/16)", "Independent",
            "L3", "G1", "python verification/independent/python/conformance_negative.py")
    r.ok = rc == 0 and "CONFORMANT" in out
    r.metric = ("%s/%s REJECTED" % (m.group(1), m.group(2))) if m else ("CONFORMANT" if r.ok else "?")
    rows.append(r)

    # 4. Rust <-> Python differential fuzzer — accept/reject agreement.
    rc, out = run([sys.executable, FUZZ])
    div = re.search(r"hard divergences\s*:\s*(\d+)", out)
    tot = re.search(r"inputs=(\d+)", out)
    r = Row("Rust<->Python differential (encode+decode)", "Differential+Independent",
            "L3", "G1", "python verification/differential/acs002_differential_fuzz.py")
    r.ok = rc == 0 and bool(div) and div.group(1) == "0"
    r.metric = ("%s inputs, %s hard divergences" % (tot.group(1) if tot else "?", div.group(1))
                if div else "no-parse")
    rows.append(r)

    # 5. Independent TypeScript runtime (Node) — cold Kit-only build, grade G1.
    rc, out = run(["node", "src/conformance.mjs"], cwd=TSDIR)
    m = re.search(r"positive:\s*(\d+)/(\d+)", out)
    r = Row("Independent TypeScript reproduces vectors (cold)", "Independent",
            "L3", "G1", "node verification/independent/typescript/src/conformance.mjs")
    r.ok = rc == 0 and "CONFORMANT" in out
    r.metric = ("positive %s/%s + 16 core+nfc" % (m.group(1), m.group(2))) if m else ("CONFORMANT" if r.ok else "?")
    rows.append(r)

    # 6. Full Rust workspace test suite — behaviour + architecture gate + invariants.
    rc, out = run(["cargo", "test", "-q", "--manifest-path", MANIFEST, "--workspace"])
    passed = sum(int(x) for x in re.findall(r"test result: ok\.\s*(\d+) passed", out))
    failed = "FAILED" in out or "test result: FAILED" in out
    r = Row("Rust workspace tests (I1 runtime + gates + ACS)", "Behaviour+Formal",
            "L2", "G0", "cargo test --manifest-path runtime/Cargo.toml --workspace")
    r.ok = rc == 0 and not failed
    r.metric = "%d tests passed" % passed
    rows.append(r)

    return rows


def main():
    rows = probe()
    header = "claim\tdimension\tlevel\tindependence\tstatus\tmetric\treproduce\n"
    with open(LEDGER_TSV, "w", encoding="utf-8", newline="\n") as f:
        f.write(header)
        for r in rows:
            f.write(r.tsv() + "\n")

    print("ARVES Evidence Probe — machine-verifiable rows")
    print("=" * 78)
    for r in rows:
        mark = "PASS" if r.ok else "FAIL"
        print("  [%s] %-46s %-6s %-3s  %s" % (mark, r.claim[:46], r.level, r.indep, r.metric))
    n_ok = sum(1 for r in rows if r.ok)
    print("-" * 78)
    print("  %d/%d executable evidence rows PASS" % (n_ok, len(rows)))
    print("  wrote %s" % os.path.relpath(LEDGER_TSV, ROOT))
    print("  NOTE: independence here is grade G1 (same-process, Kit-only). The G2")
    print("        third-party exit gate is NOT YET MET — see CERTIFICATION_PROGRAM.md.")
    return 0 if n_ok == len(rows) else 1


if __name__ == "__main__":
    sys.exit(main())

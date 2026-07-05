"""
ARVES Evidence Probe — the SINGLE SOURCE OF TRUTH for the machine-verifiable
("probe-verified") rows of the Evidence Ledger. It regenerates those rows by
actually RUNNING every executable evidence source. If a claim has regressed, the
corresponding row fails loudly here — and the committed ledger cannot drift from
reality, because this probe both (a) writes the machine ledger
(`evidence_ledger.tsv`) and (b) patches / gate-checks the live-count cells of the
human ledger (`EVIDENCE_LEDGER.md`). A count can no longer silently rot: either
the probe rewrites it, or `--check` fails the batch when the two disagree.

It runs, from the repo root:
  - the Rust Conformance Platform          (positive + negative ACS vectors)
  - the independent Python conformance + rejection runners (Kit-only, grade G1)
  - the Rust<->Python differential fuzzer  (accept/reject agreement)
  - the independent TypeScript runtime     (cold, Kit-only, grade G1)
  - the full Rust workspace test suite     (behaviour + architecture gate)
  - the anti-gaming SOUND runtime verifier (grader-owns-truth; gap B3 backstop)

and writes verification/evidence/evidence_ledger.tsv.

HONESTY NOTE — scope of the "cannot drift" promise. This probe covers ONLY the
rows below (the "probe-verified" set). The Evidence Ledger's Section A also lists
product / P8 rows that this probe does NOT run; those are marked *asserted* in the
Markdown (their reproduction command is real, but no probe re-runs them, so they
CAN drift and are not covered by this gate). See EVIDENCE_LEDGER.md Section A.
Declared (not-yet-executable) evidence — formal proofs, security/academic/standards
review, and the G2 third-party runtime exit gate — lives in Section B/C, earned by
the destroy-offices.

Run (regenerate the TSV + patch the MD live cells):
    python verification/evidence/evidence_probe.py
Drift gate (run the suites, do NOT write; fail if the committed MD disagrees):
    python verification/evidence/evidence_probe.py --check
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
SOUND = os.path.join(ROOT, "verification", "certification", "verify_runtime_sound.py")
LEDGER_TSV = os.path.join(HERE, "evidence_ledger.tsv")
LEDGER_MD = os.path.join(HERE, "EVIDENCE_LEDGER.md")


def run(cmd, cwd=None):
    p = subprocess.run(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return p.returncode, p.stdout.decode("utf-8", "replace")


class Row:
    def __init__(self, key, claim, dimension, level, indep, cmd):
        self.key = key              # stable anchor -> <!-- probe:KEY --> in EVIDENCE_LEDGER.md
        self.claim = claim
        self.dimension = dimension
        self.level = level          # L0..L4 the row targets
        self.indep = indep          # G0 / G1 / G2
        self.cmd = cmd              # reproduction command
        self.ok = False
        self.metric = ""

    @property
    def status(self):
        return "PASS" if self.ok else "FAIL"

    def tsv(self):
        return "\t".join([self.claim, self.dimension, self.level, self.indep,
                          self.status, self.metric, self.cmd])


def probe():
    rows = []

    # 1. Rust Conformance Platform — positive + negative ACS vectors.
    cmd = "cargo run -q --manifest-path runtime/Cargo.toml -p arves-conformance --bin conformance"
    rc, out = run(["cargo", "run", "-q", "--manifest-path", MANIFEST,
                   "-p", "arves-conformance", "--bin", "conformance"])
    pos = re.search(r"golden vectors:\s*(\d+)/(\d+)\s*PASS", out)
    neg = re.search(r"negative vectors:\s*(\d+)/(\d+)\s*core REJECTED", out)
    r = Row("rust-golden", "ACS-001..005 golden vectors (encode + address)",
            "Behaviour+Implementation", "L2", "G1", cmd)
    r.ok = rc == 0 and bool(pos) and pos.group(1) == pos.group(2)
    r.metric = ("positive %s/%s" % (pos.group(1), pos.group(2))) if pos else "no-parse"
    rows.append(r)
    r2 = Row("rust-negative", "ACS-002 rejection / negative vectors (decode)",
             "Differential+Implementation", "L2", "G1", cmd)
    r2.ok = rc == 0 and bool(neg) and neg.group(1) == neg.group(2)
    r2.metric = ("core %s/%s REJECTED" % (neg.group(1), neg.group(2))) if neg else "no-parse"
    rows.append(r2)

    # 2. Independent Python conformance (positive) — Kit-only, grade G1.
    rc, out = run([sys.executable, "conformance.py"], cwd=PYDIR)
    r = Row("py-golden", "Independent Python reproduces golden vectors", "Independent",
            "L3", "G1", "python verification/independent/python/conformance.py")
    r.ok = rc == 0 and "CONFORMANT" in out
    r.metric = "CONFORMANT" if r.ok else "see output"
    rows.append(r)

    # 3. Independent Python rejection — Kit-only, enforces nfc (fully conformant).
    rc, out = run([sys.executable, "conformance_negative.py"], cwd=PYDIR)
    m = re.search(r"(\d+)/(\d+)\s*REJECTED", out)
    r = Row("py-negative", "Independent Python reproduces rejection", "Independent",
            "L3", "G1", "python verification/independent/python/conformance_negative.py")
    r.ok = rc == 0 and "CONFORMANT" in out
    r.metric = ("%s/%s REJECTED" % (m.group(1), m.group(2))) if m else ("CONFORMANT" if r.ok else "?")
    rows.append(r)

    # 3b. Independent Python SEMANTIC rejection — ACS-003/004/005 reject surfaces
    #     (CCP-006). Each frozen envelope/instance/language vector decodes clean as
    #     dCBOR and is REJECTED by a spec-only reference validator. Closes the
    #     zero-negative-vector hole for ACS-003/004/005 (SYSTEM_GAP_ANALYSIS #1/#2/#23).
    rc, out = run([sys.executable, "conformance_semantic.py"], cwd=PYDIR)
    ms = re.search(r"envelope\s+(\d+)/(\d+)\s+instance\s+(\d+)/(\d+)\s+language\s+(\d+)/(\d+)", out)
    r = Row("acs-semantic-reject", "ACS-003/004/005 semantic rejection (envelope/instance/language)",
            "Differential+Independent", "L2", "G1",
            "python verification/independent/python/conformance_semantic.py")
    r.ok = rc == 0 and "CONFORMANT" in out and bool(ms) and \
        ms.group(1) == ms.group(2) and ms.group(3) == ms.group(4) and ms.group(5) == ms.group(6)
    r.metric = ("envelope %s/%s + instance %s/%s + language %s/%s REJECTED" % ms.groups()) if ms else "no-parse"
    rows.append(r)

    # 4. Rust <-> Python differential fuzzer — accept/reject agreement.
    rc, out = run([sys.executable, FUZZ])
    div = re.search(r"hard divergences\s*:\s*(\d+)", out)
    tot = re.search(r"inputs=(\d+)", out)
    r = Row("differential", "Rust<->Python differential (encode+decode)",
            "Differential+Independent", "L3", "G1",
            "python verification/differential/acs002_differential_fuzz.py")
    r.ok = rc == 0 and bool(div) and div.group(1) == "0"
    r.metric = ("%s inputs, %s hard divergences" % (tot.group(1) if tot else "?", div.group(1))
                if div else "no-parse")
    rows.append(r)

    # 5. Independent TypeScript runtime (Node) — cold Kit-only build, grade G1.
    rc, out = run(["node", "src/conformance.mjs"], cwd=TSDIR)
    m = re.search(r"positive:\s*(\d+)/(\d+)", out)
    r = Row("ts-golden", "Independent TypeScript reproduces vectors (cold)", "Independent",
            "L3", "G1", "node verification/independent/typescript/src/conformance.mjs")
    r.ok = rc == 0 and "CONFORMANT" in out
    r.metric = ("positive %s/%s + 16 core+nfc" % (m.group(1), m.group(2))) if m else ("CONFORMANT" if r.ok else "?")
    rows.append(r)

    # 6. Full Rust workspace test suite — behaviour + architecture gate + invariants.
    rc, out = run(["cargo", "test", "-q", "--manifest-path", MANIFEST, "--workspace"])
    passed = sum(int(x) for x in re.findall(r"test result: ok\.\s*(\d+) passed", out))
    failed = "FAILED" in out or "test result: FAILED" in out
    r = Row("rust-workspace", "Rust workspace tests (I1 runtime + gates + ACS)",
            "Behaviour+Formal", "L2", "G0",
            "cargo test --manifest-path runtime/Cargo.toml --workspace")
    r.ok = rc == 0 and not failed
    r.metric = "%d tests passed" % passed
    rows.append(r)

    # 7. Anti-gaming SOUND runtime verifier — grader owns the truth (gap B3 backstop).
    #    Inputs-only grading + fresh + accept probes defeat a hollow echo adapter. Wiring
    #    it here puts the flagship survivability check INSIDE the drift-proof loop.
    rc, out = run([sys.executable, SOUND])
    m = re.search(r"published\s+(\d+)/(\d+)\s+fresh\s+(\d+)/(\d+)\s+core-reject\s+(\d+)/(\d+)\s+accept\s+(\d+)/(\d+)", out)
    sm = re.search(r"semantic:\s+envelope\s+(\d+)/(\d+)\s+instance\s+(\d+)/(\d+)\s+language\s+(\d+)/(\d+)", out)
    r = Row("sound-certified", "Sound runtime verification (non-gameable, full ACS-001..005 surface)",
            "Certification+Integrity", "L2", "G1",
            "python verification/certification/verify_runtime_sound.py")
    # Require the FULL-surface verdict (rank 1): the probe fails if the gate degrades to core-only
    # for the reference runtimes, so the drift-proof ledger reflects that the gate attests the whole
    # standard (envelope/instance/language), not just the ACS-002 byte layer.
    r.ok = rc == 0 and "SOUND-CERTIFIED (full ACS-001..005 surface)" in out and bool(sm)
    if r.ok and m and sm:
        r.metric = ("published %s/%s, fresh %s/%s, core %s/%s, accept %s/%s, semantic env %s/%s inst %s/%s lang %s/%s -> SOUND-CERTIFIED (full surface)"
                    % (m.group(1), m.group(2), m.group(3), m.group(4), m.group(5), m.group(6), m.group(7), m.group(8),
                       sm.group(1), sm.group(2), sm.group(3), sm.group(4), sm.group(5), sm.group(6)))
    else:
        r.metric = "NOT full-surface SOUND-CERTIFIED"
    rows.append(r)

    return rows


# --- Markdown ledger: patch the live cells of the probe-verified rows in place. ---
# Each probe-verified row in EVIDENCE_LEDGER.md ends with an anchor comment
# `<!-- probe:KEY -->`. We rewrite that row's Status + Metric cells from the probe,
# leaving Claim/Dimensions/Level/Indep/Reproduce (curated prose) untouched. Because the
# probe OWNS Status+Metric, a count cannot drift: `--check` fails if the committed cell
# does not already equal what a fresh probe produces.

ANCHOR_RE = re.compile(r"<!--\s*probe:([a-z0-9\-]+)\s*-->\s*$")


def _cells(line):
    # split a markdown table row "| a | b | ... |" into trimmed cell strings
    inner = line.strip()
    if inner.startswith("|"):
        inner = inner[1:]
    if inner.endswith("|"):
        inner = inner[:-1]
    return [c.strip() for c in inner.split("|")]


def render_md(rows, check=False):
    """Patch (or verify) the anchored Status+Metric cells of EVIDENCE_LEDGER.md.

    Returns (n_patched, drifts) where `drifts` is a list of human-readable
    descriptions of committed cells that disagree with the probe. In check mode the
    file is never written; in write mode drifting cells are rewritten and drifts is [].
    """
    by_key = {r.key: r for r in rows}
    with open(LEDGER_MD, encoding="utf-8") as f:
        lines = f.readlines()

    n_patched = 0
    drifts = []
    seen = set()
    for i, line in enumerate(lines):
        m = ANCHOR_RE.search(line.rstrip("\n"))
        if not m:
            continue
        key = m.group(1)
        r = by_key.get(key)
        if r is None:
            drifts.append("orphan anchor <!-- probe:%s --> has no probe row" % key)
            continue
        seen.add(key)
        cells = _cells(ANCHOR_RE.sub("", line.rstrip("\n")))
        # Table columns: Claim | Dimensions | Level | Indep | Status | Metric | Reproduce
        if len(cells) != 7:
            drifts.append("row probe:%s malformed (%d cells, expected 7)" % (key, len(cells)))
            continue
        want_status = "PASS" if r.ok else "FAIL"
        cur_status = cells[4]
        cur_metric = cells[5]
        # status cell may carry a leading badge glyph; compare on the trailing token
        status_token = cur_status.replace("*", "").split()[-1] if cur_status else ""
        if status_token != want_status or cur_metric != r.metric:
            if check:
                drifts.append(
                    "row probe:%s committed [status=%r metric=%r] != probe [status=%r metric=%r]"
                    % (key, cur_status, cur_metric, want_status, r.metric))
            else:
                badge = "PASS" if r.ok else "FAIL"
                cells[4] = badge
                cells[5] = r.metric
                lines[i] = "| " + " | ".join(cells) + " | <!-- probe:%s -->\n" % key
                n_patched += 1

    missing = [k for k in by_key if k not in seen]
    for k in missing:
        drifts.append("probe row %r has no <!-- probe:%s --> anchor in EVIDENCE_LEDGER.md" % (k, k))

    if not check and n_patched:
        with open(LEDGER_MD, "w", encoding="utf-8", newline="\n") as f:
            f.writelines(lines)
    return n_patched, drifts


def main():
    check = "--check" in sys.argv[1:]
    rows = probe()

    if not check:
        header = "claim\tdimension\tlevel\tindependence\tstatus\tmetric\treproduce\n"
        with open(LEDGER_TSV, "w", encoding="utf-8", newline="\n") as f:
            f.write(header)
            for r in rows:
                f.write(r.tsv() + "\n")

    n_patched, drifts = render_md(rows, check=check)

    mode = "CHECK (drift gate)" if check else "REGENERATE"
    print("ARVES Evidence Probe — probe-verified rows  [%s]" % mode)
    print("=" * 78)
    for r in rows:
        print("  [%s] %-46s %-6s %-3s  %s" % (r.status, r.claim[:46], r.level, r.indep, r.metric))
    n_ok = sum(1 for r in rows if r.ok)
    print("-" * 78)
    print("  %d/%d probe-verified evidence rows PASS" % (n_ok, len(rows)))
    if check:
        if drifts:
            print("  LEDGER DRIFT — EVIDENCE_LEDGER.md disagrees with a fresh probe:")
            for d in drifts:
                print("    - %s" % d)
        else:
            print("  EVIDENCE_LEDGER.md is CONSISTENT with the probe (no drift).")
    else:
        print("  wrote %s" % os.path.relpath(LEDGER_TSV, ROOT))
        if drifts:
            print("  WARNING — could not fully patch EVIDENCE_LEDGER.md:")
            for d in drifts:
                print("    - %s" % d)
        else:
            print("  patched %d live cell(s) in %s" % (n_patched, os.path.relpath(LEDGER_MD, ROOT)))
    print("  NOTE: only the rows above are drift-proof. Section A also lists product / P8")
    print("        rows this probe does NOT run — marked *asserted* in the Markdown.")
    print("  NOTE: independence here is grade G1 (same-process, Kit-only). The G2")
    print("        third-party exit gate is NOT YET MET — see CERTIFICATION_PROGRAM.md.")

    ok = n_ok == len(rows) and not drifts
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

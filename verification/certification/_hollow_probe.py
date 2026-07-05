"""
Test-only hollow runtime — a line-protocol program that does NO real work.

Used by test_harness_integrity.py to prove certify_your_runtime.py's vendor line-protocol path
inherits the sound grader's non-gameability: a hollow runtime driven through the same batching /
keying / parsing MUST be NOT SOUND-CERTIFIED (it cannot address the grader's FRESH probes and
cannot ACCEPT a valid body). It emits a fixed, deliberately wrong answer per stdin line.

Not part of any certification path; it exists only so the negative case is executable and portable
(no fragile inline-shell quoting). Usage: `python _hollow_probe.py {addr|decode|validate}`.
"""

import sys

mode = sys.argv[1] if len(sys.argv) > 1 else "addr"
for _line in sys.stdin:
    if mode == "addr":
        sys.stdout.write("12" + "00" * 33 + "\n")   # well-formed 34-byte prefix, never the real CID
    elif mode == "validate":
        sys.stdout.write("REJECT\treserved-or-unsupported\n")  # reject-everything at the semantic tier
    else:
        sys.stdout.write("REJECT\tnon-shortest-int\n")         # reject-everything at the byte tier

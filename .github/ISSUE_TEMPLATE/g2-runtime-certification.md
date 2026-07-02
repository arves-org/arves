---
name: G2 Runtime Certification
about: Submit an independent (G2) ARVES runtime you built from the Standard Kit alone, with no help from the authors.
title: "[G2] <your runtime name> — independent runtime certification"
labels: ["g2", "certification"]
---

<!--
The G2 challenge (see CHALLENGE.md): you built a conformant ARVES runtime from standard/ alone,
with NO contact with the authors, and it passes the certification + soundness gates.
Independence is graded honestly — an assisted build is recorded as G1, not G2. The ambiguity
list at the end is as valuable as the pass; it hardens the Standard.
-->

## Runtime
- **Name / vendor:**
- **Language / stack:**
- **Public source link:**

## Independence declaration (required for a G2 grade)
- [ ] I built this from `standard/` (the Kit) + `verification/certification/` **only**.
- [ ] I did **not** read `runtime/` (the reference implementation).
- [ ] I had **no contact** with the ARVES authors during the build — no questions, no hints.
- [ ] I confirm this is an independent party, not the ARVES maintainers.

## Evidence (paste verbatim output)

**`python verification/certification/certify_runtime.py`** (with your runtime added as a record):
```
<paste>
```

**`python verification/certification/verify_runtime_sound.py`** (the non-gameable gate — the one that matters):
```
<paste>
```

## Kit ambiguities you hit (as valuable as the pass)
List every point where `standard/` was silent, ambiguous, or forced a guess (file + section).
Each becomes a candidate CCP Amendment.

1.
2.

## Anything else
Notes, environment, how long it took, what was hardest.

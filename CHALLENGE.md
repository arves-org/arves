# The ARVES G2 Challenge — certify a runtime we never helped you build

ARVES has one open exit gate. Everything else is done; this is the thing that is not, and
*by design* we cannot do it for ourselves.

## The claim we are trying to falsify

> **Anyone can build a conformant ARVES runtime and certify it from the published Standard
> alone — with zero contact with the authors.**

Independence is graded honestly:

- **G1 (done):** a runtime built inside this program / by the same team. The repo has two
  (Rust + Python), certified under one conformance. Real signal — but not proof.
- **G2 (open — this is the challenge):** a **genuinely unrelated party**, using **only the
  files under `standard/`** (plus the certification harness under `verification/`), builds a
  runtime that reproduces every golden vector and rejects every core negative, and certifies
  it **with no questions asked of us.**

If you do that, you are the evidence ARVES has been waiting for. Until someone does, the
project's headline independence claim stays capped at **G1**, and we say so everywhere.

## How to take it

1. **Read the cold-start packet:** [IMPLEMENTING_ARVES.md](IMPLEMENTING_ARVES.md). It routes
   you through `standard/` in order and gives the exact conformance checklist. Do **not** read
   our reference runtime under `runtime/` — a real G2 attempt builds from the spec, not our code.
2. **Build**, in any language, two things: the ACS-001 addresser and the ACS-002 canonical
   serializer/decoder (then ACS-003/004 as dCBOR maps). No network, no keys, offline.
3. **Self-check** against the frozen vectors with your own recompute-everything runner (mirror
   `verification/independent/reference-runner/run.mjs`).
4. **Certify:** the copy-paste last mile is
   [`verification/certification/CERTIFY_YOUR_RUNTIME.md`](verification/certification/CERTIFY_YOUR_RUNTIME.md).
   The shortest path: expose three tiny stdin/stdout programs (address / decode / validate) and run
   `python verification/certification/certify_your_runtime.py --addr … --decode … --validate …`
   (add `--self-test` first to see the reference bins pass through the identical vendor path). It
   grades you through the **non-gameable** `verify_runtime_sound.py` (grader owns the truth + fresh
   probes — the one that actually proves you did the work). No Python? Wire an adapter instead —
   both contracts are in that page.
5. **Win condition:** `SOUND-CERTIFIED (full ACS-001..005 surface)` from the Kit alone, with no help
   from us.

## The rules (they are the point, not red tape)

- **We will not help you during the attempt.** Questions, hints, and clarifications would
  collapse G2 back to G1. If the Kit is ambiguous, that ambiguity *is* a finding — record it.
- **No reading `runtime/`.** Build from the normative spec text; if you needed our code, the
  Kit failed and we want to know where.
- **We grade honestly.** A runtime built with our help is recorded as G1, not G2. We would
  rather report "still G1" truthfully than launder an assisted result.

## How to submit

Open a **G2 Runtime Certification** issue (template:
`.github/ISSUE_TEMPLATE/g2-runtime-certification.md`) with: your runtime + language, a link to
its source, the verbatim `certify_runtime.py` **and** `verify_runtime_sound.py` output, a
confirmation you had no contact with the authors, and every point where the Kit was ambiguous
or forced a guess. The ambiguity list is as valuable as the pass — it hardens the Standard for
the next party.

## What a genuine G2 pass earns

- Recorded as the **first external member of the Independent Runtime Alliance**.
- A **G2** row in `verification/evidence/EVIDENCE_LEDGER.md` — the first time ARVES independence
  is reported above G1.
- Every ambiguity you hit becomes a CCP Amendment, making the Standard stronger — which is the
  whole reason the Foundation exists: *to make this event possible without us.*

*Honest status: this challenge is open and unmet. Nothing in this repository claims G2 has been
reached — reaching it is your part.*

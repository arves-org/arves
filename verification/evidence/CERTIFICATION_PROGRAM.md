# ARVES Certification Program — the Evidence OS

**Status:** living-repo (ED-001); the operating framework of the **Standard Validation
Era**. It defines how any ARVES claim earns (and keeps) an **Evidence Level**, and how a
runtime becomes **certified**. It invents no architecture; it grades evidence.

> **Era philosophy.** You are no longer building ARVES. You are trying to prove ARVES
> wrong. If you fail, only then ARVES becomes stronger. The KPI is **Evidence
> Increased**, not "Tests Passed." Nothing is ever "Done"; it only earns a level.

---

## 1. The evidence tiers (what a claim can earn)

Five tiers, cumulative — each assumes the ones below it. The tiers map onto the
L0..L4 shorthand used elsewhere.

| Tier | Level | The claim is… | Earned when |
|------|-------|---------------|-------------|
| **Scientific** | L0 | *specified & sound* — defined in the frozen corpus or a ratified ACS, internally consistent, assumptions stated | spec text exists AND survives scientific review (no unproven load-bearing assumption left implicit) |
| **Engineering** | L1 | *implemented & verified* — reference code produces the behaviour and gates hold | unit + behaviour + architecture-gate tests pass |
| **Implementation** | L2 | *conformance-proven* — passes the executable conformance + property + negative + differential-fuzz suites | Conformance Platform PASS incl. rejection vectors; property/fuzz clean |
| **Independent** | L3 | *reproduced by another implementation* | ≥1 other implementation reproduces it byte-for-byte / behaviourally (see independence grades) |
| **Industrial** | L4 | *certified & operable at scale* — third-party certified; fault/replay/perf evidence under load | Certification granted to a **third-party** runtime; fault-injection + replay + performance evidence |

A property's Evidence Level is the **highest tier for which every lower tier also
holds**. A gap at any lower tier caps the level.

## 2. Independence grades (the honesty gate — never launder this)

"Independent" is not binary. Every Independent-tier claim MUST carry its grade:

| Grade | Meaning | Counts as… |
|-------|---------|-----------|
| **G0 self** | verified only by the reference / same author & context | NOT independent |
| **G1 same-process independent** | a fresh-context implementation, **Kit-only**, reference source forbidden, but produced inside this program | partial — real signal, not proof |
| **G2 third-party independent** | a different team/process with **no help and no reference access**, e.g. a stranger from a public download | the real thing |

The current ACS Python implementation is **G1** — honest, valuable, but not G2. The
Era-3 **exit gate** is a **G2** result:

> A stranger downloads ONLY the Standard Kit from a public source, implements a
> runtime, and PASSES certification — *you did not help.*

Until a G2 result exists, no ARVES claim may be reported as fully "independent."

## 3. Evidence OS — the dimensions of evidence

Conformance is only one kind of evidence. The Evidence OS tracks ten dimensions; a
mature property accumulates evidence across many of them:

`Behaviour · Differential · Formal · Performance · Fault · Replay · Independent ·
Security · Certification · Academic`

Each dimension is produced by a **destroy-office** whose job is to *fail* the claim:
Scientific Review (sound?), Security (break it), Performance/Robustness (scale it),
Academic (would SOSP/OSDI/PLDI accept it?), Standards (would an IETF WG approve it?),
Independent Runtime (can another team build it?). A dimension with no adversarial
attempt against it is graded `none`, not `strong`.

## 4. The Evidence Ledger

The single source of truth for "where does the evidence stand." One row per claim:

`claim · dimension(s) · evidence level (L0..L4) · verifier · reproduction command ·
independence grade · status · artifacts`

- **Machine-verifiable rows** (Behaviour/Differential/Implementation) are regenerated
  by `evidence_probe.py`, which actually runs the suites — so those rows cannot drift
  from reality (they fail loudly if the claim regresses).
- **Declared rows** (Formal/Academic/Security/third-party Independent) cite the
  artifact and status; they are earned by the destroy-offices, not asserted.

The ledger lives at `EVIDENCE_LEDGER.md` (human) + `evidence_ledger.tsv` (machine).
"Evidence Increased" is measured as: rows advancing a tier, dimensions moving off
`none`, and independence grades moving G0→G1→G2.

## 5. Runtime certification (the exit gate mechanics)

A runtime (not a single claim) is certified against the Kit:

1. **L1 Core Runtime** — passes ACS conformance (positive + core negative vectors).
2. **L2 Interop** — differential-identical to another implementation on the golden +
   fuzz corpus.
3. **L3 Independent** — the submitting implementation is G1 or better.
4. **L4 Certified Product** — G2 third-party, plus fault/replay/performance evidence.

A third party self-certifies with **only the Kit**: run the conformance procedure
(`standard/conformance/CONFORMANCE.md`), produce an ARVES Conformance Report, and
(for L2) exchange golden/fuzz corpora with a peer to show byte-identical verdicts. No
reference source is required or permitted as input.

## 6. How this era runs

Each validation round: the destroy-offices attack → confirmed findings are fixed or
recorded as Standard Defects → the Evidence Ledger is updated → the round's net effect
on evidence is reported. Repeat until the exit gate (G2 third-party PASS) is met, then
Era 4 (Industrialization: I2–I6, Kernel Integration) may begin — on validated ground.

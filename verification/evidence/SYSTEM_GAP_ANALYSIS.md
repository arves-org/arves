# ARVES — System Gap Analysis (Honest, Author-Run)

> **Run by:** the maintainers (Chief Architect), on 2026-07-02.
> **Grade of this analysis:** **G1 (self / same-repo).** This is an *internal* audit. It does
> **not** advance independence to G2, it does **not** change any frozen artifact, and closing
> every gap below still does **not** manufacture a G2 event. See §6.
> **Governance:** freeze discipline (CLAUDE.md rule #1, RUNTIME_FREEZE_v1.0.md, FOUNDATION.md) is
> in force. `standard/` and `runtime/` change **only** via CCP / Amendment / IDR / RCR. Everything
> marked *freeze-clean* below touches only living surfaces (`verification/`, `products/`, `tools/`,
> root docs) and needs no maintainer freeze sign-off.

---

## 1. State of the system — honest one-paragraph

**What is solid:** the I1 runtime core is real and green — `FileKernel`/WAL/recovery/checkpoint
compile and pass **71/71** workspace tests; LAYER-001 and OWN-001 are executably gated over the
real Cargo graph; ORCH-003 (replay/recovery) and ORCH-004 (idempotent commit) have genuine biting
Kernel tests; the ACS-002 canonical-serialization layer is byte-pinned, differentially fuzzed
across Rust + Python, and has a working maintainer-independent certify harness. **What is not
solid:** independence is still only **G1** — the two "independent" runtimes (Rust + Python) are
both authored and hosted in this repo; the exit gate remains a **G2 external party** that has not
happened. The falsifiable-differential-conformance thesis is proven **only for ACS-002**: the
"16/16 core-reject" gate is 100% ACS-002, so ACS-001/003/004/005 have **zero negative vectors** and
their reject rules are never gate-tested (ACS-003, the 429-line Canonical Envelope, is entirely
un-exercised negatively). The Kernel's sole commit gateway trusts a **caller-supplied ContentId**
(address integrity lives only in the optional bridge). The **products** claim the "real reference
Kernel" but `class Arves` is an in-memory `Map` with no WAL/replay/recovery — so product
persistence/audit claims are not backed by the frozen runtime they cite. And a cluster of living
evidence/attestation docs have **drifted from reality** (stale test counts, a superseded L1
attestation broadcasting already-fixed blockers, an Evidence Ledger whose "cannot drift" promise is
falsified). None of the drift breaks running code; all of it corrodes the era's one KPI —
*Evidence Increased* — which is exactly why it is worth closing.

---

## 2. Ranked confirmed gaps

Ranked by severity, then by leverage on the era KPI (evidence integrity / G2 readiness).
Arm: **STD** = Standard, **RT** = Runtime, **VER** = Verification, **PRD** = Product,
**GOV** = Governance/Docs.

| # | Gap | Arm | Sev | Instrument | Freeze-clean? | Effort |
|---|-----|-----|-----|-----------|:-------------:|:------:|
| 1 | **Negative-vector corpus is 100% ACS-002** — ACS-001/003/004/005 reject rules never gate-tested; "16/16 core-reject" over-covers | STD | major | CCP | no | M |
| 2 | ACS-003 Canonical Envelope (429 lines) has golden vector but **zero negative vectors**; validator self-declares "oracle for a future CCP" | STD | major | CCP | no | M |
| 3 | Kernel commit **trusts caller-supplied ContentId**; address integrity enforced only in bridge; no content≠hash(payload) reject vector | RT | major | RCR | no | M |
| 4 | Products commit to an **in-memory `Map`, not the frozen Rust Kernel/WAL**, while claiming "REAL reference Kernel" (no persistence/replay/recovery) | PRD | major | living_fix + doc_fix | yes | M |
| 5 | Today-dated **L1 attestation broadcasts already-fixed blockers** (0x06/0x07 collision, DRAFT status, "no ContentId asserted") + superseded CONDITIONAL verdict; cited as authoritative, no SUPERSEDED banner | VER | major | doc_fix | yes | S |
| 6 | **Evidence Ledger "cannot drift" promise falsified** — Section A header covers 17 rows but probe runs only 7; P8 cert row + 10 product rows unprobed | VER | major | tooling_fix | yes | S |
| 7 | **Sound verifier (`verify_runtime_sound.py`) grades only Python** — the load-bearing Rust reference stays on the acknowledged-gameable `certify_runtime.py` path | VER | major | tooling_fix | yes | S |
| 8 | Sound verifier **never grades the Rust ADDRESSER surface** (only Python primitives); fuzzer covers decode not address | VER | major | tooling_fix | yes | M |
| 9 | **Sound verifier sits outside the drift-proof loop** — anti-gaming backstop never wired into the probe/ledger | VER | major | tooling_fix | yes | S |
| 10 | Stale test-count literals across front-door + evidence docs (workspace 65/69 vs 71; robustness 40 vs 43), incl. the "cannot drift" ledger | GOV | major | tooling_fix | yes | S |
| 11 | **Capability determinism gate over-claimed as enforcement** — run-twice author-input probe; input-scoped/delayed non-determinism certifies + installs | PRD | major | doc_fix (+ living_fix) | yes | S |
| 12 | `certify_runtime.py` **module-level Python import crashes a Kit-only checkout** before any output (B4 only half-fixed) | VER | major | tooling_fix | yes | S |
| 13 | `@arves/*` SDK quickstarts use **bare-specifier imports that fail** `ERR_MODULE_NOT_FOUND` (private/unexported/unpublished) | PRD | major | living_fix | yes | S |
| 14 | Docs-site generator emits **15 dead relative links** (5 load-bearing ACS links 404 on Pages); RELEASING falsely certifies "links clean" | GOV | major | tooling_fix | yes | M |
| 15 | **No discoverable G2 challenge front door** — `IMPLEMENTING_ARVES.md` unlinked from README; no `CHALLENGE.md`, no intake | VER | major | doc_fix | yes | S |
| 16 | **Freeze enforced only by author discipline + a git tag** — no mechanical gate detects a silent content edit to a frozen file | GOV | major | tooling_fix | yes | M |
| 17 | Marketplace **"signed, certified" trust boundary is an unkeyed content hash** — any party can re-sign tampered code; no identity binding | PRD | minor | doc_fix (+ living_fix) | yes | S |
| 18 | **PropertyCheck/Suite runtime-proof harness unimplemented**; invariant-id→executable-proof binding unwired for the implemented Kernel | RT | minor | RCR | no | M |
| 19 | ACS-004 type codes (int/u32) **cannot type a valid ACS-002 Integer in [2^63, 2^64−1]** | STD | minor | CCP | no | S |
| 20 | ACS-004 §6.5 validation **doesn't bind an instance's Identity `urn` to its own type** — weakens the §4 "ABI loop closes" claim | STD | minor | CCP | no | S |
| 21 | ACS-005 §9.1 lists "Data Plane" as checker-required but **no GL-nnn entry resolves it** (only §7 prose) | STD | minor | CCP | no | M |
| 22 | ACS-002 §5 has **no explicit shortest-form LENGTH clause** though reason code `non-shortest-len` + vector 780161 mandate it | STD | minor | CCP | no | S |
| 23 | ACS-004 has **zero negative vectors** — §13 ACS-004-CS-1 instance-invalidity never gate-tested | STD | minor | CCP | no | S |
| 24 | ACS-005 reference checker **does not implement the §9.3 glossary-resolution lint at all** (scope-noted out) — decidable-conformance claim unrealized | STD/VER | minor | tooling_fix (+ CCP) | mixed | M |
| 25 | **11 non-kernel runtime crates are CONTRACT-ONLY skeletons** whose fidelity to frozen contracts (IDR-001..004, ORCH, Engine ABI, CAP, QUERY) was never audited | RT | minor | (audit → RCR if drift) | no | M |
| 26 | `arves create --provider claude\|gpt\|gemini` **scaffolds a dead-end** — no worked adapter example / how-to to the (existing) certify path | PRD | minor | doc_fix | yes | S |
| 27 | Cognitive-memory README calls the co-located hash chain **"tamper-evident" without the "externally-trusted head" caveat** | PRD | minor | doc_fix | yes | S |
| 28 | Stale in-code comment in `bridge.mjs` says 64-hex ContentId; code correctly requires 68-hex | PRD | minor | doc_fix | yes | S |
| 29 | Stale **"no runtime code exists yet"** clause in CLAUDE.md + Invariant Registry contradicts built I1 runtime | GOV | minor | doc_fix | no (Registry half) | S |
| 30 | CLAUDE.md PROJECT STATUS is **pre-seal** — never mentions Build seal / Growth Program that README/RELEASING/closure now declare | GOV | minor | doc_fix | yes | S |
| 31 | `standard/README.md:64` **"references no file outside itself" is false** — in-Kit RUNTIME_AUTHORS_GUIDE dangles to `../verification/`; **also** cites the AEOS Manual which *does* exist in `corpus/` (outside `standard/`) | STD/GOV | minor | Kit_packaging (+ doc_fix) | mixed | M |
| 32 | RELEASING.md asserts "links clean" with **no link checker anywhere**; generator ships broken links | GOV | minor | doc_fix (+ tooling_fix) | yes | S |
| 33 | Stale docs-site **page count 77 vs generated 83** in RELEASING checklist | GOV | minor | doc_fix | yes | S |
| 34 | Robustness count stale **40/40 vs live 43/43** across 8 front-door/evidence surfaces | GOV | minor | doc_fix | yes | S |
| 35 | Stale Rust workspace count (**65/65, 69/0, 65**) vs live 71/0 across human-facing docs | GOV | minor | doc_fix | yes | S |
| 36 | Spec corpus index lists all four Documentation Index versions with **no "current" marker** | GOV | minor | tooling_fix | yes | S |
| 37 | QUICKSTART step 5 / README "Independent runtimes" bullet advertise the **maintainers' 2-runtime one-liner with no "certify YOUR runtime" snippet** nor pointer to the guide + mandatory soundness step | VER | minor | doc_fix | yes | S |
| 38 | TLA+ kernel artifact ships a **broken reproduction path** (`verification/model-checking/` doesn't exist) and no captured TLC run | VER | minor | tooling_fix | yes | S |
| 39 | Dockerfile builds `arves-bridge` from **cargo debug (no `--release`)**; JS bridge hardcodes the debug path — release build / fresh clone silently breaks the product→Kernel seam | PRD | minor | living_fix | yes | S |

---

## 3. Three actionable buckets

### Bucket A — EXECUTABLE NOW (freeze-clean, no maintainer sign-off)
All edits are in `verification/`, `products/`, `tools/`, or living root docs. Touches **no** frozen
`standard/` or `runtime/` artifact.

- **#5** Re-attest / SUPERSEDE the stale L1 attestation (`doc_fix`)
- **#6** Wire the P8 cert row into the probe *or* honestly re-tier Section A (`tooling_fix`)
- **#7** Extend `verify_runtime_sound.py` to grade the Rust reference too (`tooling_fix`)
- **#8** Drive the Rust *addresser* through the sound grader, inputs-only (`tooling_fix`)
- **#9** Wire the sound verifier into the drift-proof probe loop (`tooling_fix`)
- **#10 / #34 / #35** One-time count sweep (71/0, 43/43) + make the probe emit/patch the ledger MD (`tooling_fix`)
- **#11** Reword the capability-determinism gate from "enforces" to "best-effort author-input probe; full enforcement is v1.1 RCR debt" (`doc_fix`); optional probe hardening (`living_fix`)
- **#12** Guard the module-level Python import in `certify_runtime.py` so a Kit-only checkout degrades, not crashes (`tooling_fix`)
- **#13** Add real `exports`/entry points to `products/*/package.json` *or* relabel as repo-local previews and switch the two README snippets to the working relative import (`living_fix`)
- **#14 / #32 / #33** Add a build-time link-gate to `build_docs_site.mjs`, fix the 15 dead links, correct "links clean" + page count (`tooling_fix` + `doc_fix`)
- **#15** Link `IMPLEMENTING_ARVES.md` from README + add `CHALLENGE.md` + a `.github` intake template (`doc_fix`)
- **#16** Add `CODEOWNERS` + a freeze-diff script comparing HEAD's `runtime/`+`standard/` against the `runtime-v1.0` manifest + a workflow (`tooling_fix`)
- **#17** Add the "signature is a re-derivable content address; no identity is bound" caveat to marketplace docs (`doc_fix`)
- **#24** Implement the §9.3 glossary-resolution lint in `acs005_checker.py` (the reference-checker half is freeze-clean; the "Data Plane" fix is CCP → bucket B)
- **#26** Add a worked local-adapter example + how-to for `--provider claude|gpt|gemini` → certify (`doc_fix`)
- **#27** Add the "requires an externally-trusted head/anchor" caveat to cognitive-memory README (`doc_fix`)
- **#28** Fix the 64-hex→68-hex comment in `bridge.mjs` (`doc_fix`)
- **#30** Refresh CLAUDE.md PROJECT STATUS to record the Build seal + Growth Program (living half of #29) (`doc_fix`)
- **#36** Annotate the current Documentation Index version in the generator (`tooling_fix`)
- **#37** Add a "certify YOUR runtime only" snippet + soundness-step cross-link to QUICKSTART/README (`doc_fix`)
- **#38** Run TLC once, capture output under `verification/formal/`, fix the ~4 dead path strings (`tooling_fix`)
- **#39** Note release-build path / build `arves-bridge` with `--release` in the container on-ramp (`living_fix`)
- **#4** Reconcile the product "REAL Kernel" claim: either route `class Arves` through the existing `bridge.mjs`→`arves-bridge` path, or scope the claim honestly (`living_fix` + `doc_fix`) — Kernel-side content-integrity is bucket B (#3)

### Bucket B — MAINTAINER-GATED (touches frozen `standard/` or `runtime/`)
Requires a CCP / Amendment / IDR / RCR / Kit-packaging record. **Not** freeze-clean.

- **#1** Add negative vectors for ACS-001/003/004/005 (`CCP`) — the single highest-leverage standard fix; the differential thesis is unproven without it
- **#2** ACS-003 Canonical Envelope negative vectors + gate wiring (`CCP`)
- **#3** Kernel commit computes/verifies the multihash at the gateway + a content≠hash(payload) reject vector (`RCR`, v1.1) — kin to the deferred "engine-enforced determinism" debt
- **#18** Implement `PropertyCheck`/`Suite` and bind ORCH-003/004 (and structural LAYER/OWN) proofs through the catalog for the implemented Kernel (`RCR`)
- **#19** ACS-004 u64/uint code *or* a clause scoping registered integers to [-2^63, 2^63−1] (`CCP`)
- **#20** ACS-004 §6.5 clause binding Identity `urn` to its own type + negative vector (`CCP`)
- **#21** ACS-005 "Data Plane" → GL-nnn resolution (promote alias or add GL-015) (`CCP`)
- **#22** ACS-002 §5 explicit shortest-form-length clause (`CCP`)
- **#23** ACS-004 instance-invalidity negative vectors (`CCP`) — folds into #1
- **#25** Audit the 11 CONTRACT-ONLY crates against their frozen contracts; **if** drift is found → `RCR`
- **#29** (Registry half) Correct "no runtime code exists yet" in the frozen Invariant Registry mirror (`CCP`/regenerate)
- **#31** Define a Kit artifact boundary + packaging tool and reconcile the "references no file outside itself" claim, incl. the AEOS-Manual citation which points at `corpus/` outside `standard/` (`Kit_packaging` + `doc_fix`)

### Bucket C — EXTERNAL (needs a real outside party / G2)
No amount of in-repo work discharges these.

- **A genuine third-party runtime** (a party outside this repo, no maintainer help) that builds from `standard/` alone and passes the conformance + soundness gates — the Era-3 / Foundation **exit gate**.
- **Third-party / arms-length architecture review** (L2–L4 certification levels) — the AEOS-Manual scenario axes are not yet populated.
- **Real organizations in production on ARVES without modifying the standard** — the north-star adoption metric.

> The Bucket-A "G2 challenge front door" (#15) and "certify YOUR runtime" signposting (#37) **remove
> friction** for a future G2 party; they do **not** perform G2. Intake records a result — it never
> assists construction and never fakes the external event.

---

## 4. Recommended execution order — Bucket A (highest value first)

Ordered by *value ÷ effort*, with the era KPI (evidence integrity that survives a G2 reviewer) as
the value axis. Group 1 restores honesty of the evidence trail; Group 2 hardens the anti-gaming
backstop; Group 3 opens/derisks the G2 on-ramp; Group 4 is mechanical hygiene.

**Group 1 — stop the evidence trail from lying (do first):**
1. **#5** SUPERSEDE the stale L1 attestation — it is *today-dated, cited as authoritative,* and broadcasts already-fixed blockers. Highest reputational bleed per hour.
2. **#10 + #34 + #35** Count sweep to 71/0 and 43/43, and make `evidence_probe.py` emit/patch the ledger MD so counts can't re-drift. Kills the whole stale-literal class at the root.
3. **#6 + #9** Re-tier Section A honestly *and* wire the sound verifier into the probe so "cannot drift" becomes true for the flagship survivability row.

**Group 2 — make the anti-gaming backstop actually bite:**
4. **#7 + #8** Grade the Rust reference (and its addresser) through `verify_runtime_sound.py`, inputs-only, with a byte-broken-adapter regression. Converts "backstop exists" into "backstop covers both runtimes."
5. **#12** Guard the Python import so the *documented Kit-only onboarding command* stops crashing before any output. Directly blocks a G2 stranger's first run today.

**Group 3 — open and derisk the G2 on-ramp:**
6. **#16** `CODEOWNERS` + freeze-diff script + workflow — converts the freeze guarantee from asserted prose to a checkable gate; underpins the whole survivability thesis.
7. **#15** README link to `IMPLEMENTING_ARVES.md` + `CHALLENGE.md` + intake. The era's single open gate is a G2 attempt; today the packet built for that stranger is unreachable from the front door.
8. **#13** Fix the `@arves/*` bare-import dead-end — the literal first line a developer copies.
9. **#37 + #26** "Certify YOUR runtime" snippet + soundness cross-link, and the `--provider` worked example. Removes the finish-line friction.
10. **#14 + #32 + #33** Docs-site link-gate + fix 15 dead links + correct "links clean"/page count — the newcomer on-ramp and a false release-checklist green.

**Group 4 — correctness-of-claim + mechanical hygiene:**
11. **#4** Reconcile the product "REAL Kernel" claim (route through `bridge.mjs` or scope honestly) — a materially misleading claim on four products.
12. **#11** Reword the capability-determinism gate to "best-effort probe."
13. **#17 + #27** Marketplace-signature and tamper-evidence caveats.
14. **#24** Implement the ACS-005 §9.3 glossary lint (reference-checker half).
15. **#30 / #28 / #36 / #39 / #38** CLAUDE.md status refresh, `bridge.mjs` comment, Documentation-Index "current" marker, container release-build note, TLC capture + path fix.

---

## 5. Notes / cross-checks folded in from the completeness critic

- **The single biggest un-audited hole is negative-vector coverage (#1/#2/#23).** All prior findings implicitly assumed the "16/16 core-reject" gate covers the standard; it is 100% ACS-002. ACS-003 (429-line Canonical Envelope) was entirely un-audited negatively.
- **The "REAL Kernel" product claim (#4)** is unbacked: `products/arves-sdk-ts/src/arves.mjs:38` is `#facts = new Map()`; the real `bridge.mjs`→`arves-bridge` path exists but `class Arves` does not use it.
- **Factual inversion corrected (#31):** the "AEOS Certification/Review Manual" the earlier self-containment finding said "does not exist" **does** exist as `ARVES_OS_Volume_6_Certification_Review_Manual_v1` in `corpus/` and `spec-markdown/`. The real defect is a Kit citing a frozen-corpus doc *outside* `standard/`, not a dangling citation to a nonexistent doc.
- **Still-open surfaces not deeply audited here:** `.docx`↔`spec-markdown` mirror fidelity, the substantive spec Volumes (Vol 9 Control Plane v2, Ontology, Engine Graph ABI, Reference Lifecycle, Scenario Conformance Framework, Vol 6 Manual), the differential fuzzer internals (seed determinism, true Rust↔Python cross-check), security/tenant-isolation (SHARD-001) enforcement, and four of seven products (marketplace, agent-runtime, enterprise-os, personal-os).

---

## 6. Freeze discipline & honesty reaffirmation

1. **Freeze is in force.** `standard/` and `runtime/` are byte-stable at `runtime-v1.0`. No item in
   Bucket A edits them. Every Bucket B item carries an explicit CCP / Amendment / IDR / RCR /
   Kit-packaging record — never a silent edit (CLAUDE.md rule #1).
2. **This analysis changes nothing.** It is a read-only audit. It creates no code, no vector, no
   spec text; it only ranks and routes work.
3. **This was run by the authors — grade G1.** It is self-assessment, not independent validation.
4. **Closing these gaps does not manufacture G2.** Making the evidence honest, wiring the
   anti-gaming backstop, and building a discoverable challenge front door *raise the quality of the
   surface a G2 party will stand on* — they do not, and cannot, substitute for a genuine external
   party building and certifying from `standard/` alone. **G2 remains external, unmet, and the
   Foundation-era exit gate.**

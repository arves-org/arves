# ARVES G2-Readiness Gap Register

**Status:** Standard-Validation-Era evidence artifact (ED-001 living). KPI = **Evidence
Increased**. This document records an honest, adversarial audit of whether the Standard
Kit (`standard/` + `verification/certification/`) is ready for a **G2** event — a
stranger who downloads only the Kit, builds a runtime, and passes certification *with no
help from the authors*.

> **Read this first — the honesty gate.** This audit was run **by the authors, inside
> the program**. It is a cold-start *simulation*, not a G2 event. Finding zero blockers
> here would **not** make ARVES G2-independent; only a genuine external team can do that
> (see §5). The value of this pass is the opposite: it tries to *fail* the Kit before a
> stranger does, so the gaps a stranger would hit are closed in advance. Independence is
> still capped at **G1** (`EVIDENCE_LEDGER.md` Section C).

---

## 1. Method

**Cold-start simulation.** We ran the Kit through the eyes of the party the G2 gate cares
about: an external team that receives only the publishable Kit, refuses to read
`runtime/crates/` (the reference source), and never contacts the authors.

- **N personas (6).** Each persona is a distinct cold-start lens over the *same* Kit:
  1. External **Go** engineering team (no Go reference exists — nothing to copy).
  2. **Spec-lawyer** lens — reads only RFC-2119 normative text, hunting clauses where two
     honest implementers could diverge and break interop.
  3. **"Bytes"** lens — implements ACS-001..004 aiming to match every golden vector
     byte-for-byte and pass `verification/certification`.
  4. **Build/ops** lens — clean machine, builds a non-Rust/non-Python runtime and runs
     the certification harness.
  5. **Hollow-attacker** lens — seeks a `CERTIFIED` stamp with the *least real work*
     (does the gate actually catch a runtime that only pretends to interoperate?).
  6. **Runtime-vendor** lens — builds a conformant runtime from `standard/` +
     `verification/certification/` alone, refusing the reference source.

- **Adversarial verification.** Every persona filed candidate gaps. Each candidate was
  then **independently checked against the actual repo** — spec text, both vector TSVs,
  `certify_runtime.py`, the three in-program runtimes (Rust/Python/TS), and the L1
  attestation — and classified with a verdict:
  `REAL_BLOCKER · ANSWERED_IN_KIT · ANSWERED_ONLY_IN_REFERENCE_SOURCE · NOT_A_GAP ·
  NICE_TO_HAVE`, a severity, and a **fix instrument**.

- **Refute pass.** For every candidate that looked like a blocker, a dedicated search
  tried to *refute* it — to find the Kit artifact that already answers it. A candidate is
  reported below **only if the refute pass failed** to resolve it (and, for the harness
  gameability finding, the exploit was reproduced). Anything the refute pass resolved is
  demoted to §3 (answered-in-Kit) or dropped — it is **not** listed as a gap.

**Explicitly excluded from the gap table (by rule):** items fully answered by Kit text,
even where a persona initially misread them (e.g. ACS-002 integer carrier range is
normatively pinned in §5.2; the reason-code folding rule is in CONFORMANCE.md; the NFC
core/full tier split is documented by design). Listing those as "gaps" would inflate the
register and hide the real ones.

---

## 2. Confirmed residual gaps (CONFIRMED only)

Four distinct defects survived the refute pass. Two are **spec/packaging** interop holes
(the certification stamp can be earned while a normative reject-surface is unimplemented);
two are **certification-tooling** defects (the harness can be gamed, or crashes on the
exact Kit-only checkout a G2 team uses). None *prevents building a correct runtime* — the
normative rules are present — but each defeats the **survivability guarantee** the G2 gate
exists to prove: *that the `CERTIFIED` stamp actually means interoperable*.

| # | Gap | Where | Sev | Why it blocks a G2 team | Fix instrument |
|---|-----|-------|-----|-------------------------|----------------|
| **B1** | **The `CERTIFIED` stamp does not attest the ACS-003/004/005 reject surface.** ACS-003 §6.3/§10.4 (missing/unknown/mistyped field, malformed 34-byte `payload_cid`, null/empty `tenant_id`/`workspace_id`), ACS-004 §6.5/§7/§8/§13 (closed-schema, cardinality, `conf`/`u32` range, `invocation`-iff-`origin==derived`), and ACS-005 §9.3 are all **normative MUST/SHALL rejects with ZERO negative vectors**. The whole negative corpus is ACS-002-only (16 core + 1 nfc). `certify_runtime.py` counts only `pos==12` + `core-reject==16` (all ACS-002). | `standard/vectors/acs_negative_vectors.tsv` (17 rows, all `ACS-002`); `certify_runtime.py:99-110`; `standard/conformance/CONFORMANCE.md` §rejection-check (ACS-002 codes only); `ACS-003 §6.3`, `ACS-004 §6.5/§8/§13`, `ACS-005 §9.3` | major | A runtime with the envelope validator and the ACS-004 instance/state-machine validator **entirely unwritten** still prints `positive 12/12 core-reject 16/16 -> CERTIFIED`. Two "certified" runtimes can then disagree on accept/reject of an envelope or typed instance — the exact interop divergence the program exists to prevent. The only place these rejects are exercised is `verification/independent/typescript/{acs004,selftest}.mjs`, which lives **outside** the Kit and is **not wired into the harness**. | **Kit packaging** — add ACS-003/004/005 negative fixtures (envelope/instance/language tiers) to the shipped TSV **and** extend `certify_runtime.py` to drive a runtime's envelope-decoder / instance-validator / term-checker over them. (No CCP: the rules are already unambiguous.) |
| **B2** | **ACS-003 `causation_id` for a root event is normatively undecided (present-`Null` vs absent) and pulls *against* ACS-004 §6.4.** ACS-003 §5 marks `causation_id` plainly OPTIONAL with **no RFC-2119 keyword** fixing the root form; the row note "or Null when the event is a root" is bare prose. ACS-002 §5.7 makes present-`Null` and absent **distinct** canonical bodies → **distinct** envelope ContentIds. ACS-004 §6.4/§8/§11.4 states the Kit's *own* convention — OPTIONAL = **absence** of the key, "not by a present Null" — so a diligent implementer omits the key, the **opposite** of the single ACS-003 golden vector (present-`Null`). | `ACS-003 §5` (line 119, no keyword), `§6.1`/`§10.4` (scoped to the one vector's value); `ACS-002 §5.7`; `ACS-004 §6.4` ("not by a present Null"), `§8`, `§11.4`; single ACS-003 golden row present-`Null`; no negative vector for the absent form | major | The **single most common envelope** — a root event — has two lawful encodings under two Kit standards, yielding two different ContentIds and an **ORCH-004 dedup / cross-runtime interop break**. A G2 team still earns `CERTIFIED` by matching the one pinned vector, then fails to interoperate on its own root events. The harness never asks a runtime to *encode a root event from a logical value*, so the divergence is invisible; the three in-program runtimes agree only because all three copy the §10.2 vector verbatim. | **CCP Amendment** — give `causation_id` a definite cardinality in ACS-003 §5 (e.g. `Text\|Null, card:1`: a root event SHALL encode present-with-`Null`; absence is non-conformant), harmonized with ACS-004 §6.4, **and** add a negative vector rejecting a root envelope that omits `causation_id`. |
| **B3** | **The certification harness is gameable — a zero-logic echo adapter is `CERTIFIED`.** `certify()` receives the adapter's return values **and the answer key** (`golden` carries `content_id`; `neg` carries `reject_reason`) in the same scope, and **never recomputes** SHA-256 or re-decodes bytes — it trusts what the adapter returns. `RUNTIME_AUTHORS_GUIDE.md` Step 4 (self-declared "the single authoritative procedure") routes a G2 vendor straight into this answer-bearing adapter shape. | `certify_runtime.py:99-124`; `standard/RUNTIME_AUTHORS_GUIDE.md` Step 4; contrast the correct pattern that *recomputes*, `verification/independent/reference-runner/run.mjs:299-321` | major | **Reproduced:** a hollow adapter `addresses=[cid for (_,_,cid) in golden]; rejects=[('REJECT',reason) for (_,_,reason) in neg]` yields `positive 12/12  core-reject 16/16 -> CERTIFIED`, a line byte-identical to a real runtime's — with no SHA-256, no CBOR, no decoder. This is the minimal-effort path to the stamp and fully defeats G2's purpose. The whole survivability claim leans on this verdict (`FOUNDATION.md`, `CCP-GATE-Ratification-v1.md`). The correct recompute-in-the-grader discipline already exists in the marketplace cert-gate and in `run.mjs`, but was never applied here. | **Tooling fix** — pass the runtime-under-test only *inputs* (`(domain, body)` / `(tier, input_hex)`) and keep the key solely in the grader; have `certify()` itself recompute the ContentId and run a canonical decode. (Adopt `run.mjs`'s structure in the official tool.) |
| **B4** | **`certify_runtime.py` crashes (`FileNotFoundError`) on a Kit-only checkout — it hard-drives the reference Rust bins unconditionally.** `main()` eagerly evaluates the two reference records inside the `records=[...]` literal *before any output prints*; `rust_addresses`/`rust_rejects` call `subprocess.run` on `runtime/target/debug/{arves-bridge,acs_decode}` with **no existence guard**. Those are build artifacts absent from a `standard/`+`verification/` checkout, and building them means compiling the reference runtime the vendor was told (IDR-006) not to depend on. | `certify_runtime.py:26-33, 61-74, 121-124`; `RUNTIME_AUTHORS_GUIDE.md` Step 4 ("add a record alongside the existing ones", "no maintainer required"); no doc under `standard/`/`verification/` says to `cargo build` or to remove/guard the two Rust records | major | The **documented Step-4 command does not run green** on the exact scope a G2 team uses; it dies with an uncaught traceback **before** printing the vendor's own verdict — never `NOT CERTIFIED`, never a diagnostic. The only documented cure (`docs/DEPLOY.md`) is to *compile the reference Rust runtime* — the dependency IDR-006 and the Guide forbid. (Secondary: the `split()[0]` parser silently miscounts an `ERR` protocol line as a content/verdict miss rather than flagging it.) | **Tooling fix** — guard `exe()` so a missing binary degrades to a printed `NOT CERTIFIED (runtime unavailable)` row; make the record list data-driven so a vendor can run **only** their own runtime; flag `ERR` lines explicitly. Pair with a one-line note in Guide Step 4 that the pre-wired reference records are optional. |

**Shared root cause.** B1–B4 are one theme: **the automated G2 gate certifies less than the
standard requires, and can be satisfied without interoperating.** B1/B2 are holes in *what*
the gate checks (envelope/instance rejects; the root-event byte form); B3/B4 are holes in
*how* the gate checks (it trusts the adapter's answers; it can't run at all on a Kit-only
box). All four are already logged, in whole or part, as **L1 Standard Lock gap G5** and the
Lock verdict of **CONDITIONAL** (`verification/certification/L1_Attestation_and_Standard_Lock_Review.md`).

### 2a. Remediation status (post-audit, 2026-07-02)

Two of the four defects were closable in the **living verification arm** with no frozen-Kit
change, and were closed + regression-tested the same session:

| Gap | Status | What shipped | Proof |
|-----|--------|--------------|-------|
| **B4** | **CLOSED (living)** | `certify_runtime.py` `main()` now guards the reference-bin invocation: a missing `arves-bridge`/`acs_decode` degrades to an `UNAVAILABLE` row instead of an uncaught `FileNotFoundError`, and the record list is data-driven so a vendor runs only their own runtime. `certify()`'s signature is unchanged, so the frozen `RUNTIME_AUTHORS_GUIDE` contract still holds. | `test_harness_integrity.py` points the harness at a missing binary and asserts it prints a verdict and returns (no crash). |
| **B3** | **Mitigated (living); documented-path fix gated to Kit 0.2.1** | `verification/certification/verify_runtime_sound.py` — a non-gameable grader that OWNS the truth (recomputes every ContentId per ACS-001 §5/§7, re-decodes every input), probes **fresh** `(domain,body)` pairs absent from the vectors, and injects **accept-probes** a conformant decoder must accept. The runtime is given inputs only, so a hollow echo adapter cannot pass. | `test_harness_integrity.py`: the real Python runtime is `SOUND-CERTIFIED`; a maximally-informed hollow echo adapter (hardcodes every published answer) scores **fresh 0/3, accept 0/3 → NOT CERTIFIED**. |

**Why B3 is "mitigated," not "closed":** the *official documented* path (`certify_runtime.py`
+ `RUNTIME_AUTHORS_GUIDE.md` Step 4) hands the grader the answer key **by design**, and that
contract is **frozen**. Converging the official path onto the sound grader is a **Kit 0.2.1**
change (frozen guide + harness contract) — maintainer-gated, never a silent edit. Until then,
the sound verifier is the Verification arm's non-gameable gate and the guide-path residual is
disclosed to implementers (`IMPLEMENTING_ARVES.md` §5).

**B1 — reclassified to CCP, and an evidence subset delivered (2026-07-02).** Scouting B1
found that the frozen reason-code registry is closed and **ACS-001 §4.1 mandates that new
reject reason codes be added _only via a CCP Amendment that also adds a negative vector_** — so
B1 is a **CCP**, not the Kit-packaging the initial triage assumed (a correction to this
register's B1 row). To de-risk that CCP and increase evidence now with **no frozen edit**, the
ACS-003/004/005 reject surfaces were implemented as **living reference validators + self-tests**,
proving the rules are implementable from the Kit spec alone:

| Validator (living) | Reject rules proven (self-test) | Result |
|---|---|---|
| `verification/independent/python/acs003_envelope.py` | §6.3: not-a-Map, missing-required, unknown-key, wrong-type, malformed `payload_cid` shape, empty tenant/workspace (SHARD-001), bad `causation_id` type | **8/8** (1 accept + 7 reject) |
| `verification/independent/python/acs004_instance.py` | §6.5/§7/§8: unknown-field, required-absent, type (int/u32/conf/urn), cardinality (both ways), provenance state machine (`invocation` iff `origin==derived`, both ways) | **11/11** (1 + 10) |
| `verification/independent/python/acs005_checker.py` | §8/§9.2/§9.3: UTF-8, NFC, no lead/trail/blank LF, strict sort, no-dup, per-body grammar; anchored to the 3 golden ContentIds | **17/17** (6 + 11) |

Combined runner: `python verification/independent/python/acs_validators_selftest.py`. These are
the **oracle** a future CCP's negative vectors will be checked against.

**Still open (the CCP itself + B2):** **B1's CCP** — define the stable reject reason codes
(extend the closed registry), ship the negative vectors in `standard/vectors/`, and wire the
harness (per ACS-001 §4.1, maintainer-authorized); and **B2** (root-event `causation_id` — a CCP
Amendment to ACS-003 §5). Both are frozen-touching spec/Kit changes for the next cycle.

> **B1 now staged (2026-07-02):** a freeze-clean **CCP-006 DRAFT** proposes the fix concretely —
> 11 stable reject reason codes + **18 oracle-verified candidate negative vectors** (7 envelope +
> 7 instance + 4 language), machine-checked against the reference validators
> (`verification/ccp-drafts/`, regenerate with `gen_candidate_vectors.py`). It touches no frozen
> byte; ratification at CCP-GATE (which appends the vectors into `standard/vectors/`, extends the
> registry + harness, and re-runs `freeze_check.py update`) remains maintainer-authorized.
>
> **✅ B1 RATIFIED (2026-07-04):** CCP-006 is ratified (`standard/acs/CCP-GATE-Ratification-v2.md`,
> Kit `0.3.0`). The 18 semantic vectors + 11 reason codes are now in the frozen Kit
> (`acs_negative_vectors.tsv` 17 → 35 rows); freeze re-baselined to **150 files, 0 drift**.
> Exercised + drift-proof: `conformance_semantic.py` → `envelope 7/7 instance 7/7 language 4/4
> REJECTED`, wired into `evidence_probe.py` (now **9/9** rows). **Caveat (honest):** the frozen
> Rust v1.0 reference has no ACS-003/004/005 validators, so it **declares the semantic tiers
> deferred** (like `nfc`); native Rust validators are tracked as **RCR-004**. B1 is closed at the
> **standard** level; the reference *runtime* does not yet reject these natively. Still **G1**.

---

## 3. What the Kit already carries (answered-in-Kit — NOT gaps)

The audit's larger result is affirmative: the great majority of adversarial probes were
**refuted by Kit materials a G2 team receives** (no reference source needed). Recording
these prevents re-filing them as gaps and shows the Kit's normative surface is genuinely
self-contained for building. The most load-bearing:

- **Integer carrier range is pinned, not ambiguous.** ACS-002 §5.2 normatively requires
  carrying an Integer exactly across the whole `[-2^64, 2^64-1]` range ("a conformance
  requirement, not advice"); ACS-004 `int`/`u32` are *field-level* refinements on the same
  wide carrier (§6.3), explicitly reconciled in §7. An int64-only runtime is non-conformant
  by plain text — no guessing.
- **Reason-code mapping is derivable and demonstrated.** The closed 13-code registry lives
  in CONFORMANCE.md (normative-by-reference from ACS-002 §6); the folding of non-UTF-8 text
  and non-Text/Integer map keys to `reserved-or-unsupported`, and the break byte to
  `indefinite-length`, is stated there and worked in the two Kit-endorsed runners the Guide
  tells vendors to port. The single-defect-corpus contract (only one rule per vector) is
  normative ACS-002 §6.
- **NFC core/full tiering is by design, and disclosed.** CONFORMANCE.md, RUNTIME_AUTHORS_GUIDE.md,
  and ACS-002 §11 all state the `nfc` tier is deferrable-with-declaration, that the verdict
  counts only `core`, and name NFC as the *one residual* parser-differential gap — the Kit
  performing as designed, not hiding a hole.
- **The map-key sort, the multi-defect reason divergence, and the alt-hash `payload_cid`
  question** are each resolved by composing normative text already in the Kit (ACS-002 §5.6
  bytewise sort; ACS-002 §6 "any one applicable code is conformant, both still reject";
  ACS-001 §5 "certification SHALL be evaluated on SHA-256" + ACS-005 §4 MAY-is-never-a-FAIL).
- **The build/certify path itself is Rust-free and language-neutral.** RUNTIME_AUTHORS_GUIDE.md
  + CONFORMANCE.md + the dependency-free `run.mjs` define conformance as two checks over the
  frozen TSVs a vendor implements in their *own* language; a Go/Java team is not blocked
  (B4 is about the *harness convenience tool* crashing, not about the documented procedure
  being undefined).

Evidence for these lives in the specs and vectors cited above and is already reflected in
`EVIDENCE_LEDGER.md` Section A (golden 12/12, ACS-002 reject 16/16, three-language
byte-agreement, all at **L3/G1**).

---

## 4. Verdict — is the Kit G2-ready today?

**NOT YET.** The Kit is **build-ready** — a diligent stranger can construct a byte-correct
runtime from `standard/` alone — but it is **not certification-sound**: the shipped G2 gate
can hand out `CERTIFIED` to a runtime that does not interoperate (B1/B2) or does not exist
at all (B3), and cannot even run on the exact checkout a G2 team uses (B4). Since the entire
point of the G2 event is that the *stamp means something to a stranger*, these four defects
block G2-readiness. They are all **closable without touching the frozen ACS specs' byte
formats** — none requires a new major version.

**Update (2026-07-02, same session — see §2a):** **B4 is CLOSED** and **B3 is backstopped**
by a non-gameable sound verifier (`verify_runtime_sound.py`), both regression-tested. But
**B1 and B2 remain open**, and B3's *official documented* path is still echo-trusting until
the maintainer-gated Kit 0.2.1 convergence — so the verdict stands at **NOT-YET**: the
documented gate can still certify a runtime that skips the ACS-003/004/005 reject surface
(B1), and the root-event encoding ambiguity (B2) is unresolved. B1's reject rules are now
**proven implementable** from the spec (living validators + self-tests, §2a), de-risking its CCP
to a maintainer-authorized change (new stable reason codes + shipped negative vectors, per
ACS-001 §4.1); the verdict is unchanged until that CCP and B2 land.

**The specific closable list (framed by instrument):**

| Gap | Instrument | Closes when |
|-----|-----------|-------------|
| ~~**B1** — ACS-003/004/005 reject surface uncertified~~ ✅ **CLOSED (standard)** 2026-07-04 via **CCP-006** | **CCP** (vectors + reason codes) | ✅ 18 envelope/instance/language negative vectors + 11 reason codes shipped in `acs_negative_vectors.tsv` (Kit 0.3.0); exercised by `conformance_semantic.py` (`evidence_probe` 9/9). Runtime-side native validators tracked as **RCR-004**. |
| **B2** — root-event `causation_id` undecided vs ACS-004 §6.4 | **CCP Amendment** (+ one negative vector) | ACS-003 §5 fixes `causation_id` cardinality (harmonized with ACS-004 §6.4) and a root-omits-`causation_id` reject vector is added. |
| **B3** — harness trusts adapter answers (gameable) | **Tooling fix** | `certify()` recomputes the ContentId and runs a canonical decode itself; the runtime-under-test is given inputs only, never the key. |
| **B4** — harness crashes on Kit-only checkout | **Tooling fix** | Missing reference bins degrade to a printed `NOT CERTIFIED` row; the record list is data-driven so a vendor runs only their own runtime; `ERR` lines are flagged. |

Net for the era KPI: this pass **increased evidence** by converting four latent
survivability defects from "unknown" to CONFIRMED-with-instrument, and by refuting a much
larger set of candidate gaps against Kit text (§3). The independence grade is unchanged:
still **G1**.

---

## 5. Reaffirmation — this audit is NOT a G2 event (still G1)

To be unambiguous, because the honesty gate forbids laundering independence:

- This audit was **run by the authors, from inside the program.** Every persona and every
  refute pass is *our* work over *our* repo. That is precisely a **G1 same-process**
  activity, exactly like the cold Python and TypeScript builds already ledgered at G1.
- **Closing B1–B4 does not, by itself, produce a G2 result.** It removes obstacles a
  stranger would hit; it does not substitute for the stranger. The G2 exit gate is defined
  in `CERTIFICATION_PROGRAM.md` §2 and remains open:

  > A stranger downloads ONLY the Standard Kit from a public source, implements a runtime,
  > and PASSES certification — *you did not help.*

- Per `EVIDENCE_LEDGER.md` Section C, the maximum independence grade is **G1**, and
  **"Third-party runtime — G2 — NOT YET MET"** stands. Nothing in this document may be
  cited as G2, independent-of-makers, or survivability-proven. The single move left toward
  G2 is a genuine external team — this register just makes the ground they will stand on
  less likely to give way.

---

*Reproduce the harness verdict (and the B3 exploit) with
`python verification/certification/certify_runtime.py`; the confirmed-gap evidence is the
spec/vector/harness lines cited inline and the L1 Lock gap G5 in
`verification/certification/L1_Attestation_and_Standard_Lock_Review.md`.*

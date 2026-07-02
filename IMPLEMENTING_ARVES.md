# Implementing ARVES — external-implementer onboarding packet

> **Status:** LIVING · NON-NORMATIVE companion. This packet is a cold-start guide, not
> a specification. It **does not change, extend, or reinterpret** the frozen ARVES
> Standard. Where this packet and any file under `standard/` disagree, `standard/`
> wins — and the disagreement is a docs bug to report, never something to work around.
> The authoritative, self-declared single procedure is
> [`standard/RUNTIME_AUTHORS_GUIDE.md`](standard/RUNTIME_AUTHORS_GUIDE.md); this packet
> is a friendlier on-ramp to it, plus an honest map of the rough edges you will hit.

---

## 1. The G2 mission (one paragraph)

ARVES's survivability property is that **anyone can build a conformant runtime and
certify it from the published Standard alone — with zero contact with the authors.**
Independence is graded: **G1** is an implementation written with maintainer help or by
the same team/model family (the repo already has G1 — a Rust reference runtime and an
independent Python runtime, certified under one conformance). **G2** is the open exit
gate: *a genuinely unknown outside team, using ONLY the files under `standard/` (plus
the certification harness under `verification/certification/`), builds a runtime that
reproduces every golden vector and rejects every core negative, and certifies it with
no questions asked.* If you can reach `positive 12/12  core-reject 16/16  ->  CERTIFIED`
from these files without ever talking to us, **you are the missing G2 evidence.** This
packet exists to make that attemptable — it does **not** claim G2 has been reached. It
has not. You reaching it is the proof.

---

## 2. Exact reading order of Kit files

Read these in order. Everything binding is under `standard/`. You do **not** need — and
per the Kit's independence test, must **not** need — to read the Rust reference runtime
source under `runtime/`. If you find yourself needing it, that is a Kit failure; report
it as an ambiguity in the relevant ACS.

| # | File | Why / what you get |
|---|------|--------------------|
| 1 | [`standard/VERSION`](standard/VERSION) | What the Kit is (`arves-standard-kit 0.2.0`), the five ACS version tags, ratification status (RATIFIED v1.1 at grade **G1**), and the verbatim **G2 exit criterion**. |
| 2 | [`standard/README.md`](standard/README.md) | Kit overview, the independence test, the 4-step "How to implement ARVES (from this Kit alone)", and the honest **G1-not-G2** status. |
| 3 | [`standard/RUNTIME_AUTHORS_GUIDE.md`](standard/RUNTIME_AUTHORS_GUIDE.md) | **The single authoritative build+certify procedure.** Steps 1–4. If any other page seems to describe a different procedure, follow this one. |
| 4 | [`standard/acs/ACS-005_Normative_Language.md`](standard/acs/ACS-005_Normative_Language.md) | **Read FIRST among the ACS specs.** Defines what MUST / SHALL / SHOULD / MAY mean here (RFC 2119/8174), the requirement-ID grammar, and the 14-term glossary the other four specs cite. |
| 5 | [`standard/acs/ACS-001_Content_Addressing.md`](standard/acs/ACS-001_Content_Addressing.md) | The identity primitive: `ContentId = 0x12 0x20 ‖ SHA-256(domain_tag ‖ body)`, and §4.1 — the single authoritative domain-tag / hash-code / reason-code **registry**. |
| 6 | [`standard/acs/ACS-002_Canonical_Serialization.md`](standard/acs/ACS-002_Canonical_Serialization.md) | **The keystone — the substantive work.** The deterministic-CBOR profile: shortest ints, always-binary64 floats, NFC text, bytewise-sorted map keys, definite lengths, `MAX_DEPTH = 128`, and every decoder rejection rule + its stable reason code. |
| 7 | [`standard/acs/ACS-003_Canonical_Envelope.md`](standard/acs/ACS-003_Canonical_Envelope.md) | The interchange envelope (a dCBOR map, domain `0x06`): 12 fields, `payload_cid` binding, SHARD-001 (tenant/workspace) rules, and §6.3 decoder-validation rules. |
| 8 | [`standard/acs/ACS-004_Universal_Type_Registry.md`](standard/acs/ACS-004_Universal_Type_Registry.md) | The type registry (domain `0x07`): schema-document shape, type codes + cardinality, the §6.5 closed-schema validator, and the §8 `invocation`-iff-`origin==derived` state machine. |
| 9 | [`standard/conformance/CONFORMANCE.md`](standard/conformance/CONFORMANCE.md) | **The pass gate.** The two language-neutral checks (encoder+addresser over the golden TSV; rejection over the negative TSV), the stable reason-code list, and the core-vs-full verdict semantics. |
| 10 | [`standard/vectors/acs_golden_vectors.tsv`](standard/vectors/acs_golden_vectors.tsv) | The 12 positive byte-exact targets you must reproduce. |
| 11 | [`standard/vectors/acs_negative_vectors.tsv`](standard/vectors/acs_negative_vectors.tsv) | The 17 negatives (16 `core` + 1 `nfc`) your decoder must reject with the matching reason. |

Optional but useful, in this order, once the above is understood:

- [`verification/independent/reference-runner/run.mjs`](verification/independent/reference-runner/run.mjs)
  — a ~300-line, dependency-free (Node built-ins only) worked runner: **copy it and port it.**
- [`verification/independent/reference-runner/README.md`](verification/independent/reference-runner/README.md)
  — how to run and port it; the "Adapt it for your language" checklist.
- [`verification/independent/python/`](verification/independent/python/) — a complete
  Kit-only Python implementation (`acs001_address.py`, `acs002_decode.py`) to study
  side by side. This is a **reference example**, not something to copy verbatim — the
  contract is the vectors, not this code.
- [`standard/certification/README.md`](standard/certification/README.md) — how the
  mechanical conformance run sits inside the broader (largely paper) certification
  process. Read so you don't conflate the two (see §5).

---

## 3. The conformance surface — a concrete checklist

The five ACS specs compose into **one byte-exact interoperability surface**. The whole
gate is two obligations (`RUNTIME_AUTHORS_GUIDE.md`): **reproduce** every golden
ContentId, and **reject** every core negative with the matching reason. Here is exactly
what each standard puts on that surface.

### The address formula (ACS-001) — memorize this

```
ContentId = 0x12 ‖ 0x20 ‖ SHA-256(domain_tag ‖ body)
             │      │       └── FIPS 180-4, 32-byte digest
             │      └── 0x20 = digest length (32)
             └── 0x12 = multihash code for SHA-256
```

`domain_tag` is a single byte from the ACS-001 §4.1 registry. `body` is the raw payload
bytes. Total ContentId is 34 bytes. Get the domain tag wrong or omit it and you collide
distinct vectors — this is the single most common cold-start bug.

### 3a. What you MUST REPRODUCE — the 12 golden vectors

For each row of [`acs_golden_vectors.tsv`](standard/vectors/acs_golden_vectors.tsv):
**encoder check** — produce the canonical `body` from the logical value described
normatively in the spec, assert `hex(body) == body_hex`; **addresser check** — compute
the ContentId, assert it equals `content_id`. (For ACS-001/005 rows the `body` is raw
bytes given directly — there is no encoding step, only the addresser check.)

| Standard | Rows | Domain(s) | What it exercises |
|---|---|---|---|
| **ACS-001** | 3 | `0x01`, `0x02`, `0x04` | Raw-body content addressing (`hello-truth`, `engine-manifest`, `invocation`). Addresser only. |
| **ACS-002** | 3 | `0x01`, `0x02`, `0x05` | Your **canonical dCBOR encoder** — a `uci.fact` map (V1), an engine-manifest map (V2), and an NFC+negative-int map (V3). `body_hex` is pinned, so the encoder is testable, not just the address. |
| **ACS-003** | 1 | `0x06` | The 12-field canonical **envelope** whose `payload_cid` is the ACS-002 V1 address. Exercises envelope **encoding + addressing**. |
| **ACS-004** | 2 | `0x01`, `0x07` | A `uci.fact@1.0` **instance** (`0x01`) and its **schema document** (`0x07`). Encoding + addressing. |
| **ACS-005** | 3 | `0x08`, `0x09` | Raw-body addressing of the glossary **term-set**, a **requirement** clause, and the **term-name** list. Addresser only. |

Passing all 12 (both checks each) = **ACS-conformant on the positive surface**. Two
runtimes that both pass agree byte-for-byte on every address and body — they
interoperate.

### 3b. What you MUST REJECT — the 17 negative vectors

Producing the right bytes is only half of conformance. A conformant **decoder** must
also refuse every non-canonical byte string, with the **exact** reason code — otherwise
two runtimes accept different encodings of "the same" value and disagree on its address.
Validate canonical form **inline while parsing**; never pattern-match whole test inputs.

Source: [`acs_negative_vectors.tsv`](standard/vectors/acs_negative_vectors.tsv). Tiers:
16 `core` (the interoperability gate) + 1 `nfc` (deferrable — see below).

| # | case | tier | reject_reason |
|---|------|------|---------------|
| 1 | non-shortest-int | core | `non-shortest-int` |
| 2 | non-shortest-len | core | `non-shortest-len` |
| 3 | indefinite-length | core | `indefinite-length` |
| 4 | unsorted-map-keys | core | `unsorted-map-keys` |
| 5 | duplicate-map-keys | core | `duplicate-map-keys` |
| 6 | float-not-float64 / half | core | `float-not-float64` |
| 7 | float-not-float64 / single | core | `float-not-float64` |
| 8 | negative-zero-float | core | `negative-zero-float` |
| 9 | non-finite-float | core | `non-finite-float` |
| 10 | trailing-data | core | `trailing-data` |
| 11 | reserved-or-unsupported / CBOR tag | core | `reserved-or-unsupported` |
| 12 | truncated | core | `truncated` |
| 13 | map-key-not-in-model | core | `reserved-or-unsupported` |
| 14 | text-invalid-utf8 | core | `reserved-or-unsupported` |
| 15 | top-level-break | core | `indefinite-length` |
| 16 | nesting-too-deep (129 nested arrays, one past `MAX_DEPTH=128`) | core | `nesting-too-deep` |
| 17 | non-nfc-text (é as NFD base+combining acute) | **nfc** | `non-nfc-text` |

The full closed reason-code vocabulary (emit these strings **verbatim** — a
differently-spelled reason silently fails certification):

```
non-shortest-int   non-shortest-len   indefinite-length   unsorted-map-keys
duplicate-map-keys float-not-float64  negative-zero-float non-finite-float
trailing-data      reserved-or-unsupported   truncated    nesting-too-deep
non-nfc-text
```

Notes that trip people up:
- `reserved-or-unsupported` is the catch-all for anything **not in the ACS-002 §4 value
  model**: CBOR tags, `undefined`/simple values, non-UTF-8 text octets, and a map key
  that is not a Text or Integer. (That is why rows 11, 13, 14 all fold to it.)
- The stray break byte `0xff` at top level rejects as `indefinite-length` (row 15) — it
  is the terminator of an indefinite item, which ACS-002 forbids.
- Reason codes are guaranteed to agree across implementations only for **single-defect**
  inputs; the corpus is built entirely from such inputs, so the exact-match check is
  well-defined.

### 3c. The `nfc` tier — core vs full conformance

There is **one** `nfc`-tier negative vector (row 17). Two honest verdicts exist, and you
must declare which one you are:

- **Core-conformant** — passes all 12 positives + all 16 `core` rejects. This is the
  interoperability gate. If your language lacks a Unicode NFC facility you MAY **defer**
  this single rule — but you MUST *declare* it (e.g.
  `VERDICT: CONFORMANT (ACS core; nfc-tier DEFERRED)`) and MUST NOT silently accept
  non-NFC text. "Defer" means "declare unenforced," not "accept."
- **Fully conformant** — additionally rejects the `nfc` row.

The certification verdict counts **only the `core` tier**, so a declared-deferring
runtime still certifies. If your runtime must be safe against hostile non-NFC input,
be **fully** conformant.

### 3d. What is on the surface but NOT vector-backed (read §5 before relying on the stamp)

ACS-003 §6.3 (envelope validation), ACS-004 §6.5/§7/§8 (instance validation + the
`origin`/`invocation` state machine), and ACS-005 §9.3 (the checker) all state
**normative MUST-reject** rules — but the shipped negative corpus is entirely
ACS-002-tier, so **none of these rejection rules is exercised by the automated gate.**
You should still implement them (the specs are unambiguous and complete), because two
"certified" runtimes that skip them will disagree on real traffic. See
[§5 Known rough edges](#5-known-rough-edges).

---

## 4. Step-by-step: implement → self-test → certify → what CERTIFIED means

### Step 1 — Implement, in your language

You need exactly two crypto/text primitives: **SHA-256** (FIPS 180-4) and, for full
conformance, **Unicode NFC normalization**. No network, no ARVES services, no license
key — the whole procedure is offline.

Build two things:

- **A. Addresser (ACS-001).** `ContentId = 0x12 0x20 ‖ SHA-256(domain_tag ‖ body)`.
  Reproduce the domain-tag registry (ACS-001 §4.1). This part is tiny.
- **B. Canonical serializer + decoder (ACS-002).** The serializer turns a logical value
  into the one canonical `body`. The decoder does the reverse **and rejects any
  non-canonical byte string** with the exact reason code. This is the substantive work.
  Then layer ACS-003 (envelope) and ACS-004 (schema/instance) as dCBOR maps on top.

**Do not** invent registry entries. If you think you need a new domain tag, hash code,
or reason code, that is a normative registry change (ACS-001 §4.1) — it goes through the
change process; certification is against the **current frozen registries**.

### Step 2 — Self-test against the frozen vectors

Copy the worked runner and port it — don't write the harness from scratch:

- [`verification/independent/reference-runner/run.mjs`](verification/independent/reference-runner/run.mjs)
  reads both TSVs, runs the positive + core-reject checks, prints
  `positive N/N  core-reject M/M`, and exits `0`/`1`.

The port checklist (from the runner's README):
1. **Load the vectors** from the TSVs — never hardcode values; read the frozen files.
2. **Addresser** — assert `hex(ContentId) == content_id` for all 12 golden rows.
3. **Canonical decoder** — assert each `core` row rejects with the exact `reject_reason`.
4. **Verdict** — `CERTIFIED` iff `positive 12/12` and `core-reject 16/16`.

Expected green output shape:

```
positive 12/12  (ACS-001 ContentId reproduced from domain+body)
core-reject 16/16  (ACS-002 non-canonical inputs rejected with the right reason)
VERDICT: CERTIFIED (ACS core)
```

Study the Python (`verification/independent/python/`) and Rust
(`cargo run -p arves-conformance --bin conformance`) ports next to it if you want a
second and third worked example.

### Step 3 — Run certification (the maintainer-independent harness)

The certification tool is
[`verification/certification/certify_runtime.py`](verification/certification/certify_runtime.py).
It drives any runtime over the same frozen vectors and prints the same verdict shape —
no reference source, no maintainer judgement.

Wire your runtime in as an **adapter** — two functions:

- `addresses(golden)` → for each golden `(domain, body)`, return your runtime's
  ContentId hex.
- `rejects(negatives)` → for each negative input, return `(verdict, reason)` where
  `verdict` is `REJECT`/`ACCEPT` and `reason` is your decoder's stable reason code.

The adapter may drive an executable over a line protocol (the Rust example) or import a
module directly (the Python example) — the harness does not mandate a transport; the
**contract is the vectors + reason codes**, not a CLI. Add a
`certify("Your Runtime (vendor)", your_addresses(golden), your_rejects(neg), golden, neg)`
record alongside the existing ones and run:

```
python verification/certification/certify_runtime.py
```

> **Two operational notes before you run it — both detailed in §5:**
> 1. The harness **also** drives the reference Rust binaries at
>    `runtime/target/debug/{arves-bridge,acs_decode}`. **As of 2026-07-02 these are
>    guarded** — if they are not built, the harness prints an `UNAVAILABLE` row and still
>    runs your record (it no longer crashes with `FileNotFoundError`). You may build them
>    (`cargo build`) or just run your own; the pre-wired reference records are **optional**.
> 2. The harness **trusts what your adapter returns** and does not re-hash or re-decode, so
>    on its own it certifies the *honesty* of your reported results, not that you did the
>    work. Use the non-gameable
>    [`verify_runtime_sound.py`](verification/certification/verify_runtime_sound.py) (grader
>    owns the truth + fresh/accept probes) and the Step-2 self-check runner (`run.mjs`) as
>    your real proof of correctness — see §5.

### Step 4 — What CERTIFIED means (and what it does not)

You are **CERTIFIED** iff you show `positive 12/12  core-reject 16/16  ->  CERTIFIED`.
Concretely, a CERTIFIED verdict from `standard/` + this harness alone attests:

- Your runtime reproduces every published ACS content address byte-for-byte, and
- Your decoder refuses every core-tier non-canonical input with the agreed reason.

That is the **ACS interoperability / identity layer** — the bytes every ARVES
implementation must agree on. It is a genuine, load-bearing property: any other
CERTIFIED runtime will interoperate with yours on addresses, canonical bodies, and
core rejection.

**CERTIFIED does NOT mean:**
- Full runtime-behaviour certification. The 12 Scenario axes and the L1–L4 levels
  (Kernel owns truth, Engine pure, Query read-only, ORCH-004 idempotency, etc.) are a
  **separate** process, defined in [`standard/certification/README.md`](standard/certification/README.md),
  involving an arms-length Independent Architecture Review. That process is largely
  paper today and its scenario suite is not populated.
- That you enforce the ACS-003/004/005 rejection rules from §3d — the automated gate
  does not exercise them (see §5).
- That you handle non-NFC input, unless you declared **full** (not core) conformance.

Reaching CERTIFIED as a genuine outside team, with no contact with us, is exactly the
**G2 evidence ARVES is still missing.** If you get there, that is the whole point.

---

## 5. Known rough edges (honest)

These are **confirmed** residual gaps, verified against the current repo. None of them
stops you from building a correct runtime or obtaining a CERTIFIED verdict — but a
diligent implementer should know them, because several mean the *stamp attests to less
than the specs require*. Where the spec text is complete but the tooling/vectors fall
short, **implement to the spec anyway.** The living gap register is the L1 Standard
Lock Review, §2 "residual gaps" (gaps **G3, G5, G7**):
[`verification/certification/L1_Attestation_and_Standard_Lock_Review.md`](verification/certification/L1_Attestation_and_Standard_Lock_Review.md).

**1. The certification gate is ACS-002-only on the reject side (gaps G3/G5).**
Every one of the 17 negative vectors is ACS-002-tier, and `certify_runtime.py` counts
only positive-12 + core-reject-16. So a runtime with the **envelope validator (ACS-003
§6.3) and the instance validator (ACS-004 §6.5/§8) entirely unwritten** still prints
`12/12 + 16/16 -> CERTIFIED`. *How to proceed:* implement ACS-003 §6.3 (reject missing
required fields, unknown keys, non-Integer `occurred_at`/`schema_version`/`payload_domain`,
malformed 34-byte `payload_cid`, null/empty `tenant_id`/`workspace_id`) and ACS-004
§6.5/§7/§8 (closed-schema unknown-field rejection, cardinality, `u32`/`conf` range,
int-vs-float discipline, and `invocation` present **iff** `origin == "derived"`) from
the spec text — it is normative, unambiguous, and complete. Do not rely on the gate to
catch their absence; it can't.

**2. The harness trusts adapter output — it is gameable.**
`certify_runtime.py`'s `certify()` receives both your returned values *and* the answer
key in the same scope, and never recomputes SHA-256 or re-decodes. A hollow "echo"
adapter that just returns the golden ContentIds and `("REJECT", reason)` for every
negative row prints `CERTIFIED`. *How to proceed:* this is a harness weakness, not a
spec gap — your obligation is real conformance. Use the Step-2 self-check runner
(`run.mjs`), which recomputes `ContentId(domain, body)` and runs its own decoder over
raw inputs (it holds no answer key), as your genuine proof. Treat a green
`certify_runtime.py` line as the verdict *format*, and the honest self-check as the
substance.
**[Addressed 2026-07-02]** A non-gameable verifier now backstops this:
[`verification/certification/verify_runtime_sound.py`](verification/certification/verify_runtime_sound.py)
gives your runtime **inputs only**, recomputes every ContentId itself, and adds **fresh**
address probes + **accept**-probes a hollow echo adapter cannot satisfy — run it as your
real proof (`python verification/certification/verify_runtime_sound.py`). The *documented*
`certify_runtime.py` path stays echo-trusting until a maintainer-gated Kit 0.2.1 converges
it onto the sound grader; the regression `test_harness_integrity.py` proves the hollow
adapter fails the sound verifier.

**3. Root-event `causation_id` encoding is not pinned by an RFC-2119 keyword (ACS-003).**
For a **root** event (no cause), ACS-003 §5 marks `causation_id` plainly OPTIONAL, while
the single ACS-003 golden vector encodes it as **present-with-Null**, and ACS-004 §6.4's
general convention ("an optional field is realized by absence of the key, not by a
present Null") pulls the other way. Present-Null and absent are **distinct** canonical
bodies → distinct ContentIds. There is no negative vector catching the wrong choice.
*How to proceed:* to match the golden vector and the reference runtimes, **encode a root
event's `causation_id` as present-with-Null** (mirror the ACS-003 §10.2 vector exactly).
Then flag the ambiguity in your report so it can be resolved by amendment. (This is
logged as an open item in the gap register.)

**4. The harness has a hard, unguarded dependency on the reference Rust binaries.**
As shipped it eagerly drives `runtime/target/debug/arves-bridge` and `acs_decode`; if
they are absent it dies with `FileNotFoundError` before printing anything. *How to
proceed:* build them (`cargo build`) **or** — cleaner for a Kit-only checkout — remove
or guard the two pre-wired reference `certify(...)` records in `main()` so only your
adapter runs. Editing files under `verification/` is expected; only `standard/` and
`runtime/` are frozen.
**[Fixed 2026-07-02]** `certify_runtime.py` now guards the reference-bin invocation: on a
Kit-only checkout the missing binaries degrade to an `UNAVAILABLE` row and your own runtime
still certifies — no more `FileNotFoundError`. (You may still build the bins or add only
your own record; the record list is data-driven.)

**5. The `nfc` deferral is unenforced and unsurfaced by the harness.**
The harness ignores the `nfc` row entirely, so a runtime that *silently accepts* non-NFC
text still shows `core-reject 16/16`. *How to proceed:* honour the declared contract —
if you cannot enforce NFC, **declare** the deferral in your verdict string; never
silently accept non-NFC text. If safety against hostile input matters, be fully
conformant.

**6. Some normative surfaces are stated but never given a positive/negative fixture.**
Examples: ACS-004 §11.4's derived-variant ContentId lives only in the spec `.md`, not in
the golden TSV; there are no vectors exercising large integers in `(2^63, 2^64-1]`
(ACS-002 §5.2 requires the full `[-2^64, 2^64-1]` carrier); ACS-001 §6 idempotency
(ORCH-004, at-most-one-commit) is a **runtime-behavioral** requirement explicitly scoped
out of the ACS layer to the separate Certification process. *How to proceed:* implement
to the spec text — it is authoritative even where a fixture is missing — and treat
gap-register items G5/G7 as the tracked remediation, not as license to skip the rule.

### Bottom line

The Standard is byte-exact and independently reproducible on its positive surface, and
the reject side is airtight for ACS-002. The rough edges above are real and are
tracked in the gap register; they mean **the CERTIFIED stamp currently proves the
ACS-001/002 interoperability core, and you must implement the ACS-003/004/005 validation
rules on your own initiative from the (complete, unambiguous) spec text.** Build to the
spec, self-check honestly with `run.mjs`, certify with the harness, and report every
ambiguity you hit as a docs/spec gap rather than working around it.

**G2 is not achieved.** This packet makes it attemptable. If you certify a genuine
third-party runtime from these files with no contact with the authors, you have
supplied the evidence ARVES has been waiting for.

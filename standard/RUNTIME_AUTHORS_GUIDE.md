# Runtime Authors' Guide — build and certify your own ARVES runtime

**This is the single authoritative procedure for implementing an ARVES runtime and
getting it certified — from `standard/` alone, with no help from the ARVES
maintainers.** If any other page seems to describe a different runtime-authoring or
certification path, **this guide is the one to follow**; the others are pointers to
it (see [Reconciling the other docs](#reconciling-the-other-docs)).

You are a **Runtime vendor**: you implement the ARVES Core Standards (`standard/acs/`)
in your language and prove your implementation agrees, byte for byte, with every other
conformant runtime. You do **not** read the Rust reference runtime source, and you do
**not** modify `runtime/` or `standard/` (they are frozen — IDR-006). Everything binding
lives under `standard/`.

> **What "conformant" means, precisely.** Your runtime is conformant iff it:
> 1. **reproduces every golden ContentId** from its `(domain, body)` — the addresses
>    everyone must agree on; and
> 2. **rejects every core negative** input with the matching reason — the non-canonical
>    bytes everyone must refuse.
>
> Those two obligations, checked against the frozen vectors, are the whole gate.

## Why this is the highest-value path (G2)

ARVES's survivability property is that **anyone can implement and certify a runtime
from the published Standard alone.** Independence is graded:

- **G1 (same-process / same-family):** an implementation written with maintainer help,
  or by the same team/model family. The repo already has G1 — the independent Python
  runtime under `verification/independent/python/`.
- **G2 (third-party):** *a genuine outside team, using ONLY `standard/`, builds a
  conformant runtime and certifies it with no questions asked.* **This is the open
  exit gate.** Reaching it is what turns ARVES from "a well-designed project" into a
  living standard that outlives its makers.

You reaching a CERTIFIED verdict from this guide, without contacting anyone, **is** the
G2 proof.

## Prerequisites

- Your language + its standard library. You need exactly two primitives:
  - **SHA-256** (FIPS 180-4) — for the content address.
  - **Unicode NFC normalization** — for full conformance (optional; see the `nfc`
    tier below — a runtime without it can still be *core*-conformant if it declares
    the deferral).
- No network, no ARVES services, no license key. The whole procedure is offline.

Everything you read is under `standard/`:

| File | What it gives you |
|---|---|
| `standard/acs/` | The five normative Core Standards (ACS-001..005, RFC 2119). |
| `standard/vectors/acs_golden_vectors.tsv` | The positive targets: `standard, vector, domain, body_hex, content_id`. |
| `standard/vectors/acs_negative_vectors.tsv` | The negatives: `standard, case, tier, input_hex, reject_reason`. |
| `standard/conformance/CONFORMANCE.md` | The language-neutral check procedure + the stable reason codes. |
| `standard/README.md` | The Standard Kit overview and independence test. |

## Step 1 — Read the specs (in this order)

1. **ACS-005** (`acs/ACS-005_*`) — normative language (RFC 2119 keywords) + glossary +
   the registered invariants as keyworded requirements. Read this first so you know
   what MUST / SHALL / MAY mean here.
2. **ACS-001** (`acs/ACS-001_Content_Addressing.md`) — the content address:
   `ContentId = 0x12 0x20 || SHA-256(domain_tag || body)` (a self-describing SHA-256
   multihash) + the domain-tag registry (§4).
3. **ACS-002** (`acs/ACS-002_Canonical_Serialization.md`) — deterministic CBOR:
   shortest integers, always-binary64 floats, NFC text, bytewise-sorted map keys,
   definite lengths, and the §5.10 max-depth bound. This is the substantive part.
4. **ACS-003 / ACS-004** — the canonical envelope and the type registry/schema
   encoding (both are dCBOR maps built on ACS-002).

## Step 2 — Implement, in your language

Implement two things. The address is tiny; the canonical serializer/decoder is the
real work.

**A. Addresser (ACS-001).** `ContentId = 0x12 0x20 || SHA-256(domain_tag || body)`,
where `domain_tag` is a single byte and `body` is the raw payload bytes. Reproduce the
domain-tag registry (ACS-001 §4).

**B. Canonical serializer + decoder (ACS-002).** The serializer turns a logical value
into the one canonical `body`. The decoder does the reverse **and rejects any byte
string that is not itself canonical** — this is not optional. Without a rejecting
decoder, two runtimes could accept different encodings of "the same" value and disagree
on its address, breaking interoperability. Validate canonical form **inline while
parsing**; never pattern-match whole test inputs.

The decoder MUST emit these exact stable reason codes (from `CONFORMANCE.md`, sourced
to ACS-002 §5):

```
non-shortest-int   non-shortest-len   indefinite-length   unsorted-map-keys
duplicate-map-keys float-not-float64  negative-zero-float non-finite-float
trailing-data      reserved-or-unsupported   truncated    nesting-too-deep
non-nfc-text
```

## Step 3 — Self-check against the frozen vectors

Run the two checks from `CONFORMANCE.md` over `standard/vectors/`:

1. **Positive (ACS-001):** for every row of `acs_golden_vectors.tsv`, compute the
   ContentId from `(domain, body_hex)` and assert it equals `content_id`. All 12 rows
   MUST pass.
2. **Core-reject (ACS-002):** for every `core`-tier row of `acs_negative_vectors.tsv`,
   feed `input_hex` to your decoder and assert it rejects with the exact
   `reject_reason`. All 16 core rows MUST pass. (There is also 1 `nfc`-tier row — see
   below.)

Your runtime is **core-conformant** iff both are full (`positive 12/12`,
`core-reject 16/16`).

### Copy the worked example

You do not have to invent the runner. A **self-contained, dependency-free** thin
runner already exists — copy it and port it:

- **[`verification/independent/reference-runner/run.mjs`](../verification/independent/reference-runner/run.mjs)**
  — reads both TSVs, checks positives + core-rejects, prints
  `positive N/N, core-reject M/M -> CERTIFIED/NOT`, exits `0`/`1`. About 300 lines,
  Node built-ins only.
- **[`verification/independent/reference-runner/README.md`](../verification/independent/reference-runner/README.md)**
  — how to run it and how to adapt it to your language.

Two more full ports to study side by side:

- **Python** — `verification/independent/python/` (`acs001_address.py`,
  `acs002_decode.py`), a complete Kit-only implementation.
- **Rust reference** — `cargo run -p arves-conformance --bin conformance`.

### The `nfc` tier (core vs full conformance)

There is one `nfc`-tier negative vector (`non-nfc-text`). If your language has NFC
normalization, enforce it — you are then **fully conformant**. If it does not, you MAY
**defer** that single rule and remain **core-conformant** — but you MUST *declare* the
deferral (e.g. `VERDICT: CONFORMANT (ACS core; nfc-tier DEFERRED)`) and MUST NOT
silently accept non-NFC text. See the "core vs full conformance" section of
`CONFORMANCE.md`. The certification verdict counts only the `core` tier, so a deferring
runtime still certifies.

## Step 4 — Get certified

Certification is run by the **maintainer-independent harness**, which drives any
runtime over the same frozen vectors and prints the same verdict shape — no reference
source, no maintainer judgement:

- **[`verification/certification/certify_runtime.py`](../verification/certification/certify_runtime.py)**

Wire your runtime in as an adapter (the script already shows two: a Rust adapter that
drives shipped binaries over stdin/stdout, and a Python adapter that imports the module
directly). An adapter is two functions:

- **addresses(golden):** given the golden rows, return your runtime's ContentId hex for
  each `(domain, body)`.
- **rejects(negatives):** given the negative rows, return `(verdict, reason)` for each
  input, where `verdict` is `REJECT`/`ACCEPT` and `reason` is your decoder's stable
  reason code.

Add a `certify("Your Runtime (vendor)", your_addresses(golden), your_rejects(neg), golden, neg)`
record alongside the existing ones. Run:

```
python verification/certification/certify_runtime.py
```

You are **CERTIFIED** iff you show `positive 12/12  core-reject 16/16  ->  CERTIFIED`.
That verdict, produced from `standard/` + this harness alone, is your independent
certification — no maintainer required, by design (the Foundation survivability
property).

> **Registry allocation.** If you need a new domain tag, hash code, or reason code,
> that is a normative registry change (ACS-001 §4.1 / `CONFORMANCE.md`) — file it
> through the change process; never invent one silently. Certification is against the
> **current frozen registries**.

## Reconciling the other docs

The DX audit (`verification/dx/DEVELOPER_JOURNEY_REPORT.md`, friction #2) found that the
runtime-authoring path was described inconsistently across pages. **This guide is now
the single source.** The other pages are correct pointers to it, and you should not
follow a different procedure from any of them:

- **`CONTRIBUTING.md`** — names the *Runtime vendor* role: "implement `standard/` only;
  certify with `verification/certification/certify_runtime.py`." That is exactly Steps
  2–4 here. Do not edit `CONTRIBUTING.md`; follow this guide for the how.
- **`standard/README.md`** — the Standard Kit overview and the independence test
  ("if you need the reference runtime source, the Kit has failed"). Its "How to
  implement ARVES" list is Steps 1–3 here, in more detail.
- **`standard/conformance/CONFORMANCE.md`** — the language-neutral check procedure and
  the authoritative reason codes and verdict semantics (core vs full). This guide
  points you at it for Step 3; it is normative for the exact checks and codes.

If any of those and this guide ever appear to disagree on *procedure*, follow this
guide and report the discrepancy as a docs gap (it must not happen — the point of this
guide is that there is one procedure).

## The bottom line

**G2 — an outside team building and certifying its own runtime from `standard/` alone,
with no help — is the open exit gate for ARVES.** This guide + the worked runner +
the certification harness are everything that team needs. Reaching
`positive 12/12  core-reject 16/16  ->  CERTIFIED` from these files, without asking a
single question, is the proof that ARVES can live independently of its makers.

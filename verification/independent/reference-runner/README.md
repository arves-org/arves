# Thin conformance runner — the artifact a new runtime vendor copies

`run.mjs` is a **self-contained, dependency-free worked example** of the smallest
thing that proves an ACS implementation conforms to the frozen ARVES Standard: it
reproduces every golden ContentId and rejects every core-tier non-canonical input,
then prints a one-line verdict and exits `0`/`1`.

It is the concrete starting point for the **G2 on-ramp** — an outside team building
and certifying its own runtime from `standard/` alone. Read the full procedure in
[`standard/RUNTIME_AUTHORS_GUIDE.md`](../../../standard/RUNTIME_AUTHORS_GUIDE.md).

## Run it

```
node verification/independent/reference-runner/run.mjs
```

Expected output (real, from this repo):

```
ARVES thin conformance runner — certifying against the frozen Standard alone
============================================================================
  positive 12/12  (ACS-001 ContentId reproduced from domain+body)
  core-reject 16/16  (ACS-002 non-canonical inputs rejected with the right reason)
----------------------------------------------------------------------------
  VERDICT: CERTIFIED (ACS core)
  Reproduced every golden address and rejected every core negative — no maintainer required.
```

Exit code `0` = CERTIFIED, `1` = NOT CERTIFIED. Node built-ins only — no
`npm install`, no network. It works fully offline.

## What it does (and why each half exists)

The runner runs the two checks from
[`standard/conformance/CONFORMANCE.md`](../../../standard/conformance/CONFORMANCE.md)
over the frozen vectors in `standard/vectors/`:

1. **Positive (ACS-001).** For each row of `acs_golden_vectors.tsv`, compute
   `ContentId = 0x12 0x20 || SHA-256(domain || body)` from the given `(domain, body_hex)`
   and assert it equals the published `content_id`. This is the "you produce the
   same addresses as everyone else" half.
2. **Core-reject (ACS-002).** For each `core` row of `acs_negative_vectors.tsv`,
   run a **canonical decoder** over `input_hex` and assert it rejects with the exact
   `reject_reason`. This is the "you refuse the same non-canonical bytes as everyone
   else" half — without it, two runtimes could accept different encodings of the same
   value and disagree on its address.

The runner deliberately shows **both implementation styles** so you see each once:

| Concern | Style in `run.mjs` | Note |
|---|---|---|
| Addressing (ACS-001) | **imports** the reference SDK codec (`products/arves-sdk-ts/src/codec.mjs`) | "reuse a trusted impl" |
| Rejection (ACS-002 canonical decoder) | **implemented inline** from the spec | "write it yourself" |

A real vendor typically writes **both** in their own language. The address is tiny
(`0x12 0x20 || SHA-256(domain || body)`); the decoder is the substantive part.

## Adapt it for your language

`run.mjs` is intentionally ~300 lines so it ports cleanly. To build the equivalent
runner in your language:

1. **Load the vectors.** Read the two TSVs from `standard/vectors/`. Never hardcode
   the values — read the frozen files so you track them as the Standard evolves.
2. **Addresser (ACS-001).** Implement `ContentId = 0x12 0x20 || SHA-256(domain_tag || body)`
   using your platform's FIPS 180-4 SHA-256. `domain_tag` is one byte; `body` are the
   raw bytes of `body_hex`. Assert the hex of the result equals the golden `content_id`.
3. **Canonical decoder (ACS-002).** Port the inline decoder. The reason codes are the
   contract — emit these exact strings (from `CONFORMANCE.md`):
   `non-shortest-int`, `non-shortest-len`, `indefinite-length`, `unsorted-map-keys`,
   `duplicate-map-keys`, `float-not-float64`, `negative-zero-float`, `non-finite-float`,
   `trailing-data`, `reserved-or-unsupported`, `truncated`, `nesting-too-deep`,
   `non-nfc-text`. Validate canonical form **inline while parsing** — never
   pattern-match whole test inputs.
4. **Verdict.** Count positive hits and core-reject hits; print `positive N/N`,
   `core-reject M/M`, and `CERTIFIED` iff both are full. Exit `0`/`1`.

Cross-language reference ports living in this repo (study these next to `run.mjs`):

- **Python** — `verification/independent/python/` (`acs001_address.py`,
  `acs002_decode.py`): a full independent implementation, Kit-only.
- **Rust** — the reference runtime's conformance binary
  (`cargo run -p arves-conformance --bin conformance`).

## The `nfc` tier

`run.mjs` enforces the one `nfc`-tier vector too (Node has
`String.prototype.normalize`), so it is **fully conformant**, not just core. A runtime
with no Unicode NFC facility MAY **defer** that single rule and still be *core*-conformant
— but it must **declare** the deferral, never silently accept non-NFC text. See the
"core vs full conformance" section of `CONFORMANCE.md`. The certification verdict here
counts only the `core` tier, so a deferring runtime still shows `core-reject M/M`.

## From this runner to certification

This runner is the self-check. To be **certified** as a runtime under the maintainer-
independent harness, wire your runtime into `verification/certification/certify_runtime.py`
(it drives runtimes over the same TSVs and prints the same
`positive N/N, core-reject M/M -> CERTIFIED` shape). The full end-to-end procedure —
from "clone the Standard" to a certification verdict, with no help from the ARVES
maintainers — is [`standard/RUNTIME_AUTHORS_GUIDE.md`](../../../standard/RUNTIME_AUTHORS_GUIDE.md).

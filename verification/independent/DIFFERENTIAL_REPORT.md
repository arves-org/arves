# ARVES Differential Validation Report #1 — Rust ↔ Python

**Program phase:** Independent Runtime (Phase 1) + Differential Validation (Phase 2).
**Claim under test:** the ARVES Standard Kit is self-sufficient — two implementations,
written independently in different languages with no shared code, produce the same
bytes and the same content addresses.

## Implementations compared
| # | Runtime | Language | Author | Inputs used |
|---|---------|----------|--------|-------------|
| A | `arves-acs` (reference) | Rust, dependency-free (hand-rolled SHA-256 + dCBOR) | reference team | the ACS specs |
| B | `verification/independent/python/` | Python 3, stdlib | an **independent agent with a fresh context, forbidden from reading any Rust source**, using ONLY `standard/` | the Standard Kit alone |

## Result — DIFFERENTIAL PASS

Both A and B reproduce **all 12 ACS golden vectors byte-for-byte** — identical
canonical `body` bytes AND identical `ContentId`s — for ACS-001, ACS-002, ACS-003,
ACS-004 (instance + 430-byte schema), and ACS-005 (tags 0x08/0x09). Each also emits
`VERDICT: CONFORMANT (ACS layer)` from its own conformance runner over
`standard/vectors/acs_golden_vectors.tsv`.

## Why this is genuine independence (not one impl copying the other)
- B was built by a fresh-context agent that never opened `runtime/` or any `.rs`;
  it derived every logical value from the ACS prose, not from the target hashes.
- **Anti-reverse-engineering evidence from B:** it authored the ACS-002 V3 `label`
  in Unicode **NFD** (decomposed `e`+U+0301) and its NFC step produced the
  precomposed `c3a9`, matching the target — proving real §5.4 normalization, not a
  copied byte string; and it **randomly shuffled** the ACS-004 instance's map keys
  and still produced the identical 264-byte body — proving real encoded-key sorting.
- The 430-byte ACS-004 schema was reconstructed **independently twice** (A's Rust
  runner and B's Python) from the field table, both hashing to `1220…6b3f99c6`.
- **Anticipation:** when the shared TSV was single-sourced from 11 to 12 (adding the
  schema row, SD-002), B's conformance runner reached **12/12 with ZERO code
  changes** — it had pre-written the schema builder from ACS-004 §11.2, anticipating
  the row. Both runners now emit `12/12 PASS · CONFORMANT`.

## Standard-Defects surfaced by B (and dispositioned)
- **SD-001** (ACS-005 §6.1 vs §9.2 clause wording diverged) — FIXED: §9.2 byte form
  declared the canonical addressed clause text.
- **SD-002** (golden-vector inventory split 11/12/+6 across TSV/.md/spec) — FIXED:
  the reference runner now single-sources **12** addressed vectors (added the ACS-004
  schema row); TSV == the human table. ACS-002 §8.2 scalars remain body-level encode
  checks (reference unit tests), not addressed rows.
- **SD-003** (ACS-005 §9.1 list "sorted" is a latent no-op) — NOTED normatively:
  the list must be maintained sorted.
- B reported **no missing algorithm, byte rule, or undefined value** — the Kit was
  complete on every load-bearing question.

## Conclusion (Report #1 — encode/address)
For the **ACS interoperability layer**, the ARVES Standard Kit is **proven
self-sufficient**: an independent team reproduced it exactly from the Kit alone, and
its output is differentially identical to the reference. This is the first
Independent-Runtime + Differential proof in the program.

---

# Differential Validation #2 — decode + rejection (ACS-002)

**Claim under test:** a conformant implementation must agree not only on what to
**produce** but on what to **reject**, and two independent implementations must never
disagree on whether a byte string is canonical.

## What was added
- A canonical **rejecting decoder** in both implementations: Rust
  `arves_acs::cbor::decode_canonical` and the independent Python `acs002_decode.decode`
  (Kit-only authorship). Each accepts a byte string only if it is in the exact
  canonical form and returns a stable ACS-002 reason code otherwise.
- The Kit gained `vectors/acs_negative_vectors.tsv` — **17 rejection vectors**
  (16 `core` + 1 `nfc`-tier) — and `conformance/CONFORMANCE.md` gained the rejection
  procedure and the core-vs-full **verdict-tier semantics**.

## Result — DIFFERENTIAL PASS (both directions)
- **Negative vectors:** the reference rejects **15/15 core** with matching reasons
  (+1 nfc-tier deferred, documented); the independent Python — reading the same TSV,
  **zero code changes** — rejects **16/16** (it enforces `nfc` via stdlib
  `unicodedata`). Positive round-trip still holds (3/3).
- **Differential fuzzer** (`verification/differential/acs002_differential_fuzz.py`,
  deterministic seed): **13,807 inputs** (canonical values + byte-level mutations +
  random) fed to *both* decoders. Result: **0 hard divergences** — 3,135 accept/accept
  with byte-identical re-encoding, 10,672 reject/reject (10,656 same reason; 16
  multi-violation inputs where the two report a different but each-valid first
  violation — permitted, ACS-002 mandates no check order, and both still REJECT).

## Adversarial red-team (Rule 5/6 — destroy own work)
A 5-lens red-team (false-accept, false-reject, differential-divergence,
kit-self-sufficiency, evidence-integrity), each finding independently verified, raised
**17 candidates → 7 confirmed**. All 7 were dispositioned:
- **[major] Integer range** — the reference `Value::Int(i64)` wrongly rejected in-model
  integers in `[i64::MAX+1, 2^64-1]` / `[-2^64, i64::MIN-1]` that ACS-002 §4 admits
  (`[-2^64, 2^64-1]`). **Fixed:** widened to `i128`; the widened range now round-trips
  and matches the independent Python.
- **[major] Map key kind** — the decoder accepted maps with Null/Bool/Float/Bytes/Array
  keys (`a1f600`), which §4 kind 8 forbids. **Fixed:** keys must be Text or Integer
  (`reserved-or-unsupported`), matching the Python decoder.
- **[minor] non-UTF-8 text** reported an unregistered code `invalid-utf8`. **Fixed:**
  folded into `reserved-or-unsupported` (the Kit's §4 catch-all, matching Python).
- **[minor] top-level `0xff` break** reported `reserved-or-unsupported`. **Fixed:** now
  `indefinite-length`, matching Python and §5.1.
- **[minor] tag with non-shortest argument** reported `non-shortest-int`. **Fixed:**
  major 6 rejected up-front as `reserved-or-unsupported`.
- **[minor] Kit prose** — ACS-002 §8.1 worked-example sort-order parenthetical
  contradicted the normative §5.6 rule and the golden body. **Fixed** in the Kit.
- **[minor] nfc-tier verdict semantics** were unspecified (the two runners diverged on
  how to report a deferral). **Fixed:** CONFORMANCE.md now defines core-conformant
  (nfc may be deferred, must be declared) vs fully-conformant (nfc enforced) tiers.

The 3 new decoder bugs (map-key, utf8, break) were each turned into a shared negative
vector; the independent Python already rejected all three with the exact reasons the
fixes aligned Rust to — direct confirmation the reference now matches a spec-faithful
peer.

## Standard Validation Era — Destroy Round 1 (six offices)

After entering the Standard Validation Era, six destroy-offices (Security, Scientific,
Academic, Standards/IETF, Independent-buildability, Robustness) attacked the codec and
standard; findings were independently verified (17 candidates → 7 confirmed). Confirmed
and **fixed** this round:
- **[blocker, Security+Robustness] Unbounded decode recursion → stack-overflow DoS**
  (`0x81`×5000+`0x00` aborted the process). Fixed: normative **ACS-002 §5.10
  `MAX_DEPTH = 128`**, enforced (reject `nesting-too-deep`) in *both* the Rust and the
  independent Python decoder, plus a shared negative vector — the 16th core vector.
- **[major, Buildability] ACS-004 `aspects` array order** was address-bearing but
  unspecified (prose swapped Trust/Temporal). Fixed: normative order clause (ACS-004 §8).
- **[major, Buildability] ACS-001 §5** stated the address three ways; one read as a
  double hash. Fixed: single-SHA-256 wording.
- **[major, Standards+Security] No Security Considerations.** Added ACS-002 §11
  (hostile-decoder model, SHA-256 reliance + multihash agility, NFC caveat).
- **[major, Scientific] Over-claim** that ACS "proves" ORCH-003/004. Corrected to
  *precondition, not proof* (ACS-004 §14).
- **[major, Scientific] NFC version unpinned.** Pinned Unicode 16.0.0 (ACS-002 §5.4).

Both decoders now reject the depth bomb (`17/17` Python; `16/16` core Rust); the
differential fuzzer re-ran clean (**13,807 inputs, 0 hard divergences**). Tracked for
Round 2 (Evidence Ledger §B): IANA/registry policy, `dCBOR` naming citation, a
quantitative ablation harness, more worked types, and NFC full-enforcement in the
dependency-free reference.

## Conclusion (Report #2)
The ACS-002 codec is now **differentially validated in both directions** (encode and
decode) across two independent implementations, over a curated 17-vector rejection
corpus and ~13.8k fuzzed inputs, after two adversarial passes (red-team + Destroy
Round 1) that found and fixed three real reference bugs (integer range, map-key kind,
recursion DoS) and hardened the standard. Evidence Level for the ACS-002 codec:
**L3 — but independence grade G1** (same-process); the G2 third-party gate is the
frontier. Nothing here is "Done"; it holds a level.

**Not yet covered (honest scope):** runtime-behaviour conformance (the 12 Scenario
axes, L1..L4 over a live Kernel/Query/Engine), and a second full *runtime* (not just
codec) in a third language. Those are later phases. The residual 16 reason-code
mismatches are interop-safe (both reject) and expected; reason agreement is guaranteed
only on the single-violation curated corpus.

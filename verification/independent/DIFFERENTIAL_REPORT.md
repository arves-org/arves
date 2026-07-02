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

## Conclusion
For the **ACS interoperability layer**, the ARVES Standard Kit is **proven
self-sufficient**: an independent team reproduced it exactly from the Kit alone, and
its output is differentially identical to the reference. This is the first
Independent-Runtime + Differential proof in the program.

**Not yet covered (honest scope):** runtime-behaviour conformance (the 12 Scenario
axes, L1..L4 over a live Kernel/Query/Engine), a second *runtime* (not just codec)
in a third language, and rejection/negative vectors. Those are later phases.

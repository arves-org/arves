# ARVES Standard Kit

**Everything needed to implement and conformance-check the ARVES interoperability
core — with NO access to the reference runtime source.** This is the publishable
artifact that turns ARVES from "a well-designed project" into an implementable
standard.

> **Independence test.** If, to build a conformant implementation, you need to read
> the Rust reference runtime, this Kit has failed its purpose. Everything binding
> lives here (specs + schemas + golden vectors + conformance procedure). Report a
> gap as an ambiguity in the relevant ACS.

## Contents

| Dir | What |
|-----|------|
| `acs/` | The five ARVES Core Standards (normative, RFC 2119): ACS-001 Content Identity · ACS-002 Canonical Serialization (dCBOR) · ACS-003 Canonical Envelope · ACS-004 Universal Type Registry · ACS-005 Normative Language + Glossary. Plus the batch consistency report. |
| `vectors/` | Golden vectors: `acs_golden_vectors.tsv` (machine-readable: standard, domain, body_hex, content_id) + `acs_golden_vectors.md` (human table). The byte-exact targets any implementation must reproduce. |
| `conformance/` | `CONFORMANCE.md` — the language-neutral procedure to self-check an implementation and emit an ARVES Conformance Report. |
| `certification/` | The certification process (L1..L4) and how a third-party runtime gets certified. |

## How to implement ARVES (from this Kit alone)

1. Read `acs/ACS-005` first (normative language + glossary + the 7 registered
   invariants as keyworded requirements).
2. Implement, in your language:
   - **ACS-001** — content address `ContentId = 0x12 0x20 ‖ SHA-256(domain_tag ‖ body)`
     (self-describing SHA-256 multihash) + the domain-tag registry.
   - **ACS-002** — deterministic CBOR (`body`): shortest ints, float64, NFC text,
     bytewise-sorted map keys, definite lengths.
   - **ACS-003** — the canonical envelope (a dCBOR map; `payload_cid` = ACS-001
     address of the dCBOR payload).
   - **ACS-004** — the type registry / schema + instance encoding.
3. Run the conformance procedure (`conformance/CONFORMANCE.md`) against
   `vectors/acs_golden_vectors.tsv`. Reproduce every `body_hex` (encoder) and every
   `content_id` (addresser).
4. If all vectors PASS, you have a conformant ARVES ACS implementation — verified
   independently, not by comparison to anyone's source.

## Status
- **Rust reference:** all vectors PASS (`cargo run -p arves-conformance --bin conformance`
  → VERDICT: CONFORMANT). The reference computes these with a dependency-free
  SHA-256 + dCBOR, so this Kit is derivable from the spec, not from a library.
- **Independent runtimes (Go / Java / Python / third-party):** ⬜ — the point of
  this Kit. Reproducing the Conformance Report from the Kit alone is the
  Independent-Runtime proof.

## Scope
This Kit is the **interoperability / identity layer** (content addressing,
serialization, envelope, types, language) — the bytes every ARVES implementation
must agree on. The full cognitive runtime (Kernel, LCW, Query, Engine, Capability,
Execution) is the **ARVES Reference Runtime** (`runtime/`), a *consumer* of this
standard, not part of the Kit.

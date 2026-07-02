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
3. Also implement a canonical **decoder** that REJECTS non-canonical input, per
   `conformance/CONFORMANCE.md` "rejection check" (incl. the §5.10 max-depth bound).
4. Run the conformance procedure against `vectors/acs_golden_vectors.tsv` (reproduce
   every `body_hex` + `content_id`) AND `vectors/acs_negative_vectors.tsv` (reject
   every `core` row with the matching reason). If both PASS, you have a conformant
   ARVES ACS implementation — verified independently, not by comparison to any source.

## Status
- **Rust reference:** positive + negative vectors PASS
  (`cargo run -p arves-conformance --bin conformance` → CONFORMANT), dependency-free
  SHA-256 + dCBOR — so this Kit is derivable from the spec, not from a library.
- **Independent runtimes:** Python (Kit-only) reproduces both directions —
  **independence grade G1 (same-process)**. Go / Java / **third-party** ⬜.
- **The exit criterion (G2):** *can a completely unknown team, using ONLY this Kit,
  build a conformant runtime without asking a single question?* Reproducing the
  Conformance Report from the Kit alone, by someone who did not help write it, is the
  proof. Until then, independence is honestly **G1**, not "independent."

## Scope
This Kit is the **interoperability / identity layer** (content addressing,
serialization, envelope, types, language) — the bytes every ARVES implementation
must agree on. The full cognitive runtime (Kernel, LCW, Query, Engine, Capability,
Execution) is the **ARVES Reference Runtime** (`runtime/`), a *consumer* of this
standard, not part of the Kit.

## Version & release
- **Kit version:** see `VERSION` (`arves-standard-kit 0.2.0`). Semantics: the Kit is
  versioned as a unit; each ACS carries its own version tag (e.g. `ACS-002/1`). A
  byte-affecting change is a new ACS profile via CCP, never a silent edit (ED-001).
- **Self-contained:** everything binding is under `standard/`. This directory is the
  publishable artifact — it can be copied out of the repo and released on its own; it
  references no file outside itself and no reference-runtime source.
- **Registries** (domain tags, hash codes, reason codes) and their allocation policy
  are normative in `acs/ACS-001 §4.1` and `conformance/CONFORMANCE.md`.

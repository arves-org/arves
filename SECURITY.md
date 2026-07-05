# Security Policy

## Reporting a vulnerability

**Please do not open a public issue for a security vulnerability.** Report it privately so it can
be triaged and fixed before disclosure:

- Use **GitHub's private vulnerability reporting** for this repository
  (*Security → Report a vulnerability*), or
- contact the maintainers privately <!-- maintainer: set a dedicated security contact address before publishing -->.

Please include: the affected component (e.g. ACS-002 decoder, `arves-bridge`, Kernel commit path,
capability/marketplace signature binding), a minimal reproduction, and the impact you observed.
We aim to acknowledge a report quickly and will credit reporters who wish to be named.

## Supported version

| Version | Supported |
|---|---|
| Runtime **v1.0** (tag `runtime-v1.0`, FROZEN) | ✅ security fixes via a Runtime Change Request (RCR) |
| pre-v1.0 / unreleased | ❌ |

The runtime is frozen and byte-stable; a security fix is a legitimate reason to open an RCR
(`runtime/RUNTIME_FREEZE_v1.0.md`) — it does not bypass the freeze, it uses the sanctioned
instrument.

## Threat model — the honest scope of v1.0

ARVES v1.0's guarantees are **identity, determinism, replay, and audit on a trusted host**. Read
this before deploying, so the security properties are not over-read:

- **Trusted single host.** v1.0's threat model is a trusted single host. `Kernel::commit` carries
  **no principal / authentication / authorization**; a multi-tenant or untrusted-host deployment
  is out of scope for v1.0.
- **Persistence integrity is error-detection + a tamper-evident chain, not full cryptographic
  tamper-resistance.** The WAL/snapshots use CRC32 (error-detection, forgeable). RCR-002 added a
  dependency-free **SHA-256 tamper-evident hash-chain digest** (`FileWal::integrity_digest`) that
  detects any alteration of any committed record — including one that repairs the per-frame CRC32.
  Still **open (tracked as v2.0 RCR debt #8):** cryptographic **signatures**, **authenticated
  commit** (principal/authN on `Kernel::commit`), and digest **anchoring**. A fully hostile host
  that rewrites the whole trace *and* the anchor still needs signatures to stop.
- **Do not imply cryptographic tamper-resistance of the persisted store under v1.0** in public
  materials. See the independent review `runtime/docs/reviews/P07_security-zero-trust.md`.

What **is** hardened in v1.0 (and where a regression would be a security bug worth reporting):
the encoders are depth/range-bounded (`MAX_DEPTH`), the ACS-002 decoder rejects every
non-canonical input with a stable reason, the bridge fails safe on a missing/dead process and
refuses protocol injection, and capability/marketplace install binds signed code + advertised
identity (a valid artifact cannot be served under a different manifest). The whole stack survived
a six-lens destroy pass, regression-locked in `products/robustness.test.mjs` and the Rust
workspace tests.

## Disclosure

We follow coordinated disclosure: report privately, we fix under an RCR with a regression test,
then the fix and an advisory are published together. Thank you for helping keep ARVES honest.

# Maintainer Rulings — recorded era/governance decisions

> The Engineering Constitution (`CLAUDE.md`) says eras and gates are **maintainer-set** and that
> changes are never silent. This file is the running record of those rulings, in order. Each
> ruling names what changed, why, and what explicitly did NOT change.

## Ruling 002 — 2026-07-05 · Industrialization un-gated; purpose reframed to internal product use

**Decided by:** the maintainer (explicit instruction, same day as the arves-org publish).

**What changed:**

1. **The Industrialization Era (I2..I6) is UN-GATED.** The build may proceed now, from the five
   adversarially-reviewed design packages under `docs/design/` (I2 Cluster Kernel → I3 Distributed
   Query → I4 Capability Scheduling → I5 Multi-Agent Runtime → I6 Reference Products), each under
   the full 15-step constitutional workflow. The prior rule — *"further implementation (I2–I6) is
   gated behind [the G2 exit gate]"* — is superseded by this ruling.
2. **Primary purpose reframed:** ARVES's driver is the **maintainer's own product** — the
   maintainer will test the system on a real product built on it. "System ready" now means:
   *the maintainer's product runs on ARVES end-to-end, honestly.* Public adoption / publicity is
   **de-prioritized**: no announcement push, no funnel campaign ("amacımız halka açmak değil").
3. **Standard Validation continues in parallel** (two-arm model retained): the repo stays public,
   CHALLENGE stays open, and CI referees every push. A genuine G2 arrival would still be graded
   and celebrated honestly — it is simply no longer the thing the program waits on.

**What did NOT change (explicitly):**

- **Independence grading is untouched.** Everything internal remains **G1**; nothing this ruling
  enables may ever be presented as G2. Agent-built work is by definition same-program (G1).
- **Freeze discipline is untouched.** `standard/` changes via CCP; `runtime/` via RCR (each with
  destroy→prove and an in-instrument freeze re-baseline); the frozen spec corpus via
  CCP/regeneration. I2..I6 lands as RCRs into the runtime workspace with the architecture gate,
  invariants (OWN-001 · LAYER-001 · SHARD-001 · ORCH-001..004), and IDR-001..005 fully binding.
- **Honest evidence discipline is untouched.** The Evidence Ledger stays drift-proof; the
  dashboard's external zeros (G2, adoption) remain zeros until real external events occur.

**Rationale:** the "prove-wrong-first" pivot assumed public validation was the goal. The
maintainer's actual goal is a working product on a sound substrate; the validation machinery
(conformance, differential fuzz, sound gate, CI) continues to do its falsification job *while*
the distributed platform is built, rather than blocking it.

## Ruling 001 — 2026-07-05 (earlier the same day) · Ch4 prep-mode + publish authorization

For the record: earlier the same day the maintainer ruled Ch4 into **prep mode** (design packages
only, gate closed) and authorized the **publish** (org `arves-org`, push + Pages + release).
Ruling 002 supersedes the prep-mode half; the publish stands.

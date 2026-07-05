# ARVES Strategic Program — "bigger · faster · stable" (2026-07-04)

> **What this is:** the synthesized program plan for progressing ARVES faster and larger *without*
> breaking freeze discipline or laundering G1 into G2. Produced by a multi-agent strategic review
> (7 parallel subsystem reviews → 3 lens proposals [evidence-first · g2-external-first ·
> conformance+product-first] → 1 Chief-Architect synthesis). **Grade of this plan: G1 (self).**
> It creates no evidence and changes no frozen byte; each item routes through its sanctioned
> instrument (CCP / RCR / freeze-clean living_fix / external).
>
> **Status (updated 2026-07-05):** execution largely COMPLETE. Done: Wave 0 (ranks 1–4, 9–11),
> the live-conformance triple (ranks 6–8, RCR-008/009/010), **rank 5** (TLC unblocked via a
> portable JRE — `SafetyInv` + `EventuallyCommitted` HOLD, `verification/formal/TLC_RUN.md`),
> rank 12 (semantic differential 62/62), rank 14 (E1/E2), rank 15 (certify-YOUR-runtime path),
> rank 19-A (3-way TS fuzz 13807/0), rank 20 (capstone). Remaining: **r16 PUBLISH** (maintainer:
> org/name + push — the only unchecked pre-publish item), r17 CI-host, r18 external funnel,
> r19-B (Connector/Query promote RCR — maintainer-gated). *(Update: r16 PUBLISH and r17 CI-host
> have since LANDED — Ruling 001 authorized the publish; see `OPEN_DEBT_REGISTER.md` §G/§F #17.)*
> ~~Ch4 (I2..I6) is in **prep mode** by maintainer ruling (2026-07-05): design packages only
> (`docs/design/`), build gate (G2) closed.~~ *(Superseded by Ruling 002, later the same day,
> `docs/MAINTAINER_RULINGS.md`: I2..I6 is UN-GATED; construction proceeds from the reviewed
> design packages via RCR under the full 15-step discipline; independence stays G1.)*
> Live tracker: `verification/OPEN_DEBT_REGISTER.md`.

---

## The decisive finding (why rank 1 was rank 1) — ✅ RESOLVED (rank 1, commit 3c08ff5)

> **RESOLVED:** the gate now grades the **full ACS-001..005 surface** — all 19 semantic vectors
> plus semantic accept-probes — and the verdict is coverage-labeled (`SOUND-CERTIFIED (full
> ACS-001..005 surface)` vs `(ACS core; semantic DEFERRED)`), never printed unqualified. The
> drift-proof probe (row `sound-certified`) FAILS if the gate ever degrades to core-only. The
> paragraph below is preserved as the historical planning-time finding.

The flagship **SOUND-CERTIFIED** gate that `CHALLENGE.md` §4/§5 names as *the G2 win-condition*
graded only `tier=="core"` at planning time (`verify_runtime_sound.py:111`;
`certify_runtime.py:105`) — **0 of the 19 ACS-003/004/005 semantic reject vectors** that
CCP-006/007 added, and RCR-004 proved a runtime *can* reject. So a genuine external pass on the
gate **as then written** would have attested only the ACS-001/002 byte layer while the stamp
implied the whole standard — a program-defining **over-claim of the exact B1 defect ARVES exists
to prevent.** The whole critical path was therefore: **make the gate attest the whole standard,
de-drift the cold-start packet, and open the front door BEFORE publishing** — everything else
parallelized to raise honestly-graded Evidence Level.

## North star (unchanged)

The single open **exit gate is G2**: a genuinely unrelated external party certifies a runtime from
`standard/` alone, zero maintainer help, earning SOUND-CERTIFIED. No internal work manufactures it —
independence stays honestly capped at **G1** (`EVIDENCE_LEDGER` Section C: "Third-party runtime — G2
— NOT YET MET") until a stranger actually passes. Era KPI = **Evidence Increased**: raise the
honestly-graded Evidence Level of the ground a stranger stands on, so a G2 event is both *possible*
(the ground holds) and *meaningful* (the stamp attests the whole standard, not just interop bytes).

## Guardrails (what keeps it stable + honest)

- **Freeze:** `standard/` only via **CCP**, `runtime/` only via **RCR**, never a silent edit;
  `freeze_check.py update` runs *inside* the instrument and re-baselines the manifest at 0 drift;
  living_fix only in `verification/`, `products/`, root docs.
- **Honesty:** independence stays **G1** across every artifact. NOTHING here is a grade raise —
  every L0→L1 populate, captured TLC proof, or in-program differential is an *Evidence-Level* raise
  at G0/G1, never an independence-*grade* raise. Four specific over-claims are guarded: (a) TLC row
  stays "proves the model, not the Rust bytes"; (b) reason-code parity labeled "mapped-then-checked"
  until validators emit native codes; (c) "2 vs 3 runtimes" states the honest distinction; (d)
  publish (rank 16) is HARD-gated behind the full-surface gate so a first G2 pass can't be a B1
  over-claim.
- **Scope:** ~~distributed I2–I6 stays **CONTRACT-ONLY** — the mandate is "prove wrong," not
  "build."~~ *(Superseded by Ruling 002, 2026-07-05: I2–I6 build is un-gated and proceeds via RCR;
  the falsification machinery keeps running in parallel. Freeze/honesty guardrails above unchanged.)*

---

## The 20-item prioritized program

`do_via`: **wf** = parallelizable / fan-out (workflow) · **solo** = focused single change.
`inst`: freeze-clean living_fix (FC) · CCP · RCR · external (EXT) · doc.

| # | Title | inst | eff | KPI | do_via | depends |
|---|-------|------|-----|-----|--------|---------|
| **1** | **Full-surface SOUND gate** — grade ACS-003/004/005 reject tiers inputs-only; SOUND-CERTIFIED requires all 4 tiers (`verify_runtime_sound.py:111`, `certify_runtime.py:105`) | FC | M | g2: gate 4/4 tiers (was 1/4) | wf | — |
| **2** | **RCR-004b** — `acs_validate` line-protocol bin exposing the native Rust semantic validators (`semantic.rs`) | RCR | M | g2: Rust ref = full-surface oracle | wf | — |
| **3** | **De-drift the cold-start packet** — Kit 0.2.0→0.3.1, 35→36 vectors, "2 vs 3 runtimes" honest distinction (IMPLEMENTING_ARVES, products/README, Master Roadmap) | FC | S | hardening: 0 drift in onboarding | solo | — |
| **4** | **One consolidated OPEN-DEBT + ambiguity register** cross-linking SYSTEM_GAP #1-39 ↔ G2_READINESS B1-B4; doubles as the CHALLENGE ambiguity artifact | doc | S | g2: all debt legible in one place | solo | — |
| **5** | **Capture a real TLC run** of the frozen Kernel TLA+ (`ARVES_Kernel.tla` + `_MC.cfg`); flip Formal row L0→mechanically-checked | FC | S | evidence: Formal L0→checked | solo | — |
| **6** | **Live L1 conformance spine + KernelProbe** — first real `ConformanceArtifact` from the frozen Kernel (the 12-axis vocabulary is typed but zero-instantiated) | FC | M | conformance: first live artifact | wf | — |
| **7** | **Reference Connector + InformationPlatformProbe** (axis 1) — canonicalize a Source → provenance/trust/tenant-scope/idempotent ProposedWrite | FC | M | conformance: axis-1 node live | wf | — |
| **8** | **Reference read-only Query projection (WAL-replay) + QueryProbe** — read-only, SHARD-001 tenant-scoped; no Kernel read hook (ORCH-001/OWN-001) | FC | L | conformance: QUERY-001 + read-isolation live | wf | — |
| **9** | **CCP-008 DRAFT** — close B2 (root-event `causation_id` = present-with-Null; matches golden → byte-clean) + 1 negative vector | CCP | M | standard: last G2 blocker drafted | solo | — |
| **10** | **Extend freeze-diff gate to the spec mirror** (`spec-markdown/`), and file V1 (Invariant Registry "no runtime code" mirror) as a CCP regen | FC | S | hardening: gate covers spec mirror | solo | — |
| **11** | **M1** — bind marketplace/registry capability identity to the signed artifact (`cap.manifest` deep-equal `artifact.manifest`) + biting test | FC | S | hardening: certified-code squatting closed | solo | — |
| **12** | **Semantic-layer differential fuzz** — Rust native vs Python must agree on accept/reject + reason code (extends the ACS-002 fuzzer above the byte layer) | FC | M | conformance: reject surface differential, not self | wf | 2 |
| **13** | **First end-to-end L1 Scenario** ("Enterprise Knowledge Query", axes 1+8+9); emit artifact; move the 12-axis ledger row L0→L1 (drift-proof) | FC | M | evidence: the L0 behaviour row → L1 | solo | 6,7,8 |
| **14** | **Product honesty** — E1 authenticated approval truths (retire proposer self-attest) + E2 input hardening; promote product evidence rows to probe-run | FC | L | hardening: E1 closed, product rows drift-proof | wf | — |
| **15** | **"Certify YOUR runtime" quickstart** + sharpen CHALLENGE intake (#37/#36) — zero-help-gradable submission (full-surface SOUND output + ambiguity list) | FC | S | g2: stranger self-certifies + submits | solo | 1 |
| **16** | **Complete RELEASING checklist + PUBLISH** — CODE_OF_CONDUCT/SECURITY, public org, Pages; **HARD-gated behind ranks 1,2,3,15** | EXT | M | g2: repo public + reachable | solo | 1,2,3,15 |
| **17** | **Provision CI host + branch protection** — freeze / evidence-probe / link-gate / cargo / certify / sound (+TLC) Required merge blockers | FC+EXT | M | hardening: gates enforced, not discipline | solo | 1,5 |
| **18** | **External-G2 funnel + leading indicators** — distribute CHALLENGE, recognition path, funnel (fetches→attempts→self-check→submissions→G2) as declared rows | EXT | M | g2: measurable funnel to the exit gate | wf | 16 |
| **19** | **Promote vetted Connector + Query into frozen crates (additive RCR)** + add TypeScript as a 3rd differential-fuzz arm (3-way byte agreement) | RCR+FC | L | conformance: reference runtime L1-live; 3-impl agreement | wf | 7,8,13 |
| **20** | **Capstone** — one reproducible end-to-end "organization day" on the real Kernel (ingest→reason→authenticated-approval→commit→replay identical truth_hash); probe-run + ConformanceArtifact | FC | L | evidence: undeniable content-addressed/replayable value @ G1 | solo | 13,11,14 |

### Wave structure
- **Wave 0 (parallel, zero-dependency):** ranks **1,2,3,4,5,6,7,8,9,10,11** — full-surface gate,
  RCR-004b bin, de-drift, open-debt register, TLC capture, L1 spine + Connector + Query, CCP-008
  draft, freeze-gate spec coverage, M1. Dispatch the `wf` items across agents simultaneously.
- **Gated:** 12 (needs 2) · 13 (needs 6,7,8) · 15 (needs 1) · **16 PUBLISH (needs 1,2,3,15)** ·
  17 (needs 1,5) · 18 (needs 16) · 19 (needs 7,8,13) · 20 (needs 13,11,14).

### Top-5 next actions (resume here)
1. **rank 2 — `acs_validate` bin (RCR-004b):** add `runtime/crates/arves-conformance/src/bin/acs_validate.rs`
   (stdin `tier<TAB>hex` → stdout `ACCEPT` | `REJECT<TAB><kebab-code>` | `ERR`), calling
   `semantic.rs` validators; `cargo test --workspace` → 81/0; `freeze_check.py update` as part of the RCR.
   *(Partial prep — a `pub uci_fact_schema()` refactor in `semantic.rs` — was reverted at pause; redo it here.)*
2. **rank 1 — full-surface SOUND gate:** in `verify_runtime_sound.py` replace the `tier=="core"` filter
   (line 111) with a per-tier loader; add grader-owned envelope/instance/language rejecters driving the
   runtime-under-test inputs-only (Python arm via the reference validators; Rust arm via the rank-2 bin);
   SOUND-CERTIFIED requires published+fresh+accept+ALL FOUR reject tiers; keep `test_harness_integrity.py`
   (real=CERTIFIED, hollow-echo=REJECTED, byte-broken=REJECTED). Mirror in `certify_runtime.py`.
3. **rank 3 — de-drift** IMPLEMENTING_ARVES.md §2/§3b, products/README.md IDR-006, Master Roadmap to
   `standard/VERSION` (0.3.1) + EVIDENCE_LEDGER counts.
4. **rank 4 — `verification/OPEN_DEBT_REGISTER.md`** cross-linking both ID schemes.
5. **rank 5 — capture TLC** on `verification/formal/ARVES_Kernel.tla`; commit the log; flip the Formal row.

---

## Execution model (how we go fast without breaking things)

- **Each frozen touch = its instrument + freeze re-baseline in the same commit**, gates green
  (`freeze_check` 0 drift · `evidence_probe --check` · `cargo test --workspace` · `certify` 2/2 ·
  `sound` 2/2) — the exact rhythm that landed CCP-006/007 + RCR-004/005/006/007 at 0 drift.
- **`wf` items → multi-agent workflows** (implement → adversarially verify); **`solo` items →
  focused single change + verify + commit.**
- **Publish (rank 16) is the one irreversible, outward step** — HARD-gated behind a full-surface
  gate + drift-free packet + ready front door, and it is maintainer-authorized (not auto).

*Recorded in the living repository (ED-001). Independence continues toward G2 — the real exit gate.*

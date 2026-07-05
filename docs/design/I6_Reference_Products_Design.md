# I6 — Reference Products: Engineering Design Package

```
=====================================================================
  STATUS: DESIGN PACKAGE (Ch4 PREP MODE) — NO CODE
  Build gate (G2) CLOSED. No implementation is authorized by this
  document. Prepared 2026-07-05 under the maintainer prep-mode ruling.
=====================================================================
```

**Milestone:** I6 — Reference Products (frozen Baseline, Part 5: *"Products on certified
runtime → ARVES v1.0 GA"* — `spec-markdown/ARVES_00_Baseline_v1.md`, Part 5).
**Certification target:** **Certified Product** level (Certification & Review Manual,
Vol 6, Part 8) — the top of the L1→L4→Certified-Product ladder.
**Constitution:** this package executes steps 1–9 of the Mandatory Engineering Workflow
(Architecture Readiness Review → Engineering Design). Step 10 (Critical Self-Review)
was executed as an external adversarial review of this package; its findings and their
disposition are recorded in §7. Steps 11–15 (Implementation → Certification Verdict)
are **gated behind G2** and are NOT begun here.

Traceability rule for this document: every load-bearing claim cites its frozen source
(document + part/section) or is explicitly listed under **Open Questions**. Where the
frozen corpus is silent, this document says so — it never fills the silence with invented
architecture (Non-Negotiable Rule 2).

---

## 1. BEFORE-WRITING-CODE — the ten constitutional answers

### 1.1 Which UCI node is affected?

**No UCI node is modified.** I6's deliverable lives entirely in the **living product
layer** (`products/`), which is a *customer* of the frozen Runtime Platform, never a
co-author of it (IDR-006, `products/README.md`; `runtime/RUNTIME_FREEZE_v1.0.md`, "The
two platforms"). The milestone *exercises* — read-only, via the published Runtime API —
these frozen nodes:

| Node exercised | How I6 touches it | Governing surface |
| --- | --- | --- |
| Kernel | commits product truth (sole commit gateway) | OWN-001, ORCH-001/004 |
| Persistence | WAL/decision trace behind every product commit | IDR-001/005, ORCH-003 |
| Capability Fabric | product capability resolution/gating | Runtime API (contract-only crates per RCR-001 item #6) |
| Engine Fabric | pure engine invocation on the work chain | Runtime API (contract-only, same) |
| Bridge | the line-protocol boundary products bind to | `runtime/RUNTIME_FREEZE_v1.0.md`, "The Runtime API" |
| Query | product read paths (I3 consistency tiers) | Vol 4 Playbook, Part 11, I3 |
| Control Plane | agent-product plan/graph orchestration | ORCH-001..004 (Vol 9 CCP v2, Part 5) |

The cluster forms of these nodes (I2–I5) are prerequisites designed in their own
packages; I6 adds **zero** node-internal design.

### 1.2 Which documents govern it?

- `spec-markdown/ARVES_00_Baseline_v1.md` — Part 5 (I6 definition and GA outcome);
  Part 3 (Marketplace / Cloud / Enterprise Governance *consciously deferred to v2* as
  normative spec scope); Part 6 (Baseline Rule: scope closed).
- `spec-markdown/ARVES_OS_Volume_6_Certification_Review_Manual_v1.md` — Part 4 (the 12
  frozen conformance axes), Part 5 (reference scenarios), Part 7 (Conformance Artifact),
  Part 8 (Certified Product level; Kernel-never-Control-Plane rule), Part 9 (Independent
  Architecture Review), Part 10 (conformance checklist).
- `spec-markdown/ARVES_OS_Volume_4_Implementation_Playbook_v1.md` — Part 11 (I6 scope,
  certification target "Certified Product", grounded in "Ecosystem goals") and §11.1
  "I6 — Reference Products" (the five I6 success criteria).
- `products/README.md` — IDR-006 (parallel Product Program; the frozen-dependency rule;
  the **four-condition GA gate**), the P-ladder, the five product rules, the
  IMPOSSIBLE-PRODUCTS filter.
- `runtime/RUNTIME_FREEZE_v1.0.md` — the v1.0 stability contract, the Runtime API
  surface, the RCR process, the v1.1 debt register.
- `ARVES_BUILD_PROGRAM_CLOSURE.md` — what is sealed, known limitations at seal
  (single-node I1; independence grade G1; formal at L0), and future governance.
- Reference Lifecycle, Part 6 (CCP-GATE) — any new conformance scenario or invariant
  enters only via CCP (also restated in Vol 6, Part 5: "the suite grows through
  CCP-gated additions").

### 1.3 Which contracts apply?

- **The Runtime API** as published in `runtime/RUNTIME_FREEZE_v1.0.md` ("The Runtime
  API"): SDK (content addressing + canonical encode; `Arves`, `FactStore`) and Bridge
  (`commit` and `invoke` over the line protocol). *"These surfaces are stable for v1.x.
  Additive, backward-compatible extension is allowed; a breaking change requires a new
  major (v2.0)."*
- **ACS-001/002** identity and canonical-bytes contracts (freeze record, "What v1.0
  guarantees": `0x12 0x20 || SHA-256(domain‖body)`, canonical dCBOR) — every product
  ContentId is stable forever; SDK-local == Kernel-committed.
- **ACS-003/004/005** semantic reject tiers (CCP-006/007; kit `arves-standard-kit
  0.3.1` per IDR-006 as amended in `products/README.md`).
- **Conformance contract:** `standard/` vectors define conformance — *"a runtime is
  conformant iff it reproduces the vectors and rejects the negatives"* (freeze record,
  "The Runtime API").
- **Node probes bind to contract clauses** (Vol 6, Part 3/6) — I6's product evidence is
  collected exclusively through this probe mechanism, not through bespoke assertions.

### 1.4 Which invariants apply?

Registered-normative only (Invariant Registry; Vol 6 Part 4 note): **OWN-001,
LAYER-001, SHARD-001, ORCH-001..004.** Full mapping with executable-proof obligations
in §4. Proposed invariants (G-001, QUERY-001, LCW-001, PERSIST-001, CAP-001..009,
ENG-001..005) are referenced in this package only with the marker **(PROPOSED —
CCP-GATE required)** and can at most yield PARTIAL, never FAIL (Vol 6, Part 6).

### 1.5 Which ownership rules apply?

- **Kernel is the sole commit gateway** for truth (OWN-001; freeze record "Truth").
  Products never commit truth by any path other than Bridge→Kernel.
- **Three teams, three mandates** (freeze record, "Organization"): Runtime Team owns
  `runtime/`+`standard/` (never break); Product Team owns `products/` (ship value);
  Verification Team owns `verification/` (break everything). I6 work is Product-Team
  and Verification-Team work exclusively.
- **Every product directory states its pinned platform version** and affirms it
  modifies no platform file (`products/README.md`, closing rule).

### 1.6 Which IDRs apply?

- **IDR-001..005** (binding, inherited through the cluster runtime I6 stands on): CP
  kernel; per-shard Raft; leader→followers→snapshots→WAL replication; joint-consensus
  membership; per-shard leader election; append-only WAL with deterministic replay.
  I6 products must *tolerate* the consequences (leader redirects, failover, consistency
  tiers) without ever violating them.
- **IDR-006** (`products/README.md`): products consume the frozen platform as a pinned
  external dependency; a needed platform change is a Platform Change Proposal / RCR,
  never a product edit; **the four-condition gate is retained for GA**.

### 1.7 Does this create architectural drift?

No — by construction, provided three disciplines hold:

1. All product↔runtime interaction flows through the published Runtime API
   (SDK/Bridge); no product reaches into Kernel, WAL, or fabrics directly (the
   `Products → SDK → Bridge → Capability → Engine → Kernel` chain of record,
   `products/README.md`, "Kernel Bridge (P2) was brought forward on purpose").
2. Cluster-endpoint exposure happens **inside the runtime via RCR** (v1.1 additive if
   the line protocol is preserved; v2.0 if breaking — freeze record, RCR process),
   never as a product-side fork of the Bridge.
3. No new milestone names, levels, axes, or invariants are minted (Vol 4 Part 11:
   "These names are used verbatim; no other milestone names exist"; Vol 6 Part 4: the
   12 axes are frozen).

Drift sentinels are listed in §3.21 (Risks).

### 1.8 Does this require CCP / Amendment / a new IDR?

Yes — I6 cannot be executed without the following instruments (none of which this
prep-mode document files; it only names them — see §6):

| Need | Instrument | Why |
| --- | --- | --- |
| Cluster client endpoint on the frozen Bridge/SDK surface | **RCR** (v1.1 additive or v2.0) | `runtime/` frozen; only an RCR changes it (freeze record) |
| Bridge request-id correlation (prereq for multi-endpoint failover) | **RCR** — already registered as v1.1 debt item #1 | freeze record, "v1.1 backlog" |
| New product-facing reference scenarios (if the 7 listed in Vol 6 Part 5 are insufficient for Certified Product) | **CCP** (CCP-GATE) | Vol 6 Part 5: suite grows only through CCP-gated additions |
| Endpoint-discovery / leader-redirect engineering decision | **IDR** (new, e.g. IDR-007) | Constitution: engineering decision → IDR |
| Ratifying any PROPOSED invariant used as a product gate | **CCP Amendment + conformance scenario** | CLAUDE.md, Registered Invariants |

### 1.9 Can another independent implementation reproduce this behaviour?

Yes — that is the milestone's exit condition, not an aspiration. Vol 4 §11.1 (I6)
requires *"Independent Runtime A and Independent Runtime B both pass certification"*,
and the Conformance Artifact (Vol 6, Part 7) is defined to be **self-sufficient**: a
third party reconstructs the verdict from the artifact alone (replay from decision
trace, ORCH-003), without re-running engines. The Foundation harness already certifies
2 runtimes (Rust + Python) under one conformance at grade G1 (`FOUNDATION.md`;
`ARVES_BUILD_PROGRAM_CLOSURE.md`, Evidence); I6 raises the required grade to G2
(genuine external team — closure record, Known Limitations).

### 1.10 Would this implementation still pass conformance five years from now?

The design binds only to frozen, versioned surfaces: the 12 axes (frozen with the
spec; new axes require a new UCS major — Vol 6 Part 4), the Certified Product level
definition (Vol 6 Part 8), ACS-001/002 identity (*"a value's ContentId is stable
forever"* — freeze record), and pinned Kit/runtime versions (IDR-006). Products pinned
to a platform version keep running until they choose to adopt a new one (freeze
record, RCR step 4). The five-year risk is therefore concentrated in the *unratified*
parts — G2 outcome, formal verification, cluster RCRs — all of which are held in Open
Questions (§3.22), not assumed.

---

## 2. Scope of the milestone (what I6 actually is)

The frozen definition is one line: **"Products on certified runtime → ARVES v1.0 GA"**
(Baseline, Part 5). Vol 4 §11.1 expands it into five success criteria; Vol 6 Part 8
gives it a certification level ("Certified Product: enterprise-readiness met; real
product built on ARVES without modifying the standard; all lower levels held"). This
package decomposes I6 into four work streams:

1. **Graduation** — select which P-ladder previews become *reference products* and
   carry them from preview to Certified Product (§2.1).
2. **GA gate** — apply the four-condition gate (IDR-006) to the I6 deliverable (§2.2).
3. **Migration** — move products from the single-node Bridge to cluster endpoints
   *without modifying the standard* (§2.3).
4. **Evidence** — define the conformance/certification evidence each product must
   carry (§2.4, §5).

### 2.1 Graduation — which products graduate from the P-ladder previews

Selection criteria (all frozen-sourced):

- **(a)** The product exists today as a ✅ preview pinned to Runtime v1.0
  (`products/README.md`, ladder + IDR-006 "products ship as previews pinned to a
  platform version").
- **(b)** It passes the IMPOSSIBLE-PRODUCTS filter ("Could this be built without
  ARVES?" → must be NO — `products/README.md`, Program 4).
- **(c)** It concretely demonstrates ≥1 core ARVES capability (product rule 3).
- **(d)** Its capability profile maps onto Certified-Product axes (Vol 6 Part 4/8).
- **(e)** It does not depend on scope the Baseline consciously deferred to v2 as
  *normative* spec surface (Baseline, Part 3).

Applying (a)–(e) to the ladder:

| P# | Product | Graduates as | Rationale (criterion) |
| --- | --- | --- | --- |
| P1 | Cognitive Memory | **Reference Product #1 (flagship)** | Proves Identity·Evidence·Replay·Truth·Audit·Dedup (ladder row); exercises axes 1, 2, 8, 12, 5 (per §5.1: Ingest-and-Derive, Stream-Under-Load, Crash-and-Replay) |
| P4 | Personal Cognitive OS | **Reference Product #2** | Persistent world model, reproducible/audited reasoning, contradiction-with-prior-decision (ladder row); first customer of Runtime v1.0; axes 4, 6, 5, 7, 12 (per §5.1: Plan-and-Act, Long-Run Saga, Crash-and-Replay) |
| P5 | Enterprise Cognitive OS | **Reference Product #3** | Multi-agent shared truth · governance · policy-as-truth · compliance ledger (ladder row); axes 3, 10, 9, 11 (+ 5, 7, 12 shared per §5.1); the natural carrier of the L4 (I5) prerequisite |
| P3 | Agent Runtime | **Graduates as a component** inside P4/P5, not standalone | Reasoning on the real Kernel (ladder row); its evidence surfaces through P4/P5 scenarios |
| P0 SDK, P2 Bridge | **Platform surfaces, not products** | They ARE the Runtime API products bind to (freeze record); certified as runtime, not as products |
| P6.5 Ecosystem SDK, P7 Marketplace | **Ecosystem deliverables** — required by Vol 4 §11.1 I6 ("SDKs / marketplace / cloud packaging available as ecosystem deliverables") but NOT normative v1.0 spec surface (Baseline Part 3 defers Marketplace to v2) | Ship alongside GA as living deliverables; carry their existing cert-gate evidence (closure record, Round-1 fix: enforced cert-gate) |
| P6 Studio, P8 Cloud, P9 Industry | **Do not graduate in I6** | Not ✅ previews; Cloud/Community are Growth-Program scope (closure record, "What We Deliberately Did NOT Build") |

The tension between Vol 4 §11.1 ("marketplace … available as ecosystem deliverables")
and Baseline Part 3 (Marketplace deferred to v2) is resolved **without amendment**: the
marketplace ships as a *living ecosystem deliverable* consuming the frozen platform; it
is not claimed as v1.0 *normative specification* scope. Any claim beyond that would
require a Baseline change, i.e., a next-major instrument (§6).

Minimum bar to close I6: **at least one** reference product certified as a Certified
Product (Vol 4 §11.1, I6 bullet 1). This package targets three (P1, P4, P5) so that a
single product failure does not stall GA; the *floor* remains the frozen "at least
one".

### 2.2 The four-condition GA gate applied to the I6 deliverable

IDR-006 (`products/README.md`): GA / any production release carrying platform-stability
guarantees requires **all four** conditions PASS. Applied concretely to I6:

| # | Condition | Concrete I6 meaning | Current honest status (closure record) |
| --- | --- | --- | --- |
| 1 | **Independent Runtime PASS** | Independent Runtime A **and** B pass certification (Vol 4 §11.1 I6 bullet 2) on the distributed (I2–I5) conformance levels, via the maintainer-independent harness (`FOUNDATION.md`) | G1 achieved (2/2 runtimes, same-process); distributed levels not yet built |
| 2 | **External Team (G2) PASS** | A genuine outside team builds a passing runtime from the Kit alone, no author contact | **NOT YET MET** — the open exit gate (closure record, Known Limitations) |
| 3 | **Certification PASS** | Each graduating product holds a Certified-Product verdict: all lower levels (L1–L4) held + product-grade review (Vol 6 Part 8) | L1 live conformance exists for reference nodes (RCR-008..010); L2–L4 pending I2–I5 |
| 4 | **Formal PASS** | The formal model (TLA+) is model-checked for the claims GA asserts | **L0 — not model-checked** (closure record, Known Limitations) |

**Gate semantics:** the gate is evaluated once, at the end of I6, over the *assembled*
deliverable (products + runtime + evidence). A single FAIL/NOT-MET blocks GA; products
remain **previews pinned to a platform version** (IDR-006) — that is the designed safe
state, not an error state. The gate is never weakened per-product; there is no
"partial GA".

### 2.3 Migration: single-node Bridge → cluster endpoints, standard unmodified

**Premise.** Today the chain of record is `Products → SDK → Bridge → Capability →
Engine → Kernel` against a single-node I1 runtime (`products/README.md`, Chain status;
closure record: "Runtime is single-node I1"). I2–I5 (designed in their own prep
packages) produce a cluster runtime per IDR-001..005. I6 must connect the graduating
products to that cluster **without one byte of change to `standard/`** and without
product-side edits to `runtime/`.

**Design principle: migrate the *endpoint*, never the *contract*.** The stability
contract products depend on is (i) ACS-001 identity, (ii) canonical ACS-002 bytes,
(iii) Bridge `commit`/`invoke` semantics with Kernel-side idempotent, content-addressed
commit (ORCH-004; freeze record "Truth"). None of these mention topology. Therefore:

1. **Runtime side (RCR-gated, not I6 product work).** The cluster Kernel exposes a
   client endpoint that preserves the existing line-protocol contract. Whether this is
   (a) the existing Bridge protocol served by every node with internal leader routing,
   or (b) an additive cluster-aware protocol extension, is an RCR decision for the
   Runtime Team (v1.1 if additive, v2.0 if breaking — freeze record). This package
   *requires only* that the chosen form keeps §1.3's contracts byte-stable.
   Prerequisite debt: **Bridge request-id correlation** (freeze-record v1.1 backlog
   item #1) — positional-FIFO correlation is not safe across reconnects/failover; the
   RCR that exposes cluster endpoints MUST close or supersede this item first. Also on
   the prerequisite list: **engine-enforced determinism** (freeze-record v1.1 backlog
   item #2) — required before any *fresh-run* equality check may be used in the
   migration acceptance test below; until it lands, determinism declarations are
   product claims, not runtime guarantees.
2. **Product side (I6 work, living).** The SDK gains additive endpoint configuration:
   a set of endpoints instead of one; retry-with-idempotent-re-propose on failover
   (safe because commit is idempotent + content-addressed — ORCH-004, and re-binding a
   ContentHash to different bytes is rejected by RCR-005 content-integrity); a
   consistency-tier selector on read paths (linearizable / bounded-staleness /
   eventual — Vol 4 §11.1, I3). Whether this is free product-side evolution or an
   RCR/PCP-triaged Runtime API extension is **not settled by the frozen record**: the
   freeze record permits additive, backward-compatible extension of the Runtime API,
   but its two-platforms table lists "SDK core" in the FROZEN Runtime Platform column
   while the SDK source lives under `products/`. Client-side failover/retry semantics
   arguably touch Runtime API stability guarantees. This design does NOT assert a
   reading; the ruling is requested from the Runtime Team via IDR-006 PCP triage and
   recorded as **OQ-9** (§3.22). No such SDK feature is designed in detail here until
   that ruling lands.
3. **Leader semantics.** Per-shard leader election (IDR-004) means a product's commit
   may land on a non-leader. The design *prefers* redirect handling below the SDK
   surface (products see only latency), but whether redirects are bridge-internal or
   SDK-visible is an engineering decision → **new IDR** (§1.8). Open Question OQ-3.
4. **Migration is a re-pin, not a rewrite.** Per IDR-006/RCR step 4, each product
   migrates by bumping its pinned runtime tag + Kit version and re-running its full
   evidence set (§2.4). A product that cannot pass on the cluster pin stays on its
   previous pin — pinning bounds the blast radius (IDR-006, residual risk).
5. **What migration must NOT do:** no product-visible change to ContentIds (identity
   is topology-independent by construction); no product-side awareness of shards
   beyond the immutable partition key it already supplies (SHARD-001); no new commit
   path bypassing the Kernel gateway (OWN-001, ORCH-001).

**Migration acceptance test (per product) — replay equivalence, not recomputation
equivalence.** The frozen spec explicitly disclaims recomputation equivalence for
non-deterministic engines: *"reproducibility means deterministic replay from recorded
outcomes, not identical re-computation"* (Vol 9 CCP v2, Part 5, ORCH-003 rationale) and
*"Recomputation is explicitly NOT guaranteed for non-deterministic engines"* (Vol 9
CCP v2, Part 8). A fresh-run ContentId-set comparison across pins would therefore
produce false FAILs for any P1/P4/P5 workload containing a non-deterministic engine.
The test is defined accordingly:

- **Primary (all products):** record the product workload's decision trace on the
  single-node pin; **replay** it against the cluster pin. The verifier must
  reconstruct the identical verdict and identical committed ContentIds from the trace
  alone, without re-running engines (ORCH-003; Vol 6 Part 7 artifact
  self-sufficiency). Replay divergence is a FAIL and, if runtime-caused, an RCR —
  never a product workaround.
- **Supplementary (deterministic workloads only):** fresh-run ContentId-set equality
  across the two pins is required *only* for workloads the product declares
  engine-deterministic. Note that determinism is currently declared, not
  runtime-enforced — engine-enforced determinism is v1.1 debt item #2 (freeze record,
  "v1.1 backlog"), listed as a prerequisite in §2.3.1 above.

### 2.4 Evidence each product must carry

Every graduating product carries, in its product directory, a versioned **evidence
pack** (living files; format follows the existing Evidence Ledger discipline,
`verification/evidence/`):

1. **Conformance Artifact** per Vol 6 Part 7, with every field populated: suite
   version, spec version (frozen 2026-07-01), runtime identity + pinned tag, axis
   coverage, per-scenario verdicts, the recorded decision trace (WAL/Raft log,
   IDR-001/005), the registered-invariant matrix, proposed-invariant notes (flagged
   pending CCP), level attestation.
2. **Certified-Product attestation** — all lower levels held (L1–L4) + the
   Kernel-never-Control-Plane check (Vol 6 Part 8).
3. **Independent Architecture Review record** — the 10-dimension adversarial review
   (Vol 6 Part 9), all dimensions PASS, with the blind-pass and adversarial-probing
   steps documented.
4. **Standard-untouched proof** — mechanical: the 266-file freeze gate green over the
   product's whole change history + the product's IDR-006 header (pinned version, "no
   platform file modified"). Vol 4 §11.1 I6 bullet 3: *"No product modifies the
   standard; all changes to meaning went through CCPs."*
5. **Migration acceptance evidence** — the §2.3 replay-equivalence result (and, for
   declared-deterministic workloads only, the supplementary fresh-run result) for the
   single-node→cluster re-pin.
6. **IMPOSSIBLE-filter statement** — why this product could not exist without ARVES
   (product rule 3 + Program 4 filter), with the demonstrating scenario named.

---

## 3. ENGINEERING DESIGN (constitution-mandated sections)

### 3.1 Responsibilities

- **Product Team:** graduate P1/P4/P5 from preview to Certified Product; implement the
  additive SDK endpoint/tier features once their governance route (product-side vs.
  RCR — OQ-9) is ruled; produce evidence packs.
- **Runtime Team:** ratify and land the cluster-endpoint RCR(s) (§2.3.1) — outside I6's
  authority but on its critical path.
- **Verification Team:** run the destroy pass, the conformance suite across the 12
  axes, and the Independent Architecture Review against each product.
- **This milestone explicitly does NOT own:** any UCI node internals, the standard,
  the conformance suite definition (CCP-gated), or the G2 event itself (external by
  definition).

### 3.2 Inputs

- Frozen: `standard/` (Kit 0.3.1 vectors, golden + negative), the runtime at its
  cluster tag (post I2–I5 RCRs), the 12 axes + 7 reference scenarios (Vol 6 Parts 4–5),
  the Certified Product definition (Vol 6 Part 8).
- Living: the ✅ preview products P0–P7; the Foundation certification harness
  (`FOUNDATION.md`); the Evidence Ledger.
- External (not controllable): the G2 team's runtime and their certification run.

### 3.3 Outputs

- Three certified reference products (P1, P4, P5) each with a §2.4 evidence pack.
- Ecosystem deliverables at GA: SDKs, marketplace, packaging (Vol 4 §11.1 I6 bullet 5)
  as living artifacts.
- The assembled GA-gate evaluation record (four conditions, §2.2).
- On all-PASS: the **ARVES v1.0 GA declaration** (Baseline Part 5 outcome). On any
  non-PASS: a documented gate-hold with products remaining pinned previews.

### 3.4 Dependencies

Hard, ordered: **I2 (Cluster Kernel) → I3 (Distributed Query) → I4 (Capability
Scheduling) → I5 (Multi-Agent Runtime) → I6**, because Certified Product requires *all
lower levels held* (Vol 6 Part 8: L3 maps to I2/I3/I4, L4 to I5). Plus: the
cluster-endpoint RCR(s); the request-id-correlation debt item; the G2 external event;
formal model-checking (condition 4). I6 is the **last** milestone by construction and
cannot be re-ordered (Vol 4 Part 11 table).

### 3.5 Lifecycle

Per product: `preview (pinned v1.0) → cluster re-pin (migration, §2.3) → conformance
runs (12 axes at Certified-Product level) → Independent Architecture Review →
Certified Product attestation → GA-gate assembly`. A product failing any stage loops
back with a defect record; a runtime-caused failure becomes an RCR and the product
*waits on its old pin* — it never patches around the runtime (IDR-006 hard guardrail).

### 3.6 State Model

Products own **no truth-plane state**. All persistent product state is Kernel-committed
truth (OWN-001) reached via Bridge `commit`; all product working state is
LCW/session-scoped per the runtime's contracts; plans live in the Control Plane and are
not persistent state (ORCH-002). Product-local caches are read projections and must be
rebuildable from committed truth (Query replay pattern per RCR-010's WAL-replay
precedent). Any product state that cannot be classified this way is a design FAIL.

### 3.7 Distributed Behaviour

Inherited entirely from IDR-001..005 — I6 adds no distributed mechanism of its own.
Product-visible consequences the design must absorb: leader redirect (IDR-004),
failover retry with idempotent re-propose (ORCH-004), consistency-tier selection on
reads (I3), immutable partition key supplied at write (SHARD-001), saga-style
compensation for cross-shard product flows — *no cross-shard atomic commit in v1*
(Vol 6 Part 9, Distribution dimension FAIL trigger; freeze-record v1.1 backlog item #3
"Kernel batch-commit" is explicitly deferred and MUST NOT be assumed by any product).

### 3.8 Concurrency

Multi-agent product flows (P5) run on the I5 substrate: concurrent agents never
produce conflicting committed truth (Vol 6 Part 5, Swarm-Coordinate scenario); the
Kernel's idempotent, content-addressed commit is the single serialization point for
truth (ORCH-004). Product-side concurrency (SDK connection pools, parallel invokes) is
bounded by the request-correlation guarantee of the post-RCR bridge; until that RCR
lands, products keep the current one-in-flight discipline.

### 3.9 Failure Modes

| Failure | Product-visible effect | Required behaviour |
| --- | --- | --- |
| Leader crash mid-commit | timeout / redirect | idempotent re-propose; same ContentId; no duplicate truth (ORCH-004) |
| Re-proposal binds same hash to different bytes | reject | surfaced as product bug; Kernel rejects (`ContentIntegrity`, RCR-005) |
| Network partition | commit unavailability (CP kernel, IDR-001) | fail closed; no partial truth (Amendment-006, axis 7) |
| Split-brain | none permitted | stale leaders step down; no truth fork (Vol 4 §11.1 I2) |
| Bridge desync / malformed frame | error | bridge fails safe (freeze record, Robustness) |
| Cross-shard product flow partially applied | compensations | saga-style compensation; rollback-by-not-committing (Amendment-006) |
| G2 / certification forces a breaking platform change | migration pressure | pinned version bounds blast radius (IDR-006 residual risk); RCR + re-pin, never a fork |

### 3.10 Recovery

Runtime recovery (WAL + snapshot → consistent truth, Vol 4 §11.1 I2) is inherited.
Product-level recovery obligation: after any crash/restart, a product reconstructs its
projections from committed truth and its session state from the decision trace —
recovery is **replay, not recomputation** (ORCH-003). Each reference product's
Crash-and-Replay scenario run (§5) is its executable recovery proof.

### 3.11 Replay

The Conformance Artifact is replay-defined: *"given the artifact, a verifier
reconstructs the verdict without re-running engines"* (Vol 6 Part 7). Every product
evidence pack ships its decision trace; the certification harness must reproduce every
verdict from trace alone. Migration acceptance (§2.3) additionally proves replay
equivalence across pins.

### 3.12 Consistency

Truth is CP (IDR-001); observability is AP (CLAUDE.md, IDR table). Product read paths
declare their tier per read (linearizable / bounded-staleness / eventual — Vol 4
§11.1 I3); presenting a stale read as linearizable is a review FAIL (Vol 6 Part 9,
Consistency dimension). The compliance-ledger views in P5 default to linearizable;
memory-recall paths in P1/P4 may use bounded-staleness — the per-product tier map is a
design deliverable of each product's graduation (recorded in its evidence pack).

### 3.13 Availability

Commit availability is bounded by quorum (CP choice, IDR-001) — products must degrade
gracefully to read-only on quorum loss rather than buffering "truth" locally
(fail-closed, axis 7). Read availability may exceed commit availability via the
eventual/bounded tiers. No product may promise higher write availability than the
kernel's CP posture provides — such a promise would be an overclaim (closure-record
discipline: zero residual overclaim).

### 3.14 Scalability

Scaling is shard-scaling (SHARD-001; per-shard Raft, IDR-002/004). Products supply
stable partition keys (tenant, world, user) and never remap them (immutable partition
key — SHARD-001; Stream-Under-Load scenario). Product-tier scaling (many SDK clients)
multiplies bridge connections, not kernel commit paths. Quantified targets are an
Open Question (OQ-6) — the frozen corpus defines the *properties* benchmarks check
(Vol 6, Part on performance: metrics are properties over recorded traces, never
truth), not GA-specific numbers.

### 3.15 Performance

Per Vol 6 (performance part): performance is measured as **properties over the same
recorded traces, never as truth**; benchmarks stress the high-volume-streaming,
long-running, and distributed axes. I6 adopts that frame verbatim: each reference
product's evidence pack includes trace-derived performance properties for its declared
axes. No performance work precedes correctness (Non-Negotiable Rule 7).

### 3.16 Security

Honest inherited posture: v1.0's threat model is a **trusted single host** — no
authenticated commit, no signatures; RCR-002 added a SHA-256 hash-chain digest
(tamper-evidence), while signatures + authN + anchoring remain v2.0 debt (freeze
record, item #8). **Consequence for I6:** an enterprise GA claim (P5) that implies a
hostile-host or zero-trust deployment model would overclaim. Either (a) GA documents
scope truth-store trust to the operator boundary explicitly, or (b) the v2.0
signed-truth-store RCR lands first. This is a GA-gate wording obligation, recorded as
OQ-5. The marketplace's enforced cert-gate (re-run at publish+install over
tamper-evident, signature-bound inputs — closure record, Round-1 fixes) is carried
forward unchanged.

### 3.17 Observability

Observability is AP (IDR table) and never a truth source. Products emit operational
telemetry outside the truth plane; node probes (Vol 6 Part 6) are the only
evidence-grade observation mechanism, and they are passive ("they observe, they do not
steer"). No product metric may be derived by writing to, or steering, the truth path.

### 3.18 Metrics

- **Milestone metrics:** # products holding Certified Product (target ≥1, floor per
  Vol 4 §11.1; plan 3) · 12/12 axes green at target level · 4/4 GA-gate conditions.
- **Program metrics (context, living):** the Growth north-star — independent teams
  building without author contact (closure record, "The metric that matters now") —
  and the Product KPI "Impossible before → possible with" (`products/README.md`).
  These inform priority but are not gate conditions.

### 3.19 Auditability

Every gate decision is evidence-backed and replayable: the Evidence Ledger row +
Conformance Artifact per product; the GA-gate assembly record cites each condition's
artifact; the freeze gate proves standard-untouched mechanically. This mirrors the
closure-audit discipline (`ARVES_BUILD_PROGRAM_CLOSURE.md`: "closed on that evidence,
not on opinion") — GA is *declared on evidence, not asserted*.

### 3.20 Trade-offs

- **Three graduates vs. one (floor):** more evidence + redundancy vs. more
  certification runs. Chosen: three, floor stays one — GA never waits on a
  nice-to-have graduate.
- **Bridge-internal vs. SDK-visible leader redirect:** product simplicity vs. runtime
  complexity inside a frozen surface. Deferred to IDR (OQ-3) — deliberately not
  decided here because it changes the RCR's scope.
- **GA strictness vs. time-to-market:** the four-condition gate keeps products in
  preview indefinitely if G2/formal stall. Accepted by IDR-006's own design — the gate
  "protects exactly what the original gate protected".
- **Marketplace at GA:** shipped as living ecosystem deliverable (Vol 4) but not
  normative v1.0 scope (Baseline Part 3) — the honest narrow reading is chosen over
  the expansive one.

### 3.21 Risks

| Risk | Class | Mitigation |
| --- | --- | --- |
| G2 never materializes (external, uncontrollable) | schedule | products remain viable pinned previews (IDR-006); Growth Program drives outreach — not an I6 lever |
| Formal stays at L0 | gate | condition 4 blocks GA; scope of "Formal PASS" must be pinned by IDR before work starts (OQ-4) |
| Cluster RCRs breach the line-protocol contract | drift | migration acceptance test (§2.3) FAILs → v2.0 route, products stay pinned |
| Positional-FIFO bridge correlation under failover | correctness | RCR prerequisite (v1.1 debt #1) is a hard dependency — no cluster migration before it |
| Product convenience pressures the standard | constitutional | IDR-006 hard guardrail + 266-file freeze gate + PCP path |
| Certified-Product review self-certified (same-author) | independence | review runs blind-pass + adversarial per Vol 6 Part 9; G2-grade external review preferred where available (graded independence, Era-3 doctrine) |
| Overclaim in GA language (security, distribution) | credibility | closure-record precedent: adversarial claim audit before the GA declaration |

### 3.22 Open Questions (honest — no silent assumptions)

- **OQ-1:** Which exact runtime tag constitutes "certified runtime" for GA — the
  post-I5 tag, or a dedicated GA tag after the four-condition run? (Baseline Part 5 is
  silent; needs maintainer ruling.)
- **OQ-2:** Do the 7 reference scenarios (Vol 6 Part 5) suffice for Certified-Product
  axis coverage of P1/P4/P5, or are product-shaped scenarios needed (→ CCP)? Initial
  mapping (§5) suggests they suffice, but this is unproven until dry-run.
- **OQ-3:** Leader-redirect visibility — bridge-internal or SDK-visible? (→ new IDR;
  interacts with the cluster-endpoint RCR's scope.)
- **OQ-4:** What is the *ratified scope* of "Formal PASS" (condition 4) — which claims,
  which model, which checker? The corpus requires it but does not size it. (→ IDR.)
- **OQ-5:** GA security wording — is trusted-operator scoping acceptable for the P5
  enterprise GA claim, or is the v2.0 signed truth store a GA prerequisite?
  (Maintainer ruling; see §3.16.)
- **OQ-6:** Quantified performance/scale targets for GA (the corpus defines property
  shape, not numbers). (→ IDR, informed by I2–I5 benchmark traces.)
- **OQ-7:** Does "Independent Runtime A and B" (Vol 4 §11.1) require both to pass at
  the *distributed* levels (L3/L4), or does L1/L2 conformance of two runtimes plus one
  distributed reference satisfy the letter? Strict reading assumed for planning
  (both at target level); needs ruling before gate assembly.
- **OQ-8:** Whether P3 Agent Runtime should additionally graduate standalone if its
  embedded evidence inside P4/P5 proves insufficient for the multi-agent axis.
- **OQ-9:** Governance of additive SDK endpoint/retry configuration (§2.3.2:
  multi-endpoint set, retry-with-re-propose, consistency-tier selector) — does it fall
  under Runtime API stability governance (→ RCR), or is it product-side evolution?
  The freeze record's two-platforms table places "SDK core" in the FROZEN Runtime
  Platform column while the SDK lives under `products/`; the frozen record does not
  settle client-side failover/retry semantics. (→ Runtime Team ruling via IDR-006 PCP
  triage; blocks detailed design of those SDK features.)

---

## 4. Invariant Mapping (registered invariants only, + proof obligations)

Registered set per CLAUDE.md / Invariant Registry: OWN-001, LAYER-001, SHARD-001,
ORCH-001..004. No invariant may remain proof-only once its owning component is
implemented (CLAUDE.md, Registered Invariants). Existing executable-proof precedents:
PropertyCheck catalog (RCR-006), two-tenant isolation (RCR-007), live L1 conformance
(RCR-008..010).

| Invariant | What I6 must uphold (product-plane reading) | Executable proof I6 needs (design-time statement; built only post-G2) |
| --- | --- | --- |
| **OWN-001** | Every product state resolves to exactly one owner; all persistent product truth owned by the Kernel | Per-product trace audit: every committed state in the Conformance Artifact resolves to one owner (Vol 6 Part 10 checklist item); extends the RCR-008 live derivation to product workloads |
| **LAYER-001** | Products sit above the SDK; dependencies downward-only; no product reaches laterally into fabrics/kernel internals | Architecture-gate extension over `products/`: import/dependency scan proving the only runtime touchpoint is the published SDK/Bridge surface (pattern: existing executable architecture gate) |
| **SHARD-001** | Partition key immutable across every product write; no cross-tenant leakage in product views | Two-tenant product-workload isolation test (pattern: RCR-007 `behaviour_8_two_tenant_isolation`) + Stream-Under-Load scenario at product level |
| **ORCH-001** | No product or product-side agent commits truth outside the Kernel; Control Plane holds no truth | Trace predicate: zero commit records originating outside the Bridge→Kernel gateway (Human-Gated-Approval + Ingest-and-Derive scenarios) |
| **ORCH-002** | Product/agent plans are never persistent state | Trace predicate over P4/P5 agent flows: no plan object appears in committed truth (Plan-and-Act scenario) |
| **ORCH-003** | Every product verdict/recovery is replay-from-trace, not recomputation | Harness replays each product's Conformance Artifact to an identical verdict; Crash-and-Replay scenario per product; migration replay equivalence (§2.3) |
| **ORCH-004** | Every product-triggered invocation idempotent + content-addressable; failover re-propose yields no duplicate truth | Property test: repeated commit/invoke under injected failover → same ContentId set, no duplicated effect (extends kernel idempotency tests to the product path; RCR-005 integrity reject as the negative case) |

**PROPOSED invariants** touched by product semantics — referenced informatively only,
each marked **(PROPOSED — CCP-GATE required)**, and capable of PARTIAL at most (Vol 6
Part 6): CAP-* (capability gating in P6.5/P7 flows), ENG-* (engine purity beneath
product invokes), QUERY-001 (read-only product projections), PERSIST-001 (WAL
discipline beneath product truth). If any is wanted as a *product gate*, it must first
be ratified via CCP Amendment **with a conformance scenario** (CLAUDE.md, Registered
Invariants; Reference Lifecycle Part 6 CCP-GATE).

---

## 5. Conformance Plan

### 5.1 Level and axes

Target level: **Certified Product** (Vol 6 Part 8) — cumulative, so L1–L4 must already
be held by the runtime (delivered by I1–I5). The Vol 6 Part 10 checklist item "All 12
axes have at least one reference scenario exercised for the target level" applies in
full at I6 — this is where **"complete conformance suite runs green across the twelve
axes"** (Vol 4 §11.1, I6 bullet 4) lands.

Axis instantiation via the frozen reference scenarios (Vol 6 Part 5), carried by the
graduating products:

| Scenario (Vol 6 Part 5) | Axes covered | Carried by |
| --- | --- | --- |
| Ingest-and-Derive | 1 Information-intensive, 2 Event-driven | P1 Cognitive Memory |
| Plan-and-Act | 4 Multi-step planning, 6 Physical-world | P4 (agent flows via P3) |
| Human-Gated Approval | 3 Human-collaboration, 10 Policy-heavy | P5 Enterprise OS |
| Long-Run Saga | 5 Long-running, 7 Safety-critical | P4 / P5 |
| Stream-Under-Load | 8 High-volume streaming, 2 Event-driven | P1 under load |
| Swarm-Coordinate | 9 Multi-agent, 11 Autonomous | P5 (multi-agent, on I5) |
| Crash-and-Replay | 12 Recovery/replay, 5 Long-running | every graduate (mandatory) |

Coverage check: axes 1–12 all appear ≥1× across the three graduates. If dry-runs show
a product cannot honestly carry a scenario, the gap is closed by a CCP-gated scenario
addition (OQ-2) — never by stretching a claim.

### 5.2 Per-milestone Success Criteria — what they concretely mean for I6

| Constitution criterion | Concrete I6 meaning |
| --- | --- |
| **Architecture PASS** | Independent Architecture Review (Vol 6 Part 9), all 10 dimensions PASS, per graduating product — including the Distribution dimension (IDR-001..005; no cross-shard atomic commit claimed) and the Kernel-never-Control-Plane rule |
| **Conformance PASS** | 12/12 axes exercised at Certified-Product level with PASS (or explicitly documented PARTIAL on non-blocking properties only — Vol 6 Part 6); Vol 6 Part 10 checklist fully demonstrable from each artifact |
| **Certification PASS** | Certified Product attestation per graduate: all lower levels held + product-grade review (Vol 6 Part 8); issued by the maintainer-independent harness (`FOUNDATION.md`), not by authorship |
| **Independent Review PASS** | The Vol 6 Part 9 process with blind pass + adversarial probing; at G2 grade where an external reviewer exists (graded-independence doctrine) — same-author review is the documented floor, not the goal |
| **Invariant Coverage 100%** | All 7 registered invariants have the §4 executable proofs green over product workloads; no registered invariant proof-only (CLAUDE.md obligation) |
| **Replay PASS** | Every product Conformance Artifact independently replays to the same verdict (Vol 6 Part 7 self-sufficiency); migration replay equivalence (§2.3) green |
| **Distributed Tests PASS** | Product workloads under failure injection (leader crash, partition, failover re-propose) commit no partial truth and fork no truth (inherits I2 criteria, exercised through the product path) |
| **No Architecture / Specification Drift** | 266-file freeze gate green; every product change traceable to the Runtime API; zero edits under `runtime/`, `standard/`, `spec-markdown/`, `corpus/`; all meaning-changes via CCP (Vol 4 §11.1 I6 bullet 3) |

I6 is additionally complete only when the **four-condition GA gate** (§2.2) is
evaluated and recorded — GA on all-PASS, documented hold otherwise.

---

## 6. NON-GOALS and change instruments

### 6.1 Explicit NON-GOALS (I6 will not do these)

- **No modification of any frozen surface** — `runtime/`, `standard/`,
  `spec-markdown/`, `corpus/` (freeze record; IDR-006; 266-file freeze gate).
- **No new UCI nodes, layers, milestones, axes, levels, or invariants**
  (Non-Negotiable Rules 2–3; Vol 4 Part 11 "no other milestone names exist"; Vol 6
  Part 4 axes frozen).
- **No normative-scope expansion of v1.0:** Marketplace, Cloud Runtime, Federated
  Kernel, Cross-Runtime Federation, Enterprise Governance remain consciously deferred
  to v2 (Baseline, Part 3) — ecosystem *deliverables* ship as living artifacts only.
- **No L3/L4 aspirational capabilities** (recursive self-improvement, embodied scale —
  Baseline, Part 4: explicitly not v1.0 commitments).
- **No cloud hosting, billing, commercial machinery, community programs** — Growth
  Program scope (closure record, "What We Deliberately Did NOT Build").
- **No cross-shard atomic commit and no reliance on Kernel batch-commit** (v1.1
  deferred debt; Vol 6 Part 9 FAIL trigger).
- **No zero-trust / hostile-host security claims** under the v1.0 threat model
  (freeze record item #8; §3.16).
- **No weakening or per-product splitting of the four-condition GA gate** (IDR-006:
  the gate is retained for GA, whole).
- **No new products** beyond the graduating set — I6 graduates existing previews; new
  ladder rungs belong to the Product Program's own cadence.
- **No implementation now** — this package is Ch4 PREP MODE output; the G2 build gate
  is closed.

### 6.2 Instruments any frozen-surface change would require

| Change class | Instrument | Source of authority |
| --- | --- | --- |
| Any `runtime/` change (cluster endpoints, bridge correlation, batch-commit, signed truth store) | **Runtime Change Request** (v1.1 additive / v2.0 breaking), with its own destroy→repair→prove cycle | `runtime/RUNTIME_FREEZE_v1.0.md`, RCR process |
| A product-discovered platform gap | **STOP + Platform Change Proposal**, triaged by the Runtime Team into the RCR path | IDR-006, `products/README.md` |
| New conformance scenario / suite growth | **CCP** (CCP-GATE) | Vol 6 Part 5; Reference Lifecycle Part 6 |
| Ratifying a PROPOSED invariant | **CCP Amendment + conformance scenario** | CLAUDE.md, Registered Invariants |
| Minor spec wording | **CCP Amendment** | CLAUDE.md, Change Management |
| Architectural ambiguity discovered during I6 | **Architecture Review** | CLAUDE.md, Change Management |
| Engineering decision (redirect visibility, formal scope, GA numbers — OQ-3/4/6) | **IDR** | CLAUDE.md, Change Management |
| Normative scope change (e.g., Marketplace into the standard; new axis; new level) | **Next Major Version** — v1.0 Baseline scope is closed and the Specification Era does not reopen | Baseline, Parts 3/6; Vol 6 Part 4 |

Never a silent edit, on any surface, for any reason (CLAUDE.md, Change Management;
Maintainer Note "frozen means frozen").

---

## 7. Critical Self-Review record (constitution step 10)

Step 10 was executed as an adversarial review of this package (independent-reviewer
posture, verdict on first pass: **PARTIAL**). Findings and disposition, all applied in
this revision:

| # | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| 1 | major | §2.3 acceptance test demanded fresh-run ContentId-set equality across pins, contradicting ORCH-003 (*"deterministic replay from recorded outcomes, not identical re-computation"* — Vol 9 CCP v2 Part 5) and Vol 9 Part 8 (*"Recomputation is explicitly NOT guaranteed for non-deterministic engines"*); v1.1 debt #2 (engine-enforced determinism) was omitted from prerequisites | **Fixed:** test redefined as replay equivalence (primary) with fresh-run equality reserved for declared-deterministic workloads; debt #2 added to §2.3.1 prerequisites |
| 2 | minor | §2.1 graduation table axes for P4 (and, on inspection, P1/P5) did not match the §5.1 scenario-carrier table | **Fixed:** §2.1 rows aligned to §5.1 (P1: 1,2,8,12,5 · P4: 4,6,5,7,12 · P5: 3,10,9,11 + shared) |
| 3 | minor | §1.7 pointed drift sentinels at §3.20 (Trade-offs) instead of §3.21 (Risks) | **Fixed:** pointer corrected |
| 4 | minor | Header claimed steps 1–10 executed while no step-10 record existed | **Fixed:** header now claims 1–9 + externally executed step 10; this section is the record |
| 5 | minor | §2.3.2 asserted additive SDK endpoint/retry configuration is product-side work "allowed by the freeze record", though the freeze record's two-platforms table lists "SDK core" as FROZEN Runtime Platform — an unsettled governance gray zone stated as fact | **Fixed:** claim softened to a pending Runtime Team ruling; recorded as OQ-9 via IDR-006 PCP triage |

No finding required an instrument filing beyond those already named in §1.8/§6.2;
finding 5 adds one PCP-triage question (OQ-9). Re-review after disposition: no
remaining known contradiction with the frozen corpus.

---

*End of I6 design package. Prepared 2026-07-05 in Ch4 PREP MODE — design only, no code,
G2 build gate closed. Successor activities: maintainer rulings on OQ-1..OQ-9, the
named RCR/IDR/CCP filings (§1.8, §6.2), and — only after G2 opens — constitution steps
11–15 for this milestone.*

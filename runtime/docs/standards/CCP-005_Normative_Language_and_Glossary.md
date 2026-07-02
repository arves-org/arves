# CCP-005 — Normative Language Convention & Terms-and-Definitions Glossary

**Type:** ARVES Core Standard (ACS) delivered as a Cognitive Change Proposal
Amendment (CCP-005). **Status:** DRAFT (Candidate on CCP-GATE pass; not yet
Ratified). **Program:** ARVES v1.1 Standardization, Goal 4 (Normative-Language
Convention) + Goal 5 (Vendor-Neutral Interoperability). **Closes:** Global
Readiness Report R-06 (Normative-Language Convention & Glossary — the ISO/IEC
Directives Part 2 / IEEE-SA prerequisite for a citable, independently-verifiable
standard). **Governs / activates:** the 7 REGISTERED invariants — `ORCH-001`,
`ORCH-002`, `ORCH-003`, `ORCH-004` (Vol 9 Cognitive Control Plane v2, Part 5) and
`OWN-001`, `LAYER-001`, `SHARD-001` (Amendments CCP Batch 1) — by restating each
as a single keyworded, addressable requirement with an explicit modality. This
amendment ADDS a language convention and a glossary; it does not edit any frozen
document (ED-001; sanctioned via the Reference Lifecycle CCP process, Part 6
CCP-GATE).

> Normative keywords (MUST / MUST NOT / SHALL / SHALL NOT / SHOULD / SHOULD NOT /
> MAY / REQUIRED / RECOMMENDED / OPTIONAL) are used per RFC 2119 as updated by
> RFC 8174. This document is the *definition* of that convention for ARVES; it
> therefore uses the convention it defines.

---

## 1. Problem

Twelve independent readiness reviews found that the frozen corpus mixes normative
force with descriptive prose. The same obligation is written "owns no truth",
"is stateless", "MUST hold", or "always" in different places, and load-bearing
nouns — *Control Plane*, *truth*, *shard*, *commit*, *decision trace* — are used
without a single authoritative definition. Three concrete failures follow:

1. **Modality is implicit.** `ORCH-001` reads "The Control Plane owns no truth."
   Is that a MUST, a SHALL, or a design note? Two independent implementers can
   read the same sentence and disagree on whether a violation is a conformance
   FAIL. A standard that cannot be cited word-for-word cannot be certified
   against.

2. **No clause-level citation.** A probe, an amendment, or a review verdict must
   be able to point at *one* requirement. Today it can only cite a whole
   invariant ("ORCH-001") or a whole document part; there is no `ORCH-001-R1`
   granularity, so partial conformance and targeted amendments are unexpressible.

3. **"Control Plane / CP" is overloaded.** In IDR-001 "CP" means the *Consistency*
   pole of the CAP theorem ("Kernel = CP"). In Vol 9 "Control Plane" means the
   *reasoning-to-action orchestration layer*. A reader who conflates them
   concludes the orchestration layer is strongly consistent, which is the exact
   opposite of `ORCH-002` (the Control Plane holds no persistent state). This is
   not hypothetical: it is the single most dangerous ambiguity in the corpus.

None of these is a specification *defect* — the meaning is fixed and consistent
across the frozen corpus (the Invariant Registry audit found 0 contradictions).
They are *expression* defects. The remedy is a clarification instrument, not a
specification change.

## 2. Scope

This standard defines four things and nothing more:

1. the **normative-keyword set** (§4) and how ARVES documents use it;
2. the **requirement-ID convention** (§5) that gives every normative clause a
   stable, citable identifier;
3. the **restatement** of the 7 registered invariants as keyworded, ID-bearing
   requirements (§6) — modality made explicit, meaning unchanged;
4. a **Terms-and-Definitions glossary** (§7) of the load-bearing vocabulary, with
   necessary/sufficient-condition definitions and stable term IDs, including the
   `Control Plane` / `CP` disambiguation.

This standard does **not** create, retire, or modify any invariant; it does not
change any ownership, layering, or partitioning rule; and it does not define
content-address bytes for rich values (that is ACS-001/ACS-002). Where it assigns
a content address (§8) it reuses the ACS-001 multihash exactly.

**Interpretation rule (normative).** Where a restated requirement in §6 and its
frozen source could be read to differ, the frozen source SHALL prevail and the
restatement SHALL be corrected by CCP. The restatements are asserted to be
meaning-preserving; §6 records the source clause for each so the assertion is
falsifiable.

## 3. Relationship to the frozen corpus

| Frozen source (read-only) | What CCP-005 does |
|---|---|
| Vol 9 Cognitive Control Plane v2, Part 5 (`ORCH-001..004`) | Restates each as `ORCH-00x-R1`, modality explicit (§6). |
| Amendments CCP Batch 1 (`OWN-001`, `LAYER-001`, `SHARD-001`) | Restates each as `*-R1` (§6). |
| Invariant Registry v1 | Consumed as the authoritative list; unchanged. |
| Ontology Spec v1, Parts 3–6 (Cognitive Entity, Fact/Evidence, aspects) | Sourced verbatim for glossary defs GL-001, GL-002 (§7). |
| Reference Lifecycle v1, Part 6 (CCP-GATE) | Supplies the ratification gate this doc passes (§9). |
| Scenario Conformance Framework v1 | Supplies the term `Conformance` (GL-014) and verdict semantics referenced by §9. |
| ACS-001/CCP-001 Content Addressing | Reused for §8 addressing; domain tags extended into the reserved range. |

Nothing above is edited. Every entry is an *extension* that points back to a
frozen clause.

## 4. Normative keywords (normative)

ARVES documents SHALL interpret the keywords **MUST**, **MUST NOT**, **REQUIRED**,
**SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **NOT
RECOMMENDED**, **MAY**, and **OPTIONAL** as described in RFC 2119 as updated by
RFC 8174.

- The keywords carry normative force **only** when they appear in ALL-CAPITALS.
  A lower-case "must" or "shall" in ARVES prose is descriptive and carries no
  conformance weight (RFC 8174).
- **MUST** and **SHALL** (and **REQUIRED**) are exact synonyms in ARVES; an
  author SHOULD prefer **MUST**/**MUST NOT** for absolute requirements and
  **SHALL**/**SHALL NOT** only where restating a frozen clause that already uses
  "shall". This document uses both to stay faithful to §6 sources.
- A conformance verdict SHALL treat violation of any **MUST**/**MUST NOT**/
  **SHALL**/**SHALL NOT** as **FAIL**. Violation of a **SHOULD**/**SHOULD NOT**
  SHALL be recorded but SHALL NOT by itself force FAIL (it MAY force PARTIAL per
  the Scenario Conformance Framework, Part 8). A **MAY**/**OPTIONAL** clause
  SHALL NOT be a basis for any non-PASS verdict.
- An author MUST NOT invent additional normative keywords (e.g. "will",
  "always", "guaranteed", "never" used as an obligation). Such words, where they
  encode force, SHOULD be rewritten to a keyword above. This rule is enforced by
  the AEOS checker (§9) as a lint, not as a conformance FAIL against a runtime.

## 5. Requirement-ID convention (normative)

Every normative clause SHALL be addressable by a **Requirement ID** so that a
probe, amendment, verdict, or cross-reference can cite exactly one obligation.

**Grammar (normative):**

```
RequirementId = Owner "-" Seq "-R" ReqNo
Owner         = uppercase-alpha { uppercase-alpha | "-" }   ; e.g. ORCH, OWN, LAYER, SHARD, CCP
Seq           = 3*DIGIT                                       ; zero-padded, e.g. 001
ReqNo         = 1*DIGIT                                       ; 1-based within the owning clause
```

- Examples: `ORCH-001-R1`, `SHARD-001-R2`, `CCP-005-R3`.
- A Requirement ID SHALL be **stable for the lifetime of the standard**: once
  assigned it MUST NOT be reused for a different obligation, and it MUST NOT be
  renumbered. If a requirement is retired, its ID SHALL be marked WITHDRAWN and
  SHALL NOT be re-issued (compare CVE / RFC errata practice).
- A single invariant MAY decompose into several Requirement IDs (`-R1`, `-R2`,
  …). When it does, the conjunction of its `-Rn` requirements SHALL be
  meaning-equivalent to the frozen invariant; no `-Rn` may add force the frozen
  source does not carry.
- The `Owner` token SHALL match the citation key already used by the reference
  runtime and the Invariant Registry (e.g. `"ORCH-001"`), so a Requirement ID is
  a strict refinement of an existing citation and never a new namespace.
- Requirement IDs are the ONLY sanctioned unit for machine-checked citation in
  conformance artifacts; a node probe that asserts an invariant SHOULD record the
  specific `-Rn` it exercised, not merely the invariant ID.

## 6. Restated registered invariants (normative)

The following restate the 7 REGISTERED invariants as keyworded, addressable
requirements. Modality is made explicit; **meaning is unchanged**. Each row cites
its frozen source; per §2 the source prevails on any apparent divergence.

### 6.1 Control-plane invariants (source: Vol 9 Part 5)

| Req ID | Requirement (normative) | Frozen source clause |
|---|---|---|
| `ORCH-001-R1` | The Control Plane (GL-007) **MUST NOT** own Cognitive Truth (GL-003); only the Kernel (GL-006) **MAY** own Cognitive Truth. | "The Control Plane owns no truth. Only the Kernel owns cognitive truth." |
| `ORCH-002-R1` | The Control Plane **MUST** produce plans only; it **MUST NOT** hold persistent state and **MUST** be restartable and stateless over the Kernel and Persistence. | "The Control Plane produces plans, never persistent state." |
| `ORCH-003-R1` | Every execution **MUST** be replayable (GL-012) from the same Goal, State, Policies, Capabilities and Runtime Fingerprint **via a recorded Decision Trace (GL-009)**, and **MUST NOT** rely on recomputation for that replay. | "Every execution is REPLAYABLE … via a recorded decision trace, not by recomputation." |
| `ORCH-004-R1` | Every Engine (GL-010) and Capability (GL-013) invocation **MUST** be idempotent and content-addressable (GL-008); a re-invocation with an equal Content Address **MUST** resolve to the existing outcome and **MUST NOT** produce a second Commit (GL-005). | "Every engine and capability invocation is idempotent and content-addressable." |

> Note on `ORCH-004-R1`: the "at most one Commit per equal address" clause is the
> idempotency reading already fixed by ACS-001 §6 (Idempotency binding); it is
> restated here for modality only, not extended.

### 6.2 Ownership, layering, partition invariants (source: Amendments CCP Batch 1)

| Req ID | Requirement (normative) | Frozen source clause |
|---|---|---|
| `OWN-001-R1` | Every unit of state **MUST** have exactly one owning layer; two layers **MUST NOT** both own the same state. | "Every state has exactly one owner." |
| `LAYER-001-R1` | Inter-layer dependencies **MUST** point downward only; a layer **MUST NOT** make a lateral peer-layer call. | "Dependencies point downward only; no lateral peer-layer calls…" |
| `LAYER-001-R2` | A cross-cutting concern **MUST** traverse the Control Plane (GL-007) or the Event Fabric rather than couple two peer layers directly. | "…cross-cutting traverses the Control Plane or Event Fabric." |
| `SHARD-001-R1` | All Tenant/Workspace (GL-004) state **MUST** be partitioned into a Shard (GL-011) keyed by tenant and workspace; a Shard **MUST NOT** contain cross-tenant data. | "Partition by tenant/workspace…" |
| `SHARD-001-R2` | An entity's partition (Shard) key **MUST** be immutable for the entity's lifetime; an implementation **MUST NOT** re-key a live entity. | "…the partition key is immutable for an entity lifetime." |

> `LAYER-001` and `SHARD-001` each decompose into two Requirement IDs because the
> frozen sentence conjoins two independently-checkable obligations; the
> conjunction is meaning-equivalent to the source (§5).

## 7. Terms-and-Definitions Glossary (normative)

Definitions are written in **necessary-and-sufficient-condition** style: the
`Definition` states what the term denotes; `Necessary` lists conditions an
instance MUST satisfy; `Sufficient` lists conditions that, together, MUST qualify
an instance. Each term has a stable **Term ID** (`GL-nnn`) subject to the same
stability rule as Requirement IDs (§5). A capitalized use of a defined term in an
ARVES normative document is a reference to its glossary entry.

| Term ID | Term | Definition (necessary/sufficient) | Grounded in |
|---|---|---|---|
| `GL-001` | **Cognitive Entity** | The root type of the ontology: a thing with identity in the cognitive world. *Necessary:* has an Identity aspect (stable id + type urn) and a Tenant Scope. *Sufficient:* being any `uci.*` type (Fact, Event, Goal, …), which all subtype `uci.entity`. | Ontology Spec Parts 3–6 (O-001, O-002) |
| `GL-002` | **Cognitive Truth** | A validated truth claim (`uci.fact`) that has been committed by the Kernel and is thereby authoritative. *Necessary:* derived from validated Evidence (O-004) **and** made durable by a Kernel Commit. *Sufficient:* being the committed outcome of a `Commit` at some `CommitIndex`. Distinct from Working Memory, which is live and mutable and is never truth. | Ontology O-004; Vol 9 ORCH-001; Amendment-001 |
| `GL-003` | **Truth (ownership sense)** | The class of state whose single owner is the Kernel. Used in `ORCH-001-R1`/`OWN-001-R1`. *Necessary:* is Cognitive Truth (GL-002). *Sufficient:* has passed through the sole Commit gateway. No non-Kernel layer MAY own it. | Vol 9 ORCH-001; Layer Matrix (Kernel row) |
| `GL-004` | **Tenant** (and **Workspace**) | The outermost isolation boundary (Tenant) and its nested sub-boundary (Workspace) within which every operation occurs. *Necessary:* every entity and every operation has a `(tenant, workspace)` scope; "no scope means no execution". *Sufficient:* being addressed by a `(tenant_id, workspace_id)` pair. The pair is the primary partition key (see GL-011). | Vol 1 §18; Amendment-004; Ontology Tenant-Scope aspect |
| `GL-005` | **Commit** | The single, atomic act by which the Kernel turns a proposed write into Cognitive Truth, and the ONLY gateway by which anything becomes truth. *Necessary:* performed by the Kernel; assigns a `CommitIndex`; is content-addressable and idempotent (`ORCH-004-R1`). *Sufficient:* a proposed write accepted by the shard leader and appended once to the committed log. | Kernel crate purpose; ORCH-001/004; ACS-001 §6 |
| `GL-006` | **Kernel** | The layer that owns Cognitive Truth and is the sole Commit gateway; it is *per-shard*. *Necessary:* owns truth (GL-003); commits truth and emits events; MUST NOT orchestrate, plan, or execute. *Sufficient:* the single authoritative owner of committed canonical state. | Layer Matrix (Kernel row); ORCH-001 |
| `GL-007` | **Control Plane** *(orchestration sense)* | The stateless, replayable reasoning-to-action layer that expands a Goal into an Engine Graph and sequences it. *Necessary:* owns the Plan/Engine-Graph artifact but MUST NOT own truth (`ORCH-001-R1`) and MUST NOT hold persistent state (`ORCH-002-R1`). *Sufficient:* a layer that produces plans and schedules and persists nothing. **This is NOT the CAP-theorem "CP" (see GL-007b).** | Vol 9 Parts 2–5; Layer Matrix (Control Plane row) |
| `GL-007b` | **CP** *(consistency sense)* — DISAMBIGUATION | The Consistency-over-Availability pole of the CAP theorem, as chosen for the Kernel by IDR-001 ("Kernel = CP"). *Necessary:* a consistency posture, not a layer. An ARVES document MUST write "Control Plane" (GL-007) for the orchestration layer and MUST write "CP (consistency)" or "CP posture" for the CAP pole; the bare token "CP" SHOULD NOT be used where either reading is possible. | IDR-001; CLAUDE.md Distributed Decisions |
| `GL-008` | **Content Address** | The self-describing multihash `0x12 0x20 ‖ SHA-256(domain_tag ‖ body)` that names a payload by its content (ACS-001). *Necessary:* self-describing (carries its hash code); computed over a domain-tagged pre-image; equal address ⇒ same content. *Sufficient:* the 34-byte SHA-256 multihash of the domain-tagged canonical pre-image. | ACS-001/CCP-001 §§4–5 |
| `GL-009` | **Decision Trace** | The recorded, replay-sufficient record of one execution: the expanded Engine Graph, engine outputs, arbitration choices, policy evaluations, and the Runtime Fingerprint. *Necessary:* sufficient to replay the run deterministically (`ORCH-003-R1`) without recomputation. *Sufficient:* the Vol 9 Part 9 record set. | Vol 9 Part 9; ORCH-003 |
| `GL-010` | **Engine** | A pure, stateless Data Plane function that reads state and produces inference and proposed effects. *Necessary:* owns nothing persistent; MUST NOT mutate truth; per-invocation scratch is ephemeral. *Sufficient:* a manifest-declared invocation whose output is inference/proposed-effect, never a Commit. | Vol 9 Part 3; Engine Graph Part 4; Amendment-001 |
| `GL-011` | **Shard** | The unit of partition and consensus: the state owned by exactly one `(tenant, workspace)` key, committed by one per-shard Raft group. *Necessary:* keyed by an immutable partition key (`SHARD-001-R2`); MUST NOT hold cross-tenant data. *Sufficient:* the truth located by one `ShardKey`. | Amendment-004; SHARD-001; IDR-001 |
| `GL-012` | **Replay** | Reconstruction of an execution's outcomes by re-reading a recorded Decision Trace deterministically — NOT by re-computing non-deterministic engines. *Necessary:* driven from GL-009; yields the recorded outcomes. *Sufficient:* deterministic re-read of the trace + Runtime Fingerprint. | Vol 9 Part 9; ORCH-003 |
| `GL-013` | **Capability** | A functional ability (`uci.capability`) that is bound by the Capability Fabric at execution time, never owned as truth. *Necessary:* declared as a manifest requirement; bound, not selected, by the Fabric; invocation is idempotent and content-addressable. *Sufficient:* a registry binding resolved for a plan. | Ontology `uci.capability`; Layer Matrix (Capability row); ORCH-004 |
| `GL-014` | **Conformance** | The structural, property-based and invariant-based judgement that a runtime upholds ARVES obligations for a scenario, yielding PASS / PARTIAL / FAIL — NOT golden-output equality. *Necessary:* invariant and property checks per the Scenario Conformance Framework Part 8; asserted against a stated suite version and spec version. *Sufficient:* all required invariants (`ORCH-001..004`, ownership/isolation) and properties hold. | Scenario Conformance Framework Parts 8, 10–11 |

> The glossary is deliberately closed at the 14 load-bearing terms named by
> R-06. Adding a term uses the same instruments as any other change (CCP /
> Amendment / IDR) and a new `GL-nnn`; a term MUST NOT be redefined by silent
> edit. `Data Plane` and `Tenant`/`Workspace` appear as capitalized normative
> terms in the checker list (§9) and resolve to GL-010's owning plane and GL-004
> respectively; `Data Plane` is defined inline as the pure-execution plane (the
> Engine/Capability/Execution layers) that owns nothing persistent.

## 8. Content address of this convention (normative)

To make the glossary and the requirement set independently citable and
tamper-evident, CCP-005 defines two content addresses using the ACS-001 multihash
(`0x12 0x20 ‖ SHA-256(domain_tag ‖ body)`). This EXTENDS the ACS-001 domain-tag
table into its reserved range (`0x06`–`0x7F`); it does not change any existing
tag.

| Tag | Domain (added by CCP-005) |
|-----|--------|
| `0x06` | normative-glossary term-set (body = canonical term list) |
| `0x07` | requirement clause (body = the keyworded clause text) |

- The **Term-Set Address** SHALL be the content address of the body formed by the
  glossary Term IDs `GL-001`…`GL-014`, sorted ascending, joined by a single `\n`
  (LF, U+000A), UTF-8, no trailing newline, under tag `0x06`.
- A **Requirement Address** SHALL be the content address of the exact clause text
  under tag `0x07`.
- Two implementations that publish the same glossary SHALL compute the same
  Term-Set Address (differential conformance). A change to the term set changes
  the address, giving a one-line integrity check that the corpus and a runtime
  agree on the vocabulary.

## 9. Conformance scenario (CCP-GATE requirement) — `CCP-005-CS-1`

**Intent.** An **AEOS-style checker** (the corpus lint of the AEOS Master Index)
scans an ARVES normative document, extracts every capitalized *defined term* and
every ALL-CAPS *keyword usage*, and FAILS the document if any capitalized
normative term lacks a glossary entry, or if a requirement lacks a Requirement
ID. `CCP-005-CS-1` pins, byte-exactly, (a) the glossary term-set the checker must
know, (b) the term-name list it scans for, and (c) one restated requirement
clause. Digest = SHA-256; ContentId = `1220 ‖ SHA256(pre-image)` (ACS-001).

### 9.1 The term list the checker enforces (normative)

The checker SHALL require a glossary entry for **each** of the following
capitalized normative terms; a document that uses any of them without a resolvable
`GL-nnn` entry SHALL be reported non-conformant:

```
Capability, Cognitive Entity, Cognitive Truth, Commit, Conformance,
Content Address, Control Plane, Data Plane, Decision Trace, Engine,
Kernel, Replay, Shard, Tenant
```

(14 terms; `Workspace` resolves via `Tenant`/GL-004 and `CP` is the disambiguation
alias GL-007b, both intentionally covered by their parent entries.)

### 9.2 Byte-exact vectors (normative)

A conformant checker/implementation, given the pre-image bytes below, SHALL
produce exactly the stated ContentId. Two independent implementations SHALL agree
on all three vectors.

| # | domain | body (canonical) | pre-image (hex) | ContentId (hex) |
|---|--------|------------------|-----------------|-----------------|
| 1 | `0x06` term-set (Term IDs) | `GL-001\nGL-002\n…\nGL-014` (LF-joined, sorted, no trailing LF) | `06474c2d3030310a474c2d3030320a474c2d3030330a474c2d3030340a474c2d3030350a474c2d3030360a474c2d3030370a474c2d3030380a474c2d3030390a474c2d3031300a474c2d3031310a474c2d3031320a474c2d3031330a474c2d303134` | `1220fb32705645b16b7231d5cc98ff6d6dd931e1610e95439d97a747e388c1fcf49b` |
| 2 | `0x07` requirement clause | `ORCH-001-R1: The Control Plane MUST NOT own cognitive truth; only the Kernel MAY own cognitive truth.` | `074f5243482d3030312d52313a2054686520436f6e74726f6c20506c616e65204d555354204e4f54206f776e20636f676e69746976652074727574683b206f6e6c7920746865204b65726e656c204d4159206f776e20636f676e69746976652074727574682e` | `1220da37d0635c49ca8648f54df28562a1673976cf9de290492b06a9f5f78e9e00c3` |
| 3 | `0x06` term-name list | `Capability\nCognitive Entity\n…\nTenant` (the §9.1 list, LF-joined, sorted, no trailing LF) | `064361706162696c6974790a436f676e697469766520456e746974790a436f676e69746976652054727574680a436f6d6d69740a436f6e666f726d616e63650a436f6e74656e7420416464726573730a436f6e74726f6c20506c616e650a4461746120506c616e650a4465636973696f6e2054726163650a456e67696e650a4b65726e656c0a5265706c61790a53686172640a54656e616e74` | `1220ceb1bf2eae8aea00e78867727d41df53493fa95b421de0225ed4b5a546231619` |

### 9.3 Pass/fail semantics (normative)

- A checker that, run over this document, reports **every** §9.1 term as *defined*
  and **every** `ORCH-/OWN-/LAYER-/SHARD-*-Rn` requirement as *ID-bearing* SHALL
  emit verdict PASS for the language axis.
- A checker that computes a different ContentId for any vector in §9.2, or that
  reports PASS for a document in which any §9.1 term lacks a glossary entry, SHALL
  be non-conformant.
- A checker that flags a **lower-case** "must"/"shall" as a normative violation
  SHALL be non-conformant (RFC 8174: only ALL-CAPS carry force).

The vectors are reproducible with SHA-256 over the stated pre-image bytes; §11
records the exact procedure used to compute them.

## 10. Proposal analysis

- **Why it matters.** A standard is only as strong as its ability to be *cited*
  and *checked* word-for-word. ISO/IEC Directives Part 2 and IEEE-SA both make a
  normative-language clause and a Terms-and-Definitions clause mandatory
  front-matter for exactly this reason. R-06 is the prerequisite that lets every
  *other* ARVES standard be verifiable rather than merely readable; ACS-001
  already "seeds the Goal-4 convention by example" — CCP-005 makes the convention
  itself normative and testable.
- **Risks.** (a) *Restatement drift* — a keyworded restatement could subtly
  change meaning. Mitigated by §2's interpretation rule (frozen source prevails),
  by recording each source clause in §6, and by keeping restatements to modality
  only. (b) *Glossary rot* — definitions could diverge from evolving usage.
  Mitigated by content-addressing the term set (§8) so any divergence flips the
  Term-Set Address and is caught by `CCP-005-CS-1`. (c) *Over-linting* — an
  aggressive checker could FAIL prose for stylistic keyword use; mitigated by
  scoping the lint to ALL-CAPS force and making keyword-invention a SHOULD, not a
  runtime FAIL.
- **Long-term consequences.** In 20 years the vocabulary and the modality of
  every ARVES obligation are pinned to bytes, not to a rendering of a `.docx`.
  A future translator, formal-methods tool, or third-party certifier can consume
  the term set and requirement IDs mechanically. Standards that skipped a
  normative-language clause (early web specs, many enterprise "architecture
  standards") became un-citable and forked; ARVES avoids that failure mode.
- **Alternatives considered.** (i) *Leave modality implicit and rely on reviewer
  judgement* — rejected: not independently reproducible, defeats certification.
  (ii) *Edit the frozen invariants in place to add keywords* — rejected: violates
  ED-001 (frozen means frozen); the whole point is to clarify without reopening
  the Specification Era. (iii) *Per-document glossaries* — rejected: guarantees
  divergent definitions of "Control Plane"; a single corpus glossary with stable
  IDs is the only vendor-neutral option. (iv) *Adopt only RFC 2119 without a
  requirement-ID convention* — rejected: keywords without citable IDs still can't
  express partial conformance or targeted amendments.
- **Recommendation.** Ratify via CCP-GATE with `CCP-005-CS-1`. Then (a) the AEOS
  checker adopts §9.1 as its required-term list and §9.2 as its self-test
  vectors; (b) new ARVES standards cite requirements by `-Rn` ID; (c) the
  reference runtime's `arves-invariants` crate, which already exposes stable
  citation keys (`ORCH-001`, …), gains the `-Rn` refinement as documentation —
  no behaviour change. This is a clarification, so no invariant proof status
  changes.
- **Implementation complexity.** Standard: low–medium (prose discipline +
  three SHA-256 vectors). Checker: low (term extraction + set membership + a
  multihash). Reference-runtime impact: near-zero (documentation-only).
- **Scientific impact.** Converts "the Control Plane owns no truth" from an
  English sentence into a citable, machine-checkable requirement `ORCH-001-R1`
  with a byte-pinned identity, making conformance a decidable property of a
  document, not an opinion about it.
- **Ecosystem impact.** Unblocks R-06 and, transitively, every downstream
  standard's front-matter; enables independent certifiers and translators to
  agree on vocabulary; removes the `Control Plane`/`CP` trap that would otherwise
  mislead a second-runtime team into building a stateful orchestrator.

## 11. Reproducibility of the vectors (informative)

The §9.2 ContentIds were computed as `multihash(SHA-256) = 0x12 0x20 ‖
SHA256(pre-image)` where `pre-image = domain_tag ‖ body`, `body` UTF-8, term/ID
lists sorted ascending and joined by a single `\n` with no trailing newline —
identical framing to ACS-001 §§4–5. Any implementation that follows this
procedure reproduces the three ContentIds bit-for-bit; that reproduction is the
CCP-GATE evidence.

## 12. Dependencies & sequence

- **Depends on:** ACS-001/CCP-001 (multihash reused in §8) — already DRAFT; the
  frozen Invariant Registry, Vol 9, and Amendments CCP Batch 1 as read-only
  sources; the Reference Lifecycle CCP-GATE (Part 6) as the ratification gate.
- **Blocks / enables:** every subsequent ARVES v1.1 standard's normative-language
  and terms front-matter (they cite this convention); the AEOS corpus lint; the
  requirement-level conformance citations used by the Scenario Conformance
  Framework's node probes.

---

*Ratification path (Reference Lifecycle): DRAFT → CCP-GATE (this doc +
`CCP-005-CS-1`) → Candidate → Ratified. On ratification this becomes a registered
normative addition (a language convention + glossary); the frozen v1.0 corpus and
the 7 registered invariants are unchanged in meaning — only made explicit and
citable.*

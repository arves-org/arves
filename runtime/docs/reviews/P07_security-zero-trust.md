# ARVES Independent Chief-Architect Review — Prompt 7: Security (Zero-Trust)

**Reviewer role:** Independent, ISO/IEEE-grade Chief Architect. Objective: maximize the probability that ARVES becomes an internationally adoptable cognitive-infrastructure standard over a 20-year horizon.
**Lens:** Zero-trust security. Threat model = nation-state, malicious insider, supply-chain, prompt injection, compromised capability, compromised engine, kernel attack, replay attack.
**Corpus status:** FROZEN. No finding below proposes modifying the frozen specification. Every proposal is one of: IDR (engineering decision), CCP Amendment (minor, additive, via CCP-GATE), Runtime, Verification, Certification, Ecosystem, or Product.
**Date:** 2026-07-02

---

## Executive Summary

ARVES has an *architecturally strong* trust skeleton (single truth owner, single commit gateway, per-tenant sharding, append-only WAL = decision trace, provenance/trust as mandatory ontology aspects, tenant isolation and zero-trust *named* as principles in Vol 2 Part 21). But when read against a real adversary, the security story is almost entirely **nominal, not mechanical**:

- **Volume 17 (Security & Governance Atlas) is a one-page list of nouns.** It names Identity/Data/Knowledge/Agent/Model/Runtime/Infrastructure security, compliance (GDPR/ISO27001/SOC2), and audit (who/what/when/why) — but defines **zero mechanisms, zero threat model, zero cryptographic controls, zero trust boundaries**. The Gap Analysis itself flags "No compliance control mapping … controls are not mapped to mechanisms" (Gap Analysis, Area 6). Vol 2 declares "Zero Trust" as a principle but provides no enforcement contract.
- **The reference runtime has no security whatsoever.** `arves-kernel::commit` accepts any `ProposedWrite` from any caller: no authentication of the proposer, no authorization check, no tenant-scope verification, no signature. `#![forbid(unsafe_code)]` is the *only* security control present in code.
- **The WAL/decision trace is tamper-evident against accidents, not adversaries.** `arves-persistence` uses CRC32 (IEEE 802.3) to detect torn writes and bit-rot. CRC32 is trivially forgeable; there is **no hash chain, no Merkle root, no signature** binding record N to record N-1. A malicious insider or a compromised host with disk access can rewrite committed truth and recompute the CRC, and recovery will accept it as authentic. This directly undermines the entire ORCH-003 "replay from recorded decision trace" guarantee — the trace is the root of trust for the whole system and it is unauthenticated.
- **`ContentHash`/`ContentId`/`InvocationKey` are called "content addresses" but the hash function is unspecified** (`ContentHash(pub Vec<u8>)`, byte layout "intentionally unspecified"). Content-addressing that is not pinned to a **collision-resistant, versioned cryptographic hash** is not an integrity primitive — it is a dictionary key. ORCH-004 idempotency is a *correctness* property here, but the same field is load-bearing for de-dup and replay, so a weak/absent hash is also a *security* problem (idempotency-key collision → truth confusion / poisoning).
- **The "Trust" ontology aspect is self-asserted by untrusted connectors** and the spec explicitly says the Kernel "adjudicates" — but no adjudication mechanism, evidence-validation rule (O-004 "Truth emerges from validated Evidence"), or provenance-signing is specified or implemented. This is the prompt-injection / data-poisoning entry point and it is wide open.
- **Capability and engine execution have no sandbox model.** The Capability Fabric records a `ProviderId` (an opaque string) and an `EffectClass` but there is no isolation contract, no least-privilege binding, no signed manifest verification. A compromised capability/engine runs with whatever ambient authority the host process has.
- **There is no supply-chain integrity story** for engine manifests, capability providers, or the reference runtime itself, despite the Engine Graph Spec explicitly modeling itself on the OCI Image Spec (which *does* have signing/attestation via cosign/in-toto).

**Verdict for the lens:** If ARVES were submitted to ISO/IEEE tomorrow, the security dimension would fail. A standard that claims "Zero Trust" and "Defense in Depth" (Vol 2 Part 21) as success criteria (Vol 2 Part 25: "100% Tenant Isolation … Policy Enforcement") but provides no normative security architecture, no threat model, no cryptographic tamper-evidence, and no conformance property that an adversary must fail to defeat, is not certifiable. The good news: the architecture's *shape* (single gateway, per-shard log, mandatory aspects, content-addressing, decision-trace-as-root) is exactly right to bolt a real zero-trust model onto **without touching the frozen corpus** — everything below is additive (IDR + Runtime + Certification + one CCP Amendment for a security-conformance axis).

---

## Severity-Ranked Findings

| # | Severity | Title | Instrument | Impl. Complexity |
|---|----------|-------|-----------|------------------|
| S1 | Critical | Decision trace / WAL has no cryptographic tamper-evidence (CRC32 only) | IDR + Runtime + Certification | High |
| S2 | Critical | Kernel commit gateway has no authN/authZ/tenant-scope enforcement | IDR + Runtime | High |
| S3 | Critical | Content-address hash is unspecified — not collision-resistant/versioned | IDR + Runtime | Medium |
| S4 | Critical | No normative threat model or security architecture (Vol 17 is nouns) | CCP Amendment + Verification | Medium |
| S5 | High | Trust/Provenance aspects are self-asserted; no evidence validation (prompt injection / poisoning) | IDR + Runtime + Certification | High |
| S6 | High | No capability/engine sandbox or least-privilege binding contract | IDR + Runtime | Very-High |
| S7 | High | No supply-chain integrity for engine/capability manifests or runtime | IDR + Certification + Ecosystem | High |
| S8 | High | Cross-tenant blast radius: shared process, no key-per-tenant crypto isolation | IDR + Runtime | High |
| S9 | Medium | No security conformance axis in the 12-axis Scenario Framework | Certification | Medium |
| S10 | Medium | Recovery trusts local disk implicitly; no quorum/peer attestation on single node | IDR + Runtime | Medium |
| S11 | Medium | No audit-trail integrity/immutability guarantee distinct from the WAL | Runtime + Certification | Medium |
| S12 | Low | `panic!`-on-recovery is a self-inflicted DoS surface | IDR + Runtime | Low |

---

## S1 — Decision trace / WAL has no cryptographic tamper-evidence (Critical)

**Where.** `arves-persistence/src/lib.rs`: `crc32_ieee` (line ~565), `encode_body`/`decode_body` (frame = `[len][body][crc32]`), `encode_snapshot`/`decode_snapshot`. IDR-005 (Raft log = WAL = decision trace, append-only). ORCH-003 (replay from recorded trace).

**Why it matters.** The decision trace is the **root of trust for the entire system**. Every correctness and reproducibility claim (ORCH-003 replay, IDR-002 outcome replication, conformance regression, audit "who/what/when/why") reduces to "the WAL faithfully records what was committed." The runtime protects this with CRC32, which is a *checksum for accidental corruption*, not an integrity primitive. CRC32 is linear and trivially recomputable: an attacker who can write to a shard directory (malicious insider, compromised node, backup exfiltration, nation-state persistence) can (a) rewrite the payload of a committed record, (b) reorder or delete interior records, (c) forge an entire alternate history, and (d) recompute the CRC so recovery accepts it as pristine. `decode_all`/`replay_from` will happily reconstruct forged "truth," and `truth_hash()` (an FNV-1a fold, also non-cryptographic) will report a stable value for the forged set. There is no chaining: record N does not commit to record N-1, so a spliced tail is undetectable.

**Attack path.** Insider with filesystem access → open `seg-*.wal` → rewrite an `Outcome` payload (e.g. change an approval decision, a risk verdict, a financial fact) → recompute CRC32 over the new body → on next recovery the Kernel loads the forged record as committed truth. Blast radius: **total loss of integrity for one shard's entire committed history**, silent. Under replication (I2) a compromised leader could propose forged outcomes that followers apply verbatim (IDR-002 says followers "do NOT recompute").

**Long-term consequences.** No forensic defensibility (a legal/regulated deployment cannot prove its audit log is authentic), no basis for third-party certification of integrity, and a permanent ceiling on adoption in finance/health/gov. This is the single most damaging gap for ISO/IEEE grade.

**Alternative designs.**
1. **Hash-chained WAL (recommended, minimal):** each frame stores `prev_hash` and `body_hash = H(body)`; `frame_hash = H(prev_hash || body_hash)`. Recovery verifies the chain; any tamper breaks it. Cheap, append-only-friendly, per-shard.
2. **Merkle-tree per segment + signed segment roots:** enables efficient inclusion proofs and partial audit disclosure; heavier.
3. **Signed commits:** the shard leader signs each committed outcome (Ed25519); followers/auditors verify. Combines with (1) for authenticated ordering.
4. **External transparency log** (Trillian/CT-style) anchoring segment roots off-host — strongest against a fully-compromised node, but adds an external dependency.

**Recommendation.** IDR-006 "Decision-Trace Tamper-Evidence": adopt (1)+(3) — a per-shard hash chain with the leader signing the head. Runtime: extend the frame format under `WAL_FRAME_VERSION` (already a versioned migration hook, cited RT-001) to carry `prev_hash`, `body_hash`, and an optional `sig`; verify on `replay_from`/recovery; make `truth_hash()` a cryptographic digest. Certification: add an integrity scenario that mutates a committed record and asserts recovery FAILS loudly. This is purely additive — the frozen spec already calls the log "append-only" and "the single source for deterministic replay"; tamper-evidence *implements* that promise.

**Complexity:** High. **Scientific impact:** brings ARVES to parity with verifiable-log research (CT, Trillian, verifiable state machines). **Ecosystem impact:** unlocks regulated verticals and independent audit.

---

## S2 — Kernel commit gateway has no authentication, authorization, or tenant-scope enforcement (Critical)

**Where.** `arves-kernel/src/lib.rs`: `Kernel::commit(&self, proposed: ProposedWrite) -> Result<TruthRef, CommitError>` and `RefKernel::commit`. The `ProposedWrite` carries `{shard, content, payload}` — **no principal, no credential, no capability token, no signature.** Vol 2 Part 18/19 (authN: password/SSO/OIDC/OAuth2/API keys/service accounts; authZ: RBAC+ABAC), Part 21 (Zero Trust, Least Privilege). Amendment-004 (SHARD-001 tenant partitioning).

**Why it matters.** "Zero trust" means *no request is trusted by virtue of its origin* — every commit must carry a verifiable principal and be authorized against the target shard/resource. The sole commit gateway is exactly where this must be enforced (it is the one place all truth mutations funnel through — an ideal, non-bypassable policy enforcement point). Today it is an open door: any code holding a `Kernel` reference can commit anything into any `ShardKey`, including another tenant's shard (nothing checks that the caller is entitled to `proposed.shard`). `CommitError` has variants for `NotLeader`, `AlreadyCommitted`, `UnknownShard`, `Rejected`, `NotReplicated` — but **none for `Unauthenticated`/`Unauthorized`/`ScopeViolation`.** The type system actively omits the security boundary.

**Attack path.** (a) Compromised engine or capability that reaches the commit path proposes a write into `tenant=victim` — accepted. (b) A privilege-confused Control Plane plan routes an outcome to the wrong shard — accepted. (c) Nation-state footholds one crate and, because there is no principal binding, forges truth for any tenant on the node. Blast radius: **cross-tenant truth forgery**, defeating the "100% Tenant Isolation" success criterion (Vol 2 Part 25).

**Long-term consequences.** Tenant isolation is the #1 selling point of a "Tenant-Aware Universal Intelligence Platform" (Vol 1 Part 1). If isolation is enforced only by *convention* (callers "should" set the right shard), it is not isolation. No multi-tenant SaaS or enterprise deployment is defensible.

**Alternative designs.**
1. **Capability-token commits (recommended):** `commit` takes a `CommitAuthorization` — a signed, scoped, time-bounded token (principal, allowed shard(s), allowed operation classes) minted by an Identity/Policy authority. Kernel verifies the token's signature and that `token.scope ⊇ proposed.shard` before appending. Object-capability style; composes with zero-trust.
2. **mTLS + principal header:** transport-level identity; weaker (does not bind the *specific* write) and doesn't survive replay/audit.
3. **ABAC policy call-out:** Kernel calls a Policy Decision Point per commit. Flexible but adds latency and a trust dependency on the PDP.

**Recommendation.** IDR-007 "Authenticated Commit": the commit surface accepts a verifiable, shard-scoped authorization; Kernel rejects unauthenticated/unauthorized/out-of-scope proposals with new (additive) error variants. Runtime: implement token verification at `RefKernel::commit` before WAL append; deny-by-default. This can be introduced as a wrapper type so the frozen `Kernel` *trait* contract is honored while the *reference implementation* enforces zero-trust. **Complexity:** High.

---

## S3 — Content-address hash is unspecified; not collision-resistant or versioned (Critical)

**Where.** `arves-kernel::ContentHash(pub Vec<u8>)` ("byte layout intentionally unspecified"); `arves-persistence::ContentId`; `arves-control-plane::InvocationKey(pub [u8;32])` ("expected to be a cryptographic digest … skeleton only fixes the shape"); `arves-information-platform::ContentHash`; `arves-consensus::ContentHash(pub String)`. ORCH-004 (idempotent + content-addressable).

**Why it matters.** ORCH-004 makes content-addressing load-bearing for idempotency, de-duplication, and replay collapse (`RefKernel::commit` keys the idempotency index on `content.0`). If the hash is not a **pinned, collision-resistant, algorithm-agile** primitive, two different payloads can share a `ContentHash` and one will be silently dropped as "already committed" (`CommitError::AlreadyCommitted`) — an attacker who can craft a collision can **suppress a legitimate write** (availability/integrity) or **cause a malicious payload to masquerade as an already-trusted one** (poisoning). Note the five crates use *five different shapes* (`Vec<u8>`, `String`, `[u8;32]`) for the same conceptual value — an interoperability and integrity hazard for the "independent implementability" acceptance bar.

**Attack path.** Compromised connector/engine computes a payload colliding (under a weak or unspecified hash, e.g. an accidental non-cryptographic FNV/CRC choice by an independent implementer) with a benign committed record → proposes it → Kernel returns `AlreadyCommitted` and the malicious intent is either dropped or aliased to trusted truth.

**Long-term consequences.** Content-addressing is the *spine* of replay and distribution (IDR-002/004). An unspecified hash means two conformant runtimes may not agree on identity — breaking cross-implementation replay and the two-track UCS/UCI portability guarantee (Reference Lifecycle Part 8).

**Alternative designs.** (1) Mandate a **multihash-style, self-describing digest** (algorithm id + length + bytes) so the address is algorithm-agile and versioned (SHA-256 baseline, upgrade path to SHA-3/BLAKE3). (2) Fixed SHA-256 only — simpler but no agility. (3) Keccak for future post-quantum posture.

**Recommendation.** IDR-008 "Canonical Content-Addressing": one normative digest scheme (self-describing multihash, SHA-256 default), one shared `ContentAddress` type reused across all crates (define once, e.g. in a tiny `arves-crypto`/`arves-types` crate). Runtime: implement canonical encoding + digest in the Information Platform (where canonicalization already lives) and thread it through. This is where content-addressing *should* be computed since canonicalization is already the connector's job. **Complexity:** Medium.

---

## S4 — No normative threat model or security architecture (Critical)

**Where.** Vol 17 (Security & Governance Atlas) — entire document is 32 lines of nouns. Vol 2 Part 21 names principles only. Gap Analysis Area 6: "No compliance control mapping … controls are not mapped to mechanisms."

**Why it matters.** ISO/IEC 27001, IEC 62443, Common Criteria, and NIST 800-53 all require an explicit, documented threat model and a mapping from *stated control* → *concrete mechanism* → *verification*. ARVES currently has stated controls with no mechanisms and no verification. A "Security by Design" claim (Vol 1 Part 9) with no adversary model is unfalsifiable — which the Reference Lifecycle Part 4 explicitly forbids ("unfalsifiable claims do not advance"). The frozen corpus cannot be edited, but a threat model is *derived analysis*, not a spec change: it can live as a companion normative document produced via CCP, exactly as the Invariant Registry was produced by "an independent audit of the frozen corpus."

**Attack path (meta).** Without a threat model, every finding in this report is discovered ad hoc rather than systematically; certifiers cannot judge completeness; and the eight required adversaries (nation-state, insider, supply-chain, prompt injection, compromised capability/engine, kernel attack, replay) have no home in the conformance suite.

**Long-term consequences.** No credible path to CC/27001/62443 certification; the "Certification Program" (deferred per Freeze Record) has nothing to certify against on the security axis.

**Alternative designs.** (1) STRIDE/LINDDUN per architectural layer (the Amendment-003 Layer Matrix is a ready-made decomposition). (2) MITRE ATT&CK mapping for AI/agentic systems. (3) Attack trees per asset (truth, trace, tenant boundary, capability).

**Recommendation.** CCP Amendment (companion doc, informative-then-ratified): **"ARVES Security Architecture & Threat Model v1"** — one section per Layer-Matrix layer, STRIDE per trust boundary, and a control→mechanism→conformance-scenario table (satisfies CCP-GATE). Verification: each control gets at least one machine-checkable property. This is the umbrella that organizes S1–S12. **Complexity:** Medium (analysis-heavy, no code).

---

## S5 — Trust/Provenance are self-asserted; no evidence validation (prompt injection / poisoning) (High)

**Where.** `arves-information-platform`: `Trust{confidence: f64}` ("connector-asserted … advisory only … only the Kernel adjudicates"), `Provenance{connector, source_kind, source_locator, raw_hash}`. Ontology O-003 (every observation has provenance), O-004 (truth emerges from validated evidence). No adjudication/validation mechanism exists in spec or code.

**Why it matters.** The Information Platform is "the ingress boundary of Reality … untrusted, heterogeneous sources (documents, event streams, APIs, sensors, human input)" (crate doc). This is precisely the **prompt-injection and data-poisoning surface**. The spec correctly states trust is advisory and the Kernel adjudicates — but **there is no adjudication rule, no evidence-validation engine, no provenance signature, and no quarantine tier.** A `confidence: 1.0` from a malicious connector flows straight to a `ProposedWrite`. Worse, downstream cognitive engines (LLMs) *read* canonical state via Query and can be steered by injected content that was ingested as "fact." O-004 ("truth emerges from validated evidence") is a *design principle*, explicitly "NOT runtime-provable" per the Invariant Registry — so nothing enforces it.

**Attack path.** Adversary controls a connected source (shared doc, compromised API, a user message) → embeds instructions/false facts → connector canonicalizes to a high-trust `ProposedWrite` → committed as truth → later engine reads it and acts (plans, decisions, executions) on injected content. Blast radius: cognitive corruption that propagates through the Information→Cognitive→Strategic→Execution spine (Vol 1 Part 17).

**Long-term consequences.** For an "intelligence platform," poisoned truth is the worst failure mode: it is durable (committed), authoritative (Kernel truth), and replayable (baked into the trace). No amount of downstream guardrail undoes committed poison.

**Alternative designs.** (1) **Signed provenance + source trust registry:** connectors sign proposals with a source-scoped key; a trust registry maps sources → max asserted trust; Kernel clamps `confidence` to the registry ceiling. (2) **Evidence-validation gate before commit:** implement O-004 as a runtime check — a proposal asserting a Fact must carry `Evidence` links (uci.evidence, `derived_from`) meeting a policy threshold, else it commits only as a low-trust Observation, not a Fact. (3) **Quarantine/staging tier:** untrusted ingress lands in an isolated staging shard, promoted to truth only after validation. (4) **Content provenance for LLM I/O** (data/instruction separation, spotlighting).

**Recommendation.** IDR-009 "Ingress Trust Adjudication": Kernel-side clamp of connector-asserted trust against a signed source registry + an evidence-gate that refuses to elevate Observation→Fact without validated Evidence (implements O-004 mechanically). Certification: a poisoning scenario asserting injected content cannot be committed as high-trust Fact. **Complexity:** High. **Scientific impact:** operationalizes O-004, a genuine research contribution to trustworthy AI.

---

## S6 — No capability/engine sandbox or least-privilege binding (High)

**Where.** `arves-capability-fabric`: `CapabilityBinding{capability, shard, version, provider: ProviderId(String), contract: InvocationContract{input_schema, output_schema, effect: EffectClass}}`. `arves-execution::Executor::execute` performs actions "to the world." Engine Graph Spec Part 10 ("Honour Capabilities Required via the Capability Fabric"). Vol 17 (Agent/Runtime/Infrastructure security named only).

**Why it matters.** Engines and capabilities are where **untrusted/third-party/marketplace code executes** (Agent Marketplace, Vol 14 Part 19; "Real products built entirely on ARVES," CLAUDE.md long-term objective 10). The binding records *what* to invoke and its *declared* effect class, but there is **no isolation contract**: no resource limits, no syscall/network egress policy, no filesystem confinement, no memory/token budget enforcement, no least-privilege grant scoping the capability to the shard/resources it may touch. `EffectClass::ProposesWrite` is *declared*, not *enforced* — a capability that declares `Pure` can still, in a real host, open sockets and exfiltrate. The Fabric explicitly "never dereferences or invokes" the provider, and Execution has no sandbox primitives, so a compromised capability runs with **full ambient host authority**.

**Attack path.** Malicious/compromised marketplace capability is bound (S7: no signature check on bindings) → invoked by Execution → reads other tenants' data on the host, exfiltrates over network, or proposes writes it wasn't scoped for (S2). Blast radius: host-wide, cross-tenant.

**Long-term consequences.** No marketplace can be trusted without capability sandboxing; this blocks long-term objectives 8–10 (Marketplace, Cloud platform, Real products).

**Alternative designs.** (1) **WASM/Wasmtime sandbox** per capability with an explicit host-function allowlist (capability-based, matches the "Capability" naming beautifully) — deny-by-default egress, metered fuel (token/compute budget from Vol 14 Part 18). (2) **OS-level isolation** (gVisor/Firecracker/seccomp) for heavier providers. (3) **Signed capability manifests** binding declared effects to an enforced policy (ties to S7). (4) **Effect-class enforcement:** the runtime *verifies* a `Pure` capability made no external calls (interpose the host API).

**Recommendation.** IDR-010 "Capability Isolation Model": every provider runs in a deny-by-default sandbox; the binding's `InvocationContract` becomes an *enforced* least-privilege grant (allowed reads, allowed effect, budget). Runtime: WASM host as the reference sandbox. Certification: a scenario where a capability declaring `Pure` attempts network egress and is blocked+recorded. **Complexity:** Very-High. **Ecosystem impact:** the precondition for a safe marketplace.

---

## S7 — No supply-chain integrity for manifests/providers/runtime (High)

**Where.** Engine Graph Spec Part 9 ("Manifests are content-addressable and versioned"; precedent explicitly "OCI Image Specification"), Part 11 (semantic versioning). Capability Fabric bindings reference `ProviderId(String)`. No signing, attestation, or provenance verification anywhere; no lockfile/SBOM discipline for the Rust runtime.

**Why it matters.** The Engine Graph Spec models itself on OCI images — but omits the half of OCI that provides *security*: image signing (cosign), provenance attestation (in-toto/SLSA), and admission verification. "Content-addressable" prevents accidental mismatch but **not** a malicious-yet-consistent manifest: an attacker who controls the registry/distribution channel serves a validly-hashed, malicious manifest. There is no notion of *who signed this engine* or *what provenance it has*. For the reference runtime crates themselves, there is no SBOM or dependency-pinning policy visible — a `cargo` supply-chain compromise (typosquat, malicious transitive dep) would land unsigned code in the trust base.

**Attack path.** Nation-state / supply-chain actor compromises an engine publisher or a build pipeline → publishes a signed-by-nobody manifest whose content hash the runtime accepts → runtime schedules and executes it (S6: no sandbox) → persistent foothold across every tenant that binds it.

**Long-term consequences.** Without SLSA-grade provenance, no regulated or government adoption; the "Third-party certification" and "Independent Runtime A/B" objectives cannot certify what they cannot verify.

**Alternative designs.** (1) **Sigstore/cosign-signed manifests + in-toto/SLSA provenance**, verified at bind/schedule time (admission control in the Capability Fabric / Control Plane). (2) **Transparency log** for published engines/capabilities (Rekor-style). (3) **Reproducible builds** for the reference runtime + published SBOM (CycloneDX) + `cargo-vet`/`cargo-deny` in CI.

**Recommendation.** IDR-011 "Supply-Chain Integrity": signed, provenance-bearing engine/capability manifests, verified before binding/scheduling; SLSA-L3 build + SBOM for UCI. Certification: a "reject unsigned/untrusted-provenance manifest" scenario. Ecosystem: an ARVES transparency log as shared infrastructure. **Complexity:** High.

---

## S8 — Cross-tenant blast radius: shared process, no per-tenant crypto isolation (High)

**Where.** SHARD-001 partitions state logically by tenant/workspace; `FileWalStore` stores each shard as a directory. But all shards share one process, one filesystem, one memory space; payloads are plaintext bytes; there is no per-tenant encryption key. Vol 2 Part 25 ("100% Tenant Isolation").

**Why it matters.** Logical partitioning is not a security boundary against a host-level adversary. A single memory-safety bug in any dependency, a side-channel, or a compromised capability (S6) reads *all* tenants' plaintext truth from disk/memory. "100% tenant isolation" cannot be claimed while isolation is only a naming convention over a shared trust domain and data at rest is unencrypted and unkeyed.

**Attack path.** Compromised capability or dependency on a multi-tenant node → reads `seg-*.wal` for every tenant directory (all plaintext) → exfiltrates. Blast radius: every tenant co-located on the node.

**Long-term consequences.** Enterprise/regulated tenants require cryptographic tenant isolation (BYOK/HYOK) and often dedicated key domains; without it, ARVES is limited to single-tenant or low-sensitivity deployments.

**Alternative designs.** (1) **Envelope encryption per shard** (per-tenant DEK wrapped by a KMS-held KEK; BYOK) — payloads encrypted at rest in the WAL/snapshots. (2) **Process/VM-per-tenant** for high-assurance tenants (Firecracker microVM per shard group). (3) **Confidential computing** (SEV-SNP/TDX) for memory isolation + attestation, complementing S1's tamper-evidence with hardware-rooted trust.

**Recommendation.** IDR-012 "Tenant Cryptographic Isolation": per-shard envelope encryption (BYOK) for WAL payloads and snapshots; document the shared-process residual risk and offer isolated-runtime deployment tiers. Runtime: encrypt `payload`/snapshot blob before `append`/`install_snapshot`. **Complexity:** High.

---

## S9 — No security conformance axis in the Scenario Framework (Medium)

**Where.** Scenario Conformance Framework Part 5 (12 axes). Axis 7 is "Safety-critical" and Axis 10 "Policy-heavy Governance," and properties include "tenant/workspace isolation" — but there is **no adversarial/security axis** and no scenario where an *attacker must fail*. Conformance is "structural, property-based, invariant-based, NOT golden-output" (Part 8) — which is exactly right for asserting negative security properties.

**Why it matters.** ARVES's whole certification philosophy is "judged by scenario results, not code inspection" (Part 13). If no scenario exercises an adversary, security is never certified. Certified Kubernetes (the stated precedent) has grown security conformance (CIS benchmarks, admission tests); ARVES needs the analogue.

**Recommendation.** Certification improvement: add security scenarios (no spec change — the framework is explicitly extensible and grows its assertion suite, Part 3). Concretely: (a) *Trace-Tamper* (mutate a committed record → recovery FAILS, ties to S1), (b) *Cross-Tenant-Commit* (commit into another tenant's shard → REJECTED, S2), (c) *Injection-Poisoning* (injected content cannot become high-trust Fact, S5), (d) *Rogue-Capability-Egress* (Pure capability blocked from network, S6), (e) *Unsigned-Manifest-Reject* (S7), (f) *Replay-Forgery* (a re-proposed outcome cannot fork or overwrite truth, S3). Verdict rule: any security-critical scenario failure = FAIL (matching Part 8's "isolation property failed = FAIL"). **Complexity:** Medium.

---

## S10 — Single-node recovery implicitly trusts local disk; no peer attestation (Medium)

**Where.** `RefKernel::try_recover`/`try_replay`: on I2 the comment says a node "will repair from a peer on such an error," but single-node recovery trusts whatever the disk holds. Combined with S1, a tampered-but-CRC-valid log is accepted.

**Why it matters.** Recovery is the moment truth is reconstituted; trusting unauthenticated local state means the tamper in S1 is realized at every restart. Even with a hash chain (S1), a single node cannot distinguish "my whole log was rewritten consistently" from "authentic" without an external anchor.

**Recommendation.** IDR (fold into IDR-006): on recovery verify the chain head against a **quorum-agreed / externally-anchored** head (in I2, majority of the shard's Raft group; in single-node, a signed checkpoint pinned off-host). Runtime: recovery compares recovered head hash to a durable, signed "expected head" marker. **Complexity:** Medium.

---

## S11 — No audit trail integrity distinct from the WAL (Medium)

**Where.** Vol 2 Part 20 / Vol 17 ("track who/what/when/why"). The runtime has no audit subsystem; the WAL records *outcomes*, not *actor/authorization decisions* (IDR-002: "committed OUTCOMES, not invocations").

**Why it matters.** "Who/what/when/why" (the four audit questions the spec repeats) are *not* in the outcome payload — the proposer's identity, the authorization that permitted the commit, the policy gate that fired, and the denied attempts are all absent. Denied/failed security events (the most important audit records) never reach the WAL at all because they never commit. An auditor cannot answer "who tried to write to tenant X and was denied?"

**Recommendation.** Runtime: an append-only, tamper-evident (reuse S1 chaining) **audit log** capturing principal, authorization decision, policy gates, and denials — separate from the truth WAL but with the same integrity primitive. Certification: an audit-completeness scenario. **Complexity:** Medium.

---

## S12 — `panic!` on unrecoverable state is a self-inflicted DoS surface (Low)

**Where.** `RefKernel::recover` → `panic!("unrecoverable durable state: {e}")`; `replay` → `panic!`; `main.rs` `process::exit(3)`. Many `.expect("… poisoned")` on mutexes.

**Why it matters.** "Lossless or loud" is the right *correctness* stance, but from a security standpoint an attacker who can induce a `Corruption`/`CompactedPrefixWithoutSnapshot` (e.g. by corrupting one interior segment — S1 shows disk write is in the threat model) achieves a **deterministic crash / refusal to start** = denial of service. A poisoned `Mutex` (via a panic in another thread) cascades to every subsequent `.expect`.

**Recommendation.** IDR/Runtime: keep "loud" but make it *recoverable* — the fallible `try_recover` path (already present) should be the only API; the panicking wrappers should be test-only. On corruption in a replicated deployment, quarantine the shard and repair from a peer (S10) rather than crashing the process (which may host many other tenants' shards). Avoid mutex-poison cascades (use a poison-tolerant lock or re-init). **Complexity:** Low.

---

## What would still be missing if ARVES were standardized by ISO/IEEE tomorrow

A normative, adversary-anchored **Security Architecture & Threat Model** companion (S4) with a control→mechanism→conformance mapping; **cryptographic tamper-evidence** for the decision trace that makes ORCH-003 defensible against an active adversary (S1); an **authenticated, authorized, tenant-scoped commit gateway** so "zero trust" and "100% tenant isolation" are enforced rather than declared (S2, S8); a **pinned content-address primitive** for cross-implementation integrity (S3); a **runtime evidence/trust adjudication** that operationalizes O-004 against prompt injection and poisoning (S5); a **capability/engine sandbox + signed supply chain** so a marketplace is safe (S6, S7); and **security scenarios in the conformance suite** so all of the above is *certified by result, not asserted by prose* (S9, S11). None of these require reopening the frozen corpus — they are IDRs, additive CCP amendments, runtime implementation, and conformance extensions. The frozen architecture (single gateway, per-shard append-only trace, mandatory provenance/trust aspects, content-addressing) is unusually well-shaped to receive them; the gap is that the security layer was named but never engineered.

// ARVES Assistant (JARVIS phase 1) — the ASSISTANT MEMORY CORE + SKILL/REASONER/GUARDRAIL layer.
//
// Stage 1 of the maintainer's product (docs/PRODUCT_BRIEF_JARVIS.md, acceptance A1 + A2):
// a personal assistant memory that observes the user's world from many sources, collapses
// the same real-world event into ONE content-addressed truth with an evidence set, records
// decisions as committed truth, detects contradictions with prior decisions — and SURVIVES
// RESTARTS: a new Assistant over the same `--wal-dir` proves its memory is still there.
//
// Stage 2 (A3 + A4 + A6) adds the governed think→act pipeline on the SAME frozen runtime:
//   think(goal) -> reasoner proposal (COMMITTED as truth)
//              -> guardrail policy gate (policies + approvals are committed truths;
//                 violations blocked AND committed as compliance events)
//              -> certified skill invocation (certification RE-RUN at registration;
//                 bind (RCR-016) -> invoke -> committed effect truth, the full chain).
// The reasoner slot ships ONLY a deterministic StubReasoner (NOT AI — see reasoner.mjs);
// the skill layer reuses the Ecosystem SDK trust boundary (see skills.mjs); the guardrails
// are the enterprise-os policy-as-truth pattern (see guardrails.mjs).
//
// Stage 3 (A5 + A7) adds sub-agents and explain-yourself on the SAME frozen runtime:
//   - agents.mjs: >=2 deterministic sub-agents over THIS one shared truth base, with
//     PRODUCT-LEVEL attribution (the agent tag is carried IN each committed truth body);
//   - why.mjs: why(assistant, truthIdOrSubject) reconstructs the decision path from
//     committed truths. Its feed is the DECISION JOURNAL below: every truth this
//     Assistant commits (or re-proves via already-committed) is journaled in commit
//     order with its id and body. HONEST MECHANISM: the journal is an in-process READ
//     PROJECTION of committed truth — rebuilt after a restart by re-running the same
//     deterministic day (every body answers already-committed, the membership proof);
//     a native WAL-scan verb over the bridge remains the recorded RCR candidate.
//
// PLATFORM BOUNDARY (IDR-006): this product CONSUMES the frozen ARVES Runtime v1.0
// exclusively through the published Runtime API — the TS SDK codec (ACS-001/002) and the
// Kernel Bridge line protocol (id= RCR-011 · shard= RCR-014 · --wal-dir RCR-015 ·
// bind RCR-016). It modifies no file under runtime/ or standard/.
//
// HONEST SCOPE (v1.0):
//  - No LLM lives here. The stage-2 reasoner slot ships a DETERMINISTIC STUB only; the
//    intelligence is the maintainer's LLM, implementing the Reasoner interface at their
//    test time, outside the repo (reasoner.mjs documents the exact contract).
//  - Single host, no authN on commit (v2.0 debt #8) — a personal assistant on the
//    maintainer's machine, stated, not hidden.
//  - The in-process indexes (#facts/#evidence/#decisions) are READ PROJECTIONS of
//    committed truth, rebuildable via `rebuild()` — they are never themselves the truth.
//
// THE REBUILD MECHANISM (A1 — closes P4's X1 caveat at the product level), stated loudly:
// the bridge line protocol has NO verb to enumerate or scan committed truth (no
// "replay"/"scan" — recorded as an RCR candidate in README.md). What the frozen Kernel
// DOES guarantee is idempotent, content-addressed commit: re-committing a byte-identical
// body answers `already-committed` with the SAME ContentId iff that exact body is already
// truth. `rebuild()` therefore re-derives candidate bodies deterministically (connectors
// re-read their sources; the caller re-supplies its decision journal), re-commits them,
// and treats the Kernel's `already-committed` answer as the MEMBERSHIP PROOF. The
// candidate list is an untrusted hint; only the Kernel's answer is evidence. Honest
// side effect: a candidate that was NOT previously committed becomes newly committed
// truth (there is no read-only membership probe over the line protocol) — the rebuild
// report separates `recovered` from `fresh` so nothing is silently smuggled in.

import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { Arves } from '../../arves-sdk-ts/src/arves.mjs';
import { certifyCapability } from '../../arves-ecosystem-sdk/src/kit.mjs';
import { Guardrails } from './guardrails.mjs';
import { validateReasoner } from './reasoner.mjs';

const arves = new Arves();

/** Canonicalize an observed fact into the abstract truth it attests. The SOURCE IS NOT
 *  PART OF IDENTITY — it is evidence — so the same real-world event seen by different
 *  systems is ONE truth (A2). `at` is BigInt milliseconds UTC; committed as exact ns. */
export function canonicalFact(fact) {
  if (fact === null || typeof fact !== 'object') throw new Error('assistant: fact must be an object');
  const { entity, event, at } = fact;
  if (typeof entity !== 'string' || entity.length === 0) throw new Error('assistant: fact.entity must be a non-empty string');
  if (typeof event !== 'string' || event.length === 0) throw new Error('assistant: fact.event must be a non-empty string');
  if (typeof at !== 'bigint' || at < 0n) throw new Error('assistant: fact.at must be a non-negative BigInt (ms since epoch)');
  return { type: 'uci.assistant.fact', entity, event, at: at * 1_000_000n }; // ms -> ns, exact
}

/** Canonical decision truth body. Decisions are committed truth, never just cache. */
export function canonicalDecision(subject, action, because) {
  for (const [n, v] of [['subject', subject], ['action', action], ['because', because]]) {
    if (typeof v !== 'string' || v.length === 0) throw new Error(`assistant: decision ${n} must be a non-empty string`);
  }
  return { type: 'uci.assistant.decision', subject, action, because };
}

/** Canonical source-attestation truth: "source X attests fact <id>". Committing these
 *  makes the EVIDENCE SET itself durable truth — after a restart, rebuild() proves each
 *  source attribution via already-committed, not by trusting the candidate list. */
export function canonicalAttestation(source, factId) {
  return { type: 'uci.assistant.attestation', source, of: factId };
}

/** BigInt-safe IMMUTABLE snapshot for the decision journal (A7): the body is deep-cloned
 *  (structuredClone preserves BigInt exactly) and then deep-frozen. So (a) a caller
 *  mutating its original object after commit cannot alter what was journaled, and (b) a
 *  consumer mutating a body returned by journal() gets a LOUD strict-mode TypeError
 *  instead of silently corrupting every later why() trace — the byte-identical-across-
 *  restart guarantee depends on journal bodies never changing after commit. */
function journalSnapshot(body) {
  const clone = structuredClone(body);
  const deepFreeze = (v) => {
    if (v !== null && typeof v === 'object') {
      Object.freeze(v);
      for (const k of Object.keys(v)) deepFreeze(v[k]);
    }
    return v;
  };
  return deepFreeze(clone);
}

export class Assistant {
  #bridge;
  #facts = new Map();      // ContentId -> canonical fact body (deduplicated truths)
  #evidence = new Map();   // fact ContentId -> Map(source -> attestation ContentId)
  #decisions = new Map();  // subject -> { id, subject, action, because } (latest per subject)
  #skills = new Map();     // name -> { cap, checks, registrationId } (certified + bound skills)
  #reasoner = null;        // the pluggable Reasoner slot (A4) — nothing attached by default
  #guardrails;             // policy-as-truth gate (A6), consulted BEFORE any skill invocation
  #journal = [];           // A7 feed: [{ seq, id, status, domain, body, meta }] in commit order
  #seq = 0;                // journal sequence (process-local ordering, NOT part of any truth body)

  /** Owns its KernelBridge. `{ walDir }` -> durable file-backed Kernel (RCR-015): a later
   *  Assistant over the SAME walDir sees the same ContentIds as already-committed.
   *  `{ tenant, workspace }` -> shard scoping (RCR-014). Absent walDir -> in-memory Kernel. */
  constructor({ tenant, workspace, walDir, exe, timeoutMs } = {}) {
    this.#bridge = new KernelBridge(exe, { tenant, workspace, walDir, timeoutMs });
    this.#guardrails = new Guardrails((body, domain) => this.#commit(body, domain, { via: 'guardrails' }));
  }

  /** EVERY commit flows through here so the decision journal (A7) misses nothing.
   *  The journal records what THIS process committed/re-proved; it is a projection,
   *  never the truth (the Kernel's WAL is). */
  async #commit(body, domain, meta = { via: 'commit' }) {
    const res = await this.#bridge.commit(body, domain);
    this.#journal.push({ seq: this.#seq++, id: res.contentId, status: res.status, domain, body: journalSnapshot(body), meta });
    return res;
  }

  /** The DECISION JOURNAL (A7 feed, consumed by why.mjs): every truth this Assistant
   *  committed (or re-proved as already-committed), in commit order, with its ContentId
   *  and canonical body. HONEST: an in-process read projection — a fresh process has an
   *  empty journal until the deterministic day is re-run (rebuild + re-register +
   *  re-think; every body then answers already-committed). A native WAL-scan verb over
   *  the bridge line protocol is the recorded RCR candidate (README.md).
   *  Returned `body` objects are IMMUTABLE snapshots (deep-frozen at commit time):
   *  mutating one throws in strict mode instead of corrupting later why() traces. */
  journal() { return this.#journal.map((e) => ({ ...e, meta: { ...e.meta } })); }

  /** Observe one slice of the user's world: commit the canonical fact as truth in the
   *  real Kernel, commit the source attestation, and fold both into the projection.
   *  Same event from another source -> deduped to the SAME ContentId, source added to
   *  the evidence set (A2). Verifies one-world identity (SDK-local id == Kernel id). */
  async observe(source, fact) {
    if (typeof source !== 'string' || source.length === 0) throw new Error('assistant: source must be a non-empty string');
    const body = canonicalFact(fact);
    const localId = arves.address(body, 'commit');
    const res = await this.#commit(body, 'commit', { via: 'observe', source }); // truth -> real Kernel
    if (res.contentId !== localId) throw new Error('one-world violation: SDK id != Kernel id');
    // Evidence is truth too: one attestation truth per (source, fact) pair, idempotent.
    const att = await this.#commit(canonicalAttestation(source, localId), 'trace', { via: 'observe', source });
    const isNew = !this.#facts.has(localId);
    if (isNew) { this.#facts.set(localId, body); this.#evidence.set(localId, new Map()); }
    this.#evidence.get(localId).set(source, att.contentId);
    return {
      id: localId,
      status: res.status,                 // 'committed' | 'already-committed' (Kernel's answer)
      attestationId: att.contentId,
      attestationStatus: att.status,
      deduped: !isNew,
      sources: this.#sources(localId),
    };
  }

  #sources(id) { return [...this.#evidence.get(id).keys()].sort(); }

  /** The memory: deduplicated truths, each with its attesting sources. Stable order. */
  truths() {
    return [...this.#facts.entries()]
      .map(([id, fact]) => ({ id, fact, sources: this.#sources(id) }))
      .sort((a, b) => (a.id < b.id ? -1 : 1));
  }

  /** Recall memory, optionally filtered by entity. */
  recall(entity) {
    const all = this.truths();
    return entity === undefined ? all : all.filter((t) => t.fact.entity === entity);
  }

  /** Record a decision as a committed, content-addressed truth (domain 'trace') and
   *  index it for contradiction detection. Latest decision per subject wins. */
  async recordDecision(subject, action, because) {
    const body = canonicalDecision(subject, action, because);
    const res = await this.#commit(body, 'trace', { via: 'decision' });
    this.#decisions.set(subject, { id: res.contentId, subject, action, because });
    return { id: res.contentId, status: res.status };
  }

  /** All indexed decisions (projection of committed decision truths). */
  decisions() {
    return [...this.#decisions.values()].sort((a, b) => (a.subject < b.subject ? -1 : 1));
  }

  /** Does a candidate decision contradict a prior COMMITTED decision (same subject,
   *  different action)? Returns the prior decision's truth id as the PROOF.
   *  An ambiguous candidate (missing/empty subject or action) is rejected loudly —
   *  the same validation bar as canonicalDecision(), never a silent `contradicts: false`. */
  checkContradiction(candidate) {
    if (candidate === null || typeof candidate !== 'object') throw new Error('assistant: candidate must be an object');
    for (const [n, v] of [['subject', candidate.subject], ['action', candidate.action]]) {
      if (typeof v !== 'string' || v.length === 0) throw new Error(`assistant: candidate.${n} must be a non-empty string`);
    }
    const prior = this.#decisions.get(candidate.subject);
    if (prior && prior.action !== candidate.action) {
      return {
        contradicts: true,
        priorId: prior.id,
        prior: { subject: prior.subject, action: prior.action, because: prior.because },
      };
    }
    return { contradicts: false };
  }

  /** A1 — rebuild the memory/decision/contradiction index from COMMITTED TRUTH after a
   *  restart, via the idempotent re-commit membership proof documented in the header.
   *  `observations`: [{ source, fact }] re-derived deterministically (e.g. connectors);
   *  `decisions`:    [{ subject, action, because }] re-supplied from the caller's journal.
   *  Every candidate is re-committed; `already-committed` = RECOVERED (it provably was
   *  truth before this process existed); `committed` = FRESH (it was not — now it is,
   *  honestly reported, never silently). Returns the proof-grade report. */
  async rebuild({ observations = [], decisions = [] } = {}) {
    const report = {
      factsRecovered: 0, factsFresh: 0,
      attestationsRecovered: 0, attestationsFresh: 0,
      decisionsRecovered: 0, decisionsFresh: 0,
      recoveredIds: [], freshIds: [],
    };
    const tally = (status, id, recKey, freshKey) => {
      if (status === 'already-committed') { report[recKey]++; report.recoveredIds.push(id); }
      else { report[freshKey]++; report.freshIds.push(id); }
    };
    for (const { source, fact } of observations) {
      const r = await this.observe(source, fact);
      tally(r.status, r.id, 'factsRecovered', 'factsFresh');
      tally(r.attestationStatus, r.attestationId, 'attestationsRecovered', 'attestationsFresh');
    }
    for (const d of decisions) {
      const r = await this.recordDecision(d.subject, d.action, d.because);
      tally(r.status, r.id, 'decisionsRecovered', 'decisionsFresh');
    }
    report.recoveredIds.sort();
    report.freshIds.sort();
    return report;
  }

  // ---- Stage 2: platform pass-throughs (the bridge stays owned and private) ----------

  /** Commit an arbitrary canonical body as truth over the owned bridge. This is the
   *  surface the skill/guardrail layers use; callers stay responsible for canonical,
   *  deterministic bodies (BigInt ints, no clocks/randomness). */
  async commitTruth(body, domain = 'trace') { return this.#commit(body, domain, { via: 'commit' }); }

  /** RCR-016: dynamically bind a capability name in this assistant's shard. */
  async bindName(capability) { return this.#bridge.bind(capability); }

  /** Run a value through the FULL runtime chain under a bound capability name:
   *  Capability (resolve) -> Engine (fabric-enforced) -> Kernel (commit truth).
   *  An unbound name is refused by the runtime (`ERR unbound`) — no implicit bind. */
  async invokeBound(value, capability, domain = 'commit') { return this.#bridge.invoke(value, capability, domain); }

  // ---- Stage 2: skills (A3) -----------------------------------------------------------

  /** Attach a skill entry to the index — the LAST gate before invokeSkill() will run its
   *  code. DEFENSE IN DEPTH: this method does NOT trust its caller. It RE-RUNS
   *  certification itself over `entry.testInputs` (caller-supplied checks/flags are
   *  IGNORED — fresh check results are stored), and it demands the registrationId of the
   *  committed admission truth. So even in-process code that bypasses registerSkill()
   *  (skills.mjs — the full path, which also commits the admission truth and binds the
   *  name, RCR-016) cannot attach uncertified code here.
   *  HONEST RESIDUAL (v1.0 trusted single host, no authN): the registrationId is
   *  shape-checked, not re-verified against the ledger; and a caller that skips
   *  bindName() is refused by the RUNTIME at invoke time (`ERR unbound`). */
  attachSkill(name, entry) {
    if (typeof name !== 'string' || name.length === 0) throw new Error('assistant: skill name must be a non-empty string');
    if (entry === null || typeof entry !== 'object' || entry.cap === null || typeof entry.cap !== 'object'
        || !entry.cap.manifest || typeof entry.cap.execute !== 'function') {
      throw new Error('assistant: attachSkill requires a capability entry — author it with defineCapability() and register via registerSkill() (skills.mjs)');
    }
    if (typeof entry.registrationId !== 'string' || !/^[0-9a-f]{68}$/.test(entry.registrationId)) {
      throw new Error('assistant: attachSkill requires the registrationId of the committed admission truth — registerSkill() commits it');
    }
    if (!Array.isArray(entry.testInputs) || entry.testInputs.length === 0) {
      throw new Error('assistant: attachSkill requires the certification testInputs — the gate is RE-RUN here, never trusted');
    }
    const cert = certifyCapability(entry.cap, entry.testInputs); // re-run; caller's checks ignored
    if (!cert.certified) {
      const failedChecks = cert.checks.filter((c) => !c.ok).map((c) => c.name).join(', ');
      throw new Error(`assistant: attachSkill refused '${name}' — fails certification (${failedChecks}); bypassing registerSkill() does not bypass the gate`);
    }
    this.#skills.set(name, { cap: entry.cap, checks: cert.checks, registrationId: entry.registrationId });
  }

  /** Names of the registered (certified + bound) skills, sorted. */
  skills() { return [...this.#skills.keys()].sort(); }

  hasSkill(name) { return this.#skills.has(name); }

  /** Invoke a REGISTERED skill: its execute() runs product-side (honest v1.0 model —
   *  the runtime does not load product code); each proposed effect value is then carried
   *  through the full chain under the bound name and committed as ACS-addressed truth. */
  async invokeSkill(name, input) {
    const entry = this.#skills.get(name);
    if (!entry) throw new Error(`assistant: skill '${name}' is not registered — registerSkill() first (certification + bind are the gate)`);
    const effects = entry.cap.execute(input);
    const truths = [];
    for (const eff of effects) {
      const res = await this.#bridge.invoke(eff.value, name, 'commit'); // bind -> invoke -> committed effect truth
      // Effects are journaled too (A7): meta.capability lets why() walk effect -> proposal.
      this.#journal.push({
        seq: this.#seq++, id: res.contentId, status: res.status, domain: 'commit', body: journalSnapshot(eff.value),
        meta: { via: 'invoke', capability: name, target: eff.target, registrationId: entry.registrationId },
      });
      truths.push({ target: eff.target, id: res.contentId, status: res.status });
    }
    return { skill: name, registrationId: entry.registrationId, truths };
  }

  // ---- Stage 2: reasoner slot (A4) + guardrails (A6) -----------------------------------

  /** Plug a Reasoner (validated against the interface contract in reasoner.mjs). The
   *  repo ships only the deterministic StubReasoner; the maintainer plugs an LLM-backed
   *  implementation OUTSIDE the repo. */
  useReasoner(reasoner) { this.#reasoner = validateReasoner(reasoner); return this; }

  get reasoner() { return this.#reasoner; }

  /** The policy-as-truth gate: setPolicy / approve / check / enforce (guardrails.mjs). */
  get guardrails() { return this.#guardrails; }

  /** The governed think→act pipeline (A3+A4+A6, in one line of causality):
   *    reasoner proposal (committed as truth) -> guardrail gate -> certified skill
   *    invocation -> committed effect truth.
   *  Whatever the reasoner proposes, it CANNOT bypass the gate or run uncertified code:
   *  policies are consulted BEFORE any skill invocation, and only a registered
   *  (certified + bound) skill can act. Deterministic with the stub reasoner: the same
   *  goal over the same state replays to the same committed truths (already-committed). */
  async think(goal) {
    if (typeof goal !== 'string' || goal.length === 0) throw new Error('assistant: goal must be a non-empty string');
    if (this.#reasoner === null) {
      throw new Error('assistant: no reasoner attached — useReasoner(new StubReasoner()) for the deterministic stub, or plug your own Reasoner (contract: src/reasoner.mjs)');
    }
    const context = { goal, truths: this.truths(), decisions: this.decisions(), skills: this.skills() };
    const proposal = await this.#reasoner.reason(context);
    if (proposal === null || typeof proposal !== 'object' || typeof proposal.action !== 'string') {
      throw new Error('assistant: reasoner returned a malformed proposal (see the contract in reasoner.mjs)');
    }

    // The proposal ITSELF becomes committed, attributed truth — the audit trail records
    // what was proposed (and by which reasoner) even when nothing ends up acting.
    const trace = {
      type: 'uci.assistant.proposal',
      reasoner: this.#reasoner.name,
      reasonerVersion: this.#reasoner.version,
      goal,
      action: proposal.action,
      because: String(proposal.because ?? ''),
      ...(proposal.action === 'invoke-skill'
        ? { skill: proposal.skill, subject: proposal.subject, actionClass: proposal.actionClass, input: proposal.input }
        : {}),
    };
    const prop = await this.#commit(trace, 'trace', { via: 'think' });

    if (proposal.action !== 'invoke-skill') {
      return { acted: false, reason: 'no-action-proposed', proposal, proposalId: prop.contentId };
    }
    if (!this.#skills.has(proposal.skill)) {
      // The refusal ITSELF is committed truth (mirror of Guardrails.enforce): the ledger
      // records not only what was proposed but that it was REFUSED for naming a skill
      // that is not certified+bound. Deterministic body (content-addressed proposalId).
      const refusal = {
        type: 'uci.assistant.compliance',
        outcome: 'refused-unregistered-skill',
        skill: typeof proposal.skill === 'string' ? proposal.skill : '',
        goal,
        proposalId: prop.contentId,
      };
      const ref = await this.#commit(refusal, 'trace', { via: 'think' });
      throw new Error(`assistant: reasoner proposed unregistered skill '${proposal.skill}' — only certified+bound skills can act (refusal committed: ${ref.contentId})`);
    }

    // Guardrails BEFORE any skill invocation: a violation is blocked AND committed
    // as a compliance-event truth (never a silent drop).
    const gate = await this.#guardrails.enforce({ ...proposal, goal });
    if (!gate.ok) {
      return {
        acted: false,
        blocked: true,
        proposal,
        proposalId: prop.contentId,
        policyId: gate.policyId,
        policy: gate.policy,
        rule: gate.rule,
        complianceId: gate.complianceId,
      };
    }

    const invocation = await this.invokeSkill(proposal.skill, proposal.input);
    return { acted: true, proposal, proposalId: prop.contentId, approvals: gate.approvals, invocation };
  }

  /** Close the owned bridge. ALWAYS call in finally. (On Windows, give the bridge
   *  process a moment to exit before deleting its walDir — see the examples.) */
  close() { this.#bridge.close(); }
}

// ARVES Enterprise Cognitive OS (P5) — an operating system for an organization's
// cognition, built entirely on the FROZEN ARVES Runtime v1.0 (SDK + Bridge → real
// Kernel). Same runtime as P4; a structurally different product — the second proof that
// v1.0 carries products unchanged.
//
// It proves the enterprise capability set that a ChatGPT/LangGraph/AutoGen/CrewAI wrapper
// cannot: MULTI-AGENT shared truth (departments reason over ONE content-addressed truth
// base, not divergent copies), POLICY enforced as truth (a decision is checked against
// committed policy truths, violations blocked + audited), a tamper-evident COMPLIANCE
// ledger, and cross-department CONSISTENCY (a conflicting decision is detected because all
// decisions are addressable truths). Every fact, policy, decision, and compliance event is
// committed as truth through the bridge to the WAL-backed Rust reference Kernel — auditable
// and replayable.
//
// SCOPE CAVEAT (honest): the durable persistence/replay/recovery claim holds ONLY along the
// `#bridge` path, and ONLY when the `arves-bridge` binary is built and running (QUICKSTART
// step 1) — otherwise `#bridge.commit(...)` rejects and nothing is committed. The policy
// engine, dedup, evidence index, and cross-department conflict check run over IN-MEMORY
// per-process maps (`#facts`/`#evidence`/`#policies`/`#decisions`) built via `class Arves` (a
// JS `Map`, no WAL/recovery); that state is NOT read back from the Kernel and is lost on
// process exit. The compliance "ledger" is tamper-evident only for as long as the process
// holds it — durable, cross-process audit lives in the Kernel WAL, reached via the bridge.

import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();

export class EnterpriseCognitiveOS {
  #bridge;
  #facts = new Map();      // ContentId → org fact
  #evidence = new Map();   // ContentId → Set(source system)
  #policies = [];          // [{ id, policy }] committed policy truths
  #decisions = new Map();  // subject → { id, decision } committed decision truths
  #approvals = new Map();  // subject → Map(role → approvalId) — SEPARATE committed approval truths (E1)

  constructor(bridge) { this.#bridge = bridge; }

  /** Coerce a USD amount to a canonical BigInt integer, or throw a CLEAR product-level error.
   *  (E2 hardening) A bare JS `Number` used to reach the ACS dCBOR encoder and crash deep inside
   *  it with an opaque `BigInt` error; here an integer-valued Number/string is accepted (coerced)
   *  and any non-integer (float / NaN / junk) is rejected EARLY, before any commit, naming the field. */
  #toIntUsd(v, field) {
    if (typeof v === 'bigint') return v;
    if (typeof v === 'number') {
      if (!Number.isInteger(v)) throw new Error(`${field} must be an integer USD amount (got non-integer Number ${v}); pass a BigInt`);
      return BigInt(v);
    }
    if (typeof v === 'string' && /^-?\d+$/.test(v)) return BigInt(v);
    throw new Error(`${field} must be a BigInt integer USD amount (got ${typeof v})`);
  }

  /** Observe org reality (ERP/CRM/HR/tickets) into ONE shared, content-addressed truth
   *  base that every department agent reasons over. */
  async observe(obs) {
    const fact = { type: 'uci.fact', entity: obs.fact.entity, event: obs.fact.event, at: BigInt(obs.fact.at) * 1_000_000n };
    const id = arves.address(fact, 'commit');
    const res = await this.#bridge.commit(fact, 'commit');
    if (res.contentId !== id) throw new Error('one-world violation');
    if (!this.#facts.has(id)) { this.#facts.set(id, fact); this.#evidence.set(id, new Set()); }
    this.#evidence.get(id).add(obs.source);
    return { id, deduped: this.#evidence.get(id).size > 1 };
  }

  truths() {
    return [...this.#facts.entries()].map(([id, fact]) => ({ id, fact, sources: [...this.#evidence.get(id)].sort() }));
  }

  /** Publish a governance policy as truth (addressable, auditable). */
  async setPolicy(policy) {
    const p = { type: 'uci.policy', domain: policy.domain, rule: policy.rule, thresholdUsd: this.#toIntUsd(policy.thresholdUsd ?? 0n, 'thresholdUsd') };
    const res = await this.#bridge.commit(p, 'trace');
    this.#policies.push({ id: res.contentId, policy: { ...policy, thresholdUsd: p.thresholdUsd } });
    return res.contentId;
  }

  /** A role actor (e.g. legal) commits a SEPARATE approval truth for a subject (E1 fix). This is
   *  the addressable, auditable "legal approved spend:x" event — a distinct truth in the ledger,
   *  NOT a string the finance proposer buries in its own decision. `checkPolicy` consults these
   *  committed approval truths, so the proposer can no longer self-clear its own gate.
   *  RESIDUAL (honest): Runtime v1.0 has no authN on `commit` (RUNTIME_FREEZE v2.0 debt #8), so the
   *  `role` tag here is NOT cryptographically bound to a real legal identity — any caller can invoke
   *  `approve({role:'legal'})`. The improvement is structural (separation of proposer from approver +
   *  a separate addressable approval truth); a cryptographically authenticated approval identity is
   *  the v2.0 authenticated-commit RCR, tracked as OPEN_DEBT E1 → runtime #8. */
  async approve({ role, subject, by } = {}) {
    if (!role || !subject) throw new Error('approve requires { role, subject }');
    const a = { type: 'uci.approval', role: String(role), subject: String(subject), by: String(by ?? role) };
    const res = await this.#bridge.commit(a, 'trace');
    if (!this.#approvals.has(subject)) this.#approvals.set(subject, new Map());
    this.#approvals.get(subject).set(String(role), res.contentId);
    return res.contentId;
  }

  /** Check a candidate decision against the committed policy truths. Deterministic.
   *  E1 (fixed): the required `legal` approval is now read from the SEPARATE committed approval
   *  truths (`#approvals`, populated by `approve()`), NOT from a proposer-supplied `d.approvals`
   *  array — a finance agent can no longer self-attest its own approval. See `approve()` for the
   *  residual v2.0 authN gap. The subject match is an exact `spend:` prefix — a renamed subject is
   *  out of policy scope (tracked; a renamed-subject gap is a CCP-class policy-language decision). */
  checkPolicy(d) {
    for (const { id, policy } of this.#policies) {
      if (policy.domain === 'spend' && String(d.subject).startsWith('spend:')) {
        const amount = this.#toIntUsd(d.amountUsd ?? 0n, 'amountUsd');
        const approved = this.#approvals.get(d.subject); // SEPARATE committed approval truths (E1)
        const hasLegal = !!approved && approved.has('legal');
        if (d.action === 'approve' && amount > (policy.thresholdUsd ?? 0n) && !hasLegal) {
          return { ok: false, policyId: id, rule: policy.rule };
        }
      }
    }
    return { ok: true };
  }

  /** A department agent proposes a decision. It is only committed as an approved-decision truth
   *  if it (a) passes every policy — where "legal approved" now means a SEPARATE committed approval
   *  truth exists (E1), not a proposer-supplied array — and (b) does not conflict with a prior
   *  committed decision on the same subject. A violation or conflict is recorded as a
   *  compliance-event truth instead — a real, replayable audit trail of what was proposed. */
  async proposeDecision(decision) {
    const amountUsd = this.#toIntUsd(decision.amountUsd ?? 0n, 'amountUsd'); // E2: fail early & clean
    const check = this.checkPolicy(decision);
    if (!check.ok) {
      const ev = { type: 'uci.compliance', outcome: 'blocked', subject: decision.subject, rule: check.rule, policy: check.policyId };
      const res = await this.#bridge.commit(ev, 'trace');
      return { committed: false, reason: check.rule, policy: check.policyId, complianceEvent: res.contentId };
    }
    const prior = this.#decisions.get(decision.subject);
    if (prior && prior.decision.action !== decision.action) {
      const ev = { type: 'uci.compliance', outcome: 'conflict', subject: decision.subject, prior: prior.id, priorAction: prior.decision.action, newAction: decision.action };
      const res = await this.#bridge.commit(ev, 'trace');
      return { committed: false, reason: 'cross-department-conflict', prior: prior.id, complianceEvent: res.contentId };
    }
    const approvedBy = [...(this.#approvals.get(decision.subject)?.values() ?? [])].sort();
    const step = {
      type: 'uci.decision',
      agent: decision.agent,
      subject: decision.subject,
      action: decision.action,
      amountUsd,
      approvedBy, // ContentIds of the SEPARATE committed approval truths that authorized this (E1)
    };
    const res = await this.#bridge.commit(step, 'trace');
    this.#decisions.set(decision.subject, { id: res.contentId, decision });
    return { committed: true, id: res.contentId };
  }
}

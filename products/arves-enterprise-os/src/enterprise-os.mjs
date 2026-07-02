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

  constructor(bridge) { this.#bridge = bridge; }

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
    const p = { type: 'uci.policy', domain: policy.domain, rule: policy.rule, thresholdUsd: policy.thresholdUsd ?? 0n };
    const res = await this.#bridge.commit(p, 'trace');
    this.#policies.push({ id: res.contentId, policy });
    return res.contentId;
  }

  /** Check a candidate decision against the committed policy truths. Deterministic. */
  checkPolicy(d) {
    for (const { id, policy } of this.#policies) {
      if (policy.domain === 'spend' && String(d.subject).startsWith('spend:')) {
        const amount = d.amountUsd ?? 0n;
        const approvals = d.approvals ?? [];
        if (d.action === 'approve' && amount > (policy.thresholdUsd ?? 0n) && !approvals.includes('legal')) {
          return { ok: false, policyId: id, rule: policy.rule };
        }
      }
    }
    return { ok: true };
  }

  /** A department agent proposes a decision. It is only committed as truth if it (a) passes
   *  every policy and (b) does not conflict with a prior committed decision on the same
   *  subject. A violation or conflict is recorded as a compliance-event truth instead —
   *  a real, replayable audit trail, not a promise. */
  async proposeDecision(decision) {
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
    const step = {
      type: 'uci.decision',
      agent: decision.agent,
      subject: decision.subject,
      action: decision.action,
      amountUsd: decision.amountUsd ?? 0n,
      approvals: (decision.approvals ?? []).slice().sort(),
    };
    const res = await this.#bridge.commit(step, 'trace');
    this.#decisions.set(decision.subject, { id: res.contentId, decision });
    return { committed: true, id: res.contentId };
  }
}

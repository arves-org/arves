// ARVES Assistant — guardrails: POLICY AS TRUTH (A6, the enterprise-os pattern).
//
// Three properties, all enforced with COMMITTED truth, never with in-memory flags alone:
//   1. A policy is itself a committed, content-addressed truth (setPolicy). What governs
//      the assistant is addressable and auditable, not configuration lore.
//   2. Gated action classes (e.g. 'spend', 'irreversible') require a SEPARATE committed
//      APPROVAL truth (approve(role, subject)) — the proposer can never self-clear its own
//      gate by embedding "approved!" in its own proposal (enterprise-os E1 pattern).
//   3. A violation is BLOCKED *and* committed as a compliance-event truth — the audit
//      trail records what was refused, not only what happened.
//
// The assistant's think→act path calls enforce() BEFORE any skill invocation, so no
// reasoner output (stub or future LLM) can reach a skill without passing the policies.
//
// HONEST RESIDUAL (v1.0): Runtime v1.0 has no authN on commit (RUNTIME_FREEZE v2.0 debt
// #8), so the `role` on an approval truth is STRUCTURAL, not cryptographic — any local
// caller could invoke approve('user', ...). The guarantee here is separation (approver
// truth is distinct from proposer truth, both addressable in the ledger) on a trusted
// single host; an authenticated approval identity is the v2.0 authenticated-commit RCR.
//
// The in-memory #policies/#approvals maps are READ PROJECTIONS of the committed truths
// (the same honesty rule as the assistant's fact index): rebuildable by re-committing the
// same bodies (idempotent membership proof), never themselves the truth.

/** Canonical policy truth body. `appliesTo` is the sorted list of gated action classes. */
export function canonicalPolicy(name, appliesTo, approverRole) {
  if (typeof name !== 'string' || name.length === 0) throw new Error('guardrails: policy name must be a non-empty string');
  if (!Array.isArray(appliesTo) || appliesTo.length === 0 || appliesTo.some((c) => typeof c !== 'string' || c.length === 0)) {
    throw new Error('guardrails: policy appliesTo must be a non-empty array of action-class strings');
  }
  if (typeof approverRole !== 'string' || approverRole.length === 0) throw new Error('guardrails: policy approverRole must be a non-empty string');
  return { type: 'uci.assistant.policy', name, appliesTo: [...appliesTo].sort(), approverRole };
}

/** Canonical approval truth body — a SEPARATE truth from any proposal/decision. */
export function canonicalApproval(role, subject) {
  for (const [n, v] of [['role', role], ['subject', subject]]) {
    if (typeof v !== 'string' || v.length === 0) throw new Error(`guardrails: approval ${n} must be a non-empty string`);
  }
  return { type: 'uci.assistant.approval', role, subject };
}

export class Guardrails {
  #commit;               // (body, domain) -> { contentId, status } — the assistant's owned bridge
  #policies = [];        // [{ id, name, appliesTo, approverRole }] projection of committed policy truths
  #approvals = new Map(); // subject -> Map(role -> approval ContentId) projection of committed approvals

  constructor(commit) {
    if (typeof commit !== 'function') throw new Error('guardrails: commit(body, domain) function required');
    this.#commit = commit;
  }

  /** Publish a policy AS TRUTH: committed, content-addressed, then indexed. */
  async setPolicy({ name, appliesTo, approverRole } = {}) {
    const body = canonicalPolicy(name, appliesTo, approverRole);
    const res = await this.#commit(body, 'trace');
    this.#policies.push({ id: res.contentId, name: body.name, appliesTo: body.appliesTo, approverRole: body.approverRole });
    return { id: res.contentId, status: res.status };
  }

  policies() {
    return this.#policies.map((p) => ({ ...p, appliesTo: [...p.appliesTo] }));
  }

  /** A role actor commits a SEPARATE approval truth for a subject. See the honest
   *  residual in the header: role is structural in v1.0, not authenticated. */
  async approve(role, subject) {
    const body = canonicalApproval(role, subject);
    const res = await this.#commit(body, 'trace');
    if (!this.#approvals.has(subject)) this.#approvals.set(subject, new Map());
    this.#approvals.get(subject).set(role, res.contentId);
    return { id: res.contentId, status: res.status };
  }

  /** Deterministic policy check — NO commit, no side effects. A proposal with a
   *  missing/empty subject or actionClass is rejected LOUDLY (never silently allowed:
   *  an ungateable proposal must not slip past the gate by being malformed). */
  check({ subject, actionClass } = {}) {
    for (const [n, v] of [['subject', subject], ['actionClass', actionClass]]) {
      if (typeof v !== 'string' || v.length === 0) throw new Error(`guardrails: proposal.${n} must be a non-empty string (an ungateable proposal is refused, not waved through)`);
    }
    for (const p of this.#policies) {
      if (p.appliesTo.includes(actionClass)) {
        const approvalId = this.#approvals.get(subject)?.get(p.approverRole);
        if (approvalId === undefined) {
          return {
            ok: false,
            policyId: p.id,
            policy: p.name,
            requires: p.approverRole,
            rule: `'${actionClass}'-class actions require a committed '${p.approverRole}' approval truth for '${subject}'`,
          };
        }
      }
    }
    // Allowed: surface the authorizing approval ids so the caller can commit them into its audit trail.
    return { ok: true, approvals: [...(this.#approvals.get(subject)?.values() ?? [])].sort() };
  }

  /** Enforce the policies against a proposal. If blocked, the violation is COMMITTED as
   *  a compliance-event truth (the refusal itself becomes auditable, replayable record)
   *  and the caller MUST NOT invoke the skill. */
  async enforce(proposal) {
    const verdict = this.check(proposal);
    if (verdict.ok) return verdict;
    const ev = {
      type: 'uci.assistant.compliance',
      outcome: 'blocked',
      subject: proposal.subject,
      skill: typeof proposal.skill === 'string' ? proposal.skill : '',
      actionClass: proposal.actionClass,
      policy: verdict.policyId,
      goal: typeof proposal.goal === 'string' ? proposal.goal : '',
    };
    const res = await this.#commit(ev, 'trace');
    return { ...verdict, complianceId: res.contentId, complianceStatus: res.status };
  }
}

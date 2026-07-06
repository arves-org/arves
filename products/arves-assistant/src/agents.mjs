// ARVES Assistant — SUB-AGENTS over ONE shared truth base (A5).
//
// ============================================================================
//  HONESTY FIRST: these agents are DETERMINISTIC ACTORS, not AI.
// ============================================================================
// ResearcherAgent and SchedulerAgent are pure, rule-based functions of the shared
// truth base — same truths in, same committed truths out, forever. They exist so the
// multi-agent orchestration pattern (shared truth, attribution, deterministic conflict
// resolution) is testable OFFLINE. The intelligence arrives when the maintainer plugs
// their LLM into the Reasoner slot (A4) — these agents stay the governed scaffolding.
//
// ATTRIBUTION — PRODUCT-LEVEL, STATED LOUDLY: every truth an agent contributes carries
// the agent identity tag IN THE COMMITTED BODY (`agent` + `agentVersion` fields), so
// attribution is content-addressed and survives restarts like any other truth. The
// runtime's Rust-level I5 attribution (agent identity as committed truth, scheduler-
// borne attributed proposals — `arves-control-plane`, RCR-029..031) is NOT exposed over
// the bridge line protocol. Until an RCR adds an attributed-commit verb, THIS is the
// honest form (recorded RCR candidate in README.md) — we never fake runtime access we
// do not have. Corollary of v1.0's no-authN scope: the tag is STRUCTURAL, not
// cryptographic — any local caller could wear any tag (v2.0 authenticated-commit debt).
//
// CONFLICT RESOLUTION — FIRST-COMMITTED-WINS, deterministically: when two agents
// propose DIFFERENT actions on ONE subject, the proposal committed first wins (the
// I5/RCR-030 runtime semantics, applied at product level). The losing proposal is
// STILL committed truth (the ledger records what was proposed, not only what won), and
// the loser records a resolution truth REFERENCING the winner — the concession itself
// is auditable, content-addressed record. Determinism holds because the day's agent
// script runs in a fixed order; replaying the same script re-commits the same bodies
// (already-committed all the way down).
//
// The AgentCouncil's #plans map is a READ PROJECTION of the committed proposal truths
// (same honesty rule as every other index in this product): rebuilt after a restart by
// re-running the same deterministic agent script, never itself the truth.

/** Canonical agent-tagged proposal truth: WHO proposes WHAT about a subject.
 *  The agent tag is part of the committed body — attribution is truth, not metadata. */
export function canonicalAgentProposal(agent, agentVersion, subject, action, because) {
  for (const [n, v] of [['agent', agent], ['agentVersion', agentVersion],
    ['subject', subject], ['action', action], ['because', because]]) {
    if (typeof v !== 'string' || v.length === 0) throw new Error(`agents: proposal ${n} must be a non-empty string`);
  }
  return { type: 'uci.assistant.agent-proposal', agent, agentVersion, subject, action, because };
}

/** Canonical research-finding truth: an agent gathered facts INTO truth. `facts` are
 *  the ContentIds of the supporting fact truths (sorted -> order-independent body). */
export function canonicalFinding(agent, agentVersion, topic, factIds) {
  for (const [n, v] of [['agent', agent], ['agentVersion', agentVersion], ['topic', topic]]) {
    if (typeof v !== 'string' || v.length === 0) throw new Error(`agents: finding ${n} must be a non-empty string`);
  }
  if (!Array.isArray(factIds) || factIds.some((f) => !/^[0-9a-f]{68}$/.test(f))) {
    throw new Error('agents: finding factIds must be an array of ContentIds');
  }
  return { type: 'uci.assistant.finding', agent, agentVersion, topic, facts: [...factIds].sort() };
}

/** Canonical conflict-resolution truth: the loser's committed reference to the winner.
 *  first-committed-wins is the rule — deterministic, stated in the body itself. */
export function canonicalResolution(subject, winnerId, loserId) {
  if (typeof subject !== 'string' || subject.length === 0) throw new Error('agents: resolution subject must be a non-empty string');
  for (const [n, v] of [['winnerId', winnerId], ['loserId', loserId]]) {
    if (!/^[0-9a-f]{68}$/.test(v)) throw new Error(`agents: resolution ${n} must be a ContentId`);
  }
  return { type: 'uci.assistant.resolution', rule: 'first-committed-wins', subject, winner: winnerId, loser: loserId };
}

/** The coordination surface: agents propose plan items about subjects over ONE shared
 *  truth base (the assistant). Every proposal is committed truth; conflicts resolve
 *  first-committed-wins; the losing agent records a resolution truth citing the winner. */
export class AgentCouncil {
  #assistant;
  #plans = new Map(); // subject -> { id, agent, agentVersion, action, because } — WINNER projection

  constructor(assistant) {
    if (assistant === null || typeof assistant !== 'object' || typeof assistant.commitTruth !== 'function') {
      throw new Error('agents: AgentCouncil requires an Assistant (the shared truth base)');
    }
    this.#assistant = assistant;
  }

  /** An agent proposes { subject, action, because }. The proposal is ALWAYS committed
   *  (audit: the ledger records what was proposed, win or lose). Returns:
   *    { won: true,  id, status }                                  — first proposal on the subject
   *    { won: true,  id, status, corroborates }                    — same action as the winner (agreement)
   *    { won: false, id, status, winnerId, winnerAgent, resolutionId } — conflict: first-committed-wins;
   *      the loser's resolution truth (referencing the winner) is committed too. */
  async propose(agent, { subject, action, because } = {}) {
    if (agent === null || typeof agent !== 'object' || typeof agent.name !== 'string' || agent.name.length === 0
        || typeof agent.version !== 'string' || agent.version.length === 0) {
      throw new Error('agents: propose() needs an agent with { name, version } — the attribution tag is mandatory');
    }
    const body = canonicalAgentProposal(agent.name, agent.version, subject, action, because);
    const res = await this.#assistant.commitTruth(body, 'trace'); // the proposal is truth, win or lose
    const prior = this.#plans.get(subject);
    if (prior === undefined) {
      this.#plans.set(subject, { id: res.contentId, agent: agent.name, agentVersion: agent.version, action, because });
      return { won: true, id: res.contentId, status: res.status, subject };
    }
    if (prior.id === res.contentId) {
      // The same agent replaying the same proposal — idempotent, still the winner.
      return { won: true, id: res.contentId, status: res.status, subject };
    }
    if (prior.action === action) {
      // A different agent proposing the SAME action: agreement, recorded as corroboration.
      return { won: true, id: res.contentId, status: res.status, subject, corroborates: prior.id };
    }
    // CONFLICT: first-committed-wins. The loser records a reference to the winner.
    const resolution = await this.#assistant.commitTruth(canonicalResolution(subject, prior.id, res.contentId), 'trace');
    return {
      won: false, id: res.contentId, status: res.status, subject,
      winnerId: prior.id, winnerAgent: prior.agent, resolutionId: resolution.contentId,
    };
  }

  /** The winning plan items (projection of committed proposal truths), sorted by subject. */
  plans() {
    return [...this.#plans.entries()]
      .map(([subject, w]) => ({ subject, ...w }))
      .sort((a, b) => (a.subject < b.subject ? -1 : 1));
  }

  /** The winning proposal for a subject, or undefined. */
  winner(subject) { const w = this.#plans.get(subject); return w === undefined ? undefined : { subject, ...w }; }
}

/** ResearcherAgent — gathers facts INTO truth. Deterministic actor: research(topic)
 *  scans the SHARED truth base for facts whose event mentions the topic (lowercased
 *  substring — a rule, not understanding) and commits an agent-tagged finding truth
 *  citing the supporting fact ContentIds. */
export class ResearcherAgent {
  name = 'researcher';
  version = '1.0.0';

  async research(assistant, topic) {
    if (typeof topic !== 'string' || topic.length === 0) throw new Error('researcher: topic must be a non-empty string');
    const hits = assistant.truths().filter((t) => t.fact.event.toLowerCase().includes(topic.toLowerCase()));
    const body = canonicalFinding(this.name, this.version, topic, hits.map((h) => h.id));
    const res = await assistant.commitTruth(body, 'trace');
    return { id: res.contentId, status: res.status, topic, facts: body.facts };
  }

  /** Propose a plan item through the council; `findingId` (if given) is cited in the
   *  because — the proposal is evidence-linked by content address. */
  async propose(council, { subject, action, findingId } = {}) {
    const because = findingId === undefined
      ? `researcher rule: subject '${subject}' needs research before action`
      : `researcher rule: act only after the cited finding (finding ${findingId})`;
    return council.propose(this, { subject, action, because });
  }
}

/** SchedulerAgent — proposes plan items from the shared truth base. Deterministic
 *  actor: planDay() walks the truths in stable (ContentId-sorted) order and, for every
 *  event matching its fixed rule (contains 'appointment' or starts with 'renew-'),
 *  proposes booking a calendar block — citing the supporting fact ContentId. */
export class SchedulerAgent {
  name = 'scheduler';
  version = '1.0.0';
  static RULE = /appointment|^renew-/;

  async planDay(assistant, council) {
    const results = [];
    for (const t of assistant.truths()) { // truths() is ContentId-sorted -> deterministic order
      if (SchedulerAgent.RULE.test(t.fact.event)) {
        results.push(await council.propose(this, {
          subject: `plan:${t.fact.event}`,
          action: `book:${t.fact.event}`,
          because: `scheduler rule: event '${t.fact.event}' needs a calendar block (fact ${t.id})`,
        }));
      }
    }
    return results;
  }
}

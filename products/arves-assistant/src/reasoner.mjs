// ARVES Assistant — the PLUGGABLE REASONER SLOT (A4).
//
// ============================================================================
//  THIS FILE CONTAINS NO AI. READ THIS BEFORE ASSUMING OTHERWISE.
// ============================================================================
// The repo ships exactly ONE reasoner: StubReasoner — a DETERMINISTIC, rule-based
// keyword→action table. It does not understand language, it does not generalize,
// it does not learn. It exists so the think→gate→act pipeline is testable OFFLINE
// and REPRODUCIBLY. The intelligence arrives when the maintainer implements the
// Reasoner interface below with a real LLM — OUTSIDE this repo, at their test time
// (PRODUCT_BRIEF_JARVIS.md A4/OQ-3). No network code, no API keys, no model calls
// live anywhere in this product.
//
// ---------------------------------------------------------------------------
// THE REASONER INTERFACE CONTRACT (implement this to plug a real LLM):
// ---------------------------------------------------------------------------
//   interface Reasoner {
//     name:    string   // stable identifier, committed into every proposal truth
//     version: string   // implementation version, committed alongside
//     reason(context) -> proposal   // synchronous or async; MUST NOT commit truth itself
//   }
//
//   context (built by assistant.think(), read-only):
//     { goal:      string                       // the user's goal, verbatim
//       truths:    [{ id, fact, sources }]      // current deduplicated truth projection
//       decisions: [{ id, subject, action, because }]
//       skills:    [string] }                   // names of REGISTERED (certified+bound) skills
//
//   proposal (what the assistant will gate and, if allowed, execute):
//     { action: 'invoke-skill',
//       skill:       string        // must name a registered skill
//       input:       ARVES value   // the exact input the skill will be executed with
//       subject:     string        // what this action is ABOUT (approvals/policies key off it)
//       actionClass: string        // e.g. 'normal' | 'spend' | 'irreversible' — guardrails gate on this
//       because:     string }      // human-readable justification (committed into the proposal truth)
//     | { action: 'none', because: string }
//
// GOVERNANCE IS OUTSIDE THE REASONER, BY DESIGN: whatever reason() proposes, the
// assistant commits the proposal as truth, consults guardrail policies BEFORE any
// skill invocation, and only a certified+bound skill can act. A hostile or buggy
// reasoner can therefore propose anything and still cannot bypass a policy or run
// uncertified code — that is the product's claim: governed, attributed, replayable
// cognition, not "smart".
//
// Determinism rule for IN-REPO reasoners: reason(context) must be a pure function
// of `context` (no clock, no RNG, no ambient state) so demos and tests replay
// byte-identically. A maintainer's LLM-backed reasoner is naturally
// NON-deterministic — that is fine at their runtime: the proposal it emits is
// committed ONCE as content-addressed truth, and replay reads the recorded truth
// rather than re-calling the model (the recorded-truth doctrine, ecosystem-sdk
// REASONING.md).

/** Validate an object against the Reasoner interface contract. Returns the reasoner
 *  or throws loudly — never silently accepts a half-implemented slot. */
export function validateReasoner(r) {
  if (r === null || typeof r !== 'object') throw new Error('reasoner: must be an object { name, version, reason }');
  if (typeof r.name !== 'string' || r.name.length === 0) throw new Error('reasoner: name must be a non-empty string');
  if (typeof r.version !== 'string' || r.version.length === 0) throw new Error('reasoner: version must be a non-empty string');
  if (typeof r.reason !== 'function') throw new Error('reasoner: reason(context) must be a function');
  return r;
}

/** Deterministic slug of a goal string — used to derive stable policy subjects. */
export function goalSlug(goal) {
  return goal.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

/** The default keyword→action rule table for StubReasoner. Each rule is
 *  { keywords, skill, actionClass, subject(ctx), input(ctx) } — subject/input are
 *  PURE functions of the context (fixture-derived, no clock/RNG), so the same
 *  context always yields the same proposal. */
export const DEFAULT_RULES = [
  {
    keywords: ['summarize', 'brief', 'digest'],
    skill: 'day.summarize',
    actionClass: 'normal',
    subject: () => 'day:briefing',
    // The skill input is derived from the TRUTH CONTEXT: the sorted event names of
    // everything the assistant currently knows. Sorted -> order-independent -> stable.
    input: (ctx) => ({ type: 'uci.assistant.skill-input', events: ctx.truths.map((t) => t.fact.event).sort() }),
  },
  {
    keywords: ['order', 'buy', 'purchase', 'pay', 'spend'],
    skill: 'spend.order',
    actionClass: 'spend', // guardrails treat spend-class actions as approval-gated (A6)
    subject: (ctx) => `spend:${goalSlug(ctx.goal)}`,
    input: (ctx) => ({ type: 'uci.assistant.order-request', request: ctx.goal }),
  },
];

/** StubReasoner — the ONLY reasoner in this repo, and it is NOT AI.
 *
 *  A deterministic keyword→action mapping: the first rule (in table order — order IS
 *  the priority, deterministically) whose keyword occurs in the lowercased goal wins;
 *  its subject/input derivations are pure functions of the context. Same context in,
 *  same proposal out, forever. It cannot understand a goal it has no keyword for and
 *  says so honestly (action:'none') instead of guessing.
 *
 *  Plugging a real LLM (maintainer-side, OUTSIDE the repo) — example wiring:
 *
 *    // my-llm-reasoner.mjs — lives in the MAINTAINER'S project, never in this repo
 *    // import Anthropic from '@anthropic-ai/sdk';            // or any model SDK
 *    // export class LlmReasoner {
 *    //   name = 'my-llm-reasoner'; version = '1.0.0';
 *    //   async reason(context) {
 *    //     const msg = await client.messages.create({ ...prompt built from context... });
 *    //     return parseProposal(msg);   // -> the proposal shape documented above
 *    //   }
 *    // }
 *    // ...
 *    // assistant.useReasoner(new LlmReasoner());   // the ONLY line that changes
 *
 *  Everything downstream (proposal-as-truth, guardrail gate, certified-skill-only
 *  execution) is identical for the stub and the LLM — that is the point of the slot. */
export class StubReasoner {
  name = 'stub-reasoner';
  version = '1.0.0';
  #rules;

  constructor(rules = DEFAULT_RULES) {
    if (!Array.isArray(rules) || rules.length === 0) throw new Error('StubReasoner: rules must be a non-empty array');
    for (const r of rules) {
      if (!Array.isArray(r.keywords) || r.keywords.length === 0
        || typeof r.skill !== 'string' || typeof r.actionClass !== 'string'
        || typeof r.subject !== 'function' || typeof r.input !== 'function') {
        throw new Error('StubReasoner: each rule needs { keywords[], skill, actionClass, subject(ctx), input(ctx) }');
      }
    }
    this.#rules = rules;
  }

  /** Pure function of `context`: same context -> same proposal (asserted by tests). */
  reason(context) {
    if (context === null || typeof context !== 'object' || typeof context.goal !== 'string' || context.goal.length === 0) {
      throw new Error('StubReasoner: context.goal must be a non-empty string');
    }
    const g = context.goal.toLowerCase();
    for (const rule of this.#rules) {
      const hits = rule.keywords.filter((k) => g.includes(k));
      if (hits.length > 0) {
        return {
          action: 'invoke-skill',
          skill: rule.skill,
          actionClass: rule.actionClass,
          subject: rule.subject(context),
          input: rule.input(context),
          because: `stub rule: keyword(s) [${hits.join(', ')}] -> skill '${rule.skill}' (deterministic table lookup, NOT reasoning)`,
        };
      }
    }
    return { action: 'none', because: 'stub-reasoner: no keyword rule matches this goal — this stub is NOT AI and cannot generalize; plug a real Reasoner (see the interface contract in reasoner.mjs)' };
  }
}

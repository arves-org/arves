// ARVES Assistant — REASONER ADAPTER EXAMPLE (how to wire a REAL LLM, safely).
//
// ============================================================================
//  THIS FILE CONTAINS NO NETWORK CODE AND NO API KEY. READ THIS FIRST.
// ============================================================================
// It is a COMPLETE, interface-conformant Reasoner (src/reasoner.mjs contract) whose
// only missing piece is YOUR model call — injected as a `client` with an async
// `complete(prompt) -> string`. The adapter builds a prompt from the read-only truth
// context, calls `client.complete`, and parses the reply into the governed proposal
// shape. Everything downstream (proposal-as-truth -> guardrail gate -> certified-skill-
// only execution) is IDENTICAL to the StubReasoner — that is the whole point of the slot.
//
// SAFETY THE ADAPTER ADDS: a real model can hallucinate a capability. `parseProposal`
// REFUSES any `invoke-skill` naming a skill that is not in `context.skills` (the
// registered, certified+bound set) and returns `action:'none'` instead — so a made-up
// skill name never even reaches the runtime. Unparseable output degrades to `none` too.
//
// It is unit-tested against a FAKE client (reasoner-adapter.test.mjs) — deterministic,
// offline, no keys — proving the shape is correct without committing a single network call.

import { validateReasoner } from './reasoner.mjs';

/** Deterministic slug of a goal (for a fallback subject). */
function slug(goal) { return goal.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'goal'; }

/** Pull the first balanced-looking JSON object out of a model reply (tolerates code
 *  fences / prose around it). Returns the substring, or the original if none found. */
function extractJson(text) {
  const a = text.indexOf('{'); const b = text.lastIndexOf('}');
  return (a >= 0 && b > a) ? text.slice(a, b + 1) : text;
}

/** Build the reasoner prompt from the read-only truth context. Pure + deterministic. */
export function buildPrompt(context) {
  const skills = context.skills.length > 0 ? context.skills.join(', ') : '(none registered)';
  const truths = context.truths.length > 0
    ? [...context.truths].map((t) => `- ${t.fact.entity} :: ${t.fact.event}`).sort().join('\n')
    : '(no truths yet)';
  return [
    'You are the reasoner for a GOVERNED assistant. You may only invoke a skill that is',
    'already registered (certified + bound). Choose exactly ONE, or choose none.',
    `Registered skills: ${skills}`,
    'Known truths:',
    truths,
    `Goal: ${context.goal}`,
    'Reply with a SINGLE JSON object and nothing else, one of:',
    '  {"action":"invoke-skill","skill":"<name>","input":{...},"subject":"<key>","actionClass":"normal|spend|irreversible","because":"<why>"}',
    '  {"action":"none","because":"<why>"}',
  ].join('\n');
}

/** Parse a model reply into a governed proposal. Refuses hallucinated skills. */
export function parseProposal(raw, context) {
  let obj;
  if (raw && typeof raw === 'object') obj = raw;
  else if (typeof raw === 'string') {
    try { obj = JSON.parse(extractJson(raw)); }
    catch { return { action: 'none', because: 'llm-adapter: model reply was not parseable JSON' }; }
  } else return { action: 'none', because: 'llm-adapter: empty model reply' };

  if (obj === null || typeof obj !== 'object' || obj.action !== 'invoke-skill') {
    return { action: 'none', because: String(obj && obj.because ? obj.because : 'llm-adapter: model proposed no action') };
  }
  if (typeof obj.skill !== 'string' || !context.skills.includes(obj.skill)) {
    return {
      action: 'none',
      because: `llm-adapter guard: model named skill '${obj.skill}', which is not registered (certified+bound) — refusing rather than inventing a capability`,
    };
  }
  return {
    action: 'invoke-skill',
    skill: obj.skill,
    input: (obj.input !== null && typeof obj.input === 'object') ? obj.input : {},
    subject: (typeof obj.subject === 'string' && obj.subject.length > 0) ? obj.subject : `goal:${slug(context.goal)}`,
    actionClass: typeof obj.actionClass === 'string' ? obj.actionClass : 'normal',
    because: String(obj.because ?? 'llm-adapter proposal'),
  };
}

/** An interface-conformant Reasoner that delegates the ACTUAL model call to an injected
 *  `client.complete(prompt) -> string`. Plug your SDK in there — never in this file. */
export class LlmReasonerAdapter {
  name; version; #client; #buildPrompt; #parse;

  constructor({ client, name = 'llm-reasoner', version = '0.1.0', buildPrompt: bp = buildPrompt, parseProposal: pp = parseProposal } = {}) {
    if (client === null || typeof client !== 'object' || typeof client.complete !== 'function') {
      throw new Error('LlmReasonerAdapter: pass { client } exposing async complete(prompt) -> string — your real LLM call lives THERE, outside this repo');
    }
    if (typeof name !== 'string' || name.length === 0 || typeof version !== 'string' || version.length === 0) {
      throw new Error('LlmReasonerAdapter: name and version must be non-empty strings (committed into every proposal truth)');
    }
    this.name = name; this.version = version;
    this.#client = client; this.#buildPrompt = bp; this.#parse = pp;
  }

  /** reason(context) -> governed proposal. MUST NOT commit truth itself (the assistant does). */
  async reason(context) {
    const prompt = this.#buildPrompt(context);

    // ============================================================================
    //  >>> PUT YOUR API CALL HERE <<<  (via the injected client — never inline here)
    // ----------------------------------------------------------------------------
    // The adapter imports no model SDK and reads no key. Your `client.complete`
    // makes the real call. Maintainer-side wiring (NOT committed to this repo):
    //
    //   import Anthropic from '@anthropic-ai/sdk';
    //   const sdk = new Anthropic();                       // reads ANTHROPIC_API_KEY
    //   const client = {
    //     async complete(prompt) {
    //       const msg = await sdk.messages.create({
    //         model: 'claude-opus-4-8',
    //         max_tokens: 1024,
    //         thinking: { type: 'adaptive' },
    //         messages: [{ role: 'user', content: prompt }],
    //       });
    //       return msg.content.filter((b) => b.type === 'text').map((b) => b.text).join('');
    //     },
    //   };
    //   assistant.useReasoner(new LlmReasonerAdapter({ client }));  // the ONLY line that changes
    //
    // An LLM-backed reasoner is naturally non-deterministic; its proposal is committed
    // ONCE as content-addressed truth and replay reads the record (recorded-truth doctrine).
    // ============================================================================
    const raw = await this.#client.complete(prompt);

    return this.#parse(raw, context);
  }
}

/** Convenience: build + validate the adapter against the Reasoner contract in one call. */
export function makeLlmReasoner(opts) { return validateReasoner(new LlmReasonerAdapter(opts)); }

// ARVES Assistant — the EXAMPLE SKILL LIBRARY (A3): a reusable set of certified
// capabilities the CLI, the quickstart, and the capstone register out of the box.
//
// Each skill is a plain `defineCapability` (ecosystem-sdk trust boundary) with a pure,
// deterministic `execute` and its own representative `testInputs`. They are the SAME kind
// of citizen as any marketplace capability — certification is RE-RUN at registration
// (skills.mjs), a forged `certified` flag is ignored, and the effect values are committed
// through the full Capability -> Engine -> Kernel chain under the bound name.
//
// HONEST SCOPE: these skills do NOT call an LLM or the network — they derive their effect
// deterministically from their input (which the StubReasoner derives, in turn, purely from
// the assistant's committed truth context; see reasoner.mjs DEFAULT_RULES). They exist so
// `ask <goal>` does something visibly useful and byte-reproducibly. Real intelligence is
// the maintainer's LLM in the Reasoner slot; a skill that must reach the network would be
// the maintainer's own capability, authored the same way (defineCapability) — the gate and
// the commit path are identical. Every effect `value` is ACS-canonical (BigInt for ints,
// no bare JS numbers — see the value model in the ecosystem-sdk README).

import { registerSkill, defineCapability } from './skills.mjs';

// ---- input guards ---------------------------------------------------------------------
// A skill's execute() receives input from the REASONER. The StubReasoner always supplies
// the exact shape, but a real LLM reasoner guesses — so a well-authored skill VALIDATES its
// input and fails with a clear domain error instead of a cryptic `Cannot read properties of
// undefined`. (The assistant governs any such throw into a `skill-execution-failed`
// compliance truth — assistant.mjs — so this only sharpens the message, it is not the
// safety net.) These are the pattern a marketplace skill author should copy.

/** Require an ARRAY field on the skill input, or throw a clear, skill-named error. */
function reqArray(input, key, skill) {
  const v = input == null ? undefined : input[key];
  if (!Array.isArray(v)) throw new Error(`${skill}: input.${key} must be an array (got ${input == null ? 'no input' : typeof v})`);
  return v;
}
/** Require a present (non-undefined) scalar field, or throw a clear, skill-named error. */
function reqField(input, key, skill) {
  const v = input == null ? undefined : input[key];
  if (v === undefined || v === null) throw new Error(`${skill}: input.${key} is required`);
  return v;
}

// ---- the skills (factories: a fresh capability object per call) -----------------------

/** day.summarize — a briefing of everything the assistant knows (event names, sorted). */
export const summarizeSkill = () => defineCapability({
  name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
  execute: (input) => {
    const events = reqArray(input, 'events', 'day.summarize');
    return [{
      target: 'uci.assistant.briefing',
      value: { type: 'uci.assistant.briefing', count: BigInt(events.length), events: [...events].sort() },
    }];
  },
});

/** spend.order — place an order request. SPEND-class: the guardrail gate holds it until a
 *  separate committed approval truth exists (A6). */
export const orderSkill = () => defineCapability({
  name: 'spend.order', version: '1.0.0', produces: ['uci.assistant.order'],
  // SPEND-class is bound to the SKILL (not the proposal): the guardrail gate resolves the
  // risk class from here, so a reasoner — stub OR a real LLM — can never downgrade this to
  // 'normal' to slip the spend gate. The default policy (openSession) gates 'spend'.
  actionClass: 'spend',
  execute: (input) => [{
    target: 'uci.assistant.order',
    value: { type: 'uci.assistant.order', request: reqField(input, 'request', 'spend.order'), state: 'placed' },
  }],
});

/** reply.draft — draft a reply addressed to the entities the assistant knows about. */
export const draftReplySkill = () => defineCapability({
  name: 'reply.draft', version: '1.0.0', produces: ['uci.assistant.draft'],
  execute: (input) => {
    const to = [...reqArray(input, 'to', 'reply.draft')].sort();
    return [{
      target: 'uci.assistant.draft',
      value: { type: 'uci.assistant.draft', to, body: `Draft reply addressed to ${to.length} contact(s): ${to.join(', ')}` },
    }];
  },
});

/** schedule.block — propose calendar blocks for the schedulable events (appointments,
 *  renewals, meetings) currently known. */
export const scheduleBlockSkill = () => defineCapability({
  name: 'schedule.block', version: '1.0.0', produces: ['uci.assistant.schedule'],
  execute: (input) => {
    const events = [...reqArray(input, 'events', 'schedule.block')].sort();
    return [{
      target: 'uci.assistant.schedule',
      value: { type: 'uci.assistant.schedule', count: BigInt(events.length), blocks: events.map((e) => `block:${e}`) },
    }];
  },
});

/** notes.digest — a compact digest of the truth base grouped by the entities involved. */
export const notesDigestSkill = () => defineCapability({
  name: 'notes.digest', version: '1.0.0', produces: ['uci.assistant.digest'],
  execute: (input) => {
    const entities = [...reqArray(input, 'entities', 'notes.digest')].sort();
    return [{
      target: 'uci.assistant.digest',
      value: { type: 'uci.assistant.digest', entities, factCount: reqField(input, 'count', 'notes.digest') },
    }];
  },
});

/** The default library, keyed by name, each with its certification test inputs.
 *  The names MUST match the DEFAULT_RULES skill targets in reasoner.mjs. */
export const EXAMPLE_SKILLS = [
  { make: summarizeSkill, testInputs: [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }] },
  { make: orderSkill, testInputs: [{ type: 'uci.assistant.order-request', request: 'order x' }] },
  { make: draftReplySkill, testInputs: [{ type: 'uci.assistant.skill-input', to: ['urn:you'] }] },
  { make: scheduleBlockSkill, testInputs: [{ type: 'uci.assistant.skill-input', events: ['dentist-appointment'] }] },
  { make: notesDigestSkill, testInputs: [{ type: 'uci.assistant.skill-input', entities: ['urn:you'], count: 1n }] },
];

/** Register the whole example library on an assistant (certification RE-RUN per skill,
 *  names bound in the shard — RCR-016). Idempotent over a durable WAL: a second run
 *  re-proves each admission truth as already-committed. Returns the registration reports. */
export async function registerExampleSkills(assistant) {
  const out = [];
  for (const { make, testInputs } of EXAMPLE_SKILLS) {
    out.push(await registerSkill(assistant, make(), testInputs));
  }
  return out;
}

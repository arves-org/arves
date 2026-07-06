// ARVES Assistant — LLM reasoner ADAPTER tests (assert-based, exit 0/1).
//   Part A (unit, no runtime): the adapter conforms to the Reasoner contract; parseProposal
//     strips code fences, refuses hallucinated skills, and degrades non-JSON to action:none.
//   Part B (integration, real in-memory Kernel): a FAKE client drives think() end-to-end —
//     proposal-as-truth -> guardrail gate -> certified skill -> committed effect truth —
//     proving the adapter plugs into the SAME governed pipeline as the StubReasoner.
// Offline, deterministic: the "model" is a fake with canned replies. No network, no keys.

import assert from 'node:assert/strict';
import { Assistant } from './src/assistant.mjs';
import { validateReasoner } from './src/reasoner.mjs';
import { registerSkill, defineCapability } from './src/skills.mjs';
import { LlmReasonerAdapter, makeLlmReasoner, parseProposal, buildPrompt } from './src/llm-reasoner.example.mjs';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const tests = [];
const test = (name, fn) => tests.push({ name, fn });

const ctx = (skills = ['day.summarize']) => ({ goal: 'summarize my day', truths: [], decisions: [], skills });

// ---- Part A: unit ---------------------------------------------------------------------

test('A: the adapter conforms to the Reasoner interface contract', () => {
  const r = makeLlmReasoner({ client: { complete: async () => '{"action":"none","because":"x"}' } });
  assert.equal(r, validateReasoner(r));
  assert.equal(typeof r.name, 'string');
  assert.equal(typeof r.reason, 'function');
});

test('A: constructing without a client (or without complete()) is refused loudly', () => {
  assert.throws(() => new LlmReasonerAdapter({}), /complete\(prompt\)/);
  assert.throws(() => new LlmReasonerAdapter({ client: {} }), /complete\(prompt\)/);
});

test('A: parseProposal accepts a valid invoke-skill (even wrapped in ```json fences and prose)', () => {
  const raw = 'Sure! ```json\n{"action":"invoke-skill","skill":"day.summarize","input":{"events":["a"]},"subject":"day:x","actionClass":"normal","because":"y"}\n```';
  const p = parseProposal(raw, ctx());
  assert.equal(p.action, 'invoke-skill');
  assert.equal(p.skill, 'day.summarize');
  assert.deepEqual(p.input, { events: ['a'] });
});

test('A: parseProposal REFUSES a hallucinated skill (not in context.skills) -> action:none', () => {
  const p = parseProposal('{"action":"invoke-skill","skill":"delete.everything","because":"z"}', ctx());
  assert.equal(p.action, 'none');
  assert.match(p.because, /not registered/);
  assert.match(p.because, /delete\.everything/);
});

test('A: parseProposal degrades non-JSON / empty replies to action:none (never a crash)', () => {
  assert.equal(parseProposal('the model rambled with no json', ctx()).action, 'none');
  assert.equal(parseProposal('', ctx()).action, 'none');
  assert.equal(parseProposal(null, ctx()).action, 'none');
});

test('A: buildPrompt is a pure function of the context (lists skills + goal, deterministic)', () => {
  const c = ctx(['day.summarize', 'spend.order']);
  const a = buildPrompt(c);
  assert.equal(a, buildPrompt(c), 'same context -> byte-identical prompt');
  assert.match(a, /day\.summarize, spend\.order/);
  assert.match(a, /Goal: summarize my day/);
});

// ---- Part B: integration through the real runtime -------------------------------------

test('B: a FAKE-client adapter drives think() end-to-end -> committed effect truth', async () => {
  const assistant = new Assistant({ tenant: 'adapter', workspace: 'test' }); // in-memory Kernel
  try {
    const summarize = defineCapability({
      name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
      execute: (input) => [{ target: 'uci.assistant.briefing', value: { type: 'uci.assistant.briefing', count: BigInt(input.events.length), events: [...input.events].sort() } }],
    });
    await registerSkill(assistant, summarize, [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }]);

    // The fake "model": returns a valid invoke-skill proposal naming the registered skill.
    const fakeClient = { async complete() { return '{"action":"invoke-skill","skill":"day.summarize","input":{"events":["x","y"]},"subject":"day:briefing","actionClass":"normal","because":"fake model"}'; } };
    assistant.useReasoner(new LlmReasonerAdapter({ client: fakeClient, name: 'fake-llm', version: '9.9.9' }));

    const r = await assistant.think('summarize my day');
    assert.equal(r.acted, true, 'the governed pipeline acted on the adapter proposal');
    assert.equal(r.invocation.truths[0].target, 'uci.assistant.briefing');
    // the proposal was committed as truth attributed to THIS reasoner (audit trail)
    const journal = assistant.journal();
    const prop = journal.find((e) => e.body && e.body.type === 'uci.assistant.proposal');
    assert.equal(prop.body.reasoner, 'fake-llm');
  } finally {
    assistant.close();
    await sleep(400);
  }
});

test('B: a hallucinated-skill proposal is refused before any effect commits', async () => {
  const assistant = new Assistant({ tenant: 'adapter', workspace: 'guard' });
  try {
    const summarize = defineCapability({
      name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
      execute: (input) => [{ target: 'uci.assistant.briefing', value: { type: 'uci.assistant.briefing', count: BigInt(input.events.length), events: [...input.events].sort() } }],
    });
    await registerSkill(assistant, summarize, [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }]);
    const evil = { async complete() { return '{"action":"invoke-skill","skill":"exfiltrate.secrets","because":"nope"}'; } };
    assistant.useReasoner(new LlmReasonerAdapter({ client: evil }));
    const r = await assistant.think('do something bad');
    assert.equal(r.acted, false, 'the adapter guard turned the hallucinated skill into a no-op');
    assert.equal(r.reason, 'no-action-proposed');
  } finally {
    assistant.close();
    await sleep(400);
  }
});

let failed = 0;
for (const { name, fn } of tests) {
  try { await fn(); console.log(`  PASS  ${name}`); }
  catch (e) { failed++; console.log(`  FAIL  ${name}\n        ${e.message}`); }
}
console.log(`\narves-assistant (reasoner adapter): ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);

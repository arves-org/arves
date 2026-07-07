// LIVE test for the real OpenAI reasoner. It SKIPS cleanly when OPENAI_API_KEY is unset,
// so CI (which has no key) stays offline-hermetic and this never bills anyone by accident.
// With a key present it makes ONE real call and proves the GOVERNED pipeline is unchanged:
// the model's proposal is committed as truth attributed to openai:<model>, and a made-up
// skill can never reach the runtime.
//
//   OPENAI_API_KEY=sk-... node products/arves-assistant/openai-reasoner.test.mjs
//   (requires the bridge once: cargo build -p arves-bridge --manifest-path runtime/Cargo.toml)
import assert from 'node:assert/strict';
import { Assistant } from './src/assistant.mjs';
import { registerSkill, defineCapability } from './src/skills.mjs';
import { validateReasoner } from './src/reasoner.mjs';
import OpenAiReasoner, { openAiClient } from './src/openai-reasoner.mjs';

if (!process.env.OPENAI_API_KEY) {
  console.log('SKIP: OPENAI_API_KEY not set — the live OpenAI reasoner test is skipped (CI stays offline/hermetic).');
  console.log('      To run it: OPENAI_API_KEY=sk-... node products/arves-assistant/openai-reasoner.test.mjs');
  process.exit(0);
}

let failed = 0;
const ok = (n) => console.log(`  ok  ${n}`);
const bad = (n, e) => { failed++; console.error(`  FAIL ${n}: ${e.message}`); };

// 0) The reasoner conforms to the interface contract (no network needed).
try {
  const r = new OpenAiReasoner();
  validateReasoner(r);
  assert.match(r.name, /^openai:/, 'name is attributed to the model');
  assert.equal(typeof r.reason, 'function');
  ok(`OpenAiReasoner conforms to the Reasoner contract (${r.name})`);
} catch (e) { bad('contract', e); }

// 1) The key is read from the ENV at call time, never captured — a missing key fails loud
//    WITHOUT a network call.
try {
  const saved = process.env.OPENAI_API_KEY;
  delete process.env.OPENAI_API_KEY;
  const client = openAiClient();
  let threw = null;
  try { await client.complete('x'); } catch (e) { threw = e; }
  process.env.OPENAI_API_KEY = saved;
  assert.ok(threw && /OPENAI_API_KEY is not set/.test(threw.message), 'missing key fails loud');
  ok('missing OPENAI_API_KEY fails loud (key read from env at call time, never stored)');
} catch (e) { bad('env-key', e); }

// 2) LIVE governed pipeline: one real call, proposal committed + attributed, no hallucination.
const assistant = new Assistant({ tenant: 'test', workspace: 'openai-live' }); // in-memory
try {
  assistant.useReasoner(new OpenAiReasoner());

  // A single certified + bound skill the model MAY choose.
  await registerSkill(assistant, defineCapability({
    name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
    execute: (input) => [{
      target: 'uci.assistant.briefing',
      value: { type: 'uci.assistant.briefing', note: String(input && input.note ? input.note : 'summary') },
    }],
  }), [{ type: 'uci.assistant.skill-input', note: 'x' }]);

  await assistant.observe('calendar', { entity: 'urn:you', event: 'q3-review', at: 1_752_000_000_000n });

  const model = new OpenAiReasoner().name;
  const r = await assistant.think('summarize my day using the day.summarize skill');

  // The proposal is ALWAYS committed as attributed truth — whether or not it acted.
  const proposalTruth = assistant.journal().find(
    (e) => e.body && e.body.type === 'uci.assistant.proposal' && e.body.reasoner === model,
  );
  assert.ok(proposalTruth, `a proposal truth attributed to '${model}' was committed`);
  ok(`LIVE: real model proposed; proposal committed as truth attributed to '${model}'`);

  // Whatever it proposed, governance held: if it acted, it acted via the REGISTERED skill.
  if (r.acted) {
    assert.equal(r.proposal.skill, 'day.summarize', 'acted only via the certified+bound skill');
    assert.ok(r.invocation.truths.length >= 1, 'effect truth committed');
    ok('LIVE: model acted THROUGH the certified skill (governed think→act end to end)');
  } else {
    // none / blocked are both honest governed outcomes — never a crash, never a bypass.
    ok(`LIVE: model did not act (${r.blocked ? 'blocked by policy' : r.reason || 'no action'}) — governance intact`);
  }
} catch (e) {
  bad('live-governed', e);
} finally {
  assistant.close();
}

if (failed) { console.error(`\nOpenAI reasoner LIVE: ${failed} FAILED`); process.exit(1); }
console.log('\nOpenAI reasoner LIVE: all checks passed');

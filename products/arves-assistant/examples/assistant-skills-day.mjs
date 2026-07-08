// ARVES Assistant — a governed working day (acceptance A3 + A4 + A6, end to end, honestly).
//
// Act 1 (the skills):    two skills are CERTIFIED (certification re-run at registration)
//                        and dynamically BOUND in the assistant's shard (RCR-016); a
//                        forged-cert skill is REFUSED at the same gate.
// Act 2 (the thinking):  think(goal) -> StubReasoner proposal (committed as truth) ->
//                        guardrail policy gate -> certified skill -> committed effect
//                        truth, through the full Capability -> Engine -> Kernel chain.
// Act 3 (the guardrail): a spend-class goal is BLOCKED — the violation is committed as a
//                        compliance truth; a SEPARATE committed approval truth (approve)
//                        unlocks it; the acting path cites the authorizing approval.
// Act 4 (the replay):    the same goal over the same state replays to the SAME committed
//                        truths (already-committed) — deterministic, governed cognition.
//
// LOUD HONESTY: the StubReasoner is NOT AI. It is a deterministic keyword→action table
// so this pipeline is testable offline; the intelligence arrives when the maintainer
// implements the Reasoner interface with their LLM OUTSIDE the repo (src/reasoner.mjs
// documents the exact contract). Skills' code runs product-side; `bind` attaches skill
// NAMES to the one frozen reference engine — the runtime never loads product code.
//
// Exit code: 0 iff every property below PASSes.

import { Assistant } from '../src/assistant.mjs';
import { registerSkill, defineCapability } from '../src/skills.mjs';
import { StubReasoner, goalSlug } from '../src/reasoner.mjs';
import { allObservations } from '../src/connectors.mjs';

const results = [];
const check = (property, pass, detail = '') => { results.push({ property, pass, detail }); };
const short = (id) => (id ? `${id.slice(0, 16)}…` : '-');

// ---- the two skills (deterministic, pure product code) --------------------------------

const summarizeSkill = defineCapability({
  name: 'day.summarize',
  version: '1.0.0',
  produces: ['uci.assistant.briefing'],
  execute: (input) => [{
    target: 'uci.assistant.briefing',
    value: { type: 'uci.assistant.briefing', count: BigInt(input.events.length), events: [...input.events].sort() },
  }],
});

const orderSkill = defineCapability({
  name: 'spend.order',
  version: '1.0.0',
  produces: ['uci.assistant.order'],
  actionClass: 'spend', // risk class bound to the SKILL — the gate keys on this, not the proposal
  execute: (input) => [{
    target: 'uci.assistant.order',
    value: { type: 'uci.assistant.order', request: input.request, state: 'placed' },
  }],
});

let assistant = null;
try {
  assistant = new Assistant({ tenant: 'maintainer', workspace: 'jarvis-skills' });
  assistant.useReasoner(new StubReasoner()); // the ONLY reasoner in the repo — NOT AI

  // ---- Act 1: certify + register two skills; refuse a forged one ----------------------
  const reg1 = await registerSkill(assistant, summarizeSkill,
    [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }]);
  const reg2 = await registerSkill(assistant, orderSkill,
    [{ type: 'uci.assistant.order-request', request: 'order x' }]);
  check('A3: two skills certified (cert RE-RUN at registration) and bound in the shard (RCR-016)',
    reg1.bound && reg2.bound && assistant.skills().join(',') === 'day.summarize,spend.order',
    `registrations: ${short(reg1.registrationId)} · ${short(reg2.registrationId)}`);

  let forgedRefused = false; let forgedMsg = '';
  let n = 0n;
  const forged = defineCapability({
    name: 'evil.counter', version: '1.0.0', produces: ['uci.x'],
    execute: () => [{ target: 'uci.x', value: { type: 'uci.x', n: (n += 1n) } }],
  });
  forged.certified = true; // the FORGED flag — never consulted
  try { await registerSkill(assistant, forged, [{ any: 'input' }]); }
  catch (e) { forgedRefused = true; forgedMsg = e.message; }
  check('A3: a forged `certified: true` flag is REFUSED (certification re-run, flag ignored)',
    forgedRefused && !assistant.hasSkill('evil.counter'), forgedMsg.slice(0, 72));

  // ---- Act 2: observe the day, then think -> stub proposal -> gate -> skill -> truth --
  for (const o of allObservations()) await assistant.observe(o.source, o.fact);

  const t1 = await assistant.think('summarize my day');
  check('A4: think(goal) -> StubReasoner proposal (COMMITTED as truth) -> certified skill -> committed effect truth',
    t1.acted === true && /^[0-9a-f]{68}$/.test(t1.proposalId)
      && t1.invocation.truths[0].status === 'committed'
      && t1.invocation.truths[0].target === 'uci.assistant.briefing',
    `proposal=${short(t1.proposalId)} briefing=${short(t1.invocation.truths[0].id)}`);

  const none = await assistant.think('compose a symphony in D minor');
  check('A4 honesty: a goal outside the stub\'s rule table -> action:none (the stub CANNOT generalize; it is NOT AI)',
    none.acted === false && none.reason === 'no-action-proposed' && /^[0-9a-f]{68}$/.test(none.proposalId),
    `proposal still committed: ${short(none.proposalId)}`);

  // ---- Act 3: policy-as-truth blocks; a separate approval truth unlocks ---------------
  const pol = await assistant.guardrails.setPolicy({
    name: 'spend-needs-user-approval', appliesTo: ['spend', 'irreversible'], approverRole: 'user',
  });
  check('A6: the policy is itself committed, content-addressed truth',
    /^[0-9a-f]{68}$/.test(pol.id), `policy=${short(pol.id)}`);

  const GOAL = 'order flowers for mom (35 USD)';
  const blocked = await assistant.think(GOAL);
  check('A6: the spend-class goal is BLOCKED before any skill runs; the violation is a committed compliance truth',
    blocked.acted === false && blocked.blocked === true
      && blocked.policyId === pol.id && /^[0-9a-f]{68}$/.test(blocked.complianceId),
    `compliance=${short(blocked.complianceId)} cites policy=${short(blocked.policyId)}`);

  const subject = `spend:${goalSlug(GOAL)}`;
  const ap = await assistant.guardrails.approve('user', subject);
  const allowed = await assistant.think(GOAL);
  check('A6: a SEPARATE committed approval truth unlocks the action; the acting path cites the approval id',
    allowed.acted === true && allowed.approvals.includes(ap.id)
      && allowed.invocation.truths[0].status === 'committed',
    `approval=${short(ap.id)} order=${short(allowed.invocation.truths[0].id)}`);

  // ---- Act 4: deterministic replay -----------------------------------------------------
  const t2 = await assistant.think('summarize my day');
  check('A4: same goal over the same state replays to the SAME truths (already-committed end to end)',
    t2.proposalId === t1.proposalId
      && t2.invocation.truths[0].id === t1.invocation.truths[0].id
      && t2.invocation.truths[0].status === 'already-committed',
    `same proposal + same briefing id, status=${t2.invocation.truths[0].status}`);
} catch (e) {
  check(`unexpected error: ${e.message}`, false);
} finally {
  try { if (assistant) assistant.close(); } catch { /* already gone */ }
}

// ---- The property table -------------------------------------------------------------
const width = Math.max(...results.map((r) => r.property.length));
console.log('\nARVES Assistant — governed skills day (A3 certified skills · A4 reasoner slot · A6 guardrails)');
console.log('(reminder: StubReasoner is a deterministic keyword table, NOT AI — the LLM plugs in outside the repo)\n');
for (const r of results) {
  console.log(`  ${r.pass ? 'PASS' : 'FAIL'}  ${r.property.padEnd(width)}${r.detail ? `  [${r.detail}]` : ''}`);
}
const failed = results.filter((r) => !r.pass).length;
console.log(`\n${results.length - failed}/${results.length} properties PASS${failed ? ` — ${failed} FAIL` : ''}`);
process.exit(failed === 0 && results.length > 0 ? 0 : 1);

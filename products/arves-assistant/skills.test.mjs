// ARVES Assistant — stage-2 product-local tests (assert-based, exit 0/1).
//
// Covers A3 + A4 + A6: certification-gate bite (forged flags refused) · bind->invoke
// round trip through the full runtime chain · guardrail block + separate-approval unlock
// (compliance truths committed) · reasoner determinism (same context -> same proposal;
// think() replays end-to-end). Offline, deterministic, no third-party deps; every bridge
// is closed in finally. The in-memory Kernel suffices here (durability is stage 1's
// proof, assistant.test.mjs); bind/invoke/commit semantics are identical.

import assert from 'node:assert/strict';
import { Assistant } from './src/assistant.mjs';
import { registerSkill, defineCapability } from './src/skills.mjs';
import { StubReasoner, DEFAULT_RULES, goalSlug, validateReasoner } from './src/reasoner.mjs';
import { canonicalPolicy } from './src/guardrails.mjs';
import { Arves } from '../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();
// BigInt-safe deep-equality snapshot (JSON can't carry BigInt).
const snap = (v) => JSON.stringify(v, (_, x) => (typeof x === 'bigint' ? `${x}n` : x));

// ---- the two reference skills used across the tests (deterministic, pure) -------------

const summarizeSkill = () => defineCapability({
  name: 'day.summarize',
  version: '1.0.0',
  produces: ['uci.assistant.briefing'],
  execute: (input) => [{
    target: 'uci.assistant.briefing',
    value: {
      type: 'uci.assistant.briefing',
      count: BigInt(input.events.length),
      events: [...input.events].sort(),
    },
  }],
});

const orderSkill = () => defineCapability({
  name: 'spend.order',
  version: '1.0.0',
  produces: ['uci.assistant.order'],
  execute: (input) => [{
    target: 'uci.assistant.order',
    value: { type: 'uci.assistant.order', request: input.request, state: 'placed' },
  }],
});

const SUMMARIZE_INPUTS = [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }];
const ORDER_INPUTS = [{ type: 'uci.assistant.order-request', request: 'order x' }];

const tests = [];
const test = (name, fn) => tests.push({ name, fn });

test('A3: certification gate BITES — forged certified flag, rogue target, vacuous inputs all refused; nothing binds', async () => {
  const a = new Assistant();
  try {
    // (1) FORGED FLAG: a non-deterministic capability with `certified: true` stapled on.
    let n = 0n;
    const forged = defineCapability({
      name: 'evil.counter', version: '1.0.0', produces: ['uci.x'],
      execute: () => [{ target: 'uci.x', value: { type: 'uci.x', n: (n += 1n) } }],
    });
    forged.certified = true;                 // the forged flag
    forged.cert = { certified: true };       // belt and braces — also ignored
    await assert.rejects(() => registerSkill(a, forged, [{ any: 'input' }]), /fails certification.*deterministic/s);

    // (2) ROGUE TARGET: an effect on an undeclared produce is refused.
    const rogue = defineCapability({
      name: 'rogue.target', version: '1.0.0', produces: ['uci.declared'],
      execute: () => [{ target: 'uci.NOT-declared', value: { type: 'uci.x' } }],
    });
    await assert.rejects(() => registerSkill(a, rogue, [{ any: 'input' }]), /fails certification.*effects-declared/s);

    // (3) VACUOUS CERTIFICATION: zero test inputs cannot certify.
    await assert.rejects(() => registerSkill(a, summarizeSkill(), []), /fails certification.*has-test-inputs/s);

    // (4) not a capability at all
    await assert.rejects(() => registerSkill(a, { certified: true }, [{}]), /not a capability/);

    // None of the refused skills was attached or bound.
    assert.deepEqual(a.skills(), []);
    await assert.rejects(() => a.invokeSkill('evil.counter', {}), /not registered/);
    // ...and the runtime itself refuses the never-bound name (the bind really is the gate).
    await assert.rejects(() => a.invokeBound({ type: 'uci.x' }, 'evil.counter'), /unbound/);
  } finally { a.close(); }
});

test('A3 hardening: attachSkill() bypass is gated — certification is RE-RUN inside the assistant itself', async () => {
  const a = new Assistant();
  try {
    let n = 0n;
    const evil = defineCapability({
      name: 'evil.counter', version: '1.0.0', produces: ['uci.x'],
      execute: () => [{ target: 'uci.x', value: { type: 'uci.x', n: (n += 1n) } }],
    });
    const fakeId = 'a'.repeat(68);
    // Casual bypass (no admission evidence at all) is refused.
    assert.throws(() => a.attachSkill('evil.counter', { cap: evil }), /registrationId/);
    // No testInputs -> the gate cannot re-run -> refused (never trusted).
    assert.throws(() => a.attachSkill('evil.counter', { cap: evil, registrationId: fakeId }), /testInputs/);
    // Deliberate forgery (fake registrationId + forged checks): certification is
    // RE-RUN inside attachSkill and BITES anyway — uncertified code cannot attach.
    assert.throws(
      () => a.attachSkill('evil.counter', {
        cap: evil, checks: [{ name: 'deterministic', ok: true }], registrationId: fakeId, testInputs: [{ any: 'input' }],
      }),
      /fails certification/);
    // Not a capability at all.
    assert.throws(() => a.attachSkill('x', { registrationId: fakeId, testInputs: [{}] }), /capability/);
    assert.deepEqual(a.skills(), []);
  } finally { a.close(); }
});

test('A3: certified skill registers (bind, RCR-016) and invokes through the FULL chain -> committed effect truth', async () => {
  const a = new Assistant({ tenant: 'skills', workspace: 'w1' });
  try {
    const reg = await registerSkill(a, summarizeSkill(), SUMMARIZE_INPUTS);
    assert.equal(reg.name, 'day.summarize');
    assert.equal(reg.bound, true);
    assert.match(reg.registrationId, /^[0-9a-f]{68}$/, 'the admission is committed truth');
    assert.deepEqual(a.skills(), ['day.summarize']);

    const input = { type: 'uci.assistant.skill-input', events: ['dentist-appointment', 'renew-passport'] };
    const r1 = await a.invokeSkill('day.summarize', input);
    assert.equal(r1.truths.length, 1);
    assert.equal(r1.truths[0].target, 'uci.assistant.briefing');
    assert.equal(r1.truths[0].status, 'committed');
    // One world: the committed effect truth id equals the SDK-local address of the effect value.
    const expected = arves.address({
      type: 'uci.assistant.briefing', count: 2n, events: ['dentist-appointment', 'renew-passport'],
    }, 'commit');
    assert.equal(r1.truths[0].id, expected);

    // Idempotent through the whole Capability->Engine->Kernel chain (ORCH-004).
    const r2 = await a.invokeSkill('day.summarize', input);
    assert.equal(r2.truths[0].id, r1.truths[0].id);
    assert.equal(r2.truths[0].status, 'already-committed');

    // An unregistered skill cannot act; an unbound name is refused BY THE RUNTIME.
    await assert.rejects(() => a.invokeSkill('spend.order', {}), /not registered/);
    await assert.rejects(() => a.invokeBound({ type: 'uci.x' }, 'spend.order'), /unbound/);
  } finally { a.close(); }
});

test('A6: guardrail BLOCKS a spend-class proposal (compliance truth committed); a SEPARATE approval truth unlocks it', async () => {
  const a = new Assistant({ tenant: 'skills', workspace: 'w2' });
  try {
    a.useReasoner(new StubReasoner());
    await registerSkill(a, orderSkill(), ORDER_INPUTS);
    const pol = await a.guardrails.setPolicy({
      name: 'spend-needs-user-approval', appliesTo: ['spend', 'irreversible'], approverRole: 'user',
    });
    assert.match(pol.id, /^[0-9a-f]{68}$/, 'the policy is committed truth');
    // The policy body is canonical + addressable (policy-as-truth, not config lore).
    assert.equal(pol.id, arves.address(canonicalPolicy('spend-needs-user-approval', ['spend', 'irreversible'], 'user'), 'trace'));

    const goal = 'order flowers for mom';
    const blocked = await a.think(goal);
    assert.equal(blocked.acted, false);
    assert.equal(blocked.blocked, true);
    assert.equal(blocked.policyId, pol.id, 'the block cites the committed policy truth');
    assert.match(blocked.complianceId, /^[0-9a-f]{68}$/, 'the violation is committed as a compliance truth');
    assert.match(blocked.proposalId, /^[0-9a-f]{68}$/, 'the refused proposal is still committed truth');
    assert.equal(blocked.proposal.subject, `spend:${goalSlug(goal)}`);

    // A WRONG-ROLE approval does NOT unlock (the policy names its approver role).
    await a.guardrails.approve('intern', blocked.proposal.subject);
    const stillBlocked = await a.think(goal);
    assert.equal(stillBlocked.blocked, true);
    assert.equal(stillBlocked.complianceId, blocked.complianceId, 'same violation -> same compliance truth (content-addressed)');

    // The SEPARATE committed approval truth (right role) unlocks the action.
    const ap = await a.guardrails.approve('user', blocked.proposal.subject);
    assert.match(ap.id, /^[0-9a-f]{68}$/);
    const allowed = await a.think(goal);
    assert.equal(allowed.acted, true);
    assert.ok(allowed.approvals.includes(ap.id), 'the acting path cites the authorizing approval truth');
    assert.equal(allowed.invocation.truths[0].status, 'committed');
    assert.equal(allowed.invocation.truths[0].id,
      arves.address({ type: 'uci.assistant.order', request: goal, state: 'placed' }, 'commit'));

    // Guardrails never wave through an ungateable proposal.
    assert.throws(() => a.guardrails.check({ subject: '', actionClass: 'spend' }), /subject/);
    assert.throws(() => a.guardrails.check({ subject: 'x' }), /actionClass/);
  } finally { a.close(); }
});

test('A4: reasoner determinism — same context -> same proposal; think() replays end-to-end (already-committed)', async () => {
  const a = new Assistant({ tenant: 'skills', workspace: 'w3' });
  try {
    // Pure determinism of the stub, no runtime involved.
    const stub = new StubReasoner();
    const ctx = {
      goal: 'summarize my day',
      truths: [{ id: 'x', fact: { entity: 'urn:you', event: 'dentist-appointment', at: 1n }, sources: ['notes-file'] }],
      decisions: [], skills: ['day.summarize'],
    };
    assert.equal(snap(stub.reason(ctx)), snap(stub.reason(ctx)), 'same context -> byte-identical proposal');
    assert.equal(stub.reason(ctx).skill, 'day.summarize');
    // Honesty: a goal with no keyword rule is action:'none', never a guess.
    assert.equal(stub.reason({ ...ctx, goal: 'compose a symphony' }).action, 'none');

    // End-to-end replay: the SAME goal over the SAME state commits the SAME truths.
    a.useReasoner(new StubReasoner(DEFAULT_RULES));
    await registerSkill(a, summarizeSkill(), SUMMARIZE_INPUTS);
    await a.observe('notes-file', { entity: 'urn:you', event: 'dentist-appointment', at: 1_751_792_400_000n });
    const t1 = await a.think('summarize my day');
    assert.equal(t1.acted, true);
    assert.equal(t1.invocation.truths[0].status, 'committed');
    const t2 = await a.think('summarize my day');
    assert.equal(t2.proposalId, t1.proposalId, 'the re-proposal is the SAME committed truth');
    assert.equal(t2.invocation.truths[0].id, t1.invocation.truths[0].id);
    assert.equal(t2.invocation.truths[0].status, 'already-committed', 'deterministic end-to-end replay');

    // Slot validation: a half-implemented reasoner is refused loudly.
    assert.throws(() => a.useReasoner({ name: 'x' }), /version/);
    assert.throws(() => a.useReasoner({ name: 'x', version: '1', reason: 'not-a-fn' }), /function/);
    assert.throws(() => validateReasoner(null), /object/);
  } finally { a.close(); }
});

test('A4: think() without a reasoner refuses loudly; a proposal for an unregistered skill cannot act', async () => {
  const a = new Assistant();
  try {
    await assert.rejects(() => a.think('summarize my day'), /no reasoner attached/);
    a.useReasoner(new StubReasoner());
    // 'day.summarize' proposed but never registered/bound -> loud refusal, no invocation.
    await assert.rejects(() => a.think('summarize my day'), /unregistered skill/);
    // The refusal itself is committed as a compliance truth (mirror of guardrail blocks):
    // the ledger records what was refused, and the same refusal replays to the SAME id.
    const idOf = (e) => e.message.match(/refusal committed: ([0-9a-f]{68})/)?.[1];
    let e1; let e2;
    await a.think('summarize my day').catch((e) => { e1 = e; });
    await a.think('summarize my day').catch((e) => { e2 = e; });
    assert.ok(idOf(e1), 'the refusal compliance truth id is surfaced');
    assert.equal(idOf(e1), idOf(e2), 'same refusal -> same content-addressed compliance truth');
    await assert.rejects(() => a.think(''), /goal/);
  } finally { a.close(); }
});

// ---- runner ---------------------------------------------------------------------------
let failed = 0;
for (const { name, fn } of tests) {
  try {
    await fn();
    console.log(`  PASS  ${name}`);
  } catch (e) {
    failed++;
    console.log(`  FAIL  ${name}\n        ${e.message}`);
  }
}
console.log(`\narves-assistant (stage 2 — skills/reasoner/guardrails): ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);

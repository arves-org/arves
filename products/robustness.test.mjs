// Product robustness regression suite — locks the fixes from the whole-system destroy
// pass so the fragilities cannot silently return. Plain Node, no deps.
// Run: node products/robustness.test.mjs   (exit 0 = all pass)

import assert from 'node:assert/strict';
import { encode, float } from './arves-sdk-ts/src/codec.mjs';
import { KernelBridge } from './arves-sdk-ts/src/bridge.mjs';
import { CognitiveMemory, replay } from './arves-cognitive-memory/src/memory.mjs';
import { allSources } from './arves-cognitive-memory/src/connectors.mjs';
import { PersonalCognitiveOS } from './arves-personal-os/src/personal-os.mjs';
import { personalReality } from './arves-personal-os/src/connectors.mjs';
import { EnterpriseCognitiveOS } from './arves-enterprise-os/src/enterprise-os.mjs';

let n = 0;
const ok = (name, cond) => { assert.ok(cond, name); n++; console.log('  ✓', name); };
const threw = (name, fn, match) => {
  let e = null;
  try { fn(); } catch (x) { e = x; }
  assert.ok(e && (!match || String(e.message).includes(match)), `${name} (expected throw${match ? ` ~ ${match}` : ''})`);
  n++; console.log('  ✓', name);
};

console.log('SDK codec:');
{
  // depth bomb -> clean typed error, not a stack overflow
  let deep = 0n; for (let i = 0; i < 20000; i++) deep = [deep];
  threw('deep nesting rejected (MAX_DEPTH)', () => encode(deep), 'MAX_DEPTH');
  threw('integer > 2^64-1 rejected', () => encode(2n ** 64n), 'range');
  threw('integer < -2^64 rejected', () => encode(-(2n ** 64n) - 1n), 'range');
  threw('undefined rejected (not silently null)', () => encode({ a: undefined }), 'undefined');
  threw('bare number rejected', () => encode(5), 'BigInt');
  ok('in-range boundary 2^64-1 encodes', encode(2n ** 64n - 1n).length === 9);
  // undefined must NOT alias to null (distinct addresses / here: undefined throws, null ok)
  ok('null encodes to 0xf6', encode(null)[0] === 0xf6);
  ok('float still works', encode(float(0.5)).length === 9);
}

console.log('Cognitive Memory:');
{
  const m = new CognitiveMemory();
  for (const o of allSources()) m.ingest(o);
  ok('audit chain verifies intact', m.verifyChain().ok === true);
  // tamper a past entry on a copy -> detected
  const log = m.auditTrail().map((e) => ({ ...e }));
  log[0] = { ...log[0], source: 'forged' };
  const det = m.verifyChain(log, m.head());
  ok('tampering a past entry is DETECTED', det.ok === false && det.brokenAt === 0);
  ok('auditTrail() is an immutable copy', Object.isFrozen(m.auditTrail()[0]));
  // false-merge fix: two genuinely different events stay distinct
  const m2 = new CognitiveMemory();
  m2.ingest({ source: 'email', raw: { attendee: 'ada@analytical.example', subject: 'Board', epochMs: 1751468400000 } });
  m2.ingest({ source: 'email', raw: { attendee: 'ada@analytical.example', subject: 'Board Meeting', epochMs: 1751468400000 } });
  ok('distinct events are NOT falsely merged (Board != Board Meeting)', m2.truths().length === 2);
  // replay determinism is order-independent
  const forward = allSources();
  const reversed = allSources().reverse();
  ok('replay root is ingest-order-independent', replay(forward).root() === replay(reversed).root());
}

console.log('Kernel bridge client:');
{
  // A missing/failed bridge exe must REJECT pending calls, never hang (and never crash
  // the process with an unhandled 'error').
  const dead = new KernelBridge('/no/such/arves-bridge-exe', { timeoutMs: 3000 });
  let rejected = false;
  try { await dead.commit({ type: 'x' }); } catch { rejected = true; }
  dead.close();
  ok('missing bridge exe rejects (no hang, no crash)', rejected);
}
{
  // Protocol injection: a capability with whitespace/newline is refused before send.
  const b = new KernelBridge('/no/such/exe', { timeoutMs: 2000 });
  let injRejected = false;
  try { await b.invoke({ type: 'x' }, 'evil cap\n01 6161'); } catch { injRejected = true; }
  b.close();
  ok('capability injection (whitespace/newline) refused', injRejected);
}

console.log('Personal Cognitive OS (P4):');
{
  const bridge = new KernelBridge();
  const decide = async (osx) => osx.recordDecision({ subject: 'invest:acme-fund', action: 'decline', because: 'risk' });
  // Reproducibility must NOT depend on ingest order (else the replay/audit claim is false).
  const osF = new PersonalCognitiveOS(bridge); await decide(osF);
  for (const o of personalReality()) await osF.observe(o);
  const bF = await osF.dailyBriefing();
  const osR = new PersonalCognitiveOS(bridge); await decide(osR);
  for (const o of [...personalReality()].reverse()) await osR.observe(o);
  const bR = await osR.dailyBriefing();
  ok('briefing id is independent of ingest order', bF.id === bR.id);
  ok('the meeting dedups to one truth (3 systems)', osF.truths().filter((t) => t.fact.event === 'q3-review').length === 1);
  ok('genuinely different events stay distinct', osF.truths().length === 4);
  // Contradiction detection must be precise: no false positives.
  ok('opposing action → contradiction', osF.checkContradiction({ subject: 'invest:acme-fund', action: 'approve' }).contradicts === true);
  ok('same action → no false contradiction', osF.checkContradiction({ subject: 'invest:acme-fund', action: 'decline' }).contradicts === false);
  ok('subject with no prior decision → no contradiction', osF.checkContradiction({ subject: 'invest:unknown', action: 'approve' }).contradicts === false);
  bridge.close();
}

console.log('Enterprise Cognitive OS (P5):');
{
  const bridge = new KernelBridge();
  const org = new EnterpriseCognitiveOS(bridge);
  await org.setPolicy({ domain: 'spend', rule: 'spend>100k requires legal approval', thresholdUsd: 100000n });
  const blocked = await org.proposeDecision({ agent: 'finance', subject: 'spend:x', action: 'approve', amountUsd: 150000n, approvals: [] });
  ok('policy blocks a violating decision', blocked.committed === false);
  const allowed = await org.proposeDecision({ agent: 'finance', subject: 'spend:x', action: 'approve', amountUsd: 150000n, approvals: ['legal'] });
  ok('compliant decision (legal approval) is committed', allowed.committed === true);
  const conflict = await org.proposeDecision({ agent: 'ops', subject: 'spend:x', action: 'cancel' });
  ok('cross-department conflict is blocked', conflict.committed === false && conflict.reason === 'cross-department-conflict');
  const small = await org.proposeDecision({ agent: 'finance', subject: 'spend:coffee', action: 'approve', amountUsd: 500n });
  ok('compliant small spend is NOT falsely blocked', small.committed === true);
  bridge.close();
}

console.log(`\n${n}/${n} robustness regressions PASS`);
process.exit(0);

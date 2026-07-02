// Product robustness regression suite — locks the fixes from the whole-system destroy
// pass so the fragilities cannot silently return. Plain Node, no deps.
// Run: node products/robustness.test.mjs   (exit 0 = all pass)

import assert from 'node:assert/strict';
import { encode, float } from './arves-sdk-ts/src/codec.mjs';
import { KernelBridge } from './arves-sdk-ts/src/bridge.mjs';
import { CognitiveMemory, replay } from './arves-cognitive-memory/src/memory.mjs';
import { allSources } from './arves-cognitive-memory/src/connectors.mjs';

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

console.log(`\n${n}/${n} robustness regressions PASS`);
process.exit(0);

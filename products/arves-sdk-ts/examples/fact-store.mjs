// Why ARVES matters, in one runnable file.
//
// An ordinary datastore gives you storage. ARVES gives you *identity, idempotency,
// integrity, and replay* for free — because identity IS the content address. This
// demo shows four things a normal app framework cannot give you without bespoke work.
//
// Run: node examples/fact-store.mjs

import { Arves, FactStore } from '../src/arves.mjs';

const arves = new Arves();
const store = new FactStore();

console.log('ARVES SDK demo — a content-addressed cognitive fact store\n');

// 1. DETERMINISTIC IDENTITY + IDEMPOTENCY.
//    The same fact authored with DIFFERENT field order is the SAME fact.
const a = store.commit({ type: 'uci.fact', claim: 'sky-is-blue', confidence: arves.float(0.5), observed_at: 1730000000000000000n });
const b = store.commit({ observed_at: 1730000000000000000n, confidence: arves.float(0.5), type: 'uci.fact', claim: 'sky-is-blue' });
console.log('1. Idempotency / canonical identity');
console.log('   fact A id :', a);
console.log('   fact B id :', b, '(same fact, keys reordered)');
console.log('   equal?    :', a === b, '| store size:', store.size, '(deduped — one truth, not two)\n');

// 2. INTEGRITY. Change one field -> a different address. Tamper is self-evident.
const c = store.commit({ type: 'uci.fact', claim: 'sky-is-green', confidence: arves.float(0.5), observed_at: 1730000000000000000n });
console.log('2. Integrity / tamper-evidence');
console.log('   changed claim id:', c);
console.log('   differs from A? :', c !== a, '| store size:', store.size, '\n');

// 3. EXACT 64-BIT INTEGERS. A nanosecond timestamp exceeds 2^53; a float-backed store
//    would silently corrupt it. ARVES mandates an exact integer carrier (ACS-002 §5.2).
const ns = 1730000000123456789n;
const t1 = arves.commit({ type: 'uci.observation', at: ns });
const t2 = arves.commit({ type: 'uci.observation', at: BigInt(Number(ns)) }); // float round-trip
console.log('3. Exact large integers (no silent corruption)');
console.log('   exact ns     :', ns.toString());
console.log('   via float    :', BigInt(Number(ns)).toString(), '(corrupted)');
console.log('   same address?:', t1 === t2, '(false — ARVES caught the precision loss)\n');

// 4. DETERMINISTIC REPLAY. A decision trace is content-addressed; recomputing the same
//    ordered steps yields the identical root — reproducible reasoning, cross-machine.
const trace = [a, c];
const root1 = arves.traceRoot(trace);
const root2 = arves.traceRoot([a, c]);
console.log('4. Replay / reproducible reasoning');
console.log('   trace root   :', root1);
console.log('   recomputed   :', root2);
console.log('   identical?   :', root1 === root2, '\n');

console.log('All four properties came for free from one primitive: the content address.');
console.log('That is why ARVES matters — build cognitive apps that are deterministic,');
console.log('idempotent, tamper-evident, and replayable by construction.');

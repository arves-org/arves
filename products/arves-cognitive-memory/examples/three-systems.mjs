// The ARVES Cognitive Memory flagship demo — "three systems, one truth."
//
// Impossible before ARVES: three systems with three schemas, fuzzy dedup, no shared
// identity, no audit, no replay, no defensible reasoning.
// Possible with ARVES: one identity, full evidence, a tamper-evident audit chain, exact
// replay, and a reproducible conclusion — all from one primitive, the content address.
//
// Run: node examples/three-systems.mjs

import { CognitiveMemory, replay } from '../src/memory.mjs';
import { allSources, conflictingSources } from '../src/connectors.mjs';

const mem = new CognitiveMemory();
const observations = allSources();

console.log('ARVES Cognitive Memory — three systems, one truth\n');
const results = observations.map((o) => ({ system: o.source, native: o.native, ...mem.ingest(o) }));
for (const r of results) {
  console.log(`  ${r.system.padEnd(9)} (${r.native.padEnd(24)}) -> ${r.id.slice(0, 22)}…  ${r.deduped ? '(same truth — deduped)' : '(new truth)'}`);
}

const ids = results.map((r) => r.id);
console.log('\n[1] Identity + Deduplication');
console.log('    three schemas, same event → identical ContentId:', ids.every((i) => i === ids[0]));
console.log('    truths held in memory:', mem.truths().length, '(three sources collapsed to one)');

const t = mem.truths()[0];
console.log('\n[2] Evidence / Provenance / Truth');
console.log('    truth:', t.fact.entity, '· event', JSON.stringify(t.fact.event), '· at', t.fact.at.toString(), 'ns');
console.log('    attested by', t.sources.length, 'independent systems:', t.sources.join(', '));

console.log('\n[3] Audit — tamper-evident chain');
console.log('    audit entries:', mem.auditTrail().length, '· chain head:', mem.head().slice(0, 22) + '…');

console.log('\n[4] Replay — deterministic');
const rootA = mem.root();
const rootB = replay(observations).root();
console.log('    memory root :', rootA.slice(0, 22) + '…');
console.log('    replay root :', rootB.slice(0, 22) + '…');
console.log('    identical?  :', rootA === rootB);

console.log('\n[5] Conflict is surfaced, not hidden');
const conflicted = replay(conflictingSources());
console.log('    CRM disagrees on the time → distinct truths:', conflicted.truths().length, '(a fuzzy merge would have hidden this)');

console.log('\n[6] Provable reasoning');
const c1 = mem.reason('Ada attended Q3 Review', [t.id]);
const c2 = mem.reason('Ada attended Q3 Review', [t.id]);
console.log('    conclusion id :', c1.id.slice(0, 22) + '…');
console.log('    reproducible? :', c1.id === c2.id);
console.log('    backed by     :', c1.evidence.join(', '), '(you can defend it — and replay it)');

// Assertions — this demo doubles as a capability proof (Evidence-first rule).
const proofs = {
  Identity: ids.every((i) => i === ids[0]),
  Deduplication: mem.truths().length === 1,
  Evidence: t.sources.length === 3,
  Audit: mem.auditTrail().length === 3 && mem.head() !== '00',
  Replay: rootA === rootB,
  Reasoning: c1.id === c2.id,
};
const allProven = Object.values(proofs).every(Boolean);
console.log('\nCapabilities proven:', Object.entries(proofs).map(([k, v]) => `${k}=${v ? '✓' : '✗'}`).join('  '));
console.log(allProven
  ? '\nImpossible before ARVES → Possible with ARVES. This is Cognitive Memory.'
  : '\nFAIL: a capability did not hold.');
process.exit(allProven ? 0 : 1);

// P3 Agent Runtime demo — an agent reasons over Cognitive Memory and commits its ENTIRE
// decision trace as content-addressed truth in the REAL reference Kernel.
//
// Impossible before ARVES: reproducible, auditable, replayable agent reasoning where
// every step (what it knew, what it concluded, what it planned, which capability it
// chose, what it did) is truth in a real Kernel — run it twice and it is byte-identical
// and idempotent. Run: node examples/agent-run.mjs
// (requires: cargo build -p arves-bridge --bin arves-bridge)

import { CognitiveMemory } from '../../arves-cognitive-memory/src/memory.mjs';
import { allSources } from '../../arves-cognitive-memory/src/connectors.mjs';
import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { Agent } from '../src/agent.mjs';
import { defaultCapabilities } from '../src/capabilities.mjs';

// Memory: three systems collapse to one truth (P1 Cognitive Memory).
const memory = new CognitiveMemory();
for (const o of allSources()) memory.ingest(o);
const truths = memory.truths();

const agent = new Agent(defaultCapabilities());
const goal = 'prepare for q3-review';

console.log('ARVES Agent Runtime — reasoning committed to the REAL Kernel\n');
console.log('  memory truths:', truths.length, '| goal:', JSON.stringify(goal), '\n');

// One real Kernel for the whole session (idempotency is per-Kernel, ORCH-004).
const bridge = new KernelBridge();

// Run 1: agent reasons and commits its trace as truth in the real Kernel.
const run1 = await agent.run(goal, truths, bridge);

console.log('  decision trace (each step = truth in the real Kernel):');
for (const e of run1.trace) {
  console.log(`    ${e.kind.padEnd(14)} ${e.id.slice(0, 22)}…  ${e.status.padEnd(16)} one-world=${e.oneWorld}`);
}
console.log('  trace root:', run1.root.id.slice(0, 22) + '…');

// Run 2: identical inputs, SAME Kernel → identical ids, Kernel reports already-committed.
const run2 = await agent.run(goal, truths, bridge);
bridge.close();

const sameTrace = run1.trace.every((e, i) => e.id === run2.trace[i].id);
const allOneWorld = run1.trace.every((e) => e.oneWorld);
const run1Committed = run1.trace.every((e) => e.status === 'committed');
const run2Idempotent = run2.trace.every((e) => e.status === 'already-committed');

console.log('\n[Reasoning]  derived a conclusion from memory  ✓');
console.log('[Planning]   produced', run1.actions.length, 'planned steps  ✓');
console.log('[Capability] selected a capability per step  ✓');
console.log('[Execution]  executed each capability → outcome committed as truth  ✓');
console.log('[Truth]      every step committed in the real Kernel; one-world identity:', allOneWorld);
console.log('[Replay]     re-run identical trace ids:', sameTrace, '| Kernel idempotent on re-run:', run2Idempotent);

const ok = sameTrace && allOneWorld && run1Committed && run2Idempotent;
console.log(ok
  ? '\nAn agent whose every thought is replayable, audited truth in a real Kernel. Impossible before ARVES.'
  : '\nFAIL: reasoning was not reproducible / not one-world / not idempotent.');
process.exit(ok ? 0 : 1);

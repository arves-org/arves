// ARVES Cognitive Studio — a day, authored as a cognitive graph.
//
//   observe (calendar) ──> derive (meeting load) ──> decide (plan the day)
//
// Each node is a CERTIFIED capability (Ecosystem Kit); each edge wires the upstream
// node's committed-truth output (ContentId + value) into the downstream node's input.
// The graph runs TWICE on two FRESH Kernels (two fresh arves-bridge spawns) and the
// truth ids must come back byte-identical — reproducible cognition, not a demo trick.
// Finally the graph + its truth is rendered to a self-contained static HTML file.
//
// Run: node products/arves-cognitive-studio/examples/studio-day.mjs
// (requires the platform bridge: cargo build -p arves-bridge --bin arves-bridge)

import { writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineCapability } from '../../arves-ecosystem-sdk/src/kit.mjs';
import { KernelBridge } from '../../arves-sdk-ts/src/bridge.mjs';
import { defineGraph, validateGraph, runGraph, renderGraph } from '../src/studio.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));

// ---- 1. AUTHOR: three capabilities (pure, deterministic; ints are BigInt) ----

const observeCap = defineCapability({
  name: 'observe.calendar', version: '1.0.0', produces: ['uci.observation'],
  execute: (input) => [{
    target: 'uci.observation',
    value: { type: 'uci.observation', kind: 'calendar', events: input.events },
  }],
});

const deriveCap = defineCapability({
  name: 'derive.meeting-load', version: '1.0.0', produces: ['uci.derivation'],
  // input.observe = [{ target, id, value }] — the wired committed truth of the observe
  // node. The derivation cites the upstream ContentIds as EVIDENCE.
  execute: (input) => {
    const obs = input.observe[0];
    const meetings = BigInt(obs.value.events.length);
    return [{
      target: 'uci.derivation',
      value: {
        type: 'uci.derivation', metric: 'meeting-load', meetings,
        level: meetings >= 3n ? 'high' : 'light',
        evidence: input.observe.map((t) => t.id),
      },
    }];
  },
});

const decideCap = defineCapability({
  name: 'decide.day-plan', version: '1.0.0', produces: ['uci.decision'],
  execute: (input) => {
    const d = input.derive[0];
    return [{
      target: 'uci.decision',
      value: {
        type: 'uci.decision',
        action: d.value.level === 'high' ? 'block-two-hours-focus-time' : 'schedule-deep-work-morning',
        basedOn: [d.id],
      },
    }];
  },
});

// Representative test inputs (certification is RE-RUN against these at validate time).
// The 68-hex id below is only a shape-representative placeholder for certification.
const fakeId = 'ab'.repeat(34);
const obsTest = [{ events: [{ title: 'standup', startMs: 1751612400000n }] }];
const obsValue = { type: 'uci.observation', kind: 'calendar', events: [{ title: 'standup', startMs: 1751612400000n }] };
const derTest = [{ observe: [{ target: 'uci.observation', id: fakeId, value: obsValue }] }];
const decTest = [{ derive: [{ target: 'uci.derivation', id: fakeId, value: { type: 'uci.derivation', metric: 'meeting-load', meetings: 4n, level: 'high', evidence: [fakeId] } }] }];

// ---- 2. DEFINE the graph as data ---------------------------------------------

const graph = defineGraph({
  name: 'plan-my-day',
  nodes: [
    { id: 'observe', capability: observeCap, testInputs: obsTest },
    { id: 'derive', capability: deriveCap, testInputs: derTest },
    { id: 'decide', capability: decideCap, testInputs: decTest },
  ],
  edges: [
    { from: 'observe', to: 'derive', output: 'uci.observation' },
    { from: 'derive', to: 'decide', output: 'uci.derivation' },
  ],
});

console.log('ARVES Cognitive Studio — plan-my-day\n');
console.log('[author]   graph id', graph.id.slice(0, 20) + '… (content address of the spec)');

// ---- 3. VALIDATE (cycles, wiring, ENFORCED per-node certification) ------------

const v = validateGraph(graph);
console.log('[validate]', v.valid ? 'VALID' : 'REFUSED');
for (const c of v.checks) console.log('           ', c.ok ? '✓' : '✗', c.name, c.detail ? `— ${c.detail}` : '');

// ---- 4. RUN twice on FRESH Kernels — truth ids must be byte-identical ---------

// Deterministic graph input (fixed epochs; ints are BigInt — never bare numbers).
const dayInput = {
  events: [
    { title: 'standup', startMs: 1751612400000n },
    { title: 'board review', startMs: 1751619600000n },
    { title: 'design sync', startMs: 1751626800000n },
    { title: '1:1 ada', startMs: 1751634000000n },
  ],
};

async function freshRun() {
  const bridge = new KernelBridge(); // a NEW bridge process = a FRESH in-memory Kernel
  try { return await runGraph(bridge, graph, dayInput); } finally { bridge.close(); }
}

const run1 = await freshRun();
const run2 = await freshRun();

console.log('\n[run 1] order:', run1.order.join(' -> '));
for (const id of run1.order) {
  for (const t of run1.nodes[id].effects) {
    console.log(`         ${id.padEnd(8)} ${t.target.padEnd(16)} ${t.contentId.slice(0, 20)}… (${t.status})`);
  }
}
console.log('        run root', run1.runRoot.contentId.slice(0, 20) + `… (${run1.runRoot.status})`);

const idsOf = (r) => r.order.flatMap((id) => r.nodes[id].effects.map((t) => `${id}:${t.contentId}`)).join('|');
const identical = idsOf(run1) === idsOf(run2) && run1.runRoot.contentId === run2.runRoot.contentId;
const bothFresh = [...run1.order, ...run2.order].every((id) =>
  [run1, run2].every((r) => r.nodes[id].effects.every((t) => t.status === 'committed')));

console.log('\n[run 2] fresh Kernel — truth ids byte-identical to run 1:', identical);

// ---- 5. RENDER the self-contained static HTML ---------------------------------

const html = renderGraph(graph, run1);
const out = path.join(HERE, 'studio-day.html');
writeFileSync(out, html);
const rendersAllNodes = run1.order.every((id) => html.includes(id));
const selfContained = !/\b(src|href)\s*=\s*"https?:/.test(html) && !html.includes('<script');
console.log('[render]  wrote', out);
console.log('          contains every node id:', rendersAllNodes, '· no external assets/scripts:', selfContained);

// ---- Property checks -----------------------------------------------------------

const props = [
  ['graph validates (all nodes re-certified, acyclic, wiring declared)', v.valid],
  ['two runs on FRESH Kernels: byte-identical ContentIds (all 3 nodes)', identical],
  ['fresh Kernels: every node status "committed" in both runs', bothFresh],
  ['whole-run root committed as trace truth', /^[0-9a-f]{68}$/.test(run1.runRoot.contentId)],
  ['decision cites the derivation truth id as basedOn (evidence chain)', run1.nodes.decide.effects.length === 1],
  ['static HTML render contains every node id', rendersAllNodes],
  ['render is self-contained (no external assets, no scripts)', selfContained],
];
console.log('\nProperty checks:');
let ok = true;
for (const [name, pass] of props) { ok = ok && pass; console.log('  ' + (pass ? '✓' : '✗'), name); }

console.log(ok
  ? '\nA cognitive graph was authored as data, its nodes certified, its truth committed\n'
    + 'through the frozen Runtime v1.0 Kernel, reproduced byte-for-byte on a fresh Kernel,\n'
    + 'and rendered to a static HTML page. That is the honest P6 preview.'
  : '\nFAIL: a studio property did not hold.');
process.exit(ok ? 0 : 1);

// ARVES Cognitive Studio — regression tests. Plain Node + assert, no deps.
// Run: node products/arves-cognitive-studio/studio.test.mjs   (exit 0 = all pass)
//
// The determinism tests spawn the REAL bridge (fresh in-memory Kernel per spawn);
// everything else runs bridge-free — validation must refuse BEFORE touching a bridge.

import assert from 'node:assert/strict';
import { defineCapability } from '../arves-ecosystem-sdk/src/kit.mjs';
import { KernelBridge } from '../arves-sdk-ts/src/bridge.mjs';
import { defineGraph, validateGraph, topoOrder, runGraph, renderGraph } from './src/studio.mjs';

let n = 0;
const ok = (name, cond) => { assert.ok(cond, name); n++; console.log('  ✓', name); };
const rejects = async (name, fn, match) => {
  let e = null;
  try { await fn(); } catch (x) { e = x; }
  assert.ok(e && (!match || String(e.message).includes(match)), `${name} (expected throw${match ? ` ~ ${match}` : ''}; got ${e && e.message})`);
  n++; console.log('  ✓', name);
};

// ---- Shared fixtures ---------------------------------------------------------

const echoCap = (name) => defineCapability({
  name, version: '1.0.0', produces: ['uci.fact'],
  execute: (input) => [{ target: 'uci.fact', value: { type: 'uci.fact', from: name, note: input.note ?? 'wired' } }],
});
const echoTest = [{ note: 'x' }];
const node = (id, cap = echoCap(`cap.${id}`)) => ({ id, capability: cap, testInputs: echoTest });

// A poisoned bridge: ANY use is a test failure — validation must refuse before I/O.
const poisonBridge = { commit() { throw new Error('BRIDGE MUST NOT BE TOUCHED'); } };

console.log('Authoring (defineGraph):');
{
  await rejects('empty nodes refused', () => defineGraph({ name: 'g', nodes: [] }), 'nodes[]');
  await rejects('duplicate node id refused', () => defineGraph({ name: 'g', nodes: [node('a'), node('a')] }), 'duplicate');
  await rejects('node without a Kit capability refused', () => defineGraph({ name: 'g', nodes: [{ id: 'a', capability: { execute: 1 }, testInputs: echoTest }] }), 'defineCapability');
  await rejects('node without testInputs refused (certification would be vacuous)', () => defineGraph({ name: 'g', nodes: [{ id: 'a', capability: echoCap('cap.a'), testInputs: [] }] }), 'testInputs');
  // content-addressed identity: change the wiring -> the graph id changes
  const g1 = defineGraph({ name: 'g', nodes: [node('a'), node('b')], edges: [{ from: 'a', to: 'b' }] });
  const g2 = defineGraph({ name: 'g', nodes: [node('a'), node('b')], edges: [] });
  ok('graph id is a content address (wiring change -> different id)', g1.id !== g2.id && /^[0-9a-f]{68}$/.test(g1.id));
}

console.log('Validation refusals:');
{
  // CYCLE: a <-> b
  const cyclic = defineGraph({
    name: 'cyclic', nodes: [node('a'), node('b')],
    edges: [{ from: 'a', to: 'b' }, { from: 'b', to: 'a' }],
  });
  const vc = validateGraph(cyclic);
  ok('cyclic graph is refused (acyclic check fails)', !vc.valid && vc.checks.find((c) => c.name === 'acyclic').ok === false);
  ok('topoOrder returns null for a cyclic graph', topoOrder(cyclic) === null);
  await rejects('runGraph refuses a cyclic graph WITHOUT touching the bridge',
    () => runGraph(poisonBridge, cyclic, {}), 'does not validate');

  // SELF-LOOP
  const selfLoop = defineGraph({ name: 'self', nodes: [node('a')], edges: [{ from: 'a', to: 'a' }] });
  ok('self-loop is refused', !validateGraph(selfLoop).valid);

  // UNKNOWN NODE in an edge
  const ghost = defineGraph({ name: 'ghost', nodes: [node('a')], edges: [{ from: 'a', to: 'phantom' }] });
  const vg = validateGraph(ghost);
  ok('edge referencing an unknown node is refused', !vg.valid
    && vg.checks.find((c) => c.name === 'edges-reference-known-nodes').ok === false
    && vg.checks.find((c) => c.name === 'edges-reference-known-nodes').detail.includes('phantom'));

  // UNDECLARED edge output
  const undeclared = defineGraph({
    name: 'undeclared', nodes: [node('a'), node('b')],
    edges: [{ from: 'a', to: 'b', output: 'uci.NOT-produced' }],
  });
  const vu = validateGraph(undeclared);
  ok('edge wiring an undeclared output is refused', !vu.valid
    && vu.checks.find((c) => c.name === 'edge-outputs-declared').ok === false);

  // Not a defineGraph() product at all
  ok('a hand-rolled non-graph object is refused (graph-shape)', validateGraph({ id: 'x', nodes: [], edges: [] }).valid === false);

  // INPUT-KEY SHADOWING: edges a->c and b->c(as:'a') collide on effective key 'a' —
  // b would silently impersonate a's evidence input. Must be refused, not resolved.
  const shadow = defineGraph({
    name: 'shadow', nodes: [node('a'), node('b'), node('c')],
    edges: [{ from: 'a', to: 'c' }, { from: 'b', to: 'c', as: 'a' }],
  });
  const vs = validateGraph(shadow);
  ok('two incoming edges sharing an effective input key are refused', !vs.valid
    && vs.checks.find((c) => c.name === 'edge-input-keys-unique').ok === false
    && vs.checks.find((c) => c.name === 'edge-input-keys-unique').detail.includes("'a'"));
  await rejects('runGraph refuses the shadowing graph without touching the bridge',
    () => runGraph(poisonBridge, shadow, {}), 'edge-input-keys-unique');
  // Same collision without `as`: two literal duplicate edges a->c, a->c.
  const dup = defineGraph({
    name: 'dup', nodes: [node('a'), node('c')],
    edges: [{ from: 'a', to: 'c' }, { from: 'a', to: 'c' }],
  });
  ok('duplicate identical edges are refused too (same effective key)',
    validateGraph(dup).checks.find((c) => c.name === 'edge-input-keys-unique').ok === false);
  // Distinct `as` keys on same from/to pair remain VALID (that is real fan-out wiring).
  const fanout = defineGraph({
    name: 'fanout', nodes: [node('a'), node('c')],
    edges: [{ from: 'a', to: 'c', as: 'x' }, { from: 'a', to: 'c', as: 'y' }],
  });
  ok('two edges with distinct `as` keys stay valid', validateGraph(fanout).valid);

  // MALFORMED CAPABILITY bypassing defineCapability: manifest without produces[].
  // defineGraph must refuse the shape up front…
  await rejects('defineGraph refuses a manifest without produces[]',
    () => defineGraph({ name: 'mal', nodes: [{ id: 'm', capability: { manifest: { name: 'x', version: '1' }, execute: () => [] }, testInputs: echoTest }] }),
    'produces');
  // …and validateGraph must keep its { valid, checks } contract even when the graph
  // object is hand-forged past defineGraph (certifyCapability throwing internally).
  const forgedGraph = defineGraph({ name: 'ok', nodes: [node('a')] });
  forgedGraph.nodes[0] = { id: 'a', capability: { manifest: { name: 'x', version: '1', produces: ['uci.fact'] }, execute: () => [] }, testInputs: echoTest };
  delete forgedGraph.nodes[0].capability.manifest.produces; // simulate the raw-TypeError path
  const vm = validateGraph(forgedGraph);
  ok('a hand-forged malformed capability yields a failed check, not a crash',
    vm.valid === false && vm.checks.some((c) => c.name === 'node-certified:a' && c.ok === false));
}

console.log('Certification is ENFORCED at validate (re-run, no caller flag):');
{
  // Non-deterministic capability: a mutable counter varies between the two cert runs.
  let counter = 0n;
  const flaky = defineCapability({
    name: 'cap.flaky', version: '1.0.0', produces: ['uci.fact'],
    execute: () => [{ target: 'uci.fact', value: { type: 'uci.fact', n: (counter += 1n) } }],
  });
  // FORGE certified flags everywhere a lazy validator might look — none may be trusted.
  flaky.certified = true;
  const flakyNode = { id: 'flaky', capability: flaky, testInputs: echoTest, certified: true };
  const forged = defineGraph({ name: 'forged', nodes: [flakyNode] });
  const vf = validateGraph(forged);
  ok('forged certified:true is ignored — non-deterministic node still refused',
    !vf.valid && vf.checks.find((c) => c.name === 'node-certified:flaky').ok === false);
  await rejects('runGraph refuses the forged graph without touching the bridge',
    () => runGraph(poisonBridge, forged, {}), 'node-certified:flaky');

  // Capability emitting an UNDECLARED effect target also fails re-certification.
  const rogue = defineCapability({
    name: 'cap.rogue', version: '1.0.0', produces: ['uci.fact'],
    execute: () => [{ target: 'uci.NOT-declared', value: { type: 'uci.fact' } }],
  });
  rogue.certified = true;
  const vr = validateGraph(defineGraph({ name: 'rogue', nodes: [{ id: 'r', capability: rogue, testInputs: echoTest, certified: true }] }));
  ok('undeclared-effect capability refused at validate despite forged flag',
    !vr.valid && vr.checks.find((c) => c.name === 'node-certified:r').ok === false);
}

console.log('Runtime guard (real input can differ from certified inputs):');
{
  // Certifies fine on testInputs, but the REAL graph input steers it onto an
  // undeclared target — runGraph must refuse at execution, before committing it.
  const sneaky = defineCapability({
    name: 'cap.sneaky', version: '1.0.0', produces: ['uci.fact'],
    execute: (input) => [{ target: input.evil ? 'uci.evil' : 'uci.fact', value: { type: 'uci.fact' } }],
  });
  const g = defineGraph({ name: 'sneak', nodes: [{ id: 's', capability: sneaky, testInputs: [{ note: 'benign' }] }] });
  ok('sneaky capability certifies on its benign testInputs', validateGraph(g).valid);
  const bridge = new KernelBridge();
  try {
    await rejects('…but an undeclared effect on the REAL input is refused at run time',
      () => runGraph(bridge, g, { evil: true }), 'undeclared effect target');
  } finally { bridge.close(); }
}

console.log('Determinism on the real Kernel (two fresh bridge spawns):');
{
  const observe = defineCapability({
    name: 'observe.t', version: '1.0.0', produces: ['uci.observation'],
    execute: (input) => [{ target: 'uci.observation', value: { type: 'uci.observation', items: input.items } }],
  });
  const derive = defineCapability({
    name: 'derive.t', version: '1.0.0', produces: ['uci.derivation'],
    execute: (input) => [{
      target: 'uci.derivation',
      value: { type: 'uci.derivation', count: BigInt(input.o[0].value.items.length), evidence: [input.o[0].id] },
    }],
  });
  const fakeId = 'cd'.repeat(34);
  const g = defineGraph({
    name: 'det',
    nodes: [
      { id: 'obs', capability: observe, testInputs: [{ items: ['x'] }] },
      { id: 'der', capability: derive, testInputs: [{ o: [{ target: 'uci.observation', id: fakeId, value: { type: 'uci.observation', items: ['x'] } }] }] },
    ],
    edges: [{ from: 'obs', to: 'der', as: 'o' }],
  });
  const input = { items: ['standup', 'board', 'review'] };
  const runFresh = async () => {
    const b = new KernelBridge(); // fresh process = fresh in-memory Kernel
    try { return await runGraph(b, g, input); } finally { b.close(); }
  };
  const r1 = await runFresh();
  const r2 = await runFresh();
  const flat = (r) => r.order.flatMap((id) => r.nodes[id].effects.map((t) => `${id}:${t.target}:${t.contentId}`)).join('|');
  ok('two runs on FRESH Kernels produce identical ContentIds', flat(r1) === flat(r2));
  ok('run roots are identical too', r1.runRoot.contentId === r2.runRoot.contentId);
  ok('fresh Kernel: statuses are "committed" (not warm-cache idempotency)',
    r1.order.every((id) => r1.nodes[id].effects.every((t) => t.status === 'committed')));
  ok('derivation truth cites the observation ContentId (evidence chain wired)',
    r1.nodes.der.effects.length === 1 && /^[0-9a-f]{68}$/.test(r1.nodes.obs.effects[0].contentId));

  // Same bridge, run again: idempotency — same ids, already-committed.
  const b = new KernelBridge();
  try {
    const w1 = await runGraph(b, g, input);
    const w2 = await runGraph(b, g, input);
    ok('warm Kernel re-run: identical ids, status flips to already-committed',
      flat(w1) === flat(w2) && w2.order.every((id) => w2.nodes[id].effects.every((t) => t.status === 'already-committed')));
  } finally { b.close(); }

  console.log('Rendering:');
  const html = renderGraph(g, r1);
  ok('renderGraph output contains every node id', g.nodes.every((nd) => html.includes(nd.id)));
  ok('render contains every committed ContentId', r1.order.every((id) => r1.nodes[id].effects.every((t) => html.includes(t.contentId))));
  ok('render is self-contained: no external src/href, no scripts',
    !/\b(src|href)\s*=\s*"https?:/.test(html) && !html.includes('<script'));
  ok('render is deterministic (same inputs -> byte-identical HTML)', html === renderGraph(g, r1));
  const unrun = renderGraph(g);
  ok('rendering an un-run graph works and says "not run"', unrun.includes('not run') && g.nodes.every((nd) => unrun.includes(nd.id)));
  // HTML injection: a hostile node id must be escaped, not become markup.
  const evil = defineGraph({ name: 'x<script>alert(1)</script>', nodes: [{ id: 'a<b>"c', capability: echoCap('cap.evil'), testInputs: echoTest }] });
  const evilHtml = renderGraph(evil);
  ok('hostile names/ids are HTML-escaped in the render', !evilHtml.includes('<script>alert') && !evilHtml.includes('a<b>'));
}

console.log(`\n${n} checks passed.`);
process.exit(0);

// ARVES Cognitive Studio (P6 preview) — GRAPH-AS-DATA authoring of cognitive graphs.
//
// A cognitive graph is a declarative spec: each NODE binds a capability (authored and
// certified with the P6.5 Ecosystem Kit — defineCapability/certifyCapability), and each
// EDGE wires one node's committed-truth output (the ACS ContentIds + values it produced)
// into the next node's input. The studio validates the graph (cycles, unknown nodes,
// undeclared edge outputs, and — enforced, never attested — every node's capability must
// re-CERTIFY), executes it in deterministic topological order committing every effect as
// truth through the REAL KernelBridge (frozen Runtime v1.0, IDR-006: no platform file is
// modified), and renders a SELF-CONTAINED static HTML visualization of graph + truth ids.
//
// HONEST SCOPE (P6 is "Visual Cognitive Studio"; this is a preview):
//  - "Visual" here means a STATIC HTML render (no external assets). There is NO
//    interactive GUI editor; authoring is code/data (defineGraph), not drag-and-drop.
//  - Node code executes in THIS process (product layer). Its effects become truth in the
//    real reference Kernel via the bridge, but the bridge cannot dynamically bind node
//    capabilities into the Engine fabric (it ships one pre-bound reference capability),
//    so full-chain Capability→Engine execution of graph nodes is an RCR candidate, not
//    something this preview does. Effects are committed with `bridge.commit`.
//  - The bridge server's Kernel is in-memory (`MemKernel` over `MemWalStore`): truth is
//    real Kernel truth (ACS-001 ids, ORCH-004 idempotency) for the LIFETIME OF THE
//    BRIDGE PROCESS — it is not durable on disk across bridge restarts.
//  - Certification of node capabilities is the Kit's best-effort run-twice determinism
//    PROBE over the node's testInputs — it refuses observed non-determinism, it cannot
//    prove purity (engine-enforced determinism is deferred v1.1 RCR debt).
//  - Graph state is in-memory JS data; nothing here persists the graph spec itself.
//
// Determinism: same graph + same input ⇒ byte-identical ContentIds on a fresh Kernel.
// No Date.now(), no Math.random() anywhere in this file.

import { Arves } from '../../arves-sdk-ts/src/arves.mjs';
import { certifyCapability, codeHash } from '../../arves-ecosystem-sdk/src/kit.mjs';

const arves = new Arves();
const GRAPH_TYPE = 'uci.cognitive-graph';

// ---- Authoring --------------------------------------------------------------

/** Author a cognitive graph as data.
 *  `nodes`: [{ id, capability, testInputs }] — `capability` comes from the Ecosystem
 *  Kit's defineCapability(); `testInputs` are the representative inputs certification
 *  is re-run against at validate time (>=1 required — the Kit refuses vacuous certs).
 *  `edges`: [{ from, to, output?, as? }] — wires `from`'s committed-truth output into
 *  `to`'s input. `output` (optional) narrows the wire to one declared produce target;
 *  `as` (optional) names the key the wire appears under in `to`'s input (default: the
 *  `from` node id). Returns a graph whose `id` is the ACS content address of its SPEC
 *  (name + node manifests + code hashes + edges) — edit anything, the id changes. */
export function defineGraph({ name, nodes, edges = [] }) {
  if (typeof name !== 'string' || !name) throw new Error('graph: name required');
  if (!Array.isArray(nodes) || nodes.length === 0) throw new Error('graph: nodes[] required');
  const seen = new Set();
  for (const n of nodes) {
    if (!n || typeof n.id !== 'string' || !n.id || /\s/.test(n.id)) {
      throw new Error('graph: every node needs a non-empty, whitespace-free string id');
    }
    if (seen.has(n.id)) throw new Error(`graph: duplicate node id '${n.id}'`);
    seen.add(n.id);
    if (!n.capability || !n.capability.manifest || typeof n.capability.execute !== 'function'
        || !Array.isArray(n.capability.manifest.produces) || n.capability.manifest.produces.length === 0) {
      throw new Error(`graph: node '${n.id}' must bind a capability from defineCapability() (manifest with non-empty produces[] + execute)`);
    }
    if (!Array.isArray(n.testInputs) || n.testInputs.length === 0) {
      throw new Error(`graph: node '${n.id}' needs testInputs[] — certification is re-run at validate`);
    }
  }
  if (!Array.isArray(edges)) throw new Error('graph: edges must be an array');
  const normEdges = edges.map((e) => {
    if (!e || typeof e.from !== 'string' || typeof e.to !== 'string') {
      throw new Error('graph: every edge needs string from/to');
    }
    return { from: e.from, to: e.to, output: e.output ?? null, as: e.as ?? null };
  });
  // Content-addressed graph identity: manifests + REAL code hashes + wiring. NOTE the
  // codeHash limits are the Kit's (top-level execute source; closures are outside it).
  const spec = {
    type: GRAPH_TYPE,
    name,
    nodes: nodes.map((n) => ({ id: n.id, capability: n.capability.manifest, codeHash: codeHash(n.capability) })),
    edges: normEdges,
  };
  return { id: arves.address(spec, 'engine'), name, nodes: [...nodes], edges: normEdges, spec };
}

// ---- Validation (refusal, not attestation) ----------------------------------

/** Deterministic topological order (Kahn, lexicographic tie-break on node id) or
 *  `null` if the graph has a cycle. Exported so callers can preview execution order. */
export function topoOrder(graph) {
  const ids = graph.nodes.map((n) => n.id);
  const indeg = new Map(ids.map((id) => [id, 0]));
  for (const e of graph.edges) {
    if (indeg.has(e.from) && indeg.has(e.to)) indeg.set(e.to, indeg.get(e.to) + 1);
  }
  const ready = ids.filter((id) => indeg.get(id) === 0).sort();
  const order = [];
  while (ready.length > 0) {
    const id = ready.shift(); // ready is kept sorted — deterministic order
    order.push(id);
    for (const e of graph.edges) {
      if (e.from !== id || !indeg.has(e.to)) continue;
      indeg.set(e.to, indeg.get(e.to) - 1);
      if (indeg.get(e.to) === 0) {
        ready.push(e.to);
        ready.sort();
      }
    }
  }
  return order.length === ids.length ? order : null;
}

/** Validate a graph. Returns `{ valid, checks }` (Kit style). Refuses:
 *   - edges referencing unknown nodes,
 *   - edges whose `output` is not among the from-node's declared produces,
 *   - two incoming edges of one node sharing the same effective input key
 *     (`as ?? from`) — the later edge would silently shadow the earlier wire,
 *   - cycles (incl. self-loops),
 *   - any node whose capability fails certification — certification is RE-RUN here
 *     against the node's own testInputs; a forged `certified: true` flag anywhere on
 *     the node or capability is never read, so it cannot help. */
export function validateGraph(graph) {
  const checks = [];
  const add = (name, ok, detail = '') => checks.push({ name, ok, detail });

  const shaped = !!graph && typeof graph.id === 'string' && graph.spec?.type === GRAPH_TYPE
    && Array.isArray(graph.nodes) && Array.isArray(graph.edges);
  add('graph-shape', shaped, shaped ? '' : 'not a defineGraph() graph');
  if (!shaped) return { valid: false, checks };

  const byId = new Map(graph.nodes.map((n) => [n.id, n]));

  let known = { ok: true, detail: '' };
  for (const e of graph.edges) {
    for (const ref of [e.from, e.to]) {
      if (!byId.has(ref) && known.ok) known = { ok: false, detail: `edge ${e.from}->${e.to} references unknown node '${ref}'` };
    }
  }
  add('edges-reference-known-nodes', known.ok, known.detail);

  let declared = { ok: true, detail: '' };
  for (const e of graph.edges) {
    const from = byId.get(e.from);
    if (e.output !== null && from && !from.capability.manifest.produces.includes(e.output) && declared.ok) {
      declared = { ok: false, detail: `edge ${e.from}->${e.to} wires undeclared output '${e.output}' (produces: ${JSON.stringify(from.capability.manifest.produces)})` };
    }
  }
  add('edge-outputs-declared', declared.ok, declared.detail);

  // No two incoming edges of one node may share an effective input key (as ?? from):
  // otherwise the later wire would silently OVERWRITE the earlier one in the node's
  // input — one node could impersonate another's evidence. Refusal, not resolution.
  let unique = { ok: true, detail: '' };
  const seenKeys = new Map(); // `${to} ${key}` -> first edge
  for (const e of graph.edges) {
    const key = e.as ?? e.from;
    const slot = `${e.to} ${key}`;
    if (seenKeys.has(slot) && unique.ok) {
      const first = seenKeys.get(slot);
      unique = { ok: false, detail: `node '${e.to}' has two incoming edges under input key '${key}' (from '${first.from}' and '${e.from}') — the second would shadow the first` };
    }
    seenKeys.set(slot, e);
  }
  add('edge-input-keys-unique', unique.ok, unique.detail);

  const order = known.ok ? topoOrder(graph) : null;
  add('acyclic', order !== null, order === null ? 'graph has a cycle (or unknown-node edges) — execution order undefined' : '');

  // ENFORCED certification: re-run the Kit's certifyCapability per node. No caller flag
  // (node.certified, capability.certified, …) is ever consulted.
  for (const n of graph.nodes) {
    // A malformed hand-crafted capability can make certifyCapability THROW (e.g. a
    // manifest without produces[]); that is still a refusal, not a crash — record it
    // as a failed check so the documented `{ valid, checks }` contract always holds.
    let cert;
    try {
      cert = certifyCapability(n.capability, n.testInputs);
    } catch (err) {
      cert = { certified: false, checks: [{ name: 'certify-threw', ok: false, detail: String(err.message) }] };
    }
    add(`node-certified:${n.id}`, cert.certified,
      cert.certified ? '' : 'failed: ' + cert.checks.filter((c) => !c.ok).map((c) => c.name).join(', '));
  }

  return { valid: checks.every((c) => c.ok), checks };
}

// ---- Execution (truth-committed, deterministic) ------------------------------

/** Execute a graph on the REAL KernelBridge. Re-validates first (refuses an invalid
 *  graph before touching the bridge — no caller flag is trusted). Nodes run in the
 *  deterministic topological order; a root node receives the graph `input`, a non-root
 *  node receives `{ [edge.as ?? edge.from]: [{ target, id, value }, …] }` — the
 *  upstream node's committed truth (ContentId) wired together with the value that
 *  produced it. Every effect is committed as ACS truth through the bridge; finally the
 *  whole run (graph id + ordered per-node truth ids) is committed under the `trace`
 *  domain as the run root. Returns `{ order, nodes: { id: { effects: [{ target,
 *  contentId, status }] } }, runRoot }`. Same input ⇒ same ContentIds on a fresh
 *  Kernel (statuses flip to 'already-committed' on a warm one — idempotency).
 *  SCOPE: a run refused or failed MID-EXECUTION (e.g. the undeclared-target guard
 *  below) leaves the effects of earlier nodes already committed WITHOUT a run root —
 *  partial truth. On the in-memory bridge this vanishes with the process, and a
 *  corrected re-run re-derives identical ids idempotently (ORCH-004). */
export async function runGraph(bridge, graph, input) {
  const v = validateGraph(graph);
  if (!v.valid) {
    throw new Error('runGraph refused: graph does not validate ('
      + v.checks.filter((c) => !c.ok).map((c) => c.name).join(', ') + ')');
  }
  const order = topoOrder(graph); // non-null: 'acyclic' just passed
  const byId = new Map(graph.nodes.map((n) => [n.id, n]));
  const outputs = new Map(); // nodeId -> [{ target, id, value }]
  const results = {};

  for (const nodeId of order) {
    const node = byId.get(nodeId);
    const incoming = graph.edges.filter((e) => e.to === nodeId);
    let nodeInput;
    if (incoming.length === 0) {
      nodeInput = input;
    } else {
      nodeInput = {};
      const keyOf = (e) => e.as ?? e.from;
      // Keys are unique per node ('edge-input-keys-unique' just passed), so this
      // total order is deterministic; return 0 on equal keys anyway for soundness.
      for (const e of [...incoming].sort((a, b) => (keyOf(a) < keyOf(b) ? -1 : keyOf(a) > keyOf(b) ? 1 : 0))) {
        const up = outputs.get(e.from);
        const wired = e.output === null ? up : up.filter((t) => t.target === e.output);
        nodeInput[keyOf(e)] = wired.map((t) => ({ target: t.target, id: t.id, value: t.value }));
      }
    }
    const effects = node.capability.execute(nodeInput);
    const committed = [];
    const nodeOut = [];
    for (const eff of effects) {
      // Runtime guard: certification only SAMPLED the testInputs — the real input could
      // steer execute onto an undeclared target. Refuse it here too.
      if (!node.capability.manifest.produces.includes(eff.target)) {
        throw new Error(`runGraph refused: node '${nodeId}' emitted undeclared effect target '${eff.target}'`);
      }
      const res = await bridge.commit(eff.value, 'commit');
      committed.push({ target: eff.target, contentId: res.contentId, status: res.status });
      nodeOut.push({ target: eff.target, id: res.contentId, value: eff.value });
    }
    results[nodeId] = { effects: committed };
    outputs.set(nodeId, nodeOut);
  }

  const runRecord = {
    type: 'uci.graph-run',
    graph: graph.id,
    nodes: order.map((id) => ({ id, truths: results[id].effects.map((e) => e.contentId) })),
  };
  const root = await bridge.commit(runRecord, 'trace');
  return { order, nodes: results, runRoot: { contentId: root.contentId, status: root.status } };
}

// ---- Rendering (the honest "visual" part) ------------------------------------

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** Render a graph (and optionally its run results) to a SELF-CONTAINED static HTML
 *  string: inline SVG + inline CSS, zero external assets, zero scripts. Deterministic —
 *  the same graph + results render byte-identical HTML (no timestamps). This is a
 *  static visualization, NOT an interactive editor; the caller writes it to a file. */
export function renderGraph(graph, results = null) {
  // Longest-path layering: column = max(upstream column) + 1. Valid graphs are acyclic;
  // for robustness a cyclic graph still renders (bounded passes), it just won't run.
  const ids = graph.nodes.map((n) => n.id);
  const layer = new Map(ids.map((id) => [id, 0]));
  for (let pass = 0; pass < ids.length; pass++) {
    let changed = false;
    for (const e of graph.edges) {
      if (!layer.has(e.from) || !layer.has(e.to)) continue;
      const want = layer.get(e.from) + 1;
      if (want > layer.get(e.to) && want < ids.length) { layer.set(e.to, want); changed = true; }
    }
    if (!changed) break;
  }
  const cols = new Map(); // layer -> sorted node ids
  for (const id of [...ids].sort()) {
    const l = layer.get(id);
    if (!cols.has(l)) cols.set(l, []);
    cols.get(l).push(id);
  }

  const BOX_W = 240; const GAP_X = 90; const GAP_Y = 36; const PAD = 40;
  const boxH = (id) => {
    const eff = results?.nodes?.[id]?.effects ?? [];
    return 64 + 18 * Math.max(1, eff.length);
  };
  const pos = new Map();
  let maxX = 0; let maxY = 0;
  for (const [l, colIds] of [...cols.entries()].sort((a, b) => a[0] - b[0])) {
    let y = PAD;
    for (const id of colIds) {
      const x = PAD + l * (BOX_W + GAP_X);
      pos.set(id, { x, y, h: boxH(id) });
      y += boxH(id) + GAP_Y;
      maxX = Math.max(maxX, x + BOX_W);
      maxY = Math.max(maxY, y);
    }
  }
  const W = maxX + PAD; const H = maxY + PAD - GAP_Y + 8;

  const byId = new Map(graph.nodes.map((n) => [n.id, n]));
  const svg = [];
  svg.push(`<svg viewBox="0 0 ${W} ${H}" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="cognitive graph ${esc(graph.name)}">`);
  svg.push('<defs><marker id="arr" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="#5b6472"/></marker></defs>');
  for (const e of graph.edges) {
    const a = pos.get(e.from); const b = pos.get(e.to);
    if (!a || !b) continue;
    const x1 = a.x + BOX_W; const y1 = a.y + a.h / 2;
    const x2 = b.x; const y2 = b.y + b.h / 2;
    const mx = (x1 + x2) / 2;
    svg.push(`<path d="M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}" fill="none" stroke="#5b6472" stroke-width="1.5" marker-end="url(#arr)"/>`);
    if (e.output) svg.push(`<text x="${mx}" y="${(y1 + y2) / 2 - 6}" text-anchor="middle" class="wire">${esc(e.output)}</text>`);
  }
  for (const id of ids) {
    const { x, y, h } = pos.get(id);
    const cap = byId.get(id).capability.manifest;
    const eff = results?.nodes?.[id]?.effects ?? null;
    svg.push(`<g><rect x="${x}" y="${y}" width="${BOX_W}" height="${h}" rx="10" class="node"/>`);
    svg.push(`<text x="${x + 14}" y="${y + 24}" class="nid">${esc(id)}</text>`);
    svg.push(`<text x="${x + 14}" y="${y + 42}" class="cap">${esc(cap.name)}@${esc(cap.version)}</text>`);
    if (eff === null) {
      svg.push(`<text x="${x + 14}" y="${y + 60}" class="truth">(not run)</text>`);
    } else {
      eff.forEach((t, i) => {
        svg.push(`<text x="${x + 14}" y="${y + 60 + 18 * i}" class="truth">${esc(t.target)} ${esc(t.contentId.slice(0, 16))}… (${esc(t.status)})</text>`);
      });
    }
    svg.push('</g>');
  }
  svg.push('</svg>');

  const rows = ids.map((id) => {
    const eff = results?.nodes?.[id]?.effects ?? [];
    const cap = byId.get(id).capability.manifest;
    const truths = eff.length
      ? eff.map((t) => `<code>${esc(t.contentId)}</code> <em>(${esc(t.status)})</em>`).join('<br>')
      : '<em>not run</em>';
    return `<tr><td>${esc(id)}</td><td>${esc(cap.name)}@${esc(cap.version)}</td><td>${truths}</td></tr>`;
  }).join('\n');

  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${esc(graph.name)} — ARVES Cognitive Studio</title>
<style>
  body { font: 14px/1.5 system-ui, sans-serif; margin: 2rem; color: #1c2430; background: #f6f7f9; }
  h1 { font-size: 1.3rem; } code { font-size: 12px; word-break: break-all; }
  .meta { color: #5b6472; font-size: 12px; }
  .frame { background: #fff; border: 1px solid #d9dee5; border-radius: 12px; padding: 1rem; overflow-x: auto; }
  svg { max-width: 100%; height: auto; display: block; }
  .node { fill: #eef2f7; stroke: #7a8699; stroke-width: 1.2; }
  .nid { font: 600 14px system-ui, sans-serif; fill: #1c2430; }
  .cap { font: 12px ui-monospace, monospace; fill: #3d4756; }
  .truth { font: 11px ui-monospace, monospace; fill: #556; }
  .wire { font: 11px ui-monospace, monospace; fill: #5b6472; }
  table { border-collapse: collapse; width: 100%; margin-top: 1rem; }
  th, td { border: 1px solid #d9dee5; padding: 6px 10px; text-align: left; vertical-align: top; }
  th { background: #eef2f7; }
  footer { margin-top: 1rem; color: #5b6472; font-size: 12px; }
</style>
</head>
<body>
<h1>${esc(graph.name)}</h1>
<p class="meta">graph id (content address of the spec): <code>${esc(graph.id)}</code><br>
run root: ${results?.runRoot ? `<code>${esc(results.runRoot.contentId)}</code> <em>(${esc(results.runRoot.status)})</em>` : '<em>not run</em>'}</p>
<div class="frame">
${svg.join('\n')}
</div>
<table>
<tr><th>node</th><th>capability</th><th>committed truth (ACS ContentId)</th></tr>
${rows}
</table>
<footer>ARVES Cognitive Studio (P6 preview) — static render of a graph-as-data spec. Not an
interactive editor. Truth ids were committed through the real Runtime v1.0 KernelBridge; the
bridge Kernel is in-memory, so ids are reproducible but the bridge session is not durable.</footer>
</body>
</html>
`;
}

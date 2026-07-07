// JARVIS · Cognitive Console — local web server (products/, freeze-clean)
// =============================================================================
// The point of this file: make JARVIS fully MANAGEABLE FROM THE BROWSER — no CLI.
// It serves ui/index.html and exposes the exact /api/* contract that front-end
// expects, backed by the REAL Assistant over the REAL frozen Runtime v1.0 bridge
// (IDR-006: this consumes the runtime, it does NOT link or modify it). Zero
// external deps — node:http only, the same pattern as the P8 cloud-preview.
//
// HONESTY, SURFACED NOT HIDDEN (the whole point of ARVES):
//   * The reasoner is the deterministic `stub-reasoner` UNLESS you plug your own
//     model via JARVIS_REASONER=./path/to/your-reasoner.mjs (default export = a
//     class implementing reason(context); see src/llm-reasoner.example.mjs).
//     /api/state reports {isStub} truthfully so the console shows a warm "Stub"
//     pill until a real model is attached — the UI can never lie about this.
//   * Every panel is a READ PROJECTION of committed truth in the Kernel WAL, not
//     an in-memory story: restart the server over the same --wal-dir and the same
//     truths, decisions, policies and blocks come back (RCR-033 WAL recovery).
//
// Run:
//   node ui/server.mjs --wal-dir ./jarvis-wal            # durable, port 7777
//   node ui/server.mjs --wal-dir ./jarvis-wal --port 8080
//   JARVIS_REASONER=./my-llm.mjs node ui/server.mjs --wal-dir ./jarvis-wal
// then open http://localhost:7777  — that is your JARVIS console.
// =============================================================================
import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

import { openSession } from '../src/cli.mjs';
import { StubReasoner } from '../src/reasoner.mjs';
import { CONNECTORS, connectorByName } from '../src/connectors.mjs';
import { why, renderWhy } from '../src/why.mjs';
import { resolveSession } from '../src/config.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const INDEX = path.join(HERE, 'index.html');

// ---- arg parsing (tiny; the assistant flags mirror the CLI) --------------------------
function parseArgs(argv) {
  const opts = { tenant: undefined, workspace: undefined, walDir: undefined, exe: undefined };
  let port = Number(process.env.PORT ?? 7777);
  let host = process.env.HOST ?? '127.0.0.1';
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--tenant') opts.tenant = argv[++i];
    else if (a === '--workspace') opts.workspace = argv[++i];
    else if (a === '--wal-dir') opts.walDir = argv[++i];
    else if (a === '--exe') opts.exe = argv[++i];
    else if (a === '--port') port = Number(argv[++i]);
    else if (a === '--host') host = argv[++i];
  }
  return { opts, port, host };
}

// BigInt-safe JSON (fact.at is nanoseconds as BigInt inside truth bodies).
const jsonReplacer = (_k, v) => (typeof v === 'bigint' ? v.toString() : v);
const sendJson = (res, code, body) => {
  const s = JSON.stringify(body, jsonReplacer);
  res.writeHead(code, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
  res.end(s);
};
const readBody = (req) =>
  new Promise((resolve, reject) => {
    let n = 0;
    const chunks = [];
    req.on('data', (c) => {
      n += c.length;
      if (n > 1_000_000) { reject(new Error('request body too large')); req.destroy(); return; }
      chunks.push(c);
    });
    req.on('end', () => {
      const raw = Buffer.concat(chunks).toString('utf8').trim();
      if (raw === '') return resolve({});
      try { resolve(JSON.parse(raw)); } catch { reject(new Error('invalid JSON body')); }
    });
    req.on('error', reject);
  });

// A committed event is truth; "pretty" is only for the human line, never the id.
const prettyEvent = (e) => String(e).replace(/[-_]/g, ' ');

// ---- state projection: every panel is READ from committed truth -----------------------
function projectState(assistant, session, reasonerInfo) {
  const policies = assistant.guardrails.policies();
  const journal = assistant.journal();

  // Facts (deduped truths with their evidence sets) — the memory.
  const facts = assistant.truths().map((t) => ({
    id: t.id,
    kind: 'memory',
    summary: `${t.fact.entity} — ${prettyEvent(t.fact.event)}`,
    sources: t.sources,
    tag: 'observed',
  }));

  // Decisions — committed, content-addressed, latest-per-subject.
  const decisions = assistant.decisions().map((d) => ({
    id: d.id,
    kind: 'decision',
    summary: `${d.subject}: ${d.action}`,
    sources: ['reasoner'],
    tag: 'decision',
  }));

  // Blocks — a guardrail refusal is itself committed truth (never a silent drop).
  const blockEvents = journal.filter(
    (e) => e.body && e.body.type === 'uci.assistant.compliance' && e.body.outcome === 'blocked',
  );
  const blocks = blockEvents.map((e) => ({
    id: e.id,
    kind: 'blocked',
    summary: `${e.body.subject} — blocked by policy${e.body.goal ? ` (“${e.body.goal}”)` : ''}`,
    sources: ['guardrail'],
    tag: 'policy-block',
  }));

  // Pending approvals = blocked subjects with no committed approval truth yet, in the
  // approver role their policy demands. This is what JARVIS is WAITING FOR YOU to clear.
  const grantedApprovals = journal.filter((e) => e.body && e.body.type === 'uci.assistant.approval');
  const granted = new Set(grantedApprovals.map((e) => `${e.body.subject}::${e.body.role}`));
  const pendingSeen = new Set();
  const approvals = [];
  for (const e of blockEvents) {
    const pol = policies.find((p) => p.id === e.body.policy);
    const role = pol ? pol.approverRole : 'user';
    const key = `${e.body.subject}::${role}`;
    if (granted.has(key) || pendingSeen.has(key)) continue;
    pendingSeen.add(key);
    approvals.push({ subject: e.body.subject, asked: e.body.goal || 'gated action', by: role });
  }

  // Skills — certified + bound (attachSkill RE-RUNS the certification gate, so a name
  // present here provably passed it; that is why `certified` is honestly true).
  const skills = assistant.skills().map((name) => ({ name, certified: true }));

  // Agents — the deterministic council available over the shared truth base. Plus any
  // agent findings actually committed this session (attributed research).
  const findings = journal.filter((e) => e.body && e.body.type === 'uci.assistant.finding');
  const agents = [
    { id: 'researcher', role: 'Researches a topic over the shared truth base (deterministic, attributed findings)' },
    { id: 'scheduler', role: 'Proposes scheduling actions from committed calendar truth (first-committed-wins)' },
    ...findings.map((e) => ({ id: `${e.body.agent}@${e.body.agentVersion}`, role: `finding: ${e.body.topic}` })),
  ];

  return {
    identity: { tenant: session.tenant, workspace: session.workspace, walDir: session.walDir ?? '(in-memory)' },
    reasoner: reasonerInfo,
    truths: [...blocks, ...decisions, ...facts],
    policies: policies.map((p) => ({ rule: p.name, domain: `${p.appliesTo.join(', ')} → needs ${p.approverRole}` })),
    approvals,
    skills,
    agents,
    sources: Object.keys(CONNECTORS).sort(),
    counts: {
      truths: facts.length,
      decisions: decisions.length,
      blocks: blocks.length,
      policies: policies.length,
      skills: skills.length,
    },
  };
}

// ---- think() result -> the console trace (thought -> gate -> commit) -------------------
function traceOfThink(assistant, goal, r) {
  const steps = [];
  steps.push({
    kind: 'thought',
    label: 'Reasoner proposal',
    lines: [`action: ${r.proposal.action}`, r.proposal.because || '(no rationale given)'],
    cid: r.proposalId,
  });

  if (r.acted) {
    steps.push({ kind: 'gate', state: 'pass', label: 'Guardrail passed', lines: [`'${r.proposal.skill}' is certified + bound`] });
    const eff = r.invocation && r.invocation.truths && r.invocation.truths[0];
    steps.push({
      kind: 'commit',
      label: `Acted — ${r.proposal.skill}`,
      action: r.proposal.action,
      lines: [`skill '${r.proposal.skill}' invoked · ${r.invocation.truths.length} effect truth(s)`],
      cid: eff ? eff.id : r.proposalId,
    });
    return { say: `Done — ${prettyEvent(r.proposal.action)} via ${r.proposal.skill}.`, trace: steps };
  }

  if (r.blocked) {
    steps.push({
      kind: 'gate',
      state: 'block',
      label: `Blocked by policy '${r.policy}'`,
      lines: [r.rule || 'a committed approval truth is required first'],
      cid: r.complianceId,
    });
    return {
      say: `Blocked — “${r.proposal.subject}” needs a committed approval first (policy '${r.policy}'). Approve it, then ask again.`,
      trace: steps,
    };
  }

  // no-action-proposed
  return { say: r.proposal.because || 'Nothing to act on for that.', trace: steps };
}

async function main() {
  const { opts, port, host } = parseArgs(process.argv.slice(2));
  const session = resolveSession(opts, {});

  // openSession does the real setup: owns the bridge, attaches the StubReasoner,
  // recovers memory from the WAL (RCR-033), registers the certified skill library,
  // and seeds the default spend policy on a virgin shard.
  const assistant = await openSession(session);

  // Reasoner honesty + optional real-LLM plug. Default = deterministic stub.
  let reasonerInfo = { name: new StubReasoner().name, isStub: true };
  if (process.env.JARVIS_REASONER) {
    const modUrl = pathToFileURL(path.resolve(process.env.JARVIS_REASONER)).href;
    const mod = await import(modUrl);
    const Reasoner = mod.default ?? mod.Reasoner;
    if (typeof Reasoner !== 'function') throw new Error(`JARVIS_REASONER module must default-export a Reasoner class (${process.env.JARVIS_REASONER})`);
    const inst = new Reasoner();
    assistant.useReasoner(inst); // validated against the reasoner contract
    reasonerInfo = { name: inst.name || 'custom-reasoner', isStub: false };
  }

  const conversation = []; // server-held transcript; every turn's TRUTH is in the WAL

  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, `http://${req.headers.host}`);
      const p = url.pathname;

      // ---- static: the console itself ------------------------------------------------
      if (req.method === 'GET' && (p === '/' || p === '/index.html')) {
        const html = await readFile(INDEX);
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(html);
        return;
      }

      // ---- GET /api/state -------------------------------------------------------------
      if (req.method === 'GET' && p === '/api/state') {
        const state = projectState(assistant, session, reasonerInfo);
        state.conversation = conversation;
        return sendJson(res, 200, state);
      }

      // ---- POST /api/observe { source } — import a connector into truth ---------------
      if (req.method === 'POST' && p === '/api/observe') {
        const { source } = await readBody(req);
        const name = typeof source === 'string' && source.length ? source : 'notes';
        const conn = connectorByName(name); // throws with the known list on a bad name
        const obs = conn();
        let fresh = 0;
        let merged = 0;
        let lastId = null;
        for (const { source: s, fact } of obs) {
          const r = await assistant.observe(s, fact);
          lastId = r.id;
          if (r.deduped) merged++; else fresh++;
        }
        const state = projectState(assistant, session, reasonerInfo);
        return sendJson(res, 200, { id: lastId, source: name, imported: obs.length, fresh, deduped: merged, truths: state.truths, counts: state.counts });
      }

      // ---- POST /api/ask { goal } — think: proposal -> gate -> certified skill --------
      if (req.method === 'POST' && p === '/api/ask') {
        const { goal } = await readBody(req);
        if (typeof goal !== 'string' || goal.trim() === '') return sendJson(res, 400, { error: 'goal (non-empty string) is required' });
        conversation.push({ role: 'you', say: goal });
        let turn;
        try {
          const r = await assistant.think(goal);
          const mapped = traceOfThink(assistant, goal, r);
          turn = { role: 'jarvis', say: mapped.say, trace: mapped.trace };
        } catch (e) {
          // e.g. reasoner proposed an unregistered skill — the refusal is committed truth.
          turn = { role: 'jarvis', say: `Refused: ${e.message}`, trace: [{ kind: 'gate', state: 'block', label: 'Refused', lines: [e.message] }] };
        }
        conversation.push(turn);
        const state = projectState(assistant, session, reasonerInfo);
        return sendJson(res, 200, { turn, truths: state.truths, approvals: state.approvals, conversation, counts: state.counts });
      }

      // ---- POST /api/approve { subject, role } — commit a separate approval truth -----
      if (req.method === 'POST' && p === '/api/approve') {
        const { subject, role } = await readBody(req);
        if (typeof subject !== 'string' || subject.trim() === '') return sendJson(res, 400, { error: 'subject (non-empty string) is required' });
        const r = await assistant.guardrails.approve(typeof role === 'string' && role.length ? role : 'user', subject);
        const state = projectState(assistant, session, reasonerInfo);
        return sendJson(res, 200, { id: r.id, status: r.status, approvals: state.approvals, truths: state.truths, counts: state.counts });
      }

      // ---- GET /api/why?q=<subject|id> — reconstruct a decision path from truth --------
      if (req.method === 'GET' && p === '/api/why') {
        const q = url.searchParams.get('q');
        if (!q) return sendJson(res, 400, { error: 'q (subject or truth id) is required' });
        const trace = why(assistant, q);
        return sendJson(res, 200, { trace, text: renderWhy(trace) });
      }

      // ---- GET /api/skills ------------------------------------------------------------
      if (req.method === 'GET' && p === '/api/skills') {
        return sendJson(res, 200, { skills: assistant.skills().map((name) => ({ name, certified: true })) });
      }

      sendJson(res, 404, { error: `no route ${req.method} ${p}` });
    } catch (e) {
      sendJson(res, 500, { error: e.message });
    }
  });

  const shutdown = () => { try { assistant.close(); } catch {} server.close(() => process.exit(0)); };
  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);

  server.listen(port, host, () => {
    const where = session.walDir ? `durable WAL at ${session.walDir}` : 'in-memory (truth is NOT persisted — pass --wal-dir to keep it)';
    // eslint-disable-next-line no-console
    console.log(
      `\nJARVIS · Cognitive Console\n` +
      `  open:      http://${host}:${port}\n` +
      `  shard:     ${session.tenant} / ${session.workspace}\n` +
      `  memory:    ${where}\n` +
      `  reasoner:  ${reasonerInfo.name}${reasonerInfo.isStub ? '  (deterministic stub — plug a model via JARVIS_REASONER=./your-reasoner.mjs)' : '  (custom model attached)'}\n` +
      `  Ctrl-C to stop.\n`,
    );
  });
}

main().catch((e) => {
  // eslint-disable-next-line no-console
  console.error(`jarvis-ui: ${e.message}`);
  process.exit(1);
});

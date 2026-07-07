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
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';

import { openSession } from '../src/cli.mjs';
import { StubReasoner } from '../src/reasoner.mjs';
import OpenAiReasoner from '../src/openai-reasoner.mjs';
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
// Deterministic subject slug for a goal title (content-addressed subject, no clock/random).
const slug = (s) => String(s).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '').slice(0, 48) || 'goal';
// A goal's subject is a readable slug PLUS a hash of the exact title, so DISTINCT titles never
// collide (slug alone maps "Ship it" / "Ship it!" / "SHIP IT" — and every non-ASCII title —
// to the same string). Deterministic: same title -> same subject (replay-stable).
const goalSubject = (title) => `goal:${slug(title)}-${createHash('sha256').update(title).digest('hex').slice(0, 8)}`;
const GOAL_STATES = ['active', 'blocked', 'done'];

// ---- state projection: every panel is READ from committed truth -----------------------
function projectState(assistant, session, reasonerInfo) {
  const policies = assistant.guardrails.policies();
  const journal = assistant.journal();

  // Facts (deduped truths with their evidence sets) — the memory.
  const facts = assistant.truths().map((t) => ({
    id: t.id,
    kind: 'memory',
    entity: t.fact.entity,
    event: prettyEvent(t.fact.event),
    summary: `${t.fact.entity} — ${prettyEvent(t.fact.event)}`,
    sources: t.sources,
    tag: 'observed',
  }));

  // ENTITIES — JARVIS's primary objects (People / Projects / Things / You). Truth is the
  // evidence BEHIND these; the user thinks in entities, not truths. The type is a HONEST
  // heuristic from the entity string (labelled as such in the UI), never invented data —
  // every entity here is one that actually appears in a committed observation.
  const classify = (e) => {
    if (e === 'urn:you' || e === session.tenant || e === `urn:${session.tenant}`) return 'You';
    if (/@/.test(e)) return 'People';
    if (/^(proj|project|repo)[:\/]/i.test(e)) return 'Projects';
    if (/^(person|user|contact)[:\/]/i.test(e)) return 'People';
    return 'Things';
  };
  const entMap = new Map();
  for (const t of facts) {
    const e = t.entity; if (!e) continue;
    if (!entMap.has(e)) entMap.set(e, { name: e, type: classify(e), truths: 0, sources: new Set() });
    const x = entMap.get(e); x.truths += 1; (t.sources || []).forEach((s) => x.sources.add(s));
  }
  const entities = [...entMap.values()]
    .map((x) => ({ name: x.name, type: x.type, truths: x.truths, sources: [...x.sources].sort() }))
    .sort((a, b) => (b.truths - a.truths) || (a.name < b.name ? -1 : 1));

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

  // Governed failures are truth too (a real reasoner supplied bad input / non-canonical
  // input). BOTH modes surface, so every committed compliance truth is a durable read
  // projection (consistent across a WAL restart), never only a transient chat line.
  const failures = journal
    .filter((e) => e.body && e.body.type === 'uci.assistant.compliance'
      && (e.body.outcome === 'skill-execution-failed' || e.body.outcome === 'proposal-rejected'))
    .map((e) => ({
      id: e.id,
      kind: 'blocked',
      summary: e.body.outcome === 'proposal-rejected'
        ? `${e.body.skill || 'proposal'} — rejected before the gate${e.body.goal ? ` (“${e.body.goal}”)` : ''}`
        : `${e.body.skill} — execution failed${e.body.goal ? ` (“${e.body.goal}”)` : ''}`,
      sources: ['runtime'],
      tag: e.body.outcome === 'proposal-rejected' ? 'proposal-rejected' : 'skill-failed',
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

  // Skills — certified + bound, with real detail (version/produces/risk-class from the
  // entry) + a run count from the journal (its "logs"). certified is honestly true because
  // attachSkill RE-RUNS the gate.
  const skillRuns = {};
  journal.forEach((e) => { if (e.meta && e.meta.via === 'invoke' && e.meta.capability) skillRuns[e.meta.capability] = (skillRuns[e.meta.capability] || 0) + 1; });
  const skills = assistant.skillsDetailed().map((s) => ({ ...s, certified: true, runs: skillRuns[s.name] || 0 }));

  // Agents — the deterministic council available over the shared truth base. Plus any
  // agent findings actually committed this session (attributed research).
  const findings = journal.filter((e) => e.body && e.body.type === 'uci.assistant.finding');
  const agents = [
    { id: 'researcher', role: 'Researches a topic over the shared truth base (deterministic, attributed findings)' },
    { id: 'scheduler', role: 'Proposes scheduling actions from committed calendar truth (first-committed-wins)' },
    ...findings.map((e) => ({ id: `${e.body.agent}@${e.body.agentVersion}`, role: `finding: ${e.body.topic}` })),
  ];

  // Timeline — the decision journal mapped to human cognition events, in commit order.
  // This is the "replay feel": every real thing JARVIS did, each a checkable truth id.
  // (No wall-clock: truth carries no clock by design — order is the commit sequence.)
  const short = (id) => (id ? id.slice(0, 10) + '…' : '');
  const eventOf = (e) => {
    const b = e.body || {};
    const m = e.meta || {};
    if (m.via === 'invoke') return { kind: 'act', label: `Acted — ${m.capability}`, detail: m.target || '' };
    switch (b.type) {
      case 'uci.assistant.fact': return { kind: 'observe', label: 'Observed', detail: `${b.entity} · ${prettyEvent(b.event)}` };
      case 'uci.assistant.attestation': return null; // evidence — folded into its fact
      case 'uci.assistant.proposal': return { kind: 'think', label: `Reasoned · ${b.reasoner}`, detail: b.action === 'invoke-skill' ? `proposed ${b.skill}` : 'no action proposed' };
      case 'uci.assistant.compliance':
        if (b.outcome === 'blocked') return { kind: 'gate', label: 'Blocked by policy', detail: b.subject || '' };
        if (b.outcome === 'skill-execution-failed') return { kind: 'gate', label: 'Skill execution failed', detail: b.skill || '' };
        if (b.outcome === 'proposal-rejected') return { kind: 'gate', label: 'Proposal rejected', detail: b.skill || '' };
        if (b.outcome === 'refused-unregistered-skill') return { kind: 'gate', label: 'Refused — unregistered skill', detail: b.skill || '' };
        return { kind: 'gate', label: 'Compliance event', detail: b.outcome || '' };
      case 'uci.assistant.approval': return { kind: 'approve', label: 'Approval granted', detail: `${b.role} → ${b.subject}` };
      case 'uci.assistant.policy': return { kind: 'policy', label: 'Policy committed', detail: b.name || '' };
      case 'uci.assistant.skill': return { kind: 'admit', label: 'Skill certified', detail: b.name || '' };
      case 'uci.assistant.decision': return { kind: 'decide', label: 'Decision recorded', detail: `${b.subject}: ${b.action}` };
      case 'uci.assistant.finding': return { kind: 'agent', label: `${b.agent} researched`, detail: b.topic || '' };
      case 'uci.assistant.resolution': return { kind: 'conflict', label: 'Conflict resolved', detail: `winner ${short(b.winner)}` };
      default: return { kind: 'commit', label: 'Committed truth', detail: b.type || e.domain };
    }
  };
  const timeline = journal.map((e) => { const v = eventOf(e); return v ? { seq: e.seq, id: e.id, status: e.status, ...v } : null; }).filter(Boolean);
  const conflicts = journal.filter((e) => e.body && e.body.type === 'uci.assistant.resolution').length;

  // Goals — a goal is a committed truth (uci.assistant.goal); its status is a later truth
  // (uci.assistant.goal-status, latest wins). "related" = observations whose entity/event
  // contains a title keyword — an HONEST keyword association, not a computed % (there is no
  // milestone truth to derive a percentage from, so we don't invent one).
  const goalMap = new Map();
  const goalStatus = new Map();
  for (const e of journal) {
    if (!e.body) continue;
    if (e.body.type === 'uci.assistant.goal') goalMap.set(e.body.subject, { subject: e.body.subject, title: e.body.title, id: e.id });
    else if (e.body.type === 'uci.assistant.goal-status') goalStatus.set(e.body.subject, e.body.status);
  }
  const goals = [...goalMap.values()].map((g) => {
    const terms = String(g.title).toLowerCase().split(/[^a-z0-9]+/).filter((w) => w.length > 3);
    const related = terms.length
      ? facts.filter((t) => terms.some((term) => t.summary.toLowerCase().includes(term))).length
      : 0;
    return { subject: g.subject, title: g.title, id: g.id, status: goalStatus.get(g.subject) || 'active', related };
  });

  return {
    identity: { tenant: session.tenant, workspace: session.workspace, walDir: session.walDir ?? '(in-memory)' },
    reasoner: reasonerInfo,
    truths: [...failures, ...blocks, ...decisions, ...facts],
    policies: policies.map((p) => ({ rule: p.name, domain: `${p.appliesTo.join(', ')} → needs ${p.approverRole}` })),
    approvals,
    skills,
    agents,
    goals,
    entities,
    timeline,
    sources: Object.keys(CONNECTORS).sort(),
    counts: {
      truths: facts.length,
      decisions: decisions.length,
      blocks: blocks.length,
      conflicts,
      goals: goals.length,
      policies: policies.length,
      skills: skills.length,
      observations: facts.length,
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

  if (r.failed && r.stage === 'proposal-rejected') {
    // The proposal was rejected BEFORE the gate ran and before any skill executed (e.g. the
    // model supplied non-canonical input). Do NOT claim the guardrail passed or the skill ran.
    steps.push({
      kind: 'gate',
      state: 'block',
      label: 'Proposal rejected (before the gate)',
      lines: [r.error || 'the reasoner’s proposed input was not valid', 'the guardrail never ran; no skill executed'],
      cid: r.complianceId,
    });
    return {
      say: `I couldn't even record that proposal — the input I produced was invalid (${r.error}). It never reached the guardrail or a skill; the rejection itself is recorded as truth.`,
      trace: steps,
    };
  }

  if (r.failed) {
    // stage === 'skill-execution': the gate DID pass and the certified skill DID run, but its
    // execute() threw. Governed: committed as a compliance truth, no effect committed.
    steps.push({ kind: 'gate', state: 'pass', label: 'Guardrail passed', lines: [`'${r.proposal.skill}' is certified + bound`] });
    steps.push({
      kind: 'gate',
      state: 'block',
      label: `Skill execution failed — ${r.proposal.skill}`,
      lines: [r.error || 'the skill could not run on the proposed input'],
      cid: r.complianceId,
    });
    return {
      say: `I proposed ${r.proposal.skill} and it passed the guardrail, but it couldn't run on the input I gave it (${r.error}). No effect was committed — the failure itself is recorded as truth.`,
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

  // Reasoner honesty + real-LLM plug. Default = deterministic stub. Resolution order,
  // logged loudly so the choice is explicit, never magic:
  //   1. JARVIS_REASONER=./module.mjs  — attach that module's default-exported Reasoner.
  //   2. else OPENAI_API_KEY is set     — attach the shipped OpenAI reasoner (src/openai-
  //      reasoner.mjs), model from OPENAI_MODEL (default gpt-4o-mini). The key is read
  //      from the env by the reasoner at call time; the server never touches it.
  //   3. else                          — the deterministic stub (honestly reported).
  let reasonerInfo = { name: new StubReasoner().name, isStub: true };
  let reasonerModulePath = null;
  let autoOpenAi = false;
  if (process.env.JARVIS_REASONER) {
    reasonerModulePath = path.resolve(process.env.JARVIS_REASONER);
  } else if (process.env.OPENAI_API_KEY) {
    reasonerModulePath = path.join(HERE, '..', 'src', 'openai-reasoner.mjs'); // shipped, zero-dep
    autoOpenAi = true;
  }
  if (reasonerModulePath) {
    const mod = await import(pathToFileURL(reasonerModulePath).href);
    const Reasoner = mod.default ?? mod.Reasoner;
    if (typeof Reasoner !== 'function') throw new Error(`reasoner module must default-export a Reasoner class (${reasonerModulePath})`);
    const inst = new Reasoner();
    assistant.useReasoner(inst); // validated against the reasoner contract
    reasonerInfo = { name: inst.name || 'custom-reasoner', isStub: false };
    if (autoOpenAi) {
      // Loud, unmissable: a BILLED model was attached solely because OPENAI_API_KEY is present
      // in the environment. No call happens until you ask, but every ask then bills OpenAI.
      // eslint-disable-next-line no-console
      console.log(
        `\n  ⚠  OPENAI_API_KEY detected → attached BILLED model '${reasonerInfo.name}' as the reasoner.\n` +
        `     Every /ask calls the OpenAI API (billed by OpenAI, not ARVES). To change:\n` +
        `       • unset OPENAI_API_KEY to fall back to the free deterministic stub, or\n` +
        `       • set JARVIS_REASONER=./your-reasoner.mjs to use a different model.`,
      );
    }
  }

  const conversation = []; // server-held transcript; every turn's TRUTH is in the WAL

  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, `http://${req.headers.host}`);
      const p = url.pathname;

      // ---- static: the console itself ------------------------------------------------
      if (req.method === 'GET' && (p === '/' || p === '/index.html')) {
        const html = await readFile(INDEX);
        res.writeHead(200, {
          'content-type': 'text/html; charset=utf-8',
          'cache-control': 'no-store',
          // Defense-in-depth behind the attribute-safe escaper: no external anything, and
          // crucially connect-src/img-src 'self' so that even if a payload ever executed it
          // could not exfiltrate memory to an outside origin. Same-origin /api/* still works.
          'content-security-policy': "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
          'x-content-type-options': 'nosniff',
        });
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

      // ---- POST /api/goals { title } — create a goal as committed truth -----------------
      if (req.method === 'POST' && p === '/api/goals') {
        const { title } = await readBody(req);
        if (typeof title !== 'string' || title.trim() === '') return sendJson(res, 400, { error: 'a goal title is required' });
        const cleanTitle = title.trim().slice(0, 200);
        const subject = goalSubject(cleanTitle);
        const r = await assistant.commitTruth({ type: 'uci.assistant.goal', subject, title: cleanTitle }, 'trace');
        const state = projectState(assistant, session, reasonerInfo);
        return sendJson(res, 200, { id: r.contentId, subject, goals: state.goals, counts: state.counts });
      }

      // ---- POST /api/goals/status { subject, status } — status is a LATER truth ---------
      if (req.method === 'POST' && p === '/api/goals/status') {
        const { subject, status } = await readBody(req);
        if (typeof subject !== 'string' || subject.trim() === '') return sendJson(res, 400, { error: 'a goal subject is required' });
        if (!GOAL_STATES.includes(status)) return sendJson(res, 400, { error: `status must be one of: ${GOAL_STATES.join(', ')}` });
        const r = await assistant.commitTruth({ type: 'uci.assistant.goal-status', subject: subject.trim(), status }, 'trace');
        const state = projectState(assistant, session, reasonerInfo);
        return sendJson(res, 200, { id: r.contentId, goals: state.goals, counts: state.counts });
      }

      // ---- POST /api/reasoner { provider, apiKey?, model? } — attach a reasoner LIVE -----
      // Switch the intelligence from the browser with no restart. SECURITY: an OpenAI key
      // provided here is passed straight into the reasoner's closure and is NEVER written to
      // process.env, disk, a log, the response, or a committed truth — it lives only in this
      // running process, only until the reasoner is replaced or the server stops.
      if (req.method === 'POST' && p === '/api/reasoner') {
        const { provider, apiKey, model } = await readBody(req);
        if (provider === 'stub') {
          assistant.useReasoner(new StubReasoner());
          reasonerInfo = { name: new StubReasoner().name, isStub: true };
        } else if (provider === 'openai') {
          if (typeof apiKey !== 'string' || apiKey.trim() === '') return sendJson(res, 400, { error: 'an OpenAI API key is required to attach a real model' });
          const cleanKey = apiKey.trim();
          // HTTP header values must be Latin-1; a key carrying a non-ASCII char (e.g. an em-dash
          // from pasted prose) would otherwise crash fetch at call time. Reject it cleanly here.
          if (!/^[\x21-\x7e]+$/.test(cleanKey)) return sendJson(res, 400, { error: 'the API key must be printable ASCII with no spaces — paste ONLY the key (e.g. sk-…), not the surrounding text' });
          const inst = new OpenAiReasoner({
            apiKey: cleanKey,
            ...(typeof model === 'string' && model.trim() ? { model: model.trim() } : {}),
          });
          assistant.useReasoner(inst); // validated against the reasoner contract
          reasonerInfo = { name: inst.name, isStub: false };
        } else {
          return sendJson(res, 400, { error: 'provider must be "openai" or "stub"' });
        }
        return sendJson(res, 200, { reasoner: reasonerInfo }); // NO key echoed, ever
      }

      // ---- GET /api/why?q=<subject|id> — reconstruct a decision path from truth --------
      if (req.method === 'GET' && p === '/api/why') {
        const q = url.searchParams.get('q');
        if (!q) return sendJson(res, 400, { error: 'q (subject or truth id) is required' });
        // why() throws for a subject with no decision path yet (e.g. a brand-new goal). That
        // is an expected empty result, not a server error — return a graceful empty trace.
        let trace;
        try { trace = why(assistant, q); }
        catch (e) {
          return sendJson(res, 200, {
            trace: { subject: q, observed: [], findings: [], proposals: [], policies: [], compliance: [], approvals: [], committed: [], decisions: [] },
            note: e.message,
          });
        }
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

  // A port conflict (a second instance / an orphaned server) is a normal operator mistake —
  // fail with a friendly line, not a raw stack trace.
  server.on('error', (e) => {
    try { assistant.close(); } catch {}
    if (e && e.code === 'EADDRINUSE') {
      console.error(`\njarvis-ui: port ${port} is already in use — another JARVIS server is running.\n  • stop it (Ctrl-C in its terminal), or\n  • start this one on another port:  --port 8080\n`);
    } else {
      console.error(`jarvis-ui: server error: ${e ? e.message : 'unknown'}`);
    }
    process.exit(1);
  });

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

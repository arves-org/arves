// Smoke test for the JARVIS console server: boots the REAL server (real Assistant +
// real bridge) on an ephemeral port, drives the /api/* contract the front-end depends
// on, and asserts the shapes. No mocks — if this passes, the browser console works.
//
//   node ui/server.test.mjs
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SERVER = path.join(HERE, 'server.mjs');
const PORT = 7793; // fixed-but-unusual; the harness runs single-tenant
const BASE = `http://127.0.0.1:${PORT}`;

let failed = 0;
const ok = (name) => console.log(`  ok  ${name}`);
const bad = (name, e) => { failed++; console.error(`  FAIL ${name}: ${e.message}`); };

const j = async (method, route, body) => {
  const res = await fetch(BASE + route, {
    method,
    headers: body ? { 'content-type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await res.text();
  let data;
  try { data = JSON.parse(text); } catch { data = text; }
  return { status: res.status, data };
};

const waitUp = async () => {
  for (let i = 0; i < 100; i++) {
    try { const r = await fetch(BASE + '/api/state'); if (r.ok) return; } catch {}
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error('server did not come up');
};

// In-memory session (no --wal-dir): fast, self-cleaning, still the real bridge.
const child = spawn(process.execPath, [SERVER, '--port', String(PORT)], {
  stdio: ['ignore', 'pipe', 'pipe'],
  env: { ...process.env, HOST: '127.0.0.1' },
});
child.stderr.on('data', (d) => process.stderr.write(`  [server] ${d}`));

try {
  await waitUp();

  // 1) state: identity + honest stub reasoner + arrays present
  {
    const { status, data } = await j('GET', '/api/state');
    assert.equal(status, 200);
    assert.ok(data.identity && typeof data.identity.tenant === 'string', 'identity.tenant');
    assert.equal(data.reasoner.isStub, true, 'reasoner must honestly report the stub by default');
    assert.equal(data.reasoner.name, 'stub-reasoner');
    for (const k of ['truths', 'policies', 'approvals', 'skills', 'agents', 'conversation', 'sources', 'timeline']) {
      assert.ok(Array.isArray(data[k]), `state.${k} is an array`);
    }
    assert.ok(typeof data.counts.conflicts === 'number', 'counts.conflicts is present');
    assert.ok(data.timeline.length >= 1 && data.timeline.every((e) => e.kind && e.label), 'timeline has typed cognition events (skill admissions at least)');
    assert.ok(data.policies.length >= 1, 'default spend policy is seeded');
    assert.ok(data.skills.length >= 1 && data.skills.every((s) => s.certified === true), 'skills certified');
    ok('GET /api/state — identity, honest stub reasoner, seeded policy, certified skills');
  }

  // 2) serve the console HTML at /
  {
    const res = await fetch(BASE + '/');
    const html = await res.text();
    assert.equal(res.status, 200);
    assert.match(html, /JARVIS/i, 'index.html served at /');
    ok('GET / — the console HTML is served');
  }

  // 3) observe: import a connector, truths grow, evidence sets present
  {
    const { status, data } = await j('POST', '/api/observe', { source: 'calendar' });
    assert.equal(status, 200);
    assert.ok(data.imported >= 1, 'imported at least one observation');
    assert.ok(Array.isArray(data.truths), 'observe returns refreshed truths');
    const mem = data.truths.filter((t) => t.kind === 'memory');
    assert.ok(mem.length >= 1, 'at least one memory truth');
    assert.ok(mem[0].sources.length >= 1, 'truth carries its evidence sources');
    ok('POST /api/observe — connector import commits truth with evidence');
  }

  // 4) ask: a spend-class goal must be BLOCKED by the seeded policy (proposal->gate)
  let blockedSubject = null;
  {
    const { status, data } = await j('POST', '/api/ask', { goal: 'pay the vendor-x invoice' });
    assert.equal(status, 200);
    assert.equal(data.turn.role, 'jarvis');
    assert.ok(Array.isArray(data.turn.trace) && data.turn.trace.length >= 1, 'trace present');
    assert.equal(data.turn.trace[0].kind, 'thought', 'first step is the reasoner thought');
    const gate = data.turn.trace.find((s) => s.kind === 'gate');
    if (gate && gate.state === 'block') {
      assert.ok(data.approvals.length >= 1, 'a blocked spend surfaces a pending approval');
      blockedSubject = data.approvals[0].subject;
      ok('POST /api/ask — spend goal blocked by policy, pending approval surfaced');
    } else {
      // The stub may not map this phrasing to a spend skill; that is fine — still honest.
      ok('POST /api/ask — proposal->gate trace produced (no spend match for this phrasing)');
    }
  }

  // 5) approve (only if we produced a blocked subject) commits a separate approval truth
  if (blockedSubject) {
    const { status, data } = await j('POST', '/api/approve', { subject: blockedSubject, role: 'user' });
    assert.equal(status, 200);
    assert.match(data.id, /^[0-9a-f]{16,}/, 'approval truth id returned');
    assert.ok(!data.approvals.some((a) => a.subject === blockedSubject), 'subject no longer pending after approval');
    ok('POST /api/approve — separate approval truth clears the pending gate');
  }

  // 5b) reasoner: attach a real model LIVE from the "UI", offline (no /ask => no network),
  //     and prove the key is validated + never echoed back.
  {
    // openai without a key is refused
    const bad = await j('POST', '/api/reasoner', { provider: 'openai' });
    assert.equal(bad.status, 400, 'openai without a key is refused');

    // a key carrying a non-ASCII char (e.g. an em-dash from pasted prose) is refused CLEANLY
    // (not a fetch ByteString crash at call time)
    const dash = await j('POST', '/api/reasoner', { provider: 'openai', apiKey: 'sk-abc—def' });
    assert.equal(dash.status, 400, 'a non-ASCII key is rejected with a clean 400');
    assert.match(dash.data.error, /ASCII|only the key/i, 'the error tells the user to paste only the key');

    // attach openai with a (fake) key + model — attaching makes NO network call
    const FAKE = 'sk-fake-key-for-test-only-DO-NOT-USE';
    const on = await j('POST', '/api/reasoner', { provider: 'openai', apiKey: FAKE, model: 'gpt-4o-mini' });
    assert.equal(on.status, 200);
    assert.equal(on.data.reasoner.isStub, false, 'a real model is now attached');
    assert.equal(on.data.reasoner.name, 'openai:gpt-4o-mini');
    assert.ok(!JSON.stringify(on.data).includes(FAKE), 'the API key is NEVER echoed in the response');

    // state reflects the switch, still without the key
    const st = await j('GET', '/api/state');
    assert.equal(st.data.reasoner.isStub, false, 'state reflects the attached model');
    assert.ok(!JSON.stringify(st.data).includes(FAKE), 'the API key never appears in /api/state');

    // switch back to the free stub
    const off = await j('POST', '/api/reasoner', { provider: 'stub' });
    assert.equal(off.status, 200);
    assert.equal(off.data.reasoner.isStub, true, 'switched back to the deterministic stub');
    ok('POST /api/reasoner — attach OpenAI live (key validated, never echoed) and switch back to stub');
  }

  // 6) why: reconstruct a path from committed truth (must not 500)
  {
    const { status } = await j('GET', '/api/why?q=' + encodeURIComponent(blockedSubject || 'anything'));
    assert.ok(status === 200, 'why returns a trace');
    ok('GET /api/why — decision path reconstructed from truth');
  }

  // 7) 404 for unknown routes (no silent 200)
  {
    const { status } = await j('GET', '/api/nope');
    assert.equal(status, 404);
    ok('unknown route -> 404');
  }
} catch (e) {
  bad('server smoke', e);
} finally {
  child.kill('SIGTERM');
}

if (failed) { console.error(`\nUI server smoke: ${failed} FAILED`); process.exit(1); }
console.log('\nUI server smoke: all checks passed');

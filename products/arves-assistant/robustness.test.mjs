// ARVES Assistant — ROBUSTNESS + completeness tests (assert-based, exit 0/1).
//   A. CLI hostile-input hardening: every bad command returns ok:false + a loud line and
//      NEVER crashes the REPL (bad ISO, empty goal, unknown connector/command, bad policy).
//   B. bridge-down mid-session: after the Kernel is closed, a command returns a clean error
//      instead of an uncaught rejection — the session survives a dead bridge.
//   C. config module: precedence (CLI > file > default), malformed-fails-loud, round-trip,
//      honest reasoner gate.
//   D. report/export module: a deterministic day export from committed truth.
// In-memory Kernel (no walDir), offline, deterministic. Bridges closed in finally.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { openSession, runCommand } from './src/cli.mjs';
import { loadConfig, saveConfig, resolveSession, validateReasonerChoice, defaultConfigPath, DEFAULT_SESSION } from './src/config.mjs';
import { reportDay, renderReport } from './src/report.mjs';
import { Assistant } from './src/assistant.mjs';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const tests = [];
const test = (name, fn) => tests.push({ name, fn });

// ---- A + B: CLI robustness over an in-memory session ----------------------------------

test('A: hostile CLI inputs return ok:false and a loud line — the REPL never crashes', async () => {
  const opts = { tenant: 'rob', workspace: 'w' }; // in-memory Kernel (no walDir)
  const a = await openSession(opts);
  const run = (line) => runCommand(a, line.split(/\s+/), opts);
  const text = (r) => r.lines.join('\n');
  try {
    for (const [line, re] of [
      ['observe email urn:you dentist not-an-iso', /ISO-8601 UTC instant/],
      ['observe email', /usage: observe/],
      ['ask', /usage: ask/],
      ['import nosuchconnector', /unknown connector/],
      ['import csv /no/such/file.csv', /cannot open/],
      ['why', /usage: why/],
      ['policy onlyname', /usage: policy/],
      ['approve user', /usage: approve/],
      ['frobnicate', /unknown command 'frobnicate'/],
    ]) {
      const r = await run(line);
      assert.equal(r.ok, false, `'${line}' -> ok:false`);
      assert.match(text(r), re, `'${line}' -> loud line`);
    }
    // a GOOD command still works afterward (no corruption from the bad ones)
    assert.equal((await run('status')).ok, true);
  } finally {
    a.close();
    await sleep(400);
  }
});

test('B: after the Kernel bridge is closed, a command returns a clean error (no uncaught crash)', async () => {
  const opts = { tenant: 'rob', workspace: 'down' };
  const a = await openSession(opts);
  a.close();                    // simulate bridge-down mid-session
  await sleep(200);
  const r = await runCommand(a, ['observe', 'email', 'urn:you', 'x', '2026-07-06T09:00:00Z'], opts);
  assert.equal(r.ok, false, 'a commit against a dead bridge -> ok:false, not a throw');
  assert.match(r.lines.join('\n'), /error:/);
});

// ---- C: config module -----------------------------------------------------------------

test('C: config precedence is CLI flag > config file > built-in default', () => {
  const cfg = { tenant: 'cfgTenant', walDir: '/from/cfg' };
  const s = resolveSession({ tenant: 'cliTenant' }, cfg);
  assert.equal(s.tenant, 'cliTenant', 'CLI flag wins');
  assert.equal(s.walDir, '/from/cfg', 'config fills what the CLI omitted');
  assert.equal(s.workspace, DEFAULT_SESSION.workspace, 'default fills what neither set');
  assert.equal(s.reasoner, 'stub');
});

test('C: loadConfig — missing is {}, malformed FAILS LOUD, round-trips through saveConfig', () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-cfg-'));
  try {
    const p = path.join(dir, '.jarvisrc.json');
    assert.deepEqual(loadConfig(p), {}, 'missing file -> {}');
    fs.writeFileSync(p, '{ not json');
    assert.throws(() => loadConfig(p), /not valid JSON/);
    saveConfig(p, { tenant: 'me', workspace: 'ws', walDir: '/w', bogus: 'dropped' });
    assert.deepEqual(loadConfig(p), { tenant: 'me', workspace: 'ws', walDir: '/w' }, 'unknown keys dropped, known kept');
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('C: only the in-repo stub reasoner is selectable (honest scope)', () => {
  assert.equal(validateReasonerChoice(undefined), 'stub');
  assert.equal(validateReasonerChoice('stub'), 'stub');
  assert.throws(() => validateReasonerChoice('gpt-9'), /not available in-repo/);
  assert.match(defaultConfigPath(), /\.jarvisrc\.json$/);
});

// ---- D: report / export ---------------------------------------------------------------

test('D: reportDay exports the day from committed truth (deterministic, grouped by entity)', async () => {
  const a = new Assistant({ tenant: 'rep', workspace: 'w' }); // in-memory
  try {
    await a.observe('notes-file', { entity: 'urn:you', event: 'dentist-appointment', at: BigInt(Date.parse('2026-07-06T09:00:00Z')) });
    await a.observe('calendar-file', { entity: 'urn:you', event: 'dentist-appointment', at: BigInt(Date.parse('2026-07-06T09:00:00Z')) });
    await a.observe('tasks-file', { entity: 'proj:arves', event: 'review-pr', at: BigInt(Date.parse('2026-07-06T11:00:00Z')) });
    const r = reportDay(a);
    assert.equal(r.counts.truths, 2, 'the cross-source dentist collapsed to ONE truth');
    assert.equal(r.counts.entities, 2);
    const you = r.entities.find((e) => e.entity === 'urn:you');
    assert.deepEqual(you.items[0].sources, ['calendar-file', 'notes-file'], 'both sources in the one truth');
    assert.equal(you.items[0].at, '2026-07-06T09:00:00.000Z', 'instant rendered from committed ns, no clock');
    // rendering is deterministic and mentions the entities
    assert.equal(renderReport(r), renderReport(reportDay(a)));
    assert.match(renderReport(r), /urn:you/);
  } finally {
    a.close();
    await sleep(400);
  }
});

let failed = 0;
for (const { name, fn } of tests) {
  try { await fn(); console.log(`  PASS  ${name}`); }
  catch (e) { failed++; console.log(`  FAIL  ${name}\n        ${e.message}`); }
}
console.log(`\narves-assistant (robustness): ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);

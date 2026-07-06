// ARVES Assistant — product-local tests (assert-based, exit 0/1).
//
// Covers: cross-source dedup + evidence sets (A2) · decision truths · contradiction
// detection · input validation · connector determinism · RESTART SURVIVAL over a durable
// --wal-dir (A1: rebuild-from-committed-truth via the idempotent re-commit membership
// proof). Every bridge is closed in finally; every walDir is a fresh temp dir cleaned up
// in finally. Offline, deterministic, no third-party deps.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Assistant, canonicalFact } from './src/assistant.mjs';
import { notesConnector, calendarConnector, tasksConnector, allObservations } from './src/connectors.mjs';
import { Arves } from '../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();
const sleep = (ms) => new Promise((res) => setTimeout(res, ms));
// BigInt-safe deep-equality snapshot (JSON can't carry BigInt).
const snap = (v) => JSON.stringify(v, (_, x) => (typeof x === 'bigint' ? `${x}n` : x));

const T = 1_751_792_400_000n; // 2026-07-06T09:00:00Z, fixed
const FACT = { entity: 'urn:you', event: 'dentist-appointment', at: T };

const tests = [];
const test = (name, fn) => tests.push({ name, fn });

test('A2: same fact from two sources -> ONE truth, both sources in evidence, Kernel-deduped', async () => {
  const a = new Assistant(); // in-memory Kernel: dedup semantics need no durability
  try {
    const r1 = await a.observe('notes-file', FACT);
    const r2 = await a.observe('calendar-file', FACT);
    assert.equal(r1.id, r2.id, 'same canonical fact must have the same ContentId');
    assert.equal(r1.status, 'committed');
    assert.equal(r2.status, 'already-committed', 'the KERNEL answers the duplicate, not just the index');
    assert.equal(r1.deduped, false);
    assert.equal(r2.deduped, true);
    assert.deepEqual(r2.sources, ['calendar-file', 'notes-file']);
    assert.equal(a.truths().length, 1);
    // One-world identity: the id the product computed locally is the id the Kernel returned.
    assert.equal(r1.id, arves.address(canonicalFact(FACT), 'commit'));
    // A different fact is a different truth.
    const r3 = await a.observe('notes-file', { ...FACT, event: 'renew-passport' });
    assert.notEqual(r3.id, r1.id);
    assert.equal(a.truths().length, 2);
    assert.equal(a.recall('urn:you').length, 2);
    assert.equal(a.recall('urn:nobody').length, 0);
  } finally { a.close(); }
});

test('A2: source attestations are committed truth (idempotent per source+fact pair)', async () => {
  const a = new Assistant();
  try {
    const r1 = await a.observe('notes-file', FACT);
    const again = await a.observe('notes-file', FACT); // same source, same fact
    assert.equal(again.attestationId, r1.attestationId);
    assert.equal(again.attestationStatus, 'already-committed');
    const other = await a.observe('calendar-file', FACT); // other source, same fact
    assert.notEqual(other.attestationId, r1.attestationId, 'evidence differs per source');
    assert.equal(other.id, r1.id, 'but the truth is still one');
  } finally { a.close(); }
});

test('connectors: deterministic, offline, 3 sources, exactly one cross-source duplicate', async () => {
  assert.equal(snap(allObservations()), snap(allObservations()), 'two reads must be byte-identical');
  const obs = allObservations();
  assert.equal(obs.length, 7);
  assert.deepEqual([...new Set(obs.map((o) => o.source))].sort(), ['calendar-file', 'notes-file', 'tasks-file']);
  assert.equal(notesConnector().length, 3);
  assert.equal(calendarConnector().length, 2);
  assert.equal(tasksConnector().length, 2);
  // Exactly one canonical body appears under two different sources (the A2 duplicate).
  const byBody = new Map();
  for (const o of obs) {
    const k = snap(canonicalFact(o.fact));
    byBody.set(k, (byBody.get(k) ?? new Set()).add(o.source));
  }
  assert.equal(byBody.size, 6, '7 observations, 6 distinct canonical facts');
  const dupes = [...byBody.values()].filter((s) => s.size > 1);
  assert.equal(dupes.length, 1);
  assert.deepEqual([...dupes[0]].sort(), ['calendar-file', 'notes-file']);
});

test('decisions: committed truth + contradiction detection with the prior id as proof', async () => {
  const a = new Assistant();
  try {
    const d = await a.recordDecision('invest:acme-fund', 'decline', 'too volatile');
    assert.equal(d.status, 'committed');
    assert.match(d.id, /^[0-9a-f]{68}$/);
    const hit = a.checkContradiction({ subject: 'invest:acme-fund', action: 'approve' });
    assert.equal(hit.contradicts, true);
    assert.equal(hit.priorId, d.id);
    assert.deepEqual(hit.prior, { subject: 'invest:acme-fund', action: 'decline', because: 'too volatile' });
    assert.equal(a.checkContradiction({ subject: 'invest:acme-fund', action: 'decline' }).contradicts, false);
    assert.equal(a.checkContradiction({ subject: 'urn:unknown', action: 'approve' }).contradicts, false);
    // Latest decision per subject wins (a revision is a NEW committed truth, not an edit).
    const d2 = await a.recordDecision('invest:acme-fund', 'approve', 'policy revised at Q3 review');
    assert.notEqual(d2.id, d.id);
    const hit2 = a.checkContradiction({ subject: 'invest:acme-fund', action: 'decline' });
    assert.equal(hit2.contradicts, true);
    assert.equal(hit2.priorId, d2.id);
    assert.equal(a.decisions().length, 1, 'one indexed decision per subject');
  } finally { a.close(); }
});

test('validation: dishonest or ambiguous inputs are rejected loudly', async () => {
  const a = new Assistant();
  try {
    await assert.rejects(() => a.observe('', FACT), /source/);
    await assert.rejects(() => a.observe('notes-file', { ...FACT, at: 123 }), /BigInt/); // bare number is ambiguous (ACS-002)
    await assert.rejects(() => a.observe('notes-file', { ...FACT, at: -1n }), /BigInt/);
    await assert.rejects(() => a.observe('notes-file', { entity: '', event: 'x', at: T }), /entity/);
    await assert.rejects(() => a.observe('notes-file', { entity: 'urn:you', event: '', at: T }), /event/);
    await assert.rejects(() => a.recordDecision('s', 'a', ''), /because/);
    assert.throws(() => a.checkContradiction(null), /candidate/);
    // An ambiguous candidate must be rejected loudly, never a silent contradicts:false.
    assert.throws(() => a.checkContradiction({ subject: undefined, action: undefined }), /candidate\.subject/);
    assert.throws(() => a.checkContradiction({ subject: 'invest:acme-fund', action: '' }), /candidate\.action/);
  } finally { a.close(); }
});

test('A1: memory + contradiction detection survive a Kernel-process restart over the same walDir', async () => {
  const walDir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-assistant-test-'));
  let a = null;
  try {
    a = new Assistant({ tenant: 't1', workspace: 'w1', walDir });
    for (const o of allObservations()) await a.observe(o.source, o.fact);
    const idsBefore = a.truths().map((t) => t.id).join('|');
    const d = await a.recordDecision('invest:acme-fund', 'decline', 'too volatile');
    a.close(); a = null;
    await sleep(400); // let the bridge process exit (Windows WAL file locks)

    a = new Assistant({ tenant: 't1', workspace: 'w1', walDir });
    assert.equal(a.truths().length, 0, 'honesty: a fresh process remembers nothing before rebuild');
    assert.equal(a.checkContradiction({ subject: 'invest:acme-fund', action: 'approve' }).contradicts, false);

    const report = await a.rebuild({
      observations: allObservations(),
      decisions: [{ subject: 'invest:acme-fund', action: 'decline', because: 'too volatile' }],
    });
    // The membership proof: EVERY candidate answers already-committed; nothing is fresh.
    assert.equal(report.factsRecovered, 7);
    assert.equal(report.factsFresh, 0);
    assert.equal(report.attestationsRecovered, 7);
    assert.equal(report.attestationsFresh, 0);
    assert.equal(report.decisionsRecovered, 1);
    assert.equal(report.decisionsFresh, 0);
    assert.equal(report.freshIds.length, 0);
    assert.equal(a.truths().map((t) => t.id).join('|'), idsBefore, 'identical ContentId set after restart');
    const hit = a.checkContradiction({ subject: 'invest:acme-fund', action: 'approve' });
    assert.equal(hit.contradicts, true);
    assert.equal(hit.priorId, d.id, 'the SAME prior decision id, across the restart');
    // A genuinely new body after rebuild is honestly reported as fresh, not recovered.
    const report2 = await a.rebuild({ observations: [{ source: 'notes-file', fact: { ...FACT, event: 'brand-new-note', at: T } }] });
    assert.equal(report2.factsFresh, 1);
    assert.equal(report2.factsRecovered, 0);
  } finally {
    try { if (a) a.close(); } catch { /* already gone */ }
    await sleep(400);
    try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort temp cleanup */ }
  }
});

test('A1 (two-process): observe/decide in a CHILD node process that fully exits; rebuild here', async () => {
  // The single-process A1 test above restarts the bridge/Kernel process while THIS node
  // process survives. This variant kills node itself: a child node process commits the
  // day and exits completely; the parent then rebuilds from the same walDir and proves
  // every body was already committed truth — identical ContentIds, same prior decision id.
  const walDir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-assistant-2proc-'));
  const assistantUrl = new URL('./src/assistant.mjs', import.meta.url).href;
  const connectorsUrl = new URL('./src/connectors.mjs', import.meta.url).href;
  const childSrc = [
    `import { Assistant } from ${JSON.stringify(assistantUrl)};`,
    `import { allObservations } from ${JSON.stringify(connectorsUrl)};`,
    `const a = new Assistant({ tenant: 't2', workspace: 'w2', walDir: process.argv[1] });`,
    `try {`,
    `  for (const o of allObservations()) await a.observe(o.source, o.fact);`,
    `  const d = await a.recordDecision('invest:acme-fund', 'decline', 'too volatile');`,
    `  console.log(JSON.stringify({ ids: a.truths().map((t) => t.id), decisionId: d.id }));`,
    `} finally { a.close(); }`,
  ].join('\n');
  let a = null;
  try {
    const run = spawnSync(process.execPath, ['--input-type=module', '-e', childSrc, walDir], { encoding: 'utf8' });
    assert.equal(run.status, 0, `child node process failed:\n${run.stderr}`);
    const child = JSON.parse(run.stdout.trim().split('\n').pop());
    assert.equal(child.ids.length, 6, 'child committed 6 deduplicated truths');
    await sleep(400); // the child (and its bridge) are gone; let WAL file locks release

    a = new Assistant({ tenant: 't2', workspace: 'w2', walDir }); // a genuinely new process's view
    assert.equal(a.truths().length, 0, 'honesty: the parent process remembers nothing before rebuild');
    const report = await a.rebuild({
      observations: allObservations(),
      decisions: [{ subject: 'invest:acme-fund', action: 'decline', because: 'too volatile' }],
    });
    assert.equal(report.factsRecovered, 7);
    assert.equal(report.attestationsRecovered, 7);
    assert.equal(report.decisionsRecovered, 1);
    assert.equal(report.freshIds.length, 0, 'NOTHING is fresh: all truth predates this process');
    assert.equal(a.truths().map((t) => t.id).join('|'), child.ids.join('|'), 'identical ContentIds across processes');
    const hit = a.checkContradiction({ subject: 'invest:acme-fund', action: 'approve' });
    assert.equal(hit.contradicts, true);
    assert.equal(hit.priorId, child.decisionId, 'the SAME prior decision id, across two separate node processes');
  } finally {
    try { if (a) a.close(); } catch { /* already gone */ }
    await sleep(400);
    try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort temp cleanup */ }
  }
});

// ---- runner ---------------------------------------------------------------------------
let failed = 0;
for (const { name, fn } of tests) {
  try {
    await fn();
    console.log(`  PASS  ${name}`);
  } catch (e) {
    failed++;
    console.log(`  FAIL  ${name}\n        ${e.message}`);
  }
}
console.log(`\narves-assistant: ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);

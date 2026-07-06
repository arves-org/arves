// ARVES Assistant — CLI/REPL tests (assert-based, exit 0/1). Two layers:
//   Part A (in-process): drive runCommand() over a durable temp WAL — every command, the
//           full govern-a-spend flow, dedup across file formats, determinism, error paths.
//   Part B (cross-process): spawn the REAL bin (`node bin/jarvis.mjs`) as separate processes
//           over ONE --wal-dir and prove durability — a FRESH process recalls truth a prior
//           process committed (RCR-033 WAL scan) and a PRIOR approval still unlocks a spend.
// Offline, deterministic, no third-party deps; bridges closed in finally, temp WALs removed.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openSession, runCommand, parseArgs } from './src/cli.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const BIN = path.join(HERE, 'bin', 'jarvis.mjs');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const mkwal = () => fs.mkdtempSync(path.join(os.tmpdir(), 'arves-cli-'));

const tests = [];
const test = (name, fn) => tests.push({ name, fn });

// ---- Part A: in-process, one durable session ------------------------------------------

test('A: the CLI drives a full assistant session over a durable WAL (all commands, govern-a-spend, dedup, determinism)', async () => {
  const walDir = mkwal();
  const opts = { tenant: 'cli', workspace: 'w', walDir };
  const a = await openSession(opts);
  const run = (line) => runCommand(a, line.split(/\s+/), opts);
  const text = (r) => r.lines.join('\n');
  try {
    // help + skills + status
    assert.match(text(await run('help')), /observe .*ask .*recall/s, 'help lists the commands');
    const skills = text(await run('skills'));
    for (const s of ['day.summarize', 'spend.order', 'reply.draft', 'schedule.block', 'notes.digest']) {
      assert.ok(skills.includes(s), `example skill '${s}' is registered + bound`);
    }
    const status = text(await run('status'));
    assert.match(status, /stub-reasoner@1\.0\.0/);
    assert.match(status, /cli\/w/, 'status shows the shard');

    // import a REAL-format file (iCalendar) -> two truths
    assert.match(text(await run('import ical')), /imported 2 observation\(s\) via 'ical'.*2 new/s);
    const recall = text(await run('recall'));
    assert.ok(recall.includes('dentist-appointment') && recall.includes('ical-file'), 'recall reflects imported truth');

    // DEDUP ACROSS FORMATS: the same real-world event from a different source name collapses
    // to ONE truth (source is evidence, not identity). The ical dentist == this notes dentist.
    const obs = text(await run('observe notes-file urn:you dentist-appointment 2026-07-06T09:00:00Z'));
    assert.match(obs, /merged into existing truth/, 'a cross-format duplicate merges, not duplicates');
    assert.match(obs, /already-committed/, 'the fact was already truth (committed by the ical import)');
    const dentist = a.recall('urn:you').find((t) => t.fact.event === 'dentist-appointment');
    assert.deepEqual(dentist.sources, ['ical-file', 'notes-file'], 'both sources are in the ONE truth\'s evidence set');

    // ask (normal, acted) — reasoner -> skill -> committed effect truth
    const summarize = text(await run('ask summarize my day'));
    assert.match(summarize, /ACTED .*day\.summarize/s);
    assert.match(summarize, /committed effect .*uci\.assistant\.briefing/s);

    // ask (spend, BLOCKED) — guardrail holds it, block is committed, unlock hint is exact
    const blocked = text(await run('ask order flowers'));
    assert.match(blocked, /BLOCKED .*spend\.order/s);
    assert.match(blocked, /to unlock:  approve user spend:order-flowers/);
    assert.match(blocked, /compliance truth/);

    // a SEPARATE approval truth unlocks it; ask now acts and cites the approval
    assert.match(text(await run('approve user spend:order-flowers')), /approved 'user'.*SEPARATE committed approval/s);
    const allowed = text(await run('ask order flowers'));
    assert.match(allowed, /ACTED .*spend\.order/s);
    assert.match(allowed, /citing approval\(s\)/);

    // why reconstructs every station from committed truth
    const trace = text(await run('why spend:order-flowers'));
    for (const station of ['OBSERVED', 'PROPOSED', 'POLICY CHECKED', 'BLOCKED', 'APPROVED', 'COMMITTED']) {
      assert.ok(trace.includes(station), `why() shows the ${station} station`);
    }

    // honesty: a goal outside the stub's rule table -> NO ACTION (not a guess)
    assert.match(text(await run('ask compose a symphony in D minor')), /NO ACTION/);

    // determinism: the same normal ask replays to the SAME committed effect (already-committed)
    assert.match(text(await run('ask summarize my day')), /already-committed/, 'deterministic replay');

    // error paths never crash the REPL — they return ok:false + a loud line
    const unknown = await run('frobnicate');
    assert.equal(unknown.ok, false);
    assert.match(text(unknown), /unknown command 'frobnicate'/);
    const noGoal = await run('ask');
    assert.equal(noGoal.ok, false);
    assert.match(text(noGoal), /usage: ask/);
  } finally {
    a.close();
    await sleep(400);
    try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort */ }
  }
});

test('A: parseArgs separates flags from the command', () => {
  const { opts, rest } = parseArgs(['--tenant', 't', '--workspace', 'w', '--wal-dir', '/x', 'ask', 'summarize', 'my', 'day']);
  assert.deepEqual(opts, { tenant: 't', workspace: 'w', walDir: '/x', exe: undefined });
  assert.deepEqual(rest, ['ask', 'summarize', 'my', 'day']);
  assert.deepEqual(parseArgs(['-h']).rest, ['help']);
});

// ---- Part B: cross-process durability via the REAL bin --------------------------------

test('B: a FRESH process recalls truth a PRIOR process committed, and a PRIOR approval still unlocks a spend (real bin, one WAL)', async () => {
  const walDir = mkwal();
  const bin = (args) => {
    const out = execFileSync(process.execPath, [BIN, '--wal-dir', walDir, ...args], { encoding: 'utf8' });
    return out;
  };
  try {
    // process 1: import + attempt a spend (blocked) + grant the approval — then it EXITS.
    bin(['import', 'ical']);
    await sleep(400);
    assert.match(bin(['ask', 'order', 'flowers']), /BLOCKED/); await sleep(400);
    bin(['approve', 'user', 'spend:order-flowers']); await sleep(400);

    // process N (brand-new): recall sees the prior truth — rebuilt READ-ONLY from the WAL scan.
    const recall = bin(['recall']); await sleep(400);
    assert.match(recall, /dentist-appointment/, 'a fresh process recalls truth from the WAL (RCR-033)');
    assert.match(recall, /q3-review-meeting/);

    // process N+1 (brand-new): the spend now ACTS — the approval committed by process 1 was
    // rehydrated from the WAL, so the gate opens across the restart (durable governance). This
    // is the FIRST time the effect actually commits (process 1 only granted the approval).
    const acted = bin(['ask', 'order', 'flowers']); await sleep(400);
    assert.match(acted, /ACTED/, 'the durable approval unlocks the spend in a fresh process');
    assert.match(acted, /citing approval\(s\)/, 'the acting path cites the durable approval truth');

    // process N+2 (brand-new): re-asking replays to the SAME effect truth — already-committed,
    // proving the whole govern-a-spend path is deterministic ACROSS separate processes.
    const again = bin(['ask', 'order', 'flowers']); await sleep(400);
    assert.match(again, /already-committed/, 'the effect replays identically across processes');
  } finally {
    await sleep(400);
    try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort */ }
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
console.log(`\narves-assistant (CLI/REPL): ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);

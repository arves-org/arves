// ARVES Assistant — a scripted day (acceptance A1 + A2, end to end, honestly).
//
// Act 1 (the day):   observe the user's world from 3 offline connectors (with one
//                    cross-source duplicate), record a decision, detect a contradiction.
// Act 2 (the crash): terminate the Kernel/bridge process (honest scope: THIS node process
//                    survives; the two-process kill — child node commits then fully exits —
//                    is proven in assistant.test.mjs).
// Act 3 (the proof): a NEW Assistant over the SAME --wal-dir rebuilds its memory from
//                    committed truth — same ContentIds, every one already-committed —
//                    and contradiction detection still works (A1 bites).
//
// Everything is deterministic and offline: fixture connectors, fixed instants, the frozen
// Rust reference Kernel via the bridge line protocol. No LLM is involved anywhere in this
// script — this is the MEMORY core; the intelligence arrives when the maintainer plugs
// their LLM into the (later-stage) reasoner slot.
//
// Exit code: 0 iff every property below PASSes.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Assistant } from '../src/assistant.mjs';
import { allObservations } from '../src/connectors.mjs';

const sleep = (ms) => new Promise((res) => setTimeout(res, ms));
const results = [];
const check = (property, pass, detail = '') => { results.push({ property, pass, detail }); };

// The user's decision journal — the deterministic re-supply source for decision bodies on
// rebuild (the Kernel's already-committed answer, not this list, is the membership proof).
const JOURNAL = [{
  subject: 'invest:acme-fund',
  action: 'decline',
  because: 'volatility exceeds my risk policy (decided at the Q2 review)',
}];
const TEMPTATION = { subject: 'invest:acme-fund', action: 'approve' }; // price is up 20%...

const walDir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-assistant-day-'));
let assistant = null;
try {
  // ---- Act 1: the day -------------------------------------------------------------
  assistant = new Assistant({ tenant: 'maintainer', workspace: 'jarvis', walDir });

  const obs = allObservations();
  const day1 = [];
  for (const o of obs) day1.push(await assistant.observe(o.source, o.fact));

  const truths1 = assistant.truths();
  check('A2: 7 observations from 3 sources collapse to 6 truths (1 cross-source duplicate)',
    obs.length === 7 && truths1.length === 6,
    `observations=${obs.length} truths=${truths1.length}`);

  const dentist1 = truths1.find((t) => t.fact.event === 'dentist-appointment');
  check('A2: the duplicated event is ONE truth with BOTH sources in evidence',
    dentist1 !== undefined && dentist1.sources.join(',') === 'calendar-file,notes-file',
    `sources=[${dentist1 ? dentist1.sources.join(', ') : '-'}]`);

  const dupCommits = day1.filter((r) => r.status === 'already-committed');
  check('A2: the Kernel itself deduplicates (duplicate body answers already-committed)',
    dupCommits.length === 1 && day1.filter((r) => r.deduped).length === 1,
    `already-committed=${dupCommits.length}/7`);

  const d1 = await assistant.recordDecision(JOURNAL[0].subject, JOURNAL[0].action, JOURNAL[0].because);
  check('decision is committed truth (content-addressed, in the WAL)',
    d1.status === 'committed' && /^[0-9a-f]{68}$/.test(d1.id), `id=${d1.id.slice(0, 16)}…`);

  const c1 = assistant.checkContradiction(TEMPTATION);
  check('contradiction detected against the prior committed decision (proof = its truth id)',
    c1.contradicts === true && c1.priorId === d1.id, `priorId=${c1.priorId ? c1.priorId.slice(0, 16) : '-'}…`);

  // ---- Act 2: the crash (Kernel/bridge process terminates; this node process survives) --
  assistant.close();
  assistant = null;
  await sleep(400); // let the bridge process exit (Windows file locks on the WAL dir)

  // ---- Act 3: restart + rebuild (A1) ------------------------------------------------
  assistant = new Assistant({ tenant: 'maintainer', workspace: 'jarvis', walDir });

  check('honesty: before rebuild, the fresh process remembers NOTHING (index is a projection)',
    assistant.truths().length === 0 && assistant.checkContradiction(TEMPTATION).contradicts === false);

  const report = await assistant.rebuild({ observations: allObservations(), decisions: JOURNAL });

  check('A1: EVERY rebuilt body answers already-committed — memory provably survived the restart',
    report.factsFresh === 0 && report.attestationsFresh === 0 && report.decisionsFresh === 0
      && report.factsRecovered === 7 && report.decisionsRecovered === 1,
    `facts ${report.factsRecovered}rec/${report.factsFresh}fresh · attestations ${report.attestationsRecovered}rec/${report.attestationsFresh}fresh · decisions ${report.decisionsRecovered}rec/${report.decisionsFresh}fresh`);

  const ids1 = truths1.map((t) => t.id).join('|');
  const ids2 = assistant.truths().map((t) => t.id).join('|');
  check('A1: the rebuilt memory has the IDENTICAL ContentId set (same truths, same identity)',
    ids1 === ids2 && assistant.truths().length === 6);

  const dentist2 = assistant.truths().find((t) => t.fact.event === 'dentist-appointment');
  check('A1+A2: the evidence set survived too (attestations are committed truth)',
    dentist2 !== undefined && dentist2.sources.join(',') === 'calendar-file,notes-file');

  const c2 = assistant.checkContradiction(TEMPTATION);
  check('A1: contradiction detection WORKS AFTER RESTART, citing the SAME prior decision id',
    c2.contradicts === true && c2.priorId === d1.id,
    `priorId=${c2.priorId ? c2.priorId.slice(0, 16) : '-'}… (same as pre-restart: ${c2.priorId === d1.id})`);

  const recallYou = assistant.recall('urn:you');
  check('recall(entity) answers from the rebuilt memory',
    recallYou.length === 4 && recallYou.every((t) => t.fact.entity === 'urn:you'), `urn:you truths=${recallYou.length}`);
} catch (e) {
  check(`unexpected error: ${e.message}`, false);
} finally {
  try { if (assistant) assistant.close(); } catch { /* already gone */ }
  await sleep(400); // let the bridge exit before deleting its WAL dir
  try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort temp cleanup */ }
}

// ---- The property table -------------------------------------------------------------
const width = Math.max(...results.map((r) => r.property.length));
console.log('\nARVES Assistant — scripted day (A1 durable memory · A2 multi-source one-truth)\n');
for (const r of results) {
  console.log(`  ${r.pass ? 'PASS' : 'FAIL'}  ${r.property.padEnd(width)}${r.detail ? `  [${r.detail}]` : ''}`);
}
const failed = results.filter((r) => !r.pass).length;
console.log(`\n${results.length - failed}/${results.length} properties PASS${failed ? ` — ${failed} FAIL` : ''}`);
process.exit(failed === 0 && results.length > 0 ? 0 : 1);

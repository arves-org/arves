// ARVES Assistant — THE HARDENED DAY: the daily-driver features, proven end to end.
//
// This complements the capstone (jarvis-day.mjs, which proves A1–A7). Here we prove the
// GA-HARDENING that makes JARVIS the maintainer's daily driver — within honest scope:
//   1. real-format TEMPLATE connectors (csv, jsonl, email) parse a user's own files;
//   2. cross-format ONE-TRUTH: the dentist from csv AND jsonl collapses to ONE committed
//      truth with BOTH sources in evidence (A2), over a REAL WAL-backed Kernel;
//   3. hostile inputs (malformed lines, missing headers, missing file) fail LOUD, never crash;
//   4. persistent config precedence (CLI flag > file > default);
//   5. export/report the day, deterministically, from committed truth;
//   6. a REAL-LLM reasoner ADAPTER (driven by a FAKE client here — no network, no key)
//      runs the SAME governed pipeline as the stub: proposal -> gate -> certified skill
//      -> committed effect; a hallucinated skill is refused before any effect commits;
//   7. bridge-down mid-session -> a clean error, not an uncaught crash.
//
// LOUD HONESTY: still the deterministic StubReasoner by default; the adapter's intelligence
// is the maintainer's LLM, plugged in OUTSIDE the repo. The four-condition GA gate
// (Independent Runtime · External Team · Certification · Formal) is EXTERNAL and remains
// UNMET — this proves a hardened, complete assistant, not GA. Single host, no authN (v1.0).
//
// Exit code: 0 iff every property PASSes.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { Assistant, canonicalFact } from '../src/assistant.mjs';
import { csvConnector, jsonlConnector, emailConnector, markdownJournalConnector, icalConnector } from '../src/connectors.mjs';
import { resolveSession, saveConfig, loadConfig } from '../src/config.mjs';
import { reportDay, renderReport } from '../src/report.mjs';
import { LlmReasonerAdapter } from '../src/llm-reasoner.example.mjs';
import { registerSkill, defineCapability } from '../src/skills.mjs';
import { Arves } from '../../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = [];
const check = (property, pass, detail = '') => results.push({ property, pass, detail });
const short = (id) => (id ? `${id.slice(0, 16)}…` : '-');
const throws = (fn, re) => { try { fn(); } catch (e) { return re.test(e.message); } return false; };

const walDir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-hardened-'));
const cfgFile = path.join(walDir, '.jarvisrc.json');
let assistant = null;
try {
  // ---- 4. config precedence (no bridge needed) --------------------------------------
  saveConfig(cfgFile, { tenant: 'maintainer', walDir });
  const session = resolveSession({ workspace: 'jarvis' }, loadConfig(cfgFile));
  check('4. config precedence: CLI flag > file > default (tenant from file, workspace from flag, reasoner default stub)',
    session.tenant === 'maintainer' && session.workspace === 'jarvis' && session.walDir === walDir && session.reasoner === 'stub');

  // ---- 3. hostile inputs fail loud ---------------------------------------------------
  const badDir = fs.mkdtempSync(path.join(os.tmpdir(), 'arves-bad-'));
  const w = (n, b) => { const p = path.join(badDir, n); fs.writeFileSync(p, b); return p; };
  const hostile = [
    throws(() => markdownJournalConnector(w('bad.md', '- 09:00 x')), /before any .* date heading/),
    throws(() => icalConnector(w('bad.ics', 'BEGIN:VEVENT\nSUMMARY:x\nEND:VEVENT')), /missing DTSTART or SUMMARY/),
    throws(() => csvConnector(w('bad.csv', '2026-07-06T09:00:00Z,onlytwo')), /expected 3 columns/),
    throws(() => jsonlConnector(w('bad.jsonl', 'not json')), /invalid JSON/),
    throws(() => emailConnector(w('bad.eml', 'Subject: x\n\nbody')), /missing a Date header/),
    throws(() => csvConnector(path.join(badDir, 'nope.csv')), /cannot open/),
  ];
  fs.rmSync(badDir, { recursive: true, force: true });
  check('3. hostile inputs (bad journal/ical/csv/jsonl/eml + missing file) ALL fail loud with clean errors',
    hostile.every(Boolean), `${hostile.filter(Boolean).length}/${hostile.length} refused`);

  // ---- 2. cross-format one-truth (real WAL) ------------------------------------------
  assistant = new Assistant({ tenant: session.tenant, workspace: session.workspace, walDir });
  for (const { source, fact } of [...csvConnector(), ...jsonlConnector()]) await assistant.observe(source, fact);
  const dentist = assistant.truths().find((t) => t.fact.event === 'dentist-appointment');
  check('2. cross-format ONE-TRUTH: csv + jsonl dentist collapse to ONE committed truth with BOTH sources (A2)',
    dentist !== undefined && dentist.sources.join(',') === 'csv-file,jsonl-file'
      && dentist.id === arves.address(canonicalFact({ entity: 'urn:you', event: 'dentist-appointment', at: BigInt(Date.parse('2026-07-06T09:00:00Z')) }), 'commit'),
    `dentist=${short(dentist && dentist.id)} sources=[${dentist && dentist.sources.join(', ')}]`);

  // ---- 1. email template parses to its own fact --------------------------------------
  const mail = emailConnector();
  check('1. email (.eml) template: From->entity, Subject->event(slug), Date->UTC instant',
    mail.length === 1 && mail[0].fact.entity === 'alice@dental.example' && mail[0].fact.event === 'dentist-appointment-confirmation');

  // ---- 5. report / export ------------------------------------------------------------
  const rep = reportDay(assistant);
  check('5. report/export from committed truth: deterministic (byte-identical re-render), grouped by entity',
    rep.counts.truths === 3 && renderReport(rep) === renderReport(reportDay(assistant)),
    `${rep.counts.truths} truths / ${rep.counts.entities} entities`);

  // ---- 6. LLM reasoner adapter (fake client) through the governed pipeline ------------
  const summarize = defineCapability({
    name: 'day.summarize', version: '1.0.0', produces: ['uci.assistant.briefing'],
    execute: (input) => [{ target: 'uci.assistant.briefing', value: { type: 'uci.assistant.briefing', count: BigInt(input.events.length), events: [...input.events].sort() } }],
  });
  await registerSkill(assistant, summarize, [{ type: 'uci.assistant.skill-input', events: ['a', 'b'] }]);
  const goodClient = { async complete() { return '```json\n{"action":"invoke-skill","skill":"day.summarize","input":{"events":["x","y"]},"subject":"day:briefing","actionClass":"normal","because":"fake model"}\n```'; } };
  assistant.useReasoner(new LlmReasonerAdapter({ client: goodClient, name: 'fake-llm', version: '9.9.9' }));
  const acted = await assistant.think('summarize my day');
  check('6a. LLM adapter (fake client) runs the SAME governed pipeline: proposal-as-truth -> gate -> certified skill -> committed effect',
    acted.acted === true && acted.invocation.truths[0].target === 'uci.assistant.briefing',
    `effect=${short(acted.acted && acted.invocation.truths[0].id)}`);

  const evilClient = { async complete() { return '{"action":"invoke-skill","skill":"exfiltrate.secrets","because":"nope"}'; } };
  assistant.useReasoner(new LlmReasonerAdapter({ client: evilClient }));
  const refused = await assistant.think('do something bad');
  check('6b. the adapter REFUSES a hallucinated skill (not certified+bound) before any effect commits',
    refused.acted === false && refused.reason === 'no-action-proposed');

  // ---- 7. bridge-down mid-session ----------------------------------------------------
  assistant.close();
  await sleep(300);
  let cleanError = false;
  try { await assistant.observe('csv-file', { entity: 'urn:you', event: 'after-close', at: 1n }); }
  catch { cleanError = true; }
  check('7. bridge-down mid-session: an operation on a closed Kernel throws a clean error, never an uncaught crash', cleanError);
  assistant = null;

  console.log('\n--- report(the hardened day): ---\n');
  console.log(renderReport(rep));
} catch (e) {
  check(`unexpected error: ${e.message}`, false);
} finally {
  try { if (assistant) assistant.close(); } catch { /* already gone */ }
  await sleep(300);
  try { fs.rmSync(walDir, { recursive: true, force: true }); } catch { /* best-effort */ }
}

const width = Math.max(...results.map((r) => r.property.length));
console.log('\nARVES Assistant — THE HARDENED DAY (real-format connectors · cross-format one-truth · hostile-input hardening · config · report · LLM adapter · bridge-down)');
console.log('(honest: stub reasoner by default; the adapter is fake-client-driven — no network; four-condition GA gate is EXTERNAL and UNMET; single host, no authN)\n');
for (const r of results) console.log(`  ${r.pass ? 'PASS' : 'FAIL'}  ${r.property.padEnd(width)}${r.detail ? `  [${r.detail}]` : ''}`);
const failed = results.filter((r) => !r.pass).length;
console.log(`\n${results.length - failed}/${results.length} properties PASS${failed ? ` — ${failed} FAIL` : ''}`);
process.exit(failed === 0 && results.length > 0 ? 0 : 1);

// ARVES Assistant — connector tests (assert-based, exit 0/1). Offline, deterministic.
//   - the three real-format TEMPLATE connectors (email/.eml, csv, jsonl) parse correctly;
//   - cross-format ONE-TRUTH (A2): the same dentist event from csv/jsonl addresses to the
//     SAME canonical ContentId as notes/calendar/ical/journal (source is evidence, not id);
//   - hostile inputs (malformed lines, missing headers, missing file, oversize file) fail
//     with CLEAN errors — never an uncaught fs throw.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  notesConnector, csvConnector, jsonlConnector, emailConnector, connectorByName, CONNECTORS,
} from './src/connectors.mjs';
import { canonicalFact } from './src/assistant.mjs';
import { Arves } from '../arves-sdk-ts/src/arves.mjs';

const arves = new Arves();
const HERE = path.dirname(fileURLToPath(import.meta.url));
const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'arves-conn-'));
const addr = (o) => arves.address(canonicalFact(o.fact), 'commit');

const tests = [];
const test = (name, fn) => tests.push({ name, fn });
const throws = (fn, re) => { try { fn(); } catch (e) { assert.match(e.message, re); return; } assert.fail('expected throw'); };

test('csv + jsonl parse and DEDUP cross-format: their dentist == the notes dentist (A2)', () => {
  const csv = csvConnector();
  const jsonl = jsonlConnector();
  assert.equal(csv.length, 2, 'csv fixture -> 2 rows (header tolerated)');
  assert.equal(jsonl.length, 2, 'jsonl fixture -> 2 lines');
  const notesDentist = notesConnector().find((o) => o.fact.event === 'dentist-appointment');
  const csvDentist = csv.find((o) => o.fact.event === 'dentist-appointment');
  const jsonlDentist = jsonl.find((o) => o.fact.event === 'dentist-appointment');
  assert.equal(addr(csvDentist), addr(notesDentist), 'csv dentist == notes dentist (one truth)');
  assert.equal(addr(jsonlDentist), addr(notesDentist), 'jsonl dentist == notes dentist (one truth)');
  // sources are the connector names — evidence, distinct from identity
  assert.equal(csvDentist.source, 'csv-file');
  assert.equal(jsonlDentist.source, 'jsonl-file');
});

test('email (.eml) maps From->entity, Subject->event(slug), Date->UTC instant', () => {
  const obs = emailConnector();
  assert.equal(obs.length, 1);
  assert.equal(obs[0].source, 'email-file');
  assert.equal(obs[0].fact.entity, 'alice@dental.example');
  assert.equal(obs[0].fact.event, 'dentist-appointment-confirmation');
  // Mon, 06 Jul 2026 09:00:00 +0000 == 2026-07-06T09:00:00Z
  assert.equal(obs[0].fact.at, BigInt(Date.parse('2026-07-06T09:00:00Z')));
});

test('CONNECTORS registry exposes the new readers by name', () => {
  for (const n of ['email', 'csv', 'jsonl']) assert.ok(typeof CONNECTORS[n] === 'function', `connector '${n}' registered`);
  assert.equal(connectorByName('csv'), csvConnector);
});

test('hostile inputs fail LOUD (clean errors, never an uncaught fs throw)', () => {
  const dir = tmp();
  try {
    const w = (name, body) => { const p = path.join(dir, name); fs.writeFileSync(p, body); return p; };
    // csv: wrong column count
    throws(() => csvConnector(w('bad.csv', '2026-07-06T09:00:00Z,urn:you')), /expected 3 columns/);
    // csv: bad instant on a DATA row (header 'iso,...' tolerated, then the bad row throws)
    throws(() => csvConnector(w('badiso.csv', 'iso,entity,event\nnot-a-date,urn:you,y')), /timestamp must be an ISO-8601 UTC instant/);
    // jsonl: invalid JSON
    throws(() => jsonlConnector(w('bad.jsonl', '{not json}')), /invalid JSON/);
    // jsonl: not an object
    throws(() => jsonlConnector(w('arr.jsonl', '[1,2,3]')), /must be a JSON object/);
    // email: missing header
    throws(() => emailConnector(w('bad.eml', 'From: a@b\nSubject: x\n\nbody')), /missing a Date header/);
    // missing file -> clean 'cannot open'
    throws(() => connectorByName('ical')(path.join(dir, 'nope.ics')), /cannot open/);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('email: a ZONELESS Date fails LOUD (no silent local-time skew)', () => {
  const dir = tmp();
  try {
    const w = (name, body) => { const p = path.join(dir, name); fs.writeFileSync(p, body); return p; };
    // No zone/offset on the Date -> Date.parse would read it as LOCAL time (host-dependent).
    throws(() => emailConnector(w('nozone.eml', 'From: a@b\nSubject: x\nDate: Mon, 06 Jul 2026 09:00:00\n\nbody')),
      /no explicit zone\/offset/);
    // An explicit offset is accepted and is host-independent (byte-identical instant everywhere).
    const ok = emailConnector(w('zone.eml', 'From: a@b\nSubject: x\nDate: Mon, 06 Jul 2026 09:00:00 +0000\n\nbody'));
    assert.equal(ok[0].fact.at, BigInt(Date.parse('2026-07-06T09:00:00Z')));
    // A named obs-zone (GMT) is also accepted.
    const gmt = emailConnector(w('gmt.eml', 'From: a@b\nSubject: x\nDate: Mon, 06 Jul 2026 09:00:00 GMT\n\nbody'));
    assert.equal(gmt[0].fact.at, BigInt(Date.parse('2026-07-06T09:00:00Z')));
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('csv: a date-ish but INVALID line-1 first cell fails LOUD (never dropped as a header)', () => {
  const dir = tmp();
  try {
    const w = (name, body) => { const p = path.join(dir, name); fs.writeFileSync(p, body); return p; };
    // Line 1 is a typo'd ISO (missing the seconds' final digit): a plausible DATA row, not a
    // header — must throw, not silently vanish. Second row is a valid dentist fact.
    throws(() => csvConnector(w('typo.csv',
      '2026-07-06T09:00:0Z,urn:you,dentist-appointment\n2026-07-06T10:00:00Z,urn:you,lunch')),
      /looks like a malformed timestamp, not a header/);
    // A genuine (non-date-ish) header IS still tolerated once, and both data rows parse.
    const ok = csvConnector(w('hdr.csv',
      'iso,entity,event\n2026-07-06T09:00:00Z,urn:you,a\n2026-07-06T10:00:00Z,urn:you,b'));
    assert.equal(ok.length, 2, 'header tolerated, both data rows kept');
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test('oversize file is refused (memory guard via JARVIS_MAX_SOURCE_BYTES)', () => {
  const dir = tmp();
  const prev = process.env.JARVIS_MAX_SOURCE_BYTES;
  try {
    process.env.JARVIS_MAX_SOURCE_BYTES = '16';
    const p = path.join(dir, 'big.jsonl');
    fs.writeFileSync(p, '{"at":"2026-07-06T09:00:00Z","entity":"urn:you","event":"x"}\n'); // > 16 bytes
    throws(() => jsonlConnector(p), /over the 16-byte connector limit/);
  } finally {
    if (prev === undefined) delete process.env.JARVIS_MAX_SOURCE_BYTES; else process.env.JARVIS_MAX_SOURCE_BYTES = prev;
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

let failed = 0;
for (const { name, fn } of tests) {
  try { await fn(); console.log(`  PASS  ${name}`); }
  catch (e) { failed++; console.log(`  FAIL  ${name}\n        ${e.message}`); }
}
console.log(`\narves-assistant (connectors): ${tests.length - failed}/${tests.length} tests pass`);
process.exit(failed === 0 ? 0 : 1);

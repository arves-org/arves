// ARVES Assistant — source connectors (A2: multi-source, one truth).
//
// Three DETERMINISTIC, OFFLINE connectors that read fixed fixture files and map raw
// items to canonical facts `{ entity, event, at }` (at = BigInt ms UTC). In production
// these become live API readers (email, Google Calendar, task systems) — maintainer-side
// wiring per PRODUCT_BRIEF_JARVIS.md OQ-1; the repo ships only deterministic readers so
// tests and demos stay offline and reproducible.
//
// THE LOAD-BEARING RULE: the SAME real-world event seen by DIFFERENT systems maps to the
// SAME canonical fact — the source name is EVIDENCE, never identity. That is what lets
// notes-file + calendar-file collapse the dentist appointment into ONE truth with two
// attesting sources.
//
// Determinism note: fixture timestamps are fixed ISO-8601 UTC instants parsed exactly;
// nothing here calls Date.now(), reads the environment clock, or draws randomness.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(HERE, '../fixtures');

// ROBUSTNESS (hostile-input hardening): every connector reads through readGuarded(),
// which refuses a missing/irregular file with a CLEAN error (never an uncaught throw
// deep in fs) and caps the file size so pointing a connector at a multi-gigabyte file
// fails loudly instead of exhausting memory. The cap is 16 MiB by default and can be
// overridden with JARVIS_MAX_SOURCE_BYTES (also lets tests exercise the guard cheaply).
const DEFAULT_MAX_SOURCE_BYTES = 16 * 1024 * 1024;
function maxSourceBytes() {
  const v = Number(process.env.JARVIS_MAX_SOURCE_BYTES);
  return Number.isFinite(v) && v > 0 ? v : DEFAULT_MAX_SOURCE_BYTES;
}

/** Read a source file's text with a clean error on a missing/irregular/oversize file. */
function readGuarded(file, source) {
  let st;
  try { st = fs.statSync(file); }
  catch (e) { throw new Error(`${source}: cannot open ${file} (${e.code ?? e.message})`); }
  if (!st.isFile()) throw new Error(`${source}: ${file} is not a regular file`);
  const cap = maxSourceBytes();
  if (st.size > cap) {
    throw new Error(`${source}: ${file} is ${st.size} bytes, over the ${cap}-byte connector limit — split it or raise JARVIS_MAX_SOURCE_BYTES`);
  }
  return fs.readFileSync(file, 'utf8');
}

/** Parse one fixture file: `<ISO-8601 UTC> | <entity> | <event>` per line;
 *  '#' comments and blank lines skipped. Returns observations `{ source, fact }`. */
function readSourceFile(file, source) {
  const out = [];
  const text = readGuarded(file, source);
  for (const [i, raw] of text.split('\n').entries()) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) continue;
    const parts = line.split('|').map((s) => s.trim());
    if (parts.length !== 3) throw new Error(`${source}: bad line ${i + 1} in ${file} (want '<iso> | <entity> | <event>')`);
    const [iso, entity, event] = parts;
    // Only full ISO-8601 UTC instants are accepted — Date.parse on this shape is
    // timezone-independent and deterministic on every host.
    if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/.test(iso)) {
      throw new Error(`${source}: line ${i + 1}: timestamp must be an ISO-8601 UTC instant ending in Z, got '${iso}'`);
    }
    const ms = Date.parse(iso);
    if (!Number.isFinite(ms)) throw new Error(`${source}: line ${i + 1}: unparseable instant '${iso}'`);
    out.push({ source, fact: { entity, event, at: BigInt(ms) } });
  }
  return out;
}

/** notes-file connector: the user's personal notes. */
export function notesConnector(file = path.join(FIXTURES, 'notes.txt')) {
  return readSourceFile(file, 'notes-file');
}

/** calendar-file connector: the user's calendar. */
export function calendarConnector(file = path.join(FIXTURES, 'calendar.txt')) {
  return readSourceFile(file, 'calendar-file');
}

/** tasks-file connector: the user's task list. */
export function tasksConnector(file = path.join(FIXTURES, 'tasks.txt')) {
  return readSourceFile(file, 'tasks-file');
}

/** The whole observable world for the scripted day, in deterministic order.
 *  Contains exactly one cross-source duplicate (dentist-appointment in notes + calendar).
 *  NOTE: intentionally the ORIGINAL three connectors only — the two real-format connectors
 *  below are ADDITIVE (their own fixtures) so every existing demo/test stays byte-stable. */
export function allObservations() {
  return [...notesConnector(), ...calendarConnector(), ...tasksConnector()];
}

// ============================================================================
//  REAL-FORMAT connectors (additive) — read file formats a user ALREADY keeps.
// ============================================================================
// Still 100% offline and deterministic (fixed instants, no clock/RNG), but they parse
// REAL formats so the maintainer can point them at their OWN files with ZERO code changes:
//   markdownJournalConnector('/path/to/your/daily-note.md')   // Obsidian / Logseq daily notes
//   icalConnector('/path/to/your/calendar.ics')               // a Google/Apple Calendar export
// The SAME dedup rule holds across formats: the dentist-appointment in journal.md, notes.txt,
// calendar.txt AND calendar.ics (same entity/event/instant) collapses to ONE truth whose
// evidence set names every source that saw it (A2). Source name is evidence, never identity.

/** Shared, validating map of a raw item to a canonical observation `{ source, fact }`.
 *  `iso` MUST be a full ISO-8601 UTC instant ending in Z — timezone-independent, so the
 *  ContentId is identical on every host. Any other shape fails LOUDLY (never a silent skew). */
function observationFrom(source, entity, event, iso, where) {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/.test(iso)) {
    throw new Error(`${source}: ${where}: timestamp must be an ISO-8601 UTC instant ending in Z, got '${iso}'`);
  }
  const ms = Date.parse(iso);
  if (!Number.isFinite(ms)) throw new Error(`${source}: ${where}: unparseable instant '${iso}'`);
  if (typeof entity !== 'string' || entity.length === 0) throw new Error(`${source}: ${where}: empty entity`);
  if (typeof event !== 'string' || event.length === 0) throw new Error(`${source}: ${where}: empty event`);
  return { source, fact: { entity, event, at: BigInt(ms) } };
}

/** markdown-journal connector: parses a real daily-note markdown file.
 *  A `# YYYY-MM-DD` heading sets the current date; each bullet
 *  `- HH:MM <event> [@entity]` (also `*` bullets) becomes a fact at that date+time UTC.
 *  Everything else (prose, other headings) is ignored — real notes have prose in them.
 *  `entity` defaults to `urn:you`. A bullet before any date heading fails loudly. */
export function markdownJournalConnector(file = path.join(FIXTURES, 'journal.md')) {
  const HEADING = /^#\s+(\d{4}-\d{2}-\d{2})\s*$/;
  const BULLET = /^[-*]\s+(\d{2}):(\d{2})\s+(\S+)(?:\s+@(\S+))?\s*$/;
  const out = [];
  let date = null;
  for (const [i, raw] of readGuarded(file, 'markdown-journal').split('\n').entries()) {
    const line = raw.replace(/\r$/, '').trim();
    const h = HEADING.exec(line);
    if (h) { date = h[1]; continue; }
    const b = BULLET.exec(line);
    if (b === null) continue; // prose / other markdown — ignored, not an error
    if (date === null) throw new Error(`markdown-journal: line ${i + 1}: a '- HH:MM <event>' entry appears before any '# YYYY-MM-DD' date heading`);
    const [, hh, mm, event, entity] = b;
    out.push(observationFrom('markdown-journal', entity ?? 'urn:you', event, `${date}T${hh}:${mm}:00Z`, `line ${i + 1}`));
  }
  return out;
}

/** ical-file connector: parses an iCalendar (RFC 5545) file. Each VEVENT's SUMMARY is the
 *  event and DTSTART (UTC basic form `YYYYMMDDTHHMMSSZ`) is the instant; an optional
 *  `X-ARVES-ENTITY:` line names the entity (default `urn:you`). SCOPE (honest): only UTC
 *  DTSTART instants are accepted — a floating/TZID time fails loudly (export as UTC). Line
 *  folding is not un-folded (SUMMARY/DTSTART are expected on one line, as most exporters do). */
export function icalConnector(file = path.join(FIXTURES, 'calendar.ics')) {
  const UTC = /^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z$/;
  const out = [];
  let cur = null;
  for (const raw of readGuarded(file, 'ical-file').split('\n')) {
    const line = raw.replace(/\r$/, '').trim();
    if (line === 'BEGIN:VEVENT') { cur = {}; continue; }
    if (line === 'END:VEVENT') {
      if (cur === null) continue;
      const { dtstart, summary, entity } = cur;
      cur = null;
      if (dtstart === undefined || summary === undefined) {
        throw new Error(`ical-file: a VEVENT is missing DTSTART or SUMMARY (${JSON.stringify({ dtstart, summary })})`);
      }
      const m = UTC.exec(dtstart);
      if (m === null) throw new Error(`ical-file: only UTC DTSTART instants (YYYYMMDDTHHMMSSZ) are supported, got '${dtstart}' — export/convert to UTC`);
      const iso = `${m[1]}-${m[2]}-${m[3]}T${m[4]}:${m[5]}:${m[6]}Z`;
      out.push(observationFrom('ical-file', entity ?? 'urn:you', summary, iso, `event '${summary}'`));
      continue;
    }
    if (cur === null) continue; // outside a VEVENT (calendar headers etc.)
    const c = line.indexOf(':');
    if (c < 0) continue;
    const key = line.slice(0, c).split(';')[0].toUpperCase(); // strip params like ;TZID=
    const val = line.slice(c + 1);
    if (key === 'DTSTART') cur.dtstart = val;
    else if (key === 'SUMMARY') cur.summary = val;
    else if (key === 'X-ARVES-ENTITY') cur.entity = val;
  }
  return out;
}

const ISO_UTC_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$/;

/** email (.eml) connector — a real-world MESSAGE TEMPLATE. Parses RFC 5322 headers
 *  (unfolding continuation lines) and maps a message to ONE fact:
 *    From  -> entity (the address inside <> if present, else the raw From, lowercased)
 *    Subject -> event (slugified)
 *    Date  -> at (an RFC 5322 date WITH an explicit zone/offset -> a UTC instant)
 *  The body is ignored (headers only), so this stays deterministic and offline. SCOPE
 *  (honest): one .eml file = one observation; a maildir/mbox loop is maintainer-side
 *  wiring. A message missing Date/Subject/From, or a Date with no parseable instant,
 *  fails loudly — never a silent skew. This is a TEMPLATE: point it at your own .eml. */
export function emailConnector(file = path.join(FIXTURES, 'message.eml')) {
  const text = readGuarded(file, 'email-file');
  const headerBlock = text.split(/\r?\n\r?\n/)[0];
  const headers = new Map();
  let lastKey = null;
  for (const raw of headerBlock.split(/\r?\n/)) {
    if (/^[ \t]/.test(raw) && lastKey !== null) { headers.set(lastKey, `${headers.get(lastKey)} ${raw.trim()}`); continue; }
    const c = raw.indexOf(':');
    if (c < 0) { lastKey = null; continue; }
    lastKey = raw.slice(0, c).trim().toLowerCase();
    headers.set(lastKey, raw.slice(c + 1).trim());
  }
  const date = headers.get('date'); const subject = headers.get('subject'); const from = headers.get('from');
  for (const [n, v] of [['Date', date], ['Subject', subject], ['From', from]]) {
    if (v === undefined || v === '') throw new Error(`email-file: ${file} is missing a ${n} header`);
  }
  // A ZONELESS RFC 5322 date does NOT fail loud on its own: Date.parse() interprets it as
  // LOCAL time, so the committed instant would silently differ by host timezone — a real
  // determinism break. Require an explicit zone/offset (RFC 5322 zone: +HHMM/-HHMM, or an
  // obs-zone name like GMT/UTC/UT/EST/…, or the military 'Z') and fail LOUD otherwise, so a
  // zoneless date is rejected instead of skewing. This is what the header comment promises.
  if (!/(?:[+-]\d{4}|GMT|UTC|Z|[A-Z]{2,5})\s*$/.test(date)) {
    throw new Error(`email-file: Date header '${date}' has no explicit zone/offset — a zoneless date is parsed as LOCAL time and would skew by host timezone; need an RFC 5322 date with an explicit zone/offset (e.g. +0000)`);
  }
  const ms = Date.parse(date);
  if (!Number.isFinite(ms)) throw new Error(`email-file: unparseable Date header '${date}' — need an RFC 5322 date with an explicit zone/offset`);
  const iso = new Date(ms).toISOString(); // canonical UTC instant (…Z); same instant, host-independent
  const addr = /<([^>]+)>/.exec(from);
  const entity = (addr ? addr[1] : from).trim().toLowerCase();
  const event = subject.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'email';
  return [observationFrom('email-file', entity, event, iso, `message ${file}`)];
}

/** csv-file connector — a generic three-column CSV a user might export:
 *  `<iso>,<entity>,<event>` per row; blank and '#' lines skipped; a leading header row
 *  (whose first cell isn't an ISO instant AND doesn't look like an attempted timestamp)
 *  is tolerated once. SCOPE (honest): simple comma split — no embedded commas or quoted
 *  fields (keep those out of the three columns, or pre-clean). A row without exactly three
 *  columns fails loudly, and so does a date-ish-but-invalid line-1 first cell (a typo'd
 *  timestamp is NOT silently swallowed as a header — see below). */
export function csvConnector(file = path.join(FIXTURES, 'events.csv')) {
  const out = [];
  let dataSeen = false;
  for (const [i, raw] of readGuarded(file, 'csv-file').split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) continue;
    const cols = line.split(',').map((s) => s.trim());
    if (cols.length !== 3) throw new Error(`csv-file: line ${i + 1}: expected 3 columns <iso>,<entity>,<event>, got ${cols.length}`);
    const [iso, entity, event] = cols;
    if (!dataSeen && !ISO_UTC_RE.test(iso)) {
      // Tolerate a leading header row ONCE — but only when line 1's first cell doesn't LOOK
      // like an attempted timestamp. A date-ish first cell (starts YYYY-MM) that fails the
      // strict ISO-UTC test is a typo'd DATA row, not a header; dropping it silently would be
      // data loss (a single-character timestamp typo → a vanished observation), so fail LOUD.
      if (/^\d{4}-\d{2}/.test(iso)) {
        throw new Error(`csv-file: line ${i + 1}: first cell '${iso}' looks like a malformed timestamp, not a header — want a full ISO-8601 UTC instant ending in Z`);
      }
      dataSeen = true; continue; // genuine header row (non-date-ish first cell), tolerated once
    }
    dataSeen = true;
    out.push(observationFrom('csv-file', entity, event, iso, `line ${i + 1}`));
  }
  return out;
}

/** jsonl-file connector — JSON Lines: one JSON object per line, keys `at` (or `iso`/
 *  `timestamp`), `entity`, `event`; blank and '#' lines skipped. Invalid JSON, or a
 *  line that isn't an object, fails loudly with the line number. A real format many
 *  tools emit; point it at your own export. */
export function jsonlConnector(file = path.join(FIXTURES, 'events.jsonl')) {
  const out = [];
  for (const [i, raw] of readGuarded(file, 'jsonl-file').split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (line === '' || line.startsWith('#')) continue;
    let obj;
    try { obj = JSON.parse(line); }
    catch (e) { throw new Error(`jsonl-file: line ${i + 1}: invalid JSON (${e.message})`); }
    if (obj === null || typeof obj !== 'object' || Array.isArray(obj)) {
      throw new Error(`jsonl-file: line ${i + 1}: each line must be a JSON object`);
    }
    const iso = obj.at ?? obj.iso ?? obj.timestamp;
    if (typeof iso !== 'string') throw new Error(`jsonl-file: line ${i + 1}: missing string 'at'/'iso'/'timestamp'`);
    out.push(observationFrom('jsonl-file', obj.entity, obj.event, iso, `line ${i + 1}`));
  }
  return out;
}

/** A named-connector registry so the CLI (and the maintainer) can select a reader by name
 *  and point it at a file: `connectorByName('ical')('/path/to/calendar.ics')`. */
export const CONNECTORS = {
  notes: notesConnector,
  calendar: calendarConnector,
  tasks: tasksConnector,
  journal: markdownJournalConnector,
  ical: icalConnector,
  email: emailConnector,
  csv: csvConnector,
  jsonl: jsonlConnector,
};

/** Resolve a connector by name, or throw with the list of known names. */
export function connectorByName(name) {
  const c = CONNECTORS[name];
  if (c === undefined) throw new Error(`unknown connector '${name}' — known: ${Object.keys(CONNECTORS).sort().join(', ')}`);
  return c;
}

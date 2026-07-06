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

/** Parse one fixture file: `<ISO-8601 UTC> | <entity> | <event>` per line;
 *  '#' comments and blank lines skipped. Returns observations `{ source, fact }`. */
function readSourceFile(file, source) {
  const out = [];
  const text = fs.readFileSync(file, 'utf8');
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
  for (const [i, raw] of fs.readFileSync(file, 'utf8').split('\n').entries()) {
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
  for (const raw of fs.readFileSync(file, 'utf8').split('\n')) {
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

/** A named-connector registry so the CLI (and the maintainer) can select a reader by name
 *  and point it at a file: `connectorByName('ical')('/path/to/calendar.ics')`. */
export const CONNECTORS = {
  notes: notesConnector,
  calendar: calendarConnector,
  tasks: tasksConnector,
  journal: markdownJournalConnector,
  ical: icalConnector,
};

/** Resolve a connector by name, or throw with the list of known names. */
export function connectorByName(name) {
  const c = CONNECTORS[name];
  if (c === undefined) throw new Error(`unknown connector '${name}' — known: ${Object.keys(CONNECTORS).sort().join(', ')}`);
  return c;
}

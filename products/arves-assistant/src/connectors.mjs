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
 *  Contains exactly one cross-source duplicate (dentist-appointment in notes + calendar). */
export function allObservations() {
  return [...notesConnector(), ...calendarConnector(), ...tasksConnector()];
}

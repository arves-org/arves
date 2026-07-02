// Source connectors for Cognitive Memory.
//
// Each connector returns a raw observation in its OWN native shape — exactly as a real
// email/calendar/CRM API would. Three different schemas, describing the SAME real-world
// truth. In production these are live API clients; here they are deterministic fixtures
// (Production-first: the observation shape is the real one, only the transport is stubbed).

// 2026-07-02T15:00:00Z, the one instant all three systems are really talking about.
export const EPOCH_MS = 1751468400000;

/** An email system (RFC-5322 calendar invite). */
export function emailSource() {
  return {
    source: 'email',
    native: 'RFC5322 calendar invite',
    raw: { attendee: 'ada@analytical.example', subject: 'Q3 Review', epochMs: EPOCH_MS },
  };
}

/** A calendar system (Google-style event). */
export function calendarSource() {
  return {
    source: 'calendar',
    native: 'Google Calendar event',
    raw: { attendeeEmail: 'ada@analytical.example', title: 'Q3 Review', epochMs: EPOCH_MS },
  };
}

/** A CRM system (Salesforce-style activity) — different field names, different event label. */
export function crmSource() {
  return {
    source: 'crm',
    native: 'Salesforce activity',
    raw: { contactId: '003AL', contactName: 'Ada Lovelace', activity: 'Q3 Review Meeting', epochMs: EPOCH_MS },
  };
}

/** All three systems reporting the same event. */
export function allSources() { return [emailSource(), calendarSource(), crmSource()]; }

/** Same three, but the CRM disagrees on the time by an hour — a genuine conflict that a
 *  fuzzy merge would silently hide, and that ARVES surfaces as two distinct truths. */
export function conflictingSources() {
  const s = allSources();
  s[2].raw.epochMs = EPOCH_MS + 3600000;
  return s;
}

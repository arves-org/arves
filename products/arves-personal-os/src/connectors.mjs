// P4 Personal Cognitive OS — Reality Acquisition (Office 1).
//
// Each connector turns one slice of a person's real life into observations. In production
// these are live API clients (Gmail, Google Calendar, Slack, GitHub, Plaid, health APIs);
// here they are deterministic fixtures with the real observation shape (Production-first).
//
// The important part: the SAME real-world event seen by DIFFERENT systems produces the
// SAME abstract fact `{ entity, event, at }` — the source is EVIDENCE, not identity. That
// is what lets three systems collapse to one truth (impossible for a stateless chatbot).

const T = 1751468400000; // 2026-07-02T15:00:00Z, the instant the day is anchored to

/** A person's multi-domain reality for one day. */
export function personalReality() {
  return [
    // One meeting, seen by three systems → ONE truth, three independent attestations.
    { source: 'calendar', fact: { entity: 'urn:you', event: 'q3-review', at: T } },
    { source: 'email', fact: { entity: 'urn:you', event: 'q3-review', at: T } },
    { source: 'slack', fact: { entity: 'urn:you', event: 'q3-review', at: T } },
    // Finance signal that will interact with a prior decision.
    { source: 'bank', fact: { entity: 'invest:acme-fund', event: 'price-up-20pct', at: T } },
    // Health signal.
    { source: 'health', fact: { entity: 'urn:you', event: 'low-sleep', at: T } },
    // Work signal.
    { source: 'github', fact: { entity: 'proj:arves', event: 'pr-review-requested', at: T } },
  ];
}

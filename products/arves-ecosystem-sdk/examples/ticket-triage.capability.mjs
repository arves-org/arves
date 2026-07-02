// A THIRD-PARTY capability, authored using ONLY the ARVES Ecosystem Author SDK.
// The author has no ARVES runtime source and never touches it — they implement a contract.
//
// support.triage — deterministic support-ticket triage. Given a raw support ticket
// (subject text, reporter, channel, submitted-at), it derives a priority (P1..P4) and a
// routing queue from keyword signals, and emits a single structured triage fact. It is a
// real capability: a help-desk/SaaS vendor (e.g. "Helply Inc.") would publish exactly this
// to the Marketplace so ARVES can turn inbound tickets into auditable, routed truth.
//
// Purity/determinism: the verdict is a pure function of the ticket text — no clock, no RNG,
// no I/O — so the same ticket always yields the same effect address (certifies & replays).

import { defineCapability } from '../src/kit.mjs';

// Deterministic keyword → priority signal. Order is fixed; first match wins.
const PRIORITY_RULES = [
  { level: 'P1', queue: 'incident',  words: ['outage', 'down', 'breach', 'data loss', 'cannot login'] },
  { level: 'P2', queue: 'billing',   words: ['charged', 'invoice', 'refund', 'payment', 'overcharged'] },
  { level: 'P3', queue: 'support',   words: ['error', 'bug', 'broken', 'not working', 'fails'] },
];
const DEFAULT = { level: 'P4', queue: 'general' };

// Pure classifier: lower-cased scan of subject against the fixed rule table.
function triage(subject) {
  const text = String(subject).toLowerCase();
  for (const rule of PRIORITY_RULES) {
    if (rule.words.some((w) => text.includes(w))) return { level: rule.level, queue: rule.queue };
  }
  return DEFAULT;
}

export const capability = defineCapability({
  name: 'support.triage',
  version: '1.0.0',
  produces: ['uci.fact'],
  // Pure, deterministic: same ticket → same triage fact (so it certifies & replays).
  execute: (input) => {
    const verdict = triage(input.subject);
    return [{
      target: 'uci.fact',
      value: {
        type: 'uci.fact',
        entity: `ticket:${input.reporter}`,
        event: `triage-${verdict.level}-queue-${verdict.queue}`,
        at: BigInt(input.submittedAt) * 1_000_000n,
      },
    }];
  },
});

// Representative inputs the author ships for certification: one per priority band, so the
// certifier exercises every branch of the classifier.
export const testInputs = [
  { reporter: 'alice', subject: 'Production API is DOWN for all users', submittedAt: 1751468400000 },
  { reporter: 'bob',   subject: 'I was overcharged on my last invoice', submittedAt: 1751468400000 },
  { reporter: 'carol', subject: 'Export button is broken and throws an error', submittedAt: 1751468400000 },
  { reporter: 'dave',  subject: 'How do I change my avatar?', submittedAt: 1751468400000 },
];

// The author's source (its bytes are what the artifact signature content-addresses).
export const source = 'support.triage@1.0.0 :: fact(ticket:{reporter}, triage-{level}-queue-{queue}, at)';

export default { capability, testInputs, source };

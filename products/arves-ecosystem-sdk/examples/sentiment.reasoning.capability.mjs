// reasoning.sentiment — an ARVES REASONING capability, authored with ONLY the AI Capability
// SDK. It classifies the sentiment of a piece of text using Providers.reference (the
// DETERMINISTIC reference reasoner) and emits a single structured verdict fact.
//
// This is the AI-Operating-System thesis in miniature: the "model" is a swappable Provider.
// Here it is `reference` (pure, offline, byte-stable) so the capability CERTIFIES and REPLAYS
// with no network and no API key. Swapping in Providers.claude/gpt/gemini would change WHO
// reasons — not the runtime, not the trust boundary, not the recorded-truth guarantee. The
// verdict, once committed, is content-addressed truth; replay reads the record, it never
// re-calls the reasoner (ORCH-003 · ACS-005 GL-012).
//
// Purity/determinism: the verdict is a pure function of the input text (fixed lexicon, no
// clock, no RNG, no I/O) — the same text always yields the same effect address.

import { defineReasoningCapability, Providers } from '../src/reasoning.mjs';

export const capability = defineReasoningCapability({
  name: 'reasoning.sentiment',
  version: '1.0.0',
  produces: ['uci.reasoning.verdict'],
  provider: Providers.reference,
  // The reasoner returns a bare ARVES value; defineReasoningCapability wraps it as an effect
  // on produces[0]. We annotate it with the reviewed entity so the committed truth is
  // self-describing in the ledger.
  reason: (input) => {
    const verdict = Providers.reference.reason(input.text);
    return {
      ...verdict,
      entity: `review:${input.id}`,
    };
  },
});

// Representative inputs the author ships for certification — one per sentiment band, so the
// certifier exercises every branch of the classifier (positive / negative / neutral).
export const testInputs = [
  { id: '1', text: 'This product is great and I love the excellent support, thanks!' },
  { id: '2', text: 'Terrible experience — the app is broken, slow, and I want a refund.' },
  { id: '3', text: 'The package arrived on Tuesday and contained three items.' },
];

// The author's source note (its bytes are what the artifact signature content-addresses).
export const source = 'reasoning.sentiment@1.0.0 :: verdict(review:{id}, label, positive, negative) via Providers.reference';

export default { capability, testInputs, source };

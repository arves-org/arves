// A THIRD-PARTY capability, authored using ONLY the ARVES Ecosystem Author SDK.
// The author has no ARVES runtime source and never touches it — they implement a contract.
// This one extracts a structured invoice fact from a raw invoice (an "Invoice OCR" vendor
// capability, of the kind a company like "Acme Docs Inc." would publish to the Marketplace).

import { defineCapability } from '../src/kit.mjs';

export const capability = defineCapability({
  name: 'invoice.ocr',
  version: '1.0.0',
  produces: ['uci.fact'],
  // Pure, deterministic: same raw invoice → same extracted fact (so it certifies & replays).
  execute: (input) => [{
    target: 'uci.fact',
    value: {
      type: 'uci.fact',
      entity: `invoice:${input.vendor}`,
      event: `amount-usd-${input.amountUsd}`,
      at: BigInt(input.date) * 1_000_000n,
    },
  }],
});

// Representative inputs the author ships for certification.
export const testInputs = [
  { vendor: 'acme', amountUsd: 1234n, date: 1751468400000 },
  { vendor: 'globex', amountUsd: 99n, date: 1751468400000 },
];

// The author's source (its bytes are what the artifact signature content-addresses).
export const source = 'invoice.ocr@1.0.0 :: fact(invoice:{vendor}, amount-usd-{amountUsd}, at)';

export default { capability, testInputs, source };

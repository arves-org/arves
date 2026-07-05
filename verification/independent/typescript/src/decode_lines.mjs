// Line-protocol ACS-002 decoder driver — the TypeScript arm of the 3-way differential fuzzer.
//
// Mirrors the Rust `acs_decode` bin's protocol exactly so the fuzzer can feed the SAME corpus to
// all three independent decoders (Rust / Python / TypeScript) and assert accept/reject agreement:
//   stdin : one lowercase body-hex per line
//   stdout: "ACCEPT\t<canonical_reencode_hex>" | "REJECT\t<reason>" | "ERR\tbad-hex"  (one per line)
//
// Core mode (`enforceNfc: false`): the nfc tier is DEFERRED, exactly like the dependency-free Rust
// reference. So on a non-NFC body this decoder ACCEPTs (as Rust does) while the Python reference
// REJECTs it as `non-nfc-text`; the fuzzer classifies that specific split as the documented nfc
// deferral, not a divergence. Any OTHER accept/reject disagreement, or a reencode-byte mismatch
// among the accepters, is a hard divergence.

import { readFileSync } from 'node:fs';
import { decode, RejectError } from './decode.mjs';
import { encode } from './encode.mjs';
import { toHex, fromHex } from './contentid.mjs';

const out = [];
for (const raw of readFileSync(0, 'utf8').split('\n')) {
  const h = raw.trim();
  if (!h) continue;
  // Validate hex ourselves — the reference fromHex() coerces a non-hex nibble to 0 rather than
  // throwing, which would silently accept a bad line. The fuzz only ever sends valid hex, but the
  // driver must be correct on its own.
  if (h.length % 2 !== 0 || !/^[0-9a-fA-F]+$/.test(h)) {
    out.push('ERR\tbad-hex');
    continue;
  }
  const bytes = fromHex(h);
  try {
    const value = decode(bytes, { enforceNfc: false });
    out.push('ACCEPT\t' + toHex(encode(value)));   // canonical re-encode, byte-comparable across arms
  } catch (e) {
    if (e instanceof RejectError) out.push('REJECT\t' + e.message);
    else out.push('ERR\t' + (e && e.message ? e.message : String(e)));  // a real parser bug → surfaced
  }
}
process.stdout.write(out.join('\n') + '\n');

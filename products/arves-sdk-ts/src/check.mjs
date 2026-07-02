// The SDK is a PRODUCT, but it must itself be standard-conformant: this reproduces the
// ARVES Standard Kit's published golden ContentIds from the logical values. If these
// pass, the SDK's codec agrees byte-for-byte with the Rust/Python/TypeScript runtimes.
// Run: node src/check.mjs

import { encode, contentId, hex, float, DOMAIN } from './codec.mjs';

const dcbor = (tag, value) => hex(contentId(tag, encode(value)));
const raw = (tag, bytes) => hex(contentId(tag, bytes));

const cases = [
  ['ACS-001 hello-truth', () => raw(DOMAIN.commit, new TextEncoder().encode('hello-truth')),
    '122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e'],
  ['ACS-002 V1 uci.fact', () => dcbor(DOMAIN.commit,
    { type: 'uci.fact', claim: 'sky-is-blue', confidence: float(0.5), observed_at: 1730000000000000000n }),
    '12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e'],
  ['ACS-002 V2 engine', () => dcbor(DOMAIN.engine,
    { engine: 'summarize', version: 1n, deterministic: true, reads: ['uci.observation', 'uci.fact'], seed: null }),
    '1220e5aad722341bd0838fb268d73a0a28401457883b9e5e623c05dc0623f57a690d'],
  // V3 fed as DECOMPOSED (NFD) text to prove the SDK normalizes to NFC, not passes through.
  ['ACS-002 V3 nfc+neg', () => dcbor(DOMAIN.trace,
    { label: 'Amélie é—中', n: -1000n }),
    '12207c5367768a3cd0d90b781cac2530335f0310ffc155eac4ac82da80af71e2366a'],
];

let ok = 0;
console.log('ARVES SDK — standard-conformance self-check (reproduces Kit golden ContentIds)');
for (const [name, fn, want] of cases) {
  const got = fn();
  const pass = got === want;
  if (pass) ok++;
  console.log(`  [${pass ? 'PASS' : 'FAIL'}] ${name}${pass ? '' : `\n     want ${want}\n     got  ${got}`}`);
}
console.log(`  ${ok}/${cases.length} ${ok === cases.length ? 'PASS — SDK is ACS-conformant' : 'FAIL'}`);
process.exit(ok === cases.length ? 0 : 1);

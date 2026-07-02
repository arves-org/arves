// ARVES thin conformance runner — the artifact a NEW RUNTIME VENDOR copies.
//
// This is the G2 on-ramp made concrete: a self-contained, dependency-free worked
// example that certifies an ACS implementation against the FROZEN Standard alone
// (standard/vectors/*.tsv). It is deliberately small — the whole point is that a
// new vendor can read it top-to-bottom in one sitting, port the ~200 lines of ACS
// logic to their language, and reproduce this exact verdict with no help.
//
// It performs the two checks that define ACS conformance (standard/conformance/
// CONFORMANCE.md):
//   1. POSITIVE (ACS-001): for every golden row, ContentId(domain, body) must equal
//      the published content_id, byte for byte.
//   2. CORE-REJECT (ACS-002): for every `core` negative row, a canonical decoder must
//      REJECT the input with the exact reason code.
//
// Two implementation choices are shown so a vendor sees both paths:
//   - ADDRESSING (ACS-001) is done by IMPORTING the reference SDK codec
//     (products/arves-sdk-ts/src/codec.mjs) — "reuse a trusted impl."
//   - REJECTION (ACS-002 canonical decoder) is IMPLEMENTED INLINE below from the
//     spec — "write it yourself." A vendor typically writes BOTH themselves; this
//     file shows each style once.
//
// Run:  node verification/independent/reference-runner/run.mjs
// Exit: 0 = CERTIFIED (all positive + all core-reject pass), 1 = NOT CERTIFIED.
//
// Node built-ins only (node:fs, node:path, node:url, node:crypto via the codec).
// No npm install, no network. Offline and hermetic by construction.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// The reference addresser (ACS-001) + hex helper. A vendor may instead implement
// the address themselves — it is exactly: ContentId = 0x12 0x20 || SHA-256(domain || body).
import { contentId, hex } from '../../../products/arves-sdk-ts/src/codec.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const VEC = join(HERE, '..', '..', '..', 'standard', 'vectors');

// --- Vector loading (the FROZEN Standard is the only input) ------------------

function loadTsv(name) {
  const text = readFileSync(join(VEC, name), 'utf8');
  const lines = text.split(/\r?\n/).filter((l) => l.length > 0);
  lines.shift(); // drop the header row
  return lines.map((l) => l.split('\t'));
}

// golden: standard, vector, domain(0xNN), body_hex, content_id
function loadGolden() {
  return loadTsv('acs_golden_vectors.tsv').map(([, , dom, bodyHex, cid]) => ({
    domain: parseInt(dom, 16),
    bodyHex: bodyHex.toLowerCase(),
    cid: cid.toLowerCase(),
  }));
}

// negative: standard, case, tier, input_hex, reject_reason
function loadNegative() {
  return loadTsv('acs_negative_vectors.tsv').map(([, , tier, inputHex, reason]) => ({
    tier,
    inputHex: inputHex.toLowerCase(),
    reason,
  }));
}

function hexToBytes(h) {
  const out = new Uint8Array(h.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(h.slice(i * 2, i * 2 + 2), 16);
  return out;
}

// =============================================================================
// ACS-002 CANONICAL DECODER (inline reference impl, from the spec — no library).
//
// A conformant decoder must REJECT any byte string that is not itself the
// canonical form; otherwise two runtimes could accept different encodings of the
// "same" value and disagree on its address. Each violation throws Rejected with a
// stable reason code (CONFORMANCE.md "Reason codes (ACS-002 §5)").
// =============================================================================

class Rejected extends Error {
  constructor(reason) {
    super(reason);
    this.reason = reason;
  }
}

const MAX_DEPTH = 128; // ACS-002 §5.10 — reject a depth bomb instead of recursing into it.

class Reader {
  constructor(buf) {
    this.buf = buf;
    this.pos = 0;
  }
  take(n) {
    if (this.pos + n > this.buf.length) throw new Rejected('truncated');
    const c = this.buf.subarray(this.pos, this.pos + n);
    this.pos += n;
    return c;
  }
  take1() {
    return this.take(1)[0];
  }
  remaining() {
    return this.buf.length - this.pos;
  }
}

// Read the CBOR argument for additional-info `ai` and return {value, ai}. §5.1/§5.2.
function readArgument(r, ai) {
  if (ai <= 23) return { value: BigInt(ai), ai };
  if (ai === 24) return { value: BigInt(r.take1()), ai };
  if (ai === 25) {
    const b = r.take(2);
    return { value: (BigInt(b[0]) << 8n) | BigInt(b[1]), ai };
  }
  if (ai === 26) {
    const b = r.take(4);
    let v = 0n;
    for (const x of b) v = (v << 8n) | BigInt(x);
    return { value: v, ai };
  }
  if (ai === 27) {
    const b = r.take(8);
    let v = 0n;
    for (const x of b) v = (v << 8n) | BigInt(x);
    return { value: v, ai };
  }
  if (ai === 31) throw new Rejected('indefinite-length');
  throw new Rejected('reserved-or-unsupported'); // ai 28,29,30 reserved
}

// The additional-info a canonical encoder MUST use for `value` (fewest bytes, §5.2).
function shortestAi(value) {
  if (value <= 23n) return Number(value); // inline
  if (value <= 0xffn) return 24;
  if (value <= 0xffffn) return 25;
  if (value <= 0xffffffffn) return 26;
  return 27;
}

// §5.2: reject a longer-than-necessary argument encoding.
function checkShortest(value, ai, reason) {
  if (ai <= 23) return; // inline is always shortest
  const width = { 24: 1, 25: 2, 26: 4, 27: 8 };
  const min = shortestAi(value);
  // A wider sentinel than needed, OR a value that fits inline but used a sentinel.
  if ((min <= 23) || width[ai] > width[min]) throw new Rejected(reason);
}

// Encode a canonical map KEY (Text or Integer) so we can compare key ordering. §5.6.
function encodeKey(key) {
  const out = [];
  if (typeof key === 'string') {
    const b = new TextEncoder().encode(key);
    head(out, 3, BigInt(b.length));
    for (const x of b) out.push(x);
  } else {
    // key is a BigInt (Integer key)
    if (key >= 0n) head(out, 0, key);
    else head(out, 1, -1n - key);
  }
  return Uint8Array.from(out);
}
function head(out, major, u) {
  const m = major << 5;
  if (u < 24n) out.push(m | Number(u));
  else if (u < 0x100n) { out.push(m | 24); out.push(Number(u)); }
  else if (u < 0x10000n) { out.push(m | 25); out.push(Number((u >> 8n) & 0xffn), Number(u & 0xffn)); }
  else if (u < 0x100000000n) {
    out.push(m | 26);
    for (let s = 24n; s >= 0n; s -= 8n) out.push(Number((u >> s) & 0xffn));
  } else {
    out.push(m | 27);
    for (let s = 56n; s >= 0n; s -= 8n) out.push(Number((u >> s) & 0xffn));
  }
}
function cmpBytes(a, b) {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return a.length - b.length;
}

function decodeItem(r, depth) {
  if (depth > MAX_DEPTH) throw new Rejected('nesting-too-deep'); // §5.10
  const ib = r.take1();
  const major = ib >> 5;
  const ai = ib & 0x1f;

  if (major === 0) { // unsigned int (§5.2)
    if (ai === 31) throw new Rejected('indefinite-length');
    const { value, ai: used } = readArgument(r, ai);
    checkShortest(value, used, 'non-shortest-int');
    return value;
  }
  if (major === 1) { // negative int (§5.2)
    if (ai === 31) throw new Rejected('indefinite-length');
    const { value, ai: used } = readArgument(r, ai);
    checkShortest(value, used, 'non-shortest-int');
    return -1n - value;
  }
  if (major === 2) { // byte string (§5.5)
    if (ai === 31) throw new Rejected('indefinite-length');
    const { value, ai: used } = readArgument(r, ai);
    checkShortest(value, used, 'non-shortest-len');
    return r.take(Number(value));
  }
  if (major === 3) { // text string (§5.4)
    if (ai === 31) throw new Rejected('indefinite-length');
    const { value, ai: used } = readArgument(r, ai);
    checkShortest(value, used, 'non-shortest-len');
    const octets = r.take(Number(value));
    // §5.4: text is UTF-8. Reject non-UTF-8 (not in the §4 model).
    let s;
    try {
      s = new TextDecoder('utf-8', { fatal: true }).decode(octets);
    } catch {
      throw new Rejected('reserved-or-unsupported');
    }
    // §5.4: a canonical body's text is NFC. Node has String.prototype.normalize,
    // so this runner ENFORCES the nfc tier. A runtime with no Unicode facility MAY
    // defer THIS ONE rule (declare it — never silently accept). See CONFORMANCE.md.
    if (s.normalize('NFC') !== s) throw new Rejected('non-nfc-text');
    return s;
  }
  if (major === 4) { // array (§5.8)
    if (ai === 31) throw new Rejected('indefinite-length');
    const { value, ai: used } = readArgument(r, ai);
    checkShortest(value, used, 'non-shortest-len');
    const n = Number(value);
    const items = [];
    for (let i = 0; i < n; i++) items.push(decodeItem(r, depth + 1));
    return items;
  }
  if (major === 5) { // map (§5.6)
    if (ai === 31) throw new Rejected('indefinite-length');
    const { value, ai: used } = readArgument(r, ai);
    checkShortest(value, used, 'non-shortest-len');
    return decodeMap(r, Number(value), depth);
  }
  if (major === 6) throw new Rejected('reserved-or-unsupported'); // tags not in §4 model
  if (major === 7) { // simple values / floats (§5.3)
    if (ai === 20 || ai === 21) return ai === 21; // false / true
    if (ai === 22) return null; // null
    if (ai === 23) throw new Rejected('reserved-or-unsupported'); // undefined (0xf7)
    if (ai === 24) { r.take1(); throw new Rejected('reserved-or-unsupported'); } // 1-byte simple
    if (ai === 25) { r.take(2); throw new Rejected('float-not-float64'); } // half
    if (ai === 26) { r.take(4); throw new Rejected('float-not-float64'); } // single
    if (ai === 27) return decodeFloat64(r); // binary64
    if (ai === 31) throw new Rejected('indefinite-length'); // stray break 0xff
    throw new Rejected('reserved-or-unsupported'); // reserved/unassigned simple
  }
  throw new Rejected('reserved-or-unsupported');
}

function decodeFloat64(r) {
  const raw = r.take(8);
  const dv = new DataView(raw.buffer, raw.byteOffset, 8);
  const x = dv.getFloat64(0, false);
  if (Number.isNaN(x) || x === Infinity || x === -Infinity) throw new Rejected('non-finite-float'); // §5.3
  // §5.3: -0.0 is non-canonical (canonical zero is fb0000000000000000).
  if (dv.getUint32(0, false) === 0x80000000 && dv.getUint32(4, false) === 0) {
    throw new Rejected('negative-zero-float');
  }
  return x;
}

function decodeMap(r, n, depth) {
  let prevKey = null;
  for (let i = 0; i < n; i++) {
    const key = decodeItem(r, depth + 1); // a key is itself a full canonical item
    if (typeof key !== 'string' && typeof key !== 'bigint') {
      throw new Rejected('reserved-or-unsupported'); // §4 kind 8: keys are Text or Integer
    }
    const keyEnc = encodeKey(key);
    if (prevKey !== null) {
      const c = cmpBytes(keyEnc, prevKey);
      if (c === 0) throw new Rejected('duplicate-map-keys'); // §5.6
      if (c < 0) throw new Rejected('unsorted-map-keys'); // §5.6 bytewise-ascending
    }
    prevKey = keyEnc;
    decodeItem(r, depth + 1); // value
  }
  return {};
}

// Decode a canonical body and validate canonical form (§5, §6.3). §5.9: exactly one
// top-level item; any remaining byte is trailing data.
function decode(buf) {
  const r = new Reader(buf);
  const v = decodeItem(r, 0);
  if (r.remaining() !== 0) throw new Rejected('trailing-data');
  return v;
}

// --- The two conformance checks ---------------------------------------------

function checkPositive(golden) {
  let ok = 0;
  for (const g of golden) {
    const got = hex(contentId(g.domain, hexToBytes(g.bodyHex)));
    if (got === g.cid) ok++;
  }
  return ok;
}

function checkReject(rows) {
  // Returns {ok, total} over the `core` tier only (the interoperability gate).
  const core = rows.filter((r) => r.tier === 'core');
  let ok = 0;
  for (const row of core) {
    try {
      decode(hexToBytes(row.inputHex));
      // accepted — a conformant decoder must have rejected it.
    } catch (e) {
      if (e instanceof Rejected && e.reason === row.reason) ok++;
    }
  }
  return { ok, total: core.length };
}

// --- Main --------------------------------------------------------------------

function main() {
  const golden = loadGolden();
  const negative = loadNegative();

  const pos = checkPositive(golden);
  const { ok: coreOk, total: coreTotal } = checkReject(negative);

  const certified = pos === golden.length && coreOk === coreTotal;

  console.log('ARVES thin conformance runner — certifying against the frozen Standard alone');
  console.log('='.repeat(76));
  console.log(`  positive ${pos}/${golden.length}  (ACS-001 ContentId reproduced from domain+body)`);
  console.log(`  core-reject ${coreOk}/${coreTotal}  (ACS-002 non-canonical inputs rejected with the right reason)`);
  console.log('-'.repeat(76));
  console.log(`  VERDICT: ${certified ? 'CERTIFIED' : 'NOT CERTIFIED'} (ACS core)`);
  if (certified) {
    console.log('  Reproduced every golden address and rejected every core negative — no maintainer required.');
  }
  return certified ? 0 : 1;
}

process.exit(main());

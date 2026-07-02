// ARVES SDK — canonical ACS codec (content addressing + deterministic dCBOR encode).
//
// This is PRODUCT code (IDR-006): it CONSUMES the ARVES standard (arves-standard-kit
// 0.2.0) by implementing ACS-001 (content address) + ACS-002 (canonical serialization)
// exactly as the Kit specifies. It modifies no platform file. It is verified conformant
// by `src/check.mjs` (it reproduces the Kit's published golden ContentIds).
//
// Node built-in SHA-256 only (a platform primitive); the dCBOR profile is hand-written
// from ACS-002 so the SDK carries no third-party serialization dependency.

import { createHash } from 'node:crypto';

/** Marks a value as an ACS Float (binary64). Integers are BigInt; the two are distinct
 *  ACS value kinds (ACS-002 §4) and MUST NOT be conflated. */
export class Flt {
  constructor(v) { this.v = v; }
}
export const float = (v) => new Flt(v);

/** ACS-001 domain-tag registry (ACS-001 §4). */
export const DOMAIN = {
  commit: 0x01, engine: 0x02, capability: 0x03, invocation: 0x04,
  trace: 0x05, envelope: 0x06, schema: 0x07,
};

function pushBE(out, u, n) {
  const b = new Array(n);
  for (let i = n - 1; i >= 0; i--) { b[i] = Number(u & 0xffn); u >>= 8n; }
  for (const x of b) out.push(x);
}

// Shortest-form major-type head (ACS-002 §5.1/§5.2); `u` is a non-negative BigInt.
function head(out, major, u) {
  const m = major << 5;
  if (u < 24n) out.push(m | Number(u));
  else if (u < 0x100n) { out.push(m | 24); out.push(Number(u)); }
  else if (u < 0x10000n) { out.push(m | 25); pushBE(out, u, 2); }
  else if (u < 0x100000000n) { out.push(m | 26); pushBE(out, u, 4); }
  else { out.push(m | 27); pushBE(out, u, 8); }
}

function encFloat(out, x) {
  if (!Number.isFinite(x)) throw new Error('ARVES: NaN/±Infinity are not canonical (ACS-002 §5.3)');
  const dv = new DataView(new ArrayBuffer(8));
  dv.setFloat64(0, x, false); // big-endian binary64
  if (dv.getUint32(0, false) === 0x80000000 && dv.getUint32(4, false) === 0) {
    dv.setUint32(0, 0, false); // -0.0 -> +0.0 (§5.3)
  }
  out.push(0xfb);
  for (let i = 0; i < 8; i++) out.push(dv.getUint8(i));
}

function cmp(a, b) {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) if (a[i] !== b[i]) return a[i] - b[i];
  return a.length - b.length;
}

function encMap(out, v, depth) {
  const entries = v instanceof Map ? [...v.entries()] : Object.entries(v);
  const parts = entries.map(([k, val]) => {
    if (typeof k !== 'string' && typeof k !== 'bigint') {
      throw new Error('ARVES: a map key MUST be a string or a BigInt (ACS-002 §4 kind 8)');
    }
    const ke = []; enc(ke, k, depth + 1);
    const ve = []; enc(ve, val, depth + 1);
    return { ke: Uint8Array.from(ke), ve: Uint8Array.from(ve) };
  });
  parts.sort((a, b) => cmp(a.ke, b.ke)); // §5.6 bytewise-sorted encoded keys
  for (let i = 1; i < parts.length; i++) {
    if (cmp(parts[i].ke, parts[i - 1].ke) === 0) throw new Error('ARVES: duplicate map key (§5.6)');
  }
  head(out, 5, BigInt(parts.length));
  for (const p of parts) { for (const x of p.ke) out.push(x); for (const x of p.ve) out.push(x); }
}

// ACS-002 §5.10: bound structural nesting so a hostile or cyclic value fails cleanly
// with a typed error instead of overflowing the JS stack (mirrors the reference
// decoder's MAX_DEPTH — a conformant body is never deeper than this).
const MAX_DEPTH = 128;

function enc(out, v, depth) {
  if (depth > MAX_DEPTH) {
    throw new Error('ARVES: nesting exceeds MAX_DEPTH=128 (ACS-002 §5.10)');
  }
  if (v === undefined) {
    // §5.7: null-vs-absent are distinct. `undefined` is neither an ACS value nor a safe
    // stand-in for null — silently mapping it to null would change the content address.
    throw new Error('ARVES: undefined is not an ACS value — use null explicitly (ACS-002 §5.7)');
  }
  if (v === null) { out.push(0xf6); return; }
  if (typeof v === 'boolean') { out.push(v ? 0xf5 : 0xf4); return; }
  if (typeof v === 'bigint') {
    // §4: the Integer model is exactly [-2^64, 2^64-1]. Outside it, head() would
    // silently wrap mod 2^64 and produce a WRONG address — reject instead.
    if (v > (1n << 64n) - 1n || v < -(1n << 64n)) {
      throw new Error('ARVES: Integer out of ACS-002 §4 range [-2^64, 2^64-1]');
    }
    if (v >= 0n) head(out, 0, v);
    else head(out, 1, -1n - v);
    return;
  }
  if (v instanceof Flt) { encFloat(out, v.v); return; }
  if (typeof v === 'number') {
    // Deliberate: ACS-002 §5.2 mandates an exact integer carrier and forbids float/int
    // ambiguity. A bare JS number is both ambiguous and lossy beyond 2^53.
    throw new Error('ARVES: pass integers as BigInt (e.g. 42n) and floats as arves.float(x) — '
      + 'a bare number is ambiguous (int vs float) and unsafe beyond 2^53 (ACS-002 §5.2/§5.3)');
  }
  if (typeof v === 'string') {
    const b = new TextEncoder().encode(v.normalize('NFC')); // §5.4 NFC
    head(out, 3, BigInt(b.length));
    for (const x of b) out.push(x);
    return;
  }
  if (v instanceof Uint8Array) { head(out, 2, BigInt(v.length)); for (const x of v) out.push(x); return; }
  if (Array.isArray(v)) { head(out, 4, BigInt(v.length)); for (const it of v) enc(out, it, depth + 1); return; }
  if (typeof v === 'object') { encMap(out, v, depth); return; }
  throw new Error('ARVES: unsupported value kind');
}

/** Canonical dCBOR body of an ARVES value (ACS-002/1). */
export function encode(v) { const out = []; enc(out, v, 0); return Uint8Array.from(out); }

export function sha256(bytes) { return new Uint8Array(createHash('sha256').update(bytes).digest()); }

/** ACS-001 content address: `0x12 0x20 || SHA-256(domain_tag || body)`. */
export function contentId(domainTag, body) {
  const pre = new Uint8Array(1 + body.length);
  pre[0] = domainTag; pre.set(body, 1);
  const d = sha256(pre);
  const id = new Uint8Array(2 + d.length);
  id[0] = 0x12; id[1] = 0x20; id.set(d, 2);
  return id;
}

export function hex(bytes) {
  let s = '';
  for (const b of bytes) s += b.toString(16).padStart(2, '0');
  return s;
}

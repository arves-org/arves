// ACS-002/1 canonical dCBOR decoder / validator.
//
// A conformant decoder MUST reject every non-canonical encoding (ACS-002 §6.3)
// with a stable reason code (CONFORMANCE.md). This decoder decodes the ACS-002
// base value model and rejects anything else. It is hostile-input-safe
// (ACS-002 §11): it enforces MAX_DEPTH *before* recursing and bounds-checks
// every declared length against the remaining input before reading.
//
// Reason codes (CONFORMANCE.md / ACS-002 §5):
//   non-shortest-int, non-shortest-len, indefinite-length, unsorted-map-keys,
//   duplicate-map-keys, float-not-float64, negative-zero-float,
//   non-finite-float, trailing-data, reserved-or-unsupported, truncated,
//   nesting-too-deep, non-nfc-text

import { NULL, Bool, Int, Float, Text, Bytes, Arr, Map_ } from './value.mjs';
import { encode, cmpBytes, MAX_DEPTH } from './encode.mjs';

export class RejectError extends Error {
  constructor(reason) {
    super(reason);
    this.reason = reason;
  }
}

// Options: { enforceNfc: boolean }
//   enforceNfc=true  -> full conformance (reject non-NFC text)
//   enforceNfc=false -> core conformance (defer the nfc tier; do NOT accept
//                       silently as canonical — callers treat "core" specially)
class Reader {
  constructor(bytes, opts) {
    this.b = bytes;
    this.pos = 0;
    this.opts = opts || {};
  }

  need(n) {
    if (this.pos + n > this.b.length) throw new RejectError('truncated');
  }

  readByte() {
    this.need(1);
    return this.b[this.pos++];
  }

  // Read the argument for a given additional-information value `ai`, enforcing
  // shortest-form. `shortReason` distinguishes int (non-shortest-int) from
  // length-prefixed items (non-shortest-len). Returns a BigInt.
  readArg(ai, shortReason) {
    if (ai < 24) {
      return BigInt(ai);
    }
    if (ai === 24) {
      this.need(1);
      const v = this.b[this.pos];
      this.pos += 1;
      // shortest: values 0..23 MUST be inline, not in a 1-byte arg.
      if (v < 24) throw new RejectError(shortReason);
      return BigInt(v);
    }
    if (ai === 25) {
      this.need(2);
      const v = (this.b[this.pos] << 8) | this.b[this.pos + 1];
      this.pos += 2;
      // shortest: must not fit in the next-smaller width (<= 0xff).
      if (v <= 0xff) throw new RejectError(shortReason);
      return BigInt(v);
    }
    if (ai === 26) {
      this.need(4);
      const v =
        (BigInt(this.b[this.pos]) << 24n) |
        (BigInt(this.b[this.pos + 1]) << 16n) |
        (BigInt(this.b[this.pos + 2]) << 8n) |
        BigInt(this.b[this.pos + 3]);
      this.pos += 4;
      if (v <= 0xffffn) throw new RejectError(shortReason);
      return v;
    }
    if (ai === 27) {
      this.need(8);
      let v = 0n;
      for (let i = 0; i < 8; i++) v = (v << 8n) | BigInt(this.b[this.pos + i]);
      this.pos += 8;
      if (v <= 0xffffffffn) throw new RejectError(shortReason);
      return v;
    }
    // ai 28, 29, 30 are reserved; ai 31 is indefinite-length.
    if (ai === 31) throw new RejectError('indefinite-length');
    throw new RejectError('reserved-or-unsupported'); // 28,29,30
  }

  // Decode exactly one data item at the given structural depth.
  decodeItem(depth) {
    const ib = this.readByte();
    const major = ib >> 5;
    const ai = ib & 0x1f;

    switch (major) {
      case 0: {
        // unsigned integer
        const n = this.readArg(ai, 'non-shortest-int');
        return Int(n);
      }
      case 1: {
        // negative integer: value = -1 - arg
        const arg = this.readArg(ai, 'non-shortest-int');
        return Int(-1n - arg);
      }
      case 2: {
        // byte string
        if (ai === 31) throw new RejectError('indefinite-length');
        const len = this.readArg(ai, 'non-shortest-len');
        const n = this.checkLen(len);
        const slice = this.b.slice(this.pos, this.pos + n);
        this.pos += n;
        return Bytes(slice);
      }
      case 3: {
        // text string
        if (ai === 31) throw new RejectError('indefinite-length');
        const len = this.readArg(ai, 'non-shortest-len');
        const n = this.checkLen(len);
        const slice = this.b.slice(this.pos, this.pos + n);
        this.pos += n;
        return this.decodeText(slice);
      }
      case 4: {
        // array
        if (ai === 31) throw new RejectError('indefinite-length');
        if (depth + 1 > MAX_DEPTH) throw new RejectError('nesting-too-deep'); // §5.10 before recursing
        const len = this.readArg(ai, 'non-shortest-len');
        const n = this.checkCount(len);
        const items = [];
        for (let i = 0; i < n; i++) items.push(this.decodeItem(depth + 1));
        return Arr(items);
      }
      case 5: {
        // map
        if (ai === 31) throw new RejectError('indefinite-length');
        if (depth + 1 > MAX_DEPTH) throw new RejectError('nesting-too-deep'); // §5.10 before recursing
        const len = this.readArg(ai, 'non-shortest-len');
        const n = this.checkCount(len);
        return this.decodeMap(n, depth + 1);
      }
      case 6: {
        // tag — NOT in the ACS-002 base model (§4)
        throw new RejectError('reserved-or-unsupported');
      }
      case 7: {
        return this.decodeSimpleOrFloat(ai);
      }
      default:
        throw new RejectError('reserved-or-unsupported');
    }
  }

  // Bounds-check a declared byte length against remaining input (ACS-002 §11).
  checkLen(lenBig) {
    const remaining = BigInt(this.b.length - this.pos);
    if (lenBig > remaining) throw new RejectError('truncated');
    return Number(lenBig);
  }

  // Bounds-check a declared element/entry count. A count that alone exceeds the
  // remaining bytes cannot be satisfied (each element is >=1 byte).
  checkCount(lenBig) {
    const remaining = BigInt(this.b.length - this.pos);
    if (lenBig > remaining) throw new RejectError('truncated');
    return Number(lenBig);
  }

  decodeText(slice) {
    // Must be valid UTF-8. Node's TextDecoder with fatal=true rejects invalid
    // sequences -> reserved-or-unsupported (non-UTF-8 text octets are not in
    // the §4 model; CONFORMANCE.md maps this to reserved-or-unsupported).
    let str;
    try {
      str = new TextDecoder('utf-8', { fatal: true }).decode(slice);
    } catch {
      throw new RejectError('reserved-or-unsupported');
    }
    // §5.4: reject non-NFC text. Deferred at core tier.
    if (this.opts.enforceNfc) {
      if (str.normalize('NFC') !== str) throw new RejectError('non-nfc-text');
    }
    return Text(str);
  }

  decodeMap(n, depth) {
    const entries = [];
    const encodedKeys = [];
    for (let i = 0; i < n; i++) {
      const keyStart = this.pos;
      const key = this.decodeItem(depth);
      const keyBytes = this.b.slice(keyStart, this.pos);
      // §4 kind 8: keys MUST be Text or Integer.
      if (key.kind !== 'text' && key.kind !== 'int') {
        throw new RejectError('reserved-or-unsupported');
      }
      const val = this.decodeItem(depth);
      entries.push([key, val]);
      encodedKeys.push(keyBytes);
    }
    // §5.6: keys MUST be in bytewise-sorted order with no duplicates.
    for (let i = 1; i < encodedKeys.length; i++) {
      const c = cmpBytes(encodedKeys[i - 1], encodedKeys[i]);
      if (c === 0) throw new RejectError('duplicate-map-keys');
      if (c > 0) throw new RejectError('unsorted-map-keys');
    }
    return Map_(entries);
  }

  decodeSimpleOrFloat(ai) {
    switch (ai) {
      case 20:
        return Bool(false); // 0xf4
      case 21:
        return Bool(true); // 0xf5
      case 22:
        return NULL; // 0xf6
      case 23:
        // undefined (0xf7) — not in §4 model
        throw new RejectError('reserved-or-unsupported');
      case 24:
        // simple value in following byte — not in §4 model
        throw new RejectError('reserved-or-unsupported');
      case 25:
        // half-precision float — not float64 (§5.3)
        this.need(2);
        this.pos += 2;
        throw new RejectError('float-not-float64');
      case 26:
        // single-precision float — not float64 (§5.3)
        this.need(4);
        this.pos += 4;
        throw new RejectError('float-not-float64');
      case 27: {
        // binary64
        this.need(8);
        const dv = new DataView(this.b.buffer, this.b.byteOffset + this.pos, 8);
        const x = dv.getFloat64(0, false);
        // Inspect the raw bits for -0.0 and non-finite before returning.
        const hi = this.b[this.pos];
        this.pos += 8;
        if (!Number.isFinite(x)) throw new RejectError('non-finite-float'); // NaN/Inf
        if (Object.is(x, -0)) throw new RejectError('negative-zero-float');
        // (hi guards nothing extra here; Object.is handles -0 detection.)
        void hi;
        return Float(x);
      }
      case 31:
        // top-level or stray break stop code -> indefinite-length
        throw new RejectError('indefinite-length');
      default:
        // simple values 0..19 (ai<20) and any other -> not in §4 model
        throw new RejectError('reserved-or-unsupported');
    }
  }
}

// decode(bytes, opts) -> ARVES value.  Throws RejectError on any non-canonical
// input. Enforces §5.9 (no trailing data) at the top level.
export function decode(bytes, opts = {}) {
  const r = new Reader(bytes, opts);
  const v = r.decodeItem(0);
  if (r.pos !== bytes.length) throw new RejectError('trailing-data');
  return v;
}

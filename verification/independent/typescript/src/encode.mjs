// ACS-002/1 canonical dCBOR encoder.
//
// Implements the ACS-002 §5 canonical encoding rules from scratch:
//   §5.1 definite lengths only
//   §5.2 integers — shortest form
//   §5.3 floats — fixed binary64, finite only, -0.0 -> +0.0
//   §5.4 text — UTF-8, NFC
//   §5.5 byte strings — verbatim
//   §5.6 maps — bytewise-sorted encoded keys, no duplicates
//   §5.8 arrays — order preserved
//   §5.10 max nesting depth 128 (also enforced on encode for symmetry)
//
// This is NOT a generic CBOR library: it emits exactly the ACS-002 profile.

const MAX_DEPTH = 128; // ACS-002 §5.10

// --- low-level append helpers ---------------------------------------------

class Buf {
  constructor() {
    this.parts = [];
  }
  push(u8) {
    this.parts.push(u8);
  }
  concat() {
    let len = 0;
    for (const p of this.parts) len += p.length;
    const out = new Uint8Array(len);
    let off = 0;
    for (const p of this.parts) {
      out.set(p, off);
      off += p.length;
    }
    return out;
  }
}

// Encode a CBOR head: major type (0..7) in the top 3 bits, plus the argument
// in the shortest additional-information form (ACS-002 §5.2 / RFC 8949 §4.2.1).
// `arg` is a BigInt in [0, 2^64-1].
function head(major, arg) {
  const mt = major << 5;
  if (arg < 0n || arg > 0xffffffffffffffffn) {
    throw new Error(`argument out of CBOR range: ${arg}`);
  }
  if (arg <= 23n) {
    return new Uint8Array([mt | Number(arg)]);
  } else if (arg <= 0xffn) {
    return new Uint8Array([mt | 24, Number(arg)]);
  } else if (arg <= 0xffffn) {
    const n = Number(arg);
    return new Uint8Array([mt | 25, (n >> 8) & 0xff, n & 0xff]);
  } else if (arg <= 0xffffffffn) {
    const n = Number(arg);
    return new Uint8Array([mt | 26, (n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]);
  } else {
    // 8-byte argument, big-endian
    const out = new Uint8Array(9);
    out[0] = mt | 27;
    let v = arg;
    for (let i = 8; i >= 1; i--) {
      out[i] = Number(v & 0xffn);
      v >>= 8n;
    }
    return out;
  }
}

function encodeInto(v, buf, depth) {
  switch (v.kind) {
    case 'null':
      buf.push(new Uint8Array([0xf6])); // major 7, simple 22
      return;
    case 'bool':
      buf.push(new Uint8Array([v.value ? 0xf5 : 0xf4])); // simple 21 / 20
      return;
    case 'int': {
      const n = v.value; // BigInt
      if (n >= 0n) {
        buf.push(head(0, n));
      } else {
        // major 1 with argument -1 - n (ACS-002 §5.2)
        buf.push(head(1, -1n - n));
      }
      return;
    }
    case 'float': {
      const x = v.value;
      // §5.3: reject non-finite; normalize -0.0 -> +0.0.
      if (!Number.isFinite(x)) {
        throw new Error('non-finite-float: encoder MUST NOT emit NaN/Inf');
      }
      let out = x;
      if (Object.is(out, -0)) out = 0; // -0.0 -> +0.0
      const b = new Uint8Array(9);
      b[0] = 0xfb; // major 7, ai 27 -> binary64
      const dv = new DataView(b.buffer);
      dv.setFloat64(1, out, false); // big-endian (network byte order)
      buf.push(b);
      return;
    }
    case 'text': {
      // §5.4: NFC-normalize, then UTF-8.
      const nfc = v.value.normalize('NFC');
      const utf8 = new TextEncoder().encode(nfc);
      buf.push(head(3, BigInt(utf8.length)));
      buf.push(utf8);
      return;
    }
    case 'bytes': {
      buf.push(head(2, BigInt(v.value.length)));
      buf.push(v.value);
      return;
    }
    case 'array': {
      if (depth + 1 > MAX_DEPTH) throw new Error('nesting-too-deep');
      buf.push(head(4, BigInt(v.value.length)));
      for (const el of v.value) encodeInto(el, buf, depth + 1);
      return;
    }
    case 'map': {
      if (depth + 1 > MAX_DEPTH) throw new Error('nesting-too-deep');
      // §5.6: sort entries by bytewise lexicographic order of each key's own
      // canonical dCBOR encoding; reject duplicate encoded keys.
      const encoded = v.value.map(([k, val]) => {
        if (k.kind !== 'text' && k.kind !== 'int') {
          throw new Error('map key must be Text or Integer (ACS-002 §4 kind 8)');
        }
        const kb = new Buf();
        encodeInto(k, kb, depth + 1);
        return { key: kb.concat(), val };
      });
      encoded.sort((a, b) => cmpBytes(a.key, b.key));
      for (let i = 1; i < encoded.length; i++) {
        if (cmpBytes(encoded[i - 1].key, encoded[i].key) === 0) {
          throw new Error('duplicate-map-keys');
        }
      }
      buf.push(head(5, BigInt(encoded.length)));
      for (const { key, val } of encoded) {
        buf.push(key);
        encodeInto(val, buf, depth + 1);
      }
      return;
    }
    default:
      throw new Error(`unknown value kind: ${v.kind}`);
  }
}

function cmpBytes(a, b) {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return a.length - b.length;
}

// canon(value) -> Uint8Array  (ACS-002 canonical body)
export function encode(v) {
  const buf = new Buf();
  encodeInto(v, buf, 0);
  return buf.concat();
}

export { cmpBytes, MAX_DEPTH };

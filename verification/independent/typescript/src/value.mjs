// ARVES ACS value model (ACS-002 §4).
//
// An ARVES value is exactly one of eight kinds. To represent them
// unambiguously in JavaScript (which conflates int/float and has no native
// "null present vs absent" distinction at the value layer), we use tagged
// wrapper objects. This keeps the Integer-vs-Float distinction (ACS-002 §4,
// which SHALL NOT be conflated) explicit, and lets Integers hold the full
// [-2^64, 2^64-1] range exactly via BigInt.
//
// Kinds: Null | Bool | Integer | Float | Text | Bytes | Array | Map

export const NULL = { kind: 'null' };

export function Bool(b) {
  return { kind: 'bool', value: !!b };
}

// Integer holds a BigInt so a 64-bit nanosecond timestamp survives exactly.
export function Int(n) {
  return { kind: 'int', value: BigInt(n) };
}

// Float is an IEEE-754 binary64 JS number.
export function Float(x) {
  return { kind: 'float', value: Number(x) };
}

export function Text(s) {
  return { kind: 'text', value: String(s) };
}

// Bytes wraps a Uint8Array.
export function Bytes(u8) {
  return { kind: 'bytes', value: u8 instanceof Uint8Array ? u8 : new Uint8Array(u8) };
}

// Array of ARVES values (order significant).
export function Arr(items) {
  return { kind: 'array', value: items };
}

// Map: array of [key, value] entries. key is a Text or Int value.
// Author order is irrelevant; the encoder sorts by encoded-key bytes.
export function Map_(entries) {
  return { kind: 'map', value: entries };
}

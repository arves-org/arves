// ACS-001 Content Addressing.
//
//   ContentId = 0x12 0x20 || SHA-256(domain_tag || body)
//
// A self-describing SHA-256 multihash (hash_code 0x12, digest_len 0x20 = 32)
// of the domain-tagged pre-image `domain_tag || body` (ACS-001 §5). SHA-256 is
// a platform primitive (node:crypto); everything else is implemented here.

import { createHash } from 'node:crypto';

// ACS-001 §4 domain-tag registry (extended by ACS-003 0x06, ACS-004 0x07,
// ACS-005 0x08/0x09; ACS-001 is the single registry authority — Batch 1 report).
export const DOMAIN = {
  COMMIT_CONTENT: 0x01,
  ENGINE_MANIFEST: 0x02,
  CAPABILITY_MANIFEST: 0x03,
  INVOCATION: 0x04,
  DECISION_TRACE: 0x05,
  CANONICAL_ENVELOPE: 0x06,
  TYPE_SCHEMA: 0x07,
  GLOSSARY_TERM_SET: 0x08,
  REQUIREMENT_CLAUSE: 0x09,
};

// contentId(domainTag, body) -> Uint8Array (34 bytes: 0x12 0x20 || 32-byte digest)
export function contentId(domainTag, body) {
  if (!Number.isInteger(domainTag) || domainTag < 0x01 || domainTag > 0x7f) {
    // ACS-001 §4: an address MUST NOT be computed without a domain tag; tags
    // 0x0A-0x7F are reserved but structurally valid single bytes.
    throw new Error(`invalid domain tag: ${domainTag}`);
  }
  const preimage = new Uint8Array(1 + body.length);
  preimage[0] = domainTag;
  preimage.set(body, 1);
  const digest = createHash('sha256').update(preimage).digest(); // Buffer, 32 bytes
  const out = new Uint8Array(2 + 32);
  out[0] = 0x12; // multihash code for SHA-256
  out[1] = 0x20; // digest length 32
  out.set(digest, 2);
  return out;
}

export function toHex(u8) {
  let s = '';
  for (const b of u8) s += b.toString(16).padStart(2, '0');
  return s;
}

export function fromHex(hex) {
  const clean = hex.trim();
  if (clean.length % 2 !== 0) throw new Error('odd hex length');
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(clean.substr(i * 2, 2), 16);
  }
  return out;
}

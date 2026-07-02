"""
ACS-001 / CCP-001 — Content Addressing Contract.

Independent implementation from the ARVES Standard Kit ONLY
(standard/acs/ACS-001_Content_Addressing.md). No reference-runtime source used.

ContentId (§5): a self-describing multihash
    ContentId = varint(hash_code) || varint(digest_len) || digest
For SHA-256 (§5): hash_code = 0x12, digest_len = 0x20 (32).
    ContentId = 0x12 0x20 || SHA256(pre-image)
Pre-image (§3): pre-image = domain_tag || body     (domain_tag is a single byte, §4).
    => ContentId = 0x12 0x20 || SHA256(domain_tag || body)   (§5, §7)
"""

import hashlib

# ACS-001 §4 domain-tag registry, extended additively by ACS-003 (0x06),
# ACS-004 (0x07), and ACS-005 (0x08, 0x09) — the ACS-001 registry is the single
# authority (ACS-003 §7, ACS-004 §5.3, ACS-005 §8).
DOMAIN_TAGS = {
    0x01: "commit-content",
    0x02: "engine-manifest",
    0x03: "capability-manifest",
    0x04: "invocation",
    0x05: "decision-trace",
    0x06: "canonical-envelope",   # ACS-003 §7
    0x07: "type-schema",          # ACS-004 §5.3
    0x08: "normative-glossary-term-set",  # ACS-005 §8
    0x09: "requirement-clause",   # ACS-005 §8
}

SHA256_HASH_CODE = 0x12   # §5
SHA256_DIGEST_LEN = 0x20  # §5 (32 bytes)


def content_id(domain_tag: int, body: bytes) -> bytes:
    """
    Compute the ACS-001 ContentId of `body` under `domain_tag`.
    §4: an implementation MUST NOT compute an address without a domain tag.
    """
    if domain_tag not in DOMAIN_TAGS:
        # §4: tags 0x0A-0x7F RESERVED; unknown tag is not addressable here.
        raise ValueError("unknown/unallocated ACS-001 domain tag: 0x%02x" % domain_tag)
    if not (0x00 <= domain_tag <= 0xFF):
        raise ValueError("domain tag is a single byte (§4)")
    pre_image = bytes([domain_tag]) + bytes(body)   # §3: domain_tag || body
    digest = hashlib.sha256(pre_image).digest()      # §5: SHA-256
    # Self-describing multihash prefix (§5): hash_code=0x12, digest_len=0x20.
    return bytes([SHA256_HASH_CODE, SHA256_DIGEST_LEN]) + digest

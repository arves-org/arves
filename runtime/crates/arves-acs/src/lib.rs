//! ARVES :: arves-acs — ARVES Core Standards codec.
//!
//! First Vertical-Proof brick (motto: PROVE the architecture). Implements
//! **ACS-001 Universal Content Identity**: a self-describing multihash content
//! address `0x12 0x20 || SHA-256(domain_tag || body)`. Dependency-free — SHA-256
//! is implemented here so the reference does not lean on a specific crate and an
//! independent runtime can derive the identical bytes from the spec alone.
//!
//! Governing: ACS-001 (runtime/docs/standards/ACS-001_CCP-001_Content_Addressing.md);
//! ORCH-004 (idempotent + content-addressable). ACS-002 dCBOR bodies follow.
//!
//! The unit tests assert the ACS-001-CS-1 golden vectors byte-for-byte; they are
//! the differential-conformance seed a second implementation must reproduce.

#![forbid(unsafe_code)]

pub mod cbor;

/// ACS-001 domain tags (the single registry authority is ACS-001).
pub mod domain {
    pub const COMMIT_CONTENT: u8 = 0x01;
    pub const ENGINE_MANIFEST: u8 = 0x02;
    pub const CAPABILITY_MANIFEST: u8 = 0x03;
    pub const INVOCATION: u8 = 0x04;
    pub const DECISION_TRACE: u8 = 0x05;
    pub const CANONICAL_ENVELOPE: u8 = 0x06; // ACS-003
    pub const TYPE_SCHEMA: u8 = 0x07; // ACS-004
    pub const GLOSSARY_TERM_SET: u8 = 0x08; // ACS-005
    pub const REQUIREMENT_CLAUSE: u8 = 0x09; // ACS-005
}

/// Multihash code + length prefix for SHA-256 (multicodec `sha2-256` = 0x12, 32B).
const MH_SHA256: [u8; 2] = [0x12, 0x20];

/// The ACS-001 content address of `body` under `domain_tag`:
/// `0x12 0x20 || SHA-256(domain_tag || body)` (34 bytes).
pub fn content_id(domain_tag: u8, body: &[u8]) -> Vec<u8> {
    let mut pre = Vec::with_capacity(1 + body.len());
    pre.push(domain_tag);
    pre.extend_from_slice(body);
    let digest = sha256(&pre);
    let mut id = Vec::with_capacity(34);
    id.extend_from_slice(&MH_SHA256);
    id.extend_from_slice(&digest);
    id
}

/// Lowercase hex of a byte slice (for logging / vector comparison).
pub fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 0x0f) as usize] as char);
    }
    s
}

// --- SHA-256 (FIPS 180-4), dependency-free ----------------------------------

#[rustfmt::skip]
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 digest of `data`. Standard FIPS 180-4; verified by the ACS-001 vectors.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let big_s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(big_s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let big_s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = big_s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[4 * i..4 * i + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // SHA-256 sanity against FIPS test vectors.
    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    // ACS-001-CS-1 golden vectors (must match the standard byte-for-byte; the
    // differential-conformance seed a second runtime has to reproduce).
    #[test]
    fn acs_001_cs_1_golden_vectors() {
        assert_eq!(
            hex(&content_id(domain::COMMIT_CONTENT, b"hello-truth")),
            "122056e30f71852b0e4c253cf05dab6be2bb5b8470ac878a52f10c5af2a40d69b76e"
        );
        assert_eq!(
            hex(&content_id(domain::ENGINE_MANIFEST, br#"{"engine":"summarize","version":"1"}"#)),
            "12205c631bd808332b0889763100ad7458710c137320381e3b4ea9cce3c0640a4e54"
        );
        assert_eq!(
            hex(&content_id(domain::INVOCATION, b"acme/research|c1|hello-truth")),
            "1220ae7a70002ef6dd81018d4715a986dae6dfdc1b7bc85acdd66698875f2fe302bc"
        );
    }

    #[test]
    fn content_id_is_34_bytes_and_self_describing() {
        let id = content_id(domain::COMMIT_CONTENT, b"x");
        assert_eq!(id.len(), 34);
        assert_eq!(&id[..2], &[0x12, 0x20]);
    }

    // Domain separation: same body, different domain => different address.
    #[test]
    fn domain_separation_holds() {
        assert_ne!(
            content_id(domain::COMMIT_CONTENT, b"same"),
            content_id(domain::DECISION_TRACE, b"same")
        );
    }
}

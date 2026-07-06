//! ARVES :: arves-kernel :: authenticated commit (RCR-036)
//!
//! Purpose: a **dependency-free** authenticated-commit primitive that proves a
//! commit came from the holder of a principal's key and detects any tampering of
//! the committed record — cryptographically. It closes the open half of v2.0
//! security debt #8 ("`Kernel::commit` carries no principal/authN/authZ") for the
//! trusted-key model, and complements RCR-002's tamper-evident WAL hash-chain
//! (`FileWal::integrity_digest`).
//!
//! # HONEST cryptographic scope — read this before trusting it
//!
//! - The MAC is **HMAC-SHA256** (RFC 2104) over a canonical record binding,
//!   built on the runtime's **own** SHA-256 (`arves-acs::sha256`). **ZERO** new
//!   third-party crates; std-only. It is a *real* MAC: a party without the key
//!   cannot forge one, and any change to the bound fields changes it.
//! - HMAC is a **symmetric / shared-key** MAC. Both the signer and the verifier
//!   hold the same key, so it is **REPUDIABLE** and gives **NO public-key
//!   non-repudiation**: it proves "*a* key-holder produced this commit and it was
//!   not tampered", not "*this specific* principal and no one else could have".
//! - **Key distribution is OUT OF SCOPE.** The Kernel is *provisioned* with each
//!   principal's shared key (`RefKernel::register_principal`); how that key
//!   reaches signer and Kernel securely is the operator's concern (the trusted-key
//!   model). No key is ever persisted by this module.
//! - Public-key **non-repudiation** (ed25519-class signatures) needs a crypto
//!   dependency and is therefore a **separate v2.0 RCR** — explicitly still open.
//!   This RCR closes the *unauthenticated-commit* hole for the trusted-key model
//!   only; it does not claim zero-trust.

use crate::{Principal, ProposedWrite};
use arves_acs::sha256;

/// SHA-256 block size in bytes (HMAC ipad/opad width).
const BLOCK: usize = 64;

/// Dependency-free **HMAC-SHA256** (RFC 2104) over the frozen ARVES SHA-256
/// (`arves-acs::sha256`). Reuses the runtime's own hash — no new dependency.
///
/// Standard construction: `HMAC(K, m) = H((K' ⊕ opad) ‖ H((K' ⊕ ipad) ‖ m))`,
/// where `K'` is `K` zero-padded to the block size (or `H(K)` first if longer).
/// Verified against RFC 4231 test vectors in this module's unit tests.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        k[..32].copy_from_slice(&sha256(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = Vec::with_capacity(BLOCK + msg.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(msg);
    let inner_digest = sha256(&inner);
    let mut outer = Vec::with_capacity(BLOCK + 32);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha256(&outer)
}

/// Length-prefix a field into `buf` (8-byte little-endian length + bytes), so the
/// canonical binding is unambiguous: no concatenation of two field sets can alias
/// another (a would-be forger cannot shift bytes across the `content`/`payload`
/// boundary to keep the same MAC).
fn put_field(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// Canonical MAC pre-image binding a commit to a principal: domain tag ‖
/// principal ‖ shard(tenant,workspace) ‖ content-address ‖ payload. Changing ANY
/// of these (a forged principal, a re-targeted shard, a swapped content address,
/// or a tampered payload) changes the MAC, so a valid MAC attests all of them.
fn commit_binding(principal: &Principal, p: &ProposedWrite) -> Vec<u8> {
    let mut m = Vec::new();
    put_field(&mut m, b"ARVES-AUTHCOMMIT-v1");
    put_field(&mut m, principal.0.as_bytes());
    put_field(&mut m, p.shard.tenant().as_bytes());
    put_field(&mut m, p.shard.workspace().as_bytes());
    put_field(&mut m, &p.content.0);
    put_field(&mut m, &p.payload);
    m
}

/// Compute the authenticated-commit MAC a key-holder must present to
/// [`crate::RefKernel::commit_authenticated`]. Both the client (to produce the
/// MAC) and the Kernel (to verify it) call this ONE function, so there is a
/// single authoritative binding.
pub fn commit_mac(key: &[u8], principal: &Principal, proposed: &ProposedWrite) -> [u8; 32] {
    hmac_sha256(key, &commit_binding(principal, proposed))
}

/// Constant-time 32-byte comparison — MAC verification must not leak, via timing,
/// how many leading bytes of a forged MAC were correct.
pub fn ct_eq(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff = 0u8;
    for i in 0..32 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Genesis of the Kernel-owned **authenticated-commit anchor** — a hash-chain,
/// like RCR-002's WAL `integrity_digest`, but folding each *authenticated* record
/// so the trail is principal-attributable as well as tamper-evident.
pub fn genesis_anchor() -> [u8; 32] {
    sha256(b"ARVES-AUTH-ANCHOR-v1")
}

/// Fold one authenticated commit into the anchor:
/// `aᵢ = SHA256(aᵢ₋₁ ‖ principal ‖ mac ‖ content ‖ offset)`.
///
/// It binds the MAC (so a forged record without the key cannot extend the trail
/// undetected) AND the committed `(content, offset)` — exactly what RCR-002's WAL
/// digest binds — so the authenticated anchor and the WAL integrity digest move
/// together: the anchor changes iff the authenticated trail changes.
pub fn fold_anchor(
    prev: &[u8; 32],
    principal: &Principal,
    mac: &[u8; 32],
    content: &[u8],
    offset: u64,
) -> [u8; 32] {
    let mut b = Vec::with_capacity(32 + principal.0.len() + 32 + content.len() + 32);
    b.extend_from_slice(prev);
    put_field(&mut b, principal.0.as_bytes());
    b.extend_from_slice(mac);
    put_field(&mut b, content);
    b.extend_from_slice(&offset.to_le_bytes());
    sha256(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 4231 known-answer vectors — proof this is a *correct* HMAC-SHA256, not a
    // home-grown keyed hash. If these fail, every MAC below is meaningless.
    fn hex(b: &[u8]) -> String {
        let mut s = String::new();
        for x in b {
            s.push_str(&format!("{x:02x}"));
        }
        s
    }

    #[test]
    fn rfc4231_case_1() {
        let key = [0x0bu8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex(&mac),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn rfc4231_case_2() {
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn rfc4231_case_3_long_block() {
        // key = 0xaa x20, data = 0xdd x50 — exercises the < block-size key path.
        let key = [0xaau8; 20];
        let data = [0xddu8; 50];
        let mac = hmac_sha256(&key, &data);
        assert_eq!(
            hex(&mac),
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
        );
    }

    #[test]
    fn rfc4231_case_6_key_longer_than_block() {
        // 131-byte key exercises the H(key) pre-hash path.
        let key = [0xaau8; 131];
        let mac = hmac_sha256(&key, b"Test Using Larger Than Block-Size Key - Hash Key First");
        assert_eq!(
            hex(&mac),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn ct_eq_matches_plain_eq() {
        let a = sha256(b"x");
        let b = sha256(b"x");
        let c = sha256(b"y");
        assert!(ct_eq(&a, &b));
        assert!(!ct_eq(&a, &c));
    }
}

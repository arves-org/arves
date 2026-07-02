//! ARVES Bridge — the single seam between the product/SDK world and the reference
//! runtime (Kernel / Engine).
//!
//! The reference Kernel addresses truth by an **opaque, caller-supplied**
//! `ContentHash` (arves-kernel), which drives ORCH-004 idempotency. Nothing in the
//! Kernel forces that address to be the ACS-001 ContentId — so a naive product could
//! commit truth under a different identity than the standard defines, creating two
//! divergent worlds (the largest architectural risk of the product era).
//!
//! The bridge closes that seam: it computes the ACS-001 address of the ACS-002
//! canonical body via `arves-acs` and commits it as the Kernel's `ContentHash`. The
//! `TruthRef` the Kernel returns is therefore addressed by the *same* ContentId the
//! SDK (Rust / Python / TypeScript) computes locally — one world, one identity. This
//! is where the Kernel CONSUMES the standard.

use arves_acs::{cbor, content_id};
use arves_kernel::{CommitError, ContentHash, Kernel, ProposedWrite, ShardKey, TruthRef};

/// Commit a canonical ACS body as truth, addressed by its ACS-001 ContentId.
/// `TruthRef.content` will be `0x12 0x20 || SHA-256(domain_tag || body)` — the exact
/// address any conformant ACS implementation computes for the same body.
pub fn commit_body(
    kernel: &impl Kernel,
    shard: ShardKey,
    domain_tag: u8,
    body: &[u8],
) -> Result<TruthRef, CommitError> {
    let cid = content_id(domain_tag, body);
    kernel.commit(ProposedWrite { shard, content: ContentHash(cid), payload: body.to_vec() })
}

/// Encode an ACS value (ACS-002 dCBOR) and commit it ACS-addressed. A rich value goes
/// in; ACS-identified truth comes out.
pub fn commit_value(
    kernel: &impl Kernel,
    shard: ShardKey,
    domain_tag: u8,
    value: &cbor::Value,
) -> Result<TruthRef, CommitError> {
    commit_body(kernel, shard, domain_tag, &cbor::encode(value))
}

/// The ACS-001 ContentId `commit_body` will use for `(domain_tag, body)`. A caller can
/// predict identity locally and assert the Kernel agrees — the "one world" check.
pub fn address(domain_tag: u8, body: &[u8]) -> Vec<u8> {
    content_id(domain_tag, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arves_acs::cbor::Value::*;
    use arves_acs::{domain, hex};
    use arves_kernel::MemKernel;
    use arves_persistence::MemWalStore;

    fn shard() -> ShardKey {
        ShardKey { tenant: "t1".into(), workspace: "w1".into() }
    }

    // The Kernel commits truth under the ACS-001 address (the golden V1 fact ContentId).
    // SDK-computed identity == Kernel-committed identity: one world.
    #[test]
    fn commit_is_acs_addressed() {
        let k = MemKernel::new(MemWalStore::new());
        let fact = Map(vec![
            (Text("type".into()), Text("uci.fact".into())),
            (Text("claim".into()), Text("sky-is-blue".into())),
            (Text("confidence".into()), Float(0.5)),
            (Text("observed_at".into()), Int(1730000000000000000)),
        ]);
        let tr = commit_value(&k, shard(), domain::COMMIT_CONTENT, &fact).expect("commit ok");
        assert_eq!(
            hex(&tr.content.0),
            "12204284f0acb42a4730633fa8d6cfbd9040d85b62ebe3769d8b7d59af4375bb363e",
            "Kernel truth is addressed by the ACS-001 ContentId"
        );
        // And the address is exactly what a caller predicts locally.
        assert_eq!(tr.content.0, address(domain::COMMIT_CONTENT, &cbor::encode(&fact)));
    }

    // ORCH-004 idempotency is now keyed on the ACS address: same body -> AlreadyCommitted.
    #[test]
    fn commit_is_idempotent_on_acs_address() {
        let k = MemKernel::new(MemWalStore::new());
        let body = b"hello-truth";
        let first = commit_body(&k, shard(), domain::COMMIT_CONTENT, body).expect("first ok");
        match commit_body(&k, shard(), domain::COMMIT_CONTENT, body) {
            Err(CommitError::AlreadyCommitted(existing)) => assert_eq!(existing, first),
            other => panic!("expected AlreadyCommitted, got {other:?}"),
        }
    }
}

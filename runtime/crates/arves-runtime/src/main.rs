//! ARVES :: arves-runtime
//!
//! I1.4 walking-skeleton demo: the first executable behaviour, runnable via
//! `cargo run -p arves-runtime`. Single node/shard; no distributed logic.

use arves_kernel::{ContentHash, Kernel, MemKernel, ProposedWrite, ShardKey};
use arves_persistence::MemWalStore;

fn main() {
    let store = MemWalStore::new();
    let shard = ShardKey {
        tenant: "acme".into(),
        workspace: "research".into(),
    };

    let kernel = MemKernel::new(store.clone());
    let tr = kernel
        .commit(ProposedWrite {
            shard: shard.clone(),
            content: ContentHash(b"c1".to_vec()),
            payload: b"hello-truth".to_vec(),
        })
        .expect("commit");
    let before = kernel.truth_hash();
    println!(
        "committed truth at index {} (count={}, truth_hash={:#018x})",
        tr.index.0,
        kernel.committed_count(),
        before
    );

    // Simulate a restart: drop nothing but rebuild a fresh Kernel from the WAL.
    let recovered = MemKernel::recover(store.clone());
    println!(
        "recovered count={} truth_hash={:#018x} identical={}",
        recovered.committed_count(),
        recovered.truth_hash(),
        recovered.truth_hash() == before
    );
}

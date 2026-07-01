//! ARVES :: arves-runtime
//!
//! Walking-skeleton demo + I1.5 real-restart harness. Runnable via
//! `cargo run -p arves-runtime`. Single node/shard; no distributed logic.
//!
//! Subcommands (I1.5/I1.6, used by the cross-process proofs):
//!   arves-runtime write      <dir>   commit a fixed sequence to a file WAL,
//!                                     fsync, print TRUTH_HASH + COUNT. (proc A)
//!   arves-runtime recover    <dir>   open the SAME dir in a fresh process,
//!                                     restore (snapshot + tail), print hash.
//!   arves-runtime checkpoint <dir>   recover, take a durable checkpoint
//!                                     (snapshot + compaction), print hash.
//! With no subcommand it runs the I1.4 in-memory demo.

use arves_kernel::{
    CommitError, ContentHash, FileKernel, Kernel, MemKernel, ProposedWrite, ShardKey,
};
use arves_persistence::{FileWalStore, MemWalStore};
use std::process;

fn demo_shard() -> ShardKey {
    ShardKey {
        tenant: "acme".into(),
        workspace: "research".into(),
    }
}

/// A fixed, deterministic commit sequence so `write` and `recover` (and repeated
/// runs) always produce the SAME `truth_hash`. Idempotent: re-committing an
/// identical proposal is an accepted no-op (ORCH-004), never a hard error.
fn commit_fixed_sequence<K: Kernel>(k: &K) {
    let commits: [(&[u8], &[u8]); 3] = [
        (&b"c1"[..], &b"hello-truth"[..]),
        (&b"c2"[..], &b"second-truth"[..]),
        (&b"c3"[..], &b"third-truth"[..]),
    ];
    for (content, payload) in commits {
        match k.commit(ProposedWrite {
            shard: demo_shard(),
            content: ContentHash(content.to_vec()),
            payload: payload.to_vec(),
        }) {
            Ok(_) | Err(CommitError::AlreadyCommitted(_)) => {}
            Err(e) => {
                eprintln!("commit failed: {e}");
                process::exit(2);
            }
        }
    }
}

fn run_write(dir: &str) {
    let store = FileWalStore::open_root(dir).expect("open file store");
    let kernel = FileKernel::new(store);
    commit_fixed_sequence(&kernel);
    // TRUTH_HASH/COUNT lines are the machine-readable contract the restart proof
    // parses; keep the format stable.
    println!("TRUTH_HASH={:#018x}", kernel.truth_hash());
    println!("COUNT={}", kernel.committed_count());
}

fn run_recover(dir: &str) {
    let store = FileWalStore::open_root(dir).expect("open file store");
    // A genuinely fresh Kernel: no commits, only restore (snapshot + tail replay).
    let kernel = FileKernel::recover(store);
    println!("TRUTH_HASH={:#018x}", kernel.truth_hash());
    println!("COUNT={}", kernel.committed_count());
}

fn run_checkpoint(dir: &str) {
    let store = FileWalStore::open_root(dir).expect("open file store");
    let kernel = FileKernel::recover(store);
    let before = kernel.truth_hash();
    let shards = kernel.checkpoint().expect("checkpoint");
    // A checkpoint is a durability operation; it must not change truth.
    println!("CHECKPOINTED_SHARDS={shards}");
    println!("TRUTH_HASH={:#018x}", kernel.truth_hash());
    println!("COUNT={}", kernel.committed_count());
    println!("TRUTH_UNCHANGED={}", kernel.truth_hash() == before);
}

fn run_demo() {
    let store = MemWalStore::new();
    let shard = demo_shard();

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

    // Simulate a restart: rebuild a fresh Kernel from the (in-memory) WAL.
    let recovered = MemKernel::recover(store.clone());
    println!(
        "recovered count={} truth_hash={:#018x} identical={}",
        recovered.committed_count(),
        recovered.truth_hash(),
        recovered.truth_hash() == before
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("write") => {
            let dir = args.get(2).unwrap_or_else(|| {
                eprintln!("usage: arves-runtime write <dir>");
                process::exit(64);
            });
            run_write(dir);
        }
        Some("recover") => {
            let dir = args.get(2).unwrap_or_else(|| {
                eprintln!("usage: arves-runtime recover <dir>");
                process::exit(64);
            });
            run_recover(dir);
        }
        Some("checkpoint") => {
            let dir = args.get(2).unwrap_or_else(|| {
                eprintln!("usage: arves-runtime checkpoint <dir>");
                process::exit(64);
            });
            run_checkpoint(dir);
        }
        _ => run_demo(),
    }
}

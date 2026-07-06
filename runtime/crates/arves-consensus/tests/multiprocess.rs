//! RCR-038 — THE cross-PROCESS proof: three raft nodes as SEPARATE OS PROCESSES,
//! talking only over real TCP sockets, run an election and one committed write,
//! and commit the byte-identical OUTCOME the in-process run commits.
//!
//! This mirrors `arves-runtime`'s `real_restart.rs`: genuinely distinct OS
//! processes (spawned via `CARGO_BIN_EXE_arves-consensus-node`), no shared
//! memory — the only channels are (a) a filesystem rendezvous for bootstrap
//! address exchange and (b) REAL TCP sockets for every raft RPC.
//!
//! ## Why `#[ignore]` by default
//! It spawns OS processes and runs a real-time election over real sockets. That
//! makes it heavier and timing-driven, so it is kept out of the default
//! `cargo test --workspace` gate (which must stay fast and deterministic). It
//! MUST pass when invoked explicitly:
//! ```text
//! cargo test --manifest-path runtime/Cargo.toml -p arves-consensus \
//!     --test multiprocess -- --ignored --nocapture
//! ```
//! The genuinely-networked send/poll/reconnect surface it exercises is ALSO
//! covered by fast, deterministic in-process unit tests in `transport.rs`
//! (`node_transport_*`), which DO run in the default gate.
//!
//! ## What it proves / does NOT prove (honest scope)
//! - PROVES: cross-PROCESS networked consensus on one host over real length-framed
//!   TCP — a quorum of separate processes commits, and the committed OUTCOME
//!   content is byte-identical to the in-process run (whichever process wins the
//!   real election, under whatever term).
//! - Does NOT prove: true multi-HOST across machines; hostile-network
//!   partition/latency/loss timing (sockets here deliver reliably); TLS/mTLS.
//!   These remain recorded OQ.

use arves_consensus::transport::{InProcessTransport, TransportRound};
use arves_consensus::{ContentHash, EntryKind, Outcome};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const SEED: u64 = 0xC0FFEE;
const TAG: &str = "payload";

fn rendezvous_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    p.push("rcr038_multiprocess");
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("create rendezvous dir");
    p
}

/// The in-process baseline's committed outcome digest — the comparison anchor.
fn in_process_committed_outcome_digest() -> u64 {
    let mut r = TransportRound::new(3, SEED);
    let mut mem = InProcessTransport::new();
    r.elect_and_commit_one(
        &mut mem,
        EntryKind::Outcome(Outcome {
            digest: ContentHash(format!("h:{TAG}")),
            payload: TAG.as_bytes().to_vec(),
        }),
    );
    r.committed_outcome_digest().expect("baseline committed an outcome")
}

fn field(stdout: &str, key: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix(key).map(|v| v.trim().to_string()))
}

#[test]
#[ignore = "spawns OS processes + real-time TCP election; run with --ignored"]
fn three_processes_commit_identical_truth_over_real_tcp() {
    let baseline = in_process_committed_outcome_digest();
    let dir = rendezvous_dir();
    let bin = env!("CARGO_BIN_EXE_arves-consensus-node");
    let peers = "n1,n2,n3";

    // Spawn all three FIRST (they must run concurrently to talk), then collect.
    let mut children = Vec::new();
    for id in ["n1", "n2", "n3"] {
        let child = Command::new(bin)
            .args(["--id", id])
            .args(["--peers", peers])
            .args(["--rendezvous", dir.to_str().unwrap()])
            .args(["--seed", &SEED.to_string()])
            .args(["--propose", TAG])
            .args(["--tick-ms", "10"])
            .args(["--deadline-ms", "30000"])
            .args(["--grace-ms", "800"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn node {id}: {e}"));
        children.push((id, child));
    }

    let mut committed = Vec::new();
    for (id, child) in children {
        let out = child.wait_with_output().expect("wait node");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let commit_index: u64 = field(&stdout, "COMMIT_INDEX=")
            .unwrap_or_else(|| panic!("node {id} missing COMMIT_INDEX:\n{stdout}\n---stderr---\n{stderr}"))
            .parse()
            .expect("commit index u64");
        let outcome = field(&stdout, "COMMIT_OUTCOME=").expect("COMMIT_OUTCOME line");
        eprintln!(
            "node {id}: role={:?} commit_index={commit_index} outcome={outcome} exit={:?}",
            field(&stdout, "ROLE="),
            out.status.code()
        );
        if commit_index >= 1 {
            let digest = u64::from_str_radix(outcome.trim_start_matches("0x"), 16)
                .expect("outcome hex u64");
            committed.push((id, digest));
        }
    }

    // A quorum of separate processes committed over real TCP.
    assert!(
        committed.len() >= 2,
        "expected a quorum (>=2) of 3 processes to commit; got {}",
        committed.len()
    );
    // Every committed process agrees, and agrees with the in-process baseline.
    let mut seen = BTreeMap::new();
    for (id, digest) in &committed {
        seen.entry(*digest).or_insert_with(Vec::new).push(*id);
        assert_eq!(
            *digest, baseline,
            "process {id} committed a DIFFERENT outcome than the in-process baseline \
             (cross-process committed truth must be byte-identical in CONTENT)"
        );
    }
    assert_eq!(seen.len(), 1, "all committed processes agree on one outcome: {seen:?}");
}

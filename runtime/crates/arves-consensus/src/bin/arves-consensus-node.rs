//! `arves-consensus-node` (RCR-038) — ONE raft node in ONE OS process, talking to
//! its peers over REAL TCP sockets. N of these processes form a genuine networked
//! cluster. It exists to PROVE cross-PROCESS networked consensus: run an election
//! and one committed write over real sockets between separate processes, and show
//! the committed OUTCOME content is byte-identical to the in-process run.
//!
//! # Usage
//! ```text
//! arves-consensus-node --id n1 --peers n1,n2,n3 --rendezvous <dir> \
//!     --seed <u64> --propose <tag> [--tick-ms 10] [--deadline-ms 30000] [--grace-ms 600]
//! ```
//!
//! # Peer discovery (bootstrap only — NOT the consensus protocol)
//! Ports are ephemeral, so addresses are exchanged through the filesystem: each
//! node binds `127.0.0.1:0`, writes its real `SocketAddr` to `<dir>/<id>.addr`,
//! then waits until every peer's `.addr` file exists and reads them. This is a
//! bootstrap rendezvous only; every raft RPC afterward travels over real TCP
//! between separate processes.
//!
//! # Determinism (HARD RULE 4)
//! The raft protocol is a pure function of `(messages, seed, tick)`. Here `tick`
//! is driven by a real wall-clock timer and the election timeout is real, so WHO
//! leads and under WHICH term vary run-to-run — that is liveness/leadership, and
//! it is NEVER allowed to feed committed-truth CONTENT. The one quantity the test
//! compares, [`outcome_content_digest`], is a pure function of the proposed
//! outcome's address + payload; it is invariant across leader, term, and timing.
//!
//! # Honest scope
//! Real length-framed TCP between separate processes on ONE host. Proves
//! cross-process networked consensus. True multi-HOST across machines and
//! hostile-network partition/latency/loss remain recorded OQ.

use arves_consensus::raft::RaftNode;
use arves_consensus::transport::{encode_envelope, outcome_content_digest, NodeTransport};
use arves_consensus::{ContentHash, EntryKind, NodeId, Outcome, Role};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

struct Args {
    id: String,
    peers: Vec<String>,
    rendezvous: PathBuf,
    seed: u64,
    propose: String,
    tick_ms: u64,
    deadline_ms: u64,
    grace_ms: u64,
}

fn parse_args() -> Args {
    let mut id = None;
    let mut peers = None;
    let mut rendezvous = None;
    let mut seed = 0u64;
    let mut propose = None;
    let mut tick_ms = 10u64;
    let mut deadline_ms = 30_000u64;
    let mut grace_ms = 600u64;

    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--id" => id = Some(it_next(&mut it, &flag)),
            "--peers" => {
                peers = Some(it_next(&mut it, &flag).split(',').map(|s| s.to_string()).collect())
            }
            "--rendezvous" => rendezvous = Some(PathBuf::from(it_next(&mut it, &flag))),
            "--seed" => seed = it_next(&mut it, &flag).parse().expect("--seed is a u64"),
            "--propose" => propose = Some(it_next(&mut it, &flag)),
            "--tick-ms" => tick_ms = it_next(&mut it, &flag).parse().expect("--tick-ms is a u64"),
            "--deadline-ms" => {
                deadline_ms = it_next(&mut it, &flag).parse().expect("--deadline-ms is a u64")
            }
            "--grace-ms" => grace_ms = it_next(&mut it, &flag).parse().expect("--grace-ms is a u64"),
            other => panic!("unknown flag: {other}"),
        }
    }
    Args {
        id: id.expect("--id required"),
        peers: peers.expect("--peers required"),
        rendezvous: rendezvous.expect("--rendezvous required"),
        seed,
        propose: propose.expect("--propose required"),
        tick_ms,
        deadline_ms,
        grace_ms,
    }
}

fn it_next(it: &mut impl Iterator<Item = String>, flag: &str) -> String {
    it.next().unwrap_or_else(|| panic!("{flag} needs a value"))
}

/// Per-node seed mix so each node draws a DIFFERENT randomized election deadline
/// (reduces split-vote stalls). Deterministic in (base seed, id); it feeds only
/// election TIMING, never committed content.
fn mix(seed: u64, id: &str) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for b in id.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Write our address, then block until every peer's `.addr` file exists and parse
/// them into a peer address map. Bounded so a missing peer fails loudly, not hangs.
fn discover(
    dir: &PathBuf,
    me: &str,
    peers: &[String],
    my_addr: SocketAddr,
) -> BTreeMap<NodeId, SocketAddr> {
    std::fs::create_dir_all(dir).expect("create rendezvous dir");
    std::fs::write(dir.join(format!("{me}.addr")), my_addr.to_string()).expect("write my addr");

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let mut map = BTreeMap::new();
        let mut missing = false;
        for p in peers {
            if p == me {
                map.insert(NodeId(p.clone()), my_addr);
                continue;
            }
            match std::fs::read_to_string(dir.join(format!("{p}.addr"))) {
                Ok(s) => match s.trim().parse::<SocketAddr>() {
                    Ok(a) => {
                        map.insert(NodeId(p.clone()), a);
                    }
                    Err(_) => missing = true, // half-written file — retry
                },
                Err(_) => missing = true,
            }
        }
        if !missing {
            return map;
        }
        assert!(Instant::now() < deadline, "peer discovery timed out for node {me}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn main() {
    let args = parse_args();
    let me = NodeId(args.id.clone());

    // Bind our real listener, publish our address, learn peers' addresses.
    let mut nt = NodeTransport::bind(me.clone(), "127.0.0.1:0", BTreeMap::new())
        .expect("bind node listener");
    let my_addr = nt.local_addr();
    let peer_addrs = discover(&args.rendezvous, &args.id, &args.peers, my_addr);
    // Same listener (published address stays valid); just learn the peer addresses.
    nt.set_peers(peer_addrs);

    // Build the raft node over the full membership.
    let members: Vec<NodeId> = args.peers.iter().map(|p| NodeId(p.clone())).collect();
    let mut node = RaftNode::new(me.clone(), members, mix(args.seed, &args.id));

    let outcome = EntryKind::Outcome(Outcome {
        digest: ContentHash(format!("h:{}", args.propose)),
        payload: args.propose.as_bytes().to_vec(),
    });

    let start = Instant::now();
    let deadline = start + Duration::from_millis(args.deadline_ms);
    let tick = Duration::from_millis(args.tick_ms);
    let grace = Duration::from_millis(args.grace_ms);
    let mut next_tick = Instant::now();
    let mut proposed = false;
    let mut committed_at: Option<Instant> = None;

    loop {
        // Deliver inbound. Canonical per-node order (best-effort): the committed
        // outcome content does not depend on it, but a stable order keeps the
        // node's processing reproducible for a fixed inbound batch.
        let mut inbound = nt.poll();
        inbound.sort_by(|a, b| encode_envelope(a).cmp(&encode_envelope(b)));
        for env in inbound {
            if env.to == me {
                for e in node.step(env) {
                    nt.send(e);
                }
            }
        }

        // Real-time tick (drives heartbeats + election timeout).
        if Instant::now() >= next_tick {
            for e in node.tick() {
                nt.send(e);
            }
            next_tick = Instant::now() + tick;
        }

        // Propose exactly once, as soon as we lead (log is empty at election, so
        // the proposal is immediately valid to commit under our own term).
        if !proposed && node.role() == Role::Leader {
            if let Ok((_idx, out)) = node.client_propose(outcome.clone()) {
                for e in out {
                    nt.send(e);
                }
                proposed = true;
            }
        }

        // Terminate once WE have committed the write and served a grace window so
        // peers can catch up (heartbeats propagate leader_commit).
        if node.commit_index().0 >= 1 {
            let t = committed_at.get_or_insert_with(Instant::now);
            if t.elapsed() >= grace {
                break;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    nt.shutdown();

    // Report — the committed outcome digest is the timing-independent anchor.
    let commit_index = node.commit_index().0;
    let committed_outcome = node
        .log()
        .iter()
        .find(|e| e.index.0 == 1)
        .and_then(|e| match &e.kind {
            EntryKind::Outcome(o) => Some(outcome_content_digest(o)),
            _ => None,
        });

    println!("NODE={}", args.id);
    println!(
        "ROLE={}",
        match node.role() {
            Role::Leader => "leader",
            Role::Follower => "follower",
            Role::Candidate => "candidate",
            Role::Learner => "learner",
        }
    );
    println!("TERM={}", node.current_term().0);
    println!("COMMIT_INDEX={commit_index}");
    match committed_outcome {
        Some(d) => println!("COMMIT_OUTCOME={d:#018x}"),
        None => println!("COMMIT_OUTCOME=none"),
    }

    if commit_index >= 1 {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

//! ARVES :: arves-consensus :: transport — the delivery-layer seam (RCR-032).
//!
//! # Gap this closes
//!
//! Through RCR-019..031 the entire I2–I5 cluster ran on ONE vehicle: the
//! in-process deterministic [`crate::sim`] `MessageBus` (a `VecDeque<Envelope>`
//! with scripted filters/mangling). That is exactly the design's mandated
//! I2.1..I2.6 vehicle (§3.7: "the network adapter is a shell around [the pure
//! core]; sockets arrive only at the step where they are the property under
//! test"). But every honesty note therefore had to read "in-process simulation,
//! no network transport". This module upgrades that claim from *"in-process
//! simulation"* to *"transport-agnostic, and exercised over a REAL OS socket"* —
//! WITHOUT weakening determinism.
//!
//! # The seam
//!
//! [`Transport`] is a pure DELIVERY layer: `send` accepts an addressed
//! [`Envelope`]; `drain` returns the envelopes delivered so far, in an
//! UNSPECIFIED order. The consensus protocol ([`crate::raft`]) is a pure
//! function of `(messages, seed, tick)`; determinism of *committed truth* is the
//! HARNESS's responsibility, not the transport's — [`TransportRound`] imposes a
//! canonical (byte-sorted) processing order on whatever `drain` returns, so a
//! socket that reorders bytes-in-flight can never reorder committed decisions.
//! This is the RCR-032 invariant: **the transport moves bytes; the protocol
//! decides truth; the harness fixes the order.**
//!
//! # Two implementations, one contract
//!
//! - [`InProcessTransport`] — the existing FIFO-bus delivery discipline as a
//!   trait impl (a `VecDeque`, `send` = push-back, `drain` = take-all). Nothing
//!   about [`crate::sim`] changed; this is the same in-memory delivery, now
//!   behind the seam so the loopback impl can stand beside it.
//! - [`LoopbackTransport`] — a REAL `std::net` TCP transport on `127.0.0.1`:
//!   each node binds a listener; `send(a→b)` length-frames the SAME serialized
//!   [`Envelope`] and writes it to a real socket connected to `b`'s listener;
//!   `drain` reads framed messages back off real sockets, handling partial
//!   reads and one connection reset (reconnect). It lives entirely in one
//!   process (both endpoints are local), which is what keeps it a deterministic
//!   TEST vehicle while genuinely crossing the OS socket boundary.
//!
//! # HONEST SCOPE (what this proves — and, exactly, what it does NOT)
//!
//! - **Proven:** the consensus core is transport-agnostic; the SAME committed
//!   truth (byte-identical log + commit index on every node) results whether
//!   messages travel through the in-memory bus OR through real length-framed TCP
//!   sockets; the wire codec round-trips every `Envelope` variant; framing
//!   survives partial reads; the round survives a full connection reset and
//!   transparently reconnects.
//! - **NOT proven / still deferred (recorded OQ, never quietly claimed):**
//!   multi-HOST networking (this is loopback, one process); TLS / mutual auth
//!   (design §"new surface" OQ-7 — mTLS is v2.0 debt); message loss / real
//!   latency / partition TIMING under a live network (the loopback delivers
//!   every sent frame; adversarial delivery stays the [`crate::sim`] filter/
//!   mangle model); a protocol-driven wire format decision (OQ-3 — this LE codec
//!   is an internal, deterministic framing, not the final dCBOR/ACS-002 choice);
//!   crash-durable raft state. Reconnect here is IDLE-time (a peer connection is
//!   reset between message generations, at a quiescent boundary), not mid-frame
//!   retransmission under loss.
//!
//! Governing: IDR-001 (per-shard CP truth), IDR-004 (leader-only), IDR-005
//! (append-only log), ORCH-003 (replay/determinism), LAYER-001 (no new crate
//! dependency — this is `std` only, `arves-consensus` stays rank 30).

use crate::raft::{Envelope, MsgBody, RaftNode};
use crate::{ContentHash, EntryKind, LogEntry, LogIndex, Membership, NodeId, Outcome, Role, Term};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

// ===========================================================================
// Wire codec — deterministic length-prefixed little-endian encoding of an
// Envelope. Zero external dependencies (no serde): the SAME discipline as the
// Kernel snapshot/ProposedWrite codec (RCR-021). Consensus decides ordering,
// never meaning, so this codec is symmetric and total; decode is STRICT
// (trailing garbage / truncation refused loudly). OQ-3 (the final protocol wire
// format — dCBOR/ACS-002 canonical) stays open; this is an internal framing.
// ===========================================================================

fn put_u64(b: &mut Vec<u8>, v: u64) {
    b.extend_from_slice(&v.to_le_bytes());
}

fn put_bytes(b: &mut Vec<u8>, s: &[u8]) {
    put_u64(b, s.len() as u64);
    b.extend_from_slice(s);
}

fn put_bool(b: &mut Vec<u8>, v: bool) {
    b.push(if v { 1 } else { 0 });
}

fn put_node(b: &mut Vec<u8>, id: &NodeId) {
    put_bytes(b, id.0.as_bytes());
}

fn put_membership(b: &mut Vec<u8>, m: &Membership) {
    match m {
        Membership::Stable { voters, learners } => {
            b.push(1);
            put_u64(b, voters.len() as u64);
            for v in voters {
                put_node(b, v);
            }
            put_u64(b, learners.len() as u64);
            for l in learners {
                put_node(b, l);
            }
        }
        Membership::Joint { old_voters, new_voters, learners } => {
            b.push(2);
            put_u64(b, old_voters.len() as u64);
            for v in old_voters {
                put_node(b, v);
            }
            put_u64(b, new_voters.len() as u64);
            for v in new_voters {
                put_node(b, v);
            }
            put_u64(b, learners.len() as u64);
            for l in learners {
                put_node(b, l);
            }
        }
    }
}

fn put_entry(b: &mut Vec<u8>, e: &LogEntry) {
    put_u64(b, e.term.0);
    put_u64(b, e.index.0);
    match &e.kind {
        EntryKind::Outcome(o) => {
            b.push(1);
            put_bytes(b, o.digest.0.as_bytes());
            put_bytes(b, &o.payload);
        }
        EntryKind::Membership(m) => {
            b.push(2);
            put_membership(b, m);
        }
    }
}

/// Serialize an [`Envelope`] to the canonical byte form (framing is added by the
/// transport, not here). This is the "SAME serialized consensus message" the
/// loopback transport puts on the wire and the in-process comparison sorts by.
pub fn encode_envelope(env: &Envelope) -> Vec<u8> {
    let mut b = Vec::new();
    put_node(&mut b, &env.from);
    put_node(&mut b, &env.to);
    match &env.body {
        MsgBody::RequestVote { term, last_log_index, last_log_term, transfer } => {
            b.push(1);
            put_u64(&mut b, term.0);
            put_u64(&mut b, *last_log_index);
            put_u64(&mut b, last_log_term.0);
            put_bool(&mut b, *transfer);
        }
        MsgBody::VoteReply { term, granted } => {
            b.push(2);
            put_u64(&mut b, term.0);
            put_bool(&mut b, *granted);
        }
        MsgBody::AppendEntries { term, prev_log_index, prev_log_term, entries, leader_commit } => {
            b.push(3);
            put_u64(&mut b, term.0);
            put_u64(&mut b, *prev_log_index);
            put_u64(&mut b, prev_log_term.0);
            put_u64(&mut b, entries.len() as u64);
            for e in entries {
                put_entry(&mut b, e);
            }
            put_u64(&mut b, *leader_commit);
        }
        MsgBody::AppendReply { term, success, match_index } => {
            b.push(4);
            put_u64(&mut b, term.0);
            put_bool(&mut b, *success);
            put_u64(&mut b, *match_index);
        }
        MsgBody::TimeoutNow { term } => {
            b.push(5);
            put_u64(&mut b, term.0);
        }
    }
    b
}

/// Strict cursor over an encoded buffer (returns `None` on any short read).
struct Cur<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.b.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        let n = self.u64()? as usize;
        Some(self.take(n)?.to_vec())
    }
    fn string(&mut self) -> Option<String> {
        String::from_utf8(self.bytes()?).ok()
    }
    fn node(&mut self) -> Option<NodeId> {
        Some(NodeId(self.string()?))
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn boolean(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn node_vec(&mut self) -> Option<Vec<NodeId>> {
        let n = self.u64()? as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.node()?);
        }
        Some(v)
    }
    fn membership(&mut self) -> Option<Membership> {
        match self.u8()? {
            1 => Some(Membership::Stable { voters: self.node_vec()?, learners: self.node_vec()? }),
            2 => Some(Membership::Joint {
                old_voters: self.node_vec()?,
                new_voters: self.node_vec()?,
                learners: self.node_vec()?,
            }),
            _ => None,
        }
    }
    fn entry(&mut self) -> Option<LogEntry> {
        let term = Term(self.u64()?);
        let index = LogIndex(self.u64()?);
        let kind = match self.u8()? {
            1 => EntryKind::Outcome(Outcome {
                digest: ContentHash(self.string()?),
                payload: self.bytes()?,
            }),
            2 => EntryKind::Membership(self.membership()?),
            _ => return None,
        };
        Some(LogEntry { term, index, kind })
    }
}

/// Decode an [`Envelope`] from the canonical byte form. Strict: trailing bytes
/// or truncation ⇒ `None` (the delivery path must never guess at a malformed
/// frame — lossless or loud).
pub fn decode_envelope(buf: &[u8]) -> Option<Envelope> {
    let mut c = Cur::new(buf);
    let from = c.node()?;
    let to = c.node()?;
    let body = match c.u8()? {
        1 => MsgBody::RequestVote {
            term: Term(c.u64()?),
            last_log_index: c.u64()?,
            last_log_term: Term(c.u64()?),
            transfer: c.boolean()?,
        },
        2 => MsgBody::VoteReply { term: Term(c.u64()?), granted: c.boolean()? },
        3 => {
            let term = Term(c.u64()?);
            let prev_log_index = c.u64()?;
            let prev_log_term = Term(c.u64()?);
            let n = c.u64()? as usize;
            let mut entries = Vec::with_capacity(n);
            for _ in 0..n {
                entries.push(c.entry()?);
            }
            let leader_commit = c.u64()?;
            MsgBody::AppendEntries { term, prev_log_index, prev_log_term, entries, leader_commit }
        }
        4 => MsgBody::AppendReply {
            term: Term(c.u64()?),
            success: c.boolean()?,
            match_index: c.u64()?,
        },
        5 => MsgBody::TimeoutNow { term: Term(c.u64()?) },
        _ => return None,
    };
    if c.pos != buf.len() {
        return None; // trailing garbage
    }
    Some(Envelope { from, to, body })
}

/// A **timing-independent** digest of a committed [`Outcome`]'s CONTENT (its
/// content-address string + opaque payload bytes) — deliberately excluding the
/// term/leader/index, which are functions of the (real-time) election and thus
/// vary run-to-run. This is the ONLY quantity that RCR-038's cross-PROCESS run
/// compares against the in-process run: whichever node wins the real election
/// and whatever term it commits under, the committed outcome *content* must be
/// byte-identical to what the client proposed. It is a pure function of the
/// proposal — never of the network timing (HARD RULE 4: the bounded, real-time
/// election feeds liveness/leadership, NEVER committed-truth content).
pub fn outcome_content_digest(o: &Outcome) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01B3);
        }
    };
    eat(o.digest.0.as_bytes());
    eat(&[0xff]); // domain separator between address and payload
    eat(&o.payload);
    h
}

// ===========================================================================
// The delivery-layer seam.
// ===========================================================================

/// A message DELIVERY layer under the deterministic consensus protocol.
///
/// Contract: `send` accepts an addressed [`Envelope`]; `drain` returns every
/// envelope delivered since the last `drain`, in an UNSPECIFIED order, and must
/// eventually deliver every sent envelope exactly once. Ordering-into-truth is
/// NOT the transport's job — the harness ([`TransportRound`]) canonicalizes
/// order — so an impl is free to deliver in socket-arrival order.
pub trait Transport {
    /// Hand one addressed envelope to the delivery layer.
    fn send(&mut self, env: Envelope);
    /// Take everything delivered so far (order unspecified; caller canonicalizes).
    fn drain(&mut self) -> Vec<Envelope>;
}

/// The in-process FIFO bus as a [`Transport`] — the existing [`crate::sim`]
/// delivery discipline, unchanged, now behind the seam. `send` = push-back,
/// `drain` = take-all-currently-queued. No filters/mangling live here: those
/// are [`crate::sim::SimCluster`]'s adversarial concerns, not the transport's.
#[derive(Default)]
pub struct InProcessTransport {
    queue: std::collections::VecDeque<Envelope>,
}

impl InProcessTransport {
    /// A new, empty in-process transport.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for InProcessTransport {
    fn send(&mut self, env: Envelope) {
        self.queue.push_back(env);
    }
    fn drain(&mut self) -> Vec<Envelope> {
        self.queue.drain(..).collect()
    }
}

// ===========================================================================
// LoopbackTransport — REAL std::net TCP on 127.0.0.1 (single process, both
// endpoints local). Framing: [u32 LE length][payload]. Handles partial reads
// (a per-reader accumulation buffer) and one connection reset (transparent
// reconnect on the next send).
// ===========================================================================

/// Accumulates bytes off one incoming socket and yields complete frames,
/// tolerating partial reads (TCP may hand us a frame in several pieces).
struct FramedReader {
    stream: TcpStream,
    buf: Vec<u8>,
    /// EOF/reset seen — drop after draining whatever already buffered.
    dead: bool,
}

impl FramedReader {
    fn new(stream: TcpStream) -> std::io::Result<Self> {
        stream.set_read_timeout(Some(Duration::from_millis(20)))?;
        Ok(Self { stream, buf: Vec::new(), dead: false })
    }

    /// One non-blocking-ish read attempt (bounded by the read timeout), then
    /// extract every complete frame currently buffered. Partial frames stay in
    /// `buf` for a later poll.
    fn poll(&mut self) -> Vec<Vec<u8>> {
        let mut tmp = [0u8; 8192];
        match self.stream.read(&mut tmp) {
            Ok(0) => self.dead = true, // clean EOF
            Ok(n) => self.buf.extend_from_slice(&tmp[..n]),
            Err(e) => match e.kind() {
                // No data within the timeout — normal while polling.
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {}
                // Peer reset (the reconnect path exercises this) — retire reader.
                _ => self.dead = true,
            },
        }
        let mut frames = Vec::new();
        loop {
            if self.buf.len() < 4 {
                break;
            }
            let len = u32::from_le_bytes([self.buf[0], self.buf[1], self.buf[2], self.buf[3]]) as usize;
            if self.buf.len() < 4 + len {
                break; // frame not fully arrived yet (partial read)
            }
            let frame = self.buf[4..4 + len].to_vec();
            self.buf.drain(..4 + len);
            frames.push(frame);
        }
        frames
    }
}

/// Everything one node owns on the loopback network: its listener, its inbound
/// readers, and its outbound connections to peers (established lazily).
struct NodeNet {
    listener: TcpListener,
    inbound: Vec<FramedReader>,
    outbound: BTreeMap<NodeId, TcpStream>,
}

/// A REAL TCP loopback [`Transport`]: N nodes, each with a `127.0.0.1` listener,
/// all in one process. `send(a→b)` writes a length-framed [`Envelope`] over a
/// real socket from `a` to `b`; `drain` reads frames back off the sockets. It
/// guarantees complete delivery per drain by tracking `sent`/`received` frame
/// counts and reading until they match (so `drain` returns exactly the
/// in-flight generation — the same batching boundary as the in-process bus),
/// then the harness canonicalizes order.
pub struct LoopbackTransport {
    nodes: BTreeMap<NodeId, NodeNet>,
    addrs: BTreeMap<NodeId, std::net::SocketAddr>,
    sent: u64,
    received: u64,
    /// Observability: connections (re-)established — a reconnect proof asserts >0.
    pub reconnects: u64,
}

impl LoopbackTransport {
    /// Bind a `127.0.0.1` listener per node and record its address. Errors
    /// surface loudly (a bind failure is a real environment problem, not to be
    /// swallowed).
    pub fn bind(node_ids: &[NodeId]) -> std::io::Result<Self> {
        let mut nodes = BTreeMap::new();
        let mut addrs = BTreeMap::new();
        for id in node_ids {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            listener.set_nonblocking(true)?; // accept() polls, never blocks
            addrs.insert(id.clone(), listener.local_addr()?);
            nodes.insert(id.clone(), NodeNet { listener, inbound: Vec::new(), outbound: BTreeMap::new() });
        }
        Ok(Self { nodes, addrs, sent: 0, received: 0, reconnects: 0 })
    }

    /// Forcibly reset every established connection (both directions) — used at a
    /// QUIESCENT boundary (no frame in flight, `sent == received`) to prove the
    /// transport transparently reconnects on the next `send`. Buffers are empty
    /// at a quiescent boundary, so no frame is lost.
    pub fn reset_connections(&mut self) {
        for net in self.nodes.values_mut() {
            net.outbound.clear();
            net.inbound.clear();
        }
    }

    /// Connect `from`'s outbound stream to `to`'s listener (blocking connect to
    /// loopback is immediate); writes stay blocking (tiny frames), reads on the
    /// accepted side use a timeout for polling.
    fn connect(&mut self, from: &NodeId, to: &NodeId) -> std::io::Result<()> {
        let addr = self.addrs[to];
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true).ok();
        self.nodes.get_mut(from).expect("known node").outbound.insert(to.clone(), stream);
        self.reconnects += 1;
        Ok(())
    }

    /// Accept any pending inbound connections across every node (non-blocking).
    fn accept_pending(&mut self) {
        for net in self.nodes.values_mut() {
            loop {
                match net.listener.accept() {
                    Ok((stream, _)) => match FramedReader::new(stream) {
                        Ok(r) => net.inbound.push(r),
                        Err(_) => break,
                    },
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }
    }
}

impl Transport for LoopbackTransport {
    fn send(&mut self, env: Envelope) {
        let (from, to) = (env.from.clone(), env.to.clone());
        let payload = encode_envelope(&env);
        let mut frame = (payload.len() as u32).to_le_bytes().to_vec();
        frame.extend_from_slice(&payload);

        // Ensure a live outbound connection; (re)connect if absent.
        if !self.nodes[&from].outbound.contains_key(&to) {
            self.connect(&from, &to).expect("loopback connect");
        }
        // Write the whole frame; on a reset, reconnect ONCE and retry.
        let write_ok = {
            let s = self.nodes.get_mut(&from).unwrap().outbound.get_mut(&to).unwrap();
            s.write_all(&frame).and_then(|_| s.flush()).is_ok()
        };
        if !write_ok {
            self.nodes.get_mut(&from).unwrap().outbound.remove(&to);
            self.connect(&from, &to).expect("loopback reconnect");
            let s = self.nodes.get_mut(&from).unwrap().outbound.get_mut(&to).unwrap();
            s.write_all(&frame).and_then(|_| s.flush()).expect("loopback write after reconnect");
        }
        self.sent += 1;
    }

    fn drain(&mut self) -> Vec<Envelope> {
        // Read until every sent frame has been received: `drain` returns exactly
        // the in-flight generation (same boundary the in-process bus gives),
        // guaranteeing complete, deterministic-in-CONTENT delivery. Loopback
        // resolves in microseconds, so on the happy path neither bound below is
        // ever approached; they exist ONLY to turn a genuinely stuck socket into a
        // loud, *fast* panic instead of a silent hang. A WALL-CLOCK deadline is the
        // primary liveness bound (a per-reader 20ms read timeout means a poll can
        // block, so a pure iteration cap could take minutes to trip); a generous
        // poll cap is the secondary bound. Neither influences committed truth — the
        // deadline is only reachable on the failure path, so HARD RULE 4
        // (determinism of committed truth) is unaffected.
        const MAX_POLLS: u64 = 100_000;
        const STALL_DEADLINE: Duration = Duration::from_secs(5);
        let started = Instant::now();
        let mut delivered = Vec::new();
        let mut polls = 0u64;
        while self.received < self.sent {
            self.accept_pending();
            let mut progressed = false;
            for net in self.nodes.values_mut() {
                for r in net.inbound.iter_mut() {
                    for frame in r.poll() {
                        let env = decode_envelope(&frame).expect("loopback frame decodes (wire codec)");
                        delivered.push(env);
                        self.received += 1;
                        progressed = true;
                    }
                }
                net.inbound.retain(|r| !r.dead || !r.buf.is_empty());
            }
            polls += 1;
            assert!(
                polls < MAX_POLLS && started.elapsed() < STALL_DEADLINE,
                "loopback drain stalled: {} of {} frames delivered (progressed={}, polls={}, elapsed={:?})",
                self.received,
                self.sent,
                progressed,
                polls,
                started.elapsed()
            );
        }
        delivered
    }
}

// ===========================================================================
// NodeTransport (RCR-038) — a GENUINELY networked endpoint for ONE node, usable
// ACROSS SEPARATE OS PROCESSES (and, in principle, hosts).
//
// The difference from LoopbackTransport: LoopbackTransport owns EVERY node's
// socket inside ONE process and therefore knows the global sent/received frame
// count, letting `drain` block until the whole in-flight generation has landed.
// A node in a SEPARATE process has no such global knowledge — it owns exactly
// one listener + its outbound connections, and can only `poll` for whatever has
// arrived so far. So this is a distinct type (NOT a `Transport` impl): the
// `Transport::drain` contract is a single-process convenience that does not
// exist in true distribution. The consensus protocol itself already tolerates
// arrival-order, partial, and retried delivery (raft retransmits via heartbeat),
// which is exactly why the cross-process run stays correct without a global
// order — committed OUTCOME content is a function of the proposal, never the
// wire (HARD RULE 4).
//
// HONEST SCOPE: real length-framed TCP between separate processes on ONE host
// (127.0.0.1 or any configurable addr:port). It PROVES cross-process networked
// consensus. True multi-HOST across machines, and hostile-network partition/
// latency/loss testing, remain recorded OQ — the sockets here deliver reliably;
// adversarial delivery stays the `sim.rs` filter/mangle model. Reconnect/backoff
// use real time in the DELIVERY layer only.
// ===========================================================================

/// Bounded reconnect backoff state for one peer (real-time; delivery-layer only,
/// never influences committed truth).
struct Backoff {
    attempts: u32,
    next_try: Instant,
}

/// One node's endpoint on a REAL TCP network. Binds a configurable `addr:port`,
/// length-frames [`Envelope`]s to peers addressed by [`SocketAddr`], accepts
/// inbound connections, tolerates partial reads, reconnects with bounded
/// backoff, and offers a graceful shutdown. Designed to be driven by ONE process
/// hosting ONE raft node, so N such processes form a genuine networked cluster.
pub struct NodeTransport {
    me: NodeId,
    listener: TcpListener,
    local_addr: SocketAddr,
    peers: BTreeMap<NodeId, SocketAddr>,
    inbound: Vec<FramedReader>,
    outbound: BTreeMap<NodeId, TcpStream>,
    backoff: BTreeMap<NodeId, Backoff>,
    /// Observability: successful (re-)connections established to peers.
    pub reconnects: u64,
}

impl NodeTransport {
    /// Bind this node's listener at `listen` (`"127.0.0.1:0"` for an ephemeral
    /// port, or any concrete `addr:port`) and record the peer address map. The
    /// listener is non-blocking (accept polls, never blocks the event loop).
    pub fn bind(
        me: NodeId,
        listen: &str,
        peers: BTreeMap<NodeId, SocketAddr>,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind(listen)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        Ok(Self {
            me,
            listener,
            local_addr,
            peers,
            inbound: Vec::new(),
            outbound: BTreeMap::new(),
            backoff: BTreeMap::new(),
            reconnects: 0,
        })
    }

    /// This node's bound address (useful when it bound an ephemeral `:0` port).
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Install (or replace) the peer address map AFTER binding. This supports the
    /// ephemeral-port bootstrap: bind `:0`, learn our own address, publish it,
    /// discover the peers' addresses, then set them here — all without rebinding
    /// (which would change our port and invalidate what we published).
    pub fn set_peers(&mut self, peers: BTreeMap<NodeId, SocketAddr>) {
        self.peers = peers;
    }

    /// This endpoint's node id.
    pub fn node_id(&self) -> &NodeId {
        &self.me
    }

    /// Ensure a live outbound connection to `to`, honoring bounded backoff. A
    /// not-yet-reachable peer is NOT fatal (raft retransmits): returns `false`
    /// and schedules a later retry. A real environment bind/addr error surfaces
    /// as a failed connect (also non-fatal — retried).
    fn ensure_conn(&mut self, to: &NodeId) -> bool {
        if self.outbound.contains_key(to) {
            return true;
        }
        if let Some(b) = self.backoff.get(to) {
            if Instant::now() < b.next_try {
                return false; // still backing off
            }
        }
        let addr = match self.peers.get(to) {
            Some(a) => *a,
            None => return false, // unknown peer — never invent an address
        };
        match TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
            Ok(s) => {
                s.set_nodelay(true).ok();
                self.outbound.insert(to.clone(), s);
                self.reconnects += 1;
                self.backoff.remove(to);
                true
            }
            Err(_) => {
                let b = self
                    .backoff
                    .entry(to.clone())
                    .or_insert(Backoff { attempts: 0, next_try: Instant::now() });
                b.attempts = (b.attempts + 1).min(8);
                // Exponential, capped at 200ms — bounded so a briefly-absent peer
                // (e.g. still starting up) is retried promptly but not hammered.
                let delay_ms = (10u64.saturating_mul(1u64 << b.attempts)).min(200);
                b.next_try = Instant::now() + Duration::from_millis(delay_ms);
                false
            }
        }
    }

    /// Send one addressed [`Envelope`] to its target peer over real TCP. Best
    /// effort by design (the protocol tolerates loss and retransmits): if the
    /// peer is unreachable the frame is dropped and a heartbeat will carry the
    /// state later. On a write failure the connection is dropped and ONE
    /// immediate reconnect+retry is attempted.
    pub fn send(&mut self, env: Envelope) {
        let to = env.to.clone();
        let payload = encode_envelope(&env);
        let mut frame = (payload.len() as u32).to_le_bytes().to_vec();
        frame.extend_from_slice(&payload);

        if !self.ensure_conn(&to) {
            return; // peer not reachable yet — raft heartbeat will retransmit
        }
        let ok = {
            let s = self.outbound.get_mut(&to).expect("connection just ensured");
            s.write_all(&frame).and_then(|_| s.flush()).is_ok()
        };
        if !ok {
            self.outbound.remove(&to);
            self.backoff.remove(&to); // allow an immediate reconnect attempt
            if self.ensure_conn(&to) {
                let s = self.outbound.get_mut(&to).expect("reconnected");
                let _ = s.write_all(&frame).and_then(|_| s.flush());
            }
        }
    }

    /// Accept any pending inbound connections, then read every complete frame
    /// currently available across all inbound sockets (non-blocking) and decode
    /// them into envelopes. Partial frames stay buffered for a later poll; dead
    /// readers are retired once fully drained.
    pub fn poll(&mut self) -> Vec<Envelope> {
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => match FramedReader::new(stream) {
                    Ok(r) => self.inbound.push(r),
                    Err(_) => break,
                },
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        let mut out = Vec::new();
        for r in self.inbound.iter_mut() {
            for frame in r.poll() {
                if let Some(env) = decode_envelope(&frame) {
                    out.push(env);
                }
                // A malformed frame on a real socket is dropped (never guessed at);
                // the strict codec already refused it. Consensus tolerates the loss.
            }
        }
        self.inbound.retain(|r| !r.dead || !r.buf.is_empty());
        out
    }

    /// Graceful shutdown: half-close every established connection (both
    /// directions) so peers observe a clean EOF, then drop all sockets.
    pub fn shutdown(&mut self) {
        for s in self.outbound.values() {
            let _ = s.shutdown(Shutdown::Both);
        }
        for r in self.inbound.iter() {
            let _ = r.stream.shutdown(Shutdown::Both);
        }
        self.outbound.clear();
        self.inbound.clear();
    }
}

// ===========================================================================
// TransportRound — the equivalence harness. Drives a small cluster over ANY
// Transport, imposing a canonical processing order so committed truth is a pure
// function of (seed, ticks) regardless of the delivery layer's arrival order.
// ===========================================================================

/// A small cluster driven over a [`Transport`]. The driver is IDENTICAL for the
/// in-process and loopback runs — the transport is the ONLY variable — so equal
/// committed digests isolate transport-agnosticism as the proven property.
///
/// Determinism discipline (RCR-032 invariant): after every `drain`, delivered
/// envelopes are sorted by their canonical [`encode_envelope`] bytes BEFORE the
/// pure protocol consumes them. A socket may hand back bytes in any order; the
/// committed log cannot depend on it.
pub struct TransportRound {
    nodes: BTreeMap<NodeId, RaftNode>,
    ids: Vec<NodeId>,
}

impl TransportRound {
    /// Build `n` replicas `n1..nN` with the SAME seeding as
    /// [`crate::sim::SimCluster::new`] (identical construction ⇒ identical
    /// election-timeout draws ⇒ the loopback run reproduces the in-process one).
    pub fn new(n: usize, seed: u64) -> Self {
        let ids: Vec<NodeId> = (1..=n).map(|i| NodeId(format!("n{i}"))).collect();
        let mut seeder = crate::raft::DetRng::new(seed);
        let nodes = ids
            .iter()
            .map(|id| (id.clone(), RaftNode::new(id.clone(), ids.clone(), seeder.next_u64())))
            .collect();
        Self { nodes, ids }
    }

    /// Node ids, in deterministic order.
    pub fn node_ids(&self) -> &[NodeId] {
        &self.ids
    }

    /// The live leader with the highest term, if any.
    pub fn current_leader(&self) -> Option<NodeId> {
        self.nodes
            .values()
            .filter(|n| n.role() == Role::Leader)
            .map(|n| (n.id().clone(), n.current_term().0))
            .max_by_key(|(_, t)| *t)
            .map(|(id, _)| id)
    }

    fn commit_of(&self, id: &NodeId) -> u64 {
        self.nodes[id].commit_index().0
    }

    /// Deterministic digest of every node's full replicated state (role, term,
    /// commit index, and the byte-identical log). Two runs that reach the same
    /// protocol state — as the in-process and loopback runs must — produce equal
    /// digests. Byte-identical logs across transports is the equivalence proof.
    pub fn committed_digest(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
        let mut eat = |bytes: &[u8]| {
            for b in bytes {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        for id in &self.ids {
            let n = &self.nodes[id];
            eat(id.0.as_bytes());
            eat(&[match n.role() {
                Role::Leader => 1,
                Role::Follower => 2,
                Role::Candidate => 3,
                Role::Learner => 4,
            }]);
            eat(&n.current_term().0.to_le_bytes());
            eat(&n.commit_index().0.to_le_bytes());
            for e in n.log() {
                // Reuse the wire codec so the digest folds the SAME bytes the
                // transport would carry (one canonical entry encoding).
                let env = Envelope {
                    from: id.clone(),
                    to: id.clone(),
                    body: MsgBody::AppendEntries {
                        term: e.term,
                        prev_log_index: e.index.0.saturating_sub(1),
                        prev_log_term: Term(0),
                        entries: vec![e.clone()],
                        leader_commit: 0,
                    },
                };
                eat(&encode_envelope(&env));
            }
        }
        h
    }

    /// The **timing-independent** digest of the first committed outcome (log
    /// index 1) on any node that has committed it — the quantity RCR-038's
    /// cross-process run compares against. Returns `None` if nothing is committed
    /// yet or index 1 is not an outcome. All committed replicas agree (raft
    /// safety), so any committed node yields the same value.
    pub fn committed_outcome_digest(&self) -> Option<u64> {
        for id in &self.ids {
            let n = &self.nodes[id];
            if n.commit_index().0 >= 1 {
                if let Some(e) = n.log().iter().find(|e| e.index.0 == 1) {
                    if let EntryKind::Outcome(o) = &e.kind {
                        return Some(outcome_content_digest(o));
                    }
                }
            }
        }
        None
    }

    /// Tick every node once (deterministic id order), sending any emitted
    /// envelopes into the transport.
    fn tick_all<T: Transport>(&mut self, t: &mut T) {
        for id in &self.ids {
            let out = self.nodes.get_mut(id).expect("node").tick();
            for env in out {
                t.send(env);
            }
        }
    }

    /// Deliver one generation: drain the transport, CANONICALIZE the order, step
    /// each envelope into its target, and feed the outputs back. Returns whether
    /// anything was delivered.
    fn deliver_generation<T: Transport>(&mut self, t: &mut T) -> bool {
        let mut delivered = t.drain();
        if delivered.is_empty() {
            return false;
        }
        // The RCR-032 invariant: order-into-truth is fixed HERE, not by the wire.
        delivered.sort_by(|a, b| encode_envelope(a).cmp(&encode_envelope(b)));
        for env in delivered {
            if let Some(node) = self.nodes.get_mut(&env.to) {
                let out = node.step(env);
                for e in out {
                    t.send(e);
                }
            }
        }
        true
    }

    /// Drive a full round over `t`: elect a leader, propose ONE outcome at that
    /// leader, and run until the leader has committed it. Returns the leader id.
    /// Panics (loudly, never silently) if the round does not converge within a
    /// deterministic step budget.
    pub fn elect_and_commit_one<T: Transport>(&mut self, t: &mut T, kind: EntryKind) -> NodeId {
        const MAX_ROUNDS: u64 = 600;
        let mut proposed_at: Option<NodeId> = None;
        for _ in 0..MAX_ROUNDS {
            self.tick_all(t);
            while self.deliver_generation(t) {}

            match &proposed_at {
                None => {
                    if let Some(leader) = self.current_leader() {
                        // Only propose once the leader can actually commit its own
                        // term (empty log ⇒ immediately valid) — here the log is
                        // empty at election, so propose straight away.
                        let out = self
                            .nodes
                            .get_mut(&leader)
                            .expect("leader")
                            .client_propose(kind.clone())
                            .expect("leader accepts the proposal");
                        for env in out.1 {
                            t.send(env);
                        }
                        while self.deliver_generation(t) {}
                        proposed_at = Some(leader);
                    }
                }
                Some(leader) => {
                    if self.commit_of(leader) >= 1 {
                        return leader.clone();
                    }
                }
            }
        }
        panic!("cluster round did not elect-and-commit within the deterministic budget");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(tag: &str) -> EntryKind {
        EntryKind::Outcome(Outcome {
            digest: ContentHash(format!("h:{tag}")),
            payload: tag.as_bytes().to_vec(),
        })
    }

    /// The wire codec round-trips EVERY Envelope/MsgBody variant exactly and
    /// refuses trailing garbage / truncation (strict, lossless-or-loud).
    #[test]
    fn wire_codec_round_trips_every_variant_and_refuses_garbage() {
        let n = |s: &str| NodeId(s.into());
        let entries = vec![
            LogEntry { term: Term(3), index: LogIndex(1), kind: outcome("e1") },
            LogEntry {
                term: Term(3),
                index: LogIndex(2),
                kind: EntryKind::Membership(Membership::Joint {
                    old_voters: vec![n("n1"), n("n2")],
                    new_voters: vec![n("n2"), n("n3")],
                    learners: vec![n("n4")],
                }),
            },
        ];
        let bodies = vec![
            MsgBody::RequestVote { term: Term(7), last_log_index: 4, last_log_term: Term(6), transfer: true },
            MsgBody::VoteReply { term: Term(7), granted: false },
            MsgBody::AppendEntries {
                term: Term(3),
                prev_log_index: 0,
                prev_log_term: Term(0),
                entries: entries.clone(),
                leader_commit: 2,
            },
            MsgBody::AppendReply { term: Term(3), success: true, match_index: 2 },
            MsgBody::TimeoutNow { term: Term(9) },
        ];
        for body in bodies {
            let env = Envelope { from: n("n1"), to: n("n2"), body };
            let enc = encode_envelope(&env);
            assert_eq!(decode_envelope(&enc).as_ref(), Some(&env), "round-trip");
            let mut garbage = enc.clone();
            garbage.push(0);
            assert_eq!(decode_envelope(&garbage), None, "trailing garbage refused");
            assert_eq!(decode_envelope(&enc[..enc.len() - 1]), None, "truncation refused");
        }
    }

    /// The in-process transport is a faithful FIFO delivery: send-then-drain
    /// returns exactly what was sent, and a second drain is empty.
    #[test]
    fn in_process_transport_is_fifo_take_all() {
        let n = |s: &str| NodeId(s.into());
        let mut t = InProcessTransport::new();
        let e1 = Envelope { from: n("n1"), to: n("n2"), body: MsgBody::TimeoutNow { term: Term(1) } };
        let e2 = Envelope { from: n("n2"), to: n("n3"), body: MsgBody::TimeoutNow { term: Term(2) } };
        t.send(e1.clone());
        t.send(e2.clone());
        assert_eq!(t.drain(), vec![e1, e2]);
        assert!(t.drain().is_empty());
    }

    /// THE EQUIVALENCE PROOF (RCR-032): a small cluster round (leader election +
    /// one commit) driven over REAL loopback TCP sockets commits the byte-
    /// identical truth as the same round over the in-process bus. Determinism is
    /// preserved because the protocol is pure and the harness canonicalizes
    /// delivery order — the socket only moves bytes.
    #[test]
    fn loopback_round_commits_identical_truth_to_in_process() {
        let seed = 0xC0FFEE;
        // In-process baseline.
        let mut r_mem = TransportRound::new(3, seed);
        let mut mem = InProcessTransport::new();
        let leader_mem = r_mem.elect_and_commit_one(&mut mem, outcome("payload"));
        let digest_mem = r_mem.committed_digest();

        // Real-socket run — identical driver, identical seed, TCP transport.
        let mut r_net = TransportRound::new(3, seed);
        let mut net = LoopbackTransport::bind(r_net.node_ids()).expect("bind loopback");
        let leader_net = r_net.elect_and_commit_one(&mut net, outcome("payload"));
        let digest_net = r_net.committed_digest();

        assert_eq!(leader_mem, leader_net, "same leader elected over real sockets");
        assert_eq!(
            digest_mem, digest_net,
            "committed truth is byte-identical over the in-process bus and real TCP sockets"
        );
        // Non-vacuity: an entry actually committed on the real-socket run.
        assert!(r_net.commit_of(&leader_net) >= 1, "the round committed one entry over sockets");
    }

    /// Reconnect + framing under a REAL connection reset: the round survives a
    /// full transport connection drop at a quiescent boundary, transparently
    /// reconnects on the next send, and still commits the identical truth.
    #[test]
    fn loopback_round_survives_a_connection_reset() {
        let seed = 0xC0FFEE;
        let mut baseline = TransportRound::new(3, seed);
        let mut mem = InProcessTransport::new();
        baseline.elect_and_commit_one(&mut mem, outcome("payload"));
        let digest_ref = baseline.committed_digest();

        let mut r = TransportRound::new(3, seed);
        let mut net = LoopbackTransport::bind(r.node_ids()).expect("bind loopback");
        // Drive the first generation, then reset EVERY connection at the
        // quiescent boundary (sent == received, buffers empty ⇒ no frame lost).
        r.tick_all(&mut net);
        while r.deliver_generation(&mut net) {}
        assert_eq!(net.sent, net.received, "quiescent boundary before reset");
        let reconnects_before = net.reconnects;
        net.reset_connections();
        // Finish the round — every subsequent send must reconnect and still land.
        let leader = r.elect_and_commit_one(&mut net, outcome("payload"));
        assert!(
            net.reconnects > reconnects_before,
            "the transport re-established connections after the reset"
        );
        assert_eq!(
            r.committed_digest(),
            digest_ref,
            "identical committed truth despite a real connection reset"
        );
        assert!(r.commit_of(&leader) >= 1);
    }

    // -----------------------------------------------------------------------
    // NodeTransport (RCR-038) — per-node networked endpoint tests. These run
    // TWO endpoints inside ONE process over real TCP (fast, no election timing),
    // isolating the genuinely-networked send/poll/reconnect/shutdown surface.
    // The full cross-PROCESS proof is `tests/multiprocess.rs`.
    // -----------------------------------------------------------------------

    /// Poll `nt` until it yields at least one envelope or a generous deadline
    /// trips (loopback resolves in microseconds; the bound only turns a genuinely
    /// stuck socket into a fast, loud failure — never influences truth).
    fn recv_some(nt: &mut NodeTransport) -> Vec<Envelope> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let got = nt.poll();
            if !got.is_empty() {
                return got;
            }
            assert!(Instant::now() < deadline, "recv_some stalled (no frame arrived)");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Bind two endpoints that know each other's real addresses.
    fn pair() -> (NodeTransport, NodeTransport) {
        let n1 = NodeId("n1".into());
        let n2 = NodeId("n2".into());
        // Bind ephemeral ports first, then hand each the other's real address.
        let a = NodeTransport::bind(n1.clone(), "127.0.0.1:0", BTreeMap::new()).expect("bind n1");
        let b = NodeTransport::bind(n2.clone(), "127.0.0.1:0", BTreeMap::new()).expect("bind n2");
        let (a_addr, b_addr) = (a.local_addr(), b.local_addr());
        let mut a = a;
        let mut b = b;
        a.peers.insert(n2, b_addr);
        b.peers.insert(n1, a_addr);
        (a, b)
    }

    /// A real length-framed [`Envelope`] crosses a genuine TCP socket between two
    /// NodeTransport endpoints and decodes byte-identically at the peer.
    #[test]
    fn node_transport_exchanges_framed_envelope_both_ways() {
        let n = |s: &str| NodeId(s.into());
        let (mut a, mut b) = pair();
        let e1 = Envelope {
            from: n("n1"),
            to: n("n2"),
            body: MsgBody::RequestVote {
                term: Term(4),
                last_log_index: 2,
                last_log_term: Term(3),
                transfer: false,
            },
        };
        a.send(e1.clone());
        assert_eq!(recv_some(&mut b), vec![e1], "n1 -> n2 over real TCP");

        let e2 = Envelope {
            from: n("n2"),
            to: n("n1"),
            body: MsgBody::AppendReply { term: Term(4), success: true, match_index: 2 },
        };
        b.send(e2.clone());
        assert_eq!(recv_some(&mut a), vec![e2], "n2 -> n1 over real TCP");
    }

    /// Multiple frames sent back-to-back all arrive intact (framing survives
    /// coalesced/partial TCP reads — FramedReader reassembly).
    #[test]
    fn node_transport_delivers_multiple_frames() {
        let n = |s: &str| NodeId(s.into());
        let (mut a, mut b) = pair();
        for t in 1..=5u64 {
            a.send(Envelope {
                from: n("n1"),
                to: n("n2"),
                body: MsgBody::TimeoutNow { term: Term(t) },
            });
        }
        let mut got = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while got.len() < 5 {
            got.extend(b.poll());
            assert!(Instant::now() < deadline, "only {} of 5 frames arrived", got.len());
            std::thread::sleep(Duration::from_millis(1));
        }
        let terms: Vec<u64> = got
            .iter()
            .map(|e| match &e.body {
                MsgBody::TimeoutNow { term } => term.0,
                _ => panic!("unexpected body"),
            })
            .collect();
        assert_eq!(terms, vec![1, 2, 3, 4, 5], "all five frames arrived in order on one stream");
    }

    /// After a graceful shutdown drops the connection, the next send transparently
    /// reconnects (the `reconnects` counter grows) and still delivers.
    #[test]
    fn node_transport_reconnects_after_shutdown() {
        let n = |s: &str| NodeId(s.into());
        let (mut a, mut b) = pair();
        let e1 = Envelope { from: n("n1"), to: n("n2"), body: MsgBody::TimeoutNow { term: Term(1) } };
        a.send(e1.clone());
        assert_eq!(recv_some(&mut b), vec![e1]);
        let reconnects_before = a.reconnects;

        // Drop every connection on both ends (a real peer reset).
        a.shutdown();
        b.shutdown();

        let e2 = Envelope { from: n("n1"), to: n("n2"), body: MsgBody::TimeoutNow { term: Term(2) } };
        a.send(e2.clone());
        assert!(a.reconnects > reconnects_before, "send re-established the connection");
        assert_eq!(recv_some(&mut b), vec![e2], "delivery resumes after reconnect");
    }

    /// The committed-outcome content digest is timing-independent: it depends
    /// ONLY on the outcome's address + payload, never on term/index — so the
    /// cross-process run (any leader, any term) can compare against it.
    #[test]
    fn outcome_content_digest_ignores_term_and_index() {
        let o = Outcome { digest: ContentHash("h:payload".into()), payload: b"payload".to_vec() };
        let d = outcome_content_digest(&o);
        // Same content in entries with different terms/indexes ⇒ same digest.
        let e_a = LogEntry { term: Term(2), index: LogIndex(1), kind: EntryKind::Outcome(o.clone()) };
        let e_b = LogEntry { term: Term(99), index: LogIndex(7), kind: EntryKind::Outcome(o.clone()) };
        for e in [e_a, e_b] {
            if let EntryKind::Outcome(oo) = &e.kind {
                assert_eq!(outcome_content_digest(oo), d);
            }
        }
        // Different content ⇒ different digest (non-vacuity).
        let other = Outcome { digest: ContentHash("h:other".into()), payload: b"other".to_vec() };
        assert_ne!(outcome_content_digest(&other), d);
    }

    /// The in-process baseline exposes the same timing-independent committed
    /// outcome digest that the cross-process nodes print — the comparison anchor.
    #[test]
    fn transport_round_exposes_committed_outcome_digest() {
        let mut r = TransportRound::new(3, 0xC0FFEE);
        let mut mem = InProcessTransport::new();
        r.elect_and_commit_one(&mut mem, outcome("payload"));
        let d = r.committed_outcome_digest().expect("committed an outcome");
        let expected =
            outcome_content_digest(&Outcome { digest: ContentHash("h:payload".into()), payload: b"payload".to_vec() });
        assert_eq!(d, expected, "baseline digest == digest of the proposed outcome");
    }
}

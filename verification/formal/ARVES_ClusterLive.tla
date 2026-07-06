--------------------------- MODULE ARVES_ClusterLive ---------------------------
(*****************************************************************************)
(* ARVES CLUSTER — LIVENESS + DURABILITY Formal Specification (TLA+).         *)
(*                                                                           *)
(* Status : EVIDENCE under verification/ (LIVING). NOT normative spec; edits *)
(*          no frozen .docx (ED-001). Companion to ARVES_Cluster.tla, which  *)
(*          checks per-shard Raft SAFETY only. THIS module extends the same  *)
(*          protocol with the two properties SAFETY alone cannot express:    *)
(*                                                                           *)
(*   (1) LIVENESS  — under weak fairness on the protocol steps, and with the *)
(*       partial-synchrony assumption Raft's liveness genuinely requires,    *)
(*       a leader is EVENTUALLY elected and a client entry is EVENTUALLY      *)
(*       committed. The cluster makes progress; it does not stall.           *)
(*   (2) DURABILITY — a node may CRASH (lose all volatile state) and RESTART *)
(*       from its persistent log. Committed truth SURVIVES every crash: it   *)
(*       remains durably held on a quorum and is never lost or contradicted. *)
(*                                                                           *)
(* Grounds: arves-consensus/src/raft.rs (the deterministic per-shard Raft    *)
(*          core: persistent {currentTerm, votedFor, log} vs volatile        *)
(*          {role, commitIndex, votes}) and arves-kernel/src/cluster.rs (the *)
(*          ClusterKernel that acks a caller only after quorum commit and     *)
(*          reconstructs volatile state by replaying the durable log on        *)
(*          restart — IDR-005 append-only WAL, ORCH-003 deterministic replay). *)
(*                                                                           *)
(* Maps to: IDR-001 (per-shard CP, quorum commit), IDR-004 (per-shard         *)
(*          election; a crashed leader is superseded by term), IDR-005        *)
(*          (append-only durable log survives crash), ORCH-003 (replay        *)
(*          rebuilds volatile truth from the durable log). SHARD-001: one     *)
(*          group scope. This is the LIVENESS+DURABILITY successor the        *)
(*          safety-only ARVES_Cluster.tla header deferred as out of scope.     *)
(*                                                                           *)
(* ─────────────────────────── HONEST SCOPE ─────────────────────────────    *)
(*  - Model of the PROTOCOL, not of the Rust. A bug in raft.rs that is not a  *)
(*    protocol-level design flaw is not caught here (the Rust suite + sim     *)
(*    harness are that evidence).                                            *)
(*  - LIVENESS is checked under an EXPLICIT partial-synchrony assumption. Raft *)
(*    provably CANNOT guarantee liveness under fully asynchronous, perpetually *)
(*    dueling elections (split votes can recur forever; at a bounded MaxTerm  *)
(*    three candidates can split 1-1-1 and deadlock with no term left to      *)
(*    break the tie). Real Raft breaks the tie with randomized election       *)
(*    timeouts. We model that outcome honestly with the constant SoleCandidate:*)
(*    when set to a server, only THAT server may time out — i.e. "eventually  *)
(*    one server's timeout fires uncontested", the exact partial-synchrony     *)
(*    condition under which Raft liveness holds. This is an ASSUMPTION stated  *)
(*    up front, not a hidden trick; with SoleCandidate = Nil (full dueling)   *)
(*    the liveness property is KNOWN to fail and we do not claim it.          *)
(*  - DURABILITY is checked as exhaustive SAFETY (no fairness needed): with   *)
(*    full nondeterministic election AND up to MaxCrash crash/restart events, *)
(*    every committed entry stays quorum-durable and committed truth never    *)
(*    diverges. Crash = lose volatile {role, commitIndex, votes}; KEEP        *)
(*    persistent {currentTerm, votedFor, log}. This is the faithful Raft       *)
(*    persistence boundary.                                                   *)
(*  - Message layer, joint-consensus membership, snapshot/compaction, and     *)
(*    wire format remain abstracted (as in ARVES_Cluster.tla).                *)
(*****************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    Server,        \* Finite set of replicas, e.g. {n1, n2, n3}.
    Nil,           \* "voted for nobody" / "no sole candidate" sentinel (not in Server).
    MaxTerm,       \* Bound on election terms (finite instance).
    MaxLogLen,     \* Bound on log length (finite instance).
    MaxCrash,      \* Bound on crash/restart events (finite instance). 0 disables crashes.
    SoleCandidate  \* Nil  => any server may time out (full dueling; durability/safety mode).
                   \* srv  => only srv may time out (partial-synchrony liveness mode).

Follower  == "Follower"
Candidate == "Candidate"
Leader    == "Leader"

Quorum == { Q \in SUBSET Server : Cardinality(Q) * 2 > Cardinality(Server) }

Min(a, b) == IF a < b THEN a ELSE b
Max(a, b) == IF a > b THEN a ELSE b

VARIABLES
    state,          \* [Server -> {Follower, Candidate, Leader}]  (VOLATILE)
    currentTerm,    \* [Server -> 0..MaxTerm]                     (PERSISTENT)
    votedFor,       \* [Server -> Server \cup {Nil}]              (PERSISTENT)
    votesGranted,   \* [Server -> SUBSET Server]                  (VOLATILE)
    log,            \* [Server -> Seq([term |-> 0..MaxTerm])]     (PERSISTENT — durable WAL, IDR-005)
    commitIndex,    \* [Server -> Nat]                            (VOLATILE — rebuilt on restart)
    committed,      \* history: SUBSET [idx, term, cterm] — every fact acked to a client
    crashes         \* Nat : number of crash events so far (bounds the instance)

vars == << state, currentTerm, votedFor, votesGranted, log, commitIndex, committed, crashes >>

(*---------------------------------------------------------------------------*)
(* Helpers (identical to ARVES_Cluster.tla)                                  *)
(*---------------------------------------------------------------------------*)
LastTerm(l) == IF Len(l) = 0 THEN 0 ELSE l[Len(l)].term

LogUpToDate(cand, v) ==
    \/ LastTerm(log[cand]) > LastTerm(log[v])
    \/ /\ LastTerm(log[cand]) = LastTerm(log[v])
       /\ Len(log[cand]) >= Len(log[v])

PrevMatch(f, ldr, k) ==
    IF k = 0 THEN TRUE ELSE log[f][k].term = log[ldr][k].term

Conflict(f, ldr, k) ==
    IF k > Len(log[ldr]) THEN TRUE ELSE log[f][k].term # log[ldr][k].term

\* Partial-synchrony gate on elections (see HONEST SCOPE). SoleCandidate = Nil
\* => any server may time out (full dueling). SoleCandidate = s => only s may.
CanTimeout(i) == (SoleCandidate = Nil) \/ (i = SoleCandidate)

TypeOK ==
    /\ state \in [Server -> {Follower, Candidate, Leader}]
    /\ currentTerm \in [Server -> 0..MaxTerm]
    /\ votedFor \in [Server -> (Server \cup {Nil})]
    /\ votesGranted \in [Server -> SUBSET Server]
    /\ \A i \in Server : \A n \in DOMAIN log[i] : log[i][n].term \in 0..MaxTerm
    /\ \A i \in Server : commitIndex[i] \in 0..Len(log[i])
    /\ crashes \in 0..MaxCrash

Init ==
    /\ state = [i \in Server |-> Follower]
    /\ currentTerm = [i \in Server |-> 0]
    /\ votedFor = [i \in Server |-> Nil]
    /\ votesGranted = [i \in Server |-> {}]
    /\ log = [i \in Server |-> << >>]
    /\ commitIndex = [i \in Server |-> 0]
    /\ committed = {}
    /\ crashes = 0

(*---------------------------------------------------------------------------*)
(* Actions                                                                   *)
(*---------------------------------------------------------------------------*)

Timeout(i) ==
    /\ CanTimeout(i)
    /\ state[i] \in {Follower, Candidate}
    /\ currentTerm[i] < MaxTerm
    /\ currentTerm' = [currentTerm EXCEPT ![i] = currentTerm[i] + 1]
    /\ state' = [state EXCEPT ![i] = Candidate]
    /\ votedFor' = [votedFor EXCEPT ![i] = i]
    /\ votesGranted' = [votesGranted EXCEPT ![i] = {i}]
    /\ UNCHANGED << log, commitIndex, committed, crashes >>

Vote(cand, v) ==
    /\ cand # v
    /\ state[cand] = Candidate
    /\ currentTerm[cand] >= currentTerm[v]
    /\ LogUpToDate(cand, v)
    /\ \/ currentTerm[cand] > currentTerm[v]
       \/ /\ currentTerm[cand] = currentTerm[v]
          /\ votedFor[v] \in {Nil, cand}
    /\ currentTerm' = [currentTerm EXCEPT ![v] = currentTerm[cand]]
    /\ state' = [state EXCEPT ![v] = Follower]
    /\ votedFor' = [votedFor EXCEPT ![v] = cand]
    /\ votesGranted' = [votesGranted EXCEPT ![cand] = votesGranted[cand] \cup {v}]
    /\ UNCHANGED << log, commitIndex, committed, crashes >>

BecomeLeader(i) ==
    /\ state[i] = Candidate
    /\ votesGranted[i] \in Quorum
    /\ state' = [state EXCEPT ![i] = Leader]
    /\ UNCHANGED << currentTerm, votedFor, votesGranted, log, commitIndex, committed, crashes >>

ClientRequest(i) ==
    /\ state[i] = Leader
    /\ Len(log[i]) < MaxLogLen
    /\ log' = [log EXCEPT ![i] = Append(log[i], [term |-> currentTerm[i]])]
    /\ UNCHANGED << state, currentTerm, votedFor, votesGranted, commitIndex, committed, crashes >>

Replicate(ldr, f) ==
    /\ ldr # f
    /\ state[ldr] = Leader
    /\ currentTerm[ldr] >= currentTerm[f]
    /\ LET k == Len(log[f])
       IN \/ /\ k < Len(log[ldr])
             /\ PrevMatch(f, ldr, k)
             /\ log' = [log EXCEPT ![f] = Append(log[f], log[ldr][k+1])]
             /\ commitIndex' = [commitIndex EXCEPT
                    ![f] = Max(commitIndex[f], Min(commitIndex[ldr], k + 1))]
          \/ /\ k > 0
             /\ k > commitIndex[f]
             /\ Conflict(f, ldr, k)
             /\ log' = [log EXCEPT ![f] = SubSeq(log[f], 1, k - 1)]
             /\ UNCHANGED commitIndex
    /\ currentTerm' = [currentTerm EXCEPT ![f] = currentTerm[ldr]]
    /\ state' = [state EXCEPT ![f] = Follower]
    /\ votedFor' = [votedFor EXCEPT
           ![f] = IF currentTerm[ldr] > currentTerm[f] THEN Nil ELSE votedFor[f]]
    /\ UNCHANGED << votesGranted, committed, crashes >>

AdvanceCommit(i) ==
    /\ state[i] = Leader
    /\ \E n \in (commitIndex[i] + 1)..Len(log[i]) :
        /\ log[i][n].term = currentTerm[i]
        /\ { k \in Server : Len(log[k]) >= n /\ log[k][n].term = log[i][n].term } \in Quorum
        /\ commitIndex' = [commitIndex EXCEPT ![i] = n]
        /\ committed' = committed \cup
               { [ idx |-> m, term |-> log[i][m].term, cterm |-> currentTerm[i] ]
                 : m \in (commitIndex[i] + 1)..n }
    /\ UNCHANGED << state, currentTerm, votedFor, votesGranted, log, crashes >>

\* Crash(i): node i crashes and restarts from its PERSISTENT state. It loses ALL
\* volatile state — role reverts to Follower, commitIndex resets to 0 (it will be
\* re-learned from the leader), collected votes are discarded — but KEEPS its
\* persistent {currentTerm, votedFor, log}: the durable WAL (IDR-005) is exactly
\* what a restart replays (ORCH-003). `committed` (the god's-eye record of what was
\* acked to callers) is UNCHANGED — the whole durability question is whether those
\* acked facts survive; the model must not "help" by editing that history.
\* Bounded by MaxCrash so the instance stays finite.
Crash(i) ==
    /\ crashes < MaxCrash
    /\ state' = [state EXCEPT ![i] = Follower]
    /\ commitIndex' = [commitIndex EXCEPT ![i] = 0]
    /\ votesGranted' = [votesGranted EXCEPT ![i] = {}]
    /\ crashes' = crashes + 1
    /\ UNCHANGED << currentTerm, votedFor, log, committed >>

Next ==
    \/ \E i \in Server : Timeout(i)
    \/ \E cand \in Server, v \in Server : Vote(cand, v)
    \/ \E i \in Server : BecomeLeader(i)
    \/ \E i \in Server : ClientRequest(i)
    \/ \E ldr \in Server, f \in Server : Replicate(ldr, f)
    \/ \E i \in Server : AdvanceCommit(i)
    \/ \E i \in Server : Crash(i)

(*---------------------------------------------------------------------------*)
(* Fairness (LIVENESS mode only). Weak fairness on every protocol step so no  *)
(* continuously-enabled step is starved. Combined with SoleCandidate # Nil    *)
(* (partial synchrony) this yields eventual election + eventual commit. Note   *)
(* Crash carries NO fairness: crashes are permitted (bounded) but never        *)
(* forced, so a fair run may stop crashing and then make progress — the         *)
(* honest "eventually the environment stops crashing" assumption.              *)
(*---------------------------------------------------------------------------*)
Fairness ==
    /\ \A i \in Server : WF_vars(Timeout(i))
    /\ \A c \in Server, v \in Server : WF_vars(Vote(c, v))
    /\ \A i \in Server : WF_vars(BecomeLeader(i))
    /\ \A i \in Server : WF_vars(ClientRequest(i))
    /\ \A l \in Server, f \in Server : WF_vars(Replicate(l, f))
    /\ \A i \in Server : WF_vars(AdvanceCommit(i))

\* Safety / durability spec (no fairness): invariants hold in every reachable state.
Spec == Init /\ [][Next]_vars

\* Liveness spec: adds weak fairness. Used only by the liveness cfg.
FairSpec == Spec /\ Fairness

(*===========================================================================*)
(* SAFETY + DURABILITY INVARIANTS                                            *)
(*===========================================================================*)

\* The Raft safety invariants (same statements as ARVES_Cluster.tla) — they must
\* continue to hold across crash/restart, which is itself a durability claim:
\* committed truth never diverges even when nodes crash and rejoin.
ElectionSafety ==
    \A i, j \in Server :
        (state[i] = Leader /\ state[j] = Leader /\ currentTerm[i] = currentTerm[j])
            => i = j

LogMatching ==
    \A i, j \in Server :
        \A n \in (DOMAIN log[i]) \cap (DOMAIN log[j]) :
            (log[i][n].term = log[j][n].term)
                => (\A m \in 1..n : log[i][m] = log[j][m])

StateMachineSafety ==
    \A e1, e2 \in committed :
        (e1.idx = e2.idx) => (e1.term = e2.term)

LeaderCompleteness ==
    \A e \in committed :
        \A i \in Server :
            (state[i] = Leader /\ currentTerm[i] > e.cterm)
                => (Len(log[i]) >= e.idx /\ log[i][e.idx].term = e.term)

LinearizableCommit ==
    \A i, j \in Server :
        \A n \in 1..Min(commitIndex[i], commitIndex[j]) :
            log[i][n].term = log[j][n].term

\* -- DURABILITY : every committed entry is durably held, at its (idx, term), by
\* a QUORUM of servers' PERSISTENT logs. Because a majority durably retains it and
\* any two quorums intersect, the entry survives ANY set of crashes (a crash never
\* touches the log) and NO later leader can be elected without it (Leader
\* Completeness), so committed truth is recoverable and can never be lost. This is
\* the formal statement of "acked truth is durable" that crash/recovery must uphold.
DurableOnQuorum ==
    \A e \in committed :
        { k \in Server : Len(log[k]) >= e.idx /\ log[k][e.idx].term = e.term } \in Quorum

\* The full safety+durability conjunction checked in every reachable state (incl.
\* post-crash states) by the durability cfg.
DurableSafetyInv ==
    /\ TypeOK
    /\ ElectionSafety
    /\ LogMatching
    /\ StateMachineSafety
    /\ LeaderCompleteness
    /\ LinearizableCommit
    /\ DurableOnQuorum

(*===========================================================================*)
(* LIVENESS PROPERTIES (checked by the liveness cfg, under FairSpec)          *)
(*===========================================================================*)

\* A leader is EVENTUALLY elected. Under partial synchrony (SoleCandidate # Nil)
\* + weak fairness, the uncontested candidate collects a quorum and becomes leader.
LeaderEventuallyElected ==
    <> (\E i \in Server : state[i] = Leader)

\* The cluster EVENTUALLY commits truth — it does not stall. Some entry reaches a
\* committed commitIndex on some replica.
ProgressToCommit ==
    <> (\E i \in Server : commitIndex[i] > 0)

\* Once a leader holds a client entry, that entry is EVENTUALLY committed (quorum
\* commit is reached). Leads-to: the strong progress statement — proposed work is
\* not merely acceptable to commit, it actually gets committed.
EntryEventuallyCommitted ==
    (\E i \in Server : state[i] = Leader /\ Len(log[i]) >= 1)
        ~> (\E i \in Server : commitIndex[i] >= 1)

\* Bound the (already finite) reachable space, belt-and-suspenders.
StateConstraint ==
    /\ \A i \in Server : currentTerm[i] <= MaxTerm
    /\ \A i \in Server : Len(log[i]) <= MaxLogLen
    /\ crashes <= MaxCrash

=============================================================================
(* Model-checking notes (see TLC_CLUSTER_LIVENESS_RUN.md for captured runs):  *)
(*  - DURABILITY: ARVES_ClusterLive_MC.cfg — SoleCandidate = Nil (full         *)
(*    election), MaxCrash >= 1, INVARIANT DurableSafetyInv, no fairness, no     *)
(*    symmetry. TLC exhausts the finite space; committed truth survives crashes.*)
(*  - LIVENESS: ARVES_Cluster_Liveness_MC.cfg — SoleCandidate = n1 (partial    *)
(*    synchrony), MaxCrash = 0, SPECIFICATION FairSpec, PROPERTY               *)
(*    {LeaderEventuallyElected, ProgressToCommit, EntryEventuallyCommitted}.    *)
(*    NO SYMMETRY (the TLC_RUN.md lesson: symmetry during liveness is unsound). *)
(*  - Falsifiability of the liveness claim is intrinsic: set SoleCandidate =   *)
(*    Nil in the liveness cfg and TLC finds a fair stuttering / split-vote      *)
(*    counterexample — Raft is NOT live under full dueling. That is the honest  *)
(*    reason the partial-synchrony assumption is declared, not assumed away.    *)
=============================================================================

---------------------------- MODULE ARVES_Cluster ----------------------------
(*****************************************************************************)
(* ARVES CLUSTER Formal Specification (TLA+) - VERIFICATION EVIDENCE.         *)
(*                                                                           *)
(* Status : EVIDENCE under verification/ (LIVING). NOT normative spec; edits *)
(*          no frozen .docx (ED-001). It renders the I2 distributed          *)
(*          commit/leader protocol in a machine-checkable form so a          *)
(*          certifier can reproduce the SAFETY argument.                     *)
(*                                                                           *)
(* Scope  : N-node per-shard Raft (the I2 substrate). ONE shard group        *)
(*          (IDR-001: one independent group per shard; groups are            *)
(*          independent, so a single group is the safety unit). Models       *)
(*          per-TERM leader election with the up-to-date vote restriction,   *)
(*          log replication with the log-matching consistency check +        *)
(*          follower truncation, and commit-on-quorum under the current-term *)
(*          rule. Bounded, finite instance (small terms/log). This is the    *)
(*          distributed successor the single-shard ARVES_Kernel.tla called   *)
(*          out as "a later, larger ARVES_Consensus.tla".                     *)
(*                                                                           *)
(* Grounds: arves-consensus/src/raft.rs (the deterministic per-shard Raft    *)
(*          core) and arves-kernel/src/cluster.rs (the ClusterKernel that     *)
(*          acks a caller only after quorum commit and applies committed      *)
(*          entries in log order on every replica -> byte-identical follower  *)
(*          truth). A modeled decision here corresponds to a Raft decision    *)
(*          there; the invariants are the safety guarantees the ClusterKernel *)
(*          relies on.                                                        *)
(*                                                                           *)
(* Maps to: IDR-001 (per-shard group, CP), IDR-002 (committed OUTCOMEs, not  *)
(*          invocations - here entries are opaque), IDR-004 (per-shard        *)
(*          election; stale leaders die by term), IDR-005 (append-only log),  *)
(*          ORCH-003 (deterministic replay: committed prefixes are identical  *)
(*          across replicas). SHARD-001: single-group scope, no cross-shard   *)
(*          bytes (the runtime asserts this in apply_all).                     *)
(*                                                                           *)
(* HONEST SCOPE - what this model IS and IS NOT:                              *)
(*  - It is a model-check of the PROTOCOL, not of the Rust code. It abstracts *)
(*    the runtime into a small state machine; a bug in raft.rs that is not a  *)
(*    protocol-level design flaw is NOT caught here (the Rust test suite +    *)
(*    the deterministic sim harness are that evidence).                       *)
(*  - Message passing is folded into atomic Vote / Replicate actions (a       *)
(*    standard, sound reduction for Raft SAFETY: the vote up-to-date check    *)
(*    and the log-matching check are evaluated on the receiver's state at the *)
(*    instant of the step, and every interleaving is explored). Commit reads  *)
(*    the replicated logs directly ("god's-eye" quorum), the usual Raft-      *)
(*    safety abstraction of match-index bookkeeping - it decides nothing that *)
(*    the AppendEntries responses would decide differently.                   *)
(*  - NO election no-op entry is appended on becoming leader: this MATCHES    *)
(*    ARVES (RCR-019 DR-2 - the frozen EntryKind carries only Outcome|        *)
(*    Membership). Raft Sec.5.4.2 is preserved instead by the current-term    *)
(*    commit guard in AdvanceCommit.                                          *)
(*  - Membership change (joint consensus, IDR-003 / RCR-020) is OUT OF SCOPE  *)
(*    of THIS model (fixed voter set); it is a separate, larger obligation.   *)
(*    Liveness is NOT modeled here (safety-only spec); the lesson from        *)
(*    TLC_RUN.md is that symmetry + liveness is unsound - we avoid that by     *)
(*    checking safety only, with no fairness and no symmetry.                  *)
(*****************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    Server,      \* Finite set of replicas, e.g. {n1, n2, n3}.
    Nil,         \* "voted for nobody" sentinel (a value NOT in Server).
    MaxTerm,     \* Bound on election terms (finite instance).
    MaxLogLen    \* Bound on log length (finite instance).

\* Roles (IDR-004). Follower/Candidate/Leader, exactly as Role in raft.rs.
Follower  == "Follower"
Candidate == "Candidate"
Leader    == "Leader"

\* A quorum is any strict majority (per-shard Raft, IDR-001). Any two quorums
\* intersect - the property the whole safety argument rests on.
Quorum == { Q \in SUBSET Server : Cardinality(Q) * 2 > Cardinality(Server) }

Min(a, b) == IF a < b THEN a ELSE b
Max(a, b) == IF a > b THEN a ELSE b

(*---------------------------------------------------------------------------*)
(* State                                                                     *)
(*                                                                           *)
(* An entry is a record [term |-> t]. The "command/value" is abstracted to   *)
(* its creating term (the standard Raft-safety abstraction): two entries with *)
(* the same (index, term) are the SAME entry by the Log Matching Property, so *)
(* the term stamp is a sufficient identity for the safety invariants. The     *)
(* runtime's opaque Outcome bytes ride at exactly this slot (ORCH-001).       *)
(*---------------------------------------------------------------------------*)
VARIABLES
    state,          \* [Server -> {Follower, Candidate, Leader}]
    currentTerm,    \* [Server -> 0..MaxTerm]
    votedFor,       \* [Server -> Server \cup {Nil}]  (per-term single vote)
    votesGranted,   \* [Server -> SUBSET Server]  (a candidate's collected votes)
    log,            \* [Server -> Seq([term |-> 0..MaxTerm])]  append-only (IDR-005)
    commitIndex,    \* [Server -> Nat]  highest index known committed at this node
    committed       \* history: SUBSET [idx, term, cterm] - every committed fact,
                    \* cterm = the committing leader's term (for Leader Completeness)

vars == << state, currentTerm, votedFor, votesGranted, log, commitIndex, committed >>

(*---------------------------------------------------------------------------*)
(* Helpers                                                                   *)
(*---------------------------------------------------------------------------*)

\* Term of the last log entry (0 = empty log). raft.rs: last_log_term().
LastTerm(l) == IF Len(l) = 0 THEN 0 ELSE l[Len(l)].term

\* Sec.5.4.1 up-to-date check: candidate cand's log is at least as up-to-date as
\* voter v's. This is the mechanism behind Leader Completeness. raft.rs step(),
\* the RequestVote arm.
LogUpToDate(cand, v) ==
    \/ LastTerm(log[cand]) > LastTerm(log[v])
    \/ /\ LastTerm(log[cand]) = LastTerm(log[v])
       /\ Len(log[cand]) >= Len(log[v])

\* AppendEntries consistency check (Log Matching maintenance). At follower length
\* k = prevLogIndex, does the follower's entry at k agree with the leader's?
\* (k = 0 is the empty-log base case - always matches.) IF-THEN-ELSE so the index
\* is applied ONLY when k is in domain.
PrevMatch(f, ldr, k) ==
    IF k = 0 THEN TRUE ELSE log[f][k].term = log[ldr][k].term

\* Does the follower's entry at k CONFLICT with the leader's (or run past the
\* leader's log)? Drives the nextIndex-backtracking truncation. Requires k > 0.
Conflict(f, ldr, k) ==
    IF k > Len(log[ldr]) THEN TRUE ELSE log[f][k].term # log[ldr][k].term

TypeOK ==
    /\ state \in [Server -> {Follower, Candidate, Leader}]
    /\ currentTerm \in [Server -> 0..MaxTerm]
    /\ votedFor \in [Server -> (Server \cup {Nil})]
    /\ votesGranted \in [Server -> SUBSET Server]
    /\ \A i \in Server : \A n \in DOMAIN log[i] : log[i][n].term \in 0..MaxTerm
    /\ \A i \in Server : commitIndex[i] \in 0..Len(log[i])

Init ==
    /\ state = [i \in Server |-> Follower]
    /\ currentTerm = [i \in Server |-> 0]
    /\ votedFor = [i \in Server |-> Nil]
    /\ votesGranted = [i \in Server |-> {}]
    /\ log = [i \in Server |-> << >>]
    /\ commitIndex = [i \in Server |-> 0]
    /\ committed = {}

(*---------------------------------------------------------------------------*)
(* Actions                                                                   *)
(*---------------------------------------------------------------------------*)

\* Timeout(i): a follower/candidate times out and starts an election at the next
\* term (raft.rs start_election: term++, vote self, become Candidate). Bounded by
\* MaxTerm so the instance is finite. The randomized-but-seeded election timeout
\* of the runtime is abstracted to pure nondeterminism (any node may time out).
Timeout(i) ==
    /\ state[i] \in {Follower, Candidate}
    /\ currentTerm[i] < MaxTerm
    /\ currentTerm' = [currentTerm EXCEPT ![i] = currentTerm[i] + 1]
    /\ state' = [state EXCEPT ![i] = Candidate]
    /\ votedFor' = [votedFor EXCEPT ![i] = i]
    /\ votesGranted' = [votesGranted EXCEPT ![i] = {i}]
    /\ UNCHANGED << log, commitIndex, committed >>

\* Vote(cand, v): voter v grants its vote to candidate cand. Folds RequestVote +
\* the reply. v adopts a strictly higher candidate term (raft.rs: "higher term
\* always wins", become_follower). At an equal term v grants only if it has not
\* already voted for someone else this term (per-term single vote - the core of
\* Election Safety). The Sec.5.4.1 up-to-date restriction gates every grant.
Vote(cand, v) ==
    /\ cand # v
    /\ state[cand] = Candidate
    /\ currentTerm[cand] >= currentTerm[v]
    /\ LogUpToDate(cand, v)
    /\ \/ currentTerm[cand] > currentTerm[v]                    \* fresh term: free to vote
       \/ /\ currentTerm[cand] = currentTerm[v]
          /\ votedFor[v] \in {Nil, cand}                        \* not yet voted for another
    /\ currentTerm' = [currentTerm EXCEPT ![v] = currentTerm[cand]]
    /\ state' = [state EXCEPT ![v] = Follower]                  \* granting => not a leader/cand
    /\ votedFor' = [votedFor EXCEPT ![v] = cand]
    /\ votesGranted' = [votesGranted EXCEPT ![cand] = votesGranted[cand] \cup {v}]
    /\ UNCHANGED << log, commitIndex, committed >>

\* BecomeLeader(i): a candidate that has collected a quorum of votes becomes
\* leader for its term (raft.rs become_leader). No no-op entry is appended
\* (RCR-019 DR-2) - Sec.5.4.2 is enforced by AdvanceCommit's current-term guard.
BecomeLeader(i) ==
    /\ state[i] = Candidate
    /\ votesGranted[i] \in Quorum
    /\ state' = [state EXCEPT ![i] = Leader]
    /\ UNCHANGED << currentTerm, votedFor, votesGranted, log, commitIndex, committed >>

\* ClientRequest(i): the leader appends one already-decided entry (the ARVES
\* ClusterKernel proposes an OUTCOME - IDR-002 - opaque here). Append-only,
\* leader-only (OWN-001: followers never write). Bounded by MaxLogLen.
ClientRequest(i) ==
    /\ state[i] = Leader
    /\ Len(log[i]) < MaxLogLen
    /\ log' = [log EXCEPT ![i] = Append(log[i], [term |-> currentTerm[i]])]
    /\ UNCHANGED << state, currentTerm, votedFor, votesGranted, commitIndex, committed >>

\* Replicate(ldr, f): leader ldr pushes ONE entry of progress to follower f, the
\* atomic fold of a single-entry AppendEntries + its handling (raft.rs step(),
\* the AppendEntries arm). f adopts a >= leader term (a stale leader, term <, is
\* refused - IDR-004). Two mutually exclusive outcomes:
\*   (a) APPEND the next entry when the preceding index matches (Log Matching),
\*       then advance follower commit to min(leaderCommit, new last index);
\*   (b) TRUNCATE the follower's conflicting tail entry (nextIndex backtracking).
\* A committed prefix is NEVER truncated (guard f > commitIndex[f]); this is the
\* Raft guarantee that the consistency check only ever conflicts above commit.
Replicate(ldr, f) ==
    /\ ldr # f
    /\ state[ldr] = Leader
    /\ currentTerm[ldr] >= currentTerm[f]
    /\ LET k == Len(log[f])   \* follower's current length = prevLogIndex we match at
       IN \/ \* (a) APPEND log[ldr][k+1]
             /\ k < Len(log[ldr])
             /\ PrevMatch(f, ldr, k)                              \* prevLogTerm matches
             /\ log' = [log EXCEPT ![f] = Append(log[f], log[ldr][k+1])]
             /\ commitIndex' = [commitIndex EXCEPT
                    ![f] = Max(commitIndex[f], Min(commitIndex[ldr], k + 1))]
          \/ \* (b) TRUNCATE the conflicting tail (never a committed entry)
             /\ k > 0
             /\ k > commitIndex[f]
             /\ Conflict(f, ldr, k)
             /\ log' = [log EXCEPT ![f] = SubSeq(log[f], 1, k - 1)]
             /\ UNCHANGED commitIndex
    /\ currentTerm' = [currentTerm EXCEPT ![f] = currentTerm[ldr]]
    /\ state' = [state EXCEPT ![f] = Follower]
    /\ votedFor' = [votedFor EXCEPT
           ![f] = IF currentTerm[ldr] > currentTerm[f] THEN Nil ELSE votedFor[f]]
    /\ UNCHANGED << votesGranted, committed >>

\* AdvanceCommit(i): the leader advances its commit index to some n that is
\* (1) from the CURRENT term (Sec.5.4.2 - never count a prior-term entry, which
\* is exactly why no election no-op is needed), and (2) stored on a quorum
\* (commit-on-quorum, IDR-001 CP: no quorum, no truth). Reading the replicated
\* logs directly is the standard match-index abstraction. Every index in the
\* newly committed range is recorded in the `committed` history with cterm = the
\* committing leader's term (the witness for Leader Completeness).
AdvanceCommit(i) ==
    /\ state[i] = Leader
    /\ \E n \in (commitIndex[i] + 1)..Len(log[i]) :
        /\ log[i][n].term = currentTerm[i]
        /\ { k \in Server : Len(log[k]) >= n /\ log[k][n].term = log[i][n].term } \in Quorum
        /\ commitIndex' = [commitIndex EXCEPT ![i] = n]
        /\ committed' = committed \cup
               { [ idx |-> m, term |-> log[i][m].term, cterm |-> currentTerm[i] ]
                 : m \in (commitIndex[i] + 1)..n }
    /\ UNCHANGED << state, currentTerm, votedFor, votesGranted, log >>

Next ==
    \/ \E i \in Server : Timeout(i)
    \/ \E cand \in Server, v \in Server : Vote(cand, v)
    \/ \E i \in Server : BecomeLeader(i)
    \/ \E i \in Server : ClientRequest(i)
    \/ \E ldr \in Server, f \in Server : Replicate(ldr, f)
    \/ \E i \in Server : AdvanceCommit(i)

\* Safety-only spec (no fairness): we check state/step invariants, not liveness.
\* This deliberately avoids the symmetry+liveness unsoundness recorded in
\* TLC_RUN.md - here there is no liveness property at all.
Spec == Init /\ [][Next]_vars

(*===========================================================================*)
(* SAFETY INVARIANTS                                                          *)
(*===========================================================================*)

\* -- Election Safety : at most one leader per term. -----------------------
\* (Raft Figure 3; IDR-004.) Guaranteed by per-term single vote + majority.
ElectionSafety ==
    \A i, j \in Server :
        (state[i] = Leader /\ state[j] = Leader /\ currentTerm[i] = currentTerm[j])
            => i = j

\* -- Log Matching : if two logs share (index, term), they share the whole ---
\* preceding prefix. (Raft Figure 3.) Maintained by the prevLogTerm check +
\* truncate-then-append in Replicate.
LogMatching ==
    \A i, j \in Server :
        \A n \in (DOMAIN log[i]) \cap (DOMAIN log[j]) :
            (log[i][n].term = log[j][n].term)
                => (\A m \in 1..n : log[i][m] = log[j][m])

\* -- State Machine Safety : no two DIFFERENT entries are ever committed at ---
\* the same index. (Raft Figure 3.) This is "committed truth never diverges" -
\* the ClusterKernel guarantee that every replica applies the identical entry
\* at every CommitIndex (arves-kernel/src/cluster.rs apply_all).
StateMachineSafety ==
    \A e1, e2 \in committed :
        (e1.idx = e2.idx) => (e1.term = e2.term)

\* -- Leader Completeness : any leader of a LATER term contains every entry ---
\* committed in an earlier term. (Raft Figure 3.) The reason a new leader can
\* never erase committed truth (raft.rs Sec.5.4.1 vote restriction).
LeaderCompleteness ==
    \A e \in committed :
        \A i \in Server :
            (state[i] = Leader /\ currentTerm[i] > e.cterm)
                => (Len(log[i]) >= e.idx /\ log[i][e.idx].term = e.term)

\* -- Linearizable commit : the committed prefixes of any two replicas AGREE. -
\* Matches the ClusterKernel contract: a caller is acked only after quorum
\* commit, and committed entries apply in log order on EVERY replica, so no two
\* replicas ever hold a different committed entry at the same CommitIndex
\* (byte-identical follower truth; ORCH-003 deterministic replay across nodes).
LinearizableCommit ==
    \A i, j \in Server :
        \A n \in 1..Min(commitIndex[i], commitIndex[j]) :
            log[i][n].term = log[j][n].term

\* The full safety conjunction the model checker enforces.
SafetyInv ==
    /\ TypeOK
    /\ ElectionSafety
    /\ LogMatching
    /\ StateMachineSafety
    /\ LeaderCompleteness
    /\ LinearizableCommit

\* Bound the (already finite) reachable space belt-and-suspenders.
StateConstraint ==
    /\ \A i \in Server : currentTerm[i] <= MaxTerm
    /\ \A i \in Server : Len(log[i]) <= MaxLogLen

=============================================================================
(* Model-checking notes (see TLC_CLUSTER_RUN.md for the captured run):        *)
(*  - Instance ARVES_Cluster_MC.cfg fixes Server = {n1,n2,n3}, MaxTerm and    *)
(*    MaxLogLen small; the reachable space is finite and TLC exhausts it.      *)
(*  - INVARIANT SafetyInv (or the six conjuncts individually for sharper       *)
(*    diagnostics). NO PROPERTY (safety-only) and NO SYMMETRY (see header).    *)
(*  - Abstracted away: real message reordering/duplication (folded into        *)
(*    atomic actions), match-index bookkeeping (commit reads logs directly),   *)
(*    joint-consensus membership change (IDR-003 - fixed voter set here),      *)
(*    crash/recovery durability, snapshot install, and wire format. Those are  *)
(*    the runtime's Rust + sim-harness evidence, not this protocol model.       *)
(*  - Falsifiability (this model HAS teeth; verified, see TLC_CLUSTER_RUN.md):   *)
(*    PROBE 1 - weaken `LogUpToDate(cand,v)` to `TRUE` (drop the Sec.5.4.1        *)
(*    election restriction) and TLC finds a LeaderCompleteness counterexample at  *)
(*    DEPTH 10 (workers 1: 16,739 states): an empty-log node is elected leader at *)
(*    a higher term while another leader commits an entry on a quorum the new     *)
(*    leader's log lacks. PROBE 2 - delete the `log[i][n].term = currentTerm[i]`  *)
(*    current-term commit guard in AdvanceCommit (the Sec.5.4.2 Figure-8 rule     *)
(*    that lets ARVES omit the election no-op, RCR-019 DR-2). HONEST: at this     *)
(*    MaxTerm=3 instance the guard is NON-load-bearing - removing it and running  *)
(*    to FULL exhaustion finds NO violation (7,390,624 states, depth 27); the     *)
(*    Figure-8 trace is simply unreachable at 3 terms. The guard's teeth appear   *)
(*    at MaxTerm=4: removing it yields a captured LeaderCompleteness Figure-8      *)
(*    counterexample, while the guarded model at MaxTerm=4 is exhaustively clean  *)
(*    (14,409,961 states, depth 33, No error). THAT pair earns the RCR-019 DR-2   *)
(*    claim. Revert any probe after confirming. See TLC_CLUSTER_RUN.md.           *)
=============================================================================

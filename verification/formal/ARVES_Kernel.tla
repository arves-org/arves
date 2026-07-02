---------------------------- MODULE ARVES_Kernel ----------------------------
(*****************************************************************************)
(* ARVES Kernel Formal Specification (TLA+) - VERIFICATION EVIDENCE.         *)
(*                                                                           *)
(* Status : EVIDENCE under verification/ . This module is NOT normative      *)
(*          specification and does NOT edit any frozen .docx (ED-001). It    *)
(*          renders the frozen prose invariants in a machine-checkable form  *)
(*          so a certifier can reproduce the safety/liveness argument.       *)
(*                                                                           *)
(* Scope  : ONE shard. A commit gateway (the sole truth-mutating action) in  *)
(*          front of an append-only log and an in-memory truth set, with a   *)
(*          replay reconstruction. Deliberately ABSTRACT: no Raft, no        *)
(*          replication, no network, no leader election, no cross-shard      *)
(*          sagas, no engine/graph. Those are later milestones (I2+) and     *)
(*          are called out honestly in the companion .md.                    *)
(*                                                                           *)
(* Maps to: OWN-001  (single writer / one owner per (shard,content))         *)
(*          ORCH-004 (idempotent, content-addressable commit)                *)
(*          ORCH-003 (replay-equivalence: truth == fold of the log)          *)
(*          Global Readiness R-05 ; P05 findings P05-1/P05-2/P05-4.          *)
(*                                                                           *)
(* Grounds: the reference kernel's commit()/replay()/truth_hash() in         *)
(*          runtime/crates/arves-kernel/src/lib.rs . The abstract "content"  *)
(*          tokens correspond to concrete ACS-001 ContentIds pinned in the   *)
(*          conformance scenario (companion .md, CS-1).                       *)
(*****************************************************************************)
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    Content     \* Finite set of distinct content addresses (ContentId tokens)
                \* offered to the single shard. e.g. {c1, c2}.

\* Symmetry set for TLC: the content tokens are interchangeable (no property
\* distinguishes them by identity), so TLC may quotient the state space by their
\* permutations. Referenced as SYMMETRY Perms in ARVES_Kernel_MC.cfg.
Perms == Permutations(Content)

(*---------------------------------------------------------------------------*)
(* State                                                                     *)
(*                                                                           *)
(* log    : the append-only committed log (Raft-log-as-WAL-as-decision-trace *)
(*          in the real system, IDR-003/005). A sequence of content tokens;  *)
(*          position i (1-based) is CommitIndex i-1 in the runtime.           *)
(* truth  : the in-memory truth set the Kernel owns (ORCH-001/OWN-001).       *)
(*          Modeled as the SET of committed content tokens. The order-        *)
(*          sensitive digest that the runtime's truth_hash() folds is         *)
(*          captured abstractly by requiring truth == Range(log) (ORCH-003b), *)
(*          i.e. replaying the log reconstructs exactly the truth set.        *)
(* pc     : per-content control state, to model "a proposal is offered, then  *)
(*          eventually committed". Values:                                     *)
(*            "open"      - not yet proposed                                   *)
(*            "proposed"  - offered to the gateway, awaiting commit            *)
(*            "committed" - reflected in truth                                 *)
(*---------------------------------------------------------------------------*)
VARIABLES
    log,        \* Seq(Content)  : append-only committed log
    truth,      \* SUBSET Content: in-memory committed truth set
    pc          \* [Content -> {"open","proposed","committed"}]

vars == << log, truth, pc >>

(*---------------------------------------------------------------------------*)
(* Helpers                                                                   *)
(*---------------------------------------------------------------------------*)

\* Range of a sequence: the set of its elements. This is the ABSTRACT replay
\* fold - the truth reconstructed from the log is exactly its element set.
Range(s) == { s[i] : i \in DOMAIN s }

\* Multiplicity of x in sequence s (how many times x appears).
Count(s, x) == Cardinality({ i \in DOMAIN s : s[i] = x })

TypeOK ==
    /\ log \in Seq(Content)
    /\ truth \subseteq Content
    /\ pc \in [Content -> {"open", "proposed", "committed"}]

Init ==
    /\ log = << >>
    /\ truth = {}
    /\ pc = [c \in Content |-> "open"]

(*---------------------------------------------------------------------------*)
(* Actions                                                                   *)
(*---------------------------------------------------------------------------*)

\* Propose(c): an external producer offers content c to the gateway. This is
\* NOT a truth mutation (ORCH-001: producers own no truth). It only arms the
\* content for a subsequent commit. Idempotent at the propose stage: re-
\* proposing an open item just (re)marks it proposed.
Propose(c) ==
    /\ pc[c] = "open"
    /\ pc' = [pc EXCEPT ![c] = "proposed"]
    /\ UNCHANGED << log, truth >>

\* Commit(c): the SOLE truth-mutating action (the single write path; OWN-001,
\* ORCH-001, G-001-proposed). Models arves-kernel commit():
\*   ORCH-004 idempotency: if c is already committed (present in truth), the
\*   re-proposal resolves to existing truth and appends NOTHING to the log.
\*   First commit: append c to the log AND add c to truth (kept in lockstep,
\*   which is exactly what the runtime does: WAL.append then truth.push).
Commit(c) ==
    /\ pc[c] = "proposed"
    /\ IF c \in truth
         THEN \* ORCH-004: idempotent no-op. No second log record; same truth.
              /\ pc' = [pc EXCEPT ![c] = "committed"]
              /\ UNCHANGED << log, truth >>
         ELSE \* First commit: one append, one truth insertion, in lockstep.
              /\ log' = Append(log, c)
              /\ truth' = truth \cup {c}
              /\ pc' = [pc EXCEPT ![c] = "committed"]

Next ==
    \E c \in Content : Propose(c) \/ Commit(c)

\* Weak fairness on Commit: any content that stays enabled to commit is
\* eventually committed. This is what makes the liveness property meaningful.
Fairness == \A c \in Content : WF_vars(Commit(c))

Spec == Init /\ [][Next]_vars /\ Fairness

(*===========================================================================*)
(* SAFETY INVARIANTS                                                          *)
(*===========================================================================*)

\* -- OWN-001 : single writer / one owner per (shard,content). -------------
\* Within the one shard, each committed content appears in the log AT MOST
\* once. There is no second write path that could fork or duplicate truth.
OWN_001 ==
    \A c \in Content : Count(log, c) <= 1

\* -- ORCH-004 : idempotent, content-addressable commit. -------------------
\* (a) At most one truth per (shard,content): truth is a set keyed by content,
\*     so a content is present 0 or 1 times - never twice.
\* (b) Log-truth lockstep: the set of log entries equals the truth set, so a
\*     re-proposal that finds c already in truth adds no log record (the log
\*     never grows without truth growing and vice-versa).
\* (a) is structural (truth : SUBSET Content). (b) is the checkable half:
ORCH_004 ==
    /\ Range(log) = truth
    /\ \A c \in Content : Count(log, c) <= 1   \* no duplicate content record

\* -- ORCH-003 (b) : replay-equivalence. -----------------------------------
\* Truth reconstructed FROM the log equals the committed truth. In the
\* abstract model, replay is the fold Range(log); replay-equivalence is the
\* invariant that this fold always equals the live truth set. In the runtime
\* this is truth_hash()-before == truth_hash()-after-replay.
ReplayOfLog == Range(log)

ORCH_003_ReplayEquiv ==
    ReplayOfLog = truth

\* Convenience: the full safety conjunction the model checker enforces.
SafetyInv ==
    /\ TypeOK
    /\ OWN_001
    /\ ORCH_004
    /\ ORCH_003_ReplayEquiv

(*===========================================================================*)
(* LIVENESS                                                                   *)
(*===========================================================================*)

\* A committed proposal is eventually reflected in truth. Under weak fairness
\* on Commit, once a content is proposed it eventually becomes committed and
\* thus a member of truth. (Leads-to; requires the Fairness conjunct in Spec.)
EventuallyCommitted ==
    \A c \in Content : (pc[c] = "proposed") ~> (c \in truth)

=============================================================================
(* Model-checking notes (see companion .md for the exact TLC/Apalache run):  *)
(*  - Instance ARVES_Kernel_MC.cfg fixes Content = {c1, c2} (2 tokens), a     *)
(*    finite state space TLC exhausts in well under a second.                 *)
(*  - INVARIANTS to check: SafetyInv (or the four separately).                *)
(*  - PROPERTY to check   : EventuallyCommitted (needs Spec's fairness).      *)
(*  - What is abstracted away: distribution, Raft, replication, leader        *)
(*    election, crash/recovery, cross-shard sagas, and the order-sensitive    *)
(*    byte-level truth_hash fold (here truth is a SET; order-equivalence is    *)
(*    pinned instead by the runtime conformance scenario CS-1 in the .md).    *)
=============================================================================

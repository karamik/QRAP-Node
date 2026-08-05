---- MODULE OrbitalBFT ----
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS Nodes,          \* Set of validator nodes
          F,              \* Max Byzantine faults tolerated
          MaxRound,       \* Bound for model checking
          Values          \* Set of possible values to commit

ASSUME Cardinality(Nodes) >= 3 * F + 1

VARIABLES round,          \* round[n] = current round of node n
          leader,         \* leader[r] = leader for round r
          prepared,       \* prepared[n] = set of values node n has prepared
          committed,      \* committed[n] = set of values node n has committed
          msgs            \* msgs = set of all broadcast messages

\* Message types
Message == [sender : Nodes, val : Values, rnd : 0..MaxRound, type : {"PREPARE", "COMMIT"}]

TypeOK ==
  /\ round \in [Nodes -> 0..MaxRound]
  /\ leader \in [0..MaxRound -> Nodes]
  /\ prepared \in [Nodes -> SUBSET [val : Values, rnd : 0..MaxRound]]
  /\ committed \in [Nodes -> SUBSET [val : Values, rnd : 0..MaxRound]]
  /\ msgs \subseteq Message

\* Initial state
Init ==
  /\ round = [n \in Nodes |-> 0]
  /\ leader \in [0..MaxRound -> Nodes]  \* arbitrary leader schedule
  /\ prepared = [n \in Nodes |-> {}]
  /\ committed = [n \in Nodes |-> {}]
  /\ msgs = {}

\* Helper: count PREPARE for (v, r)
PrepareCount(v, r) ==
  Cardinality({m \in msgs : m.type = "PREPARE" /\ m.val = v /\ m.rnd = r})

\* Helper: count COMMIT for (v, r)
CommitCount(v, r) ==
  Cardinality({m \in msgs : m.type = "COMMIT" /\ m.val = v /\ m.rnd = r})

\* Quorum size = 2F + 1
Quorum == 2 * F + 1

\* Leader proposes (broadcasts PREPARE)
LeaderPropose(n) ==
  /\ round[n] <= MaxRound
  /\ leader[round[n]] = n
  /\ \A m \in msgs : ~(m.sender = n /\ m.type = "PREPARE" /\ m.rnd = round[n])
  /\ \E v \in Values :
       LET msg == [sender |-> n, val |-> v, rnd |-> round[n], type |-> "PREPARE"]
       IN msgs' = msgs \union {msg}
  /\ UNCHANGED <<round, leader, prepared, committed>>

\* Node receives PREPARE and adds to prepared set
ReceivePrepare(n) ==
  \E m \in msgs :
    /\ m.type = "PREPARE"
    /\ m.rnd = round[n]
    /\ [val |-> m.val, rnd |-> m.rnd] \notin prepared[n]
    /\ prepared' = [prepared EXCEPT ![n] = @ \union {[val |-> m.val, rnd |-> m.rnd]}]
    /\ UNCHANGED <<round, leader, committed, msgs>>

\* Node broadcasts COMMIT after seeing Quorum PREPAREs
SendCommit(n) ==
  \E v \in Values :
    /\ PrepareCount(v, round[n]) >= Quorum
    /\ [val |-> v, rnd |-> round[n]] \in prepared[n]
    /\ \A m \in msgs : ~(m.sender = n /\ m.type = "COMMIT" /\ m.rnd = round[n])
    /\ LET msg == [sender |-> n, val |-> v, rnd |-> round[n], type |-> "COMMIT"]
       IN msgs' = msgs \union {msg}
    /\ UNCHANGED <<round, leader, prepared, committed>>

\* Node commits after seeing Quorum COMMITs
DoCommit(n) ==
  \E v \in Values :
    /\ CommitCount(v, round[n]) >= Quorum
    /\ [val |-> v, rnd |-> round[n]] \notin committed[n]
    /\ committed' = [committed EXCEPT ![n] = @ \union {[val |-> v, rnd |-> round[n]]}]
    /\ UNCHANGED <<round, leader, prepared, msgs>>

\* View change: move to next round
ViewChange(n) ==
  /\ round[n] < MaxRound
  /\ round' = [round EXCEPT ![n] = @ + 1]
  /\ UNCHANGED <<leader, prepared, committed, msgs>>

\* All possible actions
Next ==
  \/ \E n \in Nodes : LeaderPropose(n)
  \/ \E n \in Nodes : ReceivePrepare(n)
  \/ \E n \in Nodes : SendCommit(n)
  \/ \E n \in Nodes : DoCommit(n)
  \/ \E n \in Nodes : ViewChange(n)

\* Safety: No two different values committed at same round by any honest nodes
Safety ==
  \A n1, n2 \in Nodes :
    \A c1 \in committed[n1], c2 \in committed[n2] :
      c1.rnd = c2.rnd => c1.val = c2.val

\* Liveness: Eventually some value is committed (bounded for model checking)
Liveness ==
  <> \E n \in Nodes : committed[n] # {}

====

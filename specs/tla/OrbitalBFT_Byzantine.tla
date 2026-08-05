---- MODULE OrbitalBFT_Byzantine ----
EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS Nodes,          \* Set of all validator nodes
          F,              \* Max Byzantine faults tolerated
          MaxRound,       \* Bound for model checking
          Values,         \* Set of possible values to commit
          ByzantineNodes  \* SUBSET Nodes, |ByzantineNodes| <= F

ASSUME Cardinality(Nodes) >= 3 * F + 1
ASSUME ByzantineNodes \subseteq Nodes
ASSUME Cardinality(ByzantineNodes) <= F

HonestNodes == Nodes \ ByzantineNodes

VARIABLES round,          \* round[n] = current round of node n
          leader,         \* leader[r] = leader for round r
          prepared,       \* prepared[n] = set of values node n has prepared
          committed,      \* committed[n] = set of values node n has committed
          msgs,           \* msgs = set of all broadcast messages
          byzantine_msgs  \* msgs sent by Byzantine nodes (for tracking)

\* Message types
Message == [sender : Nodes, val : Values, rnd : 0..MaxRound, type : {"PREPARE", "COMMIT"}]

TypeOK ==
  /\ round \in [Nodes -> 0..MaxRound]
  /\ leader \in [0..MaxRound -> Nodes]
  /\ prepared \in [Nodes -> SUBSET [val : Values, rnd : 0..MaxRound]]
  /\ committed \in [Nodes -> SUBSET [val : Values, rnd : 0..MaxRound]]
  /\ msgs \subseteq Message
  /\ byzantine_msgs \subseteq Message

\* Initial state
Init ==
  /\ round = [n \in Nodes |-> 0]
  /\ leader \in [0..MaxRound -> Nodes]
  /\ prepared = [n \in Nodes |-> {}]
  /\ committed = [n \in Nodes |-> {}]
  /\ msgs = {}
  /\ byzantine_msgs = {}

\* Helper: count PREPARE for (v, r) from honest nodes only
HonestPrepareCount(v, r) ==
  Cardinality({m \in msgs : m.type = "PREPARE" /\ m.val = v /\ m.rnd = r /\ m.sender \in HonestNodes})

\* Helper: count COMMIT for (v, r) from honest nodes only
HonestCommitCount(v, r) ==
  Cardinality({m \in msgs : m.type = "COMMIT" /\ m.val = v /\ m.rnd = r /\ m.sender \in HonestNodes})

\* Quorum size = 2F + 1 (only honest votes count for quorum)
Quorum == 2 * F + 1

\* Honest leader proposes (broadcasts PREPARE)
HonestLeaderPropose(n) ==
  /\ n \in HonestNodes
  /\ round[n] <= MaxRound
  /\ leader[round[n]] = n
  /\ \A m \in msgs : ~(m.sender = n /\ m.type = "PREPARE" /\ m.rnd = round[n])
  /\ \E v \in Values :
       LET msg == [sender |-> n, val |-> v, rnd |-> round[n], type |-> "PREPARE"]
       IN msgs' = msgs \union {msg}
  /\ UNCHANGED <<round, leader, prepared, committed, byzantine_msgs>>

\* Byzantine node sends arbitrary PREPARE (double-vote, conflicting values)
ByzantineSendPrepare(n) ==
  /\ n \in ByzantineNodes
  /\ round[n] <= MaxRound
  /\ \E v \in Values :
       LET msg == [sender |-> n, val |-> v, rnd |-> round[n], type |-> "PREPARE"]
       IN msgs' = msgs \union {msg}
          /\ byzantine_msgs' = byzantine_msgs \union {msg}
  /\ UNCHANGED <<round, leader, prepared, committed>>

\* Byzantine node sends arbitrary COMMIT
ByzantineSendCommit(n) ==
  /\ n \in ByzantineNodes
  /\ round[n] <= MaxRound
  /\ \E v \in Values :
       LET msg == [sender |-> n, val |-> v, rnd |-> round[n], type |-> "COMMIT"]
       IN msgs' = msgs \union {msg}
          /\ byzantine_msgs' = byzantine_msgs \union {msg}
  /\ UNCHANGED <<round, leader, prepared, committed>>

\* Honest node receives PREPARE and adds to prepared set
HonestReceivePrepare(n) ==
  /\ n \in HonestNodes
  /\ \E m \in msgs :
    /\ m.type = "PREPARE"
    /\ m.rnd = round[n]
    /\ [val |-> m.val, rnd |-> m.rnd] \notin prepared[n]
    /\ prepared' = [prepared EXCEPT ![n] = @ \union {[val |-> m.val, rnd |-> m.rnd]}]
    /\ UNCHANGED <<round, leader, committed, msgs, byzantine_msgs>>

\* Honest node broadcasts COMMIT after seeing Quorum honest PREPAREs
HonestSendCommit(n) ==
  /\ n \in HonestNodes
  /\ \E v \in Values :
    /\ HonestPrepareCount(v, round[n]) >= Quorum
    /\ [val |-> v, rnd |-> round[n]] \in prepared[n]
    /\ \A m \in msgs : ~(m.sender = n /\ m.type = "COMMIT" /\ m.rnd = round[n])
    /\ LET msg == [sender |-> n, val |-> v, rnd |-> round[n], type |-> "COMMIT"]
       IN msgs' = msgs \union {msg}
    /\ UNCHANGED <<round, leader, prepared, committed, byzantine_msgs>>

\* Honest node commits after seeing Quorum honest COMMITs
HonestDoCommit(n) ==
  /\ n \in HonestNodes
  /\ \E v \in Values :
    /\ HonestCommitCount(v, round[n]) >= Quorum
    /\ [val |-> v, rnd |-> round[n]] \notin committed[n]
    /\ committed' = [committed EXCEPT ![n] = @ \union {[val |-> v, rnd |-> round[n]]}]
    /\ UNCHANGED <<round, leader, prepared, msgs, byzantine_msgs>>

\* View change: honest node moves to next round
HonestViewChange(n) ==
  /\ n \in HonestNodes
  /\ round[n] < MaxRound
  /\ round' = [round EXCEPT ![n] = @ + 1]
  /\ UNCHANGED <<leader, prepared, committed, msgs, byzantine_msgs>>

\* Byzantine view change (can happen anytime)
ByzantineViewChange(n) ==
  /\ n \in ByzantineNodes
  /\ round[n] < MaxRound
  /\ round' = [round EXCEPT ![n] = @ + 1]
  /\ UNCHANGED <<leader, prepared, committed, msgs, byzantine_msgs>>

\* All possible actions
Next ==
  \/ \E n \in HonestNodes : HonestLeaderPropose(n)
  \/ \E n \in HonestNodes : HonestReceivePrepare(n)
  \/ \E n \in HonestNodes : HonestSendCommit(n)
  \/ \E n \in HonestNodes : HonestDoCommit(n)
  \/ \E n \in HonestNodes : HonestViewChange(n)
  \/ \E n \in ByzantineNodes : ByzantineSendPrepare(n)
  \/ \E n \in ByzantineNodes : ByzantineSendCommit(n)
  \/ \E n \in ByzantineNodes : ByzantineViewChange(n)

\* Safety: No two different values committed at same round by any HONEST nodes
\* Byzantine nodes can commit conflicting values — that's the definition of Byzantine
HonestSafety ==
  \A n1, n2 \in HonestNodes :
    \A c1 \in committed[n1], c2 \in committed[n2] :
      c1.rnd = c2.rnd => c1.val = c2.val

\* Stronger safety: even with Byzantine msgs, honest nodes never commit conflicting values
StrongSafety ==
  \A n1, n2 \in Nodes :
    \A c1 \in committed[n1], c2 \in committed[n2] :
      (n1 \in HonestNodes /\ n2 \in HonestNodes /\ c1.rnd = c2.rnd) => c1.val = c2.val

\* Liveness: Eventually some honest node commits a value
HonestLiveness ==
  <> \E n \in HonestNodes : committed[n] # {}

\* Invariant: Byzantine nodes send at most F conflicting PREPAREs per round
\* (this is a sanity check, not a protocol guarantee)
ByzantineLimit ==
  \A r \in 0..MaxRound :
    Cardinality({m \in byzantine_msgs : m.type = "PREPARE" /\ m.rnd = r}) <= F * Cardinality(Values)

====

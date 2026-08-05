---- MODULE Consensus ----
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS Nodes, MaxRound, F

VARIABLES round, leader, committed, msgs

(* Type invariant *)
TypeOK ==
  /\ round \in [Nodes -> 0..MaxRound]
  /\ leader \in [Nodes -> Nodes]
  /\ committed \in [Nodes -> SUBSET [val : Nat, rnd : Nat]]
  /\ msgs \subseteq [sender : Nodes, val : Nat, rnd : Nat, type : {"PREPARE", "COMMIT"}]

(* Initial state *)
Init ==
  /\ round = [n \in Nodes |-> 0]
  /\ leader \in [Nodes -> Nodes]
  /\ committed = [n \in Nodes |-> {}]
  /\ msgs = {}

(* Safety: no two different values at same round *)
Safety == \A n1, n2 \in Nodes :
  \A c1 \in committed[n1], c2 \in committed[n2] :
    c1.rnd = c2.rnd => c1.val = c2.val
====

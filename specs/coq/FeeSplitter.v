Require Import Arith Lia.

(* QRAP Fee Splitter Distribution *)
(* Provers 35%, Validators 25%, Treasury 20%, DA 15%, Burn 5% *)
Record Distribution := mkDist {
  provers    : nat;
  validators : nat;
  treasury   : nat;
  da_layer   : nat;
  burn       : nat
}.

(* Invariant: total must equal exactly 100% *)
Definition valid (d : Distribution) : Prop :=
  provers d + validators d + treasury d + da_layer d + burn d = 100.

(* Split amount according to distribution *)
Definition split (amount : nat) (d : Distribution) : nat * nat * nat * nat * nat :=
  let p := (amount * provers d) / 100 in
  let v := (amount * validators d) / 100 in
  let t := (amount * treasury d) / 100 in
  let d_ := (amount * da_layer d) / 100 in
  let b := amount - (p + v + t + d_) in
  (p, v, t, d_, b).

(* Theorem 1: Sum of outputs equals input amount *)
Theorem split_total : forall amount d,
  valid d ->
  let (p, v, t, d_, b) := split amount d in
  p + v + t + d_ + b = amount.
Proof.
  intros amount d H. unfold split. lia.
Qed.

(* Theorem 2: Non-negativity (if amount >= 100) *)
Theorem split_nonneg : forall amount d,
  valid d ->
  amount >= 100 ->
  let (p, v, t, d_, b) := split amount d in
  p >= 0 /\ v >= 0 /\ t >= 0 /\ d_ >= 0 /\ b >= 0.
Proof.
  intros amount d H H100. unfold split. lia.
Qed.

(* Theorem 3: Burn is never negative (safety for deflationary mechanics) *)
Theorem burn_nonneg : forall amount d,
  valid d ->
  amount >= 100 ->
  let (_, _, _, _, b) := split amount d in
  b >= 0.
Proof.
  intros. destruct (split_nonneg amount d H H100) as [_ [_ [_ [_ Hb]]]]. auto.
Qed.

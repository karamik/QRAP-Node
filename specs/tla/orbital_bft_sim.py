#!/usr/bin/env python3
"""
Orbital BFT Consensus — Python Simulator (Termux fallback)
Checks Safety and Liveness via randomized + deterministic execution traces.
"""

import random
from dataclasses import dataclass, field
from typing import Set, List

@dataclass
class Message:
    sender: int
    val: str
    rnd: int
    type: str  # "PREPARE" or "COMMIT"

@dataclass
class Node:
    id: int
    round: int = 0
    prepared: Set[tuple] = field(default_factory=set)
    committed: Set[tuple] = field(default_factory=set)

class OrbitalBFT:
    def __init__(self, n_nodes=4, f=1, max_round=5, values=None):
        assert n_nodes >= 3 * f + 1, "Need n >= 3f + 1"
        self.n_nodes = n_nodes
        self.f = f
        self.quorum = 2 * f + 1
        self.max_round = max_round
        self.values = values or ["A", "B", "C"]
        self.nodes = [Node(i) for i in range(n_nodes)]
        self.msgs: List[Message] = []
        self.leader = {r: r % n_nodes for r in range(max_round + 1)}
        self.safety_violations = 0
        self.liveness_achieved = False

    def prepare_senders(self, val, rnd):
        return set(m.sender for m in self.msgs if m.type == "PREPARE" and m.val == val and m.rnd == rnd)

    def commit_senders(self, val, rnd):
        return set(m.sender for m in self.msgs if m.type == "COMMIT" and m.val == val and m.rnd == rnd)

    def leader_propose(self, n):
        r = self.nodes[n].round
        if r > self.max_round:
            return False
        if self.leader[r] != n:
            return False
        if any(m.sender == n and m.type == "PREPARE" and m.rnd == r for m in self.msgs):
            return False
        v = random.choice(self.values)
        self.msgs.append(Message(n, v, r, "PREPARE"))
        return True

    def receive_prepare(self, n):
        r = self.nodes[n].round
        candidates = [m for m in self.msgs if m.type == "PREPARE" and m.rnd == r and (m.val, m.rnd) not in self.nodes[n].prepared]
        if not candidates:
            return False
        m = random.choice(candidates)
        self.nodes[n].prepared.add((m.val, m.rnd))
        # Echo broadcast: node retransmits PREPARE so others see quorum
        if not any(m2.sender == n and m2.type == "PREPARE" and m2.rnd == r for m2 in self.msgs):
            self.msgs.append(Message(n, m.val, r, "PREPARE"))
        return True

    def send_commit(self, n):
        r = self.nodes[n].round
        for v in self.values:
            if len(self.prepare_senders(v, r)) >= self.quorum and (v, r) in self.nodes[n].prepared:
                if not any(m.sender == n and m.type == "COMMIT" and m.rnd == r for m in self.msgs):
                    self.msgs.append(Message(n, v, r, "COMMIT"))
                    return True
        return False

    def do_commit(self, n):
        r = self.nodes[n].round
        for v in self.values:
            if len(self.commit_senders(v, r)) >= self.quorum and (v, r) not in self.nodes[n].committed:
                self.nodes[n].committed.add((v, r))
                self.liveness_achieved = True
                return True
        return False

    def view_change(self, n):
        if self.nodes[n].round < self.max_round:
            self.nodes[n].round += 1
            return True
        return False

    def check_safety(self):
        commits = {}
        for n in self.nodes:
            for val, rnd in n.committed:
                if rnd not in commits:
                    commits[rnd] = set()
                commits[rnd].add(val)
        for rnd, vals in commits.items():
            if len(vals) > 1:
                self.safety_violations += 1
                return False
        return True

    def run_trace(self, steps=100):
        actions = [
            self.leader_propose,
            self.receive_prepare,
            self.send_commit,
            self.do_commit,
            self.view_change,
        ]
        for step in range(steps):
            n = random.randint(0, self.n_nodes - 1)
            action = random.choice(actions)
            action(n)
            if not self.check_safety():
                print(f"❌ SAFETY VIOLATION at step {step}!")
                return False
        return True

    def report(self):
        total_commits = sum(len(n.committed) for n in self.nodes)
        print(f"Nodes: {self.n_nodes}, F: {self.f}, Quorum: {self.quorum}")
        print(f"Messages: {len(self.msgs)}")
        print(f"Total commits: {total_commits}")
        print(f"Safety violations: {self.safety_violations}")
        print(f"Liveness achieved: {self.liveness_achieved}")
        print(f"Safety: {'✅ PASS' if self.safety_violations == 0 else '❌ FAIL'}")
        print(f"Liveness: {'✅ PASS' if self.liveness_achieved else '⚠️  No commit in trace'}")

def main():
    print("=" * 50)
    print("Orbital BFT — Randomized Simulation")
    print("=" * 50)

    # Test 1: Normal case
    print("\n--- Test 1: 4 nodes, f=1, 200 steps ---")
    sim = OrbitalBFT(n_nodes=4, f=1, max_round=5)
    sim.run_trace(steps=200)
    sim.report()

    # Test 2: Larger network
    print("\n--- Test 2: 7 nodes, f=2, 300 steps ---")
    sim2 = OrbitalBFT(n_nodes=7, f=2, max_round=5)
    sim2.run_trace(steps=300)
    sim2.report()

    # Test 3: Stress test
    print("\n--- Test 3: Stress — 100 random traces ---")
    violations = 0
    liveness_count = 0
    for i in range(100):
        sim = OrbitalBFT(n_nodes=4, f=1, max_round=5)
        sim.run_trace(steps=200)
        if sim.safety_violations > 0:
            violations += 1
        if sim.liveness_achieved:
            liveness_count += 1
    print(f"Traces: 100 | Safety violations: {violations} | Liveness: {liveness_count}/100")
    print(f"Overall Safety: {'✅ PASS' if violations == 0 else '❌ FAIL'}")

    # ─── Happy Path Test — Deterministic Commit ───
    print("\n" + "=" * 50)
    print("Happy Path Test — Deterministic Commit")
    print("=" * 50)
    
    sim = OrbitalBFT(n_nodes=4, f=1, max_round=0)
    sim.values = ["BLOCK_42"]
    
    # 1. Leader proposes
    sim.leader_propose(0)
    print(f"Step 1: Leader (node 0) proposes BLOCK_42")
    
    # 2. All nodes receive PREPARE (and echo)
    for n in range(4):
        sim.receive_prepare(n)
    print(f"Step 2: All nodes received & echoed PREPARE")
    print(f"  PREPARE senders: {sim.prepare_senders('BLOCK_42', 0)}")
    
    # 3. All nodes send COMMIT
    for n in range(4):
        sim.send_commit(n)
    print(f"Step 3: Nodes send COMMIT")
    print(f"  COMMIT senders: {sim.commit_senders('BLOCK_42', 0)}")
    
    # 4. All nodes commit
    for n in range(4):
        sim.do_commit(n)
    print(f"Step 4: Nodes commit")
    
    sim.check_safety()
    sim.report()
    assert sim.liveness_achieved, "Happy path must achieve liveness!"
    print("✅ Happy path: LIVENESS CONFIRMED")

if __name__ == "__main__":
    main()

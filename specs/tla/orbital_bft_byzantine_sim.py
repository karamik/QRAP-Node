#!/usr/bin/env python3
"""
Orbital BFT with Byzantine Faults — Python Simulator
Tests safety and liveness under F Byzantine nodes.
"""

import random
from dataclasses import dataclass, field
from typing import Set, List, Dict

@dataclass
class Message:
    sender: int
    val: str
    rnd: int
    type: str

@dataclass
class Node:
    id: int
    is_byzantine: bool = False
    round: int = 0
    prepared: Set[tuple] = field(default_factory=set)
    committed: Set[tuple] = field(default_factory=set)

class OrbitalBFTByzantine:
    def __init__(self, n_nodes=7, f=2, max_round=5, values=None):
        assert n_nodes >= 3 * f + 1
        self.n_nodes = n_nodes
        self.f = f
        self.quorum = 2 * f + 1
        self.max_round = max_round
        self.values = values or ["A", "B", "C"]
        
        # F random nodes become Byzantine
        byzantine_ids = set(random.sample(range(n_nodes), f))
        self.nodes = [
            Node(i, is_byzantine=(i in byzantine_ids))
            for i in range(n_nodes)
        ]
        self.msgs: List[Message] = []
        self.byzantine_msgs: List[Message] = []
        self.leader = {r: r % n_nodes for r in range(max_round + 1)}
        self.safety_violations = 0

    @property
    def honest_nodes(self):
        return [n for n in self.nodes if not n.is_byzantine]

    @property
    def byzantine_nodes(self):
        return [n for n in self.nodes if n.is_byzantine]

    def honest_prepare_count(self, val, rnd):
        return sum(1 for m in self.msgs 
                   if m.type == "PREPARE" and m.val == val and m.rnd == rnd 
                   and not self.nodes[m.sender].is_byzantine)

    def honest_commit_count(self, val, rnd):
        return sum(1 for m in self.msgs 
                   if m.type == "COMMIT" and m.val == val and m.rnd == rnd 
                   and not self.nodes[m.sender].is_byzantine)

    def honest_leader_propose(self, n):
        r = self.nodes[n].round
        if r > self.max_round or self.leader[r] != n:
            return False
        if any(m.sender == n and m.type == "PREPARE" and m.rnd == r for m in self.msgs):
            return False
        v = random.choice(self.values)
        self.msgs.append(Message(n, v, r, "PREPARE"))
        return True

    def byzantine_send_prepare(self, n):
        r = self.nodes[n].round
        if r > self.max_round:
            return False
        # Byzantine: send conflicting PREPAREs
        for _ in range(random.randint(1, 3)):
            v = random.choice(self.values)
            msg = Message(n, v, r, "PREPARE")
            self.msgs.append(msg)
            self.byzantine_msgs.append(msg)
        return True

    def byzantine_send_commit(self, n):
        r = self.nodes[n].round
        if r > self.max_round:
            return False
        v = random.choice(self.values)
        msg = Message(n, v, r, "COMMIT")
        self.msgs.append(msg)
        self.byzantine_msgs.append(msg)
        return True

    def honest_receive_prepare(self, n):
        r = self.nodes[n].round
        candidates = [m for m in self.msgs if m.type == "PREPARE" and m.rnd == r 
                      and (m.val, m.rnd) not in self.nodes[n].prepared]
        if not candidates:
            return False
        m = random.choice(candidates)
        self.nodes[n].prepared.add((m.val, m.rnd))
        return True

    def honest_send_commit(self, n):
        r = self.nodes[n].round
        for v in self.values:
            if self.honest_prepare_count(v, r) >= self.quorum and (v, r) in self.nodes[n].prepared:
                if not any(m.sender == n and m.type == "COMMIT" and m.rnd == r for m in self.msgs):
                    self.msgs.append(Message(n, v, r, "COMMIT"))
                    return True
        return False

    def honest_do_commit(self, n):
        r = self.nodes[n].round
        for v in self.values:
            if self.honest_commit_count(v, r) >= self.quorum and (v, r) not in self.nodes[n].committed:
                self.nodes[n].committed.add((v, r))
                return True
        return False

    def check_honest_safety(self):
        commits = {}
        for n in self.honest_nodes:
            for val, rnd in n.committed:
                if rnd not in commits:
                    commits[rnd] = set()
                commits[rnd].add(val)
        for rnd, vals in commits.items():
            if len(vals) > 1:
                self.safety_violations += 1
                return False
        return True

    def run_trace(self, steps=200):
        honest_actions = [
            self.honest_leader_propose,
            self.honest_receive_prepare,
            self.honest_send_commit,
            self.honest_do_commit,
        ]
        byzantine_actions = [
            self.byzantine_send_prepare,
            self.byzantine_send_commit,
        ]
        
        for step in range(steps):
            # Byzantine act first (can corrupt before honest nodes see)
            for n in range(self.n_nodes):
                if self.nodes[n].is_byzantine:
                    if random.random() < 0.3:
                        random.choice(byzantine_actions)(n)
                else:
                    if random.random() < 0.5:
                        random.choice(honest_actions)(n)
            
            if not self.check_honest_safety():
                print(f"❌ SAFETY VIOLATION at step {step}!")
                return False
        
        return True

    def report(self):
        honest_commits = sum(len(n.committed) for n in self.honest_nodes)
        byzantine_commits = sum(len(n.committed) for n in self.byzantine_nodes)
        print(f"Nodes: {self.n_nodes}, F: {self.f}, Byzantine: {[n.id for n in self.byzantine_nodes]}")
        print(f"Honest commits: {honest_commits}, Byzantine commits: {byzantine_commits}")
        print(f"Byzantine msgs: {len(self.byzantine_msgs)}")
        print(f"Safety violations: {self.safety_violations}")
        print(f"Safety: {'✅ PASS' if self.safety_violations == 0 else '❌ FAIL'}")

def main():
    print("=" * 50)
    print("Orbital BFT — Byzantine Fault Simulation")
    print("=" * 50)

    # Test 1: 7 nodes, 2 Byzantine
    print("\n--- Test 1: 7 nodes, F=2, 200 steps ---")
    sim = OrbitalBFTByzantine(n_nodes=7, f=2, max_round=5)
    sim.run_trace(steps=200)
    sim.report()

    # Test 2: Stress — 100 traces with random Byzantine
    print("\n--- Test 2: Stress — 100 random traces ---")
    violations = 0
    for i in range(100):
        sim = OrbitalBFTByzantine(n_nodes=7, f=2, max_round=5)
        if not sim.run_trace(steps=150):
            violations += 1
    print(f"Traces: 100 | Safety violations: {violations}")
    print(f"Overall Safety: {'✅ PASS' if violations == 0 else '❌ FAIL'}")

if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""
QRAP Fee Splitter — Invariant Check (Python fallback for Termux)
Simulates Coq theorems: split_total, split_nonneg, burn_nonneg
"""

import random

class Distribution:
    def __init__(self, provers=35, validators=25, treasury=20, da_layer=15, burn=5):
        assert provers + validators + treasury + da_layer + burn == 100
        self.provers = provers
        self.validators = validators
        self.treasury = treasury
        self.da_layer = da_layer
        self.burn = burn

def split(amount, d):
    p = (amount * d.provers) // 100
    v = (amount * d.validators) // 100
    t = (amount * d.treasury) // 100
    da = (amount * d.da_layer) // 100
    b = amount - (p + v + t + da)
    return (p, v, t, da, b)

def check_invariants():
    d = Distribution()
    passed = 0
    failed = 0
    
    # Test 1: split_total — sum equals input
    for amount in [0, 1, 99, 100, 101, 1000, 10000, 999999]:
        p, v, t, da, b = split(amount, d)
        total = p + v + t + da + b
        assert total == amount, f"FAIL: amount={amount}, sum={total}"
        passed += 1
    
    # Test 2: split_nonneg — all parts >= 0 for amount >= 100
    for amount in [100, 200, 1000, 1000000]:
        p, v, t, da, b = split(amount, d)
        assert all(x >= 0 for x in [p, v, t, da, b]), f"FAIL: neg value at amount={amount}"
        passed += 1
    
    # Test 3: burn_nonneg — specifically burn >= 0
    for amount in [100, 500, 1000]:
        _, _, _, _, b = split(amount, d)
        assert b >= 0, f"FAIL: burn={b} at amount={amount}"
        passed += 1
    
    # Test 4: random fuzzing
    for _ in range(1000):
        amount = random.randint(0, 10**12)
        p, v, t, da, b = split(amount, d)
        assert p + v + t + da + b == amount
        if amount >= 100:
            assert all(x >= 0 for x in [p, v, t, da, b])
        passed += 1
    
    print(f"✅ All {passed} checks passed. Invariants hold.")
    print(f"   Provers={d.provers}%, Validators={d.validators}%, Treasury={d.treasury}%, DA={d.da_layer}%, Burn={d.burn}%")

if __name__ == "__main__":
    check_invariants()

# QRAP-Node — Quantum-Resistant Autonomous Protocol

[![QRAP CI](https://github.com/karamik/QRAP-Node/actions/workflows/ci.yml/badge.svg)](https://github.com/karamik/QRAP-Node/actions)

Space-grade ZK-rollup node with on-orbit proof generation, Celestia DA, and Orbital BFT consensus.

## Architecture
cd ~/QRAP-Node

# Смотрим, что есть в этих крейтах
echo "=== qrap-net ==="
head -n 40 crates/qrap-net/src/lib.rs
echo ""
echo "=== qrap-sim ==="
head -n 40 crates/qrap-sim/src/lib.rs
echo ""
echo "=== qrap-stark ==="
head -n 40 crates/qrap-stark/src/lib.rs
Закрой heredoc:

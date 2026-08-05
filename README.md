# QRAP-Node — Quantum-Resistant Autonomous Protocol

[![QRAP CI](https://github.com/karamik/QRAP-Node/actions/workflows/ci.yml/badge.svg)](https://github.com/karamik/QRAP-Node/actions)

Space-grade ZK-rollup node with on-orbit proof generation, Celestia DA, and Orbital BFT consensus.

## Architecture



## Crates

| Crate | Tests | Description |
|-------|-------|-------------|
|  | 8 | Poseidon hash, LWE commitments, ML-DSA/KEM |
|  | 7 | Fee split 35/25/20/15/5%, governance timelock |
|  | 12 | Celestia blob submit, Blobstream verify, DAS |
|  | 6 | Mock/Versal/AWS-F1 provers, TMR, radiation scrubber |
|  | 6 | Orbital BFT, leader rotation, view change |
|  | 6 | Sparse Merkle Tree, sled persistence |
|  | 6 | P2P message serialization, NodeId, codec |
|  | 6 | Byzantine, partition, FPGA degradation, DA failure |
|  | 4 | STARK prove/verify placeholder |
|  | 3 | E2E integration |
| **Total** | **70** | **0 failed** |

## Formal Verification

| Spec | Tool | Status |
|------|------|--------|
| Fee Splitter Invariants | Coq (Python fallback) | Sum=100%, non-negativity, burn>=0 |
| Orbital BFT Safety | TLA+ + Python sim | No fork, 100+ traces |
| Orbital BFT Byzantine | TLA+ + Python sim | F faults tolerated, 100 traces |
| Hardware Radiation | Python calculator | LEO/GEO TID estimation |

## Quick Start



## FPGA Targets

| Platform | Chip | PLONK Proof | Power |
|----------|------|-------------|-------|
| AWS F1 | Xilinx VU9P | 5-10s (software fallback) | 25-67W |
| Sentinel Space | AMD Versal XQRVC1902 | 1.6s (Full) / 4.0s (Eco) | 25-67W |

## Roadmap

| Quarter | Milestone |
|---------|-----------|
| Q3 2026 | Math formalization, 100k-agent simulation |
| Q4 2026 | Hardware prototype (AMD Versal) |
| Q1 2027 | Radiation qualification (TID 120 krad) |
| Q2 2027 | Pilot consortium with commercial operators |

## License

Dual-licensed under **MIT/Apache-2.0**.

For commercial licensing inquiries:
- **Telegram:** [@tec_support_bot](https://t.me/tec_support_bot)

## Contributing

See [docs/VALIDATOR_GUIDE.md](docs/VALIDATOR_GUIDE.md) for setup instructions.

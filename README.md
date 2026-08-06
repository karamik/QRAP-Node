# QRAP-Node — Quantum-Resistant Autonomous Protocol

[![QRAP CI](https://github.com/karamik/QRAP-Node/actions/workflows/ci.yml/badge.svg)](https://github.com/karamik/QRAP-Node/actions)

Space-grade ZK-rollup node with on-orbit proof generation, Celestia DA, and Orbital BFT consensus.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        QRAP Node                            │
├─────────────┬─────────────┬─────────────┬─────────────────┤
│  qrap-crypto │ qrap-fee   │   qrap-da   │   qrap-fpga     │
│  Poseidon256 │ splitter   │  Celestia   │  AWS F1 /       │
│  LWE, ML-DSA │ 35/25/20/  │  Blobstream │  Versal XQRVC   │
│              │ 15/5%      │  DAS        │  1902           │
├─────────────┴─────────────┴─────────────┴─────────────────┤
│                    qrap-consensus                           │
│              Orbital BFT — leader rotation                  │
│              View change, 2f+1 quorum                       │
├─────────────────────────────────────────────────────────────┤
│  qrap-net  │  qrap-storage  │  qrap-utxo  │  qrap-stark   │
│  P2P, codec│  Sparse Merkle │  Commitments│  PLONK/STARK  │
│            │  Tree, sled    │             │  proofs       │
├─────────────────────────────────────────────────────────────┤
│                    qrap-sim                                 │
│  Byzantine, partition, FPGA degradation, DA failure         │
└─────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Tests | Description |
|-------|-------|-------------|
| `qrap-crypto` | 2 | Poseidon hash, LWE commitments, ML-DSA/KEM |
| `qrap-fee-splitter` | 8 | Fee split 35/25/20/15/5%, governance timelock |
| `qrap-da` | 12 | Celestia blob submit, Blobstream verify, DAS |
| `qrap-fpga` | 7 | Mock/Versal/AWS-F1 provers, TMR, radiation scrubber |
| `qrap-consensus` | 7 | Orbital BFT, leader rotation, view change |
| `qrap-storage` | 6 | Sparse Merkle Tree, sled persistence |
| `qrap-net` | 0 | P2P message serialization, NodeId, codec |
| `qrap-utxo` | 2 | UTXO commitments, spend verification |
| `qrap-sim` | 6 | Byzantine, partition, FPGA degradation, DA failure |
| `qrap-stark` | 6 | STARK spend proof + PLONK prover with AWS F1 |
| `qrap-node` | 3 | E2E integration, CLI |
| **Total** | **59** | **0 failed** |

## Formal Verification

| Spec | Tool | Status |
|------|------|--------|
| Fee Splitter Invariants | Coq (Python fallback) | Sum=100%, non-negativity, burn>=0 |
| Orbital BFT Safety | TLA+ + Python sim | No fork, 100+ traces |
| Orbital BFT Byzantine | TLA+ + Python sim | F faults tolerated, 100 traces |
| Hardware Radiation | Python calculator | LEO/GEO TID estimation |

## Quick Start

```bash
# Clone
git clone https://github.com/karamik/QRAP-Node.git
cd QRAP-Node

# Test (mock mode — no FPGA required)
cargo test --workspace --features mock

# Test with AWS F1 support (requires OpenCL headers on x86)
cargo test -p qrap-fpga --features aws-f1

# Build release
cargo build --release --workspace
```

## FPGA Targets

| Platform | Chip | PLONK Proof | Power |
|----------|------|-------------|-------|
| **AWS F1** | Xilinx VU9P | 5-10s (hw_emu) | 25-67W |
| **Sentinel Space** | AMD Versal XQRVC1902 | 1.6s (Full) / 4.0s (Eco) | 25-67W |

### AWS F1 Build

```bash
cd crates/qrap-fpga
./build_xclbin.sh hw_emu   # Hardware emulation (fast debug)
./build_xclbin.sh hw       # Real FPGA bitstream (4-6 hours)
```

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

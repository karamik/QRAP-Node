# QRAP-Node v0.2.0-alpha

**Quantum-Resistant Anchor Protocol Node**

Unified monorepo combining QRAP Core consensus/UTXO engine with total-net kernel-bypass networking.

## Architecture

```
┌─────────────────────────────────────────────┐
│  qrap-node      CLI + Runtime Orchestrator  │
├─────────────────────────────────────────────┤
│  qrap-consensus  Orbital BFT (4 validators) │
├─────────────────────────────────────────────┤
│  qrap-utxo       UTXO + Epoch Nullifiers    │
├─────────────────────────────────────────────┤
│  qrap-crypto     Ring-LWE, Poseidon, ML-KEM │
├─────────────────────────────────────────────┤
│  qrap-net        io_uring P2P + RPC + TLS   │
└─────────────────────────────────────────────┘
```

## Quick Start (Termux / Linux)

```bash
# 1. Clone
git clone https://github.com/karamik/QRAP-Node.git
cd QRAP-Node

# 2. Build release
cargo build --release

# 3. Generate validator keys
./target/release/qrap-node keygen --output validator.keys

# 4. Run single-node dev mode
./target/release/qrap-node run --config config/dev.toml

# 5. Run 4-node local testnet
./target/release/qrap-node benchmark --network --nodes 4
```

## Workspace Crates

| Crate | Purpose |
|-------|---------|
| `qrap-net` | io_uring P2P mesh, zero-alloc TLS, RPC gateway |
| `qrap-crypto` | Post-quantum primitives (Ring-LWE, ML-KEM, Poseidon) |
| `qrap-consensus` | Orbital BFT consensus engine |
| `qrap-utxo` | UTXO state machine + epoch nullifier trees |
| `qrap-node` | CLI binary and runtime coordinator |

## License

MIT — see [LICENSE](LICENSE).

> In Physics We Trust. Not hype. Not promises.

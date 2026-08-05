# QRAP Validator Onboarding Guide

## Requirements

### Hardware

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8 cores |
| RAM | 16 GB | 32 GB |
| Storage | 500 GB SSD | 2 TB NVMe |
| Network | 100 Mbps | 1 Gbps |
| Uptime | 95% | 99.9% |

### Software

- Rust 1.80+
- Docker (optional)
- systemd (Linux)

## Quick Start

1. Install: git clone + cargo build --release
2. Generate keys: qrap-node keygen --output validator.keys
3. Start node: qrap-node run --node-id 0 --data-dir /var/lib/qrap/data
4. Stake: Minimum 5 ETH equivalent, 14-day unbonding

## Monitoring

- qrap_blocks_proposed_total
- qrap_txs_processed_total
- qrap_fpga_proofs_total
- qrap_da_blobs_submitted

## Security

1. Use HSM or encrypted keystore
2. Firewall: allow only P2P port (10000) and RPC
3. Daily backups of /var/lib/qrap/data
4. Subscribe to security advisories

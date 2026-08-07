# QRAP Whitepaper
## Quantum-Resistant Autonomous Protocol for Orbital Economies

**Version:** 0.2.0-alpha  
**Date:** August 2026  
**Status:** Research & Development  
**License:** MIT/Apache-2.0  

---

## 1. Executive Summary

QRAP is a space-grade ZK-rollup protocol designed to enable quantum-resistant financial infrastructure for orbital and terrestrial economies. It combines zero-knowledge proof generation (PLONK) on FPGA accelerators, Celestia data availability, and Orbital BFT consensus to create a bank-grade system capable of operating in high-radiation environments.

**Key Innovation:** On-orbit proof generation with hardware-level radiation tolerance and post-quantum cryptographic primitives.

---

## 2. The Problem

### 2.1 Quantum Threat to Cryptography
Existing blockchain infrastructure (Bitcoin, Ethereum) relies on ECDSA and RSA signatures vulnerable to Shor's algorithm on quantum computers. NIST estimates cryptographically-relevant quantum computers will emerge by 2033. All current financial infrastructure becomes insecure.

### 2.2 Space Infrastructure Gap
Current satellites run standard Linux with no Byzantine fault tolerance, no cryptographic proof generation, and no economic incentive layer. A single solar flare can corrupt financial state without detection.

### 2.3 Centralization of Computation
ZK-proof generation is centralized in large data centers. There is no protocol for decentralized, incentivized proof generation on specialized hardware (FPGA/ASIC).

---

## 3. Architecture

### 3.1 Stack Overview

| Layer | Component | Function |
|-------|-----------|----------|
| **Execution** | Geth fork (QRAP-EVM) | Smart contract execution, 6s blocks |
| **Proving** | qrap-fpga (AWS F1 / Versal) | PLONK proof generation, 5-10s per proof |
| **Consensus** | Orbital BFT | 2f+1 quorum, 4s view change, leader rotation |
| **DA** | Celestia | Blob submission, Blobstream verification, DAS |
| **Settlement** | ENT Epoch Pruning | State compression, fraud proof window |

### 3.2 Orbital BFT Consensus
- **Leader Rotation:** Deterministic round-robin with VRF randomization
- **View Change:** 4-second timeout, automatic failover
- **Quorum:** 2f+1 out of 3f+1 validators
- **Space Adaptation:** TMR (Triple Modular Redundancy) on critical consensus state

### 3.3 FPGA Acceleration
Three kernels implemented in OpenCL for Xilinx VU9P (AWS F1) and AMD Versal XQRVC1902:

1. **Field Arithmetic** — 256-bit BN254 Montgomery multiplication
2. **NTT** — Radix-2 Cooley-Tukey Number Theoretic Transform
3. **MSM** — Pippenger bucket method for multi-scalar multiplication

**Performance:** 24× speedup vs CPU (120s → 5s per PLONK proof).

---

## 4. Economic Model

### 4.1 Fee Splitter
Every transaction fee is automatically distributed:

| Recipient | Share | Purpose |
|-----------|-------|---------|
| Provers | 35% | FPGA operators generating ZK proofs |
| Validators | 25% | Consensus participation (stake-weighted) |
| Treasury | 20% | Protocol development (3/5 multi-sig) |
| DA Layer | 15% | Celestia blobspace payment |
| Burn | 5% | Deflationary mechanism |

**Formal Verification:** Coq proof confirms sum = 100%, non-negativity, burn ≥ 0.

### 4.2 Prover Economics
- **Entry:** AWS F1 VU9P instance ($3,200/mo) or owned hardware
- **Revenue:** 35% of all fees + 20% FPGA bonus
- **Break-even:** Day 1 at 100 TPS, $0.01 avg fee
- **ROI:** 400-600% annually at full capacity

---

## 5. Radiation Hardening

### 5.1 AMD Versal XQRVC1902
- **Process:** 7nm space-grade, TID 100-120 krad
- **SEU Tolerance:** >80 MeV·cm²/mg
- **Scrubbing:** XiISEM at 320 MHz, 13.6ms full scan
- **Power Modes:** Full (67W), Balanced (45W), Eco (25W)

### 5.2 Software Mitigation
- Application-level TMR with voting
- Checkpoint/rollback every 1024 blocks
- PLONK proof verification as self-check

---

## 6. Roadmap

| Phase | Timeline | Deliverable |
|-------|----------|-------------|
| Alpha | Q3 2026 | Math formalization, 100k-agent simulation |
| Beta | Q4 2026 | Hardware prototype (AMD Versal dev board) |
| Testnet | Q1 2027 | Public testnet, radiation qualification |
| Mainnet Pilot | Q2 2027 | Consortium with 3+ commercial satellite operators |
| Production | 2028+ | Full orbital deployment, token generation event |

---

## 7. Team & Contacts

**Core Contributor:** karamik  
**Repository:** https://github.com/karamik/QRAP-Node  
**Technical Inquiries:** [@tec_support_bot](https://t.me/tec_support_bot)  

**Status:** Open source R&D. No token issued. No investment solicited.

---

## 8. Conclusion

QRAP represents a first-principles approach to orbital finance: quantum-resistant cryptography, hardware-accelerated zero-knowledge proofs, and Byzantine consensus designed for the radiation environment of space. By open-sourcing the protocol, we invite collaboration from cryptographers, aerospace engineers, and distributed systems researchers.

**The future of money will not be built in data centers. It will be proven in orbit.**

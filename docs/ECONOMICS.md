# QRAP Economic Security Analysis

## Tokenomics

| Parameter | Value |
|-----------|-------|
| Initial Supply | 1,000,000,000 QRAP |
| Inflation | 2% annually |
| Burn Rate | 5% of all fees |
| Staking Minimum | 5 ETH equivalent |
| Unbonding Period | 14 days |

## Fee Splitter

- Provers: 35% (+20% FPGA bonus)
- Validators: 25% (stake × uptime)
- Treasury: 20% (3/5 multi-sig)
- DA: 15% (Celestia)
- Burn: 5%

## Validator Economics

| Metric | Value |
|--------|-------|
| Min Stake | 5 ETH (~$15,000) |
| Expected APR | 15-30% |
| Slashing | Double-sign, downtime >10% |

## Prover Economics

| Setup | CAPEX | OPEX/Month | ROI Year 3 |
|-------|-------|-----------|-----------|
| CPU (mock) | $0 | $90 | N/A |
| AWS F1 | $0 | $3,200 | 400% |
| Versal Flight | $105K | $500 | 600% |

## Attack Vectors

| Attack | Cost | Defense |
|--------|------|---------|
| 51% stake | ~$7.5M | Quorum + slashing |
| FPGA centralization | ~$1M | Decentralized market |
| DA withholding | $0 | DAS + fraud proofs |
| Eclipse attack | $50K | Mesh topology |

## Break-even

| Scenario | Break-even | Monthly Revenue |
|----------|-----------|----------------|
| Conservative (100 TPS) | 18 months | $260K |
| Realistic (1,000 TPS) | 6 months | $860K |
| Optimistic (10,000 TPS) | 2 months | $8.6M |

## Recommendations

1. Bootstrap: Treasury grants for validators
2. Prover incentives: Higher FPGA bonus early
3. Slashing insurance: Optional coverage
4. Governance: Gradual decentralization over 2 years

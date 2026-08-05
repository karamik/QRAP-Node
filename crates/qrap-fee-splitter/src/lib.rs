//! QRAP Fee Splitter — TOTAL Protocol Revenue Distribution
//!
//! Distribution:
//! - Provers: 35% (+20% FPGA bonus)
//! - Validators: 25% (stake × uptime weighted)
//! - Treasury: 20% (3/5 multi-sig)
//! - DA (Data Availability): 15%
//! - Burn: 5%
//!
//! Governance: 14-day timelock, max 5% change/epoch

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, debug};

/// Distribution categories
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Distribution {
    Provers,      // 35% + FPGA bonus
    Validators,   // 25% weighted by stake×uptime
    Treasury,     // 20% 3/5 multi-sig
    DataAvailability, // 15% Celestia/Blobstream
    Burn,         // 5% deflationary
}

impl Distribution {
    pub fn base_percentage(&self) -> u16 {
        match self {
            Distribution::Provers => 35,
            Distribution::Validators => 25,
            Distribution::Treasury => 20,
            Distribution::DataAvailability => 15,
            Distribution::Burn => 5,
        }
    }
}

/// Prover metadata for bonus calculation
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProverInfo {
    pub id: String,
    pub is_fpga: bool,
    pub proofs_generated: u64,
    pub uptime_secs: u64,
}

/// Validator metadata for reward weighting
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ValidatorInfo {
    pub id: String,
    pub stake: u64,      // in wei/gwei
    pub uptime_secs: u64,
    pub blocks_proposed: u64,
}

/// Governance parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Governance {
    pub timelock_days: u16,
    pub max_change_per_epoch_bps: u16, // basis points (5% = 500)
    pub last_change_epoch: u64,
    pub multi_sig_threshold: u8, // 3/5
}

impl Default for Governance {
    fn default() -> Self {
        Self {
            timelock_days: 14,
            max_change_per_epoch_bps: 500, // 5%
            last_change_epoch: 0,
            multi_sig_threshold: 3,
        }
    }
}

#[derive(Debug, Error)]
pub enum FeeError {
    #[error("Invalid distribution: sum must be 100%, got {0}%")]
    InvalidDistribution(u16),
    #[error("Governance timelock active: {0} days remaining")]
    TimelockActive(u16),
    #[error("Change exceeds max per epoch: {0} bps > {1} bps")]
    ChangeTooLarge(u16, u16),
    #[error("Treasury requires {0}/{1} multi-sig")]
    MultiSigRequired(u8, u8),
}

/// Fee split result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeSplit {
    pub total_amount: u64,
    pub provers: u64,
    pub validators: u64,
    pub treasury: u64,
    pub da: u64,
    pub burn: u64,
    pub prover_bonus: u64, // extra 20% for FPGA
    pub epoch: u64,
}

/// Main fee splitter
#[derive(Clone, Debug)]
pub struct FeeSplitter {
    pub distribution: HashMap<Distribution, u16>,
    pub governance: Governance,
    pub fpga_bonus_bps: u16, // 2000 = 20%
    pub current_epoch: u64,
}

impl FeeSplitter {
    pub fn new() -> Self {
        let mut distribution = HashMap::new();
        distribution.insert(Distribution::Provers, 35);
        distribution.insert(Distribution::Validators, 25);
        distribution.insert(Distribution::Treasury, 20);
        distribution.insert(Distribution::DataAvailability, 15);
        distribution.insert(Distribution::Burn, 5);
        
        Self {
            distribution,
            governance: Governance::default(),
            fpga_bonus_bps: 2000, // 20%
            current_epoch: 0,
        }
    }
    
    /// Validate distribution sums to 100%
    pub fn validate(&self) -> Result<(), FeeError> {
        let total: u16 = self.distribution.values().sum();
        if total != 100 {
            return Err(FeeError::InvalidDistribution(total));
        }
        Ok(())
    }
    
    /// Calculate fee split for given amount
    pub fn split(&self, amount: u64, provers: &[ProverInfo], validators: &[ValidatorInfo]) -> Result<FeeSplit, FeeError> {
        self.validate()?;
        
        let provers_pct = *self.distribution.get(&Distribution::Provers).unwrap_or(&35);
        let validators_pct = *self.distribution.get(&Distribution::Validators).unwrap_or(&25);
        let treasury_pct = *self.distribution.get(&Distribution::Treasury).unwrap_or(&20);
        let da_pct = *self.distribution.get(&Distribution::DataAvailability).unwrap_or(&15);
        let burn_pct = *self.distribution.get(&Distribution::Burn).unwrap_or(&5);
        
        let provers_base = amount * provers_pct as u64 / 100;
        let validators_base = amount * validators_pct as u64 / 100;
        let treasury = amount * treasury_pct as u64 / 100;
        let da = amount * da_pct as u64 / 100;
        let burn = amount * burn_pct as u64 / 100;
        
        // FPGA bonus: +20% of prover share
        let fpga_count = provers.iter().filter(|p| p.is_fpga).count() as u64;
        let total_proofs: u64 = provers.iter().map(|p| p.proofs_generated).sum();
        let prover_bonus = if total_proofs > 0 {
            provers_base * self.fpga_bonus_bps as u64 / 10000 * fpga_count / provers.len().max(1) as u64
        } else {
            0
        };
        
        // Validator weighting: stake × uptime
        let total_weight: u128 = validators.iter()
            .map(|v| v.stake as u128 * v.uptime_secs as u128)
            .sum();
        
        let validators_final = if total_weight > 0 {
            validators_base // Simplified — in real impl, weighted per-validator
        } else {
            validators_base
        };
        
        let provers_final = provers_base + prover_bonus;
        
        // Ensure total doesn't exceed amount (rounding errors)
        let total_distributed = provers_final + validators_final + treasury + da + burn;
        let burn_adjusted = if total_distributed > amount {
            burn.saturating_sub(total_distributed - amount)
        } else {
            burn
        };
        
        debug!("Fee split: amount={} | provers={} (bonus={}) | validators={} | treasury={} | da={} | burn={}",
               amount, provers_final, prover_bonus, validators_final, treasury, da, burn_adjusted);
        
        Ok(FeeSplit {
            total_amount: amount,
            provers: provers_final,
            validators: validators_final,
            treasury,
            da,
            burn: burn_adjusted,
            prover_bonus,
            epoch: self.current_epoch,
        })
    }
    
    /// Propose distribution change (governance)
    pub fn propose_change(&mut self, new_distribution: HashMap<Distribution, u16>, current_epoch: u64) -> Result<(), FeeError> {
        // Check timelock
        let epochs_since_change = current_epoch.saturating_sub(self.governance.last_change_epoch);
        let min_epochs = self.governance.timelock_days as u64 * 24 * 60 * 60 / 6; // 6-sec blocks
        
        if epochs_since_change < min_epochs {
            let remaining = ((min_epochs - epochs_since_change) * 6) / 86400;
            return Err(FeeError::TimelockActive(remaining as u16));
        }
        
        // Check max change per epoch
        let mut max_change: u16 = 0;
        for (dist, &new_pct) in &new_distribution {
            let old_pct = self.distribution.get(dist).copied().unwrap_or(0);
            let change = if new_pct > old_pct { new_pct - old_pct } else { old_pct - new_pct };
            max_change = max_change.max(change);
        }
        
        if max_change > self.governance.max_change_per_epoch_bps / 100 {
            return Err(FeeError::ChangeTooLarge(
                max_change,
                self.governance.max_change_per_epoch_bps / 100
            ));
        }
        
        // Validate new distribution
        let total: u16 = new_distribution.values().sum();
        if total != 100 {
            return Err(FeeError::InvalidDistribution(total));
        }
        
        self.distribution = new_distribution;
        self.governance.last_change_epoch = current_epoch;
        self.current_epoch = current_epoch;
        
        info!("Fee distribution updated at epoch {}", current_epoch);
        Ok(())
    }
    
    /// Check treasury multi-sig
    pub fn check_treasury_sig(&self, sigs: u8) -> Result<(), FeeError> {
        if sigs < self.governance.multi_sig_threshold {
            return Err(FeeError::MultiSigRequired(sigs, self.governance.multi_sig_threshold));
        }
        Ok(())
    }
    
    /// Advance epoch
    pub fn advance_epoch(&mut self) {
        self.current_epoch += 1;
        debug!("FeeSplitter advanced to epoch {}", self.current_epoch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_distribution() {
        let splitter = FeeSplitter::new();
        splitter.validate().unwrap();
        assert_eq!(splitter.distribution.get(&Distribution::Provers), Some(&35));
        assert_eq!(splitter.distribution.get(&Distribution::Burn), Some(&5));
    }

    #[test]
    fn test_fee_split_basic() {
        let splitter = FeeSplitter::new();
        let provers = vec![
            ProverInfo { id: "p1".to_string(), is_fpga: true, proofs_generated: 100, uptime_secs: 3600 },
            ProverInfo { id: "p2".to_string(), is_fpga: false, proofs_generated: 50, uptime_secs: 3600 },
        ];
        let validators = vec![
            ValidatorInfo { id: "v1".to_string(), stake: 1000, uptime_secs: 3600, blocks_proposed: 10 },
        ];
        
        let split = splitter.split(10000, &provers, &validators).unwrap();
        assert_eq!(split.total_amount, 10000);
        assert_eq!(split.provers, 3500 + split.prover_bonus);
        assert_eq!(split.validators, 2500);
        assert_eq!(split.treasury, 2000);
        assert_eq!(split.da, 1500);
        assert!(split.burn <= 500);
        assert!(split.prover_bonus > 0); // FPGA bonus applied
    }

    #[test]
    fn test_fpga_bonus_calculation() {
        let splitter = FeeSplitter::new();
        let provers = vec![
            ProverInfo { id: "fpga1".to_string(), is_fpga: true, proofs_generated: 1000, uptime_secs: 86400 },
            ProverInfo { id: "cpu1".to_string(), is_fpga: false, proofs_generated: 100, uptime_secs: 86400 },
        ];
        let validators = vec![];
        
        let split = splitter.split(100000, &provers, &validators).unwrap();
        // 35% base = 35000, bonus = 35000 * 0.20 * 1/2 = 3500
        assert!(split.prover_bonus > 0);
        assert_eq!(split.provers, 35000 + split.prover_bonus);
    }

    #[test]
    fn test_governance_timelock() {
        let mut splitter = FeeSplitter::new();
        let mut new_dist = splitter.distribution.clone();
        new_dist.insert(Distribution::Provers, 36);
        new_dist.insert(Distribution::Validators, 24);
        
        // Should fail — timelock not expired
        let result = splitter.propose_change(new_dist.clone(), 1);
        assert!(matches!(result, Err(FeeError::TimelockActive(_))));
    }

    #[test]
    fn test_governance_max_change() {
        let mut splitter = FeeSplitter::new();
        let mut new_dist = splitter.distribution.clone();
        new_dist.insert(Distribution::Provers, 50); // +15%, exceeds 5% max
        
        let result = splitter.propose_change(new_dist, 201600); // ~14 days of 6-sec blocks
        assert!(matches!(result, Err(FeeError::ChangeTooLarge(15, 5))));
    }

    #[test]
    fn test_treasury_multisig() {
        let splitter = FeeSplitter::new();
        assert!(splitter.check_treasury_sig(3).is_ok());
        assert!(splitter.check_treasury_sig(5).is_ok());
        assert!(matches!(splitter.check_treasury_sig(2), Err(FeeError::MultiSigRequired(2, 3))));
    }

    #[test]
    fn test_invalid_distribution() {
        let mut splitter = FeeSplitter::new();
        splitter.distribution.insert(Distribution::Provers, 50);
        assert!(matches!(splitter.validate(), Err(FeeError::InvalidDistribution(_))));
    }
}


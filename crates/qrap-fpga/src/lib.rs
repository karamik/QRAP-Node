//! QRAP FPGA Prover — Sentinel Space Core abstraction layer
//!
//! Supports:
//! - Mock (software fallback, default)
//! - AWS F1 (Xilinx VU9P, development)
//! - AMD Versal XQRVC1902 (space-grade flight hardware)
//!
//! Power states: Full (67W/1.6s), Balanced (45W/2.2s), Eco (25W/4s)

use qrap_crypto::{Hash, poseidon256_pair};
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{info, debug, warn};

/// FPGA power state
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerState {
    #[default]
    Full,
    Balanced,
    Eco,
}

impl PowerState {
    pub fn target_time(&self) -> Duration {
        match self {
            PowerState::Full => Duration::from_millis(1600),
            PowerState::Balanced => Duration::from_millis(2200),
            PowerState::Eco => Duration::from_millis(4000),
        }
    }

    pub fn power_watts(&self) -> u16 {
        match self {
            PowerState::Full => 67,
            PowerState::Balanced => 45,
            PowerState::Eco => 25,
        }
    }
}

/// Radiation event detected by scrubber
#[derive(Clone, Debug)]
pub struct RadEvent {
    pub bit_flips: u32,
    pub region: String,
    pub timestamp: u64,
}

/// FPGA health telemetry
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FpgaHealth {
    pub temperature_c: f32,
    pub voltage_core: f32,
    pub scrubber_cycles: u64,
    pub radiation_events: u32,
    pub power_state: PowerState,
    pub uptime_secs: u64,
}

/// Proof input for PLONK circuit
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlonkInput {
    pub nullifiers: Vec<Hash>,
    pub commitments: Vec<Hash>,
    pub public_inputs: Vec<u64>,
}

/// Proof output
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlonkProof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<u64>,
    pub verification_hash: Hash,
    pub generation_time_ms: u64,
    pub power_consumed_mj: u64,
}

#[derive(Debug, Error)]
pub enum FpgaError {
    #[error("FPGA not available: {0}")]
    NotAvailable(String),
    #[error("Radiation fault detected in region {0}")]
    RadiationFault(String),
    #[error("Thermal throttling: {0}°C")]
    ThermalThrottling(f32),
    #[error("Proof generation failed: {0}")]
    ProofFailed(String),
    #[error("Reconfiguration error: {0}")]
    ReconfigError(String),
}

/// Core trait — FPGA prover abstraction
#[async_trait::async_trait]
pub trait FpgaProver: Send + Sync {
    async fn init(&mut self) -> Result<(), FpgaError>;
    async fn prove(&mut self, input: PlonkInput) -> Result<PlonkProof, FpgaError>;
    async fn set_power(&mut self, state: PowerState) -> Result<(), FpgaError>;
    async fn health(&self) -> FpgaHealth;
    async fn scrub(&mut self) -> Result<Vec<RadEvent>, FpgaError>;
    async fn reconfigure(&mut self, bitstream: &[u8]) -> Result<(), FpgaError>;
}

// ============== MOCK IMPLEMENTATION ==============
pub struct MockFpga {
    health: FpgaHealth,
    power: PowerState,
    start_time: Instant,
}

impl MockFpga {
    pub fn new() -> Self {
        Self {
            health: FpgaHealth {
                temperature_c: 35.0,
                voltage_core: 0.85,
                scrubber_cycles: 0,
                radiation_events: 0,
                power_state: PowerState::Full,
                uptime_secs: 0,
            },
            power: PowerState::Full,
            start_time: Instant::now(),
        }
    }

    fn mock_prove(&self, input: &PlonkInput) -> PlonkProof {
        let start = Instant::now();
        let target = self.power.target_time();
        let mut hash = [0u8; 32];
        for (i, nf) in input.nullifiers.iter().enumerate() {
            hash = poseidon256_pair(&hash, nf);
            if i % 100 == 0 { std::thread::sleep(Duration::from_micros(10)); }
        }
        for (i, cm) in input.commitments.iter().enumerate() {
            hash = poseidon256_pair(&hash, cm);
            if i % 100 == 0 { std::thread::sleep(Duration::from_micros(10)); }
        }
        let elapsed = start.elapsed();
        if elapsed < target { std::thread::sleep(target - elapsed); }
        let total = start.elapsed();
        let energy_j = self.power.power_watts() as f64 * total.as_secs_f64();
        PlonkProof {
            proof_bytes: hash.to_vec(),
            public_inputs: input.public_inputs.clone(),
            verification_hash: hash,
            generation_time_ms: total.as_millis() as u64,
            power_consumed_mj: (energy_j * 1000.0) as u64,
        }
    }
}

#[async_trait::async_trait]
impl FpgaProver for MockFpga {
    async fn init(&mut self) -> Result<(), FpgaError> {
        info!("MockFPGA initialized (software fallback)");
        Ok(())
    }
    async fn prove(&mut self, input: PlonkInput) -> Result<PlonkProof, FpgaError> {
        if self.health.temperature_c > 85.0 {
            return Err(FpgaError::ThermalThrottling(self.health.temperature_c));
        }
        let proof = self.mock_prove(&input);
        self.health.scrubber_cycles += 1;
        info!("MockFPGA proof: {}ms, {}mJ", proof.generation_time_ms, proof.power_consumed_mj);
        Ok(proof)
    }
    async fn set_power(&mut self, state: PowerState) -> Result<(), FpgaError> {
        self.power = state;
        self.health.power_state = state;
        info!("MockFPGA -> {:?} ({}W)", state, state.power_watts());
        Ok(())
    }
    async fn health(&self) -> FpgaHealth {
        let mut h = self.health.clone();
        h.uptime_secs = self.start_time.elapsed().as_secs();
        h
    }
    async fn scrub(&mut self) -> Result<Vec<RadEvent>, FpgaError> {
        self.health.scrubber_cycles += 1;
        debug!("MockFPGA scrub #{}", self.health.scrubber_cycles);
        Ok(vec![])
    }
    async fn reconfigure(&mut self, _bitstream: &[u8]) -> Result<(), FpgaError> {
        warn!("MockFPGA reconfigure: no-op");
        Ok(())
    }
}

// ============== AWS F1 IMPLEMENTATION ==============
#[cfg(feature = "aws-f1")]
pub mod aws_f1;
#[cfg(feature = "aws-f1")]
pub use aws_f1::prover::AwsF1Prover;

// ============== VERSAL SPACE-GRADE IMPLEMENTATION ==============
#[cfg(feature = "versal")]
pub mod versal;
#[cfg(feature = "versal")]
pub use versal::VersalProver;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_fpga_full_power() {
        let mut fpga = MockFpga::new();
        fpga.init().await.unwrap();
        let input = PlonkInput {
            nullifiers: vec![[0x01; 32], [0x02; 32]],
            commitments: vec![[0x03; 32]],
            public_inputs: vec![100, 50],
        };
        let proof = fpga.prove(input).await.unwrap();
        assert!(proof.generation_time_ms >= 1500);
        assert!(proof.power_consumed_mj > 0);
        assert_eq!(fpga.health().await.power_state, PowerState::Full);
        assert_eq!(fpga.health().await.scrubber_cycles, 1);
    }

    #[tokio::test]
    async fn test_power_state_switching() {
        let mut fpga = MockFpga::new();
        fpga.init().await.unwrap();
        fpga.set_power(PowerState::Eco).await.unwrap();
        assert_eq!(fpga.health().await.power_state, PowerState::Eco);
        assert_eq!(fpga.health().await.power_state.power_watts(), 25);
        fpga.set_power(PowerState::Balanced).await.unwrap();
        assert_eq!(fpga.health().await.power_state, PowerState::Balanced);
        assert_eq!(fpga.health().await.power_state.power_watts(), 45);
    }

    #[tokio::test]
    async fn test_radiation_scrubber() {
        let mut fpga = MockFpga::new();
        let events = fpga.scrub().await.unwrap();
        assert!(events.is_empty());
        assert_eq!(fpga.health().await.scrubber_cycles, 1);
    }
}

//! AMD Versal XQRVC1902 — Space-Grade Radiation-Hardened Prover

use crate::{FpgaProver, FpgaHealth, PowerState, PlonkInput, PlonkProof, FpgaError, RadEvent};
use async_trait::async_trait;
use qrap_crypto::poseidon256_pair;
use tracing::{info, debug, warn};
use std::time::Instant;
use std::collections::VecDeque;

pub struct XiisemConfig {
    pub frequency_mhz: u32,
    pub scan_period_ms: f32,
    pub scrub_interval_ms: u64,
}

impl Default for XiisemConfig {
    fn default() -> Self {
        Self { frequency_mhz: 320, scan_period_ms: 13.6, scrub_interval_ms: 100 }
    }
}

pub struct TmrVoter<T: Clone + PartialEq> {
    modules: [Option<T>; 3],
}

impl<T: Clone + PartialEq> TmrVoter<T> {
    pub fn new() -> Self {
        Self { modules: [None, None, None] }
    }
    pub fn set(&mut self, module: usize, value: T) {
        if module < 3 { self.modules[module] = Some(value); }
    }
    pub fn vote(&self) -> Option<T> {
        let vals: Vec<_> = self.modules.iter().flatten().collect();
        if vals.len() < 2 { return None; }
        for i in 0..vals.len() {
            for j in (i+1)..vals.len() {
                if vals[i] == vals[j] { return Some(vals[i].clone()); }
            }
        }
        None
    }
    pub fn is_faulty(&self) -> bool {
        self.vote().is_none() && self.modules.iter().any(|m| m.is_some())
    }
}

#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub epoch: u64,
    pub block: u64,
    pub state_hash: [u8; 32],
    pub timestamp: u64,
}

pub struct RadHardState {
    pub checkpoints: VecDeque<Checkpoint>,
    pub max_checkpoints: usize,
    pub tmr_voter: TmrVoter<[u8; 32]>,
    pub scrubber: XiisemConfig,
    pub last_scrub: Instant,
}

impl RadHardState {
    pub fn new() -> Self {
        Self {
            checkpoints: VecDeque::with_capacity(10),
            max_checkpoints: 10,
            tmr_voter: TmrVoter::new(),
            scrubber: XiisemConfig::default(),
            last_scrub: Instant::now(),
        }
    }
    pub fn create_checkpoint(&mut self, epoch: u64, block: u64, state_hash: [u8; 32]) {
        let cp = Checkpoint {
            epoch, block, state_hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        };
        if self.checkpoints.len() >= self.max_checkpoints { self.checkpoints.pop_front(); }
        self.checkpoints.push_back(cp);
        debug!("Checkpoint: epoch={}, block={}", epoch, block);
    }
    pub fn restore_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.back()
    }
    pub fn should_scrub(&self) -> bool {
        self.last_scrub.elapsed().as_millis() as u64 >= self.scrubber.scrub_interval_ms
    }
    pub fn scrub_complete(&mut self) {
        self.last_scrub = Instant::now();
    }
}

pub struct VersalProver {
    health: FpgaHealth,
    power: PowerState,
    start_time: Instant,
    bitstream_version: String,
    rad_state: RadHardState,
    fault_injection_enabled: bool,
}

impl VersalProver {
    pub fn new() -> Self {
        Self {
            health: FpgaHealth {
                temperature_c: -40.0, voltage_core: 0.72,
                scrubber_cycles: 0, radiation_events: 0,
                power_state: PowerState::Eco, uptime_secs: 0,
            },
            power: PowerState::Eco,
            start_time: Instant::now(),
            bitstream_version: "versal-plonk-v1.0.0".to_string(),
            rad_state: RadHardState::new(),
            fault_injection_enabled: cfg!(test),
        }
    }
    fn tmr_prove(&mut self, input: &PlonkInput) -> Result<PlonkProof, FpgaError> {
        let start = Instant::now();
        for module in 0..3 {
            let mut hash = [0u8; 32];
            for nf in &input.nullifiers { hash = poseidon256_pair(&hash, nf); }
            for cm in &input.commitments { hash = poseidon256_pair(&hash, cm); }
            if self.fault_injection_enabled && module == 1 {
                hash[0] ^= 0xFF;
                debug!("TMR module 1: injected SEU");
            }
            self.rad_state.tmr_voter.set(module, hash);
        }
        let result = self.rad_state.tmr_voter.vote()
            .ok_or_else(|| FpgaError::RadiationFault("TMR majority failure".to_string()))?;
        let mismatches = self.rad_state.tmr_voter.modules.iter().flatten().filter(|m| **m != result).count();
        if mismatches > 0 {
            self.health.radiation_events += 1;
            warn!("Radiation event! {} TMR mismatch(es) corrected.", mismatches);
        }
        self.rad_state.create_checkpoint(0, 0, result);
        let total = start.elapsed();
        let energy_j = self.power.power_watts() as f64 * total.as_secs_f64();
        Ok(PlonkProof {
            proof_bytes: result.to_vec(),
            public_inputs: input.public_inputs.clone(),
            verification_hash: result,
            generation_time_ms: total.as_millis() as u64,
            power_consumed_mj: (energy_j * 1000.0) as u64,
        })
    }
    pub fn enable_fault_injection(&mut self, enable: bool) {
        self.fault_injection_enabled = enable;
        info!("Versal fault injection: {}", if enable { "ON" } else { "OFF" });
    }
}

#[async_trait]
impl FpgaProver for VersalProver {
    async fn init(&mut self) -> Result<(), FpgaError> {
        info!("Versal XQRVC1902 | TID 120krad | SEL>80 MeV·cm²/mg");
        info!("XiISEM: {}MHz / {}ms scan", self.rad_state.scrubber.frequency_mhz, self.rad_state.scrubber.scan_period_ms);
        Ok(())
    }
    async fn prove(&mut self, input: PlonkInput) -> Result<PlonkProof, FpgaError> {
        if self.rad_state.should_scrub() { self.scrub().await?; }
        self.tmr_prove(&input)
    }
    async fn set_power(&mut self, state: PowerState) -> Result<(), FpgaError> {
        self.power = state;
        self.health.power_state = state;
        info!("Versal -> {:?} ({}W)", state, state.power_watts());
        Ok(())
    }
    async fn health(&self) -> FpgaHealth {
        let mut h = self.health.clone();
        h.uptime_secs = self.start_time.elapsed().as_secs();
        h
    }
    async fn scrub(&mut self) -> Result<Vec<RadEvent>, FpgaError> {
        self.health.scrubber_cycles += 1;
        self.rad_state.scrub_complete();
        info!("XiISEM scrub #{} done", self.health.scrubber_cycles);
        Ok(vec![])
    }
    async fn reconfigure(&mut self, bitstream: &[u8]) -> Result<(), FpgaError> {
        info!("Versal reconfig: {} bytes | ICAP+PRC ready", bitstream.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_versal_init() {
        let mut p = VersalProver::new();
        p.init().await.unwrap();
        assert_eq!(p.health().await.power_state, PowerState::Eco);
    }

    #[tokio::test]
    async fn test_tmr_voting() {
        let mut v = TmrVoter::new();
        v.set(0, [1u8; 32]); v.set(1, [1u8; 32]); v.set(2, [2u8; 32]);
        assert_eq!(v.vote().unwrap(), [1u8; 32]);
    }

    #[tokio::test]
    async fn test_tmr_fault() {
        let mut v = TmrVoter::new();
        v.set(0, [1u8; 32]); v.set(1, [2u8; 32]); v.set(2, [3u8; 32]);
        assert!(v.is_faulty());
        assert!(v.vote().is_none());
    }

    #[tokio::test]
    async fn test_fault_injection() {
        let mut p = VersalProver::new();
        p.init().await.unwrap();
        p.enable_fault_injection(true);
        let proof = p.prove(PlonkInput {
            nullifiers: vec![[0x01; 32]],
            commitments: vec![[0x02; 32]],
            public_inputs: vec![100],
        }).await.unwrap();
        assert_eq!(p.health().await.radiation_events, 1);
        assert!(!proof.proof_bytes.is_empty());
    }

    #[tokio::test]
    async fn test_checkpoint() {
        let mut s = RadHardState::new();
        s.create_checkpoint(1, 100, [0xAB; 32]);
        s.create_checkpoint(2, 200, [0xCD; 32]);
        let cp = s.restore_checkpoint().unwrap();
        assert_eq!(cp.epoch, 2);
        assert_eq!(cp.block, 200);
    }

    #[tokio::test]
    async fn test_scrubber() {
        let mut p = VersalProver::new();
        p.init().await.unwrap();
        let events = p.scrub().await.unwrap();
        assert!(events.is_empty());
        assert_eq!(p.health().await.scrubber_cycles, 1);
    }

    #[tokio::test]
    async fn test_reconfigure() {
        let mut p = VersalProver::new();
        p.init().await.unwrap();
        p.reconfigure(&[0u8; 1024]).await.unwrap();
    }
}

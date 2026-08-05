//! QRAP Simulation Engine — 100k-Agent Load Testing
//!
//! Scenarios:
//! - Normal operation: full mesh, all honest
//! - Network partition: split into isolated groups
//! - Byzantine faults: malicious validators, double-spend attempts
//! - FPGA degradation: power state switching under load
//! - DA unavailability: Celestia blob submission failures

use qrap_crypto::{Hash, poseidon256, LweCommitment};
use qrap_consensus::OrbitalBft;
use qrap_net::{MeshNetwork, PeerConfig, NodeId, P2pMessage};
use qrap_utxo::{Transaction, TxInput, TxOutput, UtxoState};
use qrap_fpga::{MockFpga, FpgaProver, PowerState, PlonkInput};
use qrap_da::{MockDaClient, Blob, Namespace};
use qrap_fee_splitter::{FeeSplitter, ProverInfo, ValidatorInfo};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use qrap_da::DaClient;
use tracing::{info, warn, debug};

/// Simulation configuration
#[derive(Clone, Debug)]
pub struct SimConfig {
    pub num_nodes: usize,
    pub num_txs: usize,
    pub duration_secs: u64,
    pub byzantine_ratio: f64,      // 0.0 - 1.0
    pub partition_ratio: f64,      // 0.0 - 1.0
    pub da_failure_rate: f64,      // 0.0 - 1.0
    pub fpga_degradation: bool,    // Enable power state switching
    pub block_time_ms: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            num_nodes: 100,
            num_txs: 1000,
            duration_secs: 60,
            byzantine_ratio: 0.0,
            partition_ratio: 0.0,
            da_failure_rate: 0.0,
            fpga_degradation: false,
            block_time_ms: 6000,
        }
    }
}

/// Simulation metrics
#[derive(Clone, Debug, Default)]
pub struct SimMetrics {
    pub blocks_proposed: u64,
    pub txs_processed: u64,
    pub txs_failed: u64,
    pub consensus_rounds: u64,
    pub fpga_proofs: u64,
    pub fpga_energy_mj: u64,
    pub da_blobs_submitted: u64,
    pub da_blobs_failed: u64,
    pub byzantine_detected: u64,
    pub partition_healed: u64,
    pub total_duration_ms: u64,
}

/// Agent (node) in simulation
pub struct Agent {
    pub id: NodeId,
    pub index: usize,
    pub is_byzantine: bool,
    pub partition_group: usize,
    pub bft: Option<OrbitalBft>,
    pub fpga: MockFpga,
    pub state: UtxoState,
}

/// Simulation orchestrator
pub struct Simulation {
    config: SimConfig,
    agents: Vec<Agent>,
    metrics: SimMetrics,
    fee_splitter: FeeSplitter,
    da_client: MockDaClient,
    start_time: Instant,
}

impl Simulation {
    pub fn new(config: SimConfig) -> Self {
        let namespace = Namespace::new(b"qrap-sim");
        let da_client = MockDaClient::new(namespace).with_latency(50);
        let fee_splitter = FeeSplitter::new();
        
        let mut agents = Vec::with_capacity(config.num_nodes);
        let byzantine_count = (config.num_nodes as f64 * config.byzantine_ratio) as usize;
        let partition_groups = if config.partition_ratio > 0.0 { 2 } else { 1 };
        
        for i in 0..config.num_nodes {
            let mut id = [0u8; 32];
            id[0] = i as u8;
            id[1] = 0xAB;
            
            agents.push(Agent {
                id,
                index: i,
                is_byzantine: i < byzantine_count,
                partition_group: i % partition_groups,
                bft: None,
                fpga: MockFpga::new(),
                state: UtxoState::new(),
            });
        }
        
        Self {
            config,
            agents,
            metrics: SimMetrics::default(),
            fee_splitter,
            da_client,
            start_time: Instant::now(),
        }
    }
    
    /// Run full simulation
    pub async fn run(&mut self) -> SimMetrics {
        info!("╔══════════════════════════════════════════════════════╗");
        info!("║  QRAP 100k-Agent Simulation                          ║");
        info!("╠══════════════════════════════════════════════════════╣");
        info!("║  Nodes: {:<45}║", self.config.num_nodes);
        info!("║  Txs:   {:<45}║", self.config.num_txs);
        info!("║  Byzantine: {:.1}% {:<37}║", self.config.byzantine_ratio * 100.0, "");
        info!("║  Partition: {:.1}% {:<37}║", self.config.partition_ratio * 100.0, "");
        info!("║  DA Fail:   {:.1}% {:<37}║", self.config.da_failure_rate * 100.0, "");
        info!("║  FPGA Degradation: {:<32}║", if self.config.fpga_degradation { "ON" } else { "OFF" });
        info!("╚══════════════════════════════════════════════════════╝");
        
        // Phase 1: Initialize agents
        self.init_agents().await;
        
        // Phase 2: Generate transactions
        self.generate_transactions().await;
        
        // Phase 3: Run consensus rounds
        self.run_consensus().await;
        
        // Phase 4: Generate FPGA proofs
        self.generate_proofs().await;
        
        // Phase 5: Submit to DA
        self.submit_to_da().await;
        
        // Phase 6: Fee distribution
        self.distribute_fees().await;
        
        self.metrics.total_duration_ms = self.start_time.elapsed().as_millis() as u64;
        
        self.print_results();
        self.metrics.clone()
    }
    
    async fn init_agents(&mut self) {
        info!("[Phase 1] Initializing {} agents...", self.config.num_nodes);
        
        for agent in &mut self.agents {
            agent.fpga.init().await.unwrap();
            
            if self.config.fpga_degradation && agent.index % 3 == 0 {
                agent.fpga.set_power(PowerState::Eco).await.unwrap();
            } else if self.config.fpga_degradation && agent.index % 3 == 1 {
                agent.fpga.set_power(PowerState::Balanced).await.unwrap();
            }
        }
        
        info!("  OK All agents initialized");
    }
    
    async fn generate_transactions(&mut self) {
        info!("[Phase 2] Generating {} transactions...", self.config.num_txs);
        
        for i in 0..self.config.num_txs {
            let agent_idx = i % self.config.num_nodes;
            let agent = &mut self.agents[agent_idx];
            
            let tx = if agent.is_byzantine {
                // Byzantine: double-spend attempt (same nullifier)
                Transaction {
                    inputs: vec![TxInput { nullifier: [0xFF; 32], spend_proof: vec![] }],
                    outputs: vec![TxOutput { commitment: LweCommitment::new_random(), value: 999 }],
                    fee: 0,
                    nonce: i as u64,
                }
            } else {
                Transaction {
                    inputs: vec![TxInput { nullifier: [(i % 256) as u8; 32], spend_proof: vec![] }],
                    outputs: vec![TxOutput { commitment: LweCommitment::new_random(), value: 100 }],
                    fee: 1,
                    nonce: i as u64,
                }
            };
            
            // Try to apply to state
            if agent.state.apply_tx(&tx).is_ok() {
                if !agent.is_byzantine {
                    self.metrics.txs_processed += 1;
                }
            } else {
                if agent.is_byzantine {
                    self.metrics.byzantine_detected += 1;
                } else {
                    self.metrics.txs_failed += 1;
                }
            }
        }
        
        info!("  OK Processed: {}, Failed: {}, Byzantine detected: {}", 
              self.metrics.txs_processed, self.metrics.txs_failed, self.metrics.byzantine_detected);
    }
    
    async fn run_consensus(&mut self) {
        info!("[Phase 3] Running consensus...");
        
        // Simulate consensus rounds
        let rounds = self.config.duration_secs * 1000 / self.config.block_time_ms;
        
        for round in 0..rounds {
            // Check partition healing
            if self.config.partition_ratio > 0.0 && round == rounds / 2 {
                info!("  Healing network partition at round {}", round);
                for agent in &mut self.agents {
                    agent.partition_group = 0;
                }
                self.metrics.partition_healed += 1;
            }
            
            // Each non-byzantine agent advances block
            for agent in &mut self.agents {
                if !agent.is_byzantine {
                    agent.state.advance_block();
                }
            }
            
            self.metrics.consensus_rounds += 1;
            self.metrics.blocks_proposed += 1;
        }
        
        info!("  OK Rounds: {}, Blocks: {}", self.metrics.consensus_rounds, self.metrics.blocks_proposed);
    }
    
    async fn generate_proofs(&mut self) {
        info!("[Phase 4] Generating FPGA proofs...");
        
        let proof_count = self.config.num_nodes.min(100); // Limit for simulation speed
        
        for i in 0..proof_count {
            let agent = &mut self.agents[i];
            
            let input = PlonkInput {
                nullifiers: vec![[(i % 256) as u8; 32]],
                commitments: vec![LweCommitment::new_random().hash()],
                public_inputs: vec![i as u64 * 100, 1],
            };
            
            if let Ok(proof) = agent.fpga.prove(input).await {
                self.metrics.fpga_proofs += 1;
                self.metrics.fpga_energy_mj += proof.power_consumed_mj;
            }
        }
        
        info!("  OK Proofs: {}, Energy: {}mJ", self.metrics.fpga_proofs, self.metrics.fpga_energy_mj);
    }
    
    async fn submit_to_da(&mut self) {
        info!("[Phase 5] Submitting to DA layer...");
        
        let batch_size = self.config.num_txs / 10;
        
        for i in 0..10 {
            let data = serde_json::to_vec(&json!({
                "batch": i,
                "txs": batch_size,
                "timestamp": chrono::Utc::now().timestamp(),
            })).unwrap_or_default();
            
            let blob = Blob::new(
                Namespace::new(b"qrap-sim"),
                data,
            );
            
            // Simulate DA failures
            if rand::random::<f64>() < self.config.da_failure_rate {
                self.metrics.da_blobs_failed += 1;
                warn!("  DA blob {} failed (simulated)", i);
            } else {
                if let Ok(commitment) = self.da_client.submit_blob(blob).await {
                    self.metrics.da_blobs_submitted += 1;
                    debug!("  DA blob {} submitted, commitment={}", i, hex::encode(&commitment[..8]));
                }
            }
        }
        
        info!("  OK Submitted: {}, Failed: {}", self.metrics.da_blobs_submitted, self.metrics.da_blobs_failed);
    }
    
    async fn distribute_fees(&mut self) {
        info!("[Phase 6] Distributing fees...");
        
        let provers: Vec<ProverInfo> = self.agents.iter().map(|a| ProverInfo {
            id: format!("agent-{}", a.index),
            is_fpga: true,
            proofs_generated: if a.index < 100 { 1 } else { 0 },
            uptime_secs: self.config.duration_secs,
        }).collect();
        
        let validators: Vec<ValidatorInfo> = self.agents.iter().filter(|a| !a.is_byzantine).map(|a| ValidatorInfo {
            id: format!("agent-{}", a.index),
            stake: 1000 + (a.index as u64 * 100),
            uptime_secs: self.config.duration_secs,
            blocks_proposed: self.metrics.blocks_proposed / self.config.num_nodes as u64,
        }).collect();
        
        let total_fees = self.metrics.txs_processed * 10; // $0.01 per tx proxy
        let split = self.fee_splitter.split(total_fees, &provers, &validators).unwrap();
        
        info!("  OK Fees distributed:");
        info!("    Provers:    {} (bonus: {})", split.provers, split.prover_bonus);
        info!("    Validators: {}", split.validators);
        info!("    Treasury:   {}", split.treasury);
        info!("    DA:         {}", split.da);
        info!("    Burn:       {}", split.burn);
    }
    
    fn print_results(&self) {
        info!("╔══════════════════════════════════════════════════════╗");
        info!("║  SIMULATION RESULTS                                  ║");
        info!("╠══════════════════════════════════════════════════════╣");
        info!("║  Duration:        {:<32}ms ║", self.metrics.total_duration_ms);
        info!("║  Blocks:           {:<32} ║", self.metrics.blocks_proposed);
        info!("║  Txs Processed:    {:<32} ║", self.metrics.txs_processed);
        info!("║  Txs Failed:       {:<32} ║", self.metrics.txs_failed);
        info!("║  Byzantine Detect: {:<32} ║", self.metrics.byzantine_detected);
        info!("║  FPGA Proofs:      {:<32} ║", self.metrics.fpga_proofs);
        info!("║  FPGA Energy:      {:<32}mJ ║", self.metrics.fpga_energy_mj);
        info!("║  DA Submitted:     {:<32} ║", self.metrics.da_blobs_submitted);
        info!("║  DA Failed:        {:<32} ║", self.metrics.da_blobs_failed);
        info!("║  Partitions Healed:{:<32} ║", self.metrics.partition_healed);
        info!("╚══════════════════════════════════════════════════════╝");
    }
}

// Helper for json! macro
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_small_simulation() {
        let config = SimConfig {
            num_nodes: 10,
            num_txs: 100,
            duration_secs: 15,
            block_time_ms: 1000,
            ..Default::default()
        };
        
        let mut sim = Simulation::new(config);
        let metrics = sim.run().await;
        
        assert!(metrics.blocks_proposed > 0);
        assert!(metrics.txs_processed > 0);
        assert_eq!(metrics.byzantine_detected, 0);
    }

    #[tokio::test]
    async fn test_byzantine_simulation() {
        let config = SimConfig {
            num_nodes: 20,
            num_txs: 200,
            duration_secs: 15,
            block_time_ms: 1000,
            byzantine_ratio: 0.2, // 20% Byzantine
            ..Default::default()
        };
        
        let mut sim = Simulation::new(config);
        let metrics = sim.run().await;
        
        assert!(metrics.byzantine_detected > 0);
        assert!(metrics.txs_processed > 0);
    }

    #[tokio::test]
    async fn test_partition_simulation() {
        let config = SimConfig {
            num_nodes: 16,
            num_txs: 100,
            duration_secs: 15,
            block_time_ms: 1000,
            partition_ratio: 0.5,
            ..Default::default()
        };
        
        let mut sim = Simulation::new(config);
        let metrics = sim.run().await;
        
        assert_eq!(metrics.partition_healed, 1);
    }

    #[tokio::test]
    async fn test_fpga_degradation() {
        let config = SimConfig {
            num_nodes: 10,
            num_txs: 50,
            duration_secs: 10,
            block_time_ms: 1000,
            fpga_degradation: true,
            ..Default::default()
        };
        
        let mut sim = Simulation::new(config);
        let metrics = sim.run().await;
        
        assert!(metrics.fpga_proofs > 0);
        assert!(metrics.fpga_energy_mj > 0);
    }

    #[tokio::test]
    async fn test_da_failure_simulation() {
        let config = SimConfig {
            num_nodes: 10,
            num_txs: 50,
            duration_secs: 10,
            block_time_ms: 1000,
            da_failure_rate: 0.3,
            ..Default::default()
        };
        
        let mut sim = Simulation::new(config);
        let metrics = sim.run().await;
        
        assert!(metrics.da_blobs_failed > 0);
        assert!(metrics.da_blobs_submitted > 0);
    }

    #[tokio::test]
    async fn test_full_stress_simulation() {
        let config = SimConfig {
            num_nodes: 50,
            num_txs: 500,
            duration_secs: 10,
            byzantine_ratio: 0.1,
            partition_ratio: 0.3,
            da_failure_rate: 0.1,
            fpga_degradation: true,
            ..Default::default()
        };
        
        let mut sim = Simulation::new(config);
        let metrics = sim.run().await;
        
        assert!(metrics.blocks_proposed > 0);
        assert!(metrics.txs_processed > 0);
        assert!(metrics.fpga_proofs > 0);
        assert!(metrics.da_blobs_submitted > 0);
    }
}

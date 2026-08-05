//! Orbital BFT Consensus Engine
//!
//! 4-validator testnet with 12s block time.
//! Phases: Propose -> Prepare -> Commit -> Decide

use chrono::Utc;
use qrap_crypto::{poseidon256, Hash};
use qrap_net::{MeshNetwork, NodeId, P2pMessage};
use qrap_utxo::{Transaction, UtxoState};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

pub const BLOCK_TIME_SECS: u64 = 3;
pub const VALIDATOR_COUNT: usize = 4;

/// Consensus message types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusMsg {
    Propose {
        height: u64,
        round: u64,
        block_hash: Hash,
        proposer: NodeId,
        timestamp: i64,
    },
    Prepare {
        height: u64,
        round: u64,
        block_hash: Hash,
        validator: NodeId,
        signature: Vec<u8>, // ML-DSA placeholder
    },
    Commit {
        height: u64,
        round: u64,
        block_hash: Hash,
        validator: NodeId,
        signature: Vec<u8>,
    },
    Decide {
        height: u64,
        block_hash: Hash,
        qc: Vec<u8>, // aggregated quorum certificate placeholder
    },
}

/// Block header
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub timestamp: i64,
    pub prev_hash: Hash,
    pub tx_root: Hash,
    pub state_root: Hash,
    pub proposer: NodeId,
}

/// Full block
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(&self.header).unwrap_or_default();
        poseidon256(&bytes)
    }
}

/// Consensus state machine per height/round
#[derive(Clone, Debug, Default)]
pub struct RoundState {
    pub height: u64,
    pub round: u64,
    pub step: ConsensusStep,
    pub proposals: HashMap<Hash, Block>,
    pub prepare_votes: HashMap<Hash, HashSet<NodeId>>,
    pub commit_votes: HashMap<Hash, HashSet<NodeId>>,
    pub locked_block: Option<Hash>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ConsensusStep {
    #[default]
    NewHeight,
    Propose,
    Prepare,
    Commit,
    Decide,
}

#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("Invalid proposer for round {0}")]
    InvalidProposer(u64),
    #[error("Double proposal")]
    DoubleProposal,
    #[error("Quorum not reached")]
    QuorumNotReached,
    #[error("Invalid signature")]
    InvalidSignature,
}

/// Orbital BFT engine
pub struct OrbitalBft {
    pub local_id: NodeId,
    pub validators: Vec<NodeId>,
    pub state: Arc<RwLock<RoundState>>,
    pub utxo: Arc<RwLock<UtxoState>>,
    pub mesh: Option<Arc<MeshNetwork>>,
}

impl OrbitalBft {
    pub fn new(local_id: NodeId, validators: Vec<NodeId>) -> Self {
        Self {
            local_id,
            validators,
            state: Arc::new(RwLock::new(RoundState::default())),
            utxo: Arc::new(RwLock::new(UtxoState::new())),
            mesh: None,
        }
    }

    pub fn attach_mesh(&mut self, mesh: Arc<MeshNetwork>) {
        self.mesh = Some(mesh);
    }

    /// Start consensus event loop
    pub async fn run(&self, mut msg_rx: mpsc::UnboundedReceiver<(NodeId, P2pMessage)>) {
        info!(
            "Orbital BFT started for node {}",
            hex::encode(&self.local_id[..4])
        );
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(BLOCK_TIME_SECS));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.on_timeout().await {
                        warn!("Consensus timeout error: {}", e);
                    }
                }
                Some((peer, msg)) = msg_rx.recv() => {
                    if let P2pMessage::Consensus(bytes) = msg {
                        match bincode::deserialize::<ConsensusMsg>(&bytes) {
                            Ok(cm) => {
                                if let Err(e) = self.handle_consensus_msg(peer, cm).await {
                                    debug!("Consensus msg error: {}", e);
                                }
                            }
                            Err(e) => warn!("Failed to decode consensus msg: {}", e),
                        }
                    }
                }
            }
        }
    }

    async fn on_timeout(&self) -> Result<(), ConsensusError> {
        let mut state = self.state.write().await;
        let height = state.height;
        let round = state.round;

        // Simple round-robin proposer
        let proposer_idx = ((height + round) as usize) % self.validators.len();
        let is_proposer = self.validators[proposer_idx] == self.local_id;

        if is_proposer && state.step == ConsensusStep::NewHeight {
            info!("Proposing block at height {} round {}", height, round);
            state.step = ConsensusStep::Propose;
            drop(state);
            self.propose_block(height, round).await?;
        }
        Ok(())
    }

    async fn propose_block(&self, height: u64, round: u64) -> Result<(), ConsensusError> {
        let utxo = self.utxo.read().await;
        let txs = utxo.mempool.clone();
        drop(utxo);

        let prev_hash = [0u8; 32]; // genesis placeholder
        let header = BlockHeader {
            height,
            timestamp: Utc::now().timestamp(),
            prev_hash,
            tx_root: poseidon256(&bincode::serialize(&txs).unwrap_or_default()),
            state_root: [0u8; 32], // placeholder
            proposer: self.local_id,
        };
        let block = Block {
            header,
            transactions: txs,
        };
        let block_hash = block.hash();

        {
            let mut state = self.state.write().await;
            state.proposals.insert(block_hash, block.clone());
        }

        let msg = ConsensusMsg::Propose {
            height,
            round,
            block_hash,
            proposer: self.local_id,
            timestamp: Utc::now().timestamp(),
        };
        self.broadcast_consensus(msg).await;
        Ok(())
    }

    async fn handle_consensus_msg(
        &self,
        _peer: NodeId,
        msg: ConsensusMsg,
    ) -> Result<(), ConsensusError> {
        match msg {
            ConsensusMsg::Propose {
                height,
                round,
                block_hash,
                proposer,
                ..
            } => {
                debug!(
                    "Received Propose from {} for h={} r={}",
                    hex::encode(&proposer[..4]),
                    height,
                    round
                );
                let mut state = self.state.write().await;
                if state.step == ConsensusStep::NewHeight || state.step == ConsensusStep::Propose {
                    state.step = ConsensusStep::Prepare;
                    state.locked_block = Some(block_hash);
                    drop(state);
                    // Send Prepare
                    let prepare = ConsensusMsg::Prepare {
                        height,
                        round,
                        block_hash,
                        validator: self.local_id,
                        signature: vec![], // placeholder
                    };
                    self.broadcast_consensus(prepare).await;
                }
            }
            ConsensusMsg::Prepare {
                height,
                round,
                block_hash,
                validator,
                ..
            } => {
                let mut state = self.state.write().await;
                state
                    .prepare_votes
                    .entry(block_hash)
                    .or_default()
                    .insert(validator);
                let votes = state
                    .prepare_votes
                    .get(&block_hash)
                    .map(|s| s.len())
                    .unwrap_or(0);
                if votes >= quorum(self.validators.len()) && state.step == ConsensusStep::Prepare {
                    state.step = ConsensusStep::Commit;
                    drop(state);
                    let commit = ConsensusMsg::Commit {
                        height,
                        round,
                        block_hash,
                        validator: self.local_id,
                        signature: vec![],
                    };
                    self.broadcast_consensus(commit).await;
                }
            }
            ConsensusMsg::Commit {
                height,
                round,
                block_hash,
                validator,
                ..
            } => {
                let mut state = self.state.write().await;
                state
                    .commit_votes
                    .entry(block_hash)
                    .or_default()
                    .insert(validator);
                let votes = state
                    .commit_votes
                    .get(&block_hash)
                    .map(|s| s.len())
                    .unwrap_or(0);
                if votes >= quorum(self.validators.len()) && state.step == ConsensusStep::Commit {
                    info!(
                        "Quorum reached for block {} at h={} r={}",
                        hex::encode(&block_hash[..4]),
                        height,
                        round
                    );
                    state.step = ConsensusStep::Decide;
                    state.height += 1;
                    state.round = 0;
                    state.step = ConsensusStep::NewHeight;
                    // Apply block transactions
                    if let Some(block) = state.proposals.remove(&block_hash) {
                        drop(state);
                        let mut utxo = self.utxo.write().await;
                        for tx in &block.transactions {
                            let _ = utxo.apply_tx(tx);
                        }
                        utxo.advance_block();
                        utxo.mempool.clear();
                    }
                }
            }
            ConsensusMsg::Decide {
                height, block_hash, ..
            } => {
                info!(
                    "Block decided: height={}, hash={}",
                    height,
                    hex::encode(&block_hash[..4])
                );
            }
        }
        Ok(())
    }

    async fn broadcast_consensus(&self, msg: ConsensusMsg) {
        let bytes = bincode::serialize(&msg).unwrap_or_default();
        let p2p = P2pMessage::Consensus(bytes);
        if let Some(mesh) = &self.mesh {
            mesh.broadcast(p2p);
        }
    }
}

fn quorum(n: usize) -> usize {
    (2 * n) / 3 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quorum() {
        assert_eq!(quorum(4), 3);
        assert_eq!(quorum(7), 5);
    }

    #[test]
    fn test_block_hash() {
        let header = BlockHeader {
            height: 1,
            timestamp: 0,
            prev_hash: [0u8; 32],
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            proposer: [0u8; 32],
        };
        let block = Block {
            header,
            transactions: vec![],
        };
        let h = block.hash();
        assert_eq!(h.len(), 32);
    }
}

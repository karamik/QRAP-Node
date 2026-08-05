//! QRAP UTXO Engine + Epoch Nullifier Trees
//!
//! Stateless UTXO model with Ring-LWE commitments and epoch-based
//! nullifier pruning for bounded storage.

use qrap_crypto::{poseidon256, poseidon256_pair, Hash, LweCommitment};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use tracing::{debug, info};

pub const EPOCH_LENGTH: u64 = 100; // blocks per epoch

/// Transaction output (UTXO)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Utxo {
    pub commitment: LweCommitment,
    pub value: u64,
    pub epoch: u64,
}

/// Transaction input (spends a UTXO)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxInput {
    pub nullifier: Hash,      // prevents double-spend
    pub spend_proof: Vec<u8>, // ZK-STARK placeholder
}

/// Transaction output
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOutput {
    pub commitment: LweCommitment,
    pub value: u64,
}

/// QRAP transaction
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub fee: u64,
    pub nonce: u64,
}

impl Transaction {
    pub fn hash(&self) -> Hash {
        let bytes = bincode::serialize(self).unwrap_or_default();
        poseidon256(&bytes)
    }
}

/// Sparse Merkle Tree node
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SmtNode {
    pub left: Option<Hash>,
    pub right: Option<Hash>,
    pub value: Option<Hash>,
}

/// Sparse Merkle Tree (256-depth, truncated for storage)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SparseMerkleTree {
    pub root: Hash,
    nodes: HashMap<Hash, SmtNode>,
    leaves: HashMap<Hash, Hash>, // path -> leaf hash
}

impl Default for SparseMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        Self {
            root: [0u8; 32],
            nodes: HashMap::new(),
            leaves: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: &Hash, value: &Hash) {
        self.leaves.insert(*key, *value);
        self.recompute_root();
    }

    pub fn get(&self, key: &Hash) -> Option<&Hash> {
        self.leaves.get(key)
    }

    fn recompute_root(&mut self) {
        let mut keys: Vec<_> = self.leaves.keys().collect();
        keys.sort();
        let mut current: Vec<Hash> = keys.iter().map(|k| **k).collect();
        while current.len() > 1 {
            let mut next = Vec::new();
            for chunk in current.chunks(2) {
                if chunk.len() == 2 {
                    next.push(poseidon256_pair(&chunk[0], &chunk[1]));
                } else {
                    next.push(chunk[0]);
                }
            }
            current = next;
        }
        self.root = current.first().copied().unwrap_or([0u8; 32]);
    }
}

/// Epoch state: nullifier tree + UTXO set snapshot
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochState {
    pub epoch_number: u64,
    pub nullifier_tree: SparseMerkleTree,
    pub spent_nullifiers: HashSet<Hash>,
    pub utxo_commitments: Vec<Hash>,
}

impl EpochState {
    pub fn new(epoch: u64) -> Self {
        Self {
            epoch_number: epoch,
            nullifier_tree: SparseMerkleTree::new(),
            spent_nullifiers: HashSet::new(),
            utxo_commitments: Vec::new(),
        }
    }

    pub fn spend(&mut self, nullifier: &Hash) -> Result<(), UtxoError> {
        if self.spent_nullifiers.contains(nullifier) {
            return Err(UtxoError::DoubleSpend);
        }
        self.spent_nullifiers.insert(*nullifier);
        self.nullifier_tree.insert(nullifier, &[0xff; 32]);
        Ok(())
    }

    pub fn root(&self) -> Hash {
        self.nullifier_tree.root
    }
}

#[derive(Debug, Error)]
pub enum UtxoError {
    #[error("Double spend detected")]
    DoubleSpend,
    #[error("Invalid commitment")]
    InvalidCommitment,
    #[error("Epoch mismatch")]
    EpochMismatch,
    #[error("Storage error: {0}")]
    Storage(String),
}

/// UTXO state manager across epochs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UtxoState {
    pub current_epoch: u64,
    pub current_block: u64,
    pub epochs: HashMap<u64, EpochState>,
    pub mempool: Vec<Transaction>,
    #[serde(skip)]
    db_path: Option<String>,
}

impl Default for UtxoState {
    fn default() -> Self {
        Self::new()
    }
}

impl UtxoState {
    pub fn new() -> Self {
        let mut epochs = HashMap::new();
        epochs.insert(0, EpochState::new(0));
        Self {
            current_epoch: 0,
            current_block: 0,
            epochs,
            mempool: Vec::new(),
            db_path: None,
        }
    }

    pub fn with_storage(path: &str) -> Result<Self, UtxoError> {
        let db = sled::open(path).map_err(|e| UtxoError::Storage(e.to_string()))?;
        if let Some(bytes) = db
            .get("state")
            .map_err(|e| UtxoError::Storage(e.to_string()))?
        {
            let mut state: UtxoState =
                bincode::deserialize(&bytes).map_err(|e| UtxoError::Storage(e.to_string()))?;
            state.db_path = Some(path.to_string());
            Ok(state)
        } else {
            let mut state = Self::new();
            state.db_path = Some(path.to_string());
            Ok(state)
        }
    }

    pub fn apply_tx(&mut self, tx: &Transaction) -> Result<(), UtxoError> {
        let epoch = self
            .epochs
            .get_mut(&self.current_epoch)
            .ok_or(UtxoError::EpochMismatch)?;
        for input in &tx.inputs {
            epoch.spend(&input.nullifier)?;
        }
        for output in &tx.outputs {
            epoch.utxo_commitments.push(output.commitment.hash());
        }
        debug!(
            "Applied tx {}, epoch {}",
            hex::encode(&tx.hash()[..4]),
            self.current_epoch
        );
        Ok(())
    }

    pub fn advance_block(&mut self) {
        self.current_block += 1;
        let new_epoch = self.current_block / EPOCH_LENGTH;
        if new_epoch != self.current_epoch {
            info!("Rolling over to epoch {}", new_epoch);
            self.current_epoch = new_epoch;
            self.epochs.insert(new_epoch, EpochState::new(new_epoch));
            let to_remove: Vec<_> = self
                .epochs
                .keys()
                .filter(|&&e| e + 3 < new_epoch)
                .copied()
                .collect();
            for e in to_remove {
                self.epochs.remove(&e);
            }
        }
    }

    pub fn add_to_mempool(&mut self, tx: Transaction) {
        self.mempool.push(tx);
    }

    pub fn flush(&self) -> Result<(), UtxoError> {
        if let Some(ref path) = self.db_path {
            let db = sled::open(path).map_err(|e| UtxoError::Storage(e.to_string()))?;
            let bytes = bincode::serialize(self).map_err(|e| UtxoError::Storage(e.to_string()))?;
            db.insert("state", bytes)
                .map_err(|e| UtxoError::Storage(e.to_string()))?;
            db.flush().map_err(|e| UtxoError::Storage(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utxo_spend() {
        let mut state = UtxoState::new();
        let nf = [0x01; 32];
        let tx = Transaction {
            inputs: vec![TxInput {
                nullifier: nf,
                spend_proof: vec![],
            }],
            outputs: vec![TxOutput {
                commitment: LweCommitment::new_random(),
                value: 100,
            }],
            fee: 1,
            nonce: 1,
        };
        assert!(state.apply_tx(&tx).is_ok());
        assert!(state.apply_tx(&tx).is_err());
    }

    #[test]
    fn test_epoch_rollover() {
        let mut state = UtxoState::new();
        for _ in 0..EPOCH_LENGTH + 1 {
            state.advance_block();
        }
        assert_eq!(state.current_epoch, 1);
    }
}

#[test]
fn test_sled_persistence() {
    use std::fs;
    let tmp = format!(
        "{}/qrap_utxo_test_{}",
        std::env::temp_dir().to_string_lossy(),
        rand::random::<u64>()
    );
    let _ = fs::remove_dir_all(&tmp);

    // Phase 1: create, mutate, flush
    {
        let mut state = UtxoState::with_storage(&tmp).unwrap();
        let tx = Transaction {
            inputs: vec![TxInput {
                nullifier: [0xAB; 32],
                spend_proof: vec![],
            }],
            outputs: vec![TxOutput {
                commitment: LweCommitment::new_random(),
                value: 50,
            }],
            fee: 1,
            nonce: 2,
        };
        state.apply_tx(&tx).unwrap();
        state.advance_block();
        state.add_to_mempool(tx);
        state.flush().unwrap();
    } // sled db closed here

    // Phase 2: reopen and verify crash recovery
    {
        let state = UtxoState::with_storage(&tmp).unwrap();
        assert_eq!(state.current_block, 1);
        assert_eq!(state.mempool.len(), 1);
        let epoch = state.epochs.get(&0).unwrap();
        assert!(epoch.spent_nullifiers.contains(&[0xAB; 32]));
    }

    let _ = fs::remove_dir_all(&tmp);
}

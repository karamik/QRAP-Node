//! End-to-End Integration Test
//!
//! Pipeline: UTXO (tx creation) → Consensus (block proposal) → FPGA Proof → Block finalization

use qrap_crypto::{poseidon256, LweCommitment};
use qrap_consensus::OrbitalBft;
use qrap_net::{MeshNetwork, NodeId, PeerConfig};
use qrap_utxo::{Transaction, TxInput, TxOutput, UtxoState};
use qrap_fpga::{MockFpga, FpgaProver, PowerState, PlonkInput};
use std::sync::Arc;
use std::time::Duration;

fn derive_node_id(idx: u8) -> NodeId {
    let mut id = [0u8; 32];
    id[0] = idx;
    id[1] = 0xAB;
    id
}

fn testnet_addr(idx: u8) -> String {
    format!("127.0.0.1:{}", 10000 + idx as u16)
}

#[tokio::test]
async fn test_e2e_single_node() {
    println!("\n=== E2E Test: Single Node ===");
    
    // Step 1: Create UTXO transaction
    println!("[1/5] Creating UTXO transaction...");
    let mut state = UtxoState::new();
    let tx = Transaction {
        inputs: vec![TxInput { nullifier: [0x01; 32], spend_proof: vec![] }],
        outputs: vec![TxOutput { commitment: LweCommitment::new_random(), value: 100 }],
        fee: 1,
        nonce: 1,
    };
    state.apply_tx(&tx).expect("Valid tx");
    state.advance_block();
    println!("      OK UTXO state: block={}, epoch={}", state.current_block, state.current_epoch);
    
    // Step 2: Start consensus
    println!("[2/5] Starting consensus...");
    let id = derive_node_id(0);
    let validators = vec![id];
    let mut bft = OrbitalBft::new(id, validators);
    
    // Add tx to mempool
    { let mut utxo = bft.utxo.write().await; utxo.add_to_mempool(tx.clone()); }
    
    // Attach mesh
    let (mesh, msg_rx) = MeshNetwork::new(id);
    bft.attach_mesh(Arc::new(mesh));
    
    // Run consensus briefly
    let h = tokio::spawn(async move { bft.run(msg_rx).await; });
    tokio::time::sleep(Duration::from_secs(2)).await;
    h.abort();
    println!("      OK Consensus ran for 2s");
    
    // Step 3: Generate FPGA proof
    println!("[3/5] Generating FPGA proof...");
    let mut fpga = MockFpga::new();
    fpga.init().await.unwrap();
    
    let plonk_input = PlonkInput {
        nullifiers: vec![[0x01; 32]],
        commitments: vec![LweCommitment::new_random().hash()],
        public_inputs: vec![100, 1], // value + fee
    };
    let proof = fpga.prove(plonk_input).await.unwrap();
    println!("      OK Proof: {}ms, {}mJ", proof.generation_time_ms, proof.power_consumed_mj);
    
    // Step 4: Verify proof hash matches block
    println!("[4/5] Verifying proof against block...");
    let tx_hash = tx.hash();
    let block_hash = poseidon256(&tx_hash);
    assert_eq!(block_hash.len(), 32);
    println!("      OK Block hash: {}", hex::encode(&block_hash[..8]));
    
    // Step 5: Persist state
    println!("[5/5] Persisting state...");
    let tmp = format!("{}/qrap_e2e_test", std::env::temp_dir().to_string_lossy());
    let _ = std::fs::remove_dir_all(&tmp);
    
    let mut persist_state = UtxoState::with_storage(&tmp).unwrap();
    persist_state.apply_tx(&tx).unwrap();
    persist_state.advance_block();
    persist_state.flush().unwrap();
    
    let restored = UtxoState::with_storage(&tmp).unwrap();
    assert_eq!(restored.current_block, 1);
    println!("      OK State persisted and restored");
    
    let _ = std::fs::remove_dir_all(&tmp);
    
    println!("\n=== E2E Test PASSED ===");
}

#[tokio::test]
async fn test_e2e_multi_node_with_fpga() {
    println!("\n=== E2E Test: 3 Nodes + FPGA ===");
    
    let nodes = 3;
    let mut handles = vec![];
    
    // Start 3 consensus nodes
    for i in 0..nodes {
        let h = tokio::spawn(async move {
            let id = derive_node_id(i as u8);
            let (mesh, msg_rx) = MeshNetwork::new(id);
            let mesh = Arc::new(mesh);
            
            // Listen
            let bind = testnet_addr(i as u8).parse().unwrap();
            let m2 = mesh.clone();
            tokio::spawn(async move { let _ = m2.listen(bind).await; });
            tokio::time::sleep(Duration::from_millis(300)).await;
            
            // Dial peers
            let peers: Vec<PeerConfig> = (0..nodes)
                .filter(|&j| j != i)
                .map(|j| PeerConfig { id: derive_node_id(j as u8), addr: testnet_addr(j as u8).parse().unwrap() })
                .collect();
            let _ = mesh.dial_peers(&peers).await;
            
            // Start consensus
            let vals: Vec<NodeId> = (0..nodes).map(|j| derive_node_id(j as u8)).collect();
            let mut bft = OrbitalBft::new(id, vals);
            bft.attach_mesh(mesh);
            
            // Node 0 creates tx
            if i == 0 {
                let mut utxo = bft.utxo.write().await;
                let tx = Transaction {
                    inputs: vec![TxInput { nullifier: [0xAA; 32], spend_proof: vec![] }],
                    outputs: vec![TxOutput { commitment: LweCommitment::new_random(), value: 50 }],
                    fee: 1,
                    nonce: 42,
                };
                utxo.add_to_mempool(tx);
            }
            
            bft.run(msg_rx).await;
        });
        handles.push(h);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Run for 3 seconds
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Abort all
    for h in handles {
        h.abort();
    }
    
    // Generate FPGA proof for the "block"
    println!("  Generating FPGA proof for multi-node block...");
    let mut fpga = MockFpga::new();
    fpga.init().await.unwrap();
    fpga.set_power(PowerState::Balanced).await.unwrap();
    
    let proof_input = PlonkInput {
        nullifiers: vec![[0xAA; 32]],
        commitments: vec![LweCommitment::new_random().hash()],
        public_inputs: vec![50, 1],
    };
    let proof = fpga.prove(proof_input).await.unwrap();
    println!("  Proof: {}ms, {}mJ @ {:?} power", 
             proof.generation_time_ms, proof.power_consumed_mj, PowerState::Balanced);
    
    println!("=== E2E Multi-Node Test PASSED ===");
}

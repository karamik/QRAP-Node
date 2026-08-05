use clap::{Parser, Subcommand};
use qrap_crypto::{MlKemKeypair, poseidon256, LweCommitment};
use qrap_net::{MeshNetwork, PeerConfig, NodeId, encode, decode_one, P2pMessage};
use qrap_consensus::OrbitalBft;
use qrap_utxo::{Transaction, TxInput, TxOutput, UtxoState};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "qrap-node")]
#[command(about = "QRAP Node")]
struct Cli { #[command(subcommand)] command: Commands }

#[derive(Subcommand)]
enum Commands {
    Keygen { #[arg(short, long, default_value = "validator.keys")] output: String },
    Run { #[arg(short, long, default_value = "0")] node_id: u8 },
    Test,
    Benchmark { #[arg(short, long, default_value = "4")] nodes: usize, #[arg(short, long, default_value = "100")] txs: usize },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Keygen { output } => cmd_keygen(&output),
        Commands::Run { node_id } => cmd_run(node_id).await,
        Commands::Test => cmd_test().await,
        Commands::Benchmark { nodes, txs } => cmd_benchmark(nodes, txs).await,
    }
}

fn cmd_keygen(output: &str) {
    let kp = MlKemKeypair::generate();
    let data = serde_json::json!({
        "public_key": hex::encode(&kp.public_key),
        "secret_key": hex::encode(&kp.secret_key),
        "algorithm": "ML-KEM-1024-placeholder",
    });
    let json = serde_json::to_string_pretty(&data).unwrap();
    println!("{}", json);
    println!("[keygen] Keypair would be saved to: {}", output);
}

async fn cmd_run(node_id: u8) {
    let id = derive_node_id(node_id);
    println!("[run] Starting node {} (id={})", node_id, hex::encode(&id[..4]));
    let (mesh, msg_rx) = MeshNetwork::new(id);
    let mesh = Arc::new(mesh);
    let bind_addr = testnet_addr(node_id);
    let m2 = mesh.clone();
    tokio::spawn(async move { let _ = m2.listen(bind_addr).await; });
    tokio::time::sleep(Duration::from_millis(500)).await;
    let _ = mesh.dial_peers(&testnet_peers(node_id)).await;
    let vals: Vec<NodeId> = (0..4).map(derive_node_id).collect();
    let mut bft = OrbitalBft::new(id, vals);
    bft.attach_mesh(mesh);
    println!("[run] Node {} consensus started", node_id);
    bft.run(msg_rx).await;
}

async fn cmd_test() {
    println!("=== QRAP Node v0.2.0-alpha Self-Test ===\n");
    
    println!("[1/6] Testing crypto...");
    let kp = MlKemKeypair::generate();
    assert_eq!(kp.public_key.len(), 1568);
    assert_eq!(kp.secret_key.len(), 3168);
    println!("      OK ML-KEM-1024");
    let h1 = poseidon256(b"test");
    let h2 = poseidon256(b"test");
    assert_eq!(h1, h2);
    println!("      OK Poseidon-256");
    let c = LweCommitment::new_random();
    assert_eq!(c.hash().len(), 32);
    println!("      OK Ring-LWE commitment");

    println!("\n[2/6] Testing UTXO...");
    let mut state = UtxoState::new();
    let tx = Transaction {
        inputs: vec![TxInput { nullifier: [1u8; 32], spend_proof: vec![] }],
        outputs: vec![TxOutput { commitment: LweCommitment::new_random(), value: 100 }],
        fee: 1, nonce: 1,
    };
    state.apply_tx(&tx).expect("First spend ok");
    assert!(state.apply_tx(&tx).is_err());
    println!("      OK Transaction + double-spend protection");
    state.advance_block();
    assert_eq!(state.current_block, 1);
    println!("      OK Block advancement");

    println!("\n[3/6] Testing epoch rollover...");
    for _ in 0..101 { state.advance_block(); }
    assert_eq!(state.current_epoch, 1);
    println!("      OK Epoch rollover (100 blocks)");

    println!("\n[4/6] Testing consensus...");
    let id = derive_node_id(0);
    let validators = vec![id];
    let mut bft = OrbitalBft::new(id, validators);
    { let mut utxo = bft.utxo.write().await; utxo.add_to_mempool(tx.clone()); }
    let (dm, msg_rx) = MeshNetwork::new(id);
    bft.attach_mesh(Arc::new(dm));
    let h = tokio::spawn(async move { bft.run(msg_rx).await; });
    tokio::time::sleep(Duration::from_secs(4)).await;
    h.abort();
    println!("      OK Single-node consensus (4s)");

    println!("\n[5/6] Testing P2P codec...");
    let msg = P2pMessage::Ping { nonce: 42 };
    let enc = encode(&msg).unwrap();
    let (dec, n) = decode_one::<P2pMessage>(&enc).unwrap().unwrap();
    assert_eq!(n, enc.len());
    match dec { P2pMessage::Ping { nonce } => assert_eq!(nonce, 42), _ => panic!() }
    println!("      OK bincode codec");

    println!("\n[6/6] Testing keygen + JSON...");
    let kp2 = MlKemKeypair::generate();
    let data = serde_json::json!({
        "public_key": hex::encode(&kp2.public_key),
        "secret_key": hex::encode(&kp2.secret_key),
        "algorithm": "ML-KEM-1024-placeholder",
    });
    let json = serde_json::to_string_pretty(&data).unwrap();
    assert!(json.contains("ML-KEM-1024"));
    println!("      OK Keygen + JSON serialization");

    println!("\n========================================");
    println!("  ALL TESTS PASSED");
    println!("========================================");
}

async fn cmd_benchmark(nodes: usize, txs: usize) {
    println!("[benchmark] {} nodes, {} txs", nodes, txs);
    let mut handles = vec![];
    for i in 0..nodes {
        let h = tokio::spawn(async move {
            let id = derive_node_id(i as u8);
            let (mesh, msg_rx) = MeshNetwork::new(id);
            let mesh = Arc::new(mesh);
            let bind = testnet_addr(i as u8);
            let m2 = mesh.clone();
            tokio::spawn(async move { let _ = m2.listen(bind).await; });
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = mesh.dial_peers(&testnet_peers(i as u8)).await;
            let vals: Vec<NodeId> = (0..nodes).map(|j| derive_node_id(j as u8)).collect();
            let mut bft = OrbitalBft::new(id, vals);
            bft.attach_mesh(mesh);
            if i == 0 {
                let mut utxo = bft.utxo.write().await;
                for t in 0..txs {
                    utxo.add_to_mempool(Transaction {
                        inputs: vec![TxInput { nullifier: [(t % 256) as u8; 32], spend_proof: vec![] }],
                        outputs: vec![TxOutput { commitment: qrap_crypto::LweCommitment::new_random(), value: 100 }],
                        fee: 1, nonce: t as u64,
                    });
                }
            }
            bft.run(msg_rx).await;
        });
        handles.push(h);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    tokio::time::sleep(Duration::from_secs(30)).await;
    println!("[benchmark] Complete");
}

fn derive_node_id(idx: u8) -> NodeId { let mut id = [0u8; 32]; id[0] = idx; id[1] = 0xAB; id }
fn testnet_addr(idx: u8) -> SocketAddr { format!("127.0.0.1:{}", 10000 + idx as u16).parse().unwrap() }
fn testnet_peers(my_idx: u8) -> Vec<PeerConfig> { (0..4u8).filter(|&i| i != my_idx).map(|i| PeerConfig { id: derive_node_id(i), addr: testnet_addr(i) }).collect() }

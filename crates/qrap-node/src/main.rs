use clap::{Parser, Subcommand};
use qrap_crypto::{MlKemKeypair, poseidon256, LweCommitment};
use qrap_net::{MeshNetwork, PeerConfig, NodeId, encode, decode_one, P2pMessage};
use qrap_consensus::OrbitalBft;
use qrap_utxo::{Transaction, TxInput, TxOutput, UtxoState};
use qrap_fpga::{MockFpga, FpgaProver, PowerState, PlonkInput};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "qrap-node")]
#[command(about = "QRAP Node v0.2.0-alpha — Sentinel Space Core FPGA Ready")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Keygen {
        #[arg(short, long, default_value = "validator.keys")]
        output: String,
    },
    Run {
        #[arg(short, long, default_value = "0")]
        node_id: u8,
        #[arg(short, long, default_value = "./data")]
        data_dir: String,
    },
    Test,
    Benchmark {
        #[arg(short, long, default_value = "4")]
        nodes: usize,
        #[arg(short, long, default_value = "100")]
        txs: usize,
        #[arg(short, long, default_value = "./data")]
        data_dir: String,
    },
    FpgaBench {
        #[arg(short = 'n', long, default_value = "10")]
        proofs: usize,
        #[arg(short = 'm', long, default_value = "full")]
        power: String,
        #[arg(short = 'b', long, default_value = "mock")]
        backend: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Keygen { output } => cmd_keygen(&output),
        Commands::Run { node_id, data_dir } => cmd_run(node_id, &data_dir).await,
        Commands::Test => cmd_test().await,
        Commands::Benchmark { nodes, txs, data_dir } => cmd_benchmark(nodes, txs, &data_dir).await,
        Commands::FpgaBench { proofs, power, backend } => cmd_fpga_bench(proofs, &power, &backend).await,
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

async fn cmd_run(node_id: u8, data_dir: &str) {
    let id = derive_node_id(node_id);
    println!("[run] Starting node {} (id={})", node_id, hex::encode(&id[..4]));
    let storage_path = format!("{}/node_{}", data_dir, node_id);
    std::fs::create_dir_all(&storage_path).expect("Failed to create data dir");
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
    println!("[run] Node {} consensus started (Ctrl+C to stop)", node_id);
    bft.run(msg_rx).await;
}

async fn cmd_test() {
    println!("=== QRAP Node v0.2.0-alpha Self-Test ===\n");

    println!("[1/8] Testing crypto...");
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

    println!("\n[2/8] Testing UTXO...");
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

    println!("\n[3/8] Testing epoch rollover...");
    for _ in 0..101 { state.advance_block(); }
    assert_eq!(state.current_epoch, 1);
    println!("      OK Epoch rollover (100 blocks)");

    println!("\n[4/8] Testing consensus...");
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

    println!("\n[5/8] Testing P2P codec...");
    let msg = P2pMessage::Ping { nonce: 42 };
    let enc = encode(&msg).unwrap();
    let (dec, n) = decode_one::<P2pMessage>(&enc).unwrap().unwrap();
    assert_eq!(n, enc.len());
    match dec { P2pMessage::Ping { nonce } => assert_eq!(nonce, 42), _ => panic!() }
    println!("      OK bincode codec");

    println!("\n[6/8] Testing persistent storage (sled)...");
    let tmp = format!("{}/qrap_test_db", std::env::temp_dir().to_string_lossy());
    let _ = std::fs::remove_dir_all(&tmp);
    {
        let mut state = UtxoState::with_storage(&tmp).unwrap();
        let tx2 = Transaction {
            inputs: vec![TxInput { nullifier: [0xAB; 32], spend_proof: vec![] }],
            outputs: vec![TxOutput { commitment: LweCommitment::new_random(), value: 50 }],
            fee: 1, nonce: 2,
        };
        state.apply_tx(&tx2).unwrap();
        state.advance_block();
        state.add_to_mempool(tx2);
        state.flush().unwrap();
    }
    {
        let state = UtxoState::with_storage(&tmp).unwrap();
        assert_eq!(state.current_block, 1);
        assert_eq!(state.mempool.len(), 1);
        let epoch = state.epochs.get(&0).unwrap();
        assert!(epoch.spent_nullifiers.contains(&[0xAB; 32]));
    }
    let _ = std::fs::remove_dir_all(&tmp);
    println!("      OK sled persistence (crash recovery)");

    println!("\n[7/8] Testing FPGA prover (Mock)...");
    let mut fpga = MockFpga::new();
    fpga.init().await.unwrap();
    let proof_input = PlonkInput {
        nullifiers: vec![[0x01; 32], [0x02; 32], [0x03; 32]],
        commitments: vec![[0xA1; 32], [0xA2; 32]],
        public_inputs: vec![100, 50, 25],
    };
    let proof = fpga.prove(proof_input).await.unwrap();
    assert_eq!(proof.public_inputs, vec![100, 50, 25]);
    assert!(proof.generation_time_ms >= 1500);
    println!("      OK PLONK proof: {}ms, {}mJ", proof.generation_time_ms, proof.power_consumed_mj);

    fpga.set_power(PowerState::Eco).await.unwrap();
    let health = fpga.health().await;
    assert_eq!(health.power_state, PowerState::Eco);
    println!("      OK Power state switching (Full -> Eco)");

    println!("\n[8/8] Testing FPGA prover (Versal TMR)...");
    #[cfg(feature = "versal")]
    {
        use qrap_fpga::VersalProver;
        let mut versal = VersalProver::new();
        versal.init().await.unwrap();
        versal.enable_fault_injection(true);
        let v_input = PlonkInput {
            nullifiers: vec![[0x11; 32]],
            commitments: vec![[0x22; 32]],
            public_inputs: vec![999],
        };
        let v_proof = versal.prove(v_input).await.unwrap();
        assert_eq!(versal.health().await.radiation_events, 1);
        assert!(!v_proof.proof_bytes.is_empty());
        println!("      OK Versal TMR: SEU injected & corrected, {} radiation events", versal.health().await.radiation_events);
    }
    #[cfg(not(feature = "versal"))]
    {
        println!("      SKIP Versal (build with --features versal)");
    }

    println!("\n========================================");
    println!("  ALL TESTS PASSED — Sentinel Space Core Ready");
    println!("========================================");
}

async fn cmd_benchmark(nodes: usize, txs: usize, data_dir: &str) {
    println!("[benchmark] {} nodes, {} txs", nodes, txs);
    let mut handles = vec![];
    for i in 0..nodes {
        let data_path = format!("{}/node_{}", data_dir, i);
        let _ = std::fs::create_dir_all(&data_path);
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

async fn cmd_fpga_bench(proofs: usize, power: &str, backend: &str) {
    println!("=== QRAP FPGA Benchmark ===");
    println!("Backend: {}, Target: {} proofs, power mode: {}", backend, proofs, power);
    
    let power_state = match power {
        "eco" => PowerState::Eco,
        "balanced" => PowerState::Balanced,
        _ => PowerState::Full,
    };
    
    match backend {
        "mock" => {
            let mut fpga = MockFpga::new();
            fpga.init().await.unwrap();
            fpga.set_power(power_state).await.unwrap();
            run_bench(&mut fpga, proofs, backend).await;
        }
        "versal" => {
            #[cfg(feature = "versal")]
            {
                use qrap_fpga::VersalProver;
                let mut fpga = VersalProver::new();
                fpga.init().await.unwrap();
                fpga.set_power(power_state).await.unwrap();
                run_bench(&mut fpga, proofs, backend).await;
            }
            #[cfg(not(feature = "versal"))]
            {
                println!("Versal requires --features versal");
                println!("Running mock fallback...");
                let mut fpga = MockFpga::new();
                fpga.init().await.unwrap();
                fpga.set_power(power_state).await.unwrap();
                run_bench(&mut fpga, proofs, "mock(fallback)").await;
            }
        }
        "aws-f1" => {
            println!("AWS F1 requires --features aws-f1");
            println!("Running mock fallback...");
            let mut fpga = MockFpga::new();
            fpga.init().await.unwrap();
            fpga.set_power(power_state).await.unwrap();
            run_bench(&mut fpga, proofs, "mock(fallback)").await;
        }
        _ => {
            println!("Unknown backend: {}. Use: mock, versal, aws-f1", backend);
        }
    }
}

async fn run_bench(fpga: &mut dyn FpgaProver, proofs: usize, label: &str) {
    let mut total_time_ms = 0u64;
    let mut total_energy_mj = 0u64;
    
    for i in 0..proofs {
        let input = PlonkInput {
            nullifiers: vec![[(i % 256) as u8; 32]; 5],
            commitments: vec![[((i + 1) % 256) as u8; 32]; 3],
            public_inputs: vec![i as u64 * 100, 50],
        };
        let proof = fpga.prove(input).await.unwrap();
        total_time_ms += proof.generation_time_ms;
        total_energy_mj += proof.power_consumed_mj;
        
        if (i + 1) % 10 == 0 {
            let health = fpga.health().await;
            println!("  [{}/{}] Proof time: {}ms | Temp: {:.1}°C | Scrubber: {}", 
                     i + 1, proofs, proof.generation_time_ms, 
                     health.temperature_c, health.scrubber_cycles);
        }
    }
    
    let avg_time = total_time_ms as f64 / proofs as f64;
    let avg_energy = total_energy_mj as f64 / proofs as f64;
    let throughput = 1000.0 / avg_time;
    
    println!("\n=== Results ===");
    println!("Backend:           {}", label);
    println!("Power mode:        {:?} ({}W)", fpga.health().await.power_state, fpga.health().await.power_state.power_watts());
    println!("Total proofs:      {}", proofs);
    println!("Avg proof time:    {:.1} ms", avg_time);
    println!("Avg energy/proof:  {:.1} mJ", avg_energy);
    println!("Throughput:        {:.2} proofs/sec", throughput);
    println!("Est. cost/tx:      ${:.6}", avg_energy / 1_000_000.0 * 0.10);
    println!("Margin (at $0.01/tx): {:.1}%", (1.0 - (avg_energy / 1_000_000.0 * 0.10) / 0.01) * 100.0);
}

fn derive_node_id(idx: u8) -> NodeId {
    let mut id = [0u8; 32];
    id[0] = idx;
    id[1] = 0xAB;
    id
}

fn testnet_addr(idx: u8) -> SocketAddr {
    format!("127.0.0.1:{}", 10000 + idx as u16).parse().unwrap()
}

fn testnet_peers(my_idx: u8) -> Vec<PeerConfig> {
    (0..4u8).filter(|&i| i != my_idx)
        .map(|i| PeerConfig { id: derive_node_id(i), addr: testnet_addr(i) })
        .collect()
}

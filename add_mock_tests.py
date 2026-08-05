with open('crates/qrap-fpga/src/lib.rs', 'r') as f:
    content = f.read()

new_tests = '''
    #[tokio::test]
    async fn test_mock_prove_deterministic() {
        let mut fpga = MockFpga::new();
        fpga.init().await.unwrap();
        let input = PlonkInput {
            nullifiers: vec![[0xAB; 32]],
            commitments: vec![[0xCD; 32]],
            public_inputs: vec![42],
        };
        let p1 = fpga.prove(input.clone()).await.unwrap();
        let p2 = fpga.prove(input).await.unwrap();
        assert_eq!(p1.proof_bytes, p2.proof_bytes);
        assert_eq!(p1.verification_hash, p2.verification_hash);
    }

    #[tokio::test]
    async fn test_mock_prove_empty_input() {
        let mut fpga = MockFpga::new();
        fpga.init().await.unwrap();
        let input = PlonkInput {
            nullifiers: vec![],
            commitments: vec![],
            public_inputs: vec![],
        };
        let proof = fpga.prove(input).await.unwrap();
        assert!(!proof.proof_bytes.is_empty());
        assert!(proof.generation_time_ms > 0);
    }

    #[tokio::test]
    async fn test_mock_prove_power_scaling() {
        let mut fpga = MockFpga::new();
        fpga.init().await.unwrap();
        let input = PlonkInput {
            nullifiers: vec![[0x01; 32]],
            commitments: vec![[0x02; 32]],
            public_inputs: vec![100],
        };
        fpga.set_power(PowerState::Full).await.unwrap();
        let p_full = fpga.prove(input.clone()).await.unwrap();
        fpga.set_power(PowerState::Eco).await.unwrap();
        let p_eco = fpga.prove(input).await.unwrap();
        assert!(p_full.generation_time_ms <= p_eco.generation_time_ms);
    }
'''

# Находим последнюю } в модуле tests
idx = content.rfind('}')
if idx != -1:
    content = content[:idx] + new_tests + '\n}\n'

with open('crates/qrap-fpga/src/lib.rs', 'w') as f:
    f.write(content)
print('Done')

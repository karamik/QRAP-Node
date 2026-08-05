with open('crates/qrap-fpga/src/aws_f1/prover.rs', 'r') as f:
    content = f.read()

tests = '''
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_software_prove_basic() {
        let prover = AwsF1Prover::new("/dev/null");
        let input = PlonkInput {
            nullifiers: vec![[0x01; 32], [0x02; 32]],
            commitments: vec![[0x03; 32]],
            public_inputs: vec![100, 200],
        };
        let proof = prover.software_prove(&input);
        assert!(!proof.proof_bytes.is_empty());
        assert_eq!(proof.public_inputs, vec![100, 200]);
        assert!(proof.generation_time_ms > 0);
    }

    #[tokio::test]
    async fn test_software_prove_deterministic() {
        let prover = AwsF1Prover::new("/dev/null");
        let input = PlonkInput {
            nullifiers: vec![[0xAB; 32]],
            commitments: vec![[0xCD; 32]],
            public_inputs: vec![42],
        };
        let p1 = prover.software_prove(&input);
        let p2 = prover.software_prove(&input);
        assert_eq!(p1.proof_bytes, p2.proof_bytes);
        assert_eq!(p1.verification_hash, p2.verification_hash);
    }

    #[tokio::test]
    async fn test_power_state_timing() {
        let mut prover = AwsF1Prover::new("/dev/null");
        prover.set_power(PowerState::Eco).await.unwrap();
        assert_eq!(prover.health().await.power_state, PowerState::Eco);
    }

    #[tokio::test]
    async fn test_pipeline_stats_default() {
        let stats = PipelineStats::default();
        assert_eq!(stats.ntt_runs, 0);
        assert_eq!(stats.msm_runs, 0);
        assert_eq!(stats.total_proofs, 0);
    }

    #[tokio::test]
    async fn test_software_prove_power_consumption() {
        let prover = AwsF1Prover::new("/dev/null");
        let input = PlonkInput {
            nullifiers: vec![],
            commitments: vec![],
            public_inputs: vec![],
        };
        let proof = prover.software_prove(&input);
        assert!(proof.power_consumed_mj > 0);
    }

    #[tokio::test]
    async fn test_fpga_prove_without_device_fails() {
        let mut prover = AwsF1Prover::new("/dev/null");
        let input = PlonkInput {
            nullifiers: vec![[0x01; 32]],
            commitments: vec![[0x02; 32]],
            public_inputs: vec![100],
        };
        let result = prover.fpga_prove(&input);
        assert!(result.is_err());
        assert!(matches!(result, Err(FpgaError::NotAvailable(_))));
    }
}
'''

content = content.rstrip() + '\n' + tests + '\n'

with open('crates/qrap-fpga/src/aws_f1/prover.rs', 'w') as f:
    f.write(content)
print('Done')

//! PLONK proof generation with AWS F1 FPGA acceleration
use tracing::info;

#[cfg(feature = "aws-f1")]
use qrap_fpga::aws_f1::prover::AwsF1Prover;

/// PLONK proof bytes
pub type PlonkProofBytes = Vec<u8>;

/// PLONK prover — uses FPGA when available, CPU fallback
pub struct PlonkProver {
    #[cfg(feature = "aws-f1")]
    f1: Option<AwsF1Prover>,
}

impl PlonkProver {
    pub fn new() -> Self {
        #[cfg(feature = "aws-f1")]
        {
            let f1 = AwsF1Prover::new();
            info!("PLONK prover initialized with AWS F1 support");
            Self { f1: Some(f1) }
        }
        #[cfg(not(feature = "aws-f1"))]
        {
            info!("PLONK prover initialized (CPU-only)");
            Self {}
        }
    }

    /// Initialize FPGA with xclbin (no-op if aws-f1 disabled)
    pub fn init_f1(&mut self, xclbin_path: &str) -> Result<(), &'static str> {
        #[cfg(feature = "aws-f1")]
        {
            if let Some(ref f1) = self.f1 {
                match f1.init(xclbin_path) {
                    Ok(()) => {
                        info!("AWS F1 initialized: {}", xclbin_path);
                        Ok(())
                    }
                    Err(rc) => {
                        warn!("AWS F1 init failed (rc={}), falling back to CPU", rc);
                        self.f1 = None;
                        Ok(())
                    }
                }
            } else {
                warn!("AWS F1 not available");
                Ok(())
            }
        }
        #[cfg(not(feature = "aws-f1"))]
        {
            let _ = xclbin_path;
            Ok(())
        }
    }

    /// Generate PLONK proof
    pub fn prove(&self, circuit: &[u8], witness: &[u8]) -> PlonkProofBytes {
        #[cfg(feature = "aws-f1")]
        {
            if let Some(ref f1) = self.f1 {
                info!("Generating PLONK proof via AWS F1...");
                return f1.prove(circuit, witness);
            }
        }
        info!("Generating PLONK proof via CPU fallback...");
        // CPU fallback: hash-based placeholder
        let mut result = Vec::with_capacity(256);
        result.extend_from_slice(circuit);
        result.extend_from_slice(witness);
        result
    }

    /// Verify PLONK proof (placeholder)
    pub fn verify(&self, _proof: &[u8], _circuit: &[u8]) -> bool {
        // Placeholder: real verification needs pairing checks
        true
    }
}

impl Default for PlonkProver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plonk_prover_new() {
        let prover = PlonkProver::new();
        assert!(prover.verify(&[], &[]));
    }

    #[test]
    fn test_plonk_prove_cpu() {
        let prover = PlonkProver::new();
        let proof = prover.prove(b"circuit", b"witness");
        assert!(!proof.is_empty());
    }
}

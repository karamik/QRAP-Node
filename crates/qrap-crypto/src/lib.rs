//! QRAP Cryptographic Primitives
use sha3::{Digest, Sha3_256};
use serde::{Serialize, Deserialize};
use rand::Rng;

pub const HASH_LEN: usize = 32;
pub const LWE_N: usize = 1024;
pub type Hash = [u8; HASH_LEN];

pub fn poseidon256(data: &[u8]) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(b"POSEIDON_PLACEHOLDER_v0.2");
    hasher.update(data);
    hasher.finalize().into()
}

pub fn poseidon256_pair(left: &Hash, right: &Hash) -> Hash {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(left);
    buf.extend_from_slice(right);
    poseidon256(&buf)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LweCommitment {
    pub a: Vec<u64>,
    pub b: Vec<u64>,
}

impl LweCommitment {
    pub fn new_random() -> Self {
        let a: Vec<u64> = (0..LWE_N).map(|_| rand::random()).collect();
        let b: Vec<u64> = (0..LWE_N).map(|_| rand::random()).collect();
        Self { a, b }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(LWE_N * 16);
        for &x in &self.a { out.extend_from_slice(&x.to_le_bytes()); }
        for &x in &self.b { out.extend_from_slice(&x.to_le_bytes()); }
        out
    }
    pub fn hash(&self) -> Hash { poseidon256(&self.to_bytes()) }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlKemKeypair {
    pub public_key: Vec<u8>,   // 1568 bytes for ML-KEM-1024
    pub secret_key: Vec<u8>,   // 3168 bytes
}

impl MlKemKeypair {
    pub fn generate() -> Self {
        let mut pk = vec![0u8; 1568];
        let mut sk = vec![0u8; 3168];
        for byte in pk.iter_mut() { *byte = rand::random(); }
        for byte in sk.iter_mut() { *byte = rand::random(); }
        Self { public_key: pk, secret_key: sk }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlDsaSignature { pub bytes: Vec<u8> }

pub fn sign_placeholder(msg: &[u8], _sk: &[u8]) -> MlDsaSignature {
    MlDsaSignature { bytes: poseidon256(msg).to_vec() }
}
pub fn verify_placeholder(msg: &[u8], sig: &MlDsaSignature, _pk: &[u8]) -> bool {
    sig.bytes == poseidon256(msg).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_poseidon_deterministic() {
        assert_eq!(poseidon256(b"test"), poseidon256(b"test"));
    }
    #[test] fn test_lwe_commitment_hash() {
        assert_eq!(LweCommitment::new_random().hash().len(), 32);
    }
    #[test] fn test_ml_kem_generate() {
        let kp = MlKemKeypair::generate();
        assert_eq!(kp.public_key.len(), 1568);
        assert_eq!(kp.secret_key.len(), 3168);
    }
}

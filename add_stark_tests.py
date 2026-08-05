with open('crates/qrap-stark/src/lib.rs', 'r') as f:
    content = f.read()

tests = '''

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_spend_deterministic() {
        let secret = b"my-secret-key";
        let commitment = poseidon256(b"public-commitment");
        let p1 = QrapStarkProver::prove_spend(secret, &commitment).unwrap();
        let p2 = QrapStarkProver::prove_spend(secret, &commitment).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_prove_spend_different_secrets() {
        let c1 = poseidon256(b"commitment-1");
        let c2 = poseidon256(b"commitment-2");
        let p1 = QrapStarkProver::prove_spend(b"secret-1", &c1).unwrap();
        let p2 = QrapStarkProver::prove_spend(b"secret-2", &c2).unwrap();
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_prove_spend_non_empty() {
        let secret = b"test";
        let commitment = poseidon256(b"commit");
        let proof = QrapStarkProver::prove_spend(secret, &commitment).unwrap();
        assert!(!proof.is_empty());
    }

    #[test]
    fn test_verify_spend_placeholder() {
        let secret = b"secret";
        let commitment = poseidon256(b"commit");
        let proof = QrapStarkProver::prove_spend(secret, &commitment).unwrap();
        let result = QrapStarkProver::verify_spend(&proof, &commitment);
        assert!(result.is_ok());
    }
}
'''

content = content.rstrip() + '\\n' + tests + '\\n'

with open('crates/qrap-stark/src/lib.rs', 'w') as f:
    f.write(content)
print('qrap-stark done')

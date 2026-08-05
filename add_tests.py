with open('crates/qrap-da/src/lib.rs', 'r') as f:
    content = f.read()

new_tests = '''
    #[tokio::test]
    async fn test_celestia_client_submit_pfb() {
        let config = CelestiaConfig::default();
        let client = CelestiaClient::new(config);
        let ns = Namespace::new(b"qrap-test");
        let blob = Blob::new(ns, vec![1, 2, 3, 4, 5]);
        let tx_hash = client.submit_pfb(&blob).await.unwrap();
        assert!(tx_hash.starts_with("0x"));
        assert_eq!(tx_hash.len(), 34);
    }

    #[tokio::test]
    async fn test_celestia_client_get_latest_height() {
        let config = CelestiaConfig::default();
        let client = CelestiaClient::new(config);
        let height = client.get_latest_height().await.unwrap();
        assert!(height >= 1000000);
    }

    #[test]
    fn test_blobstream_invalid_height() {
        let trusted_root = [0xAA; 32];
        let verifier = BlobstreamVerifier::new(10, trusted_root);
        let h1 = BlobstreamHeader {
            height: 12,
            data_root: poseidon256(b"header1"),
            prev_hash: trusted_root,
            timestamp: 1000,
        };
        let result = verifier.verify_header_chain(&[h1]);
        assert!(matches!(result, Err(DaError::InvalidProof)));
    }

    #[test]
    fn test_blobstream_invalid_prev_hash() {
        let trusted_root = [0xAA; 32];
        let verifier = BlobstreamVerifier::new(0, trusted_root);
        let h1 = BlobstreamHeader {
            height: 1,
            data_root: poseidon256(b"header1"),
            prev_hash: [0xBB; 32],
            timestamp: 1000,
        };
        let result = verifier.verify_header_chain(&[h1]);
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_blobstream_empty_chain() {
        let trusted_root = [0xAA; 32];
        let verifier = BlobstreamVerifier::new(0, trusted_root);
        let result = verifier.verify_header_chain(&[]);
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_blobstream_verify_blob_namespace_mismatch() {
        let ns1 = Namespace::new(b"ns1");
        let ns2 = Namespace::new(b"ns2");
        let blob = Blob::new(ns1.clone(), vec![1, 2, 3]);
        let header = BlobstreamHeader {
            height: 1,
            data_root: poseidon256(b"header1"),
            prev_hash: [0xAA; 32],
            timestamp: 1000,
        };
        let proof = InclusionProof {
            blob_commitment: blob.commitment(),
            row_roots: vec![[0u8; 32]],
            column_roots: vec![[0u8; 32]],
            row_index: 0,
            column_index: 0,
            namespace: ns2,
        };
        let verifier = BlobstreamVerifier::new(0, [0xAA; 32]);
        let result = verifier.verify_blob_in_header(&blob, &header, &proof);
        assert!(matches!(result, Err(DaError::NamespaceMismatch)));
    }
'''

content = content.rstrip()
if content.endswith('}'):
    content = content[:-1] + new_tests + '\n}\n'

with open('crates/qrap-da/src/lib.rs', 'w') as f:
    f.write(content)
print('Done')

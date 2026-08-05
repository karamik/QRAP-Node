//! QRAP-Net: Kernel-bypass networking engine for QRAP Node
//!
//! Built on Linux io_uring via monoio. Provides:
//! - P2P mesh consensus transport
//! - JSON-RPC gateway
//! - Zero-allocation TLS streaming (placeholder for ML-KEM TLS)

pub mod codec;
pub mod p2p;
pub mod rpc;
pub mod transport;

pub use codec::{decode_one, encode, CodecError};
pub use p2p::{MeshNetwork, NodeId, P2pMessage, PeerConfig};
pub use rpc::{serve_rpc, JsonRpcError, JsonRpcRequest, JsonRpcResponse, RpcHandler};
pub use transport::{accept_loop, connect_peer, read_exact, write_all};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_equality() {
        let id1: NodeId = [1u8; 32];
        let id2: NodeId = [1u8; 32];
        let id3: NodeId = [2u8; 32];
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_p2p_message_ping_pong() {
        let msg = P2pMessage::Ping { nonce: 42 };
        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode_one::<P2pMessage>(&encoded).unwrap().unwrap();
        match decoded {
            P2pMessage::Ping { nonce } => assert_eq!(nonce, 42),
            _ => panic!("Expected Ping"),
        }
    }

    #[test]
    fn test_p2p_message_consensus() {
        let payload = vec![1, 2, 3, 4, 5];
        let msg = P2pMessage::Consensus(payload.clone());
        let encoded = encode(&msg).unwrap();
        let (decoded, _) = decode_one::<P2pMessage>(&encoded).unwrap().unwrap();
        match decoded {
            P2pMessage::Consensus(p) => assert_eq!(p, payload),
            _ => panic!("Expected Consensus"),
        }
    }

    #[test]
    fn test_peer_config_creation() {
        let config = PeerConfig {
            id: [0xab; 32],
            addr: "127.0.0.1:8080".parse().unwrap(),
        };
        assert_eq!(config.id, [0xab; 32]);
        assert_eq!(config.addr.to_string(), "127.0.0.1:8080");
    }
}

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

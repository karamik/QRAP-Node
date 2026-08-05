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

pub use codec::{encode, decode_one, CodecError};
pub use p2p::{MeshNetwork, P2pMessage, PeerConfig, NodeId};
pub use rpc::{serve_rpc, JsonRpcRequest, JsonRpcResponse, RpcHandler, JsonRpcError};
pub use transport::{accept_loop, connect_peer, read_exact, write_all};

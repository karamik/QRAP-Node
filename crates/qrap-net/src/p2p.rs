use crate::codec::{encode, decode_one};
use crate::transport::{connect_peer, write_all};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use anyhow::Result;

pub type NodeId = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum P2pMessage {
    Consensus(Vec<u8>),
    MempoolTx(Vec<u8>),
    Ping { nonce: u64 },
    Pong { nonce: u64 },
}

#[derive(Clone, Debug)]
pub struct PeerConfig {
    pub id: NodeId,
    pub addr: SocketAddr,
}

struct PeerConn {
    tx: mpsc::UnboundedSender<P2pMessage>,
}

pub struct MeshNetwork {
    pub local_id: NodeId,
    peers: Arc<Mutex<HashMap<NodeId, PeerConn>>>,
    inbound_tx: mpsc::UnboundedSender<(NodeId, P2pMessage)>,
}

impl MeshNetwork {
    pub fn new(local_id: NodeId) -> (Self, mpsc::UnboundedReceiver<(NodeId, P2pMessage)>) {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let mesh = Self {
            local_id,
            peers: Arc::new(Mutex::new(HashMap::new())),
            inbound_tx,
        };
        (mesh, inbound_rx)
    }

    pub async fn listen(&self, bind_addr: SocketAddr) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        info!("Mesh listening on {}", bind_addr);
        let peers = self.peers.clone();
        let inbound = self.inbound_tx.clone();
        let local = self.local_id;
        loop {
            let (stream, addr) = listener.accept().await?;
            let peers2 = peers.clone();
            let inbound2 = inbound.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_inbound(stream, addr, local, peers2, inbound2).await {
                    warn!("Inbound handler error: {}", e);
                }
            });
        }
    }

    pub async fn dial_peers(&self, configs: &[PeerConfig]) -> Result<()> {
        for peer in configs {
            if peer.id == self.local_id { continue; }
            let stream = connect_peer(peer.addr).await?;
            let (tx, mut rx) = mpsc::unbounded_channel::<P2pMessage>();
            {
                let mut peers = self.peers.lock().unwrap();
                peers.insert(peer.id, PeerConn { tx });
            }
            info!("Dialed peer {} at {}", hex::encode(&peer.id[..4]), peer.addr);
            tokio::spawn(async move {
                let mut stream = stream;
                while let Some(msg) = rx.recv().await {
                    let buf = match encode(&msg) {
                        Ok(b) => b,
                        Err(e) => { error!("Encode error: {}", e); continue; }
                    };
                    if let Err(e) = write_all(&mut stream, &buf).await {
                        error!("Write to peer failed: {}", e);
                        break;
                    }
                }
            });
        }
        Ok(())
    }

    pub fn broadcast(&self, msg: P2pMessage) {
        let peers = self.peers.lock().unwrap();
        for (id, conn) in peers.iter() {
            if conn.tx.send(msg.clone()).is_err() {
                warn!("Peer {} disconnected", hex::encode(&id[..4]));
            }
        }
    }

    pub fn send_to(&self, peer_id: &NodeId, msg: P2pMessage) -> Result<()> {
        let peers = self.peers.lock().unwrap();
        let conn = peers.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found"))?;
        conn.tx.send(msg)?;
        Ok(())
    }
}

async fn handle_inbound(
    mut stream: TcpStream,
    addr: SocketAddr,
    _local_id: NodeId,
    _peers: Arc<Mutex<HashMap<NodeId, PeerConn>>>,
    inbound: mpsc::UnboundedSender<(NodeId, P2pMessage)>,
) -> Result<()> {
    let mut buf = vec![0u8; 4096];
    let mut cursor = 0usize;
    let peer_id = [0u8; 32];

    loop {
        let n = stream.read(&mut buf[cursor..]).await?;
        if n == 0 { break; }
        cursor += n;
        let mut consumed = 0usize;
        while consumed < cursor {
            match decode_one::<P2pMessage>(&buf[consumed..cursor])? {
                Some((msg, len)) => {
                    let _ = inbound.send((peer_id, msg));
                    consumed += len;
                }
                None => break,
            }
        }
        if consumed > 0 {
            buf.copy_within(consumed..cursor, 0);
            cursor -= consumed;
        }
        if buf.len() - cursor < 256 {
            buf.resize(buf.len() * 2, 0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_node_id_display() {
        let id = [0xab; 32];
        assert_eq!(hex::encode(&id[..4]), "abababab");
    }
}

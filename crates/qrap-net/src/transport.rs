//! Tokio-based TCP transport (Termux-compatible)

use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

pub async fn accept_loop<F>(bind_addr: SocketAddr, handler: F) -> Result<()>
where
    F: Fn(TcpStream, SocketAddr) + Send + Sync + 'static,
{
    let listener = TcpListener::bind(bind_addr).await?;
    info!("TCP listener bound to {}", bind_addr);
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Accepted connection from {}", addr);
                handler(stream, addr);
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

pub async fn connect_peer(addr: SocketAddr) -> Result<TcpStream> {
    let stream = TcpStream::connect(addr).await?;
    info!("Connected to peer {}", addr);
    Ok(stream)
}

pub async fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> Result<usize> {
    let mut read = 0;
    while read < buf.len() {
        let n = stream.read(&mut buf[read..]).await?;
        if n == 0 {
            return Err(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "peer closed").into(),
            );
        }
        read += n;
    }
    Ok(read)
}

pub async fn write_all(stream: &mut TcpStream, buf: &[u8]) -> Result<()> {
    stream.write_all(buf).await?;
    stream.flush().await?;
    Ok(())
}

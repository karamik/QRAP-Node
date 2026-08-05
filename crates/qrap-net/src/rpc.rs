use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::net::SocketAddr;
use tracing::{info, warn, error};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[async_trait]
pub trait RpcHandler: Send + Sync {
    async fn handle(&self, method: &str, params: &Value) -> Result<Value, String>;
}

pub async fn serve_rpc(bind_addr: SocketAddr, handler: Arc<dyn RpcHandler>) -> Result<()> {
    let listener = TcpListener::bind(bind_addr).await?;
    info!("JSON-RPC server listening on {}", bind_addr);
    loop {
        let (stream, addr) = listener.accept().await?;
        let h = handler.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr, h).await {
                warn!("RPC connection error from {}: {}", addr, e);
            }
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    handler: Arc<dyn RpcHandler>,
) -> Result<()> {
    let mut buf = vec![0u8; 8192];
    let mut cursor = 0usize;

    loop {
        let n = stream.read(&mut buf[cursor..]).await?;
        if n == 0 { break; }
        cursor += n;

        if let Some(header_end) = find_header_end(&buf[..cursor]) {
            let request = String::from_utf8_lossy(&buf[..header_end]);
            let body = parse_http_body(&request, &buf[header_end..cursor]);
            
            let response_body = match serde_json::from_str::<JsonRpcRequest>(&body) {
                Ok(req) => {
                    match handler.handle(&req.method, &req.params).await {
                        Ok(result) => serde_json::to_string(&JsonRpcResponse {
                            jsonrpc: "2.0".into(),
                            result: Some(result),
                            error: None,
                            id: req.id,
                        })?,
                        Err(msg) => serde_json::to_string(&JsonRpcResponse {
                            jsonrpc: "2.0".into(),
                            result: None,
                            error: Some(JsonRpcError { code: -32603, message: msg }),
                            id: req.id,
                        })?,
                    }
                }
                Err(e) => {
                    serde_json::to_string(&JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some(JsonRpcError { code: -32700, message: e.to_string() }),
                        id: Value::Null,
                    })?
                }
            };

            let http_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(http_response.as_bytes()).await?;
            stream.flush().await?;

            let total_used = header_end + body.len();
            buf.copy_within(total_used..cursor, 0);
            cursor -= total_used;
        }
    }
    info!("RPC connection closed {}", addr);
    Ok(())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    let needle = b"\r\n\r\n";
    buf.windows(needle.len()).position(|w| w == needle).map(|i| i + needle.len())
}

fn parse_http_body(headers: &str, trailing: &[u8]) -> String {
    let cl = headers.lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let available = trailing.len();
    let take = cl.min(available);
    String::from_utf8_lossy(&trailing[..take]).to_string()
}

use std::sync::Arc;

//! Length-prefixed message codec for P2P mesh

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Malformed length prefix")]
    MalformedLength,
}

/// Encode a serializable message into length-prefixed bytes.
pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, CodecError> {
    let payload = bincode::serialize(msg)
        .map_err(|e| CodecError::Serialization(e.to_string()))?;
    let len = payload.len() as u32;
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(&payload);
    Ok(buf)
}

/// Try to decode one message from the buffer. Returns (message, bytes_consumed) if complete.
pub fn decode_one<T: DeserializeOwned>(buf: &[u8]) -> Result<Option<(T, usize)>, CodecError> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if buf.len() < 4 + len {
        return Ok(None);
    }
    let msg: T = bincode::deserialize(&buf[4..4 + len])
        .map_err(|e| CodecError::Serialization(e.to_string()))?;
    Ok(Some((msg, 4 + len)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
    struct TestMsg {
        id: u64,
        data: String,
    }

    #[test]
    fn test_roundtrip() {
        let msg = TestMsg { id: 42, data: "hello".into() };
        let encoded = encode(&msg).unwrap();
        let (decoded, consumed) = decode_one::<TestMsg>(&encoded).unwrap().unwrap();
        assert_eq!(msg, decoded);
        assert_eq!(consumed, encoded.len());
    }
}

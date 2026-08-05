//! QRAP Storage — Persistent KV store via sled (pure Rust, Termux-compatible)
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Key not found: {0}")]
    NotFound(String),
}

pub struct Storage { db: sled::Db }

impl Storage {
    pub fn open(path: &str) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        info!("Storage opened at: {}", path);
        Ok(Self { db })
    }
    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let bytes = bincode::serialize(value).map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.db.insert(key, bytes)?;
        Ok(())
    }
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StorageError> {
        match self.db.get(key)? {
            Some(bytes) => {
                let value = bincode::deserialize(&bytes).map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
    pub fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.db.remove(key)?;
        Ok(())
    }
    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush()?; Ok(())
    }
    pub fn clear(&self) -> Result<(), StorageError> {
        self.db.clear()?; Ok(())
    }
}

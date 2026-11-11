//! Error types for `aqvn-crdt`.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrdtError {
    #[error("Loro CRDT error: {0}")]
    Loro(#[from] loro::LoroError),

    #[error("Loro encode/decode error: {0}")]
    LoroEncode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Integrity check failed: expected {expected}, actual {actual}")]
    IntegrityMismatch { expected: String, actual: String },

    #[error("Persistence error: {0}")]
    PersistenceError(String),

    #[error("Sync protocol error: {0}")]
    SyncError(String),
}

impl From<loro::LoroEncodeError> for CrdtError {
    fn from(err: loro::LoroEncodeError) -> Self {
        CrdtError::LoroEncode(err.to_string())
    }
}

pub type CrdtResult<T> = Result<T, CrdtError>;

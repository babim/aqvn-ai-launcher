//! Error types for the AQVN WASM plugin runtime.

use thiserror::Error;

/// Core error enum for WASM plugin operations.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Wasmtime runtime error: {0}")]
    Wasm(String),

    #[error("Fuel exhausted: consumed {consumed}/{limit} fuel units")]
    FuelExhausted { consumed: u64, limit: u64 },

    #[error("Execution timed out after {timeout_ms}ms (elapsed: {elapsed_ms}ms)")]
    Timeout { elapsed_ms: u64, timeout_ms: u64 },

    #[error(
        "Memory limit exceeded: requested {requested_bytes} bytes, limit is {max_bytes} bytes"
    )]
    MemoryLimitExceeded {
        requested_bytes: usize,
        max_bytes: usize,
    },

    #[error("Plugin not found: '{0}'")]
    PluginNotFound(String),

    #[error("Invalid plugin binary or structure: {0}")]
    InvalidPlugin(String),

    #[error("Execution trapped: {0}")]
    ExecutionTrapped(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("IO error: {0}")]
    Io(String),
}

impl From<wasmtime::Error> for PluginError {
    fn from(err: wasmtime::Error) -> Self {
        let err_str = err.to_string();
        if err_str.contains("fuel") || err_str.contains("all fuel consumed") {
            PluginError::FuelExhausted {
                consumed: 0,
                limit: 0,
            }
        } else if err_str.contains("epoch") || err_str.contains("interrupted") {
            PluginError::Timeout {
                elapsed_ms: 0,
                timeout_ms: 0,
            }
        } else if err_str.contains("memory") || err_str.contains("resource limit") {
            PluginError::MemoryLimitExceeded {
                requested_bytes: 0,
                max_bytes: 0,
            }
        } else {
            PluginError::Wasm(err_str)
        }
    }
}

impl From<std::io::Error> for PluginError {
    fn from(err: std::io::Error) -> Self {
        PluginError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(err: serde_json::Error) -> Self {
        PluginError::Serialization(err.to_string())
    }
}

/// Convenience result alias for plugin operations.
pub type PluginResult<T> = Result<T, PluginError>;

//! Configuration options for the sandboxed WASM plugin runtime.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Default fuel limit: 10,000,000 instructions per execution.
pub const DEFAULT_FUEL_LIMIT: u64 = 10_000_000;

/// Default epoch timeout: 500 milliseconds for AST rule evaluation.
pub const DEFAULT_EPOCH_TIMEOUT_MS: u64 = 500;

/// Default maximum linear memory limit: 32 Megabytes.
pub const DEFAULT_MAX_MEMORY_BYTES: usize = 32 * 1024 * 1024;

/// Default maximum stack size: 1 Megabyte.
pub const DEFAULT_MAX_STACK_BYTES: usize = 1024 * 1024;

/// Default epoch tick interval: 20 milliseconds.
pub const DEFAULT_EPOCH_TICK_INTERVAL_MS: u64 = 20;

/// Configuration for the WASM sandbox environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Maximum WebAssembly execution fuel (instructions).
    pub default_fuel_limit: u64,

    /// Execution timeout in milliseconds enforced via Epoch Interruption.
    pub default_epoch_timeout_ms: u64,

    /// Maximum linear memory size in bytes allocated for guest instances.
    pub max_memory_bytes: usize,

    /// Maximum stack size in bytes.
    pub max_stack_bytes: usize,

    /// Directories pre-opened and accessible by guest instances via WASI.
    pub allowed_directories: Vec<PathBuf>,

    /// Frequency at which host epoch timer ticks.
    pub epoch_tick_interval_ms: u64,

    /// Whether instruction fuel metering is active.
    pub enable_fuel_metering: bool,

    /// Whether epoch interruption timeout is active.
    pub enable_epoch_interruption: bool,

    /// Whether WASI Preview 2 host capabilities are enabled.
    pub enable_wasi: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            default_fuel_limit: DEFAULT_FUEL_LIMIT,
            default_epoch_timeout_ms: DEFAULT_EPOCH_TIMEOUT_MS,
            max_memory_bytes: DEFAULT_MAX_MEMORY_BYTES,
            max_stack_bytes: DEFAULT_MAX_STACK_BYTES,
            allowed_directories: Vec::new(),
            epoch_tick_interval_ms: DEFAULT_EPOCH_TICK_INTERVAL_MS,
            enable_fuel_metering: true,
            enable_epoch_interruption: true,
            enable_wasi: true,
        }
    }
}

impl PluginConfig {
    /// Create a high-security restrictive configuration suitable for untrusted third-party plugins.
    pub fn restrictive() -> Self {
        Self {
            default_fuel_limit: 5_000_000,
            default_epoch_timeout_ms: 250,
            max_memory_bytes: 16 * 1024 * 1024,
            max_stack_bytes: 512 * 1024,
            allowed_directories: Vec::new(),
            epoch_tick_interval_ms: 10,
            enable_fuel_metering: true,
            enable_epoch_interruption: true,
            enable_wasi: false,
        }
    }

    /// Duration of the default epoch timeout.
    pub fn epoch_timeout(&self) -> Duration {
        Duration::from_millis(self.default_epoch_timeout_ms)
    }

    /// Duration of the host epoch tick interval.
    pub fn epoch_tick_interval(&self) -> Duration {
        Duration::from_millis(self.epoch_tick_interval_ms)
    }
}

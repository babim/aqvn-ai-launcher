//! Sandboxed WASM Component Model Plugin Runtime with Fuel Metering and Epoch Interruption.
//!
//! Provides enterprise-grade sandbox isolation for dynamic AST security rules and Agent tools:
//! 1. Wasmtime Component Model runtime with WASI Preview 2 support.
//! 2. Instruction-level Fuel Metering (default 10,000,000 fuel units per call).
//! 3. Epoch Interruption timeout (default 500ms timeout for AST rules).
//! 4. Linear memory limit capping (default 32MB max heap per plugin instance).
//! 5. Fail-closed security architecture.

pub mod agent_tool;
pub mod ast_rule;
pub mod config;
pub mod error;
pub mod loader;
pub mod runtime;

pub use agent_tool::{ToolCallInput, ToolCallOutput, ToolInfo, WasmAgentToolInvoker};
pub use ast_rule::{AstCheckRequest, AstCheckResponse, AstRuleVerdict, WasmAstRuleEvaluator};
pub use config::{
    PluginConfig, DEFAULT_EPOCH_TICK_INTERVAL_MS, DEFAULT_EPOCH_TIMEOUT_MS, DEFAULT_FUEL_LIMIT,
    DEFAULT_MAX_MEMORY_BYTES, DEFAULT_MAX_STACK_BYTES,
};
pub use error::{PluginError, PluginResult};
pub use loader::{LoadedPlugin, PluginManager, PluginMetadata, PluginType};
pub use runtime::{EpochTimer, PluginHostState, WasmPluginEngine};

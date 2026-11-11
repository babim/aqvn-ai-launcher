//! Plugin discovery, binary loading, and lifecycle management.

use crate::agent_tool::{ToolCallInput, ToolCallOutput, WasmAgentToolInvoker};
use crate::ast_rule::{AstCheckRequest, AstCheckResponse, WasmAstRuleEvaluator};
use crate::error::{PluginError, PluginResult};
use crate::runtime::WasmPluginEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use wasmtime::component::Component;
use wasmtime::Module;

/// The category of functionality a plugin provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    AstGuard,
    AgentTool,
    Generic,
}

/// Metadata describing a loaded plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub is_component: bool,
    pub fuel_limit: Option<u64>,
    pub timeout_ms: Option<u64>,
}

/// A loaded and verified plugin ready for instantiation.
#[derive(Clone)]
pub struct LoadedPlugin {
    pub meta: PluginMetadata,
    pub module: Option<Module>,
    pub component: Option<Component>,
}

/// Central manager and registry for sandboxed WASM plugins.
#[derive(Clone)]
pub struct PluginManager {
    engine: WasmPluginEngine,
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    plugins_dir: PathBuf,
}

impl PluginManager {
    /// Initialize a new plugin manager backed by the given engine.
    pub fn new(engine: WasmPluginEngine, plugins_dir: impl AsRef<Path>) -> Self {
        Self {
            engine,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugins_dir: plugins_dir.as_ref().to_path_buf(),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &WasmPluginEngine {
        &self.engine
    }

    /// Access the configured plugins directory.
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// Load and register a plugin from in-memory binary or WAT bytes.
    pub fn load_from_bytes(
        &self,
        id: &str,
        bytes: &[u8],
        plugin_type: PluginType,
    ) -> PluginResult<LoadedPlugin> {
        let is_comp_binary =
            bytes.len() >= 8 && &bytes[0..4] == b"\0asm" && bytes[4] == 0x0d && bytes[5] == 0x00;

        let is_text = bytes.starts_with(b"(")
            || std::str::from_utf8(bytes)
                .map(|s| s.trim_start().starts_with('('))
                .unwrap_or(false);

        let (module, component, is_component) = if is_comp_binary {
            let comp = Component::new(self.engine.engine(), bytes).map_err(|e| {
                PluginError::InvalidPlugin(format!("Failed to parse component: {e}"))
            })?;
            (None, Some(comp), true)
        } else if is_text {
            let wat_str = std::str::from_utf8(bytes)
                .map_err(|e| PluginError::InvalidPlugin(format!("Invalid UTF-8 in WAT: {e}")))?;

            if wat_str.contains("(component") {
                let comp = Component::new(self.engine.engine(), wat_str).map_err(|e| {
                    PluginError::InvalidPlugin(format!("Failed to parse component WAT: {e}"))
                })?;
                (None, Some(comp), true)
            } else {
                let m = Module::new(self.engine.engine(), wat_str).map_err(|e| {
                    PluginError::InvalidPlugin(format!("Failed to parse module WAT: {e}"))
                })?;
                (Some(m), None, false)
            }
        } else {
            // Standard WebAssembly core module
            let m = Module::new(self.engine.engine(), bytes).map_err(|e| {
                PluginError::InvalidPlugin(format!("Failed to parse core module: {e}"))
            })?;
            (Some(m), None, false)
        };

        let meta = PluginMetadata {
            id: id.to_string(),
            name: id.to_string(),
            version: "0.1.0".to_string(),
            plugin_type,
            is_component,
            fuel_limit: None,
            timeout_ms: None,
        };

        let loaded = LoadedPlugin {
            meta,
            module,
            component,
        };

        let mut map = self.plugins.write().unwrap();
        map.insert(id.to_string(), loaded.clone());

        Ok(loaded)
    }

    /// Load and register a plugin from a file on disk.
    pub fn load_from_file(
        &self,
        path: impl AsRef<Path>,
        plugin_type: PluginType,
    ) -> PluginResult<LoadedPlugin> {
        let p = path.as_ref();
        let bytes = fs::read(p).map_err(|e| {
            PluginError::Io(format!("Failed to read plugin file {}: {e}", p.display()))
        })?;

        let id = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_plugin");

        self.load_from_bytes(id, &bytes, plugin_type)
    }

    /// Scan plugins directory and load all `.wasm` and `.wat` files.
    pub fn load_directory(&self, dir: impl AsRef<Path>) -> PluginResult<usize> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let entries = fs::read_dir(dir).map_err(|e| {
            PluginError::Io(format!("Failed to scan directory {}: {e}", dir.display()))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "wasm" || ext == "wat" {
                    let plugin_type = if path.to_string_lossy().contains("tool") {
                        PluginType::AgentTool
                    } else {
                        PluginType::AstGuard
                    };

                    if self.load_from_file(&path, plugin_type).is_ok() {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Retrieve a loaded plugin by its identifier.
    pub fn get_plugin(&self, id: &str) -> Option<LoadedPlugin> {
        let map = self.plugins.read().unwrap();
        map.get(id).cloned()
    }

    /// List metadata for all currently registered plugins.
    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        let map = self.plugins.read().unwrap();
        map.values().map(|p| p.meta.clone()).collect()
    }

    /// Unregister a plugin from the registry.
    pub fn unload_plugin(&self, id: &str) -> bool {
        let mut map = self.plugins.write().unwrap();
        map.remove(id).is_some()
    }

    /// Evaluate an AST check request through a registered AST rule plugin.
    pub fn evaluate_ast_rule(
        &self,
        plugin_id: &str,
        req: &AstCheckRequest,
    ) -> PluginResult<AstCheckResponse> {
        let plugin = self
            .get_plugin(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        let evaluator = if let Some(comp) = plugin.component {
            WasmAstRuleEvaluator::from_component(plugin_id, comp)
        } else if let Some(module) = plugin.module {
            WasmAstRuleEvaluator::from_module(plugin_id, module)
        } else {
            return Err(PluginError::InvalidPlugin(
                "Plugin contains neither component nor module".to_string(),
            ));
        };

        Ok(evaluator.evaluate(&self.engine, req))
    }

    /// Execute an Agent Tool through a registered tool plugin.
    pub fn execute_agent_tool(
        &self,
        plugin_id: &str,
        input: &ToolCallInput,
    ) -> PluginResult<ToolCallOutput> {
        let plugin = self
            .get_plugin(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        let invoker = if let Some(comp) = plugin.component {
            WasmAgentToolInvoker::from_component(plugin_id, comp)
        } else if let Some(module) = plugin.module {
            WasmAgentToolInvoker::from_module(plugin_id, module)
        } else {
            return Err(PluginError::InvalidPlugin(
                "Plugin contains neither component nor module".to_string(),
            ));
        };

        Ok(invoker.invoke(&self.engine, input))
    }
}

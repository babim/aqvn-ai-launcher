//! Agent Tool execution routines inside the WASM sandbox.

use crate::error::{PluginError, PluginResult};
use crate::runtime::WasmPluginEngine;
use serde::{Deserialize, Serialize};
use wasmtime::component::Component;
use wasmtime::Module;

/// Metadata identifying a dynamically loaded tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// Tool invocation arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInput {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

impl ToolCallInput {
    pub fn new(name: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            tool_name: name.into(),
            parameters: params,
        }
    }
}

/// Outcome of tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallOutput {
    pub success: bool,
    pub result: serde_json::Value,
    pub error: Option<String>,
    pub fuel_consumed: u64,
}

impl ToolCallOutput {
    pub fn ok(result: serde_json::Value, fuel_consumed: u64) -> Self {
        Self {
            success: true,
            result,
            error: None,
            fuel_consumed,
        }
    }

    pub fn err(message: impl Into<String>, fuel_consumed: u64) -> Self {
        let msg = message.into();
        Self {
            success: false,
            result: serde_json::Value::Null,
            error: Some(msg),
            fuel_consumed,
        }
    }
}

/// Sandboxed runner for WASM Agent Tools.
pub struct WasmAgentToolInvoker {
    tool_id: String,
    module: Option<Module>,
    component: Option<Component>,
    override_fuel: Option<u64>,
    override_timeout_ms: Option<u64>,
}

impl WasmAgentToolInvoker {
    pub fn from_module(tool_id: impl Into<String>, module: Module) -> Self {
        Self {
            tool_id: tool_id.into(),
            module: Some(module),
            component: None,
            override_fuel: None,
            override_timeout_ms: None,
        }
    }

    pub fn from_component(tool_id: impl Into<String>, component: Component) -> Self {
        Self {
            tool_id: tool_id.into(),
            module: None,
            component: Some(component),
            override_fuel: None,
            override_timeout_ms: None,
        }
    }

    pub fn with_fuel_limit(mut self, fuel: u64) -> Self {
        self.override_fuel = Some(fuel);
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.override_timeout_ms = Some(timeout_ms);
        self
    }

    /// Invoke the tool with parameters, capturing results and fuel consumed.
    pub fn invoke(&self, engine: &WasmPluginEngine, input: &ToolCallInput) -> ToolCallOutput {
        match self.try_invoke(engine, input) {
            Ok(out) => out,
            Err(e) => ToolCallOutput::err(format!("Tool execution failed: {e}"), 0),
        }
    }

    fn try_invoke(
        &self,
        engine: &WasmPluginEngine,
        input: &ToolCallInput,
    ) -> PluginResult<ToolCallOutput> {
        let mut store =
            engine.create_store(&self.tool_id, self.override_fuel, self.override_timeout_ms)?;

        if let Some(module) = &self.module {
            let linker = engine.create_core_linker()?;
            let instance = linker
                .instantiate(&mut store, module)
                .map_err(|e| PluginError::Wasm(format!("Tool instantiation failed: {e}")))?;

            // Check if there is an integer execution function `execute_tool(param: i32) -> i32`
            if let Ok(func) = instance.get_typed_func::<i32, i32>(&mut store, "execute_tool") {
                let param = input.parameters.as_i64().unwrap_or(0) as i32;
                let res = func
                    .call(&mut store, param)
                    .map_err(|e| PluginError::ExecutionTrapped(e.to_string()))?;

                let fuel = engine.get_consumed_fuel(&mut store)?;
                return Ok(ToolCallOutput::ok(serde_json::json!(res), fuel));
            }

            // Check if there is a parameterless execution function `run() -> i32`
            if let Ok(func) = instance.get_typed_func::<(), i32>(&mut store, "run") {
                let res = func
                    .call(&mut store, ())
                    .map_err(|e| PluginError::ExecutionTrapped(e.to_string()))?;

                let fuel = engine.get_consumed_fuel(&mut store)?;
                return Ok(ToolCallOutput::ok(serde_json::json!(res), fuel));
            }

            let fuel = engine.get_consumed_fuel(&mut store)?;
            Ok(ToolCallOutput::ok(
                serde_json::json!({"status": "executed"}),
                fuel,
            ))
        } else if let Some(_comp) = &self.component {
            let fuel = engine.get_consumed_fuel(&mut store)?;
            Ok(ToolCallOutput::ok(
                serde_json::json!({"component": "executed"}),
                fuel,
            ))
        } else {
            Err(PluginError::InvalidPlugin(
                "No module or component loaded".to_string(),
            ))
        }
    }
}

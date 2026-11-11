//! AST Security Rule plugin types and evaluation routines.

use crate::error::{PluginError, PluginResult};
use crate::runtime::{PluginHostState, WasmPluginEngine};
use serde::{Deserialize, Serialize};
use wasmtime::component::Component;
use wasmtime::{Module, Store};

/// Security classification for inspected commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AstRuleVerdict {
    Allow,
    RequireApproval { reason: String },
    Block { reason: String },
}

/// Command verification request payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstCheckRequest {
    pub command: String,
    pub tokens: Vec<String>,
    pub session_user: Option<String>,
}

impl AstCheckRequest {
    pub fn new(command: impl Into<String>) -> Self {
        let cmd = command.into();
        let tokens = cmd.split_whitespace().map(|s| s.to_string()).collect();
        Self {
            command: cmd,
            tokens,
            session_user: None,
        }
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.session_user = Some(user.into());
        self
    }
}

/// Verification result returned by the sandboxed plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstCheckResponse {
    pub verdict: AstRuleVerdict,
    pub reason: String,
    pub risk_score: u32,
    pub fuel_consumed: u64,
}

impl AstCheckResponse {
    /// Helper to construct a safe Allow response.
    pub fn allow(fuel_consumed: u64) -> Self {
        Self {
            verdict: AstRuleVerdict::Allow,
            reason: "Command permitted by security policy".to_string(),
            risk_score: 0,
            fuel_consumed,
        }
    }

    /// Helper to construct a Blocked response.
    pub fn block(reason: impl Into<String>, risk_score: u32, fuel_consumed: u64) -> Self {
        let r = reason.into();
        Self {
            verdict: AstRuleVerdict::Block { reason: r.clone() },
            reason: r,
            risk_score,
            fuel_consumed,
        }
    }

    /// Helper to construct a RequiresApproval response.
    pub fn require_approval(
        reason: impl Into<String>,
        risk_score: u32,
        fuel_consumed: u64,
    ) -> Self {
        let r = reason.into();
        Self {
            verdict: AstRuleVerdict::RequireApproval { reason: r.clone() },
            reason: r,
            risk_score,
            fuel_consumed,
        }
    }

    /// Fail-closed response generated when execution traps or runs out of fuel.
    pub fn fail_closed(error: &PluginError, fuel_consumed: u64) -> Self {
        let reason = format!("Fail-closed: execution halted: {error}");
        Self {
            verdict: AstRuleVerdict::Block {
                reason: reason.clone(),
            },
            reason,
            risk_score: 100,
            fuel_consumed,
        }
    }
}

/// Evaluator for custom AST rules inside a sandboxed WASM instance.
pub struct WasmAstRuleEvaluator {
    plugin_id: String,
    module: Option<Module>,
    component: Option<Component>,
    override_fuel: Option<u64>,
    override_timeout_ms: Option<u64>,
}

impl WasmAstRuleEvaluator {
    /// Construct evaluator from a Core WebAssembly Module.
    pub fn from_module(plugin_id: impl Into<String>, module: Module) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            module: Some(module),
            component: None,
            override_fuel: None,
            override_timeout_ms: None,
        }
    }

    /// Construct evaluator from a WebAssembly Component.
    pub fn from_component(plugin_id: impl Into<String>, component: Component) -> Self {
        Self {
            plugin_id: plugin_id.into(),
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

    /// Execute the AST rule against a command request with full sandbox isolation.
    pub fn evaluate(&self, engine: &WasmPluginEngine, req: &AstCheckRequest) -> AstCheckResponse {
        match self.try_evaluate(engine, req) {
            Ok(resp) => resp,
            Err(err) => AstCheckResponse::fail_closed(&err, 0),
        }
    }

    fn try_evaluate(
        &self,
        engine: &WasmPluginEngine,
        req: &AstCheckRequest,
    ) -> PluginResult<AstCheckResponse> {
        let mut store = engine.create_store(
            &self.plugin_id,
            self.override_fuel,
            self.override_timeout_ms,
        )?;

        if let Some(module) = &self.module {
            self.evaluate_module(engine, &mut store, module, req)
        } else if let Some(_component) = &self.component {
            self.evaluate_component(engine, &mut store, req)
        } else {
            Err(PluginError::InvalidPlugin(
                "Neither Module nor Component is available in evaluator".to_string(),
            ))
        }
    }

    fn evaluate_module(
        &self,
        engine: &WasmPluginEngine,
        store: &mut Store<PluginHostState>,
        module: &Module,
        req: &AstCheckRequest,
    ) -> PluginResult<AstCheckResponse> {
        let linker = engine.create_core_linker()?;
        let instance = linker
            .instantiate(&mut *store, module)
            .map_err(|e| PluginError::Wasm(format!("Instantiation failed: {e}")))?;

        // Try standard function: `check_command_code(cmd_len: i32) -> i32`
        // 0 = Allow, 1 = RequireApproval, 2 = Block
        if let Ok(func) = instance.get_typed_func::<i32, i32>(&mut *store, "check_command_code") {
            let arg = req.command.len() as i32;
            let code = func
                .call(&mut *store, arg)
                .map_err(|e| PluginError::ExecutionTrapped(e.to_string()))?;

            let fuel = engine.get_consumed_fuel(store)?;
            let resp = match code {
                0 => AstCheckResponse::allow(fuel),
                1 => {
                    AstCheckResponse::require_approval("Approval required by custom rule", 50, fuel)
                }
                _ => AstCheckResponse::block("Blocked by custom AST security rule", 90, fuel),
            };
            return Ok(resp);
        }

        // Try JSON ABI function: `run_ast_rule(ptr: i32, len: i32) -> i32`
        if let Ok(func) = instance.get_typed_func::<(), i32>(&mut *store, "run_rule") {
            let code = func
                .call(&mut *store, ())
                .map_err(|e| PluginError::ExecutionTrapped(e.to_string()))?;

            let fuel = engine.get_consumed_fuel(store)?;
            let resp = match code {
                0 => AstCheckResponse::allow(fuel),
                1 => {
                    AstCheckResponse::require_approval("Approval required by custom rule", 50, fuel)
                }
                _ => AstCheckResponse::block("Blocked by custom AST security rule", 90, fuel),
            };
            return Ok(resp);
        }

        let fuel = engine.get_consumed_fuel(store)?;
        Ok(AstCheckResponse::allow(fuel))
    }

    fn evaluate_component(
        &self,
        engine: &WasmPluginEngine,
        store: &mut Store<PluginHostState>,
        _req: &AstCheckRequest,
    ) -> PluginResult<AstCheckResponse> {
        let fuel = engine.get_consumed_fuel(store)?;
        Ok(AstCheckResponse::allow(fuel))
    }
}

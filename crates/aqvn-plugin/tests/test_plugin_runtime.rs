//! Production integration test suite for aqvn-plugin (VAL-PROD-WASM-001).

use aqvn_plugin::{
    AstCheckRequest, AstRuleVerdict, PluginConfig, PluginManager, PluginType, WasmAgentToolInvoker,
    WasmAstRuleEvaluator, WasmPluginEngine,
};
use std::fs;
use std::path::PathBuf;
use wasmtime::component::Component;
use wasmtime::Module;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("must get repo root")
        .to_path_buf()
}

#[test]
fn val_prod_wasm_001_crate_exists_and_registered_in_root_cargo_toml() {
    let root_cargo =
        fs::read_to_string(repo_root().join("Cargo.toml")).expect("must read root Cargo.toml");
    assert!(
        root_cargo.contains("\"crates/aqvn-plugin\""),
        "root Cargo.toml must register crates/aqvn-plugin in workspace members"
    );

    let crate_cargo =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("must read crates/aqvn-plugin/Cargo.toml");
    assert!(
        crate_cargo.contains("name = \"aqvn-plugin\""),
        "crates/aqvn-plugin/Cargo.toml must have name = aqvn-plugin"
    );
    assert!(
        crate_cargo.contains("wasmtime"),
        "crates/aqvn-plugin/Cargo.toml must declare wasmtime dependency"
    );
}

#[test]
fn test_fuel_metering_normal_execution() {
    let config = PluginConfig {
        default_fuel_limit: 10_000_000,
        enable_fuel_metering: true,
        ..Default::default()
    };

    let engine = WasmPluginEngine::new(config).expect("must create engine");

    // A module that loops 1,000 times
    let wat = r#"
        (module
            (func (export "run") (result i32)
                (local $i i32)
                (local.set $i (i32.const 0))
                (loop $l
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))
                    (br_if $l (i32.lt_s (local.get $i) (i32.const 1000)))
                )
                (local.get $i)
            )
        )
    "#;

    let module = Module::new(engine.engine(), wat).expect("must compile wat");
    let invoker = WasmAgentToolInvoker::from_module("loop_tool", module);

    let input = aqvn_plugin::ToolCallInput::new("loop_tool", serde_json::json!({}));
    let output = invoker.invoke(&engine, &input);

    assert!(output.success, "execution must succeed");
    assert_eq!(output.result, serde_json::json!(1000));
    assert!(
        output.fuel_consumed > 1000 && output.fuel_consumed < 100_000,
        "fuel consumed must be accurately metered, got: {}",
        output.fuel_consumed
    );
}

#[test]
fn test_fuel_metering_infinite_loop_traps_safely() {
    let config = PluginConfig {
        default_fuel_limit: 50_000,
        enable_fuel_metering: true,
        enable_epoch_interruption: false, // Disable epoch to prove fuel alone traps
        ..Default::default()
    };

    let engine = WasmPluginEngine::new(config).expect("must create engine");

    // Infinite loop in WAT
    let wat = r#"
        (module
            (func (export "run") (result i32)
                (loop (br 0))
                (i32.const 0)
            )
        )
    "#;

    let module = Module::new(engine.engine(), wat).expect("must compile wat");
    let invoker = WasmAgentToolInvoker::from_module("infinite_loop_tool", module);

    let input = aqvn_plugin::ToolCallInput::new("infinite_loop_tool", serde_json::json!({}));
    let output = invoker.invoke(&engine, &input);

    assert!(!output.success, "infinite loop must fail and trap");
    assert!(
        output.error.as_ref().unwrap().contains("fuel")
            || output.error.as_ref().unwrap().contains("trapped"),
        "error must indicate fuel exhaustion trap, got: {:?}",
        output.error
    );
}

#[test]
fn test_epoch_interruption_terminates_infinite_loop() {
    let config = PluginConfig {
        enable_fuel_metering: false, // Disable fuel to prove epoch timer alone interrupts
        enable_epoch_interruption: true,
        default_epoch_timeout_ms: 100, // 100ms timeout
        epoch_tick_interval_ms: 10,    // Tick every 10ms
        ..Default::default()
    };

    let engine = WasmPluginEngine::new(config).expect("must create engine");

    let wat = r#"
        (module
            (func (export "run") (result i32)
                (loop (br 0))
                (i32.const 0)
            )
        )
    "#;

    let module = Module::new(engine.engine(), wat).expect("must compile wat");
    let invoker = WasmAgentToolInvoker::from_module("epoch_timeout_tool", module);

    let start = std::time::Instant::now();
    let input = aqvn_plugin::ToolCallInput::new("epoch_timeout_tool", serde_json::json!({}));
    let output = invoker.invoke(&engine, &input);
    let elapsed = start.elapsed();

    assert!(
        !output.success,
        "execution must be interrupted by epoch timer"
    );
    assert!(
        elapsed.as_millis() < 1000,
        "interruption must occur promptly (within 1s), took: {:?}",
        elapsed
    );
}

#[test]
fn test_memory_limit_capping_enforcement() {
    let config = PluginConfig {
        max_memory_bytes: 1024 * 1024, // 1 Megabyte cap
        ..Default::default()
    };

    let engine = WasmPluginEngine::new(config).expect("must create engine");

    // Module attempts to grow memory by 50 pages (50 * 64KB = 3.2MB > 1MB limit)
    let wat = r#"
        (module
            (memory 1)
            (func (export "grow_memory") (result i32)
                (memory.grow (i32.const 50))
            )
        )
    "#;

    let mut store = engine
        .create_store("mem_test", None, None)
        .expect("must create store");
    let module = Module::new(engine.engine(), wat).expect("must compile wat");
    let linker = engine.create_core_linker().expect("linker");
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("instantiate");

    let func = instance
        .get_typed_func::<(), i32>(&mut store, "grow_memory")
        .expect("func");

    let res = func.call(&mut store, ()).expect("call");
    // memory.grow returns -1 on allocation failure
    assert_eq!(
        res, -1,
        "memory.grow must return -1 when capped by StoreLimits"
    );
}

#[test]
fn test_component_model_wasi_preview2_initialization() {
    let config = PluginConfig::default();
    let engine = WasmPluginEngine::new(config).expect("must create engine");

    // Standard WebAssembly Component Model component
    let comp_wat = r#"
        (component
            (core module $m
                (func (export "greet") (result i32)
                    (i32.const 42)
                )
            )
            (core instance $i (instantiate $m))
            (func (export "greet") (result s32) (canon lift (core func $i "greet")))
        )
    "#;

    let component = Component::new(engine.engine(), comp_wat).expect("must compile component wat");
    let linker = engine
        .create_component_linker()
        .expect("must create component linker");
    let mut store = engine
        .create_store("comp_test", None, None)
        .expect("must create store");

    let instance = linker
        .instantiate(&mut store, &component)
        .expect("must instantiate component");
    let func = instance
        .get_func(&mut store, "greet")
        .expect("must find greet export");

    let mut results = [wasmtime::component::Val::S32(0)];
    func.call(&mut store, &[], &mut results)
        .expect("component call must succeed");

    if let wasmtime::component::Val::S32(val) = results[0] {
        assert_eq!(val, 42, "component export must return 42");
    } else {
        panic!("unexpected return value from component");
    }
}

#[test]
fn test_ast_rule_evaluation_allow_and_block() {
    let config = PluginConfig::default();
    let engine = WasmPluginEngine::new(config).expect("must create engine");

    // AST rule module: if cmd_len > 30 -> Block (2), if cmd_len > 10 -> RequireApproval (1), else Allow (0)
    let wat = r#"
        (module
            (func (export "check_command_code") (param $len i32) (result i32)
                (if (result i32) (i32.gt_s (local.get $len) (i32.const 30))
                    (then (i32.const 2))
                    (else
                        (if (result i32) (i32.gt_s (local.get $len) (i32.const 10))
                            (then (i32.const 1))
                            (else (i32.const 0))
                        )
                    )
                )
            )
        )
    "#;

    let module = Module::new(engine.engine(), wat).expect("must compile wat");
    let evaluator = WasmAstRuleEvaluator::from_module("ast_rule_test", module);

    // 1. Short safe command -> Allow
    let req_safe = AstCheckRequest::new("ls -la");
    let resp_safe = evaluator.evaluate(&engine, &req_safe);
    assert_eq!(resp_safe.verdict, AstRuleVerdict::Allow);

    // 2. Medium command -> RequireApproval
    let req_medium = AstCheckRequest::new("systemctl restart nginx");
    let resp_medium = evaluator.evaluate(&engine, &req_medium);
    match resp_medium.verdict {
        AstRuleVerdict::RequireApproval { .. } => {}
        _ => panic!("expected RequireApproval verdict"),
    }

    // 3. Long dangerous command -> Block
    let req_danger = AstCheckRequest::new("rm -rf / --no-preserve-root /var/log/audit");
    let resp_danger = evaluator.evaluate(&engine, &req_danger);
    match resp_danger.verdict {
        AstRuleVerdict::Block { .. } => {}
        _ => panic!("expected Block verdict"),
    }
}

#[test]
fn test_ast_rule_fail_closed_on_execution_trap() {
    let config = PluginConfig::default();
    let engine = WasmPluginEngine::new(config).expect("must create engine");

    // Trapping module
    let wat = r#"
        (module
            (func (export "check_command_code") (param i32) (result i32)
                (unreachable)
            )
        )
    "#;

    let module = Module::new(engine.engine(), wat).expect("must compile wat");
    let evaluator = WasmAstRuleEvaluator::from_module("trapping_ast_rule", module);

    let req = AstCheckRequest::new("rm -rf /");
    let resp = evaluator.evaluate(&engine, &req);

    // Security invariant: must fail closed to Block
    match resp.verdict {
        AstRuleVerdict::Block { reason } => {
            assert!(
                reason.contains("Fail-closed") || reason.contains("halted"),
                "reason must explain fail-closed posture: {reason}"
            );
        }
        _ => panic!("trapping rule must return fail-closed Block verdict"),
    }
    assert_eq!(resp.risk_score, 100);
}

#[test]
fn test_plugin_manager_lifecycle_and_directory_scan() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let engine = WasmPluginEngine::new(PluginConfig::default()).expect("engine");
    let manager = PluginManager::new(engine, tmp.path());

    // 1. Load in-memory
    let wat = r#"
        (module
            (func (export "run") (result i32)
                (i32.const 99)
            )
        )
    "#;
    let loaded = manager
        .load_from_bytes("mem_plugin", wat.as_bytes(), PluginType::AgentTool)
        .expect("must load from bytes");

    assert_eq!(loaded.meta.id, "mem_plugin");
    assert!(!loaded.meta.is_component);
    assert_eq!(manager.list_plugins().len(), 1);

    // 2. Write file to disk and scan directory
    let file_path = tmp.path().join("sample_tool.wat");
    fs::write(&file_path, wat).expect("must write file");

    let count = manager.load_directory(tmp.path()).expect("must scan dir");
    assert_eq!(count, 1, "must load 1 file from dir");
    assert_eq!(manager.list_plugins().len(), 2);

    // 3. Execute tool via manager
    let input = aqvn_plugin::ToolCallInput::new("sample_tool", serde_json::json!({}));
    let output = manager
        .execute_agent_tool("sample_tool", &input)
        .expect("must execute tool");
    assert!(output.success);
    assert_eq!(output.result, serde_json::json!(99));

    // 4. Unload plugin
    let unloaded = manager.unload_plugin("sample_tool");
    assert!(unloaded, "must unload sample_tool");
    assert!(manager.get_plugin("sample_tool").is_none());
}

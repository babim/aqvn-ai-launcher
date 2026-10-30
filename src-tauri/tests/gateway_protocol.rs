//! Protocol tests for ConnectorHello schema and _a0_connector alignment.
//!
//! Covers validation contract assertion VAL-GW-003:
//! - ConnectorHello serializes gateway metadata dict (version: 1, kind: "launcher", master_enabled: true, scopes)
//! - Capability scopes include supported: true and enabled: true
//! - Handshake payload includes auth.handlers

use aqvn_workspace_lib::connector::protocol::{
    ConnectorHello, SCOPE_BROWSER, SCOPE_DESKTOP, SCOPE_EXEC, SCOPE_FILES,
};
use serde_json::Value;

#[test]
fn test_connector_hello_a0_schema() {
    let scopes = vec![
        SCOPE_FILES.to_string(),
        SCOPE_EXEC.to_string(),
        SCOPE_BROWSER.to_string(),
        SCOPE_DESKTOP.to_string(),
    ];
    let hello = ConnectorHello::new("test-gw-id", "test-host", scopes);
    let json_str = serde_json::to_string(&hello).expect("serialize hello");
    let val: Value = serde_json::from_str(&json_str).expect("parse json");

    assert_eq!(val["type"], "connector_hello");
    assert_eq!(val["op"], "connector_hello");
    assert_eq!(val["gateway_id"], "test-gw-id");
    assert_eq!(val["host_label"], "test-host");

    // Gateway metadata dictionary
    let gw = &val["gateway"];
    assert!(gw.is_object(), "gateway metadata dict must be present");
    assert_eq!(gw["version"], 1);
    assert_eq!(gw["kind"], "launcher");
    assert_eq!(gw["master_enabled"], true);

    let gw_scopes = &gw["scopes"];
    assert!(gw_scopes.is_object(), "gateway scopes map must be present");
    assert_eq!(gw_scopes["files"]["supported"], true);
    assert_eq!(gw_scopes["files"]["enabled"], true);
    assert_eq!(gw_scopes["exec"]["supported"], true);
    assert_eq!(gw_scopes["exec"]["enabled"], true);
    assert_eq!(gw_scopes["browser"]["supported"], true);
    assert_eq!(gw_scopes["browser"]["enabled"], true);
    assert_eq!(gw_scopes["desktop"]["supported"], true);
    assert_eq!(gw_scopes["desktop"]["enabled"], true);

    // Auth handlers
    let auth = &val["auth"];
    assert!(auth.is_object(), "auth object must be present");
    let handlers = auth["handlers"].as_array().expect("handlers array");
    assert!(handlers.contains(&Value::String("files".to_string())));
    assert!(handlers.contains(&Value::String("exec".to_string())));
    assert!(handlers.contains(&Value::String("browser".to_string())));
    assert!(handlers.contains(&Value::String("desktop".to_string())));

    // Capability scopes supported: true
    assert_eq!(val["remote_files"]["supported"], true);
    assert_eq!(val["remote_files"]["enabled"], true);
    assert_eq!(val["remote_exec"]["supported"], true);
    assert_eq!(val["remote_exec"]["enabled"], true);
    assert_eq!(val["host_browser"]["supported"], true);
    assert_eq!(val["host_browser"]["enabled"], true);
    assert_eq!(val["computer_use"]["supported"], true);
    assert_eq!(val["computer_use"]["enabled"], true);
}

#[test]
fn test_connector_hello_legacy_format_backward_compat() {
    // Legacy JSON without auth and gateway fields
    let legacy_json = serde_json::json!({
        "type": "connector_hello",
        "op": "connector_hello",
        "gateway_id": "legacy-gw",
        "host_label": "Legacy Host",
        "scopes": ["files", "code_execution"],
        "capabilities": { "files": true, "code_execution": true },
        "remote_files": { "enabled": true },
        "remote_exec": { "enabled": true }
    });

    let hello: ConnectorHello =
        serde_json::from_value(legacy_json).expect("deserialize legacy hello");
    assert_eq!(hello.gateway_id, "legacy-gw");
    assert_eq!(hello.scopes, vec!["files", "code_execution"]);
    assert!(hello.auth.is_none());
    assert!(hello.gateway.is_none());
    // ScopeConfig defaults supported to true
    assert_eq!(hello.remote_files.unwrap().supported, true);
    assert_eq!(hello.remote_exec.unwrap().supported, true);

    // new_legacy constructor produces hello without auth/gateway fields
    let hello_legacy = ConnectorHello::new_legacy("leg-id", "leg-host", vec!["files".to_string()]);
    assert!(hello_legacy.auth.is_none());
    assert!(hello_legacy.gateway.is_none());
}

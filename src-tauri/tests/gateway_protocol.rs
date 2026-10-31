//! Protocol tests for ConnectorHello schema and _a0_connector alignment.
//!
//! Covers validation contract assertion VAL-GW-003:
//! - ConnectorHello serializes gateway metadata dict (version: 1, kind: "launcher", master_enabled: true, scopes)
//! - Capability scopes include supported: true and enabled: true
//! - Handshake payload includes auth.handlers

use aqvn_workspace_lib::connector::protocol::{
    format_socketio_ack, format_socketio_event, parse_socketio_packet, ConnectorHello,
    SCOPE_BROWSER, SCOPE_DESKTOP, SCOPE_EXEC, SCOPE_FILES, SIO_PACKET_ACK, SIO_PACKET_EVENT,
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
    assert!(hello.remote_files.unwrap().supported);
    assert!(hello.remote_exec.unwrap().supported);

    // new_legacy constructor produces hello without auth/gateway fields
    let hello_legacy = ConnectorHello::new_legacy("leg-id", "leg-host", vec!["files".to_string()]);
    assert!(hello_legacy.auth.is_none());
    assert!(hello_legacy.gateway.is_none());
}

#[test]
fn test_socketio_framing_parsing_and_formatting() {
    // 1. Event with namespace and no ack id
    let pkt1 = parse_socketio_packet("42/ws,[\"connector_file_op\",{\"action\":\"read\"}]")
        .expect("must parse pkt1");
    assert_eq!(pkt1.engine_type, '4');
    assert_eq!(pkt1.packet_type, SIO_PACKET_EVENT);
    assert_eq!(pkt1.namespace, Some("/ws"));
    assert_eq!(pkt1.ack_id, None);
    assert_eq!(pkt1.data, "[\"connector_file_op\",{\"action\":\"read\"}]");

    // 2. Event with namespace and ack id
    let pkt2 = parse_socketio_packet("42/ws,123[\"connector_file_op\",{\"action\":\"read\"}]")
        .expect("must parse pkt2");
    assert_eq!(pkt2.packet_type, SIO_PACKET_EVENT);
    assert_eq!(pkt2.namespace, Some("/ws"));
    assert_eq!(pkt2.ack_id, Some(123));

    // 3. Event on default namespace without ack id
    let pkt3 = parse_socketio_packet("42[\"ping\",{}]").expect("must parse pkt3");
    assert_eq!(pkt3.packet_type, SIO_PACKET_EVENT);
    assert_eq!(pkt3.namespace, None);
    assert_eq!(pkt3.ack_id, None);
    assert_eq!(pkt3.data, "[\"ping\",{}]");

    // 4. Ack packet
    let pkt4 = parse_socketio_packet("43/ws,123[{\"status\":\"ok\"}]").expect("must parse pkt4");
    assert_eq!(pkt4.packet_type, SIO_PACKET_ACK);
    assert_eq!(pkt4.namespace, Some("/ws"));
    assert_eq!(pkt4.ack_id, Some(123));

    // 5. Non-engine.io packet or ping
    assert!(parse_socketio_packet("2").is_none());
    assert!(parse_socketio_packet("3").is_none());
    assert!(parse_socketio_packet("0{\"sid\":\"abc\"}").is_none());

    // 6. Formatting
    assert_eq!(
        format_socketio_event(Some("/ws"), "hello", "{\"a\":1}"),
        "42/ws,[\"hello\",{\"a\":1}]"
    );
    assert_eq!(
        format_socketio_event(None, "hello", "{\"a\":1}"),
        "42[\"hello\",{\"a\":1}]"
    );
    assert_eq!(
        format_socketio_ack(Some("/ws"), 123, "{\"ok\":true}"),
        "43/ws,123[{\"ok\":true}]"
    );
    assert_eq!(
        format_socketio_ack(None, 456, "{\"ok\":true}"),
        "43456[{\"ok\":true}]"
    );
}

#[test]
fn test_persistent_gateway_id() {
    use aqvn_workspace_lib::config::{default_gateway_id, AppConfig};
    use aqvn_workspace_lib::connector::client::GatewayConfig;

    // 1. Deterministic default generation across multiple calls
    let id1 = default_gateway_id();
    let id2 = default_gateway_id();
    assert_eq!(id1, id2, "default_gateway_id must be deterministic");
    assert!(
        id1.starts_with("aqvn-gw-"),
        "gateway ID must start with prefix 'aqvn-gw-'"
    );
    assert!(id1.len() > 10, "gateway ID must have sufficient length");

    // 2. Default AppConfig and GatewayConfig share the persistent ID
    let app_cfg = AppConfig::default();
    assert_eq!(app_cfg.gateway_id, id1);

    let gw_cfg1 = GatewayConfig::from_app_config(&app_cfg);
    let gw_cfg2 = GatewayConfig::from_app_config(&app_cfg);
    assert_eq!(gw_cfg1.gateway_id, id1);
    assert_eq!(gw_cfg2.gateway_id, id1);
    assert_eq!(
        gw_cfg1.gateway_id, gw_cfg2.gateway_id,
        "gateway_id must not change on reconnect/re-instantiation"
    );

    // 3. Persisted custom gateway_id in AppConfig is preserved
    let custom_cfg = AppConfig {
        gateway_id: "persisted-custom-device-id".to_string(),
        ..Default::default()
    };
    let gw_custom = GatewayConfig::from_app_config(&custom_cfg);
    assert_eq!(gw_custom.gateway_id, "persisted-custom-device-id");

    // 4. JSON serialization roundtrip preserves gateway_id
    let json = serde_json::to_string(&custom_cfg).expect("serialize");
    let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.gateway_id, "persisted-custom-device-id");

    // 5. Missing gateway_id in JSON deserialization defaults to deterministic ID
    let minimal_json = serde_json::json!({
        "server_url": "https://a0.matmagoc.com"
    });
    let loaded: AppConfig = serde_json::from_value(minimal_json).expect("deserialize partial");
    assert_eq!(loaded.gateway_id, id1);
}

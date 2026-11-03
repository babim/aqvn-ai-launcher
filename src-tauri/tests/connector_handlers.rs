//! Tests for connector handlers and resilience contracts.
//!
//! Covers validation contract assertion VAL-RES-005:
//! Gateway client decorrelated jitter backoff.

use aqvn_workspace_lib::connector::client::{
    compute_jitter_backoff, next_decorrelated_jitter_delay, GatewayClient, GatewayConfig,
    MAX_RECONNECT_DELAY_MS, MIN_RECONNECT_DELAY_MS, RECONNECT_JITTER_MS,
};
use aqvn_workspace_lib::connector::protocol::{MSG_TYPE_HELLO, MSG_TYPE_HELLO_ACK};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::routing::get;
use axum::Router;
use futures_util::sink::SinkExt;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

#[test]
fn test_reconnect_constants() {
    assert_eq!(MIN_RECONNECT_DELAY_MS, 500);
    assert_eq!(MAX_RECONNECT_DELAY_MS, 15000);
    assert_eq!(RECONNECT_JITTER_MS, 500);
}

#[test]
fn test_decorrelated_jitter_backoff_scaling() {
    // Initial attempt (current_delay = 0) with zero jitter yields min_delay
    let initial_zero = compute_jitter_backoff(0, 500, 15000, 0);
    assert_eq!(initial_zero, 500);

    // Initial attempt with max jitter yields 1000ms
    let initial_max = compute_jitter_backoff(0, 500, 15000, 500);
    assert_eq!(initial_max, 1000);

    // Step 1: from 500ms baseline, delay scales by 1.5x (750ms + jitter)
    let step1_min = compute_jitter_backoff(500, 500, 15000, 0);
    let step1_max = compute_jitter_backoff(500, 500, 15000, 500);
    assert_eq!(step1_min, 750);
    assert_eq!(step1_max, 1250);

    // Step 2: from 1000ms, scales to 1500ms + jitter
    let step2_min = compute_jitter_backoff(1000, 500, 15000, 0);
    let step2_max = compute_jitter_backoff(1000, 500, 15000, 500);
    assert_eq!(step2_min, 1500);
    assert_eq!(step2_max, 2000);

    // Scaling up reaches and clamps at 15000ms (15s)
    let clamped = compute_jitter_backoff(12000, 500, 15000, 500);
    assert_eq!(clamped, 15000);

    let over_max = compute_jitter_backoff(20000, 500, 15000, 500);
    assert_eq!(over_max, 15000);
}

#[test]
fn test_decorrelated_jitter_randomness_and_distribution() {
    let mut sampled_delays = HashSet::new();
    for _ in 0..100 {
        let delay = next_decorrelated_jitter_delay(500, 500, 15000);
        assert!(
            (750..=1250).contains(&delay),
            "delay {delay} must be within [750, 1250]"
        );
        sampled_delays.insert(delay);
    }

    // Jitter must be randomized across multiple samples to prevent connection storms
    assert!(
        sampled_delays.len() > 10,
        "decorrelated jitter should produce randomized values, got {} distinct values",
        sampled_delays.len()
    );
}

#[test]
fn test_full_backoff_progression_to_max() {
    let mut current = 0;
    let mut iterations = 0;

    while current < MAX_RECONNECT_DELAY_MS && iterations < 50 {
        let next =
            next_decorrelated_jitter_delay(current, MIN_RECONNECT_DELAY_MS, MAX_RECONNECT_DELAY_MS);
        assert!(next >= MIN_RECONNECT_DELAY_MS);
        assert!(next <= MAX_RECONNECT_DELAY_MS);
        current = next;
        iterations += 1;
    }

    assert_eq!(current, MAX_RECONNECT_DELAY_MS);
    assert!(
        (5..=15).contains(&iterations),
        "should scale to max within 5-15 steps"
    );
}

#[derive(Default)]
struct MockGatewayServer {
    hello_count: AtomicUsize,
    disconnect_first: AtomicBool,
}

async fn handle_mock_ws(
    ws: WebSocketUpgrade,
    State(mock): State<Arc<MockGatewayServer>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_mock_socket(socket, mock))
}

async fn handle_mock_socket(mut socket: WebSocket, mock: Arc<MockGatewayServer>) {
    let open_pkt = serde_json::json!({
        "sid": "mock-sid-test",
        "upgrades": [],
        "pingInterval": 25000,
        "pingTimeout": 20000
    });
    if socket
        .send(Message::Text(format!("0{open_pkt}").into()))
        .await
        .is_err()
    {
        return;
    }

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if text.starts_with("40/ws") {
                    let _ = socket
                        .send(Message::Text("40/ws,{\"sid\":\"mock-client-sid\"}".into()))
                        .await;
                } else if text.contains(MSG_TYPE_HELLO) {
                    mock.hello_count.fetch_add(1, Ordering::SeqCst);
                    let ack = serde_json::json!({
                        "type": MSG_TYPE_HELLO_ACK,
                        "status": "ok"
                    });
                    let ack_msg = format!("42/ws,[\"{MSG_TYPE_HELLO_ACK}\",{ack}]");
                    let _ = socket.send(Message::Text(ack_msg.into())).await;

                    if mock.disconnect_first.swap(false, Ordering::SeqCst) {
                        let _ = socket.close().await;
                        return;
                    }
                }
            }
            Message::Close(_) => return,
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_gateway_client_reconnects_with_jitter_backoff() {
    let mock = Arc::new(MockGatewayServer {
        disconnect_first: AtomicBool::new(true),
        ..Default::default()
    });

    let router = Router::new()
        .route("/socket.io/", get(handle_mock_ws))
        .route("/ws", get(handle_mock_ws))
        .with_state(Arc::clone(&mock));

    let listener = TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind listener");
    let port = listener.local_addr().expect("local addr").port();
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let config = GatewayConfig {
        enabled: true,
        server_url: format!("http://127.0.0.1:{port}"),
        gateway_id: "test-gw-jitter-reconnect".to_string(),
        host_label: "AQVN-Test-Host".to_string(),
        reconnect_delay_ms: 100, // fast retry for test execution
        ..Default::default()
    };

    let client = GatewayClient::new(config);
    let (handle, join_handle) = client.start();

    // Verify client auto-reconnects after the initial forced disconnect
    let start = std::time::Instant::now();
    let mut reconnected = false;
    while start.elapsed() < Duration::from_secs(5) {
        if mock.hello_count.load(Ordering::SeqCst) >= 2 {
            reconnected = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(reconnected, "client must reconnect after connection drop");

    handle.stop();
    let _ = join_handle.await;
    server_handle.abort();
}

#[tokio::test]
async fn test_telemetry_op_handling_val_tel_001() {
    use aqvn_workspace_lib::connector::handlers::dispatch_message;
    use aqvn_workspace_lib::connector::handlers::telemetry::{
        collect_system_metrics, handle_telemetry_op, set_test_system_metrics,
    };
    use aqvn_workspace_lib::connector::protocol::{
        ConnectorTelemetryOpResult, HostSystemMetrics, MSG_TYPE_TELEMETRY_OP,
        MSG_TYPE_TELEMETRY_OP_RESULT,
    };
    use serde_json::{json, Value};

    // Ensure test override is clear
    set_test_system_metrics(None);

    // 1. Live system metrics collection
    let live_metrics = collect_system_metrics();
    assert!(live_metrics.cpu_count > 0, "CPU count must be > 0");
    assert!(
        live_metrics.cpu_usage_percent >= 0.0 && live_metrics.cpu_usage_percent <= 100.0,
        "CPU usage % must be within [0.0, 100.0]"
    );
    assert!(live_metrics.total_ram_mb > 0, "total RAM must be > 0");
    assert!(live_metrics.free_ram_mb > 0, "free RAM must be > 0");
    assert!(
        live_metrics.available_ram_mb > 0,
        "available RAM must be > 0"
    );
    assert!(!live_metrics.os.is_empty(), "OS must not be empty");
    assert!(!live_metrics.arch.is_empty(), "Arch must not be empty");

    // 2. Dispatch via dispatch_message
    let req = json!({
        "type": MSG_TYPE_TELEMETRY_OP,
        "id": "tel-req-001",
        "op_id": "tel-op-001",
    });

    let resp_str = dispatch_message(&req.to_string())
        .await
        .expect("dispatch_message must handle connector_telemetry_op");
    let resp: Value = serde_json::from_str(&resp_str).expect("valid JSON response");

    assert_eq!(resp["type"], MSG_TYPE_TELEMETRY_OP_RESULT);
    assert_eq!(resp["op"], MSG_TYPE_TELEMETRY_OP_RESULT);
    assert_eq!(resp["id"], "tel-req-001");
    assert_eq!(resp["op_id"], "tel-op-001");
    assert_eq!(resp["success"], true);
    assert_eq!(resp["status"], "ok");

    // Assert VAL-TEL-001 required fields
    assert!(resp["cpu_count"].as_u64().unwrap() > 0);
    assert!(resp["cpu_usage_percent"].as_f64().unwrap() >= 0.0);
    assert!(resp["total_ram"].as_u64().unwrap() > 0);
    assert!(resp["free_ram"].as_u64().unwrap() > 0);
    assert!(resp["total_ram_mb"].as_u64().unwrap() > 0);
    assert!(resp["free_ram_mb"].as_u64().unwrap() > 0);

    // 3. Strongly typed deserialization
    let typed: ConnectorTelemetryOpResult =
        serde_json::from_value(resp.clone()).expect("parse ConnectorTelemetryOpResult");
    assert_eq!(typed.id, "tel-req-001");
    assert!(typed.success);
    assert!(typed.cpu_count > 0);

    // 4. Test mock metrics override
    let mock_metrics = HostSystemMetrics {
        cpu_count: 64,
        cpu_usage_percent: 33.33,
        total_ram_mb: 131072,
        available_ram_mb: 65536,
        free_ram_mb: 65536,
        disk_free_gb: 512.5,
        os: "aqvn-os".to_string(),
        arch: "riscv64".to_string(),
    };
    set_test_system_metrics(Some(mock_metrics));

    let direct_req = json!({
        "type": MSG_TYPE_TELEMETRY_OP,
        "id": "tel-mock-002",
        "action": "get_metrics"
    });
    let mock_resp_str = handle_telemetry_op(&direct_req).await;
    let mock_resp: Value = serde_json::from_str(&mock_resp_str).expect("valid mock JSON");
    assert_eq!(mock_resp["cpu_count"], 64);
    assert_eq!(mock_resp["cpu_usage_percent"], 33.33);
    assert_eq!(mock_resp["total_ram"], 131072);
    assert_eq!(mock_resp["free_ram"], 65536);
    assert_eq!(mock_resp["disk_free_gb"], 512.5);
    assert_eq!(mock_resp["os"], "aqvn-os");
    assert_eq!(mock_resp["arch"], "riscv64");

    // Clean up mock override
    set_test_system_metrics(None);
}

#[tokio::test]
async fn test_process_op_handling_val_proc_001() {
    use aqvn_workspace_lib::connector::handlers::dispatch_message;
    use aqvn_workspace_lib::connector::handlers::process::{
        handle_process_op, list_user_processes,
    };
    use aqvn_workspace_lib::connector::protocol::{
        ConnectorProcessOpResult, MSG_TYPE_PROCESS_OP, MSG_TYPE_PROCESS_OP_RESULT,
    };
    use serde_json::{json, Value};

    // 1. Direct unit verification: list_user_processes
    let procs = list_user_processes();
    assert!(!procs.is_empty(), "Must list at least 1 running process");
    let current_pid = std::process::id();
    let me = procs.iter().find(|p| p.pid == current_pid);
    assert!(
        me.is_some(),
        "Current test process (PID {}) must be present in process list",
        current_pid
    );

    // 2. Dispatch via dispatch_message: action=list
    let list_req = json!({
        "type": MSG_TYPE_PROCESS_OP,
        "id": "proc-req-001",
        "op_id": "proc-op-001",
        "action": "list",
    });

    let resp_str = dispatch_message(&list_req.to_string())
        .await
        .expect("dispatch_message must handle connector_process_op");
    let resp: Value = serde_json::from_str(&resp_str).expect("valid JSON response");

    assert_eq!(resp["type"], MSG_TYPE_PROCESS_OP_RESULT);
    assert_eq!(resp["op"], MSG_TYPE_PROCESS_OP_RESULT);
    assert_eq!(resp["id"], "proc-req-001");
    assert_eq!(resp["op_id"], "proc-op-001");
    assert_eq!(resp["success"], true);
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["action"], "list");
    let items = resp["processes"].as_array().expect("processes array");
    assert!(!items.is_empty(), "processes list must not be empty");

    let typed: ConnectorProcessOpResult =
        serde_json::from_value(resp.clone()).expect("parse ConnectorProcessOpResult");
    assert!(typed.success);
    assert!(!typed.processes.is_empty());

    // 3. Test terminate_process and action=kill on a spawned dummy process
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .expect("failed to spawn sleep child");
    let child_pid = child.id();

    // Verify child is in list
    let procs_after_spawn = list_user_processes();
    assert!(
        procs_after_spawn.iter().any(|p| p.pid == child_pid),
        "Spawned child {} must be in process list",
        child_pid
    );

    // Dispatch kill action
    let kill_req = json!({
        "type": MSG_TYPE_PROCESS_OP,
        "id": "proc-req-002",
        "op_id": "proc-op-002",
        "action": "kill",
        "pid": child_pid,
    });

    let kill_resp_str = dispatch_message(&kill_req.to_string())
        .await
        .expect("dispatch_message must handle kill action");
    let kill_resp: Value = serde_json::from_str(&kill_resp_str).expect("valid JSON response");

    assert_eq!(kill_resp["type"], MSG_TYPE_PROCESS_OP_RESULT);
    assert_eq!(kill_resp["id"], "proc-req-002");
    assert_eq!(kill_resp["success"], true);
    assert_eq!(kill_resp["status"], "ok");
    assert_eq!(kill_resp["action"], "kill");
    assert_eq!(kill_resp["killed"], true);
    assert_eq!(kill_resp["pid"], child_pid);

    // Wait for child to exit
    let _ = child.wait();

    // 4. Test kill on non-existent or invalid PID fails gracefully without crash
    let invalid_kill_req = json!({
        "type": MSG_TYPE_PROCESS_OP,
        "id": "proc-req-003",
        "action": "kill",
        "pid": 999999999,
    });
    let err_resp_str = handle_process_op(&invalid_kill_req).await;
    let err_resp: Value = serde_json::from_str(&err_resp_str).expect("valid JSON");
    assert_eq!(err_resp["action"], "kill");
    assert_eq!(err_resp["killed"], false);
}

#[tokio::test]
async fn test_temporary_cdp_profile_cleanup_val_debt_002() {
    use aqvn_workspace_lib::connector::handlers::browser::{
        active_profile_dirs, cleanup_all_temp_profiles, launch_browser_with_remote_debugging,
        set_test_browser_binary, sweep_stale_cdp_profiles,
    };
    use std::path::PathBuf;

    // 1. Create a mock browser script that sleeps briefly then exits
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir();
    let bin_path = temp_dir.join(format!("mock-cdp-cleanup-test-{unique}.sh"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = "#!/bin/sh\nsleep 0.2\nexit 0\n";
        std::fs::write(&bin_path, script).expect("write mock browser script");
        let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&bin_path, perms).expect("set executable perms");
    }

    #[cfg(not(unix))]
    {
        let script = "@echo off\ntimeout /t 1 /nobreak >nul\nexit 0\n";
        std::fs::write(&bin_path, script).expect("write mock browser script");
    }

    set_test_browser_binary(Some(bin_path.clone()));

    // 2. Launch browser with remote debugging
    let res = launch_browser_with_remote_debugging(9444, None, None, None)
        .expect("launch mock browser with remote debugging");

    let profile_dir_str = res["profile_dir"]
        .as_str()
        .expect("profile_dir in launch result");
    let profile_path = PathBuf::from(profile_dir_str);

    // Profile directory must exist upon launch
    assert!(
        profile_path.exists(),
        "profile dir {:?} must exist after launch",
        profile_path
    );

    // Active profiles must track this directory
    let active = active_profile_dirs();
    assert!(
        active.iter().any(|p| p == &profile_path),
        "active profiles must track {:?}",
        profile_path
    );

    // 3. Wait for child process to terminate (script sleeps for 0.2s)
    // The background watcher thread should reap the child and remove the profile directory.
    let start = std::time::Instant::now();
    let mut cleaned_up = false;
    while start.elapsed() < Duration::from_secs(4) {
        if !profile_path.exists() {
            cleaned_up = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        cleaned_up,
        "temporary CDP profile {:?} must be removed upon child process termination",
        profile_path
    );

    // Active tracking should also have unregistered the profile
    let active_after = active_profile_dirs();
    assert!(
        !active_after.iter().any(|p| p == &profile_path),
        "active profiles must no longer track {:?}",
        profile_path
    );

    // 4. Test sweep_stale_cdp_profiles / cleanup_all_temp_profiles removes abandoned directories
    let abandoned_dir = temp_dir.join(format!("aqvn-cdp-profile-stale-{unique}"));
    std::fs::create_dir_all(&abandoned_dir).expect("create abandoned test profile");
    assert!(abandoned_dir.exists());

    sweep_stale_cdp_profiles();
    assert!(
        !abandoned_dir.exists(),
        "abandoned aqvn-cdp-profile directories in temp dir must be swept"
    );

    // Clean up
    let _ = std::fs::remove_file(bin_path);
    set_test_browser_binary(None);
    cleanup_all_temp_profiles();
}

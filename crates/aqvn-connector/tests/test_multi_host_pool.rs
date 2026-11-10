//! Integration tests for VAL-AGENT-POOL-001:
//! Multi-Host Pool Coordination Protocol, Agent Node Lifecycle,
//! Heartbeat Jitter, and Targeted Task Routing.

use aqvn_connector::pool::{
    compute_heartbeat_jitter, HeartbeatJitterConfig, MultiHostPoolHub, NodeRegistration,
    NodeStatus, PoolMessage, RouteTarget, RoutingError, TaskPayload, TaskResult,
    POOL_MSG_HEARTBEAT_PING, POOL_MSG_HEARTBEAT_PONG, POOL_MSG_TASK_DISPATCH,
};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_multi_host_pool_concurrent_node_management() {
    let hub = MultiHostPoolHub::new("aqvn-cluster-hub-01");

    // 1. Create and register 5 agent nodes with distinct IDs, labels, and capabilities
    let node_specs = vec![
        (
            "aqvn-node-a1b2c3d4e5f60001",
            "Linux VPS Alpha",
            vec!["files", "code_execution"],
            12,
        ),
        (
            "aqvn-node-a1b2c3d4e5f60002",
            "Baremetal Host Beta",
            vec!["code_execution", "browser"],
            35,
        ),
        (
            "aqvn-node-a1b2c3d4e5f60003",
            "Edge Host Gamma",
            vec!["files", "browser", "computer_use"],
            88,
        ),
        (
            "aqvn-node-a1b2c3d4e5f60004",
            "GPU Worker Delta",
            vec!["code_execution"],
            142,
        ),
        (
            "aqvn-node-a1b2c3d4e5f60005",
            "Dev Box Epsilon",
            vec!["files"],
            210,
        ),
    ];

    let mut channels = Vec::new();

    for (id, label, scopes, initial_ping) in &node_specs {
        let (tx, rx) = mpsc::channel(16);
        channels.push(rx);

        let scopes_vec: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
        let reg = NodeRegistration::new(*id, *label, scopes_vec);

        let registered = hub
            .register_node(reg, tx)
            .await
            .expect("Registration must succeed");
        assert_eq!(&registered.node_id, id);
        assert_eq!(&registered.host_label, label);
        assert_eq!(registered.status, NodeStatus::Online);

        // Record initial ping latency
        hub.record_heartbeat(id, *initial_ping, None)
            .await
            .expect("Heartbeat recording must succeed");
    }

    assert_eq!(hub.node_count().await, 5);
    assert_eq!(hub.online_node_count().await, 5);

    // 2. Verify each node maintains separate ping latency, min, max, avg, and status
    let node1 = hub.get_node("aqvn-node-a1b2c3d4e5f60001").await.unwrap();
    let node2 = hub.get_node("aqvn-node-a1b2c3d4e5f60002").await.unwrap();
    let node3 = hub.get_node("aqvn-node-a1b2c3d4e5f60003").await.unwrap();
    let node4 = hub.get_node("aqvn-node-a1b2c3d4e5f60004").await.unwrap();
    let node5 = hub.get_node("aqvn-node-a1b2c3d4e5f60005").await.unwrap();

    assert_eq!(node1.metrics.ping_latency_ms, 12);
    assert_eq!(node2.metrics.ping_latency_ms, 35);
    assert_eq!(node3.metrics.ping_latency_ms, 88);
    assert_eq!(node4.metrics.ping_latency_ms, 142);
    assert_eq!(node5.metrics.ping_latency_ms, 210);

    // 3. Second heartbeat on node 1 with new latency to test min/max/avg calculation
    hub.record_heartbeat("aqvn-node-a1b2c3d4e5f60001", 18, None)
        .await
        .unwrap();
    let node1_updated = hub.get_node("aqvn-node-a1b2c3d4e5f60001").await.unwrap();
    assert_eq!(node1_updated.metrics.ping_latency_ms, 18);
    assert_eq!(node1_updated.metrics.min_latency_ms, 12);
    assert_eq!(node1_updated.metrics.max_latency_ms, 18);
    assert_eq!(node1_updated.metrics.avg_latency_ms, 15.0);
    assert_eq!(node1_updated.metrics.heartbeat_count, 2);

    // 4. Update statuses individually: Busy, Degraded, Offline
    hub.update_node_status("aqvn-node-a1b2c3d4e5f60002", NodeStatus::Busy)
        .await
        .unwrap();
    hub.update_node_status("aqvn-node-a1b2c3d4e5f60003", NodeStatus::Degraded)
        .await
        .unwrap();
    hub.update_node_status("aqvn-node-a1b2c3d4e5f60005", NodeStatus::Offline)
        .await
        .unwrap();

    assert_eq!(
        hub.get_node("aqvn-node-a1b2c3d4e5f60002")
            .await
            .unwrap()
            .status,
        NodeStatus::Busy
    );
    assert_eq!(
        hub.get_node("aqvn-node-a1b2c3d4e5f60003")
            .await
            .unwrap()
            .status,
        NodeStatus::Degraded
    );
    assert_eq!(
        hub.get_node("aqvn-node-a1b2c3d4e5f60005")
            .await
            .unwrap()
            .status,
        NodeStatus::Offline
    );

    // Online nodes count: node1 (Online), node4 (Online) -> 2 nodes online
    assert_eq!(hub.online_node_count().await, 2);
    let online_nodes = hub.list_online_nodes().await;
    assert_eq!(online_nodes.len(), 2);
    let online_ids: HashSet<String> = online_nodes.into_iter().map(|n| n.node_id).collect();
    assert!(online_ids.contains("aqvn-node-a1b2c3d4e5f60001"));
    assert!(online_ids.contains("aqvn-node-a1b2c3d4e5f60004"));
}

#[tokio::test]
async fn test_multi_host_pool_heartbeat_jitter() {
    // 1. Jitter interval calculation: base 25s, +/- 20% -> [20000ms, 30000ms]
    let base_interval_ms = 25_000;
    let jitter_ratio = 0.20;
    let config = HeartbeatJitterConfig::new(base_interval_ms, jitter_ratio);

    assert_eq!(config.min_interval_ms, 20_000);
    assert_eq!(config.max_interval_ms, 30_000);

    let mut generated_intervals = Vec::with_capacity(100);
    for _ in 0..100 {
        let interval = config.next_interval();
        let ms = interval.as_millis() as u64;
        assert!(
            (20_000..=30_000).contains(&ms),
            "Interval {ms}ms must lie within 20s - 30s jitter bounds"
        );
        generated_intervals.push(ms);
    }

    // Verify non-deterministic distribution across heartbeats
    let unique_values: HashSet<u64> = generated_intervals.into_iter().collect();
    assert!(
        unique_values.len() > 10,
        "Jitter must generate varied intervals to avoid cluster heartbeat storms, got {} unique",
        unique_values.len()
    );

    // 2. Helper function compute_heartbeat_jitter
    let dur = compute_heartbeat_jitter(10, 20);
    let ms = dur.as_millis() as u64;
    assert!((8_000..=12_000).contains(&ms));

    // 3. Ping-pong RTT tracking simulation
    let hub = MultiHostPoolHub::new("aqvn-hub");
    let (tx, mut rx) = mpsc::channel(10);
    let reg = NodeRegistration::new("aqvn-node-test-jitter", "Jitter Node", vec![]);
    hub.register_node(reg, tx).await.unwrap();

    let ping_ts = hub
        .send_heartbeat_ping("aqvn-node-test-jitter")
        .await
        .unwrap();
    let ping_msg = rx.recv().await.unwrap();
    assert_eq!(ping_msg.msg_type, POOL_MSG_HEARTBEAT_PING);

    // Simulate node response with 15ms latency
    tokio::time::sleep(Duration::from_millis(15)).await;
    let latency = hub
        .handle_heartbeat_pong("aqvn-node-test-jitter", ping_ts)
        .await
        .unwrap();
    assert!(latency >= 15, "Measured latency must be >= sleep time");

    let node = hub.get_node("aqvn-node-test-jitter").await.unwrap();
    assert_eq!(node.metrics.ping_latency_ms, latency);
    assert_eq!(node.metrics.heartbeat_count, 1);
}

#[tokio::test]
async fn test_multi_host_pool_targeted_task_routing() {
    let hub = MultiHostPoolHub::new("aqvn-hub-router");

    let (tx_a, mut rx_a) = mpsc::channel(16);
    let (tx_b, mut rx_b) = mpsc::channel(16);
    let (tx_c, mut rx_c) = mpsc::channel(16);

    let reg_a = NodeRegistration::new(
        "aqvn-node-worker-a",
        "Worker Alpha",
        vec!["files".to_string(), "code_execution".to_string()],
    );
    let reg_b = NodeRegistration::new(
        "aqvn-node-worker-b",
        "Worker Beta",
        vec!["browser".to_string(), "computer_use".to_string()],
    );
    let reg_c = NodeRegistration::new(
        "aqvn-node-worker-c",
        "Worker Gamma",
        vec!["code_execution".to_string(), "browser".to_string()],
    );

    hub.register_node(reg_a, tx_a).await.unwrap();
    hub.register_node(reg_b, tx_b).await.unwrap();
    hub.register_node(reg_c, tx_c).await.unwrap();

    // Set latencies: A = 10ms, B = 40ms, C = 25ms
    hub.record_heartbeat("aqvn-node-worker-a", 10, None)
        .await
        .unwrap();
    hub.record_heartbeat("aqvn-node-worker-b", 40, None)
        .await
        .unwrap();
    hub.record_heartbeat("aqvn-node-worker-c", 25, None)
        .await
        .unwrap();

    // 1. Direct Target Routing to Worker B
    let task_direct = TaskPayload::new(
        "task-direct-001",
        "browser_navigate",
        serde_json::json!({ "url": "https://example.com" }),
    );
    let dispatch_res = hub
        .dispatch_task(
            RouteTarget::Node("aqvn-node-worker-b".to_string()),
            task_direct,
        )
        .await
        .expect("Direct dispatch to Worker B must succeed");
    assert_eq!(dispatch_res.target_node_id, "aqvn-node-worker-b");

    let msg_b = rx_b.recv().await.expect("Worker B must receive the task");
    assert_eq!(msg_b.msg_type, POOL_MSG_TASK_DISPATCH);
    assert_eq!(msg_b.correlation_id, "task-direct-001");
    assert_eq!(msg_b.target_id.as_deref(), Some("aqvn-node-worker-b"));

    // 2. Capability-based Routing (scope: "files" -> only Worker A has it)
    let task_files = TaskPayload::new(
        "task-cap-files",
        "file_write",
        serde_json::json!({ "path": "/tmp/test.txt", "content": "hello" }),
    );
    let dispatch_cap = hub
        .dispatch_task(RouteTarget::Capability("files".to_string()), task_files)
        .await
        .expect("Capability routing must find Worker A");
    assert_eq!(dispatch_cap.target_node_id, "aqvn-node-worker-a");

    let msg_a = rx_a.recv().await.expect("Worker A must receive files task");
    assert_eq!(msg_a.correlation_id, "task-cap-files");

    // 3. Capability-based Routing with multiple candidates (scope: "code_execution" -> A: 10ms, C: 25ms)
    // Selects lowest latency among eligible: Worker A (10ms)
    let task_exec = TaskPayload::new(
        "task-cap-exec",
        "exec",
        serde_json::json!({ "command": "cargo build" }),
    );
    let dispatch_exec = hub
        .dispatch_task(
            RouteTarget::Capability("code_execution".to_string()),
            task_exec,
        )
        .await
        .unwrap();
    assert_eq!(dispatch_exec.target_node_id, "aqvn-node-worker-a");

    // 4. Lowest Latency Routing across all online nodes (A: 10ms)
    let task_low = TaskPayload::new("task-lowest-lat", "ping", serde_json::json!({}));
    let dispatch_low = hub
        .dispatch_task(RouteTarget::LowestLatency, task_low)
        .await
        .unwrap();
    assert_eq!(dispatch_low.target_node_id, "aqvn-node-worker-a");

    // 5. Broadcast Routing to all online nodes
    let task_bcast = TaskPayload::new("task-broadcast-sync", "sync_clock", serde_json::json!({}));
    let dispatch_bcast = hub
        .dispatch_task(RouteTarget::Broadcast, task_bcast)
        .await
        .unwrap();
    assert_eq!(dispatch_bcast.status, "broadcast_dispatched");

    // Ensure all 3 workers received broadcast
    let _ = rx_a.recv().await.unwrap();
    let _ = rx_b.recv().await.unwrap();
    let _ = rx_c.recv().await.unwrap();

    // 6. Routing to Offline Node fails with NodeOffline
    hub.update_node_status("aqvn-node-worker-c", NodeStatus::Offline)
        .await
        .unwrap();
    let task_offline = TaskPayload::new("task-off", "test", serde_json::json!({}));
    let err_off = hub
        .dispatch_task(
            RouteTarget::Node("aqvn-node-worker-c".to_string()),
            task_offline,
        )
        .await;
    assert!(matches!(err_off, Err(RoutingError::NodeOffline(_))));

    // 7. Routing to Non-existent Node fails with NodeNotFound
    let task_missing = TaskPayload::new("task-miss", "test", serde_json::json!({}));
    let err_miss = hub
        .dispatch_task(
            RouteTarget::Node("aqvn-node-phantom".to_string()),
            task_missing,
        )
        .await;
    assert!(matches!(err_miss, Err(RoutingError::NodeNotFound(_))));

    // 8. Routing with Non-existent Capability fails with NoEligibleNode
    let task_nocap = TaskPayload::new("task-nocap", "test", serde_json::json!({}));
    let err_nocap = hub
        .dispatch_task(
            RouteTarget::Capability("quantum_teleportation".to_string()),
            task_nocap,
        )
        .await;
    assert!(matches!(err_nocap, Err(RoutingError::NoEligibleNode(_))));
}

#[tokio::test]
async fn test_multi_host_pool_request_response_execution() {
    let hub = MultiHostPoolHub::new("aqvn-hub-exec");
    let (tx_node, mut rx_node) = mpsc::channel(16);

    let reg = NodeRegistration::new(
        "aqvn-node-runner",
        "Task Runner",
        vec!["code_execution".to_string()],
    );
    hub.register_node(reg, tx_node).await.unwrap();

    // Spawn simulated agent node worker
    let hub_clone = hub.clone();
    let worker_handle = tokio::spawn(async move {
        while let Some(msg) = rx_node.recv().await {
            if msg.msg_type == POOL_MSG_TASK_DISPATCH {
                let task: TaskPayload = serde_json::from_value(msg.payload).unwrap();
                let output = serde_json::json!({
                    "exit_code": 0,
                    "stdout": format!("output of {}", task.operation),
                });
                let result = TaskResult::ok(task.task_id, "aqvn-node-runner", output, 35);
                hub_clone.handle_task_result(result).await.unwrap();
            }
        }
    });

    let task = TaskPayload::new(
        "task-e2e-001",
        "run_diagnostics",
        serde_json::json!({ "check": "memory" }),
    );

    let result = hub
        .execute_task(
            RouteTarget::Node("aqvn-node-runner".to_string()),
            task,
            Duration::from_secs(5),
        )
        .await
        .expect("Task execution must succeed");

    assert!(result.success);
    assert_eq!(result.task_id, "task-e2e-001");
    assert_eq!(result.node_id, "aqvn-node-runner");
    assert_eq!(
        result.output.get("exit_code").and_then(|v| v.as_i64()),
        Some(0)
    );
    assert_eq!(result.execution_time_ms, 35);

    // Verify active task count decremented back to 0
    let node = hub.get_node("aqvn-node-runner").await.unwrap();
    assert_eq!(node.metrics.active_tasks, 0);

    worker_handle.abort();
}

#[tokio::test]
async fn test_multi_host_pool_websocket_framing() {
    // 1. Serialization to and from JSON
    let ping = PoolMessage::heartbeat_ping("hub-1", "node-1");
    let json_str = ping.to_json().unwrap();
    let parsed_msg = PoolMessage::from_json(&json_str).unwrap();
    assert_eq!(ping, parsed_msg);

    let pong = PoolMessage::heartbeat_pong("node-1", "hub-1", 1234567890);
    assert_eq!(pong.msg_type, POOL_MSG_HEARTBEAT_PONG);
    assert_eq!(pong.source_id, "node-1");
    assert_eq!(pong.target_id.as_deref(), Some("hub-1"));

    // 2. Socket.io format encoding & decoding
    let sio_event = ping.to_socketio_event(Some("/ws"));
    assert!(sio_event.starts_with("42/ws,[\"pool_heartbeat_ping\",{"));

    let decoded =
        PoolMessage::from_socketio_event(&sio_event).expect("Must decode socket.io event");
    assert_eq!(decoded.msg_type, POOL_MSG_HEARTBEAT_PING);
    assert_eq!(decoded.source_id, "hub-1");
    assert_eq!(decoded.target_id.as_deref(), Some("node-1"));
}

#[tokio::test]
async fn test_multi_host_pool_stale_node_pruning() {
    let hub = MultiHostPoolHub::new("aqvn-hub-prune");
    let (tx1, _rx1) = mpsc::channel(4);
    let (tx2, _rx2) = mpsc::channel(4);

    let reg1 = NodeRegistration::new("aqvn-node-active", "Active Node", vec![]);
    let reg2 = NodeRegistration::new("aqvn-node-stale", "Stale Node", vec![]);

    hub.register_node(reg1, tx1).await.unwrap();
    hub.register_node(reg2, tx2).await.unwrap();

    // Sleep 15ms
    tokio::time::sleep(Duration::from_millis(15)).await;

    // Refresh node1's heartbeat
    hub.record_heartbeat("aqvn-node-active", 10, None)
        .await
        .unwrap();

    // Prune nodes with timeout 10ms: node2 should be pruned to Offline
    let pruned = hub.prune_stale_nodes(Duration::from_millis(10)).await;
    assert_eq!(pruned, vec!["aqvn-node-stale".to_string()]);

    let node1 = hub.get_node("aqvn-node-active").await.unwrap();
    let node2 = hub.get_node("aqvn-node-stale").await.unwrap();

    assert_eq!(node1.status, NodeStatus::Online);
    assert_eq!(node2.status, NodeStatus::Offline);
    assert_eq!(hub.online_node_count().await, 1);
}

#[tokio::test]
async fn test_multi_host_pool_telemetry_benchmarks_val_v2_bench_002() {
    // 1. Empirical Jitter Distribution Benchmark (1,000 samples)
    let base_interval_ms = 10_000u64;
    let jitter_ratio = 0.20f64;
    let config = HeartbeatJitterConfig::new(base_interval_ms, jitter_ratio);

    assert_eq!(config.min_interval_ms, 8_000);
    assert_eq!(config.max_interval_ms, 12_000);

    let mut samples: Vec<f64> = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        let ms = config.next_interval().as_millis() as f64;
        assert!((8_000.0..=12_000.0).contains(&ms));
        samples.push(ms);
    }

    let mean: f64 = samples.iter().sum::<f64>() / (samples.len() as f64);
    let variance: f64 =
        samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (samples.len() as f64);
    let std_dev: f64 = variance.sqrt();

    // Verify mean is tightly centered around 10,000ms (+/- 250ms)
    assert!(
        (mean - 10_000.0).abs() < 250.0,
        "Mean jitter interval {mean} must be close to base 10000ms"
    );
    // Verify standard deviation indicates uniform non-deterministic dispersion
    // For uniform distribution on [-2000, 2000], theoretical stddev is 4000 / sqrt(12) ~ 1154.7ms
    assert!(
        std_dev > 900.0 && std_dev < 1400.0,
        "Standard deviation {std_dev}ms confirms healthy anti-storm entropy"
    );

    // 2. Ping/Pong RTT Tracking & Moving Average Benchmark
    let hub = MultiHostPoolHub::new("aqvn-bench-hub");
    let (tx1, mut rx1) = mpsc::channel(1500);
    let (tx2, mut rx2) = mpsc::channel(1500);

    let reg1 = NodeRegistration::new(
        "node-alpha",
        "Alpha Host",
        vec!["files".into(), "exec".into()],
    );
    let reg2 = NodeRegistration::new("node-beta", "Beta Host", vec!["browser".into()]);

    hub.register_node(reg1, tx1).await.unwrap();
    hub.register_node(reg2, tx2).await.unwrap();

    let simulated_latencies_alpha = [12, 14, 18, 11, 15, 20, 13, 16];
    for &lat in &simulated_latencies_alpha {
        hub.record_heartbeat("node-alpha", lat, None).await.unwrap();
    }

    let node_a = hub.get_node("node-alpha").await.unwrap();
    assert_eq!(node_a.metrics.heartbeat_count, 8);
    assert_eq!(node_a.metrics.min_latency_ms, 11);
    assert_eq!(node_a.metrics.max_latency_ms, 20);
    let expected_avg_a = (12 + 14 + 18 + 11 + 15 + 20 + 13 + 16) as f64 / 8.0;
    assert!((node_a.metrics.avg_latency_ms - expected_avg_a).abs() < 0.001);

    // 3. High-Throughput Targeted Task Routing Benchmark (1,000 dispatches)
    let start_bench = Instant::now();
    for i in 0..1_000 {
        let task = TaskPayload::new(
            format!("bench-task-{i}"),
            "eval_operation",
            serde_json::json!({ "iter": i }),
        );
        let target = match i % 4 {
            0 => RouteTarget::Node("node-alpha".to_string()),
            1 => RouteTarget::Capability("browser".to_string()),
            2 => RouteTarget::LowestLatency,
            _ => RouteTarget::RoundRobin,
        };
        let res = hub.dispatch_task(target, task).await.unwrap();
        assert!(!res.target_node_id.is_empty());
    }
    let elapsed = start_bench.elapsed();
    assert!(
        elapsed < Duration::from_secs(2),
        "1,000 task dispatches must complete under 2.0s, took {:?}",
        elapsed
    );

    // Verify messages delivered into receiver channels
    let mut received_1 = 0;
    while rx1.try_recv().is_ok() {
        received_1 += 1;
    }
    let mut received_2 = 0;
    while rx2.try_recv().is_ok() {
        received_2 += 1;
    }
    assert_eq!(
        received_1 + received_2,
        1_000,
        "All 1,000 dispatched tasks must reach nodes"
    );
}

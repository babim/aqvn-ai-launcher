//! Multi-Host Cluster Pool Coordination Protocol (`Multi-Host Pool`).
//!
//! Provides coordination, state management, heartbeat jitter, and targeted task
//! routing across multiple distributed agent nodes in an AQVN cluster:
//! - Multi-node connection coordination & status lifecycle tracking
//! - Node registration, metadata, and capability tracking (files, exec, browser, computer_use)
//! - Latency measurement & heartbeats with decorrelated / randomized jitter
//! - Targeted and capability-based task routing with feedback channels
//! - Centralized WebSocket message framing and routing for cluster operations

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, RwLock};

pub const POOL_MSG_REGISTER: &str = "pool_register";
pub const POOL_MSG_REGISTER_ACK: &str = "pool_register_ack";
pub const POOL_MSG_HEARTBEAT_PING: &str = "pool_heartbeat_ping";
pub const POOL_MSG_HEARTBEAT_PONG: &str = "pool_heartbeat_pong";
pub const POOL_MSG_TASK_DISPATCH: &str = "pool_task_dispatch";
pub const POOL_MSG_TASK_RESULT: &str = "pool_task_result";
pub const POOL_MSG_STATUS_UPDATE: &str = "pool_status_update";
pub const POOL_MSG_UNREGISTER: &str = "pool_unregister";

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PoolError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Node already registered: {0}")]
    NodeAlreadyExists(String),
    #[error("Node is offline: {0}")]
    NodeOffline(String),
    #[error("Node channel closed: {0}")]
    ChannelClosed(String),
    #[error("Task not found or already completed: {0}")]
    TaskNotFound(String),
    #[error("Invalid message format: {0}")]
    InvalidMessage(String),
    #[error("Operation timed out: {0}")]
    Timeout(String),
    #[error("Routing error: {0}")]
    Routing(#[from] RoutingError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RoutingError {
    #[error("Target node not found: {0}")]
    NodeNotFound(String),
    #[error("Target node offline: {0}")]
    NodeOffline(String),
    #[error("Target node busy: {0}")]
    NodeBusy(String),
    #[error("No eligible node with required capability: {0}")]
    NoEligibleNode(String),
    #[error("Message delivery failed to node {0}: {1}")]
    DeliveryFailed(String, String),
    #[error("Task dispatch timed out: {0}")]
    Timeout(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    #[default]
    Online,
    Busy,
    Degraded,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCapabilities {
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub cpu_count: usize,
    #[serde(default)]
    pub total_ram_mb: u64,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            scopes: Vec::new(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            cpu_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            total_ram_mb: 0,
        }
    }
}

impl NodeCapabilities {
    pub fn new(scopes: Vec<String>) -> Self {
        Self {
            scopes,
            ..Default::default()
        }
    }

    pub fn with_system_info(
        scopes: Vec<String>,
        os: String,
        arch: String,
        cpu_count: usize,
        total_ram_mb: u64,
    ) -> Self {
        Self {
            scopes,
            os,
            arch,
            cpu_count,
            total_ram_mb,
        }
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s.eq_ignore_ascii_case(scope))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub cpu_usage_percent: f64,
    pub available_ram_mb: u64,
    pub active_tasks: usize,
    pub ping_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub avg_latency_ms: f64,
    pub heartbeat_count: u64,
    pub last_heartbeat_at: u64,
}

impl Default for NodeMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            available_ram_mb: 0,
            active_tasks: 0,
            ping_latency_ms: 0,
            min_latency_ms: 0,
            max_latency_ms: 0,
            avg_latency_ms: 0.0,
            heartbeat_count: 0,
            last_heartbeat_at: now_unix_ms(),
        }
    }
}

impl NodeMetrics {
    pub fn record_latency(&mut self, latency_ms: u64) {
        self.ping_latency_ms = latency_ms;
        self.heartbeat_count = self.heartbeat_count.saturating_add(1);
        self.last_heartbeat_at = now_unix_ms();
        if self.heartbeat_count == 1 {
            self.min_latency_ms = latency_ms;
            self.max_latency_ms = latency_ms;
            self.avg_latency_ms = latency_ms as f64;
        } else {
            self.min_latency_ms = self.min_latency_ms.min(latency_ms);
            self.max_latency_ms = self.max_latency_ms.max(latency_ms);
            let prev_total = self.avg_latency_ms * ((self.heartbeat_count - 1) as f64);
            self.avg_latency_ms = (prev_total + latency_ms as f64) / (self.heartbeat_count as f64);
        }
    }

    pub fn update_resources(&mut self, cpu_percent: f64, available_ram: u64) {
        self.cpu_usage_percent = cpu_percent;
        self.available_ram_mb = available_ram;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub node_id: String,
    pub host_label: String,
    pub status: NodeStatus,
    pub capabilities: NodeCapabilities,
    pub metrics: NodeMetrics,
    pub registered_at: u64,
    #[serde(skip)]
    pub last_seen: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRegistration {
    pub node_id: String,
    pub host_label: String,
    pub capabilities: NodeCapabilities,
    #[serde(default)]
    pub initial_metrics: Option<NodeMetrics>,
}

impl NodeRegistration {
    pub fn new(
        node_id: impl Into<String>,
        host_label: impl Into<String>,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            host_label: host_label.into(),
            capabilities: NodeCapabilities::new(scopes),
            initial_metrics: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeartbeatJitterConfig {
    pub base_interval_ms: u64,
    pub jitter_ratio: f64,
    pub min_interval_ms: u64,
    pub max_interval_ms: u64,
}

impl Default for HeartbeatJitterConfig {
    fn default() -> Self {
        Self {
            base_interval_ms: 10_000,
            jitter_ratio: 0.20,
            min_interval_ms: 1_000,
            max_interval_ms: 60_000,
        }
    }
}

impl HeartbeatJitterConfig {
    pub fn new(base_interval_ms: u64, jitter_ratio: f64) -> Self {
        let min = (base_interval_ms as f64 * (1.0 - jitter_ratio)).max(500.0) as u64;
        let max = (base_interval_ms as f64 * (1.0 + jitter_ratio)).max(min as f64 + 1.0) as u64;
        Self {
            base_interval_ms,
            jitter_ratio,
            min_interval_ms: min,
            max_interval_ms: max,
        }
    }

    pub fn next_interval(&self) -> Duration {
        let mut rng = rand::thread_rng();
        let min = (self.base_interval_ms as f64 * (1.0 - self.jitter_ratio))
            .max(self.min_interval_ms as f64) as u64;
        let max = (self.base_interval_ms as f64 * (1.0 + self.jitter_ratio))
            .min(self.max_interval_ms as f64) as u64;
        let ms = if min < max {
            rng.gen_range(min..=max)
        } else {
            min
        };
        Duration::from_millis(ms)
    }
}

pub fn compute_heartbeat_jitter(base_interval_secs: u64, jitter_percent: u32) -> Duration {
    let base_ms = base_interval_secs.max(1) * 1000;
    let ratio = (jitter_percent as f64 / 100.0).clamp(0.0, 0.9);
    let cfg = HeartbeatJitterConfig::new(base_ms, ratio);
    cfg.next_interval()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteTarget {
    Node(String),
    Capability(String),
    LowestLatency,
    LeastLoaded,
    RoundRobin,
    Broadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskPayload {
    pub task_id: String,
    pub operation: String,
    pub params: serde_json::Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl TaskPayload {
    pub fn new(
        task_id: impl Into<String>,
        operation: impl Into<String>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            operation: operation.into(),
            params,
            timeout_ms: None,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskResult {
    pub task_id: String,
    pub node_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    #[serde(default)]
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl TaskResult {
    pub fn ok(
        task_id: impl Into<String>,
        node_id: impl Into<String>,
        output: serde_json::Value,
        exec_time: u64,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            node_id: node_id.into(),
            success: true,
            output,
            error: None,
            execution_time_ms: exec_time,
        }
    }

    pub fn err(
        task_id: impl Into<String>,
        node_id: impl Into<String>,
        error: impl Into<String>,
        exec_time: u64,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            node_id: node_id.into(),
            success: false,
            output: serde_json::Value::Null,
            error: Some(error.into()),
            execution_time_ms: exec_time,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskDispatchResult {
    pub task_id: String,
    pub target_node_id: String,
    pub status: String,
    pub dispatched_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PoolMessage {
    pub msg_type: String,
    pub source_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    pub correlation_id: String,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub payload: serde_json::Value,
}

impl PoolMessage {
    pub fn new(
        msg_type: impl Into<String>,
        source_id: impl Into<String>,
        target_id: Option<String>,
        correlation_id: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            msg_type: msg_type.into(),
            source_id: source_id.into(),
            target_id,
            correlation_id: correlation_id.into(),
            timestamp_ms: now_unix_ms(),
            payload,
        }
    }

    pub fn register(node_id: &str, host_label: &str, capabilities: NodeCapabilities) -> Self {
        let payload = serde_json::json!({
            "node_id": node_id,
            "host_label": host_label,
            "capabilities": capabilities,
        });
        Self::new(
            POOL_MSG_REGISTER,
            node_id,
            None,
            format!("reg-{}", now_unix_ms()),
            payload,
        )
    }

    pub fn register_ack(source_id: &str, target_id: &str, success: bool, message: &str) -> Self {
        let payload = serde_json::json!({
            "success": success,
            "message": message,
        });
        Self::new(
            POOL_MSG_REGISTER_ACK,
            source_id,
            Some(target_id.to_string()),
            format!("ack-{}", now_unix_ms()),
            payload,
        )
    }

    pub fn heartbeat_ping(source_id: &str, target_id: &str) -> Self {
        let now = now_unix_ms();
        let payload = serde_json::json!({
            "ping_timestamp_ms": now,
        });
        Self::new(
            POOL_MSG_HEARTBEAT_PING,
            source_id,
            Some(target_id.to_string()),
            format!("ping-{now}"),
            payload,
        )
    }

    pub fn heartbeat_pong(source_id: &str, target_id: &str, ping_timestamp_ms: u64) -> Self {
        let now = now_unix_ms();
        let payload = serde_json::json!({
            "ping_timestamp_ms": ping_timestamp_ms,
            "pong_timestamp_ms": now,
        });
        Self::new(
            POOL_MSG_HEARTBEAT_PONG,
            source_id,
            Some(target_id.to_string()),
            format!("pong-{now}"),
            payload,
        )
    }

    pub fn task_dispatch(source_id: &str, target_node_id: &str, task: TaskPayload) -> Self {
        let payload = serde_json::to_value(&task).unwrap_or(serde_json::Value::Null);
        Self::new(
            POOL_MSG_TASK_DISPATCH,
            source_id,
            Some(target_node_id.to_string()),
            task.task_id.clone(),
            payload,
        )
    }

    pub fn task_result(source_id: &str, target_id: &str, result: TaskResult) -> Self {
        let payload = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
        Self::new(
            POOL_MSG_TASK_RESULT,
            source_id,
            Some(target_id.to_string()),
            result.task_id.clone(),
            payload,
        )
    }

    pub fn status_update(source_id: &str, status: NodeStatus, metrics: NodeMetrics) -> Self {
        let payload = serde_json::json!({
            "status": status,
            "metrics": metrics,
        });
        Self::new(
            POOL_MSG_STATUS_UPDATE,
            source_id,
            None,
            format!("status-{}", now_unix_ms()),
            payload,
        )
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn to_socketio_event(&self, namespace: Option<&str>) -> String {
        let payload = self.to_json().unwrap_or_default();
        crate::protocol::format_socketio_event(namespace, &self.msg_type, &payload)
    }

    pub fn from_socketio_event(text: &str) -> Option<Self> {
        if let Some(packet) = crate::protocol::parse_socketio_packet(text) {
            if packet.packet_type == crate::protocol::SIO_PACKET_EVENT {
                if let Ok(serde_json::Value::Array(arr)) =
                    serde_json::from_str::<serde_json::Value>(packet.data)
                {
                    if let Some(payload_val) = arr.get(1) {
                        return serde_json::from_value::<Self>(payload_val.clone()).ok();
                    }
                }
            }
        }
        Self::from_json(text).ok()
    }
}

struct NodeEntry {
    node: AgentNode,
    tx: mpsc::Sender<PoolMessage>,
}

#[derive(Clone)]
pub struct MultiHostPoolHub {
    hub_id: String,
    nodes: Arc<RwLock<HashMap<String, NodeEntry>>>,
    round_robin_counter: Arc<AtomicUsize>,
    pending_tasks: Arc<RwLock<HashMap<String, oneshot::Sender<TaskResult>>>>,
    jitter_config: HeartbeatJitterConfig,
}

impl MultiHostPoolHub {
    pub fn new(hub_id: impl Into<String>) -> Self {
        Self::with_jitter_config(hub_id, HeartbeatJitterConfig::default())
    }

    pub fn with_jitter_config(
        hub_id: impl Into<String>,
        jitter_config: HeartbeatJitterConfig,
    ) -> Self {
        Self {
            hub_id: hub_id.into(),
            nodes: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counter: Arc::new(AtomicUsize::new(0)),
            pending_tasks: Arc::new(RwLock::new(HashMap::new())),
            jitter_config,
        }
    }

    pub fn hub_id(&self) -> &str {
        &self.hub_id
    }

    pub fn jitter_config(&self) -> &HeartbeatJitterConfig {
        &self.jitter_config
    }

    pub fn calculate_next_heartbeat_interval(&self) -> Duration {
        self.jitter_config.next_interval()
    }

    pub async fn register_node(
        &self,
        reg: NodeRegistration,
        tx: mpsc::Sender<PoolMessage>,
    ) -> Result<AgentNode, PoolError> {
        let node_id = reg.node_id.clone();
        let registered_at = now_unix_ms();
        let agent_node = AgentNode {
            node_id: node_id.clone(),
            host_label: reg.host_label,
            status: NodeStatus::Online,
            capabilities: reg.capabilities,
            metrics: reg.initial_metrics.unwrap_or_default(),
            registered_at,
            last_seen: Some(Instant::now()),
        };

        let mut nodes = self.nodes.write().await;
        nodes.insert(
            node_id.clone(),
            NodeEntry {
                node: agent_node.clone(),
                tx,
            },
        );

        Ok(agent_node)
    }

    pub async fn unregister_node(&self, node_id: &str) -> Option<AgentNode> {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id).map(|e| e.node)
    }

    pub async fn get_node(&self, node_id: &str) -> Option<AgentNode> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).map(|e| e.node.clone())
    }

    pub async fn list_nodes(&self) -> Vec<AgentNode> {
        let nodes = self.nodes.read().await;
        nodes.values().map(|e| e.node.clone()).collect()
    }

    pub async fn list_online_nodes(&self) -> Vec<AgentNode> {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|e| e.node.status == NodeStatus::Online)
            .map(|e| e.node.clone())
            .collect()
    }

    pub async fn node_count(&self) -> usize {
        let nodes = self.nodes.read().await;
        nodes.len()
    }

    pub async fn online_node_count(&self) -> usize {
        let nodes = self.nodes.read().await;
        nodes
            .values()
            .filter(|e| e.node.status == NodeStatus::Online)
            .count()
    }

    pub async fn update_node_status(
        &self,
        node_id: &str,
        status: NodeStatus,
    ) -> Result<(), PoolError> {
        let mut nodes = self.nodes.write().await;
        let entry = nodes
            .get_mut(node_id)
            .ok_or_else(|| PoolError::NodeNotFound(node_id.to_string()))?;
        entry.node.status = status;
        entry.node.last_seen = Some(Instant::now());
        Ok(())
    }

    pub async fn record_heartbeat(
        &self,
        node_id: &str,
        latency_ms: u64,
        metrics: Option<NodeMetrics>,
    ) -> Result<(), PoolError> {
        let mut nodes = self.nodes.write().await;
        let entry = nodes
            .get_mut(node_id)
            .ok_or_else(|| PoolError::NodeNotFound(node_id.to_string()))?;
        entry.node.metrics.record_latency(latency_ms);
        if let Some(m) = metrics {
            entry.node.metrics.cpu_usage_percent = m.cpu_usage_percent;
            entry.node.metrics.available_ram_mb = m.available_ram_mb;
            entry.node.metrics.active_tasks = m.active_tasks;
        }
        if entry.node.status == NodeStatus::Offline {
            entry.node.status = NodeStatus::Online;
        }
        entry.node.last_seen = Some(Instant::now());
        Ok(())
    }

    pub async fn send_heartbeat_ping(&self, node_id: &str) -> Result<u64, PoolError> {
        let (tx, ping_ts) = {
            let nodes = self.nodes.read().await;
            let entry = nodes
                .get(node_id)
                .ok_or_else(|| PoolError::NodeNotFound(node_id.to_string()))?;
            if entry.node.status == NodeStatus::Offline {
                return Err(PoolError::NodeOffline(node_id.to_string()));
            }
            let msg = PoolMessage::heartbeat_ping(&self.hub_id, node_id);
            (entry.tx.clone(), msg.timestamp_ms)
        };

        let msg = PoolMessage::heartbeat_ping(&self.hub_id, node_id);
        tx.send(msg)
            .await
            .map_err(|e| PoolError::ChannelClosed(e.to_string()))?;
        Ok(ping_ts)
    }

    pub async fn handle_heartbeat_pong(
        &self,
        node_id: &str,
        ping_timestamp_ms: u64,
    ) -> Result<u64, PoolError> {
        let now = now_unix_ms();
        let latency_ms = now.saturating_sub(ping_timestamp_ms);
        self.record_heartbeat(node_id, latency_ms, None).await?;
        Ok(latency_ms)
    }

    pub async fn select_target_node(&self, target: &RouteTarget) -> Result<String, RoutingError> {
        let nodes = self.nodes.read().await;
        match target {
            RouteTarget::Node(id) => {
                let entry = nodes
                    .get(id)
                    .ok_or_else(|| RoutingError::NodeNotFound(id.clone()))?;
                if entry.node.status == NodeStatus::Offline {
                    return Err(RoutingError::NodeOffline(id.clone()));
                }
                Ok(id.clone())
            }
            RouteTarget::Capability(scope) => {
                let eligible: Vec<&NodeEntry> = nodes
                    .values()
                    .filter(|e| {
                        e.node.status == NodeStatus::Online && e.node.capabilities.has_scope(scope)
                    })
                    .collect();
                if eligible.is_empty() {
                    return Err(RoutingError::NoEligibleNode(scope.clone()));
                }
                let best = eligible
                    .into_iter()
                    .min_by_key(|e| e.node.metrics.ping_latency_ms)
                    .unwrap();
                Ok(best.node.node_id.clone())
            }
            RouteTarget::LowestLatency => {
                let online: Vec<&NodeEntry> = nodes
                    .values()
                    .filter(|e| e.node.status == NodeStatus::Online)
                    .collect();
                if online.is_empty() {
                    return Err(RoutingError::NoEligibleNode("online".to_string()));
                }
                let best = online
                    .into_iter()
                    .min_by_key(|e| e.node.metrics.ping_latency_ms)
                    .unwrap();
                Ok(best.node.node_id.clone())
            }
            RouteTarget::LeastLoaded => {
                let online: Vec<&NodeEntry> = nodes
                    .values()
                    .filter(|e| e.node.status == NodeStatus::Online)
                    .collect();
                if online.is_empty() {
                    return Err(RoutingError::NoEligibleNode("online".to_string()));
                }
                let best = online
                    .into_iter()
                    .min_by(|a, b| {
                        a.node
                            .metrics
                            .active_tasks
                            .cmp(&b.node.metrics.active_tasks)
                            .then_with(|| {
                                a.node
                                    .metrics
                                    .cpu_usage_percent
                                    .partial_cmp(&b.node.metrics.cpu_usage_percent)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                    })
                    .unwrap();
                Ok(best.node.node_id.clone())
            }
            RouteTarget::RoundRobin => {
                let online: Vec<String> = nodes
                    .values()
                    .filter(|e| e.node.status == NodeStatus::Online)
                    .map(|e| e.node.node_id.clone())
                    .collect();
                if online.is_empty() {
                    return Err(RoutingError::NoEligibleNode("online".to_string()));
                }
                let idx = self.round_robin_counter.fetch_add(1, Ordering::SeqCst) % online.len();
                Ok(online[idx].clone())
            }
            RouteTarget::Broadcast => {
                let online: Vec<String> = nodes
                    .values()
                    .filter(|e| e.node.status == NodeStatus::Online)
                    .map(|e| e.node.node_id.clone())
                    .collect();
                if online.is_empty() {
                    return Err(RoutingError::NoEligibleNode("online".to_string()));
                }
                Ok(online[0].clone())
            }
        }
    }

    pub async fn dispatch_task(
        &self,
        target: RouteTarget,
        task: TaskPayload,
    ) -> Result<TaskDispatchResult, RoutingError> {
        if target == RouteTarget::Broadcast {
            let (online_nodes, node_ids): (Vec<(String, mpsc::Sender<PoolMessage>)>, Vec<String>) = {
                let nodes = self.nodes.read().await;
                let pairs: Vec<(String, mpsc::Sender<PoolMessage>)> = nodes
                    .values()
                    .filter(|e| e.node.status == NodeStatus::Online)
                    .map(|e| (e.node.node_id.clone(), e.tx.clone()))
                    .collect();
                let ids = pairs.iter().map(|p| p.0.clone()).collect();
                (pairs, ids)
            };

            if online_nodes.is_empty() {
                return Err(RoutingError::NoEligibleNode("online".to_string()));
            }

            for (node_id, tx) in online_nodes {
                let msg = PoolMessage::task_dispatch(&self.hub_id, &node_id, task.clone());
                let _ = tx.send(msg).await;
            }

            return Ok(TaskDispatchResult {
                task_id: task.task_id,
                target_node_id: format!("broadcast({})", node_ids.join(",")),
                status: "broadcast_dispatched".to_string(),
                dispatched_at: now_unix_ms(),
            });
        }

        let target_node_id = self.select_target_node(&target).await?;
        let tx = {
            let nodes = self.nodes.read().await;
            let entry = nodes
                .get(&target_node_id)
                .ok_or_else(|| RoutingError::NodeNotFound(target_node_id.clone()))?;
            entry.tx.clone()
        };

        let msg = PoolMessage::task_dispatch(&self.hub_id, &target_node_id, task.clone());
        tx.send(msg)
            .await
            .map_err(|e| RoutingError::DeliveryFailed(target_node_id.clone(), e.to_string()))?;

        {
            let mut nodes = self.nodes.write().await;
            if let Some(entry) = nodes.get_mut(&target_node_id) {
                entry.node.metrics.active_tasks = entry.node.metrics.active_tasks.saturating_add(1);
            }
        }

        Ok(TaskDispatchResult {
            task_id: task.task_id,
            target_node_id,
            status: "dispatched".to_string(),
            dispatched_at: now_unix_ms(),
        })
    }

    pub async fn execute_task(
        &self,
        target: RouteTarget,
        task: TaskPayload,
        timeout: Duration,
    ) -> Result<TaskResult, RoutingError> {
        let (tx_res, rx_res) = oneshot::channel();
        let task_id = task.task_id.clone();
        {
            let mut pending = self.pending_tasks.write().await;
            pending.insert(task_id.clone(), tx_res);
        }

        let dispatch_res = self.dispatch_task(target, task).await;
        if let Err(e) = dispatch_res {
            let mut pending = self.pending_tasks.write().await;
            pending.remove(&task_id);
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx_res).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => {
                let mut pending = self.pending_tasks.write().await;
                pending.remove(&task_id);
                Err(RoutingError::DeliveryFailed(
                    task_id,
                    "response channel dropped".to_string(),
                ))
            }
            Err(_) => {
                let mut pending = self.pending_tasks.write().await;
                pending.remove(&task_id);
                Err(RoutingError::Timeout(format!(
                    "Task {task_id} timed out after {:?}",
                    timeout
                )))
            }
        }
    }

    pub async fn handle_task_result(&self, result: TaskResult) -> Result<(), PoolError> {
        {
            let mut nodes = self.nodes.write().await;
            if let Some(entry) = nodes.get_mut(&result.node_id) {
                entry.node.metrics.active_tasks = entry.node.metrics.active_tasks.saturating_sub(1);
            }
        }

        let mut pending = self.pending_tasks.write().await;
        if let Some(tx) = pending.remove(&result.task_id) {
            let _ = tx.send(result);
        }
        Ok(())
    }

    pub async fn prune_stale_nodes(&self, timeout: Duration) -> Vec<String> {
        let mut offline_ids = Vec::new();
        let mut nodes = self.nodes.write().await;
        for (id, entry) in nodes.iter_mut() {
            if entry.node.status != NodeStatus::Offline
                && entry
                    .node
                    .last_seen
                    .is_some_and(|ls| ls.elapsed() > timeout)
            {
                entry.node.status = NodeStatus::Offline;
                offline_ids.push(id.clone());
            }
        }
        offline_ids
    }

    pub async fn broadcast(&self, msg: PoolMessage) -> usize {
        let nodes = self.nodes.read().await;
        let mut count = 0;
        for entry in nodes.values() {
            if entry.node.status == NodeStatus::Online && entry.tx.send(msg.clone()).await.is_ok() {
                count += 1;
            }
        }
        count
    }

    pub async fn handle_incoming_message(
        &self,
        msg: PoolMessage,
    ) -> Result<Option<PoolMessage>, PoolError> {
        match msg.msg_type.as_str() {
            POOL_MSG_REGISTER => {
                let node_id = msg.source_id.clone();
                let _host_label = msg
                    .payload
                    .get("host_label")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&node_id)
                    .to_string();
                let _capabilities = msg
                    .payload
                    .get("capabilities")
                    .and_then(|v| serde_json::from_value::<NodeCapabilities>(v.clone()).ok())
                    .unwrap_or_default();

                // Note: caller registers mpsc sender via register_node
                let ack = PoolMessage::register_ack(&self.hub_id, &node_id, true, "registered");
                Ok(Some(ack))
            }
            POOL_MSG_HEARTBEAT_PONG => {
                let ping_ts = msg
                    .payload
                    .get("ping_timestamp_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_else(now_unix_ms);
                self.handle_heartbeat_pong(&msg.source_id, ping_ts).await?;
                Ok(None)
            }
            POOL_MSG_HEARTBEAT_PING => {
                let ping_ts = msg
                    .payload
                    .get("ping_timestamp_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_else(now_unix_ms);
                let pong = PoolMessage::heartbeat_pong(&self.hub_id, &msg.source_id, ping_ts);
                Ok(Some(pong))
            }
            POOL_MSG_TASK_RESULT => {
                if let Ok(res) = serde_json::from_value::<TaskResult>(msg.payload) {
                    self.handle_task_result(res).await?;
                }
                Ok(None)
            }
            POOL_MSG_STATUS_UPDATE => {
                if let Some(status_val) = msg.payload.get("status") {
                    if let Ok(status) = serde_json::from_value::<NodeStatus>(status_val.clone()) {
                        let _ = self.update_node_status(&msg.source_id, status).await;
                    }
                }
                if let Some(metrics_val) = msg.payload.get("metrics") {
                    if let Ok(metrics) = serde_json::from_value::<NodeMetrics>(metrics_val.clone())
                    {
                        let mut nodes = self.nodes.write().await;
                        if let Some(entry) = nodes.get_mut(&msg.source_id) {
                            entry.node.metrics.update_resources(
                                metrics.cpu_usage_percent,
                                metrics.available_ram_mb,
                            );
                        }
                    }
                }
                Ok(None)
            }
            POOL_MSG_UNREGISTER => {
                self.unregister_node(&msg.source_id).await;
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_jitter_bounds_and_randomness() {
        let base_interval_ms = 10_000;
        let jitter_ratio = 0.20;
        let config = HeartbeatJitterConfig::new(base_interval_ms, jitter_ratio);

        let mut intervals = Vec::new();
        for _ in 0..100 {
            let dur = config.next_interval();
            let ms = dur.as_millis() as u64;
            assert!(
                (8000..=12000).contains(&ms),
                "Jitter interval {ms}ms must be within [8000, 12000]"
            );
            intervals.push(ms);
        }

        let first = intervals[0];
        let has_variation = intervals.iter().any(|&v| v != first);
        assert!(
            has_variation,
            "Heartbeat intervals must have random variation"
        );

        let helper_dur = compute_heartbeat_jitter(20, 25);
        let helper_ms = helper_dur.as_millis() as u64;
        assert!((15000..=25000).contains(&helper_ms));
    }

    #[tokio::test]
    async fn test_pool_node_registration_and_metrics() {
        let hub = MultiHostPoolHub::new("aqvn-hub-central");
        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        let reg1 = NodeRegistration::new(
            "aqvn-node-alpha",
            "VPS Alpha",
            vec!["files".to_string(), "code_execution".to_string()],
        );
        let reg2 = NodeRegistration::new(
            "aqvn-node-beta",
            "Baremetal Beta",
            vec!["browser".to_string(), "computer_use".to_string()],
        );

        hub.register_node(reg1, tx1).await.unwrap();
        hub.register_node(reg2, tx2).await.unwrap();

        assert_eq!(hub.node_count().await, 2);
        assert_eq!(hub.online_node_count().await, 2);

        hub.record_heartbeat("aqvn-node-alpha", 15, None)
            .await
            .unwrap();
        hub.record_heartbeat("aqvn-node-alpha", 25, None)
            .await
            .unwrap();
        hub.record_heartbeat("aqvn-node-beta", 60, None)
            .await
            .unwrap();

        let node1 = hub.get_node("aqvn-node-alpha").await.unwrap();
        let node2 = hub.get_node("aqvn-node-beta").await.unwrap();

        assert_eq!(node1.metrics.ping_latency_ms, 25);
        assert_eq!(node1.metrics.min_latency_ms, 15);
        assert_eq!(node1.metrics.max_latency_ms, 25);
        assert_eq!(node1.metrics.avg_latency_ms, 20.0);
        assert_eq!(node1.metrics.heartbeat_count, 2);

        assert_eq!(node2.metrics.ping_latency_ms, 60);
        assert_eq!(node2.metrics.heartbeat_count, 1);

        hub.update_node_status("aqvn-node-alpha", NodeStatus::Busy)
            .await
            .unwrap();
        let node1_busy = hub.get_node("aqvn-node-alpha").await.unwrap();
        assert_eq!(node1_busy.status, NodeStatus::Busy);
        assert_eq!(hub.online_node_count().await, 1);

        drop(_rx1);
        drop(_rx2);
    }

    #[tokio::test]
    async fn test_pool_targeted_task_routing() {
        let hub = MultiHostPoolHub::new("aqvn-hub-central");
        let (tx_alpha, mut rx_alpha) = mpsc::channel(10);
        let (tx_beta, mut rx_beta) = mpsc::channel(10);

        let reg_alpha = NodeRegistration::new(
            "aqvn-node-alpha",
            "Alpha Node",
            vec!["files".to_string(), "code_execution".to_string()],
        );
        let reg_beta = NodeRegistration::new(
            "aqvn-node-beta",
            "Beta Node",
            vec!["browser".to_string(), "computer_use".to_string()],
        );

        hub.register_node(reg_alpha, tx_alpha).await.unwrap();
        hub.register_node(reg_beta, tx_beta).await.unwrap();

        hub.record_heartbeat("aqvn-node-alpha", 10, None)
            .await
            .unwrap();
        hub.record_heartbeat("aqvn-node-beta", 50, None)
            .await
            .unwrap();

        let task1 = TaskPayload::new(
            "task-001",
            "exec",
            serde_json::json!({ "command": "cargo --version" }),
        );
        let res1 = hub
            .dispatch_task(RouteTarget::Node("aqvn-node-beta".to_string()), task1)
            .await
            .unwrap();
        assert_eq!(res1.target_node_id, "aqvn-node-beta");

        let received_beta = rx_beta.recv().await.unwrap();
        assert_eq!(received_beta.msg_type, POOL_MSG_TASK_DISPATCH);
        assert_eq!(received_beta.target_id.as_deref(), Some("aqvn-node-beta"));
        assert_eq!(received_beta.correlation_id, "task-001");

        let task2 = TaskPayload::new(
            "task-002",
            "file_read",
            serde_json::json!({ "path": "/etc/hosts" }),
        );
        let res2 = hub
            .dispatch_task(RouteTarget::Capability("files".to_string()), task2)
            .await
            .unwrap();
        assert_eq!(res2.target_node_id, "aqvn-node-alpha");

        let received_alpha = rx_alpha.recv().await.unwrap();
        assert_eq!(received_alpha.correlation_id, "task-002");

        let task3 = TaskPayload::new("task-003", "ping", serde_json::json!({}));
        let res3 = hub
            .dispatch_task(RouteTarget::LowestLatency, task3)
            .await
            .unwrap();
        assert_eq!(res3.target_node_id, "aqvn-node-alpha");

        let task4 = TaskPayload::new("task-004", "invalid", serde_json::json!({}));
        let err4 = hub
            .dispatch_task(
                RouteTarget::Capability("quantum_compute".to_string()),
                task4,
            )
            .await;
        assert!(matches!(err4, Err(RoutingError::NoEligibleNode(_))));
    }

    #[tokio::test]
    async fn test_pool_execute_task_request_response() {
        let hub = MultiHostPoolHub::new("aqvn-hub-central");
        let (tx, mut rx) = mpsc::channel(10);

        let reg = NodeRegistration::new(
            "aqvn-node-worker",
            "Worker 1",
            vec!["code_execution".to_string()],
        );
        hub.register_node(reg, tx).await.unwrap();

        let hub_clone = hub.clone();
        tokio::spawn(async move {
            if let Some(msg) = rx.recv().await {
                if msg.msg_type == POOL_MSG_TASK_DISPATCH {
                    let task: TaskPayload = serde_json::from_value(msg.payload).unwrap();
                    let result = TaskResult::ok(
                        task.task_id,
                        "aqvn-node-worker",
                        serde_json::json!({ "stdout": "cargo 1.97.1\n" }),
                        42,
                    );
                    let _ = hub_clone.handle_task_result(result).await;
                }
            }
        });

        let task = TaskPayload::new(
            "task-exec-1",
            "exec",
            serde_json::json!({ "command": "cargo --version" }),
        );
        let result = hub
            .execute_task(
                RouteTarget::Node("aqvn-node-worker".to_string()),
                task,
                Duration::from_secs(3),
            )
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.task_id, "task-exec-1");
        assert_eq!(result.node_id, "aqvn-node-worker");
        assert_eq!(
            result.output.get("stdout").and_then(|v| v.as_str()),
            Some("cargo 1.97.1\n")
        );
    }
}

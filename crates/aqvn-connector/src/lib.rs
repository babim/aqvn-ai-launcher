//! Outbound Host Gateway Hub (`_a0_connector`).
//!
//! Connects outbound via WebSocket to upstream `/ws` without requiring open
//! inbound ports. Performs the `connector_hello` handshake, maintains
//! heartbeat pings, auto-reconnects on disconnection, and multiplexes
//! control operations (files, exec, browser, computer use).

pub mod client;
pub mod handlers;
pub mod pool;
pub mod protocol;

pub use client::{GatewayClient, GatewayConfig, GatewayHandle, GatewayManager};
pub use pool::{
    compute_heartbeat_jitter, AgentNode, HeartbeatJitterConfig, MultiHostPoolHub, NodeCapabilities,
    NodeMetrics, NodeRegistration, NodeStatus, PoolError, PoolMessage, RouteTarget, RoutingError,
    TaskDispatchResult, TaskPayload, TaskResult, POOL_MSG_HEARTBEAT_PING, POOL_MSG_HEARTBEAT_PONG,
    POOL_MSG_REGISTER, POOL_MSG_REGISTER_ACK, POOL_MSG_STATUS_UPDATE, POOL_MSG_TASK_DISPATCH,
    POOL_MSG_TASK_RESULT, POOL_MSG_UNREGISTER,
};
pub use protocol::{
    AuthPayload, ConnectorBrowserOp, ConnectorBrowserOpResult, ConnectorGatewayControl,
    ConnectorGatewayControlResult, ConnectorHello, ConnectorHelloAck, ConnectorRemoteTreeUpdate,
    GatewayMetadata, GatewayScopes, ScopeCapability, ScopeConfig, MSG_TYPE_GATEWAY_CONTROL,
    MSG_TYPE_GATEWAY_CONTROL_RESULT, MSG_TYPE_REMOTE_TREE_UPDATE, SCOPE_FILE_WRITE,
};

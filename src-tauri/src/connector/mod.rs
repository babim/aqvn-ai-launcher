//! Outbound Host Gateway Hub (`_a0_connector`).
//!
//! Connects outbound via WebSocket to upstream `/ws` without requiring open
//! inbound ports. Performs the `connector_hello` handshake, maintains
//! heartbeat pings, auto-reconnects on disconnection, and multiplexes
//! control operations (files, exec, browser, computer use).

pub mod client;
pub mod handlers;
pub mod protocol;

pub use client::{GatewayClient, GatewayConfig, GatewayHandle};
pub use protocol::{
    AuthPayload, ConnectorBrowserOp, ConnectorBrowserOpResult, ConnectorGatewayControl,
    ConnectorGatewayControlResult, ConnectorHello, ConnectorHelloAck, ConnectorRemoteTreeUpdate,
    GatewayMetadata, GatewayScopes, ScopeCapability, ScopeConfig, MSG_TYPE_GATEWAY_CONTROL,
    MSG_TYPE_GATEWAY_CONTROL_RESULT, MSG_TYPE_REMOTE_TREE_UPDATE, SCOPE_FILE_WRITE,
};

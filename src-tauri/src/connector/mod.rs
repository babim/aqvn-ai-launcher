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
    AuthPayload, ConnectorBrowserOp, ConnectorBrowserOpResult, ConnectorHello, ConnectorHelloAck,
    GatewayMetadata, ScopeCapability, ScopeConfig,
};

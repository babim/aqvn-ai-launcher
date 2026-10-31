use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const MSG_TYPE_HELLO: &str = "connector_hello";
pub const MSG_TYPE_HELLO_ACK: &str = "connector_hello_ack";

pub const MSG_TYPE_FILE_OP: &str = "connector_file_op";
pub const MSG_TYPE_FILE_OP_RESULT: &str = "connector_file_op_result";

pub const MSG_TYPE_EXEC_OP: &str = "connector_exec_op";
pub const MSG_TYPE_EXEC_OP_RESULT: &str = "connector_exec_op_result";

pub const MSG_TYPE_BROWSER_OP: &str = "connector_browser_op";
pub const MSG_TYPE_BROWSER_OP_RESULT: &str = "connector_browser_op_result";

pub const MSG_TYPE_COMPUTER_USE_OP: &str = "connector_computer_use_op";
pub const MSG_TYPE_COMPUTER_USE_OP_RESULT: &str = "connector_computer_use_op_result";

pub const RUNTIME_TERMINAL: &str = "terminal";

pub const TRANSFER_PROTOCOL_V1: &str = "v1";

pub const SCOPE_FILES: &str = "files";
pub const SCOPE_EXEC: &str = "code_execution";
pub const SCOPE_BROWSER: &str = "browser";
pub const SCOPE_DESKTOP: &str = "computer_use";

// Engine.IO Packet Types (v4)
pub const EIO_PACKET_OPEN: char = '0';
pub const EIO_PACKET_CLOSE: char = '1';
pub const EIO_PACKET_PING: char = '2';
pub const EIO_PACKET_PONG: char = '3';
pub const EIO_PACKET_MESSAGE: char = '4';
pub const EIO_PACKET_UPGRADE: char = '5';
pub const EIO_PACKET_NOOP: char = '6';

// Socket.IO Packet Types (wrapped in Engine.IO Message packet '4')
pub const SIO_PACKET_CONNECT: char = '0';
pub const SIO_PACKET_DISCONNECT: char = '1';
pub const SIO_PACKET_EVENT: char = '2';
pub const SIO_PACKET_ACK: char = '3';
pub const SIO_PACKET_CONNECT_ERROR: char = '4';
pub const SIO_PACKET_BINARY_EVENT: char = '5';
pub const SIO_PACKET_BINARY_ACK: char = '6';

/// Represents a parsed Socket.IO packet over Engine.IO framing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketIoPacket<'a> {
    pub engine_type: char,
    pub packet_type: char,
    pub namespace: Option<&'a str>,
    pub ack_id: Option<u64>,
    pub data: &'a str,
}

/// Parses an inbound Socket.IO frame text (e.g. `42/ws,123["event", {...}]`).
pub fn parse_socketio_packet(text: &str) -> Option<SocketIoPacket<'_>> {
    let mut chars = text.chars();
    let engine_type = chars.next()?;
    if engine_type != EIO_PACKET_MESSAGE {
        return None;
    }
    let packet_type = chars.next()?;
    let mut rest = &text[2..];

    let namespace = if rest.starts_with('/') {
        if let Some(comma_idx) = rest.find(',') {
            let ns = &rest[..comma_idx];
            rest = &rest[comma_idx + 1..];
            Some(ns)
        } else {
            None
        }
    } else {
        None
    };

    let digits_len = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let ack_id = if digits_len > 0 {
        let id = rest[..digits_len].parse::<u64>().ok();
        rest = &rest[digits_len..];
        id
    } else {
        None
    };

    Some(SocketIoPacket {
        engine_type,
        packet_type,
        namespace,
        ack_id,
        data: rest,
    })
}

/// Formats a Socket.IO EVENT packet (`42[namespace,]["event", payload]`).
pub fn format_socketio_event(namespace: Option<&str>, event_name: &str, payload: &str) -> String {
    match namespace {
        Some(ns) if ns != "/" => format!("42{ns},[\"{event_name}\",{payload}]"),
        _ => format!("42[\"{event_name}\",{payload}]"),
    }
}

/// Formats a Socket.IO ACK packet (`43[namespace,]ack_id[payload]`).
pub fn format_socketio_ack(namespace: Option<&str>, ack_id: u64, payload: &str) -> String {
    match namespace {
        Some(ns) if ns != "/" => format!("43{ns},{ack_id}[{payload}]"),
        _ => format!("43{ack_id}[{payload}]"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SocketIoConnectAuth {
    pub handlers: Vec<String>,
    pub client_type: String,
}

impl Default for SocketIoConnectAuth {
    fn default() -> Self {
        Self {
            handlers: vec!["plugins/_a0_connector/ws_connector".to_string()],
            client_type: "launcher".to_string(),
        }
    }
}

/// Formats a Socket.IO CONNECT packet (`40[namespace,][payload]`).
pub fn format_socketio_connect<T: Serialize>(namespace: Option<&str>, auth: Option<&T>) -> String {
    let mut pkt = String::from("40");
    if let Some(ns) = namespace {
        if ns != "/" {
            pkt.push_str(ns);
        }
    }
    if let Some(auth_val) = auth {
        pkt.push(',');
        pkt.push_str(&serde_json::to_string(auth_val).unwrap_or_default());
    } else if let Some(ns) = namespace {
        if ns != "/" {
            pkt.push(',');
        }
    }
    pkt
}

fn default_true() -> bool {
    true
}

fn default_gateway_version() -> u32 {
    1
}

fn default_gateway_kind() -> String {
    "launcher".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeCapability {
    #[serde(default = "default_true")]
    pub supported: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GatewayMetadata {
    #[serde(default = "default_gateway_version")]
    pub version: u32,
    #[serde(default = "default_gateway_kind")]
    pub kind: String,
    #[serde(default = "default_true")]
    pub master_enabled: bool,
    #[serde(default)]
    pub scopes: HashMap<String, ScopeCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthPayload {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handlers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeConfig {
    #[serde(default = "default_true")]
    pub supported: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorHello {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub gateway_id: String,
    pub host_label: String,
    pub scopes: Vec<String>,
    pub capabilities: HashMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_files: Option<ScopeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_exec: Option<ScopeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_browser: Option<ScopeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computer_use: Option<ScopeConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<GatewayMetadata>,
}

impl ConnectorHello {
    pub fn new(
        gateway_id: impl Into<String>,
        host_label: impl Into<String>,
        scopes: Vec<String>,
    ) -> Self {
        let mut capabilities = HashMap::new();
        for scope in &scopes {
            capabilities.insert(scope.clone(), true);
        }

        let has_files = scopes.iter().any(|s| s == SCOPE_FILES || s == "files");
        let has_exec = scopes.iter().any(|s| s == SCOPE_EXEC || s == "exec");
        let has_browser = scopes.iter().any(|s| s == SCOPE_BROWSER || s == "browser");
        let has_desktop = scopes.iter().any(|s| s == SCOPE_DESKTOP || s == "desktop");

        let mut gateway_scopes = HashMap::new();
        gateway_scopes.insert(
            "files".to_string(),
            ScopeCapability {
                supported: true,
                enabled: has_files,
            },
        );
        gateway_scopes.insert(
            "exec".to_string(),
            ScopeCapability {
                supported: true,
                enabled: has_exec,
            },
        );
        gateway_scopes.insert(
            "browser".to_string(),
            ScopeCapability {
                supported: true,
                enabled: has_browser,
            },
        );
        gateway_scopes.insert(
            "desktop".to_string(),
            ScopeCapability {
                supported: true,
                enabled: has_desktop,
            },
        );

        let gateway = GatewayMetadata {
            version: 1,
            kind: "launcher".to_string(),
            master_enabled: true,
            scopes: gateway_scopes,
        };

        let mut auth_handlers = Vec::new();
        if has_files {
            auth_handlers.push("files".to_string());
        }
        if has_exec {
            auth_handlers.push("exec".to_string());
        }
        if has_browser {
            auth_handlers.push("browser".to_string());
        }
        if has_desktop {
            auth_handlers.push("desktop".to_string());
        }

        let auth = AuthPayload {
            handlers: auth_handlers,
            token: None,
        };

        Self {
            msg_type: MSG_TYPE_HELLO.to_string(),
            op: MSG_TYPE_HELLO.to_string(),
            gateway_id: gateway_id.into(),
            host_label: host_label.into(),
            capabilities,
            remote_files: Some(ScopeConfig {
                supported: true,
                enabled: has_files,
            }),
            remote_exec: Some(ScopeConfig {
                supported: true,
                enabled: has_exec,
            }),
            host_browser: Some(ScopeConfig {
                supported: true,
                enabled: has_browser,
            }),
            computer_use: Some(ScopeConfig {
                supported: true,
                enabled: has_desktop,
            }),
            auth: Some(auth),
            gateway: Some(gateway),
            scopes,
        }
    }

    pub fn new_legacy(
        gateway_id: impl Into<String>,
        host_label: impl Into<String>,
        scopes: Vec<String>,
    ) -> Self {
        let mut hello = Self::new(gateway_id, host_label, scopes);
        hello.auth = None;
        hello.gateway = None;
        hello
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorHelloAck {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorFileOp {
    #[serde(rename = "type", default)]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub old_text: Option<String>,
    #[serde(default)]
    pub new_text: Option<String>,
    #[serde(default)]
    pub patch_text: Option<String>,
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub length: Option<u64>,
    #[serde(default)]
    pub include_hidden: Option<bool>,
    #[serde(default)]
    pub recursive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorFileOpResult {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub id: String,
    pub success: bool,
    pub status: String,
    pub action: String,
    pub transfer_protocol: String,
    pub protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_written: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_lines: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hunk_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorExecOp {
    #[serde(rename = "type", default)]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub reset: Option<bool>,
    #[serde(default)]
    pub cols: Option<u16>,
    #[serde(default)]
    pub rows: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorExecOpResult {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub id: String,
    pub session_id: String,
    pub success: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorBrowserOp {
    #[serde(rename = "type", default)]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub selector: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub full_page: Option<bool>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub cdp_url: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorBrowserOpResult {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub id: String,
    pub success: bool,
    pub status: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorComputerUseOp {
    #[serde(rename = "type", default)]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub button: Option<String>,
    #[serde(default)]
    pub clicks: Option<u32>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub keys: Option<Vec<String>>,
    #[serde(default)]
    pub modifiers: Option<Vec<String>>,
    #[serde(default)]
    pub delta_x: Option<f64>,
    #[serde(default)]
    pub delta_y: Option<f64>,
    #[serde(default)]
    pub start_x: Option<f64>,
    #[serde(default)]
    pub start_y: Option<f64>,
    #[serde(default)]
    pub end_x: Option<f64>,
    #[serde(default)]
    pub end_y: Option<f64>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub quality: Option<u8>,
    #[serde(default)]
    pub display_id: Option<u32>,
    #[serde(default)]
    pub active_window: Option<bool>,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorComputerUseOpResult {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub id: String,
    pub success: bool,
    pub status: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

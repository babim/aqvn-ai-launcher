//! Protocol definitions for outbound gateway connection.

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeConfig {
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

        let has_files = scopes.iter().any(|s| s == SCOPE_FILES);
        let has_exec = scopes.iter().any(|s| s == SCOPE_EXEC);
        let has_browser = scopes.iter().any(|s| s == SCOPE_BROWSER);
        let has_desktop = scopes.iter().any(|s| s == SCOPE_DESKTOP);

        Self {
            msg_type: MSG_TYPE_HELLO.to_string(),
            op: MSG_TYPE_HELLO.to_string(),
            gateway_id: gateway_id.into(),
            host_label: host_label.into(),
            capabilities,
            remote_files: Some(ScopeConfig { enabled: has_files }),
            remote_exec: Some(ScopeConfig { enabled: has_exec }),
            host_browser: Some(ScopeConfig {
                enabled: has_browser,
            }),
            computer_use: Some(ScopeConfig {
                enabled: has_desktop,
            }),
            scopes,
        }
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

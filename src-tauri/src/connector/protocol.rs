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

pub const MSG_TYPE_GATEWAY_CONTROL: &str = "connector_gateway_control";
pub const MSG_TYPE_GATEWAY_CONTROL_RESULT: &str = "connector_gateway_control_result";

pub const RUNTIME_TERMINAL: &str = "terminal";

pub const TRANSFER_PROTOCOL_V1: &str = "v1";

pub const SCOPE_FILES: &str = "files";
pub const SCOPE_FILE_WRITE: &str = "file_write";
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

#[derive(Debug, Clone, Eq)]
pub struct GatewayScopes {
    pub files: bool,
    pub file_write: bool,
    pub code_execution: bool,
    pub browser: bool,
    pub computer_use: bool,
    pub active_list: Vec<String>,
}

impl GatewayScopes {
    pub fn from_vec(scopes: &[String]) -> Self {
        let has_files = scopes.iter().any(|s| s == SCOPE_FILES || s == "files");
        let has_exec = scopes.iter().any(|s| s == SCOPE_EXEC || s == "exec");
        let has_browser = scopes.iter().any(|s| s == SCOPE_BROWSER || s == "browser");
        let has_desktop = scopes.iter().any(|s| s == SCOPE_DESKTOP || s == "desktop");

        let active_list = scopes.to_vec();

        Self {
            files: has_files,
            file_write: has_files,
            code_execution: has_exec,
            browser: has_browser,
            computer_use: has_desktop,
            active_list,
        }
    }

    pub fn from_bools(
        files: bool,
        file_write: bool,
        code_execution: bool,
        browser: bool,
        computer_use: bool,
    ) -> Self {
        let mut active_list = Vec::new();
        if files {
            active_list.push(SCOPE_FILES.to_string());
        }
        if code_execution {
            active_list.push(SCOPE_EXEC.to_string());
        }
        if browser {
            active_list.push(SCOPE_BROWSER.to_string());
        }
        if computer_use {
            active_list.push(SCOPE_DESKTOP.to_string());
        }

        Self {
            files,
            file_write,
            code_execution,
            browser,
            computer_use,
            active_list,
        }
    }
}

impl Default for GatewayScopes {
    fn default() -> Self {
        Self::from_vec(&[
            SCOPE_FILES.to_string(),
            SCOPE_EXEC.to_string(),
            SCOPE_BROWSER.to_string(),
            SCOPE_DESKTOP.to_string(),
        ])
    }
}

impl From<Vec<String>> for GatewayScopes {
    fn from(scopes: Vec<String>) -> Self {
        Self::from_vec(&scopes)
    }
}

impl Serialize for GatewayScopes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("files", &self.files)?;
        map.serialize_entry("file_write", &self.file_write)?;
        map.serialize_entry("code_execution", &self.code_execution)?;
        map.serialize_entry("browser", &self.browser)?;
        map.serialize_entry("computer_use", &self.computer_use)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for GatewayScopes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct GatewayScopesVisitor;

        impl<'de> serde::de::Visitor<'de> for GatewayScopesVisitor {
            type Value = GatewayScopes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean map of scopes or a list of scope strings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut list = Vec::new();
                while let Some(item) = seq.next_element::<String>()? {
                    list.push(item);
                }
                Ok(GatewayScopes::from_vec(&list))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                let mut files = false;
                let mut file_write = false;
                let mut code_execution = false;
                let mut browser = false;
                let mut computer_use = false;

                while let Some((key, val)) = map.next_entry::<String, Value>()? {
                    let b = val.as_bool().unwrap_or(false);
                    match key.as_str() {
                        "files" => files = b,
                        "file_write" => file_write = b,
                        "code_execution" | "exec" => code_execution = b,
                        "browser" => browser = b,
                        "computer_use" | "desktop" => computer_use = b,
                        _ => {}
                    }
                }

                if files && !file_write {
                    file_write = true;
                }

                Ok(GatewayScopes::from_bools(
                    files,
                    file_write,
                    code_execution,
                    browser,
                    computer_use,
                ))
            }
        }

        deserializer.deserialize_any(GatewayScopesVisitor)
    }
}

impl PartialEq for GatewayScopes {
    fn eq(&self, other: &Self) -> bool {
        self.files == other.files
            && self.file_write == other.file_write
            && self.code_execution == other.code_execution
            && self.browser == other.browser
            && self.computer_use == other.computer_use
            && self.active_list == other.active_list
    }
}

impl<T> PartialEq<Vec<T>> for GatewayScopes
where
    String: PartialEq<T>,
{
    fn eq(&self, other: &Vec<T>) -> bool {
        self.active_list == *other
    }
}

impl<T> PartialEq<GatewayScopes> for Vec<T>
where
    T: PartialEq<String>,
{
    fn eq(&self, other: &GatewayScopes) -> bool {
        *self == other.active_list
    }
}

impl<T> PartialEq<[T]> for GatewayScopes
where
    String: PartialEq<T>,
{
    fn eq(&self, other: &[T]) -> bool {
        self.active_list.as_slice() == other
    }
}

impl<'a, T> PartialEq<&'a [T]> for GatewayScopes
where
    String: PartialEq<T>,
{
    fn eq(&self, other: &&'a [T]) -> bool {
        self.active_list.as_slice() == *other
    }
}

impl std::ops::Deref for GatewayScopes {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.active_list
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScopeConfig {
    #[serde(default = "default_true")]
    pub supported: bool,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectorHello {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub gateway_id: String,
    pub host_label: String,
    pub scopes: GatewayScopes,
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
        let gateway_scopes_obj = GatewayScopes::from_vec(&scopes);
        let mut capabilities = HashMap::new();
        for scope in &scopes {
            capabilities.insert(scope.clone(), true);
        }

        let has_files = scopes.iter().any(|s| s == SCOPE_FILES || s == "files");
        let has_exec = scopes.iter().any(|s| s == SCOPE_EXEC || s == "exec");
        let has_browser = scopes.iter().any(|s| s == SCOPE_BROWSER || s == "browser");
        let has_desktop = scopes.iter().any(|s| s == SCOPE_DESKTOP || s == "desktop");

        if has_files {
            capabilities.insert(SCOPE_FILE_WRITE.to_string(), true);
        }

        let mut gateway_scopes = HashMap::new();
        gateway_scopes.insert(
            "files".to_string(),
            ScopeCapability {
                supported: true,
                enabled: has_files,
            },
        );
        gateway_scopes.insert(
            "file_write".to_string(),
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
                write_enabled: Some(has_files),
                mode: Some(if has_files {
                    "read_write".to_string()
                } else {
                    "none".to_string()
                }),
                status: None,
            }),
            remote_exec: Some(ScopeConfig {
                supported: true,
                enabled: has_exec,
                write_enabled: None,
                mode: None,
                status: None,
            }),
            host_browser: Some(ScopeConfig {
                supported: true,
                enabled: has_browser,
                write_enabled: None,
                mode: None,
                status: None,
            }),
            computer_use: Some(ScopeConfig {
                supported: true,
                enabled: has_desktop,
                write_enabled: None,
                mode: None,
                status: if has_desktop {
                    Some("ready".to_string())
                } else {
                    None
                },
            }),
            auth: Some(auth),
            gateway: Some(gateway),
            scopes: gateway_scopes_obj,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorGatewayControl {
    #[serde(rename = "type", default)]
    pub msg_type: Option<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub op_id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorGatewayControlResult {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub op: String,
    pub id: String,
    pub op_id: String,
    pub ok: bool,
    pub success: bool,
    pub status: String,
    pub state: Value,
}

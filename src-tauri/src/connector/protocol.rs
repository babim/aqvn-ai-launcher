//! Protocol definitions for outbound gateway connection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MSG_TYPE_HELLO: &str = "connector_hello";
pub const MSG_TYPE_HELLO_ACK: &str = "connector_hello_ack";

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

//! Core configuration schemas and watcher for AQVN Workspace.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::SystemTime;
use tokio::sync::watch;

pub const DEFAULT_SERVER_URL: &str = "https://a0.matmagoc.com";
pub const DEFAULT_SERVER_PROFILES: &[&str] = &[DEFAULT_SERVER_URL];
pub const DEFAULT_RFC_ENABLED: bool = false;
pub const DEFAULT_RFC_PORT: u16 = 55080;
pub const DEFAULT_SSH_PORT: u16 = 55022;
pub const DEFAULT_ALLOWED_PATHS: &[&str] = &["~/Workspace"];
pub const DEFAULT_WORKSPACE_PATH: &str = "~/Workspace";
pub const DEFAULT_CONNECTOR_ENABLED: bool = true;
pub const DEFAULT_SCOPE_FILES: bool = true;
pub const DEFAULT_SCOPE_EXEC: bool = true;
pub const DEFAULT_SCOPE_BROWSER: bool = true;
pub const DEFAULT_SCOPE_DESKTOP: bool = true;
pub const DEFAULT_GLOBAL_HOTKEY: &str = "Alt+Space";
pub const DEFAULT_CLOSE_TO_TRAY: bool = true;
pub const DEFAULT_APP_LANGUAGE: &str = "vi";
pub const DEFAULT_SHOW_MENU_BAR: bool = false;
pub const DEFAULT_TASK_CHIME_ENABLED: bool = true;
pub const DEFAULT_TASK_CHIME_VOLUME: f32 = 0.5;
pub const DEFAULT_TASK_CHIME_TONE: &str = "classic";
pub const STORE_FILENAME: &str = "settings.json";

/// Returns a stable, deterministic gateway identifier for this device/installation.
pub fn default_gateway_id() -> String {
    deterministic_gateway_id()
}

pub fn deterministic_gateway_id() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aqvn-workspace-gateway-seed:");

    let mut found_seed = false;
    for path in &["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(id) = std::fs::read_to_string(path) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                hasher.update(trimmed.as_bytes());
                found_seed = true;
                break;
            }
        }
    }

    if let Ok(hostname) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
        hasher.update(hostname.as_bytes());
        found_seed = true;
    }

    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        hasher.update(user.as_bytes());
    }

    if !found_seed {
        hasher.update(b"aqvn-default-deterministic-seed");
    }

    let hash = hasher.finalize();
    let hex_part = format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5]
    );
    format!("aqvn-gw-{hex_part}")
}

pub fn default_app_language() -> String {
    DEFAULT_APP_LANGUAGE.to_string()
}

pub fn default_global_hotkey() -> String {
    DEFAULT_GLOBAL_HOTKEY.to_string()
}

pub fn default_workspace_path() -> String {
    DEFAULT_WORKSPACE_PATH.to_string()
}

pub const fn default_true() -> bool {
    true
}

pub const fn default_task_chime_volume() -> f32 {
    DEFAULT_TASK_CHIME_VOLUME
}

pub fn default_task_chime_tone() -> String {
    DEFAULT_TASK_CHIME_TONE.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f64,
    pub height: f64,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub is_maximized: bool,
}

impl WindowState {
    pub fn new(width: f64, height: f64, x: f64, y: f64, is_maximized: bool) -> Self {
        Self {
            width,
            height,
            x,
            y,
            is_maximized,
        }
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1100.0,
            height: 720.0,
            x: 0.0,
            y: 0.0,
            is_maximized: false,
        }
    }
}

pub fn default_security_policy() -> String {
    "supervised".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub server_url: String,
    pub server_profiles: Vec<String>,
    #[serde(default = "default_gateway_id")]
    pub gateway_id: String,
    #[serde(default = "default_workspace_path")]
    pub workspace_path: String,
    pub rfc_enabled: bool,
    pub rfc_password: String,
    pub ssh_password: String,
    pub allowed_paths: Vec<String>,
    pub rfc_port: u16,
    pub ssh_port: u16,
    #[serde(default = "default_true")]
    pub connector_enabled: bool,
    #[serde(default = "default_true")]
    pub scope_files: bool,
    #[serde(default = "default_true", alias = "scope_code_execution")]
    pub scope_exec: bool,
    #[serde(default = "default_true")]
    pub scope_browser: bool,
    #[serde(default = "default_true", alias = "scope_computer_use")]
    pub scope_desktop: bool,
    #[serde(default = "default_global_hotkey")]
    pub global_hotkey: String,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default = "default_app_language")]
    pub app_language: String,
    #[serde(default)]
    pub show_menu_bar: bool,
    #[serde(default = "default_true")]
    pub task_chime_enabled: bool,
    #[serde(default = "default_true")]
    pub task_notification_enabled: bool,
    #[serde(default = "default_task_chime_volume")]
    pub task_chime_volume: f32,
    #[serde(default = "default_task_chime_tone")]
    pub task_chime_tone: String,
    #[serde(default = "default_security_policy")]
    pub security_policy: String,
    #[serde(default)]
    pub window_state: Option<WindowState>,
    #[serde(default)]
    pub tls_ca_cert_path: Option<String>,
    #[serde(default)]
    pub tls_skip_verify: bool,
    #[serde(default)]
    pub mtls_enabled: bool,
    #[serde(default = "default_mtls_cert_ttl_hours")]
    pub mtls_cert_ttl_hours: u64,
    #[serde(default)]
    pub mtls_cert_path: Option<String>,
    #[serde(default)]
    pub mtls_key_path: Option<String>,
}

pub fn default_mtls_cert_ttl_hours() -> u64 {
    24
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            server_profiles: DEFAULT_SERVER_PROFILES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            gateway_id: default_gateway_id(),
            workspace_path: DEFAULT_WORKSPACE_PATH.to_string(),
            rfc_enabled: DEFAULT_RFC_ENABLED,
            rfc_password: String::new(),
            ssh_password: String::new(),
            allowed_paths: DEFAULT_ALLOWED_PATHS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            rfc_port: DEFAULT_RFC_PORT,
            ssh_port: DEFAULT_SSH_PORT,
            connector_enabled: DEFAULT_CONNECTOR_ENABLED,
            scope_files: DEFAULT_SCOPE_FILES,
            scope_exec: DEFAULT_SCOPE_EXEC,
            scope_browser: DEFAULT_SCOPE_BROWSER,
            scope_desktop: DEFAULT_SCOPE_DESKTOP,
            global_hotkey: DEFAULT_GLOBAL_HOTKEY.to_string(),
            close_to_tray: DEFAULT_CLOSE_TO_TRAY,
            app_language: DEFAULT_APP_LANGUAGE.to_string(),
            show_menu_bar: DEFAULT_SHOW_MENU_BAR,
            task_chime_enabled: DEFAULT_TASK_CHIME_ENABLED,
            task_notification_enabled: true,
            task_chime_volume: DEFAULT_TASK_CHIME_VOLUME,
            task_chime_tone: DEFAULT_TASK_CHIME_TONE.to_string(),
            security_policy: default_security_policy(),
            window_state: None,
            tls_ca_cert_path: None,
            tls_skip_verify: false,
            mtls_enabled: false,
            mtls_cert_ttl_hours: default_mtls_cert_ttl_hours(),
            mtls_cert_path: None,
            mtls_key_path: None,
        }
    }
}

impl AppConfig {
    /// Returns unified allowed paths merging workspace_path with allowed_paths, removing duplicates.
    pub fn unified_allowed_paths(&self) -> Vec<String> {
        let mut unified = Vec::new();
        let ws = self.workspace_path.trim();
        let include_ws = !ws.is_empty()
            && (ws != DEFAULT_WORKSPACE_PATH
                || self.allowed_paths.is_empty()
                || self.allowed_paths.iter().any(|p| p.trim() == ws));

        if include_ws {
            unified.push(ws.to_string());
        }

        for p in &self.allowed_paths {
            let trimmed = p.trim();
            if !trimmed.is_empty() && !unified.iter().any(|u| u == trimmed) {
                unified.push(trimmed.to_string());
            }
        }

        if unified.is_empty() {
            unified.push(DEFAULT_WORKSPACE_PATH.to_string());
        }
        unified
    }
}

/// Channel for watching configuration changes and notifying listeners.
#[derive(Clone)]
pub struct ConfigWatcher {
    sender: watch::Sender<AppConfig>,
    receiver: watch::Receiver<AppConfig>,
}

impl ConfigWatcher {
    pub fn new(initial: AppConfig) -> Self {
        let (sender, receiver) = watch::channel(initial);
        Self { sender, receiver }
    }

    pub fn current(&self) -> AppConfig {
        self.receiver.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<AppConfig> {
        self.receiver.clone()
    }

    pub fn update(&self, new_config: AppConfig) {
        let _ = self.sender.send(new_config);
    }
}

/// Loads configuration straight from the on-disk store file.
pub fn load_from_file(path: &Path) -> Option<AppConfig> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    serde_json::from_value(value).ok()
}

/// Loads configuration from file only if file modification timestamp (mtime) has changed.
pub fn load_from_file_if_modified(
    path: &Path,
    last_mtime: &mut Option<SystemTime>,
) -> Option<AppConfig> {
    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())?;
    if let Some(prev) = *last_mtime {
        if prev == mtime {
            return None;
        }
    }
    let config = load_from_file(path)?;
    *last_mtime = Some(mtime);
    Some(config)
}

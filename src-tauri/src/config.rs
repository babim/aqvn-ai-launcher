//!
//! Exposes typed settings over tauri-plugin-store `settings.json`.
//! Default values required by validation contract VAL-SCAF-010:
//!   server_url = "https://a0.matmagoc.com"
//!   rfc_enabled = false
//!   rfc_port = 55080
//!   ssh_port = 55022

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
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

fn default_app_language() -> String {
    DEFAULT_APP_LANGUAGE.to_string()
}

fn default_global_hotkey() -> String {
    DEFAULT_GLOBAL_HOTKEY.to_string()
}

pub fn default_workspace_path() -> String {
    DEFAULT_WORKSPACE_PATH.to_string()
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        }
    }
}

impl AppConfig {
    /// Loads configuration from tauri-plugin-store settings.json.
    /// Missing fields fall back to default values.
    pub fn load_from_store<R: Runtime>(app: &AppHandle<R>) -> Result<Self, String> {
        let store = app
            .store(STORE_FILENAME)
            .map_err(|e| format!("Failed to open store {STORE_FILENAME}: {e}"))?;

        let mut config = Self::default();

        if let Some(val) = store.get("server_url") {
            if let Some(s) = val.as_str() {
                config.server_url = s.to_string();
            }
        }
        if let Some(val) = store.get("server_profiles") {
            if let Some(arr) = val.as_array() {
                config.server_profiles = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
        if config.server_profiles.is_empty() {
            config.server_profiles = vec![config.server_url.clone()];
        }
        if let Some(val) = store.get("gateway_id") {
            if let Some(s) = val.as_str() {
                if !s.trim().is_empty() {
                    config.gateway_id = s.to_string();
                }
            }
        }
        if let Some(val) = store.get("workspace_path") {
            if let Some(s) = val.as_str() {
                if !s.trim().is_empty() {
                    config.workspace_path = s.to_string();
                }
            }
        }
        if let Some(val) = store.get("rfc_enabled") {
            if let Some(b) = val.as_bool() {
                config.rfc_enabled = b;
            }
        }
        if let Some(val) = store.get("rfc_password") {
            if let Some(s) = val.as_str() {
                config.rfc_password = s.to_string();
            }
        }
        if let Some(val) = store.get("ssh_password") {
            if let Some(s) = val.as_str() {
                config.ssh_password = s.to_string();
            }
        }
        if let Some(val) = store.get("allowed_paths") {
            if let Some(arr) = val.as_array() {
                config.allowed_paths = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
        if let Some(val) = store.get("rfc_port") {
            if let Some(p) = val.as_u64() {
                config.rfc_port = p as u16;
            }
        }
        if let Some(val) = store.get("ssh_port") {
            if let Some(p) = val.as_u64() {
                config.ssh_port = p as u16;
            }
        }
        if let Some(val) = store.get("connector_enabled") {
            if let Some(b) = val.as_bool() {
                config.connector_enabled = b;
            }
        }
        if let Some(val) = store.get("scope_files") {
            if let Some(b) = val.as_bool() {
                config.scope_files = b;
            }
        }
        if let Some(val) = store
            .get("scope_exec")
            .or_else(|| store.get("scope_code_execution"))
        {
            if let Some(b) = val.as_bool() {
                config.scope_exec = b;
            }
        }
        if let Some(val) = store.get("scope_browser") {
            if let Some(b) = val.as_bool() {
                config.scope_browser = b;
            }
        }
        if let Some(val) = store
            .get("scope_desktop")
            .or_else(|| store.get("scope_computer_use"))
        {
            if let Some(b) = val.as_bool() {
                config.scope_desktop = b;
            }
        }
        if let Some(val) = store.get("global_hotkey") {
            if let Some(s) = val.as_str() {
                config.global_hotkey = s.to_string();
            }
        }
        if let Some(val) = store.get("close_to_tray") {
            if let Some(b) = val.as_bool() {
                config.close_to_tray = b;
            }
        }
        if let Some(val) = store.get("app_language") {
            if let Some(s) = val.as_str() {
                config.app_language = s.to_string();
            }
        }
        if let Some(val) = store.get("show_menu_bar") {
            if let Some(b) = val.as_bool() {
                config.show_menu_bar = b;
            }
        }

        Ok(config)
    }

    /// Saves configuration to tauri-plugin-store settings.json.
    pub fn save_to_store<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        let store = app
            .store(STORE_FILENAME)
            .map_err(|e| format!("Failed to open store {STORE_FILENAME}: {e}"))?;

        store.set("server_url", serde_json::json!(self.server_url));
        store.set("server_profiles", serde_json::json!(self.server_profiles));
        store.set("gateway_id", serde_json::json!(self.gateway_id));
        store.set("workspace_path", serde_json::json!(self.workspace_path));
        store.set("rfc_enabled", serde_json::json!(self.rfc_enabled));
        store.set("rfc_password", serde_json::json!(self.rfc_password));
        store.set("ssh_password", serde_json::json!(self.ssh_password));
        store.set("allowed_paths", serde_json::json!(self.allowed_paths));
        store.set("rfc_port", serde_json::json!(self.rfc_port));
        store.set("ssh_port", serde_json::json!(self.ssh_port));
        store.set(
            "connector_enabled",
            serde_json::json!(self.connector_enabled),
        );
        store.set("scope_files", serde_json::json!(self.scope_files));
        store.set("scope_exec", serde_json::json!(self.scope_exec));
        store.set("scope_browser", serde_json::json!(self.scope_browser));
        store.set("scope_desktop", serde_json::json!(self.scope_desktop));
        store.set("global_hotkey", serde_json::json!(self.global_hotkey));
        store.set("close_to_tray", serde_json::json!(self.close_to_tray));
        store.set("app_language", serde_json::json!(self.app_language));
        store.set("show_menu_bar", serde_json::json!(self.show_menu_bar));

        store
            .save()
            .map_err(|e| format!("Failed to save store {STORE_FILENAME}: {e}"))?;

        Ok(())
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
///
/// The settings UI writes through the store plugin, so a periodic read of this
/// file is enough to drive hot reload without restarting the app.
pub fn load_from_file(path: &Path) -> Option<AppConfig> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    serde_json::from_value(value).ok()
}

/// Resolves the store file path using tauri-plugin-store.
pub fn store_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    tauri_plugin_store::resolve_store_path(app, STORE_FILENAME)
        .map_err(|e| format!("resolve store path failed: {e}"))
}

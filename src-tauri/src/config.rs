//! Configuration module for AQVN Workspace.
//!
//! Exposes typed settings over tauri-plugin-store `settings.json`.
//! Default values required by validation contract VAL-SCAF-010:
//!   server_url = "https://a0.matmagoc.com"
//!   rfc_enabled = false
//!   rfc_port = 55080
//!   ssh_port = 55022

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
use tokio::sync::watch;

pub const DEFAULT_SERVER_URL: &str = "https://a0.matmagoc.com";
pub const DEFAULT_RFC_ENABLED: bool = false;
pub const DEFAULT_RFC_PORT: u16 = 55080;
pub const DEFAULT_SSH_PORT: u16 = 55022;
pub const DEFAULT_ALLOWED_PATHS: &[&str] = &["~/Workspace"];
pub const STORE_FILENAME: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub server_url: String,
    pub rfc_enabled: bool,
    pub rfc_password: String,
    pub ssh_password: String,
    pub allowed_paths: Vec<String>,
    pub rfc_port: u16,
    pub ssh_port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: DEFAULT_SERVER_URL.to_string(),
            rfc_enabled: DEFAULT_RFC_ENABLED,
            rfc_password: String::new(),
            ssh_password: String::new(),
            allowed_paths: DEFAULT_ALLOWED_PATHS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            rfc_port: DEFAULT_RFC_PORT,
            ssh_port: DEFAULT_SSH_PORT,
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

        Ok(config)
    }

    /// Saves configuration to tauri-plugin-store settings.json.
    pub fn save_to_store<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), String> {
        let store = app
            .store(STORE_FILENAME)
            .map_err(|e| format!("Failed to open store {STORE_FILENAME}: {e}"))?;

        store.set("server_url", serde_json::json!(self.server_url));
        store.set("rfc_enabled", serde_json::json!(self.rfc_enabled));
        store.set("rfc_password", serde_json::json!(self.rfc_password));
        store.set("ssh_password", serde_json::json!(self.ssh_password));
        store.set("allowed_paths", serde_json::json!(self.allowed_paths));
        store.set("rfc_port", serde_json::json!(self.rfc_port));
        store.set("ssh_port", serde_json::json!(self.ssh_port));

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

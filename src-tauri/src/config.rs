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
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_store::StoreExt;
use tokio::sync::{mpsc, oneshot, watch};

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
    aqvn_core::config::default_gateway_id()
}

pub fn deterministic_gateway_id() -> String {
    aqvn_core::config::deterministic_gateway_id()
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

const fn default_task_chime_volume() -> f32 {
    DEFAULT_TASK_CHIME_VOLUME
}

fn default_task_chime_tone() -> String {
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
}

fn default_security_policy() -> String {
    "supervised".to_string()
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

    /// Builds a unified Sandbox merging workspace_path and allowed_paths.
    #[cfg(not(target_os = "android"))]
    pub fn build_sandbox(&self) -> crate::rfc::sandbox::Sandbox {
        crate::rfc::sandbox::Sandbox::from_allowed_paths(&self.unified_allowed_paths())
    }

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
        if let Some(val) = store.get("task_chime_enabled") {
            if let Some(b) = val.as_bool() {
                config.task_chime_enabled = b;
            }
        }
        if let Some(val) = store.get("task_notification_enabled") {
            if let Some(b) = val.as_bool() {
                config.task_notification_enabled = b;
            }
        }
        if let Some(val) = store.get("task_chime_volume") {
            if let Some(f) = val.as_f64() {
                config.task_chime_volume = f.clamp(0.0, 1.0) as f32;
            }
        }
        if let Some(val) = store.get("task_chime_tone") {
            if let Some(s) = val.as_str() {
                config.task_chime_tone = s.to_string();
            }
        }
        if let Some(val) = store.get("security_policy") {
            if let Some(s) = val.as_str() {
                config.security_policy = s.to_string();
            }
        }
        if let Some(val) = store.get("window_state") {
            if !val.is_null() {
                if let Ok(ws) = serde_json::from_value::<WindowState>(val.clone()) {
                    config.window_state = Some(ws);
                }
            }
        }
        if let Some(val) = store.get("tls_ca_cert_path") {
            if let Some(s) = val.as_str() {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    config.tls_ca_cert_path = Some(trimmed.to_string());
                } else {
                    config.tls_ca_cert_path = None;
                }
            } else if val.is_null() {
                config.tls_ca_cert_path = None;
            }
        }
        if let Some(val) = store.get("tls_skip_verify") {
            if let Some(b) = val.as_bool() {
                config.tls_skip_verify = b;
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
        store.set(
            "task_chime_enabled",
            serde_json::json!(self.task_chime_enabled),
        );
        store.set(
            "task_notification_enabled",
            serde_json::json!(self.task_notification_enabled),
        );
        store.set(
            "task_chime_volume",
            serde_json::json!(self.task_chime_volume),
        );
        store.set("task_chime_tone", serde_json::json!(self.task_chime_tone));
        store.set("security_policy", serde_json::json!(self.security_policy));
        store.set("window_state", serde_json::json!(self.window_state));
        store.set("tls_ca_cert_path", serde_json::json!(self.tls_ca_cert_path));
        store.set("tls_skip_verify", serde_json::json!(self.tls_skip_verify));

        store
            .save()
            .map_err(|e| format!("Failed to save store {STORE_FILENAME}: {e}"))?;

        Ok(())
    }
}

/// Persists window geometry to settings.json and updates ConfigWatcher if present.
pub fn persist_window_geometry<R: Runtime>(
    app: &AppHandle<R>,
    state: WindowState,
) -> Result<(), String> {
    let store = app
        .store(STORE_FILENAME)
        .map_err(|e| format!("Failed to open store {STORE_FILENAME}: {e}"))?;

    store.set("window_state", serde_json::json!(state));
    store
        .save()
        .map_err(|e| format!("Failed to save store {STORE_FILENAME}: {e}"))?;

    if let Some(watcher) = app.try_state::<ConfigWatcher>() {
        let mut conf = watcher.current();
        conf.window_state = Some(state);
        watcher.update(conf);
    }

    Ok(())
}

/// Applies saved window state (dimensions, position, maximized) to a WindowConfig.
pub fn apply_window_state(
    window_config: &mut tauri::utils::config::WindowConfig,
    state: &WindowState,
) {
    window_config.width = state.width;
    window_config.height = state.height;
    window_config.x = Some(state.x);
    window_config.y = Some(state.y);
    window_config.center = false;
    window_config.maximized = state.is_maximized;
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

/// Loads configuration from file only if file modification timestamp (mtime) has changed.
///
/// Skips file reading and JSON deserialization when `last_mtime` matches the file's current mtime.
/// On successful parse, updates `*last_mtime` to the file's mtime and returns `Some(AppConfig)`.
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

/// Resolves the store file path using tauri-plugin-store.
pub fn store_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    tauri_plugin_store::resolve_store_path(app, STORE_FILENAME)
        .map_err(|e| format!("resolve store path failed: {e}"))
}

enum DebounceMsg {
    Update(WindowState),
    Flush(Option<oneshot::Sender<()>>),
}

/// Debounces window geometry writes to settings store by 500ms (VAL-RES-004).
#[derive(Clone)]
pub struct GeometryDebouncer {
    tx: mpsc::UnboundedSender<DebounceMsg>,
}

impl GeometryDebouncer {
    pub const DEFAULT_DELAY: Duration = Duration::from_millis(500);

    pub fn new<R: Runtime>(app: AppHandle<R>) -> Self {
        Self::new_with_delay(app, Self::DEFAULT_DELAY)
    }

    pub fn new_with_delay<R: Runtime>(app: AppHandle<R>, delay: Duration) -> Self {
        Self::new_with_callback(delay, move |state| {
            let _ = persist_window_geometry(&app, state);
        })
    }

    pub fn new_with_callback<F>(delay: Duration, on_persist: F) -> Self
    where
        F: Fn(WindowState) + Send + 'static,
    {
        let (tx, mut rx) = mpsc::unbounded_channel::<DebounceMsg>();
        tauri::async_runtime::spawn(async move {
            let mut pending: Option<WindowState> = None;
            let mut deadline: Option<tokio::time::Instant> = None;

            loop {
                let sleep_fut = async {
                    match deadline {
                        Some(d) => tokio::time::sleep_until(d).await,
                        None => std::future::pending().await,
                    }
                };

                tokio::select! {
                    msg = rx.recv() => {
                        match msg {
                            Some(DebounceMsg::Update(state)) => {
                                pending = Some(state);
                                deadline = Some(tokio::time::Instant::now() + delay);
                            }
                            Some(DebounceMsg::Flush(ack)) => {
                                if let Some(state) = pending.take() {
                                    on_persist(state);
                                }
                                deadline = None;
                                if let Some(ack) = ack {
                                    let _ = ack.send(());
                                }
                            }
                            None => {
                                if let Some(state) = pending.take() {
                                    on_persist(state);
                                }
                                break;
                            }
                        }
                    }
                    _ = sleep_fut, if deadline.is_some() => {
                        deadline = None;
                        if let Some(state) = pending.take() {
                            on_persist(state);
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub fn debounce(&self, state: WindowState) {
        let _ = self.tx.send(DebounceMsg::Update(state));
    }

    pub fn flush(&self) {
        let _ = self.tx.send(DebounceMsg::Flush(None));
    }

    pub async fn flush_wait(&self) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self.tx.send(DebounceMsg::Flush(Some(ack_tx))).is_ok() {
            let _ = ack_rx.await;
        }
    }
}

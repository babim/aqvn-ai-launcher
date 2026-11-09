//! AQVN Standalone Headless Agent & Multi-Host Bridge (`aqvn-agent`).
//!
//! Executes as a standalone CLI daemon without GUI or display server requirements.

use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(not(target_os = "android"))]
use aqvn_connector::GatewayConfig;

const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_SERVER_URL: &str = "https://a0.matmagoc.com";
pub const DEFAULT_HOST_LABEL: &str = "aqvn-agent-node";
pub const DEFAULT_WORKSPACE_PATH: &str = "~/Workspace";

const HELP_TEXT: &str = r#"aqvn-agent {VERSION}
AQVN Standalone Headless Agent & Multi-Host Bridge

USAGE:
    aqvn-agent [COMMAND] [OPTIONS]

COMMANDS:
    start       Start agent connection in foreground
    config      Configure agent settings
    install     Install and enable systemd daemon service
    uninstall   Stop and remove systemd daemon service
    status      Show connection status and diagnostics

OPTIONS:
    -s, --server <URL>       AQVN Gateway server URL
    -u, --user <USER>        Username for authentication
    -p, --pass <PASS>        Password for authentication
    -t, --token <TOKEN>      Session token or API key
    -l, --label <LABEL>      Human-readable host label
    --scopes <SCOPES>        Enabled scopes (files,code_execution,browser,computer_use)
    -w, --workspace <PATH>   Workspace root directory
    -c, --config <PATH>      Path to settings.json
    --user                   Target user systemd unit (~/.config/systemd/user)
    --system                 Target system systemd unit (/etc/systemd/system)
    --gateway-id <ID>        Custom gateway ID
    --ping-interval <SECS>   Keepalive ping interval in seconds
    --tls-skip-verify        Accept self-signed or invalid TLS certs
    -h, --help               Print help information
    -V, --version            Print version information
"#;

pub fn format_version() -> String {
    format!("aqvn-agent {VERSION}\n")
}

pub fn format_help() -> String {
    HELP_TEXT.replace("{VERSION}", VERSION)
}

fn default_server_url() -> String {
    DEFAULT_SERVER_URL.to_string()
}

fn default_gateway_id() -> String {
    aqvn_core::config::default_gateway_id()
}

fn default_host_label() -> String {
    if let Ok(hostname) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
        let trimmed = hostname.trim();
        if !trimmed.is_empty() {
            return format!("aqvn-{trimmed}");
        }
    }
    DEFAULT_HOST_LABEL.to_string()
}

fn default_workspace_path() -> String {
    DEFAULT_WORKSPACE_PATH.to_string()
}

fn default_ping_interval_secs() -> u64 {
    if let Ok(v) = env::var("AQVN_PING_INTERVAL_SECS") {
        if let Ok(secs) = v.trim().parse::<u64>() {
            if secs > 0 {
                return secs;
            }
        }
    }
    aqvn_connector::client::DEFAULT_PING_INTERVAL_SECS
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSettings {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default = "default_gateway_id")]
    pub gateway_id: String,
    #[serde(default = "default_host_label")]
    pub host_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default = "default_workspace_path")]
    pub workspace_path: String,
    #[serde(default = "default_true")]
    pub scope_files: bool,
    #[serde(default = "default_true")]
    pub scope_exec: bool,
    #[serde(default = "default_true")]
    pub scope_browser: bool,
    #[serde(default = "default_true")]
    pub scope_desktop: bool,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    #[serde(default = "default_ping_interval_secs")]
    pub ping_interval_secs: u64,
    #[serde(default)]
    pub tls_skip_verify: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_ca_cert_path: Option<String>,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            gateway_id: default_gateway_id(),
            host_label: default_host_label(),
            username: None,
            password: None,
            token: None,
            workspace_path: default_workspace_path(),
            scope_files: true,
            scope_exec: true,
            scope_browser: true,
            scope_desktop: true,
            allowed_paths: Vec::new(),
            ping_interval_secs: default_ping_interval_secs(),
            tls_skip_verify: false,
            tls_ca_cert_path: None,
        }
    }
}

impl AgentSettings {
    pub fn active_scopes(&self) -> Vec<String> {
        let mut scopes = Vec::new();
        if self.scope_files {
            scopes.push("files".to_string());
        }
        if self.scope_exec {
            scopes.push("code_execution".to_string());
        }
        if self.scope_browser {
            scopes.push("browser".to_string());
        }
        if self.scope_desktop {
            scopes.push("computer_use".to_string());
        }
        scopes
    }

    #[cfg(not(target_os = "android"))]
    pub fn to_gateway_config(&self) -> GatewayConfig {
        GatewayConfig {
            enabled: true,
            server_url: self.server_url.clone(),
            gateway_id: if self.gateway_id.trim().is_empty() {
                default_gateway_id()
            } else {
                self.gateway_id.clone()
            },
            host_label: if self.host_label.trim().is_empty() {
                default_host_label()
            } else {
                self.host_label.clone()
            },
            session_cookie: self.token.clone().or_else(|| self.password.clone()),
            scope_files: self.scope_files,
            scope_exec: self.scope_exec,
            scope_browser: self.scope_browser,
            scope_desktop: self.scope_desktop,
            ping_interval_secs: self.ping_interval_secs,
            reconnect_delay_ms: aqvn_connector::client::DEFAULT_RECONNECT_DELAY_MS,
            workspace_path: self.workspace_path.clone(),
            allowed_paths: self.allowed_paths.clone(),
            tls_skip_verify: self.tls_skip_verify,
            tls_ca_cert_path: self.tls_ca_cert_path.clone(),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        if p.exists() {
            if let Ok(content) = std::fs::read_to_string(p) {
                if let Ok(settings) = serde_json::from_str::<Self>(&content) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json =
            serde_json::to_string_pretty(self).map_err(|e| io::Error::other(e.to_string()))?;
        std::fs::write(p, json)?;
        Ok(())
    }
}

pub fn default_settings_path() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg)
                .join("net.aqvn.agent")
                .join("settings.json");
        }
    }
    if let Ok(home) = env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home)
                .join(".config")
                .join("net.aqvn.agent")
                .join("settings.json");
        }
    }
    if let Ok(profile) = env::var("USERPROFILE") {
        if !profile.trim().is_empty() {
            return PathBuf::from(profile)
                .join(".config")
                .join("net.aqvn.agent")
                .join("settings.json");
        }
    }
    env::temp_dir().join("net.aqvn.agent").join("settings.json")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliOptions {
    pub server: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub token: Option<String>,
    pub label: Option<String>,
    pub scopes: Option<String>,
    pub workspace: Option<String>,
    pub gateway_id: Option<String>,
    pub ping_interval: Option<u64>,
    pub tls_skip_verify: Option<bool>,
    pub config_path: Option<PathBuf>,
    pub user_unit: Option<bool>,
}

impl CliOptions {
    pub fn is_empty(&self) -> bool {
        self.server.is_none()
            && self.user.is_none()
            && self.pass.is_none()
            && self.token.is_none()
            && self.label.is_none()
            && self.scopes.is_none()
            && self.workspace.is_none()
            && self.gateway_id.is_none()
            && self.ping_interval.is_none()
            && self.tls_skip_verify.is_none()
            && self.config_path.is_none()
            && self.user_unit.is_none()
    }

    pub fn apply_to(&self, settings: &mut AgentSettings) {
        if let Some(ref server) = self.server {
            settings.server_url = server.clone();
        }
        if let Some(ref user) = self.user {
            settings.username = Some(user.clone());
        }
        if let Some(ref pass) = self.pass {
            settings.password = Some(pass.clone());
        }
        if let Some(ref token) = self.token {
            settings.token = Some(token.clone());
        }
        if let Some(ref label) = self.label {
            settings.host_label = label.clone();
        }
        if let Some(ref scopes) = self.scopes {
            parse_scopes_into(scopes, settings);
        }
        if let Some(ref ws) = self.workspace {
            settings.workspace_path = ws.clone();
        }
        if let Some(ref gw) = self.gateway_id {
            settings.gateway_id = gw.clone();
        }
        if let Some(interval) = self.ping_interval {
            settings.ping_interval_secs = interval;
        }
        if let Some(skip) = self.tls_skip_verify {
            settings.tls_skip_verify = skip;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliAction {
    Help,
    Version,
    Start,
    Config,
    Install,
    Uninstall,
    Status,
    Menu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCli {
    pub action: CliAction,
    pub options: CliOptions,
}

pub fn parse_cli_args(args: &[String]) -> Result<ParsedCli, String> {
    if args.is_empty() {
        return Ok(ParsedCli {
            action: CliAction::Menu,
            options: CliOptions::default(),
        });
    }

    let mut action: Option<CliAction> = None;
    let mut options = CliOptions::default();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        if arg == "-h" || arg == "--help" {
            action = Some(CliAction::Help);
            i += 1;
            continue;
        }
        if arg == "-V" || arg == "--version" {
            action = Some(CliAction::Version);
            i += 1;
            continue;
        }

        if arg.starts_with('-') {
            let (key, inline_val) = match arg.split_once('=') {
                Some((k, v)) => (k, Some(v.to_string())),
                None => (arg.as_str(), None),
            };

            let get_val = |i: &mut usize| -> Result<String, String> {
                if let Some(ref v) = inline_val {
                    Ok(v.clone())
                } else if *i + 1 < args.len() {
                    *i += 1;
                    Ok(args[*i].clone())
                } else {
                    Err(format!("Option '{key}' requires a value"))
                }
            };

            match key {
                "-s" | "--server" => {
                    options.server = Some(get_val(&mut i)?);
                }
                "-u" | "--user" => {
                    let is_service_cmd = action == Some(CliAction::Install)
                        || action == Some(CliAction::Uninstall)
                        || action == Some(CliAction::Status)
                        || args
                            .iter()
                            .any(|a| a == "install" || a == "uninstall" || a == "status");
                    if is_service_cmd
                        && (inline_val.is_none()
                            || inline_val.as_deref() == Some("true")
                            || inline_val.as_deref() == Some("false"))
                        && (inline_val.is_some()
                            || i + 1 >= args.len()
                            || args[i + 1].starts_with('-')
                            || args[i + 1] == "install"
                            || args[i + 1] == "uninstall"
                            || args[i + 1] == "status")
                    {
                        if let Some(ref v) = inline_val {
                            options.user_unit = Some(v.parse().unwrap_or(true));
                        } else {
                            options.user_unit = Some(true);
                        }
                    } else {
                        options.user = Some(get_val(&mut i)?);
                    }
                }
                "--system" => {
                    options.user_unit = Some(false);
                }
                "-p" | "--pass" | "--password" => {
                    options.pass = Some(get_val(&mut i)?);
                }
                "-t" | "--token" => {
                    options.token = Some(get_val(&mut i)?);
                }
                "-l" | "--label" => {
                    options.label = Some(get_val(&mut i)?);
                }
                "--scopes" => {
                    options.scopes = Some(get_val(&mut i)?);
                }
                "-w" | "--workspace" => {
                    options.workspace = Some(get_val(&mut i)?);
                }
                "-c" | "--config" => {
                    let val = get_val(&mut i)?;
                    options.config_path = Some(PathBuf::from(val));
                }
                "--gateway-id" => {
                    options.gateway_id = Some(get_val(&mut i)?);
                }
                "--ping-interval" => {
                    let val = get_val(&mut i)?;
                    let parsed_secs = val
                        .parse::<u64>()
                        .map_err(|e| format!("Invalid --ping-interval value: {e}"))?;
                    if parsed_secs == 0 {
                        return Err("Invalid --ping-interval value: must be >= 1".to_string());
                    }
                    options.ping_interval = Some(parsed_secs);
                }
                "--tls-skip-verify" => {
                    if let Some(ref v) = inline_val {
                        let parsed_bool = v.parse::<bool>().unwrap_or(true);
                        options.tls_skip_verify = Some(parsed_bool);
                    } else {
                        options.tls_skip_verify = Some(true);
                    }
                }
                other => {
                    return Err(format!("Unknown option '{other}'"));
                }
            }
        } else {
            let cmd = match arg.as_str() {
                "start" => CliAction::Start,
                "config" => CliAction::Config,
                "install" => CliAction::Install,
                "uninstall" => CliAction::Uninstall,
                "status" => CliAction::Status,
                "help" => CliAction::Help,
                "version" => CliAction::Version,
                other => {
                    return Err(format!("Unknown command or unexpected argument '{other}'"));
                }
            };

            if let Some(existing) = action {
                if existing != cmd {
                    return Err(format!("Unexpected argument '{arg}'"));
                }
            } else {
                action = Some(cmd);
            }
        }

        i += 1;
    }

    let final_action = match action {
        Some(a) => a,
        None => {
            if options.is_empty() {
                CliAction::Menu
            } else {
                CliAction::Start
            }
        }
    };

    Ok(ParsedCli {
        action: final_action,
        options,
    })
}

pub fn parse_action(args: &[String]) -> CliAction {
    match parse_cli_args(args) {
        Ok(parsed) => parsed.action,
        Err(_) => CliAction::Help,
    }
}

pub fn print_menu<W: Write>(writer: &mut W) -> io::Result<()> {
    writeln!(
        writer,
        "======================================================"
    )?;
    writeln!(writer, "AQVN Standalone Headless Agent & Multi-Host Bridge")?;
    writeln!(
        writer,
        "======================================================"
    )?;
    writeln!(writer, "1. Start (Foreground Agent Loop)")?;
    writeln!(writer, "2. Configure (Settings & Scopes)")?;
    writeln!(writer, "3. Install Service (systemd)")?;
    writeln!(writer, "4. Uninstall Service (systemd)")?;
    writeln!(writer, "5. Status (Connectivity & Diagnostics)")?;
    writeln!(writer, "0. Exit")?;
    writeln!(
        writer,
        "======================================================"
    )?;
    write!(writer, "Enter selection [0-5]: ")?;
    writer.flush()?;
    Ok(())
}

pub fn parse_scopes_into(input: &str, settings: &mut AgentSettings) {
    let lower = input.trim().to_lowercase();
    if lower == "all" || lower == "default" {
        settings.scope_files = true;
        settings.scope_exec = true;
        settings.scope_browser = true;
        settings.scope_desktop = true;
        return;
    }
    if lower == "none" {
        settings.scope_files = false;
        settings.scope_exec = false;
        settings.scope_browser = false;
        settings.scope_desktop = false;
        return;
    }
    settings.scope_files = lower.contains("files");
    settings.scope_exec = lower.contains("code_execution") || lower.contains("exec");
    settings.scope_browser = lower.contains("browser");
    settings.scope_desktop = lower.contains("computer_use") || lower.contains("desktop");
}

fn read_prompt_line<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut line = String::new();
    let bytes = reader.read_line(&mut line)?;
    if bytes == 0 {
        return Ok(None);
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(Some(String::new()))
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

pub fn run_configure<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    settings: &mut AgentSettings,
    settings_path: &Path,
) -> io::Result<()> {
    writeln!(writer, "\n--- Configure Agent Settings ---")?;

    write!(writer, "Server URL [{}]: ", settings.server_url)?;
    writer.flush()?;
    if let Some(val) = read_prompt_line(reader)? {
        if !val.is_empty() {
            settings.server_url = val;
        }
    }

    let cur_user = settings.username.as_deref().unwrap_or("none");
    write!(writer, "Username [{}]: ", cur_user)?;
    writer.flush()?;
    if let Some(val) = read_prompt_line(reader)? {
        if !val.is_empty() {
            settings.username = Some(val);
        }
    }

    let cur_auth = if settings.token.is_some() || settings.password.is_some() {
        "configured"
    } else {
        "none"
    };
    write!(writer, "Password or Token [{}]: ", cur_auth)?;
    writer.flush()?;
    if let Some(val) = read_prompt_line(reader)? {
        if !val.is_empty() {
            settings.token = Some(val.clone());
            settings.password = Some(val);
        }
    }

    write!(writer, "Host Label [{}]: ", settings.host_label)?;
    writer.flush()?;
    if let Some(val) = read_prompt_line(reader)? {
        if !val.is_empty() {
            settings.host_label = val;
        }
    }

    let cur_scopes = settings.active_scopes().join(", ");
    write!(
        writer,
        "Scopes (files, code_execution, browser, computer_use) [{}]: ",
        if cur_scopes.is_empty() {
            "none"
        } else {
            &cur_scopes
        }
    )?;
    writer.flush()?;
    if let Some(val) = read_prompt_line(reader)? {
        if !val.is_empty() {
            parse_scopes_into(&val, settings);
        }
    }

    write!(writer, "Workspace Path [{}]: ", settings.workspace_path)?;
    writer.flush()?;
    if let Some(val) = read_prompt_line(reader)? {
        if !val.is_empty() {
            settings.workspace_path = val;
        }
    }

    settings.save_to_file(settings_path)?;
    writeln!(
        writer,
        "Configuration persisted to {}\n",
        settings_path.display()
    )?;
    writer.flush()?;
    Ok(())
}

pub async fn test_server_connectivity(
    server_url: &str,
    tls_skip_verify: bool,
) -> Result<u16, String> {
    let mut builder = reqwest::Client::builder().timeout(Duration::from_secs(5));
    if tls_skip_verify {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = builder
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let resp = client
        .get(server_url)
        .send()
        .await
        .map_err(|e| format!("{e}"))?;

    Ok(resp.status().as_u16())
}

pub async fn run_status<W: Write>(
    writer: &mut W,
    settings: &AgentSettings,
    settings_path: &Path,
    test_conn: bool,
) -> io::Result<()> {
    run_status_ex(writer, settings, settings_path, None, test_conn).await
}

pub async fn run_status_ex<W: Write>(
    writer: &mut W,
    settings: &AgentSettings,
    settings_path: &Path,
    user_unit_opt: Option<bool>,
    test_conn: bool,
) -> io::Result<()> {
    writeln!(
        writer,
        "\n======================================================"
    )?;
    writeln!(writer, "aqvn-agent Status & Diagnostics")?;
    writeln!(
        writer,
        "======================================================"
    )?;
    writeln!(writer, "Server URL:     {}", settings.server_url)?;
    writeln!(writer, "Gateway ID:     {}", settings.gateway_id)?;
    writeln!(writer, "Host Label:     {}", settings.host_label)?;
    writeln!(
        writer,
        "Username:       {}",
        settings.username.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "Auth Token:     {}",
        if settings.token.is_some() || settings.password.is_some() {
            "Configured"
        } else {
            "None"
        }
    )?;
    writeln!(writer, "Active Scopes:  {:?}", settings.active_scopes())?;
    writeln!(writer, "Workspace Path: {}", settings.workspace_path)?;
    writeln!(
        writer,
        "Settings File:  {} ({})",
        settings_path.display(),
        if settings_path.exists() {
            "Found"
        } else {
            "Not found"
        }
    )?;
    let service_status = query_service_status(user_unit_opt);
    writeln!(
        writer,
        "Service Unit:   {} ({}, {})",
        service_status.unit_path.display(),
        if service_status.is_user_unit {
            "user"
        } else {
            "system"
        },
        if service_status.installed {
            "Installed"
        } else {
            "Not installed"
        }
    )?;

    if test_conn {
        writeln!(
            writer,
            "\nTesting connectivity to {}...",
            settings.server_url
        )?;
        match test_server_connectivity(&settings.server_url, settings.tls_skip_verify).await {
            Ok(status_code) => {
                writeln!(writer, "Connectivity:   OK (HTTP {})", status_code)?;
            }
            Err(err) => {
                writeln!(writer, "Connectivity:   FAILED ({})", err)?;
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    {
        let metrics = aqvn_connector::handlers::telemetry::collect_system_metrics();
        writeln!(writer, "\nSystem Resources:")?;
        writeln!(writer, "OS:             {}", metrics.os)?;
        writeln!(writer, "Architecture:   {}", metrics.arch)?;
        writeln!(writer, "CPU Cores:      {}", metrics.cpu_count)?;
        writeln!(writer, "CPU Usage:      {:.1}%", metrics.cpu_usage_percent)?;
        writeln!(writer, "RAM Total:      {} MB", metrics.total_ram_mb)?;
        writeln!(writer, "RAM Available:  {} MB", metrics.available_ram_mb)?;
        writeln!(writer, "RAM Free:       {} MB", metrics.free_ram_mb)?;
        writeln!(writer, "Disk Free:      {:.2} GB", metrics.disk_free_gb)?;
    }

    writeln!(
        writer,
        "======================================================\n"
    )?;
    writer.flush()?;
    Ok(())
}

pub async fn run_start<W: Write>(
    writer: &mut W,
    settings: &AgentSettings,
    stop_signal: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(writer, "Starting aqvn-agent in foreground...")?;
    writeln!(writer, "Server URL:     {}", settings.server_url)?;
    writeln!(writer, "Gateway ID:     {}", settings.gateway_id)?;
    writeln!(writer, "Host Label:     {}", settings.host_label)?;
    writeln!(writer, "Active Scopes:  {:?}", settings.active_scopes())?;
    writeln!(writer, "Workspace:      {}", settings.workspace_path)?;
    writeln!(writer, "Press Ctrl+C to stop.\n")?;
    writer.flush()?;

    #[cfg(not(target_os = "android"))]
    {
        let gateway_config = settings.to_gateway_config();
        let client = aqvn_connector::GatewayClient::new(gateway_config);
        let (handle, join_handle) = client.start();

        let handle_clone = handle.clone();
        let handshake_logger = tokio::spawn(async move {
            if let Ok(()) = handle_clone
                .wait_for_handshake(Duration::from_secs(10))
                .await
            {
                println!("Node successfully registered in AQVN client pool.");
            }
        });

        if let Some(stop_rx) = stop_signal {
            let _ = stop_rx.await;
        } else {
            let _ = tokio::signal::ctrl_c().await;
        }

        handshake_logger.abort();
        writeln!(writer, "\nStopping aqvn-agent...")?;
        writer.flush()?;
        handle.stop();
        let _ = join_handle.await;
        writeln!(writer, "Agent stopped.")?;
        writer.flush()?;
    }

    #[cfg(target_os = "android")]
    {
        let _ = stop_signal;
    }

    Ok(())
}

pub fn resolve_is_user_unit(user_unit_opt: Option<bool>) -> bool {
    if let Some(user_unit) = user_unit_opt {
        return user_unit;
    }
    #[cfg(unix)]
    {
        if unsafe { libc::geteuid() } == 0 {
            return false;
        }
    }
    true
}

pub fn resolve_systemd_unit_path(is_user_unit: bool) -> PathBuf {
    if let Ok(override_path) = env::var("AQVN_SYSTEMD_UNIT_PATH") {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if is_user_unit {
        if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed)
                    .join("systemd")
                    .join("user")
                    .join("aqvn-agent.service");
            }
        }
        if let Ok(home) = env::var("HOME") {
            let trimmed = home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed)
                    .join(".config")
                    .join("systemd")
                    .join("user")
                    .join("aqvn-agent.service");
            }
        }
        PathBuf::from("/etc/systemd/user/aqvn-agent.service")
    } else {
        PathBuf::from("/etc/systemd/system/aqvn-agent.service")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub is_user_unit: bool,
    pub unit_path: PathBuf,
    pub installed: bool,
}

pub fn query_service_status(user_unit_opt: Option<bool>) -> ServiceStatus {
    let is_user_unit = resolve_is_user_unit(user_unit_opt);
    let unit_path = resolve_systemd_unit_path(is_user_unit);
    let installed = unit_path.exists();
    ServiceStatus {
        is_user_unit,
        unit_path,
        installed,
    }
}

pub fn resolve_binary_path() -> PathBuf {
    if let Ok(override_bin) = env::var("AQVN_AGENT_BIN_PATH") {
        let trimmed = override_bin.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Ok(canon) = exe.canonicalize() {
            return canon;
        }
        return exe;
    }
    PathBuf::from("/usr/local/bin/aqvn-agent")
}

pub fn resolve_working_directory(raw_path: &str) -> PathBuf {
    let trimmed = raw_path.trim();
    if trimmed.starts_with("~/") || trimmed == "~" {
        if let Ok(home) = env::var("HOME") {
            if trimmed == "~" {
                return PathBuf::from(home);
            } else {
                return PathBuf::from(home).join(&trimmed[2..]);
            }
        }
    }
    let p = PathBuf::from(trimmed);
    if p.is_absolute() {
        p
    } else if let Ok(cwd) = env::current_dir() {
        cwd.join(p)
    } else {
        p
    }
}

pub fn generate_systemd_unit(
    binary_path: &Path,
    working_dir: &Path,
    config_path: Option<&Path>,
    is_user_unit: bool,
) -> String {
    let exec_start = match config_path {
        Some(cfg) => format!("{} start --config {}", binary_path.display(), cfg.display()),
        None => format!("{} start", binary_path.display()),
    };

    let wanted_by = if is_user_unit {
        "default.target"
    } else {
        "multi-user.target"
    };

    format!(
        "[Unit]\n\
Description=AQVN Standalone Headless Agent & Multi-Host Bridge\n\
After=network.target\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart={exec_start}\n\
WorkingDirectory={}\n\
Restart=always\n\
RestartSec=5\n\
KillMode=process\n\
\n\
[Install]\n\
WantedBy={wanted_by}\n",
        working_dir.display()
    )
}

fn run_systemctl_cmd(is_user_unit: bool, args: &[&str]) -> io::Result<std::process::ExitStatus> {
    let mut cmd = std::process::Command::new("systemctl");
    if is_user_unit {
        cmd.arg("--user");
    }
    cmd.args(args);
    cmd.status()
}

fn execute_systemctl_action<W: Write>(
    writer: &mut W,
    is_user_unit: bool,
    action: &str,
    extra_args: &[&str],
) {
    let unit_flag = if is_user_unit { " --user" } else { "" };
    let mut all_args = Vec::with_capacity(1 + extra_args.len());
    all_args.push(action);
    all_args.extend_from_slice(extra_args);

    let cmd_str = format!("systemctl{unit_flag} {}", all_args.join(" "));
    let _ = writeln!(writer, "Running: {cmd_str}");

    match run_systemctl_cmd(is_user_unit, &all_args) {
        Ok(status) => {
            if status.success() {
                let _ = writeln!(writer, "Command '{cmd_str}' succeeded.");
            } else {
                let _ = writeln!(
                    writer,
                    "Notice: command '{cmd_str}' exited with status: {}",
                    status
                );
            }
        }
        Err(err) => {
            let _ = writeln!(
                writer,
                "Notice: could not run '{cmd_str}': {err} (systemd may not be running in this environment)"
            );
        }
    }
}

pub fn run_install<W: Write>(
    writer: &mut W,
    options: &CliOptions,
    settings: &AgentSettings,
) -> io::Result<PathBuf> {
    writeln!(writer, "Installing aqvn-agent systemd service...")?;

    let is_user_unit = resolve_is_user_unit(options.user_unit);
    let unit_path = resolve_systemd_unit_path(is_user_unit);
    let binary_path = resolve_binary_path();
    let working_dir = resolve_working_directory(&settings.workspace_path);

    let _ = std::fs::create_dir_all(&working_dir);

    let unit_content = generate_systemd_unit(
        &binary_path,
        &working_dir,
        options.config_path.as_deref(),
        is_user_unit,
    );

    if let Some(parent) = unit_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let is_update = unit_path.exists();
    if is_update {
        writeln!(
            writer,
            "Updating existing systemd service unit at: {}",
            unit_path.display()
        )?;
    }
    std::fs::write(&unit_path, &unit_content)?;

    writeln!(
        writer,
        "Systemd unit file ({}) written to: {}",
        if is_user_unit { "user" } else { "system" },
        unit_path.display()
    )?;

    // Execute daemon-reload and enable
    execute_systemctl_action(writer, is_user_unit, "daemon-reload", &[]);
    execute_systemctl_action(writer, is_user_unit, "enable", &["aqvn-agent.service"]);

    writeln!(
        writer,
        "aqvn-agent systemd service installed and enabled successfully."
    )?;
    writer.flush()?;
    Ok(unit_path)
}

pub fn run_uninstall<W: Write>(writer: &mut W, options: &CliOptions) -> io::Result<()> {
    writeln!(writer, "Uninstalling aqvn-agent systemd service...")?;

    let is_user_unit = resolve_is_user_unit(options.user_unit);
    let unit_path = resolve_systemd_unit_path(is_user_unit);

    // Stop and disable service
    execute_systemctl_action(writer, is_user_unit, "stop", &["aqvn-agent.service"]);
    execute_systemctl_action(writer, is_user_unit, "disable", &["aqvn-agent.service"]);

    if unit_path.exists() {
        std::fs::remove_file(&unit_path)?;
        writeln!(writer, "Removed unit file: {}", unit_path.display())?;
    } else {
        writeln!(
            writer,
            "Unit file not found or already removed: {}",
            unit_path.display()
        )?;
    }

    // Reload daemon after removal
    execute_systemctl_action(writer, is_user_unit, "daemon-reload", &[]);
    execute_systemctl_action(
        writer,
        is_user_unit,
        "reset-failed",
        &["aqvn-agent.service"],
    );

    writeln!(writer, "aqvn-agent systemd service uninstalled cleanly.")?;
    writer.flush()?;
    Ok(())
}

pub async fn run_menu<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    custom_settings_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings_path = custom_settings_path
        .map(PathBuf::from)
        .unwrap_or_else(default_settings_path);
    let mut settings = AgentSettings::load_from_file(&settings_path);

    loop {
        print_menu(writer)?;
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break;
        }

        match line.trim() {
            "1" => {
                let _ = run_start(writer, &settings, None).await;
            }
            "2" => {
                let _ = run_configure(reader, writer, &mut settings, &settings_path);
            }
            "3" => {
                let options = CliOptions::default();
                if let Err(e) = run_install(writer, &options, &settings) {
                    writeln!(writer, "Failed to install service: {e}\n")?;
                    let _ = writer.flush();
                }
            }
            "4" => {
                let options = CliOptions::default();
                if let Err(e) = run_uninstall(writer, &options) {
                    writeln!(writer, "Failed to uninstall service: {e}\n")?;
                    let _ = writer.flush();
                }
            }
            "5" => {
                let _ = run_status(writer, &settings, &settings_path, true).await;
            }
            "0" => {
                writeln!(writer, "Exiting aqvn-agent.")?;
                writer.flush()?;
                break;
            }
            "" => continue,
            other => {
                writeln!(writer, "Invalid option '{}'. Please select 0-5.\n", other)?;
                writer.flush()?;
            }
        }
    }

    Ok(())
}

pub async fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let parsed = match parse_cli_args(&args) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("Error: {err}\n\n{}", format_help());
            std::process::exit(1);
        }
    };

    let settings_path = parsed
        .options
        .config_path
        .clone()
        .unwrap_or_else(default_settings_path);

    match parsed.action {
        CliAction::Version => {
            print!("{}", format_version());
        }
        CliAction::Help => {
            print!("{}", format_help());
        }
        CliAction::Start => {
            let mut settings = AgentSettings::load_from_file(&settings_path);
            parsed.options.apply_to(&mut settings);
            run_start(&mut io::stdout(), &settings, None).await?;
        }
        CliAction::Config => {
            let mut settings = AgentSettings::load_from_file(&settings_path);
            if parsed.options.is_empty() {
                run_configure(
                    &mut io::stdin().lock(),
                    &mut io::stdout(),
                    &mut settings,
                    &settings_path,
                )?;
            } else {
                parsed.options.apply_to(&mut settings);
                settings.save_to_file(&settings_path)?;
                println!(
                    "Configuration updated and saved to {}",
                    settings_path.display()
                );
            }
        }
        CliAction::Install => {
            let mut settings = AgentSettings::load_from_file(&settings_path);
            parsed.options.apply_to(&mut settings);
            run_install(&mut io::stdout(), &parsed.options, &settings)?;
        }
        CliAction::Uninstall => {
            run_uninstall(&mut io::stdout(), &parsed.options)?;
        }
        CliAction::Status => {
            let mut settings = AgentSettings::load_from_file(&settings_path);
            parsed.options.apply_to(&mut settings);
            run_status_ex(
                &mut io::stdout(),
                &settings,
                &settings_path,
                parsed.options.user_unit,
                true,
            )
            .await?;
        }
        CliAction::Menu => {
            run_menu(
                &mut io::stdin().lock(),
                &mut io::stdout(),
                Some(&settings_path),
            )
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_format_version() {
        let v = format_version();
        assert!(v.starts_with("aqvn-agent "));
        assert!(v.contains(VERSION));
    }

    #[test]
    fn test_format_help() {
        let h = format_help();
        assert!(h.contains("USAGE:"));
        assert!(h.contains("aqvn-agent"));
        assert!(h.contains("--help"));
        assert!(h.contains("--version"));
        assert!(h.contains("start"));
    }

    #[test]
    fn test_parse_action() {
        assert_eq!(parse_action(&[]), CliAction::Menu);
        assert_eq!(parse_action(&["-h".to_string()]), CliAction::Help);
        assert_eq!(parse_action(&["--help".to_string()]), CliAction::Help);
        assert_eq!(parse_action(&["-V".to_string()]), CliAction::Version);
        assert_eq!(parse_action(&["--version".to_string()]), CliAction::Version);
        assert_eq!(parse_action(&["start".to_string()]), CliAction::Start);
        assert_eq!(parse_action(&["config".to_string()]), CliAction::Config);
        assert_eq!(parse_action(&["install".to_string()]), CliAction::Install);
        assert_eq!(
            parse_action(&["uninstall".to_string()]),
            CliAction::Uninstall
        );
        assert_eq!(parse_action(&["status".to_string()]), CliAction::Status);
        assert_eq!(parse_action(&["unknown".to_string()]), CliAction::Help);
    }

    #[test]
    fn test_agent_settings_defaults_and_scopes() {
        let settings = AgentSettings::default();
        assert_eq!(settings.server_url, DEFAULT_SERVER_URL);
        assert!(!settings.gateway_id.is_empty());
        assert_eq!(
            settings.active_scopes(),
            vec!["files", "code_execution", "browser", "computer_use"]
        );
    }

    #[test]
    fn test_parse_scopes_into() {
        let mut settings = AgentSettings::default();

        parse_scopes_into("none", &mut settings);
        assert_eq!(settings.active_scopes().len(), 0);

        parse_scopes_into("all", &mut settings);
        assert_eq!(settings.active_scopes().len(), 4);

        parse_scopes_into("files, exec", &mut settings);
        assert!(settings.scope_files);
        assert!(settings.scope_exec);
        assert!(!settings.scope_browser);
        assert!(!settings.scope_desktop);
    }

    #[test]
    fn test_settings_load_save_roundtrip() {
        let temp_dir = env::temp_dir().join("aqvn_test_agent_settings");
        let settings_path = temp_dir.join("settings.json");

        let settings = AgentSettings {
            server_url: "https://custom.server.test".to_string(),
            host_label: "Test-Node-42".to_string(),
            username: Some("testuser".to_string()),
            token: Some("secret-token-123".to_string()),
            ..Default::default()
        };

        settings
            .save_to_file(&settings_path)
            .expect("save settings");
        assert!(settings_path.exists());

        let loaded = AgentSettings::load_from_file(&settings_path);
        assert_eq!(loaded.server_url, "https://custom.server.test");
        assert_eq!(loaded.host_label, "Test-Node-42");
        assert_eq!(loaded.username.as_deref(), Some("testuser"));
        assert_eq!(loaded.token.as_deref(), Some("secret-token-123"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_print_menu_output() {
        let mut out = Vec::new();
        print_menu(&mut out).expect("print menu");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("1. Start"));
        assert!(text.contains("2. Configure"));
        assert!(text.contains("3. Install Service"));
        assert!(text.contains("4. Uninstall Service"));
        assert!(text.contains("5. Status"));
        assert!(text.contains("0. Exit"));
        assert!(text.contains("Enter selection [0-5]:"));
    }

    #[tokio::test]
    async fn test_run_menu_exit_option() {
        let mut input = Cursor::new(b"0\n");
        let mut output = Vec::new();
        let res = run_menu(&mut input, &mut output, None).await;
        assert!(res.is_ok());
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("Exiting aqvn-agent."));
    }

    #[tokio::test]
    async fn test_run_menu_invalid_then_exit() {
        let mut input = Cursor::new(b"99\n0\n");
        let mut output = Vec::new();
        let res = run_menu(&mut input, &mut output, None).await;
        assert!(res.is_ok());
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("Invalid option '99'"));
        assert!(text.contains("Exiting aqvn-agent."));
    }

    #[test]
    fn test_run_configure_interactive() {
        let temp_dir = env::temp_dir().join("aqvn_test_configure_interactive");
        let settings_path = temp_dir.join("settings.json");

        // Input lines:
        // Server URL: https://new-a0.test
        // Username: worker_bob
        // Password/Token: secret_pass_99
        // Host Label: Worker-Bob-Host
        // Scopes: files, code_execution
        // Workspace: /custom/workspace
        let input_data = b"https://new-a0.test\nworker_bob\nsecret_pass_99\nWorker-Bob-Host\nfiles, code_execution\n/custom/workspace\n";
        let mut reader = Cursor::new(input_data);
        let mut writer = Vec::new();
        let mut settings = AgentSettings::default();

        run_configure(&mut reader, &mut writer, &mut settings, &settings_path)
            .expect("configure succeeds");

        assert_eq!(settings.server_url, "https://new-a0.test");
        assert_eq!(settings.username.as_deref(), Some("worker_bob"));
        assert_eq!(settings.token.as_deref(), Some("secret_pass_99"));
        assert_eq!(settings.host_label, "Worker-Bob-Host");
        assert!(settings.scope_files);
        assert!(settings.scope_exec);
        assert!(!settings.scope_browser);
        assert_eq!(settings.workspace_path, "/custom/workspace");

        assert!(settings_path.exists());
        let reloaded = AgentSettings::load_from_file(&settings_path);
        assert_eq!(reloaded, settings);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_run_status_renders_diagnostics() {
        let temp_dir = env::temp_dir().join("aqvn_test_status_diagnostics");
        let settings_path = temp_dir.join("settings.json");
        let settings = AgentSettings::default();

        let mut output = Vec::new();
        run_status(&mut output, &settings, &settings_path, false)
            .await
            .expect("status succeeds");

        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("Status & Diagnostics"));
        assert!(text.contains("Server URL:"));
        assert!(text.contains("Gateway ID:"));
        assert!(text.contains("System Resources:"));
        assert!(text.contains("CPU Cores:"));
    }

    #[tokio::test]
    async fn test_run_start_with_stop_signal() {
        let settings = AgentSettings::default();
        let mut output = Vec::new();
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Send stop signal after 50ms
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = tx.send(());
        });

        let res = run_start(&mut output, &settings, Some(rx)).await;
        assert!(res.is_ok());
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("Starting aqvn-agent in foreground..."));
        assert!(text.contains("Stopping aqvn-agent..."));
        assert!(text.contains("Agent stopped."));
    }

    #[test]
    fn test_parse_cli_args_all_flags_val_agent_cli_003() {
        let args = vec![
            "start".to_string(),
            "--server".to_string(),
            "https://my-a0.domain.test:55088".to_string(),
            "--user".to_string(),
            "agent_user".to_string(),
            "--pass".to_string(),
            "pass_secret_123".to_string(),
            "--token".to_string(),
            "tok_secret_456".to_string(),
            "--label".to_string(),
            "Agent-Node-99".to_string(),
            "--scopes".to_string(),
            "files,code_execution".to_string(),
            "--workspace".to_string(),
            "/srv/workspace".to_string(),
            "--gateway-id".to_string(),
            "gw-custom-123".to_string(),
            "--tls-skip-verify".to_string(),
        ];

        let parsed = parse_cli_args(&args).expect("parse_cli_args succeeds");
        assert_eq!(parsed.action, CliAction::Start);
        assert_eq!(
            parsed.options.server.as_deref(),
            Some("https://my-a0.domain.test:55088")
        );
        assert_eq!(parsed.options.user.as_deref(), Some("agent_user"));
        assert_eq!(parsed.options.pass.as_deref(), Some("pass_secret_123"));
        assert_eq!(parsed.options.token.as_deref(), Some("tok_secret_456"));
        assert_eq!(parsed.options.label.as_deref(), Some("Agent-Node-99"));
        assert_eq!(
            parsed.options.scopes.as_deref(),
            Some("files,code_execution")
        );
        assert_eq!(parsed.options.workspace.as_deref(), Some("/srv/workspace"));
        assert_eq!(parsed.options.gateway_id.as_deref(), Some("gw-custom-123"));
        assert_eq!(parsed.options.tls_skip_verify, Some(true));

        let mut settings = AgentSettings::default();
        parsed.options.apply_to(&mut settings);

        assert_eq!(settings.server_url, "https://my-a0.domain.test:55088");
        assert_eq!(settings.username.as_deref(), Some("agent_user"));
        assert_eq!(settings.password.as_deref(), Some("pass_secret_123"));
        assert_eq!(settings.token.as_deref(), Some("tok_secret_456"));
        assert_eq!(settings.host_label, "Agent-Node-99");
        assert_eq!(settings.active_scopes(), vec!["files", "code_execution"]);
        assert_eq!(settings.workspace_path, "/srv/workspace");
        assert_eq!(settings.gateway_id, "gw-custom-123");
        assert!(settings.tls_skip_verify);
    }

    #[test]
    fn test_cli_flags_override_file_settings_val_agent_cli_003() {
        let temp_dir = env::temp_dir().join("aqvn_test_cli_override");
        let settings_path = temp_dir.join("settings.json");

        let initial_settings = AgentSettings {
            server_url: "https://old-server.test".to_string(),
            host_label: "Old-Label".to_string(),
            workspace_path: "/old/workspace".to_string(),
            scope_files: false,
            scope_exec: false,
            scope_browser: false,
            scope_desktop: false,
            ..Default::default()
        };
        initial_settings
            .save_to_file(&settings_path)
            .expect("save initial");

        let args = vec![
            "start".to_string(),
            "--config".to_string(),
            settings_path.to_string_lossy().to_string(),
            "--server".to_string(),
            "https://new-override.test".to_string(),
            "--label".to_string(),
            "Overridden-Label".to_string(),
            "--scopes".to_string(),
            "files,browser".to_string(),
        ];

        let parsed = parse_cli_args(&args).expect("parse_cli_args succeeds");
        assert_eq!(parsed.action, CliAction::Start);

        let mut loaded = AgentSettings::load_from_file(&settings_path);
        parsed.options.apply_to(&mut loaded);

        // Overridden values:
        assert_eq!(loaded.server_url, "https://new-override.test");
        assert_eq!(loaded.host_label, "Overridden-Label");
        assert_eq!(loaded.active_scopes(), vec!["files", "browser"]);

        // Preserved unmodified values:
        assert_eq!(loaded.workspace_path, "/old/workspace");
        assert_eq!(loaded.gateway_id, initial_settings.gateway_id);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_agent_host_pool_registration_settings_val_host_pool_001() {
        let args = vec![
            "start".to_string(),
            "--server".to_string(),
            "https://pool.agentzero.internal:55088".to_string(),
            "--gateway-id".to_string(),
            "deterministic-node-uuid-42".to_string(),
            "--label".to_string(),
            "Production-VPS-Singapore".to_string(),
            "--scopes".to_string(),
            "files,code_execution".to_string(),
            "--ping-interval".to_string(),
            "3".to_string(),
        ];

        let parsed = parse_cli_args(&args).expect("parse_cli_args succeeds");
        assert_eq!(parsed.action, CliAction::Start);
        assert_eq!(parsed.options.ping_interval, Some(3));
        assert_eq!(
            parsed.options.gateway_id.as_deref(),
            Some("deterministic-node-uuid-42")
        );
        assert_eq!(
            parsed.options.label.as_deref(),
            Some("Production-VPS-Singapore")
        );

        let mut settings = AgentSettings::default();
        parsed.options.apply_to(&mut settings);

        assert_eq!(settings.gateway_id, "deterministic-node-uuid-42");
        assert_eq!(settings.host_label, "Production-VPS-Singapore");
        assert_eq!(settings.active_scopes(), vec!["files", "code_execution"]);
        assert_eq!(settings.ping_interval_secs, 3);

        #[cfg(not(target_os = "android"))]
        {
            let gw_config = settings.to_gateway_config();
            assert_eq!(gw_config.gateway_id, "deterministic-node-uuid-42");
            assert_eq!(gw_config.host_label, "Production-VPS-Singapore");
            assert_eq!(gw_config.active_scopes(), vec!["files", "code_execution"]);
            assert_eq!(gw_config.ping_interval_secs, 3);
            assert!(gw_config.scope_files);
            assert!(gw_config.scope_exec);
            assert!(!gw_config.scope_browser);
            assert!(!gw_config.scope_desktop);

            // Empty fallbacks
            let empty_settings = AgentSettings {
                gateway_id: "".to_string(),
                host_label: "".to_string(),
                ..Default::default()
            };
            let fallback_cfg = empty_settings.to_gateway_config();
            assert_eq!(fallback_cfg.gateway_id, default_gateway_id());
            assert_eq!(fallback_cfg.host_label, default_host_label());
        }
    }

    #[test]
    fn test_agent_host_pool_remote_control_settings_val_host_pool_002() {
        let args = vec![
            "start".to_string(),
            "--server".to_string(),
            "https://pool.agentzero.internal:55088".to_string(),
            "--workspace".to_string(),
            "/var/aqvn/workspace".to_string(),
            "--scopes".to_string(),
            "files,code_execution".to_string(),
        ];

        let parsed = parse_cli_args(&args).expect("parse_cli_args succeeds");
        assert_eq!(parsed.action, CliAction::Start);
        assert_eq!(
            parsed.options.workspace.as_deref(),
            Some("/var/aqvn/workspace")
        );
        assert_eq!(
            parsed.options.scopes.as_deref(),
            Some("files,code_execution")
        );

        let mut settings = AgentSettings::default();
        parsed.options.apply_to(&mut settings);

        assert_eq!(settings.workspace_path, "/var/aqvn/workspace");
        assert_eq!(settings.active_scopes(), vec!["files", "code_execution"]);

        #[cfg(not(target_os = "android"))]
        {
            let gw_config = settings.to_gateway_config();
            assert_eq!(gw_config.workspace_path, "/var/aqvn/workspace");
            assert!(gw_config.scope_files);
            assert!(gw_config.scope_exec);
            assert!(!gw_config.scope_browser);
            assert!(!gw_config.scope_desktop);
        }
    }

    #[test]
    fn test_parse_cli_args_invalid_arguments_val_agent_cli_003() {
        // Unknown option
        let res = parse_cli_args(&["start".to_string(), "--invalid-option".to_string()]);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("Unknown option '--invalid-option'"));

        // Missing value for option
        let res_missing = parse_cli_args(&["start".to_string(), "--server".to_string()]);
        assert!(res_missing.is_err());
        assert!(res_missing
            .unwrap_err()
            .contains("Option '--server' requires a value"));

        // Unknown command
        let res_cmd = parse_cli_args(&["unknown-cmd".to_string()]);
        assert!(res_cmd.is_err());
        assert!(res_cmd
            .unwrap_err()
            .contains("Unknown command or unexpected argument 'unknown-cmd'"));
    }

    #[test]
    fn test_parse_cli_args_inline_equals_syntax() {
        let args = vec![
            "start".to_string(),
            "--server=https://inline.test".to_string(),
            "--label=Inline-Node".to_string(),
            "--workspace=/inline/ws".to_string(),
            "--user=bob".to_string(),
            "--token=inline_tok".to_string(),
            "--tls-skip-verify=true".to_string(),
        ];

        let parsed = parse_cli_args(&args).expect("parse inline equals");
        assert_eq!(
            parsed.options.server.as_deref(),
            Some("https://inline.test")
        );
        assert_eq!(parsed.options.label.as_deref(), Some("Inline-Node"));
        assert_eq!(parsed.options.workspace.as_deref(), Some("/inline/ws"));
        assert_eq!(parsed.options.user.as_deref(), Some("bob"));
        assert_eq!(parsed.options.token.as_deref(), Some("inline_tok"));
        assert_eq!(parsed.options.tls_skip_verify, Some(true));
    }

    #[test]
    fn test_parse_cli_args_rejects_zero_ping_interval() {
        let space = vec![
            "start".to_string(),
            "--ping-interval".to_string(),
            "0".to_string(),
        ];
        let err = parse_cli_args(&space).expect_err("zero ping interval must be rejected");
        assert!(
            err.contains("must be >= 1"),
            "unexpected error message: {err}"
        );

        let equals = vec!["start".to_string(), "--ping-interval=0".to_string()];
        let err = parse_cli_args(&equals).expect_err("zero inline ping interval must be rejected");
        assert!(
            err.contains("must be >= 1"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn test_generate_systemd_unit_val_agent_cli_004() {
        let bin_path = Path::new("/usr/local/bin/aqvn-agent");
        let ws_path = Path::new("/home/alice/Workspace");
        let unit_content = generate_systemd_unit(bin_path, ws_path, None, true);

        assert!(unit_content.contains("[Unit]"));
        assert!(
            unit_content.contains("Description=AQVN Standalone Headless Agent & Multi-Host Bridge")
        );
        assert!(unit_content.contains("After=network.target"));
        assert!(unit_content.contains("[Service]"));
        assert!(unit_content.contains("Type=simple"));
        assert!(unit_content.contains("ExecStart=/usr/local/bin/aqvn-agent start"));
        assert!(unit_content.contains("WorkingDirectory=/home/alice/Workspace"));
        assert!(unit_content.contains("Restart=always"));
        assert!(unit_content.contains("RestartSec=5"));
        assert!(unit_content.contains("KillMode=process"));
        assert!(unit_content.contains("[Install]"));
        assert!(unit_content.contains("WantedBy=default.target"));
    }

    #[test]
    fn test_generate_systemd_unit_system_and_config_val_agent_cli_004() {
        let bin_path = Path::new("/opt/aqvn/bin/aqvn-agent");
        let ws_path = Path::new("/srv/aqvn-data");
        let config_path = Path::new("/etc/aqvn/agent-settings.json");
        let unit_content = generate_systemd_unit(bin_path, ws_path, Some(config_path), false);

        assert!(unit_content.contains(
            "ExecStart=/opt/aqvn/bin/aqvn-agent start --config /etc/aqvn/agent-settings.json"
        ));
        assert!(unit_content.contains("WorkingDirectory=/srv/aqvn-data"));
        assert!(unit_content.contains("WantedBy=multi-user.target"));
        assert!(unit_content.contains("Restart=always"));
    }

    #[test]
    fn test_resolve_working_directory_val_agent_cli_004() {
        let raw_tilde = "~/MyWorkspace";
        let resolved = resolve_working_directory(raw_tilde);
        assert!(resolved.is_absolute());
        if let Ok(home) = env::var("HOME") {
            assert_eq!(resolved, PathBuf::from(home).join("MyWorkspace"));
        }

        let abs_path = "/tmp/fixed-workspace";
        assert_eq!(resolve_working_directory(abs_path), PathBuf::from(abs_path));
    }

    static TEST_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_resolve_systemd_unit_path_val_agent_cli_004() {
        let _lock = TEST_ENV_MUTEX.lock().unwrap();
        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        let user_path = resolve_systemd_unit_path(true);
        assert!(user_path.ends_with("systemd/user/aqvn-agent.service"));

        let sys_path = resolve_systemd_unit_path(false);
        assert_eq!(
            sys_path,
            PathBuf::from("/etc/systemd/system/aqvn-agent.service")
        );
    }

    #[test]
    fn test_parse_cli_args_install_uninstall_flags_val_agent_cli_004() {
        let p1 = parse_cli_args(&["install".to_string()]).expect("install defaults");
        assert_eq!(p1.action, CliAction::Install);
        assert_eq!(p1.options.user_unit, None);

        let p2 =
            parse_cli_args(&["install".to_string(), "--user".to_string()]).expect("install --user");
        assert_eq!(p2.action, CliAction::Install);
        assert_eq!(p2.options.user_unit, Some(true));

        let p3 = parse_cli_args(&["install".to_string(), "--system".to_string()])
            .expect("install --system");
        assert_eq!(p3.action, CliAction::Install);
        assert_eq!(p3.options.user_unit, Some(false));

        let p4 = parse_cli_args(&["uninstall".to_string()]).expect("uninstall defaults");
        assert_eq!(p4.action, CliAction::Uninstall);
        assert_eq!(p4.options.user_unit, None);

        let p5 = parse_cli_args(&["uninstall".to_string(), "--user".to_string()])
            .expect("uninstall --user");
        assert_eq!(p5.action, CliAction::Uninstall);
        assert_eq!(p5.options.user_unit, Some(true));

        let p6 = parse_cli_args(&["uninstall".to_string(), "--system".to_string()])
            .expect("uninstall --system");
        assert_eq!(p6.action, CliAction::Uninstall);
        assert_eq!(p6.options.user_unit, Some(false));

        let p7 =
            parse_cli_args(&["--user".to_string(), "install".to_string()]).expect("--user install");
        assert_eq!(p7.action, CliAction::Install);
        assert_eq!(p7.options.user_unit, Some(true));
    }

    #[test]
    fn test_run_install_and_uninstall_lifecycle_val_agent_cli_004() {
        let _lock = TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir =
            env::temp_dir().join(format!("test_systemd_lifecycle_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let unit_path = temp_dir.join("aqvn-agent.service");
        env::set_var("AQVN_SYSTEMD_UNIT_PATH", &unit_path);
        env::set_var("AQVN_AGENT_BIN_PATH", "/usr/bin/aqvn-agent");

        let mut output = Vec::new();
        let options = CliOptions {
            workspace: Some(temp_dir.to_string_lossy().to_string()),
            user_unit: Some(true),
            ..Default::default()
        };
        let settings = AgentSettings {
            workspace_path: temp_dir.to_string_lossy().to_string(),
            ..Default::default()
        };

        // 1. Install
        let installed_path =
            run_install(&mut output, &options, &settings).expect("install service");
        assert_eq!(installed_path, unit_path);
        assert!(unit_path.exists(), "unit file must exist after install");

        let content = std::fs::read_to_string(&unit_path).expect("read unit file");
        assert!(content.contains("Restart=always"));
        assert!(content.contains("ExecStart=/usr/bin/aqvn-agent start"));
        assert!(content.contains(&format!("WorkingDirectory={}", temp_dir.display())));
        assert!(content.contains("WantedBy=default.target"));

        let install_log = String::from_utf8_lossy(&output);
        assert!(install_log.contains("Installing aqvn-agent systemd service..."));
        assert!(
            install_log.contains("aqvn-agent systemd service installed and enabled successfully.")
        );

        // 2. Uninstall
        output.clear();
        run_uninstall(&mut output, &options).expect("uninstall service");
        assert!(
            !unit_path.exists(),
            "unit file must be deleted after uninstall"
        );

        let uninstall_log = String::from_utf8_lossy(&output);
        assert!(uninstall_log.contains("Uninstalling aqvn-agent systemd service..."));
        assert!(uninstall_log.contains("aqvn-agent systemd service uninstalled cleanly."));

        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        env::remove_var("AQVN_AGENT_BIN_PATH");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_query_service_status_installed_and_not_installed_val_pkg_003() {
        let _lock = TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir = env::temp_dir().join(format!("test_status_query_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let unit_path = temp_dir.join("aqvn-agent.service");
        env::set_var("AQVN_SYSTEMD_UNIT_PATH", &unit_path);

        // Not installed yet
        let status_pre = query_service_status(Some(true));
        assert!(status_pre.is_user_unit);
        assert_eq!(status_pre.unit_path, unit_path);
        assert!(!status_pre.installed);

        // Simulate installation by creating the unit file
        std::fs::write(&unit_path, "[Unit]\nDescription=Test\n").expect("write dummy unit");
        let status_post = query_service_status(Some(true));
        assert!(status_post.installed);

        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_run_install_idempotent_update_val_pkg_003() {
        let _lock = TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir =
            env::temp_dir().join(format!("test_idempotent_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let unit_path = temp_dir.join("aqvn-agent.service");
        env::set_var("AQVN_SYSTEMD_UNIT_PATH", &unit_path);
        env::set_var("AQVN_AGENT_BIN_PATH", "/usr/bin/aqvn-agent");

        let mut output = Vec::new();
        let options = CliOptions {
            workspace: Some(temp_dir.to_string_lossy().to_string()),
            user_unit: Some(true),
            ..Default::default()
        };
        let settings = AgentSettings {
            workspace_path: "/workspace/v1".to_string(),
            ..Default::default()
        };

        // 1. Initial install
        let installed_path =
            run_install(&mut output, &options, &settings).expect("first install succeeds");
        assert_eq!(installed_path, unit_path);
        assert!(unit_path.exists());
        let content_v1 = std::fs::read_to_string(&unit_path).unwrap();
        assert!(content_v1.contains("WorkingDirectory=/workspace/v1"));

        // Verify status reflects installed
        let s_installed = query_service_status(Some(true));
        assert!(s_installed.installed);

        // 2. Re-install / update with new configuration (idempotent)
        output.clear();
        let settings_v2 = AgentSettings {
            workspace_path: "/workspace/v2_updated".to_string(),
            ..Default::default()
        };
        let reinstalled_path =
            run_install(&mut output, &options, &settings_v2).expect("second install succeeds");
        assert_eq!(reinstalled_path, unit_path);
        let log_v2 = String::from_utf8_lossy(&output);
        assert!(log_v2.contains("Updating existing systemd service unit at:"));
        assert!(log_v2.contains("installed and enabled successfully."));

        let content_v2 = std::fs::read_to_string(&unit_path).unwrap();
        assert!(content_v2.contains("WorkingDirectory=/workspace/v2_updated"));

        // Clean up
        output.clear();
        run_uninstall(&mut output, &options).expect("uninstall succeeds");
        assert!(!unit_path.exists());

        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        env::remove_var("AQVN_AGENT_BIN_PATH");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_run_uninstall_idempotency_val_pkg_003() {
        let _lock = TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir =
            env::temp_dir().join(format!("test_idempotent_uninstall_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let unit_path = temp_dir.join("aqvn-agent.service");
        env::set_var("AQVN_SYSTEMD_UNIT_PATH", &unit_path);
        env::set_var("AQVN_AGENT_BIN_PATH", "/usr/bin/aqvn-agent");

        let mut output = Vec::new();
        let options = CliOptions {
            user_unit: Some(true),
            ..Default::default()
        };
        let settings = AgentSettings::default();

        // Install first
        run_install(&mut output, &options, &settings).expect("install service");
        assert!(unit_path.exists());

        // 1. First uninstall removes unit file
        output.clear();
        run_uninstall(&mut output, &options).expect("first uninstall succeeds");
        assert!(!unit_path.exists());
        let log_uninst1 = String::from_utf8_lossy(&output);
        assert!(log_uninst1.contains("Removed unit file:"));
        assert!(log_uninst1.contains("uninstalled cleanly."));

        // 2. Second uninstall on non-existent unit file handles absence idempotently
        output.clear();
        run_uninstall(&mut output, &options).expect("second uninstall succeeds idempotently");
        let log_uninst2 = String::from_utf8_lossy(&output);
        assert!(log_uninst2.contains("Unit file not found or already removed:"));
        assert!(log_uninst2.contains("uninstalled cleanly."));

        let s_uninstalled = query_service_status(Some(true));
        assert!(!s_uninstalled.installed);

        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        env::remove_var("AQVN_AGENT_BIN_PATH");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_run_install_system_unit_idempotent_lifecycle_val_pkg_003() {
        let _lock = TEST_ENV_MUTEX.lock().unwrap();
        let temp_dir =
            env::temp_dir().join(format!("test_sys_unit_lifecycle_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let unit_path = temp_dir.join("aqvn-agent-sys.service");
        env::set_var("AQVN_SYSTEMD_UNIT_PATH", &unit_path);
        env::set_var("AQVN_AGENT_BIN_PATH", "/usr/bin/aqvn-agent");

        let mut output = Vec::new();
        let options = CliOptions {
            user_unit: Some(false),
            ..Default::default()
        };
        let settings = AgentSettings {
            workspace_path: "/var/aqvn-system-ws".to_string(),
            ..Default::default()
        };

        // 1. Install system unit
        let installed = run_install(&mut output, &options, &settings).expect("install system unit");
        assert_eq!(installed, unit_path);
        assert!(unit_path.exists());
        let content = std::fs::read_to_string(&unit_path).unwrap();
        assert!(content.contains("WantedBy=multi-user.target"));
        assert!(content.contains("WorkingDirectory=/var/aqvn-system-ws"));

        let status = query_service_status(Some(false));
        assert!(!status.is_user_unit);
        assert!(status.installed);

        // 2. Re-install system unit idempotently
        output.clear();
        run_install(&mut output, &options, &settings).expect("reinstall system unit");
        let log_re = String::from_utf8_lossy(&output);
        assert!(log_re.contains("Updating existing systemd service unit at:"));

        // 3. Uninstall system unit
        output.clear();
        run_uninstall(&mut output, &options).expect("uninstall system unit");
        assert!(!unit_path.exists());

        // 4. Uninstall system unit second time (idempotent)
        output.clear();
        run_uninstall(&mut output, &options).expect("second uninstall system unit");
        let status_after = query_service_status(Some(false));
        assert!(!status_after.installed);

        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        env::remove_var("AQVN_AGENT_BIN_PATH");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_run_status_renders_service_unit_diagnostic_val_pkg_003() {
        let temp_dir =
            env::temp_dir().join(format!("test_status_service_diag_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let unit_path = temp_dir.join("aqvn-agent.service");
        let settings_path = temp_dir.join("settings.json");
        let settings = AgentSettings::default();

        let lock = TEST_ENV_MUTEX.lock().unwrap();
        env::set_var("AQVN_SYSTEMD_UNIT_PATH", &unit_path);
        drop(lock);

        // When not installed
        let mut output = Vec::new();
        run_status_ex(&mut output, &settings, &settings_path, Some(true), false)
            .await
            .expect("status succeeds");
        let text_pre = String::from_utf8_lossy(&output);
        assert!(text_pre.contains("Service Unit:"));
        assert!(text_pre.contains("user, Not installed"));

        // When installed
        std::fs::write(&unit_path, "[Unit]\nDescription=Test\n").unwrap();
        output.clear();
        run_status_ex(&mut output, &settings, &settings_path, Some(true), false)
            .await
            .expect("status succeeds");
        let text_post = String::from_utf8_lossy(&output);
        assert!(text_post.contains("Service Unit:"));
        assert!(text_post.contains("user, Installed"));

        let lock = TEST_ENV_MUTEX.lock().unwrap();
        env::remove_var("AQVN_SYSTEMD_UNIT_PATH");
        drop(lock);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_cli_args_status_service_flags_val_pkg_003() {
        let p1 =
            parse_cli_args(&["status".to_string(), "--user".to_string()]).expect("status --user");
        assert_eq!(p1.action, CliAction::Status);
        assert_eq!(p1.options.user_unit, Some(true));

        let p2 = parse_cli_args(&["status".to_string(), "--system".to_string()])
            .expect("status --system");
        assert_eq!(p2.action, CliAction::Status);
        assert_eq!(p2.options.user_unit, Some(false));

        let p3 =
            parse_cli_args(&["--user".to_string(), "status".to_string()]).expect("--user status");
        assert_eq!(p3.action, CliAction::Status);
        assert_eq!(p3.options.user_unit, Some(true));
    }
}

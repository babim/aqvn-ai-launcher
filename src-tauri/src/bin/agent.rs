//! AQVN Standalone Headless Agent & Multi-Host Bridge (`aqvn-agent`).
//!
//! Executes as a standalone CLI daemon without GUI or display server requirements.

use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(not(target_os = "android"))]
use aqvn_workspace_lib::connector::GatewayConfig;

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
    -h, --help       Print help information
    -V, --version    Print version information
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
    aqvn_workspace_lib::config::default_gateway_id()
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
            host_label: self.host_label.clone(),
            session_cookie: self.token.clone().or_else(|| self.password.clone()),
            scope_files: self.scope_files,
            scope_exec: self.scope_exec,
            scope_browser: self.scope_browser,
            scope_desktop: self.scope_desktop,
            ping_interval_secs: aqvn_workspace_lib::connector::client::DEFAULT_PING_INTERVAL_SECS,
            reconnect_delay_ms: aqvn_workspace_lib::connector::client::DEFAULT_RECONNECT_DELAY_MS,
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
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::other(e.to_string()))?;
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

#[derive(Debug, PartialEq, Eq)]
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

pub fn parse_action(args: &[String]) -> CliAction {
    if args.is_empty() {
        return CliAction::Menu;
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => CliAction::Help,
        "-V" | "--version" | "version" => CliAction::Version,
        "start" => CliAction::Start,
        "config" => CliAction::Config,
        "install" => CliAction::Install,
        "uninstall" => CliAction::Uninstall,
        "status" => CliAction::Status,
        _ => CliAction::Help,
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
        let metrics = aqvn_workspace_lib::connector::handlers::telemetry::collect_system_metrics();
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
        let client = aqvn_workspace_lib::connector::GatewayClient::new(gateway_config);
        let (handle, join_handle) = client.start();

        if let Some(stop_rx) = stop_signal {
            let _ = stop_rx.await;
        } else {
            let _ = tokio::signal::ctrl_c().await;
        }

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

pub fn run_install<W: Write>(writer: &mut W) -> io::Result<()> {
    writeln!(writer, "Installing aqvn-agent systemd service...")?;
    writer.flush()?;
    Ok(())
}

pub fn run_uninstall<W: Write>(writer: &mut W) -> io::Result<()> {
    writeln!(writer, "Uninstalling aqvn-agent systemd service...")?;
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
                let _ = run_install(writer);
            }
            "4" => {
                let _ = run_uninstall(writer);
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = parse_action(&args);
    let settings_path = default_settings_path();

    match action {
        CliAction::Version => {
            print!("{}", format_version());
        }
        CliAction::Help => {
            print!("{}", format_help());
        }
        CliAction::Start => {
            let settings = AgentSettings::load_from_file(&settings_path);
            run_start(&mut io::stdout(), &settings, None).await?;
        }
        CliAction::Config => {
            let mut settings = AgentSettings::load_from_file(&settings_path);
            run_configure(
                &mut io::stdin().lock(),
                &mut io::stdout(),
                &mut settings,
                &settings_path,
            )?;
        }
        CliAction::Install => {
            run_install(&mut io::stdout())?;
        }
        CliAction::Uninstall => {
            run_uninstall(&mut io::stdout())?;
        }
        CliAction::Status => {
            let settings = AgentSettings::load_from_file(&settings_path);
            run_status(&mut io::stdout(), &settings, &settings_path, true).await?;
        }
        CliAction::Menu => {
            run_menu(&mut io::stdin().lock(), &mut io::stdout(), None).await?;
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
}

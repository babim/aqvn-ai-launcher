//! Integration tests for standalone aqvn-agent CLI binary (VAL-AGENT-CLI-001).

use std::process::Command;

fn agent_bin() -> std::path::PathBuf {
    // Cargo sets CARGO_BIN_EXE_<name> during integration tests
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_aqvn-agent") {
        return std::path::PathBuf::from(path);
    }
    // Fallback to target debug dir
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("aqvn-agent");
    path
}

#[test]
fn test_agent_cli_version_flag() {
    let bin = agent_bin();
    let output = Command::new(&bin)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute {}: {e}", bin.display()));

    assert!(output.status.success(), "--version must exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("aqvn-agent") && (stdout.contains("0.1.8") || stdout.contains("0.1.9")),
        "expected version string with aqvn-agent, got: {stdout}"
    );

    // Also verify -V
    let output_short = Command::new(&bin)
        .arg("-V")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute {}: {e}", bin.display()));
    assert!(output_short.status.success(), "-V must exit with code 0");
}

#[test]
fn test_agent_cli_help_flag() {
    let bin = agent_bin();
    let output = Command::new(&bin)
        .arg("--help")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute {}: {e}", bin.display()));

    assert!(output.status.success(), "--help must exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("aqvn-agent") && stdout.contains("USAGE:"),
        "expected help text with USAGE and aqvn-agent, got: {stdout}"
    );
    assert!(stdout.contains("--help"), "help must document --help");
    assert!(stdout.contains("--version"), "help must document --version");

    // Also verify -h
    let output_short = Command::new(&bin)
        .arg("-h")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute {}: {e}", bin.display()));
    assert!(output_short.status.success(), "-h must exit with code 0");
}

#[test]
fn test_agent_cli_headless_execution() {
    let bin = agent_bin();
    // Strip DISPLAY and WAYLAND_DISPLAY to guarantee headless execution
    let output = Command::new(&bin)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .arg("--help")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute in headless mode {}: {e}", bin.display()));

    assert!(
        output.status.success(),
        "aqvn-agent must execute in headless terminal without DISPLAY or WAYLAND_DISPLAY"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_agent_cli_no_gtk_or_webkit_runtime_deps() {
    let bin = agent_bin();
    // Run ldd on Linux to ensure no dynamic linkage to GTK or WebKit
    let output = Command::new("ldd")
        .arg(&bin)
        .output()
        .expect("ldd command must execute");

    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    assert!(
        !stdout.contains("libgtk") && !stdout.contains("libwebkit"),
        "aqvn-agent binary must not link libgtk or libwebkit: {stdout}"
    );
}

#[test]
fn test_agent_cli_interactive_menu_renders_options_val_agent_cli_002() {
    let bin = agent_bin();
    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"0\n");
    }

    let output = child.wait_with_output().expect("wait for child exit");
    assert!(output.status.success(), "exit with code 0 on option 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("1") && stdout.contains("Start"),
        "menu must show option 1 (Start), got:\n{stdout}"
    );
    assert!(
        stdout.contains("2") && stdout.contains("Configure"),
        "menu must show option 2 (Configure), got:\n{stdout}"
    );
    assert!(
        stdout.contains("3") && stdout.contains("Install Service"),
        "menu must show option 3 (Install Service), got:\n{stdout}"
    );
    assert!(
        stdout.contains("4") && stdout.contains("Uninstall Service"),
        "menu must show option 4 (Uninstall Service), got:\n{stdout}"
    );
    assert!(
        stdout.contains("5") && stdout.contains("Status"),
        "menu must show option 5 (Status), got:\n{stdout}"
    );
    assert!(
        stdout.contains("0") && stdout.contains("Exit"),
        "menu must show option 0 (Exit), got:\n{stdout}"
    );
}

#[test]
fn test_agent_cli_menu_option_5_status_val_agent_cli_002() {
    let bin = agent_bin();
    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"5\n0\n");
    }

    let output = child.wait_with_output().expect("wait for child exit");
    assert!(output.status.success(), "exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Status & Diagnostics"),
        "must display status header"
    );
    assert!(stdout.contains("Server URL:"), "must display server URL");
    assert!(stdout.contains("Gateway ID:"), "must display gateway id");
    assert!(
        stdout.contains("System Resources:"),
        "must display system resources"
    );
    assert!(stdout.contains("CPU Cores:"), "must display CPU metrics");
}

#[test]
fn test_agent_cli_menu_option_2_configure_val_agent_cli_002() {
    let bin = agent_bin();
    let temp_dir = std::env::temp_dir().join("test_agent_cli_opt2");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let mut child = Command::new(&bin)
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", &temp_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        // Option 2 (Configure), then prompts:
        // Server URL
        // Username
        // Password/Token
        // Host Label
        // Scopes
        // Workspace
        // Then Option 0 (Exit)
        let input = "2\nhttps://custom.agent.test\nworker_alice\ntoken_xyz_999\nAlice-Node\nfiles, browser\n/tmp/alice-work\n0\n";
        let _ = stdin.write_all(input.as_bytes());
    }

    let output = child.wait_with_output().expect("wait for child exit");
    assert!(output.status.success(), "exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Configure Agent Settings"),
        "must display configure header"
    );
    assert!(
        stdout.contains("Configuration persisted to"),
        "must confirm persistence"
    );

    let saved_settings_path = temp_dir
        .join(".config")
        .join("net.aqvn.agent")
        .join("settings.json");
    assert!(
        saved_settings_path.exists(),
        "settings.json must exist at {}",
        saved_settings_path.display()
    );

    let content = std::fs::read_to_string(&saved_settings_path).expect("read settings.json");
    assert!(content.contains("https://custom.agent.test"));
    assert!(content.contains("worker_alice"));
    assert!(content.contains("token_xyz_999"));
    assert!(content.contains("Alice-Node"));
    assert!(content.contains("/tmp/alice-work"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_cli_menu_option_3_and_4_service_val_agent_cli_002() {
    let bin = agent_bin();
    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(b"3\n4\n0\n");
    }

    let output = child.wait_with_output().expect("wait for child exit");
    assert!(output.status.success(), "exit with code 0");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("Installing aqvn-agent systemd service"));
    assert!(stdout.contains("Uninstalling aqvn-agent systemd service"));
}

#[test]
fn test_agent_cli_start_with_flags_val_agent_cli_003() {
    let bin = agent_bin();
    let mut child = Command::new(&bin)
        .arg("start")
        .arg("--server")
        .arg("https://cli-test.agent.local:55088")
        .arg("--user")
        .arg("test_cli_user")
        .arg("--pass")
        .arg("test_cli_pass")
        .arg("--token")
        .arg("test_cli_token")
        .arg("--label")
        .arg("CLI-Test-Host")
        .arg("--scopes")
        .arg("files,code_execution")
        .arg("--workspace")
        .arg("/tmp/cli-test-ws")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    // Read stdout lines until agent start banner is rendered
    let stdout = child.stdout.take().expect("capture child stdout");
    use std::io::BufRead;
    let mut reader = std::io::BufReader::new(stdout);
    let mut banner_lines = Vec::new();

    for _ in 0..10 {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim().to_string();
        banner_lines.push(trimmed.clone());
        if trimmed.contains("Press Ctrl+C to stop.") {
            break;
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    let full_text = banner_lines.join("\n");
    assert!(
        full_text.contains("Starting aqvn-agent in foreground..."),
        "expected start banner, got:\n{full_text}"
    );
    assert!(
        full_text.contains("Server URL:     https://cli-test.agent.local:55088"),
        "expected server url, got:\n{full_text}"
    );
    assert!(
        full_text.contains("Host Label:     CLI-Test-Host"),
        "expected host label, got:\n{full_text}"
    );
    assert!(
        full_text.contains(r#"Active Scopes:  ["files", "code_execution"]"#),
        "expected active scopes, got:\n{full_text}"
    );
    assert!(
        full_text.contains("Workspace:      /tmp/cli-test-ws"),
        "expected workspace path, got:\n{full_text}"
    );
    assert!(
        !full_text.contains("Enter selection [0-5]:"),
        "must not open interactive menu"
    );
}

#[test]
fn test_agent_cli_flags_override_settings_json_val_agent_cli_003() {
    let bin = agent_bin();
    let temp_dir = std::env::temp_dir().join("test_agent_cli_override_json");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let settings_path = temp_dir.join("settings.json");

    let initial_json = r#"{
  "server_url": "https://base.agent.local",
  "gateway_id": "gw-base-42",
  "host_label": "Base-Host-Node",
  "workspace_path": "/base/workspace/path",
  "scope_files": true,
  "scope_exec": false,
  "scope_browser": false,
  "scope_desktop": false
}"#;
    std::fs::write(&settings_path, initial_json).expect("write initial settings");

    let mut child = Command::new(&bin)
        .arg("start")
        .arg("--config")
        .arg(&settings_path)
        .arg("--server")
        .arg("https://overridden.agent.local:9999")
        .arg("--label")
        .arg("Overridden-Host-Node")
        .arg("--scopes")
        .arg("files,browser")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", bin.display()));

    let stdout = child.stdout.take().expect("capture child stdout");
    use std::io::BufRead;
    let mut reader = std::io::BufReader::new(stdout);
    let mut banner_lines = Vec::new();

    for _ in 0..10 {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let trimmed = line.trim().to_string();
        banner_lines.push(trimmed.clone());
        if trimmed.contains("Press Ctrl+C to stop.") {
            break;
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(&temp_dir);

    let full_text = banner_lines.join("\n");
    // Overridden fields
    assert!(
        full_text.contains("Server URL:     https://overridden.agent.local:9999"),
        "expected overridden server url, got:\n{full_text}"
    );
    assert!(
        full_text.contains("Host Label:     Overridden-Host-Node"),
        "expected overridden host label, got:\n{full_text}"
    );
    assert!(
        full_text.contains(r#"Active Scopes:  ["files", "browser"]"#),
        "expected overridden scopes, got:\n{full_text}"
    );
    // Preserved fields from settings.json
    assert!(
        full_text.contains("Gateway ID:     gw-base-42"),
        "expected preserved gateway_id from json, got:\n{full_text}"
    );
    assert!(
        full_text.contains("Workspace:      /base/workspace/path"),
        "expected preserved workspace from json, got:\n{full_text}"
    );
}

#[test]
fn test_agent_cli_invalid_arguments_exit_nonzero_val_agent_cli_003() {
    let bin = agent_bin();

    // 1. Unknown option flag
    let output1 = Command::new(&bin)
        .arg("--nonexistent-flag")
        .output()
        .expect("execute with unknown flag");
    assert!(
        !output1.status.success(),
        "--nonexistent-flag must exit non-zero"
    );
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    assert!(
        stderr1.contains("Unknown option '--nonexistent-flag'"),
        "expected unknown option error, got:\n{stderr1}"
    );
    assert!(
        stderr1.contains("USAGE:"),
        "expected usage message on error, got:\n{stderr1}"
    );

    // 2. Missing option value
    let output2 = Command::new(&bin)
        .arg("start")
        .arg("--server")
        .output()
        .expect("execute with missing value");
    assert!(
        !output2.status.success(),
        "--server without value must exit non-zero"
    );
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr2.contains("Option '--server' requires a value"),
        "expected missing value error, got:\n{stderr2}"
    );
    assert!(
        stderr2.contains("USAGE:"),
        "expected usage message on error, got:\n{stderr2}"
    );

    // 3. Unknown command
    let output3 = Command::new(&bin)
        .arg("bogus-command")
        .output()
        .expect("execute with bogus command");
    assert!(
        !output3.status.success(),
        "bogus-command must exit non-zero"
    );
    let stderr3 = String::from_utf8_lossy(&output3.stderr);
    assert!(
        stderr3.contains("Unknown command or unexpected argument 'bogus-command'"),
        "expected unknown command error, got:\n{stderr3}"
    );
    assert!(
        stderr3.contains("USAGE:"),
        "expected usage message on error, got:\n{stderr3}"
    );
}

#[test]
fn test_agent_cli_config_non_interactive_val_agent_cli_003() {
    let bin = agent_bin();
    let temp_dir = std::env::temp_dir().join("test_agent_cli_config_non_interactive");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let settings_path = temp_dir.join("settings.json");

    let output = Command::new(&bin)
        .arg("config")
        .arg("--config")
        .arg(&settings_path)
        .arg("--server")
        .arg("https://config-updated.agent.local")
        .arg("--user")
        .arg("config_user")
        .arg("--pass")
        .arg("config_secret")
        .arg("--token")
        .arg("config_tok")
        .arg("--label")
        .arg("Config-Updated-Node")
        .arg("--scopes")
        .arg("files,code_execution,computer_use")
        .arg("--workspace")
        .arg("/srv/config-ws")
        .output()
        .expect("execute non-interactive config");

    assert!(
        output.status.success(),
        "non-interactive config command must exit with 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Configuration updated and saved to"),
        "expected update message, got:\n{stdout}"
    );

    assert!(settings_path.exists(), "settings.json must be written");
    let content = std::fs::read_to_string(&settings_path).expect("read settings.json");
    assert!(content.contains("https://config-updated.agent.local"));
    assert!(content.contains("config_user"));
    assert!(content.contains("Config-Updated-Node"));
    assert!(content.contains("/srv/config-ws"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_cli_install_and_uninstall_user_unit_val_agent_cli_004() {
    let bin = agent_bin();
    let temp_dir = std::env::temp_dir().join(format!(
        "test_agent_cli_install_user_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let unit_path = temp_dir.join("aqvn-agent.service");

    // 1. aqvn-agent install --user
    let output = Command::new(&bin)
        .env("AQVN_SYSTEMD_UNIT_PATH", &unit_path)
        .arg("install")
        .arg("--user")
        .arg("--workspace")
        .arg("/tmp/test-agent-ws-cli")
        .output()
        .expect("execute install --user");

    assert!(
        output.status.success(),
        "aqvn-agent install must exit with 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installing aqvn-agent systemd service..."));
    assert!(stdout.contains("Systemd unit file (user) written to:"));
    assert!(stdout.contains("aqvn-agent systemd service installed and enabled successfully."));

    assert!(unit_path.exists(), "unit file must be written to disk");
    let content = std::fs::read_to_string(&unit_path).expect("read generated unit file");
    assert!(content.contains("[Unit]"), "must have [Unit] section");
    assert!(content.contains("Description=AQVN Standalone Headless Agent & Multi-Host Bridge"));
    assert!(content.contains("After=network.target"));
    assert!(content.contains("[Service]"), "must have [Service] section");
    assert!(content.contains("Type=simple"));
    assert!(content.contains("ExecStart="), "must declare ExecStart");
    assert!(
        content.contains("start"),
        "ExecStart must invoke start command"
    );
    assert!(
        content.contains("WorkingDirectory=/tmp/test-agent-ws-cli"),
        "must set WorkingDirectory"
    );
    assert!(
        content.contains("Restart=always"),
        "must specify Restart=always"
    );
    assert!(content.contains("RestartSec=5"));
    assert!(content.contains("[Install]"), "must have [Install] section");
    assert!(
        content.contains("WantedBy=default.target"),
        "user unit must target default.target"
    );

    // 2. aqvn-agent uninstall --user
    let output_uninst = Command::new(&bin)
        .env("AQVN_SYSTEMD_UNIT_PATH", &unit_path)
        .arg("uninstall")
        .arg("--user")
        .output()
        .expect("execute uninstall --user");

    assert!(
        output_uninst.status.success(),
        "aqvn-agent uninstall must exit with 0"
    );
    let stdout_uninst = String::from_utf8_lossy(&output_uninst.stdout);
    assert!(stdout_uninst.contains("Uninstalling aqvn-agent systemd service..."));
    assert!(stdout_uninst.contains("Removed unit file:"));
    assert!(stdout_uninst.contains("aqvn-agent systemd service uninstalled cleanly."));

    assert!(
        !unit_path.exists(),
        "unit file must be removed after uninstall"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_cli_install_system_unit_val_agent_cli_004() {
    let bin = agent_bin();
    let temp_dir =
        std::env::temp_dir().join(format!("test_agent_cli_install_sys_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let unit_path = temp_dir.join("aqvn-agent.service");

    let output = Command::new(&bin)
        .env("AQVN_SYSTEMD_UNIT_PATH", &unit_path)
        .arg("install")
        .arg("--system")
        .arg("--workspace")
        .arg("/srv/test-agent-sys")
        .output()
        .expect("execute install --system");

    assert!(
        output.status.success(),
        "aqvn-agent install --system must exit with 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Systemd unit file (system) written to:"));

    assert!(unit_path.exists(), "unit file must exist");
    let content = std::fs::read_to_string(&unit_path).expect("read system unit file");
    assert!(content.contains("Restart=always"));
    assert!(content.contains("WorkingDirectory=/srv/test-agent-sys"));
    assert!(
        content.contains("WantedBy=multi-user.target"),
        "system unit must target multi-user.target"
    );

    let output_uninst = Command::new(&bin)
        .env("AQVN_SYSTEMD_UNIT_PATH", &unit_path)
        .arg("uninstall")
        .arg("--system")
        .output()
        .expect("execute uninstall --system");

    assert!(output_uninst.status.success());
    assert!(!unit_path.exists(), "system unit file must be removed");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_agent_cli_install_with_custom_config_val_agent_cli_004() {
    let bin = agent_bin();
    let temp_dir =
        std::env::temp_dir().join(format!("test_agent_cli_install_cfg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let unit_path = temp_dir.join("aqvn-agent.service");
    let config_path = temp_dir.join("custom-settings.json");
    std::fs::write(&config_path, "{}").expect("write dummy config");

    let output = Command::new(&bin)
        .env("AQVN_SYSTEMD_UNIT_PATH", &unit_path)
        .arg("install")
        .arg("--config")
        .arg(&config_path)
        .arg("--workspace")
        .arg("/var/custom-ws")
        .output()
        .expect("execute install with config");

    assert!(output.status.success());
    let content = std::fs::read_to_string(&unit_path).expect("read unit file");
    assert!(content.contains("ExecStart="));
    assert!(content.contains(&format!("--config {}", config_path.display())));
    assert!(content.contains("WorkingDirectory=/var/custom-ws"));
    assert!(content.contains("Restart=always"));

    let output_uninst = Command::new(&bin)
        .env("AQVN_SYSTEMD_UNIT_PATH", &unit_path)
        .arg("uninstall")
        .output()
        .expect("execute uninstall");

    assert!(output_uninst.status.success());
    assert!(!unit_path.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

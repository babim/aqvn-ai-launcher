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

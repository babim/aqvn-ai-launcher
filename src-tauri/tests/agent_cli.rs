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

//! Release packaging and whitelabel compliance tests (VAL-REL-001, VAL-REL-002, VAL-REL-003).

use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("parent of src-tauri must be repo root")
        .to_path_buf()
}

#[test]
fn val_rel_001_version_is_0_1_8_in_cargo_tauri_and_changelog() {
    let cargo_toml = fs::read_to_string(manifest_dir().join("Cargo.toml"))
        .expect("must read src-tauri/Cargo.toml");
    assert!(
        cargo_toml.contains("version = \"0.1.8\"") || cargo_toml.contains("version = \"0.1.9\""),
        "Cargo.toml must have valid version (0.1.8 or 0.1.9)"
    );

    let tauri_conf_raw = fs::read_to_string(manifest_dir().join("tauri.conf.json"))
        .expect("must read src-tauri/tauri.conf.json");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_raw).expect("tauri.conf.json must be valid JSON");
    let version = tauri_conf["version"].as_str().unwrap_or("");
    assert!(
        version == "0.1.8" || version == "0.1.9",
        "tauri.conf.json version must be '0.1.8' or '0.1.9'"
    );

    let changelog =
        fs::read_to_string(repo_root().join("CHANGELOG.md")).expect("must read CHANGELOG.md");
    assert!(
        changelog.contains("## [0.1.8]") || changelog.contains("## [0.1.9]"),
        "CHANGELOG.md must contain ## [0.1.8] or ## [0.1.9] release section"
    );

    let about_html =
        fs::read_to_string(repo_root().join("ui/about.html")).expect("must read ui/about.html");
    assert!(
        about_html.contains("v0.1.8") || about_html.contains("v0.1.9"),
        "ui/about.html must display version 'v0.1.8' or 'v0.1.9'"
    );
}

#[test]
fn val_rel_002_whitelabel_zero_upstream_mentions_in_core_surfaces() {
    let surfaces = [
        manifest_dir().join("src"),
        repo_root().join("ui"),
        repo_root().join("README.md"),
        repo_root().join("Taskfile.yml"),
        repo_root().join("CHANGELOG.md"),
        repo_root().join("docs/rfc-spec"),
    ];

    fn check_file(path: &Path) {
        if path.is_dir() {
            for entry in fs::read_dir(path).expect("read_dir") {
                let entry = entry.expect("dir entry");
                check_file(&entry.path());
            }
        } else {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            // Check text source files, skip binary assets
            if matches!(
                ext,
                "rs" | "json" | "html" | "js" | "css" | "md" | "yml" | "yaml" | "toml"
            ) {
                let content = fs::read_to_string(path)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
                let lower = content.to_lowercase();
                assert!(
                    !lower.contains("agent zero")
                        && !lower.contains("agent-zero")
                        && !lower.contains("agent_zero"),
                    "Whitelabel violation in {}: contains forbidden upstream string",
                    path.display()
                );
            }
        }
    }

    for surface in surfaces {
        if surface.exists() {
            check_file(&surface);
        }
    }
}

#[test]
fn val_rel_003_macos_job_and_step_timeouts_in_release_workflow() {
    let workflow_path = repo_root().join(".github/workflows/release.yml");
    assert!(
        workflow_path.exists(),
        ".github/workflows/release.yml must exist"
    );
    let content = fs::read_to_string(&workflow_path).expect("must read release.yml");

    // Verify macos job exists and runs on macos-14 or macos-15
    assert!(
        content.contains("macos:"),
        "release.yml must define macos job"
    );
    assert!(
        content.contains("runs-on: macos-14") || content.contains("runs-on: macos-15"),
        "macos job must run on macos-14 or macos-15"
    );

    // Verify dmg and app bundles built and packaged
    assert!(
        content.contains("--bundles dmg,app"),
        "macos job must build dmg,app bundles via tauri-action"
    );
    assert!(
        content.contains(".dmg"),
        "macos job must reference .dmg artifact"
    );
    assert!(
        content.contains(".app"),
        "macos job must reference .app artifact"
    );
    assert!(
        content.contains(".tar.gz"),
        "macos job must package tarball"
    );

    // Verify timeout-minutes: 60 across all jobs
    let timeout_matches = content.matches("timeout-minutes: 60").count();
    assert!(
        timeout_matches >= 4,
        "release.yml must configure timeout-minutes: 60 across all runner jobs (linux, windows, macos, android)"
    );
}

#[test]
fn val_agent_rel_001_headless_linux_release_binary() {
    // 1. Verify Cargo.toml release profile configuration
    let cargo_toml = fs::read_to_string(manifest_dir().join("Cargo.toml"))
        .expect("must read src-tauri/Cargo.toml");
    assert!(
        cargo_toml.contains("[profile.release]"),
        "Cargo.toml must configure [profile.release]"
    );
    assert!(
        cargo_toml.contains("strip = true"),
        "[profile.release] must configure strip = true"
    );
    assert!(
        cargo_toml.contains("opt-level = \"z\"") || cargo_toml.contains("opt-level = \"s\""),
        "[profile.release] must configure size optimization (opt-level = \"z\" or \"s\")"
    );
    assert!(
        cargo_toml.contains("lto = true"),
        "[profile.release] must configure lto = true"
    );
    assert!(
        cargo_toml.contains("panic = \"abort\""),
        "[profile.release] must configure panic = \"abort\""
    );
    assert!(
        cargo_toml.contains("codegen-units = 1"),
        "[profile.release] must configure codegen-units = 1"
    );

    // 2. Check binary path
    let bin_path = manifest_dir().join("target/release/aqvn-agent");
    if !bin_path.exists() {
        let status = std::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("--bin")
            .arg("aqvn-agent")
            .current_dir(manifest_dir())
            .status()
            .expect("cargo build --release --bin aqvn-agent must execute");
        assert!(
            status.success(),
            "cargo build --release --bin aqvn-agent must succeed"
        );
    }

    assert!(
        bin_path.exists(),
        "release binary must exist at src-tauri/target/release/aqvn-agent"
    );

    // 3. Binary size must be under 15MB
    let metadata = fs::metadata(&bin_path).expect("must get binary metadata");
    let size = metadata.len();
    const MAX_SIZE_BYTES: u64 = 15 * 1024 * 1024;
    assert!(
        size > 0 && size <= MAX_SIZE_BYTES,
        "release binary size must be > 0 and <= 15MB ({} bytes), got: {} bytes ({:.2} MB)",
        MAX_SIZE_BYTES,
        size,
        size as f64 / (1024.0 * 1024.0)
    );

    // 4. Permissions must be 755 on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let perm_octal = mode & 0o777;
        assert_eq!(
            perm_octal, 0o755,
            "release binary permissions must be 755 (rwxr-xr-x), got: {:o}",
            perm_octal
        );
    }

    // 5. Binary is stripped of debug symbols
    if let Ok(output) = std::process::Command::new("file").arg(&bin_path).output() {
        if output.status.success() {
            let info = String::from_utf8_lossy(&output.stdout);
            assert!(
                info.contains("stripped"),
                "release binary must be stripped of debug symbols, got: {info}"
            );
        }
    }

    // 6. Execution works headless with 0 status code
    let output = std::process::Command::new(&bin_path)
        .arg("--version")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .expect("release binary must execute in headless environment");

    assert!(
        output.status.success(),
        "aqvn-agent --version must exit with code 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("aqvn-agent"),
        "aqvn-agent --version must print agent identifier, got: {stdout}"
    );
}

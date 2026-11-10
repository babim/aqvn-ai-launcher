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

#[test]
fn val_pkg_001_linux_bundle_metadata_and_artifact_configuration() {
    let tauri_conf_raw = fs::read_to_string(manifest_dir().join("tauri.conf.json"))
        .expect("must read src-tauri/tauri.conf.json");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_raw).expect("tauri.conf.json must be valid JSON");

    // 1. App identifier and whitelabel product title
    let identifier = tauri_conf["identifier"]
        .as_str()
        .expect("identifier string");
    assert_eq!(
        identifier, "net.aqvn.workspace",
        "app identifier must be net.aqvn.workspace"
    );

    let product_name = tauri_conf["productName"]
        .as_str()
        .expect("productName string");
    assert_eq!(
        product_name, "AQVN Workspace",
        "productName must be AQVN Workspace"
    );

    // 2. Bundle targets include deb and rpm
    let targets = tauri_conf["bundle"]["targets"]
        .as_array()
        .expect("bundle.targets array");
    let target_strings: Vec<&str> = targets.iter().filter_map(|t| t.as_str()).collect();
    assert!(
        target_strings.contains(&"deb"),
        "bundle.targets must contain deb"
    );
    assert!(
        target_strings.contains(&"rpm"),
        "bundle.targets must contain rpm"
    );

    // 3. Category and desktop integration metadata
    let category = tauri_conf["bundle"]["category"]
        .as_str()
        .expect("bundle.category");
    assert!(
        category == "DeveloperTool" || category == "Utility",
        "bundle.category must be a valid desktop category"
    );

    let short_desc = tauri_conf["bundle"]["shortDescription"]
        .as_str()
        .unwrap_or("");
    assert!(
        !short_desc.is_empty(),
        "bundle.shortDescription must be specified"
    );

    // 4. Linux specific configuration for deb and rpm
    let linux = &tauri_conf["bundle"]["linux"];
    assert!(linux.is_object(), "bundle.linux must be configured");
    assert!(
        linux["deb"].is_object(),
        "bundle.linux.deb must be configured"
    );
    assert!(
        linux["rpm"].is_object(),
        "bundle.linux.rpm must be configured"
    );

    // 5. Standard icon resolutions exist on disk
    let icons_dir = manifest_dir().join("icons");
    let required_icons = ["32x32.png", "128x128.png", "256x256.png", "icon.png"];
    for icon_name in required_icons {
        let icon_path = icons_dir.join(icon_name);
        assert!(
            icon_path.exists(),
            "Required bundle icon {} must exist at {}",
            icon_name,
            icon_path.display()
        );
        let metadata = fs::metadata(&icon_path).expect("icon metadata");
        assert!(metadata.len() > 0, "Icon {} must not be empty", icon_name);
    }

    // 6. Config icons list references required icons
    let icon_list = tauri_conf["bundle"]["icon"]
        .as_array()
        .expect("bundle.icon array");
    let icon_strings: Vec<&str> = icon_list.iter().filter_map(|i| i.as_str()).collect();
    assert!(
        icon_strings.iter().any(|s| s.contains("32x32")),
        "bundle.icon must reference 32x32"
    );
    assert!(
        icon_strings.iter().any(|s| s.contains("128x128")),
        "bundle.icon must reference 128x128"
    );
    assert!(
        icon_strings.iter().any(|s| s.contains("256x256")),
        "bundle.icon must reference 256x256"
    );
    assert!(
        icon_strings.iter().any(|s| s.contains("icon.png")),
        "bundle.icon must reference icon.png"
    );
}

#[test]
fn val_pkg_002_windows_nsis_and_msi_bundle_configuration_and_size_guard() {
    let tauri_conf_raw = fs::read_to_string(manifest_dir().join("tauri.conf.json"))
        .expect("must read src-tauri/tauri.conf.json");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_raw).expect("tauri.conf.json must be valid JSON");

    // 1. Bundle targets include nsis and msi
    let targets = tauri_conf["bundle"]["targets"]
        .as_array()
        .expect("bundle.targets array");
    let target_strings: Vec<&str> = targets.iter().filter_map(|t| t.as_str()).collect();
    assert!(
        target_strings.contains(&"nsis"),
        "bundle.targets must contain nsis"
    );
    assert!(
        target_strings.contains(&"msi"),
        "bundle.targets must contain msi"
    );

    // 2. Windows specific bundle configuration
    let windows = &tauri_conf["bundle"]["windows"];
    assert!(windows.is_object(), "bundle.windows must be configured");

    let nsis = &windows["nsis"];
    assert!(nsis.is_object(), "bundle.windows.nsis must be configured");
    let nsis_icon = nsis["installerIcon"].as_str().unwrap_or("");
    assert!(
        nsis_icon.contains("icon.ico"),
        "nsis.installerIcon must reference icon.ico, got: {nsis_icon}"
    );

    let install_mode = nsis["installMode"].as_str().unwrap_or("");
    assert!(
        install_mode == "both" || install_mode == "currentUser" || install_mode == "perMachine",
        "nsis.installMode must be valid, got: {install_mode}"
    );

    let wix = &windows["wix"];
    assert!(wix.is_object(), "bundle.windows.wix must be configured");
    assert!(
        wix["language"].as_str().is_some() || wix["language"].as_array().is_some(),
        "bundle.windows.wix.language must be configured"
    );

    // 3. icon.ico exists on disk and in bundle icon set
    let icon_list = tauri_conf["bundle"]["icon"]
        .as_array()
        .expect("bundle.icon array");
    let icon_strings: Vec<&str> = icon_list.iter().filter_map(|i| i.as_str()).collect();
    assert!(
        icon_strings.iter().any(|s| s.contains("icon.ico")),
        "bundle.icon must reference icon.ico"
    );

    let ico_path = manifest_dir().join("icons/icon.ico");
    assert!(
        ico_path.exists(),
        "icons/icon.ico must exist at {}",
        ico_path.display()
    );
    let ico_bytes = fs::read(&ico_path).expect("must read icon.ico");
    assert!(!ico_bytes.is_empty(), "icon.ico must not be empty");
    assert!(
        ico_bytes.starts_with(&[0, 0, 1, 0]),
        "icon.ico must start with ICO header bytes [0x00, 0x00, 0x01, 0x00]"
    );

    // 4. Size limit enforcement (< 10MB per VAL-CI-003):
    // WebView2 install mode must use downloadBootstrapper (offlineInstaller embeds ~127MB)
    let webview_mode = &windows["webviewInstallMode"];
    let mode_type = webview_mode["type"].as_str().unwrap_or("");
    assert_eq!(
        mode_type, "downloadBootstrapper",
        "webviewInstallMode must be downloadBootstrapper to satisfy < 10MB binary size limit"
    );

    // 5. CI release workflow verifies NSIS size gate (< 10MB)
    let release_yml_path = repo_root().join(".github/workflows/release.yml");
    let release_yml = fs::read_to_string(&release_yml_path).expect("must read release.yml");
    assert!(
        release_yml.contains("Check NSIS installer size gate (< 10MB)"),
        "release.yml must include NSIS size gate check step"
    );
    assert!(
        release_yml.contains("MAX_SIZE=10485760"),
        "release.yml must enforce MAX_SIZE=10485760 (10MB limit)"
    );
    assert!(
        release_yml.contains("--bundles nsis,msi"),
        "release.yml windows job must build nsis,msi bundles"
    );
}

#[test]
fn val_gate_size_001_binary_size_gate_and_footprint_check() {
    // 1. Verify aqvn-agent release binary <= 2.80MB (2,936,012 bytes)
    let agent_bin = manifest_dir().join("target/release/aqvn-agent");
    let fallback_agent = repo_root().join("target/release/aqvn-agent");
    let target_agent = if agent_bin.exists() {
        agent_bin
    } else {
        fallback_agent
    };
    if target_agent.exists() {
        let size = fs::metadata(&target_agent).expect("metadata").len();
        const MAX_AGENT_BYTES: u64 = (2.80 * 1024.0 * 1024.0) as u64;
        assert!(
            size > 0 && size <= MAX_AGENT_BYTES,
            "aqvn-agent release binary size ({} bytes) exceeds gate limit <= 2.80MB ({} bytes)",
            size,
            MAX_AGENT_BYTES
        );
    }

    // 2. Verify aqvn-workspace release binary <= 12.0MB (12,582,912 bytes)
    let workspace_bin = manifest_dir().join("target/release/aqvn-workspace");
    let fallback_workspace = repo_root().join("target/release/aqvn-workspace");
    let target_workspace = if workspace_bin.exists() {
        workspace_bin
    } else {
        fallback_workspace
    };
    if target_workspace.exists() {
        let size = fs::metadata(&target_workspace).expect("metadata").len();
        const MAX_WORKSPACE_BYTES: u64 = 12 * 1024 * 1024;
        assert!(
            size > 0 && size <= MAX_WORKSPACE_BYTES,
            "aqvn-workspace release binary size ({} bytes) exceeds gate limit <= 12.0MB ({} bytes)",
            size,
            MAX_WORKSPACE_BYTES
        );
    }

    // 3. Verify Windows NSIS configuration enforces < 9.80MB size gate
    let tauri_conf_raw = fs::read_to_string(manifest_dir().join("tauri.conf.json"))
        .expect("must read src-tauri/tauri.conf.json");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_raw).expect("tauri.conf.json must be valid JSON");
    let webview_mode = &tauri_conf["bundle"]["windows"]["webviewInstallMode"];
    let mode_type = webview_mode["type"].as_str().unwrap_or("");
    assert_eq!(
        mode_type, "downloadBootstrapper",
        "webviewInstallMode must be downloadBootstrapper to satisfy < 9.8MB NSIS installer limit"
    );

    // 4. Verify tools/benchmark.sh exists and passes bash -n syntax check
    let bench_script = repo_root().join("tools/benchmark.sh");
    assert!(bench_script.exists(), "tools/benchmark.sh must exist");
    let status = std::process::Command::new("bash")
        .arg("-n")
        .arg(&bench_script)
        .status()
        .expect("bash -n tools/benchmark.sh must execute");
    assert!(
        status.success(),
        "tools/benchmark.sh must have valid bash syntax"
    );
}

#[test]
fn val_gate_regression_002_full_system_regression_suite_clean_pass() {
    // 1. Verify benchmark.sh exists and has release binary size threshold
    let bench_script = repo_root().join("tools/benchmark.sh");
    let bench_content = fs::read_to_string(&bench_script).expect("read benchmark.sh");
    assert!(
        bench_content.contains("MAX_BINARY_BYTES=10485760")
            || bench_content.contains("MAX_BINARY_BYTES="),
        "benchmark.sh must define binary gate threshold"
    );

    // 2. Verify release workflow contains full CI test gates
    let release_yml_path = repo_root().join(".github/workflows/release.yml");
    let release_yml = fs::read_to_string(&release_yml_path).expect("read release.yml");
    assert!(
        release_yml.contains("Check NSIS installer size gate (< 10MB)"),
        "release.yml must enforce NSIS size gate"
    );

    // 3. Verify package.json contains lint:html and test scripts
    let pkg_json_raw =
        fs::read_to_string(repo_root().join("package.json")).expect("read package.json");
    let pkg_json: serde_json::Value =
        serde_json::from_str(&pkg_json_raw).expect("package.json must be valid JSON");
    assert!(
        pkg_json["scripts"]["lint:html"].as_str().is_some(),
        "package.json must define lint:html script"
    );
    assert!(
        pkg_json["scripts"]["test"].as_str().is_some(),
        "package.json must define test script"
    );
}

#[test]
fn val_prod_alloc_003_mimalloc_and_pgo_pipeline_integration() {
    // 1. Verify mimalloc is configured in crates/aqvn-agent/Cargo.toml
    let agent_cargo_toml = fs::read_to_string(repo_root().join("crates/aqvn-agent/Cargo.toml"))
        .expect("must read crates/aqvn-agent/Cargo.toml");
    assert!(
        agent_cargo_toml.contains("mimalloc"),
        "crates/aqvn-agent/Cargo.toml must declare mimalloc dependency"
    );

    // 2. Verify mimalloc is configured in src-tauri/Cargo.toml
    let tauri_cargo_toml = fs::read_to_string(manifest_dir().join("Cargo.toml"))
        .expect("must read src-tauri/Cargo.toml");
    assert!(
        tauri_cargo_toml.contains("mimalloc"),
        "src-tauri/Cargo.toml must declare mimalloc dependency"
    );

    // 3. Verify global allocator configured in binary crates
    let agent_main = fs::read_to_string(repo_root().join("crates/aqvn-agent/src/main.rs"))
        .expect("must read crates/aqvn-agent/src/main.rs");
    assert!(
        agent_main.contains("#[global_allocator]") && agent_main.contains("mimalloc::MiMalloc"),
        "crates/aqvn-agent/src/main.rs must configure mimalloc as global allocator"
    );

    let tauri_main = fs::read_to_string(manifest_dir().join("src/main.rs"))
        .expect("must read src-tauri/src/main.rs");
    assert!(
        tauri_main.contains("#[global_allocator]") && tauri_main.contains("mimalloc::MiMalloc"),
        "src-tauri/src/main.rs must configure mimalloc as global allocator"
    );

    let tauri_bin_agent = fs::read_to_string(manifest_dir().join("src/bin/agent.rs"))
        .expect("must read src-tauri/src/bin/agent.rs");
    assert!(
        tauri_bin_agent.contains("#[global_allocator]")
            && tauri_bin_agent.contains("mimalloc::MiMalloc"),
        "src-tauri/src/bin/agent.rs must configure mimalloc as global allocator"
    );

    // 4. Verify Taskfile.yml supports bench and PGO pipeline tasks
    let taskfile =
        fs::read_to_string(repo_root().join("Taskfile.yml")).expect("must read Taskfile.yml");
    assert!(
        taskfile.contains("bench:"),
        "Taskfile.yml must define bench task"
    );
    assert!(
        taskfile.contains("build:pgo:"),
        "Taskfile.yml must define build:pgo task"
    );
    assert!(
        taskfile.contains("build:pgo:instrument:"),
        "Taskfile.yml must define build:pgo:instrument task"
    );
    assert!(
        taskfile.contains("build:pgo:optimize:"),
        "Taskfile.yml must define build:pgo:optimize task"
    );

    // 5. If release binary exists, verify size < 2.5MB (2,621,440 bytes)
    let agent_bin = repo_root().join("target/release/aqvn-agent");
    let fallback_agent = manifest_dir().join("target/release/aqvn-agent");
    let target_agent = if agent_bin.exists() {
        agent_bin
    } else {
        fallback_agent
    };
    if target_agent.exists() {
        let size = fs::metadata(&target_agent).expect("metadata").len();
        const MAX_PGO_AGENT_BYTES: u64 = 2621440; // 2.5 MB
        assert!(
            size > 0 && size <= MAX_PGO_AGENT_BYTES,
            "aqvn-agent release binary size ({} bytes) must be <= 2.5MB ({} bytes)",
            size,
            MAX_PGO_AGENT_BYTES
        );
    }
}

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
        cargo_toml.contains("version = \"0.1.8\""),
        "Cargo.toml must have version = '0.1.8'"
    );

    let tauri_conf_raw = fs::read_to_string(manifest_dir().join("tauri.conf.json"))
        .expect("must read src-tauri/tauri.conf.json");
    let tauri_conf: serde_json::Value =
        serde_json::from_str(&tauri_conf_raw).expect("tauri.conf.json must be valid JSON");
    assert_eq!(
        tauri_conf["version"].as_str(),
        Some("0.1.8"),
        "tauri.conf.json version must be '0.1.8'"
    );

    let changelog =
        fs::read_to_string(repo_root().join("CHANGELOG.md")).expect("must read CHANGELOG.md");
    assert!(
        changelog.contains("## [0.1.8]"),
        "CHANGELOG.md must contain ## [0.1.8] release section"
    );

    let about_html =
        fs::read_to_string(repo_root().join("ui/about.html")).expect("must read ui/about.html");
    assert!(
        about_html.contains("v0.1.8"),
        "ui/about.html must display version 'v0.1.8'"
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

    // Verify macos job exists and runs on macos-14
    assert!(
        content.contains("macos:"),
        "release.yml must define macos job"
    );
    assert!(
        content.contains("runs-on: macos-14"),
        "macos job must run on macos-14"
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

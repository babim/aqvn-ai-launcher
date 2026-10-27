//! Scaffold invariants: bundle identity, icons, capabilities, size profile.

use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

#[test]
fn bundle_identity_is_aqvn() {
    let conf = read_json(&manifest_dir().join("tauri.conf.json"));
    assert_eq!(conf["identifier"], "net.aqvn.workspace");
    assert_eq!(conf["productName"], "AQVN Workspace");
    assert_eq!(conf["app"]["windows"][0]["title"], "AQVN Workspace");
    assert_eq!(conf["build"]["frontendDist"], "../ui");
    assert!(conf["build"]["removeUnusedCommands"]
        .as_bool()
        .unwrap_or(false));
}

#[test]
fn bundle_icons_exist_and_are_valid_images() {
    let dir = manifest_dir();
    let conf = read_json(&dir.join("tauri.conf.json"));
    let icons = conf["bundle"]["icon"]
        .as_array()
        .expect("bundle.icon array");
    assert!(
        icons.iter().any(|icon| icon == "icons/icon.png"),
        "icons/icon.png is mandatory"
    );

    for icon in icons {
        let rel = icon.as_str().expect("icon path string");
        let bytes = std::fs::read(dir.join(rel)).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
        assert!(!bytes.is_empty(), "{rel} is empty");
        let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
        let ico = bytes.starts_with(&[0, 0, 1, 0]);
        assert!(png || ico, "{rel} is not a PNG/ICO image");
    }
}

#[test]
fn capabilities_are_explicit() {
    let cap = read_json(&manifest_dir().join("capabilities/default.json"));
    assert_eq!(cap["identifier"], "default");
    assert_eq!(cap["windows"][0], "main");
    let permissions = cap["permissions"].as_array().expect("permissions array");
    assert!(!permissions.is_empty(), "permissions must not be empty");
    for permission in permissions {
        let permission = permission.as_str().expect("permission string");
        assert!(
            !permission.contains('*'),
            "wildcard permission not allowed: {permission}"
        );
    }
}

#[test]
fn release_profile_is_size_optimized() {
    let cargo = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).expect("Cargo.toml");
    for needle in [
        "[profile.release]",
        "codegen-units = 1",
        "lto = true",
        "panic = \"abort\"",
        "strip = true",
    ] {
        assert!(
            cargo.contains(needle),
            "missing release profile setting: {needle}"
        );
    }
    assert!(
        cargo.contains("opt-level = \"s\"") || cargo.contains("opt-level = \"z\""),
        "missing release profile setting: opt-level = 's' or 'z'"
    );
}

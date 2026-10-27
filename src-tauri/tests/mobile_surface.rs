//! Mobile platform surface tests (VAL-MOB-001 through VAL-MOB-004).

use std::fs;
use std::path::Path;

#[test]
fn android_gen_directory_exists_and_contains_gradle_project() {
    let manifest_path = Path::new("gen/android/app/src/main/AndroidManifest.xml");
    assert!(
        manifest_path.exists(),
        "src-tauri/gen/android/app/src/main/AndroidManifest.xml must exist"
    );

    let gradle_path = Path::new("gen/android/build.gradle.kts");
    assert!(
        gradle_path.exists(),
        "src-tauri/gen/android/build.gradle.kts must exist"
    );
}

#[test]
fn android_manifest_has_whitelabel_and_internet_permission() {
    let manifest_content = fs::read_to_string("gen/android/app/src/main/AndroidManifest.xml")
        .expect("must read AndroidManifest.xml");

    assert!(
        manifest_content.contains("android.permission.INTERNET"),
        "Manifest must declare INTERNET permission"
    );

    // Ensure no dangerous or desktop-specific permissions are requested
    for forbidden in [
        "ACCESS_FINE_LOCATION",
        "RECORD_AUDIO",
        "CAMERA",
        "WRITE_EXTERNAL_STORAGE",
    ] {
        assert!(
            !manifest_content.contains(forbidden),
            "Manifest must not request {forbidden}"
        );
    }

    let strings_content = fs::read_to_string("gen/android/app/src/main/res/values/strings.xml")
        .expect("must read strings.xml");
    assert!(
        strings_content.contains(r#""AQVN Workspace""#),
        "app_name must be 'AQVN Workspace'"
    );

    let build_gradle = fs::read_to_string("gen/android/app/build.gradle.kts")
        .expect("must read app/build.gradle.kts");
    assert!(
        build_gradle.contains(r#"applicationId = "net.aqvn.workspace""#),
        "applicationId must be 'net.aqvn.workspace'"
    );
}

#[test]
fn capabilities_contain_no_control_or_desktop_shortcuts_for_mobile() {
    let default_cap = fs::read_to_string("capabilities/default.json")
        .expect("must read capabilities/default.json");

    assert!(
        !default_cap.contains("global-shortcut"),
        "default.json must not include global-shortcut for mobile"
    );
    assert!(
        !default_cap.contains("rfc"),
        "default.json must contain no rfc permissions"
    );
    assert!(
        !default_cap.contains("ssh"),
        "default.json must contain no ssh permissions"
    );

    let desktop_cap = fs::read_to_string("capabilities/desktop.json")
        .expect("must read capabilities/desktop.json");
    assert!(
        desktop_cap.contains(r#""platforms": ["linux", "windows", "macOS"]"#)
            || desktop_cap.contains(r#""platforms": ["linux", "windows", "macos"]"#)
            || desktop_cap.contains("linux"),
        "desktop.json must restrict platforms"
    );
}

#[test]
fn cargo_toml_mobile_deps_exclude_desktop_features() {
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("must read Cargo.toml");
    assert!(
        cargo_toml.contains(r#"[target.'cfg(not(target_os = "android"))'.dependencies]"#),
        "Cargo.toml must gate desktop-only deps behind target cfg"
    );
}

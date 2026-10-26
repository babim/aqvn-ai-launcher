//! Window title sync invariants (VAL-SCAF-007/008/009).
//!
//! Title must flow from the page's `document.title` to the native window through
//! Tauri's `on_document_title_changed` handler — never through JS `setTitle`,
//! which would need the `core:window:allow-set-title` permission.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a parent")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn rust_builder_wires_document_title_to_window_title() {
    let lib = read("src-tauri/src/lib.rs");
    assert!(
        lib.contains(".on_document_title_changed("),
        "lib.rs must wire on_document_title_changed"
    );
    assert!(
        lib.contains("set_title(&title)"),
        "handler must call win.set_title(&title)"
    );
}

#[test]
fn no_js_set_title_permission() {
    let caps = read("src-tauri/capabilities/default.json");
    assert!(
        !caps.contains("allow-set-title"),
        "title sync must not need core:window:allow-set-title"
    );
    assert!(
        !read("ui/index.html").contains("setTitle"),
        "UI must not call JS setTitle"
    );
    assert!(
        !read("ui/settings.html").contains("setTitle"),
        "UI must not call JS setTitle"
    );
}

#[test]
fn main_window_config_defers_creation_to_rust() {
    let conf: serde_json::Value =
        serde_json::from_str(&read("src-tauri/tauri.conf.json")).expect("valid tauri.conf.json");
    let main_window = &conf["app"]["windows"][0];
    assert_eq!(main_window["label"], "main");
    assert_eq!(
        main_window["create"], false,
        "window must defer creation so Rust WebviewWindowBuilder attaches on_document_title_changed"
    );
    let lib = read("src-tauri/src/lib.rs");
    assert!(
        lib.contains("from_config"),
        "Rust setup must build window via from_config with title sync handler"
    );
}

#[test]
fn index_sets_document_title_and_links_settings() {
    let index = read("ui/index.html");
    assert!(
        index.contains("document.title = \"AQVN AI Workspace\""),
        "index.html must set document.title = \"AQVN AI Workspace\""
    );
    assert!(
        index.contains("href=\"settings.html\""),
        "index.html must link to settings.html"
    );
}

#[test]
fn settings_has_own_title_and_links_back() {
    let settings = read("ui/settings.html");
    assert!(
        settings.contains("document.title = "),
        "settings.html must set its own document.title"
    );
    assert!(
        settings.contains("href=\"index.html\""),
        "settings.html must link back to index.html"
    );
}

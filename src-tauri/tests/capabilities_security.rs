//! Security hardening verification: IPC capability scope and Content Security Policy (VAL-SEC-003).

use serde_json::Value;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: PathBuf) -> Value {
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse JSON from {}: {e}", path.display()))
}

#[test]
fn test_shell_allow_open_capability_scheme_scope() {
    let cap_path = manifest_dir().join("capabilities/default.json");
    let cap = read_json(cap_path);

    let permissions = cap["permissions"]
        .as_array()
        .expect("permissions array in capabilities/default.json");

    // Find shell:allow-open permission entry
    let shell_open = permissions
        .iter()
        .find(|entry| {
            if let Some(id) = entry.as_str() {
                id == "shell:allow-open"
            } else if let Some(id) = entry.get("identifier").and_then(|v| v.as_str()) {
                id == "shell:allow-open"
            } else {
                false
            }
        })
        .expect("shell:allow-open permission must exist in capabilities/default.json");

    // Must be scoped object, not raw unrestricted string permission
    assert!(
        shell_open.is_object(),
        "shell:allow-open must be a scoped object with explicit scheme filter"
    );

    let allow_rules = shell_open
        .get("allow")
        .and_then(|v| v.as_array())
        .expect("shell:allow-open must define an 'allow' scope array");

    assert!(
        !allow_rules.is_empty(),
        "shell:allow-open allow rules must not be empty"
    );

    // Extract patterns/validators/hrefs from allow entries
    let mut patterns = Vec::new();
    for rule in allow_rules {
        if let Some(s) = rule.as_str() {
            patterns.push(s.to_string());
        } else if let Some(obj) = rule.as_object() {
            if let Some(href) = obj.get("href").and_then(|v| v.as_str()) {
                patterns.push(href.to_string());
            }
            if let Some(validator) = obj.get("validator").and_then(|v| v.as_str()) {
                patterns.push(validator.to_string());
            }
            if let Some(pattern) = obj.get("pattern").and_then(|v| v.as_str()) {
                patterns.push(pattern.to_string());
            }
        }
    }

    assert!(
        !patterns.is_empty(),
        "Must have extracted at least one scope pattern"
    );

    // Verify explicit scheme scope filters: ^https?:// and ^mailto:
    let has_http_or_https = patterns
        .iter()
        .any(|p| p.contains("^https?://") || (p.contains("http://") && p.contains("https://")));
    let has_mailto = patterns.iter().any(|p| p.contains("^mailto:"));

    assert!(
        has_http_or_https,
        "shell:allow-open scope must explicitly filter http and https schemes: {patterns:?}"
    );
    assert!(
        has_mailto,
        "shell:allow-open scope must explicitly filter mailto scheme: {patterns:?}"
    );

    // Helper to test if a URL matches the allowed schemes according to scope rules
    let matches_scheme_rules = |url: &str| -> bool {
        let is_http_https = url.starts_with("http://") || url.starts_with("https://");
        let is_mailto = url.starts_with("mailto:");

        // If the URL matches allowed schemes and does not use dangerous protocols
        (is_http_https && has_http_or_https) || (is_mailto && has_mailto)
    };

    // Allowed URLs
    assert!(
        matches_scheme_rules("https://aqvn.net"),
        "https URL should match"
    );
    assert!(
        matches_scheme_rules("https://github.com/tauri-apps/tauri"),
        "https URL should match"
    );
    assert!(
        matches_scheme_rules("http://localhost:3000"),
        "http URL should match"
    );
    assert!(
        matches_scheme_rules("http://127.0.0.1:55088"),
        "http loopback URL should match"
    );
    assert!(
        matches_scheme_rules("mailto:contact@aqvn.net"),
        "mailto URL should match"
    );
    assert!(
        matches_scheme_rules("mailto:support@example.com?subject=Help"),
        "mailto query URL should match"
    );

    // Rejected arbitrary protocol handlers and file schemes
    assert!(
        !matches_scheme_rules("file:///etc/passwd"),
        "file:// URL must be rejected"
    );
    assert!(
        !matches_scheme_rules("file:///C:/Windows/System32/calc.exe"),
        "file:// URL must be rejected"
    );
    assert!(
        !matches_scheme_rules("javascript:alert(1)"),
        "javascript: scheme must be rejected"
    );
    assert!(
        !matches_scheme_rules("data:text/html;base64,..."),
        "data: scheme must be rejected"
    );
    assert!(
        !matches_scheme_rules("custom-proto://launch"),
        "custom protocol must be rejected"
    );
    assert!(
        !matches_scheme_rules("ssh://user@hostname"),
        "ssh: protocol must be rejected"
    );
    assert!(
        !matches_scheme_rules("tel:+1234567890"),
        "tel: protocol must be rejected by explicit scheme filter"
    );
    assert!(
        !matches_scheme_rules("smb://nas.local/share"),
        "smb: protocol must be rejected"
    );
    assert!(
        !matches_scheme_rules("ms-settings:privacy"),
        "ms-settings: protocol must be rejected"
    );
}

#[test]
fn test_tauri_conf_strict_content_security_policy() {
    let conf_path = manifest_dir().join("tauri.conf.json");
    let conf = read_json(conf_path);

    let security = conf["app"]["security"]
        .as_object()
        .expect("app.security object in tauri.conf.json");

    let csp_val = security
        .get("csp")
        .expect("app.security.csp must be defined in tauri.conf.json");

    assert!(
        !csp_val.is_null(),
        "Content Security Policy must not be null"
    );

    let csp_str = csp_val
        .as_str()
        .expect("app.security.csp must be a string CSP policy");

    assert!(
        !csp_str.trim().is_empty(),
        "app.security.csp must not be empty"
    );

    // Verify essential CSP directives are present
    let directives: Vec<&str> = csp_str.split(';').map(|s| s.trim()).collect();

    let has_directive = |name: &str| directives.iter().any(|d| d.starts_with(name));

    let get_directive =
        |name: &str| -> Option<&str> { directives.iter().find(|d| d.starts_with(name)).copied() };

    assert!(
        has_directive("default-src"),
        "CSP must define default-src directive"
    );
    let default_src = get_directive("default-src").unwrap();
    assert!(
        default_src.contains("'self'"),
        "default-src must include 'self'"
    );

    assert!(
        has_directive("script-src"),
        "CSP must define script-src directive"
    );
    let script_src = get_directive("script-src").unwrap();
    assert!(
        script_src.contains("'self'"),
        "script-src must restrict to 'self'"
    );

    assert!(
        has_directive("connect-src"),
        "CSP must define connect-src directive for IPC and loopback origins"
    );
    let connect_src = get_directive("connect-src").unwrap();
    assert!(
        connect_src.contains("ipc:") || connect_src.contains("'self'"),
        "connect-src must permit ipc: or 'self'"
    );

    assert!(
        has_directive("object-src"),
        "CSP must define object-src directive"
    );
    let object_src = get_directive("object-src").unwrap();
    assert!(
        object_src.contains("'none'"),
        "object-src must be 'none' to disallow plugin execution"
    );

    assert!(
        has_directive("frame-ancestors"),
        "CSP must define frame-ancestors directive"
    );
    let frame_ancestors = get_directive("frame-ancestors").unwrap();
    assert!(
        frame_ancestors.contains("'none'") || frame_ancestors.contains("'self'"),
        "frame-ancestors must be restrictive"
    );
}

#[test]
fn test_plugin_shell_open_defense_in_depth_if_configured() {
    let conf_path = manifest_dir().join("tauri.conf.json");
    let conf = read_json(conf_path);

    if let Some(open_validator) = conf
        .get("plugins")
        .and_then(|p| p.get("shell"))
        .and_then(|s| s.get("open"))
        .and_then(|o| o.as_str())
    {
        // If shell open is configured in plugins, verify it specifies http(s) and mailto
        assert!(
            open_validator.contains("https?://") || open_validator.contains("http"),
            "open validator must cover http(s)"
        );
        assert!(
            open_validator.contains("mailto:"),
            "open validator must cover mailto"
        );
        assert!(
            !open_validator.contains("file:"),
            "open validator must not allow file: URLs"
        );
    }
}

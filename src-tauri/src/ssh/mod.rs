//! Embedded SSH bridge.
//!
//! Password-authenticated PTY shell gated by the same master toggle as the RFC
//! server (`rfc_enabled`, default OFF = no listener). The command denylist in
//! [`guard`] is a best-effort secondary layer on top of the real boundary.

pub mod guard;
pub mod server;
pub mod shell;

/// Resolves the shell working directory: the first allowed path, `~` expanded.
///
/// Falls back to the sandbox default when nothing is configured.
pub fn session_cwd(allowed_paths: &[String]) -> std::path::PathBuf {
    let raw = allowed_paths
        .first()
        .map(String::as_str)
        .unwrap_or(crate::rfc::sandbox::DEFAULT_SANDBOX);
    expand_tilde(raw)
}

fn expand_tilde(raw: &str) -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    if let Some(home) = home {
        if let Some(rest) = raw.strip_prefix("~/") {
            return home.join(rest);
        }
        if raw == "~" {
            return home;
        }
    }
    std::path::PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_cwd_uses_first_allowed_path() {
        let cwd = session_cwd(&["/tmp/aaa".into(), "/tmp/bbb".into()]);
        assert_eq!(cwd, std::path::PathBuf::from("/tmp/aaa"));
    }

    #[test]
    fn session_cwd_expands_tilde() {
        let home = std::env::var_os("HOME").expect("HOME set in tests");
        let cwd = session_cwd(&["~/Workspace".into()]);
        assert_eq!(cwd, std::path::PathBuf::from(home).join("Workspace"));
    }

    #[test]
    fn session_cwd_falls_back_to_default() {
        let cwd = session_cwd(&[]);
        assert!(cwd.ends_with("Workspace"));
    }
}

//! Embedded SSH bridge for AQVN Workspace.

pub mod guard;
pub mod landlock;
pub mod server;
pub mod shell;

pub use landlock::{
    get_landlock_abi_version, is_landlock_supported, restrict_current_process, run_in_confinement,
    session_confinement, test_landlock_confinement, ConfinementError, ConfinementProfile,
};
pub use server::{run_supervised, ClientHandler, LockoutTracker, SshServer};

/// Resolves the shell working directory: the first allowed path, `~` expanded.
pub fn session_cwd(allowed_paths: &[String]) -> std::path::PathBuf {
    let raw = allowed_paths
        .first()
        .map(String::as_str)
        .unwrap_or(aqvn_rfc::sandbox::DEFAULT_SANDBOX);
    expand_tilde(raw)
}

fn expand_tilde(raw: &str) -> std::path::PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from);
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

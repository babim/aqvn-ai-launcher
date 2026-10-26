//! Path jail: every agent-supplied path is mapped into the configured sandbox
//! root and denied when it escapes (foreign absolute paths, `..`, symlink out).
//!
//! Callers send their own absolute layout (for example `/a0/usr/workdir/x`), so
//! the `/a0` prefix maps onto the sandbox root. Nothing outside the root is
//! readable, writable or deletable.

use std::path::{Component, Path, PathBuf};

/// Fallback sandbox root when no allowed path is configured.
pub const DEFAULT_SANDBOX: &str = "~/Workspace";

/// Agent-facing prefix that maps onto the sandbox root.
const AGENT_ROOT_PREFIX: &str = "/a0";

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("path is outside the allowed sandbox: {0}")]
    Denied(String),
    #[error("sandbox root unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Builds a sandbox from the first configured allowed path.
    ///
    /// A missing root is created so the default `~/Workspace` works on a fresh
    /// install; a failed creation surfaces later as `Unavailable` on use.
    pub fn from_allowed_paths(allowed: &[String]) -> Self {
        let raw = allowed
            .first()
            .map(String::as_str)
            .unwrap_or(DEFAULT_SANDBOX);
        let sandbox = Self::new(expand_tilde(raw));
        let _ = std::fs::create_dir_all(sandbox.root());
        sandbox
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonical root (symlinks in the configured root resolved).
    pub fn canonical_root(&self) -> Result<PathBuf, SandboxError> {
        std::fs::canonicalize(&self.root)
            .map_err(|err| SandboxError::Unavailable(format!("{}: {err}", self.root.display())))
    }

    /// Path relative to the sandbox root, when `path` is inside it.
    pub fn relative(&self, path: &Path) -> Option<PathBuf> {
        if let Ok(rest) = path.strip_prefix(&self.root) {
            return Some(rest.to_path_buf());
        }
        let canonical = self.canonical_root().ok()?;
        path.strip_prefix(&canonical).ok().map(Path::to_path_buf)
    }

    /// Maps an agent-facing path into the sandbox and verifies containment.
    ///
    /// Existing path components are canonicalized (symlink-safe); components
    /// that do not exist yet cannot be symlinks, so the check stays sound for
    /// create/write targets.
    pub fn jail(&self, requested: &str) -> Result<PathBuf, SandboxError> {
        let root = self.canonical_root()?;
        let expanded;
        let requested = if requested.starts_with('~') {
            expanded = expand_tilde(requested).to_string_lossy().into_owned();
            expanded.as_str()
        } else {
            requested
        };
        let relative = relative_to_root(&root, requested)?;
        let candidate = root.join(relative);

        let existing = deepest_existing(&candidate).ok_or_else(|| {
            SandboxError::Denied(format!("cannot resolve path inside sandbox: {requested}"))
        })?;
        let resolved = std::fs::canonicalize(&existing)
            .map_err(|err| SandboxError::Denied(format!("cannot resolve {requested}: {err}")))?;
        if !resolved.starts_with(&root) {
            return Err(SandboxError::Denied(requested.to_string()));
        }
        Ok(candidate)
    }
}

/// Strips the agent-facing prefix (or an already-rooted sandbox path) and
/// rejects absolute paths and parent traversal.
fn relative_to_root(root: &Path, requested: &str) -> Result<PathBuf, SandboxError> {
    let requested_path = Path::new(requested);
    let mapped = requested
        .strip_prefix(AGENT_ROOT_PREFIX)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'));
    let relative = if let Some(rest) = mapped {
        PathBuf::from(rest.trim_start_matches('/'))
    } else if let Ok(rest) = requested_path.strip_prefix(root) {
        rest.to_path_buf()
    } else {
        return Err(SandboxError::Denied(requested.to_string()));
    };

    let mut normalized = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SandboxError::Denied(requested.to_string()));
            }
        }
    }
    Ok(normalized)
}

/// Deepest ancestor (including the candidate itself) that exists on disk.
/// `symlink_metadata` deliberately does not follow links: a dangling symlink is
/// then canonicalized to an error and denied rather than followed outside the
/// sandbox.
fn deepest_existing(candidate: &Path) -> Option<PathBuf> {
    let mut current = candidate.to_path_buf();
    loop {
        if std::fs::symlink_metadata(&current).is_ok() {
            return Some(current);
        }
        let parent = current.parent()?.to_path_buf();
        if parent == current {
            return None;
        }
        current = parent;
    }
}

fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if raw == "~" {
        return home_dir();
    }
    PathBuf::from(raw)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let unique = crate::rfc::util::now_nanos();
        let root =
            std::env::temp_dir().join(format!("aqvn-sandbox-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create root");
        root
    }

    #[test]
    fn maps_a0_prefix_only_on_a_path_boundary() {
        let root = temp_root();
        let sandbox = Sandbox::new(root.clone());
        std::fs::write(root.join("file.txt"), b"x").expect("fixture");

        assert_eq!(
            sandbox.jail("/a0/file.txt").expect("mapped"),
            sandbox.canonical_root().expect("root").join("file.txt")
        );
        assert_eq!(
            sandbox.jail("/a0").expect("root maps"),
            sandbox.canonical_root().expect("root")
        );
        // `/a0foo` is not the agent prefix and is not inside the sandbox.
        assert!(sandbox.jail("/a0foo").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn denies_parent_traversal_and_foreign_paths() {
        let root = temp_root();
        let sandbox = Sandbox::new(root.clone());
        assert!(sandbox.jail("/etc/passwd").is_err());
        assert!(sandbox.jail(&format!("{}/../etc", root.display())).is_err());
        assert!(sandbox.jail("/a0/../etc").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn denies_symlink_escape() {
        let root = temp_root();
        let sandbox = Sandbox::new(root.clone());
        std::os::unix::fs::symlink("/etc", root.join("escape")).expect("symlink");
        assert!(sandbox.jail("/a0/escape/passwd").is_err());
        // A dangling absolute link cannot be followed either.
        std::os::unix::fs::symlink("/nonexistent/target", root.join("dangling")).expect("symlink");
        assert!(sandbox.jail("/a0/dangling").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn allows_new_paths_under_the_root() {
        let root = temp_root();
        let sandbox = Sandbox::new(root.clone());
        let jailed = sandbox.jail("/a0/new/deep/file.bin").expect("allowed");
        assert!(jailed.starts_with(sandbox.canonical_root().expect("root")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn denies_tilde_paths_outside_the_sandbox() {
        let root = temp_root();
        let sandbox = Sandbox::new(root.clone());
        assert!(sandbox.jail("~/.ssh/id_rsa").is_err());
        assert!(sandbox.jail("~/").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn expands_tilde_for_the_default_root() {
        let home = home_dir();
        let sandbox = Sandbox::from_allowed_paths(&["~/Workspace".to_string()]);
        assert_eq!(sandbox.root(), home.join("Workspace"));
    }
}

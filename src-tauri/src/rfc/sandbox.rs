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
    pub fn from_allowed_paths(allowed: &[String]) -> Self {
        let raw = allowed
            .first()
            .map(String::as_str)
            .unwrap_or(DEFAULT_SANDBOX);
        Self::new(expand_tilde(raw))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Maps an agent-facing path into the sandbox and verifies containment.
    ///
    /// Existing path components are canonicalized (symlink-safe); components
    /// that do not exist yet cannot be symlinks, so the check stays sound for
    /// create/write targets.
    pub fn jail(&self, requested: &str) -> Result<PathBuf, SandboxError> {
        let root = std::fs::canonicalize(&self.root)
            .map_err(|err| SandboxError::Unavailable(format!("{}: {err}", self.root.display())))?;
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
    let relative = if let Some(rest) = requested.strip_prefix(AGENT_ROOT_PREFIX) {
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

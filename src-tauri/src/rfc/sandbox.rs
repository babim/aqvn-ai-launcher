//! Path jail: every agent-supplied path is mapped into the configured sandbox
//! root and denied when it escapes (foreign absolute paths, `..`, symlink out).
//!
//! Callers send their own absolute layout (for example `/a0/usr/workdir/x`), so
//! the `/a0` prefix maps onto the sandbox root. Nothing outside the root is
//! readable, writable or deletable.

use std::path::{Component, Path, PathBuf};

/// Default sandbox root used by the shell working directory.
pub const DEFAULT_SANDBOX: &str = "~/Workspace";

/// Agent-facing prefix that maps onto the first sandbox root.
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
    roots: Vec<PathBuf>,
}

impl Sandbox {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            roots: vec![root.into()],
        }
    }

    /// Builds a sandbox from every configured allowed path.
    ///
    /// Configured roots are created so the default `~/Workspace` works on a
    /// fresh install. An empty list denies every path: there is no implicit
    /// fallback root.
    pub fn from_allowed_paths(allowed: &[String]) -> Self {
        let roots: Vec<PathBuf> = allowed
            .iter()
            .filter(|raw| !raw.trim().is_empty())
            .map(|raw| expand_tilde(raw))
            .collect();
        for root in &roots {
            let _ = std::fs::create_dir_all(root);
        }
        Self { roots }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Canonical roots (symlinks in each configured root resolved).
    ///
    /// Roots that cannot be resolved are skipped; with no resolvable root the
    /// sandbox denies everything (`Unavailable`).
    pub fn canonical_roots(&self) -> Result<Vec<PathBuf>, SandboxError> {
        let roots: Vec<PathBuf> = self
            .roots
            .iter()
            .filter_map(|root| std::fs::canonicalize(root).ok())
            .collect();
        if roots.is_empty() {
            return Err(SandboxError::Unavailable(
                "no allowed path is configured or resolvable".to_string(),
            ));
        }
        Ok(roots)
    }

    /// Path relative to any sandbox root, when `path` is inside one.
    pub fn relative(&self, path: &Path) -> Option<PathBuf> {
        for root in &self.roots {
            if let Ok(rest) = path.strip_prefix(root) {
                return Some(rest.to_path_buf());
            }
            if let Ok(canonical) = std::fs::canonicalize(root) {
                if let Ok(rest) = path.strip_prefix(&canonical) {
                    return Some(rest.to_path_buf());
                }
            }
        }
        None
    }

    /// Maps an agent-facing path into the sandbox and verifies containment.
    ///
    /// Every root is tried in configured order, so all `allowed_paths` entries
    /// are honored. Existing path components are canonicalized (symlink-safe);
    /// components that do not exist yet cannot be symlinks, so the check stays
    /// sound for create/write targets.
    pub fn jail(&self, requested: &str) -> Result<PathBuf, SandboxError> {
        let roots = self.canonical_roots()?;
        let expanded;
        let requested = if requested.starts_with('~') {
            expanded = expand_tilde(requested).to_string_lossy().into_owned();
            expanded.as_str()
        } else {
            requested
        };
        for root in &roots {
            let Ok(relative) = relative_to_root(root, requested) else {
                continue;
            };
            if let Ok(mapped) = contained_candidate(root, &root.join(relative), requested) {
                return Ok(mapped);
            }
        }
        Err(SandboxError::Denied(requested.to_string()))
    }
}

/// Returns `candidate` when its deepest existing ancestor resolves inside
/// `root`; any resolution failure is a denial.
fn contained_candidate(
    root: &Path,
    candidate: &Path,
    requested: &str,
) -> Result<PathBuf, SandboxError> {
    let existing = deepest_existing(candidate).ok_or_else(|| {
        SandboxError::Denied(format!("cannot resolve path inside sandbox: {requested}"))
    })?;
    let resolved = std::fs::canonicalize(&existing)
        .map_err(|err| SandboxError::Denied(format!("cannot resolve {requested}: {err}")))?;
    if !resolved.starts_with(root) {
        return Err(SandboxError::Denied(requested.to_string()));
    }
    Ok(candidate.to_path_buf())
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
            sandbox.canonical_roots().expect("root")[0].join("file.txt")
        );
        assert_eq!(
            sandbox.jail("/a0").expect("root maps"),
            sandbox.canonical_roots().expect("root")[0]
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
        assert!(jailed.starts_with(&sandbox.canonical_roots().expect("root")[0]));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn denies_everything_when_allowed_paths_is_empty() {
        let sandbox = Sandbox::from_allowed_paths(&[]);
        assert!(sandbox.roots().is_empty(), "no implicit fallback root");
        assert!(sandbox.jail("/a0/anything").is_err());
        assert!(sandbox.jail("/tmp").is_err());
        assert!(sandbox.canonical_roots().is_err());
    }

    #[test]
    fn honors_every_allowed_path() {
        let first = temp_root();
        let second = temp_root();
        std::fs::write(first.join("one.txt"), b"1").expect("fixture");
        std::fs::write(second.join("two.txt"), b"2").expect("fixture");
        let sandbox = Sandbox::from_allowed_paths(&[
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ]);

        let from_first = sandbox
            .jail(&format!("{}/one.txt", first.display()))
            .expect("first root honored");
        assert_eq!(from_first, first.join("one.txt"));
        let from_second = sandbox
            .jail(&format!("{}/two.txt", second.display()))
            .expect("second root honored");
        assert_eq!(from_second, second.join("two.txt"));
        assert_eq!(
            sandbox.relative(&from_second),
            Some(PathBuf::from("two.txt"))
        );
        // `/a0` maps onto the first configured root, not the second.
        assert_eq!(
            sandbox.jail("/a0/two.txt").expect("maps to first root"),
            sandbox.canonical_roots().expect("roots")[0].join("two.txt")
        );

        let _ = std::fs::remove_dir_all(&first);
        let _ = std::fs::remove_dir_all(&second);
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
    fn expands_tilde_for_configured_roots() {
        let home = home_dir();
        let sandbox = Sandbox::from_allowed_paths(&["~/Workspace".to_string()]);
        assert_eq!(sandbox.roots(), [home.join("Workspace")]);
    }
}

//! Linux Landlock LSM and Seccomp-BPF kernel confinement for SSH sessions.
//!
//! Restricts process filesystem access to authorized directories and blocks
//! dangerous syscalls at the Linux kernel layer.

pub use aqvn_rfc::confinement::{
    default_blocked_syscalls, default_system_paths, get_landlock_abi_version,
    is_landlock_supported, ConfinementError, ConfinementProfile,
};

/// Creates a confinement profile tailored for an SSH shell session.
///
/// Grants full access to `allowed_paths` (expanding tildes and resolving canonical paths),
/// read-only access to standard system paths (`/usr`, `/lib`, `/bin`, etc.) for shell execution,
/// and applies Seccomp-BPF to filter dangerous syscalls (`ptrace`, `mount`, `reboot`, etc.).
pub fn session_confinement(allowed_paths: &[String]) -> ConfinementProfile {
    let mut profile = ConfinementProfile::new();

    for raw in allowed_paths {
        if raw.trim().is_empty() {
            continue;
        }
        let expanded = crate::session_cwd(std::slice::from_ref(raw));
        profile.add_allowed_path(expanded);
    }

    profile.with_system_paths(true);
    profile.with_seccomp(true);
    profile
}

/// Applies kernel confinement (Landlock + Seccomp) directly to the calling process.
pub fn restrict_current_process(allowed_paths: &[String]) -> Result<(), ConfinementError> {
    session_confinement(allowed_paths).apply()
}

/// Executes a closure inside an isolated child process confined by Landlock and Seccomp.
pub fn run_in_confinement<F, R>(allowed_paths: &[String], f: F) -> Result<R, ConfinementError>
where
    F: FnOnce() -> Result<R, String> + Send + 'static,
    R: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + 'static,
{
    session_confinement(allowed_paths).run_isolated(f)
}

/// Verifies that Landlock LSM and Seccomp-BPF confinement are functioning correctly.
///
/// Creates a temporary workspace, launches an isolated process under Landlock confinement,
/// confirms that files inside the allowed path can be read and written, confirms that
/// files outside the allowed path return EACCES (Permission Denied) at the syscall level,
/// and verifies that blocked syscalls return EPERM.
pub fn test_landlock_confinement() -> Result<(), ConfinementError> {
    if !is_landlock_supported() {
        return Ok(());
    }

    let temp_base = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let allowed_dir = temp_base.join(format!(
        "aqvn-landlock-allowed-{}-{}",
        std::process::id(),
        nonce
    ));
    let forbidden_dir = temp_base.join(format!(
        "aqvn-landlock-forbidden-{}-{}",
        std::process::id(),
        nonce
    ));

    std::fs::create_dir_all(&allowed_dir)?;
    std::fs::create_dir_all(&forbidden_dir)?;

    let allowed_file = allowed_dir.join("allowed.txt");
    let forbidden_file = forbidden_dir.join("forbidden.txt");

    std::fs::write(&allowed_file, b"valid workspace content")?;
    std::fs::write(&forbidden_file, b"sensitive system content")?;

    let allowed_str = allowed_dir.to_string_lossy().to_string();
    let allowed_file_clone = allowed_file.clone();
    let forbidden_file_clone = forbidden_file.clone();

    let result = run_in_confinement(&[allowed_str], move || {
        // 1. Read allowed file: MUST SUCCEED
        let content = std::fs::read_to_string(&allowed_file_clone)
            .map_err(|e| format!("Failed to read allowed file: {e}"))?;
        if content != "valid workspace content" {
            return Err("Allowed file content mismatch".to_string());
        }

        // 2. Write to allowed file: MUST SUCCEED
        std::fs::write(&allowed_file_clone, b"updated workspace content")
            .map_err(|e| format!("Failed to write allowed file: {e}"))?;

        // 3. Read forbidden file: MUST FAIL AT SYSCALL LEVEL WITH EACCES
        match std::fs::read_to_string(&forbidden_file_clone) {
            Ok(_) => {
                return Err("Kernel Landlock failed to block access to forbidden path!".to_string());
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::PermissionDenied {
                    return Err(format!("Expected PermissionDenied (EACCES), got: {e:?}"));
                }
            }
        }

        // 4. Write to forbidden file: MUST FAIL AT SYSCALL LEVEL WITH EACCES
        match std::fs::write(&forbidden_file_clone, b"exploit payload") {
            Ok(_) => {
                return Err("Kernel Landlock failed to block write to forbidden path!".to_string());
            }
            Err(e) => {
                if e.kind() != std::io::ErrorKind::PermissionDenied {
                    return Err(format!(
                        "Expected PermissionDenied (EACCES) on write, got: {e:?}"
                    ));
                }
            }
        }

        // 5. Test dangerous syscall blocked by Seccomp (ptrace)
        #[cfg(target_os = "linux")]
        unsafe {
            let ptrace_res = libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
            if ptrace_res != -1 {
                return Err("Seccomp failed to block ptrace syscall!".to_string());
            }
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EPERM) {
                return Err(format!("Expected EPERM for blocked syscall, got: {err:?}"));
            }
        }

        Ok(true)
    });

    let _ = std::fs::remove_dir_all(&allowed_dir);
    let _ = std::fs::remove_dir_all(&forbidden_dir);

    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(ConfinementError::ChildProcessError(
            "Verification returned false".to_string(),
        )),
        Err(e) => Err(e),
    }
}

//! Integration tests for Linux Landlock LSM and Seccomp-BPF confinement.
//!
//! Verifies:
//! - Kernel support detection and ABI inspection.
//! - Filesystem confinement blocking access outside allowed paths with EACCES.
//! - Read and write access preserved inside allowed paths.
//! - Seccomp-BPF blocking dangerous syscalls (ptrace) with EPERM.

use aqvn_ssh::landlock::{
    default_blocked_syscalls, default_system_paths, get_landlock_abi_version,
    is_landlock_supported, run_in_confinement, session_confinement, test_landlock_confinement,
    ConfinementProfile,
};
use std::fs;
use std::io::ErrorKind;

#[test]
fn test_landlock_abi_and_support_detection() {
    let supported = is_landlock_supported();
    let abi = get_landlock_abi_version();

    if cfg!(target_os = "linux") {
        println!("Linux host detected. Landlock supported: {supported}, ABI version: {abi}");
        if supported {
            assert!(abi >= 1, "Landlock supported must have ABI >= 1");
        }
    } else {
        assert!(!supported, "Landlock must not be reported on non-Linux");
        assert_eq!(abi, -1);
    }
}

#[test]
fn test_confinement_profile_builder() {
    let mut profile = ConfinementProfile::new();
    profile.add_allowed_path("/tmp/workspace");
    profile.add_read_only_path("/etc");
    profile.with_seccomp(true);
    profile.with_system_paths(true);

    assert_eq!(profile.allowed_paths.len(), 1);
    assert_eq!(profile.read_only_paths.len(), 1);
    assert!(profile.enforce_seccomp);
    assert!(profile.allow_system_paths);
    assert!(!profile.blocked_syscalls.is_empty());

    let sys_paths = default_system_paths();
    assert!(
        !sys_paths.is_empty(),
        "system paths should include /usr or /bin"
    );

    let blocked = default_blocked_syscalls();
    if cfg!(target_os = "linux") {
        assert!(blocked.contains(&libc::SYS_ptrace));
        assert!(blocked.contains(&libc::SYS_mount));
        assert!(blocked.contains(&libc::SYS_reboot));
    }
}

#[test]
fn test_session_confinement_helper() {
    let paths = vec!["/tmp/test_ws_1".to_string(), "/tmp/test_ws_2".to_string()];
    let profile = session_confinement(&paths);

    assert!(profile.allow_system_paths);
    assert!(profile.enforce_seccomp);
    assert_eq!(profile.allowed_paths.len(), 2);
}

#[test]
fn test_landlock_confinement_contract() {
    let result = test_landlock_confinement();
    assert!(
        result.is_ok(),
        "test_landlock_confinement contract verification failed: {result:?}"
    );
}

#[test]
fn test_landlock_blocks_unauthorized_filesystem_access() {
    if !is_landlock_supported() {
        eprintln!("Landlock LSM not supported on this host/kernel, skipping test");
        return;
    }

    let temp_dir = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let allowed_workspace = temp_dir.join(format!("aqvn-test-ws-{}-{}", std::process::id(), nonce));
    let outside_forbidden = temp_dir.join(format!(
        "aqvn-test-outside-{}-{}",
        std::process::id(),
        nonce
    ));

    fs::create_dir_all(&allowed_workspace).expect("create allowed ws");
    fs::create_dir_all(&outside_forbidden).expect("create outside dir");

    let inside_file = allowed_workspace.join("hello.txt");
    let outside_file = outside_forbidden.join("secret.txt");

    fs::write(&inside_file, b"inside workspace data").expect("write inside");
    fs::write(&outside_file, b"secret credentials").expect("write outside");

    let ws_str = allowed_workspace.to_string_lossy().to_string();
    let inside_clone = inside_file.clone();
    let outside_clone = outside_file.clone();

    let outcome = run_in_confinement(&[ws_str], move || {
        // 1. Reading authorized file inside allowed_paths succeeds
        let read_back = fs::read_to_string(&inside_clone)
            .map_err(|e| format!("cannot read inside allowed workspace: {e}"))?;
        if read_back != "inside workspace data" {
            return Err("Inside data mismatch".to_string());
        }

        // 2. Writing inside allowed_paths succeeds
        fs::write(&inside_clone, b"modified inside data")
            .map_err(|e| format!("cannot write inside allowed workspace: {e}"))?;

        // 3. Reading outside path must be DENIED at syscall layer (EACCES)
        match fs::read_to_string(&outside_clone) {
            Ok(_) => {
                return Err(
                    "CRITICAL: Landlock failed to block read on forbidden file!".to_string()
                );
            }
            Err(err) => {
                if err.kind() != ErrorKind::PermissionDenied {
                    return Err(format!("Expected PermissionDenied (EACCES), got {err:?}"));
                }
            }
        }

        // 4. Writing outside path must be DENIED at syscall layer (EACCES)
        match fs::write(&outside_clone, b"tampered") {
            Ok(_) => {
                return Err(
                    "CRITICAL: Landlock failed to block write on forbidden file!".to_string(),
                );
            }
            Err(err) => {
                if err.kind() != ErrorKind::PermissionDenied {
                    return Err(format!(
                        "Expected PermissionDenied (EACCES) on write, got {err:?}"
                    ));
                }
            }
        }

        Ok("verified".to_string())
    });

    let _ = fs::remove_dir_all(&allowed_workspace);
    let _ = fs::remove_dir_all(&outside_forbidden);

    assert!(
        outcome.is_ok(),
        "Confinement verification failed: {outcome:?}"
    );
    assert_eq!(outcome.unwrap(), "verified");
}

#[test]
fn test_seccomp_bpf_blocks_dangerous_syscalls() {
    if !cfg!(target_os = "linux") {
        return;
    }

    let temp_dir = std::env::temp_dir();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let allowed_workspace = temp_dir.join(format!(
        "aqvn-test-seccomp-{}-{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&allowed_workspace).expect("create allowed ws");
    let ws_str = allowed_workspace.to_string_lossy().to_string();

    let outcome = run_in_confinement(&[ws_str], move || {
        // Attempt blocked syscall (ptrace)
        unsafe {
            let res = libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0);
            if res != -1 {
                return Err("Seccomp failed to intercept ptrace syscall!".to_string());
            }
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EPERM) {
                return Err(format!(
                    "Expected EPERM for intercepted syscall, got: {err:?}"
                ));
            }
        }
        Ok(true)
    });

    let _ = fs::remove_dir_all(&allowed_workspace);

    assert!(outcome.is_ok(), "Seccomp filter check failed: {outcome:?}");
    assert!(outcome.unwrap());
}

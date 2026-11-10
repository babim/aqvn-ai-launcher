//! Kernel-level confinement via Linux Landlock LSM and Seccomp-BPF.
//!
//! Provides filesystem isolation at the kernel syscall layer (Landlock LSM)
//! and blocks dangerous syscalls (Seccomp-BPF) to prevent privilege escalation.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum ConfinementError {
    #[error("Landlock LSM is not supported on this kernel (ABI version: {0})")]
    Unsupported(i32),
    #[error("Failed to set PR_SET_NO_NEW_PRIVS: {0}")]
    NoNewPrivsFailed(String),
    #[error("Failed to create Landlock ruleset: {0}")]
    CreateRulesetFailed(String),
    #[error("Failed to add Landlock path rule for {path}: {error}")]
    AddRuleFailed { path: String, error: String },
    #[error("Failed to restrict self with Landlock: {0}")]
    RestrictFailed(String),
    #[error("Failed to apply Seccomp-BPF filter: {0}")]
    SeccompFailed(String),
    #[error("Child process error in isolated execution: {0}")]
    ChildProcessError(String),
    #[error("IO error: {0}")]
    Io(String),
}

impl From<std::io::Error> for ConfinementError {
    fn from(err: std::io::Error) -> Self {
        ConfinementError::Io(err.to_string())
    }
}

#[cfg(target_os = "linux")]
pub mod sys {
    pub const SYS_LANDLOCK_CREATE_RULESET: libc::c_long = 444;
    pub const SYS_LANDLOCK_ADD_RULE: libc::c_long = 445;
    pub const SYS_LANDLOCK_RESTRICT_SELF: libc::c_long = 446;

    pub const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1 << 0;
    pub const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;

    #[repr(C)]
    #[derive(Debug, Default)]
    pub struct LandlockRulesetAttr {
        pub handled_access_fs: u64,
        pub handled_access_net: u64,
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct LandlockPathBeneathAttr {
        pub allowed_access: u64,
        pub parent_fd: i32,
    }

    pub const ACCESS_FS_EXECUTE: u64 = 1 << 0;
    pub const ACCESS_FS_WRITE_FILE: u64 = 1 << 1;
    pub const ACCESS_FS_READ_FILE: u64 = 1 << 2;
    pub const ACCESS_FS_READ_DIR: u64 = 1 << 3;
    pub const ACCESS_FS_REMOVE_DIR: u64 = 1 << 4;
    pub const ACCESS_FS_REMOVE_FILE: u64 = 1 << 5;
    pub const ACCESS_FS_MAKE_CHAR: u64 = 1 << 6;
    pub const ACCESS_FS_MAKE_DIR: u64 = 1 << 7;
    pub const ACCESS_FS_MAKE_REG: u64 = 1 << 8;
    pub const ACCESS_FS_MAKE_SOCK: u64 = 1 << 9;
    pub const ACCESS_FS_MAKE_FIFO: u64 = 1 << 10;
    pub const ACCESS_FS_MAKE_BLOCK: u64 = 1 << 11;
    pub const ACCESS_FS_MAKE_SYM: u64 = 1 << 12;
    pub const ACCESS_FS_REFER: u64 = 1 << 13;
    pub const ACCESS_FS_TRUNCATE: u64 = 1 << 14;
    pub const ACCESS_FS_IOCTL_DEV: u64 = 1 << 15;
}

/// Checks whether the running Linux kernel supports Landlock LSM.
pub fn is_landlock_supported() -> bool {
    get_landlock_abi_version() > 0
}

/// Queries the Landlock ABI version supported by the running kernel.
/// Returns -1 on non-Linux platforms or when Landlock is not enabled.
#[cfg(target_os = "linux")]
pub fn get_landlock_abi_version() -> i32 {
    unsafe {
        let version = libc::syscall(
            sys::SYS_LANDLOCK_CREATE_RULESET,
            std::ptr::null::<sys::LandlockRulesetAttr>(),
            0usize,
            sys::LANDLOCK_CREATE_RULESET_VERSION,
        );
        version as i32
    }
}

#[cfg(not(target_os = "linux"))]
pub fn get_landlock_abi_version() -> i32 {
    -1
}

/// Default list of dangerous syscalls blocked by Seccomp-BPF.
pub fn default_blocked_syscalls() -> Vec<i64> {
    #[cfg(target_os = "linux")]
    {
        vec![
            libc::SYS_ptrace,
            libc::SYS_mount,
            libc::SYS_umount2,
            libc::SYS_reboot,
            libc::SYS_kexec_load,
            libc::SYS_bpf,
            libc::SYS_pivot_root,
            libc::SYS_swapon,
            libc::SYS_swapoff,
        ]
    }
    #[cfg(not(target_os = "linux"))]
    {
        vec![]
    }
}

/// Standard system directories required for reading libraries and running standard tools.
pub fn default_system_paths() -> Vec<PathBuf> {
    let candidates = ["/usr", "/lib", "/lib64", "/bin", "/etc", "/dev"];
    candidates
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

/// Confinement profile managing Landlock LSM filesystem rules and Seccomp syscall filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfinementProfile {
    pub allowed_paths: Vec<PathBuf>,
    pub read_only_paths: Vec<PathBuf>,
    pub blocked_syscalls: Vec<i64>,
    pub allow_system_paths: bool,
    pub enforce_seccomp: bool,
}

impl Default for ConfinementProfile {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfinementProfile {
    pub fn new() -> Self {
        Self {
            allowed_paths: Vec::new(),
            read_only_paths: Vec::new(),
            blocked_syscalls: default_blocked_syscalls(),
            allow_system_paths: true,
            enforce_seccomp: true,
        }
    }

    pub fn from_allowed_paths<P: AsRef<Path>>(paths: &[P]) -> Self {
        let mut profile = Self::new();
        for p in paths {
            profile.add_allowed_path(p);
        }
        profile
    }

    pub fn add_allowed_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let p = path.as_ref().to_path_buf();
        if !self.allowed_paths.contains(&p) {
            self.allowed_paths.push(p);
        }
        self
    }

    pub fn add_read_only_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let p = path.as_ref().to_path_buf();
        if !self.read_only_paths.contains(&p) {
            self.read_only_paths.push(p);
        }
        self
    }

    pub fn with_seccomp(&mut self, enable: bool) -> &mut Self {
        self.enforce_seccomp = enable;
        self
    }

    pub fn with_system_paths(&mut self, enable: bool) -> &mut Self {
        self.allow_system_paths = enable;
        self
    }

    pub fn add_blocked_syscall(&mut self, syscall: i64) -> &mut Self {
        if !self.blocked_syscalls.contains(&syscall) {
            self.blocked_syscalls.push(syscall);
        }
        self
    }

    /// Applies both Landlock LSM and Seccomp-BPF filters to the current process.
    pub fn apply(&self) -> Result<(), ConfinementError> {
        #[cfg(target_os = "linux")]
        {
            if self.enforce_seccomp {
                self.apply_seccomp()?;
            }
            if is_landlock_supported() {
                self.apply_landlock()?;
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    /// Applies Seccomp-BPF filter to block dangerous syscalls.
    #[cfg(target_os = "linux")]
    pub fn apply_seccomp(&self) -> Result<(), ConfinementError> {
        if self.blocked_syscalls.is_empty() {
            return Ok(());
        }

        unsafe {
            // PR_SET_NO_NEW_PRIVS is required before setting seccomp filter
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(ConfinementError::NoNewPrivsFailed(
                    std::io::Error::last_os_error().to_string(),
                ));
            }

            const BPF_LD: u16 = 0x00;
            const BPF_W: u16 = 0x00;
            const BPF_ABS: u16 = 0x20;
            const BPF_JMP: u16 = 0x05;
            const BPF_JEQ: u16 = 0x10;
            const BPF_K: u16 = 0x00;
            const BPF_RET: u16 = 0x06;

            const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;
            const SECCOMP_RET_ERRNO: u32 = 0x00050000;

            #[repr(C)]
            #[derive(Clone, Copy)]
            struct SockFilter {
                code: u16,
                jt: u8,
                jf: u8,
                k: u32,
            }

            #[repr(C)]
            struct SockFprog {
                len: u16,
                filter: *const SockFilter,
            }

            let n = self.blocked_syscalls.len();
            if n > 250 {
                return Err(ConfinementError::SeccompFailed(
                    "Too many blocked syscalls for BPF filter".to_string(),
                ));
            }

            let mut instructions = Vec::with_capacity(n + 3);
            // Instruction 0: load syscall nr
            instructions.push(SockFilter {
                code: BPF_LD | BPF_W | BPF_ABS,
                jt: 0,
                jf: 0,
                k: 0, // offsetof(struct seccomp_data, nr)
            });

            // Instructions 1..=n: jump to DENY if match
            for (i, &sc) in self.blocked_syscalls.iter().enumerate() {
                let jt = (n - i) as u8;
                instructions.push(SockFilter {
                    code: BPF_JMP | BPF_JEQ | BPF_K,
                    jt,
                    jf: 0,
                    k: sc as u32,
                });
            }

            // Allow all other syscalls
            instructions.push(SockFilter {
                code: BPF_RET | BPF_K,
                jt: 0,
                jf: 0,
                k: SECCOMP_RET_ALLOW,
            });

            // Deny with EPERM
            instructions.push(SockFilter {
                code: BPF_RET | BPF_K,
                jt: 0,
                jf: 0,
                k: SECCOMP_RET_ERRNO | (libc::EPERM as u32),
            });

            let prog = SockFprog {
                len: instructions.len() as u16,
                filter: instructions.as_ptr(),
            };

            if libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &prog) != 0 {
                return Err(ConfinementError::SeccompFailed(
                    std::io::Error::last_os_error().to_string(),
                ));
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn apply_seccomp(&self) -> Result<(), ConfinementError> {
        Ok(())
    }

    /// Applies Landlock LSM filesystem confinement to the current process.
    #[cfg(target_os = "linux")]
    pub fn apply_landlock(&self) -> Result<(), ConfinementError> {
        let abi = get_landlock_abi_version();
        if abi <= 0 {
            return Err(ConfinementError::Unsupported(abi));
        }

        unsafe {
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(ConfinementError::NoNewPrivsFailed(
                    std::io::Error::last_os_error().to_string(),
                ));
            }

            let mut handled_fs: u64 = 0x1fff; // ABI v1 bits (0..12)
            if abi >= 2 {
                handled_fs |= sys::ACCESS_FS_REFER;
            }
            if abi >= 3 {
                handled_fs |= sys::ACCESS_FS_TRUNCATE;
            }
            if abi >= 5 {
                handled_fs |= sys::ACCESS_FS_IOCTL_DEV;
            }

            let attr = sys::LandlockRulesetAttr {
                handled_access_fs: handled_fs,
                handled_access_net: 0,
            };

            let ruleset_fd = libc::syscall(
                sys::SYS_LANDLOCK_CREATE_RULESET,
                &attr as *const sys::LandlockRulesetAttr,
                std::mem::size_of::<sys::LandlockRulesetAttr>(),
                0u32,
            ) as i32;

            if ruleset_fd < 0 {
                return Err(ConfinementError::CreateRulesetFailed(
                    std::io::Error::last_os_error().to_string(),
                ));
            }

            // Helper to add rules for a path
            let add_rule = |path: &Path, access: u64| -> Result<(), ConfinementError> {
                let Ok(canonical) = std::fs::canonicalize(path) else {
                    return Ok(()); // Path does not exist; skip
                };
                let c_path = std::ffi::CString::new(canonical.to_string_lossy().as_bytes())
                    .map_err(|e| ConfinementError::AddRuleFailed {
                        path: canonical.display().to_string(),
                        error: e.to_string(),
                    })?;

                let mut fd = libc::open(
                    c_path.as_ptr(),
                    libc::O_PATH | libc::O_CLOEXEC | libc::O_DIRECTORY,
                );
                if fd < 0 {
                    fd = libc::open(c_path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC);
                }
                if fd < 0 {
                    return Ok(()); // Skip unopenable paths
                }

                let path_attr = sys::LandlockPathBeneathAttr {
                    allowed_access: access,
                    parent_fd: fd,
                };

                let res = libc::syscall(
                    sys::SYS_LANDLOCK_ADD_RULE,
                    ruleset_fd,
                    sys::LANDLOCK_RULE_PATH_BENEATH,
                    &path_attr as *const sys::LandlockPathBeneathAttr,
                    0u32,
                );
                libc::close(fd);

                if res < 0 {
                    return Err(ConfinementError::AddRuleFailed {
                        path: canonical.display().to_string(),
                        error: std::io::Error::last_os_error().to_string(),
                    });
                }
                Ok(())
            };

            // 1. Add read-only system paths if requested
            if self.allow_system_paths {
                let sys_access =
                    sys::ACCESS_FS_READ_FILE | sys::ACCESS_FS_READ_DIR | sys::ACCESS_FS_EXECUTE;
                for p in default_system_paths() {
                    add_rule(&p, sys_access)?;
                }
            }

            // 2. Add explicit read-only paths
            let ro_access =
                sys::ACCESS_FS_READ_FILE | sys::ACCESS_FS_READ_DIR | sys::ACCESS_FS_EXECUTE;
            for p in &self.read_only_paths {
                add_rule(p, ro_access)?;
            }

            // 3. Add full-access paths
            for p in &self.allowed_paths {
                add_rule(p, handled_fs)?;
            }

            // 4. Restrict self
            let restrict_res = libc::syscall(sys::SYS_LANDLOCK_RESTRICT_SELF, ruleset_fd, 0u32);
            libc::close(ruleset_fd);

            if restrict_res < 0 {
                return Err(ConfinementError::RestrictFailed(
                    std::io::Error::last_os_error().to_string(),
                ));
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn apply_landlock(&self) -> Result<(), ConfinementError> {
        Ok(())
    }

    /// Runs a closure inside an isolated child process confined with this profile.
    /// Returns the closure's result or an error if confinement or execution failed.
    #[cfg(target_os = "linux")]
    pub fn run_isolated<F, R>(&self, f: F) -> Result<R, ConfinementError>
    where
        F: FnOnce() -> Result<R, String> + Send + 'static,
        R: Serialize + for<'de> Deserialize<'de> + Send + 'static,
    {
        unsafe {
            let mut fds = [0i32; 2];
            if libc::pipe(fds.as_mut_ptr()) != 0 {
                return Err(ConfinementError::Io(
                    std::io::Error::last_os_error().to_string(),
                ));
            }

            let pid = libc::fork();
            if pid < 0 {
                libc::close(fds[0]);
                libc::close(fds[1]);
                return Err(ConfinementError::Io(
                    std::io::Error::last_os_error().to_string(),
                ));
            }

            if pid == 0 {
                // Child process
                libc::close(fds[0]); // Close read end

                let execute = || -> Result<R, ConfinementError> {
                    self.apply()?;
                    f().map_err(ConfinementError::ChildProcessError)
                };

                let outcome = execute();
                let encoded = serde_json::to_vec(&outcome).unwrap_or_default();
                let len = encoded.len() as u32;

                let _ = libc::write(
                    fds[1],
                    &len as *const u32 as *const libc::c_void,
                    std::mem::size_of::<u32>(),
                );
                if len > 0 {
                    let _ = libc::write(
                        fds[1],
                        encoded.as_ptr() as *const libc::c_void,
                        encoded.len(),
                    );
                }
                libc::close(fds[1]);

                let code = if outcome.is_ok() { 0 } else { 1 };
                libc::_exit(code);
            }

            // Parent process
            libc::close(fds[1]); // Close write end

            let mut len: u32 = 0;
            let mut read_bytes = 0;
            while read_bytes < std::mem::size_of::<u32>() {
                let ptr = (&mut len as *mut u32 as *mut u8).add(read_bytes);
                let r = libc::read(
                    fds[0],
                    ptr as *mut libc::c_void,
                    std::mem::size_of::<u32>() - read_bytes,
                );
                if r <= 0 {
                    break;
                }
                read_bytes += r as usize;
            }

            let mut buffer = vec![0u8; len as usize];
            let mut payload_read = 0;
            while payload_read < len as usize {
                let r = libc::read(
                    fds[0],
                    buffer[payload_read..].as_mut_ptr() as *mut libc::c_void,
                    len as usize - payload_read,
                );
                if r <= 0 {
                    break;
                }
                payload_read += r as usize;
            }
            libc::close(fds[0]);

            let mut status = 0i32;
            libc::waitpid(pid, &mut status, 0);

            if len == 0 || payload_read < len as usize {
                return Err(ConfinementError::ChildProcessError(format!(
                    "Child process terminated unexpectedly (exit status: {status})"
                )));
            }

            let outcome: Result<R, ConfinementError> = serde_json::from_slice(&buffer)
                .map_err(|e| ConfinementError::ChildProcessError(e.to_string()))?;
            outcome
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn run_isolated<F, R>(&self, f: F) -> Result<R, ConfinementError>
    where
        F: FnOnce() -> Result<R, String> + Send + 'static,
        R: Serialize + for<'de> Deserialize<'de> + Send + 'static,
    {
        f().map_err(ConfinementError::ChildProcessError)
    }
}

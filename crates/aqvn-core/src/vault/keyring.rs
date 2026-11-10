//! OS Native Keyring Vault (Secret Service / Keychain / Credential Manager / Kernel Keyring)
//! layered with ChaCha20-Poly1305 authenticated envelope encryption and Zeroize.

use super::envelope::{
    derive_key_argon2id, derive_machine_vault_key, EnvelopeCipher, KdfParams, VaultError, SALT_LEN,
};
use crate::secret::{SecretKey, SecretString, TokenCredentials};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeroize::Zeroize;

/// The active OS or local storage backend used for raw encrypted envelope persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyringBackendKind {
    LinuxSecretService,
    LinuxKernelKeyring,
    MacOsKeychain,
    WindowsCredentialManager,
    EncryptedEnvelopeFile,
    InMemory,
}

/// Abstract contract for raw byte persistence into an OS-native or local vault backend.
pub trait KeyringBackend: Send + Sync {
    fn kind(&self) -> KeyringBackendKind;
    fn is_available(&self) -> bool;
    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError>;
    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError>;
    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError>;
}

// ============================================================================
// In-Memory Backend (for testing and ephemeral sessions)
// ============================================================================

#[derive(Default)]
pub struct InMemoryBackend {
    store: Mutex<HashMap<(String, String), Vec<u8>>>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyringBackend for InMemoryBackend {
    fn kind(&self) -> KeyringBackendKind {
        KeyringBackendKind::InMemory
    }

    fn is_available(&self) -> bool {
        true
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        let mut map = self
            .store
            .lock()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        map.insert((service.to_string(), account.to_string()), data.to_vec());
        Ok(())
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let map = self
            .store
            .lock()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        Ok(map
            .get(&(service.to_string(), account.to_string()))
            .cloned())
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let mut map = self
            .store
            .lock()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        if let Some(mut old) = map.remove(&(service.to_string(), account.to_string())) {
            old.zeroize();
        }
        Ok(())
    }
}

impl Drop for InMemoryBackend {
    fn drop(&mut self) {
        if let Ok(mut map) = self.store.lock() {
            for (_, mut val) in map.drain() {
                val.zeroize();
            }
        }
    }
}

// ============================================================================
// Encrypted File Backend (cross-platform, 0600 permissions, envelope encrypted)
// ============================================================================

pub struct EncryptedFileBackend {
    path: PathBuf,
    lock: Mutex<()>,
}

impl EncryptedFileBackend {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.path
    }

    fn read_map(&self) -> Result<HashMap<String, String>, VaultError> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let raw = std::fs::read_to_string(&self.path)
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        serde_json::from_str(&raw).map_err(|e| VaultError::Serialization(e.to_string()))
    }

    fn write_map(&self, map: &HashMap<String, String>) -> Result<(), VaultError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| VaultError::BackendIo(e.to_string()))?;
        }
        let json = serde_json::to_string_pretty(map)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;

        let temp_path = self
            .path
            .with_extension(format!("tmp.{}", std::process::id()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp_path)
                .map_err(|e| VaultError::BackendIo(e.to_string()))?;
            use std::io::Write;
            file.write_all(json.as_bytes())
                .map_err(|e| VaultError::BackendIo(e.to_string()))?;
            file.sync_all()
                .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(&temp_path, json.as_bytes())
                .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        }

        std::fs::rename(&temp_path, &self.path)
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        Ok(())
    }
}

impl KeyringBackend for EncryptedFileBackend {
    fn kind(&self) -> KeyringBackendKind {
        KeyringBackendKind::EncryptedEnvelopeFile
    }

    fn is_available(&self) -> bool {
        true
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        let mut map = self.read_map()?;
        let key = format!("{service}:{account}");
        map.insert(key, BASE64.encode(data));
        self.write_map(&map)
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        let map = self.read_map()?;
        let key = format!("{service}:{account}");
        match map.get(&key) {
            Some(encoded) => {
                let bytes = BASE64
                    .decode(encoded)
                    .map_err(|e| VaultError::Serialization(e.to_string()))?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let _guard = self
            .lock
            .lock()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        let mut map = self.read_map()?;
        let key = format!("{service}:{account}");
        if map.remove(&key).is_some() {
            self.write_map(&map)?;
        }
        Ok(())
    }
}

// ============================================================================
// Linux Kernel Keyring Backend (Linux Key Retention System: SYS_add_key)
// ============================================================================

#[cfg(target_os = "linux")]
pub struct LinuxKernelKeyringBackend {
    keyring_id: libc::c_int,
}

#[cfg(target_os = "linux")]
impl LinuxKernelKeyringBackend {
    pub const KEY_SPEC_USER_KEYRING: libc::c_int = -4;
    pub const KEY_SPEC_SESSION_KEYRING: libc::c_int = -3;
    const KEYCTL_READ: libc::c_ulong = 11;
    const KEYCTL_UNLINK: libc::c_ulong = 9;
    const KEYCTL_REVOKE: libc::c_ulong = 3;

    pub fn new(keyring_id: libc::c_int) -> Self {
        Self { keyring_id }
    }

    pub fn user_keyring() -> Self {
        Self::new(Self::KEY_SPEC_USER_KEYRING)
    }

    pub fn session_keyring() -> Self {
        Self::new(Self::KEY_SPEC_SESSION_KEYRING)
    }

    fn key_description(service: &str, account: &str) -> String {
        format!("aqvn:{service}:{account}")
    }
}

#[cfg(target_os = "linux")]
impl KeyringBackend for LinuxKernelKeyringBackend {
    fn kind(&self) -> KeyringBackendKind {
        KeyringBackendKind::LinuxKernelKeyring
    }

    fn is_available(&self) -> bool {
        // Probe whether SYS_add_key is accessible in current cgroup / namespace
        let desc = "aqvn:probe:test\0";
        let res = unsafe {
            libc::syscall(
                libc::SYS_add_key,
                c"user".as_ptr(),
                desc.as_ptr() as *const libc::c_char,
                c"probe".as_ptr() as *const libc::c_void,
                5usize,
                self.keyring_id,
            )
        };
        if res > 0 {
            // Revoke probe key immediately
            unsafe {
                libc::syscall(
                    libc::SYS_keyctl,
                    Self::KEYCTL_UNLINK,
                    res as libc::c_int,
                    self.keyring_id,
                );
                libc::syscall(libc::SYS_keyctl, Self::KEYCTL_REVOKE, res as libc::c_int);
            }
            true
        } else {
            false
        }
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        let desc = format!("{}\0", Self::key_description(service, account));
        let key_serial = unsafe {
            libc::syscall(
                libc::SYS_add_key,
                c"user".as_ptr(),
                desc.as_ptr() as *const libc::c_char,
                data.as_ptr() as *const libc::c_void,
                data.len(),
                self.keyring_id,
            )
        };
        if key_serial <= 0 {
            let err = std::io::Error::last_os_error();
            return Err(VaultError::BackendIo(format!(
                "Linux kernel SYS_add_key failed for {desc}: {err}"
            )));
        }
        Ok(())
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let desc = format!("{}\0", Self::key_description(service, account));
        let key_serial = unsafe {
            libc::syscall(
                libc::SYS_request_key,
                c"user".as_ptr(),
                desc.as_ptr() as *const libc::c_char,
                std::ptr::null::<libc::c_char>(),
                self.keyring_id,
            )
        };
        if key_serial <= 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ENOKEY)
                || err.raw_os_error() == Some(libc::EKEYEXPIRED)
            {
                return Ok(None);
            }
            return Err(VaultError::BackendIo(format!(
                "Linux kernel SYS_request_key failed for {desc}: {err}"
            )));
        }

        // Query required buffer size
        let size = unsafe {
            libc::syscall(
                libc::SYS_keyctl,
                Self::KEYCTL_READ,
                key_serial as libc::c_int,
                std::ptr::null_mut::<libc::c_char>(),
                0usize,
            )
        };
        if size <= 0 {
            return Ok(None);
        }

        let mut buf = vec![0u8; size as usize];
        let read_bytes = unsafe {
            libc::syscall(
                libc::SYS_keyctl,
                Self::KEYCTL_READ,
                key_serial as libc::c_int,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        if read_bytes < 0 {
            buf.zeroize();
            let err = std::io::Error::last_os_error();
            return Err(VaultError::BackendIo(format!(
                "Linux kernel KEYCTL_READ failed for key {key_serial}: {err}"
            )));
        }
        buf.truncate(read_bytes as usize);
        Ok(Some(buf))
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let desc = format!("{}\0", Self::key_description(service, account));
        let key_serial = unsafe {
            libc::syscall(
                libc::SYS_request_key,
                c"user".as_ptr(),
                desc.as_ptr() as *const libc::c_char,
                std::ptr::null::<libc::c_char>(),
                self.keyring_id,
            )
        };
        if key_serial > 0 {
            unsafe {
                libc::syscall(
                    libc::SYS_keyctl,
                    Self::KEYCTL_UNLINK,
                    key_serial as libc::c_int,
                    self.keyring_id,
                );
                libc::syscall(
                    libc::SYS_keyctl,
                    Self::KEYCTL_REVOKE,
                    key_serial as libc::c_int,
                );
            }
        }
        Ok(())
    }
}

// ============================================================================
// Linux Secret Service Backend (org.freedesktop.secrets via secret-tool)
// ============================================================================

#[cfg(target_os = "linux")]
pub struct LinuxSecretServiceBackend;

#[cfg(target_os = "linux")]
impl LinuxSecretServiceBackend {
    fn has_dbus_secrets_service() -> bool {
        // Probe D-Bus NameHasOwner for org.freedesktop.secrets
        let output = std::process::Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.freedesktop.DBus",
                "--type=method_call",
                "--print-reply",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus.NameHasOwner",
                "string:org.freedesktop.secrets",
            ])
            .output();

        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            s.contains("boolean true")
        } else {
            false
        }
    }

    fn has_secret_tool() -> bool {
        std::process::Command::new("which")
            .arg("secret-tool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(target_os = "linux")]
impl KeyringBackend for LinuxSecretServiceBackend {
    fn kind(&self) -> KeyringBackendKind {
        KeyringBackendKind::LinuxSecretService
    }

    fn is_available(&self) -> bool {
        Self::has_secret_tool() && Self::has_dbus_secrets_service()
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        let b64 = BASE64.encode(data);
        let mut child = std::process::Command::new("secret-tool")
            .args([
                "store",
                "--label=AQVN Workspace Secret",
                "service",
                service,
                "account",
                account,
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin
                .write_all(b64.as_bytes())
                .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        }
        let status = child
            .wait()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        if !status.success() {
            return Err(VaultError::BackendIo(
                "secret-tool store returned non-zero".into(),
            ));
        }
        Ok(())
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let output = std::process::Command::new("secret-tool")
            .args(["lookup", "service", service, "account", account])
            .output()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;

        if !output.status.success() || output.stdout.is_empty() {
            return Ok(None);
        }
        let trimmed = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let bytes = BASE64
            .decode(&trimmed)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        Ok(Some(bytes))
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let _ = std::process::Command::new("secret-tool")
            .args(["clear", "service", service, "account", account])
            .output();
        Ok(())
    }
}

// ============================================================================
// macOS Keychain Backend (Apple Keychain Services via /usr/bin/security)
// ============================================================================

#[cfg(target_os = "macos")]
pub struct MacOsKeychainBackend;

#[cfg(target_os = "macos")]
impl KeyringBackend for MacOsKeychainBackend {
    fn kind(&self) -> KeyringBackendKind {
        KeyringBackendKind::MacOsKeychain
    }

    fn is_available(&self) -> bool {
        std::path::Path::new("/usr/bin/security").exists()
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        let b64 = BASE64.encode(data);
        let status = std::process::Command::new("/usr/bin/security")
            .args([
                "add-generic-password",
                "-U",
                "-s",
                service,
                "-a",
                account,
                "-w",
                &b64,
            ])
            .status()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        if !status.success() {
            return Err(VaultError::BackendIo(
                "security add-generic-password failed".into(),
            ));
        }
        Ok(())
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let output = std::process::Command::new("/usr/bin/security")
            .args(["find-generic-password", "-s", service, "-a", account, "-w"])
            .output()
            .map_err(|e| VaultError::BackendIo(e.to_string()))?;
        if !output.status.success() || output.stdout.is_empty() {
            return Ok(None);
        }
        let trimmed = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let bytes = BASE64
            .decode(&trimmed)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        Ok(Some(bytes))
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let _ = std::process::Command::new("/usr/bin/security")
            .args(["delete-generic-password", "-s", service, "-a", account])
            .output();
        Ok(())
    }
}

// ============================================================================
// Windows Credential Manager Backend (advapi32.dll Win32 Credential API)
// ============================================================================

#[cfg(target_os = "windows")]
pub struct WindowsCredentialManagerBackend;

#[cfg(target_os = "windows")]
#[repr(C)]
struct CREDENTIALW {
    flags: u32,
    cred_type: u32,
    target_name: *mut u16,
    comment: *mut u16,
    last_written: [u32; 2],
    credential_blob_size: u32,
    credential_blob: *mut u8,
    persist: u32,
    attribute_count: u32,
    attributes: *mut std::ffi::c_void,
    target_alias: *mut u16,
    user_name: *mut u16,
}

#[cfg(target_os = "windows")]
extern "system" {
    fn CredWriteW(credential: *const CREDENTIALW, flags: u32) -> libc::c_int;
    fn CredReadW(
        target_name: *const u16,
        cred_type: u32,
        flags: u32,
        credential: *mut *mut CREDENTIALW,
    ) -> libc::c_int;
    fn CredDeleteW(target_name: *const u16, cred_type: u32, flags: u32) -> libc::c_int;
    fn CredFree(buffer: *mut std::ffi::c_void);
}

#[cfg(target_os = "windows")]
impl KeyringBackend for WindowsCredentialManagerBackend {
    fn kind(&self) -> KeyringBackendKind {
        KeyringBackendKind::WindowsCredentialManager
    }

    fn is_available(&self) -> bool {
        true
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        use std::os::windows::ffi::OsStrExt;
        let target: Vec<u16> = std::ffi::OsStr::new(&format!("{service}:{account}"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let user: Vec<u16> = std::ffi::OsStr::new(account)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut blob = data.to_vec();
        let cred = CREDENTIALW {
            flags: 0,
            cred_type: 1, // CRED_TYPE_GENERIC
            target_name: target.as_ptr() as *mut u16,
            comment: std::ptr::null_mut(),
            last_written: [0, 0],
            credential_blob_size: blob.len() as u32,
            credential_blob: blob.as_mut_ptr(),
            persist: 2, // CRED_PERSIST_LOCAL_MACHINE
            attribute_count: 0,
            attributes: std::ptr::null_mut(),
            target_alias: std::ptr::null_mut(),
            user_name: user.as_ptr() as *mut u16,
        };
        let ok = unsafe { CredWriteW(&cred, 0) };
        blob.zeroize();
        if ok == 0 {
            return Err(VaultError::BackendIo("Windows CredWriteW failed".into()));
        }
        Ok(())
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        use std::os::windows::ffi::OsStrExt;
        let target: Vec<u16> = std::ffi::OsStr::new(&format!("{service}:{account}"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();
        let ok = unsafe { CredReadW(target.as_ptr(), 1, 0, &mut cred_ptr) };
        if ok == 0 || cred_ptr.is_null() {
            return Ok(None);
        }

        let raw_slice = unsafe {
            std::slice::from_raw_parts(
                (*cred_ptr).credential_blob,
                (*cred_ptr).credential_blob_size as usize,
            )
        };
        let out = raw_slice.to_vec();
        unsafe { CredFree(cred_ptr as *mut std::ffi::c_void) };
        Ok(Some(out))
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        use std::os::windows::ffi::OsStrExt;
        let target: Vec<u16> = std::ffi::OsStr::new(&format!("{service}:{account}"))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe { CredDeleteW(target.as_ptr(), 1, 0) };
        Ok(())
    }
}

// ============================================================================
// Hybrid Keyring Backend (mirrors to Linux Kernel Keyring + Encrypted File)
// ============================================================================

pub struct HybridKeyringBackend {
    primary: Box<dyn KeyringBackend>,
    secondary: Box<dyn KeyringBackend>,
}

impl HybridKeyringBackend {
    pub fn new(primary: Box<dyn KeyringBackend>, secondary: Box<dyn KeyringBackend>) -> Self {
        Self { primary, secondary }
    }
}

impl KeyringBackend for HybridKeyringBackend {
    fn kind(&self) -> KeyringBackendKind {
        self.primary.kind()
    }

    fn is_available(&self) -> bool {
        self.primary.is_available() || self.secondary.is_available()
    }

    fn store_raw(&self, service: &str, account: &str, data: &[u8]) -> Result<(), VaultError> {
        let _ = self.primary.store_raw(service, account, data);
        self.secondary.store_raw(service, account, data)
    }

    fn load_raw(&self, service: &str, account: &str) -> Result<Option<Vec<u8>>, VaultError> {
        if let Ok(Some(bytes)) = self.primary.load_raw(service, account) {
            return Ok(Some(bytes));
        }
        self.secondary.load_raw(service, account)
    }

    fn delete_raw(&self, service: &str, account: &str) -> Result<(), VaultError> {
        let _ = self.primary.delete_raw(service, account);
        self.secondary.delete_raw(service, account)
    }
}

// ============================================================================
// KeyringVault: High-level OS Keyring Vault with Envelope Encryption & Zeroize
// ============================================================================

pub struct KeyringVault {
    service: String,
    backend: Box<dyn KeyringBackend>,
    master_key: SecretKey,
}

impl KeyringVault {
    /// Opens the best available OS Native Keyring vault.
    ///
    /// - Linux Desktop: Secret Service (`org.freedesktop.secrets`).
    /// - Linux Headless / Container: Linux Kernel Keyring (`SYS_add_key`), mirrored with `vault.enc` if `vault_dir` is provided.
    /// - macOS: Apple Keychain Services.
    /// - Windows: Windows Credential Manager.
    pub fn open_native(
        service: impl Into<String>,
        vault_dir: Option<PathBuf>,
    ) -> Result<Self, VaultError> {
        let service = service.into();
        let salt = [0x5au8; SALT_LEN];
        let master_key = derive_machine_vault_key(None, &salt, KdfParams::fast())?;

        let file_backend = vault_dir.map(|dir| {
            Box::new(EncryptedFileBackend::new(dir.join("vault.enc"))) as Box<dyn KeyringBackend>
        });

        #[cfg(target_os = "linux")]
        {
            let secret_service = LinuxSecretServiceBackend;
            if secret_service.is_available() {
                return Ok(Self::with_backend(
                    service,
                    Box::new(secret_service),
                    master_key,
                ));
            }

            let kernel_backend = LinuxKernelKeyringBackend::user_keyring();
            if kernel_backend.is_available() {
                if let Some(fb) = file_backend {
                    let hybrid = HybridKeyringBackend::new(Box::new(kernel_backend), fb);
                    return Ok(Self::with_backend(service, Box::new(hybrid), master_key));
                }
                return Ok(Self::with_backend(
                    service,
                    Box::new(kernel_backend),
                    master_key,
                ));
            }
        }

        #[cfg(target_os = "macos")]
        {
            let keychain = MacOsKeychainBackend;
            if keychain.is_available() {
                return Ok(Self::with_backend(service, Box::new(keychain), master_key));
            }
        }

        #[cfg(target_os = "windows")]
        {
            let cred_mgr = WindowsCredentialManagerBackend;
            if cred_mgr.is_available() {
                return Ok(Self::with_backend(service, Box::new(cred_mgr), master_key));
            }
        }

        if let Some(fb) = file_backend {
            Ok(Self::with_backend(service, fb, master_key))
        } else {
            // Ephemeral in-memory fallback
            Ok(Self::with_backend(
                service,
                Box::new(InMemoryBackend::new()),
                master_key,
            ))
        }
    }

    /// Opens an encrypted file vault using an explicit master passphrase.
    pub fn open_encrypted_file(
        service: impl Into<String>,
        file_path: PathBuf,
        passphrase: &SecretString,
    ) -> Result<Self, VaultError> {
        let salt = [0x42u8; SALT_LEN];
        let master_key = derive_key_argon2id(passphrase.as_bytes(), &salt, KdfParams::fast())?;
        let backend = Box::new(EncryptedFileBackend::new(file_path));
        Ok(Self::with_backend(service, backend, master_key))
    }

    /// In-memory vault with random master key for isolated tests.
    pub fn in_memory(service: impl Into<String>) -> Self {
        let master_key =
            SecretKey::generate_random().unwrap_or_else(|_| SecretKey::from_bytes([0x77u8; 32]));
        Self::with_backend(service, Box::new(InMemoryBackend::new()), master_key)
    }

    /// Constructs vault with a specific backend and master key.
    pub fn with_backend(
        service: impl Into<String>,
        backend: Box<dyn KeyringBackend>,
        master_key: SecretKey,
    ) -> Self {
        Self {
            service: service.into(),
            backend,
            master_key,
        }
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn backend_kind(&self) -> KeyringBackendKind {
        self.backend.kind()
    }

    fn aad(&self, account: &str) -> Vec<u8> {
        format!("{}:{}", self.service, account).into_bytes()
    }

    /// Encrypts `secret` with ChaCha20-Poly1305 and stores it in the keyring backend.
    pub fn set_secret(&self, account: &str, secret: &SecretString) -> Result<(), VaultError> {
        let aad = self.aad(account);
        let sealed = EnvelopeCipher::seal_with_key(&self.master_key, &aad, secret.as_bytes())?;
        self.backend.store_raw(&self.service, account, &sealed)
    }

    /// Loads encrypted envelope from keyring backend, authenticates, and decrypts into `SecretString`.
    pub fn get_secret(&self, account: &str) -> Result<Option<SecretString>, VaultError> {
        let aad = self.aad(account);
        let raw = self.backend.load_raw(&self.service, account)?;
        let Some(sealed) = raw else {
            return Ok(None);
        };
        let decrypted = EnvelopeCipher::open_with_key(&self.master_key, &aad, &sealed)?;
        let s = String::from_utf8(decrypted.expose_bytes().to_vec())
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        Ok(Some(SecretString::new(s)))
    }

    /// Removes a secret from the vault.
    pub fn delete_secret(&self, account: &str) -> Result<(), VaultError> {
        self.backend.delete_raw(&self.service, account)
    }

    /// Stores RFC password securely in vault.
    pub fn store_rfc_password(&self, password: &SecretString) -> Result<(), VaultError> {
        self.set_secret("rfc_password", password)
    }

    /// Loads RFC password securely from vault.
    pub fn load_rfc_password(&self) -> Result<Option<SecretString>, VaultError> {
        self.get_secret("rfc_password")
    }

    /// Stores SSH password securely in vault.
    pub fn store_ssh_password(&self, password: &SecretString) -> Result<(), VaultError> {
        self.set_secret("ssh_password", password)
    }

    /// Loads SSH password securely from vault.
    pub fn load_ssh_password(&self) -> Result<Option<SecretString>, VaultError> {
        self.get_secret("ssh_password")
    }

    /// Encrypts and stores full token credentials in vault.
    pub fn store_credentials(&self, creds: &TokenCredentials) -> Result<(), VaultError> {
        let secret_json = creds
            .to_secret_json()
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        let aad = self.aad(&creds.account);
        let sealed =
            EnvelopeCipher::seal_with_key(&self.master_key, &aad, secret_json.expose_bytes())?;
        self.backend
            .store_raw(&self.service, &creds.account, &sealed)
    }

    /// Loads and decrypts full token credentials from vault.
    pub fn load_credentials(&self, account: &str) -> Result<Option<TokenCredentials>, VaultError> {
        let aad = self.aad(account);
        let raw = self.backend.load_raw(&self.service, account)?;
        let Some(sealed) = raw else {
            return Ok(None);
        };
        let decrypted = EnvelopeCipher::open_with_key(&self.master_key, &aad, &sealed)?;
        let creds = TokenCredentials::from_secret_json(&decrypted)
            .map_err(|e| VaultError::Serialization(e.to_string()))?;
        Ok(Some(creds))
    }
}

/// Verification helper executed by `cargo test -p aqvn-core test_keyring_vault`.
pub fn test_keyring_vault() -> Result<(), VaultError> {
    let vault = KeyringVault::in_memory("aqvn-test-verification");
    let account = "test_user";
    let original = SecretString::new("super-secret-token-12345");

    vault.set_secret(account, &original)?;
    let retrieved = vault.get_secret(account)?.ok_or_else(|| {
        VaultError::BackendIo("KeyringVault failed to retrieve saved secret".to_string())
    })?;

    if !original.constant_time_eq(&retrieved) {
        return Err(VaultError::AuthenticationFailed);
    }

    vault.delete_secret(account)?;
    if vault.get_secret(account)?.is_some() {
        return Err(VaultError::BackendIo("Secret was not deleted".to_string()));
    }

    Ok(())
}

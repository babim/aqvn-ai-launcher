//! OS Native Keyring Vault and ChaCha20-Poly1305 Envelope Encryption.

pub mod envelope;
pub mod keyring;

pub use envelope::{
    derive_key_argon2id, derive_machine_vault_key, machine_identity_material, EnvelopeCipher,
    KdfParams, VaultError, ENVELOPE_MAGIC, SALT_LEN,
};
pub use keyring::{
    test_keyring_vault, EncryptedFileBackend, InMemoryBackend, KeyringBackend, KeyringBackendKind,
    KeyringVault,
};

#[cfg(target_os = "linux")]
pub use keyring::{LinuxKernelKeyringBackend, LinuxSecretServiceBackend};

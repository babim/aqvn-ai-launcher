//! ChaCha20-Poly1305 authenticated envelope encryption with Argon2id key derivation.

use crate::secret::{SecretBytes, SecretKey, SecretString};
use argon2::{Algorithm, Argon2, Params, Version};
use aws_lc_rs::aead::{Aad, LessSafeKey, Nonce, UnboundKey, CHACHA20_POLY1305, NONCE_LEN};
use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use thiserror::Error;
use zeroize::Zeroize;

pub const ENVELOPE_MAGIC: &[u8; 5] = b"AQVN1";
pub const SALT_LEN: usize = 16;
pub const HEADER_LEN: usize = ENVELOPE_MAGIC.len() + SALT_LEN; // 21 bytes
pub const MIN_ENVELOPE_LEN: usize = HEADER_LEN + NONCE_LEN + 16; // + 16-byte Poly1305 tag

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("decryption authentication failed (wrong key, tampered data, or AAD mismatch)")]
    AuthenticationFailed,
    #[error("invalid encrypted envelope format: {0}")]
    InvalidEnvelope(String),
    #[error("OS keyring backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("keyring backend I/O error: {0}")]
    BackendIo(String),
    #[error("credential serialization error: {0}")]
    Serialization(String),
    #[error("invalid vault argument: {0}")]
    InvalidInput(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl KdfParams {
    /// OWASP-aligned Argon2id default (19 MiB, 2 iterations, 1 lane).
    pub const fn standard() -> Self {
        Self {
            m_cost_kib: 19_456,
            t_cost: 2,
            p_cost: 1,
        }
    }

    /// Lighter Argon2id profile for low-memory / high-frequency tests (4 MiB, 1 iteration).
    pub const fn fast() -> Self {
        Self {
            m_cost_kib: 4_096,
            t_cost: 1,
            p_cost: 1,
        }
    }
}

impl Default for KdfParams {
    fn default() -> Self {
        Self::standard()
    }
}

/// Derives a 256-bit `SecretKey` from `passphrase` and `salt` using Argon2id.
pub fn derive_key_argon2id(
    passphrase: &[u8],
    salt: &[u8],
    params: KdfParams,
) -> Result<SecretKey, VaultError> {
    if salt.len() < 8 {
        return Err(VaultError::InvalidInput(
            "Argon2id salt must be at least 8 bytes".to_string(),
        ));
    }
    let argon2_params = Params::new(params.m_cost_kib, params.t_cost, params.p_cost, Some(32))
        .map_err(|e| VaultError::KeyDerivationFailed(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params);
    let mut raw_key = [0u8; 32];
    argon2
        .hash_password_into(passphrase, salt, &mut raw_key)
        .map_err(|e| VaultError::KeyDerivationFailed(e.to_string()))?;
    let key = SecretKey::from_bytes(raw_key);
    raw_key.zeroize();
    Ok(key)
}

/// Collects machine-bound entropy seed (zeroized on drop) for headless master key derivation.
pub fn machine_identity_material() -> SecretBytes {
    let mut buf = Vec::with_capacity(128);
    buf.extend_from_slice(b"aqvn-vault-machine-seed-v1:");
    buf.extend_from_slice(crate::config::deterministic_gateway_id().as_bytes());
    for path in &["/etc/machine-id", "/var/lib/dbus/machine-id"] {
        if let Ok(mut id) = std::fs::read_to_string(path) {
            let trimmed = id.trim();
            if !trimmed.is_empty() {
                buf.push(b':');
                buf.extend_from_slice(trimmed.as_bytes());
            }
            id.zeroize();
        }
    }
    SecretBytes::new(buf)
}

/// Derives a persistent vault key bound to the local machine identity + optional caller secret.
pub fn derive_machine_vault_key(
    extra_secret: Option<&SecretString>,
    salt: &[u8; SALT_LEN],
    params: KdfParams,
) -> Result<SecretKey, VaultError> {
    let machine = machine_identity_material();
    let mut combined = Vec::with_capacity(machine.len() + 64);
    combined.extend_from_slice(machine.expose_bytes());
    if let Some(secret) = extra_secret {
        combined.push(b':');
        combined.extend_from_slice(secret.as_bytes());
    }
    let res = derive_key_argon2id(&combined, salt, params);
    combined.zeroize();
    res
}

fn build_effective_aad(aad: &[u8], header: &[u8; HEADER_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(aad.len() + 5 + HEADER_LEN);
    out.extend_from_slice(aad);
    out.extend_from_slice(b"|hdr:");
    out.extend_from_slice(header);
    out
}

/// ChaCha20-Poly1305 AEAD envelope cipher.
pub struct EnvelopeCipher;

impl EnvelopeCipher {
    /// Seals `plaintext` using `key` and authenticated `aad` with ChaCha20-Poly1305.
    pub fn seal_with_key(
        key: &SecretKey,
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        Self::seal_with_key_and_salt(key, &[0u8; SALT_LEN], aad, plaintext)
    }

    fn seal_with_key_and_salt(
        key: &SecretKey,
        salt: &[u8; SALT_LEN],
        aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, VaultError> {
        let mut header = [0u8; HEADER_LEN];
        header[..ENVELOPE_MAGIC.len()].copy_from_slice(ENVELOPE_MAGIC);
        header[ENVELOPE_MAGIC.len()..].copy_from_slice(salt);

        let effective_aad = build_effective_aad(aad, &header);
        let unbound_key = UnboundKey::new(&CHACHA20_POLY1305, key.expose_bytes())
            .map_err(|_| VaultError::EncryptionFailed("invalid ChaCha20-Poly1305 key".into()))?;
        let less_safe_key = LessSafeKey::new(unbound_key);

        let mut nonce_bytes = [0u8; NONCE_LEN];
        SystemRandom::new()
            .fill(&mut nonce_bytes)
            .map_err(|_| VaultError::EncryptionFailed("CSPRNG nonce generation failed".into()))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let mut ct_and_tag = Vec::with_capacity(plaintext.len() + 16);
        ct_and_tag.extend_from_slice(plaintext);

        if less_safe_key
            .seal_in_place_append_tag(nonce, Aad::from(&effective_aad), &mut ct_and_tag)
            .is_err()
        {
            ct_and_tag.zeroize();
            return Err(VaultError::EncryptionFailed(
                "ChaCha20-Poly1305 seal failed".into(),
            ));
        }

        let mut out = Vec::with_capacity(HEADER_LEN + NONCE_LEN + ct_and_tag.len());
        out.extend_from_slice(&header);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ct_and_tag);
        Ok(out)
    }

    /// Opens and authenticates a ChaCha20-Poly1305 envelope encrypted with `key`.
    pub fn open_with_key(
        key: &SecretKey,
        aad: &[u8],
        envelope: &[u8],
    ) -> Result<SecretBytes, VaultError> {
        if envelope.len() < MIN_ENVELOPE_LEN {
            return Err(VaultError::InvalidEnvelope(format!(
                "envelope too short ({} bytes < {MIN_ENVELOPE_LEN})",
                envelope.len()
            )));
        }
        if &envelope[..ENVELOPE_MAGIC.len()] != ENVELOPE_MAGIC {
            return Err(VaultError::InvalidEnvelope(
                "unrecognized envelope magic header".to_string(),
            ));
        }

        let mut header = [0u8; HEADER_LEN];
        header.copy_from_slice(&envelope[..HEADER_LEN]);
        let effective_aad = build_effective_aad(aad, &header);

        let nonce_start = HEADER_LEN;
        let nonce_end = nonce_start + NONCE_LEN;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        nonce_bytes.copy_from_slice(&envelope[nonce_start..nonce_end]);
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        let unbound_key = UnboundKey::new(&CHACHA20_POLY1305, key.expose_bytes())
            .map_err(|_| VaultError::AuthenticationFailed)?;
        let less_safe_key = LessSafeKey::new(unbound_key);

        let mut buf = envelope[nonce_end..].to_vec();
        let res = match less_safe_key.open_in_place(nonce, Aad::from(&effective_aad), &mut buf) {
            Ok(pt) => Ok(SecretBytes::from_slice(pt)),
            Err(_) => Err(VaultError::AuthenticationFailed),
        };
        buf.zeroize();
        res
    }

    /// Derives a fresh Argon2id key from `passphrase` and random salt, then seals with ChaCha20-Poly1305.
    pub fn seal_with_passphrase(
        passphrase: &SecretString,
        aad: &[u8],
        plaintext: &[u8],
        params: KdfParams,
    ) -> Result<Vec<u8>, VaultError> {
        let mut salt = [0u8; SALT_LEN];
        SystemRandom::new()
            .fill(&mut salt)
            .map_err(|_| VaultError::EncryptionFailed("CSPRNG salt generation failed".into()))?;
        let key = derive_key_argon2id(passphrase.as_bytes(), &salt, params)?;
        Self::seal_with_key_and_salt(&key, &salt, aad, plaintext)
    }

    /// Reads the Argon2id salt from `envelope`, derives key from `passphrase`, and decrypts.
    pub fn open_with_passphrase(
        passphrase: &SecretString,
        aad: &[u8],
        envelope: &[u8],
        params: KdfParams,
    ) -> Result<SecretBytes, VaultError> {
        if envelope.len() < MIN_ENVELOPE_LEN {
            return Err(VaultError::InvalidEnvelope(format!(
                "envelope too short ({} bytes)",
                envelope.len()
            )));
        }
        if &envelope[..ENVELOPE_MAGIC.len()] != ENVELOPE_MAGIC {
            return Err(VaultError::InvalidEnvelope(
                "unrecognized envelope magic header".to_string(),
            ));
        }
        let salt = &envelope[ENVELOPE_MAGIC.len()..HEADER_LEN];
        let key = derive_key_argon2id(passphrase.as_bytes(), salt, params)?;
        Self::open_with_key(&key, aad, envelope)
    }
}

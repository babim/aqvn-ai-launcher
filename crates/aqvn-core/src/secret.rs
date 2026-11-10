//! Zeroize-backed secret types to prevent sensitive material from lingering in RAM or logs.

use aws_lc_rs::constant_time::verify_slices_are_equal;
use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Arbitrary-length sensitive byte buffer zeroized when dropped.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }

    pub fn expose_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn constant_time_eq(&self, other: &Self) -> bool {
        verify_slices_are_equal(&self.0, &other.0).is_ok()
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretBytes([REDACTED; {} bytes])", self.0.len())
    }
}

impl From<Vec<u8>> for SecretBytes {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

/// Sensitive UTF-8 string zeroized when dropped and redacted in Debug/Display output.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn constant_time_eq(&self, other: &Self) -> bool {
        verify_slices_are_equal(self.0.as_bytes(), other.0.as_bytes()).is_ok()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Fixed 256-bit cryptographic key zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKey([u8; 32]);

impl SecretKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn generate_random() -> Result<Self, String> {
        let mut bytes = [0u8; 32];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| "CSPRNG key generation failed".to_string())?;
        Ok(Self(bytes))
    }

    pub fn expose_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn constant_time_eq(&self, other: &Self) -> bool {
        verify_slices_are_equal(&self.0, &other.0).is_ok()
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey([REDACTED; 32 bytes])")
    }
}

/// Bundle of service credentials and bridge secrets protected by Zeroize-on-drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct TokenCredentials {
    #[zeroize(skip)]
    pub service: String,
    #[zeroize(skip)]
    pub account: String,
    pub token: SecretString,
    pub rfc_password: Option<SecretString>,
    pub ssh_password: Option<SecretString>,
}

#[derive(Serialize, Deserialize, Zeroize)]
struct WireTokenCredentials {
    service: String,
    account: String,
    token: String,
    rfc_password: Option<String>,
    ssh_password: Option<String>,
}

impl TokenCredentials {
    pub fn new(
        service: impl Into<String>,
        account: impl Into<String>,
        token: SecretString,
    ) -> Self {
        Self {
            service: service.into(),
            account: account.into(),
            token,
            rfc_password: None,
            ssh_password: None,
        }
    }

    pub fn with_rfc_password(mut self, password: SecretString) -> Self {
        self.rfc_password = Some(password);
        self
    }

    pub fn with_ssh_password(mut self, password: SecretString) -> Self {
        self.ssh_password = Some(password);
        self
    }

    pub(crate) fn to_secret_json(&self) -> Result<SecretBytes, serde_json::Error> {
        let mut wire = WireTokenCredentials {
            service: self.service.clone(),
            account: self.account.clone(),
            token: self.token.expose_secret().to_string(),
            rfc_password: self
                .rfc_password
                .as_ref()
                .map(|s| s.expose_secret().to_string()),
            ssh_password: self
                .ssh_password
                .as_ref()
                .map(|s| s.expose_secret().to_string()),
        };
        let encoded = serde_json::to_vec(&wire);
        wire.zeroize();
        encoded.map(SecretBytes::new)
    }

    pub(crate) fn from_secret_json(bytes: &SecretBytes) -> Result<Self, serde_json::Error> {
        let mut wire: WireTokenCredentials = serde_json::from_slice(bytes.expose_bytes())?;
        let creds = Self {
            service: std::mem::take(&mut wire.service),
            account: std::mem::take(&mut wire.account),
            token: SecretString::new(std::mem::take(&mut wire.token)),
            rfc_password: wire.rfc_password.take().map(SecretString::new),
            ssh_password: wire.ssh_password.take().map(SecretString::new),
        };
        wire.zeroize();
        Ok(creds)
    }
}

impl fmt::Debug for TokenCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenCredentials")
            .field("service", &self.service)
            .field("account", &self.account)
            .field("token", &"[REDACTED]")
            .field(
                "rfc_password",
                &self.rfc_password.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "ssh_password",
                &self.ssh_password.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

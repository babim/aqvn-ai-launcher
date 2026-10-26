//! `helpers.rfc_exchange._provide_root_password`.
//!
//! The caller sends its RSA-2048 public key as a hex-encoded PEM string; the
//! reply is the hex ciphertext of the configured SSH password under
//! RSA-OAEP (SHA-256 digest, MGF1-SHA256, no label) — exactly what the remote
//! terminal client decrypts.

use crate::rfc::dispatch::{DispatchContext, DispatchError, RfcCall};
use crate::rfc::util::string_arg;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Oaep, RsaPublicKey};
use serde_json::Value;
use sha2::Sha256;

pub(crate) fn provide_root_password(
    context: &DispatchContext,
    call: &RfcCall,
) -> Result<Value, DispatchError> {
    let encoded_key = string_arg(call, 0)
        .ok_or_else(|| DispatchError::Handler("public key argument is required".to_string()))?;
    let pem = decode_hex(&encoded_key)?;
    let pem = String::from_utf8(pem).map_err(|_| {
        DispatchError::Handler("public key must be a hex-encoded PEM string".to_string())
    })?;
    let public_key = RsaPublicKey::from_public_key_pem(&pem)
        .map_err(|err| DispatchError::Handler(format!("invalid RSA public key: {err}")))?;

    let mut rng = rand::thread_rng();
    let ciphertext = public_key
        .encrypt(
            &mut rng,
            Oaep::new::<Sha256>(),
            context.ssh_password.as_bytes(),
        )
        .map_err(|err| DispatchError::Handler(format!("RSA-OAEP encryption failed: {err}")))?;
    Ok(Value::String(encode_hex(&ciphertext)))
}

fn decode_hex(input: &str) -> Result<Vec<u8>, DispatchError> {
    let trimmed = input.trim();
    if trimmed.is_empty() || !trimmed.len().is_multiple_of(2) {
        return Err(DispatchError::Handler(
            "public key must be an even-length hex string".to_string(),
        ));
    }
    (0..trimmed.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&trimmed[index..index + 2], 16).map_err(|_| {
                DispatchError::Handler("public key contains non-hex characters".to_string())
            })
        })
        .collect()
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

//! `helpers.rfc_exchange._provide_root_password`.
//!
//! The caller sends its RSA-2048 public key as a hex-encoded PEM string; the
//! reply is the hex ciphertext of the configured SSH password under
//! RSA-OAEP (SHA-256 digest, MGF1-SHA256, no label) — exactly what the remote
//! terminal client decrypts.

use crate::dispatch::{DispatchContext, DispatchError, RfcCall};
use crate::util::{decode_base64, string_arg};
use aws_lc_rs::rsa::{OaepPublicEncryptingKey, PublicEncryptingKey, OAEP_SHA256_MGF1SHA256};
use serde_json::Value;

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
    let der = decode_pem_subject_public_key_info(&pem)?;
    let public_key = PublicEncryptingKey::from_der(&der)
        .map_err(|err| DispatchError::Handler(format!("invalid RSA public key: {err}")))?;
    let public_key = OaepPublicEncryptingKey::new(public_key)
        .map_err(|err| DispatchError::Handler(format!("invalid RSA public key: {err}")))?;

    let mut buffer = vec![0u8; public_key.ciphertext_size()];
    let ciphertext = public_key
        .encrypt(
            &OAEP_SHA256_MGF1SHA256,
            context.ssh_password.as_bytes(),
            &mut buffer,
            None,
        )
        .map_err(|err| DispatchError::Handler(format!("RSA-OAEP encryption failed: {err}")))?;
    Ok(Value::String(encode_hex(ciphertext)))
}

/// Strips the `PUBLIC KEY` armor and base64-decodes the SubjectPublicKeyInfo body.
fn decode_pem_subject_public_key_info(pem: &str) -> Result<Vec<u8>, DispatchError> {
    const BEGIN: &str = "-----BEGIN PUBLIC KEY-----";
    const END: &str = "-----END PUBLIC KEY-----";
    let invalid = || {
        DispatchError::Handler("public key must be a PEM-encoded SubjectPublicKeyInfo".to_string())
    };
    let body_start = pem.find(BEGIN).ok_or_else(invalid)? + BEGIN.len();
    let body_end = pem[body_start..].find(END).ok_or_else(invalid)? + body_start;
    let body: String = pem[body_start..body_end]
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if body.is_empty() {
        return Err(invalid());
    }
    decode_base64(&body)
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

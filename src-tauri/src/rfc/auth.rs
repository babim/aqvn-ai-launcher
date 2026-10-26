//! HMAC-SHA256 request authentication for `/api/rfc`.
//!
//! The digest covers the exact received `rfc_input` string bytes (never a
//! re-serialization) and is compared in constant time.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Lowercase hex length of a SHA-256 digest.
const HASH_HEX_LEN: usize = 64;

/// Computes the lowercase-hex HMAC-SHA256 of `data` keyed by the password's
/// UTF-8 bytes.
pub fn compute_hash(password: &str, data: &str) -> String {
    let mut mac = new_mac(password);
    mac.update(data.as_bytes());
    to_lower_hex(&mac.finalize().into_bytes())
}

/// Verifies a lowercase-hex digest in constant time.
///
/// Rejects malformed digests (wrong length, uppercase or non-hex characters)
/// without panicking, matching the lowercase-hex wire format.
pub fn verify(password: &str, data: &str, expected_hex: &str) -> bool {
    let Some(expected) = decode_lower_hex(expected_hex) else {
        return false;
    };
    let mut mac = new_mac(password);
    mac.update(data.as_bytes());
    mac.verify_slice(&expected).is_ok()
}

fn new_mac(password: &str) -> HmacSha256 {
    // HMAC accepts keys of any length.
    HmacSha256::new_from_slice(password.as_bytes()).expect("HMAC accepts any key length")
}

fn decode_lower_hex(input: &str) -> Option<Vec<u8>> {
    if input.len() != HASH_HEX_LEN {
        return None;
    }
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Some((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn to_lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

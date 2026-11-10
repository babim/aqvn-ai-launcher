//! RFC 5280 X.509 v3 Certificate and Local Root CA generation for mTLS 1.3.
//!
//! Provides automated, zero-external-dependency certificate generation, SPIFFE ID binding,
//! and rotation tracking backed by Ed25519 (`aws-lc-rs`) and `rustls-pki-types`.

use crate::secret::{SecretBytes, SecretString};
use aws_lc_rs::encoding::AsDer;
use aws_lc_rs::rand::{SecureRandom, SystemRandom};
use aws_lc_rs::signature::{Ed25519KeyPair, KeyPair};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CertError {
    #[error("cryptographic key generation or signing error: {0}")]
    Crypto(String),
    #[error("X.509 ASN.1 DER encoding error: {0}")]
    DerEncoding(String),
    #[error("invalid SPIFFE ID URI: {0}")]
    InvalidSpiffeId(String),
    #[error("rustls certificate store error: {0}")]
    RustlsPki(String),
    #[error("I/O error: {0}")]
    Io(String),
}

/// Validated SPIFFE ID URI (`spiffe://<trust-domain>/<path>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiffeId(String);

impl SpiffeId {
    pub fn parse(uri: &str) -> Result<Self, CertError> {
        let trimmed = uri.trim();
        if !trimmed.starts_with("spiffe://") {
            return Err(CertError::InvalidSpiffeId(
                "SPIFFE ID must start with 'spiffe://'".to_string(),
            ));
        }
        let rest = &trimmed[9..];
        let Some((trust_domain, path)) = rest.split_once('/') else {
            return Err(CertError::InvalidSpiffeId(
                "SPIFFE ID must contain a trust domain and path".to_string(),
            ));
        };
        if trust_domain.is_empty() || path.is_empty() {
            return Err(CertError::InvalidSpiffeId(
                "SPIFFE trust domain and path cannot be empty".to_string(),
            ));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn for_gateway(trust_domain: &str, gateway_id: &str) -> Result<Self, CertError> {
        let uri = format!("spiffe://{trust_domain}/gateway/{gateway_id}");
        Self::parse(&uri)
    }

    pub fn for_node(trust_domain: &str, node_id: &str) -> Result<Self, CertError> {
        let uri = format!("spiffe://{trust_domain}/node/{node_id}");
        Self::parse(&uri)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An X.509 v3 Certificate and PKCS#8 private key pair.
#[derive(Clone)]
pub struct CertificateBundle {
    pub cert_der: Vec<u8>,
    pub cert_pem: String,
    pub key_pkcs8_der: SecretBytes,
    pub key_pem: SecretString,
    pub serial_hex: String,
    pub subject_cn: String,
    pub spiffe_id: Option<String>,
    pub not_before_unix: u64,
    pub not_after_unix: u64,
    pub subject_key_id: [u8; 20],
}

impl CertificateBundle {
    pub fn is_expired_at(&self, now_unix: u64) -> bool {
        now_unix >= self.not_after_unix
    }

    pub fn needs_rotation_at(&self, now_unix: u64, window_secs: u64) -> bool {
        now_unix.saturating_add(window_secs) >= self.not_after_unix
    }

    pub fn to_rustls_cert_der(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.cert_der.clone())
    }

    pub fn to_rustls_private_key(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            self.key_pkcs8_der.expose_bytes().to_vec(),
        ))
    }
}

// ============================================================================
// ASN.1 DER Builder Primitives
// ============================================================================

mod der {
    pub fn encode_len(len: usize) -> Vec<u8> {
        if len < 128 {
            vec![len as u8]
        } else if len < 256 {
            vec![0x81, len as u8]
        } else if len < 65536 {
            vec![0x82, (len >> 8) as u8, (len & 0xFF) as u8]
        } else {
            vec![
                0x83,
                (len >> 16) as u8,
                ((len >> 8) & 0xFF) as u8,
                (len & 0xFF) as u8,
            ]
        }
    }

    pub fn tl(tag: u8, val: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + 4 + val.len());
        out.push(tag);
        out.extend_from_slice(&encode_len(val.len()));
        out.extend_from_slice(val);
        out
    }

    pub fn sequence(children: &[&[u8]]) -> Vec<u8> {
        let total_len: usize = children.iter().map(|c| c.len()).sum();
        let mut out = Vec::with_capacity(1 + 4 + total_len);
        out.push(0x30); // SEQUENCE
        out.extend_from_slice(&encode_len(total_len));
        for c in children {
            out.extend_from_slice(c);
        }
        out
    }

    pub fn set(children: &[&[u8]]) -> Vec<u8> {
        let total_len: usize = children.iter().map(|c| c.len()).sum();
        let mut out = Vec::with_capacity(1 + 4 + total_len);
        out.push(0x31); // SET
        out.extend_from_slice(&encode_len(total_len));
        for c in children {
            out.extend_from_slice(c);
        }
        out
    }

    pub fn boolean(b: bool) -> Vec<u8> {
        tl(0x01, if b { &[0xFF] } else { &[0x00] })
    }

    pub fn integer_positive(bytes: &[u8]) -> Vec<u8> {
        if bytes.is_empty() {
            return tl(0x02, &[0x00]);
        }
        if bytes[0] & 0x80 != 0 {
            let mut prepended = Vec::with_capacity(bytes.len() + 1);
            prepended.push(0x00);
            prepended.extend_from_slice(bytes);
            tl(0x02, &prepended)
        } else {
            tl(0x02, bytes)
        }
    }

    pub fn bit_string(unused_bits: u8, data: &[u8]) -> Vec<u8> {
        let mut val = Vec::with_capacity(1 + data.len());
        val.push(unused_bits);
        val.extend_from_slice(data);
        tl(0x03, &val)
    }

    pub fn octet_string(data: &[u8]) -> Vec<u8> {
        tl(0x04, data)
    }

    pub fn oid(data: &[u8]) -> Vec<u8> {
        tl(0x06, data)
    }

    pub fn utf8_string(s: &str) -> Vec<u8> {
        tl(0x0C, s.as_bytes())
    }

    #[allow(dead_code)]
    pub fn ia5_string(s: &str) -> Vec<u8> {
        tl(0x16, s.as_bytes())
    }

    pub fn context_explicit(tag_num: u8, content: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + 4 + content.len());
        out.push(0xA0 | tag_num);
        out.extend_from_slice(&encode_len(content.len()));
        out.extend_from_slice(content);
        out
    }

    pub fn context_implicit(tag_num: u8, content: &[u8]) -> Vec<u8> {
        tl(0x80 | tag_num, content)
    }

    pub fn format_utc_time(epoch_secs: u64) -> Vec<u8> {
        // Civil calendar breakdown from unix seconds
        let (y, m, d, hh, mm, ss) = unix_to_utc_ymd_hms(epoch_secs);
        let yr_2digit = y % 100;
        let s = format!("{yr_2digit:02}{m:02}{d:02}{hh:02}{mm:02}{ss:02}Z");
        tl(0x17, s.as_bytes())
    }

    fn unix_to_utc_ymd_hms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
        let days = (secs / 86400) as i64;
        let rem = (secs % 86400) as u32;
        let hh = rem / 3600;
        let mm = (rem % 3600) / 60;
        let ss = rem % 60;

        // Euclidean affine day conversion (Hinnant algorithm)
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u32;
        let yoe = (doe - doe / 1024 + doe / 1461 - doe / 142401) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let final_y = if m <= 2 { y + 1 } else { y } as u32;

        (final_y, m, d, hh, mm, ss)
    }
}

// Common Standard OIDs
const OID_ED25519: &[u8] = &[0x2B, 0x65, 0x70]; // 1.3.101.112
const OID_COMMON_NAME: &[u8] = &[0x55, 0x04, 0x03]; // 2.5.4.3
const OID_ORGANIZATION: &[u8] = &[0x55, 0x04, 0x0A]; // 2.5.4.10
const OID_BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x13]; // 2.5.29.19
const OID_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x0F]; // 2.5.29.15
const OID_EXT_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x25]; // 2.5.29.37
const OID_SUBJECT_ALT_NAME: &[u8] = &[0x55, 0x1D, 0x11]; // 2.5.29.17
const OID_SUBJECT_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x0E]; // 2.5.29.14
const OID_AUTHORITY_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x23]; // 2.5.29.35

const OID_SERVER_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01]; // 1.3.6.1.5.5.7.3.1
const OID_CLIENT_AUTH: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x02]; // 1.3.6.1.5.5.7.3.2

fn build_ed25519_alg_id() -> Vec<u8> {
    // RFC 8410 §3: AlgorithmIdentifier for Ed25519 MUST NOT include parameters
    der::sequence(&[&der::oid(OID_ED25519)])
}

fn build_distinguished_name(common_name: &str, org: &str) -> Vec<u8> {
    let cn_seq = der::sequence(&[&der::oid(OID_COMMON_NAME), &der::utf8_string(common_name)]);
    let org_seq = der::sequence(&[&der::oid(OID_ORGANIZATION), &der::utf8_string(org)]);
    der::sequence(&[&der::set(&[&cn_seq]), &der::set(&[&org_seq])])
}

fn compute_ski(pubkey_der: &[u8]) -> [u8; 20] {
    let mut hasher = Sha256::new();
    hasher.update(pubkey_der);
    let digest = hasher.finalize();
    let mut ski = [0u8; 20];
    ski.copy_from_slice(&digest[..20]);
    ski
}

fn to_pem(tag: &str, der: &[u8]) -> String {
    let b64 = BASE64.encode(der);
    let mut out = format!("-----BEGIN {tag}-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        out.push('\n');
    }
    out.push_str(&format!("-----END {tag}-----\n"));
    out
}

// ============================================================================
// Local Root CA and Leaf Certificate Issuer
// ============================================================================

pub struct LocalCa {
    ca_bundle: CertificateBundle,
    ca_keypair: Ed25519KeyPair,
    revoked_serials: std::sync::RwLock<Vec<String>>,
}

impl LocalCa {
    /// Generates a self-signed Local Root Certificate Authority.
    pub fn generate(common_name: &str, ttl_hours: u64) -> Result<Self, CertError> {
        let rng = SystemRandom::new();
        let keypair = Ed25519KeyPair::generate()
            .map_err(|e| CertError::Crypto(format!("Ed25519 key generation failed: {e:?}")))?;

        let spki_der = keypair
            .public_key()
            .as_der()
            .map_err(|e| CertError::DerEncoding(format!("Public key as DER failed: {e:?}")))?;
        let spki_bytes = spki_der.as_ref();
        let ski = compute_ski(spki_bytes);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let not_before = now.saturating_sub(60); // 1 minute clock skew allowance
        let not_after = now.saturating_add(ttl_hours * 3600);

        let mut serial_bytes = [0u8; 16];
        rng.fill(&mut serial_bytes)
            .map_err(|_| CertError::Crypto("CSPRNG serial generation failed".into()))?;
        serial_bytes[0] &= 0x7F; // Ensure positive integer
        if serial_bytes[0] == 0 {
            serial_bytes[0] = 0x01;
        }
        let serial_hex = serial_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();

        let name = build_distinguished_name(common_name, "AQVN Workspace Local CA");
        let validity = der::sequence(&[
            &der::format_utc_time(not_before),
            &der::format_utc_time(not_after),
        ]);

        // Extensions for Root CA
        let bc_val = der::sequence(&[&der::boolean(true)]); // cA = true
        let bc_ext = der::sequence(&[
            &der::oid(OID_BASIC_CONSTRAINTS),
            &der::boolean(true), // critical
            &der::octet_string(&bc_val),
        ]);

        // KeyUsage: keyCertSign (bit 5) | cRLSign (bit 6) -> 0x06
        let ku_ext = der::sequence(&[
            &der::oid(OID_KEY_USAGE),
            &der::boolean(true), // critical
            &der::octet_string(&der::bit_string(1, &[0x06])),
        ]);

        let ski_ext = der::sequence(&[
            &der::oid(OID_SUBJECT_KEY_IDENTIFIER),
            &der::octet_string(&der::octet_string(&ski)),
        ]);

        let extensions = der::context_explicit(3, &der::sequence(&[&bc_ext, &ku_ext, &ski_ext]));

        // TBSCertificate
        let version = der::context_explicit(0, &der::integer_positive(&[2])); // v3
        let serial = der::integer_positive(&serial_bytes);
        let alg_id = build_ed25519_alg_id();

        let tbs = der::sequence(&[
            &version,
            &serial,
            &alg_id,
            &name, // issuer
            &validity,
            &name, // subject
            spki_bytes,
            &extensions,
        ]);

        // Self-sign TBSCertificate
        let sig = keypair.sign(&tbs);
        let sig_bitstring = der::bit_string(0, sig.as_ref());
        let cert_der = der::sequence(&[&tbs, &alg_id, &sig_bitstring]);
        let cert_pem = to_pem("CERTIFICATE", &cert_der);

        let pkcs8_doc = keypair
            .to_pkcs8v1()
            .map_err(|e| CertError::Crypto(format!("PKCS#8 export failed: {e:?}")))?;
        let key_pkcs8_der = SecretBytes::new(pkcs8_doc.as_ref().to_vec());
        let key_pem = SecretString::new(to_pem("PRIVATE KEY", key_pkcs8_der.expose_bytes()));

        let ca_bundle = CertificateBundle {
            cert_der,
            cert_pem,
            key_pkcs8_der,
            key_pem,
            serial_hex,
            subject_cn: common_name.to_string(),
            spiffe_id: None,
            not_before_unix: not_before,
            not_after_unix: not_after,
            subject_key_id: ski,
        };

        Ok(Self {
            ca_bundle,
            ca_keypair: keypair,
            revoked_serials: std::sync::RwLock::new(Vec::new()),
        })
    }

    pub fn ca_bundle(&self) -> &CertificateBundle {
        &self.ca_bundle
    }

    /// Issues an X.509 v3 leaf certificate signed by this CA for Server and/or Client mTLS.
    pub fn issue_leaf_cert(
        &self,
        common_name: &str,
        dns_sans: &[&str],
        ip_sans: &[IpAddr],
        spiffe_id: Option<&str>,
        ttl_hours: u64,
    ) -> Result<CertificateBundle, CertError> {
        let rng = SystemRandom::new();
        let leaf_keypair = Ed25519KeyPair::generate()
            .map_err(|e| CertError::Crypto(format!("Leaf key generation failed: {e:?}")))?;

        let spki_der = leaf_keypair
            .public_key()
            .as_der()
            .map_err(|e| CertError::DerEncoding(format!("Public key as DER failed: {e:?}")))?;
        let spki_bytes = spki_der.as_ref();
        let ski = compute_ski(spki_bytes);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        let not_before = now.saturating_sub(60);
        let not_after = now.saturating_add(ttl_hours * 3600);

        let mut serial_bytes = [0u8; 16];
        rng.fill(&mut serial_bytes)
            .map_err(|_| CertError::Crypto("CSPRNG serial generation failed".into()))?;
        serial_bytes[0] &= 0x7F;
        if serial_bytes[0] == 0 {
            serial_bytes[0] = 0x01;
        }
        let serial_hex = serial_bytes
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();

        let issuer_name =
            build_distinguished_name(&self.ca_bundle.subject_cn, "AQVN Workspace Local CA");
        let subject_name = build_distinguished_name(common_name, "AQVN Workspace");
        let validity = der::sequence(&[
            &der::format_utc_time(not_before),
            &der::format_utc_time(not_after),
        ]);

        // Extensions for Leaf Cert
        // 1. BasicConstraints: cA = false (empty sequence)
        let bc_val = der::sequence(&[]);
        let bc_ext = der::sequence(&[
            &der::oid(OID_BASIC_CONSTRAINTS),
            &der::boolean(true), // critical
            &der::octet_string(&bc_val),
        ]);

        // 2. KeyUsage: digitalSignature (bit 0) -> 0x80, unused 7
        let ku_ext = der::sequence(&[
            &der::oid(OID_KEY_USAGE),
            &der::boolean(true), // critical
            &der::octet_string(&der::bit_string(7, &[0x80])),
        ]);

        // 3. ExtendedKeyUsage: serverAuth + clientAuth
        let eku_val = der::sequence(&[&der::oid(OID_SERVER_AUTH), &der::oid(OID_CLIENT_AUTH)]);
        let eku_ext = der::sequence(&[&der::oid(OID_EXT_KEY_USAGE), &der::octet_string(&eku_val)]);

        // 4. SubjectAltName: DNS names + IPs + SPIFFE URI
        let mut san_entries = Vec::new();
        for dns in dns_sans {
            san_entries.push(der::context_implicit(2, dns.as_bytes())); // dNSName [2]
        }
        for ip in ip_sans {
            match ip {
                IpAddr::V4(v4) => {
                    san_entries.push(der::context_implicit(7, &v4.octets())); // iPAddress [7]
                }
                IpAddr::V6(v6) => {
                    san_entries.push(der::context_implicit(7, &v6.octets()));
                }
            }
        }
        if let Some(spiffe) = spiffe_id {
            san_entries.push(der::context_implicit(6, spiffe.as_bytes())); // uniformResourceIdentifier [6]
        }
        let san_slices: Vec<&[u8]> = san_entries.iter().map(|e| e.as_slice()).collect();
        let san_val = der::sequence(&san_slices);
        let san_ext = der::sequence(&[
            &der::oid(OID_SUBJECT_ALT_NAME),
            &der::octet_string(&san_val),
        ]);

        // 5. SubjectKeyIdentifier & AuthorityKeyIdentifier
        let ski_ext = der::sequence(&[
            &der::oid(OID_SUBJECT_KEY_IDENTIFIER),
            &der::octet_string(&der::octet_string(&ski)),
        ]);
        let aki_val = der::sequence(&[&der::context_implicit(0, &self.ca_bundle.subject_key_id)]);
        let aki_ext = der::sequence(&[
            &der::oid(OID_AUTHORITY_KEY_IDENTIFIER),
            &der::octet_string(&aki_val),
        ]);

        let extensions = der::context_explicit(
            3,
            &der::sequence(&[&bc_ext, &ku_ext, &eku_ext, &san_ext, &ski_ext, &aki_ext]),
        );

        let version = der::context_explicit(0, &der::integer_positive(&[2]));
        let serial = der::integer_positive(&serial_bytes);
        let alg_id = build_ed25519_alg_id();

        let tbs = der::sequence(&[
            &version,
            &serial,
            &alg_id,
            &issuer_name,
            &validity,
            &subject_name,
            spki_bytes,
            &extensions,
        ]);

        // Sign TBSCertificate with CA private key
        let sig = self.ca_keypair.sign(&tbs);
        let sig_bitstring = der::bit_string(0, sig.as_ref());
        let cert_der = der::sequence(&[&tbs, &alg_id, &sig_bitstring]);
        let cert_pem = to_pem("CERTIFICATE", &cert_der);

        let pkcs8_doc = leaf_keypair
            .to_pkcs8v1()
            .map_err(|e| CertError::Crypto(format!("PKCS#8 export failed: {e:?}")))?;
        let key_pkcs8_der = SecretBytes::new(pkcs8_doc.as_ref().to_vec());
        let key_pem = SecretString::new(to_pem("PRIVATE KEY", key_pkcs8_der.expose_bytes()));

        Ok(CertificateBundle {
            cert_der,
            cert_pem,
            key_pkcs8_der,
            key_pem,
            serial_hex,
            subject_cn: common_name.to_string(),
            spiffe_id: spiffe_id.map(|s| s.to_string()),
            not_before_unix: not_before,
            not_after_unix: not_after,
            subject_key_id: ski,
        })
    }

    /// Builds a rustls `RootCertStore` containing this CA's certificate.
    pub fn root_cert_store(&self) -> Result<rustls::RootCertStore, CertError> {
        let mut store = rustls::RootCertStore::empty();
        store
            .add(self.ca_bundle.to_rustls_cert_der())
            .map_err(|e| CertError::RustlsPki(e.to_string()))?;
        Ok(store)
    }

    pub fn revoke_serial(&self, serial_hex: &str) {
        if let Ok(mut lock) = self.revoked_serials.write() {
            lock.push(serial_hex.to_string());
        }
    }

    pub fn is_revoked(&self, serial_hex: &str) -> bool {
        if let Ok(lock) = self.revoked_serials.read() {
            lock.iter().any(|s| s == serial_hex)
        } else {
            false
        }
    }
}

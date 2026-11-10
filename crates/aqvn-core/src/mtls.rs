//! Mutual TLS 1.3 (mTLS) configuration and provisioner using `aws-lc-rs` and `rustls`.

use crate::cert::{CertError, CertificateBundle, LocalCa, SpiffeId};
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use std::net::IpAddr;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MtlsError {
    #[error("certificate generation error: {0}")]
    Cert(#[from] CertError),
    #[error("mTLS client cert verifier error: {0}")]
    Verifier(String),
    #[error("rustls configuration error: {0}")]
    Config(String),
    #[error("invalid mTLS parameter: {0}")]
    InvalidParameter(String),
}

/// Builds an mTLS 1.3 `ServerConfig` with mandatory client certificate verification.
pub fn build_mtls_server_config(
    server_bundle: &CertificateBundle,
    client_ca_store: Arc<RootCertStore>,
) -> Result<Arc<ServerConfig>, MtlsError> {
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let verifier =
        WebPkiClientVerifier::builder_with_provider(client_ca_store, Arc::new(provider.clone()))
            .build()
            .map_err(|e| MtlsError::Verifier(e.to_string()))?;

    let config = ServerConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| MtlsError::Config(e.to_string()))?
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![server_bundle.to_rustls_cert_der()],
            server_bundle.to_rustls_private_key(),
        )
        .map_err(|e| MtlsError::Config(e.to_string()))?;

    Ok(Arc::new(config))
}

/// Builds an mTLS 1.3 `ClientConfig` with trusted root certificates and client credentials.
pub fn build_mtls_client_config(
    client_bundle: &CertificateBundle,
    server_ca_store: Arc<RootCertStore>,
) -> Result<Arc<ClientConfig>, MtlsError> {
    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| MtlsError::Config(e.to_string()))?
        .with_root_certificates(server_ca_store)
        .with_client_auth_cert(
            vec![client_bundle.to_rustls_cert_der()],
            client_bundle.to_rustls_private_key(),
        )
        .map_err(|e| MtlsError::Config(e.to_string()))?;

    Ok(Arc::new(config))
}

/// Self-contained cluster credentials for local RFC/SSH mTLS 1.3 channels.
pub struct ClusterMtlsCredentials {
    pub ca: LocalCa,
    pub server_bundle: CertificateBundle,
    pub client_bundle: CertificateBundle,
    pub server_config: Arc<ServerConfig>,
    pub client_config: Arc<ClientConfig>,
}

impl ClusterMtlsCredentials {
    /// Auto-provisions a local Root CA, server cert, client cert, and mutual TLS 1.3 configs.
    pub fn provision_local(
        server_cn: &str,
        bind_ip: IpAddr,
        gateway_id: &str,
        ttl_hours: u64,
    ) -> Result<Self, MtlsError> {
        let ca = LocalCa::generate("AQVN Local Root CA", ttl_hours * 2)?;
        let spiffe_server = SpiffeId::for_gateway("aqvn.workspace", gateway_id)?;
        let spiffe_client = SpiffeId::for_node("aqvn.workspace", gateway_id)?;

        let server_bundle = ca.issue_leaf_cert(
            server_cn,
            &["localhost", "aqvn-workspace", "127.0.0.1"],
            &[bind_ip, IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)],
            Some(spiffe_server.as_str()),
            ttl_hours,
        )?;

        let client_bundle = ca.issue_leaf_cert(
            "aqvn-client-agent",
            &["localhost"],
            &[IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)],
            Some(spiffe_client.as_str()),
            ttl_hours,
        )?;

        let root_store = Arc::new(ca.root_cert_store()?);
        let server_config = build_mtls_server_config(&server_bundle, root_store.clone())?;
        let client_config = build_mtls_client_config(&client_bundle, root_store)?;

        Ok(Self {
            ca,
            server_bundle,
            client_bundle,
            server_config,
            client_config,
        })
    }
}

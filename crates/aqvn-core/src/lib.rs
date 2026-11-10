//! Core schemas, primitives, cryptographic vault, and state for AQVN Workspace.

pub mod cert;
pub mod config;
pub mod mtls;
pub mod secret;
pub mod state;
pub mod telemetry;
pub mod vault;

pub use cert::{CertError, CertificateBundle, LocalCa, SpiffeId};
pub use config::{AppConfig, ConfigWatcher, WindowState};
pub use mtls::{
    build_mtls_client_config, build_mtls_server_config, ClusterMtlsCredentials, MtlsError,
};
pub use secret::{SecretBytes, SecretKey, SecretString, TokenCredentials};
pub use state::{is_gateway_connected, set_gateway_connected};
pub use telemetry::HostSystemMetrics;
pub use vault::{
    test_keyring_vault, EncryptedFileBackend, EnvelopeCipher, InMemoryBackend, KeyringBackend,
    KeyringBackendKind, KeyringVault, VaultError,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_gateway_id_format() {
        let id = config::deterministic_gateway_id();
        assert!(id.starts_with("aqvn-gw-"));
        assert_eq!(id.len(), 8 + 12); // aqvn-gw- + 12 hex chars
    }

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server_url, config::DEFAULT_SERVER_URL);
        assert_eq!(config.rfc_port, 55080);
        assert_eq!(config.ssh_port, 55022);
        assert!(!config.mtls_enabled);
        assert_eq!(config.mtls_cert_ttl_hours, 24);
    }

    #[test]
    fn test_gateway_connected_state() {
        set_gateway_connected(false);
        assert!(!is_gateway_connected());
        set_gateway_connected(true);
        assert!(is_gateway_connected());
    }

    #[test]
    fn test_keyring_vault_contract() {
        let res = test_keyring_vault();
        assert!(res.is_ok(), "keyring vault contract failed: {res:?}");
    }
}

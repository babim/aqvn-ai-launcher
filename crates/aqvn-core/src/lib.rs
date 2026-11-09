pub mod config;
pub mod state;
pub mod telemetry;

pub use config::{AppConfig, ConfigWatcher, WindowState};
pub use state::{is_gateway_connected, set_gateway_connected};
pub use telemetry::HostSystemMetrics;

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
    }

    #[test]
    fn test_gateway_connected_state() {
        set_gateway_connected(false);
        assert!(!is_gateway_connected());
        set_gateway_connected(true);
        assert!(is_gateway_connected());
    }
}

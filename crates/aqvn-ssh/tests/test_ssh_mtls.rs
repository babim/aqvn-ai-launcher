//! mTLS 1.3 tunnel verification tests for SSH remote control bridge.
//!
//! Validates:
//! - SSH bridge enforces mTLS 1.3 client authentication when mtls_enabled is configured.
//! - Valid client certificate allows establishing the secure tunnel and receiving the SSH banner.
//! - Anonymous client (no client certificate) is rejected during TLS 1.3 handshake.
//! - Untrusted client certificate is rejected by the TLS verifier.

use aqvn_core::cert::LocalCa;
use aqvn_core::config::AppConfig;
use aqvn_core::mtls::{build_mtls_client_config, ClusterMtlsCredentials};
use aqvn_ssh::server::{self, LockoutTracker};
use rustls_pki_types::ServerName;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsConnector;

#[tokio::test]
async fn test_ssh_mtls_13_tunnel_and_rejection() {
    let config = AppConfig {
        rfc_enabled: true,
        ssh_password: "test-ssh-password-123".to_string(),
        mtls_enabled: true,
        gateway_id: "test-gw-ssh-01".to_string(),
        ..AppConfig::default()
    };

    let creds = ClusterMtlsCredentials::provision_local(
        "localhost",
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        &config.gateway_id,
        24,
    )
    .expect("provision ssh mtls credentials");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let server_addr = listener.local_addr().expect("local addr");
    let server_config = creds.server_config.clone();

    // Spawn SSH mTLS bridge
    tokio::spawn(async move {
        let _ =
            server::serve_mtls_tunnel(listener, config, server_config, LockoutTracker::default())
                .await;
    });

    // 1. Valid mTLS tunnel connection -> should receive SSH banner
    {
        let connector = TlsConnector::from(creds.client_config.clone());
        let tcp_stream = TcpStream::connect(server_addr)
            .await
            .expect("tcp connect to ssh bridge");
        let domain = ServerName::try_from("localhost").unwrap().to_owned();

        let mut tls_stream = connector
            .connect(domain, tcp_stream)
            .await
            .expect("mTLS handshake must succeed for valid SSH client");

        // Read SSH server identification banner
        let mut banner_buf = [0u8; 256];
        let bytes_read = tls_stream
            .read(&mut banner_buf)
            .await
            .expect("read ssh banner");
        assert!(bytes_read > 0, "ssh server should send banner");

        let banner_str = String::from_utf8_lossy(&banner_buf[..bytes_read]);
        assert!(
            banner_str.starts_with("SSH-2.0-aqvn-workspace"),
            "expected aqvn-workspace SSH banner over mTLS, got: {banner_str}"
        );
    }

    // 2. Reject connection presenting NO client certificate
    {
        let provider = rustls::crypto::aws_lc_rs::default_provider();
        let root_store = Arc::new(creds.ca.root_cert_store().expect("ca store"));
        let no_auth_config = Arc::new(
            rustls::ClientConfig::builder_with_provider(Arc::new(provider))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .expect("tls13 client")
                .with_root_certificates(root_store)
                .with_no_client_auth(),
        );

        let connector = TlsConnector::from(no_auth_config);
        let tcp_stream = TcpStream::connect(server_addr).await.expect("tcp connect");
        let domain = ServerName::try_from("localhost").unwrap().to_owned();

        let connect_res = connector.connect(domain, tcp_stream).await;
        let rejected = match connect_res {
            Err(_) => true,
            Ok(mut tls_stream) => {
                let mut buf = [0u8; 128];
                let read_res = tls_stream.read(&mut buf).await;
                read_res.is_err() || read_res.unwrap() == 0
            }
        };
        assert!(
            rejected,
            "SSH mTLS server must reject client with no client certificate"
        );
    }

    // 3. Reject connection with untrusted client certificate (foreign CA)
    {
        let foreign_ca = LocalCa::generate("Foreign Rogue CA", 24).expect("generate foreign ca");
        let rogue_leaf = foreign_ca
            .issue_leaf_cert(
                "rogue-ssh-client",
                &["localhost"],
                &[IpAddr::V4(Ipv4Addr::LOCALHOST)],
                None,
                24,
            )
            .expect("issue rogue cert");

        let trusted_server_store = Arc::new(creds.ca.root_cert_store().expect("ca store"));
        let rogue_client_config = build_mtls_client_config(&rogue_leaf, trusted_server_store)
            .expect("rogue client config");

        let connector = TlsConnector::from(rogue_client_config);
        let tcp_stream = TcpStream::connect(server_addr).await.expect("tcp connect");
        let domain = ServerName::try_from("localhost").unwrap().to_owned();

        let connect_res = connector.connect(domain, tcp_stream).await;
        let rejected = match connect_res {
            Err(_) => true,
            Ok(mut tls_stream) => {
                let mut buf = [0u8; 128];
                let read_res = tls_stream.read(&mut buf).await;
                read_res.is_err() || read_res.unwrap() == 0
            }
        };
        assert!(
            rejected,
            "SSH mTLS server must reject client with certificate from untrusted CA"
        );
    }
}

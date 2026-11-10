//! Zero-Trust Mutual TLS 1.3 (mTLS) handshake verification tests for `/api/rfc`.
//!
//! Validates:
//! - RFC server enforces mTLS 1.3 with client certificate validation.
//! - Valid client certificate issued by the cluster Root CA establishes an encrypted session.
//! - HTTP request/response succeeds over mutual TLS 1.3.
//! - Connection attempts without client certificate are rejected during TLS handshake.
//! - Connection attempts with an untrusted client certificate are rejected.

use aqvn_core::cert::LocalCa;
use aqvn_core::config::AppConfig;
use aqvn_core::mtls::{build_mtls_client_config, ClusterMtlsCredentials};
use aqvn_rfc::server;
use rustls_pki_types::ServerName;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsConnector;

#[tokio::test]
async fn test_mtls_13_handshake_and_dispatch() {
    let config = AppConfig {
        rfc_enabled: true,
        rfc_password: "test-mtls-password".to_string(),
        mtls_enabled: true,
        gateway_id: "test-gw-mtls-01".to_string(),
        ..AppConfig::default()
    };

    let creds = ClusterMtlsCredentials::provision_local(
        "localhost",
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        &config.gateway_id,
        24,
    )
    .expect("provision mTLS credentials");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let server_addr = listener.local_addr().expect("local addr");
    let server_config = creds.server_config.clone();
    let rfc_state = server::build_state(&config);

    // Spawn RFC mTLS server
    tokio::spawn(async move {
        let _ = server::serve_mtls(listener, rfc_state, server_config).await;
    });

    // 1. Valid mTLS connection
    {
        let connector = TlsConnector::from(creds.client_config.clone());
        let tcp_stream = TcpStream::connect(server_addr)
            .await
            .expect("tcp connect to server");
        let domain = ServerName::try_from("localhost")
            .expect("valid server name")
            .to_owned();

        let mut tls_stream = connector
            .connect(domain, tcp_stream)
            .await
            .expect("mTLS handshake must succeed for valid client");

        // Send HTTP GET request
        let request_bytes =
            b"GET /api/rfc HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        tls_stream
            .write_all(request_bytes)
            .await
            .expect("write http request");

        let mut response_buf = Vec::new();
        tls_stream
            .read_to_end(&mut response_buf)
            .await
            .expect("read response");
        let response_str = String::from_utf8_lossy(&response_buf);
        assert!(
            response_str.starts_with("HTTP/1.1"),
            "expected HTTP response over mTLS, got: {response_str}"
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
                let write_res = tls_stream.write_all(b"GET /api/rfc HTTP/1.1\r\n\r\n").await;
                let mut buf = [0u8; 128];
                let read_res = tls_stream.read(&mut buf).await;
                write_res.is_err() || read_res.is_err() || read_res.unwrap() == 0
            }
        };
        assert!(
            rejected,
            "mTLS server must reject client with no client certificate"
        );
    }

    // 3. Reject connection with untrusted client certificate (issued by foreign CA)
    {
        let foreign_ca = LocalCa::generate("Foreign Rogue CA", 24).expect("generate foreign ca");
        let rogue_leaf = foreign_ca
            .issue_leaf_cert(
                "attacker-agent",
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
                let write_res = tls_stream.write_all(b"GET /api/rfc HTTP/1.1\r\n\r\n").await;
                let mut buf = [0u8; 128];
                let read_res = tls_stream.read(&mut buf).await;
                write_res.is_err() || read_res.is_err() || read_res.unwrap() == 0
            }
        };
        assert!(
            rejected,
            "mTLS server must reject client with certificate from untrusted CA"
        );
    }
}

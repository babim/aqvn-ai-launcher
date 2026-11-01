use aqvn_workspace_lib::config::AppConfig;
use aqvn_workspace_lib::proxy::upstream::Upstream;
use axum::http::Method;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

struct HttpsServerGuard {
    child: Child,
    port: u16,
    cert_path: PathBuf,
    _temp_dir: tempfile_fixture::TempDirFixture,
}

impl Drop for HttpsServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

mod tempfile_fixture {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct TempDirFixture {
        path: PathBuf,
    }

    impl TempDirFixture {
        pub fn new(prefix: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDirFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Spawns a lightweight Python HTTPS server using an OpenSSL-generated self-signed cert.
fn start_test_https_server() -> HttpsServerGuard {
    let temp_dir = tempfile_fixture::TempDirFixture::new("test-https");
    let key_path = temp_dir.path().join("server.key");
    let cert_path = temp_dir.path().join("server.crt");

    // Generate self-signed certificate for localhost
    let status = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            key_path.to_str().unwrap(),
            "-out",
            cert_path.to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=localhost",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("generate self-signed cert");
    assert!(status.success(), "openssl cert generation failed");

    let python_script = r#"
import http.server, ssl, sys

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{"status":"ok","token":"custom-ca-test-pass"}')
    def log_message(self, format, *args): pass

server = http.server.HTTPServer(('127.0.0.1', 0), Handler)
port = server.server_port
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
ctx.load_cert_chain(sys.argv[1], sys.argv[2])
server.socket = ctx.wrap_socket(server.socket)
print(f"READY:{port}", flush=True)
server.serve_forever()
"#;

    let mut child = Command::new("python3")
        .args([
            "-c",
            python_script,
            cert_path.to_str().unwrap(),
            key_path.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn python https server");

    let stdout = child.stdout.as_mut().expect("child stdout");
    use std::io::BufRead;
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read readiness");
    let line = line.trim();
    assert!(
        line.starts_with("READY:"),
        "expected READY:port, got: {line}"
    );
    let port: u16 = line
        .strip_prefix("READY:")
        .unwrap()
        .parse()
        .expect("parsed port");

    HttpsServerGuard {
        child,
        port,
        cert_path,
        _temp_dir: temp_dir,
    }
}

#[tokio::test]
async fn test_custom_ca_cert_and_skip_verify() {
    // 1. AppConfig default values and serialization
    let default_cfg = AppConfig::default();
    assert_eq!(default_cfg.tls_ca_cert_path, None);
    assert!(!default_cfg.tls_skip_verify);

    let mut custom_cfg = AppConfig {
        tls_ca_cert_path: Some("/etc/ssl/certs/custom-ca.pem".to_string()),
        tls_skip_verify: true,
        ..Default::default()
    };
    let json = serde_json::to_string(&custom_cfg).expect("serialize config");
    let parsed: AppConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(
        parsed.tls_ca_cert_path,
        Some("/etc/ssl/certs/custom-ca.pem".to_string())
    );
    assert!(parsed.tls_skip_verify);

    // 2. Start local HTTPS server with self-signed certificate
    let server = start_test_https_server();
    let https_url = format!("https://localhost:{}", server.port);
    let loopback_ip_url = format!("https://127.0.0.1:{}", server.port);

    // 3. Upstream with tls_skip_verify = true allows testing against self-signed loopback endpoints
    let skip_verify_upstream = Upstream::new_with_tls(&loopback_ip_url, None::<&str>, true)
        .expect("create upstream with tls_skip_verify");
    assert!(skip_verify_upstream.tls_skip_verify());
    assert_eq!(skip_verify_upstream.tls_ca_cert_path(), None);

    let response = skip_verify_upstream
        .forward_raw(Method::GET, "/", None, Vec::new())
        .await
        .expect("forward_raw with tls_skip_verify should succeed against self-signed endpoint");
    assert_eq!(response.status, 200);
    assert!(String::from_utf8_lossy(&response.body).contains("custom-ca-test-pass"));

    // 4. Default Upstream (tls_skip_verify = false, no CA cert) should fail against self-signed server
    let default_upstream = Upstream::new(&https_url).expect("create default upstream");
    assert!(!default_upstream.tls_skip_verify());
    let default_res = default_upstream
        .forward_raw(Method::GET, "/", None, Vec::new())
        .await;
    assert!(
        default_res.is_err(),
        "unverified request to self-signed server should fail TLS validation"
    );

    // 5. Upstream with custom CA cert path loads root cert and succeeds
    let ca_upstream = Upstream::new_with_tls(&https_url, Some(&server.cert_path), false)
        .expect("create upstream with custom ca cert");
    assert!(!ca_upstream.tls_skip_verify());
    assert_eq!(
        ca_upstream.tls_ca_cert_path(),
        Some(server.cert_path.as_path())
    );

    let ca_response = ca_upstream
        .forward_raw(Method::GET, "/", None, Vec::new())
        .await
        .expect("forward_raw with custom CA cert bundle should verify and succeed");
    assert_eq!(ca_response.status, 200);
    assert!(String::from_utf8_lossy(&ca_response.body).contains("custom-ca-test-pass"));

    // 6. Upstream from_config helper method works as expected
    custom_cfg.server_url = https_url.clone();
    custom_cfg.tls_ca_cert_path = Some(server.cert_path.to_str().unwrap().to_string());
    custom_cfg.tls_skip_verify = false;

    let from_cfg_upstream = Upstream::from_config(&custom_cfg).expect("upstream from_config");
    let from_cfg_resp = from_cfg_upstream
        .forward_raw(Method::GET, "/", None, Vec::new())
        .await
        .expect("upstream from_config with ca cert should succeed");
    assert_eq!(from_cfg_resp.status, 200);

    // 7. Non-existent CA certificate path returns an informative error
    let bad_path = server
        .cert_path
        .parent()
        .unwrap()
        .join("non_existent_ca.crt");
    let bad_path_res = Upstream::new_with_tls(&https_url, Some(&bad_path), false);
    match bad_path_res {
        Err(err_msg) => {
            assert!(
                err_msg.contains("failed to read TLS CA certificate"),
                "error message should indicate read failure, got: {err_msg}"
            );
        }
        Ok(_) => panic!("expected non-existent cert path to fail"),
    }

    // 8. Invalid CA certificate content returns an informative error
    let invalid_cert_path = server.cert_path.parent().unwrap().join("invalid.crt");
    std::fs::write(&invalid_cert_path, b"not a valid pem or der cert").unwrap();
    let invalid_res = Upstream::new_with_tls(&https_url, Some(&invalid_cert_path), false);
    match invalid_res {
        Err(invalid_err) => {
            assert!(
                invalid_err.contains("failed to parse TLS CA certificate"),
                "error message should indicate parse failure, got: {invalid_err}"
            );
        }
        Ok(_) => panic!("expected invalid cert data to fail"),
    }
}

//! RFC server core and supervisor auto-recovery tests.
//!
//! Covers validation contract assertion VAL-RES-001:
//! When the RFC server experiences a transient listener or socket error,
//! the supervisor logs the failure, backs off exponentially, and restarts
//! without exiting the background task.

use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::rfc::{auth, server};
use serde_json::json;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TEST_PASSWORD: &str = "secret-supervisor-test";
const WORKED_INPUT: &str = r#"{"module": "helpers.rfc_files", "function_name": "_path_exists_impl", "args": ["/a0/test.txt"], "kwargs": {}}"#;

struct TempSandbox {
    dir: PathBuf,
}

impl TempSandbox {
    fn new(files: &[(&str, &[u8])]) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("aqvn-rfc-core-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create sandbox root");
        for (relative, contents) in files {
            let path = dir.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create sandbox parent");
            }
            std::fs::write(&path, contents).expect("write sandbox fixture");
        }
        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for TempSandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn config_for(sandbox: &Path, port: u16) -> AppConfig {
    let mut config = AppConfig {
        rfc_enabled: true,
        allowed_paths: vec![sandbox.to_string_lossy().into_owned()],
        rfc_port: port,
        ..Default::default()
    };
    config.rfc_password.push_str(TEST_PASSWORD);
    config
}

async fn wait_for_http_200(port: u16, timeout: Duration) -> bool {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let deadline = Instant::now() + timeout;
    let local = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port);
    let hash = auth::compute_hash(TEST_PASSWORD, WORKED_INPUT);
    let body = json!({ "rfc_input": WORKED_INPUT, "hash": hash }).to_string();
    let request = format!(
        "POST /api/rfc HTTP/1.1\r\nHost: {local}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    while Instant::now() < deadline {
        if let Ok(mut stream) = tokio::net::TcpStream::connect(local).await {
            if stream.write_all(request.as_bytes()).await.is_ok() {
                let mut resp = Vec::new();
                if stream.read_to_end(&mut resp).await.is_ok() {
                    let text = String::from_utf8_lossy(&resp);
                    if text.starts_with("HTTP/1.1 200 OK") {
                        return true;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[test]
fn val_res_001_exponential_backoff_progression() {
    let min = server::INITIAL_BACKOFF;
    let max = server::MAX_BACKOFF;
    let factor = server::BACKOFF_FACTOR;

    assert_eq!(min, Duration::from_secs(1));
    assert_eq!(max, Duration::from_secs(10));
    assert_eq!(factor, 2.0);

    let step1 = server::compute_backoff(min, factor, max);
    assert_eq!(step1, Duration::from_secs(2));

    let step2 = server::compute_backoff(step1, factor, max);
    assert_eq!(step2, Duration::from_secs(4));

    let step3 = server::compute_backoff(step2, factor, max);
    assert_eq!(step3, Duration::from_secs(8));

    let step4 = server::compute_backoff(step3, factor, max);
    assert_eq!(step4, Duration::from_secs(10));

    let step5 = server::compute_backoff(step4, factor, max);
    assert_eq!(step5, Duration::from_secs(10));
}

#[tokio::test]
async fn val_res_001_rfc_supervisor_auto_recovers_on_transient_listener_error() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);

    // Bind an ephemeral port so it is initially occupied and unavailable to the RFC supervisor.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let busy_port = probe.local_addr().expect("probe addr").port();

    let config = config_for(sandbox.path(), busy_port);
    let watcher = ConfigWatcher::new(config);
    let sup_watcher = watcher.clone();

    // Launch supervisor with fast backoff for testing.
    let supervisor = tokio::spawn(async move {
        server::run_supervised_with_backoff(
            sup_watcher,
            Duration::from_millis(50),
            Duration::from_millis(300),
        )
        .await
    });

    // Let the supervisor attempt to bind and fail, entering backoff retry loop.
    tokio::time::sleep(Duration::from_millis(120)).await;

    // Verify supervisor did NOT terminate on bind error!
    assert!(
        !supervisor.is_finished(),
        "supervisor background task must survive transient socket bind error"
    );

    // Release the port: now the supervisor can bind on its next retry.
    drop(probe);

    // Wait for the supervisor to recover, bind, and serve requests.
    let recovered = wait_for_http_200(busy_port, Duration::from_secs(5)).await;
    assert!(
        recovered,
        "RFC server should auto-recover and serve requests after port becomes available"
    );

    // Verify supervisor is still healthy and running after recovery.
    assert!(
        !supervisor.is_finished(),
        "supervisor must remain active after recovering from socket error"
    );

    supervisor.abort();
}

#[tokio::test]
async fn val_res_001_rfc_supervisor_default_loop_does_not_exit_on_error() {
    let sandbox = TempSandbox::new(&[("test.txt", b"core content")]);

    // Bind ephemeral port
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let busy_port = probe.local_addr().expect("probe addr").port();

    let config = config_for(sandbox.path(), busy_port);
    let watcher = ConfigWatcher::new(config);
    let sup_watcher = watcher.clone();

    // Run standard run_supervised (1s..10s backoff)
    let supervisor = tokio::spawn(async move { server::run_supervised(sup_watcher).await });

    // Wait 150ms while socket is busy
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        !supervisor.is_finished(),
        "run_supervised must not exit on initial bind collision"
    );

    // Drop collision and allow standard 1s retry to bind
    drop(probe);

    let recovered = wait_for_http_200(busy_port, Duration::from_secs(5)).await;
    assert!(
        recovered,
        "run_supervised should recover within 1s exponential backoff window"
    );

    assert!(!supervisor.is_finished());
    supervisor.abort();
}

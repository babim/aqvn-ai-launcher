//! Integration tests for the embedded SSH bridge.
//!
//! Drives the real russh server with a real russh client:
//! VAL-SSH-001 (listener gated by the master toggle), VAL-SSH-002 (password auth
//! → PTY shell with client geometry and allowed-path cwd), VAL-SSH-003 (wrong
//! password rejected), VAL-SSH-004/005 (denylist refusal, session survives),
//! VAL-SSH-006 (benign commands unimpeded), VAL-SSH-009 (ssh_port hot-applied).

use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::ssh;
use async_trait::async_trait;
use russh::client::{self, Msg};
use russh::ChannelMsg;
use std::io::Read;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

const PASSWORD: &str = "integration-ssh-pass";

// ------------------------------------------------------------ test harness

struct TestServer {
    port: u16,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[allow(clippy::field_reassign_with_default)]
fn base_config(port: u16, cwd: &std::path::Path) -> AppConfig {
    let mut config = AppConfig::default();
    config.rfc_enabled = true;
    config.ssh_password.push_str(PASSWORD);
    config.ssh_port = port;
    config.allowed_paths = vec![cwd.to_string_lossy().into_owned()];
    config
}

async fn start_server(config: AppConfig) -> TestServer {
    let listener = ssh::server::bind_if_enabled(&config)
        .await
        .expect("bind ssh listener")
        .expect("listener enabled");
    let port = listener.local_addr().expect("local addr").port();
    let task = tokio::spawn(async move {
        let _ = ssh::server::serve(listener, config).await;
    });
    wait_until_listening(port).await;
    TestServer { port, task }
}

async fn wait_until_listening(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("SSH server did not start listening on port {port}");
}

async fn port_is_closed(port: u16) -> bool {
    tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_err()
}

#[derive(Clone)]
struct ClientHandler;

#[async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

async fn connect(port: u16) -> russh::client::Handle<ClientHandler> {
    client::connect(
        Arc::new(client::Config::default()),
        ("127.0.0.1", port),
        ClientHandler,
    )
    .await
    .expect("ssh connect")
}

/// Authenticates, opens a PTY at the requested geometry and starts a shell.
async fn open_shell(
    session: &mut russh::client::Handle<ClientHandler>,
    cols: u32,
    rows: u32,
) -> russh::Channel<Msg> {
    assert!(
        session
            .authenticate_password("root", PASSWORD)
            .await
            .expect("auth request"),
        "password auth must succeed"
    );
    let channel = session.channel_open_session().await.expect("open session");
    channel
        .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
        .await
        .expect("request pty");
    channel.request_shell(false).await.expect("request shell");
    channel
}

/// Reads channel output until `needle` appears (or the timeout expires).
async fn read_until(channel: &mut russh::Channel<Msg>, needle: &str, timeout: Duration) -> String {
    let mut total = Vec::new();
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, channel.wait()).await {
            Ok(Some(ChannelMsg::Data { data })) => {
                total.extend_from_slice(&data);
            }
            Ok(Some(ChannelMsg::ExtendedData { data, .. })) => {
                total.extend_from_slice(&data);
            }
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
        if String::from_utf8_lossy(&total).contains(needle) {
            break;
        }
    }
    String::from_utf8_lossy(&total).into_owned()
}

async fn run_command(channel: &mut russh::Channel<Msg>, command: &str, needle: &str) -> String {
    channel
        .data(format!("{command}\n").as_bytes())
        .await
        .expect("send command");
    read_until(channel, needle, Duration::from_secs(10)).await
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "aqvn-ssh-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe");
    listener.local_addr().expect("probe addr").port()
}

// ---------------------------------------------------------------- VAL-SSH-001

#[tokio::test]
async fn listener_only_exists_when_master_toggle_is_on() {
    let cwd = temp_dir("toggle");

    // Toggle OFF (default): no listener at all.
    let mut off = base_config(0, &cwd);
    off.rfc_enabled = false;
    assert!(
        ssh::server::bind_if_enabled(&off)
            .await
            .expect("bind check")
            .is_none(),
        "toggle OFF must not create a listener"
    );

    // Toggle ON: port listens and speaks SSH.
    let server = start_server(base_config(0, &cwd)).await;
    let banner = tokio::time::timeout(Duration::from_secs(5), async {
        use tokio::io::AsyncReadExt;
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", server.port))
            .await
            .expect("connect");
        let mut buf = [0u8; 128];
        let n = stream.read(&mut buf).await.expect("read banner");
        String::from_utf8_lossy(&buf[..n]).into_owned()
    })
    .await
    .expect("banner timeout");
    assert!(
        banner.starts_with("SSH-2.0-"),
        "expected SSH protocol banner, got {banner:?}"
    );

    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- VAL-SSH-002

#[tokio::test]
async fn password_auth_opens_pty_shell_with_client_geometry_and_cwd() {
    let cwd = temp_dir("pty-auth");
    let server = start_server(base_config(0, &cwd)).await;

    // First session: 100 cols x 50 rows.
    let mut session = connect(server.port).await;
    let mut channel = open_shell(&mut session, 100, 50).await;
    let out = run_command(&mut channel, "stty size", "50 100").await;
    println!("session1 geometry: {out:?}");
    assert!(
        out.contains("50 100"),
        "expected geometry 50 100, got {out:?}"
    );

    let out = run_command(&mut channel, "pwd", &cwd.to_string_lossy()).await;
    assert!(
        out.contains(&cwd.to_string_lossy().to_string()),
        "expected cwd {cwd:?}, got {out:?}"
    );

    let out = run_command(&mut channel, "basename $SHELL", "bash").await;
    assert!(out.contains("bash"), "expected bash shell, got {out:?}");

    let out = run_command(&mut channel, "echo rfcss-\"\"ok", "rfcss-ok").await;
    assert!(
        out.contains("rfcss-ok"),
        "echo must round-trip, got {out:?}"
    );

    // Second session: 80 cols x 24 rows.
    let mut session2 = connect(server.port).await;
    let mut channel2 = open_shell(&mut session2, 80, 24).await;
    let out2 = run_command(&mut channel2, "stty size", "24 80").await;
    println!("session2 geometry: {out2:?}");
    assert!(
        out2.contains("24 80"),
        "expected geometry 24 80, got {out2:?}"
    );

    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- VAL-SSH-003

#[tokio::test]
async fn wrong_password_is_rejected_and_server_stays_up() {
    let cwd = temp_dir("wrong-pass");
    let server = start_server(base_config(0, &cwd)).await;

    let mut bad = connect(server.port).await;
    let accepted = bad
        .authenticate_password("root", "definitely-wrong")
        .await
        .expect("auth request");
    assert!(!accepted, "wrong password must be rejected");

    // The server survives: a correct login still works.
    let mut good = connect(server.port).await;
    let mut channel = open_shell(&mut good, 80, 24).await;
    let out = run_command(&mut channel, "echo still-serving", "still-serving").await;
    assert!(
        out.contains("still-serving"),
        "server should stay up: {out:?}"
    );

    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- VAL-SSH-004

#[tokio::test]
async fn denylist_refuses_rm_rf_root_and_session_survives() {
    let cwd = temp_dir("guard-rm");
    let server = start_server(base_config(0, &cwd)).await;

    let mut session = connect(server.port).await;
    let mut channel = open_shell(&mut session, 80, 24).await;

    let refused = run_command(&mut channel, "rm -rf /", "[guard]").await;
    assert!(
        refused.contains("refused"),
        "rm -rf / must be refused, got {refused:?}"
    );

    let refused = run_command(&mut channel, "rm -fr --no-preserve-root /", "[guard]").await;
    assert!(
        refused.contains("refused"),
        "rm -fr --no-preserve-root / must be refused, got {refused:?}"
    );

    let alive = run_command(&mut channel, "echo still-alive", "still-alive").await;
    assert!(
        alive.contains("still-alive"),
        "session must survive refusals, got {alive:?}"
    );

    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- VAL-SSH-005

#[tokio::test]
async fn denylist_covers_named_destructive_population() {
    let cwd = temp_dir("guard-population");
    let server = start_server(base_config(0, &cwd)).await;

    let mut session = connect(server.port).await;
    let mut channel = open_shell(&mut session, 80, 24).await;

    for command in [
        "mkfs.ext4 /dev/sda",
        "dd if=/dev/zero of=/dev/sda",
        ":(){ :|:& };:",
        "shutdown now",
        "reboot",
    ] {
        let out = run_command(&mut channel, command, "[guard]").await;
        assert!(
            out.contains("refused"),
            "command {command:?} must be refused, got {out:?}"
        );
        let ok = run_command(&mut channel, "echo ok", "ok").await;
        assert!(
            ok.contains("ok"),
            "session must survive refusal of {command:?}, got {ok:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- VAL-SSH-006

#[tokio::test]
async fn benign_commands_pass_through_unimpeded() {
    let cwd = temp_dir("benign");
    let server = start_server(base_config(0, &cwd)).await;

    let mut session = connect(server.port).await;
    let mut channel = open_shell(&mut session, 80, 24).await;

    let out = run_command(
        &mut channel,
        "echo a && echo b; ls -la; pwd",
        &cwd.to_string_lossy(),
    )
    .await;
    assert!(out.contains("a"), "echo a output missing: {out:?}");
    assert!(out.contains("b"), "echo b output missing: {out:?}");
    assert!(
        !out.contains("[guard]"),
        "benign command was guarded: {out:?}"
    );

    let out = run_command(&mut channel, "python3 -c 'print(41+1)'", "42").await;
    assert!(out.contains("42"), "python output missing: {out:?}");

    let out = run_command(&mut channel, "seq 3 | wc -l", "3").await;
    assert!(out.contains('3'), "pipeline output missing: {out:?}");

    assert!(
        !out.contains("[guard]"),
        "pipeline must not be guarded: {out:?}"
    );

    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- VAL-SSH-009

#[tokio::test]
async fn ssh_port_is_hot_applied_without_restart() {
    let cwd = temp_dir("hot-port");
    let first_port = free_port();
    let second_port = free_port();

    let watcher = ConfigWatcher::new(base_config(first_port, &cwd));
    let supervisor_watcher = watcher.clone();
    let supervisor = tokio::spawn(async move {
        let _ = ssh::server::run_supervised(supervisor_watcher).await;
    });

    wait_until_listening(first_port).await;
    let mut session = connect(first_port).await;
    let mut channel = open_shell(&mut session, 80, 24).await;
    let out = run_command(&mut channel, "echo first-port", "first-port").await;
    assert!(out.contains("first-port"), "session failed: {out:?}");
    drop(channel);
    drop(session);

    // Same process, new port: the listener moves.
    watcher.update(base_config(second_port, &cwd));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        wait_until_listening(second_port).await;
        if port_is_closed(first_port).await {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "old port {first_port} stayed bound after hot reload"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let mut session = connect(second_port).await;
    let mut channel = open_shell(&mut session, 80, 24).await;
    let out = run_command(&mut channel, "echo second-port", "second-port").await;
    assert!(out.contains("second-port"), "new port auth failed: {out:?}");
    drop(channel);
    drop(session);

    // Reverting restores the original listener.
    watcher.update(base_config(first_port, &cwd));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        wait_until_listening(first_port).await;
        if port_is_closed(second_port).await {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "reverted port {second_port} stayed bound"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    supervisor.abort();
    let _ = std::fs::remove_dir_all(&cwd);
}

// ---------------------------------------------------------------- shell layer

/// Streams PTY output from a reader thread so the test can time out instead of
/// blocking forever on a quiet shell.
struct PtyOutput {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl PtyOutput {
    fn new(mut reader: Box<dyn Read + Send>) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Self { rx }
    }

    fn read_until(&self, needle: &str, timeout: Duration) -> String {
        let mut total = Vec::new();
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self
                .rx
                .recv_timeout(remaining.min(Duration::from_millis(250)))
            {
                Ok(chunk) => {
                    total.extend_from_slice(&chunk);
                    if String::from_utf8_lossy(&total).contains(needle) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if String::from_utf8_lossy(&total).contains(needle) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        String::from_utf8_lossy(&total).into_owned()
    }
}

#[test]
fn pty_shell_spawns_bash_with_requested_dims_and_cwd() {
    let temp = temp_dir("pty-direct");
    let shell = ssh::shell::spawn(&temp, 100, 50).expect("spawn pty shell");
    let output = PtyOutput::new(shell.take_output().expect("take output"));

    shell.write(b"stty size; pwd; echo MARKER_DONE\n");
    // "50 100" is command output only, never part of the echoed command line.
    let text = output.read_until("50 100", Duration::from_secs(15));
    assert!(
        text.contains("50 100"),
        "PTY geometry should be 50 rows 100 cols, got: {text:?}"
    );

    let _ = std::fs::remove_dir_all(&temp);
}

#[test]
fn pty_shell_resizes_to_client_geometry() {
    let temp = temp_dir("pty-resize");
    let shell = ssh::shell::spawn(&temp, 100, 50).expect("spawn pty shell");
    let output = PtyOutput::new(shell.take_output().expect("take output"));

    shell.resize(80, 24);
    shell.write(b"stty size\n");
    let text = output.read_until("24 80", Duration::from_secs(15));

    assert!(
        text.contains("24 80"),
        "PTY should resize to 24 rows 80 cols, got: {text:?}"
    );

    let _ = std::fs::remove_dir_all(&temp);
}

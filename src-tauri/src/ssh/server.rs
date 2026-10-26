//! russh server: password auth → command guard → PTY shell.
//!
//! The listener exists only while the master toggle (`rfc_enabled`) is on, and
//! rebinds whenever the configuration changes, so `ssh_port` is hot-appliable
//! without restarting the process (validation assertions VAL-SSH-001/009).

use crate::config::{AppConfig, ConfigWatcher};
use crate::ssh::guard;
use crate::ssh::session_cwd;
use crate::ssh::shell::{self, ShellSession, SHELL_PROGRAM};
use async_trait::async_trait;
use russh::server::{Auth, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId, CryptoVec};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// The bridge is a local control service; never expose it on a routable
/// interface (same posture as the RFC endpoint).
const BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Default geometry when a client opens a shell without a PTY request.
const DEFAULT_COLS: u32 = 80;
const DEFAULT_ROWS: u32 = 24;

/// Rendered client-side when the denylist refuses a command line.
fn refusal_text(reason: &str) -> Vec<u8> {
    format!("\r\n[guard] command refused: {reason}\r\n").into_bytes()
}

/// One authenticated shell channel.
struct ChannelState {
    shell: ShellSession,
    /// Bytes of the line currently being typed (mirrors the tty buffer).
    line: Vec<u8>,
}

/// Per-connection handler. Each client gets its own instance.
pub struct ClientHandler {
    password: String,
    cwd: PathBuf,
    channels: HashMap<ChannelId, ChannelState>,
    /// Geometry requested before the shell was spawned.
    pending_geometry: HashMap<ChannelId, (u32, u32)>,
}

impl ClientHandler {
    fn new(config: &AppConfig) -> Self {
        Self {
            password: config.ssh_password.clone(),
            cwd: session_cwd(&config.allowed_paths),
            channels: HashMap::new(),
            pending_geometry: HashMap::new(),
        }
    }

    /// Constant-time password comparison.
    ///
    /// An unset password rejects every attempt: enabling the toggle without
    /// configuring a password must never yield an anonymous shell.
    fn password_matches(&self, candidate: &str) -> bool {
        if self.password.is_empty() || candidate.is_empty() {
            return false;
        }
        // HMAC tag over a fixed message keyed by the candidate, verified in
        // constant time against the configured password (see rfc::auth).
        let tag = crate::rfc::auth::compute_hash(candidate, "aqvn-ssh-auth");
        crate::rfc::auth::verify(&self.password, "aqvn-ssh-auth", &tag)
    }

    /// Spawns the shell for `channel` and starts streaming its output back.
    fn start_shell(
        &mut self,
        channel: ChannelId,
        cols: u32,
        rows: u32,
        session: &mut Session,
    ) -> Result<(), russh::Error> {
        let shell = shell::spawn(&self.cwd, cols, rows).map_err(|_| russh::Error::Inconsistent)?;
        let Some(output) = shell.take_output() else {
            return Err(russh::Error::Inconsistent);
        };

        route_output(session.handle(), channel, output);
        self.channels.insert(
            channel,
            ChannelState {
                shell,
                line: Vec::new(),
            },
        );
        Ok(())
    }
}

/// Runs one completed input line through the denylist.
///
/// Refused lines never reach the shell: the pending tty buffer is killed
/// and the refusal is written straight to the client, so the session stays
/// alive and nothing executes.
fn commit_line(state: &mut ChannelState, session: &mut Session, channel: ChannelId) {
    let text = String::from_utf8_lossy(&state.line).to_string();
    state.line.clear();
    let command = sanitize(&text);
    match guard::check(&command) {
        Some(reason) => {
            state.shell.write(b"\x15".to_vec()); // Ctrl-U: discard the tty line
            session.data(channel, CryptoVec::from(refusal_text(reason)));
        }
        None => state.shell.write(b"\r".to_vec()),
    }
}

/// Streams PTY output to the SSH client until the shell exits.
fn route_output(
    handle: russh::server::Handle,
    channel: ChannelId,
    mut output: Box<dyn std::io::Read + Send>,
) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    tokio::task::spawn_blocking(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match std::io::Read::read(&mut output, &mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if tx.send(buffer[..read].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    tokio::spawn(async move {
        while let Some(bytes) = rx.recv().await {
            if handle.data(channel, CryptoVec::from(bytes)).await.is_err() {
                return;
            }
        }
        let _ = handle.eof(channel).await;
        let _ = handle.close(channel).await;
    });
}

/// Drops non-printable control bytes so a mid-line edit cannot hide a pattern.
fn sanitize(line: &str) -> String {
    line.chars().filter(|c| !c.is_control()).collect()
}

#[async_trait]
impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, _user: &str, password: &str) -> Result<Auth, Self::Error> {
        if self.password_matches(password) {
            return Ok(Auth::Accept);
        }
        Ok(Auth::Reject {
            proceed_with_methods: None,
        })
    }

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        col_width: u32,
        row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        // Honour whatever geometry the client asked for.
        let geometry = (
            non_zero(col_width, DEFAULT_COLS),
            non_zero(row_height, DEFAULT_ROWS),
        );
        if let Some(state) = self.channels.get(&channel) {
            state.shell.resize(geometry.0, geometry.1);
        } else {
            self.pending_geometry.insert(channel, geometry);
        }
        session.channel_success(channel);
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let (cols, rows) = self
            .pending_geometry
            .remove(&channel)
            .unwrap_or((DEFAULT_COLS, DEFAULT_ROWS));
        match self.start_shell(channel, cols, rows, session) {
            Ok(()) => session.channel_success(channel),
            Err(err) => {
                session.channel_failure(channel);
                return Err(err);
            }
        }
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let command = sanitize(&String::from_utf8_lossy(data));
        if let Some(reason) = guard::check(&command) {
            session.data(channel, CryptoVec::from(refusal_text(reason)));
            session.exit_status_request(channel, 1);
            session.eof(channel);
            session.close(channel);
            return Ok(());
        }

        let (cols, rows) = self
            .pending_geometry
            .remove(&channel)
            .unwrap_or((DEFAULT_COLS, DEFAULT_ROWS));
        self.start_shell(channel, cols, rows, session)?;
        if let Some(state) = self.channels.get(&channel) {
            state
                .shell
                .write(format!("{command}\r\nexit\r\n").into_bytes());
        }
        session.channel_success(channel);
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        col_width: u32,
        row_height: u32,
        _pix_width: u32,
        _pix_height: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let geometry = (
            non_zero(col_width, DEFAULT_COLS),
            non_zero(row_height, DEFAULT_ROWS),
        );
        if let Some(state) = self.channels.get(&channel) {
            state.shell.resize(geometry.0, geometry.1);
        } else {
            self.pending_geometry.insert(channel, geometry);
        }
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let Some(state) = self.channels.get_mut(&channel) else {
            return Ok(());
        };

        let mut i = 0;
        while i < data.len() {
            if let Some(pos) = data[i..].iter().position(|&b| b == b'\r' || b == b'\n') {
                let chunk = &data[i..i + pos];
                for &b in chunk {
                    if b == b'\x7f' || b == b'\x08' {
                        state.line.pop();
                    } else {
                        state.line.push(b);
                    }
                }
                state.shell.write(chunk.to_vec());
                commit_line(state, session, channel);
                i += pos + 1;
            } else {
                let chunk = &data[i..];
                for &b in chunk {
                    if b == b'\x7f' || b == b'\x08' {
                        state.line.pop();
                    } else {
                        state.line.push(b);
                    }
                }
                state.shell.write(chunk.to_vec());
                break;
            }
        }
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(state) = self.channels.get(&channel) {
            // Ask the interactive shell to terminate so the reader can finish.
            state.shell.write(b"exit\r".to_vec());
        }
        Ok(())
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(mut state) = self.channels.remove(&channel) {
            state.shell.kill();
        }
        self.pending_geometry.remove(&channel);
        Ok(())
    }
}

fn non_zero(value: u32, fallback: u32) -> u32 {
    if value == 0 {
        fallback
    } else {
        value
    }
}

/// russh server factory: one handler per client connection.
pub struct SshServer {
    config: AppConfig,
}

impl Server for SshServer {
    type Handler = ClientHandler;

    fn new_client(&mut self, _peer_addr: Option<SocketAddr>) -> Self::Handler {
        ClientHandler::new(&self.config)
    }
}

/// Binds the listener only when the master toggle is on.
///
/// The listener follows the toggle alone: the password gate is enforced per
/// connection, so an unconfigured password still yields a bound-but-unusable
/// port (every auth attempt is rejected) rather than a silent no-op.
pub async fn bind_if_enabled(config: &AppConfig) -> Result<Option<TcpListener>, String> {
    if !config.rfc_enabled {
        return Ok(None);
    }
    let address = SocketAddr::new(BIND_ADDRESS, config.ssh_port);
    let listener = TcpListener::bind(address)
        .await
        .map_err(|err| format!("SSH server failed to bind {address}: {err}"))?;
    Ok(Some(listener))
}

fn server_config() -> Result<Arc<russh::server::Config>, String> {
    let host_key = russh::keys::key::KeyPair::generate_ed25519()
        .ok_or_else(|| "failed to generate SSH host key".to_string())?;
    Ok(Arc::new(russh::server::Config {
        server_id: russh::SshId::Standard(format!(
            "SSH-2.0-aqvn-workspace_{}",
            env!("CARGO_PKG_VERSION")
        )),
        methods: russh::MethodSet::PASSWORD,
        keys: vec![host_key],
        auth_rejection_time: Duration::from_millis(100),
        inactivity_timeout: None,
        ..Default::default()
    }))
}

/// Serves connections until the listener is dropped.
pub async fn serve(listener: TcpListener, config: AppConfig) -> Result<(), String> {
    if let Ok(address) = listener.local_addr() {
        println!("SSH bridge listening on {address} (user root, PTY {SHELL_PROGRAM})");
    }
    let mut server = SshServer { config };
    let ssh_config = server_config()?;
    server
        .run_on_socket(ssh_config, &listener)
        .await
        .map_err(|err| format!("SSH server error: {err}"))
}

/// Setup-hook entry point. Does nothing when the master toggle is off.
pub async fn run(config: AppConfig) -> Result<(), String> {
    let Some(listener) = bind_if_enabled(&config).await? else {
        return Ok(());
    };
    serve(listener, config).await
}

/// Long-running supervisor: rebinds whenever the configuration changes
/// (toggle, port, password, allowed paths) while the process keeps running.
pub async fn run_supervised(watcher: ConfigWatcher) -> Result<(), String> {
    let mut receiver = watcher.subscribe();
    loop {
        let config = receiver.borrow_and_update().clone();
        let listener = match bind_if_enabled(&config).await {
            Ok(Some(listener)) => listener,
            Ok(None) => {
                if receiver.changed().await.is_err() {
                    return Ok(());
                }
                continue;
            }
            Err(err) => {
                eprintln!("{err}");
                if receiver.changed().await.is_err() {
                    return Ok(());
                }
                continue;
            }
        };

        tokio::select! {
            changed = receiver.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
                // Dropping `serve` closes the listener before the next bind.
            }
            result = serve(listener, config) => {
                return result;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler_with(password: &str) -> ClientHandler {
        let mut config = AppConfig::default();
        config.ssh_password.push_str(password);
        ClientHandler::new(&config)
    }

    #[test]
    fn correct_password_matches() {
        let handler = handler_with("s3cret-pass");
        assert!(handler.password_matches("s3cret-pass"));
    }

    #[test]
    fn wrong_password_is_rejected() {
        let handler = handler_with("s3cret-pass");
        assert!(!handler.password_matches("wrong"));
        assert!(!handler.password_matches(""));
        assert!(!handler.password_matches("s3cret-pas"));
        assert!(!handler.password_matches("s3cret-pass "));
    }

    #[test]
    fn unset_password_rejects_everything() {
        let handler = handler_with("");
        assert!(!handler.password_matches(""));
        assert!(!handler.password_matches("anything"));
    }

    #[test]
    fn refusal_text_is_visible_text() {
        let text = String::from_utf8(refusal_text("rm -rf /")).expect("utf8");
        assert!(text.contains("[guard]"));
        assert!(text.contains("refused"));
    }

    #[test]
    fn sanitize_strips_control_bytes() {
        assert_eq!(sanitize("rm\u{1}-rf /"), "rm-rf /");
    }
}

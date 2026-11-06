//! russh server: password auth → command guard → PTY shell.
//!
//! The listener exists only while the master toggle (`rfc_enabled`) is on, and
//! rebinds whenever the configuration changes, so `ssh_port` is hot-appliable
//! without restarting the process (validation assertions VAL-SSH-001/009).

use crate::config::{AppConfig, ConfigWatcher};
use crate::ssh::guard;
use crate::ssh::session_cwd;
use crate::ssh::shell::{self, ShellSession, SHELL_PROGRAM};
use russh::keys::ssh_key::private::Ed25519Keypair;
use russh::keys::PrivateKey;
use russh::server::{Auth, ChannelOpenHandle, Handler, Msg, Server, Session};
use russh::{ChannelId, ChannelOpenFailure};
use std::borrow::Cow;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

/// Cloud-reachable listener (contract ruling R2): bind the wildcard address so
/// the remote agent can reach the host at `:55022`. Authentication is the
/// boundary, and an empty `ssh_password` fails closed before any socket exists.
const BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

/// Default geometry when a client opens a shell without a PTY request.
const DEFAULT_COLS: u32 = 80;
const DEFAULT_ROWS: u32 = 24;

/// Tracks consecutive failed authentication attempts and enforces IP lockout.
#[derive(Debug, Clone)]
pub struct LockoutTracker {
    max_attempts: usize,
    lockout_duration: Duration,
    entries: Arc<Mutex<HashMap<IpAddr, LockoutEntry>>>,
}

#[derive(Debug, Clone)]
struct LockoutEntry {
    consecutive_failures: usize,
    locked_until: Option<Instant>,
}

impl Default for LockoutTracker {
    fn default() -> Self {
        Self::new(5, Duration::from_secs(900))
    }
}

impl LockoutTracker {
    pub fn new(max_attempts: usize, lockout_duration: Duration) -> Self {
        Self {
            max_attempts,
            lockout_duration,
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    pub fn lockout_duration(&self) -> Duration {
        self.lockout_duration
    }

    pub fn is_locked(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut map = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(entry) = map.get_mut(&ip) {
            if let Some(until) = entry.locked_until {
                if now < until {
                    return true;
                }
                entry.locked_until = None;
                entry.consecutive_failures = 0;
            }
        }
        false
    }

    pub fn record_failure(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut map = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Sweep expired lockout entries
        if map.len() > 64 {
            map.retain(|_, entry| {
                if let Some(until) = entry.locked_until {
                    now < until
                } else {
                    entry.consecutive_failures > 0
                }
            });
        }

        let entry = map.entry(ip).or_insert(LockoutEntry {
            consecutive_failures: 0,
            locked_until: None,
        });

        if let Some(until) = entry.locked_until {
            if now < until {
                return true;
            }
            entry.locked_until = None;
            entry.consecutive_failures = 0;
        }

        entry.consecutive_failures += 1;
        if entry.consecutive_failures >= self.max_attempts {
            entry.locked_until = Some(now + self.lockout_duration);
            true
        } else {
            false
        }
    }

    pub fn record_success(&self, ip: IpAddr) {
        let mut map = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        map.remove(&ip);
    }

    pub fn failed_attempts(&self, ip: IpAddr) -> usize {
        let map = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        map.get(&ip).map(|e| e.consecutive_failures).unwrap_or(0)
    }

    pub fn unlock(&self, ip: IpAddr) {
        let mut map = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        map.remove(&ip);
    }

    pub fn reset(&self) {
        let mut map = match self.entries.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        map.clear();
    }
}

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
    peer_ip: IpAddr,
    lockout: LockoutTracker,
}

impl ClientHandler {
    pub fn new(config: &AppConfig) -> Self {
        Self::with_lockout(
            config,
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            LockoutTracker::default(),
        )
    }

    pub fn with_lockout(config: &AppConfig, peer_ip: IpAddr, lockout: LockoutTracker) -> Self {
        Self {
            password: config.ssh_password.clone(),
            cwd: session_cwd(&config.unified_allowed_paths()),
            channels: HashMap::new(),
            pending_geometry: HashMap::new(),
            peer_ip,
            lockout,
        }
    }

    pub fn peer_ip(&self) -> IpAddr {
        self.peer_ip
    }

    pub fn lockout(&self) -> &LockoutTracker {
        &self.lockout
    }

    pub fn cwd(&self) -> &std::path::Path {
        &self.cwd
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
            let _ = session.data(channel, refusal_text(reason));
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
            if handle.data(channel, bytes).await.is_err() {
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

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, _user: &str, password: &str) -> Result<Auth, Self::Error> {
        if self.lockout.is_locked(self.peer_ip) {
            return Ok(Auth::reject());
        }

        if self.password_matches(password) {
            self.lockout.record_success(self.peer_ip);
            return Ok(Auth::Accept);
        }

        self.lockout.record_failure(self.peer_ip);
        Ok(Auth::reject())
    }

    async fn channel_open_session(
        &mut self,
        _channel: russh::Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        const MAX_ACTIVE_CHANNELS: usize = 10;
        if self.channels.len() >= MAX_ACTIVE_CHANNELS {
            reply.reject(ChannelOpenFailure::ResourceShortage).await;
            return Ok(());
        }
        reply.accept().await;
        Ok(())
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
        let _ = session.channel_success(channel);
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
            Ok(()) => {
                let _ = session.channel_success(channel);
            }
            Err(err) => {
                let _ = session.channel_failure(channel);
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
            let _ = session.data(channel, refusal_text(reason));
            let _ = session.exit_status_request(channel, 1);
            let _ = session.eof(channel);
            let _ = session.close(channel);
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
        let _ = session.channel_success(channel);
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
    lockout: LockoutTracker,
}

impl SshServer {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            lockout: LockoutTracker::default(),
        }
    }

    pub fn with_lockout(config: AppConfig, lockout: LockoutTracker) -> Self {
        Self { config, lockout }
    }

    pub fn lockout(&self) -> &LockoutTracker {
        &self.lockout
    }
}

impl Server for SshServer {
    type Handler = ClientHandler;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        let peer_ip = peer_addr
            .map(|addr| addr.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        ClientHandler::with_lockout(&self.config, peer_ip, self.lockout.clone())
    }
}

/// Binds the listener only when the master toggle is on and a password exists.
///
/// `Ok(None)` means the toggle is off: no socket is created, so the port stays
/// closed (validation assertion VAL-SSH-001). An enabled server with an
/// empty password fails closed (`Err`) instead of exposing an unauthenticated
/// wildcard SSH listener. Password auth is the real boundary; the denylist is
/// only a best-effort secondary layer.
pub async fn bind_if_enabled(config: &AppConfig) -> Result<Option<TcpListener>, String> {
    if !config.rfc_enabled {
        return Ok(None);
    }
    if config.ssh_password.trim().is_empty() {
        return Err("SSH server refused to start: no SSH password configured; \
             refusing to expose an unauthenticated wildcard listener"
            .to_string());
    }
    let address = SocketAddr::new(BIND_ADDRESS, config.ssh_port);
    let listener = TcpListener::bind(address)
        .await
        .map_err(|err| format!("SSH server failed to bind {address}: {err}"))?;
    Ok(Some(listener))
}

fn server_config() -> Result<Arc<russh::server::Config>, String> {
    // Fresh Ed25519 host key per bind (unchanged behaviour). The seed comes from
    // the existing process CSPRNG rather than adding a second RNG dependency.
    let mut seed = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut seed);
    let host_key = PrivateKey::from(Ed25519Keypair::from_seed(&seed));
    Ok(Arc::new(russh::server::Config {
        server_id: russh::SshId::Standard(Cow::Owned(format!(
            "SSH-2.0-aqvn-workspace_{}",
            env!("CARGO_PKG_VERSION")
        ))),
        methods: russh::MethodSet::from(&[russh::MethodKind::Password][..]),
        keys: vec![host_key],
        auth_rejection_time: Duration::from_millis(100),
        inactivity_timeout: Some(Duration::from_secs(600)),
        ..Default::default()
    }))
}

/// Serves connections until the listener is dropped.
pub async fn serve(listener: TcpListener, config: AppConfig) -> Result<(), String> {
    serve_with_lockout(listener, config, LockoutTracker::default()).await
}

pub async fn serve_with_lockout(
    listener: TcpListener,
    config: AppConfig,
    lockout: LockoutTracker,
) -> Result<(), String> {
    if let Ok(address) = listener.local_addr() {
        println!("SSH bridge listening on {address} (user root, PTY {SHELL_PROGRAM})");
    }
    let mut server = SshServer::with_lockout(config, lockout);
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

/// Initial exponential backoff delay for supervisor auto-recovery.
pub const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
/// Maximum exponential backoff delay for supervisor auto-recovery.
pub const MAX_BACKOFF: Duration = Duration::from_secs(10);
/// Exponential backoff growth factor.
pub const BACKOFF_FACTOR: f64 = 2.0;

/// Computes next backoff step with multiplier capped at `max`.
pub fn compute_backoff(current: Duration, factor: f64, max: Duration) -> Duration {
    let next_millis = (current.as_millis() as f64 * factor).round() as u64;
    Duration::from_millis(next_millis).min(max)
}

/// Long-running supervisor: rebinds whenever the configuration changes
/// (toggle, port, password, allowed paths) while the process keeps running.
/// On transient listener or runtime socket errors, backs off exponentially
/// (1s..10s) and retries without exiting the background task.
pub async fn run_supervised(watcher: ConfigWatcher) -> Result<(), String> {
    run_supervised_with_backoff(watcher, INITIAL_BACKOFF, MAX_BACKOFF).await
}

pub async fn run_supervised_with_backoff(
    watcher: ConfigWatcher,
    initial_backoff: Duration,
    max_backoff: Duration,
) -> Result<(), String> {
    let mut receiver = watcher.subscribe();
    let lockout = LockoutTracker::default();
    let mut current_backoff = initial_backoff;

    loop {
        let config = receiver.borrow_and_update().clone();
        let listener = match bind_if_enabled(&config).await {
            Ok(Some(listener)) => {
                current_backoff = initial_backoff;
                listener
            }
            Ok(None) => {
                current_backoff = initial_backoff;
                if receiver.changed().await.is_err() {
                    return Ok(());
                }
                continue;
            }
            Err(err) => {
                eprintln!("SSH supervisor bind error: {err}; backing off for {current_backoff:?}");
                tokio::select! {
                    changed = receiver.changed() => {
                        if changed.is_err() {
                            return Ok(());
                        }
                    }
                    _ = tokio::time::sleep(current_backoff) => {
                        current_backoff = compute_backoff(current_backoff, BACKOFF_FACTOR, max_backoff);
                    }
                }
                continue;
            }
        };

        let lockout_clone = lockout.clone();
        tokio::select! {
            changed = receiver.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
                // Dropping `serve` closes the listener before the next bind.
            }
            result = serve_with_lockout(listener, config, lockout_clone) => {
                if let Err(err) = result {
                    eprintln!("SSH server runtime socket error: {err}; backing off for {current_backoff:?}");
                    tokio::select! {
                        changed = receiver.changed() => {
                            if changed.is_err() {
                                return Ok(());
                            }
                        }
                        _ = tokio::time::sleep(current_backoff) => {
                            current_backoff = compute_backoff(current_backoff, BACKOFF_FACTOR, max_backoff);
                        }
                    }
                } else {
                    current_backoff = initial_backoff;
                }
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

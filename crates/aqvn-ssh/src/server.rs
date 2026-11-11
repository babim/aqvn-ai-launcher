//! russh server: password auth → command guard → PTY shell.

use crate::guard;
use crate::session_cwd;
use crate::shell::{self, ShellSession, SHELL_PROGRAM};
use aqvn_core::config::{AppConfig, ConfigWatcher};
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

const BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const DEFAULT_COLS: u32 = 80;
const DEFAULT_ROWS: u32 = 24;

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

fn refusal_text(reason: &str) -> Vec<u8> {
    format!("\r\n[guard] command refused: {reason}\r\n").into_bytes()
}

struct ChannelState {
    shell: ShellSession,
    line: Vec<u8>,
}

pub struct ClientHandler {
    password: String,
    cwd: PathBuf,
    allowed_paths: Vec<String>,
    channels: HashMap<ChannelId, ChannelState>,
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
        let allowed_paths = config.unified_allowed_paths();
        Self {
            password: config.ssh_password.clone(),
            cwd: session_cwd(&allowed_paths),
            allowed_paths,
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

    pub fn allowed_paths(&self) -> &[String] {
        &self.allowed_paths
    }

    fn password_matches(&self, candidate: &str) -> bool {
        if self.password.is_empty() || candidate.is_empty() {
            return false;
        }
        let tag = aqvn_rfc::auth::compute_hash(candidate, "aqvn-ssh-auth");
        aqvn_rfc::auth::verify(&self.password, "aqvn-ssh-auth", &tag)
    }

    fn start_shell(
        &mut self,
        channel: ChannelId,
        cols: u32,
        rows: u32,
        session: &mut Session,
    ) -> Result<(), russh::Error> {
        let shell = shell::spawn_with_confinement(&self.cwd, cols, rows, &self.allowed_paths)
            .map_err(|_| russh::Error::Inconsistent)?;
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

fn commit_line(state: &mut ChannelState, session: &mut Session, channel: ChannelId) {
    let text = String::from_utf8_lossy(&state.line).to_string();
    state.line.clear();
    let command = sanitize(&text);
    match guard::check(&command) {
        Some(reason) => {
            state.shell.write(b"\x15".to_vec());
            let _ = session.data(channel, refusal_text(reason));
        }
        None => state.shell.write(b"\r".to_vec()),
    }
}

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
                    } else if b == b'\x15' {
                        state.line.clear();
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
                    } else if b == b'\x15' {
                        state.line.clear();
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

#[derive(Clone)]
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

pub fn ssh_mtls_server_config(
    config: &AppConfig,
) -> Result<Option<Arc<rustls::ServerConfig>>, String> {
    if !config.mtls_enabled {
        return Ok(None);
    }
    let creds = aqvn_core::mtls::ClusterMtlsCredentials::provision_local(
        "aqvn-ssh-server",
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        &config.gateway_id,
        config.mtls_cert_ttl_hours,
    )
    .map_err(|e| format!("SSH mTLS credential error: {e}"))?;
    Ok(Some(creds.server_config))
}

pub async fn serve_mtls_tunnel(
    listener: TcpListener,
    config: AppConfig,
    tls_config: Arc<rustls::ServerConfig>,
    lockout: LockoutTracker,
) -> Result<(), String> {
    if let Ok(address) = listener.local_addr() {
        println!("SSH bridge listening on {address} (mTLS 1.3 tunnel enforced)");
    }
    let acceptor = tokio_rustls::TlsAcceptor::from(tls_config);
    let server = SshServer::with_lockout(config, lockout);
    let ssh_config = server_config()?;

    loop {
        let (tcp_stream, peer_addr) = listener
            .accept()
            .await
            .map_err(|err| format!("SSH TCP accept error: {err}"))?;

        let acceptor = acceptor.clone();
        let ssh_config = ssh_config.clone();
        let mut server_clone = server.clone();

        tokio::spawn(async move {
            match acceptor.accept(tcp_stream).await {
                Ok(tls_stream) => {
                    let handler = server_clone.new_client(Some(peer_addr));
                    let _ = russh::server::run_stream(ssh_config, tls_stream, handler).await;
                }
                Err(err) => {
                    eprintln!("SSH mTLS handshake failed from {peer_addr}: {err}");
                }
            }
        });
    }
}

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

pub async fn run(config: AppConfig) -> Result<(), String> {
    let Some(listener) = bind_if_enabled(&config).await? else {
        return Ok(());
    };
    if config.mtls_enabled {
        let tls_config = ssh_mtls_server_config(&config)?
            .ok_or_else(|| "SSH mTLS enabled but no server config generated".to_string())?;
        serve_mtls_tunnel(listener, config, tls_config, LockoutTracker::default()).await
    } else {
        serve(listener, config).await
    }
}

pub const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
pub const MAX_BACKOFF: Duration = Duration::from_secs(10);
pub const BACKOFF_FACTOR: f64 = 2.0;

pub fn compute_backoff(current: Duration, factor: f64, max: Duration) -> Duration {
    let next_millis = (current.as_millis() as f64 * factor).round() as u64;
    Duration::from_millis(next_millis).min(max)
}

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

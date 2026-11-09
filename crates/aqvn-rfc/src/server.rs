//! `/api/rfc` HTTP endpoint: authentication, dispatch, and wire-exact errors.

use aqvn_core::config::{AppConfig, ConfigWatcher};
use crate::auth;
use crate::dispatch::{self, DispatchContext, DispatchError, RfcCall};
use crate::sandbox::Sandbox;
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{header, HeaderValue, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Json, Router,
};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

const ROUTE_NAME: &str = "rfc";
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;
const BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

#[derive(Clone)]
pub struct RateLimiter {
    max_requests: usize,
    window: Duration,
    clients: Arc<Mutex<HashMap<IpAddr, VecDeque<Instant>>>>,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn default_rfc() -> Self {
        Self::new(60, Duration::from_secs(60))
    }

    pub fn max_requests(&self) -> usize {
        self.max_requests
    }

    pub fn window(&self) -> Duration {
        self.window
    }

    pub fn check_and_record(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        let mut map = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if map.len() > 64 {
            map.retain(|_, ts| {
                while let Some(&front) = ts.front() {
                    if front < cutoff {
                        ts.pop_front();
                    } else {
                        break;
                    }
                }
                !ts.is_empty()
            });
        }

        let timestamps = map.entry(ip).or_default();
        while let Some(&front) = timestamps.front() {
            if front < cutoff {
                timestamps.pop_front();
            } else {
                break;
            }
        }
        if timestamps.len() >= self.max_requests {
            return false;
        }
        timestamps.push_back(now);
        true
    }

    pub fn reset(&self) {
        let mut map = match self.clients.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        map.clear();
    }
}

fn is_trusted_proxy(ip: IpAddr) -> bool {
    ip.is_loopback()
}

pub fn extract_client_ip(request: &Request<Body>) -> IpAddr {
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip());

    if let Some(peer) = peer_ip {
        if !is_trusted_proxy(peer) {
            return peer;
        }
    }

    if let Some(forwarded) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(first) = forwarded.split(',').next() {
            if let Ok(ip) = first.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }
    if let Some(real_ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
    {
        if let Ok(ip) = real_ip.trim().parse::<IpAddr>() {
            return ip;
        }
    }

    peer_ip.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

#[derive(Clone)]
pub struct RfcState {
    pub password: String,
    pub context: DispatchContext,
    pub rate_limiter: RateLimiter,
}

impl RfcState {
    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }
}

pub fn build_state(config: &AppConfig) -> RfcState {
    build_state_with_rate_limiter(config, RateLimiter::default_rfc())
}

pub fn build_state_with_rate_limiter(config: &AppConfig, rate_limiter: RateLimiter) -> RfcState {
    RfcState {
        password: config.rfc_password.trim().to_string(),
        context: DispatchContext {
            sandbox: Sandbox::from_allowed_paths(&config.unified_allowed_paths()),
            ssh_password: config.ssh_password.clone(),
        },
        rate_limiter,
    }
}

pub fn router(state: RfcState) -> Router {
    Router::new()
        .route("/api/rfc", any(handle_rfc))
        .fallback(handle_not_found)
        .with_state(state)
}

async fn handle_rfc(State(state): State<RfcState>, request: Request<Body>) -> Response {
    let client_ip = extract_client_ip(&request);
    if !state.rate_limiter.check_and_record(client_ip) {
        return text_error(
            StatusCode::TOO_MANY_REQUESTS,
            "Too Many Requests: rate limit exceeded (>60 req/min)",
        );
    }

    if request.method() != Method::POST {
        return html_error(
            StatusCode::METHOD_NOT_ALLOWED,
            format!("Method {} not allowed for: {ROUTE_NAME}", request.method()),
        );
    }

    let body = match axum::body::to_bytes(request.into_body(), MAX_BODY_BYTES).await {
        Ok(body) => body,
        Err(err) => {
            return text_error(
                StatusCode::BAD_REQUEST,
                format!("request body could not be read: {err}"),
            )
        }
    };
    let envelope: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(err) => {
            return text_error(
                StatusCode::BAD_REQUEST,
                format!("request body is not valid JSON: {err}"),
            )
        }
    };

    let Some(rfc_input) = envelope.get("rfc_input").and_then(Value::as_str) else {
        return text_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "request body is missing the string field `rfc_input`",
        );
    };
    let hash = envelope
        .get("hash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !auth::verify(&state.password, rfc_input, hash) {
        return text_error(
            StatusCode::FORBIDDEN,
            "RFC request rejected: invalid HMAC hash",
        );
    }

    let call = match RfcCall::parse(rfc_input) {
        Ok(call) => call,
        Err(err) => return text_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    };

    match dispatch::dispatch(&state.context, &call) {
        Ok(result) => Json(result).into_response(),
        Err(DispatchError::NotWhitelisted(module, function)) => text_error(
            StatusCode::FORBIDDEN,
            format!("RFC dispatch rejected: {module}.{function} is not whitelisted"),
        ),
        Err(DispatchError::Denied(message)) => text_error(StatusCode::FORBIDDEN, message),
        Err(err) => text_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

async fn handle_not_found(request: Request<Body>) -> Response {
    let segment = request
        .uri()
        .path()
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or_default();
    html_error(
        StatusCode::NOT_FOUND,
        format!("API endpoint not found: {segment}"),
    )
}

fn text_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        message.into(),
    )
        .into_response()
}

fn html_error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        message.into(),
    )
        .into_response()
}

pub async fn bind_if_enabled(config: &AppConfig) -> Result<Option<TcpListener>, String> {
    if !config.rfc_enabled {
        return Ok(None);
    }
    let password = config.rfc_password.trim();
    if password.is_empty() {
        return Err("RFC server refused to start: no RFC password configured; \
             refusing to expose an unauthenticated listener"
            .to_string());
    }
    let address = SocketAddr::new(BIND_ADDRESS, config.rfc_port);
    let listener = TcpListener::bind(address)
        .await
        .map_err(|err| format!("RFC server failed to bind {address}: {err}"))?;
    Ok(Some(listener))
}

pub async fn serve(listener: TcpListener, state: RfcState) -> std::io::Result<()> {
    if let Ok(address) = listener.local_addr() {
        println!("RFC server listening on http://{address}/api/rfc");
    }
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}

pub async fn run(config: AppConfig) -> Result<(), String> {
    let Some(listener) = bind_if_enabled(&config).await? else {
        return Ok(());
    };
    serve(listener, build_state(&config))
        .await
        .map_err(|err| format!("RFC server error: {err}"))
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
    let rate_limiter = RateLimiter::default_rfc();
    let mut current_backoff = initial_backoff;

    loop {
        let config = receiver.borrow_and_update().clone();
        if !config.rfc_enabled {
            current_backoff = initial_backoff;
            if receiver.changed().await.is_err() {
                return Ok(());
            }
            continue;
        }

        let listener = match bind_if_enabled(&config).await {
            Ok(Some(listener)) => {
                current_backoff = initial_backoff;
                listener
            }
            Ok(None) => {
                current_backoff = initial_backoff;
                continue;
            }
            Err(err) => {
                eprintln!("RFC supervisor bind error: {err}; backing off for {current_backoff:?}");
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

        let state = build_state_with_rate_limiter(&config, rate_limiter.clone());
        tokio::select! {
            changed = receiver.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
            }
            result = serve(listener, state) => {
                if let Err(err) = result {
                    eprintln!("RFC server runtime socket error: {err}; backing off for {current_backoff:?}");
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

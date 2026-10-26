//! `/api/rfc` HTTP endpoint: authentication, dispatch, and wire-exact errors.
//!
//! Success is `200 application/json` with the bare handler result. Failures use
//! plain-text bodies; method/path errors use the exact HTML text the caller
//! expects (`Method <VERB> not allowed for: rfc`, `API endpoint not found: ...`).

use crate::config::{AppConfig, ConfigWatcher};
use crate::rfc::auth;
use crate::rfc::dispatch::{self, DispatchContext, DispatchError, RfcCall};
use crate::rfc::sandbox::Sandbox;
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Json, Router,
};
use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;

/// Route segment used in the method-not-allowed message.
const ROUTE_NAME: &str = "rfc";
/// Accepted request body size (base64 file payloads cross this endpoint).
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;
/// Cloud-reachable listener (contract ruling R2): bind the wildcard address so
/// the remote agent can reach the host. Authentication is the boundary, and an
/// empty password fails closed before any socket exists.
const BIND_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

/// Per-generation server state, cloned per request.
#[derive(Clone)]
pub struct RfcState {
    password: String,
    context: DispatchContext,
}

pub fn build_state(config: &AppConfig) -> RfcState {
    RfcState {
        password: config.rfc_password.clone(),
        context: DispatchContext {
            sandbox: Sandbox::from_allowed_paths(&config.allowed_paths),
            ssh_password: config.ssh_password.clone(),
        },
    }
}

pub fn router(state: RfcState) -> Router {
    Router::new()
        .route("/api/rfc", any(handle_rfc))
        .fallback(handle_not_found)
        .with_state(state)
}

async fn handle_rfc(State(state): State<RfcState>, request: Request<Body>) -> Response {
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

/// Binds the listener only when the master toggle is on and a password exists.
///
/// `Ok(None)` means the toggle is off: no socket is created, so the port stays
/// closed (validation assertion VAL-RFC-025). An enabled server with an empty
/// password fails closed (`Err`) instead of exposing an unauthenticated
/// wildcard listener.
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

/// Serves until the process exits.
pub async fn serve(listener: TcpListener, state: RfcState) -> std::io::Result<()> {
    if let Ok(address) = listener.local_addr() {
        println!("RFC server listening on http://{address}/api/rfc");
    }
    axum::serve(listener, router(state)).await
}

/// Setup-hook entry point. Does nothing when the master toggle is off.
pub async fn run(config: AppConfig) -> Result<(), String> {
    let Some(listener) = bind_if_enabled(&config).await? else {
        return Ok(());
    };
    serve(listener, build_state(&config))
        .await
        .map_err(|err| format!("RFC server error: {err}"))
}

/// Long-running supervisor: restarts the listener whenever the configuration
/// changes (password, allowed paths, port, master toggle) without restarting
/// the process.
pub async fn run_supervised(watcher: ConfigWatcher) -> Result<(), String> {
    let mut receiver = watcher.subscribe();
    loop {
        let config = receiver.borrow_and_update().clone();
        if !config.rfc_enabled {
            if receiver.changed().await.is_err() {
                return Ok(());
            }
            continue;
        }

        let listener = match bind_if_enabled(&config).await {
            Ok(Some(listener)) => listener,
            Ok(None) => continue,
            Err(err) => {
                eprintln!("{err}");
                if receiver.changed().await.is_err() {
                    return Ok(());
                }
                continue;
            }
        };

        let state = build_state(&config);
        tokio::select! {
            changed = receiver.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
                // Dropping `serve` closes the listener before the next bind.
            }
            result = serve(listener, state) => {
                return result.map_err(|err| format!("RFC server error: {err}"));
            }
        }
    }
}

//! Host browser automation handler bridging `connector_browser_op` to Chrome/Edge via CDP.
//!
//! Handles `open`, `navigate`, `click`, `type`, `screenshot`, `evaluate`, and
//! `open_remote_debugging` bridging to local browser on Chrome DevTools Protocol port 9222.

use crate::connector::protocol::{
    attach_id_and_op_id, extract_id_and_op_id, MSG_TYPE_BROWSER_OP_RESULT,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Duration;

pub const DEFAULT_CDP_PORT: u16 = 9222;
pub const DEFAULT_CDP_URL: &str = "http://127.0.0.1:9222";

static CUSTOM_CDP_ENDPOINT: RwLock<Option<String>> = RwLock::new(None);
static TEST_BROWSER_BINARY: RwLock<Option<PathBuf>> = RwLock::new(None);

/// Overrides the active CDP endpoint for testing.
pub fn set_test_cdp_endpoint(endpoint: Option<String>) {
    if let Ok(mut lock) = CUSTOM_CDP_ENDPOINT.write() {
        *lock = endpoint;
    }
}

/// Returns the currently active CDP endpoint.
pub fn current_cdp_endpoint() -> String {
    if let Ok(lock) = CUSTOM_CDP_ENDPOINT.read() {
        if let Some(ref ep) = *lock {
            return ep.clone();
        }
    }
    DEFAULT_CDP_URL.to_string()
}

/// Overrides the browser executable path for testing.
pub fn set_test_browser_binary(binary: Option<PathBuf>) {
    if let Ok(mut lock) = TEST_BROWSER_BINARY.write() {
        *lock = binary;
    }
}

/// Resolves a WebSocket debugger URL by matching its host and port with the CDP HTTP base URL.
pub fn resolve_ws_url(raw_ws_url: &str, cdp_base: &str) -> String {
    if let (Ok(ws_parsed), Ok(http_parsed)) =
        (url::Url::parse(raw_ws_url), url::Url::parse(cdp_base))
    {
        let mut fixed = ws_parsed;
        let scheme = match http_parsed.scheme() {
            "https" => "wss",
            _ => "ws",
        };
        let _ = fixed.set_scheme(scheme);
        let _ = fixed.set_host(http_parsed.host_str());
        let _ = fixed.set_port(http_parsed.port());
        return fixed.to_string();
    }
    raw_ws_url.to_string()
}

fn which_executable(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(name);
    if (p.is_absolute() || name.contains(std::path::MAIN_SEPARATOR)) && p.is_file() {
        return Some(p);
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let full = dir.join(name);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}

/// Locates a Chrome, Edge, or Chromium browser executable.
pub fn find_browser_executable(preferred: Option<&str>) -> Option<PathBuf> {
    if let Ok(lock) = TEST_BROWSER_BINARY.read() {
        if let Some(ref path) = *lock {
            return Some(path.clone());
        }
    }

    if let Ok(env_path) = std::env::var("AQVN_BROWSER_BIN") {
        let p = PathBuf::from(env_path);
        if p.is_file() {
            return Some(p);
        }
    }

    if let Some(pref) = preferred {
        if let Some(found) = which_executable(pref) {
            return Some(found);
        }
    }

    let candidates: Vec<&str> = if cfg!(target_os = "windows") {
        vec![
            "chrome.exe",
            "msedge.exe",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "google-chrome",
            "chromium",
        ]
    } else {
        vec![
            "google-chrome",
            "google-chrome-stable",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "microsoft-edge-stable",
            "/usr/bin/google-chrome",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
        ]
    };

    for cand in candidates {
        if let Some(found) = which_executable(cand) {
            return Some(found);
        }
        let pb = PathBuf::from(cand);
        if pb.is_file() {
            return Some(pb);
        }
    }

    None
}

/// Launches a browser process with remote debugging enabled on the specified port.
pub fn launch_browser_with_remote_debugging(
    port: u16,
    url: Option<&str>,
    browser: Option<&str>,
    custom_executable: Option<&str>,
) -> Result<Value, String> {
    let executable_path = if let Some(custom) = custom_executable {
        let p = PathBuf::from(custom);
        if p.is_file() {
            p
        } else {
            return Err(format!("specified browser executable not found: {custom}"));
        }
    } else if let Some(found) = find_browser_executable(browser) {
        found
    } else {
        return Err(
            "failed to locate Chrome or Edge browser executable on system. Set AQVN_BROWSER_BIN or provide executable path."
                .to_string(),
        );
    };

    let mut cmd = std::process::Command::new(&executable_path);
    cmd.arg(format!("--remote-debugging-port={port}"));
    cmd.arg("--remote-allow-origins=*");

    let temp_profile = std::env::temp_dir().join(format!(
        "aqvn-cdp-profile-{port}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::create_dir_all(&temp_profile);
    cmd.arg(format!("--user-data-dir={}", temp_profile.display()));
    cmd.arg("--no-first-run");
    cmd.arg("--no-default-browser-check");
    cmd.arg("--disable-background-networking");
    cmd.arg("--disable-sync");

    if let Some(target_url) = url {
        cmd.arg(target_url);
    } else {
        cmd.arg("about:blank");
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn browser {}: {e}", executable_path.display()))?;

    let pid = child.id();
    Ok(json!({
        "launched": true,
        "port": port,
        "pid": pid,
        "executable": executable_path.to_string_lossy(),
        "profile_dir": temp_profile.to_string_lossy(),
    }))
}

/// Fetches the list of active CDP targets from `/json/list` or `/json`.
pub async fn get_page_targets(cdp_url: &str) -> Result<Vec<Value>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("reqwest client error: {e}"))?;

    let base = cdp_url.trim_end_matches('/');
    let list_url = format!("{base}/json/list");
    let resp = match client.get(&list_url).send().await {
        Ok(r) => r,
        Err(_) => {
            let alt_url = format!("{base}/json");
            client
                .get(&alt_url)
                .send()
                .await
                .map_err(|e| format!("failed to connect to CDP at {base}: {e}"))?
        }
    };

    if !resp.status().is_success() {
        return Err(format!("CDP list returned status: {}", resp.status()));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| format!("failed to read CDP targets body: {e}"))?;
    let targets: Vec<Value> = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse CDP targets JSON: {e}"))?;
    Ok(targets)
}

async fn query_or_create_page_target(
    cdp_url: &str,
    fallback_url: Option<&str>,
) -> Result<String, String> {
    if let Ok(targets) = get_page_targets(cdp_url).await {
        for target in targets {
            let is_page = target
                .get("type")
                .and_then(|v| v.as_str())
                .map(|t| t == "page")
                .unwrap_or(false);
            if is_page {
                if let Some(ws_raw) = target.get("webSocketDebuggerUrl").and_then(|v| v.as_str()) {
                    return Ok(resolve_ws_url(ws_raw, cdp_url));
                }
            }
        }
    }

    // Attempt creating a new tab via /json/new
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("reqwest client error: {e}"))?;

    let base = cdp_url.trim_end_matches('/');
    let target_url = fallback_url.unwrap_or("about:blank");
    let new_url = format!("{base}/json/new?{target_url}");
    let resp = client
        .put(&new_url)
        .send()
        .await
        .map_err(|e| format!("failed to create new CDP page at {new_url}: {e}"))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("failed to read new CDP page response: {e}"))?;
    let val: Value = serde_json::from_str(&text)
        .map_err(|e| format!("failed to parse new CDP page target: {e}"))?;
    let ws_raw = val
        .get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "CDP new target response missing webSocketDebuggerUrl".to_string())?;

    Ok(resolve_ws_url(ws_raw, cdp_url))
}

/// Retrieves or creates an active page target's WebSocket debugger URL.
///
/// If connection to CDP port 9222 fails, attempts automatic local browser launch
/// and retries target discovery transparently.
pub async fn get_or_create_page_ws_url(
    cdp_url: &str,
    fallback_url: Option<&str>,
) -> Result<String, String> {
    match query_or_create_page_target(cdp_url, fallback_url).await {
        Ok(ws_url) => Ok(ws_url),
        Err(initial_err) => {
            let port = url::Url::parse(cdp_url)
                .ok()
                .and_then(|u| u.port())
                .unwrap_or(DEFAULT_CDP_PORT);

            if port == DEFAULT_CDP_PORT {
                if let Ok(_launch) =
                    launch_browser_with_remote_debugging(port, fallback_url, None, None)
                {
                    // Retries target discovery after short sleep when auto-launch succeeds
                    for _ in 0..10 {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        if let Ok(ws_url) = query_or_create_page_target(cdp_url, fallback_url).await
                        {
                            return Ok(ws_url);
                        }
                    }
                }
            }

            Err(initial_err)
        }
    }
}

/// Sends a JSON-RPC command over WebSocket to CDP and waits for the matching result.
pub async fn send_cdp_command(ws_url: &str, method: &str, params: Value) -> Result<Value, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let (mut stream, _) = tokio::time::timeout(
        Duration::from_secs(5),
        tokio_tungstenite::connect_async(ws_url),
    )
    .await
    .map_err(|_| format!("CDP websocket connect timed out: {ws_url}"))?
    .map_err(|e| format!("CDP websocket connect error to {ws_url}: {e}"))?;

    let req_id = 1u64;
    let req_payload = json!({
        "id": req_id,
        "method": method,
        "params": params,
    });

    stream
        .send(Message::Text(req_payload.to_string().into()))
        .await
        .map_err(|e| format!("failed to send CDP command {method}: {e}"))?;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while tokio::time::Instant::now() < deadline {
        let read_result = tokio::time::timeout(Duration::from_secs(5), stream.next()).await;
        match read_result {
            Ok(Some(Ok(Message::Text(text)))) => {
                if let Ok(resp) = serde_json::from_str::<Value>(text.as_str()) {
                    if resp.get("id").and_then(|v| v.as_u64()) == Some(req_id) {
                        if let Some(err) = resp.get("error") {
                            let err_msg = err
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown CDP error");
                            return Err(format!("CDP {method} error: {err_msg}"));
                        }
                        return Ok(resp.get("result").cloned().unwrap_or(Value::Null));
                    }
                }
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                return Err("CDP websocket closed unexpectedly".to_string());
            }
            Ok(Some(Err(e))) => {
                return Err(format!("CDP websocket read error: {e}"));
            }
            _ => {}
        }
    }
    Err(format!("CDP command {method} timed out"))
}

fn make_ok_reply(id: &str, action: &str, data: Value) -> String {
    json!({
        "type": MSG_TYPE_BROWSER_OP_RESULT,
        "op": MSG_TYPE_BROWSER_OP_RESULT,
        "id": id,
        "op_id": id,
        "success": true,
        "status": "ok",
        "action": action,
        "data": data.clone(),
        "result": data,
        "error": Value::Null,
    })
    .to_string()
}

fn make_error_reply(id: &str, action: &str, error_msg: impl Into<String>) -> String {
    json!({
        "type": MSG_TYPE_BROWSER_OP_RESULT,
        "op": MSG_TYPE_BROWSER_OP_RESULT,
        "id": id,
        "op_id": id,
        "success": false,
        "status": "error",
        "action": action,
        "data": Value::Null,
        "result": Value::Null,
        "error": error_msg.into(),
    })
    .to_string()
}

/// Dispatches an incoming `connector_browser_op` envelope.
pub async fn handle_browser_op(envelope: &Value) -> String {
    let (id_str, op_id_str) = extract_id_and_op_id(envelope, "browser-op-req");
    let id = id_str.as_str();
    let op_id = op_id_str.as_str();

    let params_obj = envelope.get("params");

    let action = envelope
        .get("action")
        .or_else(|| params_obj.and_then(|p| p.get("action")))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    let cdp_url = envelope
        .get("cdp_url")
        .or_else(|| params_obj.and_then(|p| p.get("cdp_url")))
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(current_cdp_endpoint);

    let raw_reply = match action {
        "open" => {
            let target_url = envelope
                .get("url")
                .or_else(|| params_obj.and_then(|p| p.get("url")))
                .and_then(|v| v.as_str())
                .unwrap_or("about:blank");

            // Try creating tab via /json/new
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(e) => return make_error_reply(id, action, format!("client build error: {e}")),
            };

            let base = cdp_url.trim_end_matches('/');
            let new_url = format!("{base}/json/new?{target_url}");
            if let Ok(resp) = client.put(&new_url).send().await {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if let Ok(created) = serde_json::from_str::<Value>(&text) {
                            return make_ok_reply(
                                id,
                                action,
                                json!({
                                    "opened": true,
                                    "url": target_url,
                                    "target": created,
                                }),
                            );
                        }
                    }
                }
            }

            // Fallback: navigate existing active page
            match get_or_create_page_ws_url(&cdp_url, Some(target_url)).await {
                Ok(ws_url) => {
                    let nav_params = json!({ "url": target_url });
                    match send_cdp_command(&ws_url, "Page.navigate", nav_params).await {
                        Ok(res) => make_ok_reply(
                            id,
                            action,
                            json!({
                                "opened": true,
                                "url": target_url,
                                "cdp_result": res,
                            }),
                        ),
                        Err(e) => make_error_reply(id, action, e),
                    }
                }
                Err(e) => make_error_reply(id, action, e),
            }
        }

        "navigate" | "goto" => {
            let target_url = match envelope
                .get("url")
                .or_else(|| params_obj.and_then(|p| p.get("url")))
                .and_then(|v| v.as_str())
            {
                Some(u) => u,
                None => return make_error_reply(id, action, "missing url for navigate"),
            };

            let ws_url = match get_or_create_page_ws_url(&cdp_url, Some(target_url)).await {
                Ok(w) => w,
                Err(e) => return make_error_reply(id, action, e),
            };

            let nav_params = json!({ "url": target_url });
            match send_cdp_command(&ws_url, "Page.navigate", nav_params).await {
                Ok(res) => make_ok_reply(
                    id,
                    action,
                    json!({
                        "navigated": true,
                        "url": target_url,
                        "frame_id": res.get("frameId"),
                        "loader_id": res.get("loaderId"),
                    }),
                ),
                Err(e) => make_error_reply(id, action, e),
            }
        }

        "click" => {
            let selector = envelope
                .get("selector")
                .or_else(|| params_obj.and_then(|p| p.get("selector")))
                .and_then(|v| v.as_str());
            let x = envelope
                .get("x")
                .or_else(|| params_obj.and_then(|p| p.get("x")))
                .and_then(|v| v.as_f64());
            let y = envelope
                .get("y")
                .or_else(|| params_obj.and_then(|p| p.get("y")))
                .and_then(|v| v.as_f64());

            let ws_url = match get_or_create_page_ws_url(&cdp_url, None).await {
                Ok(w) => w,
                Err(e) => return make_error_reply(id, action, e),
            };

            if let Some(sel) = selector {
                let script = format!(
                    r#"(() => {{
                        const el = document.querySelector({});
                        if (!el) throw new Error("Element not found: " + {});
                        el.scrollIntoView({{ block: "center", inline: "center" }});
                        el.click();
                        return true;
                    }})()"#,
                    serde_json::to_string(sel).unwrap_or_default(),
                    serde_json::to_string(sel).unwrap_or_default()
                );

                match send_cdp_command(
                    &ws_url,
                    "Runtime.evaluate",
                    json!({
                        "expression": script,
                        "returnByValue": true,
                        "awaitPromise": true,
                    }),
                )
                .await
                {
                    Ok(res) => {
                        if let Some(exc) = res.get("exceptionDetails") {
                            make_error_reply(
                                id,
                                action,
                                format!("click evaluation exception: {exc}"),
                            )
                        } else {
                            make_ok_reply(
                                id,
                                action,
                                json!({
                                    "clicked": true,
                                    "selector": sel,
                                }),
                            )
                        }
                    }
                    Err(e) => make_error_reply(id, action, e),
                }
            } else if let (Some(px), Some(py)) = (x, y) {
                // Dispatch mouse click at coordinates
                let press_params = json!({
                    "type": "mousePressed",
                    "x": px,
                    "y": py,
                    "button": "left",
                    "clickCount": 1,
                });
                let release_params = json!({
                    "type": "mouseReleased",
                    "x": px,
                    "y": py,
                    "button": "left",
                    "clickCount": 1,
                });

                let _ = send_cdp_command(&ws_url, "Input.dispatchMouseEvent", press_params).await;
                match send_cdp_command(&ws_url, "Input.dispatchMouseEvent", release_params).await {
                    Ok(_) => make_ok_reply(
                        id,
                        action,
                        json!({
                            "clicked": true,
                            "x": px,
                            "y": py,
                        }),
                    ),
                    Err(e) => make_error_reply(id, action, e),
                }
            } else {
                make_error_reply(id, action, "missing selector or coordinates for click")
            }
        }

        "type" | "input" | "fill" => {
            let text = match envelope
                .get("text")
                .or_else(|| envelope.get("value"))
                .or_else(|| params_obj.and_then(|p| p.get("text").or_else(|| p.get("value"))))
                .and_then(|v| v.as_str())
            {
                Some(t) => t,
                None => return make_error_reply(id, action, "missing text for type action"),
            };

            let selector = envelope
                .get("selector")
                .or_else(|| params_obj.and_then(|p| p.get("selector")))
                .and_then(|v| v.as_str());

            let ws_url = match get_or_create_page_ws_url(&cdp_url, None).await {
                Ok(w) => w,
                Err(e) => return make_error_reply(id, action, e),
            };

            if let Some(sel) = selector {
                let script = format!(
                    r#"(() => {{
                        const el = document.querySelector({});
                        if (!el) throw new Error("Element not found: " + {});
                        el.focus();
                        if ("value" in el) {{
                            el.value = {};
                        }} else {{
                            el.innerText = {};
                        }}
                        el.dispatchEvent(new Event("input", {{ bubbles: true }}));
                        el.dispatchEvent(new Event("change", {{ bubbles: true }}));
                        return true;
                    }})()"#,
                    serde_json::to_string(sel).unwrap_or_default(),
                    serde_json::to_string(sel).unwrap_or_default(),
                    serde_json::to_string(text).unwrap_or_default(),
                    serde_json::to_string(text).unwrap_or_default()
                );

                match send_cdp_command(
                    &ws_url,
                    "Runtime.evaluate",
                    json!({
                        "expression": script,
                        "returnByValue": true,
                        "awaitPromise": true,
                    }),
                )
                .await
                {
                    Ok(res) => {
                        if let Some(exc) = res.get("exceptionDetails") {
                            make_error_reply(
                                id,
                                action,
                                format!("type evaluation exception: {exc}"),
                            )
                        } else {
                            make_ok_reply(
                                id,
                                action,
                                json!({
                                    "typed": true,
                                    "selector": sel,
                                    "length": text.len(),
                                }),
                            )
                        }
                    }
                    Err(e) => make_error_reply(id, action, e),
                }
            } else {
                // Insert text at current active element
                match send_cdp_command(&ws_url, "Input.insertText", json!({ "text": text })).await {
                    Ok(_) => make_ok_reply(
                        id,
                        action,
                        json!({
                            "typed": true,
                            "length": text.len(),
                        }),
                    ),
                    Err(e) => make_error_reply(id, action, e),
                }
            }
        }

        "screenshot" | "capture_screenshot" => {
            let format = envelope
                .get("format")
                .or_else(|| params_obj.and_then(|p| p.get("format")))
                .and_then(|v| v.as_str())
                .unwrap_or("png");

            let ws_url = match get_or_create_page_ws_url(&cdp_url, None).await {
                Ok(w) => w,
                Err(e) => return make_error_reply(id, action, e),
            };

            let mut screenshot_params = json!({ "format": format });
            if let Some(quality) = envelope
                .get("quality")
                .or_else(|| params_obj.and_then(|p| p.get("quality")))
                .and_then(|v| v.as_u64())
            {
                screenshot_params["quality"] = json!(quality);
            }

            match send_cdp_command(&ws_url, "Page.captureScreenshot", screenshot_params).await {
                Ok(res) => {
                    let data_b64 = res.get("data").and_then(|v| v.as_str()).unwrap_or_default();
                    make_ok_reply(
                        id,
                        action,
                        json!({
                            "format": format,
                            "data": data_b64,
                            "size": data_b64.len(),
                        }),
                    )
                }
                Err(e) => make_error_reply(id, action, e),
            }
        }

        "evaluate" | "eval" | "exec_js" => {
            let expression = match envelope
                .get("expression")
                .or_else(|| envelope.get("script"))
                .or_else(|| envelope.get("code"))
                .or_else(|| {
                    params_obj.and_then(|p| {
                        p.get("expression")
                            .or_else(|| p.get("script"))
                            .or_else(|| p.get("code"))
                    })
                })
                .and_then(|v| v.as_str())
            {
                Some(expr) => expr,
                None => return make_error_reply(id, action, "missing expression for evaluate"),
            };

            let ws_url = match get_or_create_page_ws_url(&cdp_url, None).await {
                Ok(w) => w,
                Err(e) => return make_error_reply(id, action, e),
            };

            match send_cdp_command(
                &ws_url,
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                }),
            )
            .await
            {
                Ok(res) => {
                    if let Some(exc) = res.get("exceptionDetails") {
                        make_error_reply(id, action, format!("evaluation exception: {exc}"))
                    } else {
                        let eval_val = res
                            .get("result")
                            .and_then(|r| r.get("value"))
                            .cloned()
                            .unwrap_or_else(|| res.get("result").cloned().unwrap_or(Value::Null));

                        make_ok_reply(
                            id,
                            action,
                            json!({
                                "expression": expression,
                                "value": eval_val.clone(),
                                "result": eval_val,
                            }),
                        )
                    }
                }
                Err(e) => make_error_reply(id, action, e),
            }
        }

        "open_remote_debugging" | "launch_browser" | "launch" | "start_browser" => {
            let port = envelope
                .get("port")
                .or_else(|| params_obj.and_then(|p| p.get("port")))
                .and_then(|v| v.as_u64())
                .map(|p| p as u16)
                .unwrap_or(DEFAULT_CDP_PORT);

            let url = envelope
                .get("url")
                .or_else(|| params_obj.and_then(|p| p.get("url")))
                .and_then(|v| v.as_str());

            let browser = envelope
                .get("browser")
                .or_else(|| params_obj.and_then(|p| p.get("browser")))
                .and_then(|v| v.as_str());

            let custom_exec = envelope
                .get("executable")
                .or_else(|| envelope.get("browser_bin"))
                .or_else(|| {
                    params_obj.and_then(|p| p.get("executable").or_else(|| p.get("browser_bin")))
                })
                .and_then(|v| v.as_str());

            match launch_browser_with_remote_debugging(port, url, browser, custom_exec) {
                Ok(launch_data) => make_ok_reply(id, action, launch_data),
                Err(e) => make_error_reply(id, action, e),
            }
        }

        "version" => {
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
            {
                Ok(c) => c,
                Err(e) => return make_error_reply(id, action, format!("client error: {e}")),
            };
            let base = cdp_url.trim_end_matches('/');
            let version_url = format!("{base}/json/version");
            match client.get(&version_url).send().await {
                Ok(resp) => {
                    if let Ok(text) = resp.text().await {
                        if let Ok(val) = serde_json::from_str::<Value>(&text) {
                            make_ok_reply(id, action, val)
                        } else {
                            make_error_reply(id, action, "failed to parse CDP version response")
                        }
                    } else {
                        make_error_reply(id, action, "failed to read CDP version response")
                    }
                }
                Err(e) => make_error_reply(id, action, format!("failed to reach CDP version: {e}")),
            }
        }

        "targets" | "list" => match get_page_targets(&cdp_url).await {
            Ok(targets) => make_ok_reply(id, action, json!({ "targets": targets })),
            Err(e) => make_error_reply(id, action, e),
        },

        other => make_error_reply(
            id,
            other,
            format!("unknown or unsupported browser action: {other}"),
        ),
    };
    attach_id_and_op_id(raw_reply, id, op_id)
}

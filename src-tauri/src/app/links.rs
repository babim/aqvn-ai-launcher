//! External-link policy.
//!
//! App pages (the embedded `ui/` bundle) load in the webview. Any other
//! `http(s)` navigation is cancelled and handed to the system browser through
//! `tauri-plugin-shell`, so the chat surface never leaves the app origin.

use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// Embedded app page: keep it in the webview.
    App,
    /// Non-app page: open in the system browser.
    External,
    /// Unsupported scheme: refuse and stay put.
    Blocked,
}

/// Classifies a navigation target without side effects.
pub fn classify(url: &Url) -> Nav {
    classify_with_proxy(url, None)
}

/// Returns the webview container root URL pointing to the local loopback proxy.
pub fn container_url(port: u16) -> Url {
    Url::parse(&format!("http://{}:{port}/", std::net::Ipv4Addr::LOCALHOST))
        .expect("valid container root url")
}

/// Configures a Tauri window config to load the webview container from the local accelerated proxy.
pub fn configure_container_window(
    config: &tauri::utils::config::WindowConfig,
    proxy_port: u16,
) -> tauri::utils::config::WindowConfig {
    let mut config = config.clone();
    config.url = tauri::WebviewUrl::External(container_url(proxy_port));
    config
}
/// Classifies a navigation target, allowing the local bridge proxy.
pub fn classify_with_proxy(url: &Url, proxy_port: Option<u16>) -> Nav {
    let app_host = url.host_str() == Some("tauri.localhost");
    if app_host || matches!(url.scheme(), "tauri" | "about" | "data" | "blob") {
        Nav::App
    } else if let Some(port) = proxy_port {
        if url.scheme() == "http"
            && (url.host_str() == Some("127.0.0.1") || url.host_str() == Some("localhost"))
            && url.port() == Some(port)
        {
            Nav::App
        } else if matches!(url.scheme(), "http" | "https") {
            Nav::External
        } else {
            Nav::Blocked
        }
    } else if matches!(url.scheme(), "http" | "https") {
        Nav::External
    } else {
        Nav::Blocked
    }
}

/// Navigation handler for the main webview: returns whether to allow navigation.
///
/// `Shell::open` is the plugin API mandated by the architecture; it is marked
/// deprecated upstream in favour of a newer plugin but remains functional here.
#[allow(deprecated)]
pub fn handle_navigation(app: &AppHandle, url: &Url) -> bool {
    let proxy_port = app
        .try_state::<crate::proxy::server::ProxyInfo>()
        .map(|p| p.port);
    match classify_with_proxy(url, proxy_port) {
        Nav::App => true,
        Nav::External => {
            match app.shell().open(url.as_str(), None) {
                Ok(()) => eprintln!("aqvn: external link opened in system browser: {url}"),
                Err(err) => eprintln!("aqvn: external link open failed ({url}): {err}"),
            }
            false
        }
        Nav::Blocked => {
            eprintln!("aqvn: blocked navigation to unsupported scheme: {url}");
            false
        }
    }
}

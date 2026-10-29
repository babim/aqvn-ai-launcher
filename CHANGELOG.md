# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.7] - 2026-10-30

### Added
- **Upstream HTML Bridge Injection**:
  - Automatically injects `/container.js` bridge script into upstream HTML responses before `</head>` or `</body>` tags, ensuring container shortcuts, notifications, and event bridges work seamlessly even when navigating remote pages.
  - Hardened loopback proxy handling for HTML content injection with proper Content-Length recalculation.
- **Enhanced Web Container Zoom & Viewport Controls**:
  - Keyboard zoom shortcuts (`Ctrl` + `+`/`=`, `Ctrl` + `-`, `Ctrl` + `0`, and numpad keys `NumpadAdd`, `NumpadSubtract`).
  - Persistent zoom state across sessions and iframes saved in `localStorage.aqvn_zoom`.
  - Hard refresh shortcut (`Ctrl+Shift+R` and `F5`) resetting RAM state, CSRF, and cache.
- **Standalone Portable Distribution**:
  - Standalone portable binaries for Linux (`.tar.gz`) and Windows (`.portable.exe`, `.zip`) alongside existing `.deb`, `.rpm`, `.msi`, and NSIS `.exe` installers.
  - Size-gated release packaging under 10MB.

### Fixed
- **Android CI Workflow**:
  - Fixed NDK 27.x setup and environment path resolution using runner-native Android SDK in GitHub Actions.

## [0.1.6] - 2026-10-29

### Added
- **Accelerated Full-Web Container**:
  - Directly embeds the 100% full-featured upstream Web UI (Sidebar, Chat, DAG Work Map, Intel Suite Canvas, Plugins, Settings, Prompts) inside the native Tauri container.
  - Local asset pre-caching layer (`src-tauri/src/proxy/cache.rs`) caching fonts, CSS, and JS libraries to SSD/RAM, achieving sub-100ms FCP.
  - Injected container bridge script (`ui/container.js`) detecting task completions and dispatching native desktop notifications when the app is hidden or minimized.
  - Native navigation policy keeping container and frame routes in-window while external links open in system default browser.
- **Outbound Host Gateway Hub (`_a0_connector`)**:
  - Zero-config outbound WebSocket client connecting directly to workspace server over `/ws`, traversing NAT and firewalls without open inbound ports.
  - Protocol handshake `connector_hello` supporting 4 core capabilities:
    * Sandboxed remote files (`connector_file_op`) with Transfer Protocol v1 (SHA-256 chunking).
    * Remote PTY terminal execution (`connector_exec_op`) via `portable-pty`.
    * Host browser automation (`connector_browser_op`) via Chrome DevTools Protocol (CDP port 9222).
    * Desktop computer use (`connector_computer_use_op`) with screen capture and native mouse/keyboard injection.
  - Granular security scopes (files, execution, browser, desktop) and master toggle in Settings with hot-reload supervision.
- **Persistent Session Cookie Jar**:
  - Disk serialization for `CookieJar` under `$XDG_CONFIG_HOME/net.aqvn.workspace/cookies/`.
  - Preserves 30-day "Remember me" session cookies across app restarts without re-authentication.
- **Stable Loopback Proxy Port (55088)**:
  - Binds deterministic port range `55088..=55099` on loopback, ensuring a permanent origin and preserving browser `localStorage` (themes, dark mode, server state) across app restarts.
- **Official Brand Tray Icon**:
  - Uses the official 32x32 cybernetic brand icon for the system tray glyph with an active amber status badge dot during processing.
- **Background Execution (Close-to-Tray)**:
  - Minimizes to system tray upon clicking window close (`X`) with a user-configurable toggle in Settings.
- **Help Guide & Shortcut Configuration**:
  - Dedicated Help & Shortcuts guide (`ui/help.html`) accessible via `F1`, System Tray menu (`Help & Shortcuts (F1)...`), or Settings.
  - Customizable Global Summon hotkey with hot-reload in Settings.
  - Native window close IPC command (`close_current_window`) and Escape key support for auxiliary windows.
  - Bilingual interface (Tiếng Việt / English) with dynamic live switching in Settings and Help.
- **Backend Proxy Passthrough & Form Login**:
  - Raw route forwarding for `/`, `/login.css`, `/vendor/*`, `/css/*`, `/js/*`, `/plugins/*`, `/extensions/*`, `/public/*`, `/ui/*`, `/sw.js`, and fallback routes.
  - Support for both JSON and `application/x-www-form-urlencoded` POST login requests with 303 redirect handling.
- **Frontend 3-Column Layout & Offline Outbox**:
  - Dual draggable splitters, Markdown rendering with syntax highlighting, 1-click code copying, and terminal `#0d1117` styling.
  - Durable offline outbox in `localStorage` with claim-before-send protection and automatic drain on reconnect.
- **CI & Build Enhancements**:
  - Fixed Android CI workflow NDK 27.x setup using runner-native SDK.
  - Maintained 100% whitelabel compliance across code, UI, configuration, and documentation.

## [0.1.5] - 2026-10-28

### Added
- **Enterprise Native Desktop Client**: Lightweight native application powered by Tauri v2 with vanilla JS/HTML webview, completely removed of upstream branding.
- **Whitelabeled Chat Shell**: Routed through local loopback bridge proxy with transparent session cookie jar, CSRF auto-bootstrap, and resilient WebSocket with HTTP long-polling fallback.
- **Multi-Server URL Switcher & Profiles**: Server URL switching and profile presets on login and settings screens, supporting live proxy retargeting without application restarts.
- **Embedded RFC Daemon**: Local RFC daemon running on port 55080, wire-compatible with upstream RFC daemon protocol v2.14, secured with constant-time HMAC-SHA256 verification and strict directory sandbox jailing.
- **Embedded SSH Bridge Server**: Secure SSH access on port 55022 using `russh`, featuring PTY shell allocation, command denylist enforcement, and RSA key exchange.
- **OS Native Integrations**:
  - System Tray with dynamic idle and busy glyph state indicators.
  - Alt+Space global hotkey to show, hide, and summon the main window.
  - Native OS desktop notifications for background task events.
  - External link jailing via OS default browser opening.
- **Official "AI AQ" Brand Assets**: Complete cybernetic brand icon set and multi-format application icons.
- **Multi-Platform Packaging**: Distribution targets configured for Linux (.deb, .rpm), Windows (NSIS, MSI), and Android (APK).

### Fixed
- **Session Stability**:
  - Preserved session cookies across CSRF token rotations.
  - Added `NO_HANDLERS` poll fallback recovery.
  - Handled null or malformed contexts gracefully during `/api/poll` requests.

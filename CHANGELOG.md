# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-10-28

### Added
- **Backend Proxy Passthrough**:
  - Raw route forwarding for `/plugins/*`, `/extensions/*`, and `/public/*` on the loopback proxy without CSRF gating.
  - Full preservation of upstream HTTP status, headers (Content-Type, Cache-Control), query strings, and binary body bytes.
  - Transparent cookie jar session replay and upstream `Set-Cookie` synchronization on raw plugin and asset requests.
- **Frontend 3-Column Layout & Splitters**:
  - Three-column desktop layout featuring collapsible `#sidebar` and collapsible Intel Suite `#canvas-pane`.
  - Keyboard shortcuts `Alt+S` (sidebar toggle) and `Alt+C` (canvas toggle) with persistent collapse state in `localStorage`.
  - Dual draggable splitters (`#splitter-left` and `#splitter-right`) with minimum/maximum constraint clamping.
- **Markdown Rendering & Code Highlighting**:
  - Offline vendored `marked.min.js` and `highlight.min.js` assets in `ui/vendor/`.
  - Structured Markdown rendering with preserved paragraph breaks and single newlines.
  - Syntax highlighting across multiple programming languages with language badge labels and 1-click clipboard copy buttons.
  - Cyan `$ EXECUTE` terminal styling with dark `#0d1117` background for command and tool outputs.
  - Collapsible `<details class="thought-step">` containers for model reasoning inspection.
- **Distraction-Free Focus Mode**:
  - Quick toggle via `F11` and `Ctrl+Shift+F`.
  - Full-width centered chat view (840px max width) hiding peripheral sidebars and canvas panes while auto-focusing the message composer.
- **Desktop Notifications & Offline Outbox**:
  - OS-native background task completion notifications dispatched when the window is hidden.
  - 3-state connection LED pill indicator: 🟢 Online (WebSocket), 🟡 Polling (HTTP fallback), and 🔴 Offline.
  - Durable offline outbox in `localStorage` with claim-before-send protection, size and byte quotas, and automatic flush upon WebSocket reconnect.
- **UI Zoom & Hard Refresh**:
  - Keyboard zoom shortcuts `Ctrl +`, `Ctrl -`, and `Ctrl 0` within [0.7, 2.0] zoom range with persisted settings in `localStorage`.
  - Hard refresh shortcut (`Ctrl+Shift+R` / `F5`) and topbar button to reset RAM state, refresh CSRF, reconnect sockets, and reload the canvas iframe.
- **Release Packaging & Whitelabel**:
  - 100% whitelabel compliance across code, UI, configuration, and documentation.
  - Version bump to 0.1.6 across Cargo, Tauri configuration, and changelog.

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

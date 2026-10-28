# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2026-10-28

### Added
- **Enterprise Native Desktop Client**: Lightweight native application powered by Tauri v2 with vanilla JS/HTML webview, completely removed of upstream Agent Zero branding.
- **Whitelabeled Chat Shell**: Routed through local loopback bridge proxy with transparent session cookie jar, CSRF auto-bootstrap, and resilient WebSocket with HTTP long-polling fallback.
- **Multi-Server URL Switcher & Profiles**: Server URL switching and profile presets on login and settings screens, supporting live proxy retargeting without application restarts.
- **Embedded RFC Daemon**: Local RFC daemon running on port 55080, wire-compatible with Agent Zero v2.14, secured with constant-time HMAC-SHA256 verification and strict directory sandbox jailing.
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

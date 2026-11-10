# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Windows WebView2 Origin Normalization**: The local loopback proxy (`src-tauri/src/proxy/server.rs`) accepts Microsoft Edge WebView2 origins (`http://tauri.localhost`, `https://tauri.localhost`, `http://127.0.0.1:<port>`, and `http://localhost:<port>`), preventing WebSocket connection rejections on Windows (`VAL-WIN-001`).
- **Multi-Crate Cargo Workspace Architecture (`change225`)**: Re-architected monolithic `src-tauri` into a root Cargo Workspace with 6 specialized member crates:
  - `crates/aqvn-core`: Common protocol schemas, IPC models, configuration types, and telemetry primitives.
  - `crates/aqvn-rfc`: RFC host bridge HTTP server with HMAC-SHA256 constant-time verification.
  - `crates/aqvn-ssh`: Dedicated SSH listener (`russh`), RSA-OAEP authentication, and PTY shell manager.
  - `crates/aqvn-proxy`: Axum loopback bridge proxy, cookie jar, CSRF engine, and WebSocket tunnels.
  - `crates/aqvn-connector`: Outbound gateway client, Multi-Host Pool coordinator, and remote ops handlers.
  - `crates/aqvn-agent`: Standalone headless Linux daemon (2.33 MB, zero GUI dependencies).
  - `src-tauri/`: Lightweight GUI wrapper managing webview lifecycle, window title sync, system tray, and accelerators.
- **Post-Modernization Comprehensive Audit & Next-Gen Roadmap (Round 2, `change235-change242`)**:
  - `change235`: Detailed structural and DAG dependency audit of the 6 member crates (`v2_01_crates_architecture.md`).
  - `change236`: CBM graph scan across 7,066 nodes / 22,980 edges with complexity hot-spot analysis (`v2_02_cbm_graph_coverage.md`).
  - `change237`: Custom Elements lifecycle, fine-grained Signals reactivity, and DOM virtualization analysis (`v2_03_web_components_signals.md`).
  - `change238`: Empirical benchmarks of xterm.js WebGL rendering, FitAddon auto-resize, and CodeMirror 6 diff performance (`v2_04_terminal_editor_ux.md`).
  - `change239`: In-depth security verification of RFC AntiReplayWindow, Bash AST tokenizer, and HITL fail-closed semantics (`v2_05_security_ast_replay.md`).
  - `change240`: MultiHostPoolHub empirical dispatch benchmark, uniform jitter entropy, and release size gate verification (`v2_06_pool_telemetry_benchmarks.md`).
  - `change241`: Master Production Hardening Blueprint covering eBPF kernel confinement, CRDT Loro sync, and WASI plugins (`v2_production_hardening_blueprint.md`).
  - `change242`: Master Roadmap v2.0 and actionable draft change specifications `change243` through `change248` (`v2_master_roadmap_openspec.md`).
- **RFC Anti-Replay Sliding Window (`change226`)**: Enhanced `POST /api/rfc` on port 55080 with Timestamp and Nonce validation enforced via an in-memory sliding-window cache (TTL 300s, max 10,000 nonces), rejecting duplicate nonces and drifted timestamps with HTTP 401/403 (`VAL-SEC-REPLAY-001`).
- **SSH PTY AST Command Guard & Tokenizer (`change227`)**: Replaced crude regex denylist in `crates/aqvn-ssh` with a full Bash AST parser and lexer, 3-tier security classification (Safe, Sensitive, Dangerous), obfuscation de-normalizer, and 30s timeout fail-closed Human-in-the-Loop approval gate (`VAL-SEC-AST-001`).
- **Deterministic Machine Identity & Systemd Watchdog (`change228`)**: Extended `aqvn-agent` with persistent deterministic identity generation via SHA-256 (`/etc/machine-id` + primary MAC), periodic `sd_notify` watchdog heartbeat support (`WATCHDOG=1`), and automated user/system privilege resolution (`VAL-AGENT-ID-001`).
- **Multi-Host Pool Cluster Coordination (`change229`)**: Added `MultiHostPoolHub` in `crates/aqvn-connector` supporting concurrent agent node management, individual ping latency tracking, decorrelated heartbeat jitter, and multi-strategy task dispatch (`VAL-AGENT-POOL-001`).
- **W3C Web Components Shell Modernization (`change230`)**: Refactored frontend UI into modular Custom Elements (`<aqvn-shell>`, `<aqvn-topbar>`, `<aqvn-sidebar>`, `<aqvn-chat-pane>`, `<aqvn-canvas-pane>`, `<aqvn-virtual-message-list>`), integrated Centralized Signals Store (`@preact/signals-core`, 1.6 KB), and eliminated the monolithic `chat.js` architecture while preserving FCP < 35ms (`VAL-UI-COMPONENTS-001`, `VAL-UI-SIGNALS-002`).
- **xterm.js WebGL Terminal Drawer (`change231`)**: Integrated `xterm.js` 5.3.0 with WebGL hardware acceleration, FitAddon auto-resize, and 4 OKLCH theme synchronization into the collapsible `Alt+T` terminal drawer (`VAL-UI-XTERM-001`).
- **CodeMirror 6 In-App Code Editor & Diff Viewer (`change232`)**: Embedded CodeMirror 6 into the Workspace file browser drawer with syntax highlighting, safe write protection, and side-by-side visual diff inspection (`VAL-UI-EDITOR-001`).
- **Adaptive Telemetry Radar & Resilient Outbox (`change233`)**: Added topbar sparklines for CPU, RAM, and ping latency with adaptive polling (1s busy, 5s idle, 15s hidden) and transactional IndexedDB offline queueing with crash recovery (`VAL-TELEM-RADAR-001`, `VAL-OUTBOX-PERSIST-002`).
- **Container Deployment & Packaging Gates (`change234`)**: Added multi-stage `docker/Dockerfile.agent` (~15MB runtime), static UI `docker/Dockerfile.ui`, `docker/docker-compose.yml`, Taskfile recipes, and comprehensive size gate test verification (`VAL-GATE-SIZE-001`, `VAL-GATE-REGRESSION-002`).
- **Windows NSIS & MSI Installer Bundles**: Configured NSIS `.exe` and WiX `.msi` targets in `src-tauri/tauri.conf.json` with `.ico` app icon, user/machine install modes, and `downloadBootstrapper` webview install mode to enforce compliance with the `< 10 MB` binary size gate (`VAL-PKG-002`).
- **Linux deb and rpm Bundle Standardization**: Standardized bundle configurations in `src-tauri/tauri.conf.json` with `DeveloperTool` desktop category and standard icon resolutions (`256x256.png`) (`VAL-PKG-001`).
- **Headless Agent Service Management**: Extended `aqvn-agent` with service status diagnostics (`query_service_status`, `--user` and `--system` flags) and verified idempotent unit install, update, and uninstall routines (`VAL-PKG-003`).
- **PTY Buffer Backpressure Protection**: Bounded channel backpressure (`DEFAULT_PTY_BUFFER_CAPACITY = 64`) for PTY stdout streaming in `src-tauri/src/connector/handlers/exec.rs`, preventing unbounded memory growth during high-throughput terminal dumps (`VAL-DEBT-002`).
- **Standalone Headless Agent (`aqvn-agent`)**: New `[[bin]]` target (`src-tauri/src/bin/agent.rs`) providing an interactive menu, CLI flags (`--server`, `--token`, `--label`, `--scopes`, `--workspace`, `--gateway-id`, `--ping-interval`, `--tls-skip-verify`, `--config`), and `install` / `uninstall` / `status` systemd service lifecycle commands.
- **Multi-Host Pool Identity & Remote Operations**: Distinct `gateway_id` / `host_label` registration with user or `--system` systemd units, exposing `connector_exec_op` remote execution and `connector_file_op` file operations sandboxed to the configured workspace.

### Changed
- **Softened WebSocket Upstream Handshake**: On connection/handshake failure with the upstream gateway, the proxy emits close code `1001 (Going Away)` with safe UTF-8 character boundary truncation instead of `1011 (Server Error)`, enabling smooth client Socket.IO reconnection backoff without breaking the webview session (`VAL-WIN-002`).
- **Non-Blocking UI Connection Status & Retry**: Replaced intrusive startup connection error dialogs in `ui/chat.js` and `ui/container.js` with an inline non-blocking status pill and manual retry button (`#connection-retry-btn`) (`VAL-WIN-003`).
- **Dependency Security Upgrade**: `zip` 2.6.1 → 9.4.1 (`deflate-flate2-zlib-rs`), `russh` 0.45.0 → 0.65.1 with the `aws-lc-rs` backend, and the `rsa` crate removed. The RFC root-password exchange now uses `aws-lc-rs` RSA-OAEP-SHA256 with the identical wire format (PEM SPKI in, hex ciphertext out). `cargo audit` reports 0 vulnerabilities (6 non-vulnerability warnings remain for unmaintained/yanked transitive crates).
- **HTML5 Normalization**: all 92 `html-validate` errors cleared across `ui/index.html`, `ui/settings.html`, `ui/help.html` and `ui/about.html` with no rule disabled — uppercase doctype, omitted void-element slashes, inline styles moved to CSS classes, explicit input types, unique landmark labels, and native `section`/`ul`/`li` elements with the listbox role applied by the renderer. Element IDs consumed by the UI scripts are unchanged.
- **CI Quality Gates**: new `audit` job runs `cargo audit` on the lockfile; new `html` job runs the pinned `html-validate` 11.18.0 toolchain (Node 24) via `npm ci` and `npm run lint:html`. Windows release builds set `AWS_LC_SYS_PREBUILT_NASM=1` so `aws-lc-sys` assembles without a manual NASM install.

### Security
- Cleared 8 advisories: `zip` (RUSTSEC-2026-0367/0368/0369/0429, including one critical), `russh` (RUSTSEC-2026-0154), `russh-cryptovec` (RUSTSEC-2026-0153), `crypto-bigint` (RUSTSEC-2026-0387), and `rsa` (RUSTSEC-2023-0071, no patched version) by removing the crate entirely.

### Fixed
- **Clippy Lock Across Await & Formatting in Agent**: Dropped `TEST_ENV_MUTEX` guard before await points in `src-tauri/src/bin/agent.rs` to satisfy `clippy::await_holding_lock`, and formatted agent source and test files.
- **OpenSpec Change 206 Archival**: Marked completed tasks in `openspec/changes/change206-fix-clippy-as-chunks/tasks.md` and archived directory into `openspec/changes/archive/` (`VAL-DEBT-001`).
- **Single-Point UI Zoom Normalization**: Zoom is applied once on `documentElement` and any leftover `body` zoom is cleared, so `Ctrl`/`Cmd` `+`/`-` no longer compounds two zoom surfaces (`ui/chat.js`, `ui/container.js`).
- **RFC Hex Decoder Clippy 1.99 Lint**: Switched slice decoding in `src-tauri/src/rfc/auth.rs` from `chunks_exact(2)` to `as_chunks::<2>()` to satisfy rustc 1.99 lint rules.

## [0.1.9] - 2026-11-01

### Added
- **Security Hardening & Sandboxing**:
  - **IPC Capability Scheme Restriction & Strict CSP**: Scoped `shell:allow-open` capability to explicit scheme regexes (`^https?://`, `^mailto:`), preventing dangerous protocol invocation, and defined strict Content Security Policy in `tauri.conf.json` (`VAL-SEC-003`).
  - **Unified Sandbox Applied to RFC & SSH Bridges**: Bound `config.unified_allowed_paths()` across RFC HTTP handlers and embedded SSH daemon sessions, ensuring sandbox jail integrity when `workspace_path` is configured (`VAL-SEC-004`).
  - **Inbound Rate Limiting & SSH Lockout Defense**: Implemented 60 req/min sliding-window rate limiter on `/api/rfc` (returning HTTP 429) and consecutive failed authentication lockout (5 attempts) on SSH bridge (`VAL-SEC-005`).
- **Agent Hub & Transfer Resilience**:
  - **PTY Session Capacity & Background Idle Reaper**: Enforced `MAX_CONCURRENT_PTY_SESSIONS` cap of 16 in `exec.rs` and wired a 60-second periodic background reaper terminating idle sessions (>30 min) (`VAL-HUB-004`).
  - **Chunked File Write Rollback on SHA-256 Mismatch**: Automatically truncated/removed corrupted partial files upon checksum validation failure during `final: true` chunk write, reporting `rollback: true` in error payload (`VAL-HUB-005`).
- **Proxy TLS & Disk Cache Lifecycle**:
  - **Custom Root CA & TLS Skip Verify Configuration**: Added `tls_ca_cert_path` and `tls_skip_verify` to `AppConfig` and upstream proxy `reqwest::ClientBuilder` for self-hosted and private CA endpoints (`VAL-NET-003`).
  - **Bounded 256MB Disk Cache Quota & Manual Purge**: Implemented automatic LRU disk cache pruning of oldest assets and `.meta`/`.etag` files upon exceeding 256MB quota, with `clear_disk_cache` API and UI action in Settings (`VAL-CACHE-004`).
- **Cross-Platform & Operator Diagnostics**:
  - **Wayland Native Screen Capture & Input Support**: Added `WAYLAND_DISPLAY` detection integrating `grim` for screen capture and `ydotool`/`wtype` for input simulation, with explicit error diagnostics on failure (`VAL-DESK-001`).
  - **Connection Diagnostics Modal & Network Recovery Badge**: Added interactive connection badge (Connected, Reconnecting, Offline) and Diagnostics Modal in `ui/container.js` displaying proxy port, server profile, latency, and manual reconnect action (`VAL-UI-004`).
  - **Android Kotlin Test Harness & Taskfile Parity**: Added `test:android` and `test:node` tasks to `Taskfile.yml` and established unit test harness `AndroidHarnessTest.kt` under `src-tauri/gen/android/app/src/test/java/` (`VAL-MOB-006`).
- **Network & Proxy Hardening (Round 4 Upgrades)**:
  - **Upstream HTTP Client Retry with Exponential Backoff**: Automatic retry up to 3 attempts (`MAX_UPSTREAM_ATTEMPTS = 3`, initial delay 100ms doubling exponentially) on transient connection errors (`is_connect`, `is_timeout`) and a dedicated 10s connection timeout in `src-tauri/src/proxy/upstream.rs` (`VAL-NET-001`).
  - **WebSocket Heartbeat Watchdog**: 60s inactivity watchdog in `src-tauri/src/proxy/ws.rs` issuing periodic 15s ping probes and terminating hung or idle bridge legs cleanly, allowing automatic client reconnection (`VAL-NET-002`).
- **Agent Control Hub Upgrades**:
  - **PTY Session Lifecycle Commands**: Added `list_sessions` and `kill_session` operations in `src-tauri/src/connector/handlers/exec.rs`, returning active session descriptors (ID, CWD, liveness status, and idle time) and terminating child processes cleanly (`VAL-HUB-001`).
  - **CDP Multi-Tab Management & Tab Targeting**: Added `list_tabs`, `new_tab`, `activate_tab`, and `close_tab` commands in `src-tauri/src/connector/handlers/browser.rs`, with `target_id` / `tab_id` parameter support routing web interactions to designated browser tabs (`VAL-HUB-002`).
  - **Transfer Protocol v1 Full-File SHA-256 Validation**: Verified full-file SHA-256 checksum upon receiving `final: true` chunk write in `src-tauri/src/connector/handlers/files.rs`, validating against `expected_sha256` and returning `file_sha256` in payload (`VAL-HUB-003`).
- **Security & Sandboxing Unification**:
  - **Unified Allowed Paths Management**: Merged `config.allowed_paths` with `config.workspace_path` into `unified_allowed_paths()` in `src-tauri/src/connector/handlers/files.rs`, enforcing consistent jail boundary checks across multiple configured working directories (`VAL-SEC-001`).
  - **Computer Use Multi-Display Targeting & Crosshair Overlay**: Added `list_displays` and `display_id` selection parameter in `src-tauri/src/connector/handlers/computer_use.rs`, with visual high-contrast crosshair cursor overlay rendering on captured image buffers when `show_cursor: true` (`VAL-SEC-002`).
- **Operator UX & Desktop Ergonomics**:
  - **Window Geometry Persistence Across Restarts**: Persisted window position (`x`, `y`), dimensions (`width`, `height`), and maximized state (`is_maximized`) to `settings.json` on move/resize events, automatically restoring geometry upon application launch (`VAL-UX-001`).
  - **Web Audio Chime Volume & Tone Customization**: Added `task_chime_volume` (0.0..=1.0) and `task_chime_tone` (classic, bell, beep, subtle) in `ui/settings.html`, `ui/settings.js`, and `ui/container.js` with live audio preview and Web Audio synthesis (`VAL-UX-002`).
  - **System Tray Server Profiles Submenu & Workspace Launcher**: Added "Server Profiles" submenu with dynamic checkmark selection switching active server without restarting, and "Open Workspace Folder" launcher opening the system file manager at `workspace_path` (`VAL-UX-003`).
  - **Comprehensive Zero-Regression Verification**: Verified all 220+ Rust unit and integration tests and 80 Node.js test cases with zero regressions, strict clippy with warnings denied, and rustfmt check passing (`VAL-R4-CROSS-001`).
- **Remote Control & Operator Experience Polish**:
  - **PTY Child Exit Code Tracking**: Captured child process exit status on shell termination rather than hardcoded 0.
  - **Fail-Closed CWD Sandbox Enforcement**: Denied execution and returned error reply when requested working directory escapes sandbox root.
  - **Windows PowerShell Error Handling**: Combined `$LASTEXITCODE` and `$?` in `format_pty_command_line` to reliably capture native cmdlet errors.
  - **Multi-Root Remote Tree Snapshot**: Updated `generate_remote_tree_update` to traverse all configured canonical roots.
  - **File Patch SHA-256 Verification & Rollback**: Added `expected_sha256` validation and automatic rollback in `op_patch` on checksum mismatch.
  - **Stale Browser Temp Profile Pruning**: Added startup cleanup of temporary CDP user-data-dir profiles (`aqvn-cdp-profile-*`) in `/tmp`.
  - **Rate Limiter & Lockout Cache Sweeper**: Automatically pruned expired IP entries in RFC RateLimiter and SSH LockoutTracker.
  - **Double-Zoom Mitigation & Panel Hotkey Relay**: Stopped zoom key event propagation and permitted `Alt+S` and `Alt+C` shortcut relay from child iframes.
  - **Connection Diagnostics Reconnect & Badge Unification**: Added `reconnect_network` IPC command, unified topbar status badge, and wired `aqvn:reconnect` to Socket.IO.
  - **Desktop Notification Toggle**: Added user-configurable `task_notification_enabled` setting with toggle switch in Settings.
  - **Diagnostics Modal Focus Trap**: Enforced keyboard `Tab` focus trap within modal dialogs.
  - **macOS Mouse Scroll Implementation**: Implemented native macOS wheel scrolling via AppleScript System Events.

## [0.1.8] - 2026-10-31

### Added
- **Native Context Menu & Viewport Zoom Engine**:
  - Right-click native context menu with dynamic zoom level indicator (e.g. `Zoom (100%)`).
  - Native viewport scaling (`w.set_zoom(scale)`) clamping between 70% and 200% with 0.1 step size.
  - Context menu actions for Zoom In (`CmdOrCtrl+=`), Zoom Out (`CmdOrCtrl+-`), and Reset Zoom (`CmdOrCtrl+0`).
  - Full Tauri v2 ACL capabilities configuration for remote loopback origins (`http://[IP]:*` and `http://localhost:*`).
- **Window Menu Bar Toggle in Settings (Default OFF)**:
  - Added `show_menu_bar` configuration (defaulting to `false`).
  - Main window startup hides top menu bar by default (`remove_menu()`).
  - Toggle switch in Settings UI (General section) with bilingual Vietnamese/English labels.
  - Live hot-reload dynamically attaches (`set_menu`) or detaches (`remove_menu`) the menu bar on setting change without app restart.
- **Native Keyboard Shortcuts & Cross-Iframe Relay**:
  - Native `zoom_hotkeys_enabled(true)` on WebviewWindowBuilder for smooth OS-level zoom.
  - Window accelerators binding `F1` (Help), `F5` (Reload), `CmdOrCtrl+,` (Settings), `CmdOrCtrl+Shift+R` (Hard Refresh), and `F11` (Focus Mode).
  - Cross-iframe key event propagation and transparent error logging in `container.js`.
- **A0 Workspace Remote Tools & Control Compatibility**:
  - Socket.IO namespace connect handshake carrying explicit auth handlers (`plugins/_a0_connector/ws_connector`), activating remote tools in assistant prompts.
  - Standardized boolean scopes (`files`, `file_write`, `code_execution`, `browser`, `computer_use`) and `write_enabled: true` in `remote_files` metadata.
  - Dual-key tracking (`id` and `op_id`) across all operation results (`connector_*_result`), eliminating server timeout errors.
  - Flexible PTY session parsing for both integer and string session IDs.
  - Persistent machine-bound `gateway_id` in configuration, preventing `multiple_hosts` lockout across reconnects.
  - Configurable `workspace_path` in Settings with automatic periodic directory tree updates (`connector_remote_tree_update`) sent to cloud assistant.
- **Dedicated About Modal Dialog**:
  - Dedicated `About AQVN Workspace` modal dialog (`ui/about.html`) showing system info, runtime engine, gateway connector status, and copyright.
  - Added About menu item in both System Tray menu and native context menu.
- **Gateway Automation & Reliability Upgrades**:
  - Auto-launch local Chrome/Edge on port 9222 when browser automation encounters a closed CDP port.
  - PTY session reaper automatically terminating sessions inactive for over 30 minutes, with optional ANSI sequence stripping.
  - Streamed file reading using `File::seek` offsets, preventing full-file heap allocations for large files.
  - Resolution scaling and downsampling for Computer Use screenshots prior to base64 encoding.
  - Bounded in-memory `AssetCache` capped at 32MB with LRU eviction of oldest entries, plus automatic cache purge on server retarget.
  - Ephemeral test proxy port binding (port `0`) eliminating test collisions on fixed port `55088`.
- **Desktop UX & Notification Polish**:
  - Unfocused window task completion alerts triggering when `!document.hasFocus()` or when window is hidden.
  - Synthesized Web Audio dual-frequency task completion chime with toggle in General Settings (`task_chime_enabled`).
  - Dynamic System Tray status menu items displaying Gateway connection state and active server URL.
- **Cross-Platform Release Enhancements**:
  - Added macOS runner job on `macos-14` building `.dmg` and `.app` bundles in `.github/workflows/release.yml`.
  - Configured 60-minute job timeouts across all CI runner jobs.
- **Windows UI Stability & Architecture Fixes**:
  - Single-target root CSS zoom on `documentElement` without mutating `body.style.zoom`, preventing compound 1.72x visual magnification.
  - Added 150ms debounce window on zoom keyboard shortcuts (`Ctrl +/-/0`) in `container.js`, resolving multi-step zoom jumps from single keypresses on Windows WebView2.
  - Disabled `zoom_hotkeys_enabled(false)` in `src-tauri/src/lib.rs` to eliminate WebView2 engine shortcut collision with application menu accelerators.
  - Synchronized absolute target scale `applyZoom(scale)` from Rust context menu action dispatching, eliminating repeated zoom increments.
  - Added primary mouse button filter (`e.button === 0 && !e.ctrlKey`) and 300ms cooldown timestamp guard in `container.js`, preventing left-click menu popups during Win32 popup dismissal.
  - Native Windows input injection (`SendInput`) and macOS fallback in `computer_use.rs` for mouse clicks, movements, scroll, text typing, and keys.
  - Transfer Protocol v1 chunked file write with `offset` seek in `files.rs`.
  - Dynamic cache ETag generation and `304 Not Modified` validation on `If-None-Match` in proxy server.
  - Native window fullscreen control (`core:window:allow-set-fullscreen`, `toggle_fullscreen`, and `F11` shortcut).
  - Internal unit test coverage for connector handlers (`computer_use`, `files`, `exec`).

### Changed
- Refactored outbound connector WebSocket to use Socket.IO protocol framing (`/socket.io/?EIO=4&transport=websocket`).
- Isolated integration test suites to avoid port contention during parallel test runs.

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

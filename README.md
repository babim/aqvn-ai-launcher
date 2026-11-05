# AQVN Native Workspace Client & RFC Host Bridge

Enterprise-grade desktop and mobile native client built with **Tauri v2** and **Rust**, integrating an embedded RFC host bridge daemon and loopback bridge proxy for secure agent-driven desktop automation.

---

## Overview

AQVN Workspace combines three unified components into a single native binary:

1. **Accelerated Webview Container**: Embedded native desktop webview that loads the **100% full-featured Upstream Web Interface** (Sidebar, Chat, DAG Work Map, Right Canvas, Plugins, Settings, Themes, Prompts) accelerated through local asset pre-caching:
   - **Local Asset Cache**: Static assets (Inter fonts, CSS, JS libraries) are cached locally on SSD/memory, achieving instant launch times (<100ms FCP) significantly faster than loading over the public web.
   - **Desktop Notification & Tray Integration**: Native bridge detects agent task completion events and dispatches OS desktop notifications (`notify_task_complete`) when the app is hidden/minimized, updates System Tray status (Idle 🟢 / Busy 🟡), provides a dynamic **Server Profiles** submenu with checkmark selection for switching endpoints on-the-fly, and features an **Open Workspace Folder** launcher.
   - **Multi-Theme Engine**: 4 curated OKLCH themes (AQVN Dark, OLED Black, Cyberpunk Terminal, Clean Studio Light) with instant topbar toggle and anti-flicker restoration.
   - **In-Chat Search & Filtering**: `Ctrl+F` / `Cmd+F` search drawer with live match navigation (Enter / Shift+Enter) and category filter pills (`All`, `Code Execution`, `Tool Ops`, `Thoughts`, `Errors`).
   - **Transcript Export & Sharing**: Conversation export dropdown supporting Markdown (`.md`), structured JSON (`.json`), and 1-click clipboard copying.
   - **Workspace File Explorer & Uploader**: Sidebar tab switcher between Sessions and Workspace file browser (`~/Workspace`) with drag-and-drop file ingestion into chat or workspace dropzones.
   - **Interactive Terminal / PTY Drawer**: Collapsible `Alt+T` bottom drawer streaming real-time PTY terminal output with an interactive command prompt.
   - **Real-Time Topbar Telemetry**: Status pill displaying live CPU %, RAM RSS, WebSocket ping latency, and agent state (`Idle`, `Thinking`, `Executing`) with a detailed diagnostics modal.
   - **Native Navigation**: Internal app links and iframe canvases navigate inside the webview without opening an external browser; external hyperlinks open cleanly in the system default browser.
   - **Native Context Menu & Zoom Controls**: Right-click opens a native OS context menu with live scale indicator (`Zoom (100%)`), `Zoom In` (`CmdOrCtrl+=`), `Zoom Out` (`CmdOrCtrl+-`), and `Reset Zoom` (`CmdOrCtrl+0`) driving native WebviewWindow viewport scaling directly.
   - **Window & Webview Shortcuts Engine**: Native `zoom_hotkeys_enabled(false)` eliminates WebView2 double-triggering, while window menu accelerators bind `F1` (Help), `F5` (Reload), `CmdOrCtrl+,` (Settings), `CmdOrCtrl+Shift+R` (Hard Refresh), and `F11` (Focus Mode) across all focused elements including nested iframes.
   - **Window Geometry Persistence**: Window dimensions, coordinates, and maximized state are automatically stored in `settings.json` and restored on startup.
   - **Custom Web Audio Chimes**: Configurable task completion notification chime volume (`task_chime_volume`) and tone (`task_chime_tone`: classic, bell, beep, subtle) with live audio preview in Settings.
2. **Outbound Host Gateway Hub (`_a0_connector`)**: Zero-config outbound WebSocket client connecting directly to cloud workspace instances over `/ws`, traversing NAT and firewalls without requiring open inbound ports:
   - **Resilient Network & Proxy Layer**:
     - Upstream HTTP client retry with exponential backoff (`MAX_UPSTREAM_ATTEMPTS = 3`, 100ms base delay) on transient connection failures (`is_connect`, `is_timeout`) and a dedicated 10s connection timeout.
     - 60s activity timeout WebSocket heartbeat watchdog (`HEARTBEAT_INTERVAL_SECS = 15`) terminating hung or idle bridge sessions to trigger clean client reconnection.
     - Decorrelated jitter exponential backoff reconnect algorithm preventing thundering-herd spikes on network interruptions.
     - Wired `reconnect_network` Tauri command to cleanly abort gateway sessions and refresh loopback bridge state on demand.
   - **Remote Files (`connector_file_op`)**: Unified sandboxed file read, write, patch, and file-browser operations merging `config.allowed_paths` with `config.workspace_path`, supporting Transfer Protocol v1 chunked uploads with full-file SHA-256 integrity verification on final chunk.
   - **Remote Execution (`connector_exec_op`)**: Local PTY terminal session execution via `portable-pty` with session listing (`list_sessions`) and lifecycle management (`kill_session`).
   - **Host Telemetry Protocol (`connector_telemetry_op`)**: Real-time performance telemetry reporting host CPU usage percentage, RSS memory footprint, and system load.
   - **Process Inspector Protocol (`connector_process_op`)**: Process inspection protocol to query running processes, inspect resource usage, and issue targeted PID termination signals.
   - **Host Browser Automation (`connector_browser_op`)**: Chrome DevTools Protocol (CDP port 9222) bridge with multi-tab management (`list_tabs`, `new_tab`, `activate_tab`, `close_tab`) and tab-specific targeting.
   - **Host Desktop Control (`connector_computer_use_op`)**: Native screen capture with multi-display selection (`display_id`), high-contrast crosshair cursor overlay (`show_cursor: true`), and OS-native mouse/keyboard event injection.
   - **A0 Workspace Remote Tools & Control**: Full compatibility with upstream workspace assistants:
     - **Handshake Authentication**: Socket.IO namespace connect packet (`40/ws,`) carries explicit auth metadata (`plugins/_a0_connector/ws_connector`), activating remote tool definitions in agent system prompts.
     - **Normalized Scopes & Write Permissions**: Advertises standard boolean scopes (`files`, `file_write`, `code_execution`, `browser`, `computer_use`) and `write_enabled: true` for remote editing and shell execution.
     - **Operation Tracking (`op_id`)**: Dual-key propagation of `id` and `op_id` across all result envelopes (`connector_*_result`), eliminating server timeouts.
     - **Configurable Workspace Sandbox**: Configurable `workspace_path` in Settings with dynamic directory tree snapshots (`connector_remote_tree_update`) sent to the agent.
     - **Persistent Device Gateway ID**: Deterministic machine-bound gateway identifier prevents `multiple_hosts` safety lockouts across reconnects.
   - **Security Scopes**: Independent user controls to grant/revoke permissions for files, terminal, browser, and desktop.
3. **Backend RFC Host Bridge (Legacy/Direct)**: High-performance HTTP daemon (`POST /api/rfc` on port `55080`) and embedded SSH terminal server (port `55022`, powered by `russh`) for direct local network control, reinforced with supervisor auto-recovery and peer IP rate limiting.

---

## Enterprise Capabilities & Features

### 1. Multi-Theme Engine
- **4 Curated OKLCH Themes**:
  - `AQVN Dark` (default): Precision dark theme optimized for long-session visual comfort.
  - `OLED Black`: True `#000000` pitch-black mode engineered for OLED displays and battery conservation.
  - `Cyberpunk Terminal`: High-contrast retro terminal theme with phosphor greens and electric cyan highlights.
  - `Clean Studio Light`: High-clarity light theme tailored for daylight environments and document reviews.
- **Instant Toggle & Anti-Flicker Persistence**: Quick-toggle theme button in the topbar, selection dropdown in Settings, and synchronous anti-flicker CSS variable restoration on application launch.

### 2. In-Chat Search & Context Filtering
- **Keyboard Shortcut Overlay**: `Ctrl+F` / `Cmd+F` opens the in-chat search drawer above the transcript.
- **Match Navigation & Real-Time Highlighting**: Instant keyword highlighting across message nodes, match count indicator (`current / total`), and match jumping via `Enter` (next match) and `Shift+Enter` (previous match).
- **Category Filter Pills**: Fast single-click context filtering to isolate specific transcript elements:
  - `All`: Full chronological conversation flow.
  - `Code Execution`: Command executions, compiler runs, and script outputs.
  - `Tool Ops`: Tool invocations, parameters, and structured result payloads.
  - `Thoughts`: Inner reasoning traces, planning chains, and agent deliberations.
  - `Errors`: Exceptions, execution warnings, and error diagnostics.

### 3. Transcript Export & Session Sharing
- **Multi-Format Export Dropdown**:
  - **Export Markdown (`.md`)**: Formatted Markdown document with role indicators and syntax-highlighted code fences.
  - **Export JSON (`.json`)**: Structured JSON dump with message metadata, execution telemetry, and timestamps.
- **1-Click Clipboard Sharing**: One-click "Copy Transcript" button to instantly copy the conversation to the system clipboard.

### 4. Workspace File Explorer & Drag-and-Drop Uploader
- **Sidebar Tab Switcher**: Top-level tabs in the sidebar allow seamless switching between **Sessions** history and the **Workspace** file tree.
- **Live File Browser**: Real-time browsing of `~/Workspace` showcasing filenames, file sizes, and modification timestamps.
- **Drag-and-Drop Ingestion**: Drag files directly from your desktop file manager into either the chat composer or the workspace dropzone for immediate upload and attachment.

### 5. Interactive Terminal / PTY Drawer
- **Collapsible Drawer**: `Alt+T` keyboard shortcut or bottom-bar toggle button reveals an embedded PTY terminal drawer.
- **Live Output Streaming & Interactive Prompt**: Real-time terminal streaming of active PTY sessions with a manual command prompt for executing ad-hoc shell commands directly within the sandboxed environment.

### 6. Real-Time Topbar Telemetry Status Widget
- **Topbar Status Pill**: Compact live status pill in the topbar monitoring:
  - Host CPU usage percentage (`CPU: X%`).
  - Resident Set Size memory consumption (`RAM: X MB`).
  - WebSocket round-trip ping latency (`Ping: X ms`).
  - Agent activity states (`Idle` 🟢, `Thinking` 🟡, `Executing` 🔵).
- **Detailed Diagnostics Modal**: Clicking the telemetry pill opens a detailed diagnostic popup with host uptime, system metrics, and connection health statistics.

### 7. Backend Resilience & Protocol Extensions
- **Supervisor Auto-Recovery**: RFC HTTP and SSH daemon supervisors automatically detect socket dropouts or process termination, triggering automatic restarts with exponential backoff and jitter.
- **Peer IP Rate Limiting**: Inbound sliding-window rate limiters validate physical peer socket IPs on public listeners, eliminating bypasses from spoofed `X-Forwarded-For` headers.
- **Gateway Decorrelated Jitter Backoff**: Outbound WebSocket client reconnects use decorrelated jitter exponential backoff to prevent thundering herd spikes during network reconnects.
- **Host Telemetry & Process Inspector Protocols**:
  - `connector_telemetry_op`: Real-time host performance reporting (CPU, RAM, system load).
  - `connector_process_op`: Remote process manager to list active processes, check resource consumption, and trigger PID-targeted termination (`kill_session`).
- **Wired Network Reconnect IPC**: Exposes the `reconnect_network` Tauri command, safely terminating hung gateway sockets and refreshing loopback proxy cookie jars on demand.

### 8. Workspace File Explorer 2.0 & Diff Inspection
- **Hierarchical Directory Tree & Breadcrumbs**: Navigate through nested directories with breadcrumbs toolbar (`~/Workspace > path > to > dir`).
- **File Preview & In-App Editor**: Inspect Markdown with rendered HTML, code with syntax highlighting, and images. Built-in inline editor allows quick atomic saves directly to the workspace.
- **File CRUD & Directory Archive**: Create new files, folders, rename, delete, and download entire folders as compressed ZIP archives.
- **Visual Patch Diff Viewer**: Side-by-side or unified green/red diff modal inspecting agent code edits with hunk markers.

### 9. Interactive PTY Terminal Engine 2.0
- **ANSI VT100 Color Rendering**: Full support for 16-color, 256-color, RGB truecolor, bold, dim, and line clear escape sequences.
- **Command History Ring-Buffer**: Navigate past shell commands using `ArrowUp` and `ArrowDown` keys.
- **Dynamic PTY Window Resizing**: Synchronizes terminal dimensions (`cols`, `rows`) to backend `portable-pty` on window or drawer resize.
- **Multi-Tab Terminal Shells**: Add, switch, and close multiple independent shell sessions (`+ Shell`, `×` close).

### 10. Agent Activity Feed, Human-in-the-Loop Approval Gate & Audit Trail
- **Live Activity Drawer**: Topbar badge tracking active agent tool calls, displaying a real-time timeline of file reads, writes, and commands.
- **Action Approval Gate**: Prompts user confirmation before executing sensitive commands (e.g. shell execution, file deletion) with options for `Allow Once`, `Allow for Session`, or `Deny`.
- **Security Policy Presets**: Configurable policy modes (`Permissive`, `Supervised`, `Strict`) in Settings governing approval thresholds.
- **Immutable Audit Trail Export**: Comprehensive audit log of all agent operations and approval decisions, exportable to JSON or CSV.

### 11. Modern UI Polish & Multi-Server Gateway Radar
- **Toast Notifications**: Lightweight floating notification toasts with auto-dismiss (3s) for status, success, and errors.
- **Smart Scroll Anchoring**: Prevents jerky scroll jumps during message streaming when reviewing history; reveals a floating `↓ New messages below (N)` jump button.
- **Message Action Bar & Code Collapse**: Hover action toolbar on messages to copy raw Markdown or quote-reply into composer; collapsible blocks for code/terminal output over 50 lines.
- **Command Palette (`Ctrl+K`) & Shortcuts Sheet (`?`)**: Instant action palette with Arrow keyboard navigation (`role="option"`), theme switcher, and navigation shortcuts.
- **Topbar Fast Server Switcher**: Dropdown button in topbar to switch between saved server profiles on-the-fly without logging out.
- **Connection Health Radar**: Periodic latency sparkline (last 20 pings) and connection quality grading (`Excellent`, `Fair`, `Poor`) in diagnostics modal.

### 12. Phase 2 Hardening & Protocol Resilience
- **Atomic File Writes & Safe Rollback**: File operations stage incoming chunks in sibling `.tmp` files and verify full-file SHA-256 before atomic rename. Target files remain uncorrupted on mismatch.
- **Bounded Chunked Reads**: Read requests are clamped to at most 8 MB (`MAX_READ_CHUNK_BYTES`) to protect host memory.
- **Debounced Remote Tree Updates**: Trailing edge debounce (250ms) filters rapid mutations and excludes build directories (`node_modules`, `target`, `dist`, `.git`).
- **SSH Concurrency Cap & Timeout**: Embedded russh server enforces an inactivity timeout of 600 seconds and limits active channels to 10.
- **Offline Outbox Durability**: Messages composed offline are deduplicated by `message_id`, staged through in-flight storage, and safely restored across reloads.
- **Accessibility & Focus Trapping**: Modals implement Tab/Shift+Tab focus traps, high-visibility focus rings, and universal Escape dismissal.

---

## Multi-Server Support

- **Login screen**: the Server URL field is editable — prefilled from saved settings, accepts any workspace server URL.
- **Saved profiles**: add, remove, and select server profiles in Settings; selections sync with the login screen, and the active server is shown as a badge in the topbar.
- **Live retarget**: changing `server_url` retargets the loopback bridge proxy within ~500 ms (session cookie jar reset), with no app restart required.
- **Deployment**: a single client install serves many users — each enters their own server link.

---

## Architecture & Security Model

The system is designed with defense-in-depth principles:

- **Master Toggle Default OFF**: Both the RFC HTTP server (port 55080) and SSH bridge (port 55022) are closed by default. No listening socket is bound until the user explicitly toggles "Allow agent control" in Settings and configures a non-empty password.
- **Supervisor Auto-Recovery**: Both RFC HTTP and SSH daemons run under supervisor loops with exponential backoff and crash recovery, ensuring persistent bridge availability across transient failures or socket dropouts.
- **Inbound Rate Limiting & Auth Lockout**: The RFC server enforces a 60 req/min sliding-window rate limit (HTTP 429), and the SSH daemon enforces client IP lockout after 5 consecutive failed authentication attempts. Direct peer IP socket validation protects public listeners against spoofed `X-Forwarded-For` bypasses.
- **Gateway Decorrelated Jitter Backoff**: Outbound WebSocket client connections apply decorrelated jitter exponential backoff to eliminate reconnection stampedes across distributed client fleets.
- **Strict Path Sandboxing**: All incoming path arguments across RFC file operations are jailed to configured `allowed_paths` merged with `workspace_path`. Paths are canonicalized, symlink escapes are blocked, parent traversals (`..`) are denied, and virtual prefixes (`/a0/...`) resolve strictly within the sandbox root. Any sandbox boundary violation immediately yields HTTP 403 Forbidden.
- **IPC Capability Scope & CSP**: The `shell:allow-open` capability is strictly filtered to `^https?://` and `^mailto:` scheme regexes, and a strict Content Security Policy blocks unauthorized script execution.
- **Constant-Time HMAC Authentication**: Incoming RFC requests require an HMAC-SHA256 signature calculated over the exact received request payload bytes. Hashes are verified using constant-time comparison (`subtle` / `hmac::Mac::verify_slice`) to protect against timing attacks. Invalid or missing signatures return HTTP 403 Forbidden.
- **Command Denylist Protection**: Embedded SSH PTY sessions enforce a defense-in-depth command filter that inspects command lines and rejects destructive patterns (including variants of `rm -rf /`, `mkfs`, raw block writes via `dd`, fork bombs, `reboot`, and `shutdown`). Refused commands are intercepted, returning error notices to the client while preserving the terminal session and filesystem integrity.
- **Loopback Bridge Proxy**: Webview communication routes through an internal proxy listening exclusively on `127.0.0.1` on an ephemeral kernel-assigned port. The Rust proxy manages CSRF tokens and HTTP cookies, spoofs required origin headers, and forwards WebSockets for real-time streaming, preventing cookie exposure or CORS leakage in the UI layer.

---

## Prerequisites & Setup

### Linux (Host Environment)

Install the required development libraries:

- **Rust**: 1.80+ (recommended 1.90+ with `rustup`)
- **Tauri CLI**: `cargo install tauri-cli --version "^2.0"` (or via npm)
- **C Libraries**:
  - `webkit2gtk-4.1` (or `webkitgtk3-devel`)
  - `gtk3` / `gtk3-devel`
  - `libappindicator3` / `libayatana-appindicator3`
  - `openssl` / `openssl-devel`
- **Task**: `task` CLI runner (for `Taskfile.yml`)

```bash
# Verify Rust and toolchain
cargo --version
rustc --version
```

### Android Toolchain

- **Android SDK**: Platform 35 (`platforms;android-35`), build-tools 35.0.0
- **Android NDK**: 27.x (e.g. `27.3.13750724`)
- **JDK**: Java 17 (OpenJDK 17)
- **Rust Targets**: `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`

Ensure environment variables are exported (e.g., in `~/.bashrc`):

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/27.3.13750724"
export JAVA_HOME="/usr/lib64/jvm/java-17-openjdk"
export PATH="$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$JAVA_HOME/bin"
```

---

## Running the Application

### Development Mode

Run the app with live webview assets from `ui/`:

```bash
cd src-tauri
cargo tauri dev
```

### Running Release Binary Locally

```bash
./src-tauri/target/release/aqvn-workspace
```

### Headless / Xvfb Rig Execution

To launch the app headlessly for automation or CI:

```bash
Xvfb :97 -screen 0 1280x800x24 &
DISPLAY=:97 WEBKIT_DISABLE_DMABUF_RENDERER=1 ./src-tauri/target/release/aqvn-workspace
```

---

## Headless Agent (aqvn-agent)

`aqvn-agent` is a standalone headless CLI/daemon built from the same crate as the desktop client. It uses no GTK/WebKit, so it runs over SSH or on servers without `DISPLAY`.

### Build

```bash
cargo build --release --manifest-path src-tauri/Cargo.toml --bin aqvn-agent
# -> src-tauri/target/release/aqvn-agent
```

### Usage

Run with no arguments for the interactive menu:

```
1. Start (Foreground Agent Loop)     4. Uninstall Service (systemd)
2. Configure (Settings & Scopes)     5. Status (Connectivity & Diagnostics)
3. Install Service (systemd)         0. Exit
```

Non-interactive:

```bash
./src-tauri/target/release/aqvn-agent start \
  --server https://a0.matmagoc.com --token "$AQVN_TOKEN" \
  --label build-node-01 --gateway-id build-node-01 \
  --scopes files,code_execution --workspace ~/Workspace --ping-interval 25
```

Commands: `start`, `config`, `install`, `uninstall`, `status`. Flags: `-s/--server`, `-u/--user`, `-p/--pass`, `-t/--token`, `-l/--label`, `--scopes`, `-w/--workspace`, `--gateway-id`, `--ping-interval <secs>`, `--tls-skip-verify`, `-c/--config`. `--scopes` accepts `all`/`none` or a list of `files`, `code_execution`, `browser`, `computer_use` (all enabled by default).

Defaults: server `https://a0.matmagoc.com`, label `aqvn-<HOSTNAME>`, workspace `~/Workspace`, ping interval 25s. Settings file: `$XDG_CONFIG_HOME/net.aqvn.agent/settings.json`, else `~/.config/net.aqvn.agent/settings.json`; CLI flags override it.

### Service Lifecycle

```bash
aqvn-agent install                 # user unit: ~/.config/systemd/user/aqvn-agent.service
sudo aqvn-agent install --system   # system unit: /etc/systemd/system/aqvn-agent.service
aqvn-agent status
aqvn-agent uninstall               # stop, disable, remove unit file
```

The user unit is the default unless running as root, where the system unit is used; `--system` forces it.

### Multi-Host Pool

Multiple agents register under distinct `--gateway-id`/`--label`, so Agent Zero targets each host by label while remote execution (`connector_exec_op`) and file operations (`connector_file_op`) stay jailed to `--workspace`.

---

## Testing & Verification

The project provides automated tests across unit, integration, and performance layers:

### Run All Tests

```bash
# Using Taskfile
task test

# Or directly via cargo
cd src-tauri
cargo test
```

### Linting & Formatting

```bash
# Using Taskfile
task lint

# Or directly via cargo
cd src-tauri
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Running Performance Benchmarks

Run the five performance benchmark gates under an isolated Xvfb display:

```bash
# Using Taskfile
task bench

# Or directly
bash tools/benchmark.sh
```

### E2E Test Suites

- **RFC Protocol & Sandbox**: `python3 tools/spec_client.py`
- **Full RFC + SSH Terminal Flow**: `bash tools/a0_e2e.sh`
- **Bridge Proxy & WebSocket**: `bash tools/proxy_e2e.sh`
- **Chat UI Flow**: `bash tools/chat_e2e.sh`
- **Cross-Area Journeys**: `bash tools/cross_area_e2e.sh`

---

## Building Across Platforms

The project defines unified build tasks in `Taskfile.yml`:

### 1. Linux

Builds the optimized release binary and packaging bundles (`.deb` and `.rpm`):

```bash
task build:linux
```

Artifacts:
- Release binary: `src-tauri/target/release/aqvn-workspace` (size-optimized `< 8 MB`)
- Debian package: `src-tauri/target/release/bundle/deb/*.deb`
- RPM package: `src-tauri/target/release/bundle/rpm/*.rpm`

### 2. Windows

Windows installers (`nsis` and `msi`) are built via GitHub Actions CI:

```bash
task build:windows
```

*Note: Windows builds run in the automated GitHub Actions release workflow on `windows-latest` with a strict `< 10 MB` binary size gate.*

### 3. Android

Builds an unsigned debug APK for Android:

```bash
task build:android
```

Artifact:
- Universal debug APK: `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`

On Android, desktop-only modules (RFC server, SSH server, tray, global shortcut) are compiled out via Cargo target configuration, leaving a lightweight chat client.

---

## Known Limitations

- **Wayland Global Hotkey**: The global shortcut `Alt+Space` utilizes `global-hotkey` (0.8), which relies on X11 `XGrabKey`. Under native Wayland compositors that prohibit application-level global key grabs, global hotkey registration will fail gracefully. Use the **System Tray menu Show/Hide toggle item** as the supported fallback on Wayland.
- **Linux System Tray**: On Linux desktop environments, clicking the system tray icon does not emit click toggle events. Window visibility is toggled via the tray context menu item (`Show / Hide window`).
- **CI-Deferred Windows & Actions Runtime**: Windows binaries (`.exe`, `.msi`) and GitHub Actions release artifact creation are triggered automatically on tag pushes (`v*`) in `.github/workflows/release.yml`. Local cross-compilation for Windows is deferred to the CI environment.
- **Android APK Signing**: Local builds generate unsigned debug APKs (`app-universal-debug.apk`). Release keystore signing is configured in production release pipelines.

---

## License & Whitelabel Notice

This software is an enterprise client for the AQVN platform. All branding, identifiers, window titles, icons, and package namespaces are configured for `AQVN Workspace` (`net.aqvn.workspace`).

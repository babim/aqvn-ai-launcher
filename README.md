# AQVN Native Workspace Client & RFC Host Bridge

Enterprise-grade desktop and mobile native client built with **Tauri v2** and **Rust**, integrating an embedded RFC host bridge daemon and loopback bridge proxy for secure agent-driven desktop automation.

---

## Overview

AQVN Workspace combines two unified components into a single native binary:

1. **Frontend Client**: Embedded vanilla HTML/CSS/JavaScript webview (no Node/bundler runtime dependency), featuring:
   - **3-Column Workspace Layout**: Collapsible Sidebar (`Alt+S`), Core Chat, and Right Canvas Host (`Alt+C`) embedding AQVN Intel Orchestrator (`/plugins/aqvn_intel_suite/webui/canvas.html`) via drag-resizable splitters.
   - **Markdown & Code Highlighting**: In-repo vendored `marked.min.js` and `highlight.min.js` with 1-click code copying, terminal hacker theme (`#0d1117`), and collapsible `<details class="thought-step">` thought sections.
   - **Focus Mode**: Distraction-free full-screen chat (`F11` / `Ctrl+Shift+F`), auto-centering and focusing composer.
   - **System Integration & Offline Resilience**: OS notifications on background task completion, 3-state glowing LED indicator (🟢 Online / 🟡 Polling / 🔴 Offline), persistent offline outbox queue (`aqvn_outbox`) with automatic replay on reconnect, smooth UI Zoom (`Ctrl +/-/0`), and Hard Refresh (`Ctrl+Shift+R` / `F5`).
   - Native window title synchronization (`AQVN AI Workspace`), system tray integration with busy/idle state indicators, and a loopback bridge proxy.
2. **Backend RFC Host Bridge**: High-performance HTTP daemon (`POST /api/rfc` on port `55080`) and embedded SSH terminal server (port `55022`, powered by `russh`) allowing verified cloud agents to inspect files, browse workspace directories, and execute terminal sessions within an isolated filesystem sandbox.
   - **Raw Forwarding Proxy**: The Axum loopback proxy seamlessly routes `/plugins/*`, `/extensions/*`, and `/public/*` directly to the upstream server with automated session cookie jar replay and origin spoofing.

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
- **Strict Path Sandboxing**: All incoming path arguments across RFC file operations are jailed to configured `allowed_paths` (default `~/Workspace`). Paths are canonicalized, symlink escapes are blocked, parent traversals (`..`) are denied, and virtual prefixes (`/a0/...`) resolve strictly within the sandbox root. Any sandbox boundary violation immediately yields HTTP 403 Forbidden.
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

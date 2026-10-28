# AQVN Native Workspace Client & RFC Host Bridge

Enterprise-grade desktop and mobile native client built with **Tauri v2** and **Rust**, integrating an embedded RFC host bridge daemon and loopback bridge proxy for secure agent-driven desktop automation.

---

## Overview

AQVN Workspace combines three unified components into a single native binary:

1. **Accelerated Webview Container**: Embedded native desktop webview that loads the **100% full-featured Upstream Web Interface** (Sidebar, Chat, DAG Work Map, Right Canvas, Plugins, Settings, Themes, Prompts) accelerated through local asset pre-caching:
   - **Local Asset Cache**: Static assets (Inter fonts, CSS, JS libraries) are cached locally on SSD/memory, achieving instant launch times (<100ms FCP) significantly faster than loading over the public web.
   - **Desktop Notification & Tray Integration**: Native bridge detects agent task completion events and dispatches OS desktop notifications (`notify_task_complete`) when the app is hidden/minimized, and updates System Tray status (Idle 🟢 / Busy 🟡).
   - **Native Navigation**: Internal app links and iframe canvases navigate inside the webview without opening an external browser; external hyperlinks open cleanly in the system default browser.
2. **Outbound Host Gateway Hub (`_a0_connector`)**: Zero-config outbound WebSocket client connecting directly to cloud workspace instances over `/ws`, traversing NAT and firewalls without requiring open inbound ports:
   - **Remote Files (`connector_file_op`)**: Sandboxed file read, write, patch, and file-browser operations within `~/Workspace`, with Transfer Protocol v1 support (SHA-256 validation).
   - **Remote Execution (`connector_exec_op`)**: Local PTY terminal session execution via `portable-pty`.
   - **Host Browser Automation (`connector_browser_op`)**: Chrome DevTools Protocol (CDP port 9222) bridge for web navigation, element clicks, typing, and screenshots.
   - **Host Desktop Control (`connector_computer_use_op`)**: Native screen capture and mouse/keyboard event injection.
   - **Security Scopes**: Independent user controls to grant/revoke permissions for files, terminal, browser, and desktop.
3. **Backend RFC Host Bridge (Legacy/Direct)**: High-performance HTTP daemon (`POST /api/rfc` on port `55080`) and embedded SSH terminal server (port `55022`, powered by `russh`) for direct local network control.

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

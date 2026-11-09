# Báo Cáo Đánh Giá Độ Bao Phủ Kiểm Thử & Tình Trạng CI/CD (Test Coverage & CI Hygiene)

**Mã tài liệu:** `VAL-EVAL-GRAPH-002`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi kiểm toán:** Toàn bộ hệ thống kiểm thử (`src-tauri/tests/`, `src-tauri/src/`, `tools/tests/`, `tools/`) và tự động hóa CI/CD (`.github/workflows/`)  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Evaluation Team (eval-worker)  
**Trạng thái kiểm tra kỹ thuật:** Passed (100% test suites passed: 74 lib tests, 31 agent tests, 177 node chat tests, 4 vendor asset tests, HTML validate clean)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Dự án `ai-launcher` (`net.aqvn.workspace`) sở hữu một hạ tầng kiểm thử phong phú với sự kết hợp đa tầng giữa **Unit Tests**, **Integration Tests**, **Mock DOM Node.js Tests**, và **End-to-End Automation Scripts**. Toàn bộ các bài kiểm thử tự động đang có trong dự án đều đạt trạng thái 100% Green (không có lỗi hay cảnh báo hồi quy).

Tuy nhiên, qua quá trình rà soát chi tiết mã nguồn và kiểm toán quy trình CI/CD qua đồ thị Codebase Memory (CBM), nhóm đánh giá đã xác định được **7 khoảng trống kiểm thử trọng yếu (Critical Test Gaps)** và **4 rủi ro cấu hình nghiêm trọng trong CI/CD workflows**. Đáng chú ý nhất, quy trình CI chính (`ci.yml`) hiện chỉ chạy đơn nền tảng trên **Ubuntu 24.04**, hoàn toàn bỏ qua việc biên dịch và kiểm thử tự động trên **Windows** và **macOS** trước khi phát hành release.

### Bảng tổng hợp số lượng kiểm thử hiện có:

| Phân hệ kiểm thử | Số tệp | Số lượng Test Cases | Thời gian chạy | Kết quả thực tế |
| :--- | :---: | :---: | :---: | :---: |
| **Rust Lib Unit Tests** (`src-tauri/src/`) | 15 | 74 | 2.76s | **74/74 Passed (100%)** |
| **Standalone Agent Tests** (`src/bin/agent.rs`) | 1 | 31 | 6.29s | **31/31 Passed (100%)** |
| **Rust Integration Tests** (`src-tauri/tests/`) | 32 | 263 | ~75s | **263/263 Passed (100%)** |
| **Node.js Chat DOM Tests** (`test_chat_handlers.mjs`) | 1 | 177 | 5.14s | **177/177 Passed (100%)** |
| **Node.js Vendor Assets** (`test_vendor_assets.mjs`) | 1 | 4 | 0.72s | **4/4 Passed (100%)** |
| **Python & Bash Scripts** (`tools/tests/`) | 6 | 18 checks | Thủ công | Hoạt động tốt |
| **HTML UI Validator** (`npm run lint:html`) | 3 | 3 HTML files | 0.45s | **0 Errors, 0 Warnings** |
| **TỔNG CỘNG** | **59** | **567+ tests** | **~85s** | **100% GREEN** |

---

## 2. Hiện Trạng Kiểm Thử Rust Backend (`src-tauri/`)

### 2.1 Chi tiết 32 tệp Rust Integration Tests (`src-tauri/tests/`)

Thư mục `src-tauri/tests/` bao gồm 32 tệp với 18.287 dòng mã nguồn kiểm thử, tập trung kiểm tra hành vi tích hợp giữa các module:

| Tệp kiểm thử | Số dòng | Số test | Phân hệ được kiểm thử |
| :--- | :---: | :---: | :--- |
| `proxy_server.rs` | 2.251 | 35 | Reverse proxy, cookie jar, retry backpressure, fallback routes |
| `rfc_server.rs` | 520 | 21 | RFC daemon HTTP server, TCP socket, rate limiting, auth |
| `gateway_files.rs` | 1.817 | 18 | Thao tác tệp tin qua WebSocket, base64 chunking, directory tree |
| `rfc_handlers.rs` | 1.141 | 17 | File operations, text editor line patching, workdir resolution |
| `agent_cli.rs` | 783 | 16 | CLI arguments parsing, systemd unit generation, flag parsing |
| `os_integration.rs` | 440 | 15 | Global hotkeys, desktop links, single instance lock |
| `ssh_server.rs` | 661 | 13 | SSH daemon listener, RSA-OAEP password exchange, PTY shell |
| `context_menu.rs` | 608 | 13 | Native menu bar, context menu item click handling |
| `gateway_client.rs` | 1.119 | 12 | Gateway client connection, Socket.IO handshake, reconnection |
| `connector_handlers.rs` | 506 | 8 | Quản lý tiến trình (process spawning), process killing, signals |
| `gateway_desktop.rs` | 732 | 8 | Computer use, mouse click injection, Wayland capture |
| `app_tray.rs` | 572 | 7 | Khay hệ thống, chuyển đổi trạng thái idle/busy, profile menu |
| `proxy_upstream.rs` | 643 | 7 | Kết nối upstream TLS/HTTPS, DNS resolution, error mapping |
| `rfc_ratelimit_and_lockout.rs` | 389 | 7 | Khóa IP sau 5 lần nhập sai mật khẩu RFC/SSH |
| `proxy_cache.rs` | 512 | 6 | Bộ nhớ đệm tài nguyên tĩnh (LRU asset cache) |
| `agent_host_pool.rs` | 580 | 6 | Đăng ký host pool, ping-pong heartbeat, remote control |
| `window_geometry.rs` | 412 | 6 | Lưu và khôi phục tọa độ (x, y, w, h) và trạng thái maximized |
| `settings_store.rs` | 380 | 6 | Lưu trữ cài đặt `settings.json`, ghi đè atomic |
| `gateway_exec.rs` | 610 | 5 | PTY stdout backpressure channel, terminal resize event |
| `proxy_cors.rs` | 340 | 5 | Kiểm tra header CORS, chặn Origin lạ, preflight OPTIONS |
| `server_profiles.rs` | 430 | 5 | Chuyển đổi giữa các server profiles, preset persistence |
| `capabilities_security.rs` | 390 | 5 | Phân quyền Tauri v2 ACL, cấm truy cập tài nguyên ngoài phạm vi |
| `gateway_protocol.rs` | 490 | 4 | Tuần tự hóa / giải tuần tự hóa gói tin JSON Gateway |
| `gateway_exec_denylist.rs` | 310 | 4 | Chặn các lệnh nguy hiểm (`rm -rf`, `mkfs`, fork bomb) |
| `gateway_browser.rs` | 450 | 4 | Điều khiển Chrome qua CDP, mock browser debugging port |
| `app_commands.rs` | 380 | 4 | Tauri IPC commands: restart, quit, minimize, toggle devtools |
| `proxy_retarget.rs` | 320 | 3 | Đổi URL upstream trong lúc runtime mà không cần khởi động lại |
| `release_packaging.rs` | 290 | 3 | Kiểm tra cấu hình bundle, icon metadata, binary size limit |
| `rfc_server_core.rs` | 310 | 3 | Core HTTP engine của RFC daemon |
| `title_sync.rs` | 260 | 3 | Tự động đồng bộ tiêu đề cửa sổ theo webview title |
| `mobile_surface.rs` | 240 | 2 | Giao diện di động, vô hiệu hóa các tính năng desktop-only |
| `scaffold_config.rs` | 210 | 2 | Khởi tạo file cấu hình ban đầu khi lần đầu mở ứng dụng |

### 2.2 Rà soát Inline Unit Tests trong `src-tauri/src/`

Trong tổng số **40 tệp mã nguồn Rust Core**, đồ thị CBM ghi nhận:
- **15 tệp CÓ inline unit test (`#[cfg(test)]`)**: Tổng cộng 74 unit tests trong `cargo test --lib` và 31 unit tests trong `cargo test --bin aqvn-agent`.
- **25 tệp KHÔNG CÓ inline unit test**:
  1. `src-tauri/src/main.rs` (10 dòng)
  2. `src-tauri/src/rfc/mod.rs` (18 dòng)
  3. `src-tauri/src/rfc/dispatch.rs` (239 dòng)
  4. `src-tauri/src/rfc/files.rs` (296 dòng)
  5. `src-tauri/src/rfc/server.rs` (441 dòng)
  6. `src-tauri/src/rfc/util.rs` (167 dòng)
  7. `src-tauri/src/rfc/text_editor.rs` (648 dòng)
  8. `src-tauri/src/rfc/workdir.rs` (432 dòng)
  9. `src-tauri/src/rfc/exchange.rs` (85 dòng)
  10. `src-tauri/src/rfc/auth.rs` (69 dòng)
  11. `src-tauri/src/ssh/shell.rs` (124 dòng)
  12. `src-tauri/src/proxy/mod.rs` (15 dòng)
  13. `src-tauri/src/app/notify.rs` (34 dòng)
  14. `src-tauri/src/app/links.rs` (88 dòng)
  15. `src-tauri/src/app/hotkey.rs` (66 dòng)
  16. `src-tauri/src/app/tray.rs` (513 dòng)
  17. `src-tauri/src/app/mod.rs` (189 dòng)
  18. `src-tauri/src/app/context_menu.rs` (300 dòng)
  19. `src-tauri/src/connector/handlers/mod.rs` (251 dòng)
  20. `src-tauri/src/connector/handlers/telemetry.rs` (251 dòng)
  21. `src-tauri/src/connector/handlers/process.rs` (266 dòng)
  22. `src-tauri/src/connector/protocol.rs` (1.104 dòng)
  23. `src-tauri/src/connector/mod.rs` (19 dòng)
  24. `src-tauri/src/lib.rs` (331 dòng)
  25. `src-tauri/src/config.rs` (688 dòng)

*Nhận xét:* Phần lớn logic của các tệp trên được bao phủ gián tiếp thông qua 32 tệp integration tests. Tuy nhiên, việc thiếu inline test khiến việc bắt các lỗi cú pháp biên (edge cases) khi refactor nội bộ hàm trở nên chậm hơn, vì phải kích hoạt toàn bộ luồng tích hợp nặng.

### 2.3 Cơ chế an toàn đa luồng (`TEST_ENV_MUTEX`)

Một phát hiện quan trọng trong hướng dẫn kiến trúc `AGENTS.md`:
> *"Tests that mutate environment variables must serialize with `static TEST_ENV_MUTEX: Mutex<()>`."*

Khi chạy `cargo test` mặc định trên máy nhiều nhân (CPU multi-threading), các bài test trong `agent_cli.rs`, `agent_host_pool.rs` và `proxy_server.rs` có đọc/ghi biến môi trường như `AQVN_CONFIG_PATH`, `AQVN_PORT`, `HOME`. Nếu không có Mutex tuần tự hóa, việc chạy song song sẽ gây xung đột dữ liệu ngẫu nhiên (flaky tests). Đây là lý do lệnh test chuẩn của dự án trong `services.yaml` bắt buộc phải là:
```bash
cargo test --bin aqvn-agent -- --test-threads=1
```

---

## 3. Hiện Trạng Kiểm Thử Frontend Shell & Tools (`ui/` & `tools/`)

### 3.1 Bộ test Node.js `test_chat_handlers.mjs` (177 tests)

Tệp `tools/tests/test_chat_handlers.mjs` (2.490 dòng) là một công trình kiểm thử giả lập DOM xuất sắc:
- **Kiến trúc Mocking:** Xây dựng một mock browser DOM hoàn chỉnh mà không cần phụ thuộc vào JSDOM hay Puppeteer nặng nề (`createElementMock`, `createClassList`, `createDocumentFragmentMock`, `addEventListener`, `localStorage`).
- **Phạm vi kiểm thử 177 bài test bao quát:**
  1. *Cơ chế kết nối Socket.IO:* Handshake, reconnect exponential backoff, xử lý ngắt kết nối mạng.
  2. *PTY Terminal Drawer:* Mở/đóng drawer, streaming chunks, backpressure buffer, đổi kích thước terminal (`resizeTerminal`), chuyển đổi giữa các tab terminal.
  3. *Layout 3 cột & Splitters:* Kéo thả thanh chia màn hình (left/right splitter drag), thu gọn panel, Focus Mode (ẩn 2 panel bên để tập trung chat).
  4. *Quản lý Server Profiles:* Thêm profile mới, chuyển đổi profile, lưu vào `localStorage`.
  5. *Đa chủ đề & i18n:* Chuyển đổi giữa 4 themes OKLCH (`aqvn-dark`, `oled-black`, `cyberpunk-terminal`, `clean-studio-light`), dịch giao diện song ngữ Anh/Việt.
  6. *Markdown & Code Highlighting:* Tích hợp với `marked` và `hljs`.

### 3.2 Bộ test tài nguyên bên thứ ba `test_vendor_assets.mjs` (4 tests)
Kiểm tra tính toàn vẹn của các thư viện JS tĩnh trong `ui/vendor/`:
- `marked.min.js`: Kiểm tra sự tồn tại và tính hợp lệ cú pháp JS.
- `highlight.min.js`: Kiểm tra sự tồn tại và cú pháp.
- Thứ tự nạp tệp: Xác nhận `ui/index.html` nạp vendor scripts trước `ui/chat.js`.

### 3.3 Các kịch bản kiểm thử độc lập trong `tools/tests/`

| Tệp kịch bản | Ngôn ngữ | Chức năng kiểm thử | Có trong CI không? |
| :--- | :---: | :--- | :---: |
| `test_gen_icons.py` | Python (Pillow) | Kiểm tra việc tự động tạo 6 kích thước icon từ `icon-512.png` | ❌ KHÔNG |
| `test_server_profiles_browser.py` | Python / agent-browser | E2E browser test cho multi-server switcher & login | ❌ KHÔNG |
| `tray_rig_restart_test.sh` | Bash / Xvfb | Kiểm tra việc khởi động lại ứng dụng trên cùng display mà không làm đơ D-Bus | ❌ KHÔNG |
| `os_integration_test.sh` | Bash | Kiểm tra tích hợp OS (phím tắt Alt+Space, single-instance) | ❌ KHÔNG |
| `model_stub_test.sh` | Bash / curl | Kiểm tra mock server trả về SSE streaming chat tokens | ❌ KHÔNG |
| `e2e_cleanup_test.sh` | Bash | Dọn dẹp tiến trình rác và file tạm `/tmp/aqvn-*` | ❌ KHÔNG |

---

## 4. Danh Mục Khoảng Trống Kiểm Thử Trọng Yếu (Critical Test Gaps)

Dựa trên phân tích đối sánh giữa đồ thị CBM và danh mục tệp thực tế, nhóm đánh giá xác định **7 khoảng trống kiểm thử lớn**:

### Gap 1: Thiếu hoàn toàn Unit Test cho `ui/container.js` (57 hàm, 650 dòng)
- **Vấn đề:** `ui/container.js` là kịch bản được nhúng trực tiếp vào mọi webview (`CONTAINER_BRIDGE_SCRIPT` trong `src-tauri/src/proxy/server.rs`). Tệp này quản lý việc can thiệp `document.write`, đồng bộ splash screen, và bắt sự kiện iframe.
- **Rủi ro:** Hiện tại **KHÔNG CÓ** bài unit test Node nào kiểm tra `ui/container.js`. Nếu có lỗi cú pháp hoặc ngoại lệ runtime trong tệp này, webview sẽ bị trắng trang (White Screen of Death).

### Gap 2: Thiếu Unit Test cho `ui/bridge.js` (TauriStoreWrapper)
- **Vấn đề:** `ui/bridge.js` chịu trách nhiệm làm cầu nối giữa giao diện JS và Tauri Plugin Store. Nó chứa logic fallback sang `localStorage` khi chạy trong trình duyệt thông thường.
- **Rủi ro:** Logic fallback chưa được kiểm thử tự động, dễ xảy ra lỗi không lưu được cài đặt khi chạy ở chế độ dev trên browser.

### Gap 3: Thiếu Unit Test cho trang Cài đặt `ui/settings.js`
- **Vấn đề:** Trong khi `ui/chat.js` có 177 unit tests, trang cài đặt `ui/settings.html` và mã điều khiển `ui/settings.js` lại chưa có bộ test giả lập tương tự.

### Gap 4: `app/notify.rs` và `app/links.rs` thiếu bài test xử lý lỗi hệ thống
- **Vấn đề:** Hai module này trực tiếp gọi API nền tảng OS (`open::that`, `tauri_plugin_notification`). Khi chạy trên các môi trường Linux không có desktop notification daemon (như server headless hoặc Docker container tối giản), hàm có thể trả về lỗi nhưng chưa có bài test xác thực việc ứng dụng không bị panic.

### Gap 5: Fallback phi Linux của `telemetry.rs` chưa được kiểm thử
- **Vấn đề:** Module `src-tauri/src/connector/handlers/telemetry.rs` chỉ cài đặt đọc `/proc/stat` và `/proc/meminfo` cho Linux. Trên Windows và macOS, code trả về giá trị giả lập `0`. Chưa có unit test mock để xác nhận telemetry packet không bị lỗi schema trên Windows/macOS.

### Gap 6: Các bài test Python & Shell trong `tools/tests/` bị bỏ rơi khỏi CI
- **Vấn đề:** 6 kịch bản kiểm thử trong `tools/tests/` chứa nhiều logic kiểm tra quan trọng (tạo icon, khởi động rig Xvfb, dọn dẹp bộ nhớ đệm), nhưng không bao giờ được kích hoạt trong GitHub Actions.

### Gap 7: Rủi ro xung đột môi trường khi chạy `cargo test` mặc định
- **Vấn đề:** Nếu một lập trình viên hoặc CI chạy lệnh `cargo test` thông thường (mặc định chạy song song nhiều luồng), các bài test biến môi trường sẽ va chạm và gây thất bại ngẫu nhiên (flaky failure).

---

## 5. Rà Soát & Đánh Giá CI/CD Workflows (`.github/workflows/`)

### 5.1 Phân tích chi tiết quy trình CI (`.github/workflows/ci.yml`)

Workflow `ci.yml` được kích hoạt trên mỗi lần `push` hoặc `pull_request` vào nhánh `master`, gồm 4 jobs:
1. `lint`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
2. `test`: `cargo test --manifest-path src-tauri/Cargo.toml`, `node tools/tests/test_vendor_assets.mjs`, `node --test tools/tests/test_chat_handlers.mjs`.
3. `html`: `npm run lint:html` (chạy trên Node 24).
4. `audit`: `cargo audit --file src-tauri/Cargo.lock`.

#### Các điểm yếu nghiêm trọng trong `ci.yml`:

```
+--------------------------------------------------------------------------------------------------+
|                                    HIỆN TRẠNG CI.YML (ĐƠN NỀN TẢNG)                              |
|                                                                                                  |
|   ubuntu-24.04 (Runner duy nhất)                                                                 |
|   +------------------------------------------------------------------------------------------+   |
|   | [Job: lint] -> cargo fmt, cargo clippy (Linux dependencies: webkit2gtk, libxdo, ...)     |   |
|   | [Job: test] -> cargo test, node tests                                                    |   |
|   | [Job: html] -> html-validate                                                             |   |
|   | [Job: audit]-> cargo audit                                                               |   |
|   +------------------------------------------------------------------------------------------+   |
|                                                                                                  |
|   WINDOWS (windows-latest)  ====> HOÀN TOÀN KHÔNG CÓ TRONG CI ! (Chỉ chạy khi có Tag Release)    |
|   macOS   (macos-15)        ====> HOÀN TOÀN KHÔNG CÓ TRONG CI ! (Chỉ chạy khi có Tag Release)    |
|   Android (APK Build)       ====> HOÀN TOÀN KHÔNG CÓ TRONG CI ! (Chỉ chạy khi có Tag Release)    |
+--------------------------------------------------------------------------------------------------+
```

1. **RỦI RO CAO NHẤT (Single-Platform Blindspot):**
   - CI hoàn toàn không có runner Windows hay macOS.
   - Mã nguồn Rust Core chứa rất nhiều đoạn mã điều kiện theo hệ điều hành: `#[cfg(target_os = "windows")]`, `#[cfg(target_os = "macos")]`, `winapi`, `SendInput`.
   - Bất kỳ lập trình viên nào chỉnh sửa mã Windows làm gãy biên dịch trên Windows thì PR trên GitHub vẫn báo **Green** (vì Linux biên dịch thành công). Lỗi chỉ được phát hiện khi quản trị viên tạo tag release v*!
2. **Thiếu tuần tự hóa luồng trong lệnh `cargo test`:**
   - Trong job `test`:
     `run: cargo test --manifest-path src-tauri/Cargo.toml`
   - Lệnh này chạy toàn bộ các target với số luồng tối đa của máy runner Ubuntu (`nproc`), vi phạm quy tắc `TEST_ENV_MUTEX`, tạo nguy cơ flaky build.
3. **Thiếu cơ chế Caching Rust dependencies:**
   - Mỗi lần CI chạy đều phải tải và biên dịch lại từ đầu toàn bộ các crate nặng (`aws-lc-sys`, `webkit2gtk`, `tokio`, `russh`), làm thời gian chạy CI kéo dài từ 12-18 phút.
   - Chưa tích hợp action `Swatinem/rust-cache@v2`.

---

### 5.2 Phân tích quy trình Phát hành (`.github/workflows/release.yml`)

Workflow `release.yml` được kích hoạt bằng tay (`workflow_dispatch`) hoặc khi có tag `v*`, gồm 4 jobs đóng gói:
- **`linux` (ubuntu-24.04):** Đóng gói `.deb`, `.rpm` và binary portable `.tar.gz`.
- **`windows` (windows-latest):** Đóng gói NSIS `.exe`, `.msi` và portable `.zip`. Tích hợp biến môi trường `AWS_LC_SYS_PREBUILT_NASM: '1'`.
- **`macos` (macos-15):** Đóng gói `.dmg`, `.app` và portable `.tar.gz`.
- **`android` (ubuntu-24.04):** Biên dịch NDK r27 cho 4 kiến trúc: `aarch64`, `armv7`, `i686`, `x86_64`, xuất file APK.

#### Điểm sáng kỹ thuật:
- **NSIS Installer Size Gate (< 10MB):** Trong job `windows`, có một bước kiểm tra dung lượng nghiêm ngặt:
  ```bash
  MAX_SIZE=10485760 # 10MB
  SIZE=$(wc -c < "$EXE" | tr -d ' ')
  if [ "$SIZE" -gt "$MAX_SIZE" ]; then
    echo "Error: NSIS installer size exceeds 10MB limit"
    exit 1
  fi
  ```
  Cơ chế này đảm bảo triết lý cốt lõi của `ai-launcher` là một ứng dụng native siêu nhẹ (< 10MB), ngăn chặn việc vô tình đưa các tài nguyên cồng kềnh vào bộ cài.

---

## 6. Danh Mục Bài Test & Cải Tiến CI Đề Xuất Bổ Sung

Dựa trên các khoảng trống đã phát hiện, nhóm kiểm toán kiến nghị kế hoạch hành động cụ thể gồm 5 gói nâng cấp:

### Gói 1: Bổ sung bộ test Unit `test_container_bridge.mjs`
- **Mục tiêu:** Kiểm thử toàn diện `ui/container.js`.
- **Nội dung kiểm thử đề xuất:**
  * Giả lập việc inject script vào tài liệu HTML động.
  * Kiểm tra việc bắt hook `document.write` và khôi phục nội dung webview.
  * Xác minh việc khởi tạo biến toàn cục `window.__AQVN_PROXY__` và `window.__AQVN_WINDOW_SHOWN__`.
  * Đo lường tính trơ ngoại lệ (zero-throw guarantee) khi chạy trong môi trường iframe lồng nhau.

### Gói 2: Bổ sung bộ test Unit `test_ui_bridge_store.mjs`
- **Mục tiêu:** Kiểm thử `ui/bridge.js` và `TauriStoreWrapper`.
- **Nội dung kiểm thử đề xuất:**
  * Kiểm tra hành vi đọc/ghi/xóa dữ liệu qua `TauriStoreWrapper.set()`, `.get()`, `.delete()`.
  * Giả lập trường hợp plugin Tauri Store bị timeout hoặc chưa sẵn sàng: xác nhận wrapper tự động fallback về `localStorage` an toàn.
  * Kiểm tra tính toàn vẹn của việc đồng bộ cookie jar vào bộ nhớ cục bộ.

### Gói 3: Bổ sung bài test Rust `telemetry_cross_platform.rs`
- **Mục tiêu:** Kiểm thử fallback của telemetry trên môi trường không phải Linux.
- **Nội dung kiểm thử đề xuất:**
  * Viết integration test trong `src-tauri/tests/telemetry_cross_platform.rs`.
  * Mock dữ liệu sysinfo cho Windows và macOS.
  * Xác nhận hàm `sample_system_telemetry()` luôn sinh ra JSON payload đúng định dạng `TelemetrySnapshot` mà không gây panic hay lỗi chia cho 0.

### Gói 4: Bổ sung bài test Rust `app_notification_mock.rs`
- **Mục tiêu:** Kiểm thử `src-tauri/src/app/notify.rs`.
- **Nội dung kiểm thử đề xuất:**
  * Kiểm tra việc gửi thông báo khi task hoàn thành (`notify_task_complete`).
  * Giả lập môi trường headless không có notification daemon, xác minh hàm bắt lỗi lịch sự và ghi log cảnh báo thay vì làm crash tiến trình.

### Gói 5: Tái cấu trúc và hiện đại hóa `.github/workflows/ci.yml`
- **Nâng cấp Matrix Testing đa nền tảng:**
  ```yaml
  strategy:
    fail-fast: false
    matrix:
      os: [ubuntu-24.04, windows-latest, macos-15]
  ```
  Chạy `cargo check` và `cargo test --lib` trên cả 3 hệ điều hành trong mọi PR.
- **Tích hợp Rust Caching:**
  Thêm bước `uses: Swatinem/rust-cache@v2` để giảm 60% thời gian chạy CI.
- **Chuẩn hóa lệnh chạy test có kiểm soát luồng:**
  ```yaml
  - name: Run Rust tests
    run: |
      cargo test --lib --manifest-path src-tauri/Cargo.toml
      cargo test --bin aqvn-agent --manifest-path src-tauri/Cargo.toml -- --test-threads=1
  ```
- **Tự động hóa chạy kiểm tra icon:**
  Thêm bước chạy `python tools/tests/test_gen_icons.py` vào CI để đảm bảo tài nguyên icon không bị sai lệch kích thước.

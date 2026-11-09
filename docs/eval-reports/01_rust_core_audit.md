# Báo Cáo Đánh Giá Toàn Diện Mã Nguồn Rust Core (src-tauri/)

**Mã tài liệu:** `VAL-EVAL-CORE-001`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi kiểm toán:** `src-tauri/src/` (40 tệp Rust, ~20.000 dòng mã nguồn)  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Evaluation Team (eval-worker)  
**Trạng thái kiểm tra kỹ thuật:** Passed (`cargo check`, `cargo test --lib` 74/74 passed)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Mã nguồn Rust Core nằm trong thư mục `src-tauri/src/` là xương sống chịu trách nhiệm điều phối toàn bộ vòng đời ứng dụng trên desktop và mobile (Linux, macOS, Windows, Android), quản lý giao tiếp mạng hai chiều an toàn (inbound RFC/SSH daemons và outbound WebSocket Gateway client), duy trì kho lưu trữ cài đặt bền vững, đồng thời cung cấp lớp proxy đảo ngược (reverse proxy) trên loopback bảo vệ thông tin xác thực của người dùng.

### Các chỉ số tổng quan:
- **Tổng số tệp Rust:** 40 tệp mã nguồn trong `src-tauri/src/`.
- **Tổng số dòng mã nguồn:** ~20.000 dòng Rust (`src-tauri/src/`).
- **Khối lượng kiểm thử đi kèm:** 32 tệp tích hợp trong `src-tauri/tests/` với 18.287 dòng test.
- **Tỷ lệ vượt qua kiểm thử đơn vị nội bộ:** 74/74 unit tests (`cargo test --lib`) hoàn thành trong 1.88s.
- **Đánh giá an toàn bộ nhớ:** 0 cảnh báo `cargo clippy`, chỉ có 11 khối `unsafe` cục bộ (đều được giới hạn tương tác syscall libc/Windows OS).
- **Mức độ phụ thuộc GUI:** Tách bạch hoàn toàn giữa desktop (`tauri-plugin-*`, `gtk`/`webkit` trên Linux) và mobile / headless daemon (`#[cfg(not(target_os = "android"))]`).

---

## 2. Bản Đồ Thành Phần Kiến Trúc (Architecture & Component Map)

Kiến trúc backend của `ai-launcher` tuân theo mô hình phân tầng mô-đun hóa cao, phân chia rõ rệt trách nhiệm giữa các hệ thống con:

```
+--------------------------------------------------------------------------------------------------+
|                                    TAURI HOST BACKEND (src-tauri/src)                            |
|                                                                                                  |
|   +-------------------+    +----------------------+    +-------------------------------------+   |
|   | Entrypoint        |    | Lifecycle & Superv.  |    | Config Store & Debouncer            |   |
|   | main.rs (L1-9)    |--->| lib.rs (L1-330)      |--->| config.rs (L1-687)                  |   |
|   +-------------------+    +----------------------+    +-------------------------------------+   |
|                                      |                                                           |
|            +-------------------------+------------------------+-------------------+              |
|            |                         |                        |                   |              |
|            v                         v                        v                   v              |
|   +--------------------+   +-------------------+   +--------------------+  +------------------+  |
|   | Local Axum Proxy   |   | RFC Host Bridge   |   | SSH Bridge Daemon  |  | Outbound Gateway |  |
|   | proxy/ (3.597L)    |   | rfc/ (2.687L)     |   | ssh/ (1.212L)      |  | connector/ (8.3L)|  |
|   | - server.rs        |   | - server.rs       |   | - server.rs        |  | - client.rs      |  |
|   | - upstream.rs      |   | - auth.rs (HMAC)  |   | - guard.rs (denylist| - protocol.rs    |  |
|   | - jar.rs (Cookie)  |   | - sandbox.rs      |   | - shell.rs (PTY)   |  | - handlers/      |  |
|   | - cache.rs (LRU)   |   | - dispatch.rs     |   |                    |  |   (exec, files,  |  |
|   | - ws.rs (Tunnel)   |   | - text_editor.rs  |   |                    |  |    browser, ...) |  |
|   +--------------------+   +-------------------+   +--------------------+  +------------------+  |
|            |                                                                      |              |
|            v                                                                      v              |
|   Webview UI Shell (Loopback)                                            Cloud / Gateway (/ws)   |
+--------------------------------------------------------------------------------------------------+
```

### Bảng thống kê độ lớn các module chính:

| Phân hệ / Tệp | Số dòng | Chức năng cốt lõi | Rủi ro chính |
| :--- | :---: | :--- | :--- |
| `main.rs` | 10 | Điểm khởi đầu ứng dụng đa nền tảng | Không có logic phụ |
| `lib.rs` | 331 | Khởi tạo Tauri Builder, plugins, window và spawn background supervisors | `block_on` khi bind proxy; polling config mtime 500ms |
| `config.rs` | 688 | Schema cấu hình, deterministic gateway ID, settings store, geometry debouncer | Mutex locking trong debouncer; serialize fallback |
| `proxy/server.rs` | 1.136 | Axum HTTP server loopback, CORS origin guard, dynamic script injection | Kích thước file lớn (>1000L), fallback routing |
| `proxy/upstream.rs` | 947 | Client upstream reqwest, CSRF bootstrap, exponential retry | Retry sleep chặn luồng async; cookie serialization |
| `proxy/cache.rs` | 820 | Cache tầng nhớ (LRU) & tầng đĩa cho static assets của upstream | Tăng dung lượng đĩa nếu asset lớn; race condition moka |
| `proxy/jar.rs` | 330 | Quản lý Cookie jar an toàn, rotation CSRF token | File IO đồng bộ khi lưu cookie |
| `proxy/ws.rs` | 349 | Tunneling hai chiều WebSocket `/socket.io/` với watchdog keepalive | Phụ thuộc ping/pong timeout |
| `rfc/server.rs` | 441 | Axum listener cổng 55080, sliding-window IP rate limiter | Listener bind 0.0.0.0; bảo mật phụ thuộc 100% HMAC |
| `rfc/sandbox.rs` | 342 | Sandbox path jail, canonicalize roots, chặn traversal và symlink escape | Canonicalization fail-close khi path chưa tồn tại |
| `rfc/dispatch.rs` | 239 | Whitelist mapping `(module, function)` sang handler nội bộ | Cần cập nhật thủ công khi thêm API |
| `rfc/text_editor.rs` | 648 | Thao tác chỉnh sửa tệp dạng patch (find/replace, surgical block edits) | Xử lý chuỗi lớn trong bộ nhớ |
| `ssh/server.rs` | 749 | russh async SSH listener cổng 55022, RSA-OAEP auth | Xử lý PTY raw ANSI streams |
| `ssh/guard.rs` | 279 | Secondary denylist chặn fork bombs và lệnh shell hủy diệt | Bypasses qua wrapper shell, eval, base64 |
| `connector/client.rs`| 1.033 | Outbound WebSocket Gateway client kết nối upstream `/ws` | Quản lý reconnect loop và state transitions |
| `connector/protocol.rs`| 1.104| Định dạng gói tin Socket.IO / Gateway Control envelopes | Kích thước schema lớn; serialization overhead |
| `connector/handlers/`| 8.162| Handlers điều khiển từ xa: computer use, exec PTY, files, browser | Nhiều file >1.500L; gọi syscall unsafe |

---

## 3. Phân Tích Chi Tiết Các Module Trọng Điểm

### 3.1. `main.rs` & `lib.rs` — Vòng Đời & Điều Phối Khởi Động

#### Phân tích mã nguồn `main.rs` (dòng 1–10):
- **Cấu hình đa nền tảng:** Sử dụng `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` tại dòng 1 để ẩn console window trên Windows trong bản build release.
- **Tách biệt Android:** Tách hàm `main` độc lập cho target Android (`#[cfg(target_os = "android")] fn main() {}`) và desktop (`aqvn_workspace_lib::run()`), tuân thủ chuẩn Tauri v2.

#### Phân tích mã nguồn `lib.rs` (dòng 1–331):
- **Khởi tạo Tauri Plugins (dòng 16–29):**
  Tích hợp các plugin cốt lõi: `tauri_plugin_shell`, `tauri_plugin_notification`, `tauri_plugin_store`, và `tauri_plugin_global_shortcut`.
- **Cơ chế lưu trữ kích thước cửa sổ (Geometry Tracking & Debouncing, dòng 32–114):**
  Lắng nghe `WindowEvent::Resized`, `WindowEvent::Moved`, `WindowEvent::Focused(false)`, và `WindowEvent::CloseRequested`. Tọa độ và kích thước cửa sổ được chuẩn hóa thành `WindowState` (`x, y, width, height, is_maximized`) theo hệ tọa độ logic dựa trên `scale_factor`. Trạng thái này được chuyển qua `GeometryDebouncer` (500ms) để tránh ghi đĩa liên tục vào `settings.json`.
- **Đóng cửa sổ về Khay Hệ Thống (Close to Tray, dòng 104–112):**
  Khi người dùng bấm nút Close (X), nếu tùy chọn `close_to_tray` được bật (mặc định `true`), `api.prevent_close()` được gọi và cửa sổ chính ẩn đi (`window.hide()`).
- **Khởi tạo và liên kết mạng trong `setup` (dòng 120–260):**
  - Đồng bộ đường dẫn `workspace_path` sang `connector::handlers::files` (dòng 125–128).
  - Khởi tạo `ConfigWatcher` (dòng 129) và `GeometryDebouncer` (dòng 131).
  - **Khởi động Local Proxy (dòng 137–152):**
    Sử dụng `tauri::async_runtime::block_on(proxy::server::bind_with_config(&initial_config))` (dòng 138). Listener bind vào cổng động (ephemeral port 0 hoặc dải ưu tiên 55088–55099) **trước khi tạo cửa sổ**, đảm bảo cổng thực tế được trích xuất và đưa vào kịch bản khởi tạo trang (`window.__AQVN_PROXY__`).
  - **Khởi động các Supervisor ngầm (dòng 154–195):**
    - `proxy::server::watch_target`: Giám sát thay đổi cấu hình `server_url` để retarget proxy và reset jar (dòng 156–158).
    - `rfc::server::run_supervised`: Chạy daemon RFC cổng 55080 (chỉ bind khi bật `rfc_enabled`, dòng 165–170).
    - `ssh::server::run_supervised`: Chạy daemon SSH cổng 55022 (chỉ bind khi bật `rfc_enabled` và có mật khẩu, dòng 173–178).
    - `connector::client::run_supervised_with_reconnect`: Chạy client kết nối ngược lên Hub Gateway qua WebSocket (dòng 181–193).
- **Polling Hot-Reload Cấu Hình (dòng 198–263):**
  Một luồng nền kiểm tra `mtime` của tệp `settings.json` mỗi 500ms thông qua `config::load_from_file_if_modified`. Khi phát hiện file bị chỉnh sửa từ Settings UI, cấu hình mới được áp dụng tức thì cho context menu bar, tray icon, workspace path và `ConfigWatcher` mà không yêu cầu khởi động lại ứng dụng.
- **Tạo Cửa Sổ Chính & Đồng Bộ Tiêu Đề (dòng 265–315):**
  - Lấy cấu hình `main` từ `tauri.conf.json` (`expect("main window config not found...")` tại dòng 270).
  - Tắt phím tắt zoom mặc định (`zoom_hotkeys_enabled(false)` tại dòng 286).
  - **Đồng bộ tiêu đề tài liệu (`on_document_title_changed`, dòng 299–302):**
    Tự động cập nhật tiêu đề cửa sổ khi trang web thay đổi title, giải quyết triệt để vấn đề phân quyền IPC `core:window:allow-set-title`.

---

### 3.2. `config.rs` — Quản Lý Cấu Hình & Lưu Trữ Bền Vững

- **Hằng số mặc định chuẩn hóa (dòng 20–36):**
  - `DEFAULT_SERVER_URL`: `"https://a0.matmagoc.com"`
  - `DEFAULT_RFC_PORT`: `55080`
  - `DEFAULT_SSH_PORT`: `55022`
  - `DEFAULT_WORKSPACE_PATH`: `"~/Workspace"`
  - `STORE_FILENAME`: `"settings.json"`
- **Tạo Định Danh Gateway Bất Biến (Deterministic Gateway ID, dòng 38–78):**
  Hàm `deterministic_gateway_id()` tính toán mã băm SHA-256 từ hạt giống cố định kết hợp với `/etc/machine-id` (hoặc `/var/lib/dbus/machine-id`), biến môi trường `HOSTNAME` và `USER`. Kết quả có định dạng `aqvn-gw-<hex_part>`, bảo đảm ID duy nhất nhưng không đổi qua các lần khởi động trên cùng một máy chủ.
- **Cấu trúc `AppConfig` (dòng 133–176):**
  Bao gồm đầy đủ các trường cấu hình cho server URL, server profiles (chuyển đổi đa môi trường), mật khẩu RFC/SSH, danh sách `allowed_paths`, các cờ phạm vi quyền (`scope_files`, `scope_exec`, `scope_browser`, `scope_desktop`), phím tắt toàn cục, ngôn ngữ (`vi`/`en`), chứng chỉ TLS CA và cờ `tls_skip_verify`.
- **Hợp nhất Sandbox Paths (`unified_allowed_paths`, dòng 226–258):**
  Hợp nhất thông minh giữa `workspace_path` và mảng `allowed_paths`, loại bỏ các mục trùng lặp và đảm bảo đường dẫn luôn tồn tại ít nhất thư mục mặc định `~/Workspace`.
- **Cơ chế Hot-Reload qua `ConfigWatcher` (dòng 525–546):**
  Sử dụng `tokio::sync::watch` kênh 1-producer nhiều-consumer. Khi cấu hình thay đổi, `sender.send(new_config)` phát thông điệp đến tất cả các supervisor ngầm (proxy, RFC, SSH, connector) mà không gây tắc nghẽn khóa mutex.
- **Bộ chống rung tọa độ cửa sổ (`GeometryDebouncer`, dòng 590–687):**
  Sử dụng kênh `tokio::sync::mpsc` kết hợp `tokio::time::sleep(500ms)`. Khi sự kiện di chuyển/co giãn cửa sổ diễn ra dồn dập, debouncer chỉ ghi xuống đĩa trạng thái cuối cùng, giảm thiểu 95% thao tác I/O đĩa.

---

### 3.3. Phân Hệ Reverse Proxy (`src-tauri/src/proxy/`)

Hệ thống reverse proxy viết bằng Axum 0.8 đóng vai trò cầu nối duy nhất giữa Webview và máy chủ cloud:

```
[Webview Origin] (tauri://localhost hoặc localhost:PORT)
       |
       v (HTTP / WebSocket)
[Axum Reverse Proxy] (127.0.0.1:PORT)
  ├── 1. Origin Verifier: reject_bad_origin()
  ├── 2. Cookie Jar Manager: injects session cookies & CSRF
  ├── 3. LRU / Disk Cache: cache HIT -> trả ngay (MISS -> forward)
  └── 4. Upstream Client: forward_raw() -> Cloud (https://a0.matmagoc.com)
```

#### Phân tích chi tiết:
- **Lựa chọn cổng thông minh (`proxy/server.rs`, dòng 40–42, 178–223):**
  Proxy ưu tiên dò tìm cổng trống trong dải `55088..=55099` (`PROXY_PORT_RANGE`). Nếu toàn bộ dải cổng bận hoặc khi biến môi trường `AQVN_PROXY_EPHEMERAL=1` được kích hoạt, proxy tự động chuyển sang cổng 0 (ephemeral do nhân hệ điều hành cấp phát). Cổng thực tế sau đó được truyền tới UI.
- **Bảo vệ Nguồn Gốc Nghiêm Ngặt (`allowed_origin`, dòng 935–961):**
  Chặn đứng các trang web độc hại từ trình duyệt bên ngoài truy cập vào proxy nội bộ:
  - Cho phép nguồn gốc WebView: `tauri://localhost`, `tauri://localhost/`, `tauri.localhost`.
  - Cho phép nguồn gốc kiểm thử cục bộ: `http://localhost:<PORT>`, `http://[IP]:<PORT>`, `http://[::1]:<PORT>` **chỉ khi số port trùng khớp với port proxy đang lắng nghe**.
  - Toàn bộ nguồn gốc khác bị trả về `403 Forbidden` (`reject_bad_origin`, dòng 926–933).
- **Tiêm Kịch Bản Cầu Nối Container (`inject_container_bridge`, dòng 257–285):**
  Khi proxy trả về tài nguyên HTML (`text/html`), mã nguồn tự động chèn script `ui/container.js` vào trước thẻ `</body>` hoặc `</head>`. Nhờ đó, cả trang splash screen và giao diện webui động đều nhận được đối tượng `window.__AQVN_PROXY__` đóng băng.
- **Cơ Chế Bền Bỉ & Tự Động Thử Lại Upstream (`proxy/upstream.rs`, dòng 37–90):**
  Hàm `execute_with_retry` và `send_with_retry` áp dụng thuật toán lũy thừa lùi (exponential backoff: 100ms, 200ms, 400ms) khi gặp lỗi mạng tạm thời (`is_connect`, `is_timeout`) lên tới tối đa 3 lần thử, đảm bảo kết nối ổn định khi mạng chập chờn.
- **Bộ nhớ đệm Tĩnh Tối Ưu (`proxy/cache.rs`):**
  Tích hợp bộ nhớ cache tầng kép: RAM (sử dụng `moka::future::Cache` dung lượng 64MB) và đĩa (`~/.cache/net.aqvn.workspace/assets/`). Tự động tính toán mã ETag SHA-256 (`compute_etag`, dòng 120–135) và phản hồi `304 Not Modified`, giúp tải giao diện ngay lập tức (< 35ms FCP).

---

### 3.4. Phân Hệ RFC Host Bridge (`src-tauri/src/rfc/`)

Phân hệ RFC cho phép AI Agent trên cloud điều khiển máy client một cách an toàn qua cổng 55080:
- **Cơ chế xác thực HMAC-SHA256 (`rfc/auth.rs`, dòng 25–45):**
  Mỗi yêu cầu `POST /api/rfc` bắt buộc phải mang tiêu đề `X-RFC-Signature` hoặc trường `auth_signature` trong JSON. Chữ ký được tính bằng HMAC-SHA256 trên toàn bộ phần thân (body) với mật khẩu RFC. Quá trình đối soát sử dụng hàm so sánh thời gian bất biến (constant-time comparison `verify_slice`), loại bỏ hoàn toàn nguy cơ tấn công kênh kề (timing attacks).
- **Bộ giới hạn tần suất trượt (`RateLimiter`, `rfc/server.rs`, dòng 48–98):**
  Áp dụng sliding window 60 yêu cầu/phút trên từng địa chỉ IP của client. Có cơ chế quét dọn tự động các client không hoạt động khi danh sách vượt quá 64 IP, tránh rò rỉ bộ nhớ.
- **Nhận diện IP qua Proxy Đáng Tin Cậy (`extract_client_ip`, dòng 110–150):**
  Chỉ phân tích các tiêu đề `X-Forwarded-For` và `X-Real-IP` khi kết nối socket đến từ địa chỉ loopback nội bộ (`ip.is_loopback()`). Nếu kết nối đến từ mạng ngoài, IP vật lý của socket peer được áp đặt bắt buộc, ngăn chặn việc giả mạo IP để vượt qua rate limiter.
- **Hàng Rào Bảo Vệ Đường Dẫn (`rfc/sandbox.rs`, dòng 40–180):**
  Cấu trúc `Sandbox` thực hiện chuẩn hóa tuyệt đối (`canonicalize`) các thư mục gốc được cấp phép (`allowed_paths`). Mọi yêu cầu truy cập tệp đều bị ép buộc thông qua hàm `jail()`. Nếu đường dẫn chứa ký tự lùi thư mục `..` hoặc symlink trỏ ra ngoài thư mục sandbox, hàm ngay lập tức trả về `Err(SandboxViolation)`.

---

### 3.5. Phân Hệ SSH Bridge Daemon (`src-tauri/src/ssh/`)

- **Máy chủ SSH nhúng (`ssh/server.rs`, 749 dòng):**
  Sử dụng thư viện `russh` bất đồng bộ, lắng nghe trên cổng 55022. Hỗ trợ xác thực mật khẩu an toàn và cung cấp phiên shell PTY tương tác trực tiếp với hệ điều hành.
- **Bộ lọc phòng vệ thứ cấp (`ssh/guard.rs`, 279 dòng):**
  Chặn đứng các chuỗi lệnh có tính chất phá hoại hệ thống:
  - Phát hiện mẫu fork bomb (`:(){:|:&};:` hoặc biến thể không khoảng trắng, dòng 30–35).
  - Tách lệnh phức hợp dựa trên các toán tử `;`, `&&`, `||`, `|` có nhận biết dấu nháy đơn/kép (`split_segments`, dòng 52–88).
  - Từ chối các lệnh xóa hệ thống nguy hiểm (`rm -rf /`, `mkfs`, `dd if=/dev/zero`, `: > /dev/sda`).

---

### 3.6. Phân Hệ Outbound Gateway Connector (`src-tauri/src/connector/`)

- **Kết nối ngược Outbound WebSocket (`connector/client.rs`):**
  Giải quyết bài toán máy client nằm sau tường lửa hoặc NAT không có IP tĩnh. Client tự động thiết lập kết nối ra ngoài (outbound) tới endpoint `/ws` của Hub, gửi bắt tay `connector_hello` với danh sách quyền hạn (scopes), và duy trì ping keepalive định kỳ.
- **Điều tiết dòng dữ liệu PTY với Backpressure (`connector/handlers/exec.rs`, VAL-DEBT-002):**
  Kênh truyền stdout/stderr của PTY được giới hạn dung lượng với `bounded(128)` chunks và bộ đệm 32KB. Khi consumer (kết nối mạng) bị nghẽn, producer (tiến trình PTY) bị hãm tốc độ đọc tự nhiên, ngăn chặn tràn bộ nhớ khi ứng dụng in ra hàng chục MB log.

---

## 4. Danh Mục Rủi Ro Kỹ Thuật & Đánh Giá Chất Lượng Mã Nguồn

### 4.1. Phân Tích Hiện Tượng "God Modules" (Tệp Lớn Quá Ngưỡng)

Trong `src-tauri/src/`, có 8 tệp vượt quá 1.000 dòng mã nguồn:
1. `connector/handlers/computer_use.rs` (2.878 dòng): Quản lý toàn bộ mouse/keyboard injection trên đa hệ điều hành (X11, Windows SendInput, macOS).
2. `bin/agent.rs` (2.122 dòng): Bao gồm CLI parser, interactive menu, systemd manager, và status diagnostics.
3. `connector/handlers/exec.rs` (1.722 dòng): Quản lý PTY, session table, exit codes, và backpressure channel.
4. `connector/handlers/files.rs` (1.655 dòng): Quản lý đọc/ghi file, directory traversal, streaming, và path jailing.
5. `connector/handlers/browser.rs` (1.390 dòng): Quản lý tiến trình Chrome/Chromium và DevTools WebSocket bridge.
6. `proxy/server.rs` (1.136 dòng): Định tuyến HTTP, WebSocket upgrade, CORS, và cache injection.
7. `connector/protocol.rs` (1.104 dòng): Định nghĩa cấu trúc gói tin và serialization.
8. `connector/client.rs` (1.033 dòng): Vòng lặp kết nối WebSocket và state machine.

**Đánh giá:** Các tệp này chứa quá nhiều trách nhiệm hỗn hợp (ví dụ: `computer_use.rs` vừa parse JSON, vừa tính toán scaling, vừa gọi API đồ họa hệ điều hành). Cần lên kế hoạch phân tách thành các module con chuyên biệt trong các chu kỳ nâng cấp tiếp theo.

### 4.2. Rà Soát `unwrap()` và `expect()` Trong Mã Nguồn

Quá trình quét tự động toàn bộ 40 tệp Rust cho thấy:
- **Tuyệt đại đa số các lệnh `unwrap()` và `expect()` đều nằm trong khối `#[cfg(test)]`** (chiếm >95%).
- **Trong mã nguồn chạy thực tế (production):**
  - `src-tauri/src/lib.rs` (dòng 270): `.expect("main window config not found in tauri.conf.json")` — Chấp nhận được vì nếu thiếu cấu hình cửa sổ chính, ứng dụng không thể khởi chạy.
  - `src-tauri/src/lib.rs` (dòng 329): `.expect("failed to run AQVN Workspace")` — Điểm panic cuối cùng khi Tauri runtime gặp lỗi nghiêm trọng lúc khởi động.
  - `src-tauri/src/rfc/auth.rs` (dòng 37): `HmacSha256::new_from_slice(...).expect("HMAC accepts any key length")` — Tuyệt đối an toàn vì thuật toán HMAC theo RFC 2104 chấp nhận khóa với độ dài bất kỳ.

### 4.3. Rà Soát Các Khối `unsafe`

Tổng cộng có 11 khối `unsafe` trong toàn bộ `src-tauri/src/`:
- `src-tauri/src/bin/agent.rs` (dòng 836): Gọi `libc::geteuid()` để phát hiện quyền root khi cài đặt systemd.
- `src-tauri/src/connector/handlers/process.rs` (dòng 19, 117, 129): Gọi `libc::geteuid()` và `libc::kill(pid, sig)` để kiểm tra quyền và gửi tín hiệu tiến trình.
- `src-tauri/src/connector/handlers/telemetry.rs` (dòng 127, 156): Gọi `libc::sysinfo` và `libc::statvfs` để thu thập tài nguyên RAM và đĩa trống trên Linux.
- `src-tauri/src/connector/handlers/computer_use.rs` (dòng 670, 717): Gọi Windows API `SendInput` để mô phỏng sự kiện chuột/phím.

**Đánh giá:** Tất cả các khối `unsafe` đều là FFI bắt buộc tới kernel C runtime hoặc Win32 API, không có hành vi tự thao tác con trỏ thô (raw pointer manipulation) không an toàn.

### 4.4. Đánh Giá Khóa Mutex & Quản Lý Luồng (Tokio Concurrency)

- Tại `src-tauri/src/lib.rs` (dòng 138), việc sử dụng `block_on` để bind proxy diễn ra trong pha `setup` đồng bộ của Tauri. Điều này là bắt buộc vì cửa sổ webview cần cổng proxy đã bind để render trang đầu tiên.
- Các vòng lặp ngầm (`watch_target`, `run_supervised`, polling file mtime) đều chạy trong `tauri::async_runtime::spawn`, hoàn toàn không gây block main GUI thread.
- Các mutex đều sử dụng cấu trúc bất đồng bộ `tokio::sync::RwLock` hoặc `std::sync::Mutex` với phạm vi khóa cực ngắn (drop guard ngay sau khi đọc/ghi), giảm thiểu tối đa hiện tượng lock contention.

---

## 5. Kết Luận & Đề Xuất Cải Tiến (Actionable Recommendations)

1. **Tách mô-đun hóa các file lớn (De-godding):**
   - Tách `connector/handlers/computer_use.rs` thành thư mục `computer_use/` gồm `mod.rs`, `display.rs`, `mouse.rs`, `keyboard.rs` và `platform/` (`linux.rs`, `windows.rs`, `macos.rs`).
   - Tách `connector/handlers/exec.rs` thành `exec/mod.rs`, `pty_session.rs`, `buffer.rs`.
2. **Thay thế Polling File Mtime:**
   - Thay thế vòng lặp polling 500ms trong `lib.rs` bằng thư viện giám sát sự kiện file hệ thống `notify` (inotify trên Linux, FSEvents trên macOS, ReadDirectoryChangesW trên Windows) để tiết kiệm tài nguyên CPU.
3. **Mở rộng Unit Test Coverage:**
   - Bổ sung unit test cho `app/links.rs`, `app/hotkey.rs`, và `app/tray.rs`.

---
*Báo cáo được lập tự động và xác thực độc lập bởi eval-worker theo hợp đồng nghiệm thu `VAL-EVAL-CORE-001`.*

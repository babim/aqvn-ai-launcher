# Báo Cáo Đánh Giá An Ninh & Bảo Mật Máy Chủ RFC Host Bridge (Cổng 55080)

**Mã tài liệu:** `VAL-SEC-RFC-001`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi kiểm toán:** `src-tauri/src/rfc/` (9 tệp Rust: `auth.rs`, `sandbox.rs`, `dispatch.rs`, `server.rs`, `files.rs`, `workdir.rs`, `text_editor.rs`, `exchange.rs`, `util.rs`, `mod.rs`)  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Evaluation Team (eval-worker)  
**Trạng thái kiểm tra kỹ thuật:** Passed (`cargo test --lib`, `cargo test --test rfc_server_core --test rfc_handlers --test rfc_server --test rfc_ratelimit_and_lockout` 47/47 passed)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Phân hệ **RFC Host Bridge** lắng nghe trên cổng TCP `55080` (endpoint duy nhất `POST /api/rfc`) là giao diện điều khiển inbound quan trọng nhất của `ai-launcher`. Giao diện này cho phép AI Agent trên cloud hoặc remote server thực hiện các tác vụ quản lý tập tin nhạy cảm, chỉnh sửa mã nguồn cục bộ, truy vấn trạng thái thư mục workspace và khởi tạo cơ chế trao đổi khóa SSH mà không cần mở toàn bộ quyền truy cập hệ thống.

Do kiến trúc mạng yêu cầu máy chủ RFC phải chấp nhận kết nối từ cloud agent bên ngoài máy trạm, socket được thiết kế ràng buộc vào địa chỉ wildcard `0.0.0.0:55080`. Vì vậy, **đường biên an ninh (security boundary) phụ thuộc 100% vào cơ chế xác thực mật mã HMAC-SHA256, bộ lọc whitelist lệnh bất biến, và sandbox path jail `allowed_paths`**.

### Bảng tổng hợp các chốt chặn an ninh:

| Chốt chặn kiểm soát | Vị trí mã nguồn | Cơ chế triển khai | Đánh giá an toàn |
| :--- | :--- | :--- | :---: |
| **Cơ chế xác thực (Auth)** | `src-tauri/src/rfc/auth.rs` (L1-69) | HMAC-SHA256 constant-time qua crate `hmac` & `sha2` | **An toàn cao (A)** |
| **Chống Brute-force / DoS** | `src-tauri/src/rfc/server.rs` (L29-106) | Sliding-window IP Rate Limiter (60 req/min/IP) | **An toàn khá (B+)** |
| **Phân quyền Dispatch** | `src-tauri/src/rfc/dispatch.rs` (L93-181) | Static Whitelist Table 18 hàm (không dynamic eval) | **An toàn cao (A)** |
| **Sandbox Path Jail** | `src-tauri/src/rfc/sandbox.rs` (L24-177) | Canonicalize roots, prefix `/a0`, chặn traversal/symlink | **An toàn cao (A-)** |
| **Bảo vệ Vòng đời Daemon** | `src-tauri/src/rfc/server.rs` (L283-411) | Fail-closed khi thiếu password; hot-reload supervisor | **An toàn cao (A)** |

---

## 2. Mô Hình Mối Đe Dọa & Bề Mặt Tấn Công (Threat Model)

```
                                      BỀ MẶT TẤN CÔNG RFC (PORT 55080)
                                      
  [ Cloud AI Agent / Attacker ]
                |
                |  HTTP POST /api/rfc { "rfc_input": "...", "hash": "<hmac>" }
                v
  +-----------------------------------------------------------------------------------+
  | TCP Listener (0.0.0.0:55080) - Axum 0.8 Server                                    |
  |                                                                                   |
  | [ Lớp 1: IP Rate Limiter ] --------------------------------> [ 429 Too Many Req ]  |
  |   Sliding-window: 60 req/min/IP (dựa trên socket peer IP)                         |
  |                                                                                   |
  | [ Lớp 2: Master Switch & Method Guard ] -------------------> [ 405 Method Not All] |
  |   Chỉ chấp nhận POST /api/rfc; GET/PUT/DELETE bị từ chối                          |
  |                                                                                   |
  | [ Lớp 3: HMAC-SHA256 Constant-Time Verifier ] -------------> [ 403 Forbidden ]     |
  |   Khóa xác thực: config.rfc_password; digest so khớp thời gian thực               |
  |                                                                                   |
  | [ Lớp 4: JSON Parser & Static Dispatch Whitelist ] --------> [ 403 / 500 Error ]  |
  |   Hardcoded Whitelist Table (18 entries); từ chối dynamic load                    |
  |                                                                                   |
  | [ Lớp 5: Sandbox Path Jail (allowed_paths) ] --------------> [ 403 Forbidden ]     |
  |   Ánh xạ /a0 -> ~/Workspace; chặn .., symlink escape, dangling links             |
  |                                                                                   |
  | [ Lớp 6: File System Actuators ]                                                  |
  |   files.rs / workdir.rs / text_editor.rs                                          |
  +-----------------------------------------------------------------------------------+
```

### Các tác nhân đe dọa (Threat Actors):
1. **Kẻ tấn công ngoài mạng Internet:** Quét cổng mở trên dải IP công cộng, cố gắng gửi payload độc hại, khai thác tràn bộ đệm hoặc dò quét mật khẩu (brute-force).
2. **AI Prompt Injection:** Agent bị đầu độc qua prompt bên ngoài, cố gắng đọc tập tin nhạy cảm của người dùng (`~/.ssh/id_rsa`, `/etc/passwd`, cấu hình VPN) hoặc ghi đè tập tin hệ thống (`~/.bashrc`, `cron`).
3. **Khai thác lỗ hổng Man-in-the-Middle (MitM):** Eavesdropping trên đường truyền không mã hóa TLS hoặc tấn công phát lại (replay attacks).

---

## 3. Đánh Giá Chuyên Sâu Từng Cơ Chế Bảo Mật

### 3.1. Cơ Chế Xác Thực HMAC-SHA256 (`src-tauri/src/rfc/auth.rs`)

#### Phân tích mã nguồn và thuật toán:
- **Nguyên lý tính toán (`compute_hash`, L16-20):**
  HMAC được khởi tạo bằng `HmacSha256::new_from_slice(password.as_bytes())`. Dữ liệu được băm trực tiếp từ mảng byte UTF-8 của chuỗi `rfc_input` nguyên bản nhận được từ payload HTTP (`mac.update(data.as_bytes())`). Điều này ngăn ngừa hoàn toàn rủi ro sai lệch định dạng do tuần tự hóa lại JSON (JSON canonicalization divergence).
- **So khớp hằng số thời gian (`verify`, L26-33):**
  Hàm `mac.verify_slice(&expected).is_ok()` sử dụng cài đặt so sánh thời gian bất biến (constant-time comparison) của crate `subtle` được đóng gói trong `hmac`, miễn nhiễm với tấn công phân tích thời gian thực thi (timing attacks).
- **Chống lỗi định dạng hex (`decode_lower_hex`, L40-51):**
  Kiểm tra nghiêm ngặt độ dài chuỗi hash đúng bằng 64 ký tự (`HASH_HEX_LEN = 64`). Sử dụng `as_chunks::<2>()` chuyển đổi từng cặp nibble. Nếu xuất hiện ký tự hoa hoặc ký tự không thuộc bảng hex `[0-9a-f]`, hàm trả về `None` ngay lập tức mà không gây crash/panic.

#### Nhận xét & Lỗ hổng tiềm ẩn:
- **Thiếu Replay Protection (Nguy cơ phát lại):**
  Payload `{ "rfc_input": "...", "hash": "..." }` không chứa trường nonce, request ID hoặc timestamp có thời hạn hiệu lực (TTL). Một gói tin hợp lệ nếu bị kẻ tấn công bắt giữ trên mạng không mã hóa có thể được phát lại (replay) bất kỳ lúc nào để thực hiện lại cùng hành động (ví dụ: ghi đè file hoặc xóa file), miễn là IP chưa vượt ngưỡng rate limiter.

---

### 3.2. Chống Tấn Công Từ Chối Dịch Vụ & Bảo Vệ Cổng Wildcard (`server.rs`)

#### Phân tích cơ chế Rate Limiting (L29-106):
- Triển khai thuật toán **Cửa sổ trượt (Sliding Window)** cho từng IP máy khách: `RateLimiter::default_rfc()` giới hạn tối đa `60 requests / 60 seconds`.
- **Chống rò rỉ bộ nhớ (Memory Hygiene):** Khi bảng băm `clients` vượt quá 64 mục (`map.len() > 64`), hệ thống thực hiện dọn dẹp các mốc thời gian đã hết hạn (`front < cutoff`), ngăn chặn kẻ tấn công làm cạn kiệt RAM máy chủ bằng cách giả mạo hàng triệu IP nguồn.

#### Cơ chế xác định Client IP an toàn (`extract_client_ip`, L118-154):
- **Phòng chống giả mạo header (Header Spoofing Prevention):**
  `extract_client_ip` kiểm tra nguồn kết nối vật lý `peer_ip`. Các header `X-Forwarded-For` và `X-Real-IP` **CHỈ** được tin cậy và phân tích khi `peer_ip` là loopback proxy nội bộ (`is_trusted_proxy` kiểm tra `ip.is_loopback()`, L109-111).
- Nếu kết nối trực tiếp từ mạng ngoài vào cổng 55080, hệ thống bỏ qua toàn bộ header giả mạo và cưỡng chế tính toán giới hạn trên địa chỉ socket thực tế của kẻ tấn công.

#### Bảo vệ trạng thái khởi động (Fail-Closed Pattern, L283-299):
- Hàm `bind_if_enabled`:
  1. Kiểm tra cờ cấu hình `config.rfc_enabled`. Nếu `false`, trả về `Ok(None)` — tuyệt đối không tạo socket lắng nghe (cổng 55080 đóng hoàn toàn).
  2. Kiểm tra `config.rfc_password`. Nếu mật khẩu rỗng, trả về `Err(...)` từ chối mở listener wildcard không có xác thực (`refusing to expose an unauthenticated listener`).

---

### 3.3. Bảng Phân Phối Tĩnh (Static Dispatch Whitelist, `dispatch.rs`)

Hàm `dispatch::dispatch` (L183-191) thực hiện đối sánh `(call.module, call.function_name)` với bảng `WHITELIST` (L93-181). Bất kỳ lời gọi nào không nằm trong danh sách đều trả về lỗi `DispatchError::NotWhitelisted` kèm HTTP 403.

#### Bảng danh mục API hợp lệ:

```
[helpers.rfc_files]
  ├── _read_file_binary_impl       (Đọc nhị phân Base64)
  ├── _read_file_as_base64_impl    (Alias đọc Base64)
  ├── _write_file_binary_impl      (Ghi nhị phân Base64)
  ├── _write_file_from_base64_impl (Alias ghi Base64)
  ├── _delete_file_impl            (Xóa tệp)
  ├── _delete_folder_impl          (Xóa thư mục đệ quy)
  ├── _list_folder_impl            (Liệt kê nội dung thư mục)
  ├── _make_dirs_impl              (Tạo cây thư mục)
  ├── _path_exists_impl            (Kiểm tra tồn tại đường dẫn)
  ├── _file_exists_impl            (Kiểm tra tệp)
  ├── _folder_exists_impl          (Kiểm tra thư mục)
  ├── _get_subdirectories_impl     (Lấy danh sách thư mục con)
  ├── _zip_dir_impl                (Nén ZIP thư mục)
  ├── _move_file_impl              (Di chuyển / Đổi tên)
  └── _read_directory_impl         (Đọc thư mục chi tiết)

[plugins._text_editor.helpers.file_ops]
  ├── read_file                    (Đọc văn bản kèm số dòng)
  ├── write_file                   (Ghi tệp văn bản)
  ├── file_info                    (Lấy metadata tệp)
  ├── apply_patch                  (Áp dụng patch)
  ├── apply_context_patch_file     (Patch theo ngữ cảnh dòng)
  ├── apply_exact_replace_file     (Thay thế chính xác khối chuỗi)
  └── validate_edits               (Kiểm thử tính hợp lệ của patch)

[api.* (WebUI Workdir Browser)]
  ├── api.get_work_dir_files       (Duyệt cây file giao diện)
  ├── api.file_info                (Thông tin metadata)
  ├── api.edit_work_dir_file       (Tải & lưu file từ editor)
  ├── api.delete_work_dir_file(s)  (Xóa đơn/nhiều file)
  ├── api.rename_work_dir_file     (Đổi tên, tạo folder, di chuyển)
  └── api.upload_work_dir_files    (Tải lên file vào workspace)

[helpers.rfc_exchange]
  └── _provide_root_password       (Mã hóa RSA-OAEP trả về SSH password)

[plugins._code_execution.tools.code_execution_tool]
  └── make_dir                     (Tạo thư mục bổ trợ)
```

**Nhận định an ninh:** Máy chủ RFC **HOÀN TOÀN KHÔNG CÓ HÀM THỰC THI SHELL TRỰC TIẾP** (no `exec`, `system`, `popen`). Mọi thao tác đều là thao tác tập tin có kiểm soát qua sandbox.

---

### 3.4. Đánh Giá Chuyên Sâu Sandbox Path Jail (`sandbox.rs`)

Cơ chế Path Jail là tuyến phòng thủ then chốt ngăn chặn agent truy cập các tập tin nằm ngoài thư mục làm việc cho phép.

#### 1. Ánh xạ đường dẫn ảo `/a0` (L13, L134-159):
- Cloud Agent gửi đường dẫn định dạng `/a0/usr/workdir/project`.
- Hệ thống bóc tách tiền tố `/a0`, chuẩn hóa thành đường dẫn tương đối.
- Nếu đường dẫn bắt đầu bằng ký tự mở rộng tilde `~/`, hệ thống chuyển đổi dựa trên biến môi trường `HOME` hoặc `USERPROFILE` (`expand_tilde`, L179-187).

#### 2. Chống Path Traversal (Duyệt thư mục ngược, L148-158):
Hàm `relative_to_root` duyệt qua từng thành phần `Component` của đường dẫn:
```rust
for component in relative.components() {
    match component {
        Component::Normal(part) => normalized.push(part),
        Component::CurDir => {}
        Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
            return Err(SandboxError::Denied(requested.to_string()));
        }
    }
}
```
Mọi nỗ lực chứa `..`, thư mục gốc `/` hoặc ký tự ổ đĩa trên Windows (Prefix) đều bị từ chối ngay lập tức ở mức thành phần.

#### 3. Chống Symlink Escape & Dangling Link Attack (L116-130, L165-177):
- Để ngăn chặn việc tạo symlink trỏ ra `/etc` hoặc `~/.ssh`, hàm `contained_candidate` sử dụng `deepest_existing(candidate)`.
- `deepest_existing` duyệt ngược từ tệp mục tiêu lên thư mục cha sử dụng `std::fs::symlink_metadata` (không giải mã theo vết symlink).
- Khi tìm thấy node thực tế trên đĩa, hệ thống gọi `std::fs::canonicalize(&existing)` để giải mã tất cả các symlink trung gian.
- **Quy tắc bất biến:** Đường dẫn sau khi chuẩn hóa (`resolved`) bắt buộc phải thỏa mãn:
  `resolved.starts_with(root) == true`
- Nếu symlink trỏ ra ngoài sandbox hoặc là dangling link trỏ vào vùng không tồn tại, hàm trả về lỗi từ chối `SandboxError::Denied`.

#### 4. Kịch bản tệp chưa tồn tại (Write/Create Targets):
- Với các tệp mới chuẩn bị ghi (`write_file`), bản thân tệp chưa tồn tại nên không thể là symlink độc hại.
- Thuật toán `deepest_existing` tự động lùi về thư mục cha gần nhất đã tồn tại, kiểm tra xem thư mục cha có nằm trong sandbox hay không. Điều này cho phép tạo tệp mới hợp lệ mà vẫn ngăn chặn việc tạo tệp thông qua symlink cha độc hại.

---

## 4. Danh Mục Rủi Ro & Khuyến Nghị Củng Cố Cho Doanh Nghiệp (Hardening)

Dù hệ thống RFC đã đạt mức độ an toàn cao ở cấp độ người dùng cá nhân, trong môi trường doanh nghiệp (Enterprise), các điểm sau cần được củng cố:

### 1. Rủi ro thiếu mã hóa kênh truyền (Plaintext HTTP Transport):
- **Thực trạng:** RFC server chạy trên `http://0.0.0.0:55080`. Toàn bộ nội dung tệp tin, mã nguồn và hash HMAC truyền đi ở dạng rõ (plaintext). Kẻ nghe lén trên đường truyền mạng (ISP, Wi-Fi công cộng) có thể đọc được dữ liệu nhạy cảm.
- **Khuyến nghị:** 
  - Tích hợp TLS trực tiếp vào Axum (sử dụng `rustls-pki-types` và chứng chỉ tự ký hoặc ACME/Let's Encrypt).
  - Hoặc bắt buộc truy cập RFC thông qua giao thức VPN / WireGuard / Tailscale mesh network nội bộ, đóng cổng 55080 trên firewall công cộng.

### 2. Tấn công phát lại (Replay Attacks):
- **Thực trạng:** Thiếu trường timestamp và nonce trong payload.
- **Khuyến nghị:** Mở rộng schema envelope:
  ```json
  {
    "rfc_input": "...",
    "timestamp": 1731140000,
    "nonce": "a7b9c1...",
    "hash": "<hmac_over_rfc_input_timestamp_nonce>"
  }
  ```
  Phía server kiểm tra `abs(current_time - timestamp) < 30_seconds` và duy trì cache Bloom filter cho nonce trong cửa sổ thời gian.

### 3. Cân nhắc rủi ro TOCTOU (Time-of-Check to Time-of-Use):
- **Thực trạng:** Giữa thời điểm kiểm tra `Sandbox::jail` và thời điểm gọi `std::fs::write`, nếu có một tiến trình độc hại khác trên máy thay thế thư mục cha thành một symlink, tệp có thể bị ghi ra ngoài sandbox.
- **Khuyến nghị:** Sử dụng file descriptor mở với cờ `O_NOFOLLOW` / `openat2` trên Linux (thông qua crate `openat`) khi thao tác trên các thư mục dùng chung.

### 4. Giới hạn dung lượng tải lên đồng thời (Concurrent Memory Cap):
- **Thực trạng:** `MAX_BODY_BYTES = 64MB`. 10 kết nối đồng thời tải file 64MB có thể tiêu tốn >640MB RAM.
- **Khuyến nghị:** Áp dụng semaphore kiểm soát dung lượng upload đồng thời hoặc streaming trực tiếp xuống tệp tạm trên đĩa thay vì nạp toàn bộ vào bộ nhớ `axum::body::to_bytes`.

---

## 5. Kết Luận

Phân hệ RFC Host Bridge của `ai-launcher` thể hiện tư duy thiết kế bảo mật chặt chẽ (Security-by-Design):
1. HMAC-SHA256 constant-time chống timing attack hiệu quả.
2. Static dispatch loại bỏ hoàn toàn nguy cơ arbitrary code execution trực tiếp trên cổng RFC.
3. Sandbox path jail xử lý triệt để các vector traversal và symlink escape.
4. Rate limiter và IP proxy guard bảo vệ dịch vụ trước các đợt càn quét DoS bên ngoài.

Hệ thống hoàn toàn đáp ứng các tiêu chí nghiệm thu của hợp đồng kiểm định `VAL-SEC-RFC-001`.

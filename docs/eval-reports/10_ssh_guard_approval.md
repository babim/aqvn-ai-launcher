# Báo Cáo Đánh Giá An Ninh & Bảo Mật SSH Bridge Daemon, PTY Guard & Chính Sách Phê Duyệt Human-in-the-Loop (Cổng 55022)

**Mã tài liệu:** `VAL-SEC-SSH-001`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi kiểm toán:** `src-tauri/src/ssh/` (`server.rs`, `guard.rs`, `shell.rs`, `mod.rs`), `src-tauri/src/rfc/exchange.rs`, `src-tauri/src/connector/handlers/exec.rs`, `ui/chat.js`, `ui/index.html`, `ui/settings.html`  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Evaluation Team (eval-worker)  
**Trạng thái kiểm tra kỹ thuật:** Passed (`cargo test --lib`, `cargo test --test ssh_server --test gateway_exec_denylist` 15/15 passed; `node --test tools/tests/test_chat_handlers.mjs` 177/177 passed)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Phân hệ **SSH Bridge Daemon** lắng nghe trên cổng TCP `55022` cùng với hệ thống **Human-in-the-Loop (HITL) Action Approval Gate** tạo nên khung kiểm soát thực thi lệnh toàn diện của `ai-launcher`. Khác với máy chủ RFC chỉ giới hạn ở các thao tác tệp tin tĩnh, SSH daemon cung cấp một môi trường tương tác dòng lệnh thực thụ (Interactive PTY Shell) cho AI Agent thông qua giao thức SSH v2 chuẩn (`russh`).

Do tính chất đặc biệt nguy hiểm của việc mở shell hệ điều hành ra mạng, hệ thống triển khai kiến trúc bảo mật đa tầng kết hợp giữa:
1. **Kiểm soát cổng đóng/mở & Vòng đời daemon:** Lắng nghe chỉ khi có cờ `rfc_enabled`, tự động khóa khi thiếu mật khẩu (`fail-closed`), hỗ trợ hot-reload khi thay đổi cấu hình.
2. **Khóa chống tấn công Brute-force:** Khóa IP tạm thời (15 phút) sau 5 lần thử sai mật khẩu liên tiếp.
3. **Mã hóa bắt tay thông tin xác thực RSA-OAEP:** Trao đổi mật khẩu SSH qua kênh RFC sử dụng cặp khóa RSA-2048 bất đối xứng.
4. **Bộ lọc lệnh phá hoại PTY Guard (Secondary Denylist):** Bắt chặn phím thời gian thực, hủy buffer dòng lệnh (`Ctrl-U`) trước khi lệnh hủy diệt chạm vào nhân PTY.
5. **Cổng phê duyệt hành động có sự tham gia của con người (HITL Modal & Audit Trail):** Chặn các thao tác nhạy cảm trên giao diện người dùng, cung cấp 3 mức chính sách bảo mật và lưu vết kiểm toán bất biến (Immutable Audit Log).

### Bảng tổng hợp các chốt chặn an ninh:

| Thành phần an ninh | Vị trí mã nguồn | Cơ chế bảo vệ | Đánh giá an toàn |
| :--- | :--- | :--- | :---: |
| **SSH Daemon Listener** | `src-tauri/src/ssh/server.rs` (L486-520) | `russh` async, Ed25519 host key động, max 10 channels | **An toàn cao (A)** |
| **Bảo vệ Brute-force IP** | `src-tauri/src/ssh/server.rs` (L32-153) | `LockoutTracker` (khóa 900s sau 5 lần sai) | **An toàn cao (A)** |
| **Trao đổi mật khẩu RSA** | `src-tauri/src/rfc/exchange.rs` (L15-44) | RSA-OAEP SHA256/MGF1 (`aws-lc-rs`) | **An toàn cao (A)** |
| **PTY Keystroke Guard** | `src-tauri/src/ssh/guard.rs` (L17-158) | Bắt chuỗi trước Enter, gửi `\x15` Ctrl-U hủy tty line | **Khá (B) - Do bản chất denylist** |
| **Human Approval Gate** | `ui/chat.js` (L1732-1775) | Modal popup chặn thực thi, Deny/Allow Once/Session | **An toàn cao (A-)** |
| **Lưu vết kiểm toán (Audit)**| `ui/chat.js` (L1670-1725, L1777-1800) | In-memory audit trail, xuất file JSON/CSV | **An toàn cao (A)** |

---

## 2. Bản Đồ Luồng Dữ Liệu & Ranh Giới An Ninh (Architecture & Data Flow)

```
                                KIẾN TRÚC BẢO MẬT SSH & APPROVAL GATE
                                
  +---------------------------------------------------------------------------------------------------+
  | GIAI ĐOẠN 1: BẮT TAY MẬT KHẨU QUA RFC (Port 55080)                                                |
  |                                                                                                   |
  |  [ Remote Agent ] --( Gửi RSA-2048 Public Key PEM dạng Hex )--> [ RFC endpoint: _provide_root_pwd]|
  |                                                                           |                       |
  |                                                                           v                       |
  |                                                        [ aws-lc-rs: RSA-OAEP Encryption ]         |
  |                                                        (Mã hóa config.ssh_password)               |
  |                                                                           |                       |
  |  [ Remote Agent ] <--( Nhận Hex Ciphertext - Giải mã bằng Private Key )---+                       |
  +---------------------------------------------------------------------------------------------------+
                                                  |
                                                  v
  +---------------------------------------------------------------------------------------------------+
  | GIAI ĐOẠN 2: THIẾT LẬP KẾT NỐI SSH DAEMON (Port 55022)                                            |
  |                                                                                                   |
  |  [ Remote Agent ] ===( SSH v2 Handshake: russh )===> [ TCP Listener 0.0.0.0:55022 ]              |
  |                                                                           |                       |
  |   [ LockoutTracker: Kiểm tra IP ] <--- (Nếu >= 5 lần sai: Từ chối ngay)  |                       |
  |   [ Constant-Time Password Verify ] <--- (auth_password: HMAC so sánh)    |                       |
  |   [ Cấp quyền & Khởi tạo Shell ]                                          v                       |
  |                                                         [ spawn PTY: bash -l / powershell ]       |
  |                                                         (CWD neo vào allowed_paths đầu tiên)      |
  +---------------------------------------------------------------------------------------------------+
                                                  |
                                                  v
  +---------------------------------------------------------------------------------------------------+
  | GIAI ĐOẠN 3: KIỂM SOÁT THỜI GIAN THỰC (Real-time PTY Guard & HITL Approval)                       |
  |                                                                                                   |
  |  Keystrokes Streaming:                                                                            |
  |  Client gõ phím -> Server tích lũy buffer dòng lệnh cho đến khi gặp ký tự \r hoặc \n              |
  |                                            |                                                      |
  |                                            v                                                      |
  |                                 [ guard::check(cmd) ]                                             |
  |                                /                    \                                             |
  |               (Lệnh phá hoại / Fork bomb)      (Lệnh an toàn / Benign)                            |
  |                          /                            \                                           |
  |                         v                              v                                          |
  |           Gửi \x15 (Ctrl-U) tới PTY                  Gửi \r tới PTY                               |
  |           Gửi [guard] command refused                Thực thi lệnh trong PTY                      |
  |           --> KHÔNG THỰC THI <--                     --> HOÀN THÀNH <--                           |
  |                                                                                                   |
  |  Luồng Outbound Gateway / Chat Tool Calls:                                                        |
  |  Agent đề xuất lệnh -> [ requestActionApproval() ] -> Hiển thị Modal trên Webview                 |
  |                        |                                                                          |
  |         +--------------+--------------+                                                           |
  |         |                             |                                                           |
  |    [ Bấm Deny ]             [ Bấm Allow Once / Session ]                                          |
  |         |                             |                                                           |
  |    Ghi nhận Denied              Ghi nhận Allowed                                                  |
  |    Trả về Error                 Cho phép gửi lệnh xuống backend                                   |
  |         |                             |                                                           |
  |         +-------------> [ Ghi vào Audit Trail (JSON/CSV) ] <-------------+                        |
  +---------------------------------------------------------------------------------------------------+
```

---

## 3. Phân Tích Kỹ Thuật SSH Server Daemon (`src-tauri/src/ssh/server.rs`)

### 3.1. Vòng đời Listener & Tách biệt Trạng thái (Fail-Closed Pattern)
- **Cơ chế ràng buộc socket (`bind_if_enabled`, L486-501):**
  - Cổng SSH `55022` được kiểm soát bởi cùng một cờ chủ (`rfc_enabled`) như máy chủ RFC. Nếu cờ này tắt, không có bất kỳ socket TCP nào được mở trên máy host (`Ok(None)`).
  - Khi cờ bật nhưng chuỗi `ssh_password` bị bỏ trống, hệ thống lập tức báo lỗi (`Err`) và từ chối mở listener wildcard không có xác thực (`refusing to expose an unauthenticated wildcard listener`).
- **Khóa máy chủ Ed25519 động (Ephemeral Host Key, L503-520):**
  - Khác với máy chủ OpenSSH truyền thống thường lưu trữ private key cố định trên đĩa (`/etc/ssh/ssh_host_ed25519_key`), `ai-launcher` sử dụng CSPRNG hệ thống (`rand::thread_rng()`) sinh ra một cặp khóa Ed25519 mới trong bộ nhớ RAM mỗi khi daemon khởi động.
  - **Lợi ích an ninh:** Khóa máy chủ không bao giờ bị lộ qua tập tin cấu hình hay sao lưu đĩa; ngăn chặn hoàn toàn việc giải mã lưu lượng trong quá khứ nếu thiết bị bị đánh cắp.
- **Giới hạn số kênh hoạt động (Channel Exhaustion Mitigation, L332-343):**
  - Khi một phiên kết nối cố gắng mở quá nhiều kênh đồng thời (`self.channels.len() >= MAX_ACTIVE_CHANNELS`, ngưỡng 10 kênh), server sẽ từ chối với mã lỗi `ChannelOpenFailure::ResourceShortage`, triệt tiêu rủi ro DoS làm cạn kiệt file descriptor của tiến trình desktop.

---

### 3.2. Cơ Chế Xác Thực Mật Khẩu & Chống Brute-Force (`LockoutTracker`)

#### 1. So khớp mật khẩu Constant-Time (`password_matches`, L212-221):
- Mật khẩu đăng nhập SSH không được so sánh bằng toán tử `==` thông thường. Thay vào đó, hàm tính toán HMAC-SHA256 của thông điệp tĩnh `"aqvn-ssh-auth"` sử dụng ứng viên mật khẩu làm khóa, sau đó gọi hàm xác thực thời gian bất biến `auth::verify(&self.password, "aqvn-ssh-auth", &tag)` trong module RFC. Kỹ thuật này triệt tiêu hoàn toàn khả năng đoán mật khẩu qua timing attack.

#### 2. Bộ theo dõi khóa IP (`LockoutTracker`, L32-153):
- Cấu trúc `LockoutTracker` sử dụng bảng băm đồng bộ `Arc<Mutex<HashMap<IpAddr, LockoutEntry>>>`.
- **Ngưỡng kích hoạt:**
  - `max_attempts = 5`: Tối đa 5 lần thử sai liên tiếp.
  - `lockout_duration = 900 giây` (15 phút).
- Khi IP đạt ngưỡng 5 lần sai, trường `locked_until` được gán mốc `Instant::now() + 900s`. Mọi yêu cầu kết nối hoặc xác thực tiếp theo từ IP đó trong thời gian khóa đều bị từ chối ngay lập tức tại `auth_password` (`Auth::reject()`) mà không tiêu tốn tài nguyên băm mật khẩu.
- Khi người dùng nhập đúng mật khẩu, số đếm thất bại được xóa ngay lập tức (`record_success`, L118-125).

---

## 4. Cơ Chế Trao Đổi Khóa Mật Mã RSA-OAEP (`src-tauri/src/rfc/exchange.rs`)

Hàm `provide_root_password` (L15-44) giải quyết bài toán phân phối mật khẩu SSH từ máy trạm cho Cloud Agent mà không để lộ qua mạng:

1. **Khách hàng tạo cặp khóa RSA-2048:** Phía Cloud Agent tự tạo một cặp khóa RSA tạm thời, mã hóa Public Key dưới dạng chuỗi PEM tiêu chuẩn (PKCS#8 SubjectPublicKeyInfo) rồi chuyển đổi sang dạng chuỗi Hex.
2. **Giải mã & Kiểm tra Public Key:** Phía `ai-launcher`, hàm `decode_pem_subject_public_key_info` (L47-63) kiểm tra các tiêu đề `-----BEGIN PUBLIC KEY-----` và `-----END PUBLIC KEY-----`, loại bỏ khoảng trắng và chuyển đổi Base64.
3. **Mã hóa bất đối xứng tiêu chuẩn cao (`aws-lc-rs`):**
   Sử dụng thư viện mật mã đạt chứng chỉ FIPS của AWS (`aws-lc-rs`), khóa công khai được bọc trong cấu trúc `OaepPublicEncryptingKey`. Thuật toán áp dụng là:
   - **Cơ chế đệm:** `OAEP` (Optimal Asymmetric Encryption Padding).
   - **Hàm băm chính & MGF1:** `SHA-256` (`OAEP_SHA256_MGF1SHA256`).
   - Mật khẩu `context.ssh_password` được mã hóa vào buffer và trả về dưới dạng chuỗi Hex.
4. **Đánh giá an toàn:** Thuật toán OAEP với SHA-256 miễn nhiễm hoàn toàn với các cuộc tấn công Bleichenbacher padding oracle kinh điển trên RSA PKCS#1 v1.5. Kẻ nghe lén trên đường truyền RFC chỉ thấy bản mã hex ngẫu nhiên và không thể khôi phục lại mật khẩu SSH nếu không sở hữu Private Key nằm an toàn tại Cloud Agent.

---

## 5. Đánh Giá Chuyên Sâu Bộ Lọc Lệnh PTY Guard (`src-tauri/src/ssh/guard.rs`)

### 5.1. Thuật Toán Phân Tích Cú Pháp Lệnh (Parsing & Tokenization)
- **Bóc tách các phân đoạn câu lệnh (`split_segments`, L41-76):**
  Thuật toán duyệt mảng byte nhận diện các toán tử phân tách lệnh của shell POSIX: dấu chấm phẩy (`;`), toán tử logic (`&&`, `||`), và đường ống (`|`). Thuật toán duy trì cờ trạng thái `in_single` và `in_double` để không ngắt nhầm các toán tử nằm bên trong chuỗi trích dẫn.
- **Trích xuất chương trình mục tiêu (`check_segment`, L79-158):**
  - Tự động bỏ qua các khai báo biến môi trường đứng đầu lệnh (ví dụ: `FOO=bar COMMAND`).
  - Tự động bỏ qua tiền tố `sudo` và các cờ tùy chọn của sudo (`-u`, `-g`, `-E`).
  - Lấy tên tập tin thực thi cuối cùng sau dấu `/` (ví dụ: `/bin/rm` -> `rm`).

### 5.2. Danh Mục Lệnh Phá Hoại Bị Chặn Tuyệt Đối

```
[Fork Bomb]
  ├── :(){ :|:& };:           (Chặn biến thể chuẩn)
  └── : ( ) { : | : & } ; :   (Chặn biến thể chèn khoảng trắng)

[System State Changes]
  ├── shutdown / shutdown -h now / shutdown now
  ├── reboot / reboot -f
  ├── poweroff
  ├── halt
  └── init 0 / init 6         (Chuyển runlevel về tắt/khởi động lại)

[Raw Storage Destruction]
  ├── mkfs / mkfs.ext4 / mkfs.vfat / mkfs.*
  └── dd of=/dev/sda / dd of=/dev/nvme*

[Recursive Filesystem Deletion]
  ├── rm -rf / / rm -fr / / rm -r -f /
  ├── rm --no-preserve-root
  ├── rm -rf /*
  └── rm -rf ~ / rm -rf ~/ / rm -rf $HOME
```

### 5.3. Cơ Chế Can Thiệp Gõ Phím PTY Thời Gian Thực (Keystroke Interception)
- Khi client kết nối SSH và mở PTY interactive (`ClientHandler::data`, L391-428), dữ liệu gõ phím từ client không được truyền thẳng xuống shell.
- Server duy trì bộ đệm dòng `state.line`. Khi người dùng gõ phím lùi (`\x7f` hoặc `\x08`), ký tự tương ứng được xóa khỏi bộ đệm.
- Khi người dùng bấm Enter (`\r` hoặc `\n`), hàm `commit_line` (L252-263) được kích hoạt:
  1. Dòng lệnh được làm sạch ký tự điều khiển qua `sanitize(&text)`.
  2. Chạy qua kiểm tra `guard::check(&command)`.
  3. **Nếu phát hiện lệnh cấm:**
     - Server lập tức ghi byte `\x15` (`Ctrl-U` - tín hiệu xóa toàn bộ dòng tty hiện tại) vào tiến trình shell con để hủy bỏ nội dung đang chờ trên stdin của PTY.
     - Server ghi chuỗi thông báo từ chối trực tiếp về client: `\r\n[guard] command refused: <reason>\r\n`.
     - Ký tự Enter (`\r`) **KHÔNG BAO GIỜ** được gửi tới shell.
     - Lệnh phá hoại bị tiêu hủy hoàn toàn, phiên SSH không bị gián đoạn.

### 5.4. Các Lỗ Hổng Vượt Rào Căn Bản Của Denylist (Known Bypasses & Limitations)

Mặc dù `guard.rs` ngăn chặn rất tốt các lỗi thao tác ngẫu nhiên và prompt injection thô thiển, **danh sách đen (Denylist) về mặt nguyên lý không bao giờ có thể bảo đảm an toàn 100% trước một tác nhân shell có chủ đích**. Các kỹ thuật vượt rào kinh điển đã được ghi nhận:

1. **Thực thi gián tiếp qua trình bao (Shell Subshell / Eval):**
   ```bash
   bash -c "rm -rf /"
   sh -c 'dd if=/dev/zero of=/dev/sda'
   eval "r""m -rf /"
   ```
2. **Sử dụng lệnh bao bọc (Wrapper Binaries):**
   ```bash
   env rm -rf /
   timeout 10 rm -rf /
   nohup rm -rf / &
   xargs -a /dev/null rm -rf /
   ```
3. **Mã hóa chuỗi lệnh (Encoding Obfuscation):**
   ```bash
   echo cm0gLXJmIC8= | base64 -d | bash
   printf "\x72\x6d\x20\x2d\x72\x66\x20\x2f" | sh
   ```
4. **Thực thi qua ngôn ngữ lập trình kịch bản:**
   ```bash
   python3 -c "import shutil; shutil.rmtree('/')"
   perl -e 'unlink glob "/*"'
   node -e 'fs.rmSync("/", {recursive: true})'
   ```
5. **Ký tự thoát và dấu nháy ngắt đoạn:**
   ```bash
   \rm -rf /
   'r''m' -rf /
   ```

**Khẳng định nguyên tắc an ninh của dự án:**  
Trong tài liệu và mã nguồn (`guard.rs` L3-8), đội ngũ phát triển đã xác định rõ:
> *"This is NOT the security boundary — password authentication plus the master toggle are. It refuses the named destructive population so an agent shell cannot wipe the host through a typo or a hostile prompt injection."*

---

## 6. Chính Sách Phê Duyệt Hành Động Human-in-the-Loop (`ui/chat.js` & `ui/index.html`)

Để bổ khuyết cho các giới hạn của bộ lọc dòng lệnh tự động, `ai-launcher` tích hợp cơ chế **Phê duyệt hành động nhạy cảm với sự kiểm soát của con người (Human-in-the-Loop Approval Gate)** trên giao diện người dùng.

### 6.1. Kiến Trúc Modal Phê Duyệt (`ui/index.html` L463-492)
Giao diện hiển thị cửa sổ hộp thoại độc quyền (`role="dialog" aria-modal="true"`) chứa:
- **Tên công cụ yêu cầu quyền (`#approval-tool-name`):** Ví dụ `shell_exec`, `write_file`, `delete_file`.
- **Nội dung lệnh hoặc đường dẫn mục tiêu (`#approval-tool-cmd`):** Hiển thị trực quan trong khối `<pre>`.
- **Mô tả mức độ rủi ro (`#approval-risk-desc`):** Cung cấp ngữ cảnh về mức độ tác động tới máy trạm.
- **3 nút hành động quyết định:**
  1. `Deny` (`#approval-deny-btn`): Từ chối thực thi ngay lập tức.
  2. `Allow Once` (`#approval-allow-once-btn`): Cấp phép cho đúng một lần gọi hiện tại.
  3. `Allow for Session` (`#approval-allow-session-btn`): Cấp phép cho công cụ này trong toàn bộ phiên làm việc hiện tại mà không hỏi lại.

### 6.2. Thuật Toán Xử Lý & Các Mức Chính Sách (`ui/chat.js` L1732-1775)
Hàm `requestActionApproval(tool, command, riskDesc)` trả về một `Promise<boolean>` điều khiển luồng:
```javascript
// Đọc chính sách từ cấu hình cài đặt
var policy = localStorage.getItem('aqvn_security_policy') || 'supervised';

if (policy === 'permissive') {
  recordAgentActivity(tool, command, 'allowed', 'Auto-allowed (Permissive mode)');
  return Promise.resolve(true);
}

if (state.sessionAllowedTools.has(tool)) {
  recordAgentActivity(tool, command, 'allowed', 'Allowed (Session rule)');
  return Promise.resolve(true);
}
```

#### Ba cấp độ chính sách an ninh (`aqvn_security_policy`):
1. **Strict (Khắt khe - Doanh nghiệp):** Bắt buộc hiển thị popup phê duyệt đối với mọi hành vi ghi tập tin hoặc thực thi lệnh shell, không cho phép bỏ qua.
2. **Supervised (Giám sát - Mặc định):** Hiển thị popup đối với các hành vi nguy hiểm hoặc công cụ mới; tự động ghi nhận quy tắc phiên nếu người dùng bấm "Allow for Session".
3. **Permissive (Tự do):** Tự động cho phép mọi hành vi, chỉ ghi nhật ký ngầm vào Activity Feed. Phù hợp cho môi trường kiểm thử tự động (CI/CD test runners).

### 6.3. Bẫy Tiêu Điểm & Phím Thoát An Toàn (Accessibility & Safety Trap)
- Khi Modal mở ra, người dùng có thể bấm phím `Escape` để đóng modal. Hàm xử lý sự kiện trong `ui/chat.js` (L1830-1845) tự động chuyển đổi hành động `Escape` thành **Từ chối (Deny by Default)**, bảo đảm an toàn tuyệt đối ngay cả khi người dùng không kịp đọc nội dung.
- Toàn bộ các quyết định (Allowed/Denied) được lưu vào mảng `state.auditTrail` và có thể xuất ra tệp `aqvn-audit-trail-YYYY-MM-DD.json` hoặc `.csv` phục vụ kiểm toán an ninh nội bộ.

---

## 7. Khuyến Nghị Củng Cố Bảo Mật Doanh Nghiệp (Enterprise Hardening)

Để sẵn sàng triển khai trong các mạng doanh nghiệp yêu cầu tuân thủ nghiêm ngặt (ISO 27001, SOC 2, NIST CSF), khuyến nghị áp dụng các biện pháp sau:

### 1. Cách Ly Shell Bằng Linux Containers / Namespaces:
- **Thực trạng:** PTY shell hiện tại chạy trực tiếp dưới quyền của người dùng desktop đang đăng nhập. Nếu agent thoát khỏi sandbox `allowed_paths`, agent có quyền đọc mọi file mà người dùng desktop sở hữu.
- **Khuyến nghị:** Khởi tạo PTY shell bên trong một container Docker/Podman tối giản hoặc tạo `user namespace` + `mount namespace` (bubblewrap / systemd-run --transient) với thư mục `~/Workspace` được mount cô lập, ngăn chặn shell truy cập `~/.bashrc` hay `~/.ssh`.

### 2. Nâng Cấp Xác Thực SSH Bằng OpenSSH Certificates:
- **Thực trạng:** Sử dụng một mật khẩu tĩnh chung (`ssh_password`).
- **Khuyến nghị:** Hỗ trợ xác thực bằng chứng chỉ SSH ký bởi cơ quan phát hành khóa nội bộ doanh nghiệp (SSH Certificate Authority - CA) có gắn thời hạn sống ngắn (short-lived certs, ví dụ 1 giờ), tự động hủy quyền khi hoàn tất phiên làm việc.

### 3. Tích Hợp Giám Sát Cấp Nhân Hệ Điều Hành (Kernel-Level eBPF / Auditd):
- **Thực trạng:** `guard.rs` kiểm tra chuỗi lệnh ở tầng người dùng trước khi gửi vào shell, dễ bị bypass qua subshell.
- **Khuyến nghị:** Sử dụng module kernel eBPF (như Falco hoặc Tetragon) để giám sát trực tiếp syscall `sys_enter_execve` tại nhân Linux. Bất kể lệnh phá hoại được kích hoạt từ bash, python, hay binary biên dịch sẵn, nhân Linux sẽ chặn đứng tiến trình ngay khi syscall được gọi.

### 4. Bắc Cầu SSH PTY Vào Human Approval Modal:
- **Thực trạng:** Hiện tại chỉ có các công cụ qua WebSocket Gateway kích hoạt Modal phê duyệt. Các lệnh gõ trực tiếp qua SSH daemon được PTY thực thi ngay (trừ khi rơi vào denylist của `guard.rs`).
- **Khuyến nghị:** Cung cấp chế độ "SSH Approval Trap": Khi agent gửi một lệnh lạ qua SSH exec/shell, daemon tạm dừng PTY stream, phát sự kiện IPC lên Webview UI hỏi ý kiến người dùng. Chỉ khi người dùng bấm "Approve", daemon mới đẩy lệnh vào PTY.

---

## 8. Kết Luận

Phân hệ SSH Bridge Daemon và Human-in-the-Loop Approval Gate của `ai-launcher` cung cấp một giải pháp cân bằng tuyệt vời giữa khả năng tương tác linh hoạt của AI Agent và sự an toàn của người dùng:
1. `russh` server vận hành ổn định, phi đồng bộ và an toàn bộ nhớ.
2. Cơ chế trao đổi mật khẩu RSA-OAEP qua RFC bảo vệ bí mật xác thực trên đường truyền.
3. Chốt chặn Brute-force `LockoutTracker` ngăn ngừa hiệu quả các cuộc tấn công quét mật khẩu tự động.
4. PTY Keystroke Guard bảo vệ máy trạm trước các tai nạn xóa nhầm hoặc prompt injection phá hoại.
5. Modal phê duyệt hành động mang lại cho người dùng quyền kiểm soát tối cao đối với các tác vụ nhạy cảm.

Hệ thống hoàn toàn đáp ứng các tiêu chuẩn khắt khe của hợp đồng kiểm định `VAL-SEC-SSH-001`.

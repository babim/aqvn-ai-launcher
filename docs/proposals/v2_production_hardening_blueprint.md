# Thiết Kế Kiến Trúc Hoàn Thiện Sản Xuất & Tối Ưu Hóa Thế Hệ Mới (Next-Gen Production Hardening & Optimization Blueprint)

**Mã tài liệu:** `VAL-V2-PROD-BLUEPRINT-001`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Tác giả:** AQVN Architecture & Synthesis Team (`v2-synthesis-worker`)  
**Ngày công bố:** 2026-11-10  
**Trạng thái:** Bản Thảo Kiến Trúc Chuẩn Hóa Sản Xuất (Production RFC)  
**Tài liệu tham chiếu:** 
- `docs/eval-reports/v2_01_crates_architecture.md` (Cargo Workspace & 6 Member Crates)
- `docs/eval-reports/v2_02_cbm_graph_coverage.md` (CBM Graph 7,066 Nodes & 441 Tests)
- `docs/eval-reports/v2_03_web_components_signals.md` (Web Components, Signals & Virtualization)
- `docs/eval-reports/v2_04_terminal_editor_ux.md` (xterm.js WebGL 60fps & CodeMirror 6)
- `docs/eval-reports/v2_05_security_ast_replay.md` (AntiReplayWindow 300s TTL, Bash AST Guard & HITL)
- `docs/eval-reports/v2_06_pool_telemetry_benchmarks.md` (MultiHostPoolHub, Outbox Crash Durability & Size Gates)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Dự án `ai-launcher` (`net.aqvn.workspace`) đã hoàn thành thành công giai đoạn tái cấu trúc nền tảng trong Round 2:
1. **Phân rã đa crate hoàn chỉnh:** Tách biệt 6 member crates trong `crates/` (`aqvn-core`, `aqvn-rfc`, `aqvn-ssh`, `aqvn-proxy`, `aqvn-connector`, `aqvn-agent`) dưới sự quản lý của Cargo Workspace 2021 (`resolver = "2"`), bảo đảm đồ thị phụ thuộc đơn hướng (DAG acyclic tuyệt đối) và đưa daemon headless `aqvn-agent` đạt kích thước siêu nhẹ **2.33 MB** (vượt ngưỡng gác cổng `< 2.80 MB`).
2. **Giao diện Web Components chuẩn W3C:** Loại bỏ hoàn toàn bundler cồng kềnh, đóng gói 8 Custom Elements chuẩn v1 kết hợp Reactive Signals Store (`@preact/signals-core`, 1.6 KB gzipped), duy trì thời gian hiển thị nội dung đầu tiên (FCP) dưới **35ms**.
3. **Bộ đôi công cụ Pro UX:** Tích hợp thành công **xterm.js WebGL GPU rendering** (60fps) kèm tự động phục hồi mất ngữ cảnh (`onContextLoss`) và **CodeMirror 6 In-App Editor** hỗ trợ Visual Diff hai cột nguyên tử (`revert chunk` < 10ms).
4. **An ninh hạt nhân & Độ bền phân tán:** Triển khai cơ chế chống phát lại gói tin `AntiReplayWindow` (cửa sổ trượt 300s TTL) trên cổng `55080`, bộ phân tích cú pháp `Bash AST Guard` khử 100% mẫu lệnh obfuscation kèm cổng phê duyệt `Human-in-the-Loop` (HITL 30s fail-closed) trên cổng `55022`, cùng `MultiHostPoolHub` (35,714 ops/giây, Heartbeat Jitter chống bão mạng) và `Resilient Outbox` khôi phục dữ liệu sau crash trong **0.089 ms**.
5. **Độ tin cậy kiểm thử tuyệt đối:** 100% test suite xanh với **238 Rust unit/integration tests** và **203 Node.js component tests** (tổng cộng 441 tests PASS).

Tuy nhiên, khi đối chiếu với các yêu cầu vận hành ở quy mô doanh nghiệp (Enterprise Production Grade), hệ thống vẫn đối mặt với các ranh giới kỹ thuật cần hoàn thiện:
- **An ninh tầng hạt nhân (Kernel-Level Isolation):** Sandbox hiện tại dựa trên kiểm tra đường dẫn ứng dụng (`crates/aqvn-rfc/src/sandbox.rs`) và bộ lọc cú pháp shell AST (`crates/aqvn-ssh/src/guard/ast.rs`). Cần bổ sung cơ chế cô lập mạng và tài nguyên cấp hệ điều hành (eBPF network filtering, Linux Landlock LSM / seccomp-bpf) để ngăn chặn tận gốc rủi ro zero-day shell escape.
- **Xác thực kênh truyền & Quản lý bí mật:** Cổng RFC 55080 và SSH 55022 hiện sử dụng HMAC-SHA256 và mật khẩu tĩnh/cấu hình file; cần nâng cấp lên mạng lưới zero-trust với xác thực hai chiều mTLS (Mutual TLS), cấp phát chứng chỉ động và mã hóa khóa bảo mật lúc nghỉ (Encryption at Rest) qua hệ thống Keyring phần cứng hệ điều hành.
- **Đồng bộ trạng thái phân tán (Distributed State Sync):** Mô hình Hub-Spoke RPC hiện tại phụ thuộc vào điều phối tập trung; khi nhiều máy chủ Agent cùng thao tác trên cây tệp hoặc phiên làm việc ngoại tuyến, cần động cơ đồng bộ phi xung đột dựa trên CRDTs (Conflict-free Replicated Data Types).
- **Kiến trúc Plugin WebAssembly (WASM):** Cần cho phép mở rộng các công cụ Agent và bộ lọc cú pháp tùy biến một cách an toàn mà không cần biên dịch lại nhị phân lõi.
- **Tối ưu hóa chuỗi công cụ biên dịch (Compilation & Profile-Guided Optimization):** Khai thác PGO (Profile-Guided Optimization), tối ưu hóa bộ cấp phát bộ nhớ (mimalloc/jemalloc), và nâng cao tỷ lệ culling thực tế của danh sách tin nhắn ảo hóa (`<aqvn-virtual-message-list>`).

Tài liệu này công bố **Blueprint Hoàn Thiện Sản Xuất & Tối Ưu Hóa Thế Hệ Mới (Next-Gen Production Hardening & Optimization Blueprint)** nhằm định hướng triển khai các trụ cột công nghệ trên cho các giai đoạn tiếp theo.

---

### Bảng Điểm Kiến Trúc Sản Xuất Thế Hệ Mới (Target Hardening Scorecard)

| Hạng mục kiến trúc | Hiện trạng Round 2 | Mục tiêu Target Blueprint (v2 Production) | Chỉ số đo lường cam kết |
|---|---|---|---|
| **Cách ly mạng & File System** | Ứng dụng: Path Jail + AST Guard | Hạt nhân: eBPF Socket Filter + Landlock LSM | 100% chặn syscalls ngoài danh mục cho phép |
| **Bảo vệ kênh truyền** | HTTP 55080 + SSH 55022 (HMAC/Passwd) | Zero-Trust mTLS với SPIFFE/SPIRE certificate rotation | Cấp chứng chỉ X.509 ngắn hạn (TTL < 24h) |
| **Quản lý khóa & Mật mật** | Config JSON + EnvVars | OS Native Keyring + ChaCha20-Poly1305 Envelope Encryption | Zero secret rò rỉ ra disk/logs, zeroize RAM |
| **Đồng bộ hóa Workspace** | Hub-Spoke RPC via WebSocket | Local-First Distributed CRDT Engine (`aqvn-crdt`) | Phục hồi xung đột 100% tự động, sync latency < 15ms |
| **Cơ chế mở rộng Plugin** | Crate nội bộ biên dịch tĩnh | Sandboxed WASM/WASI Component Model Runtime | Khởi tạo sandbox < 2ms, giới hạn bộ nhớ/CPU cứng |
| **Dung lượng nhị phân Agent** | 2.33 MB (Linux x86_64 strip) | < 2.00 MB với PGO + custom allocator | Giảm thêm 14% dung lượng release |
| **Dung lượng Desktop Shell** | 10.56 MB (Linux Desktop) | < 9.50 MB với Thin LTO + icon compression | Giảm thêm 10% dung lượng release |
| **Ảo hóa DOM Chat Stream** | Spacer Height Simulator | True Viewport Node Culling (Windowed Slicing) | Giữ vững DOM < 80 nodes cho luồng 10,000+ tin |
| **Tốc độ biên dịch CI** | ~75s full workspace compile | < 30s với sccache + mold linker + cargo-nextest | Tăng tốc độ chu kỳ CI/CD 2.5x |

---

## 2. Tổng Hợp Các Phát Hiện & Điểm Nghẽn Kỹ Thuật Từ Round 2 (M1, M2, M3)

Quá trình kiểm toán chuyên sâu 6 báo cáo Round 2 đã làm sáng tỏ cấu trúc toàn diện của `ai-launcher`:

```
+---------------------------------------------------------------------------------------------------+
|                        TỔNG THỂ CÁC PHÁT HIỆN KIỂM TOÁN TỪ ROUND 2                                 |
+---------------------------------------------------------------------------------------------------+
|  M1: WORKSPACE & CBM GRAPH (v2_01, v2_02)                                                         |
|  - 6 member crates DAG đơn hướng tuyệt đối, 0 package cycles.                                     |
|  - CBM Graph: 7,066 nodes, 22,980 edges, 99 source files, 72 test files.                          |
|  - Điểm nghẽn: God files (ui/chat.js 4,243L, computer_use.rs 73 fns), initLayout 46 outgoing calls.|
|  - Cần tách nhỏ các mô-đun connector nặng và tăng tính độc lập của telemetry engine.             |
+---------------------------------------------------------------------------------------------------+
|  M2: WEB COMPONENTS & PRO UX (v2_03, v2_04)                                                       |
|  - 8 Custom Elements W3C v1, @preact/signals-core 1.6KB, FCP < 35ms.                             |
|  - xterm.js WebGL 60fps GPU acceleration (render frame 0.8-1.2ms) + onContextLoss fallback.       |
|  - CodeMirror 6 In-App Editor (100L: 9.49ms, 1,000L: 39.32ms) + Visual Diff atomic revert.        |
|  - Điểm nghẽn: <aqvn-virtual-message-list> chưa culling thật sự; tô màu cú pháp chưa offload      |
|    Web Worker đối với tập tin cực lớn (>50,000L).                                                 |
+---------------------------------------------------------------------------------------------------+
|  M3: SECURITY & BENCHMARK GATES (v2_05, v2_06)                                                    |
|  - AntiReplayWindow 300s TTL (10,000 nonces) cổng 55080, HMAC constant-time.                     |
|  - Bash AST Lexer (718L) + Normalizer (646L) bóc tách 100% obfuscation (quotes, base64, wrappers).|
|  - HITL Approval Modal 30s timeout fail-closed bảo vệ lệnh nhạy cảm.                             |
|  - MultiHostPoolHub 35,714 ops/s, Heartbeat Jitter (8-12s) triệt tiêu bão mạng thundering herd.  |
|  - Resilient Outbox phục hồi crash 0.089ms (zero data loss); Release size gates: Agent 2.33MB.    |
|  - Điểm nghẽn: Chưa có cơ chế bảo vệ hạt nhân (eBPF/LSM), thiếu mTLS và mã hóa dữ liệu tại chỗ.   |
+---------------------------------------------------------------------------------------------------+
```

---

## 3. Trụ Cột 1: Bảo Mật Cấp Doanh Nghiệp (Enterprise Security Hardening)

Kiến trúc bảo mật thế hệ mới chuyển dịch từ phòng thủ lớp ứng dụng (Application-level defense) sang **Mô hình Phòng thủ Chiều sâu Tích hợp Hạt nhân (Kernel-Integrated Defense-in-Depth)**.

```
                           KIẾN TRÚC PHÒNG THỦ HẠT NHÂN THẾ HỆ MỚI
                           
  [ External Network / Cloud Agents ]
                 |
                 v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG 0: eBPF CGroup Socket Filter (crates/aqvn-ebpf)                                         |
  |  - Giám sát mọi socket ingress/egress tại tầng kernel trước khi chạm tới TCP stack người dùng.|
  |  - Ràng buộc cổng 55080 và 55022 chỉ chấp nhận gói tin từ dải IP/mTLS được ủy quyền.         |
  |  - Rate-limiting tức thời tại kernel: DROP các gói tin SYN flood, DDoS mà không tốn CPU Rust.|
  +-----------------------------------------------------------------------------------------------+
                 |
                 v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG 1: Zero-Trust mTLS Bridge Engine                                                         |
  |  - Xác thực chứng chỉ đôi chiều X.509 giữa Client và Hub/Agent.                               |
  |  - Mã hóa toàn bộ luồng truyền bằng TLS 1.3 (Cipher Suites: ChaCha20-Poly1305, AES-256-GCM).  |
  |  - Cấp phát và luân chuyển chứng chỉ tự động định kỳ (SPIFFE Workload ID / Let's Encrypt CA).  |
  +-----------------------------------------------------------------------------------------------+
                 |
                 v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG 2: Ứng Dụng Đa Crate (AntiReplayWindow + Bash AST Lexer / Normalizer)                    |
  |  - AntiReplayWindow 300s TTL + Nonce Cache 10,000 entries.                                    |
  |  - Phân tích cú pháp AST, bóc tách lệnh vỏ bọc, phát hiện fork bomb.                          |
  |  - Human-in-the-Loop (HITL) 30s timeout fail-closed.                                          |
  +-----------------------------------------------------------------------------------------------+
                 |
                 v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG 3: Hộp Cát Hệ Thống Linux Landlock LSM & Seccomp-BPF Jail                                |
  |  - Giới hạn quyền đọc/ghi hệ thống tập tin ở mức syscall (chỉ cho phép thư mục workspace).    |
  |  - Chặn các syscall nguy hiểm: ptrace, kexec_load, bpf (cho child), mount, reboot.           |
  |  - Ngăn chặn hoàn toàn leo thang đặc quyền ngay cả khi tiến trình PTY bị chiếm quyền.         |
  +-----------------------------------------------------------------------------------------------+
```

### 3.1. Hộp Cát Mạng eBPF & Lọc Lưu Lượng Socket
- **Vấn đề hiện tại:** Rate limiting và kiểm tra xác thực cổng 55080/55022 được xử lý bởi `axum` và `russh` trong không gian người dùng (user space). Một cuộc tấn công bão gói tin SYN hoặc flood HTTP có thể gây nghẽn Tokio runtime.
- **Giải pháp đề xuất:**
  - Phát triển crate mới `crates/aqvn-ebpf` sử dụng thư viện `aya` (Rust eBPF framework thuần).
  - Tải chương trình eBPF loại `cgroup/sock_addr` và `xdp` (eXpress Data Path).
  - Lọc lưu lượng trực tiếp tại kernel: chỉ cho phép các gói tin đến từ danh sách IP được ủy quyền hoặc cổng nội bộ loopback.
  - Tự động phát hiện và triệt tiêu các cuộc tấn công replay hoặc quét cổng (port scan) với chi phí CPU gần như bằng 0.

### 3.2. Mạng Lưới Zero-Trust mTLS & Xác Thực Hai Chiều
- **Vấn đề hiện tại:** Kênh truyền RFC 55080 dùng HMAC-SHA256 trên HTTP phẳng; kênh SSH 55022 dùng xác thực mật khẩu. Kẻ tấn công nằm giữa mạng nội bộ có thể nghe lén gói tin hoặc phân tích siêu dữ liệu.
- **Giải pháp đề xuất:**
  - Nâng cấp RFC Host Bridge sang HTTPS với mTLS (Mutual TLS 1.3).
  - Sử dụng `rustls` v0.23 với nền tảng mật mã `aws-lc-rs` (chuẩn FIPS 140-3).
  - Client và Agent trao đổi chứng chỉ X.509 được ký bởi Local Root CA nội bộ được sinh ra tự động lúc cài đặt hoặc tích hợp SPIFFE Workload API.
  - Cơ chế thu hồi chứng chỉ tức thời (CRL) khi phát hiện node agent có dấu hiệu bất thường.

### 3.3. Mã Hóa Bí Mật Lúc Nghỉ (Secret Encryption at Rest) & Keyring Hệ Điều Hành
- **Vấn đề hiện tại:** Khóa HMAC, mật khẩu SSH và token phiên được lưu trong tệp JSON (`config.json`, `settings.json`) hoặc biến môi trường dạng plaintext.
- **Giải pháp đề xuất:**
  - Tích hợp crate `keyring` giao tiếp trực tiếp với kho lưu trữ chứng chỉ an toàn của hệ điều hành:
    - Linux: `Secret Service API` qua D-Bus / `libsecret`.
    - macOS: `Apple Keychain Services`.
    - Windows: `Windows Credential Manager`.
  - Đối với các môi trường headless Linux không có D-Bus: áp dụng kỹ thuật Mã hóa bao bọc (Envelope Encryption) sử dụng thuật toán **ChaCha20-Poly1305** kết hợp dẫn xuất khóa từ Machine-ID và Master Passphrase qua **Argon2id**.
  - Ứng dụng trait `Zeroize` từ crate `zeroize` cho toàn bộ các struct chứa khóa bí mật trong bộ nhớ RAM, bảo đảm dữ liệu nhạy cảm bị ghi đè số 0 ngay khi giải phóng khỏi scope.

---

## 4. Trụ Cột 2: Đồng Bộ Không Gian Làm Việc Phân Tán Dựa Trên CRDTs (Distributed CRDT Workspace Sync)

Khi triển khai môi trường nhiều máy chủ Agent (`MultiHostPoolHub`), việc phối hợp chỉnh sửa tài liệu, cây thư mục và trạng thái phiên giữa người dùng và các Agent đòi hỏi một giải pháp đồng bộ phi tập trung, hỗ trợ làm việc ngoại tuyến (Offline-First) và tự động giải quyết xung đột không cần khóa phân tán.

```
                  KIẾN TRÚC ĐỒNG BỘ PHÂN TÁN AQVN-CRDT
                  
          +--------------------------------------------------+
          |         Local User (CodeMirror 6 Editor)         |
          |  State: DocState { text: Y.Text, undo: UndoMgr }  |
          +--------------------------------------------------+
                                   |
                          CRDT State Vector
                                   v
          +--------------------------------------------------+
          |        crates/aqvn-crdt (Loro / Yrs Engine)      |
          |  - Fast In-Memory DAG Delta Updates (< 1ms)      |
          |  - Compressed Binary State Serialization         |
          |  - Persistent Store: SQLite / redb               |
          +--------------------------------------------------+
                    ^                      ^
                    |                      |
             Peer-to-Peer Sync      P2P WebRTC / WS
                    |                      |
                    v                      v
     +--------------------------+  +--------------------------+
     | Agent Alpha (Headless)   |  | Agent Beta (Remote Server)|
     | crates/aqvn-crdt Node    |  | crates/aqvn-crdt Node    |
     | - Autonomous Code Patch  |  | - Autonomous Code Review |
     | - Local Workspace Jail   |  | - Local Workspace Jail   |
     +--------------------------+  +--------------------------+
```

### 4.1. Đánh Giá & Lựa Chọn Động Cơ CRDT
So sánh các thư viện CRDT hiện đại trong hệ sinh thái Rust:

| Tiêu chí | Automerge (Rust) | Yrs (Yjs Rust Port) | Loro (Rust Native) |
|---|---|---|---|
| **Mô hình dữ liệu** | Op-based CRDT | State-based CRDT (YATA) | State/Op Hybrid (Movable Tree + Text) |
| **Tốc độ Encode/Decode** | Trung bình (~15 MB/s) | Nhanh (~85 MB/s) | **Cực nhanh (> 250 MB/s)** |
| **Dung lượng Document** | Khá lớn (do lưu lịch sử) | Nhỏ gọn | **Tối ưu vượt trội (RLE compression)** |
| **Hỗ trợ Cây Thư Mục** | Phức tạp | Cần tự cài đặt | **Tích hợp sẵn Movable Tree CRDT** |
| **Giao tiếp Webview JS** | WASM bridge | `y-websocket` / WASM | WASM / Shared ArrayBuffer |
| **Đánh giá lựa chọn** | Phù hợp tài liệu tĩnh | Rất tốt cho văn bản thuần | **LỰA CHỌN TỐI ƯU (Khuyên dùng)** |

### 4.2. Thiết Kế Cấu Trúc Dữ Liệu Đồng Bộ `aqvn-crdt`
Phát triển crate chuyên trách `crates/aqvn-crdt`:
1. **Movable Tree CRDT (Workspace File Explorer):** Quản lý cấu trúc cây thư mục. Cho phép Agent di chuyển (move), đổi tên (rename) và tạo mới thư mục đồng thời với người dùng mà không bao giờ gây ra hiện tượng chu trình mồ côi (cycle orphan).
2. **Text Buffer CRDT (CodeMirror 6 In-App Editor):** Quản lý nội dung từng tập tin mã nguồn. Mọi thay đổi vi phân (differential diff / hunk) được áp dụng dưới dạng delta nhị phân nén, loại bỏ hoàn toàn tình trạng mất dữ liệu (overwriting) khi người và máy cùng gõ code.
3. **Session & Awareness CRDT:** Chia sẻ vị trí con trỏ chuột (cursor presence), vùng chọn (selection highlights) và trạng thái suy nghĩ của Agent (`agent thoughts`) theo thời gian thực tới toàn bộ các node trong cluster.
4. **Lưu trữ cục bộ bền vững:** Lưu snapshot CRDT vào `SQLite` nhúng hoặc `redb` siêu nhẹ tại thư mục `.aqvn/workspace.crdt`, kết hợp phục hồi tức thì qua `resilient-outbox`.

---

## 5. Trụ Cột 3: Kiến Trúc Plugin WebAssembly (WASM Plugin Architecture)

Để mở rộng hệ sinh thái công cụ của `ai-launcher` (custom tools, bộ phân tích cú pháp AST chuyên biệt, các cổng giao thức tùy biến) mà không làm tăng dung lượng nhị phân chính hay đe dọa sự ổn định của hệ thống, kiến trúc WASM đóng vai trò như một môi trường thực thi hộp cát tuyệt đối an toàn.

```
                     MÔ HÌNH HỘP CÁT PLUGIN WASM THẾ HỆ MỚI
                     
  +-------------------------------------------------------------------------+
  | AQVN HOST APPLICATION (crates/aqvn-agent / crates/aqvn-core)            |
  |                                                                         |
  |  +-------------------------------------------------------------------+  |
  |  | WASM RUNTIME MANAGER (Wasmtime 20+ Component Model Engine)        |  |
  |  |                                                                   |  |
  |  |  - Fuel Metering Engine (Giới hạn số chỉ lệnh CPU: 10,000,000 ops)|  |
  |  |  - Memory Allocation Guard (Giới hạn cứng RAM: tối đa 32 MB)      |  |
  |  |  - Epoch Interruption Timer (Timeout thực thi cứng: 500 ms)       |  |
  |  +-------------------------------------------------------------------+  |
  |                                  | (WASI Preview 2 Capability Interface)|
  |                                  v                                      |
  |  +-------------------------------------------------------------------+  |
  |  | ISOLATED WASM GUEST INSTANCE (Custom Extension Plugin)             |  |
  |  |                                                                   |  |
  |  |  - Sandbox Memory: Không thể truy cập địa chỉ RAM của Host Process|  |
  |  |  - Virtual FS: Chỉ đọc được thư mục plugin ảo hóa                 |  |
  |  |  - Zero Network: Không được cấp socket trực tiếp (trừ khi cấp quyền)|
  |  +-------------------------------------------------------------------+  |
  +-------------------------------------------------------------------------+
```

### 5.1. Lựa Chọn Runtime & Tiêu Chuẩn Kỹ Thuật
- **Wasmtime Engine (Bytecode Alliance):**
  - Hỗ trợ tiêu chuẩn **WASI Preview 2** và **WebAssembly Component Model**.
  - Tính năng **Fuel Consumption**: Tự động đếm chỉ lệnh CPU thực thi. Khi plugin rơi vào vòng lặp vô tận, nhiên liệu cạn kiệt và engine tự động hủy instance mà không ảnh hưởng tới tiến trình mẹ.
  - Tính năng **Epoch Interruption**: Thiết lập timeout cưỡng bức (ví dụ: 500ms đối với parser, 5s đối với tool) qua đồng hồ định thời của Host.
  - **Memory Limits**: Khóa cứng kích thước bộ nhớ tuyến tính (Linear Memory) tối đa 16 MB – 32 MB cho mỗi guest instance.

### 5.2. Các Trường Hợp Sử Dụng Cốt Lõi (Core Use Cases)
1. **Dynamic AST Security Rule Packs:** Cho phép các tổ chức tải nạp các bộ quy tắc bảo mật tùy biến (custom AST regex/grammar) để phát hiện các mẫu lệnh đặc thù mà không cần cập nhật mã nguồn `crates/aqvn-ssh`.
2. **Custom Agent Tool Plugins:** Lập trình viên có thể viết các công cụ Agent (ví dụ: truy vấn cơ sở dữ liệu nội bộ, kết nối Jira/GitLab, chuyển đổi định dạng tệp) bằng bất kỳ ngôn ngữ nào (Rust, C, Go, AssemblyScript) rồi biên dịch thành tệp `.wasm`.
3. **High-Performance Syntax Tokenizers:** Mở rộng khả năng highlight cú pháp của CodeMirror 6 cho các ngôn ngữ mới bằng các bộ parser compiled sang WASM chạy trong Web Worker.

---

## 6. Trụ Cột 4: Tối Ưu Hóa Chuỗi Biên Dịch & Dung Lượng Nhị Phân (Compilation & Binary Footprint)

Duy trì tính nhỏ gọn của các gói phân phối là nguyên tắc bất biến của dự án. Sau Round 2, hệ thống đã đạt 2.33 MB (Agent) và 10.56 MB (Desktop). Trụ cột này đưa ra các kỹ thuật tối ưu hóa cấp sâu hơn để đẩy giới hạn nhị phân xuống mức kỷ lục.

```
                 QUY TRÌNH TỐI ƯU HÓA BIÊN DỊCH NÂNG CAO (PGO + BOLT)
                 
  +---------------------------------------------------------------------+
  | 1. INSTRUMENTED BUILD: `cargo build --profile release-pgo`           |
  |    Thêm mã theo dõi hiệu năng và đường đi thực thi vào nhị phân.     |
  +---------------------------------------------------------------------+
                                   |
                                   v
  +---------------------------------------------------------------------+
  | 2. PROFILE WORKLOAD COLLECTION: `tools/benchmark.sh`                |
  |    Chạy kịch bản tải mô phỏng thực tế (PTY, RFC, CBM, Hub Routing). |
  |    Sinh tệp nhật ký thực thi: `default_*.profdata`.                 |
  +---------------------------------------------------------------------+
                                   |
                                   v
  +---------------------------------------------------------------------+
  | 3. PGO-OPTIMIZED BUILD: `cargo build --release -Z pgo-use=...`      |
  |    Tái sắp xếp khối mã nguồn (basic blocks), inline hóa các nhánh    |
  |    thường xuyên được gọi, loại bỏ các nhánh dự đoán sai lệch.        |
  +---------------------------------------------------------------------+
                                   |
                                   v
  +---------------------------------------------------------------------+
  | 4. LLVM BOLT POST-LINK OPTIMIZATION: `llvm-bolt`                    |
  |    Tối ưu hóa sắp xếp bộ nhớ đệm lệnh (L1i Cache locality)           |
  |    trên tập tin thực thi ELF đã biên dịch.                           |
  +---------------------------------------------------------------------+
```

### 6.1. Áp Dụng Profile-Guided Optimization (PGO)
- **Cơ chế:** Trình biên dịch LLVM tối ưu hóa cách sắp xếp mã máy dựa trên hồ sơ dữ liệu chạy thực tế thay vì dự đoán tĩnh.
- **Kết quả kỳ vọng:**
  - Tăng tốc độ thực thi các luồng xử lý CPU-bound (như AST parser trong `aqvn-ssh` và giải mã HMAC trong `aqvn-rfc`) thêm **15% – 25%**.
  - Giảm dung lượng nhị phân thêm **5% – 8%** do LLVM chủ động thu gọn các đoạn mã ít được gọi (cold code).

### 6.2. Thay Thế Bộ Cấp Phát Bộ Nhớ (Custom Memory Allocator)
- **Hiện trạng:** Nhị phân đang sử dụng bộ cấp phát mặc định của hệ thống (`glibc malloc` trên Linux).
- **Đề xuất:**
  - Đối với `aqvn-agent`: Giữ nguyên hoặc cấu hình `musl-libc` tĩnh để tối ưu dung lượng và loại bỏ phụ thuộc shared libraries.
  - Đối với `aqvn-workspace` (Tauri Desktop Shell): Tích hợp **`mimalloc`** (`mimalloc = { version = "0.1", default-features = false }`) làm global allocator.
  - **Lợi ích:** Giảm hiện tượng phân mảnh bộ nhớ (heap fragmentation) khi nhận luồng PTY streaming cường độ cao và giảm độ trễ cấp phát bộ nhớ xuống dưới **12ns/op**.

### 6.3. Tối Ưu Hóa Thời Gian Biên Dịch CI/CD
- Sử dụng trình liên kết siêu tốc **`mold`** (hoặc `lld` trên macOS/Windows) thay thế cho GNU `ld`, giảm thời gian linking giai đoạn cuối từ 18s xuống dưới **1.8s**.
- Kích hoạt **`sccache`** với Redis/S3 caching cho GitHub Actions workflow, tái sử dụng 100% artifacts không thay đổi giữa các commit.
- Chuyển đổi bộ chạy test sang **`cargo-nextest`**, tận dụng thực thi song song đa luồng tối đa để rút ngắn thời gian kiểm thử toàn bộ 238 Rust tests từ 75s xuống dưới **18s**.

---

## 7. Trụ Cột 5: Hoàn Thiện Trải Nghiệm Giao Diện Người Dùng (Frontend Hardening & Pro UX)

Báo cáo `v2_03` và `v2_04` đã chứng minh sức mạnh của Web Components và Signals Store, đồng thời chỉ rõ những điểm cần gia cố để giao diện đạt độ tin cậy tuyệt đối trong các phiên làm việc kéo dài hàng tuần.

```
                      MÔ HÌNH VIEWPORT VIRTUALIZATION NÂNG CAO
                      
  Total Message Log (10,000 Messages in Memory Signals Array)
  +--------------------------------------------------------------------+
  | [Spacer Top: Height = 452,000 px] (Giữ đúng độ dài thanh cuộn)    |
  +--------------------------------------------------------------------+
  | VISIBLE VIEWPORT BUFFER (Chỉ render 30-50 Nodes trong DOM thực tế):|
  |   - Message #4521 (\aqvn-chat-message\)                            |
  |   - Message #4522 (\aqvn-chat-message\)                            |
  |   - Message #4523 (\aqvn-chat-message\ với CodeMirror snippet)     |
  |   - Message #4524 (\aqvn-chat-message\)                            |
  |   - Message #4525 (\aqvn-chat-message\)                            |
  +--------------------------------------------------------------------+
  | [Spacer Bottom: Height = 548,000 px]                               |
  +--------------------------------------------------------------------+
```

### 7.1. Cắt Tỉa DOM Khung Nhìn Tuyệt Đối (True Viewport Culling)
- **Hiện trạng:** `<aqvn-virtual-message-list>` hiện tại tính toán khoảng đệm chiều cao nhưng vẫn duy trì các phần tử DOM hiện hữu hoặc dựa vào giải thuật cắt tỉa đơn giản.
- **Giải pháp:**
  - Triển khai giải thuật **Dynamic Windowed Slice**: Chỉ duy trì tối đa **40 – 60 phần tử DOM con** trực tiếp trong khung nhìn hiển thị cộng thêm 5 phần tử đệm trên/dưới (overscan buffer).
  - Khi người dùng cuộn nhanh (fast scrolling), sử dụng `requestAnimationFrame` và tính toán nhị phân vị trí tin nhắn (`binary search index`) dựa trên bản đồ chiều cao ước lượng (`estimatedHeightMap`).
  - **Kết quả:** Giữ mức tiêu thụ RAM của tiến trình Webview luôn ổn định dưới **120 MB** ngay cả khi phiên hội thoại tích lũy hơn 20,000 tin nhắn.

### 7.2. Tách Biệt Tô Màu Cú Pháp Qua Web Worker (Syntax Highlighting Offloading)
- **Hiện trạng:** Khi nạp một tệp mã nguồn lớn (> 10,000 dòng) vào CodeMirror 6, việc tokenize và phân tích cú pháp AST trên main thread gây trễ khung hình (mất 356ms tại 10,000 dòng).
- **Giải pháp:**
  - Chuyển giao toàn bộ tác vụ tính toán token và tạo `HighlightTree` sang một **Dedicated Web Worker** (`ui/workers/syntax-worker.js`).
  - Giao tiếp giữa main thread và worker thông qua cấu trúc dữ liệu nhị phân phẳng (`Transferable ArrayBuffer`), giữ main thread luôn rảnh rỗi để duy trì tần số quét giao diện **60fps** không tì vết.

### 7.3. Thu Gom Bộ Nhớ Đệm xterm WebGL Texture Atlas
- Cài đặt cơ chế tự động tái thiết lập kết cấu đồ họa (Texture Atlas reset) khi chuyển đổi theme OKLCH hoặc khi terminal trải qua hơn 500,000 ký tự stream.
- Thu hẹp bộ đệm cuộn ngược (scrollback) từ 2,000 dòng xuống đệm nén nhị phân dạng ring-buffer khi tab rơi vào trạng thái ẩn (inactive background tab).

---

## 8. Ma Trận Phân Tích Rủi Ro & Chế Độ Thất Bại (Risk Analysis & Failure Modes Matrix)

| Mã Rủi Ro | Nguy cơ / Kịch bản tấn công | Mức độ | Cơ chế phát hiện | Giải pháp phòng thủ & Chế độ Thất bại (Fail-Closed) |
|:---:|---|:---:|---|---|
| **RSK-01** | **Kernel BPF Incompatibility:** Nhân Linux cũ (< 5.8) không hỗ trợ Landlock LSM hoặc eBPF CO-RE. | Trung bình | `aqvn-ebpf` kiểm tra phiên bản kernel tại lúc khởi động (`uname` + syscall test). | **Graceful Degradation:** Tự động fallback về Hộp cát ứng dụng (Path Jail + AST Guard); phát cảnh báo trong telemetry. |
| **RSK-02** | **Replay Attack qua Desync Thời Gian:** Đồng hồ máy chủ bị sai lệch quá 300s dẫn đến từ chối gói tin hợp lệ hoặc chấp nhận gói tin cũ. | Cao | `AntiReplayWindow` ghi nhận tỷ lệ `TimestampDriftExceeded` tăng đột biến. | **Fail-Closed:** Từ chối yêu cầu (HTTP 401); đồng thời kích hoạt cảnh báo đồng bộ NTP trên giao diện Topbar. |
| **RSK-03** | **WASM Memory Exhaustion / CPU Hang:** Plugin tùy biến bị lỗi hoặc chứa mã độc lặp vô tận / ngốn RAM. | Cao | Fuel Metering và Epoch Timer của Wasmtime kích hoạt ngắt quãng. | **Fail-Closed:** Cưỡng bức kết thúc phiên guest instance trong 500ms; cô lập plugin và không làm sập host daemon. |
| **RSK-04** | **CRDT State Divergence / Network Split:** Cụm Agent bị phân mảnh mạng kéo dài dẫn đến phân kỳ lịch sử chỉnh sửa. | Trung bình | Cơ chế so sánh State Vector định kỳ phát hiện phân kỳ phiên bản (Vector Clock mismatch). | **Deterministic Merge:** Áp dụng thuật toán Loro CRDT giải quyết xung đột tất định; tạo snapshot dự phòng tự động. |
| **RSK-05** | **WebGL Context Loss không phục hồi:** Driver card màn hình bị crash khiến WebglAddon không thể khởi tạo lại. | Thấp | Bộ bắt sự kiện `webglAddon.onContextLoss` kích hoạt. | **Graceful Fallback:** Tự động chuyển đổi sang Canvas 2D / DOM Renderer không làm đứt đoạn phiên PTY shell. |
| **RSK-06** | **Rò rỉ Khóa lúc giải phóng bộ nhớ:** Tiến trình bị kill đột ngột để lại vùng nhớ RAM chứa mật khẩu chưa dọn sạch. | Cao | Quét mã nguồn tĩnh qua công cụ Miri và ASAN (AddressSanitizer). | **Zeroization:** Toàn bộ cấu trúc khóa thực thi trait `Zeroize` và `Drop` bảo đảm ghi đè 0 vào RAM ngay khi rời scope. |

---

## 9. Ma Trận Chi Phí / Lợi Ích & Đánh Đổi Kỹ Thuật (Cost-Benefit & Trade-Off Matrix)

| Trụ cột đề xuất | Chi phí kỹ thuật & Độ phức tạp | Tác động tài nguyên (RAM/Binary) | Lợi ích kinh doanh & Vận hành (ROI) | Đánh giá ưu tiên |
|---|---|---|---|:---:|
| **eBPF & Landlock Sandbox** | Cao (Đòi hỏi chuyên môn sâu về Linux Kernel và Rust FFI/Aya). | +150 KB nhị phân; 0 MB RAM phụ trội (xử lý tại kernel). | Ngăn chặn 100% nguy cơ container escape; đạt tiêu chuẩn tuân thủ an ninh ngân hàng/chính phủ. | **P1 (Ưu tiên cao)** |
| **Zero-Trust mTLS & Keyring** | Trung bình (Tích hợp Rustls và OS Keyring APIs). | +200 KB nhị phân; +1.2 MB RAM cho TLS context. | Triệt tiêu hoàn toàn tấn công giả mạo node Agent; bảo vệ 100% bí mật khách hàng. | **P1 (Ưu tiên cao)** |
| **Loro / Yrs CRDT Sync** | Cao (Tái thiết kế tầng đồng bộ trạng thái cây tệp). | +400 KB nhị phân; +8 MB RAM lưu trữ lịch sử vector. | Hỗ trợ cộng tác đa người dùng/đa Agent thời gian thực; loại bỏ hoàn toàn lỗi đè mã nguồn. | **P2 (Chiến lược)** |
| **WASM Component Plugin** | Rất cao (Tích hợp Wasmtime engine vào `aqvn-agent`). | +1.2 MB nhị phân; +16 MB RAM tối đa per sandbox. | Mở rộng hệ sinh thái không giới hạn; cho phép cộng đồng phát triển công cụ an toàn tuyệt đối. | **P2 (Chiến lược)** |
| **PGO + BOLT Optimization** | Thấp (Chỉ điều chỉnh cấu hình build pipeline CI/CD). | **Giảm 5-8% nhị phân; tăng tốc CPU 15-25%**. | Tăng tốc độ phản hồi client; giảm chi phí tài nguyên hạ tầng server. | **P0 (Thực hiện ngay)** |
| **True Viewport DOM Culling** | Thấp (Nâng cấp giải thuật trong Web Component có sẵn). | **Giảm 60% RAM Webview khi chat dài**. | Trải nghiệm cuộn mượt mà 60fps; không bao giờ bị đơ trình duyệt. | **P0 (Thực hiện ngay)** |

---

## 10. Kế Hoạch Hành Động Thực Thi Cụ Thể (Actionable Execution Plan)

Kế hoạch triển khai được phân kỳ thành 4 giai đoạn logic, bảo đảm tính liên tục và không làm gián đoạn 441 bài kiểm thử hiện có:

```
+---------------------------------------------------------------------------------------------------+
| LỘ TRÌNH TRIỂN KHAI TOÀN DIỆN (PHASED EXECUTION ROADMAP)                                          |
+---------------------------------------------------------------------------------------------------+
| GIAI ĐOẠN 1: TỐI ƯU HÓA TỨC THÌ & GIA CỐ NỀN TẢNG (Sprint 1 - 2)                                  |
|  - Nhiệm vụ 1.1: Tích hợp PGO và tối ưu hóa Profile release tại Cargo.toml gốc.                   |
|  - Nhiệm vụ 1.2: Triển khai True Viewport Culling trong <aqvn-virtual-message-list>.              |
|  - Nhiệm vụ 1.3: Chuyển giao bộ tô màu cú pháp CodeMirror 6 sang Web Worker.                     |
|  - Tiêu chí nghiệm thu: FCP < 30ms, RAM chat 10k tin < 120MB, CI test suite hoàn tất < 20s.      |
+---------------------------------------------------------------------------------------------------+
| GIAI ĐOẠN 2: AN NINH ZERO-TRUST & BẢO VỆ HẠT NHÂN (Sprint 3 - 4)                                  |
|  - Nhiệm vụ 2.1: Triển khai mTLS 1.3 và cơ chế cấp chứng chỉ X.509 giữa Client và RFC Bridge.    |
|  - Nhiệm vụ 2.2: Tích hợp Landlock LSM và Seccomp-BPF cho tiến trình thực thi PTY trong aqvn-ssh. |
|  - Nhiệm vụ 2.3: Xây dựng crate aqvn-ebpf thực hiện lọc gói tin socket cgroup cấp kernel.         |
|  - Nhiệm vụ 2.4: Tích hợp OS Keyring và mã hóa khóa bí mật bằng ChaCha20-Poly1305.               |
|  - Tiêu chí nghiệm thu: Chặn 100% hành vi đọc ngoài thư mục sandbox ở cấp syscall; zero secret rò rỉ.|
+---------------------------------------------------------------------------------------------------+
| GIAI ĐOẠN 3: ĐỒNG BỘ PHÂN TÁN CRDT & KHÔNG GIAN LÀM VIỆC (Sprint 5 - 6)                           |
|  - Nhiệm vụ 3.1: Xây dựng crate crates/aqvn-crdt tích hợp động cơ Loro CRDT.                     |
|  - Nhiệm vụ 3.2: Ánh xạ Movable Tree CRDT vào Workspace File Explorer.                           |
|  - Nhiệm vụ 3.3: Kết nối hai chiều Text CRDT với CodeMirror 6 EditorState.                       |
|  - Nhiệm vụ 3.4: Triển khai đồng bộ trạng thái P2P giữa các node trong MultiHostPoolHub.          |
|  - Tiêu chí nghiệm thu: Đồng thời chỉnh sửa 5 node không phát sinh xung đột, delta sync < 15ms.  |
+---------------------------------------------------------------------------------------------------+
| GIAI ĐOẠN 4: HỆ SINH THÁI PLUGIN WASM & PHÁT HÀNH DOANH NGHIỆP (Sprint 7 - 8)                     |
|  - Nhiệm vụ 4.1: Xây dựng crate crates/aqvn-plugin tích hợp Wasmtime Component Model.            |
|  - Nhiệm vụ 4.2: Định nghĩa giao diện WASI Preview 2 capabilities (Filesystem, Network, AST).     |
|  - Nhiệm vụ 4.3: Viết bộ SDK mẫu cho nhà phát triển plugin bên ngoài.                             |
|  - Nhiệm vụ 4.4: Kiểm toán an ninh độc lập và hoàn thiện tài liệu phát hành v2 Production.        |
|  - Tiêu chí nghiệm thu: Thực thi plugin trong sandbox độc lập < 2ms, fuel metering hoạt động chuẩn.|
+---------------------------------------------------------------------------------------------------+
```

---

## 11. Kết Luận & Định Hướng Bàn Giao

Blueprint Hoàn Thiện Sản Xuất & Tối Ưu Hóa Thế Hệ Mới (`VAL-V2-PROD-BLUEPRINT-001`) đã thiết lập một tầm nhìn kỹ thuật rõ ràng, chuẩn mực và khả thi cao cho `ai-launcher`. Bằng cách kết hợp giữa tính an toàn cấp độ hạt nhân (eBPF, Landlock, mTLS), tính bền bỉ của dữ liệu phân tán (Loro CRDTs), khả năng mở rộng không giới hạn (WASM Plugins), và hiệu năng hiển thị siêu tốc (Web Components, Signals, Viewport Culling), hệ thống sẵn sàng đáp ứng các tiêu chuẩn khắt khe nhất trong môi trường vận hành thực tế.

Tài liệu này đóng vai trò là nền tảng kiến trúc trực tiếp để xây dựng **Master Roadmap v2.0 và Các Dự Thảo Đặc Tả OpenSpec Hoàn Chỉnh** tại `docs/proposals/v2_master_roadmap_openspec.md`.

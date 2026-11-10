# Lộ Trình Phát Triển Master Roadmap v2.0 & Các Dự Thảo Đặc Tả OpenSpec Changes Hoàn Chỉnh

**Mã tài liệu:** `VAL-V2-PROD-OPEN-002`  
**Dự án:** AQVN Native Workspace Client, RFC Host Bridge & Headless Agent (`net.aqvn.workspace`)  
**Tác giả:** AQVN Architecture & Synthesis Team (`v2-synthesis-worker`)  
**Ngày công bố:** 2026-11-10  
**Trạng thái:** Dự Thảo Đặc Tả OpenSpec Chính Thức (Official Actionable OpenSpec Draft Specifications)  
**Tài liệu tham chiếu:**  
- `docs/eval-reports/v2_01_crates_architecture.md` (Cargo Workspace 6 Crates Architecture Audit)  
- `docs/eval-reports/v2_02_cbm_graph_coverage.md` (CBM Graph Scan 7,066 Nodes & 441 Tests)  
- `docs/eval-reports/v2_03_web_components_signals.md` (Custom Elements, Signals Store & Virtualization)  
- `docs/eval-reports/v2_04_terminal_editor_ux.md` (xterm.js WebGL 60fps & CodeMirror 6 In-App Editor)  
- `docs/eval-reports/v2_05_security_ast_replay.md` (AntiReplayWindow 300s TTL, Bash AST Guard & HITL)  
- `docs/eval-reports/v2_06_pool_telemetry_benchmarks.md` (MultiHostPoolHub, Outbox Durability & Size Gates)  
- `docs/proposals/v2_production_hardening_blueprint.md` (Next-Gen Production Hardening Blueprint)  

---

## 1. Tầm Nhìn Chiến Lược & Lộ Trình Master Roadmap v2.0

### 1.1. Tầm Nhìn Tổng Thể (Vision & Core Objectives)
Hệ thống `ai-launcher` (`net.aqvn.workspace`) đã hoàn thành xuất sắc giai đoạn tái cấu trúc nền tảng Round 2, sở hữu một kiến trúc phân rã đa crate vững chắc (`crates/aqvn-core`, `aqvn-rfc`, `aqvn-ssh`, `aqvn-proxy`, `aqvn-connector`, `aqvn-agent`), giao diện Web Components chuẩn W3C siêu nhẹ (FCP < 35ms, RAM Signals Store 1.6KB), terminal xterm.js WebGL 60fps, trình soạn thảo CodeMirror 6, và nhị phân daemon headless `aqvn-agent` đạt dung lượng kỷ lục **2.33 MB** (100% test suite xanh với 441 tests).

Bước sang giai đoạn tiếp theo (v2.0 Production Phase), mục tiêu tối thượng là chuyển hóa `ai-launcher` từ một client desktop/daemon nội bộ thành **Nền Tảng Điều Phối Agent Doanh Nghiệp Cấp Độ Hạt Nhân (Kernel-Integrated Enterprise Agent Platform)**. Lộ trình phát triển được xây dựng trên 4 trụ cột công nghệ then chốt:
1. **An ninh hạt nhân tuyệt đối (Zero-Trust Kernel Security):** Cách ly socket và hệ thống tập tin qua **eBPF CGroup Socket Filters** và **Linux Landlock LSM / Seccomp-BPF**, kết hợp xác thực **mTLS 1.3** và mã hóa bí mật lúc nghỉ bằng **OS Keyring** (ChaCha20-Poly1305).
2. **Đồng bộ phân tán phi xung đột (Distributed CRDT Workspace):** Triển khai động cơ CRDT hiệu năng cao (`crates/aqvn-crdt` trên nền Loro) hỗ trợ Movable Tree cho cây thư mục và Text Buffer cho CodeMirror 6, cho phép nhiều Agent và lập trình viên cùng làm việc thời gian thực (Local-First Offline Sync).
3. **Môi trường mở rộng Plugin WebAssembly (Sandboxed WASM Plugin Runtime):** Tích hợp Wasmtime Component Model (WASI Preview 2) vào `aqvn-agent`, kiểm soát tài nguyên nghiêm ngặt bằng Fuel Metering và Epoch Interruption để thực thi an toàn các quy tắc AST và công cụ Agent tùy biến.
4. **Tối ưu hóa siêu việt chuỗi biên dịch & Hiển thị DOM:** Khai thác Profile-Guided Optimization (PGO), LLVM BOLT, bộ cấp phát bộ nhớ `mimalloc`, và thuật toán True Viewport Culling nhằm giữ chân DOM dưới 60 nodes bất kể độ dài hội thoại.

---

### 1.2. Sơ Đồ Chuyển Dịch Kiến Trúc Hệ Thống (Architecture Transition Diagram)

```
===================================================================================================
                   KIẾN TRÚC HIỆN TẠI (ROUND 2 BASELINE - HIỆN HỮU)
===================================================================================================

  [ Browser / Desktop UI ]            [ External Agents / Network ]
             |                                     |
   (Custom Elements W3C + Signals)       (HTTP 55080 HMAC / SSH 55022 AST)
             |                                     |
             v                                     v
  +-----------------------------------------------------------------------------------------------+
  | CARGO WORKSPACE 6 CRATES (DAG Acyclic)                                                        |
  |  - aqvn-core: Primitives, schemas, error types                                                |
  |  - aqvn-rfc: AntiReplayWindow 300s TTL, path jail sandbox                                     |
  |  - aqvn-ssh: Bash AST Lexer/Normalizer, HITL 30s fail-closed modal                             |
  |  - aqvn-proxy: Axum loopback, cookie jar, LRU cache                                           |
  |  - aqvn-connector: Outbound Gateway WSS client                                                |
  |  - aqvn-agent: Linux headless daemon (2.33MB), MultiHostPoolHub 35k ops/s                     |
  +-----------------------------------------------------------------------------------------------+
             |                                     |
             v                                     v
  [ Local Filesystem (POSIX) ]           [ Linux Process PTY (Standard User Space) ]

===================================================================================================
                   KIẾN TRÚC MỤC TIÊU (V2.0 PRODUCTION HARDENED TARGET)
===================================================================================================

  [ Pro UI: Web Components + True Viewport Culling + Web Worker Syntax Highlighting ]
                                         |
                       (Shared ArrayBuffer / Binary Delta)
                                         v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG ĐỒNG BỘ PHÂN TÁN (crates/aqvn-crdt - Loro CRDT Engine)                                   |
  |  - Movable Tree CRDT: Workspace File Explorer đa Agent                                        |
  |  - Text CRDT Buffer: CodeMirror 6 Real-time Collaboration                                     |
  |  - Peer-to-Peer State Vector Sync (Delta updates < 15ms)                                      |
  +-----------------------------------------------------------------------------------------------+
                                         |
                                         v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG HỘP CÁT MỞ RỘNG (crates/aqvn-plugin - Wasmtime Component Model / WASI Preview 2)         |
  |  - Fuel Metering (10M ops limit) + Epoch Timer (500ms timeout)                                |
  |  - Dynamic AST Rules + Custom Agent Tool Extensions                                           |
  +-----------------------------------------------------------------------------------------------+
                                         |
                                         v
  +-----------------------------------------------------------------------------------------------+
  | TẦNG AN NINH ZERO-TRUST & HẠT NHÂN (crates/aqvn-ebpf & Linux Landlock LSM)                    |
  |  - eBPF Socket Filter: Lọc lưu lượng cổng 55080/55022 tại kernel space (triệt tiêu SYN flood)  |
  |  - mTLS 1.3: Xác thực chứng chỉ đôi chiều X.509 + SPIFFE Workload ID                          |
  |  - OS Keyring Vault: Envelope Encryption (ChaCha20-Poly1305 + Argon2id) + Zeroize RAM         |
  |  - Landlock LSM + Seccomp-BPF: Khóa cứng syscalls và đường dẫn tập tin cấp kernel             |
  +-----------------------------------------------------------------------------------------------+
                                         |
                                         v
  [ High-Performance Engine: PGO + BOLT Optimized Binary + mimalloc Allocator (< 2.0MB Agent) ]
```

---

### 1.3. Lộ Trình Phân Kỳ Thực Thi (4-Phase Phased Execution Roadmap)

| Giai đoạn (Phase) | Tên Giai Đoạn | Mã OpenSpec Tương Ứng | Thời Lượng | Trọng Tâm Kỹ Thuật |
|---|---|---|:---:|---|
| **Phase 1** | **Quick-Win Performance & Frontend Hardening** | `change243`, `change244` | Sprint 1–2 (2 tuần) | True Viewport Culling cho tin nhắn hội thoại; chuyển tokenizer CodeMirror 6 sang Web Worker; kích hoạt PGO/BOLT và tối ưu hóa thời gian build CI với mold/cargo-nextest. |
| **Phase 2** | **Kernel-Level Security & Zero-Trust mTLS** | `change245`, `change246` | Sprint 3–4 (2 tuần) | Hộp cát mạng eBPF qua Aya; Landlock LSM cấp syscall; nâng cấp RFC/SSH lên mTLS 1.3; lưu trữ bí mật an toàn qua OS Keyring và Zeroize RAM. |
| **Phase 3** | **Distributed CRDT Workspace & P2P Sync** | `change247` | Sprint 5–6 (2 tuần) | Tạo crate `crates/aqvn-crdt` tích hợp Loro CRDT; đồng bộ cây tệp Movable Tree; gắn kết Text CRDT vào CodeMirror 6; đồng bộ phân tán đa node qua MultiHostPoolHub. |
| **Phase 4** | **Sandboxed WASM Plugin Architecture & Enterprise Release** | `change248` | Sprint 7–8 (2 tuần) | Tạo crate `crates/aqvn-plugin` tích hợp Wasmtime Component Model (WASI Preview 2); nạp động bộ quy tắc bảo mật AST; phát hành SDK công cụ cho Agent; kiểm toán xuất xưởng v2. |

---

## 2. Mục Lục Các Đề Xuất OpenSpec Changes Hoàn Chỉnh

Dưới đây là 6 dự thảo đặc tả OpenSpec hoàn chỉnh, sẵn sàng đưa vào chu trình thực thi `aqvn-opsx-doit`:

| Mã Đề Xuất OpenSpec | Phân Hệ | Tiêu Đề Đề Xuất | Độ Ưu Tiên | Độ Phức Tạp | Giai Đoạn |
| :--- | :--- | :--- | :---: | :---: | :---: |
| **`change243-frontend-viewport-culling-syntax-worker`** | Frontend / Pro UX | Cắt tỉa DOM Viewport thực tế & Web Worker CodeMirror | P0 (Khẩn) | Trung bình (5) | Phase 1 |
| **`change244-build-pgo-bolt-optimization-pipeline`** | Core / Build | Tối ưu hóa biên dịch PGO, LLVM BOLT & mimalloc | P0 (Khẩn) | Thấp (4) | Phase 1 |
| **`change245-security-ebpf-network-sandbox`** | Security / Kernel | Hộp cát mạng eBPF Socket Filter & Linux Landlock LSM | P1 (Cao) | Cao (8) | Phase 2 |
| **`change246-security-mtls-os-keyring-vault`** | Security / Auth | Mạng lưới Zero-Trust mTLS 1.3 & Kho khóa OS Keyring | P1 (Cao) | Trung bình (6) | Phase 2 |
| **`change247-sync-distributed-crdt-workspace`** | Sync / Workspace | Động cơ đồng bộ không gian làm việc phân tán Loro CRDT | P2 (Chiến lược) | Rất cao (9) | Phase 3 |
| **`change248-plugin-wasm-component-model`** | Plugin / Runtime | Kiến trúc Plugin WebAssembly Hộp cát Wasmtime Component | P2 (Chiến lược) | Cao (8) | Phase 4 |

---

# ĐỀ XUẤT 1: `change243-frontend-viewport-culling-syntax-worker`

## 1.1. `proposal.md`

### Why
Báo cáo kiểm toán M2 (`docs/eval-reports/v2_03_web_components_signals.md` và `docs/eval-reports/v2_04_terminal_editor_ux.md`) đã chỉ ra hai hạn chế nghiêm trọng về giao diện người dùng:
1. **Ảo hóa DOM chưa hoàn toàn (Incomplete DOM Virtualization):** Phần tử `<aqvn-virtual-message-list>` hiện tại tính toán khoảng đệm chiều cao (`spacer top/bottom`) nhưng vẫn giữ các nodes tin nhắn trong cây DOM khi cuộn, dẫn đến việc tiêu tốn hơn **280 MB RAM** khi phiên hội thoại tích lũy trên 5,000 tin nhắn.
2. **Khóa luồng chính khi nạp tập tin lớn (Main Thread Freezing):** Trình soạn thảo CodeMirror 6 chạy bộ phân tích cú pháp (AST tokenizer) và tô màu trực tiếp trên main thread. Khi nạp tập tin nguồn trên 10,000 dòng, giao diện bị đơ cứng mất **356 ms**, làm tụt khung hình nghiêm trọng.
3. **Hiện tượng rò rỉ Texture Atlas trong xterm.js WebGL:** Sau hơn 500,000 ký tự streaming hoặc sau nhiều lần đổi theme, bộ đệm texture không được tự động dọn dẹp, gây lãng phí bộ nhớ GPU.

### What Changes
- **243-001**: Nâng cấp giải thuật trong `<aqvn-virtual-message-list>` sang **True Viewport Slicing**:
  - Tính toán chính xác vị trí tin nhắn bằng tìm kiếm nhị phân (`binarySearchIndex`) trên bản đồ chiều cao động (`estimatedHeightMap`).
  - Cắt tỉa (detach) hoàn toàn các node ngoài khung nhìn, chỉ giữ lại đúng **30 đến 50 nodes DOM hiển thị** cộng thêm 5 nodes đệm trên/dưới (`overscan buffer`).
  - Giảm số lượng DOM nodes xuống dưới 60 nodes cố định, duy trì mức tiêu thụ RAM Webview ổn định dưới **110 MB** ngay cả với 20,000 tin nhắn.
- **243-002**: Xây dựng **Dedicated Syntax Highlighting Web Worker** (`ui/workers/syntax-worker.js`):
  - Chuyển toàn bộ tác vụ tính toán token và tạo cây `HighlightTree` cho CodeMirror 6 sang chạy nền trong Web Worker.
  - Sử dụng giao thức truyền nhận dữ liệu qua `Transferable ArrayBuffer` không sao chép bộ nhớ (zero-copy), giữ main thread luôn duy trì tần số quét **60fps**.
- **243-003**: Triển khai cơ chế tự động dọn dẹp và tái tạo Texture Atlas (`webglAddon.clearTextureAtlas()`) trong `<aqvn-terminal-drawer>` khi phát hiện sự kiện đổi theme OKLCH hoặc khi bộ đệm PTY chạm ngưỡng 200,000 ký tự.
- **243-004**: Bổ sung bộ kiểm thử tự động đo lường số lượng DOM nodes và tính năng Web Worker trong `tools/tests/test_chat_handlers.mjs`.

### Goals
- Số lượng DOM nodes trong `<aqvn-virtual-message-list>` không vượt quá 60 nodes khi tải 10,000 tin nhắn mẫu.
- Thời gian trễ main thread khi mở tập tin 10,000 dòng trong CodeMirror 6 giảm từ 356ms xuống dưới **16ms** (đạt tiêu chuẩn 60fps).
- 100% các bài kiểm thử hiện có trong Node test suite (203 tests) tiếp tục vượt qua thành công.

### Non-Goals
- Không thay thế lõi CodeMirror 6 bằng Monaco Editor (giữ nguyên kích thước bundle nhẹ).
- Không làm thay đổi giao diện trực quan hay hành vi cuộn chuột tự nhiên của người dùng.

### Validation Contract Assertions Fulfilled
- `VAL-V2-PROD-FE-001`, `VAL-V2-PROD-FE-002`

---

## 1.2. `tasks.md`

### Execution Tasks
- [ ] C243-001: Cài đặt giải thuật Binary Search Windowed Slicing trong `ui/components/aqvn-virtual-message-list.js`
- [ ] C243-002: Xây dựng Web Worker `ui/workers/syntax-worker.js` cho bộ tô màu cú pháp CodeMirror 6
- [ ] C243-003: Tích hợp cầu nối `Transferable ArrayBuffer` giữa `aqvn-code-editor.js` và syntax worker
- [ ] C243-004: Thêm cơ chế tái thiết lập Texture Atlas tự động trong `ui/components/aqvn-terminal-drawer.js`
- [ ] C243-005: Viết bộ kiểm thử xác thực culling và Web Worker trong `tools/tests/test_chat_handlers.mjs`
- [ ] C243-006: Đo lường FCP và mức tiêu hao RAM thực tế, xác nhận đạt tiêu chí < 110MB RAM

### Task Inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C243-001 | Dynamic Windowed Slicing cho Virtual Message List | frontend-worker | `surf:ui-virtual-list` | `ui/components/aqvn-virtual-message-list.js` | impl | DOM count assertion under 10k messages | — | P0 | 2 | web-components | planned |
| C243-002 | Xây dựng Syntax Worker cho CodeMirror 6 | frontend-worker | `surf:ui-syntax-worker` | `ui/workers/syntax-worker.js` | impl | Worker message exchange test | `C243-001` | P0 | 2 | web-worker | planned |
| C243-003 | Giao tiếp Zero-Copy giữa Editor và Worker | frontend-worker | `surf:ui-editor` | `ui/components/aqvn-code-editor.js` | impl | Benchmark latency < 16ms | `C243-002` | P0 | 2 | editor-ux | planned |
| C243-004 | Tự động Reset WebGL Texture Atlas | frontend-worker | `surf:ui-terminal` | `ui/components/aqvn-terminal-drawer.js` | impl | Texture cleanup event verification | `C243-001` | P1 | 1 | xterm-webgl | planned |
| C243-005 | Cập nhật Suite Test Node.js | frontend-worker | `surf:tests-node` | `tools/tests/test_chat_handlers.mjs` | test | Run `node --test` verify 208+ tests PASS | `C243-003` | P0 | 1 | test-suite | planned |
| C243-006 | Đo kiểm RAM & FCP thực tế | frontend-worker | `surf:eval-reports` | `docs/eval-reports/v2_07_fe_hardened.md` | docs | Verification report check | `C243-005` | P1 | 1 | report | planned |

---

# ĐỀ XUẤT 2: `change244-build-pgo-bolt-optimization-pipeline`

## 2.1. `proposal.md`

### Why
Báo cáo kiểm toán M1 và M3 (`docs/eval-reports/v2_01_crates_architecture.md` và `docs/eval-reports/v2_06_pool_telemetry_benchmarks.md`) chỉ ra:
1. **Dung lượng nhị phân và thời gian biên dịch:** Mặc dù nhị phân headless `aqvn-agent` đã đạt 2.33 MB và Desktop Shell đạt 10.56 MB, mã máy vẫn được sinh ra dựa trên phỏng đoán tĩnh của LLVM mà chưa tận dụng dữ liệu hành vi thực tế (Profile-Guided Optimization).
2. **Bộ cấp phát bộ nhớ chuẩn (System Allocator):** `glibc malloc` trên Linux gây phân mảnh bộ nhớ (heap fragmentation) khi xử lý hàng triệu bản tin WebSocket và PTY chunks truyền liên tục.
3. **Thời gian chạy kiểm thử CI còn cao:** Lệnh `cargo test --all-targets` tốn ~75s và lệnh linking cuối cùng của GNU `ld` mất 18s.

### What Changes
- **244-001**: Thiết lập cấu hình **Profile-Guided Optimization (PGO)** và **LLVM BOLT** trong pipeline đóng gói phát hành:
  - Bổ sung profile `[profile.release-pgo]` trong `Cargo.toml`.
  - Tự động thu thập hồ sơ dữ liệu thực tế (`default_*.profdata`) qua kịch bản giả lập tải thực tế `tools/benchmark.sh`.
  - Giảm thêm 5%–8% dung lượng nhị phân và tăng tốc độ xử lý CPU-bound từ 15%–25%.
- **244-002**: Thay thế bộ cấp phát bộ nhớ toàn cục (Global Allocator) bằng **`mimalloc`** trong `src-tauri` và `aqvn-agent`:
  - Giảm độ trễ cấp phát bộ nhớ xuống dưới 12ns/op và triệt tiêu triệt để hiện tượng phân mảnh bộ nhớ khi stream PTY dài hạn.
- **244-003**: Nâng cấp công cụ CI/CD:
  - Tích hợp trình liên kết siêu tốc **`mold`** (Linux) / **`lld`** (macOS/Windows), rút ngắn thời gian liên kết xuống dưới 2s.
  - Tích hợp **`cargo-nextest`** cho phép chạy song song toàn bộ 238 bài test Rust trong dưới 20s.
- **244-004**: Cập nhật bài kiểm tra gác cổng kích thước nhị phân `tests/release_packaging.rs` phản ánh mục tiêu mới (`< 2.0 MB` cho Agent và `< 9.8 MB` cho Desktop).

### Goals
- Dung lượng nhị phân `aqvn-agent` sau PGO và strip giảm xuống dưới **2.00 MB**.
- Thời gian biên dịch và kiểm thử toàn diện trên CI hoàn thành trong dưới **30 giây**.
- Không làm suy giảm độ tin cậy của bất kỳ bài test nào trong hệ thống.

### Non-Goals
- Không áp dụng `upx` nén thô (tránh kích hoạt cảnh báo giả từ các phần mềm Antivirus doanh nghiệp).

### Validation Contract Assertions Fulfilled
- `VAL-V2-PROD-BUILD-001`, `VAL-V2-PROD-BUILD-002`

---

## 2.2. `tasks.md`

### Execution Tasks
- [ ] C244-001: Cấu hình profile `release-pgo` và cờ rustc PGO trong root `Cargo.toml`
- [ ] C244-002: Thêm phụ thuộc `mimalloc` vào `src-tauri` và `crates/aqvn-agent`
- [ ] C244-003: Xây dựng script thu thập profile tự động `tools/pgo_instrument.sh`
- [ ] C244-004: Cập nhật cấu hình linker `mold` trong `.cargo/config.toml` và tích hợp `cargo-nextest`
- [ ] C244-005: Điều chỉnh ngưỡng kiểm thử trong `src-tauri/tests/release_packaging.rs`

### Task Inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C244-001 | Cấu hình Cargo PGO Profiles | rust-worker | `surf:cargo-config` | `Cargo.toml`, `.cargo/config.toml` | config | `cargo check --profile release-pgo` | — | P0 | 1 | cargo-workspace | planned |
| C244-002 | Tích hợp Bộ Cấp Phát mimalloc | rust-worker | `surf:rust-alloc` | `src-tauri/Cargo.toml`, `crates/aqvn-agent/Cargo.toml` | impl | Memory benchmark allocation latency | `C244-001` | P0 | 1 | memory-alloc | planned |
| C244-003 | Script Thu Thập Profile Tải Thực Tế | system-worker | `surf:build-scripts` | `tools/pgo_instrument.sh` | script | Run script & generate `.profdata` | `C244-001` | P1 | 2 | shell-script | planned |
| C244-004 | Tích hợp Linker mold & cargo-nextest | system-worker | `surf:ci-pipeline` | `Taskfile.yml`, `.github/workflows/ci.yml` | config | CI runtime benchmark < 30s | `C244-002` | P1 | 1 | dev-ops | planned |
| C244-005 | Cập nhật Test Size Gate Release | rust-worker | `surf:tests-rust` | `src-tauri/tests/release_packaging.rs` | test | `cargo test --test release_packaging` | `C244-002` | P0 | 1 | release-gate | planned |

---

# ĐỀ XUẤT 3: `change245-security-ebpf-network-sandbox`

## 3.1. `proposal.md`

### Why
Báo cáo kiểm toán M3 (`docs/eval-reports/v2_05_security_ast_replay.md`) chỉ ra rằng mặc dù cơ chế `AntiReplayWindow` và `Bash AST Guard` hoạt động xuất sắc ở tầng ứng dụng, hệ thống vẫn tồn tại các lỗ hổng cấp hệ điều hành:
1. **Tấn công từ chối dịch vụ (DoS / SYN Flood):** Các cổng nội bộ `55080` (RFC) và `55022` (SSH) lắng nghe trực tiếp trên không gian người dùng. Khi gặp bão gói tin, Tokio runtime bị nghẽn CPU trước khi mã nguồn Rust kịp từ chối kết nối.
2. **Nguy cơ vượt hộp cát ứng dụng (Application Jail Escape):** Hộp cát đường dẫn hiện tại kiểm tra chuỗi ký tự đường dẫn (`path.starts_with(sandbox_root)`). Nếu kẻ tấn công khai thác được một lỗ hổng trong binary để thực thi lệnh hệ thống tùy ý, chúng có thể bỏ qua lớp bảo vệ này thông qua các syscalls thô.

### What Changes
- **245-001**: Khởi tạo crate mới **`crates/aqvn-ebpf`** sử dụng framework **Aya** (Rust eBPF thuần túy):
  - Biên dịch và tải chương trình eBPF loại `cgroup/sock_addr` và `xdp` (eXpress Data Path).
  - Giám sát mọi socket ingress/egress tại tầng kernel; chỉ cho phép các gói tin đến từ danh sách IP loopback hoặc dải mạng mTLS được ủy quyền.
  - Drop tức thời các gói tin vi phạm tại driver card mạng trước khi chạm tới TCP/IP stack người dùng, giảm tải 99% CPU khi bị flood.
- **245-002**: Triển khai **Linux Landlock LSM (Linux Security Module)** cho tiến trình PTY và worker trong `crates/aqvn-ssh`:
  - Khóa quyền truy cập hệ thống tập tin ở cấp độ syscall: chỉ cho phép đọc/ghi duy nhất thư mục làm việc được chỉ định (`workspace_path`).
  - Chặn đứng hoàn toàn mọi nỗ lực duyệt ra `/etc`, `/root`, hoặc các tiến trình khác trong `/proc`.
- **245-003**: Áp dụng **Seccomp-BPF Syscall Filter** thu hẹp danh mục syscalls cho phép:
  - Cấm hoàn toàn các syscall nguy hiểm: `ptrace`, `kexec_load`, `bpf` (đối với child process), `mount`, `reboot`, `setuid`.
- **245-004**: Cơ chế phòng thủ thoái lui mềm (Graceful Degradation): Nếu chạy trên nhân Linux cũ (< 5.8) không hỗ trợ Landlock hoặc trên Windows/macOS, hệ thống tự động fallback về Path Jail và AST Guard mà không làm gián đoạn dịch vụ.
- **245-005**: Bổ sung bộ kiểm thử an ninh mô phỏng tấn công vượt ngục hộp cát trong `crates/aqvn-ebpf/tests/`.

### Goals
- Chặn đứng 100% các hành vi mở tệp ngoài thư mục workspace ở cấp độ kernel syscall.
- Khả năng xử lý hơn 100,000 gói tin vi phạm/giây với mức tiêu thụ CPU của `aqvn-agent` dưới 1%.
- Tự động fallback an toàn trên môi trường phi Linux.

### Non-Goals
- Không thay thế tường lửa mạng phần cứng doanh nghiệp (chỉ tập trung bảo vệ cục bộ máy chủ Agent).

### Validation Contract Assertions Fulfilled
- `VAL-V2-PROD-EBPF-001`, `VAL-V2-PROD-SEC-002`

---

## 3.2. `tasks.md`

### Execution Tasks
- [ ] C245-001: Khởi tạo crate `crates/aqvn-ebpf` và tích hợp vào Cargo Workspace
- [ ] C245-002: Viết mã nguồn chương trình eBPF lọc socket cgroup sử dụng Aya
- [ ] C245-003: Tích hợp thư viện `landlock` và cài đặt bộ lọc tệp cấp syscall trong `crates/aqvn-ssh`
- [ ] C245-004: Tích hợp bộ lọc Seccomp-BPF giới hạn danh mục syscalls cho tiến trình thực thi PTY
- [ ] C245-005: Xây dựng cơ chế phát hiện phiên bản nhân và fallback tự động
- [ ] C245-006: Viết kiểm thử tự động giả lập vi phạm syscall và xác nhận chặn thành công

### Task Inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C245-001 | Khởi tạo crate aqvn-ebpf | rust-worker | `surf:crates-ebpf` | `crates/aqvn-ebpf/` | init | Workspace compilation check | — | P1 | 1 | cargo-workspace | planned |
| C245-002 | CGroup Socket Filter với Aya | rust-worker | `surf:ebpf-programs` | `crates/aqvn-ebpf/src/programs/` | impl | Ingress drop benchmark | `C245-001` | P1 | 3 | kernel-ebpf | planned |
| C245-003 | Khóa Thư Mục Bằng Landlock LSM | rust-worker | `surf:ssh-sandbox` | `crates/aqvn-ssh/src/guard/landlock.rs` | impl | Syscall EACCES assertion test | `C245-001` | P1 | 2 | security-lsm | planned |
| C245-004 | Thu Hẹp Syscalls Bằng Seccomp-BPF | rust-worker | `surf:ssh-seccomp` | `crates/aqvn-ssh/src/guard/seccomp.rs` | impl | Forbidden syscall SIGSYS trigger test | `C245-003` | P1 | 2 | security-seccomp | planned |
| C245-005 | Logic Tự Động Fallback Khi Thiếu LSM | rust-worker | `surf:security-fallback` | `crates/aqvn-ssh/src/guard/mod.rs` | impl | Mock legacy kernel test | `C245-004` | P1 | 1 | compatibility | planned |
| C245-006 | Suite Kiểm Thử Xâm Nhập An Ninh | rust-worker | `surf:tests-ebpf` | `crates/aqvn-ebpf/tests/sandbox_security.rs` | test | `cargo test -p aqvn-ebpf` | `C245-005` | P1 | 2 | test-suite | planned |

---

# ĐỀ XUẤT 4: `change246-security-mtls-os-keyring-vault`

## 4.1. `proposal.md`

### Why
Báo cáo kiểm toán M3 (`docs/eval-reports/v2_05_security_ast_replay.md`) và Blueprint chỉ rõ:
1. **Bảo mật kênh truyền còn dựa trên khóa tĩnh:** Cổng RFC 55080 chạy HTTP phẳng với HMAC ký trên header; cổng SSH 55022 dùng mật khẩu tĩnh hoặc RSA keypair không có cơ chế luân chuyển (rotation). Môi trường mạng nội bộ doanh nghiệp không được coi là vùng an toàn (Zero-Trust Model).
2. **Khóa bí mật lưu trữ dạng văn bản thuần:** Token kết nối, secret HMAC và SSH keys được lưu trong `settings.json` hoặc biến môi trường. Khi bị sao chép ổ đĩa trái phép hoặc chụp ảnh bộ nhớ (RAM dump), các bí mật này dễ dàng bị trích xuất.

### What Changes
- **246-001**: Nâng cấp kết nối RFC 55080 và SSH 55022 lên chuẩn **Mutual TLS 1.3 (mTLS)**:
  - Tích hợp `rustls` v0.23 sử dụng crypto provider `aws-lc-rs` (chuẩn FIPS 140-3).
  - Cả Client và Agent đều phải xuất trình chứng chỉ X.509 hợp lệ được ký bởi Root CA nội bộ được tạo tự động lúc cài đặt hoặc cấp phát từ SPIFFE Workload API.
  - Hỗ trợ luân chuyển chứng chỉ tự động định kỳ (Certificate Rotation, TTL < 24h) mà không làm ngắt quãng phiên kết nối.
- **246-002**: Tích hợp **OS Native Keyring Vault** qua crate `keyring`:
  - Lưu trữ toàn bộ master secrets vào kho an toàn của hệ điều hành (Linux Secret Service / D-Bus, macOS Keychain, Windows Credential Manager).
- **246-003**: Triển khai cơ chế **Mã hóa bao bọc (Envelope Encryption)** đối với các tệp cấu hình trên đĩa:
  - Dữ liệu cấu hình nhạy cảm được mã hóa bằng thuật toán **ChaCha20-Poly1305** với khóa dẫn xuất qua **Argon2id** (kết hợp Machine-ID và Master Passphrase).
- **246-004**: Triển khai trait **`Zeroize`** (crate `zeroize`) cho toàn bộ cấu trúc dữ liệu chứa khóa và chứng chỉ trong bộ nhớ RAM:
  - Tự động ghi đè byte 0 vào vùng nhớ ngay khi cấu trúc rời khỏi phạm vi thực thi (out of scope), triệt tiêu nguy cơ dump bộ nhớ.
- **246-005**: Bổ sung bộ kiểm thử mTLS handshake và kiểm tra Zeroize trong `crates/aqvn-rfc/tests/`.

### Goals
- 100% các kết nối RFC/SSH được bảo vệ bằng TLS 1.3 với xác thực hai chiều mTLS.
- Zero secret dạng plaintext xuất hiện trong tệp cấu hình hoặc bản ghi nhật ký (log files).
- Vùng nhớ chứa khóa được dọn dẹp sạch sẽ 100% khi tiến trình giải phóng bộ nhớ.

### Non-Goals
- Không phụ thuộc vào các dịch vụ đám mây bên thứ ba như HashiCorp Vault (ưu tiên tính tự chứa local-first).

### Validation Contract Assertions Fulfilled
- `VAL-V2-PROD-MTLS-001`, `VAL-V2-PROD-KEYRING-002`

---

## 4.2. `tasks.md`

### Execution Tasks
- [ ] C246-001: Nâng cấp máy chủ Axum trong `crates/aqvn-rfc` hỗ trợ mTLS 1.3 với `rustls`
- [ ] C246-002: Xây dựng trình quản lý chứng chỉ nội bộ và cấp phát SPIFFE ID trong `crates/aqvn-core`
- [ ] C246-003: Tích hợp thư viện `keyring` giao tiếp với Linux Secret Service / macOS Keychain / Windows Credential
- [ ] C246-004: Cài đặt mô-đun mã hóa cấu hình ChaCha20-Poly1305 + Argon2id trong `crates/aqvn-core/src/crypto/`
- [ ] C246-005: Thực thi trait `Zeroize` và `Drop` cho các struct `SecretKey`, `TokenCredentials`
- [ ] C246-006: Viết suite kiểm thử mTLS handshake và kiểm chứng Zeroize trong RAM

### Task Inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C246-001 | Nâng cấp Máy Chủ RFC sang mTLS 1.3 | rust-worker | `surf:rfc-mtls` | `crates/aqvn-rfc/src/server.rs` | impl | mTLS handshake connection test | — | P1 | 2 | networking-tls | planned |
| C246-002 | Quản Lý Chứng Chỉ X.509 Cục Bộ | rust-worker | `surf:cert-manager` | `crates/aqvn-core/src/cert.rs` | impl | Certificate generation & rotation test | `C246-001` | P1 | 2 | crypto-pki | planned |
| C246-003 | Tích hợp OS Keyring Storage | rust-worker | `surf:keyring-vault` | `crates/aqvn-core/src/vault/keyring.rs` | impl | Secret save and retrieve test | `C246-001` | P1 | 2 | security-vault | planned |
| C246-004 | Mã Hóa Envelope ChaCha20-Poly1305 | rust-worker | `surf:envelope-crypto` | `crates/aqvn-core/src/vault/envelope.rs` | impl | Ciphertext decrypt & MAC verify test | `C246-003` | P1 | 2 | crypto-cipher | planned |
| C246-005 | Kích Hoạt Zeroize RAM Sanitization | rust-worker | `surf:zeroize-memory` | `crates/aqvn-core/src/types/secret.rs` | impl | Memory sanitization drop test | `C246-004` | P1 | 1 | memory-security | planned |
| C246-006 | Suite Kiểm Thử Bảo Mật Toàn Diện | rust-worker | `surf:tests-security` | `crates/aqvn-rfc/tests/mtls_vault_test.rs` | test | `cargo test -p aqvn-rfc` | `C246-005` | P1 | 1 | test-suite | planned |

---

# ĐỀ XUẤT 5: `change247-sync-distributed-crdt-workspace`

## 5.1. `proposal.md`

### Why
Báo cáo kiểm toán M1 và Blueprint chỉ ra hạn chế của mô hình Hub-Spoke RPC hiện tại:
1. **Ghi đè tệp khi cộng tác đa Agent (File Overwriting Races):** Khi nhiều máy chủ Agent trong `MultiHostPoolHub` hoặc khi người dùng và Agent cùng chỉnh sửa một tệp mã nguồn trong workspace, hệ thống dùng cơ chế ghi đè tệp toàn bộ (`save_file`), dẫn đến nguy cơ xung đột (race conditions) và mất dữ liệu.
2. **Thiếu hỗ trợ làm việc ngoại tuyến (Offline-First):** Nếu mạng bị ngắt quãng, các thay đổi trên máy khách không thể hòa nhập mượt mà vào máy chủ từ xa khi có mạng trở lại.
3. **Cây thư mục phân tán phức tạp:** Các thao tác di chuyển (move), tạo mới và xóa thư mục giữa các node chưa được đồng bộ hóa tất định.

### What Changes
- **247-001**: Khởi tạo crate mới **`crates/aqvn-crdt`** tích hợp động cơ **Loro CRDT** (Rust-native CRDT siêu tốc, > 250 MB/s encode/decode):
  - Hỗ trợ mô hình dữ liệu lai (State/Op Hybrid CRDT) với thuật toán nén nhị phân RLE.
- **247-002**: Triển khai **Movable Tree CRDT** cho Trình duyệt cây thư mục Workspace File Explorer:
  - Cho phép người dùng và các Agent di chuyển (move), đổi tên (rename) thư mục đồng thời mà không bao giờ sinh ra lỗi chu trình mồ côi (cycle orphan).
- **247-003**: Triển khai **Text Buffer CRDT** tích hợp trực tiếp vào trình soạn thảo CodeMirror 6:
  - Mọi thao tác gõ code được chuyển hóa thành các delta nhị phân vi phân nhỏ gọn (< 100 bytes).
  - Tự động hòa nhập các thay đổi đồng thời mà không cần cơ chế khóa tệp (lock-free collaborative editing).
- **247-004**: Triển khai **Session Awareness CRDT**:
  - Đồng bộ vị trí con trỏ chuột (cursor position), vùng chọn mã nguồn (selection highlights) và dòng suy nghĩ của Agent (`agent thoughts stream`) theo thời gian thực tới toàn bộ các node.
- **247-005**: Tích hợp cơ sở dữ liệu nhúng siêu nhẹ **`redb`** (hoặc SQLite) để lưu trữ bền vững snapshot CRDT tại thư mục `.aqvn/workspace.crdt`, kết hợp phục hồi tức thì qua `resilient-outbox`.
- **247-006**: Bổ sung bộ kiểm thử kiểm chứng giải quyết xung đột đồng thời trong `crates/aqvn-crdt/tests/`.

### Goals
- Độ trễ đồng bộ hóa delta giữa các node trong mạng nội bộ dưới **15 ms**.
- Khả năng tự động hòa nhập 100% các xung đột chỉnh sửa đồng thời giữa 5 máy chủ Agent mà không làm mất dòng mã nào.
- Giữ dung lượng nhật ký CRDT nhỏ gọn nhờ cơ chế snapshot tự động định kỳ.

### Non-Goals
- Không biến `ai-launcher` thành một dịch vụ lưu trữ đám mây thay thế Google Drive/Dropbox.

### Validation Contract Assertions Fulfilled
- `VAL-V2-PROD-CRDT-001`, `VAL-V2-PROD-CRDT-002`

---

## 5.2. `tasks.md`

### Execution Tasks
- [ ] C247-001: Khởi tạo crate `crates/aqvn-crdt` và khai báo phụ thuộc `loro`
- [ ] C247-002: Xây dựng cấu trúc dữ liệu Movable Tree CRDT quản lý cây thư mục tệp
- [ ] C247-003: Xây dựng cấu trúc Text Buffer CRDT và cầu nối giao tiếp với CodeMirror 6
- [ ] C247-004: Cài đặt kênh đồng bộ Awareness State qua WebSocket và MultiHostPoolHub
- [ ] C247-005: Tích hợp kho lưu trữ snapshot nhúng `redb` bền vững trên đĩa
- [ ] C247-006: Viết component UI hiển thị con trỏ Agent trong CodeMirror 6
- [ ] C247-007: Viết suite kiểm thử mô phỏng 5 node phân tán chỉnh sửa mã đồng thời

### Task Inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C247-001 | Khởi tạo crate aqvn-crdt | rust-worker | `surf:crates-crdt` | `crates/aqvn-crdt/` | init | Workspace compilation check | — | P2 | 1 | cargo-workspace | planned |
| C247-002 | Movable Tree CRDT cho Cây Thư Mục | rust-worker | `surf:crdt-tree` | `crates/aqvn-crdt/src/tree.rs` | impl | Concurrent move/rename collision test | `C247-001` | P2 | 3 | crdt-engine | planned |
| C247-003 | Text Buffer CRDT cho CodeMirror 6 | rust-worker | `surf:crdt-text` | `crates/aqvn-crdt/src/text.rs` | impl | Concurrent text edits merge test | `C247-001` | P2 | 3 | crdt-engine | planned |
| C247-004 | Awareness Sync qua MultiHostPoolHub | rust-worker | `surf:crdt-awareness` | `crates/aqvn-crdt/src/awareness.rs` | impl | Cursor presence broadcast test | `C247-003` | P2 | 2 | p2p-sync | planned |
| C247-005 | Lưu Trữ Snapshot Bền Vững redb | rust-worker | `surf:crdt-store` | `crates/aqvn-crdt/src/store.rs` | impl | Crash recovery snapshot reload test | `C247-002` | P2 | 2 | persistence | planned |
| C247-006 | Giao Diện Hiển Thị Con Trỏ Đa Node | frontend-worker | `surf:ui-editor-collab` | `ui/components/aqvn-code-editor.js` | impl | Collaborative cursor render test | `C247-004` | P2 | 2 | editor-ux | planned |
| C247-007 | Suite Kiểm Thử Hòa Nhập Phân Tán | rust-worker | `surf:tests-crdt` | `crates/aqvn-crdt/tests/sync_convergence.rs` | test | `cargo test -p aqvn-crdt` | `C247-005` | P2 | 2 | test-suite | planned |

---

# ĐỀ XUẤT 6: `change248-plugin-wasm-component-model`

## 6.1. `proposal.md`

### Why
Báo cáo kiểm toán M1 và Blueprint xác định nhu cầu mở rộng hệ thống:
1. **Thiếu khả năng mở rộng động (No Dynamic Extensibility):** Để thêm một bộ quy tắc bảo mật AST mới hoặc một công cụ Agent đặc thù (ví dụ: query PostgreSQL nội bộ, tích hợp GitLab), hiện tại bắt buộc phải chỉnh sửa mã nguồn Rust và biên dịch lại toàn bộ nhị phân `aqvn-agent`.
2. **Rủi ro an ninh khi chạy mã bên thứ ba (Security Risks of Native Plugins):** Việc tải trực tiếp thư viện động (`.so`/`.dll`) đem lại rủi ro nghiêm trọng về rò rỉ bộ nhớ, treo tiến trình (segfault) hoặc mã độc leo thang đặc quyền.
3. **Cần môi trường thực thi hộp cát với định mức tài nguyên nghiêm ngặt (Strict Resource Metering):** Cần một giải pháp cho phép thực thi mã mở rộng với giới hạn cứng về RAM, CPU và thời gian thực thi.

### What Changes
- **248-001**: Khởi tạo crate mới **`crates/aqvn-plugin`** tích hợp runtime **Wasmtime 20+ Component Model**:
  - Hỗ trợ tiêu chuẩn **WASI Preview 2** (`wasi:cli`, `wasi:filesystem`, `wasi:clocks`).
  - Hộp cát hóa tuyệt đối: Guest instance chạy trên không gian địa chỉ bộ nhớ tuyến tính riêng biệt, không thể đọc/ghi bộ nhớ của Host daemon.
- **248-002**: Triển khai cơ chế **Định mức Nhiên liệu (Fuel Metering)**:
  - Thiết lập định mức thực thi cứng (ví dụ: 10,000,000 chỉ lệnh WebAssembly cho mỗi tác vụ phân tích).
  - Khi plugin bị treo hoặc rơi vào vòng lặp vô tận, nhiên liệu cạn kiệt và runtime tự động ngắt instance mà không ảnh hưởng tới tiến trình chính.
- **248-003**: Cài đặt bộ định thời **Epoch Interruption & Memory Limits**:
  - Giới hạn cứng dung lượng RAM tối đa **16 MB – 32 MB** cho mỗi plugin instance.
  - Thiết lập timeout thực thi cưỡng bức (500 ms đối với AST rules, 5s đối với Agent tools) qua tín hiệu Epoch của Host.
- **248-004**: Định nghĩa giao diện Component Model cho các trường hợp cốt lõi:
  - `plugin-ast-guard`: Nạp động các tập quy tắc bảo mật cú pháp tùy biến.
  - `plugin-agent-tool`: Đóng gói các công cụ mở rộng của Agent (nhận input JSON, trả về output JSON có kiểm soát).
- **248-005**: Phát hành **Bộ SDK Mẫu (Rust & AssemblyScript SDK)** và bộ kiểm thử an ninh plugin trong `crates/aqvn-plugin/tests/`.

### Goals
- Thời gian khởi tạo một instance hộp cát WASM dưới **2 ms**.
- 100% các plugin lỗi (vòng lặp vô hạn hoặc tiêu tốn bộ nhớ vượt mức) bị runtime cô lập và triệt tiêu an toàn trong dưới 500 ms.
- Cung cấp cơ chế mở rộng an toàn tuyệt đối cho cộng đồng và doanh nghiệp.

### Non-Goals
- Không cấp quyền truy cập socket mạng tùy tiện cho guest plugin (chỉ cấp phép thông qua các Host capabilities được người dùng phê duyệt).

### Validation Contract Assertions Fulfilled
- `VAL-V2-PROD-WASM-001`, `VAL-V2-PROD-WASM-002`

---

## 6.2. `tasks.md`

### Execution Tasks
- [ ] C248-001: Khởi tạo crate `crates/aqvn-plugin` và tích hợp `wasmtime` Component Model
- [ ] C248-002: Cài đặt động cơ đo lường chỉ lệnh CPU (Fuel Consumption) và Epoch Interruption Timer
- [ ] C248-003: Định nghĩa file WIT interface (`wit/aqvn-plugin.wit`) cho AST Guard và Agent Tool
- [ ] C248-004: Xây dựng trình nạp và quản lý vòng đời plugin từ thư mục `~/.config/net.aqvn.workspace/plugins/`
- [ ] C248-005: Xây dựng bộ SDK mẫu bằng Rust (`examples/plugins/custom-ast-rule/`)
- [ ] C248-006: Viết suite kiểm thử tự động kiểm chứng ngắt vòng lặp vô tận và khóa bộ nhớ RAM

### Task Inventory

| ID | Title | Owner skill | Affected surface IDs | Write scope | Task type | Test strategy | Dependencies | Priority | Complexity | Surface hint | Status |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C248-001 | Khởi tạo crate aqvn-plugin | rust-worker | `surf:crates-plugin` | `crates/aqvn-plugin/` | init | Workspace compilation check | — | P2 | 1 | cargo-workspace | planned |
| C248-002 | Fuel Metering & Epoch Timer Engine | rust-worker | `surf:wasm-metering` | `crates/aqvn-plugin/src/runtime.rs` | impl | Infinite loop termination test < 500ms | `C248-001` | P2 | 3 | wasm-runtime | planned |
| C248-003 | Định Nghĩa WIT Interface | rust-worker | `surf:wasm-wit` | `crates/aqvn-plugin/wit/` | impl | wit-bindgen code generation test | `C248-001` | P2 | 2 | wasm-interface | planned |
| C248-004 | Plugin Lifecycle & Loader Manager | rust-worker | `surf:wasm-loader` | `crates/aqvn-plugin/src/loader.rs` | impl | Dynamic `.wasm` loading and execution | `C248-002` | P2 | 2 | plugin-system | planned |
| C248-005 | Bộ SDK Mẫu Cho Plugin Nhà Phát Triển | rust-worker | `surf:plugin-sdk` | `examples/plugins/` | sample | Compile sample plugin to WASM | `C248-003` | P2 | 2 | sdk-example | planned |
| C248-006 | Suite Kiểm Thử Cô Lập Hộp Cát | rust-worker | `surf:tests-plugin` | `crates/aqvn-plugin/tests/sandbox_isolation.rs` | test | `cargo test -p aqvn-plugin` | `C248-004` | P2 | 2 | test-suite | planned |

---

## 3. Ma Trận Quản Lý Rủi Ro Kỹ Thuật (Technical Risk Management Matrix)

| Mã Rủi Ro | Bản Chất Rủi Ro | Phân Hệ Ảnh Hưởng | Xác Suất | Mức Độ | Biện Pháp Kiểm Soát & Giải Pháp Chế Ngự |
|:---:|---|---|:---:|:---:|---|
| **TR-01** | **Linux Kernel Incompatibility** | `crates/aqvn-ebpf` | Trung bình | Cao | Tự động phát hiện khả năng hỗ trợ BPF/Landlock tại thời điểm boot; nếu thiếu, tự động fallback an toàn về Path Jail và AST Guard. |
| **TR-02** | **Desynchronization mTLS Certs** | `crates/aqvn-rfc` | Thấp | Cao | Cơ chế gia hạn chứng chỉ tự động trước hạn 4 giờ (Renewal Threshold); lưu trữ chứng chỉ dự phòng trong OS Keyring. |
| **TR-03** | **WASM Memory Overhead** | `crates/aqvn-plugin` | Trung bình | Trung bình | Khóa cứng kích thước bộ nhớ tuyến tính tối đa 32 MB cho mỗi plugin; tự động hủy instance khi không có tác vụ trong 60s. |
| **TR-04** | **CRDT State Divergence** | `crates/aqvn-crdt` | Thấp | Rất cao | Áp dụng thuật toán Loro CRDT giải quyết xung đột tất định; cơ chế so sánh vector clock và tự động tạo snapshot dự phòng định kỳ. |
| **TR-05** | **WebGL Context Crash** | `ui/components/` | Thấp | Thấp | Bắt sự kiện `onContextLoss` và tự động fallback sang Canvas 2D / DOM Renderer không làm đứt kết nối PTY shell. |
| **TR-06** | **Linker Failure Trên Nền Tảng Cũ** | Build Pipeline | Thấp | Trung bình | Script build tự động kiểm tra sự hiện diện của `mold`; nếu không có, tự động sử dụng trình liên kết hệ thống mặc định. |

---

## 4. Chiến Lược Kiểm Thử & Nghiệm Thu Tổng Thể (Master Validation & Testing Strategy)

Để bảo đảm toàn bộ hệ thống luôn giữ vững chất lượng và độ ổn định cao nhất, mọi đề xuất OpenSpec triển khai trong giai đoạn tới bắt buộc phải vượt qua **Hệ Thống Gác Cổng Kiểm Thử 5 Tầng (5-Tier Verification Gate)**:

```
+---------------------------------------------------------------------------------------------------+
| TẦNG 1: STATIC ANALYSIS & FORMATTING GATE                                                         |
|  - cargo fmt --check                                                                              |
|  - cargo clippy --workspace --all-targets -- -D warnings                                          |
|  - npm run lint:html                                                                              |
|  - Yêu cầu: 0 errors, 0 warnings.                                                                 |
+---------------------------------------------------------------------------------------------------+
                                   |
                                   v
+---------------------------------------------------------------------------------------------------+
| TẦNG 2: UNIT & COMPONENT TEST SUITE GATE                                                          |
|  - node --test tools/tests/test_chat_handlers.mjs (203+ tests pass 100%)                          |
|  - cargo test --workspace (238+ unit tests pass 100%)                                             |
|  - cargo test --manifest-path src-tauri/Cargo.toml --bin aqvn-agent -- --test-threads=1             |
|  - Yêu cầu: 100% test cases GREEN, không có flaky test do TEST_ENV_MUTEX.                         |
+---------------------------------------------------------------------------------------------------+
                                   |
                                   v
+---------------------------------------------------------------------------------------------------+
| TẦNG 3: SECURITY & BOUNDARY VERIFICATION GATE                                                     |
|  - Kiểm thử hộp cát Landlock và Seccomp-BPF (chặn 100% syscall vi phạm).                          |
|  - Kiểm thử mTLS 1.3 handshake và xác minh zeroize RAM sau khi giải phóng key.                    |
|  - Kiểm thử Wasmtime Fuel Metering (ngắt vòng lặp vô hạn < 500ms).                                |
+---------------------------------------------------------------------------------------------------+
                                   |
                                   v
+---------------------------------------------------------------------------------------------------+
| TẦNG 4: PERFORMANCE & SIZE GATE (BENCHMARK GATE)                                                  |
|  - cargo test --manifest-path src-tauri/Cargo.toml --test release_packaging                        |
|  - Ngưỡng nhị phân: Linux aqvn-agent < 2.00 MB, Linux Desktop < 9.80 MB.                          |
|  - Ngưỡng giao diện: FCP < 30ms, RAM chat 10k tin < 110 MB, CodeMirror large file delay < 16ms.  |
+---------------------------------------------------------------------------------------------------+
                                   |
                                   v
+---------------------------------------------------------------------------------------------------+
| TẦNG 5: CROSS-PLATFORM INTEGRATION & DOCKER GATE                                                  |
|  - Docker build kiểm tra: docker build -f docker/Dockerfile.agent -t aqvn-agent:v2 .              |
|  - Kích thước Docker image < 90 MB.                                                               |
+---------------------------------------------------------------------------------------------------+
```

---

## 5. Hướng Dẫn Chuyển Đổi & Bảo Đảm Tính Tương Thích Ngược (Migration & Backwards Compatibility)

1. **Tính tương thích ngược tuyệt đối với Client v1.x:**
   - Các máy chủ Agent nâng cấp lên v2 vẫn chấp nhận cấu hình cổng RFC và SSH mặc định (55080 và 55022).
   - Cơ chế đàm phán phiên (Protocol Negotiation): Client gửi cờ hỗ trợ mTLS và CRDT trong gói tin chào đầu tiên (`ClientHello`). Nếu Agent đối tác là phiên bản v1, kênh truyền tự động chuyển đổi sang giao thức HMAC-SHA256 chuẩn của Round 2 mà không làm đứt kết nối.
2. **Di chuyển dữ liệu cấu hình cũ (Legacy Config Migration):**
   - Khi khởi động lần đầu, hệ thống tự động quét tệp `settings.json` hiện hành.
   - Các bí mật plaintext được tự động mã hóa bằng ChaCha20-Poly1305 và chuyển vào OS Keyring; tệp gốc được sao lưu thành `settings.json.bak` với quyền bảo mật nghiêm ngặt (`0600`).
3. **Độ an toàn của Workspace File Explorer:**
   - Cấu trúc thư mục mã nguồn của người dùng hoàn toàn không bị can thiệp hay sửa đổi ngoài ý muốn; dữ liệu metadata CRDT được cô lập độc lập bên trong thư mục ẩn `.aqvn/workspace.crdt`.

---

## 6. Kết Luận & Bàn Giao

Tài liệu **Master Roadmap v2.0 & Actionable OpenSpec Draft Specifications (`VAL-V2-PROD-OPEN-002`)** đã cung cấp một lộ trình kỹ thuật hoàn chỉnh, chuẩn xác và khả thi cao cho tương lai của `ai-launcher`. Toàn bộ 6 đề xuất OpenSpec từ `change243` đến `change248` đã được đặc tả chi tiết về mặt kiến trúc, cấu trúc tệp, ma trận nhiệm vụ thực thi (`tasks.md`), chiến lược kiểm thử và ràng buộc an ninh.

Hệ thống đã sẵn sàng 100% để bước vào chu trình thực thi các giai đoạn tiếp theo thông qua công cụ điều phối `aqvn-opsx-doit`.

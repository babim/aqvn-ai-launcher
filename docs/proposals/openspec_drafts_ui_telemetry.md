# OpenSpec Change Proposals: UI/UX & Telemetry Subsystems

**Mã tài liệu:** `VAL-ROAD-OPEN-002`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Tác giả:** AQVN Architecture & Synthesis Team (`synthesis-worker`)  
**Ngày công bố:** 2026-11-09  
**Trạng thái đề xuất:** Actionable OpenSpec Draft Specifications (Sẵn sàng thực thi)  
**Tài liệu tham chiếu:**  
- `docs/eval-reports/05_ui_shell_components.md` (Frontend Shell & Architecture Audit)  
- `docs/eval-reports/06_interactive_drawers_ux.md` (Interactive Terminal, Workspace & Feed Audit)  
- `docs/eval-reports/07_settings_themes_i18n.md` (Settings Store, OKLCH Themes & i18n Audit)  
- `docs/eval-reports/08_a11y_shortcuts_palette.md` (Accessibility, Shortcuts & Command Palette Audit)  
- `docs/eval-reports/11_performance_benchmarks.md` (FCP, Bundle Size & Backpressure Benchmarks)  
- `docs/eval-reports/12_telemetry_outbox_durability.md` (Telemetry Stream, Radar & Outbox Audit)  
- `docs/proposals/01_frontend_modernization_blueprint.md` (Next-Gen Frontend Modernization Blueprint)  
- `docs/proposals/AQVN_LAUNCHER_UPGRADE_MASTER_PLAN.md` (Master Upgrade Plan)

---

## Mục lục Đề Xuất (UI/UX & Telemetry Proposals Index)

| Mã Đề Xuất OpenSpec | Phân Hệ | Tiêu Đề Đề Xuất | Độ Ưu Tiên | Độ Phức Tạp |
| :--- | :--- | :--- | :---: | :---: |
| **`change229-ui-web-components-shell-modernization`** | UI Architecture | Hiện đại hóa Shell sang Web Components & Reactive Signals Store | P1 (Khẩn) | Cao (8) |
| **`change230-ui-xterm-webgl-terminal-upgrade`** | UI/Terminal | Nâng cấp lõi Terminal sang xterm.js WebGL & Tự động co giãn Fit | P1 (Khẩn) | Trung bình (6) |
| **`change231-ui-codemirror-workspace-editor`** | UI/Workspace | Tích hợp Trình soạn thảo mã CodeMirror 6 & Visual Diff Viewer | P2 (Cao) | Trung bình (5) |
| **`change232-telemetry-radar-resilient-outbox`** | Telemetry/Reliability | Radar tài nguyên thời gian thực & Hộp thư chờ ngoại tuyến IndexedDB | P2 (Cao) | Trung bình (5) |

---

# ĐỀ XUẤT 1: `change229-ui-web-components-shell-modernization`

## 1.1. `proposal.md`

### Why
Kiểm toán kiến trúc giao diện M2 (`VAL-UI-SHELL-001`) và phân tích đồ thị CBM (`VAL-EVAL-GRAPH-001`) đã chỉ ra các điểm nghẽn nghiêm trọng trong frontend hiện tại (`ui/`):
1. **Khủng hoảng tệp khổng lồ:** `ui/chat.js` đã phình to lên tới **4.243 dòng**, chứa hàm `initLayout` với **49 lời gọi hàm ra ngoài**, quản lý hàng chục trạng thái phân tán trong đối tượng mutable `let state = { ... }`.
2. **Ô nhiễm phạm vi toàn cục (Global Scope Pollution):** Hơn 50 hàm và bộ lắng nghe sự kiện (event listeners) được gắn trực tiếp vào đối tượng `window` hoặc thẻ `document`, gây nguy cơ xung đột tên biến và khó khăn khi viết kiểm thử cô lập.
3. **Thiếu tính năng ảo hóa danh sách (DOM Virtualization):** Khi cuộc hội thoại với AI kéo dài qua 1.000 tin nhắn, toàn bộ các phần tử DOM vẫn được giữ nguyên trong cây DOM, gây lãng phí bộ nhớ RAM (lên tới hơn 300MB Webview RSS) và gây giật lag khi cuộn trang.
4. **CSS không có Scoping:** `ui/chat.css` (2.480 dòng) định nghĩa các selector cấp cao, dễ dẫn đến hiện tượng rò rỉ kiểu dáng (CSS Bleed) khi thêm các ngăn kéo hoặc component mới.

### What Changes
- **229-001**: Tái cấu trúc giao diện thành các thành phần **Web Components chuẩn W3C (Custom Elements v1)** với Shadow DOM để cô lập hoàn toàn HTML và CSS:
  - `<aqvn-shell>`: Vỏ bọc ứng dụng chính, quản lý bố cục 3 cột và splitters co giãn.
  - `<aqvn-topbar>`: Thanh tiêu đề, trạng thái kết nối, nút Focus Mode, và chuyển theme.
  - `<aqvn-sidebar>`: Danh sách phiên làm việc, lịch sử hội thoại, bộ lọc tìm kiếm.
  - `<aqvn-chat-pane>`: Khung hội thoại AI, thanh nhập lệnh, danh sách tin nhắn ảo hóa.
  - `<aqvn-canvas-pane>`: Vùng nhúng ứng dụng ngoại vi hoặc iframe Intel Suite.
- **229-002**: Triển khai tầng quản lý trạng thái phản ứng trung tâm (Centralized Reactive Signals Store) sử dụng thư viện siêu nhẹ `@preact/signals-core` (chỉ nặng **1.6 KB**):
  - Thay thế toàn bộ đối tượng mutable `state = { ... }` bằng các Signals nguyên tử: `connectionState`, `activeSessionId`, `activeTheme`, `telemetryMetrics`.
  - Tự động kích hoạt cập nhật DOM có chủ đích khi tín hiệu thay đổi mà không cần nạp VDOM cồng kềnh.
- **229-003**: Cài đặt thành phần danh sách tin nhắn ảo hóa (`<aqvn-virtual-message-list>`):
  - Chỉ duy trì hiển thị trong DOM các phần tử tin nhắn nằm trong khung nhìn (Viewport) cộng thêm đệm trên/dưới 5 tin nhắn (Windowing Buffer).
  - Giảm số lượng DOM nodes từ hàng nghìn xuống dưới 30 nodes, giải phóng 60% mức tiêu thụ RAM khi hội thoại dài.
- **229-004**: Đóng gói hệ thống màu sắc OKLCH vào các biến CSS Custom Properties kế thừa xuyên qua Shadow DOM (`:host`, `var(--color-...)`).
- **229-005**: Bảo đảm duy trì chỉ số First Contentful Paint (FCP) $\le 35\text{ ms}$ thông qua việc không sử dụng các bundler nặng nề tại thời điểm khởi động (Zero-Hydration Start).

### Goals
- Xóa bỏ hoàn toàn God File `ui/chat.js` bằng cách chia nhỏ thành các tệp component độc lập dưới 300 dòng mã mỗi tệp.
- Giữ vững FCP dưới 35ms và vượt qua 100% bộ 177 bài kiểm thử giao diện hiện có trong `tools/tests/test_chat_handlers.mjs`.

### Non-Goals
- Không sử dụng các framework nặng (như React full, Angular hoặc Next.js) làm phình kích thước bundle hoặc vi phạm triết lý siêu nhẹ.

### Validation Contract Assertions Fulfilled
- `VAL-UI-SHELL-001`, `VAL-UI-THEME-001`, `VAL-ROAD-BLUEPRINT-001`

---

## 1.2. `tasks.md`

### Execution Tasks
- [ ] C229-001: Khởi tạo cấu trúc thư mục mô-đun frontend `ui/components/`, `ui/state/`, `ui/styles/`
- [ ] C229-002: Cài đặt bus trạng thái phản ứng `ui/state/signals.js` với `@preact/signals-core`
- [ ] C229-003: Xây dựng Custom Element `<aqvn-topbar>` thay thế thanh header tĩnh
- [ ] C229-004: Xây dựng Custom Element `<aqvn-sidebar>` quản lý phiên hội thoại
- [ ] C229-005: Xây dựng Custom Element `<aqvn-virtual-message-list>` ảo hóa tin nhắn
- [ ] C229-006: Xây dựng Custom Element `<aqvn-shell>` điều phối splitter kéo thả và bố cục 3 cột
- [ ] C229-007: Di chuyển và chuẩn hóa bộ Tokens OKLCH vào `ui/styles/tokens.css`
- [ ] C229-008: Cập nhật bộ test `tools/tests/test_chat_handlers.mjs` bảo đảm 177 bài test tương thích hoàn toàn
- [ ] C229-009: Chạy benchmark tự động kiểm chứng FCP $\le 35\text{ ms}$ và HTML linter hợp lệ

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C229-001 | Frontend Directory Scaffolding | UI | `ui/components/`, `ui/state/` | config | File tree existence check | — | P1 | 1 | ready |
| C229-002 | Reactive Signals Store | UI/State | `ui/state/signals.js` | code | Unit test state subscription & mutations | C229-001 | P1 | 2 | ready |
| C229-003 | Web Component Topbar | UI/Component | `ui/components/topbar.js` | code | Component DOM render test | C229-002 | P1 | 3 | ready |
| C229-004 | Web Component Sidebar | UI/Component | `ui/components/sidebar.js` | code | Session switch event dispatch test | C229-002 | P1 | 3 | ready |
| C229-005 | Virtual Message List | UI/Component | `ui/components/virtual-list.js` | code | 2.000 messages DOM count test (<35 nodes) | C229-002 | P1 | 4 | ready |
| C229-006 | Root Container Shell | UI/Component | `ui/components/shell.js` | code | 3-column splitter resize test | C229-003, C229-004, C229-005 | P1 | 4 | ready |
| C229-007 | Scoped OKLCH Tokens | UI/Styles | `ui/styles/tokens.css` | styles | Theme color switch test (no FOUC) | C229-001 | P1 | 2 | ready |
| C229-008 | Handler Regression Suite | QA/UI | `tools/tests/test_chat_handlers.mjs` | test | `node --test` 177 tests passed | C229-006 | P1 | 3 | ready |
| C229-009 | FCP & A11y Gate Benchmark | QA/Perf | `tools/tests/fcp_benchmark.mjs` | test | FCP < 35ms & `npm run lint:html` valid | C229-008 | P1 | 2 | ready |

### Dependency Graph & Gate
- `C229-001` -> `C229-002`, `C229-007` -> `C229-003`, `C229-004`, `C229-005` -> `C229-006` -> `C229-008` -> `C229-009`
- **Granularity Gate:** Pass. Độ phức tạp tối đa là 4. Ranh giới giữa các component rõ ràng.

---

## 1.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-UI-01: Tốc độ khởi động FCP:** Giao diện người dùng phải hiển thị nội dung có nghĩa đầu tiên (First Contentful Paint) trong thời gian dưới 35ms trên máy thử nghiệm tiêu chuẩn.
2. **R-UI-02: Giới hạn số lượng DOM nodes:** Trong mọi phiên hội thoại (kể cả khi có trên 10.000 tin nhắn trong lịch sử), thành phần `<aqvn-virtual-message-list>` không được chứa quá 50 phần tử `<div>` tin nhắn trong cây DOM cùng một lúc.
3. **R-UI-03: Tính cô lập CSS:** Việc thay đổi CSS bên trong một Web Component không được làm ảnh hưởng đến hiển thị của các thành phần khác bên ngoài Shadow Root.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: Modern Web Components Shell and Virtual Message List
  Scenario: Virtual Scrolling Keeps DOM Lean Under Heavy Chat History
    Given a chat session loaded with 2,500 historical messages
    When the user inspects the DOM of "<aqvn-virtual-message-list>"
    Then the number of rendered message elements in the DOM is between 15 and 35
    And scrolling up and down maintains a smooth 60 frames per second
    And memory usage remains below 60 MB

  Scenario: Switching OKLCH Theme Synchronizes Across All Shadow Roots
    Given the application running in "aqvn-dark" theme
    When the user changes theme signal to "cyberpunk-terminal"
    Then all Web Components inherit the new OKLCH CSS Custom Properties immediately
    And there is zero Flash of Unstyled Content (FOUC)
```

---
---

# ĐỀ XUẤT 2: `change230-ui-xterm-webgl-terminal-upgrade`

## 2.1. `proposal.md`

### Why
Kiểm toán các ngăn kéo tương tác M2 (`VAL-UI-SHELL-002`) và đánh giá hiệu năng M3 (`VAL-PERF-FCP-001`) đã chỉ ra các hạn chế cố hữu của trình giả lập Terminal hiện tại:
1. Trình giả lập ANSI dựa trên regex (`parseAnsiToHtml` trong `ui/chat.js` L1455-1510) chỉ chuyển đổi được các mã màu SGR cơ bản sang thẻ `<span style="color: ...">`.
2. Không hỗ trợ các chuỗi điều khiển con trỏ (Cursor Movement ESC sequences), vùng đệm màn hình thay thế (Alternate Screen Buffer), hoặc báo cáo chuột (Mouse Reporting). Do đó, các ứng dụng TUI tương tác phổ biến của lập trình viên như `vim`, `nano`, `htop`, `mc`, hoặc `tig` bị vỡ giao diện hoàn toàn khi chạy trong Terminal Drawer.
3. Render trực tiếp hàng nghìn thẻ `<pre>` và `<span>` vào DOM khiến Webview bị nghẽn (render bottleneck) khi nhận luồng stdout lớn từ các lệnh như `cargo build` hoặc `npm install`.

### What Changes
- **230-001**: Tích hợp thư viện chuẩn công nghiệp **`xterm.js` 5.x** kết hợp tiện ích tăng tốc phần cứng **`@xterm/addon-webgl`** và bộ dự phòng Canvas Fallback tự động khi chạy trên GPU ảo hóa.
- **230-002**: Tích hợp tiện ích tự động khớp kích thước **`@xterm/addon-fit`**: Lắng nghe sự kiện co giãn của Terminal Drawer để tự động tính toán số dòng/cột (`rows`, `cols`) và gửi lệnh IPC `resize_pty` về backend Rust (`crates/aqvn-ssh` hoặc `connector/handlers/exec.rs`).
- **230-003**: Áp dụng chiến lược **Nạp động theo nhu cầu (On-Demand Dynamic Import)**: Toàn bộ module `xterm.js` và WebGL addon chỉ được tải về và khởi tạo khi người dùng lần đầu tiên bấm mở Terminal Drawer (`Ctrl+\`` hoặc click biểu tượng Terminal). Nhờ vậy, kích thước bundle ban đầu hoàn toàn không bị ảnh hưởng, bảo toàn tuyệt đối chỉ số FCP < 35ms.
- **230-004**: Đồng bộ bảng màu Terminal với 4 theme OKLCH của hệ thống (`aqvn-dark`, `oled-black`, `cyberpunk-terminal`, `clean-studio-light`), hỗ trợ hiển thị 24-bit Truecolor chân thực.
- **230-005**: Kết nối luồng dữ liệu PTY trực tiếp với bộ đệm Bounded Channel Backpressure ở backend, triệt tiêu nguy cơ tràn bộ nhớ khi ứng dụng nhận luồng dữ liệu stdout tốc độ cao.

### Goals
- Hỗ trợ 100% các ứng dụng toàn màn hình (`vim`, `htop`, `less`) hoạt động mượt mà trong Terminal Drawer.
- Đạt tốc độ render ổn định 60 FPS khi xử lý luồng văn bản 10.000 dòng/giây nhờ bộ tăng tốc WebGL.

### Non-Goals
- Không thay thế terminal mặc định của hệ điều hành máy chủ bên ngoài ứng dụng.

### Validation Contract Assertions Fulfilled
- `VAL-UI-SHELL-002`, `VAL-PERF-FCP-001`, `VAL-ROAD-BLUEPRINT-001`

---

## 2.2. `tasks.md`

### Execution Tasks
- [ ] C230-001: Đóng gói các tệp ES Modules của `xterm.js` 5.x, WebGL Addon và Fit Addon vào `ui/vendor/xterm/`
- [ ] C230-002: Xây dựng Web Component `<aqvn-terminal-drawer>` tại `ui/components/terminal-drawer.js`
- [ ] C230-003: Cài đặt cơ chế Dynamic Lazy-Import tải module xterm khi mở drawer lần đầu
- [ ] C230-004: Cài đặt bộ ánh xạ bảng màu theme OKLCH sang `ITheme` của xterm
- [ ] C230-005: Kết nối sự kiện Resize Observer với Tauri IPC command `resize_pty(rows, cols)`
- [ ] C230-006: Kết nối luồng PTY binary / text stream từ backend Rust vào `terminal.write()`
- [ ] C230-007: Viết kiểm thử tự động kiểm tra nạp lazy, co giãn kích thước và chạy lệnh TUI mô phỏng

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C230-001 | Vendor xterm.js ES Assets | UI/Vendor | `ui/vendor/xterm/` | asset | Asset integrity and size check | — | P1 | 1 | ready |
| C230-002 | Terminal Drawer Component | UI/Component | `ui/components/terminal-drawer.js` | code | Component instantiation test | C230-001 | P1 | 3 | ready |
| C230-003 | On-Demand Lazy Loader | UI/Perf | `ui/components/terminal-drawer.js` | code | FCP measurement test (no xterm loaded at start) | C230-002 | P1 | 2 | ready |
| C230-004 | OKLCH Terminal Palette | UI/Theme | `ui/styles/terminal-themes.js` | code | Contrast ratio & theme switch test | C230-002 | P1 | 2 | ready |
| C230-005 | PTY Auto-Resize IPC Hook | UI/IPC | `ui/components/terminal-drawer.js` | code | Mock IPC resize call verification | C230-002 | P1 | 3 | ready |
| C230-006 | PTY Stream Ingestion | UI/Backend | `ui/components/terminal-drawer.js` | code | Stream write 10.000 lines benchmark | C230-005 | P1 | 3 | ready |
| C230-007 | Terminal E2E Test Suite | QA | `tools/tests/test_terminal_drawer.mjs` | test | 100% tests pass on resize & data stream | C230-006 | P1 | 3 | ready |

### Dependency Graph & Gate
- `C230-001` -> `C230-002` -> `C230-003`, `C230-004`, `C230-005` -> `C230-006` -> `C230-007`
- **Granularity Gate:** Pass. Tác vụ nạp tài nguyên và xử lý IPC được cô lập độc lập.

---

## 2.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-TRM-01: Tốc độ kết xuất WebGL:** Khi luồng dữ liệu stdout đạt 10.000 dòng/giây, bộ hiển thị xterm.js WebGL phải duy trì tốc độ khung hình từ 55 - 60 FPS, không gây đơ giao diện chính.
2. **R-TRM-02: Zero-Impact FCP:** Việc tích hợp thư viện `xterm.js` không được làm tăng chỉ số FCP của trang chính quá 1ms khi ứng dụng khởi động lần đầu (nhờ cơ chế lazy-load).
3. **R-TRM-03: Hỗ trợ ncurses:** Hỗ trợ đầy đủ các phím điều hướng, phím chức năng F1-F12, chuột tương tác và Alternate Screen Buffer cho các ứng dụng như `vim` và `htop`.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: xterm.js WebGL Terminal Drawer Upgrade
  Scenario: Lazy-loading Terminal Does Not Impact Initial FCP
    Given the user launches the application
    When the initial window renders and FCP is recorded
    Then the FCP is strictly less than 35 ms
    And "window.Terminal" is undefined (xterm.js has not been fetched)

  Scenario: Opening Terminal Drawer Loads xterm and Handles TUI Application
    Given the application is running
    When the user clicks the Terminal Drawer toggle or presses Ctrl+`
    Then the xterm.js and WebGL addon scripts are loaded on demand
    And the terminal initializes and fits the container width and height
    And launching "htop" displays an interactive layout with functioning mouse clicks
```

---
---

# ĐỀ XUẤT 3: `change231-ui-codemirror-workspace-editor`

## 3.1. `proposal.md`

### Why
Kiểm toán ngăn kéo không gian làm việc Workspace Drawer (`VAL-UI-SHELL-002`) cho thấy tính năng xem trước và chỉnh sửa tập tin hiện tại (`ui/chat.js` L3200-3350) sử dụng thẻ `<textarea>` HTML thông thường kết hợp thư viện `highlight.js`.
Cách tiếp cận này bộc lộ những khiếm khuyết lớn đối với một công cụ hỗ trợ phát triển:
1. Thiếu các tính năng soạn thảo thiết yếu: Không có số dòng (line numbers), không thể gấp mã (code folding), không hiển thị lỗi cú pháp (syntax errors) hoặc tự động đóng ngoặc.
2. Khó khăn khi kiểm tra thay đổi: Khi lập trình viên hoặc AI Agent chỉnh sửa một tập tin trong dự án, người dùng không có công cụ trực quan để so sánh sự khác biệt (Visual Diff Viewer) trước khi lưu hoặc gửi lệnh commit.
3. Nguy cơ ghi đè dữ liệu: Thao tác lưu tập tin hiện tại chưa có cơ chế kiểm tra xung đột (dirty state detection) hoặc cảnh báo mất dữ liệu khi chuyển tệp.

### What Changes
- **231-001**: Tích hợp bộ soạn thảo mã nguồn **CodeMirror 6** (kiến trúc mô-đun siêu nhẹ, chỉ nạp những tính năng cần thiết) vào Workspace Drawer:
  - Hỗ trợ đánh số dòng, gấp mã, tô sáng dòng hiện tại, tìm kiếm và thay thế (`@codemirror/search`).
  - Hỗ trợ các gói ngôn ngữ thông dụng: Rust, JavaScript, TypeScript, Python, HTML/CSS, JSON, Markdown, YAML.
- **231-002**: Xây dựng trình xem khác biệt phân nhánh trực quan (**Visual Split Diff Viewer**) sử dụng `@codemirror/merge`:
  - Hiển thị song song hai khung (Original vs Modified) với đánh dấu xanh/đỏ cho các dòng được thêm, bớt hoặc sửa đổi.
  - Hỗ trợ nút "Khôi phục thay đổi" (Revert Chunk) và "Áp dụng thay đổi" (Accept Chunk) trực tiếp.
- **231-003**: Kết nối trực tiếp với API RFC Server `crates/aqvn-rfc` để thực hiện thao tác lưu tập tin nguyên tử (Atomic File Write):
  - Kiểm tra mã băm SHA-256 của tập tin gốc trước khi ghi đè để phát hiện xung đột sửa đổi đồng thời.
- **231-004**: Triển khai cơ chế nạp động (Lazy-load CodeMirror Bundle) chỉ khi người dùng mở xem trước hoặc chỉnh sửa một tập tin.

### Goals
- Cung cấp trải nghiệm biên tập mã chuyên nghiệp ngay trong Native Workspace Client.
- Hỗ trợ người dùng kiểm tra trực quan 100% các sửa đổi tệp mã nguồn trước khi áp dụng.

### Non-Goals
- Không xây dựng một IDE hoàn chỉnh thay thế VS Code (tập trung vào xem nhanh, kiểm tra diff và sửa lỗi ngắn).

### Validation Contract Assertions Fulfilled
- `VAL-UI-SHELL-002`, `VAL-ROAD-BLUEPRINT-001`

---

## 3.2. `tasks.md`

### Execution Tasks
- [ ] C231-001: Đóng gói bộ thư viện CodeMirror 6 cốt lõi vào `ui/vendor/codemirror/`
- [ ] C231-002: Xây dựng Web Component `<aqvn-code-editor>` tại `ui/components/code-editor.js`
- [ ] C231-003: Cài đặt tiện ích mở rộng Visual Diff Viewer với `@codemirror/merge`
- [ ] C231-004: Xây dựng cơ chế phát hiện sửa đổi chưa lưu (Dirty State Tracking) và Modal cảnh báo
- [ ] C231-005: Kết nối hành động lưu với API RFC `write_file_atomic` kèm kiểm tra hash
- [ ] C231-006: Viết kiểm thử tự động cho các thao tác tải nội dung, hiển thị diff và lưu tệp an toàn

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C231-001 | Vendor CodeMirror 6 Modules | UI/Vendor | `ui/vendor/codemirror/` | asset | Asset integrity and size check | — | P2 | 1 | ready |
| C231-002 | Code Editor Component | UI/Component | `ui/components/code-editor.js` | code | Editor instantiation & syntax highlighting test | C231-001 | P2 | 3 | ready |
| C231-003 | Split Diff Merge Extension | UI/Component | `ui/components/diff-viewer.js` | code | Diff chunk calculation and rendering test | C231-002 | P2 | 3 | ready |
| C231-004 | Dirty State Modal Gate | UI/UX | `ui/components/dirty-modal.js` | code | Unsaved changes navigation block test | C231-002 | P2 | 2 | ready |
| C231-005 | RFC Atomic File Save IPC | UI/RFC | `ui/components/code-editor.js` | code | Mock RFC atomic write verification | C231-002 | P2 | 3 | ready |
| C231-006 | Editor Test Suite | QA | `tools/tests/test_code_editor.mjs` | test | 100% tests pass on load, edit, diff, and save | C231-005 | P2 | 2 | ready |

### Dependency Graph & Gate
- `C231-001` -> `C231-002` -> `C231-003`, `C231-004`, `C231-005` -> `C231-006`
- **Granularity Gate:** Pass. Tác vụ biên tập và so sánh diff được phân rã độc lập.

---

## 3.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-EDT-01: Tốc độ mở tệp:** Thời gian từ lúc người dùng nhấn vào tệp trong Workspace Explorer đến khi CodeMirror render hoàn chỉnh nội dung (cho tệp mã nguồn < 5.000 dòng) không được vượt quá 100ms.
2. **R-EDT-02: An toàn lưu trữ nguyên tử:** Nếu nội dung tệp trên đĩa bị thay đổi bởi một tiến trình khác trong khi đang mở trong Editor, hành động lưu phải tạm dừng và hiển thị hộp thoại cảnh báo xung đột (Conflict Resolution Dialog).
3. **R-EDT-03: Trực quan hóa Diff:** Trình xem diff phải phân biệt rõ ràng màu sắc dòng thêm mới (xanh lá cây) và dòng xóa bỏ (đỏ cam) theo đúng tỷ lệ tương phản WCAG 2.2 AA.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: CodeMirror 6 Visual Workspace & Diff Editor
  Scenario: Viewing Git Diff of Modified File
    Given a modified file in the workspace
    When the user clicks "View Diff" in the file preview
    Then the split diff editor displays side-by-side comparison
    And modified lines are highlighted with high-contrast background tokens
    And clicking "Revert Chunk" discards the selected modification cleanly

  Scenario: Prevent Accidental Data Loss on Unsaved Changes
    Given an open file with unsaved edits in the editor
    When the user attempts to close the drawer or select another file
    Then a confirmation modal appears: "You have unsaved changes. Discard or Save?"
    And navigation is paused until the user confirms
```

---
---

# ĐỀ XUẤT 4: `change232-telemetry-radar-resilient-outbox`

## 4.1. `proposal.md`

### Why
Kiểm toán hệ thống viễn thông M3 (`VAL-PERF-TELEM-001`) và kiểm toán hiệu năng (`VAL-PERF-FCP-001`) đã chỉ ra các điểm cần hoàn thiện:
1. **Giao diện hiển thị viễn thông còn đơn giản:** Biểu đồ sparkline độ trễ ping và mức chiếm dụng CPU/RAM hiện tại chỉ là các chuỗi SVG polyline nhỏ trên topbar, chưa hỗ trợ theo dõi chuyên sâu khi điều phối nhiều máy chủ (Multi-Host Monitoring).
2. **Hộp thư chờ ngoại tuyến (`localStorage` Outbox) có giới hạn dung lượng nghiêm ngặt:** `localStorage` chỉ hỗ trợ tối đa 5MB dữ liệu (trên thực tế thiết kế đặt ngưỡng an toàn 256KB/50 tin nhắn), không mã hóa dữ liệu nghỉ (unencrypted at rest), và có nguy cơ làm nghẽn main thread do API `localStorage` là đồng bộ (synchronous).
3. **Thiếu công cụ trực quan hóa Outbox:** Người dùng không thể xem danh sách các tin nhắn đang chờ gửi khi mất mạng, không thể hủy bỏ các tin nhắn gửi lỗi hoặc kích hoạt gửi lại thủ công.

### What Changes
- **232-001**: Xây dựng thành phần Radar viễn thông trực quan (**`<aqvn-telemetry-radar>`**):
  - Hiển thị đồng hồ đo tải CPU, RAM RSS, GPU VRAM, Tốc độ I/O đĩa và biểu đồ biến thiên độ trễ ping (Latency Jitter Sparkline) với bộ đệm lịch sử 60 giây.
  - Hỗ trợ chuyển đổi nhanh giữa các máy chủ trong Multi-Host Pool để so sánh hiệu năng tức thời.
- **232-002**: Nâng cấp bộ lưu trữ Hộp thư chờ ngoại tuyến từ `localStorage` sang **IndexedDB** bất đồng bộ (Asynchronous Storage Engine):
  - Hỗ trợ lưu trữ dung lượng lớn (lên tới 50 MB, chứa hàng nghìn tin nhắn và tệp đính kèm ngoại tuyến).
  - Áp dụng mã hóa dữ liệu nghỉ (Encryption at Rest) bằng thuật toán chuẩn AES-GCM-256 sử dụng khóa mã hóa phái sinh từ phiên làm việc.
- **232-003**: Xây dựng Ngăn kéo trực quan hóa Hộp thư chờ (**Resilient Outbox Drawer**):
  - Hiển thị danh sách các gói tin đang chờ gửi kèm số lần thử lại (retry attempts), thời gian chờ tiếp theo (exponential backoff countdown).
  - Cung cấp nút thao tác: "Gửi lại ngay" (Flush Now) và "Xóa bỏ" (Discard).
- **232-004**: Tối ưu hóa giao thức truyền tin viễn thông bằng định dạng nhị phân **MessagePack**:
  - Giảm kích thước gói tin gửi định kỳ từ 1.2 KB (JSON) xuống dưới 450 bytes (nhị phân), giảm tải CPU phân tích cú pháp.

### Goals
- Đảm bảo không mất mát bất kỳ tin nhắn nào khi người dùng nhập liệu trong điều kiện mạng bị ngắt quãng.
- Cung cấp góc nhìn toàn cảnh về sức khỏe tài nguyên của toàn bộ các máy chủ trong hệ sinh thái.

### Non-Goals
- Không xây dựng hệ thống Time-Series Database dài hạn (như Prometheus/InfluxDB) bên trong client (chỉ lưu trữ bộ đệm ngắn hạn 60s - 1h phục vụ hiển thị trực tiếp).

### Validation Contract Assertions Fulfilled
- `VAL-PERF-TELEM-001`, `VAL-ROAD-BLUEPRINT-001`

---

## 4.2. `tasks.md`

### Execution Tasks
- [ ] C232-001: Xây dựng Web Component `<aqvn-telemetry-radar>` tại `ui/components/telemetry-radar.js`
- [ ] C232-002: Cài đặt module lưu trữ IndexedDB mã hóa `ui/state/resilient-outbox.js`
- [ ] C232-003: Xây dựng Ngăn kéo giao diện Outbox Viewer hiển thị hàng đợi và trạng thái thử lại
- [ ] C232-004: Tích hợp bộ giải mã nhị phân MessagePack cho luồng telemetry WebSocket
- [ ] C232-005: Kết nối cơ chế tự động gửi lại khi máy trạm chuyển trạng thái sang `online`
- [ ] C232-006: Viết kiểm thử mô phỏng ngắt kết nối mạng, hàng đợi outbox và tự phục hồi khi kết nối lại

### Task Inventory
| ID | Tiêu đề nhiệm vụ | Phân hệ | Phạm vi tệp | Loại tác vụ | Chiến lược kiểm thử | Phụ thuộc | Ưu tiên | Độ phức tạp | Trạng thái |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :---: | :---: | :---: |
| C232-001 | Telemetry Radar Component | UI/Telemetry | `ui/components/telemetry-radar.js` | code | Component sparkline render test | — | P2 | 3 | ready |
| C232-002 | IndexedDB Encrypted Outbox | UI/Storage | `ui/state/resilient-outbox.js` | code | IndexedDB write/read & AES-GCM test | — | P2 | 4 | ready |
| C232-003 | Outbox Viewer Drawer | UI/UX | `ui/components/outbox-drawer.js` | code | Drawer open & message list test | C232-002 | P2 | 2 | ready |
| C232-004 | MessagePack Protocol Adapter | UI/Backend | `ui/vendor/msgpack.js`, `crates/aqvn-core/src/telemetry.rs` | code | Serialization & payload size benchmark | C232-001 | P2 | 3 | ready |
| C232-005 | Auto-Flush Connection Listener | UI/State | `ui/state/connection-manager.js` | code | State change offline -> online flush test | C232-002 | P2 | 2 | ready |
| C232-006 | Network Disconnect Test Suite | QA | `tools/tests/test_resilient_outbox.mjs` | test | 100% tests pass on network drop & flush | C232-005 | P2 | 3 | ready |

### Dependency Graph & Gate
- `C232-001`, `C232-002` -> `C232-003`, `C232-004`, `C232-005` -> `C232-006`
- **Granularity Gate:** Pass. Phân rã độc lập giữa radar hiển thị và cơ chế lưu trữ IndexedDB.

---

## 4.3. `specs/` (Requirements & Scenarios)

### Yêu cầu Kỹ Thuật (Requirements):
1. **R-TLM-01: Zero Data Loss:** Mọi tin nhắn hoặc lệnh gửi đi khi máy trạm mất mạng phải được ghi nhận an toàn vào IndexedDB trước khi trả trạng thái phản hồi cho giao diện.
2. **R-TLM-02: Tự động phục hồi:** Khi máy trạm khôi phục kết nối mạng (trạng thái WebSocket chuyển sang `online`), các tin nhắn trong Outbox phải được tuần tự hóa và gửi đi tự động mà không yêu cầu người dùng gõ lại.
3. **R-TLM-03: Hiệu năng Radar:** Tần suất vẽ lại biểu đồ sparkline (1 lần/giây) phải chiếm dụng dưới 1% CPU của tiến trình Webview.

### Kịch Bản Kiểm Định (BDD Scenarios):
```gherkin
Feature: Telemetry Radar and Resilient IndexedDB Outbox
  Scenario: Offline Queuing and Automatic Reconnect Flush
    Given the client is disconnected from the Gateway (state is "offline")
    When the user submits a prompt message in the chat pane
    Then the message is saved immediately to IndexedDB Encrypted Outbox
    And the UI displays an "Enqueued Offline" badge on the message
    When the network reconnects and state transitions to "online"
    Then the Outbox manager flushes the queued message automatically
    And the message status updates to "Sent"

  Scenario: Inspecting Outbox Queue Manually
    Given 3 messages pending in the offline outbox
    When the user opens the Outbox Drawer
    Then all 3 pending items are displayed with creation timestamp and payload size
    And clicking "Discard" on item 2 removes it from IndexedDB
    And the remaining queue length becomes 2
```

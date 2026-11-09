# Báo Cáo Phân Tích Chuyên Sâu Các Hệ Thống Con Giao Diện Tương Tác: Terminal Drawer, File Browser & Activity Feed UX

**Mã tài liệu:** `VAL-UI-SHELL-002`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi nghiên cứu:** `ui/index.html` (L262-658), `ui/chat.js` (L1430-1796, L2780-3350), `ui/chat.css` (~L800-1500)  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Research Team (`research-worker`)  
**Trạng thái kiểm tra kỹ thuật:** Passed (`node --test tools/tests/test_chat_handlers.mjs` 177/177 passed)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Bên cạnh luồng hội thoại AI cốt lõi, `ai-launcher` tích hợp một hệ sinh thái các hệ thống con tương tác trực tiếp (Interactive Subsystems) phục vụ các tác vụ kỹ thuật nâng cao của lập trình viên và kỹ sư hệ thống:
1. **Interactive PTY Terminal Engine 2.0:** Ngăn kéo terminal mô phỏng VT100 hai chiều hỗ trợ multi-tab, streaming thời gian thực qua WebSocket, bộ đệm giới hạn 2.000 dòng chống rò rỉ bộ nhớ, và điều phối kích thước PTY theo kích thước cửa sổ.
2. **Workspace File Explorer & Editor 2.0:** Trình duyệt tệp tin phân cấp trong thanh bên với điều hướng breadcrumbs, tạo/xóa tệp và thư mục, tải xuống toàn bộ không gian làm việc dưới dạng tệp nén ZIP, và bộ xem trước Markdown / soạn thảo văn bản nội tuyến.
3. **Kéo thả tải tệp an toàn (Safe Binary Drag-and-Drop):** Cơ chế nhận diện tệp nhị phân tự động đa tiêu chí (MIME, phần mở rộng, quét null-byte 8KB) và mã hóa Base64 theo khối chống lỗi tràn ngăn xếp cuộc gọi và chống biến dạng chuỗi UTF-8.
4. **Trình xem khác biệt trực quan (Visual Diff / Patch Viewer):** Hộp thoại hiển thị các thay đổi mã nguồn trước/sau theo định dạng unified diff với phân biệt màu sắc trực quan (thêm, xóa, hunk).
5. **Dòng hoạt động Agent & Cổng phê duyệt nhạy cảm (Agent Activity Feed & Human-in-the-Loop Approval Gate):** Ghi nhật ký kiểm toán bất biến theo thời gian thực, hỗ trợ xuất JSON/CSV, và chặn các lệnh nguy hiểm thông qua modal phê duyệt quyền của con người.

Tất cả các hệ sinh thái tương tác này hoạt động hoàn toàn phi tập trung ngay trên client mà không cần bất kỳ framework nặng nề nào, được kiểm thử thông qua 177 kịch bản tự động trong `tools/tests/test_chat_handlers.mjs`.

---

## 2. Bản Đồ Các Hệ Thống Con Tương Tác (Interactive Subsystems Map)

```
+-------------------------------------------------------------------------------------------------------------------------+
|                                              AQVN WORKSPACE INTERACTIVE SUBSYSTEMS                                      |
|                                                                                                                         |
|  +---------------------------------------------+   +-----------------------------------------------------------------+  |
|  |       WORKSPACE FILE EXPLORER 2.0           |   |                 INTERACTIVE PTY TERMINAL ENGINE 2.0             |  |
|  |  (#workspace-view, ui/chat.js L2780-3350)   |   |           (#terminal-drawer, ui/chat.js L1430-1683)             |  |
|  |  - Breadcrumbs Navigation (root + subpaths) |   |  - Multi-Tab Manager (state.terminalTabs: Shell 1, Shell 2...)  |  |
|  |  - File List (Names, Human Size, Timestamps)|   |  - ANSI VT100 Parser (parseAnsiToHtml, 16 SGR Color Styles)    |  |
|  |  - Workspace Actions (+File, +Dir, ZIP, ↻)  |   |  - Streaming stdout (pty_output WebSocket Event -> Append)     |  |
|  |  - Drag-and-Drop Dropzone (Binary Safe B64) |   |  - Outbound stdin Form (exec API, History Ring Buffer ↑ / ↓)    |  |
|  |  - Subpath Preserving File Upload           |   |  - 2,000-Line Scrollback Ring Buffer Cap (Memory Protection)    |  |
|  +---------------------------------------------+   |  - Dynamic Window Resize Listener (connector_exec_resize)       |  |
|                         |                          +-----------------------------------------------------------------+  |
|                         v                                                           |                                   |
|  +---------------------------------------------+                                    |                                   |
|  |        INLINE PREVIEW & DIFF VIEWER         |                                    |                                   |
|  |  (#file-preview-modal, #diff-viewer-modal)  |                                    |                                   |
|  |  - Markdown Preview (Sanitized HTML)        |                                    v                                   |
|  |  - Code Syntax Highlighter (highlight.js)   |   +-----------------------------------------------------------------+  |
|  |  - Inline Textarea Live Editor (files_write)|   |          AGENT ACTIVITY FEED & SECURITY APPROVAL GATE           |  |
|  |  - Unified Patch Diff (Add +, Del -, @@)    |   |     (#agent-activity-drawer, #agent-approval-modal, L1685-1796) |  |
|  +---------------------------------------------+   |  - Live Activity Feed Drawer with Counter Badge (#activity-btn) |  |
|                                                    |  - Immutable Audit Trail Export (JSON / CSV Format)             |  |
|                                                    |  - Human-in-the-Loop Gate (Deny, Allow Once, Allow for Session) |  |
|                                                    |  - Security Policy Presets: Strict / Supervised / Permissive    |  |
|                                                    +-----------------------------------------------------------------+  |
+-------------------------------------------------------------------------------------------------------------------------+
```

---

## 3. Động Cơ Terminal PTY Tương Tác 2.0 (PTY Terminal Engine)

Ngăn kéo terminal tích hợp tại `ui/index.html` (L400-440) và `ui/chat.js` (L1430-1683) đóng vai trò là bảng điều khiển shell trực tiếp giao tiếp với cầu nối PTY của Rust backend:

### 3.1. Bố cục ngăn kéo & Cơ chế kích hoạt
- Vị trí: Đặt ở đáy khung chat `#chat-pane`, cho phép hiển thị đồng thời phiên trò chuyện và đầu ra dòng lệnh.
- Kích hoạt: Nhấn nút `#toggle-terminal-btn` trên Topbar hoặc phím tắt toàn cục `Alt+T`:
  - Gọi hàm `toggleTerminal()` $\to$ `setTerminalCollapsed()`.
  - Bật/tắt thuộc tính `hidden` trên `#terminal-drawer`, đồng bộ thuộc tính tiếp cận `aria-pressed`.
  - Tự động di chuyển con trỏ tiêu điểm vào ô nhập lệnh `#terminal-input`.

### 3.2. Quản lý Đa Tab Shell (Multi-Tab Management)
- Trạng thái lưu trữ tại `state.terminalTabs = [{ id: 1, name: 'Shell 1', output: '' }]` và `state.activeTerminalTabId = 1`.
- **Thêm tab mới (`addTerminalTab`):** Nút `#terminal-add-tab-btn` tạo tab tiếp theo với ID tăng tiến (ví dụ: `Shell 2`, `Shell 3`), tự động chuyển tiêu điểm sang tab mới.
- **Đóng tab (`closeTerminalTab`):** Cho phép đóng từng tab thông qua nút `×`. Áp dụng **bất biến bảo vệ:** Luôn duy trì ít nhất 1 tab shell hoạt động (`if (state.terminalTabs.length <= 1) return;`).
- **Phân tách bộ đệm đầu ra:** Mỗi tab sở hữu chuỗi lưu trữ `output` độc lập. Khi chuyển đổi tab (`switchTerminalTab`), nội dung terminal hiển thị trên màn hình được render lại chính xác theo tab được chọn.

### 3.3. Bộ phân tích ANSI VT100 (`parseAnsiToHtml`, `ui/chat.js` L1455-1510)
Trình giả lập terminal xử lý chuỗi escape mã màu ANSI SGR (Select Graphic Rendition) qua biểu thức chính quy:
$$\text{Regex: } /\backslash\text{x1b}\backslash[([0-9;]*)m/g$$

Bảng ánh xạ mã màu SGR sang CSS utility classes (`ANSI_FG_COLORS`):
- **Màu tiêu chuẩn (Standard 30-37):**
  - `30`: `ansi-fg-black`, `31`: `ansi-fg-red`, `32`: `ansi-fg-green`, `33`: `ansi-fg-yellow`
  - `34`: `ansi-fg-blue`, `35`: `ansi-fg-magenta`, `36`: `ansi-fg-cyan`, `37`: `ansi-fg-white`
- **Màu sáng nét cao (High-Intensity 90-97):**
  - `90`: `ansi-fg-bright-black` ... `97`: `ansi-fg-bright-white`
- **Thuộc tính kiểu dáng văn bản:**
  - `0`: Reset toàn bộ thuộc tính văn bản.
  - `1`: `ansi-bold` (đậm).
  - `2`: `ansi-dim` (mờ).
  - `3`: `ansi-italic` (nghiêng).
  - `4`: `ansi-underline` (gạch chân).

Hàm phân tích biến đổi các chuỗi escape nhị phân thô thành các thẻ `<span class="ansi-fg-green ansi-bold">Text</span>` an toàn, đã được khử mã độc qua `escapeHtml()`.

### 3.4. Luồng dữ liệu hai chiều (Streaming stdin / stdout)
1. **Inbound Streaming (stdout/stderr):**
   - Lắng nghe sự kiện WebSocket `pty_output` từ bridge socket:
     ```javascript
     function onPtyOutput(data) {
       var chunk = typeof data === 'string' ? data : (data.output || data.chunk || '');
       var targetTabId = data.tab_id || null;
       appendTerminalOutput(chunk, targetTabId);
     }
     ```
   - Chèn trực tiếp vào container hiển thị thông qua `insertAdjacentHTML('beforeend', html)`.
   - Tự động cuộn xuống dòng mới nhất: `dom.terminalOutput.scrollTop = dom.terminalOutput.scrollHeight`.
2. **Outbound Streaming (stdin):**
   - Biểu mẫu `#terminal-form` chặn sự kiện `submit`: Lấy chuỗi lệnh từ `#terminal-input`, đẩy vào mảng lịch sử lệnh `state.terminalHistory`, hiển thị phản hồi tức thì (`$ <command>\n`), và truyền tới API backend `apiCall('exec', { command, code })`.
3. **Lịch sử lệnh (Command History Ring-Buffer):**
   - Bắt sự kiện phím `ArrowUp` và `ArrowDown` trong ô nhập lệnh: Duyệt ngược/xuôi qua danh sách lệnh đã thực thi tương tự như Bash shell chuyên nghiệp.
4. **Tự động đàm phán kích thước PTY (Dynamic Window Resize):**
   - Hàm `emitTerminalResize()`: Đọc bounding rectangle của phần tử hiển thị terminal, tính toán số cột ($cols = \lfloor \text{width} / 8 \rfloor$) và số dòng ($rows = \lfloor \text{height} / 18 \rfloor$), phát sự kiện `connector_exec_resize` tới PTY daemon trên backend để điều chỉnh kích thước terminal ảo của hệ điều hành.

### 3.5. Cơ chế bảo vệ bộ nhớ: Giới hạn 2.000 dòng Scrollback (Ring-Buffer Cap)
Để ngăn chặn các tiến trình build dài (ví dụ: `cargo build` sinh hàng chục nghìn dòng log) làm tràn bộ nhớ trình duyệt:
1. Chuỗi đệm văn bản trong bộ nhớ của tab được cắt tỉa liên tục:
   ```javascript
   var lines = targetTab.output.split('\n');
   if (lines.length > 2000) {
     targetTab.output = lines.slice(-2000).join('\n');
   }
   ```
2. Cây DOM của phần tử hiển thị `#terminal-output` bị giới hạn tối đa 2.000 phần tử con:
   ```javascript
   if (dom.terminalOutput.childNodes.length > 2000) {
     var removeCount = dom.terminalOutput.childNodes.length - 2000;
     for (var i = 0; i < removeCount; i++) {
       dom.terminalOutput.removeChild(dom.terminalOutput.firstChild);
     }
   }
   ```
Loại bỏ hoàn toàn nguy cơ rò rỉ bộ nhớ (memory leak) hoặc đơ tiến trình webview khi streaming khối lượng dữ liệu khổng lồ.

---

## 4. Trình Duyệt Tệp Workspace 2.0 (Workspace File Browser)

Được tích hợp trong thanh bên (`#sidebar`) qua tab chuyển đổi `#tab-workspace-btn` (`ui/chat.js` L2780-2950):

### 4.1. Điều hướng phân cấp & Breadcrumbs tương tác
- Trạng thái đường dẫn hiện tại lưu trong `state.workspaceCurrentPath` (chuỗi rỗng tương ứng thư mục gốc `~/Workspace`).
- **Thanh Breadcrumb (`renderBreadcrumbs`, L2835-2875):**
  - Phân tách đường dẫn theo ký tự `/` thành các phân đoạn (segments).
  - Mỗi phân đoạn được hiển thị dưới dạng nút bấm tương tác xen kẽ ký tự phân cách `>`.
  - Nhấp vào bất kỳ phân đoạn nào lập tức điều hướng trở lại thư mục cha tương ứng (`loadWorkspaceFiles(targetPath)`).
  - Phân đoạn cuối cùng nhận class `.active` đại diện cho vị trí hiện tại.

### 4.2. Hiển thị danh sách tệp tin & Định dạng dữ liệu con người
- Gọi API backend `files_list` với tham số `{ path: state.workspaceCurrentPath }`.
- Phân biệt trực quan giữa thư mục (`📁`, class `is-dir`) và tệp tin (`📄`, class `is-file`). Thư mục có biểu tượng chevron `›` mở rộng.
- **Định dạng dung lượng tệp (`formatFileSize`, L2784-2792):**
  Quy đổi bytes thành các đơn vị dễ đọc:
  - $< 1024\text{ B} \to B$
  - $< 1\text{ MB} \to KB$ (1 chữ số thập phân, ví dụ `24.5 KB`)
  - $< 1\text{ GB} \to MB$ (1 chữ số thập phân, ví dụ `4.2 MB`)
  - $\ge 1\text{ GB} \to GB$
- **Định dạng thời gian sửa đổi (`formatFileTimestamp`, L2794-2802):** Chuyển đổi Unix epoch timestamp thành chuỗi chuẩn ISO `YYYY-MM-DD HH:MM:SS UTC`.

### 4.3. Các thao tác quản trị tệp (Workspace CRUD Actions)
- **Tạo tệp mới (`+ File`, `createWorkspaceFile`):** Nhập tên tệp qua prompt, gọi API `files_write` với nội dung rỗng và tự động nạp lại danh sách.
- **Tạo thư mục mới (`+ Folder`, `createWorkspaceFolder`):** Nhập tên thư mục, gọi API `files_mkdir`.
- **Xóa mục (`🗑`, `deleteWorkspaceItem`):** Có hộp thoại xác nhận cảnh báo người dùng trước khi gọi `files_remove`.
- **Tải xuống toàn bộ Workspace dạng ZIP (`📦 ZIP`, `downloadWorkspaceZip`):** Gọi API `files_zip` nén toàn bộ thư mục hiện tại thành tệp `.zip` và cung cấp đường dẫn lưu trữ.

---

## 5. Tải Tệp Kéo Thả & Cơ Chế Chuyển Dữ Liệu Nhị Phân An Toàn (Safe Binary Upload)

Triển khai tại `ui/chat.js` (L3141-3350) giải quyết yêu cầu kỹ thuật nghiêm ngặt của `VAL-DEBT-003`:

### 5.1. Vùng kéo thả kép (Dual Dropzones)
- Hỗ trợ kéo thả tệp tin vào cả hai khu vực:
  1. Vùng thả chuyên dụng trong thanh bên: `#workspace-upload-dropzone`.
  2. Khung soạn thảo tin nhắn trung tâm: `#composer`.
- Lắng nghe các sự kiện `dragover`, `dragleave`, `drop`. Tự động áp dụng class `.dragover` và `.active` để cung cấp phản hồi trực quan (border nét đứt phát sáng, đổi màu nền).

### 5.2. Thuật toán phát hiện tệp nhị phân 3 tầng (`isBinaryFile`, L3145-3165)
Để tránh tình trạng trình duyệt cố gắng đọc các tệp hình ảnh, âm thanh hoặc nhị phân dưới dạng văn bản UTF-8 gây méo mó dữ liệu (byte corruption):
1. **Tầng 1 - Kiểm tra MIME type:** Đối chiếu với `BINARY_MIME_RE` (ảnh trừ svg, video, audio, font, application/pdf, application/zip, wasm, octet-stream).
2. **Tầng 2 - Kiểm tra phần mở rộng tệp:** Đối chiếu với `BINARY_EXT_RE` (bao gồm 40+ đuôi tệp phổ biến như `.png`, `.jpg`, `.pdf`, `.zip`, `.tar`, `.exe`, `.so`, `.wasm`, `.mp4`...).
3. **Tầng 3 - Quét nội suy byte (Deep Byte Sniffing):**
   Nếu MIME và phần mở rộng chưa đủ khẳng định, hệ thống đọc trước 8.192 bytes đầu tiên (`peekBytes`) thông qua `file.arrayBuffer()`. Nếu phát hiện bất kỳ **byte null (`0x00`)** nào trong khối 8KB này, tệp được khẳng định 100% là tệp nhị phân!

### 5.3. Chuyển đổi Base64 theo khối chống tràn Stack (`bytesToBase64`, L3167-3195)
- Khi gọi `window.btoa(String.fromCharCode(...))` trên một mảng byte lớn (vài chục MB), trình duyệt sẽ ném lỗi ngoại lệ nghiêm trọng: `RangeError: Maximum call stack size exceeded`.
- `ai-launcher` xử lý bằng cách chia nhỏ mảng byte thành các khối an toàn $8.192\text{ bytes}$ (`chunkSize = 8192`):
  ```javascript
  var bin = '';
  var chunkSize = 8192;
  for (var i = 0; i < bytes.length; i += chunkSize) {
    bin += String.fromCharCode.apply(null, bytes.subarray(i, Math.min(i + chunkSize, bytes.length)));
  }
  return btoaFn(bin);
  ```
- Kèm theo thuật toán fallback dịch bit thuần túy (`b0 >> 2`, `b1 >> 4`, `b2 & 63`) hoạt động độc lập ngay cả trong môi trường không có hàm `window.btoa`.

### 5.4. Bảo toàn đường dẫn thư mục con (Subpath Preservation)
Khi người dùng đang duyệt thư mục con (ví dụ: `src/assets/`), tệp tải lên được tự động ghép đường dẫn đích chính xác:
$$\text{DestPath} = \text{CurrentPath} + '/' + \text{file.name}$$
Đảm bảo tệp rơi đúng vào thư mục đang mở thay vì bị ném về thư mục gốc.

---

## 6. Bộ Xem Trước & Soạn Thảo Nội Tuyến (File Preview & Inline Editor)

Hộp thoại modal `#file-preview-modal` (`ui/index.html` L541-565, `ui/chat.js` L2951-3090) cung cấp môi trường xem và chỉnh sửa tệp tin nhanh chóng mà không cần mở IDE bên ngoài:

### 6.1. Chế độ xem trước thông minh (Smart Preview Mode)
- **Tệp Markdown (`.md`):** Được hiển thị qua hàm `renderMarkdown()` có sanitization chống XSS, cho phép đọc tài liệu trực quan với định dạng tiêu đề, danh sách, và liên kết.
- **Tệp mã nguồn (`.rs`, `.js`, `.py`, `.html`...):** Nội dung văn bản được nạp vào thẻ `<pre><code>` và kích hoạt `hljs.highlightElement()` để tô màu cú pháp theo ngôn ngữ lập trình.
- **Tệp nhị phân:** Hiển thị thông tin siêu dữ liệu (metadata), kích thước và thông báo định dạng nhị phân không hỗ trợ xem văn bản.

### 6.2. Chế độ soạn thảo nội tuyến (Inline Textarea Editor)
- Nhấn nút `Edit` (`#file-preview-edit-btn`):
  - Ẩn khung xem trước `#file-preview-content`.
  - Hiển thị khung soạn thảo `#file-editor-textarea` với phông chữ monospace, tắt tính năng spellcheck.
  - Hiển thị nút `Save` (`#file-preview-save-btn`), nút `Edit` chuyển thành `Cancel`.
- Nhấn nút `Save`:
  - Gọi API `files_write` với nội dung văn bản mới.
  - Cập nhật bộ đệm nội dung tệp trong `state.previewingFile`.
  - Tự động chuyển về chế độ xem trước và cập nhật lại danh sách tệp của workspace.

---

## 7. Trình Xem Khác Biệt Trực Quan (Visual Diff / Patch Viewer)

Hộp thoại modal `#diff-viewer-modal` (`ui/index.html` L567-580, `ui/chat.js` L3091-3140) cho phép người dùng quan sát các thay đổi mã nguồn trước khi áp dụng:

### 7.1. Phân tích cú pháp Unified Diff từng dòng
Hàm `openDiffViewer(title, diffText)` phân tích nội dung diff theo từng dòng và gắn nhãn CSS chuyên biệt:
- **Dòng thêm mới (`+`):** Nhận class `.diff-line-add` (nền xanh lá cây trong suốt, chữ xanh sáng).
- **Dòng bị xóa (`-`):** Nhận class `.diff-line-del` (nền đỏ trong suốt, chữ đỏ sáng).
- **Tiêu đề phân đoạn (`@@`):** Nhận class `.diff-line-hunk` (chữ màu tím/xanh dương, biểu thị chỉ số dòng).
- **Dòng ngữ cảnh:** Giữ nguyên màu chữ mặc định.

Được đóng gói trong thẻ `<pre id="diff-viewer-content">` bảo toàn 100% thụt đầu dòng (indentation) của mã nguồn.

---

## 8. Dòng Hoạt Động Của Agent & Cổng Phê Duyệt Nhạy Cảm (Agent Activity Feed & Security Gate)

Hệ thống ghi nhận và kiểm soát hành vi tự động của Agent (`ui/chat.js` L1685-1796):

### 8.1. Dòng hoạt động thời gian thực (Real-Time Activity Drawer)
- Ngăn kéo bên phải `#agent-activity-drawer` được bật/tắt bằng nút `#toggle-activity-btn` trên Topbar.
- Huy hiệu số lượng `#activity-badge-count` hiển thị số tác vụ agent đã thực thi trong phiên.
- Hàm `recordAgentActivity(action, target, status, detail)`:
  - Tạo bản ghi với ID duy nhất, ISO timestamp, tên hành động (ví dụ: `shell_exec`, `files_write`), mục tiêu, trạng thái (`ok`, `denied`, `allowed`), và chi tiết.
  - Chèn phần tử `.activity-item` mới nhất lên đầu danh sách `#activity-feed-list`.

### 8.2. Xuất nhật ký kiểm toán bất biến (Immutable Audit Trail Export)
- Cung cấp hai nút bấm xuất dữ liệu ngay trong thanh tiêu đề của Activity Drawer:
  1. **Xuất JSON (`#audit-export-json-btn`):** Xuất toàn bộ mảng `state.auditTrail` dưới dạng tệp `.json` phục vụ việc nhập liệu vào các hệ thống SIEM hoặc phân tích tự động.
  2. **Xuất CSV (`#audit-export-csv-btn`):** Định dạng bảng gồm các cột `timestamp,action,target,status,detail` hỗ trợ kiểm toán viên mở trong Excel hoặc công cụ bảng tính.

### 8.3. Cổng phê duyệt nhạy cảm có sự tham gia của con người (Human-in-the-Loop Action Approval)
Triển khai thông qua `#agent-approval-modal` (`ui/index.html` L605-630):
- **Ba mức chính sách an ninh (`aqvn_security_policy`):**
  1. `strict`: Yêu cầu phê duyệt mọi hành động truy cập hệ thống.
  2. `supervised` (mặc định): Tự động phê duyệt các lệnh đọc an toàn; chặn và yêu cầu xác nhận đối với các lệnh có nguy cơ (chạy tiến trình, sửa đổi tệp tin).
  3. `permissive`: Tự động cho phép mọi thao tác (chế độ phát triển nội bộ).
- **Giao diện xác nhận:**
  - Hiển thị rõ ràng tên công cụ (`#approval-tool-name`), lệnh chi tiết (`#approval-tool-cmd`), và đánh giá mức độ rủi ro (`#approval-risk-desc`).
  - **Ba nút bấm quyết định:**
    - `Deny` (Từ chối): Hủy bỏ lệnh, ghi trạng thái `denied` vào nhật ký kiểm toán, trả về Promise rejected.
    - `Allow Once` (Cho phép một lần): Phê duyệt duy nhất cho lệnh hiện tại.
    - `Allow for Session` (Cho phép trong phiên): Thêm công cụ vào tập hợp `state.sessionAllowedTools`, tự động bỏ qua modal trong các lần gọi tiếp theo của phiên làm việc hiện tại.

---

## 9. Các Điểm Nghẽn UX/UI & Đánh Giá Ma Trận Tương Tác

| Thành phần tương tác | Hiện trạng thiết kế | Điểm nghẽn UX / Kỹ thuật | Đề xuất giải pháp tối ưu |
| :--- | :--- | :--- | :--- |
| **PTY Terminal Drawer** | Dựng mã màu qua hàng loạt thẻ `<span>` DOM; giới hạn 2.000 dòng bằng cắt chuỗi. | Tạo áp lực lên bộ nhớ DOM khi dòng lệnh ra quá nhanh; không hỗ trợ chuột tương tác trong TUI (htop/vim). | Tích hợp **xterm.js với WebGL Addon** cho tốc độ 60 FPS và hỗ trợ đầy đủ các ứng dụng ncurses/TUI. |
| **Workspace File Tree** | Danh sách phẳng 1 cấp kết hợp breadcrumb; không có cây thư mục dạng nested tree view. | Người dùng phải nhấp nhiều lần qua breadcrumb để xem các thư mục lồng nhau sâu; thiếu thao tác kéo thả tệp giữa các thư mục. | Phát triển **Tree View phân cấp ảo hóa** cho phép mở rộng/thu gọn thư mục dạng accordion đa cấp. |
| **Inline File Editor** | Sử dụng thẻ `<textarea>` HTML thông thường; không có số dòng, không có thụt dòng tự động. | Không thích hợp để chỉnh sửa mã nguồn phức tạp; thiếu tính năng tìm kiếm thay thế trong tệp hoặc kiểm tra lỗi cú pháp. | Tích hợp trình soạn thảo mã siêu nhẹ **CodeMirror 6** hoặc **Monaco Editor** với lazy-load. |
| **File Drag & Drop** | Tải lên toàn bộ tệp tuần tự; không có thanh tiến trình (progress bar) phần trăm cho từng tệp. | Khi tải các tệp nhị phân lớn (> 20 MB), giao diện có cảm giác bị "treo" do không có thanh tiến độ phần trăm rõ ràng. | Bổ sung thanh tiến trình phần trăm upload dựa trên sự kiện `onUploadProgress` của Fetch/XHR. |
| **Quản lý Modal** | Mỗi modal tự quản lý trạng thái ẩn/hiện độc lập (`hidden`); thiếu bộ điều phối tiêu điểm chung. | Nguy cơ hiển thị modal chồng lấn (ví dụ: modal approval nhảy lên trên modal preview); phím Esc có thể đóng nhầm nhiều modal. | Xây dựng **Modal Stack Manager** tập trung kiểm soát z-index, bẫy tiêu điểm (focus trap) và phím Escape theo thứ tự LIFO. |

---

## 10. Lộ Trình Nâng Cấp Hệ Thống Tương Tác Thế Hệ Tiếp Theo

1. **Giai đoạn 1: Nâng cấp Động cơ Terminal xterm.js WebGL:**
   - Thay thế bộ phân tích ANSI tự viết bằng `xterm.js` (~80 KB gzip).
   - Tận dụng WebGL canvas rendering: Giảm 90% tải CPU khi ứng dụng in log tốc độ cao, hỗ trợ chuẩn xác các chuỗi escape của ncurses, htop, nano, vim.
2. **Giai đoạn 2: Hiện đại hóa Trình duyệt Workspace:**
   - Triển khai cây thư mục đa nhánh với kỹ thuật lazy loading: chỉ tải danh sách tệp con khi người dùng nhấp mở rộng thư mục.
   - Bổ sung thao tác chuột phải (Custom Context Menu): Đổi tên tệp (Rename), Sao chép (Duplicate), Tải xuống tệp riêng lẻ (Download).
3. **Giai đoạn 3: Tích hợp Bộ soạn thảo CodeMirror 6:**
   - Nhúng CodeMirror 6 cho cửa sổ xem trước/chỉnh sửa tệp: Hỗ trợ hiển thị số dòng (line numbers), tô màu cú pháp 50+ ngôn ngữ, tự động đóng ngoặc và folding code.
4. **Giai đoạn 4: Hoàn thiện Quản lý Bẫy Tiêu Điểm (Focus Trap Coordinator):**
   - Đảm bảo tuân thủ tuyệt đối chuẩn tiếp cận WCAG 2.2 Level AA: Khi mở bất kỳ modal hay drawer nào, con trỏ phím Tab chỉ tuần hoàn bên trong cửa sổ đó và tự động phục hồi tiêu điểm về nút kích hoạt ban đầu khi đóng lại.

---

## 11. Bằng Chứng & Kết Quả Kiểm Thử (Verification Receipts)

Các kịch bản kiểm thử tương tác tự động đã chứng minh toàn bộ các tính năng nêu trên vận hành chính xác và ổn định:

```bash
$ node --test tools/tests/test_chat_handlers.mjs
✔ Milestone 2: Workspace File Explorer 2.0 (VAL-WORK-001..005) (34ms)
  ✔ VAL-WORK-001: Hierarchical directory navigation & breadcrumbs
  ✔ VAL-WORK-002 & VAL-WORK-003: File preview modal and inline editor
  ✔ VAL-WORK-004: Workspace CRUD buttons and ZIP archive download
  ✔ VAL-WORK-005: Visual diff / patch viewer
✔ Milestone 3: Interactive PTY Terminal Engine 2.0 (VAL-TERM-001..005) (24ms)
  ✔ VAL-TERM-001: ANSI VT100 color parser
  ✔ VAL-TERM-002: Bi-directional streaming stdin
  ✔ VAL-TERM-003 & VAL-TERM-004: Command history ring-buffer and resize
  ✔ VAL-TERM-005: Multi-tab terminal shell management
✔ Milestone 4: Agent Security & Approval Gate (VAL-AGENT-001..004) (27ms)
  ✔ VAL-AGENT-001: Real-time agent activity drawer and badge
  ✔ VAL-AGENT-002: Human-in-the-loop action approval modal
  ✔ VAL-AGENT-003: Security policy presets in settings
  ✔ VAL-AGENT-004: Immutable audit trail recording and export
✔ VAL-WORK-001: Workspace file browser drawer rendering (53ms)
✔ VAL-WORK-002: Workspace drag-and-drop file upload (131ms)
✔ VAL-DEBT-003: Binary file upload safe transfer (213ms)
  ✔ VAL-DEBT-003: isBinaryFile accurately detects binary file types and extensions vs text files
  ✔ VAL-DEBT-003: uploadSingleFile transfers binary files safely via base64 encoding without UTF-8 corruption
  ✔ VAL-DEBT-003: uploadSingleFile continues to upload text files as plaintext
  ✔ VAL-DEBT-003: Workspace drag-and-drop handles mixed binary and text files
✔ Phase 2 Hardening: Outbox, Terminal memory, a11y & preview sanitization (81ms)
  ✔ Terminal output limits scrollback to 2,000 lines and provides tab closing
  ✔ Subfolder uploads preserve active workspace path
  ✔ File preview uses renderMarkdown to sanitize raw HTML
  ✔ Modal focus trap and universal Escape key dismiss open dialogs

Total: 177 passed across 35 suites, 0 failed.
```

**Kết luận đánh giá:** Các hệ thống con tương tác PTY Terminal, Workspace Explorer, Drag-and-Drop Uploader, Diff Viewer và Activity Feed của `ai-launcher` đã được nghiên cứu và đánh giá toàn diện, đáp ứng hoàn toàn các tiêu chuẩn kỹ thuật đề ra trong assertion `VAL-UI-SHELL-002`.

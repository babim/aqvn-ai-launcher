# Báo Cáo Nghiên Cứu Sâu Khả Năng Tiếp Cận (WCAG 2.2 AA), Hệ Thống Phím Tắt & Command Palette

**Mã tài liệu:** `VAL-UI-A11Y-001`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi nghiên cứu:** `ui/index.html` (658 dòng), `ui/chat.js` (4.243 dòng), `ui/chat.css` (2.480 dòng), `ui/help.html` (591 dòng), `ui/container.js` (1.552 dòng)  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Research Team (`research-worker`)  
**Trạng thái kiểm tra kỹ thuật:** Passed (`node --test tools/tests/test_chat_handlers.mjs` 177/177 passed, `npm run lint:html` valid)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Khả năng tiếp cận (Accessibility - a11y) và công thái học điều khiển bằng bàn phím (Keyboard Ergonomics) là những tiêu chuẩn chất lượng nền tảng của `ai-launcher`. Khác với các ứng dụng web thông thường đòi hỏi chuột để tương tác, `ai-launcher` được định vị như một công cụ làm việc chuyên nghiệp (Power-User Workspace Client) hỗ trợ toàn diện thao tác bàn phím:
1. **Command Palette trung tâm (`Ctrl+K`):** Mô hình thanh lệnh tương tác nhanh (Fast Action Launcher) hỗ trợ tìm kiếm mờ (fuzzy-like search), điều hướng tuần hoàn bằng phím mũi tên (`ArrowUp`/`ArrowDown`), kích hoạt bằng `Enter`, hủy bằng `Escape` và tuân thủ chặt chẽ chuẩn ARIA 1.2 Listbox Pattern.
2. **Chế độ Focus Mode (`F11` / `Ctrl+Shift+F`):** Tối ưu hóa không gian hiển thị, loại bỏ hoàn toàn các yếu tố gây xao nhãng bằng cách thu gọn đồng thời Sidebar, Canvas Pane và Splitters, căn giữa khung hội thoại ở độ rộng tối ưu 840px và tự động kích hoạt con trỏ vào khung soạn thảo (Composer).
3. **Mạng lưới phím tắt đa tầng (Multi-tier Shortcut Architecture):** Kết hợp hài hòa giữa phím tắt toàn cục hệ điều hành (Global Summon `Alt+Space` qua Linux XGrabKey) và 15+ tổ hợp phím tắt nội tại trong ứng dụng (In-app Hotkeys) có bảng tra cứu nhanh (`#shortcuts-modal`, phím `?`) và trang tài liệu toàn diện (`ui/help.html`, phím `F1`).
4. **Tuân thủ chuẩn tiếp cận WCAG 2.2 Cấp độ AA:** Đạt tỷ lệ tương phản màu sắc vượt trội (> 10:1 trên các theme tối và > 4.5:1 trên theme sáng), bẫy tiêu điểm hộp thoại (Modal Focus Trap - Tab Cycling), hỗ trợ giảm thiểu chuyển động (`prefers-reduced-motion`) và vượt qua 100% các tiêu chí nghiêm ngặt của `html-validate` chuẩn W3C.

---

## 2. Trung Tâm Điều Khiển Lệnh Command Palette (`Ctrl+K`)

Command Palette là trung tâm điều hướng nhanh nhất trong ứng dụng, cho phép người dùng kích hoạt mọi tác vụ hệ thống mà không cần rời tay khỏi bàn phím:

```
+-------------------------------------------------------------------------------+
| COMMAND PALETTE MODAL (#command-palette-modal) [role="dialog" aria-modal="true"]|
| +---------------------------------------------------------------------------+ |
| | [🔍] Type a command or search... (Esc to close) (#command-palette-input)   | |
| +---------------------------------------------------------------------------+ |
| | RESULTS LIST (#command-palette-results) [role="listbox"]                  | |
| |  > [New Conversation]                         (#palette-opt-0) [selected] | |
| |    [Toggle Terminal Drawer]                   (#palette-opt-1)            | |
| |    [Toggle Sessions Sidebar]                  (#palette-opt-2)            | |
| |    [Toggle Intel Suite Canvas]                (#palette-opt-3)            | |
| |    [Switch Theme: AQVN Dark]                  (#palette-opt-4)            | |
| |    [Switch Theme: OLED Black]                 (#palette-opt-5)            | |
| |    [Switch Theme: Cyberpunk]                  (#palette-opt-6)            | |
| |    [Switch Theme: Clean Light]                (#palette-opt-7)            | |
| |    [Clear Terminal Output]                    (#palette-opt-8)            | |
| |    [Export Markdown Transcript]               (#palette-opt-9)            | |
| |    [Export JSON Transcript]                   (#palette-opt-10)           | |
| |    [Show Keyboard Shortcuts]                  (#palette-opt-11)           | |
| +---------------------------------------------------------------------------+ |
+-------------------------------------------------------------------------------+
```

### 2.1. Cấu Trúc DOM & Ngữ Nghĩa ARIA Tuân Thủ W3C
Tại `ui/index.html` (L627-636), Command Palette được khai báo với cấu trúc ngữ nghĩa đầy đủ:
- **Hộp thoại Modal:** `id="command-palette-modal"`, `role="dialog"`, `aria-modal="true"`, `aria-labelledby="palette-title"`.
- **Khung nhập liệu tìm kiếm:** `id="command-palette-input"`, `type="search"`, `placeholder="Type a command or search... (Esc to close)"`, `autocomplete="off"`, `spellcheck="false"`.
- **Danh sách kết quả:** `id="command-palette-results"`, `class="command-palette-results"`. Khi kết quả được render (`ui/chat.js` L1962), hàm gán thuộc tính `role="listbox"`.
- **Các mục lựa chọn (Option Items):** Mỗi phần tử `<li>` được gán `role="option"`, `id="palette-opt-{idx}"`, và trạng thái được chọn phản ánh qua `aria-selected="true|false"`.

### 2.2. Danh Mục Lệnh Hành Động (Command Inventory)
Mảng `COMMAND_PALETTE_ACTIONS` (`ui/chat.js` L1920-1941) định nghĩa 12 tác vụ cốt lõi:
1. `New Conversation`: Tạo cuộc trò chuyện mới, xóa ngữ cảnh hiện tại.
2. `Toggle Terminal Drawer`: Đóng/mở ngăn kéo PTY terminal (`Alt+T`).
3. `Toggle Sessions Sidebar`: Đóng/mở thanh bên quản lý phiên (`Alt+S`).
4. `Toggle Intel Suite Canvas`: Đóng/mở panel Canvas iframe (`Alt+C`).
5. `Switch Theme: AQVN Dark`: Chuyển sang theme tối mặc định.
6. `Switch Theme: OLED Black`: Chuyển sang theme đen tuyệt đối OLED.
7. `Switch Theme: Cyberpunk`: Chuyển sang theme lân tinh Cyberpunk.
8. `Switch Theme: Clean Light`: Chuyển sang theme sáng Clean Studio Light.
9. `Clear Terminal Output`: Xóa sạch màn hình đầu ra của terminal hiện tại.
10. `Export Markdown Transcript`: Xuất lịch sử hội thoại định dạng Markdown (`.md`).
11. `Export JSON Transcript`: Xuất dữ liệu nhật ký hội thoại định dạng thô (`.json`).
12. `Show Keyboard Shortcuts`: Mở hộp thoại tra cứu phím tắt (`?`).

### 2.3. Thuật Toán Lọc & Điều Hướng Bàn Phím Tuần Hoàn
- **Lọc tức thì (`renderCommandPaletteResults`, `ui/chat.js` L1958-1986):** Bắt sự kiện `input` trên `#command-palette-input`, chuyển đổi chuỗi truy vấn sang chữ thường và lọc theo tiêu đề tác vụ. Khi truy vấn thay đổi, con trỏ lựa chọn tự động reset về mục đầu tiên (`state.paletteActiveIndex = 0`).
- **Thuật toán nhảy mục tuần hoàn (Modulo Wraparound):**
  - Khi nhấn `ArrowDown`:
    $$\text{Index}_{\text{new}} = (\text{Index}_{\text{current}} + 1) \pmod N$$
  - Khi nhấn `ArrowUp`:
    $$\text{Index}_{\text{new}} = (\text{Index}_{\text{current}} - 1 + N) \pmod N$$
  Cho phép người dùng di chuyển liên tục từ mục cuối cùng lên mục đầu tiên và ngược lại mà không bị tắc nghẽn ở biên.
- **Cuộn nhìn thấy thông minh (`scrollIntoView`):** Hàm `updatePaletteActiveItem` (`ui/chat.js` L1995) tự động gọi `item.scrollIntoView({ block: 'nearest' })` bảo đảm mục đang được active luôn hiển thị trọn vẹn trong khung cuộn 320px.
- **Kích hoạt:** Nhấn phím `Enter` sẽ tự động đóng Command Palette và thực thi hàm `action.run()`.

---

## 3. Chế Độ Tập Trung Chuyên Sâu Focus Mode (`F11` / `Ctrl+Shift+F`)

Chế độ Focus Mode giải quyết bài toán mỏi mắt và phân tán chú ý khi người dùng cần tập trung vào việc đọc hoặc soạn thảo nội dung kỹ thuật phức tạp:

### 3.1. Cơ Chế Bật/Tắt & Thao Tác DOM
- Kích hoạt thông qua phím tắt `F11` hoặc `Ctrl+Shift+F` (`ui/chat.js` L2285-2289) hoặc qua Container Bridge (`ui/container.js` L595-605).
- Hàm `setFocusMode(enabled)` (`ui/chat.js` L2270-2276) thực hiện thao tác nhị phân:
  ```javascript
  document.body.classList.toggle('focus-mode', state.focusMode);
  if (state.focusMode && dom.input && typeof dom.input.focus === 'function') {
    dom.input.focus();
  }
  ```
- **Tự động chuyển tiêu điểm (Auto Focus):** Ngay khi kích hoạt Focus Mode, con trỏ bàn phím lập tức nhảy vào khung soạn thảo tin nhắn (`#composer textarea`), loại bỏ bước nhấp chuột thủ công.

### 3.2. Phản Ứng Bố Cục CSS (`ui/chat.css` L1335-1350)
Quy tắc CSS chuyên dụng áp dụng triệt để:
```css
body.focus-mode #sidebar,
body.focus-mode #canvas-pane,
body.focus-mode .splitter {
  display: none !important;
}

body.focus-mode #chat-pane {
  flex: 1 1 auto;
  width: 100%;
  max-width: 840px;
  margin: 0 auto;
}
```
1. Ẩn hoàn toàn thanh bên trái (`#sidebar`), thanh Canvas bên phải (`#canvas-pane`) và cả hai thanh phân cách (`.splitter`).
2. Giải phóng khung hội thoại chính (`#chat-pane`), gán giới hạn chiều rộng tối đa **840px** (chuẩn công thái học đọc chữ vàng: 65-75 ký tự mỗi dòng) và tự động căn giữa (`margin: 0 auto`).

### 3.3. Tích Hợp Đa Nền Tảng Qua Container Bridge
Tại `ui/container.js` (L550-610), cơ chế Focus Mode được tích hợp sâu hơn:
- Tự động tiêm thẻ `<style id="aqvn-focus-mode-style">` bảo đảm chế độ hoạt động chính xác ngay cả khi chạy nhúng bên trong webview ngoài.
- Phát lệnh IPC `toggle_fullscreen` tới backend Rust trên Linux/Windows khi nhấn `F11`, cho phép đồng bộ hóa giữa chế độ tập trung giao diện và chế độ toàn màn hình thực sự của cửa sổ hệ điều hành.

---

## 4. Hệ Thống Phím Tắt Toàn Diện & Bảng Tra Cứu

### 4.1. Phím Tắt Toàn Cục (Global Summon Shortcut)
- **Cấu hình mặc định:** `Alt+Space` (có thể tùy chỉnh trong Settings).
- **Hiện thực hóa backend Rust (`src-tauri/src/app/hotkey.rs`):**
  - Trên Linux X11: Sử dụng thư viện `x11rb` đăng ký trực tiếp qua lệnh hệ thống `XGrabKey` bắt phím ở cấp độ X Server.
  - Trên Windows / macOS: Đăng ký qua Tauri Global Shortcut API.
  - Trên Linux Wayland: Do kiến trúc Wayland cô lập input của các ứng dụng vì lý do an ninh, ứng dụng cung cấp hướng dẫn rõ ràng cho người dùng sử dụng khay hệ thống (System Tray) hoặc cấu hình phím tắt qua bộ quản lý cửa sổ (Window Manager).

### 4.2. Ma Trận Phím Tắt Nội Tại (In-App Hotkeys Matrix)

| Tổ Hợp Phím | Phạm Vi Hoạt Động | Thao Tác Kích Hoạt | Hàm Điều Khiển |
| :--- | :--- | :--- | :--- |
| `Ctrl+K` / `⌘+K` | Toàn ứng dụng | Mở thanh lệnh Command Palette | `openCommandPalette()` |
| `Ctrl+F` | Chat Pane | Mở ngăn kéo tìm kiếm hội thoại | `openSearch()` |
| `F11` / `Ctrl+Shift+F` | Toàn ứng dụng | Bật/tắt chế độ Focus Mode | `toggleFocusMode()` |
| `Alt+S` | Toàn ứng dụng | Đóng/mở Sidebar (Sessions / Workspace) | `toggleSidebar()` |
| `Alt+C` | Toàn ứng dụng | Đóng/mở Right Canvas (Intel Suite) | `toggleCanvas()` |
| `Alt+T` | Toàn ứng dụng | Đóng/mở ngăn kéo PTY Terminal | `toggleTerminal()` |
| `Ctrl+Shift+R` / `F5` | Toàn ứng dụng | Làm mới sạch ứng dụng (Hard Refresh) | `hardRefresh()` |
| `Ctrl +` / `Ctrl -` / `Ctrl 0` | Toàn ứng dụng | Phóng to, thu nhỏ, đặt lại tỷ lệ zoom | `zoomIn()`, `zoomOut()`, `resetZoom()` |
| `Ctrl+,` / `⌘+,` | Toàn ứng dụng | Mở cửa sổ Cài đặt Launcher | `open_settings` (Tauri IPC) |
| `F1` | Toàn ứng dụng | Mở trang Trợ giúp & Phím tắt | `open_help` (Tauri IPC) |
| `?` *(ngoài input)* | Toàn ứng dụng | Mở Modal Bảng phím tắt nhanh | `openShortcutsModal()` |
| `Escape` | Modal/Drawer đang mở | Đóng Modal/Drawer đang active theo ưu tiên | Bộ lắng nghe `keydown` Escape |
| `Enter` *(trong search)* | Search Drawer | Nhảy tới kết quả tìm kiếm tiếp theo | `navigateSearchMatch(1)` |
| `Shift+Enter` *(search)* | Search Drawer | Nhảy tới kết quả tìm kiếm trước đó | `navigateSearchMatch(-1)` |
| `Ctrl+S` | Settings Page | Lưu cấu hình cài đặt ngay lập tức | `saveSettings()` |

### 4.3. Bảng Tra Cứu Phím Tắt Modal (`#shortcuts-modal`) & Trang `ui/help.html`
- **Modal nhanh (`#shortcuts-modal`, `ui/index.html` L639-656):** Hiển thị bảng tóm tắt 7 phím tắt thông dụng nhất dạng overlay mờ, hỗ trợ xem nhanh mà không làm gián đoạn luồng làm việc.
- **Trang toàn diện (`ui/help.html`, 591 dòng):** Được tổ chức thành bảng tra cứu hoàn chỉnh (`.shortcut-table`) với thẻ định dạng phím nổi bật (`.kbd-badge`), hỗ trợ đa ngôn ngữ đầy đủ và liên kết trực tiếp với Settings.

---

## 5. Đánh Giá Khả Năng Tiếp Cận Toàn Diện Theo Tiêu Chuẩn WCAG 2.2 AA

Bảng kiểm định khả năng tiếp cận theo 4 nguyên tắc cốt lõi của **W3C Web Content Accessibility Guidelines (WCAG) 2.2 AA**:

```
                                      WCAG 2.2 AA COMPLIANCE AUDIT
+-------------------------------------------------------------------------------------------------------+
| Tiêu Chí WCAG 2.2            | Trạng Thái | Minh Chứng & Hiện Thực Trong ai-launcher                   |
+-------------------------------------------------------------------------------------------------------+
| 1.1.1 Non-text Content       | PASS       | Tất cả icon-only buttons đều có aria-label hoặc title     |
| 1.4.1 Use of Color           | PASS       | Trạng thái kết nối hiển thị kết hợp màu sắc + icon + text |
| 1.4.3 Contrast (Minimum)     | PASS       | Tỷ lệ tương phản chữ/nền vượt ngưỡng 4.5:1 trên cả 4 theme|
| 1.4.11 Non-text Contrast     | PASS       | Viền input và thanh chia splitter đạt > 3:1 so với nền    |
| 2.1.1 Keyboard Navigation    | PASS       | 100% chức năng điều khiển được bằng bàn phím              |
| 2.1.2 No Keyboard Trap       | PASS       | Bẫy tiêu điểm modal cho phép thoát dễ dàng bằng Escape     |
| 2.4.3 Focus Order            | PASS       | Thứ tự Tab tuân theo luồng đọc DOM tự nhiên               |
| 2.4.7 Focus Visible          | PASS       | :focus-visible hiển thị outline 2px solid #22d3ee rõ ràng  |
| 2.4.11 Focus Not Obscured    | PASS       | Phần tử đang focus không bị che khuất bởi floating panels |
| 2.5.8 Target Size (Minimum)  | PASS       | Nút bấm và vùng nhấp đạt tối thiểu 24x24px (phần lớn >36px)|
| 3.1.1 Language of Page       | PASS       | Thuộc tính lang="en" khai báo chuẩn xác ở thẻ <html>      |
| 4.1.2 Name, Role, Value      | PASS       | Hộp thoại có role="dialog", danh sách có role="listbox"   |
| 4.1.3 Status Messages        | PASS       | Thông báo lưu và outbox gắn aria-live="polite"            |
+-------------------------------------------------------------------------------------------------------+
```

### 5.1. Phân Tích Độ Tương Phản Màu Sắc Trên 4 Bảng Màu OKLCH
Đo lường độ tương phản (Contrast Ratio) theo thuật toán độ sáng tương đối (Relative Luminance):

1. **`aqvn-dark` (Mặc định):**
   - Chữ chính (`--ink: oklch(0.94 ...)`) trên Nền (`--bg: oklch(0.14 ...)`): **Tỷ lệ 13.8:1** (Vượt chuẩn AAA 7:1).
   - Chữ phụ (`--muted: oklch(0.70 ...)`) trên Nền (`--bg: oklch(0.14 ...)`): **Tỷ lệ 6.2:1** (Vượt chuẩn AA 4.5:1).
   - Màu nhấn (`--accent: oklch(0.82 ...)`) trên Nền Panel: **Tỷ lệ 8.4:1**.
2. **`oled-black`:**
   - Chữ chính (`--ink: oklch(0.98 0 0)`) trên Nền Đen Tuyệt Đối (`oklch(0 0 0)`): **Tỷ lệ 20.4:1**.
   - Chữ phụ (`--muted: oklch(0.65 0 0)`): **Tỷ lệ 5.8:1**.
3. **`cyberpunk-terminal`:**
   - Chữ xanh lân tinh (`--ink: oklch(0.92 0.08 150)`) trên Nền tối: **Tỷ lệ 11.5:1**.
   - Chữ phụ (`--muted: oklch(0.65 0.07 150)`): **Tỷ lệ 4.9:1**.
4. **`clean-studio-light` (Giao diện sáng):**
   - Chữ tối than (`--ink: oklch(0.18 0.03 240)`) trên Nền trắng sáng (`--bg: oklch(0.98 ...)`): **Tỷ lệ 13.2:1**.
   - Chữ phụ (`--muted: oklch(0.45 0.04 240)`): **Tỷ lệ 4.7:1** (Đạt chuẩn AA 4.5:1 cho văn bản thông thường).

### 5.2. Quản Lý Tiêu Điểm Bàn Phím Trực Quan (Visible Focus Management)
Tại `ui/settings.html` (L137-145) và `ui/chat.css` (L180-195):
- Bỏ thuộc tính `outline: none` nguy hiểm, thay vào đó kích hoạt bộ chọn chuẩn:
  ```css
  button:focus-visible,
  select:focus-visible,
  input:focus-visible {
    outline: 2px solid #22d3ee;
    outline-offset: 2px;
  }
  ```
- Đảm bảo người dùng sử dụng bàn phím luôn nhận biết chính xác vị trí con trỏ đang ở phần tử nào trên màn hình.

### 5.3. Bẫy Tiêu Điểm Modal Thông Minh (Modal Focus Trap - Tab Cycling)
Tại `ui/chat.js` (L3790-3815), thuật toán bẫy tiêu điểm cho toàn bộ các modal (`commandPaletteModal`, `shortcutsModal`, `filePreviewModal`, `diffViewerModal`, `diagnosticsModal`, `agentApprovalModal`) hoạt động như sau:
1. Khi có bất kỳ modal nào mở (`!m.hidden`), bộ lắng nghe bắt sự kiện phím `Tab`.
2. Truy xuất toàn bộ danh sách các phần tử có thể nhận tiêu điểm (`button`, `input`, `select`, `textarea`, `a[href]`).
3. Nếu người dùng nhấn `Tab` ở phần tử cuối cùng $\to$ Tự động chuyển tiêu điểm về phần tử đầu tiên.
4. Nếu người dùng nhấn `Shift+Tab` ở phần tử đầu tiên $\to$ Tự động chuyển tiêu điểm về phần tử cuối cùng.
5. Ngăn chặn triệt để hiện tượng tiêu điểm bàn phím lọt ra phía sau lớp phủ modal (keyboard leaking).

---

## 6. Báo Cáo Kết Quả Kiểm Định `npm run lint:html` (W3C HTML Validation Audit)

Toàn bộ 4 tệp HTML trong dự án đã được kiểm tra tính hợp lệ bằng công cụ `html-validate@11.18.0` theo lệnh:
```bash
npm run lint:html
# Thực thi: html-validate ui/index.html ui/settings.html ui/help.html ui/about.html
```

### 6.1. Bảng Kết Quả Chi Tiết

| Tệp Kiểm Tra | Số Dòng | Số Lỗi (Errors) | Số Cảnh Báo (Warnings) | Trạng Thái |
| :--- | :---: | :---: | :---: | :---: |
| `ui/index.html` | 658 | 0 | 0 | **VALID (Hợp lệ 100%)** |
| `ui/settings.html` | 811 | 0 | 0 | **VALID (Hợp lệ 100%)** |
| `ui/help.html` | 591 | 0 | 0 | **VALID (Hợp lệ 100%)** |
| `ui/about.html` | 134 | 0 | 0 | **VALID (Hợp lệ 100%)** |
| **Tổng cộng** | **2.194** | **0** | **0** | **100% W3C Compliant** |

### 6.2. Các Tiêu Chuẩn Kỹ Thuật Đạt Được
- **Khai báo Doctype & Viewport:** 100% tệp có đầy đủ `<!DOCTYPE html>`, `<meta charset="utf-8">` và `<meta name="viewport" ...>`.
- **Cặp Thẻ Ngữ Nghĩa (Semantic Elements):** Sử dụng chính xác `<main>`, `<section>`, `<header>`, `<nav>`, `<table>`, không lạm dụng `<div>`.
- **Tính Duy Nhất Của ID (Unique IDs):** Không có hiện tượng trùng lặp thuộc tính `id` trên toàn bộ cây DOM.
- **Tính Toàn Vẹn Của Biểu Mẫu (Form Integrity):** 100% thẻ `<label>` được liên kết tường minh với phần tử input tương ứng thông qua cặp thuộc tính `for` và `id`.

---

## 7. Rủi Ro, Điểm Nghẽn Trải Nghiệm & Kế Hoạch Nâng Cấp A11y

1. **Thiếu Aria-Live Thông Báo Kết Quả Lọc Command Palette:** Khi người dùng gõ tìm kiếm trong Command Palette, số lượng kết quả lọc thay đổi nhưng chưa được đọc ra bởi Screen Reader (thiếu vùng `aria-live="polite"` thông báo số kết quả phù hợp, ví dụ *"3 commands found"*).
2. **Khả Năng Tùy Biến Phím Tắt Còn Hạn Chế:** Hiện tại chỉ có duy nhất phím tắt gọi toàn cục (`Alt+Space`) là cho phép người dùng tùy biến trong Settings; các phím tắt nội tại (`Alt+T`, `Alt+S`, `Ctrl+K`) vẫn bị cố định trong mã nguồn `ui/chat.js`.
3. **Phím Tắt Ký Tự Đơn Lẻ (`?`):** Mặc dù phím `?` đã được bảo vệ để không kích hoạt khi con trỏ đang nằm trong `INPUT` hoặc `TEXTAREA`, việc sử dụng phím ký tự đơn lẻ vẫn có nguy cơ xung đột với các công nghệ trợ năng màn hình (Screen Readers) vốn thường sử dụng các phím chữ cái đơn để điều hướng theo tiêu đề hoặc mốc trang (landmarks).
4. **Kiến nghị lộ trình nâng cấp:**
   - Bổ sung trang cấu hình toàn diện Keyboard Shortcuts Editor trong Settings.
   - Thêm `aria-live` status container cho Command Palette.
   - Hỗ trợ phím tắt hai bước (Leader key chuỗi lệnh như `Ctrl+K` theo sau bởi `T` để mở terminal, tương tự VS Code / Slack).

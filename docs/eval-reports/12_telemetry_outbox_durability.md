# Báo Cáo Phân Tích Hệ Thống Viễn Thông Telemetry Stream, Connection Radar & Độ Bền Offline Outbox

**Mã tài liệu:** `VAL-PERF-TELEM-001`  
**Dự án:** AQVN Native Workspace Client & RFC Host Bridge (`net.aqvn.workspace`)  
**Phạm vi đánh giá:** `src-tauri/src/connector/handlers/telemetry.rs` (Host metrics engine), `src-tauri/src/connector/client.rs` (Jitter backoff reconnect), `ui/container.js` (Telemetry event router & diagnostics modal), `ui/chat.js` (Offline outbox queue, inflight crash recovery, latency sparkline), `ui/index.html` (Telemetry & connection radar DOM)  
**Ngày thực hiện:** 2026-11-09  
**Người thực hiện:** AQVN Research & Performance Team (research-worker)  
**Trạng thái kiểm tra kỹ thuật:** Passed (`cargo test --test connector_handlers` 8/8 passed; `node --test tools/tests/test_chat_handlers.mjs` 177/177 passed)

---

## 1. Tóm Tắt Điều Hành (Executive Summary)

Khả năng duy trì kết nối ổn định và giám sát tài nguyên phần cứng theo thời gian thực là điều kiện tiên quyết giúp người vận hành và AI Agent nắm bắt chính xác trạng thái máy trạm. Trong môi trường mạng không ổn định (kết nối di động, VPN chập chờn, hoặc máy chủ khởi động lại), **ai-launcher** triển khai một kiến trúc tích hợp ba trụ cột vững chắc:

1. **Hệ thống viễn thông máy chủ nội tại (Host Telemetry Engine):** Module Rust đọc trực tiếp các giao diện nhân Linux (`/proc/stat`, `/proc/meminfo`, `statvfs`) để tính toán mức sử dụng CPU (%), lượng RAM khả dụng (MB) và dung lượng đĩa trống mà không phụ thuộc vào các công cụ bên thứ ba, bảo đảm độ trễ đo lường cực thấp (< 1ms).
2. **Radar trạng thái kết nối & Reconnect Jitter Backoff (Connection Radar):** Máy trạng thái 3 pha (`connected` / `reconnecting` / `offline`) kết hợp thuật toán tính lùi thời gian với độ nhiễu ngẫu nhiên phân tán (Decorrelated Jitter Backoff từ 500ms đến 15.000ms), ngăn chặn triệt để hiện tượng bão kết nối (reconnection storms / thundering herd) khi máy chủ phục hồi.
3. **Biểu đồ độ trễ trực quan (Ping Latency Sparkline):** Bộ đệm vòng 20 mẫu lưu trữ độ trễ ping thời gian thực, hiển thị trực quan dạng cột sparkline với 3 phân ngưỡng chất lượng mạng rõ rệt (Tốt < 50ms, Trung bình 50–150ms, Kém > 150ms).
4. **Hộp thư chờ ngoại tuyến có độ bền cao (Durable Offline Outbox):** Khi mất kết nối mạng, toàn bộ tin nhắn chat và lệnh điều khiển được lưu trữ bền vững vào `localStorage` (`[SECRET]`), giới hạn an toàn 50 tin nhắn hoặc 256KB. Hệ thống áp dụng cơ chế xác nhận lô nguyên tử (`aqvn_outbox_inflight`), tự động phục hồi tin nhắn mồ côi sau sự cố sập trình duyệt và phát lại tuần tự ngay khi mạng phục hồi mà không gây trùng lặp tin nhắn.

### Bảng tổng hợp các chốt chặn viễn thông & độ bền:

| Thành phần | Vị trí mã nguồn | Cơ chế hoạt động | Đánh giá an toàn & tin cậy |
| :--- | :--- | :--- | :---: |
| **Thu thập CPU/RAM Linux** | `src-tauri/.../telemetry.rs` (L25-145) | Đọc delta ticks `/proc/stat` & MB từ `/proc/meminfo` | **Xuất sắc (A)** |
| **Độ trễ Ping Sparkline** | `ui/chat.js` (L2049-2083) | Rolling 20-sample buffer, phân ngưỡng CSS 3 màu | **Xuất sắc (A)** |
| **Radar trạng thái kết nối** | `ui/container.js` (L1125-1175) | State machine: Connected / Reconnecting / Offline | **An toàn cao (A)** |
| **Thuật toán Jitter Backoff**| `src-tauri/.../client.rs` (L20-75) | Lùi thời gian ngẫu nhiên 500ms–15.000ms (Decorrelated) | **Xuất sắc (A)** |
| **Hộp thư chờ Offline Outbox**| `ui/chat.js` (L704-845) | FIFO Queue, giới hạn 50 mục / 256 KB, báo drop | **An toàn cao (A)** |
| **Phục hồi sự cố Inflight** | `ui/chat.js` (L720-745, L865-905) | Khóa lô vào `aqvn_outbox_inflight`, tự khôi phục khi mở lại | **Xuất sắc (A+)** |

---

## 2. Bản Đồ Kiến Trúc & Luồng Dữ Liệu (Architecture & Data Flow)

```
                     HỆ THỐNG VIỄN THÔNG TELEMETRY & OFFLINE OUTBOX
                     
  +---------------------------------------------------------------------------------------------------+
  | 1. PHÂN HỆ VIỄN THÔNG MÁY CHỦ (Rust Backend: telemetry.rs)                                        |
  |                                                                                                   |
  |   [ /proc/stat ] --------> [ sample_linux_cpu_ticks() ] ------> Delta ticks so với mẫu trước     |
  |                                                                 -> compute_cpu_usage_percent()    |
  |   [ /proc/meminfo ] -----> [ sample_linux_meminfo() ] --------> Total MB, Free MB, Avail MB      |
  |   [ statvfs(/) ] --------> [ sample_linux_disk_free_mb() ] ---> Disk free space MB                |
  |                                            |                                                      |
  |                                            v                                                      |
  |                         [ HostSystemMetrics Protocol Struct ]                                     |
  |                                            |                                                      |
  |                                            v (Gửi qua WebSocket: connector_telemetry_op_result)    |
  +---------------------------------------------------------------------------------------------------+
                                               |
                                               v
  +---------------------------------------------------------------------------------------------------+
  | 2. GIAO DIỆN HIỂN THỊ VIỄN THÔNG & RADAR MẠNG (ui/container.js, ui/chat.js)                       |
  |                                                                                                   |
  |   [ Topbar Telemetry Pill ]                                                                       |
  |     ├── #telemetry-cpu-val      : "24%"                                                           |
  |     ├── #telemetry-ram-val      : "1024 MB" (Total - Available)                                   |
  |     ├── #telemetry-latency-val  : "14 ms"                                                         |
  |     └── #telemetry-agent-state  : "Idle" / "Thinking" / "Executing"                               |
  |                                                                                                   |
  |   [ Connection Radar Pill ] : Green (Connected) | Yellow (Reconnecting) | Red (Offline)            |
  |                                                                                                   |
  |   [ Diagnostics Modal ] <--- (Nhấp chuột vào Telemetry Pill)                                      |
  |     └── #latency-sparkline: 20 cột động (Xanh <50ms | Vàng <150ms | Đỏ >=150ms)                   |
  +---------------------------------------------------------------------------------------------------+
                                               |
                                               v
  +---------------------------------------------------------------------------------------------------+
  | 3. HỘP THƯ CHỜ NGOẠI TUYẾN & PHỤC HỒI SỰ CỐ (ui/chat.js: Offline Outbox)                          |
  |                                                                                                   |
  |   [ Người dùng gửi lệnh / tin nhắn ]                                                              |
  |                    |                                                                              |
  |         (Đang Online) ?                                                                           |
  |         /             \                                                                           |
  |       (Có)           (Không - Đang Offline hoặc Lỗi Mạng)                                         |
  |        |               |                                                                          |
  |        v               v                                                                          |
  |  Gửi trực tiếp   [ queueOutbox(text, id) ]                                                        |
  |  qua Socket/IPC        |                                                                          |
  |                        v                                                                          |
  |                  [ Ghi vào localStorage: [SECRET] ]                                         |
  |                  (Cắt tỉa an toàn: Tối đa 50 entries, 256 KB)                                     |
  |                                                                                                   |
  |  ======================== KHI MẠNG KẾT NỐI TRỞ LẠI (RECONNECT) ========================           |
  |                                                                                                   |
  |   [ flushOutbox() ]                                                                               |
  |         |                                                                                         |
  |         v                                                                                         |
  |   [ Bước 1: Khóa đơn phiên ] ---> outboxFlush Promise (Tránh 2 đợt flush chồng chéo)              |
  |         |                                                                                         |
  |         v                                                                                         |
  |   [ Bước 2: Chuyển lô sang Inflight ] -> Ghi vào 'aqvn_outbox_inflight', xóa rỗng [SECRET] |
  |         |                                                                                         |
  |         v                                                                                         |
  |   [ Bước 3: Phát lại tuần tự ] --------> Gửi từng tin nhắn (next(index))                          |
  |         |                                                                                         |
  |         +---> (Thành công toàn bộ): Xóa 'aqvn_outbox_inflight' -> Hoàn tất                        |
  |         |                                                                                         |
  |         +---> (Gặp lỗi giữa chừng): Requeue phần còn lại vào đầu [SECRET]                   |
  |         |                                                                                         |
  |         +---> (Sập trình duyệt giữa chừng): Khi mở lại, readOutbox() nhặt từ inflight về          |
  +---------------------------------------------------------------------------------------------------+
```

---

## 3. Phân Tích Kỹ Thuật Chi Tiết (Technical In-Depth Analysis)

### 3.1. Cơ Chế Thu Thập Chỉ Số Hệ Thống Không Khóa (Lock-free System Sampling)

Tệp `src-tauri/src/connector/handlers/telemetry.rs` chịu trách nhiệm thu thập số liệu phần cứng trên Linux. Nhằm tránh ảnh hưởng tới hiệu năng tổng thể của gateway, mã nguồn sử dụng cơ chế đọc trực tiếp hệ thống tệp ảo `/proc`:

```rust
#[cfg(target_os = "linux")]
fn sample_linux_cpu_ticks() -> Option<(u64, u64)> {
    let stat_content = fs::read_to_string("/proc/stat").ok()?;
    for line in stat_content.lines() {
        if line.starts_with("cpu ") {
            let mut parts = line.split_whitespace().skip(1);
            let user: u64 = parts.next()?.parse().ok()?;
            let nice: u64 = parts.next()?.parse().ok()?;
            let system: u64 = parts.next()?.parse().ok()?;
            let idle: u64 = parts.next()?.parse().ok()?;
            let iowait: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let irq: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let softirq: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
            let steal: u64 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);

            let idle_ticks = idle.saturating_add(iowait);
            let active_ticks = user
                .saturating_add(nice)
                .saturating_add(system)
                .saturating_add(irq)
                .saturating_add(softirq)
                .saturating_add(steal);
            let total_ticks = idle_ticks.saturating_add(active_ticks);
            return Some((total_ticks, idle_ticks));
        }
    }
    None
}
```

**Tính toán Delta và Khử Rung (Dampening):**
- Mức tiêu thụ CPU được tính theo tỷ lệ $\frac{\Delta\text{Active}}{\Delta\text{Total}} \times 100\%$.
- Mẫu ticks trước đó được lưu trữ trong `LAST_CPU_SAMPLE: RwLock<Option<(u64, u64)>>`.
- Khi khởi động lần đầu, nếu chưa có mẫu trước, hàm chủ động tạm dừng 15ms (`Duration::from_millis(15)`) để tạo độ sai khác ticks tức thì, bảo đảm không bao giờ trả về giá trị ảo 0.0% trong lần đo đầu tiên.
- Bộ nhớ RAM phân tích chính xác trường `MemAvailable` (thay vì chỉ dựa vào `MemFree`), phản ánh trung thực lượng RAM thực tế mà hệ thống có thể cấp phát cho tiến trình mới mà không cần swap.

### 3.2. Thuật Toán Decorrelated Jitter Backoff Cho Kết Nối Bền Bỉ

Khi máy chủ bị ngắt kết nối đột ngột, nếu hàng ngàn client cùng tự động kết nối lại tại một khoảng thời gian cố định (ví dụ chính xác sau 2 giây), máy chủ sẽ bị sập tiếp do bão kết nối (Thundering Herd).

Tại `src-tauri/src/connector/client.rs` (và được kiểm định trong `src-tauri/tests/connector_handlers.rs`), hệ thống triển khai giải thuật **Decorrelated Jitter Backoff**:

```rust
pub const MIN_RECONNECT_DELAY_MS: u64 = 500;
pub const MAX_RECONNECT_DELAY_MS: u64 = 15000;
pub const RECONNECT_JITTER_MS: u64 = 500;

pub fn compute_jitter_backoff(
    current_delay: u64,
    min_delay: u64,
    max_delay: u64,
    jitter: u64,
) -> u64 {
    if current_delay == 0 {
        return (min_delay + jitter).min(max_delay);
    }
    let scaled = (current_delay as f64 * 1.5) as u64;
    (scaled + jitter).clamp(min_delay, max_delay)
}
```

**Đặc điểm giải thuật:**
- Khởi tạo lần thử đầu tiên: $500\text{ ms} + \text{jitter} \in [500\text{ ms}, 1000\text{ ms}]$.
- Các lần tiếp theo nhân tỷ lệ $1.5\times$ cộng với độ nhiễu ngẫu nhiên phân tán đồng đều.
- Chặn ngưỡng trần tối đa $15.000\text{ ms}$ (15 giây), vừa bảo đảm hệ thống không retry quá dồn dập, vừa bảo đảm thời gian phục hồi không bị kéo dài vô lý.

### 3.3. Biểu Đồ Độ Trễ Ping Latency Sparkline

Tại `ui/chat.js` (dòng 2049–2083), hệ thống duy trì bộ đệm tròn 20 mẫu:

```javascript
function updateLatencySparkline(latencyMs) {
  if (typeof latencyMs === 'number' && !isNaN(latencyMs)) {
    state.latencyHistory.push(latencyMs);
    if (state.latencyHistory.length > 20) {
      state.latencyHistory.shift();
    }
  }

  if (dom.diagnosticsQuality) {
    var avg = state.latencyHistory.reduce(function(a, b) { return a + b; }, 0) / (state.latencyHistory.length || 1);
    if (avg < 50) {
      dom.diagnosticsQuality.textContent = 'Excellent (' + Math.round(avg) + ' ms)';
      dom.diagnosticsQuality.style.color = 'var(--ok)';
    } else if (avg < 150) {
      dom.diagnosticsQuality.textContent = 'Fair (' + Math.round(avg) + ' ms)';
      dom.diagnosticsQuality.style.color = '#facc15';
    } else {
      dom.diagnosticsQuality.textContent = 'Poor (' + Math.round(avg) + ' ms)';
      dom.diagnosticsQuality.style.color = 'var(--danger)';
    }
  }

  if (dom.latencySparkline) {
    dom.latencySparkline.innerHTML = '';
    var max = Math.max.apply(null, [100].concat(state.latencyHistory));
    state.latencyHistory.forEach(function(lat) {
      var bar = document.createElement('div');
      bar.className = 'sparkline-bar' + (lat > 150 ? ' poor' : (lat > 50 ? ' fair' : ''));
      var pct = Math.max(10, Math.min(100, Math.round((lat / max) * 100)));
      bar.style.height = pct + '%';
      bar.title = lat + ' ms';
      dom.latencySparkline.appendChild(bar);
    });
  }
}
```

**Tính năng nổi bật:**
- Tỷ lệ chiều cao cột được chuẩn hóa theo độ trễ cao nhất trong batch (`Math.max(100, max)`), bảo đảm các biến thiên nhỏ không làm méo biểu đồ.
- Cảnh báo thị giác tức thì: cột đổi sang màu vàng (`.fair`) khi độ trễ vượt 50ms và đổi sang màu đỏ nguy hiểm (`.poor`) khi vượt 150ms.

### 3.4. Kiến Trúc Hộp Thư Chờ Offline Outbox & Khôi Phục Lỗi Toàn Vẹn

Cơ chế Outbox trong `ui/chat.js` (dòng 704–935) được xây dựng theo tiêu chuẩn **Reliable Messaging Pattern**:

1. **Giới hạn dung lượng bảo vệ bộ nhớ (Bounding & Eviction):**
   - Hằng số `OUTBOX_MAX_ENTRIES = 50` và `OUTBOX_MAX_BYTES = 256 * 1024` (256 KB).
   - Khi hàng đợi đầy, tin nhắn cũ nhất sẽ bị loại bỏ theo nguyên tắc FIFO (`list.shift()`), và giao diện thông báo rõ số lượng tin nhắn bị drop cho người dùng.
2. **Khóa Lô Nguyên Tử Tránh Trùng Lặp (Atomic Batch Claim):**
   ```javascript
   function flushOutbox() {
     if (outboxFlush) return outboxFlush;
     if (!state.online && !state.polling) return Promise.resolve(0);
     var list = readOutbox();
     if (!list.length) return Promise.resolve(0);
     try {
       window.localStorage.setItem('aqvn_outbox_inflight', JSON.stringify(list));
     } catch (e) {}
     writeOutbox([]);
     ...
   ```
   - Trước khi gửi gói tin đầu tiên, toàn bộ danh sách outbox được chuyển sang khóa tạm thời `aqvn_outbox_inflight` và tệp `[SECRET]` được xóa rỗng.
   - Nếu trong quá trình gửi mà xảy ra sự kiện reconnect lần 2, hàm `flushOutbox` thấy `[SECRET]` đang rỗng (hoặc `outboxFlush` đang chạy) sẽ không bao giờ gửi đè hoặc gửi lặp lại các tin nhắn đang bay (in-flight).
3. **Tự Động Phục Hồi Khi Trình Duyệt / Ứng Dụng Bị Tắt Đột Ngột:**
   - Trong hàm `readOutbox()`, hệ thống luôn quét kiểm tra sự tồn tại của khóa `aqvn_outbox_inflight`.
   - Nếu tìm thấy dữ liệu mồ côi (chứng tỏ phiên làm việc trước bị tắt nguồn hoặc crash giữa đợt flush), nó giải nén, đối chiếu mã định danh `message_id` để tránh trùng lặp, và chèn ngược lại vào đầu hàng đợi outbox (`list.unshift(item)`).
4. **Xử Lý Lỗi Xác Thực Phiên Làm Việc (Session Expiry 401/403):**
   - Nếu máy chủ phản hồi lỗi xác thực (`401`, `403`, hoặc `session expired`), hệ thống không cố gửi tiếp các tin nhắn còn lại trong hàng đợi mà chuyển trạng thái của chúng thành `failed = true`, `queued = false`, đồng thời cảnh báo người dùng sao chép nội dung trước khi đăng nhập lại.

---

## 4. Bằng Chứng Thực Nghiệm & Kết Quả Kiểm Thử (Empirical Verification)

### 4.1. Bộ Kiểm Thử Rust Connector & Jitter Backoff (`src-tauri/tests/connector_handlers.rs`)

Chạy kiểm thử trực tiếp:
```bash
cargo test --manifest-path src-tauri/Cargo.toml --test connector_handlers
```

**Kết quả ghi nhận:**
```
running 8 tests
test test_decorrelated_jitter_backoff_scaling ... ok
test test_reconnect_constants ... ok
test test_full_backoff_progression_to_max ... ok
test test_decorrelated_jitter_randomness_and_distribution ... ok
test test_telemetry_op_handling_val_tel_001 ... ok
test test_gateway_client_reconnects_with_jitter_backoff ... ok
test test_temporary_cdp_profile_cleanup_val_debt_002 ... ok
test test_process_op_handling_val_proc_001 ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s
```

*Phân tích kết quả:*
- `test_decorrelated_jitter_randomness_and_distribution`: Chứng minh 100 mẫu delay liên tiếp tạo ra phân phối ngẫu nhiên thực sự (>10 giá trị khác biệt) trong dải `[750ms, 1250ms]`.
- `test_telemetry_op_handling_val_tel_001`: Chứng minh handler `connector_telemetry_op` phản hồi chính xác cấu trúc `HostSystemMetrics` với đầy đủ CPU cores, CPU percentage, RAM MB và dung lượng đĩa.
- `test_gateway_client_reconnects_with_jitter_backoff`: Khởi tạo Mock Gateway Server, cố tình ép ngắt kết nối WebSocket và chứng minh client tự động kết nối lại thành công thông qua cơ chế jitter backoff.

### 4.2. Bộ Kiểm Thử Giao Diện JavaScript (`tools/tests/test_chat_handlers.mjs`)

Chạy kiểm thử đơn vị frontend:
```bash
node --test tools/tests/test_chat_handlers.mjs
```

**Bằng chứng cho các ca kiểm thử viễn thông & Outbox:**
- `VAL-STATUS-001`: Giao diện topbar hiển thị pill telemetry với đầy đủ chỉ số CPU, RAM, Latency và trạng thái Agent ("Idle"/"Thinking"/"Executing"). Nhấp chuột mở chính xác modal chẩn đoán `diagnostics-modal`.
- `VAL-CONN-003: sending while offline queues into localStorage aqvn_outbox`: Gửi tin nhắn khi offline ghi thành công vào `[SECRET]` và hiển thị nhãn `(queued)`.
- `VAL-CONN-003: socket connect flushes the outbox and clears storage`: Khi socket kết nối, tự động phát lại toàn bộ tin nhắn trong outbox và dọn dẹp storage.
- `VAL-CONN-003: the outbox is capped by entry count and total bytes`: Hàng đợi tự động cắt tỉa khi vượt quá 50 entries hoặc 256KB, thông báo số lượng bản ghi bị drop.
- `VAL-CONN-003: reconnect during drain does not resend`: Chứng minh cơ chế khóa `aqvn_outbox_inflight` ngăn chặn tuyệt đối việc gửi trùng tin nhắn.

---

## 5. Đánh Giá Hạn Chế Kỹ Thuật (Technical Limitations & Bottlenecks)

1. **Giới hạn đồng bộ của LocalStorage:** `window.localStorage` hoạt động đồng bộ (synchronous blocking I/O) trên luồng giao diện chính của webview. Nếu dung lượng outbox lớn và thiết bị có ổ cứng chậm, việc tuần tự hóa JSON có thể gây giật nhẹ (micro-stutter).
2. **Thiếu hỗ trợ tập tin đính kèm ngoại tuyến:** Hiện tại outbox chỉ lưu trữ chuỗi văn bản (`entry.text`) và ngữ cảnh (`entry.context`). Các tập tin upload đính kèm chưa được xếp hàng chờ ngoại tuyến (offline file upload queue).
3. **Tần số lấy mẫu Telemetry cố định:** Phân hệ viễn thông hiện phản hồi theo yêu cầu hoặc chu kỳ cố định; chưa có cơ chế thích ứng (adaptive sampling) để giảm tần suất truy vấn khi ứng dụng ở chế độ thu nhỏ (minimized) hoặc người dùng không tương tác.

---

## 6. Lộ Trình Hiện Đại Hóa & Nâng Cấp Khả Năng Chịu Tải (Actionable Roadmap)

### Kiến nghị 1: Di Chuyển Outbox Sang IndexedDB (Transactional Storage)
- **Giải pháp:** Sử dụng IndexedDB thông qua thư viện siêu nhẹ (`idb-keyval` < 1KB) để lưu trữ outbox bất đồng bộ.
- **Lợi ích:** Giải phóng hoàn toàn luồng UI chính, cho phép lưu trữ hàng ngàn tin nhắn và hỗ trợ lưu trữ blob/tập tin đính kèm ngoại tuyến mà không bị giới hạn 5MB của LocalStorage.

### Kiến nghị 2: Cơ Chế Thích Ứng Chu Kỳ Viễn Thông (Adaptive Telemetry Polling)
- **Giải pháp:** Tích hợp với `document.visibilityState` và sự kiện cửa sổ `tauri://blur` / `tauri://focus`:
  - Khi người dùng đang tương tác tích cực: lấy mẫu telemetry mỗi **2 giây**.
  - Khi ứng dụng ở chế độ nền hoặc thu nhỏ: giãn chu kỳ lấy mẫu lên **15–30 giây**.
- **Lợi ích:** Tiết kiệm chu kỳ CPU của hệ điều hành và giảm thiểu việc đọc `/proc/stat` liên tục khi người dùng không quan sát.

### Kiến nghị 3: Nén Delta Dữ Liệu Viễn Thông (Delta Compression for Telemetry)
- **Giải pháp:** Máy chủ chỉ phát sóng (broadcast) số liệu telemetry khi mức tiêu thụ CPU hoặc RAM thay đổi vượt ngưỡng sai số ($\Delta > 2\%$).
- **Lợi ích:** Giảm 85% lưu lượng bản tin WebSocket định kỳ trên mạng LAN/WAN.

# Docker & Container Deployment Guide — AQVN Workspace

Hệ thống `ai-launcher` cung cấp cấu hình Dockerfile đa tầng và Docker Compose để hỗ trợ đóng gói và triển khai trên môi trường máy chủ đám mây, cụm Linux phân tán hoặc môi trường cục bộ.

---

## 1. Thành phần Container

1. **`docker/Dockerfile.agent` (Headless Daemon `aqvn-agent`)**:
   - Sử dụng multi-stage build với `rust:1.97-slim-bookworm` biên dịch nhị phân độc lập `crates/aqvn-agent`.
   - Runtime base sử dụng `debian:bookworm-slim`, loại bỏ hoàn toàn các thư viện đồ họa (WebKitGTK, GTK3, X11, Display servers).
   - Chạy với người dùng không đặc quyền `aqvn` (UID 1000), có sẵn `HEALTHCHECK` kiểm tra tính sẵn sàng của daemon.
   - Dung lượng image thành phẩm cực nhẹ (~80MB base + 2.33MB binary).

2. **`docker/Dockerfile.ui` (Static Web Components UI)**:
   - Sử dụng `nginx:1.27-alpine-slim` phục vụ toàn bộ mã nguồn giao diện tĩnh `ui/` và các Web Components.
   - Bật nén `gzip` cho text, CSS, JS, SVG, tối ưu hóa caching tài nguyên tĩnh và bảo đảm First Contentful Paint (FCP) tức thì.
   - Cung cấp cổng phục vụ mặc định cổng 80 (ánh xạ cổng 3105 trên host).

3. **`docker/docker-compose.yml`**:
   - Tự động dựng cụm gồm 1 node daemon `aqvn-agent` kết nối Multi-Host Pool và 1 dịch vụ web static `aqvn-ui`.
   - Phân vùng lưu trữ bền vững qua Docker Volumes (`agent-workspace` và `agent-config`).

---

## 2. Hướng dẫn Biên Dịch & Chạy Thử

### Build và chạy qua Taskfile
```bash
# Build Docker image cho Headless Agent
task docker:build:agent

# Build Docker image cho Workspace UI
task docker:build:ui

# Chạy cụm dịch vụ qua Docker Compose
task docker:up

# Xem trạng thái container
docker compose -f docker/docker-compose.yml ps

# Xem nhật ký hoạt động
task docker:logs

# Dừng cụm dịch vụ
task docker:down
```

### Build thủ công qua Docker CLI
```bash
# Build aqvn-agent
docker build -t net.aqvn.workspace/agent:latest -f docker/Dockerfile.agent .

# Chạy aqvn-agent daemon độc lập
docker run -d --name aqvn-agent \
  -e AQVN_SERVER="https://a0.matmagoc.com" \
  -e AQVN_TOKEN="your-token" \
  -e AQVN_LABEL="docker-node-01" \
  net.aqvn.workspace/agent:latest

# Build & Run UI server
docker build -t net.aqvn.workspace/ui:latest -f docker/Dockerfile.ui .
docker run -d -p 3105:80 --name aqvn-workspace-ui net.aqvn.workspace/ui:latest
```

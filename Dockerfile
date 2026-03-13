# Stage 1: Build Frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Stage 2: Build Backend
FROM rust:1.85-bookworm AS backend-builder
WORKDIR /app
# 複製後端目錄
COPY src-tauri ./src-tauri
WORKDIR /app/src-tauri
# 為了避免 tauri 相關路徑問題，之後我們會將 Cargo.toml 中的 tauri 移除並改為 axum
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
# 安裝必要的相依套件 (ffmpeg 等，因原程式可能有呼叫 ffprobe/ffmpeg)
RUN apt-get update && apt-get install -y ffmpeg libsqlite3-dev ca-certificates tzdata && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 複製編譯完成的前端與後端
COPY --from=frontend-builder /app/dist /app/dist
COPY --from=backend-builder /app/src-tauri/target/release/stickplay /app/stickplay

# 建立供掛載的目錄
RUN mkdir -p /media /config

# 設定環境變數
ENV STICKPLAY_FRONTEND_DIR=/app/dist
ENV STICKPLAY_CONFIG_DIR=/config
ENV STICKPLAY_MEDIA_DIR=/media
ENV PUID=1000
ENV PGID=1000

# 曝露 port 8099
EXPOSE 8099

CMD ["/app/stickplay"]

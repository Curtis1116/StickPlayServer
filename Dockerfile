# 第一階段：編譯前端 (Node.js)
FROM node:20-alpine AS frontend-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# 第二階段：編譯後端 (Rust)
FROM rust:1.85-bookworm AS backend-builder
WORKDIR /app
# 安裝編譯所需的系統依賴 (openssl-sys, sqlite3-sys 等需使用)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# 複製後端源碼
COPY src-tauri ./src-tauri
WORKDIR /app/src-tauri
# 執行編譯 (核心為 Axum 伺服器)
RUN cargo build --release

# 第三階段：執行環境 (Runtime)
FROM debian:bookworm-slim
# 安裝必要的運行時依賴 (ffmpeg 是掃描影片資訊所需)
RUN apt-get update && apt-get install -y \
    ffmpeg \
    libsqlite3-0 \
    ca-certificates \
    tzdata \
    openssl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 從前面的階段複製編譯產物
COPY --from=frontend-builder /app/dist /app/dist
COPY --from=backend-builder /app/src-tauri/target/release/stickplay-server /app/stickplay

# 建立外部掛載點：/media (影片) 與 /config (資料庫與設定)
RUN mkdir -p /media /config

# 設定應用程式環境變數
ENV STICKPLAY_FRONTEND_DIR=/app/dist
ENV STICKPLAY_CONFIG_DIR=/config
ENV STICKPLAY_MEDIA_DIR=/media
ENV PORT=8099

# 預設曝露連接埠
EXPOSE 8099

# 啟動伺服器
CMD ["/app/stickplay"]

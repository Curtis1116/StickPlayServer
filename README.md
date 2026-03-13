# StickPlay Server 影片管理資料庫 (Docker 版本)

StickPlay Server 是一款基於 Docker 的現代化影片管理解決方案。它將原有的 StickPlay 桌面端功能轉化為高效能的 Web 伺服器架構，讓您可以在 NAS (如 Synology) 或家用伺服器上部署，並透過任何裝置的瀏覽器進行存取。

![StickPlay Icon](./public/icon.svg)

## ✨ 核心特色

-   **Web 化存取**：不再侷限於單機，透過瀏覽器即可在手機、平板或電腦上管理您的影片庫。
-   **現代化介面**：延續 macOS 原生美學的深色毛玻璃 (Glassmorphism) 主題，支援流暢的動畫與響應式格狀佈局。
-   **智慧監控與自動更新**：後端整合檔案系統監控 (Notify)，當影片資料夾有變動時，系統會自動在背景進行重整。
-   **增強型 NFO 管理**：
    -   **優先權**：掃描時優先讀取 `.nfos` 檔案，確保手動修正的資料（如評分、演員、分級）被優先採用。
    -   **分離儲存**：所有的修改都會寫入專屬的 `.nfos` 檔案，完美保護原始 `.nfo` 資料夾的完整性。
-   **智慧海報選擇**：優先採用檔名為 `stick_poster.jpg` 的圖片作為封面，並具備人像偵測自動裁切功能（2:3 比例）。
-   **多裝置設定同步**：媒體庫設定存儲於伺服器端，無論從哪個瀏覽器登入，都能享有一致的媒體庫路徑與設定。
-   **Docker 優化**：
    -   為 Synology 與 Linux 伺服器優化，支援跨平台目錄掛載。
    -   檔案選擇器限制於 `/media` 路徑下，防止誤選系統目錄。

## 🛠️ 技術棧

-   **前端 (Frontend)**：React 19 + TypeScript / Vite / Tailwind CSS / Lucide React
-   **後端 (Backend)**：Rust (Axum Web Framework)
-   **資料庫**：SQLite (Rusqlite) + WAL 模式提升並行效能
-   **檔案監控**：Notify (Rust)
-   **影像處理**：Image (海報生成與裁切)

## 📦 部署指南 (Docker)

### 使用 Docker Compose (推薦)

您可以直接使用 `docker-compose.yml` 快速啟動：

```yaml
version: '3.8'
services:
  stickplay:
    image: stickplay:latest
    container_name: stickplay
    restart: always
    ports:
      - "8099:8099"
    volumes:
      - ./config:/config       # 儲存資料庫與設定檔
      - /path/to/your/video:/media # 您的影片資料夾
    environment:
      - TZ=Asia/Taipei
      - STICKPLAY_CONFIG_DIR=/config
      - STICKPLAY_MEDIA_DIR=/media
```

### Synology NAS 安裝建議

1.  將專案資料夾上傳至 `File Station`。
2.  開啟 **Container Manager**，新增專案。
3.  匯入 `docker-compose.yml` 並視需求修改 `volumes` 路徑。
4.  啟動後即可透過 `http://NAS_IP:8099` 訪問。

## 🛠️ 開發說明

如果您需要自行編譯：

```bash
# 1. 確保已安裝 Docker
# 2. 在根目錄建置 Image
docker build -t stickplay:latest .

# 3. 匯出 Image (供 NAS 使用)
docker save stickplay:latest -o stickplay_image.tar
```

## 📄 授權條款

本專案採用 [MIT License](LICENSE) 授權 - Copyright (c) 2026 huachun

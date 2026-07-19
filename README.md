# StickPlayServer 影片管理資料庫 (Docker 版本)

StickPlayServer 是一款基於 Docker 的現代化影片管理解決方案。它將原有的 StickPlay 桌面端功能轉化為高效能的 Web 伺服器架構，讓您可以在 NAS (如 Synology) 或家用伺服器上部署，並透過 any 裝置的瀏覽器進行存取。

![StickPlay Icon](./public/icon.svg)

## ✨ 核心特色

-   **Web 化存取**：不再侷限於單機，透過瀏覽器即可在手機、平板或電腦上管理您的影片庫。
-   **現代化介面**：延續 macOS 原生美學的深色毛玻璃 (Glassmorphism) 主題，支援流暢的動畫與響應式格狀佈局。
-   **智慧監控與自動更新**：後端整合檔案系統監控 (Notify)，當影片資料夾有變動時，系統會自動在背景進行重整。
-   **增強型 NFO 管理**：
    -   **無損修改**：系統支援在網頁或 App 端修改影片資訊（如評分、演員、分級等）。
    -   **手術式更新 (Surgical Update)**：所有的修改都會直接回寫至原始的 `.nfo` 檔案。系統採用精準的 XML 節點替換技術，完美保留原本不受管理的標籤（如 `<fileinfo>` 等影片編碼資訊），確保不破壞原始檔案結構。
-   **智慧海報選擇**：系統會自動尋找資料夾內的 `poster.jpg` 作為封面，若不存在則會挑選比例最合適（2:3）的圖片，或提供人像偵測自動裁切功能。
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
  stickplayserver:
    image: stickplay-server:latest
    container_name: stickplay-server
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

## 🎬 使用本機播放器開啟影片

設定頁可選擇點擊播放時要使用的播放器，清單會依裝置自動調整：

-   **Windows / 桌面瀏覽器**：瀏覽器 / PotPlayer / VLC
-   **iPhone / iPad（Safari 或 Chrome）**：瀏覽器 / VLC / Infuse

在 iOS/iPadOS 上選擇 VLC 或 Infuse **不需要任何額外設定**，只要裝置上已安裝該 App，網頁會直接透過官方支援的網址格式呼叫它開啟串流。

在 Windows 上選擇 PotPlayer 或 VLC，因為瀏覽器本身無法直接啟動本機執行檔，需要**先完成一次性的通訊協定註冊**，之後每次點擊播放才會自動喚起對應的播放器：

1.  開啟 `tools/external-players/` 資料夾。
2.  對照你要使用的播放器，雙擊匯入 `stickplay-potplayer.reg` 和/或 `stickplay-vlc.reg`（僅寫入目前使用者的登錄檔，不需要系統管理員權限）。
3.  若 PotPlayer / VLC 並非安裝在標準路徑（`C:\Program Files\DAUM\PotPlayer` 或 `C:\Program Files\VideoLAN\VLC`），請編輯 `tools/external-players/launch-player.ps1` 內的候選路徑清單。
4.  若專案資料夾搬移過位置，`.reg` 檔內指向 `launch-player.ps1` 的絕對路徑也要一併更新。

設定完成後，每次點擊播放時瀏覽器仍會跳出一次「是否允許開啟外部應用程式」的確認視窗，這是瀏覽器原生的安全機制，可勾選「一律允許」關閉提示。若清單中的播放器實際上未安裝在該裝置，點擊播放將不會有反應。

## 🛠️ 開發說明

如果您需要自行編譯：

```bash
# 1. 確保已安裝 Docker
# 2. 在根目錄建置 Image
docker build -t stickplay-server:latest .

# 3. 匯出 Image (供 NAS 使用)
docker save stickplay-server:latest -o stickplay-server.tar
```

## 📄 授權條款

本專案採用 [MIT License](LICENSE) 授權 - Copyright (c) 2026 huachun

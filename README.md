# StickPlayServer 影片管理資料庫 (Docker 版本)

StickPlayServer 是一款基於 Docker 的現代化影片管理解決方案。它將原有的 StickPlay 桌面端功能轉化為高效能的 Web 伺服器架構，讓您可以在 NAS (如 Synology) 或家用伺服器上部署，並透過 any 裝置的瀏覽器進行存取。

![StickPlay Icon](./public/icon.svg)

## ✨ 核心特色

-   **Web 化存取**：不再侷限於單機，透過瀏覽器即可在手機、平板或電腦上管理您的影片庫。
-   **現代化介面**：延續 macOS 原生美學的深色毛玻璃 (Glassmorphism) 主題，支援流暢的動畫與響應式格狀佈局。
-   **智慧監控與自動更新**：後端整合目錄輪詢，當影片資料夾有變動時，系統會自動在背景進行重整。
-   **增強型 NFO 管理**：
    -   **無損修改**：系統支援在網頁修改影片資訊（如評分、演員、分級等）。
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
-   **檔案監控**：遞迴目錄輪詢 (Rust)
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
      - "127.0.0.1:8099:8099"
    volumes:
      - ./config:/config       # 儲存資料庫與設定檔
      - /path/to/your/video:/media # 您的影片資料夾
    environment:
      - TZ=Asia/Taipei
      - STICKPLAY_CONFIG_DIR=/config
      - STICKPLAY_MEDIA_DIR=/media
      - STICKPLAY_PUBLIC_ORIGIN=https://stickplay.example.com
```

### Synology NAS 安裝建議

1.  在 **Container Manager** 匯入新版 `stickplay-server.tar`。
2.  保留原容器的連接埠與 `/config`、`/media` 掛載，讓容器改用 `stickplay-server:latest` 後重新建立。
3.  設定 `STICKPLAY_PUBLIC_ORIGIN`；使用 HTTP 時另設 `STICKPLAY_ALLOW_INSECURE_HTTP=true`。
4.  從容器日誌取得首次設定碼並建立管理員。

## 管理員登入與首次設定

目前支援純 Web、單一管理員。手機、平板與電腦均使用網頁；`StickPlayApp/` 為歷史程式碼，不再支援新伺服器的驗證流程。

1. 第一次啟動後，從 Container Manager 的容器日誌或 `docker logs stickplay-server` 取得一次性設定碼。
2. 開啟網站，輸入設定碼、管理員帳號（預填 `admin`）、自訂密碼及確認密碼。沒有預設密碼，密碼至少 12 位元組、最多 256 位元組。
3. 按「建立並開始使用」後直接登入；設定碼立即失效，首次設定入口關閉。
4. 設定碼有效 30 分鐘。未完成初始化時可重啟容器取得新碼，或執行下方的伺服器端指令重新產生。

「記住這台裝置」預設勾選：90 天未使用才失效，持續使用最長一年；未勾選時使用瀏覽器工作階段 Cookie，伺服器最長允許 12 小時。部分瀏覽器的還原分頁功能會還原工作階段 Cookie，共用裝置使用完請主動登出。

登入狀態與帳號儲存在 `/config/auth.db`，保留整個 `/config` 後，切換媒體庫、NAS 重啟或容器更新均不影響登入。瀏覽器使用 `HttpOnly`、`Secure`、`SameSite=Lax` Cookie，不在 localStorage 保存密碼或 Session。設定頁可登出目前／指定／其他所有裝置；變更密碼需要目前密碼，完成後撤銷舊憑證並保留目前裝置的新登入。

忘記密碼時，由 NAS 管理員執行：

```bash
docker exec stickplay-server /app/stickplay --auth-recovery
```

將輸出的 30 分鐘一次性碼填入登入頁的「忘記密碼？」表單。復原成功後所有舊登入與播放連結失效，影片、索引及收藏保留。復原碼只能在伺服器端產生。

### HTTPS、反向代理與本機開發

正式使用請設定 `STICKPLAY_PUBLIC_ORIGIN=https://你的網域`，Web 與 API 使用同一個來源。伺服器直接比對 `Origin`，不信任用戶端可偽造的 `X-Forwarded-*` 標頭；代理需保留原始 `Origin`、Cookie，並關閉 SSE 路徑 `/api/events` 的回應緩衝。不要替 `/api/` 啟用共享快取；播放連結有短期權限，建議代理存取日誌不記錄查詢字串。

若反向代理位於另一台主機，請配合 NAS 防火牆僅允許該代理存取後端連接埠，並調整上方的 loopback 綁定。伺服器不自行終止 TLS。

僅在信任的 LAN／本機開發需要 HTTP 時，明確設定以下兩項；此模式的 Cookie 不會有 `Secure`，傳輸內容也未加密：

```bash
STICKPLAY_PUBLIC_ORIGIN=http://localhost:1420
STICKPLAY_ALLOW_INSECURE_HTTP=true
```

`npm run dev` 的 Vite 已將 `/api` 代理到 `127.0.0.1:8099`。後端從專案根目錄啟動時，另設 `STICKPLAY_FRONTEND_DIR=./dist`，媒體目錄需使用絕對路徑。未設定來源時預設為 `https://localhost`，請在部署前改成實際網址。

### 媒體庫與檔案權限

每個媒體請求必須攜帶 `X-Library-Id`（圖片、影片及 SSE 使用 `libraryId` 查詢參數）；各裝置切換媒體庫不會更改其他裝置的操作目標。所有管理 API、圖片、影片與 SSE 均需要有效登入。

`STICKPLAY_MEDIA_DIR` 定義唯一的媒體根目錄；媒體讀寫還會限制於已儲存的該媒體庫路徑。拒絕 `..`、符號連結越界、設定目錄及不符用途的副檔名。升級後，請把原先不在此根目錄內的路徑掛載到 `/media` 之下，再於設定頁更新。`/config` 應與媒體目錄分開掛載。

刪除媒體庫需要確認，伺服器先以 SQLite snapshot 備份至 `/config/backups/`，成功後才從清單移除；影片檔案與原資料庫保留。若要復原，先停止容器，以備份覆蓋同名資料庫、清除該資料庫的舊 WAL／SHM，再將原本 `id`、`db_name`、`paths` 加回 `libraries.json` 並重啟。請勿修改 `auth.db` 來切換媒體庫。

## 🎬 使用本機播放器開啟影片

瀏覽器播放沿用登入 Cookie；外部播放器使用僅能讀取指定影片的限時連結，有效 6 小時，可重複請求 Range 以拖曳進度。登出或撤銷該裝置後，後續請求失效；已開始傳輸的回應不會中途切斷。連結到期後請回到網頁再次播放。

設定頁可選擇點擊播放時要使用的播放器，清單會依裝置自動調整：

-   **Windows**：瀏覽器 / PotPlayer / VLC
-   **macOS**：瀏覽器 / VLC / Infuse
-   **iPhone / iPad**：瀏覽器 / Infuse / VLC
-   **Android**：瀏覽器 / VLC / Just Player

在 iOS/iPadOS 上選擇 VLC 或 Infuse **不需要任何額外設定**，只要裝置上已安裝該 App，網頁會直接透過官方支援的網址格式呼叫它開啟串流。

Android 建議使用 **Just Player**；它專注於本機與網路影片播放，支援常見影音格式，且不含廣告或追蹤。VLC 仍保留為功能較完整的通用選項。Android 會透過 Chrome Intent 指定開啟所選 App；若 App 未安裝，則回到瀏覽器播放。

Windows 或 macOS 的設定頁會依目前裝置顯示播放器工具下載按鈕。下載需要先登入；ZIP 安裝包已直接編入 Server，不包含帳號、Cookie、播放憑證或伺服器網址。

-   **Windows**：下載並解壓縮後執行 `install.cmd`。工具會複製到 `%LOCALAPPDATA%\StickPlay\PlayerBridge`，並為目前 Windows 帳號註冊 PotPlayer 與 VLC，不需要系統管理員權限。執行同一個 ZIP 內的 `uninstall.cmd` 可完整移除。
-   **macOS**：先安裝 VLC，下載並解壓縮後執行 `Install StickPlay VLC.command`。工具會在 `~/Applications` 建立輕量啟動 App 並註冊 `stickplay-vlc:`，不需要系統管理員權限。若 Gatekeeper 阻擋，請在 Finder 對該檔案按右鍵並選擇「打開」。同一個 ZIP 內提供解除安裝工具。

macOS 的 VLC 工具會為每次點擊的影片開啟獨立 VLC 視窗，讓多部影片同時播放。已安裝舊版工具的使用者，請在更新伺服器後重新下載並執行安裝工具，以套用此行為。

macOS 上的 Infuse 安裝後即可直接使用，不需要下載橋接工具。

設定完成後，每次點擊播放時瀏覽器仍會跳出一次「是否允許開啟外部應用程式」的確認視窗，這是瀏覽器原生的安全機制，可勾選「一律允許」關閉提示。若清單中的播放器實際上未安裝在該裝置，點擊播放將不會有反應。

## 🛠️ 開發說明

如果您需要自行編譯：

```bash
# 1. 確保已安裝 Docker
# 2. 在根目錄建置 Image
docker build --platform linux/amd64 -t stickplay-server:latest .

# 3. 匯出 Image (供 NAS 使用)
docker save stickplay-server:latest -o stickplay-server.tar
```

## 📄 授權條款

本專案採用 [MIT License](LICENSE) 授權 - Copyright (c) 2026 huachun

## 驗證修改

```bash
npm run build
node --experimental-strip-types tests/player-options.test.ts
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml --bin stickplay-server
python3 -m unittest discover -s tests -p '*_integration.py' -v
```

整合測試使用臨時目錄與本機臨時連接埠，不會讀寫現有 `/config` 或媒體庫。

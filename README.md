# StickPlay 影片管理資料庫

StickPlay 是一款跨平台的現代化本地影片管理應用程式。採用書架式的視覺介面，結合智慧海報裁切與強大的中繼資料（Metadata）解析能力，為您提供頂級的影片收藏與瀏覽體驗。

![StickPlay Icon](./public/icon.svg)

## ✨ 核心特色

- **現代化介面**：採用 macOS 原生美學的深色毛玻璃 (Glassmorphism) 主題，流暢的動畫與響應式格狀佈局。
- **增強型影片編輯對話框**：全新設計的互動式編輯介面，支援評分、分級 (Level)、演員、日期等完整編輯，並優化欄位佈局以避免出現捲軸。
- **智慧海報裁切**：當原始圖片比例不符時，系統會自動偵測人像並裁切出完美的 2:3 比例海報。
- **彈性中繼資料 (NFO)**：
  - 支援雙重 NFO 檔案架構。讀取時優先載入 `.nfos`，若無則讀取原始唯讀的 `.nfo` 檔。
  - 對評分與 ID 修改會獨立寫回專屬的 `.nfos`，完美保護您的原始中繼資料不被覆寫。
- **智慧解析與 ID 擷取**：
  - 支援各種路徑前綴與包含數字的品牌代碼（如 `300MIUM`, `259LUXU`）。
  - 精確識別資料夾括號內的演員與分級 (Level) 資訊。
- **單一影片更新**：卡片提供獨立「重新整理」按鈕，編輯面板更整合了「開啟資料夾」捷徑，管理更直覺。

## 🛠️ 技術棧

- **前端 (Frontend)**：
  - [React 18](https://reactjs.org/) + [TypeScript](https://www.typescriptlang.org/)
  - [Vite](https://vitejs.dev/)
  - [Tailwind CSS v3](https://tailwindcss.com/)
  - [Lucide React](https://lucide.dev/) (向量圖示)
- **後端 (Backend)**：
  - [Tauri v2](https://v2.tauri.app/)
  - [Rust](https://www.rust-lang.org/)
  - **Rusqlite** (SQLite 儲存與快取)
  - **Quick-XML** (NFO 檔案微秒級解析)
  - **Image** (影像處理與生成)

## 📦 安裝與啟動

### 環境要求
- Node.js (建議 v18 以上)
- Rust (可透過 [rustup](https://rustup.rs/) 安裝)
- Tauri 的作業系統相依套件 (請參閱 [Tauri 官方指南](https://v2.tauri.app/start/prerequisites/))

### 開發模式
```bash
# 1. 安裝前端依賴
npm install

# 2. 啟動 Tauri 開發環境
npm run tauri dev
```

### 打包應用程式
```bash
# 生成您的作業系統的安裝檔 (如 .dmg, .msi, .AppImage)
npm run tauri build
```

## 📖 使用指南

1. **新增路徑**：初次使用請點擊畫面右上角的「設定 (⚙️)」圖示，新增包含影片的根資料夾。
2. **掃描索引**：回到主畫面，點擊工具列的「重新整理 (🔄)」，系統將自動掃描並建立影片庫。
3. **播放觀看**：雙擊任一影片卡片，或點擊懸停選項中的「播放影片」，即會使用作業系統預設的播放器開啟該影片。
4. **精確控制**：透過頂部工具列可自由縮放書架尺寸、過濾廠牌/等級、或是透過右上角下拉選單調整排序方式。

## 📄 授權條款

本專案採用 [MIT License](LICENSE) 授權 - Copyright (c) 2026 huachun

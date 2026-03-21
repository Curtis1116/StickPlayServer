# StickPlayApp (iOS/iPadOS)

這個專案是依據 `StickPlayServer` 提供的 API 所開發的 SwiftUI 影片播放客戶端。

## 系統需求
- iOS 16.0 或以上
- Xcode 15 或以上
- 已安裝 [Infuse](https://firecore.com/infuse) App 以提供影片播放功能
- [xcodegen](https://github.com/yonaskolb/XcodeGen) (用來生成 Xcode 專案)

## 如何啟動專案
由於環境限制，我們採用 `xcodegen` 來自動生成 `.xcodeproj`，以確保 iOS 的 App Scheme 與 Info.plist 動態更新。

1. 安裝 xcodegen (如果您尚未安裝)：
   ```bash
   brew install xcodegen
   ```
2. 在此目錄 (`StickPlayApp`) 執行 xcodegen：
   ```bash
   xcodegen
   ```
3. 開啟生成的 `StickPlayApp.xcodeproj`：
   ```bash
   open StickPlayApp.xcodeproj
   ```
4. 選擇您的 iPhone/iPad 模擬器或實機，點擊左上角的 **Run** (或 `Cmd+R`) 進行編譯與執行。

## 功能與使用流程
1. **設定 Server IP**：初次開啟 App，請點選右上角齒輪設定 `StickPlayServer` 的 IP 及埠號（預設為 8000）。
2. **影片列表**：App 會自動拉取並以毛玻璃現代網格 UI 展示由伺服器提供的影片資源。
3. **影片播放**：點選任一影片海報將觸發 Infuse URL Deep Link。如果裝置中沒有安裝 Infuse，App 將會彈窗提示且不會中斷。

## 授權
Copyright (c) 2026 huachun
Licensed under the MIT License.

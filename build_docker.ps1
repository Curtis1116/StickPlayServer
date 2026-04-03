# StickPlayServer Docker 建置腳本 (Windows PowerShell)

Write-Host "--- 開始建置 StickPlayServer Docker 映像檔 ---" -ForegroundColor Cyan

# 檢查 Docker 是否正在執行
docker info | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker 未啟動，或是當前系統無法連接 Docker Engine。請先啟動 Docker Desktop。"
    exit 1
}

# 執行建置：使用 Docker Compose 進行編譯
Write-Host "正在編譯前、後端並封裝映像檔 (這可能需要幾分鐘)..." -ForegroundColor Yellow
docker compose build

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n[成功] 映像檔 stickplay-server:latest 已建置完成。" -ForegroundColor Green
    Write-Host "您可以使用 'docker compose up -d' 啟動服務。" -ForegroundColor Cyan
} else {
    Write-Error "建置失敗，請檢查上方錯誤訊息。"
    exit 1
}

# StickPlayServer Docker 映像建置腳本 (Windows PowerShell)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$tag = "stickplay-server:latest"
$output = Join-Path $root "stickplay-server.tar"

Write-Host "--- 開始建置 Synology x86_64 Docker 映像檔 ---" -ForegroundColor Cyan

docker info | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker 未啟動，或是當前系統無法連接 Docker Engine。請先啟動 Docker Desktop。"
    exit 1
}

docker build --platform linux/amd64 -t $tag $root
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker 映像建置失敗。"
    exit 1
}

$actual = docker image inspect $tag --format '{{.Os}}/{{.Architecture}}'
if ($LASTEXITCODE -ne 0 -or $actual -ne "linux/amd64") {
    Write-Error "映像架構不符：$actual"
    exit 1
}

docker save $tag -o $output
if ($LASTEXITCODE -ne 0) {
    Write-Error "Docker 映像匯出失敗。"
    exit 1
}

Write-Host "`n[成功] 已產生 $output" -ForegroundColor Green
Write-Host "映像名稱：$tag；架構：$actual" -ForegroundColor Cyan

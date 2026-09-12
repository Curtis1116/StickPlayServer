$ErrorActionPreference = "Stop"

foreach ($scheme in @("stickplay-potplayer", "stickplay-vlc")) {
    $protocolRoot = "HKCU:\Software\Classes\$scheme"
    if (Test-Path -LiteralPath $protocolRoot) {
        Remove-Item -LiteralPath $protocolRoot -Recurse -Force
    }
}

$installDirectory = Join-Path $env:LOCALAPPDATA "StickPlay\PlayerBridge"
if (Test-Path -LiteralPath $installDirectory) {
    Remove-Item -LiteralPath $installDirectory -Recurse -Force
}

Write-Host "StickPlay player tools were removed from this Windows account."

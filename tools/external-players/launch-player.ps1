param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("potplayer", "vlc")]
    [string]$Player,

    [Parameter(Mandatory = $true)]
    [string]$Uri
)

$prefix = "stickplay-${Player}:"
$encoded = ($Uri -replace [regex]::Escape($prefix), "").TrimEnd("/")
$videoUrl = [System.Uri]::UnescapeDataString($encoded)

$candidates = if ($Player -eq "potplayer") {
    @(
        "$env:ProgramFiles\DAUM\PotPlayer\PotPlayerMini64.exe",
        "${env:ProgramFiles(x86)}\DAUM\PotPlayer\PotPlayerMini64.exe"
    )
} else {
    @(
        "$env:ProgramFiles\VideoLAN\VLC\vlc.exe",
        "${env:ProgramFiles(x86)}\VideoLAN\VLC\vlc.exe"
    )
}

$exe = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1

if (-not $exe) {
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show(
        "找不到 $Player 執行檔，請確認已安裝，或修改本腳本內的路徑清單。",
        "StickPlay 播放器啟動失敗"
    ) | Out-Null
    exit 1
}

Start-Process -FilePath $exe -ArgumentList $videoUrl

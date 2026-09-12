$ErrorActionPreference = "Stop"

$sourceLauncher = Join-Path $PSScriptRoot "launch-player.ps1"
if (-not (Test-Path -LiteralPath $sourceLauncher)) {
    throw "launch-player.ps1 is missing from the downloaded package."
}

$installDirectory = Join-Path $env:LOCALAPPDATA "StickPlay\PlayerBridge"
$installedLauncher = Join-Path $installDirectory "launch-player.ps1"
New-Item -ItemType Directory -Path $installDirectory -Force | Out-Null
Copy-Item -LiteralPath $sourceLauncher -Destination $installedLauncher -Force

$players = @(
    @{ Scheme = "stickplay-potplayer"; Player = "potplayer"; Name = "StickPlay PotPlayer" },
    @{ Scheme = "stickplay-vlc"; Player = "vlc"; Name = "StickPlay VLC" }
)

foreach ($entry in $players) {
    $protocolRoot = "HKCU:\Software\Classes\$($entry.Scheme)"
    $commandKey = Join-Path $protocolRoot "shell\open\command"
    New-Item -Path $commandKey -Force | Out-Null
    Set-Item -Path $protocolRoot -Value "URL:$($entry.Name) Protocol"
    New-ItemProperty -Path $protocolRoot -Name "URL Protocol" -Value "" -PropertyType String -Force | Out-Null
    $command = 'powershell.exe -NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File "{0}" -Player {1} -Uri "%1"' -f $installedLauncher, $entry.Player
    Set-Item -Path $commandKey -Value $command
}

Write-Host "StickPlay player tools were installed for this Windows account."
Write-Host "Installed launcher: $installedLauncher"

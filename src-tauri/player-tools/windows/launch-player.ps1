param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("potplayer", "vlc")]
    [string]$Player,

    [Parameter(Mandatory = $true)]
    [string]$Uri
)

$prefix = "stickplay-${Player}:"
if (-not $Uri.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    exit 1
}

$encoded = $Uri.Substring($prefix.Length).TrimEnd("/")
try {
    $videoUrl = [System.Uri]::UnescapeDataString($encoded)
    $parsedUrl = [System.Uri]$videoUrl
} catch {
    exit 1
}
if (-not $parsedUrl.IsAbsoluteUri -or $parsedUrl.Scheme -notin @("http", "https")) {
    exit 1
}

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

$executable = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $executable) {
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show(
        "The selected player is not installed in a standard location.",
        "StickPlay player launch failed"
    ) | Out-Null
    exit 1
}

Start-Process -FilePath $executable -ArgumentList $videoUrl

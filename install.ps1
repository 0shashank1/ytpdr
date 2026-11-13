<#
.SYNOPSIS
    Automated installer script for ytpdr on Windows.
.DESCRIPTION
    Checks for and installs dependencies (ffmpeg, yt-dlp via winget),
    builds and installs ytpdr via cargo, and ensures ~/.cargo/bin is in PATH.
#>

$ErrorActionPreference = "Stop"

Write-Host "=== YTPDR Windows Installer ===" -ForegroundColor Cyan

# 1. Check for Rust / Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Warning "Cargo/Rust is not detected on your PATH."
    Write-Host "Please install Rust from https://rustup.rs and re-run this script." -ForegroundColor Yellow
    exit 1
}

# 2. Check for yt-dlp
if (-not (Get-Command yt-dlp -ErrorAction SilentlyContinue)) {
    Write-Host "yt-dlp not found on PATH. Attempting to install via winget..." -ForegroundColor Yellow
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        winget install --id yt-dlp.yt-dlp -e --accept-package-agreements --accept-source-agreements
        # Refresh current session PATH
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
    } else {
        Write-Warning "winget is not available. Please install yt-dlp manually: https://github.com/yt-dlp/yt-dlp"
    }
}

# 3. Check for ffmpeg
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "ffmpeg not found on PATH. Attempting to install via winget..." -ForegroundColor Yellow
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        winget install --id Gyan.FFmpeg -e --accept-package-agreements --accept-source-agreements
        # Refresh current session PATH
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
    } else {
        Write-Warning "winget is not available. Please install ffmpeg manually: https://ffmpeg.org/download.html"
    }
}

# 4. Install ytpdr via cargo
Write-Host "Building and installing ytpdr..." -ForegroundColor Cyan
cargo install --path .

# 5. Ensure ~/.cargo/bin is in User PATH
$cargoBin = Join-Path $HOME ".cargo\bin"
$userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$cargoBin*") {
    Write-Host "Adding $cargoBin to User PATH..." -ForegroundColor Yellow
    [System.Environment]::SetEnvironmentVariable("Path", "$userPath;$cargoBin", "User")
}

if ($env:Path -notlike "*$cargoBin*") {
    $env:Path += ";$cargoBin"
}

Write-Host "`nInstallation complete!" -ForegroundColor Green
Write-Host "You can now run 'ytpdr --help' or download a video:"
Write-Host "  ytpdr 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'" -ForegroundColor Gray

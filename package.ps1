<#
.SYNOPSIS
    Builds release binaries and generates Windows installer (.exe) & portable (.zip).
.DESCRIPTION
    1. Compiles ytpdr in release mode.
    2. Bundles yt-dlp.exe and ffmpeg.exe.
    3. Generates Inno Setup graphical installer (ytpdr-Setup-v1.0.0.exe).
    4. Generates a standalone portable zip (ytpdr-v1.0.0-windows-portable.zip).
#>

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
Set-Location $ScriptDir

Write-Host "=== Building YTPDR Distribution Packages ===" -ForegroundColor Cyan

# 1. Build release binary
Write-Host "`n[1/5] Compiling ytpdr in release mode..." -ForegroundColor Yellow
cargo build --release

$releaseExe = Join-Path $ScriptDir "target\release\ytpdr.exe"
if (-not (Test-Path $releaseExe)) {
    Write-Error "Release executable not found at $releaseExe"
    exit 1
}

# 2. Locate yt-dlp.exe and ffmpeg.exe
Write-Host "`n[2/5] Locating dependencies (yt-dlp.exe and ffmpeg.exe)..." -ForegroundColor Yellow

$bundleDir = Join-Path $ScriptDir "installer\bundle"
if (-not (Test-Path $bundleDir)) {
    New-Item -ItemType Directory -Path $bundleDir -Force | Out-Null
}

# Locate yt-dlp
$ytdlpCmd = Get-Command yt-dlp -ErrorAction SilentlyContinue
$ytdlpSource = if ($ytdlpCmd) { $ytdlpCmd.Source } else { $null }

if (-not $ytdlpSource -or -not (Test-Path $ytdlpSource)) {
    Write-Host "Downloading latest yt-dlp.exe..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe" -OutFile (Join-Path $bundleDir "yt-dlp.exe")
} else {
    Write-Host "Found yt-dlp at: $ytdlpSource"
    Copy-Item -Path $ytdlpSource -Destination (Join-Path $bundleDir "yt-dlp.exe") -Force
}

# Locate ffmpeg
$ffmpegCmd = Get-Command ffmpeg -ErrorAction SilentlyContinue
$ffmpegSource = if ($ffmpegCmd) { $ffmpegCmd.Source } else { $null }

if (-not $ffmpegSource -or -not (Test-Path $ffmpegSource)) {
    Write-Host "ffmpeg not found on PATH. Please ensure ffmpeg is installed or placed in installer\bundle\ffmpeg.exe" -ForegroundColor Red
    exit 1
} else {
    Write-Host "Found ffmpeg at: $ffmpegSource"
    Copy-Item -Path $ffmpegSource -Destination (Join-Path $bundleDir "ffmpeg.exe") -Force
}

# 3. Create dist directory
$distDir = Join-Path $ScriptDir "dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir -Force | Out-Null
}

# 4. Compile Inno Setup Installer
Write-Host "`n[3/5] Compiling Inno Setup Windows Installer (.exe)..." -ForegroundColor Yellow

$isccCmd = Get-Command iscc -ErrorAction SilentlyContinue
$isccFromPath = if ($isccCmd) { $isccCmd.Source } else { $null }

$iscc = $null
$candidates = @(
    $isccFromPath,
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
)
foreach ($c in $candidates) {
    if ($c -and (Test-Path $c)) {
        $iscc = $c
        break
    }
}

if ($iscc) {
    Write-Host "Using Inno Setup compiler: $iscc"
    & "$iscc" (Join-Path $ScriptDir "installer\ytpdr.iss")
    Write-Host "Installer created successfully!" -ForegroundColor Green
} else {
    Write-Warning "Inno Setup compiler (ISCC.exe) not found. Skipping .exe installer compilation."
    Write-Host "Tip: install Inno Setup via 'winget install JRSoftware.InnoSetup' to build the setup wizard." -ForegroundColor Gray
}

# 5. Build Portable ZIP Package
Write-Host "`n[4/5] Creating portable ZIP bundle..." -ForegroundColor Yellow

$portableDir = Join-Path $distDir "ytpdr-portable"
if (Test-Path $portableDir) {
    Remove-Item -Recurse -Force $portableDir
}
New-Item -ItemType Directory -Path $portableDir -Force | Out-Null

Copy-Item -Path $releaseExe -Destination (Join-Path $portableDir "ytpdr.exe")
Copy-Item -Path (Join-Path $bundleDir "yt-dlp.exe") -Destination (Join-Path $portableDir "yt-dlp.exe")
Copy-Item -Path (Join-Path $bundleDir "ffmpeg.exe") -Destination (Join-Path $portableDir "ffmpeg.exe")
Copy-Item -Path (Join-Path $ScriptDir "README.md") -Destination (Join-Path $portableDir "README.txt")

# Helper scripts for portable package
$installBat = @'
@echo off
setlocal
echo Adding current folder to User PATH...
set "CURRENT_DIR=%~dp0"
set "CURRENT_DIR=%CURRENT_DIR:~0,-1%"

for /f "tokens=2*" %%A in ('reg query HKCU\Environment /v Path 2^>nul') do set "USER_PATH=%%B"

if defined USER_PATH (
    echo %USER_PATH% | findstr /I /C:"%CURRENT_DIR%" >nul
    if errorlevel 1 (
        setx Path "%USER_PATH%;%CURRENT_DIR%" >nul
        echo Successfully added %CURRENT_DIR% to your User PATH!
    ) else (
        echo Folder is already in your PATH.
    )
) else (
    setx Path "%CURRENT_DIR%" >nul
    echo Successfully set User PATH!
)

echo.
echo Setup complete! Restart any open Command Prompt or PowerShell windows.
echo Then type: ytpdr --help
echo.
pause
'@
Set-Content -Path (Join-Path $portableDir "install-path.bat") -Value $installBat

$uninstallBat = @'
@echo off
setlocal enabledelayedexpansion
echo Removing current folder from User PATH...
set "CURRENT_DIR=%~dp0"
set "CURRENT_DIR=%CURRENT_DIR:~0,-1%"

for /f "tokens=2*" %%A in ('reg query HKCU\Environment /v Path 2^>nul') do set "USER_PATH=%%B"

if defined USER_PATH (
    set "NEW_PATH=!USER_PATH:%CURRENT_DIR%;=!"
    set "NEW_PATH=!NEW_PATH:;%CURRENT_DIR%=!"
    set "NEW_PATH=!NEW_PATH:%CURRENT_DIR%=!"
    setx Path "!NEW_PATH!" >nul
    echo Removed from PATH.
)
echo.
pause
'@
Set-Content -Path (Join-Path $portableDir "uninstall-path.bat") -Value $uninstallBat

$zipPath = Join-Path $distDir "ytpdr-v1.0.0-windows-portable.zip"
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}
Compress-Archive -Path "$portableDir\*" -DestinationPath $zipPath -CompressionLevel Optimal
Remove-Item -Recurse -Force $portableDir

# 6. Report outputs
Write-Host "`n[5/5] Done! Packages generated in dist\:" -ForegroundColor Green
Get-ChildItem -Path $distDir | ForEach-Object {
    $sizeMb = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  - $($_.Name) ($sizeMb MB)" -ForegroundColor Cyan
}

Write-Host "`nYou can share either of these files with your friends:" -ForegroundColor Yellow
Write-Host "  1. ytpdr-Setup-v1.0.0.exe (Recommended: standard double-click Windows Setup wizard)"
Write-Host "  2. ytpdr-v1.0.0-windows-portable.zip (Portable: extract and use anywhere, with optional install-path.bat)"

# ─────────────────────────────────────────────────
# OpenAnalyst CLI Installer — Windows PowerShell
#
# Usage:
#   irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex
# ─────────────────────────────────────────────────

$ErrorActionPreference = "Stop"
$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"

Write-Host ""
Write-Host "  ######   #####" -ForegroundColor Cyan
Write-Host " ##    ## ##   ##" -ForegroundColor Cyan
Write-Host " ##    ## #######" -ForegroundColor Cyan
Write-Host " ##    ## ##   ##" -ForegroundColor Cyan
Write-Host "  ######  ##   ##" -ForegroundColor Cyan
Write-Host ""
Write-Host " OpenAnalyst CLI Installer" -ForegroundColor White
Write-Host " --------------------------" -ForegroundColor DarkGray
Write-Host ""

$Target = "x86_64-pc-windows-msvc"
Write-Host " Platform:  Windows x64" -ForegroundColor DarkGray
Write-Host " Install:   $InstallDir" -ForegroundColor DarkGray
Write-Host ""

# Create install dir
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$Downloaded = $false

# Try downloading prebuilt binary from latest release
Write-Host " [1/3] Fetching latest release..." -ForegroundColor DarkGray
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{"User-Agent"="openanalyst-cli"} -ErrorAction Stop
    $Version = $Release.tag_name -replace "^v", ""
    $AssetUrl = "https://github.com/$Repo/releases/download/v$Version/openanalyst-$Target.exe"

    Write-Host " [2/3] Downloading v$Version..." -ForegroundColor DarkGray
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -ErrorAction Stop
        $Downloaded = $true
        Write-Host " Downloaded prebuilt binary" -ForegroundColor Green
    } catch {
        Write-Host " No prebuilt binary, will build from source" -ForegroundColor DarkGray
    }
} catch {
    Write-Host " Could not fetch release info, will build from source" -ForegroundColor DarkGray
}

# Fall back to building from source
if (-not $Downloaded) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host " Rust is required to build from source." -ForegroundColor Red
        Write-Host " Install: winget install Rustlang.Rustup" -ForegroundColor Yellow
        Write-Host " Or visit: https://rustup.rs" -ForegroundColor Yellow
        exit 1
    }

    Write-Host " [2/3] Building from source (this takes a few minutes)..." -ForegroundColor DarkGray

    $TempDir = Join-Path $env:TEMP "openanalyst-build-$(Get-Random)"
    git clone --depth 1 "https://github.com/$Repo.git" $TempDir 2>$null
    Push-Location "$TempDir\rust"
    cargo build --release -p openanalyst-cli 2>&1
    Pop-Location
    Copy-Item "$TempDir\rust\target\release\$BinaryName" "$InstallDir\$BinaryName" -Force
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    Write-Host " Built from source" -ForegroundColor Green
}

# Add to PATH
Write-Host " [3/3] Configuring PATH..." -ForegroundColor DarkGray
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host " Added to user PATH" -ForegroundColor Green
} else {
    Write-Host " PATH already configured" -ForegroundColor DarkGray
}

Write-Host ""
Write-Host " Installation complete" -ForegroundColor Green
Write-Host ""

& "$InstallDir\$BinaryName" --version 2>&1 | ForEach-Object { Write-Host " $_" }

Write-Host ""
Write-Host " Configure your API:" -ForegroundColor White
Write-Host ""
Write-Host '   $env:OPENANALYST_AUTH_TOKEN = "your-api-key-here"' -ForegroundColor Cyan
Write-Host ""
Write-Host " Start using:" -ForegroundColor White
Write-Host ""
Write-Host "   > openanalyst" -ForegroundColor Green
Write-Host ""
Write-Host " Restart terminal for PATH to take effect." -ForegroundColor DarkGray
Write-Host " Questions? anit@openanalyst.com" -ForegroundColor DarkGray
Write-Host ""

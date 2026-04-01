# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://openanalyst.com/install.ps1 | iex

$ErrorActionPreference = "Stop"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$RepoDir = Split-Path -Parent $MyInvocation.MyCommand.Definition

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

# Check Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host " [!] Rust/Cargo not found. Install from https://rustup.rs" -ForegroundColor Red
    Write-Host "     winget install Rustlang.Rustup" -ForegroundColor Yellow
    exit 1
}

# Build
Write-Host " [1/3] Building release binary..." -ForegroundColor DarkGray
Push-Location "$RepoDir\rust"
cargo build --release --quiet 2>&1
Pop-Location

$BinaryPath = "$RepoDir\rust\target\release\$BinaryName"
if (-not (Test-Path $BinaryPath)) {
    Write-Host " [!] Build failed - binary not found" -ForegroundColor Red
    exit 1
}

# Install
Write-Host " [2/3] Installing to $InstallDir..." -ForegroundColor DarkGray
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item $BinaryPath "$InstallDir\$BinaryName" -Force

# Add to PATH
Write-Host " [3/3] Configuring system PATH..." -ForegroundColor DarkGray
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host " Added $InstallDir to user PATH" -ForegroundColor Green
} else {
    Write-Host " PATH already configured" -ForegroundColor DarkGray
}

Write-Host ""
Write-Host " -- Installation complete --" -ForegroundColor Green
Write-Host ""

# Show version
& "$InstallDir\$BinaryName" --version 2>&1 | ForEach-Object { Write-Host " $_" }

Write-Host ""
Write-Host " Configure your API credentials:" -ForegroundColor White
Write-Host ""
Write-Host '   # OpenAnalyst API' -ForegroundColor DarkGray
Write-Host '   $env:OPENANALYST_AUTH_TOKEN = "your-api-key-here"' -ForegroundColor Cyan
Write-Host ""
Write-Host '   # Or use Anthropic / OpenAI / OpenRouter / Bedrock' -ForegroundColor DarkGray
Write-Host '   $env:ANTHROPIC_API_KEY = "sk-ant-..."' -ForegroundColor Yellow
Write-Host '   $env:OPENAI_API_KEY = "sk-..."' -ForegroundColor Yellow
Write-Host '   $env:OPENROUTER_API_KEY = "sk-or-..."' -ForegroundColor Yellow
Write-Host ""
Write-Host '   # Override default model' -ForegroundColor DarkGray
Write-Host '   $env:ANTHROPIC_DEFAULT_SONNET_MODEL = "openanalyst-beta"' -ForegroundColor Cyan
Write-Host ""
Write-Host " Start using:" -ForegroundColor White
Write-Host ""
Write-Host "   > openanalyst" -ForegroundColor Green
Write-Host ""
Write-Host " Note: Restart your terminal for PATH changes to take effect." -ForegroundColor DarkGray
Write-Host ""

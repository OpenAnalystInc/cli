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
$ConfigDir = "$env:USERPROFILE\.openanalyst"

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
Write-Host " Config:    $ConfigDir" -ForegroundColor DarkGray
Write-Host ""

# Create directories
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

$Downloaded = $false

# ── Step 1: Download or build ──
Write-Host " [1/4] Fetching latest release..." -ForegroundColor DarkGray
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{"User-Agent"="openanalyst-cli"} -ErrorAction Stop
    $Version = $Release.tag_name -replace "^v", ""
    $AssetUrl = "https://github.com/$Repo/releases/download/v$Version/openanalyst-$Target.exe"

    Write-Host " [2/4] Downloading v$Version..." -ForegroundColor DarkGray
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -ErrorAction Stop
        $Downloaded = $true
        Write-Host " $(([char]0x2713)) Downloaded prebuilt binary" -ForegroundColor Green
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

    Write-Host " [2/4] Building from source (this takes a few minutes)..." -ForegroundColor DarkGray

    $TempDir = Join-Path $env:TEMP "openanalyst-build-$(Get-Random)"
    git clone --depth 1 "https://github.com/$Repo.git" $TempDir 2>$null
    Push-Location "$TempDir\rust"
    cargo build --release -p openanalyst-cli 2>&1
    Pop-Location
    Copy-Item "$TempDir\rust\target\release\$BinaryName" "$InstallDir\$BinaryName" -Force
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    Write-Host " $(([char]0x2713)) Built from source" -ForegroundColor Green
}

# ── Step 2: Add to PATH ──
Write-Host " [3/4] Configuring PATH..." -ForegroundColor DarkGray
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host " $(([char]0x2713)) Added to user PATH" -ForegroundColor Green
} else {
    Write-Host " PATH already configured" -ForegroundColor DarkGray
}

# ── Step 3: Create .env template ──
Write-Host " [4/4] Setting up config..." -ForegroundColor DarkGray
$EnvFile = "$ConfigDir\.env"
if (-not (Test-Path $EnvFile)) {
    @"
# ===================================================================
#  OpenAnalyst CLI - Environment Configuration
# ===================================================================
#
#  Add your API keys here. The CLI loads this file on startup.
#  Only uncomment and fill in the providers you want to use.
#  You can also use ``openanalyst login`` for interactive setup.
#
#  Docs: https://github.com/AnitChaudhry/openanalyst-cli
# ===================================================================

# -- Provider API Keys ---------------------------------------------

# OpenAnalyst (default provider)
# OPENANALYST_API_KEY=
# OPENANALYST_AUTH_TOKEN=

# Anthropic / Claude (opus, sonnet, haiku)
# ANTHROPIC_API_KEY=sk-ant-...

# OpenAI / Codex (gpt-4o, o3, codex-mini)
# OPENAI_API_KEY=sk-...

# Google Gemini (gemini-2.5-pro, flash)
# GEMINI_API_KEY=AIza...

# xAI / Grok (grok-3, grok-mini)
# XAI_API_KEY=xai-...

# OpenRouter (350+ models via one key)
# OPENROUTER_API_KEY=sk-or-...

# Amazon Bedrock
# BEDROCK_API_KEY=

# Stability AI (image generation via /image)
# STABILITY_API_KEY=sk-...

# -- Base URL Overrides (optional) ---------------------------------

# OPENANALYST_BASE_URL=https://api.openanalyst.com/api
# ANTHROPIC_BASE_URL=https://api.anthropic.com
# OPENAI_BASE_URL=https://api.openai.com/v1
# GEMINI_BASE_URL=https://generativelanguage.googleapis.com/v1beta/openai
# XAI_BASE_URL=https://api.x.ai/v1
# OPENROUTER_BASE_URL=https://openrouter.ai/api/v1
# BEDROCK_BASE_URL=https://bedrock-runtime.us-east-1.amazonaws.com/v1

# -- Model Configuration ------------------------------------------

# OPENANALYST_MODEL=claude-sonnet-4-6
"@ | Out-File -FilePath $EnvFile -Encoding utf8
    Write-Host " $(([char]0x2713)) Created $EnvFile" -ForegroundColor Green
} else {
    Write-Host " Config already exists" -ForegroundColor DarkGray
}

# ── Done ──
Write-Host ""
Write-Host " $(([char]0x2713)) Installation complete" -ForegroundColor Green
Write-Host ""

try {
    $VersionOutput = & "$InstallDir\$BinaryName" --version 2>&1 | Select-Object -Skip 1 -First 1
    Write-Host " Version:   $($VersionOutput.Trim())" -ForegroundColor DarkGray
} catch {}

Write-Host " Binary:    $InstallDir\$BinaryName" -ForegroundColor DarkGray
Write-Host " Config:    $EnvFile" -ForegroundColor DarkGray
Write-Host ""
Write-Host " Get started:" -ForegroundColor White
Write-Host ""
Write-Host "   openanalyst login" -ForegroundColor Cyan -NoNewline
Write-Host "              # Interactive login (browser or API key)" -ForegroundColor DarkGray
Write-Host "   openanalyst" -ForegroundColor Cyan -NoNewline
Write-Host "                    # Start a new session" -ForegroundColor DarkGray
Write-Host ""
Write-Host " Or edit $EnvFile to add your API keys directly." -ForegroundColor DarkGray
Write-Host ""
Write-Host " Available providers:" -ForegroundColor White
Write-Host "   OpenAnalyst (default)  *  Anthropic/Claude  *  OpenAI/Codex" -ForegroundColor DarkGray
Write-Host "   Google Gemini  *  xAI/Grok  *  OpenRouter  *  Amazon Bedrock" -ForegroundColor DarkGray
Write-Host ""
Write-Host " Restart terminal for PATH to take effect." -ForegroundColor DarkGray
Write-Host " Questions? anit@openanalyst.com" -ForegroundColor DarkGray
Write-Host ""

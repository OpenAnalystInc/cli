# ─────────────────────────────────────────────────
# OpenAnalyst CLI Installer — Windows PowerShell
#
# Usage:
#   irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex
# ─────────────────────────────────────────────────

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$ConfigDir = "$env:USERPROFILE\.openanalyst"
$Target = "x86_64-pc-windows-msvc"

Clear-Host

Write-Host ""
Write-Host ""
Write-Host "     ██████╗   █████╗ " -ForegroundColor Blue
Write-Host "    ██╔═══██╗ ██╔══██╗" -ForegroundColor Cyan
Write-Host "    ██║   ██║ ███████║" -ForegroundColor Cyan
Write-Host "    ██║   ██║ ██╔══██║" -ForegroundColor Cyan
Write-Host "    ╚██████╔╝ ██║  ██║" -ForegroundColor Blue
Write-Host "     ╚═════╝  ╚═╝  ╚═╝" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   OpenAnalyst CLI  " -ForegroundColor White -NoNewline
Write-Host "v1.0.2" -ForegroundColor DarkGray
Write-Host "   The Universal AI Agent for Your Terminal" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   ────────────────────────────────────────────" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   ┌──────────────────────────────────────────┐" -ForegroundColor DarkGray
Write-Host "   │  System       Windows x64                │" -ForegroundColor DarkGray
Write-Host "   │  Install to   $InstallDir" -ForegroundColor DarkGray
Write-Host "   │  Config at    $ConfigDir" -ForegroundColor DarkGray
Write-Host "   └──────────────────────────────────────────┘" -ForegroundColor DarkGray
Write-Host ""

# Create directories
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

$Downloaded = $false

# Step 1 — Download
Write-Host "   › Fetching latest release..." -ForegroundColor Cyan -NoNewline
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{"User-Agent"="openanalyst-cli"} -ErrorAction Stop
    $Version = $Release.tag_name -replace "^v", ""
    Write-Host " v$Version" -ForegroundColor Green
    $AssetUrl = "https://github.com/$Repo/releases/download/v$Version/openanalyst-$Target.exe"

    Write-Host "   › Downloading binary..." -ForegroundColor Cyan -NoNewline
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -ErrorAction Stop
        $Downloaded = $true
        Write-Host " ✓ done" -ForegroundColor Green
    } catch {
        Write-Host " unavailable" -ForegroundColor Yellow
    }
} catch {
    Write-Host " not found" -ForegroundColor Yellow
}

# Fallback — build from source
if (-not $Downloaded) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "   Rust is required to build from source." -ForegroundColor Red
        Write-Host ""
        Write-Host "   Install:" -ForegroundColor White
        Write-Host "   winget install Rustlang.Rustup" -ForegroundColor Cyan
        Write-Host ""
        exit 1
    }

    Write-Host "   › Building from source (a few minutes)..." -ForegroundColor Cyan -NoNewline
    $TempDir = Join-Path $env:TEMP "openanalyst-build-$(Get-Random)"
    git clone --depth 1 "https://github.com/$Repo.git" $TempDir 2>$null
    Push-Location "$TempDir\rust"
    cargo build --release -p openanalyst-cli 2>&1 | Out-Null
    Pop-Location
    Copy-Item "$TempDir\rust\target\release\$BinaryName" "$InstallDir\$BinaryName" -Force
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    Write-Host " ✓" -ForegroundColor Green
}

# Step 2 — PATH
Write-Host "   › Configuring PATH..." -ForegroundColor Cyan -NoNewline
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host " ✓ added" -ForegroundColor Green
} else {
    Write-Host " ✓ already set" -ForegroundColor DarkGray
}

# Step 3 — .env config
Write-Host "   › Creating config..." -ForegroundColor Cyan -NoNewline
$EnvFile = "$ConfigDir\.env"
if (-not (Test-Path $EnvFile)) {
    @"
# ═══════════════════════════════════════════════════════════════════
#  OpenAnalyst CLI — Environment Configuration
# ═══════════════════════════════════════════════════════════════════
#
#  Add your API keys below. The CLI loads this file on every startup.
#  Uncomment and fill in the providers you want to use.
#  Or run ``openanalyst login`` for interactive browser-based setup.
#
#  Docs: https://github.com/AnitChaudhry/openanalyst-cli
# ═══════════════════════════════════════════════════════════════════

# ── Provider API Keys ─────────────────────────────────────────────

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

# ── Base URL Overrides (optional) ─────────────────────────────────

# OPENANALYST_BASE_URL=https://api.openanalyst.com/api
# ANTHROPIC_BASE_URL=https://api.anthropic.com
# OPENAI_BASE_URL=https://api.openai.com/v1

# ── Model Override ────────────────────────────────────────────────

# OPENANALYST_MODEL=claude-sonnet-4-6
"@ | Out-File -FilePath $EnvFile -Encoding utf8
    Write-Host " ✓ created" -ForegroundColor Green
} else {
    Write-Host " ✓ exists" -ForegroundColor DarkGray
}

# ═══════════════════════════════════════════════════
#  Summary
# ═══════════════════════════════════════════════════
Write-Host ""
Write-Host "   ────────────────────────────────────────────" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   ✓ Installation complete" -ForegroundColor Green
Write-Host ""
Write-Host "   ┌──────────────────────────────────────────┐" -ForegroundColor DarkGray

try {
    $VersionOutput = & "$InstallDir\$BinaryName" --version 2>&1 | Select-Object -Skip 1 -First 1
    Write-Host "   │  Version    $($VersionOutput.Trim().PadRight(29))│" -ForegroundColor DarkGray
} catch {}

Write-Host "   │  Binary     $("$InstallDir\$BinaryName".PadRight(29))│" -ForegroundColor DarkGray
Write-Host "   │  Config     $("$EnvFile".PadRight(29))│" -ForegroundColor DarkGray
Write-Host "   └──────────────────────────────────────────┘" -ForegroundColor DarkGray

Write-Host ""
Write-Host "   Next steps" -ForegroundColor White
Write-Host ""
Write-Host "   1. Login to your LLM provider" -ForegroundColor White
Write-Host ""
Write-Host "      › openanalyst login" -ForegroundColor Cyan
Write-Host ""
Write-Host "      Select a provider, authenticate via browser" -ForegroundColor DarkGray
Write-Host "      or paste your API key. Credentials are saved" -ForegroundColor DarkGray
Write-Host "      and remembered across sessions." -ForegroundColor DarkGray
Write-Host ""
Write-Host "   2. Start coding" -ForegroundColor White
Write-Host ""
Write-Host "      › openanalyst" -ForegroundColor Cyan
Write-Host ""
Write-Host ""
Write-Host "   ┌──────────────────────────────────────────┐" -ForegroundColor DarkGray
Write-Host "   │                                          │" -ForegroundColor DarkGray
Write-Host "   │  7 LLM Providers. One Interface.         │" -ForegroundColor White
Write-Host "   │                                          │" -ForegroundColor DarkGray
Write-Host "   │  ■ OpenAnalyst (default)                 │" -ForegroundColor DarkGray
Write-Host "   │  ■ Anthropic / Claude  · direct API      │" -ForegroundColor DarkGray
Write-Host "   │  ■ OpenAI / Codex     · direct API       │" -ForegroundColor DarkGray
Write-Host "   │  ■ Google Gemini      · direct API       │" -ForegroundColor DarkGray
Write-Host "   │  ■ xAI / Grok                            │" -ForegroundColor DarkGray
Write-Host "   │  ■ OpenRouter         · 350+ models      │" -ForegroundColor DarkGray
Write-Host "   │  ■ Amazon Bedrock                        │" -ForegroundColor DarkGray
Write-Host "   │                                          │" -ForegroundColor DarkGray
Write-Host "   │  Switch: /model gpt-4o                   │" -ForegroundColor DarkGray
Write-Host "   │  Update: openanalyst update              │" -ForegroundColor DarkGray
Write-Host "   │                                          │" -ForegroundColor DarkGray
Write-Host "   └──────────────────────────────────────────┘" -ForegroundColor DarkGray

Write-Host ""
Write-Host "   Restart terminal for PATH changes." -ForegroundColor DarkGray
Write-Host "   Docs: github.com/AnitChaudhry/openanalyst-cli" -ForegroundColor DarkGray
Write-Host ""

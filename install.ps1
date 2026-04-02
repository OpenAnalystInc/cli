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
$Target = "x86_64-pc-windows-msvc"
$Check = [char]0x2713
$Arrow = [char]0x203A
$Block = [char]0x25A0

Clear-Host

Write-Host ""
Write-Host ""
Write-Host "    " -NoNewline; Write-Host ([char]0x2588)*6 -ForegroundColor Blue -NoNewline; Write-Host ([char]0x2557) -ForegroundColor Blue -NoNewline
Write-Host "  " -NoNewline; Write-Host ([char]0x2588)*5 -ForegroundColor Cyan -NoNewline; Write-Host ([char]0x2557) -ForegroundColor Cyan
Write-Host "   " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Blue -NoNewline; Write-Host "    " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Blue -NoNewline
Write-Host " " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan -NoNewline; Write-Host "   " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan
Write-Host "   " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Blue -NoNewline; Write-Host "    " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Blue -NoNewline
Write-Host " " -NoNewline; Write-Host ([char]0x2588)*5 -ForegroundColor Cyan -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan
Write-Host "   " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Blue -NoNewline; Write-Host "    " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Blue -NoNewline
Write-Host " " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan -NoNewline; Write-Host "   " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan
Write-Host "    " -NoNewline; Write-Host ([char]0x2588)*6 -ForegroundColor Blue -NoNewline; Write-Host ([char]0x255D) -ForegroundColor Blue -NoNewline
Write-Host " " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan -NoNewline; Write-Host "   " -NoNewline; Write-Host ([char]0x2551) -ForegroundColor Cyan
Write-Host ""
Write-Host "   OpenAnalyst CLI" -ForegroundColor White -NoNewline; Write-Host "  v1.0.1" -ForegroundColor DarkGray
Write-Host "   The Universal AI Agent for Your Terminal" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   " -NoNewline; Write-Host ([string]([char]0x2500) * 44) -ForegroundColor DarkGray
Write-Host ""

# System info box
Write-Host "   " -NoNewline; Write-Host ([char]0x250C) -ForegroundColor DarkGray -NoNewline; Write-Host ([string]([char]0x2500) * 42) -ForegroundColor DarkGray -NoNewline; Write-Host ([char]0x2510) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
Write-Host "  " -NoNewline; Write-Host "System" -ForegroundColor White -NoNewline; Write-Host "       Windows x64                " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
Write-Host "  " -NoNewline; Write-Host "Install to" -ForegroundColor White -NoNewline; Write-Host "   $InstallDir" -NoNewline
$pad1 = 42 - ("  Install to   $InstallDir").Length
if ($pad1 -gt 0) { Write-Host (" " * $pad1) -NoNewline }
Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
Write-Host "  " -NoNewline; Write-Host "Config at" -ForegroundColor White -NoNewline; Write-Host "    $ConfigDir" -NoNewline
$pad2 = 42 - ("  Config at    $ConfigDir").Length
if ($pad2 -gt 0) { Write-Host (" " * $pad2) -NoNewline }
Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2514) -ForegroundColor DarkGray -NoNewline; Write-Host ([string]([char]0x2500) * 42) -ForegroundColor DarkGray -NoNewline; Write-Host ([char]0x2518) -ForegroundColor DarkGray
Write-Host ""

# Create directories
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

$Downloaded = $false

# Step 1 — Download
Write-Host "   $Arrow " -ForegroundColor Cyan -NoNewline; Write-Host "Fetching latest release..." -ForegroundColor DarkGray -NoNewline
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{"User-Agent"="openanalyst-cli"} -ErrorAction Stop
    $Version = $Release.tag_name -replace "^v", ""
    Write-Host " v$Version" -ForegroundColor Green
    $AssetUrl = "https://github.com/$Repo/releases/download/v$Version/openanalyst-$Target.exe"

    Write-Host "   $Arrow " -ForegroundColor Cyan -NoNewline; Write-Host "Downloading binary..." -ForegroundColor DarkGray -NoNewline
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -ErrorAction Stop
        $Downloaded = $true
        Write-Host " $Check done" -ForegroundColor Green
    } catch {
        Write-Host " unavailable" -ForegroundColor DarkYellow
    }
} catch {
    Write-Host " not found" -ForegroundColor DarkYellow
}

# Fallback — build from source
if (-not $Downloaded) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "   Rust is required to build from source." -ForegroundColor Red
        Write-Host ""
        Write-Host "   Install:" -ForegroundColor White
        Write-Host "   winget install Rustlang.Rustup" -ForegroundColor Cyan
        Write-Host "   Or visit: https://rustup.rs" -ForegroundColor DarkGray
        Write-Host ""
        exit 1
    }

    Write-Host "   $Arrow " -ForegroundColor Cyan -NoNewline; Write-Host "Building from source (a few minutes)..." -ForegroundColor DarkGray -NoNewline
    $TempDir = Join-Path $env:TEMP "openanalyst-build-$(Get-Random)"
    git clone --depth 1 "https://github.com/$Repo.git" $TempDir 2>$null
    Push-Location "$TempDir\rust"
    cargo build --release -p openanalyst-cli 2>&1 | Out-Null
    Pop-Location
    Copy-Item "$TempDir\rust\target\release\$BinaryName" "$InstallDir\$BinaryName" -Force
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    Write-Host " $Check" -ForegroundColor Green
}

# Step 2 — PATH
Write-Host "   $Arrow " -ForegroundColor Cyan -NoNewline; Write-Host "Configuring PATH..." -ForegroundColor DarkGray -NoNewline
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host " $Check added" -ForegroundColor Green
} else {
    Write-Host " $Check already set" -ForegroundColor DarkGray
}

# Step 3 — .env config
Write-Host "   $Arrow " -ForegroundColor Cyan -NoNewline; Write-Host "Creating config..." -ForegroundColor DarkGray -NoNewline
$EnvFile = "$ConfigDir\.env"
if (-not (Test-Path $EnvFile)) {
    @"
# ===================================================================
#  OpenAnalyst CLI - Environment Configuration
# ===================================================================
#
#  Add your API keys below. The CLI loads this file on every startup.
#  Uncomment and fill in the providers you want to use.
#  Or run ``openanalyst login`` for interactive browser-based setup.
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

# -- Model Override ------------------------------------------------

# OPENANALYST_MODEL=claude-sonnet-4-6
"@ | Out-File -FilePath $EnvFile -Encoding utf8
    Write-Host " $Check ~/.openanalyst/.env" -ForegroundColor Green
} else {
    Write-Host " $Check already exists" -ForegroundColor DarkGray
}

# ═══════════════════════════════════════════════════
#  Summary
# ═══════════════════════════════════════════════════
Write-Host ""
Write-Host ""
Write-Host "   " -NoNewline; Write-Host ([string]([char]0x2500) * 44) -ForegroundColor DarkGray
Write-Host ""
Write-Host "   $Check Installation complete" -ForegroundColor Green
Write-Host ""

# Info box
Write-Host "   " -NoNewline; Write-Host ([char]0x250C) -ForegroundColor DarkGray -NoNewline; Write-Host ([string]([char]0x2500) * 42) -ForegroundColor DarkGray -NoNewline; Write-Host ([char]0x2510) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "                                          " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray

try {
    $CliVer = & "$InstallDir\$BinaryName" --version 2>&1 | Select-Object -Skip 1 -First 1
    Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
    Write-Host "  " -NoNewline; Write-Host "Version" -ForegroundColor White -NoNewline; Write-Host "    $($CliVer.Trim())" -NoNewline
    $pad = 42 - ("  Version    $($CliVer.Trim())").Length
    if ($pad -gt 0) { Write-Host (" " * $pad) -NoNewline }
    Write-Host ([char]0x2502) -ForegroundColor DarkGray
} catch {}

Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
Write-Host "  " -NoNewline; Write-Host "Binary" -ForegroundColor White -NoNewline; Write-Host "     $InstallDir\$BinaryName" -NoNewline
$pad3 = 42 - ("  Binary     $InstallDir\$BinaryName").Length
if ($pad3 -gt 0) { Write-Host (" " * [Math]::Max(0,$pad3)) -NoNewline }
Write-Host ([char]0x2502) -ForegroundColor DarkGray

Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
Write-Host "  " -NoNewline; Write-Host "Config" -ForegroundColor White -NoNewline; Write-Host "     $EnvFile" -NoNewline
$pad4 = 42 - ("  Config     $EnvFile").Length
if ($pad4 -gt 0) { Write-Host (" " * [Math]::Max(0,$pad4)) -NoNewline }
Write-Host ([char]0x2502) -ForegroundColor DarkGray

Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "                                          " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2514) -ForegroundColor DarkGray -NoNewline; Write-Host ([string]([char]0x2500) * 42) -ForegroundColor DarkGray -NoNewline; Write-Host ([char]0x2518) -ForegroundColor DarkGray

Write-Host ""
Write-Host "   " -NoNewline; Write-Host "Next steps" -ForegroundColor White
Write-Host ""
Write-Host "   " -NoNewline; Write-Host "1." -ForegroundColor Cyan -NoNewline; Write-Host " Login to your LLM provider" -ForegroundColor White
Write-Host ""
Write-Host "      " -NoNewline; Write-Host "> openanalyst login" -ForegroundColor Cyan
Write-Host ""
Write-Host "      Select a provider, authenticate via browser" -ForegroundColor DarkGray
Write-Host "      or paste your API key. Credentials are saved" -ForegroundColor DarkGray
Write-Host "      and remembered across sessions." -ForegroundColor DarkGray
Write-Host ""
Write-Host "   " -NoNewline; Write-Host "2." -ForegroundColor Cyan -NoNewline; Write-Host " Start coding" -ForegroundColor White
Write-Host ""
Write-Host "      " -NoNewline; Write-Host "> openanalyst" -ForegroundColor Cyan
Write-Host ""
Write-Host ""

# Provider box
Write-Host "   " -NoNewline; Write-Host ([char]0x250C) -ForegroundColor DarkGray -NoNewline; Write-Host ([string]([char]0x2500) * 42) -ForegroundColor DarkGray -NoNewline; Write-Host ([char]0x2510) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "                                          " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "  " -NoNewline; Write-Host "7 LLM Providers. One Interface." -ForegroundColor White -NoNewline; Write-Host "         " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "                                          " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray

$providers = @(
    @("OpenAnalyst", "(default)"),
    @("Anthropic / Claude", "direct API"),
    @("OpenAI / Codex", "direct API"),
    @("Google Gemini", "direct API"),
    @("xAI / Grok", ""),
    @("OpenRouter", "350+ models"),
    @("Amazon Bedrock", "")
)

foreach ($p in $providers) {
    $name = $p[0]
    $note = $p[1]
    $line = "  $Block $name"
    if ($note) { $line += "  $note" }
    $padP = 42 - $line.Length
    Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
    Write-Host "  " -NoNewline; Write-Host $Block -ForegroundColor Cyan -NoNewline
    Write-Host " $name" -NoNewline
    if ($note) { Write-Host "  " -NoNewline; Write-Host $note -ForegroundColor DarkGray -NoNewline }
    if ($padP -gt 0) { Write-Host (" " * [Math]::Max(0,$padP)) -NoNewline }
    Write-Host ([char]0x2502) -ForegroundColor DarkGray
}

Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "                                          " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline
Write-Host "  Switch models: " -ForegroundColor DarkGray -NoNewline; Write-Host "/model gpt-4o" -ForegroundColor Cyan -NoNewline; Write-Host "            " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray -NoNewline; Write-Host "                                          " -NoNewline; Write-Host ([char]0x2502) -ForegroundColor DarkGray
Write-Host "   " -NoNewline; Write-Host ([char]0x2514) -ForegroundColor DarkGray -NoNewline; Write-Host ([string]([char]0x2500) * 42) -ForegroundColor DarkGray -NoNewline; Write-Host ([char]0x2518) -ForegroundColor DarkGray

Write-Host ""
Write-Host "   Restart terminal for PATH changes." -ForegroundColor DarkGray
Write-Host "   Docs: github.com/AnitChaudhry/openanalyst-cli" -ForegroundColor DarkGray
Write-Host "   Support: anit@openanalyst.com" -ForegroundColor DarkGray
Write-Host ""
Write-Host ""

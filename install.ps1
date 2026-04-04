# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$ConfigDir = "$env:USERPROFILE\.openanalyst"
$W = 44  # box inner width

# ── Fetch version before rendering ──
$GhHeaders = @{ "User-Agent" = "openanalyst-cli" }
if ($env:GITHUB_TOKEN) { $GhHeaders["Authorization"] = "Bearer $env:GITHUB_TOKEN" }
elseif ($env:GH_TOKEN) { $GhHeaders["Authorization"] = "Bearer $env:GH_TOKEN" }
else { try { $t = (gh auth token 2>$null); if ($t) { $GhHeaders["Authorization"] = "Bearer $t" } } catch {} }

$Version = "latest"
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $GhHeaders -ErrorAction Stop
    $Version = $Release.tag_name -replace "^v", ""
} catch {}

# ── Header ──
$B = [char]0x2502  # │
$TL = [char]0x250C  # ┌
$TR = [char]0x2510  # ┐
$BL = [char]0x2514  # └
$BR = [char]0x2518  # ┘
$H = [char]0x2500   # ─
$ML = [char]0x251C  # ├
$MR = [char]0x2524  # ┤
$Bar = "$H" * $W

Write-Host ""
Write-Host "  $TL$Bar$TR" -ForegroundColor DarkGray
Write-Host "  $B                                            $B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
Write-Host -NoNewline "   " -ForegroundColor DarkGray
Write-Host -NoNewline "██████  █████" -ForegroundColor Cyan
Write-Host -NoNewline "   OpenAnalyst CLI       " -ForegroundColor White
Write-Host "$B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
Write-Host -NoNewline "  " -ForegroundColor DarkGray
Write-Host -NoNewline "██    ██ ██   ██" -ForegroundColor Cyan
Write-Host -NoNewline "  v$($Version.PadRight(25))" -ForegroundColor DarkGray
Write-Host "$B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
Write-Host -NoNewline "  " -ForegroundColor DarkGray
Write-Host -NoNewline "██    ██ ███████" -ForegroundColor Cyan
Write-Host -NoNewline "                           " -ForegroundColor DarkGray
Write-Host "$B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
Write-Host -NoNewline "  " -ForegroundColor DarkGray
Write-Host -NoNewline "██    ██ ██   ██" -ForegroundColor Cyan
Write-Host -NoNewline "  Windows x64             " -ForegroundColor DarkGray
Write-Host "$B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
Write-Host -NoNewline "   " -ForegroundColor DarkGray
Write-Host -NoNewline "██████  ██   ██" -ForegroundColor Cyan
Write-Host -NoNewline "                          " -ForegroundColor DarkGray
Write-Host "$B" -ForegroundColor DarkGray
Write-Host "  $B                                            $B" -ForegroundColor DarkGray
Write-Host "  $ML$Bar$MR" -ForegroundColor DarkGray

# ── Create directories ──
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# ── Step 1: Download ──
$Downloaded = $false
Write-Host -NoNewline "  $B  [1/3] Download " -ForegroundColor DarkGray
$dots = "." * 22
Write-Host -NoNewline "$dots " -ForegroundColor DarkGray

# Kill running instance
Get-Process -Name "openanalyst" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

if ($Release) {
    $Asset = $Release.assets | Where-Object { $_.name -eq "openanalyst.exe" } | Select-Object -First 1
    if (-not $Asset) { $Asset = $Release.assets | Where-Object { $_.name -like "*.exe" } | Select-Object -First 1 }

    if ($Asset -and $Asset.browser_download_url) {
        $AssetUrl = $Asset.browser_download_url
    } else {
        $AssetUrl = "https://github.com/$Repo/releases/download/$($Release.tag_name)/openanalyst.exe"
    }

    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -UseBasicParsing -ErrorAction Stop
        $Downloaded = $true
        Write-Host -NoNewline "✓" -ForegroundColor Green
    } catch {
        Write-Host -NoNewline "✗" -ForegroundColor Red
    }
} else {
    Write-Host -NoNewline "✗" -ForegroundColor Red
}
Write-Host "   $B" -ForegroundColor DarkGray

if (-not $Downloaded) {
    Write-Host "  $B                                            $B" -ForegroundColor DarkGray
    Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
    Write-Host -NoNewline "  Download failed. Visit:                  " -ForegroundColor Red
    Write-Host "$B" -ForegroundColor DarkGray
    Write-Host -NoNewline "  $B" -ForegroundColor DarkGray
    Write-Host -NoNewline "  github.com/$Repo/releases  " -ForegroundColor White
    Write-Host "$B" -ForegroundColor DarkGray
    Write-Host "  $B                                            $B" -ForegroundColor DarkGray
    Write-Host "  $BL$Bar$BR" -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

# ── Step 2: PATH ──
Write-Host -NoNewline "  $B  [2/3] PATH " -ForegroundColor DarkGray
$dots = "." * 26
Write-Host -NoNewline "$dots " -ForegroundColor DarkGray
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
}
Write-Host -NoNewline "✓" -ForegroundColor Green
Write-Host "   $B" -ForegroundColor DarkGray

# ── Step 3: Config ──
Write-Host -NoNewline "  $B  [3/3] Config " -ForegroundColor DarkGray
$dots = "." * 24
Write-Host -NoNewline "$dots " -ForegroundColor DarkGray
$EnvFile = "$ConfigDir\.env"
if (-not (Test-Path $EnvFile)) {
    @"
# OpenAnalyst CLI — Environment Configuration
# Add API keys below or run: openanalyst login

# OPENANALYST_API_KEY=
# ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
# XAI_API_KEY=xai-...
"@ | Out-File -FilePath $EnvFile -Encoding utf8
}
Write-Host -NoNewline "✓" -ForegroundColor Green
Write-Host "   $B" -ForegroundColor DarkGray

# ── Result ──
Write-Host "  $B                                            $B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B  " -ForegroundColor DarkGray
Write-Host -NoNewline "✓ Installed to " -ForegroundColor Green
Write-Host -NoNewline "~/.openanalyst/bin        " -ForegroundColor White
Write-Host "$B" -ForegroundColor DarkGray
Write-Host "  $B                                            $B" -ForegroundColor DarkGray

# ── Footer ──
Write-Host "  $ML$Bar$MR" -ForegroundColor DarkGray
Write-Host "  $B                                            $B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B  " -ForegroundColor DarkGray
Write-Host -NoNewline "openanalyst login" -ForegroundColor Cyan
Write-Host -NoNewline "    authenticate           " -ForegroundColor DarkGray
Write-Host "$B" -ForegroundColor DarkGray
Write-Host -NoNewline "  $B  " -ForegroundColor DarkGray
Write-Host -NoNewline "openanalyst" -ForegroundColor Cyan
Write-Host -NoNewline "          start coding           " -ForegroundColor DarkGray
Write-Host "$B" -ForegroundColor DarkGray
Write-Host "  $B                                            $B" -ForegroundColor DarkGray
Write-Host "  $BL$Bar$BR" -ForegroundColor DarkGray
Write-Host ""

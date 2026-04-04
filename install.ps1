# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$ConfigDir = "$env:USERPROFILE\.openanalyst"

# ── Fetch version ──
$GhHeaders = @{ "User-Agent" = "openanalyst-cli" }
if ($env:GITHUB_TOKEN) { $GhHeaders["Authorization"] = "Bearer $env:GITHUB_TOKEN" }
elseif ($env:GH_TOKEN) { $GhHeaders["Authorization"] = "Bearer $env:GH_TOKEN" }
else { try { $t = (gh auth token 2>$null); if ($t) { $GhHeaders["Authorization"] = "Bearer $t" } } catch {} }

$Version = "latest"
$Release = $null
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $GhHeaders -ErrorAction Stop
    $Version = $Release.tag_name -replace "^v", ""
} catch {}

# ── Render ──
function Ln($text, $fg) { Write-Host $text -ForegroundColor $fg }
function Ok { Write-Host " done" -ForegroundColor Green -NoNewline }
function Fail { Write-Host " FAIL" -ForegroundColor Red -NoNewline }

Write-Host ""
Ln "  +--------------------------------------------+" DarkGray
Ln "  |                                            |" DarkGray
Write-Host "  |   " -ForegroundColor DarkGray -NoNewline
Write-Host "OO  AA" -ForegroundColor Cyan -NoNewline
Write-Host "   OpenAnalyst CLI            |" -ForegroundColor DarkGray
Write-Host "  |   " -ForegroundColor DarkGray -NoNewline
Write-Host "OO  AA" -ForegroundColor Cyan -NoNewline
Write-Host "   v$($Version.PadRight(30))|" -ForegroundColor DarkGray
Write-Host "  |   " -ForegroundColor DarkGray -NoNewline
Write-Host "OO  AA" -ForegroundColor Cyan -NoNewline
Write-Host "   Windows x64                |" -ForegroundColor DarkGray
Ln "  |                                            |" DarkGray
Ln "  +--------------------------------------------+" DarkGray

# ── Create directories ──
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# ── Step 1: Download ──
Write-Host "  |  [1/3] Download ...................." -ForegroundColor DarkGray -NoNewline
$Downloaded = $false
Get-Process -Name "openanalyst" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

if ($Release) {
    $Asset = $Release.assets | Where-Object { $_.name -eq "openanalyst.exe" } | Select-Object -First 1
    if (-not $Asset) { $Asset = $Release.assets | Where-Object { $_.name -like "*.exe" } | Select-Object -First 1 }
    if ($Asset -and $Asset.browser_download_url) { $AssetUrl = $Asset.browser_download_url }
    else { $AssetUrl = "https://github.com/$Repo/releases/download/$($Release.tag_name)/openanalyst.exe" }
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -UseBasicParsing -ErrorAction Stop
        $Downloaded = $true
        Ok
    } catch { Fail }
} else { Fail }
Write-Host "  |" -ForegroundColor DarkGray

if (-not $Downloaded) {
    Ln "  |                                            |" DarkGray
    Ln "  |  Download failed. Visit:                   |" Red
    Ln "  |  github.com/$Repo/releases    |" White
    Ln "  |                                            |" DarkGray
    Ln "  +--------------------------------------------+" DarkGray
    Write-Host ""
    exit 1
}

# ── Step 2: PATH ──
Write-Host "  |  [2/3] PATH .........................." -ForegroundColor DarkGray -NoNewline
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
}
Ok
Write-Host "  |" -ForegroundColor DarkGray

# ── Step 3: Config ──
Write-Host "  |  [3/3] Config ........................" -ForegroundColor DarkGray -NoNewline
$EnvFile = "$ConfigDir\.env"
if (-not (Test-Path $EnvFile)) {
    @"
# OpenAnalyst CLI — Environment Configuration
# OPENANALYST_API_KEY=
# ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
"@ | Out-File -FilePath $EnvFile -Encoding utf8
}
Ok
Write-Host "  |" -ForegroundColor DarkGray

# ── Result ──
Ln "  |                                            |" DarkGray
Write-Host "  |  " -ForegroundColor DarkGray -NoNewline
Write-Host "Installed to ~/.openanalyst/bin" -ForegroundColor Green -NoNewline
Write-Host "          |" -ForegroundColor DarkGray
Ln "  |                                            |" DarkGray
Ln "  +--------------------------------------------+" DarkGray

# ── Footer ──
Write-Host "  |  " -ForegroundColor DarkGray -NoNewline
Write-Host "openanalyst login" -ForegroundColor Cyan -NoNewline
Write-Host "   authenticate          |" -ForegroundColor DarkGray
Write-Host "  |  " -ForegroundColor DarkGray -NoNewline
Write-Host "openanalyst" -ForegroundColor Cyan -NoNewline
Write-Host "         start coding          |" -ForegroundColor DarkGray
Ln "  +--------------------------------------------+" DarkGray
Write-Host ""

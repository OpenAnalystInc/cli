# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$ConfigDir = "$env:USERPROFILE\.openanalyst"

# Fixed-width row: exactly 43 chars content + padding to align |
$Border = "  +--------------------------------------------+"
$Blank  = "  |                                            |"
function Row([string]$t) { Write-Host "  | $($t.PadRight(43))|" -ForegroundColor DarkGray }

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

# ── Header ──
Write-Host ""
Write-Host $Border -ForegroundColor DarkGray
Write-Host $Blank -ForegroundColor DarkGray
Row "       ,---.  ,---."
Row "      / . . \/ ,-. \"
Row "     | |   | | |-| |"
Row "      \ ^-^ /\ ^-^ /"
Row "       '---'  '---'"
Write-Host $Blank -ForegroundColor DarkGray
Row "         OpenAnalyst CLI"
Row "      v$Version - Windows x64"
Write-Host $Blank -ForegroundColor DarkGray
Write-Host $Border -ForegroundColor DarkGray

# ── Setup ──
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# ── Step 1: Download ──
Write-Host $Blank -ForegroundColor DarkGray
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
        Row "   [1/3] Download .................. done"
    } catch {
        Row "   [1/3] Download .................. FAIL"
    }
} else {
    Row "   [1/3] Download .................. FAIL"
}

if (-not $Downloaded) {
    Write-Host $Blank -ForegroundColor DarkGray
    Row "   Download failed. Visit:"
    Row "   github.com/$Repo/releases"
    Write-Host $Blank -ForegroundColor DarkGray
    Write-Host $Border -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

# ── Step 2: PATH ──
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
}
Row "   [2/3] PATH ...................... done"

# ── Step 3: Config ──
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
Row "   [3/3] Config .................... done"

# ── Result ──
Write-Host $Blank -ForegroundColor DarkGray
Row "   Installed to ~/.openanalyst/bin"
Write-Host $Blank -ForegroundColor DarkGray
Write-Host $Border -ForegroundColor DarkGray
Write-Host $Blank -ForegroundColor DarkGray
Row "   openanalyst login   authenticate"
Row "   openanalyst         start coding"
Write-Host $Blank -ForegroundColor DarkGray
Write-Host $Border -ForegroundColor DarkGray
Write-Host ""

# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$ConfigDir = "$env:USERPROFILE\.openanalyst"

# ── Helpers ──
function Row($text, $fg) {
    $pad = 42 - $text.Length; if ($pad -lt 0) { $pad = 0 }
    Write-Host "  | " -ForegroundColor DarkGray -NoNewline
    Write-Host "$text$(" " * $pad)" -ForegroundColor $fg -NoNewline
    Write-Host " |" -ForegroundColor DarkGray
}
function Row2($a, $af, $b, $bf) {
    $pad = 42 - $a.Length - $b.Length; if ($pad -lt 0) { $pad = 0 }
    Write-Host "  | " -ForegroundColor DarkGray -NoNewline
    Write-Host $a -ForegroundColor $af -NoNewline
    Write-Host "$(" " * $pad)" -ForegroundColor DarkGray -NoNewline
    Write-Host $b -ForegroundColor $bf -NoNewline
    Write-Host " |" -ForegroundColor DarkGray
}
function Line { Write-Host "  +--------------------------------------------+" -ForegroundColor DarkGray }
function Empty { Write-Host "  |                                            |" -ForegroundColor DarkGray }

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
Line; Empty
Row "        OpenAnalyst CLI" White
Row "        v$Version - Windows x64" DarkGray
Empty; Line

# ── Create directories ──
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# ── Step 1: Download ──
Empty
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
        Row2 "  [1/3] Download ...................." DarkGray "done" Green
    } catch {
        Row2 "  [1/3] Download ...................." DarkGray "FAIL" Red
    }
} else {
    Row2 "  [1/3] Download ...................." DarkGray "FAIL" Red
}

if (-not $Downloaded) {
    Empty
    Row "  Download failed. Visit:" Red
    Row "  github.com/$Repo/releases" White
    Empty; Line
    Write-Host ""
    exit 1
}

# ── Step 2: PATH ──
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
}
Row2 "  [2/3] PATH ........................" DarkGray "done" Green

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
Row2 "  [3/3] Config ......................" DarkGray "done" Green

# ── Result ──
Empty
Row "  Installed to ~/.openanalyst/bin" Green
Empty; Line

# ── Footer ──
Empty
Row2 "  openanalyst login" Cyan "authenticate" DarkGray
Row2 "  openanalyst" Cyan "start coding" DarkGray
Empty; Line
Write-Host ""

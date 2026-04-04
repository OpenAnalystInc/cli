# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/AnitChaudhry/openanalyst-cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "AnitChaudhry/openanalyst-cli"
$BinaryName = "openanalyst.exe"
$InstallDir = "$env:USERPROFILE\.openanalyst\bin"
$ConfigDir = "$env:USERPROFILE\.openanalyst"

# ── Colors ──
function W([string]$t) { Write-Host $t -NoNewline -ForegroundColor White }
function WB([string]$t) { Write-Host $t -NoNewline -ForegroundColor White; }
function Dim([string]$t) { Write-Host $t -NoNewline -ForegroundColor DarkGray }
function Br([string]$t) { Write-Host $t -NoNewline -ForegroundColor DarkCyan }
function Grn([string]$t) { Write-Host $t -NoNewline -ForegroundColor Green }
function Yl([string]$t) { Write-Host $t -NoNewline -ForegroundColor Yellow }
function Acc([string]$t) { Write-Host $t -NoNewline -ForegroundColor Cyan }
function Nl { Write-Host "" }

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
Nl; Nl
Dim "   "; Br "########  "; Acc "  ####   "; Nl
Dim "   "; Br "##    ##  "; Acc " ##  ##  "; Nl
Dim "   "; Br "##    ##  "; Acc "##    ## "; Nl
Dim "   "; Br "##    ##  "; Acc "######## "; Nl
Dim "   "; Br "##    ##  "; Acc "##    ## "; Nl
Dim "   "; Br "########  "; Acc "##    ## "; Nl
Nl
Write-Host "   " -NoNewline
Write-Host "OpenAnalyst CLI" -NoNewline -ForegroundColor White
Dim "  v$Version"
Nl
Dim "   The Universal AI Agent for Your Terminal"
Nl; Nl
Dim "   "; Dim ([char]0x2500 * 44); Nl; Nl

# ── System info ──
$OsLabel = "Windows"
$ArchLabel = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }

Dim "   "; Dim ([char]0x250C); Dim ([char]0x2500 * 42); Dim ([char]0x2510); Nl
Dim "   "; Dim ([char]0x2502); W "  System       "; Dim "$OsLabel $ArchLabel"; Dim (" " * (42 - 15 - "$OsLabel $ArchLabel".Length)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); W "  Install to   "; Dim "~\.openanalyst\bin"; Dim (" " * (42 - 15 - 18)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); W "  Config at    "; Dim "~\.openanalyst"; Dim (" " * (42 - 15 - 14)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2514); Dim ([char]0x2500 * 42); Dim ([char]0x2518); Nl
Nl

# ── Setup ──
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# ── Step 1: Download ──
$Downloaded = $false
Get-Process -Name "openanalyst" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

Dim "   "; Acc ([char]0x203A); Dim " Downloading binary..."
if ($Release) {
    $Asset = $Release.assets | Where-Object { $_.name -eq "openanalyst.exe" } | Select-Object -First 1
    if (-not $Asset) { $Asset = $Release.assets | Where-Object { $_.name -like "*.exe" } | Select-Object -First 1 }
    if ($Asset -and $Asset.browser_download_url) { $AssetUrl = $Asset.browser_download_url }
    else { $AssetUrl = "https://github.com/$Repo/releases/download/$($Release.tag_name)/openanalyst.exe" }
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile "$InstallDir\$BinaryName" -UseBasicParsing -ErrorAction Stop
        $Downloaded = $true
        Write-Host " " -NoNewline; Grn ([char]0x2713); Dim " done"; Nl
    } catch {
        Write-Host " " -NoNewline; Yl "failed"; Nl
    }
} else {
    Write-Host " " -NoNewline; Yl "no release found"; Nl
}

if (-not $Downloaded) {
    Nl
    Yl "   Download failed. Visit:"
    Nl
    Acc "   github.com/$Repo/releases"
    Nl; Nl
    exit 1
}

# ── Step 2: PATH ──
Dim "   "; Acc ([char]0x203A); Dim " Configuring PATH..."
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$CurrentPath", "User")
    $env:Path = "$InstallDir;$env:Path"
    Write-Host " " -NoNewline; Grn ([char]0x2713); Dim " added"; Nl
} else {
    Write-Host " " -NoNewline; Grn ([char]0x2713); Dim " already configured"; Nl
}

# ── Step 3: Config ──
Dim "   "; Acc ([char]0x203A); Dim " Creating config..."
$EnvFile = "$ConfigDir\.env"
if (-not (Test-Path $EnvFile)) {
    @"
# OpenAnalyst CLI — Environment Configuration
# OPENANALYST_API_KEY=
# ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...
# GEMINI_API_KEY=AIza...
"@ | Out-File -FilePath $EnvFile -Encoding utf8
    Write-Host " " -NoNewline; Grn ([char]0x2713); Dim " ~\.openanalyst\.env"; Nl
} else {
    Write-Host " " -NoNewline; Grn ([char]0x2713); Dim " already exists"; Nl
}

# ── Summary ──
Nl; Nl
Dim "   "; Dim ([char]0x2500 * 44); Nl; Nl
Grn "   "; Write-Host "$([char]0x2713) Installation complete" -ForegroundColor Green
Nl; Nl

Dim "   "; Dim ([char]0x250C); Dim ([char]0x2500 * 42); Dim ([char]0x2510); Nl
Dim "   "; Dim ([char]0x2502); Dim (" " * 42); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); W "  Binary     "; Dim "~\.openanalyst\bin\openanalyst.exe"; Dim (" " * (42 - 13 - 33)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); W "  Config     "; Dim "~\.openanalyst\.env"; Dim (" " * (42 - 13 - 19)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); W "  Version    "; Dim "v$Version"; Dim (" " * (42 - 13 - "v$Version".Length)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); Dim (" " * 42); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2514); Dim ([char]0x2500 * 42); Dim ([char]0x2518); Nl
Nl

# ── Next steps ──
Write-Host "   Next steps" -ForegroundColor White
Nl
Acc "   1."; W " Login to your LLM provider"; Nl
Nl
Acc "      `$ openanalyst login"; Nl
Nl
Dim "      Select a provider, use the free model"
Nl
Dim "      or paste your API key."
Nl; Nl

Acc "   2."; W " Start coding"; Nl
Nl
Acc "      `$ openanalyst"; Nl
Nl; Nl

# ── Provider list ──
Dim "   "; Dim ([char]0x250C); Dim ([char]0x2500 * 42); Dim ([char]0x2510); Nl
Dim "   "; Dim ([char]0x2502); Dim (" " * 42); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); W "  7 LLM Providers. One Interface."; Dim (" " * (42 - 34)); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2502); Dim (" " * 42); Dim ([char]0x2502); Nl
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
    $desc = $p[1]
    $line = "  "
    Dim "   "; Dim ([char]0x2502)
    Acc "  $([char]0x25A0)"
    W " $name"
    if ($desc) {
        $pad = 42 - 4 - $name.Length - 3 - $desc.Length
        if ($pad -lt 0) { $pad = 1 }
        Dim (" " * $pad)
        Dim $desc
    } else {
        $pad = 42 - 4 - $name.Length
        if ($pad -lt 0) { $pad = 1 }
        Dim (" " * $pad)
    }
    Dim ([char]0x2502); Nl
}
Dim "   "; Dim ([char]0x2502); Dim (" " * 42); Dim ([char]0x2502); Nl
Dim "   "; Dim ([char]0x2514); Dim ([char]0x2500 * 42); Dim ([char]0x2518); Nl
Nl

Dim "   Documentation:  "
Acc "github.com/AnitChaudhry/openanalyst-cli"
Nl
Dim "   Support:        "
Acc "anit@openanalyst.com"
Nl; Nl

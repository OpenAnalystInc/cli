# OpenAnalyst CLI Installer - Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$PackageName = "@openanalystinc/openanalyst-cli"
$LegacyPackageName = "@openanalyst/cli"

function Test-OpenAnalystShim {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }

    try {
        $Content = Get-Content -LiteralPath $Path -Raw -ErrorAction Stop
    } catch {
        return $false
    }

    return $Content -match '@openanalyst[\\/]+cli' `
        -or $Content -match '@openanalystinc[\\/]+openanalyst-cli' `
        -or $Content -match 'node_modules[\\/]+@openanalyst'
}

function Repair-OpenAnalystGlobalBin {
    $Prefix = $null
    try { $Prefix = (npm prefix -g 2>$null).Trim() } catch {}
    if (-not $Prefix) { return }

    try {
        $ResolvedPrefix = [System.IO.Path]::GetFullPath($Prefix)
        $PrefixRoot = $ResolvedPrefix.TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    } catch {
        return
    }

    foreach ($Name in @("openanalyst", "openanalyst.cmd", "openanalyst.ps1")) {
        $Candidate = Join-Path $ResolvedPrefix $Name
        try { $FullPath = [System.IO.Path]::GetFullPath($Candidate) } catch { continue }
        if (-not $FullPath.StartsWith($PrefixRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            continue
        }

        if (Test-OpenAnalystShim $FullPath) {
            try {
                Remove-Item -LiteralPath $FullPath -Force -ErrorAction Stop
            } catch {
                Write-Host "   Could not remove stale shim: $FullPath" -ForegroundColor Yellow
            }
        }
    }
}

Write-Host ""
Write-Host "   OpenAnalyst CLI" -ForegroundColor DarkCyan
Write-Host "   The Universal AI Agent for Your Terminal" -ForegroundColor DarkGray
Write-Host ""

# Check for Node.js
$NodeVersion = $null
try { $NodeVersion = (node --version 2>$null) } catch {}

if (-not $NodeVersion) {
    Write-Host "   Node.js is required but not found." -ForegroundColor Red
    Write-Host "   Install from: https://nodejs.org/" -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

Write-Host "   Node.js $NodeVersion detected" -ForegroundColor DarkGray

# Repair legacy global installs before npm links the new binary.
Repair-OpenAnalystGlobalBin
try { npm uninstall -g $LegacyPackageName $PackageName --silent 2>&1 | Out-Null } catch {}
Repair-OpenAnalystGlobalBin

# Install via npm
Write-Host ""
Write-Host "   Installing..." -ForegroundColor White -NoNewline

try {
    npm install -g "$PackageName@latest" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "npm install failed" }
    Write-Host " done" -ForegroundColor Green
} catch {
    Write-Host " failed" -ForegroundColor Red
    Write-Host "   Try manually:" -ForegroundColor DarkGray
    Write-Host "     npm uninstall -g $LegacyPackageName $PackageName" -ForegroundColor DarkGray
    Write-Host "     npm install -g $PackageName@latest" -ForegroundColor DarkGray
    exit 1
}

# Verify
$Version = $null
try { $Version = (openanalyst --version 2>$null) } catch {}

Write-Host ""
if ($Version) {
    Write-Host "   $([char]0x2713) $Version" -ForegroundColor Green
} else {
    Write-Host "   $([char]0x2713) Installed" -ForegroundColor Green
}

Write-Host ""
Write-Host "   To get started:" -ForegroundColor White
Write-Host ""
Write-Host "     openanalyst" -ForegroundColor Cyan
Write-Host ""

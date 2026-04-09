# OpenAnalyst CLI Installer — Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

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

# Install via npm
Write-Host ""
Write-Host "   Installing..." -ForegroundColor White -NoNewline

try {
    npm install -g @openanalystinc/openanalyst-cli@latest 2>&1 | Out-Null
    Write-Host " done" -ForegroundColor Green
} catch {
    Write-Host " failed" -ForegroundColor Red
    Write-Host "   Try manually: npm install -g @openanalystinc/openanalyst-cli" -ForegroundColor DarkGray
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

# OpenAnalyst CLI Installer - Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$repo = "OpenAnalystInc/cli"
$asset = "openanalyst-x86_64-pc-windows-msvc.exe"
$downloadUrl = "https://github.com/$repo/releases/latest/download/$asset"
$installDir = if ($env:OPENANALYST_INSTALL_DIR) { $env:OPENANALYST_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".openanalyst\bin" }
$installPath = Join-Path $installDir "openanalyst.exe"

Write-Host ""
Write-Host "   OpenAnalyst CLI" -ForegroundColor DarkCyan
Write-Host "   The Universal AI Agent for Your Terminal" -ForegroundColor DarkGray
Write-Host ""

if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
}

Write-Host "   Downloading $asset" -ForegroundColor DarkGray
Invoke-WebRequest -Uri $downloadUrl -OutFile $installPath

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @()
if ($userPath) {
    $pathEntries = $userPath.Split(';') | Where-Object { $_ }
}

if ($pathEntries -notcontains $installDir) {
    $newPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = "$env:Path;$installDir"
    Write-Host "   Added $installDir to your user PATH" -ForegroundColor DarkGray
} else {
    $env:Path = "$env:Path;$installDir"
}

Write-Host ""
Write-Host "   Installed to $installPath" -ForegroundColor Green

Write-Host ""
try {
    $version = & $installPath --version
    Write-Host "   OK  $version" -ForegroundColor Green
} catch {
    Write-Host "   Installed, but version check did not complete." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "   To get started:" -ForegroundColor White
Write-Host ""
Write-Host "     openanalyst" -ForegroundColor Cyan
Write-Host "     openanalyst --notui" -ForegroundColor Cyan
Write-Host "     openanalyst --serve 8080" -ForegroundColor Cyan
Write-Host ""

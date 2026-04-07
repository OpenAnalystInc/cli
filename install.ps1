# OpenAnalyst CLI Installer - Windows PowerShell
# Usage: irm https://raw.githubusercontent.com/OpenAnalystInc/cli/main/install.ps1 | iex

$ErrorActionPreference = "Continue"
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}

$Repo = "OpenAnalystInc/cli"
$BaseUrl = "https://github.com/$Repo/releases/latest/download"

Write-Host ""
Write-Host "   OpenAnalyst CLI" -ForegroundColor DarkCyan
Write-Host "   The Universal AI Agent for Your Terminal" -ForegroundColor DarkGray
Write-Host ""

$Arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()) {
    "X64" { "x86_64" }
    "Arm64" { "aarch64" }
    default { $null }
}

if (-not $Arch) {
    Write-Host "   Unsupported architecture for automatic install." -ForegroundColor Red
    Write-Host "   Download a release manually: https://github.com/$Repo/releases/latest" -ForegroundColor DarkGray
    Write-Host ""
    exit 1
}

$Asset = "openanalyst-$Arch-pc-windows-msvc.exe"
$DownloadUrl = "$BaseUrl/$Asset"
$InstallDir = Join-Path $env:USERPROFILE ".openanalyst\bin"
$InstallPath = Join-Path $InstallDir "openanalyst.exe"

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

Write-Host "   Download target: $Asset" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   Downloading..." -ForegroundColor White -NoNewline
try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $InstallPath -UseBasicParsing
    Write-Host " done" -ForegroundColor Green
} catch {
    Write-Host " failed" -ForegroundColor Red
    Write-Host "   Download a release manually: https://github.com/$Repo/releases/latest" -ForegroundColor DarkGray
    exit 1
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
$PathParts = @()
if ($UserPath) {
    $PathParts = $UserPath -split ';' | Where-Object { $_ }
}
if ($PathParts -notcontains $InstallDir) {
    $NewPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    $env:Path = "$env:Path;$InstallDir"
}

$Version = $null
try { $Version = (& $InstallPath --version 2>$null) } catch {}

Write-Host ""
if ($Version) {
    Write-Host "   $([char]0x2713) $Version" -ForegroundColor Green
} else {
    Write-Host "   $([char]0x2713) Installed" -ForegroundColor Green
}

Write-Host ""
Write-Host "   Installed to: $InstallPath" -ForegroundColor DarkGray
Write-Host ""
Write-Host "   To get started:" -ForegroundColor White
Write-Host ""
Write-Host "     openanalyst" -ForegroundColor Cyan
Write-Host ""

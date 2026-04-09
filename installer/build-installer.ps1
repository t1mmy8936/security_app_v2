# ═══════════════════════════════════════════════════════════════
# Watchtower — Build Installer
# ═══════════════════════════════════════════════════════════════
#
# Prerequisites:
#   - Inno Setup 6 installed (https://jrsoftware.org/isdl.php)
#
# Usage:
#   .\installer\build-installer.ps1
#
# Output:
#   dist\Watchtower-Setup.exe
# ═══════════════════════════════════════════════════════════════

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$projectRoot = Split-Path -Parent $scriptDir

Write-Host ""
Write-Host "  Building Watchtower Installer..." -ForegroundColor Cyan
Write-Host ""

# Find Inno Setup compiler
$isccPaths = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 5\ISCC.exe"
)

$iscc = $null
foreach ($p in $isccPaths) {
    if (Test-Path $p) {
        $iscc = $p
        break
    }
}

if (-not $iscc) {
    Write-Host "  [X] Inno Setup not found." -ForegroundColor Red
    Write-Host ""
    Write-Host "  Install Inno Setup 6 from: https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
    Write-Host "  Then run this script again." -ForegroundColor Yellow
    Write-Host ""
    exit 1
}

Write-Host "  Found Inno Setup: $iscc" -ForegroundColor Green

# Create dist directory
$distDir = Join-Path $projectRoot "dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir -Force | Out-Null
}

# Build the installer
$issFile = Join-Path $scriptDir "watchtower-setup.iss"
Write-Host "  Compiling installer..." -ForegroundColor Yellow

& $iscc $issFile

if ($LASTEXITCODE -eq 0) {
    $exe = Join-Path $distDir "Watchtower-Setup.exe"
    $size = (Get-Item $exe).Length / 1MB
    Write-Host ""
    Write-Host "  Installer built successfully!" -ForegroundColor Green
    Write-Host "  Output: $exe ($([math]::Round($size, 2)) MB)" -ForegroundColor Cyan
    Write-Host ""
} else {
    Write-Host ""
    Write-Host "  [X] Installer build failed." -ForegroundColor Red
    Write-Host ""
    exit 1
}

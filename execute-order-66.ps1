# ═══════════════════════════════════════════════════════════════
# ⚔️  Watchtower — Execute Order 66 (Rust Edition)
# ═══════════════════════════════════════════════════════════════

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "  A long time ago, in a network far, far away...." -ForegroundColor DarkGray
Write-Host ""
Start-Sleep -Milliseconds 1200
Write-Host "  ╔══════════════════════════════════════════════════════════╗" -ForegroundColor Yellow
Write-Host "  ║                                                          ║" -ForegroundColor Yellow
Write-Host "  ║        ░██████╗  ██████╗ █████╗ ███╗  ██╗               ║" -ForegroundColor Yellow
Write-Host "  ║        ██╔════╝ ██╔════╝██╔══██╗████╗ ██║               ║" -ForegroundColor Yellow
Write-Host "  ║        ╚█████╗  ██║     ███████║██╔██╗██║               ║" -ForegroundColor Yellow
Write-Host "  ║         ╚═══██╗ ██║     ██╔══██║██║╚████║               ║" -ForegroundColor Yellow
Write-Host "  ║        ██████╔╝ ╚██████╗██║  ██║██║ ╚███║               ║" -ForegroundColor Yellow
Write-Host "  ║        ╚═════╝   ╚═════╝╚═╝  ╚═╝╚═╝  ╚══╝               ║" -ForegroundColor Yellow
Write-Host "  ║                                                          ║" -ForegroundColor Yellow
Write-Host "  ║              EXECUTE ORDER 66                           ║" -ForegroundColor Red
Write-Host "  ║           ── INITIATING SECURITY SCAN ──                ║" -ForegroundColor DarkRed
Write-Host "  ║                                                          ║" -ForegroundColor Yellow
Write-Host "  ║   `"The Force is strong with this vulnerability.`"        ║" -ForegroundColor DarkYellow
Write-Host "  ║                                                          ║" -ForegroundColor Yellow
Write-Host "  ╚══════════════════════════════════════════════════════════╝" -ForegroundColor Yellow
Write-Host ""
Start-Sleep -Milliseconds 800

# ── Find project directory ──
$scriptDir = $PSScriptRoot
if (-not $scriptDir) {
    $desktopPath = [Environment]::GetFolderPath("Desktop")
    $scriptDir = Join-Path $desktopPath "Watchtower"
}

if (-not (Test-Path (Join-Path $scriptDir "Cargo.toml"))) {
    Write-Host "  [X] Cannot find Cargo.toml in $scriptDir" -ForegroundColor Red
    exit 1
}

Set-Location $scriptDir
Write-Host "  📂 Project: $scriptDir" -ForegroundColor Cyan

# ── Check Docker ──
$dockerPaths = @(
    "C:\Program Files\Docker\Docker\resources\bin",
    "$env:LOCALAPPDATA\Docker\wsl\docker-cli-tools"
)
foreach ($p in $dockerPaths) {
    if ((Test-Path $p) -and ($env:PATH -notlike "*$p*")) {
        $env:PATH = "$p;$env:PATH"
    }
}

try {
    $null = docker info 2>&1
    Write-Host "  ✅ Imperial fleet (Docker) is ready" -ForegroundColor Green
} catch {
    Write-Host "  [X] The fleet is grounded. Start Docker Desktop first." -ForegroundColor Red
    exit 1
}

# ── Set up .env ──
if (-not (Test-Path ".env")) {
    $localProjects = "C:\Users\$env:USERNAME\source\repos"
    if (-not (Test-Path $localProjects)) { $localProjects = "C:\Projects" }
    "LOCAL_PROJECTS_PATH=$localProjects" | Out-File -FilePath ".env" -Encoding UTF8
    Write-Host "  📝 Holocron created: LOCAL_PROJECTS_PATH=$localProjects" -ForegroundColor Yellow
} else {
    Write-Host "  📝 Holocron found" -ForegroundColor Green
}

# ── Detect drives and generate docker-compose.override.yml ──
$drives = Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Root -match '^[A-Z]:\\$' }
$overrideLines = @(
    "services:",
    "  watchtower:",
    "    volumes:"
)
foreach ($drv in $drives) {
    $letter = $drv.Name.ToLower()
    $overrideLines += "      - `"$($drv.Name):/:/host-$($letter):ro`""
}
$overrideContent = $overrideLines -join "`n"
$overrideContent | Out-File -FilePath "docker-compose.override.yml" -Encoding UTF8 -Force
$driveLetters = ($drives | ForEach-Object { $_.Name }) -join ", "
Write-Host "  💾 Scanning galaxy sectors: $driveLetters" -ForegroundColor Cyan

# ── Build and launch ──
Write-Host ""
Write-Host "  ⚡ The Death Star scans are commencing..." -ForegroundColor Yellow
Write-Host "  🛸 Deploying rebel-hunting containers..." -ForegroundColor DarkYellow
Write-Host ""

docker compose up --build -d

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "  ╔══════════════════════════════════════════════════════════╗" -ForegroundColor Yellow
    Write-Host "  ║  ⚡ ORDER 66 EXECUTED — SCAN GRID ONLINE                 ║" -ForegroundColor Yellow
    Write-Host "  ║                                                          ║" -ForegroundColor Yellow
    Write-Host "  ║  🌐 Command Bridge:  http://localhost:66                  ║" -ForegroundColor Cyan
    Write-Host "  ║  🔍 Probe Droid:     http://localhost:8081                ║" -ForegroundColor Cyan
    Write-Host "  ║  📊 Intel Station:   http://localhost:9091                ║" -ForegroundColor Cyan
    Write-Host "  ║                                                          ║" -ForegroundColor Yellow
    Write-Host "  ║  `"Your scan has been set in motion, my lord.`"          ║" -ForegroundColor DarkYellow
    Write-Host "  ║                                                          ║" -ForegroundColor Yellow
    Write-Host "  ╚══════════════════════════════════════════════════════════╝" -ForegroundColor Yellow
    Write-Host ""
} else {
    Write-Host ""
    Write-Host "  [X] The scan has failed. Check docker compose logs, Lord Vader." -ForegroundColor Red
    Write-Host ""
}

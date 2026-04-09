# ═══════════════════════════════════════════════════════════════
# ⚔️  Watchtower — Execute Order 66 (Rust Edition)
# ═══════════════════════════════════════════════════════════════

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════════════════╗" -ForegroundColor Red
Write-Host "  ║                                                          ║" -ForegroundColor Red
Write-Host "  ║   ⚔️  Watchtower — Execute Order 66                     ║" -ForegroundColor Red
Write-Host "  ║        Rust Edition                                      ║" -ForegroundColor Red
Write-Host "  ║                                                          ║" -ForegroundColor Red
Write-Host "  ║   `"I find your lack of security disturbing.`"            ║" -ForegroundColor DarkRed
Write-Host "  ║                                                          ║" -ForegroundColor Red
Write-Host "  ╚══════════════════════════════════════════════════════════╝" -ForegroundColor Red
Write-Host ""

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
    Write-Host "  ✅ Docker is running" -ForegroundColor Green
} catch {
    Write-Host "  [X] Docker is not running. Start Docker Desktop first." -ForegroundColor Red
    exit 1
}

# ── Set up .env ──
if (-not (Test-Path ".env")) {
    $localProjects = "C:\Users\$env:USERNAME\source\repos"
    if (-not (Test-Path $localProjects)) { $localProjects = "C:\Projects" }
    "LOCAL_PROJECTS_PATH=$localProjects" | Out-File -FilePath ".env" -Encoding UTF8
    Write-Host "  📝 Created .env with LOCAL_PROJECTS_PATH=$localProjects" -ForegroundColor Yellow
} else {
    Write-Host "  📝 .env exists" -ForegroundColor Green
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
Write-Host "  💾 Detected drives: $driveLetters" -ForegroundColor Cyan

# ── Build and launch ──
Write-Host ""
Write-Host "  🚀 Executing Order 66... Building containers..." -ForegroundColor Yellow
Write-Host ""

docker compose up --build -d

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "  ╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "  ║  ✅ Watchtower is online!                                ║" -ForegroundColor Green
    Write-Host "  ║                                                          ║" -ForegroundColor Green
    Write-Host "  ║  🌐 Dashboard:  http://localhost:67                      ║" -ForegroundColor Cyan
    Write-Host "  ║  🔍 ZAP:        http://localhost:8081                    ║" -ForegroundColor Cyan
    Write-Host "  ║  📊 SonarQube:  http://localhost:9091                    ║" -ForegroundColor Cyan
    Write-Host "  ║                                                          ║" -ForegroundColor Green
    Write-Host "  ║  Built with Rust 🦀 + Leptos + Actix-web                ║" -ForegroundColor DarkYellow
    Write-Host "  ╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host ""
    Write-Host "  [X] Build failed. Check docker compose logs." -ForegroundColor Red
    Write-Host ""
}

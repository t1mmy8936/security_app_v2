# ═══════════════════════════════════════════════════════════════
# Watchtower — Server Setup Script
# Installs Docker Desktop + Azure DevOps self-hosted agent
#
# Run as Administrator on the target Windows server:
#   .\installer\setup-server-agent.ps1 `
#       -AzdoOrg    "https://dev.azure.com/your-org" `
#       -AzdoToken  "your-PAT-here" `
#       -AgentPool  "watchtower-server"
# ═══════════════════════════════════════════════════════════════

param(
    [Parameter(Mandatory)][string] $AzdoOrg,
    [Parameter(Mandatory)][string] $AzdoToken,
    [string] $AgentPool  = "watchtower-server",
    [string] $AgentName  = $env:COMPUTERNAME,
    [string] $AgentDir   = "C:\azure-agent",
    [string] $DeployPath = "C:\watchtower"
)

$ErrorActionPreference = "Stop"

function Write-Step([string]$msg) {
    Write-Host ""
    Write-Host "  ── $msg" -ForegroundColor Cyan
}

# Must run as admin
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "  [X] Run this script as Administrator." -ForegroundColor Red
    exit 1
}

# ── 1. Install Winget if missing ─────────────────────────────────────────────
Write-Step "Checking winget"
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host "  winget not found — installing App Installer from Microsoft Store..." -ForegroundColor Yellow
    Start-Process "ms-appinstaller:?source=https://aka.ms/getwinget"
    Read-Host "  Press Enter once the App Installer install is complete"
}
Write-Host "  winget OK" -ForegroundColor Green

# ── 2. Install Docker Desktop ────────────────────────────────────────────────
Write-Step "Installing Docker Desktop"
$dockerRunning = $false
try { $null = docker info 2>&1; $dockerRunning = $true } catch {}

if ($dockerRunning) {
    Write-Host "  Docker already running — skipping install" -ForegroundColor Green
} else {
    winget install -e --id Docker.DockerDesktop --accept-source-agreements --accept-package-agreements
    Write-Host ""
    Write-Host "  !! Docker Desktop installed." -ForegroundColor Yellow
    Write-Host "  !! Start Docker Desktop once, accept the licence, then re-run this script." -ForegroundColor Yellow
    Write-Host "  !! Exiting for now." -ForegroundColor Yellow
    exit 0
}

# ── 3. Install Inno Setup (needed by the pipeline to build the installer) ────
Write-Step "Installing Inno Setup 6"
$iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
if (Test-Path $iscc) {
    Write-Host "  Inno Setup already installed" -ForegroundColor Green
} else {
    winget install -e --id JRSoftware.InnoSetup --accept-source-agreements --accept-package-agreements
    Write-Host "  Inno Setup installed" -ForegroundColor Green
}

# ── 4. Create deploy folder ──────────────────────────────────────────────────
Write-Step "Creating deploy folder: $DeployPath"
if (-not (Test-Path $DeployPath)) {
    New-Item -ItemType Directory -Path $DeployPath -Force | Out-Null
}
Write-Host "  $DeployPath ready" -ForegroundColor Green

# ── 4. Download & install Azure Pipelines agent ──────────────────────────────
Write-Step "Installing Azure DevOps agent → $AgentDir"

if (Test-Path (Join-Path $AgentDir "config.cmd")) {
    Write-Host "  Agent already configured at $AgentDir — skipping download" -ForegroundColor Green
} else {
    # Get latest agent version from Azure DevOps REST API
    $headers = @{ Authorization = "Basic " + [Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes(":$AzdoToken")) }
    $pkgUrl = "https://vstsagentpackage.azureedge.net/agent/4.248.0/vsts-agent-win-x64-4.248.0.zip"
    $zipPath = "$env:TEMP\azure-agent.zip"

    Write-Host "  Downloading agent..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $pkgUrl -OutFile $zipPath -UseBasicParsing

    if (-not (Test-Path $AgentDir)) {
        New-Item -ItemType Directory -Path $AgentDir -Force | Out-Null
    }

    Write-Host "  Extracting agent..." -ForegroundColor Yellow
    Expand-Archive -Path $zipPath -DestinationPath $AgentDir -Force
    Remove-Item $zipPath

    # Configure the agent
    Write-Host "  Configuring agent..." -ForegroundColor Yellow
    $configCmd = Join-Path $AgentDir "config.cmd"
    & $configCmd `
        --unattended `
        --url        $AzdoOrg `
        --auth       pat `
        --token      $AzdoToken `
        --pool       $AgentPool `
        --agent      $AgentName `
        --work       "_work" `
        --runAsService `
        --windowsLogonAccount "NT AUTHORITY\SYSTEM"

    if ($LASTEXITCODE -ne 0) {
        Write-Host "  [X] Agent configuration failed." -ForegroundColor Red
        exit 1
    }
}

# ── 5. Start agent service ───────────────────────────────────────────────────
Write-Step "Starting agent service"
$svc = Get-Service -Name "vstsagent*" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($svc) {
    if ($svc.Status -ne "Running") {
        Start-Service $svc.Name
    }
    Write-Host "  Service '$($svc.Name)' is $($svc.Status)" -ForegroundColor Green
} else {
    Write-Host "  No vstsagent service found — the agent may need to be started manually." -ForegroundColor Yellow
}

# ── Done ─────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "  ╔══════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "  ║  ✅ Server setup complete!                               ║" -ForegroundColor Green
Write-Host "  ║                                                          ║" -ForegroundColor Green
Write-Host "  ║  Agent pool : $AgentPool" -ForegroundColor Cyan
Write-Host "  ║  Agent name : $AgentName" -ForegroundColor Cyan
Write-Host "  ║  Deploy path: $DeployPath" -ForegroundColor Cyan
Write-Host "  ║                                                          ║" -ForegroundColor Green
Write-Host "  ║  Push to main → pipeline checks → auto-deploys here.    ║" -ForegroundColor Green
Write-Host "  ╚══════════════════════════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""

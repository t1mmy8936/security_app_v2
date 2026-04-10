# Watchtower — Security Testing Suite

A full-stack cybersecurity scanning suite built with **Rust**, **Leptos**, and **Actix-web**, deployed via **Docker**.

---

## Prerequisites

- **Docker Desktop** — [Download](https://docker.com/products/docker-desktop)
- **PowerShell 5.1+** (included with Windows 10/11)

---

## Quick Start

### Option 1: Execute Order 66 (automated)

```powershell
.\execute-order-66.ps1
```

This auto-detects drives, creates the `.env` file, and builds/starts all containers.

### Option 2: Manual startup

**1. Create a `.env` file** in the project root with the path to your local source code:

```
LOCAL_PROJECTS_PATH=C:\Users\YourName\source\repos
```

This is mounted read-only into the container at `/projects` so scanners can analyse your code.

**2. Start Docker Desktop** and make sure it's running.

**3. Build and start the containers:**

```powershell
docker compose up --build -d
```

**4. Open the dashboard:**

- **Watchtower:** [http://localhost:67](http://localhost:67)
- **ZAP Proxy:** [http://localhost:8081](http://localhost:8081)
- **SonarQube:** [http://localhost:9091](http://localhost:9091) (default login: `admin` / `admin`)

### Stopping the app

```powershell
docker compose down
```

To also remove scan data and volumes:

```powershell
docker compose down -v
```

---

## Containers

| Service    | Port  | Description                          |
|------------|-------|--------------------------------------|
| watchtower | 67    | Main app (dashboard, API, scanners)  |
| zap        | 8081  | OWASP ZAP proxy (daemon mode)       |
| sonarqube  | 9091  | SonarQube Community Edition          |

---

## Integrated Scanners

- **Nmap** — Network/port scanning
- **Nikto** — Web server vulnerability scanning
- **SQLMap** — SQL injection testing
- **Trivy** — Container & dependency vulnerability scanning
- **Bandit** — Python static analysis
- **OWASP ZAP** — Dynamic web application scanning
- **SonarQube** — Code quality & security analysis
- **OWASP Dependency-Check** — Known vulnerability detection in dependencies

---

## Project Structure

```
├── Cargo.toml              # Rust project config
├── Dockerfile              # Multi-stage build (Rust + tools)
├── docker-compose.yml      # Container orchestration
├── execute-order-66.ps1    # Automated launcher
├── installer/              # Inno Setup installer scripts
├── public/                 # Static assets (favicon, JS)
├── style/                  # CSS
└── src/
    ├── main.rs             # Entrypoint
    ├── api.rs              # REST API endpoints (~35 routes)
    ├── db.rs               # SQLite database layer
    ├── models.rs           # Data models
    ├── frontend/           # Leptos SSR + WASM UI
    │   ├── app.rs          # Router & layout
    │   ├── components/     # Sidebar, status bar, badges, toasts
    │   └── pages/          # Dashboard, scans, results, settings, etc.
    ├── scanners/           # Scanner integrations
    │   ├── runner.rs       # Scan orchestrator (stop/resume support)
    │   └── *.rs            # Individual scanner modules
    └── services/           # Email, reports, Azure DevOps integration
```

---

## Building the Installer

To create a distributable `Watchtower-Setup.exe` (requires [Inno Setup 6+](https://jrsoftware.org/isdl.php)):

```powershell
.\installer\build-installer.ps1
```

Output: `dist\Watchtower-Setup.exe` — installs Watchtower and auto-downloads Docker Desktop if needed.

---

## Useful Commands

```powershell
# Rebuild after code changes
docker compose up --build -d

# View logs
docker compose logs -f watchtower

# Restart a single service
docker compose restart watchtower

# Check running containers
docker compose ps

# Shell into the watchtower container
docker compose exec watchtower bash
```

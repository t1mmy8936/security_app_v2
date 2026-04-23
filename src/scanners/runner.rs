use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

/// Cancellation tokens for running scans. Set to true = stop requested.
pub static SCAN_CANCEL: Lazy<Arc<RwLock<HashMap<i64, bool>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub web_only: bool,
}

pub static TOOLS: &[ToolDef] = &[
    ToolDef { name: "zap", display_name: "OWASP ZAP", description: "Web application security scanner", category: "DAST", web_only: true },
    ToolDef { name: "nmap", display_name: "Nmap", description: "Network discovery and security auditing", category: "Network", web_only: true },
    ToolDef { name: "nikto", display_name: "Nikto", description: "Web server vulnerability scanner", category: "DAST", web_only: true },
    ToolDef { name: "sqlmap", display_name: "SQLMap", description: "SQL injection detection and exploitation", category: "DAST", web_only: true },
    ToolDef { name: "bandit", display_name: "Bandit", description: "Python source code security analyzer", category: "SAST", web_only: false },
    ToolDef { name: "trivy", display_name: "Trivy", description: "Vulnerability scanner for containers/filesystems", category: "SCA", web_only: false },
    ToolDef { name: "sonarqube", display_name: "SonarQube", description: "Code quality and security analysis", category: "SAST", web_only: false },
    ToolDef { name: "dependency_check", display_name: "Dependency-Check", description: "OWASP dependency vulnerability scanner", category: "SCA", web_only: false },
    ToolDef { name: "openvas", display_name: "OpenVAS", description: "Network vulnerability scanner (Greenbone Community Edition)", category: "Network", web_only: true },
];

pub struct ScanPresetDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub tools: &'static [&'static str],
}

pub static PRESETS: &[ScanPresetDef] = &[
    ScanPresetDef { name: "web", display_name: "Web Application Scan", description: "Full web application security assessment", tools: &["zap", "nikto", "sqlmap", "nmap", "openvas"] },
    ScanPresetDef { name: "sast", display_name: "Static Analysis", description: "Source code security analysis", tools: &["bandit", "sonarqube"] },
    ScanPresetDef { name: "network", display_name: "Network Scan", description: "Network reconnaissance and vulnerability detection", tools: &["nmap"] },
    ScanPresetDef { name: "dependency", display_name: "Dependency Scan", description: "Check dependencies for known vulnerabilities", tools: &["trivy", "dependency_check"] },
    ScanPresetDef { name: "full", display_name: "Full Security Audit", description: "Comprehensive scan with all available tools", tools: &["zap", "nmap", "nikto", "sqlmap", "bandit", "trivy", "sonarqube", "dependency_check", "openvas"] },
];

const CATCHPHRASES: &[&str] = &[
    "I find your lack of security disturbing.",
    "The vulnerability is strong with this one.",
    "You underestimate the power of the dark side.",
    "I am altering the firewall. Pray I don't alter it further.",
    "The ability to destroy a server is insignificant next to the power of the Force.",
    "Perhaps you think you're being treated unfairly?",
    "Be careful not to choke on your insecure code, Director.",
    "He will join us or die, master.",
    "You have controlled your fear. Now, release your patches.",
    "I have brought peace, freedom, justice, and security to my new Empire.",
    "The circle is now complete. When I left you, I was but the learner.",
    "There is no escape. Don't make me destroy your build pipeline.",
    "You don't know the power of secure coding.",
    "It is useless to resist. Deploy the patch.",
    "All too easy.",
    "Impressive. Most impressive. But you are not a secure app yet.",
    "You are beaten. It is useless to resist.",
    "If you only knew the power of proper authentication.",
    "The Force is with you, young developer, but you are not secure yet.",
    "Apology accepted, Captain Needa.",
    "We would be honored if you would join us... in fixing these CVEs.",
    "I have you now.",
    "No disintegrations... just thorough scanning.",
    "What is thy bidding, my master? Scanning in progress.",
    "The Emperor is not as forgiving as I am. Patch your code.",
    "This will be a day long remembered. Multiple criticals found.",
];

fn is_url(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

fn filter_tools(tools: &[String], target_source: &str, target: &str) -> Vec<String> {
    let is_web = target_source == "url" || is_url(target);
    tools
        .iter()
        .filter(|t| {
            let def = TOOLS.iter().find(|d| d.name == t.as_str());
            match def {
                Some(d) => {
                    if is_web {
                        true
                    } else {
                        !d.web_only
                    }
                }
                None => false,
            }
        })
        .cloned()
        .collect()
}

pub async fn run_scan(pool: DbPool, scan_job_id: i64) {
    // Fetch the scan job
    let job = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT scan_type, target, target_source, tools_run FROM scan_jobs WHERE id = ?"
    )
    .bind(scan_job_id)
    .fetch_optional(&pool)
    .await;

    let (scan_type, target, target_source, tools_run_raw) = match job {
        Ok(Some(j)) => j,
        _ => return,
    };

    // Determine which tools to run
    let requested_tools: Vec<String> = if let Some(ref raw) = tools_run_raw {
        if !raw.is_empty() {
            raw.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            preset_tools(&scan_type)
        }
    } else {
        preset_tools(&scan_type)
    };

    let tools = filter_tools(&requested_tools, &target_source, &target);
    let total = tools.len() as i64;

    // Update tools_total
    // Check which tools have already been run (for resume support)
    let completed_tools: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT tool FROM findings WHERE scan_job_id = ?"
    )
    .bind(scan_job_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(t,)| t)
    .collect();

    let tools_to_run: Vec<String> = tools.iter()
        .filter(|t| !completed_tools.contains(t))
        .cloned()
        .collect();

    let already_done = (total - tools_to_run.len() as i64).max(0);

    sqlx::query("UPDATE scan_jobs SET status = 'running', tools_total = ?, tools_completed = ? WHERE id = ?")
        .bind(total)
        .bind(already_done)
        .bind(scan_job_id)
        .execute(&pool)
        .await
        .ok();

    if already_done > 0 {
        db::insert_scan_log(&pool, scan_job_id, "info", None,
            &format!("▶️ Resuming scan — {}/{} tools already completed, {} remaining",
                already_done, total, tools_to_run.len())).await;
    } else {
        db::insert_scan_log(&pool, scan_job_id, "info", None,
            &format!("🚀 Scan initiated — {} tool(s) queued", total)).await;
    }

    // Register cancellation token
    SCAN_CANCEL.write().await.insert(scan_job_id, false);

    let mut all_findings: Vec<ToolFinding> = Vec::new();
    let mut completed = already_done;

    for tool_name in &tools_to_run {
        // Check for stop request
        {
            let tokens = SCAN_CANCEL.read().await;
            if tokens.get(&scan_job_id).copied().unwrap_or(false) {
                db::insert_scan_log(&pool, scan_job_id, "warn", None,
                    "🛑 Scan stopped by user").await;

                let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
                // Update summary counts — findings are already in DB from per-tool saves.
                save_findings(&pool, scan_job_id, &tools, &now).await;

                sqlx::query("UPDATE scan_jobs SET status = 'stopped', current_tool = NULL WHERE id = ?")
                    .bind(scan_job_id)
                    .execute(&pool)
                    .await
                    .ok();

                SCAN_CANCEL.write().await.remove(&scan_job_id);
                return;
            }
        }

        // Pick a catchphrase
        let idx = (scan_job_id as usize + completed as usize) % CATCHPHRASES.len();
        let phrase = CATCHPHRASES[idx];

        db::insert_scan_log(&pool, scan_job_id, "info", Some(tool_name),
            &format!("Starting {} ... {}", tool_name, phrase)).await;

        sqlx::query("UPDATE scan_jobs SET current_tool = ? WHERE id = ?")
            .bind(tool_name.as_str())
            .bind(scan_job_id)
            .execute(&pool)
            .await
            .ok();

        let findings = run_tool(&pool, scan_job_id, tool_name, &target, &target_source).await;

        let count = findings.len();

        // Persist this tool's findings to the DB immediately so they survive a restart.
        for f in &findings {
            sqlx::query(
                "INSERT INTO findings (scan_job_id, tool, severity, title, description, file_path, line_number, cwe_id, cvss_score, recommendation, issue_type)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(scan_job_id)
            .bind(&f.tool)
            .bind(&f.severity)
            .bind(&f.title)
            .bind(&f.description)
            .bind(&f.file_path)
            .bind(f.line_number)
            .bind(&f.cwe_id)
            .bind(f.cvss_score)
            .bind(&f.recommendation)
            .bind(&f.issue_type)
            .execute(&pool)
            .await
            .ok();
        }

        all_findings.extend(findings);
        completed += 1;

        sqlx::query("UPDATE scan_jobs SET tools_completed = ? WHERE id = ?")
            .bind(completed)
            .bind(scan_job_id)
            .execute(&pool)
            .await
            .ok();

        db::insert_scan_log(&pool, scan_job_id, "info", Some(tool_name),
            &format!("✅ {} complete — {} finding(s)", tool_name, count)).await;
    }

    // Update summary counts from DB (findings were already persisted per-tool above).
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    save_findings(&pool, scan_job_id, &tools, &now).await;

    sqlx::query("UPDATE scan_jobs SET status = 'completed', current_tool = NULL WHERE id = ?")
        .bind(scan_job_id)
        .execute(&pool)
        .await
        .ok();

    // Calculate duration
    sqlx::query(
        "UPDATE scan_jobs SET duration_seconds = CAST(
            (julianday(completed_at) - julianday(started_at)) * 86400 AS INTEGER
        ) WHERE id = ?"
    )
    .bind(scan_job_id)
    .execute(&pool)
    .await
    .ok();

    // Clean up cancellation token
    SCAN_CANCEL.write().await.remove(&scan_job_id);

    let (total_findings, crit, high, med, low, info): (i64, i64, i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT COUNT(*),
                    SUM(CASE WHEN lower(severity)='critical' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN lower(severity)='high' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN lower(severity)='medium' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN lower(severity)='low' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN lower(severity)='info' THEN 1 ELSE 0 END)
             FROM findings WHERE scan_job_id = ?"
        )
        .bind(scan_job_id)
        .fetch_one(&pool)
        .await
        .unwrap_or((0, 0, 0, 0, 0, 0));
    db::insert_scan_log(&pool, scan_job_id, "info", None,
        &format!("🏁 Scan complete — {} total finding(s) ({}C/{}H/{}M/{}L/{}I)",
            total_findings, crit, high, med, low, info)).await;

    // Auto-generate HTML and PDF reports
    db::insert_scan_log(&pool, scan_job_id, "info", None, "📄 Generating reports...").await;
    match crate::services::report::generate_html_report(&pool, scan_job_id).await {
        Ok(_) => {
            db::insert_scan_log(&pool, scan_job_id, "info", None, "✅ HTML report generated").await;
        }
        Err(e) => {
            db::insert_scan_log(&pool, scan_job_id, "warn", None, &format!("⚠️ HTML report failed: {}", e)).await;
        }
    }
    match crate::services::report::generate_pdf_report(&pool, scan_job_id, &crate::models::PdfExportOptions::default()).await {
        Ok(_) => {
            db::insert_scan_log(&pool, scan_job_id, "info", None, "✅ PDF report generated").await;
        }
        Err(e) => {
            db::insert_scan_log(&pool, scan_job_id, "warn", None, &format!("⚠️ PDF report failed: {}", e)).await;
        }
    }
}

async fn save_findings(pool: &DbPool, scan_job_id: i64, tools: &[String], now: &str) {
    // Findings are already persisted per-tool; just recompute the summary counts from DB.
    let counts: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*),
                SUM(CASE WHEN lower(severity)='critical' THEN 1 ELSE 0 END),
                SUM(CASE WHEN lower(severity)='high' THEN 1 ELSE 0 END),
                SUM(CASE WHEN lower(severity)='medium' THEN 1 ELSE 0 END),
                SUM(CASE WHEN lower(severity)='low' THEN 1 ELSE 0 END),
                SUM(CASE WHEN lower(severity)='info' THEN 1 ELSE 0 END)
         FROM findings WHERE scan_job_id = ?"
    )
    .bind(scan_job_id)
    .fetch_one(pool)
    .await
    .unwrap_or((0, 0, 0, 0, 0, 0));

    sqlx::query(
        "UPDATE scan_jobs SET completed_at = ?, total_findings = ?,
         critical_count = ?, high_count = ?, medium_count = ?, low_count = ?, info_count = ?,
         tools_run = ? WHERE id = ?"
    )
    .bind(now)
    .bind(counts.0)
    .bind(counts.1)
    .bind(counts.2)
    .bind(counts.3)
    .bind(counts.4)
    .bind(counts.5)
    .bind(tools.join(", "))
    .bind(scan_job_id)
    .execute(pool)
    .await
    .ok();
}

fn preset_tools(scan_type: &str) -> Vec<String> {
    PRESETS
        .iter()
        .find(|p| p.name == scan_type)
        .map(|p| p.tools.iter().map(|s| s.to_string()).collect())
        .unwrap_or_else(|| vec!["nmap".into()])
}

async fn run_tool(pool: &DbPool, scan_job_id: i64, tool: &str, target: &str, _target_source: &str) -> Vec<ToolFinding> {
    match tool {
        "zap" => super::zap::scan(pool, scan_job_id, target).await,
        "nmap" => super::nmap::scan(pool, scan_job_id, target).await,
        "nikto" => super::nikto::scan(pool, scan_job_id, target).await,
        "sqlmap" => super::sqlmap::scan(pool, scan_job_id, target).await,
        "bandit" => super::bandit::scan(pool, scan_job_id, target).await,
        "trivy" => super::trivy::scan(pool, scan_job_id, target).await,
        "sonarqube" => super::sonarqube::scan(pool, scan_job_id, target).await,
        "dependency_check" => super::dependency_check::scan(pool, scan_job_id, target).await,
        "openvas" => super::openvas::scan(pool, scan_job_id, target).await,
        _ => {
            db::insert_scan_log(pool, scan_job_id, "warn", Some(tool),
                &format!("Unknown tool: {}", tool)).await;
            vec![]
        }
    }
}

use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use tokio::process::Command;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    db::insert_scan_log(pool, scan_job_id, "info", Some("sqlmap"),
        &format!("Running SQLMap scan on {}", target)).await;

    let output = Command::new("sqlmap")
        .args(["-u", target, "--batch", "--level=1", "--risk=1", "--forms", "--crawl=2", "--output-dir=/tmp/sqlmap"])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            db::insert_scan_log(pool, scan_job_id, "info", Some("sqlmap"),
                &format!("SQLMap output: {} bytes", stdout.len())).await;
            parse_sqlmap_output(&stdout)
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("sqlmap"),
                &format!("SQLMap failed: {}", e)).await;
            vec![]
        }
    }
}

fn parse_sqlmap_output(output: &str) -> Vec<ToolFinding> {
    let mut findings = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        if line.contains("is vulnerable") || line.contains("injectable") {
            findings.push(ToolFinding {
                tool: "sqlmap".into(),
                severity: "critical".into(),
                title: "SQL Injection Vulnerability Detected".into(),
                description: Some(line.to_string()),
                file_path: None,
                line_number: None,
                cwe_id: Some("CWE-89".into()),
                cvss_score: Some(9.8),
                recommendation: Some("Use parameterized queries or prepared statements".into()),
                issue_type: Some("Vulnerability".into()),
            });
        }

        if line.contains("Type:") && line.contains("Title:") {
            findings.push(ToolFinding {
                tool: "sqlmap".into(),
                severity: "high".into(),
                title: line.to_string(),
                description: Some("SQLMap identified an injection technique".into()),
                file_path: None,
                line_number: None,
                cwe_id: Some("CWE-89".into()),
                cvss_score: Some(8.0),
                recommendation: Some("Sanitize user input and use parameterized queries".into()),
                issue_type: Some("Vulnerability".into()),
            });
        }
    }

    findings
}

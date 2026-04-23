use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use tokio::process::Command;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    db::insert_scan_log(pool, scan_job_id, "info", Some("nikto"),
        &format!("Running Nikto scan on {}", target)).await;

    let output = Command::new("nikto")
        .args(["-h", target, "-Format", "json", "-output", "-"])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            parse_nikto_output(&stdout)
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("nikto"),
                &format!("Nikto failed: {}", e)).await;
            vec![]
        }
    }
}

fn parse_nikto_output(output: &str) -> Vec<ToolFinding> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.as_array()) {
            for vuln in vulns {
                let osvdb = vuln["OSVDB"].as_str().unwrap_or("");
                let msg = vuln["msg"].as_str().unwrap_or("Nikto finding");
                let url = vuln["url"].as_str().map(|s| s.to_string());
                let method = vuln["method"].as_str().unwrap_or("");

                let severity = if msg.to_lowercase().contains("critical") || msg.to_lowercase().contains("remote code") {
                    "high"
                } else if msg.to_lowercase().contains("outdated") || msg.to_lowercase().contains("default") {
                    "medium"
                } else {
                    "low"
                };

                findings.push(ToolFinding {
                    tool: "nikto".into(),
                    severity: severity.into(),
                    title: if osvdb.is_empty() { msg.to_string() } else { format!("OSVDB-{}: {}", osvdb, msg) },
                    description: Some(format!("{} {} — {}", method, url.as_deref().unwrap_or(""), msg)),
                    file_path: url,
                    line_number: None,
                    cwe_id: None,
                    cvss_score: None,
                    recommendation: Some("Review the finding and apply appropriate fixes".into()),
                    issue_type: None,
                });
            }
        }
    } else {
        // Fallback: parse text output line by line
        for line in output.lines() {
            let line = line.trim();
            if line.starts_with('+') && line.len() > 2 {
                findings.push(ToolFinding {
                    tool: "nikto".into(),
                    severity: "info".into(),
                    title: line.trim_start_matches('+').trim().to_string(),
                    description: None,
                    file_path: None,
                    line_number: None,
                    cwe_id: None,
                    cvss_score: None,
                    recommendation: None,
                    issue_type: None,
                });
            }
        }
    }

    findings
}

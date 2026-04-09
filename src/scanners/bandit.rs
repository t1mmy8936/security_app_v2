use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use tokio::process::Command;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    db::insert_scan_log(pool, scan_job_id, "info", Some("bandit"),
        &format!("Running Bandit SAST on {}", target)).await;

    let output = Command::new("bandit")
        .args(["-r", target, "-f", "json", "-ll"])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            parse_bandit_json(&stdout)
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("bandit"),
                &format!("Bandit failed: {}", e)).await;
            vec![]
        }
    }
}

fn parse_bandit_json(output: &str) -> Vec<ToolFinding> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(results) = json["results"].as_array() {
            for r in results {
                let sev = r["issue_severity"].as_str().unwrap_or("LOW");
                let severity = match sev {
                    "HIGH" => "high",
                    "MEDIUM" => "medium",
                    "LOW" => "low",
                    _ => "info",
                };

                let cwe_raw = r["issue_cwe"]["id"].as_i64();
                let cwe = cwe_raw.map(|id| format!("CWE-{}", id));

                findings.push(ToolFinding {
                    tool: "bandit".into(),
                    severity: severity.into(),
                    title: format!("{}: {}",
                        r["test_id"].as_str().unwrap_or(""),
                        r["issue_text"].as_str().unwrap_or("Bandit finding")),
                    description: r["issue_text"].as_str().map(|s| s.to_string()),
                    file_path: r["filename"].as_str().map(|s| s.to_string()),
                    line_number: r["line_number"].as_i64(),
                    cwe_id: cwe,
                    cvss_score: None,
                    recommendation: Some(format!("Confidence: {}. Review and fix the identified issue.",
                        r["issue_confidence"].as_str().unwrap_or("UNKNOWN"))),
                });
            }
        }
    }

    findings
}

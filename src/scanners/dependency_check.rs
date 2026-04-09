use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use tokio::process::Command;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    db::insert_scan_log(pool, scan_job_id, "info", Some("dependency_check"),
        &format!("Running OWASP Dependency-Check on {}", target)).await;

    let report_dir = format!("/tmp/dc-report-{}", scan_job_id);
    let _ = tokio::fs::create_dir_all(&report_dir).await;
    let report_path = format!("{}/dependency-check-report.json", report_dir);

    let output = Command::new("/opt/dependency-check/bin/dependency-check.sh")
        .args([
            "--scan", target,
            "--format", "JSON",
            "--out", &report_dir,
            "--data", "/opt/dependency-check/data",
            "--exclude", "**/node_modules/**",
            "--exclude", "**/.git/**",
            "--exclude", "**/venv/**",
            "--exclude", "**/__pycache__/**",
            "--exclude", "**/target/**",
        ])
        .output()
        .await;

    match output {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                db::insert_scan_log(pool, scan_job_id, "warn", Some("dependency_check"),
                    &format!("Dependency-Check exited with warnings: {}", &stderr[..stderr.len().min(500)])).await;
            }

            // Parse the JSON report
            match tokio::fs::read_to_string(&report_path).await {
                Ok(contents) => parse_dc_json(&contents),
                Err(e) => {
                    db::insert_scan_log(pool, scan_job_id, "error", Some("dependency_check"),
                        &format!("Failed to read report: {}", e)).await;
                    vec![]
                }
            }
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("dependency_check"),
                &format!("Dependency-Check failed: {}", e)).await;
            vec![]
        }
    }
}

fn parse_dc_json(output: &str) -> Vec<ToolFinding> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(deps) = json["dependencies"].as_array() {
            for dep in deps {
                if let Some(vulns) = dep["vulnerabilities"].as_array() {
                    let file = dep["filePath"].as_str().unwrap_or("");
                    for vuln in vulns {
                        let sev = vuln["severity"].as_str().unwrap_or("UNKNOWN");
                        let severity = match sev {
                            "CRITICAL" => "critical",
                            "HIGH" => "high",
                            "MEDIUM" => "medium",
                            "LOW" => "low",
                            _ => "info",
                        };

                        let cvss = vuln["cvssv3"].as_object()
                            .and_then(|o| o["baseScore"].as_f64())
                            .or_else(|| vuln["cvssv2"].as_object()
                                .and_then(|o| o["score"].as_f64()));

                        let cwes: Option<String> = vuln["cwes"].as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        findings.push(ToolFinding {
                            tool: "dependency_check".into(),
                            severity: severity.into(),
                            title: format!("{}: {}",
                                vuln["name"].as_str().unwrap_or("CVE"),
                                dep["fileName"].as_str().unwrap_or("")),
                            description: vuln["description"].as_str().map(|s| {
                                if s.len() > 500 { format!("{}...", &s[..500]) } else { s.to_string() }
                            }),
                            file_path: Some(file.to_string()),
                            line_number: None,
                            cwe_id: cwes,
                            cvss_score: cvss,
                            recommendation: Some("Update the dependency to a non-vulnerable version".into()),
                        });
                    }
                }
            }
        }
    }

    findings
}

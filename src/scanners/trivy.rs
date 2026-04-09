use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use tokio::process::Command;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    db::insert_scan_log(pool, scan_job_id, "info", Some("trivy"),
        &format!("Running Trivy scan on {}", target)).await;

    let scan_type = if target.contains(':') && !target.starts_with('/') && !target.starts_with("http") {
        "image"
    } else {
        "fs"
    };

    let output = Command::new("trivy")
        .args([scan_type, "--format", "json", "--quiet", target])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            parse_trivy_json(&stdout)
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("trivy"),
                &format!("Trivy failed: {}", e)).await;
            vec![]
        }
    }
}

fn parse_trivy_json(output: &str) -> Vec<ToolFinding> {
    let mut findings = Vec::new();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        let results = json["Results"].as_array()
            .or_else(|| json.as_array());

        if let Some(results) = results {
            for result in results {
                let target_name = result["Target"].as_str().unwrap_or("");
                if let Some(vulns) = result["Vulnerabilities"].as_array() {
                    for vuln in vulns {
                        let sev = vuln["Severity"].as_str().unwrap_or("UNKNOWN");
                        let severity = match sev {
                            "CRITICAL" => "critical",
                            "HIGH" => "high",
                            "MEDIUM" => "medium",
                            "LOW" => "low",
                            _ => "info",
                        };

                        let cvss = vuln["CVSS"].as_object()
                            .and_then(|m| m.values().next())
                            .and_then(|v| v["V3Score"].as_f64());

                        findings.push(ToolFinding {
                            tool: "trivy".into(),
                            severity: severity.into(),
                            title: format!("{}: {} ({})",
                                vuln["VulnerabilityID"].as_str().unwrap_or(""),
                                vuln["PkgName"].as_str().unwrap_or(""),
                                vuln["InstalledVersion"].as_str().unwrap_or("")),
                            description: vuln["Title"].as_str()
                                .or(vuln["Description"].as_str())
                                .map(|s| s.to_string()),
                            file_path: Some(target_name.to_string()),
                            line_number: None,
                            cwe_id: vuln["CweIDs"].as_array()
                                .and_then(|a| a.first())
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            cvss_score: cvss,
                            recommendation: vuln["FixedVersion"].as_str()
                                .map(|v| format!("Upgrade to version {}", v)),
                        });
                    }
                }
            }
        }
    }

    findings
}

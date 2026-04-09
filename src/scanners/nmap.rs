use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use tokio::process::Command;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    // Extract host from URL if needed
    let host = if target.starts_with("http") {
        url_to_host(target)
    } else {
        target.to_string()
    };

    db::insert_scan_log(pool, scan_job_id, "info", Some("nmap"),
        &format!("Running Nmap scan on {}", host)).await;

    let output = Command::new("nmap")
        .args(["-sV", "-sC", "--top-ports", "1000", "-oX", "-", &host])
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);

            if !stderr.is_empty() {
                db::insert_scan_log(pool, scan_job_id, "warn", Some("nmap"),
                    &format!("Nmap stderr: {}", &stderr[..stderr.len().min(500)])).await;
            }

            parse_nmap_xml(&stdout)
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("nmap"),
                &format!("Nmap failed: {}", e)).await;
            vec![]
        }
    }
}

fn url_to_host(url: &str) -> String {
    let without_scheme = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    without_scheme.split('/').next().unwrap_or(url)
        .split(':').next().unwrap_or(url)
        .to_string()
}

fn parse_nmap_xml(xml: &str) -> Vec<ToolFinding> {
    let mut findings = Vec::new();

    // Simple XML parsing for open ports and services
    for line in xml.lines() {
        let line = line.trim();
        if line.starts_with("<port ") {
            let port = extract_attr(line, "portid").unwrap_or_default();
            let protocol = extract_attr(line, "protocol").unwrap_or_default();
            // Look ahead for service info
            findings.push(ToolFinding {
                tool: "nmap".into(),
                severity: "info".into(),
                title: format!("Open port {}/{}", port, protocol),
                description: Some(format!("Port {}/{} is open", port, protocol)),
                file_path: None,
                line_number: None,
                cwe_id: None,
                cvss_score: None,
                recommendation: Some("Review if this port should be exposed".into()),
            });
        }
        if line.contains("<script ") && line.contains("VULNERABLE") {
            let script_id = extract_attr(line, "id").unwrap_or_else(|| "unknown".into());
            findings.push(ToolFinding {
                tool: "nmap".into(),
                severity: "high".into(),
                title: format!("Nmap script vulnerability: {}", script_id),
                description: Some("Nmap NSE script detected a vulnerability".into()),
                file_path: None,
                line_number: None,
                cwe_id: None,
                cvss_score: None,
                recommendation: Some("Investigate and patch the identified vulnerability".into()),
            });
        }
    }

    findings
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    if let Some(start) = tag.find(&needle) {
        let rest = &tag[start + needle.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

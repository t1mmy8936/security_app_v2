use crate::db::{self, DbPool};
use crate::models::ToolFinding;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    let zap_api_key = db::get_setting(pool, "zap_api_key").await;
    let api_key = if zap_api_key.is_empty() { "changeme".to_string() } else { zap_api_key };
    let zap_url = std::env::var("ZAP_URL").unwrap_or_else(|_| "http://zap:8080".into());

    db::insert_scan_log(pool, scan_job_id, "info", Some("zap"),
        &format!("Starting ZAP spider on {}", target)).await;

    // Start spider
    let spider_url = format!("{}/JSON/spider/action/scan/?apikey={}&url={}&maxChildren=10&recurse=true",
        zap_url, api_key, target);
    let client = reqwest::Client::new();
    let resp = client.get(&spider_url).send().await;
    if let Err(e) = &resp {
        db::insert_scan_log(pool, scan_job_id, "error", Some("zap"),
            &format!("ZAP spider failed: {}", e)).await;
        return vec![];
    }

    // Wait for spider to complete
    loop {
        let status_url = format!("{}/JSON/spider/view/status/?apikey={}", zap_url, api_key);
        if let Ok(r) = client.get(&status_url).send().await {
            if let Ok(body) = r.json::<serde_json::Value>().await {
                if let Some(s) = body["status"].as_str() {
                    if s == "100" { break; }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    db::insert_scan_log(pool, scan_job_id, "info", Some("zap"), "Spider complete, starting active scan").await;

    // Active scan
    let ascan_url = format!("{}/JSON/ascan/action/scan/?apikey={}&url={}&recurse=true",
        zap_url, api_key, target);
    let _ = client.get(&ascan_url).send().await;

    // Wait for active scan
    loop {
        let status_url = format!("{}/JSON/ascan/view/status/?apikey={}", zap_url, api_key);
        if let Ok(r) = client.get(&status_url).send().await {
            if let Ok(body) = r.json::<serde_json::Value>().await {
                if let Some(s) = body["status"].as_str() {
                    if s == "100" { break; }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Get alerts
    let alerts_url = format!("{}/JSON/alert/view/alerts/?apikey={}&baseurl={}&start=0&count=500",
        zap_url, api_key, target);

    let mut findings = Vec::new();
    if let Ok(r) = client.get(&alerts_url).send().await {
        if let Ok(body) = r.json::<serde_json::Value>().await {
            if let Some(alerts) = body["alerts"].as_array() {
                for alert in alerts {
                    let risk = alert["risk"].as_str().unwrap_or("Informational");
                    let severity = match risk {
                        "High" => "high",
                        "Medium" => "medium",
                        "Low" => "low",
                        _ => "info",
                    };
                    let cwe = alert["cweid"].as_str()
                        .filter(|s| !s.is_empty() && *s != "-1")
                        .map(|s| format!("CWE-{}", s));

                    findings.push(ToolFinding {
                        tool: "zap".into(),
                        severity: severity.into(),
                        title: alert["alert"].as_str().unwrap_or("ZAP Alert").into(),
                        description: alert["description"].as_str().map(|s| s.to_string()),
                        file_path: alert["url"].as_str().map(|s| s.to_string()),
                        line_number: None,
                        cwe_id: cwe,
                        cvss_score: None,
                        recommendation: alert["solution"].as_str().map(|s| s.to_string()),
                        issue_type: None,
                    });
                }
            }
        }
    }

    findings
}

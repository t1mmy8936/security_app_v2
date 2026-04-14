use crate::db::{self, DbPool};
use crate::models::ToolFinding;
use quick_xml::events::Event;
use quick_xml::Reader;

/// Full-and-Fast scan config UUID (built into every GVM installation).
const FULL_AND_FAST_CONFIG: &str = "daba56c8-73ec-11df-a475-002264764cea";
/// Scanner UUID for ospd-openvas (built into every GVM installation).
const OPENVAS_SCANNER: &str = "08b69003-5fc2-4037-a479-93b440211c73";
/// How long to wait between status polls.
const POLL_INTERVAL_SECS: u64 = 30;
/// Maximum time to wait for a scan to complete (45 minutes).
const MAX_POLL_ATTEMPTS: u32 = 90;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    let url = db::get_setting(pool, "openvas_url").await;
    let username = db::get_setting(pool, "openvas_username").await;
    let password = db::get_setting(pool, "openvas_password").await;

    if url.is_empty() {
        db::insert_scan_log(pool, scan_job_id, "warn", Some("openvas"),
            "OpenVAS URL not configured — skipping").await;
        return vec![];
    }
    if username.is_empty() || password.is_empty() {
        db::insert_scan_log(pool, scan_job_id, "warn", Some("openvas"),
            "OpenVAS credentials not configured — skipping").await;
        return vec![];
    }

    db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
        &format!("Starting OpenVAS scan against {}", target)).await;

    let client = reqwest::Client::builder()
        // gsad may use a self-signed cert in local deployments
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default();

    // 1. Authenticate
    let token = match authenticate(&client, &url, &username, &password).await {
        Ok(t) => t,
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("openvas"),
                &format!("Authentication failed: {}", e)).await;
            return vec![];
        }
    };

    db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
        "Authenticated with OpenVAS").await;

    // 2. Create target
    let target_id = match create_target(&client, &url, &token, target, scan_job_id).await {
        Ok(id) => id,
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("openvas"),
                &format!("Failed to create target: {}", e)).await;
            let _ = logout(&client, &url, &token).await;
            return vec![];
        }
    };

    db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
        &format!("Created OpenVAS target: {}", target_id)).await;

    // 3. Create task
    let task_id = match create_task(&client, &url, &token, &target_id, target).await {
        Ok(id) => id,
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("openvas"),
                &format!("Failed to create task: {}", e)).await;
            delete_target(&client, &url, &token, &target_id).await;
            let _ = logout(&client, &url, &token).await;
            return vec![];
        }
    };

    db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
        &format!("Created OpenVAS task: {}", task_id)).await;

    // 4. Start task
    let report_id = match start_task(&client, &url, &token, &task_id).await {
        Ok(id) => id,
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("openvas"),
                &format!("Failed to start task: {}", e)).await;
            delete_task(&client, &url, &token, &task_id).await;
            delete_target(&client, &url, &token, &target_id).await;
            let _ = logout(&client, &url, &token).await;
            return vec![];
        }
    };

    db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
        "OpenVAS scan started, polling for completion...").await;

    // 5. Poll until done
    let mut poll_count = 0u32;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
        poll_count += 1;

        let status = match get_task_status(&client, &url, &token, &task_id).await {
            Ok(s) => s,
            Err(e) => {
                db::insert_scan_log(pool, scan_job_id, "warn", Some("openvas"),
                    &format!("Status poll error (attempt {}): {}", poll_count, e)).await;
                if poll_count >= MAX_POLL_ATTEMPTS {
                    break;
                }
                continue;
            }
        };

        db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
            &format!("OpenVAS scan status: {} (poll {}/{})", status, poll_count, MAX_POLL_ATTEMPTS)).await;

        if status == "Done" || status == "Stopped" {
            break;
        }
        if status == "Interrupted" || status == "Delete Requested" {
            db::insert_scan_log(pool, scan_job_id, "error", Some("openvas"),
                &format!("OpenVAS scan ended unexpectedly with status: {}", status)).await;
            // Still try to fetch partial results
            break;
        }
        if poll_count >= MAX_POLL_ATTEMPTS {
            db::insert_scan_log(pool, scan_job_id, "warn", Some("openvas"),
                "OpenVAS scan timed out after 45 minutes, collecting partial results").await;
            break;
        }
    }

    // 6. Fetch report XML
    let findings = match fetch_report(&client, &url, &token, &report_id, pool, scan_job_id).await {
        Ok(f) => {
            db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
                &format!("OpenVAS scan complete — {} findings", f.len())).await;
            f
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("openvas"),
                &format!("Failed to fetch report: {}", e)).await;
            vec![]
        }
    };

    // 7. Cleanup
    delete_task(&client, &url, &token, &task_id).await;
    delete_target(&client, &url, &token, &target_id).await;
    let _ = logout(&client, &url, &token).await;

    findings
}

// ─── API helpers ──────────────────────────────────────────────────────────────

async fn authenticate(
    client: &reqwest::Client,
    url: &str,
    username: &str,
    password: &str,
) -> Result<String, String> {
    let body = serde_json::json!({ "username": username, "password": password });
    let resp = client
        .post(format!("{}/api/v1/tokens", url))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json["data"]["token"]
        .as_str()
        .map(|t| t.to_string())
        .ok_or_else(|| "Token not found in auth response".into())
}

async fn logout(client: &reqwest::Client, url: &str, token: &str) -> Result<(), String> {
    client
        .delete(format!("{}/api/v1/tokens/{}", url, token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn create_target(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    host: &str,
    scan_job_id: i64,
) -> Result<String, String> {
    let name = format!("watchtower-target-{}-{}", scan_job_id, sanitize_name(host));
    let body = serde_json::json!({
        "name": name,
        "hosts": host,
        "port_list": { "id": "33d0cd82-57c6-11e1-8251-406186ea4fc5" }  // All IANA assigned TCP
    });
    let resp = client
        .post(format!("{}/api/v1/targets", url))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {} — {}", status, text));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    extract_id(&json, "target")
}

async fn create_task(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    target_id: &str,
    host: &str,
) -> Result<String, String> {
    let name = format!("watchtower-scan-{}", sanitize_name(host));
    let body = serde_json::json!({
        "name": name,
        "target": { "id": target_id },
        "config": { "id": FULL_AND_FAST_CONFIG },
        "scanner": { "id": OPENVAS_SCANNER }
    });
    let resp = client
        .post(format!("{}/api/v1/tasks", url))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {} — {}", status, text));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    extract_id(&json, "task")
}

async fn start_task(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    task_id: &str,
) -> Result<String, String> {
    let resp = client
        .post(format!("{}/api/v1/tasks/{}/start", url, task_id))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {} — {}", status, text));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    // Response contains the report ID that will be created
    json["data"]["report_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "report_id not found in start response".into())
}

async fn get_task_status(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    task_id: &str,
) -> Result<String, String> {
    let resp = client
        .get(format!("{}/api/v1/tasks/{}", url, task_id))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    json["data"]["status"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "status field not found in task response".into())
}

async fn fetch_report(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    report_id: &str,
    pool: &DbPool,
    scan_job_id: i64,
) -> Result<Vec<ToolFinding>, String> {
    // Request XML format; filter to High/Medium/Low/Log severity levels
    let resp = client
        .get(format!(
            "{}/api/v1/reports/{}?filter=levels%3Dhml&report_format=a994b278-1f62-11e1-96ac-406186ea4fc5",
            url, report_id
        ))
        .bearer_auth(token)
        .header("Accept", "application/xml")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let text = resp.text().await.map_err(|e| e.to_string())?;

    db::insert_scan_log(pool, scan_job_id, "info", Some("openvas"),
        &format!("Retrieved report XML ({} bytes), parsing...", text.len())).await;

    parse_xml_report(&text)
}

async fn delete_target(client: &reqwest::Client, url: &str, token: &str, target_id: &str) {
    let _ = client
        .delete(format!("{}/api/v1/targets/{}", url, target_id))
        .bearer_auth(token)
        .send()
        .await;
}

async fn delete_task(client: &reqwest::Client, url: &str, token: &str, task_id: &str) {
    let _ = client
        .delete(format!("{}/api/v1/tasks/{}", url, task_id))
        .bearer_auth(token)
        .send()
        .await;
}

// ─── XML Parsing ──────────────────────────────────────────────────────────────

fn parse_xml_report(xml: &str) -> Result<Vec<ToolFinding>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut findings: Vec<ToolFinding> = Vec::new();

    // State variables for the current <result> element being parsed
    let mut in_result = false;
    let mut current_name = String::new();
    let mut current_host = String::new();
    let mut current_port = String::new();
    let mut current_severity = String::new();
    let mut current_description = String::new();
    let mut current_nvt_oid = String::new();
    let mut current_cves: Vec<String> = Vec::new();
    let mut current_tag = String::new(); // name of inner element we're reading text for

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref()).unwrap_or("").to_string();
                match tag.as_str() {
                    "result" => {
                        in_result = true;
                        current_name.clear();
                        current_host.clear();
                        current_port.clear();
                        current_severity.clear();
                        current_description.clear();
                        current_nvt_oid.clear();
                        current_cves.clear();
                        current_tag.clear();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"oid" {
                                if let Ok(v) = attr.unescape_value() {
                                    current_nvt_oid = v.to_string();
                                }
                            }
                        }
                    }
                    "nvt" if in_result => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"oid" {
                                if let Ok(v) = attr.unescape_value() {
                                    current_nvt_oid = v.to_string();
                                }
                            }
                        }
                    }
                    name if in_result => {
                        current_tag = name.to_string();
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref()).unwrap_or("").to_string();
                if in_result && tag == "host" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"ip" {
                            if let Ok(v) = attr.unescape_value() {
                                current_host = v.to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if !in_result {
                    continue;
                }
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    continue;
                }
                match current_tag.as_str() {
                    "name" => current_name = text,
                    "host" => {
                        if current_host.is_empty() {
                            current_host = text;
                        }
                    }
                    "port" => current_port = text,
                    "severity" => current_severity = text,
                    "description" => current_description = text,
                    "ref" => {
                        if text.starts_with("CVE-") {
                            current_cves.push(text);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let tag = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                if tag == "result" && in_result {
                    let severity_f: f64 = current_severity.parse().unwrap_or(0.0);
                    if severity_f > 0.0 && !current_name.is_empty() {
                        let severity_label = cvss_to_severity(severity_f);
                        let location = if current_port.is_empty() {
                            current_host.clone()
                        } else {
                            format!("{}:{}", current_host, current_port)
                        };
                        let cve_str = current_cves.join(", ");
                        findings.push(ToolFinding {
                            tool: "openvas".into(),
                            severity: severity_label,
                            title: current_name.clone(),
                            description: Some(if cve_str.is_empty() {
                                current_description.clone()
                            } else {
                                format!("{}\n\nCVEs: {}", current_description, cve_str)
                            }),
                            file_path: Some(location),
                            line_number: None,
                            cwe_id: None,
                            cvss_score: Some(severity_f),
                            recommendation: Some(format!("NVT OID: {}", current_nvt_oid)),
                        });
                    }
                    in_result = false;
                } else if in_result {
                    current_tag.clear();
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    Ok(findings)
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn cvss_to_severity(score: f64) -> String {
    if score >= 9.0 {
        "critical".into()
    } else if score >= 7.0 {
        "high".into()
    } else if score >= 4.0 {
        "medium".into()
    } else {
        "low".into()
    }
}

fn sanitize_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .take(40)
        .collect()
}

fn extract_id(json: &serde_json::Value, resource: &str) -> Result<String, String> {
    // Try common response shapes: { "data": { "id": "..." } } or { "id": "..." }
    if let Some(id) = json["data"]["id"].as_str() {
        return Ok(id.to_string());
    }
    if let Some(id) = json["id"].as_str() {
        return Ok(id.to_string());
    }
    Err(format!("Could not find ID in {} creation response: {}", resource, json))
}

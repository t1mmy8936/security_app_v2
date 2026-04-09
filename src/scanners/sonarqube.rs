use crate::db::{self, DbPool};
use crate::models::ToolFinding;

pub async fn scan(pool: &DbPool, scan_job_id: i64, target: &str) -> Vec<ToolFinding> {
    let sonar_url = db::get_setting(pool, "sonarqube_url").await;
    let sonar_url = if sonar_url.is_empty() { "http://sonarqube:9000".to_string() } else { sonar_url };
    let sonar_token = db::get_setting(pool, "sonarqube_token").await;
    let project_key = db::get_setting(pool, "sonarqube_project_key").await;
    let project_key = if project_key.is_empty() { "watchtower-scan".to_string() } else { project_key };
    let exclusions = db::get_setting(pool, "sonarqube_exclusions").await;
    let quality_profile = db::get_setting(pool, "sonarqube_quality_profile").await;

    if sonar_token.is_empty() {
        db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"),
            "No SonarQube token configured, attempting auto-provision").await;
        if let Err(e) = auto_provision(pool, &sonar_url).await {
            db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                &format!("Auto-provision failed: {}", e)).await;
            return vec![];
        }
    }

    let token = db::get_setting(pool, "sonarqube_token").await;

    // Run sonar-scanner
    db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"),
        &format!("Running sonar-scanner on {}", target)).await;

    let work_dir = format!("/tmp/sonar-work-{}", scan_job_id);
    let _ = tokio::fs::create_dir_all(&work_dir).await;

    let mut args = vec![
        format!("-Dsonar.projectKey={}", project_key),
        format!("-Dsonar.sources={}", target),
        format!("-Dsonar.host.url={}", sonar_url),
        format!("-Dsonar.token={}", token),
        format!("-Dsonar.working.directory={}", work_dir),
        "-Dsonar.scm.disabled=true".to_string(),
    ];

    if !exclusions.is_empty() {
        args.push(format!("-Dsonar.exclusions={}", exclusions));
    }

    if !quality_profile.is_empty() {
        args.push(format!("-Dsonar.qualityprofile={}", quality_profile));
    }

    // Auto-detect java binaries
    let java_bin_path = format!("{}/target/classes", target);
    if tokio::fs::metadata(&java_bin_path).await.is_ok() {
        args.push(format!("-Dsonar.java.binaries={}", java_bin_path));
    } else {
        args.push("-Dsonar.java.binaries=/tmp/empty-sonar".to_string());
        let _ = tokio::fs::create_dir_all("/tmp/empty-sonar").await;
    }

    let output = tokio::process::Command::new("sonar-scanner")
        .args(&args)
        .output()
        .await;

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if !out.status.success() {
                db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                    &format!("sonar-scanner failed: {}", &stderr[..stderr.len().min(500)])).await;
                return vec![];
            }
            db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"), "sonar-scanner complete, fetching results").await;
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                &format!("sonar-scanner failed to start: {}", e)).await;
            return vec![];
        }
    }

    // Wait for analysis processing
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Fetch issues from SonarQube API
    fetch_issues(pool, scan_job_id, &sonar_url, &token, &project_key).await
}

async fn fetch_issues(pool: &DbPool, scan_job_id: i64, sonar_url: &str, token: &str, project_key: &str) -> Vec<ToolFinding> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/issues/search?componentKeys={}&ps=500&resolved=false", sonar_url, project_key);

    let resp = client.get(&url)
        .bearer_auth(token)
        .send()
        .await;

    let mut findings = Vec::new();

    match resp {
        Ok(r) => {
            if let Ok(body) = r.json::<serde_json::Value>().await {
                if let Some(issues) = body["issues"].as_array() {
                    for issue in issues {
                        let sev = issue["severity"].as_str().unwrap_or("INFO");
                        let severity = match sev {
                            "BLOCKER" | "CRITICAL" => "critical",
                            "MAJOR" => "high",
                            "MINOR" => "medium",
                            "INFO" => "low",
                            _ => "info",
                        };

                        let cwe = extract_cwe(issue);
                        let text_start = issue["textRange"]["startLine"].as_i64();
                        let text_end = issue["textRange"]["endLine"].as_i64();

                        let rule = issue["rule"].as_str().unwrap_or("");
                        let rule_url = if !rule.is_empty() {
                            Some(format!("{}/coding_rules?open={}", sonar_url, rule))
                        } else {
                            None
                        };

                        findings.push(ToolFinding {
                            tool: "sonarqube".into(),
                            severity: severity.into(),
                            title: issue["message"].as_str().unwrap_or("SonarQube Issue").into(),
                            description: Some(format!("Rule: {} | Type: {} | Effort: {}",
                                rule,
                                issue["type"].as_str().unwrap_or(""),
                                issue["effort"].as_str().unwrap_or(""))),
                            file_path: issue["component"].as_str().map(|s| s.to_string()),
                            line_number: issue["line"].as_i64(),
                            cwe_id: cwe,
                            cvss_score: None,
                            recommendation: rule_url,
                        });
                    }
                }
            }
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                &format!("Failed to fetch SonarQube issues: {}", e)).await;
        }
    }

    findings
}

fn extract_cwe(issue: &serde_json::Value) -> Option<String> {
    if let Some(tags) = issue["tags"].as_array() {
        for tag in tags {
            if let Some(s) = tag.as_str() {
                if s.starts_with("cwe-") || s.starts_with("CWE-") {
                    return Some(s.to_uppercase());
                }
            }
        }
    }
    None
}

async fn auto_provision(pool: &DbPool, sonar_url: &str) -> Result<(), String> {
    let client = reqwest::Client::new();

    // Try configured password first, then default
    let configured_pw = db::get_setting(pool, "sonarqube_password").await;
    let passwords = if configured_pw.is_empty() {
        vec!["admin".to_string()]
    } else {
        vec![configured_pw, "admin".to_string()]
    };

    let mut auth_password = String::new();
    for pw in &passwords {
        let resp = client.get(&format!("{}/api/system/status", sonar_url))
            .basic_auth("admin", Some(pw))
            .send()
            .await
            .map_err(|e| format!("Cannot reach SonarQube: {}", e))?;

        if resp.status().is_success() {
            auth_password = pw.clone();
            break;
        }
    }

    if auth_password.is_empty() {
        return Err("Cannot authenticate to SonarQube with any known password".into());
    }

    // Generate token
    let token_resp = client.post(&format!("{}/api/user_tokens/generate", sonar_url))
        .basic_auth("admin", Some(&auth_password))
        .form(&[
            ("name", format!("watchtower-{}", chrono::Utc::now().timestamp())),
            ("type", "GLOBAL_ANALYSIS_TOKEN".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Token generation failed: {}", e))?;

    if !token_resp.status().is_success() {
        let body = token_resp.text().await.unwrap_or_default();
        return Err(format!("Token generation returned error: {}", body));
    }

    let token_body: serde_json::Value = token_resp.json().await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let token = token_body["token"].as_str()
        .ok_or("No token in response")?
        .to_string();

    db::set_setting(pool, "sonarqube_token", &token).await;
    db::set_setting(pool, "sonarqube_url", sonar_url).await;

    Ok(())
}

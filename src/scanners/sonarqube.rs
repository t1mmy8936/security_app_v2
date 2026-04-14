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

    // Verify token can read issues (USER_TOKEN required), re-provision if not
    {
        let test_client = reqwest::Client::new();
        let test_url = format!("{}/api/projects/search?ps=1", sonar_url);
        let test_ok = match test_client.get(&test_url).bearer_auth(&token).send().await {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        };
        if !test_ok {
            db::insert_scan_log(pool, scan_job_id, "warn", Some("sonarqube"),
                "Current token lacks read permissions, re-provisioning with USER_TOKEN").await;
            if let Err(e) = auto_provision(pool, &sonar_url).await {
                db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                    &format!("Re-provision failed: {}", e)).await;
                return vec![];
            }
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
        format!("-Dsonar.projectBaseDir={}", target),
        "-Dsonar.sources=.".to_string(),
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

            // Log sonar-scanner output for debugging
            let combined = format!("{}\n{}", stdout, stderr);
            let log_snippet: String = combined.lines()
                .filter(|l| l.contains("WARN") || l.contains("ERROR") || l.contains("INFO  Sensor") || l.contains("indexed") || l.contains("source file"))
                .take(10)
                .collect::<Vec<_>>()
                .join("\n");
            if !log_snippet.is_empty() {
                db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"),
                    &format!("Scanner output:\n{}", &log_snippet[..log_snippet.len().min(800)])).await;
            }

            // Extract the CE task ID from scanner output so we can poll it directly
            let task_id: Option<String> = combined.lines()
                .find(|l| l.contains("api/ce/task?id="))
                .and_then(|l| l.split("api/ce/task?id=").nth(1))
                .map(|s| s.split_whitespace().next().unwrap_or(s).trim().to_string());

            if !out.status.success() {
                db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                    &format!("sonar-scanner failed: {}", &stderr[..stderr.len().min(500)])).await;
                return vec![];
            }
            db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"), "sonar-scanner complete, waiting for SonarQube to process analysis...").await;

            // Wait for SonarQube CE task to complete
            // Poll the specific task ID if we have it (most reliable), otherwise fall back to component endpoint
            let client = reqwest::Client::new();
            let mut waited = 0u64;
            let poll_interval = 5u64;
            let timeout = 300u64; // 5 minutes

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
                waited += poll_interval;

                let done = if let Some(ref tid) = task_id {
                    // Poll the specific task
                    let task_url = format!("{}/api/ce/task?id={}", sonar_url, tid);
                    if let Ok(resp) = client.get(&task_url).bearer_auth(&token).send().await {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let status = body["task"]["status"].as_str().unwrap_or("");
                            match status {
                                "SUCCESS" => true,
                                "FAILED" | "CANCELED" => {
                                    let err_msg = body["task"]["errorMessage"].as_str().unwrap_or("unknown error");
                                    db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                                        &format!("Analysis {}: {}", status, err_msg)).await;
                                    true
                                }
                                _ => false,
                            }
                        } else { false }
                    } else { false }
                } else {
                    // Fallback: poll component endpoint, require seeing the task in-progress before declaring done
                    let ce_url = format!("{}/api/ce/component?component={}", sonar_url, project_key);
                    if let Ok(resp) = client.get(&ce_url).bearer_auth(&token).send().await {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            let has_current = body.get("current").is_some() && !body["current"].is_null();
                            if has_current {
                                let status = body["current"]["status"].as_str().unwrap_or("");
                                match status {
                                    "SUCCESS" => true,
                                    "FAILED" | "CANCELED" => {
                                        let err_msg = body["current"]["errorMessage"].as_str().unwrap_or("unknown error");
                                        db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                                            &format!("Analysis {}: {}", status, err_msg)).await;
                                        true
                                    }
                                    _ => false,
                                }
                            } else {
                                // No current task visible yet — keep waiting (don't break early)
                                false
                            }
                        } else { false }
                    } else { false }
                };

                if done { break; }

                if waited >= timeout {
                    db::insert_scan_log(pool, scan_job_id, "warn", Some("sonarqube"),
                        &format!("Timed out waiting for analysis ({}s), fetching whatever results are available", timeout)).await;
                    break;
                }
            }

            db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"),
                &format!("Analysis complete after ~{}s, fetching results", waited)).await;
        }
        Err(e) => {
            db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                &format!("sonar-scanner failed to start: {}", e)).await;
            return vec![];
        }
    }

    // Fetch issues from SonarQube API
    fetch_issues(pool, scan_job_id, &sonar_url, &token, &project_key).await
}

async fn fetch_issues(pool: &DbPool, scan_job_id: i64, sonar_url: &str, token: &str, project_key: &str) -> Vec<ToolFinding> {
    let client = reqwest::Client::new();
    let page_size = 500;
    let mut page = 1;
    let mut findings = Vec::new();
    let mut total_logged = false;

    loop {
        let url = format!(
            "{}/api/issues/search?componentKeys={}&ps={}&p={}&resolved=false",
            sonar_url, project_key, page_size, page
        );

        let resp = client.get(&url).bearer_auth(token).send().await;

        match resp {
            Ok(r) => {
                let status = r.status();
                if !status.is_success() {
                    db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                        &format!("SonarQube issues API returned HTTP {}", status)).await;
                    break;
                }
                if let Ok(body) = r.json::<serde_json::Value>().await {
                    let total = body["paging"]["total"].as_i64()
                        .or_else(|| body["total"].as_i64())
                        .unwrap_or(0);

                    if !total_logged {
                        db::insert_scan_log(pool, scan_job_id, "info", Some("sonarqube"),
                            &format!("SonarQube API returned {} total issues, fetching all pages...", total)).await;
                        total_logged = true;
                    }

                    let issues = match body["issues"].as_array() {
                        Some(a) => a,
                        None => break,
                    };

                    if issues.is_empty() { break; }

                    for issue in issues {
                        // Try legacy `severity` first, then `impacts[0].severity` (SonarQube 10+)
                        let sev = issue["severity"].as_str().unwrap_or_else(|| {
                            issue["impacts"].as_array()
                                .and_then(|a| a.iter().find(|i| {
                                    i["softwareQuality"].as_str()
                                        .map(|q| q == "SECURITY" || q == "RELIABILITY")
                                        .unwrap_or(false)
                                }))
                                .and_then(|i| i["severity"].as_str())
                                .unwrap_or_else(|| {
                                    issue["impacts"].as_array()
                                        .and_then(|a| a.first())
                                        .and_then(|i| i["severity"].as_str())
                                        .unwrap_or("INFO")
                                })
                        });

                        let severity = match sev.to_uppercase().as_str() {
                            "BLOCKER" | "CRITICAL" => "critical",
                            "HIGH" | "MAJOR" => "high",
                            "MEDIUM" | "MINOR" => "medium",
                            "LOW" | "INFO" => "low",
                            _ => "info",
                        };

                        let cwe = extract_cwe(issue);

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

                    // Check if there are more pages
                    let fetched_so_far = (page * page_size) as i64;
                    if fetched_so_far >= total { break; }
                    page += 1;
                } else {
                    break;
                }
            }
            Err(e) => {
                db::insert_scan_log(pool, scan_job_id, "error", Some("sonarqube"),
                    &format!("Failed to fetch SonarQube issues: {}", e)).await;
                break;
            }
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

    // Generate token (USER_TOKEN so it can both submit analysis and read issues)
    let token_resp = client.post(&format!("{}/api/user_tokens/generate", sonar_url))
        .basic_auth("admin", Some(&auth_password))
        .form(&[
            ("name", format!("watchtower-{}", chrono::Utc::now().timestamp())),
            ("type", "USER_TOKEN".to_string()),
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

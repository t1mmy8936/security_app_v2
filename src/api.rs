use actix_web::{web, HttpResponse};
use crate::db::{self, DbPool};
use crate::models::*;
use crate::scanners::runner;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/dashboard", web::get().to(dashboard))
            .route("/scans", web::get().to(list_scans))
            .route("/scans/{id}", web::get().to(get_scan))
            .route("/scans/{id}/status", web::get().to(scan_status))
            .route("/scans/{id}/logs", web::get().to(scan_logs))
            .route("/scans/{id}/findings", web::get().to(scan_findings))
            .route("/scans/{id}/score", web::get().to(scan_score))
            .route("/scans/start", web::post().to(start_scan))
            .route("/scans/{id}/stop", web::post().to(stop_scan))
            .route("/scans/{id}/resume", web::post().to(resume_scan))
            .route("/scans/{id}/cancel", web::post().to(cancel_scan))
            .route("/tools", web::get().to(list_tools))
            .route("/presets", web::get().to(list_presets))
            .route("/settings", web::get().to(get_settings))
            .route("/settings", web::post().to(save_settings))
            .route("/settings/test-email", web::post().to(test_email))
            .route("/settings/test-sonarqube", web::post().to(test_sonarqube))
            .route("/settings/test-openvas", web::post().to(test_openvas))
            .route("/settings/sonarqube-setup", web::post().to(sonarqube_auto_setup))
            .route("/sonarqube/profiles", web::get().to(sonarqube_quality_profiles))
            .route("/reports/{id}/generate", web::post().to(generate_report))
            .route("/reports/{id}/generate-pdf", web::post().to(generate_pdf_report))
            .route("/reports/{id}/download-pdf", web::get().to(download_pdf_report))
            .route("/reports/{id}/email", web::post().to(email_report))
            .route("/reports", web::get().to(list_reports))
            .route("/browse-folders", web::post().to(browse_folders))
            .route("/drives", web::get().to(list_drives))
            .route("/azure/projects", web::get().to(azure_projects))
            .route("/azure/repos/{project}", web::get().to(azure_repos))
            .route("/azure/branches/{project}/{repo}", web::get().to(azure_branches))
    );
}

async fn dashboard(pool: web::Data<DbPool>) -> HttpResponse {
    let total_scans: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scan_jobs")
        .fetch_one(pool.get_ref())
        .await
        .unwrap_or((0,));

    let sums: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(total_findings),0), COALESCE(SUM(critical_count),0),
                COALESCE(SUM(high_count),0), COALESCE(SUM(medium_count),0),
                COALESCE(SUM(low_count),0), COALESCE(SUM(info_count),0) FROM scan_jobs"
    )
    .fetch_one(pool.get_ref())
    .await
    .unwrap_or((0,0,0,0,0,0));

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let scans_today: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scan_jobs WHERE started_at LIKE ?")
        .bind(format!("{}%", today))
        .fetch_one(pool.get_ref())
        .await
        .unwrap_or((0,));

    let avg_dur: (f64,) = sqlx::query_as(
        "SELECT COALESCE(AVG(duration_seconds), 0) FROM scan_jobs WHERE duration_seconds IS NOT NULL"
    )
    .fetch_one(pool.get_ref())
    .await
    .unwrap_or((0.0,));

    let recent: Vec<ScanJob> = fetch_scan_jobs(pool.get_ref(), Some(10)).await;

    let stats = DashboardStats {
        total_scans: total_scans.0,
        total_findings: sums.0,
        critical_findings: sums.1,
        high_findings: sums.2,
        medium_findings: sums.3,
        low_findings: sums.4,
        info_findings: sums.5,
        scans_today: scans_today.0,
        avg_duration: avg_dur.0 as i64,
        most_common_tool: "N/A".into(),
        recent_scans: recent,
    };

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(stats),
    })
}

async fn list_scans(pool: web::Data<DbPool>) -> HttpResponse {
    let scans = fetch_scan_jobs(pool.get_ref(), None).await;
    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(scans) })
}

async fn get_scan(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    let scan = fetch_scan_job(pool.get_ref(), id).await;
    match scan {
        Some(s) => HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(s) }),
        None => HttpResponse::NotFound().json(ApiResponse::<()> { success: false, message: Some("Scan not found".into()), data: None }),
    }
}

#[allow(clippy::type_complexity)]
async fn scan_status(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    let row: Option<(String, Option<String>, Option<i64>, Option<i64>, i64, i64, i64, i64, i64, i64, Option<i64>, Option<String>)> =
        sqlx::query_as(
            "SELECT status, current_tool, tools_total, tools_completed,
                    total_findings, critical_count, high_count, medium_count, low_count, info_count,
                    duration_seconds, completed_at
             FROM scan_jobs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
        .unwrap_or(None);

    match row {
        Some(r) => {
            let resp = ScanStatusResponse {
                status: r.0,
                current_tool: r.1,
                tools_total: r.2,
                tools_completed: r.3,
                total_findings: r.4,
                critical_count: r.5,
                high_count: r.6,
                medium_count: r.7,
                low_count: r.8,
                info_count: r.9,
                duration_seconds: r.10,
                completed_at: r.11,
            };
            HttpResponse::Ok().json(resp)
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()> { success: false, message: Some("Not found".into()), data: None }),
    }
}

async fn scan_logs(pool: web::Data<DbPool>, path: web::Path<i64>, query: web::Query<LogsQuery>) -> HttpResponse {
    let id = path.into_inner();
    let after = query.after.unwrap_or(0);

    let logs: Vec<(String, String, Option<String>, String)> = sqlx::query_as(
        "SELECT timestamp, level, tool, message FROM scan_logs WHERE scan_job_id = ? AND id > ? ORDER BY id ASC"
    )
    .bind(id)
    .bind(after)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let entries: Vec<LogEntry> = logs.into_iter().map(|l| LogEntry {
        timestamp: l.0,
        level: l.1,
        tool: l.2,
        message: l.3,
    }).collect();

    HttpResponse::Ok().json(entries)
}

#[derive(serde::Deserialize)]
struct LogsQuery {
    after: Option<i64>,
}

async fn scan_findings(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    let findings: Vec<Finding> = fetch_findings(pool.get_ref(), id).await;
    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(findings) })
}

fn compute_score(critical: i64, high: i64, medium: i64, low: i64, _info: i64) -> i64 {
    let deductions = critical * 15 + high * 8 + medium * 3 + low;
    (100 - deductions).max(0)
}

fn score_to_grade(score: i64) -> String {
    match score {
        90..=100 => "A+",
        80..=89 => "A",
        70..=79 => "B",
        60..=69 => "C",
        50..=59 => "D",
        25..=49 => "E",
        _ => "F",
    }.to_string()
}

async fn scan_score(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();

    let rows: Vec<(String, i64, i64, i64, i64, i64)> = sqlx::query_as(
        "SELECT tool,
                SUM(CASE WHEN severity='critical' THEN 1 ELSE 0 END),
                SUM(CASE WHEN severity='high' THEN 1 ELSE 0 END),
                SUM(CASE WHEN severity='medium' THEN 1 ELSE 0 END),
                SUM(CASE WHEN severity='low' THEN 1 ELSE 0 END),
                SUM(CASE WHEN severity='info' THEN 1 ELSE 0 END)
         FROM findings WHERE scan_job_id = ? GROUP BY tool"
    )
    .bind(id)
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let mut tool_scores = Vec::new();
    let mut total_c = 0i64;
    let mut total_h = 0i64;
    let mut total_m = 0i64;
    let mut total_l = 0i64;
    let mut total_i = 0i64;

    for (tool, c, h, m, l, i) in &rows {
        total_c += c; total_h += h; total_m += m; total_l += l; total_i += i;
        let score = compute_score(*c, *h, *m, *l, *i);
        tool_scores.push(ToolScore {
            tool: tool.clone(),
            score,
            findings: c + h + m + l + i,
            critical: *c,
            high: *h,
            medium: *m,
            low: *l,
            info: *i,
            grade: score_to_grade(score),
        });
    }

    // If no findings at all, check if scan has tools run (score 100 for each clean tool)
    if tool_scores.is_empty() {
        let tools_run: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT tools_run FROM scan_jobs WHERE id = ?"
        ).bind(id).fetch_optional(pool.get_ref()).await.unwrap_or(None);

        if let Some((Some(tools_str),)) = tools_run {
            for t in tools_str.split(", ") {
                let t = t.trim();
                if !t.is_empty() {
                    tool_scores.push(ToolScore {
                        tool: t.to_string(), score: 100, findings: 0,
                        critical: 0, high: 0, medium: 0, low: 0, info: 0,
                        grade: "A+".to_string(),
                    });
                }
            }
        }
    }

    let overall_score = compute_score(total_c, total_h, total_m, total_l, total_i);
    let overall_grade = score_to_grade(overall_score);

    tool_scores.sort_by(|a, b| a.score.cmp(&b.score));

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: None,
        data: Some(ScanScoreResponse { overall_score, overall_grade, tool_scores }),
    })
}

async fn start_scan(pool: web::Data<DbPool>, body: web::Json<StartScanRequest>) -> HttpResponse {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let tools_str = body.tools.as_ref().map(|t| t.join(", ")).unwrap_or_default();

    let result = sqlx::query(
        "INSERT INTO scan_jobs (scan_type, target, target_source, status, started_at, tools_run) VALUES (?, ?, ?, 'pending', ?, ?)"
    )
    .bind(&body.scan_type)
    .bind(&body.target)
    .bind(&body.target_source)
    .bind(&now)
    .bind(&tools_str)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(r) => {
            let scan_id = r.last_insert_rowid();
            let pool_clone = pool.get_ref().clone();
            tokio::spawn(async move {
                runner::run_scan(pool_clone, scan_id).await;
            });

            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: Some(format!("Scan {} started", scan_id)),
                data: Some(serde_json::json!({ "scan_id": scan_id })),
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            message: Some(format!("Failed to create scan: {}", e)),
            data: None,
        }),
    }
}

async fn stop_scan(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();

    // Check scan is actually running
    let status: Option<(String,)> = sqlx::query_as("SELECT status FROM scan_jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
        .unwrap_or(None);

    match status {
        Some((s,)) if s == "running" => {
            // Signal the runner to stop
            runner::SCAN_CANCEL.write().await.insert(id, true);

            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: Some(format!("Stop signal sent to scan {}", id)),
                data: Some(serde_json::json!({ "scan_id": id })),
            })
        }
        Some((s,)) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: Some(format!("Scan is '{}', cannot stop", s)),
            data: None,
        }),
        None => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: Some("Scan not found".into()),
            data: None,
        }),
    }
}

async fn resume_scan(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();

    let job: Option<(String, String, String, String, Option<String>, Option<i64>)> = sqlx::query_as(
        "SELECT status, scan_type, target, target_source, tools_run, tools_completed FROM scan_jobs WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await
    .unwrap_or(None);

    match job {
        Some((status, _scan_type, _target, _target_source, _tools_run, _tools_completed)) if status == "stopped" => {
            // Update status back to running
            sqlx::query("UPDATE scan_jobs SET status = 'running' WHERE id = ?")
                .bind(id)
                .execute(pool.get_ref())
                .await
                .ok();

            db::insert_scan_log(pool.get_ref(), id, "info", None,
                "▶️ Scan resumed by user").await;

            // Re-spawn the scan runner — it will pick up from where it left off
            // by checking which tools already have findings
            let pool_clone = pool.get_ref().clone();
            tokio::spawn(async move {
                runner::run_scan(pool_clone, id).await;
            });

            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: Some(format!("Scan {} resumed", id)),
                data: Some(serde_json::json!({ "scan_id": id })),
            })
        }
        Some((s,..)) => HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: Some(format!("Scan is '{}', can only resume stopped scans", s)),
            data: None,
        }),
        None => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: Some("Scan not found".into()),
            data: None,
        }),
    }
}

async fn cancel_scan(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();

    // Check scan exists
    let status: Option<(String,)> = sqlx::query_as("SELECT status FROM scan_jobs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
        .unwrap_or(None);

    match status {
        Some((s,)) => {
            // If running, signal stop first
            if s == "running" {
                runner::SCAN_CANCEL.write().await.insert(id, true);
                // Give the runner a moment to notice
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }

            // Delete all related data
            sqlx::query("DELETE FROM scan_logs WHERE scan_job_id = ?").bind(id).execute(pool.get_ref()).await.ok();
            sqlx::query("DELETE FROM findings WHERE scan_job_id = ?").bind(id).execute(pool.get_ref()).await.ok();
            sqlx::query("DELETE FROM reports WHERE scan_job_id = ?").bind(id).execute(pool.get_ref()).await.ok();
            sqlx::query("DELETE FROM scan_jobs WHERE id = ?").bind(id).execute(pool.get_ref()).await.ok();

            // Clean up any report files
            let _ = tokio::fs::remove_file(format!("/app/reports/scan_{}_report.html", id)).await;
            let _ = tokio::fs::remove_file(format!("/app/reports/scan_{}_report.pdf", id)).await;

            // Clean up cancellation token
            runner::SCAN_CANCEL.write().await.remove(&id);

            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: Some(format!("Scan {} cancelled and removed", id)),
                data: Some(serde_json::json!({ "scan_id": id })),
            })
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: Some("Scan not found".into()),
            data: None,
        }),
    }
}

async fn list_tools() -> HttpResponse {
    let tools: Vec<ToolInfo> = runner::TOOLS.iter().map(|t| ToolInfo {
        name: t.name.into(),
        display_name: t.display_name.into(),
        description: t.description.into(),
        available: true,
        category: t.category.into(),
    }).collect();

    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(tools) })
}

async fn list_presets() -> HttpResponse {
    let presets: Vec<ScanPreset> = runner::PRESETS.iter().map(|p| ScanPreset {
        name: p.name.into(),
        display_name: p.display_name.into(),
        description: p.description.into(),
        tools: p.tools.iter().map(|s| s.to_string()).collect(),
    }).collect();

    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(presets) })
}

async fn get_settings(pool: web::Data<DbPool>) -> HttpResponse {
    let keys = [
        "azure_devops_org", "azure_devops_pat", "azure_devops_project",
        "smtp_server", "smtp_port", "smtp_username", "smtp_password",
        "email_from", "email_to",
        "sonarqube_url", "sonarqube_token", "sonarqube_project_key", "sonarqube_exclusions",
        "sonarqube_quality_profile",
        "openvas_url", "openvas_username", "openvas_password",
    ];

    let mut settings = AllSettings {
        azure_devops_org: String::new(),
        azure_devops_pat: String::new(),
        azure_devops_project: String::new(),
        smtp_server: String::new(),
        smtp_port: String::new(),
        smtp_username: String::new(),
        smtp_password: String::new(),
        email_from: String::new(),
        email_to: String::new(),
        sonarqube_url: String::new(),
        sonarqube_token: String::new(),
        sonarqube_project_key: String::new(),
        sonarqube_exclusions: String::new(),
        sonarqube_quality_profile: String::new(),
        openvas_url: String::new(),
        openvas_username: String::new(),
        openvas_password: String::new(),
    };

    for key in &keys {
        let val = db::get_setting(pool.get_ref(), key).await;
        match *key {
            "azure_devops_org" => settings.azure_devops_org = val,
            "azure_devops_pat" => settings.azure_devops_pat = val,
            "azure_devops_project" => settings.azure_devops_project = val,
            "smtp_server" => settings.smtp_server = val,
            "smtp_port" => settings.smtp_port = val,
            "smtp_username" => settings.smtp_username = val,
            "smtp_password" => settings.smtp_password = val,
            "email_from" => settings.email_from = val,
            "email_to" => settings.email_to = val,
            "sonarqube_url" => settings.sonarqube_url = val,
            "sonarqube_token" => settings.sonarqube_token = val,
            "sonarqube_project_key" => settings.sonarqube_project_key = val,
            "sonarqube_exclusions" => settings.sonarqube_exclusions = val,
            "sonarqube_quality_profile" => settings.sonarqube_quality_profile = val,
            "openvas_url" => settings.openvas_url = val,
            "openvas_username" => settings.openvas_username = val,
            "openvas_password" => settings.openvas_password = val,
            _ => {}
        }
    }

    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(settings) })
}

async fn save_settings(pool: web::Data<DbPool>, body: web::Json<SaveSettingsRequest>) -> HttpResponse {
    for pair in &body.settings {
        db::set_setting(pool.get_ref(), &pair.key, &pair.value).await;
    }
    HttpResponse::Ok().json(ApiResponse::<()> { success: true, message: Some("Settings saved".into()), data: None })
}

async fn test_email(pool: web::Data<DbPool>) -> HttpResponse {
    match crate::services::email::send_report_email(
        pool.get_ref(),
        "Watchtower — Test Email",
        "<h1>Test Email</h1><p>If you see this, email is configured correctly.</p>",
        None,
    ).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> { success: true, message: Some("Test email sent".into()), data: None }),
        Err(e) => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(format!("Email failed: {}", e)), data: None }),
    }
}

async fn test_sonarqube(pool: web::Data<DbPool>) -> HttpResponse {
    let url = db::get_setting(pool.get_ref(), "sonarqube_url").await;
    let token = db::get_setting(pool.get_ref(), "sonarqube_token").await;

    if url.is_empty() {
        return HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some("SonarQube URL not configured".into()), data: None });
    }

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/system/status", url))
        .bearer_auth(&token)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            HttpResponse::Ok().json(ApiResponse::<()> { success: true, message: Some("SonarQube connection OK".into()), data: None })
        }
        Ok(r) => {
            HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(format!("SonarQube returned {}", r.status())), data: None })
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(format!("Cannot reach SonarQube: {}", e)), data: None })
        }
    }
}

async fn test_openvas(pool: web::Data<DbPool>) -> HttpResponse {
    let url = db::get_setting(pool.get_ref(), "openvas_url").await;
    let username = db::get_setting(pool.get_ref(), "openvas_username").await;
    let password = db::get_setting(pool.get_ref(), "openvas_password").await;

    if url.is_empty() {
        return HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some("OpenVAS URL not configured".into()), data: None });
    }
    if username.is_empty() || password.is_empty() {
        return HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some("OpenVAS credentials not configured".into()), data: None });
    }

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default();

    let body = serde_json::json!({ "username": username, "password": password });
    let resp = client
        .post(format!("{}/api/v1/tokens", url))
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            // Log out immediately — just testing connectivity
            if let Ok(json) = r.json::<serde_json::Value>().await {
                if let Some(token) = json["data"]["token"].as_str() {
                    let _ = client.delete(format!("{}/api/v1/tokens/{}", url, token)).send().await;
                }
            }
            HttpResponse::Ok().json(ApiResponse::<()> { success: true, message: Some("OpenVAS connection OK".into()), data: None })
        }
        Ok(r) => {
            HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(format!("OpenVAS returned {}", r.status())), data: None })
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(format!("Cannot reach OpenVAS: {}", e)), data: None })
        }
    }
}

async fn sonarqube_auto_setup(pool: web::Data<DbPool>) -> HttpResponse {
    let url = db::get_setting(pool.get_ref(), "sonarqube_url").await;
    let url = if url.is_empty() { "http://sonarqube:9000".to_string() } else { url };

    db::set_setting(pool.get_ref(), "sonarqube_url", &url).await;

    HttpResponse::Ok().json(ApiResponse::<()> {
        success: true,
        message: Some("SonarQube URL saved. Token will be auto-generated on next scan.".into()),
        data: None,
    })
}

async fn sonarqube_quality_profiles(pool: web::Data<DbPool>) -> HttpResponse {
    let url = db::get_setting(pool.get_ref(), "sonarqube_url").await;
    let token = db::get_setting(pool.get_ref(), "sonarqube_token").await;

    if url.is_empty() {
        return HttpResponse::Ok().json(ApiResponse::<Vec<QualityProfile>> {
            success: false,
            message: Some("SonarQube URL not configured".into()),
            data: None,
        });
    }

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/qualityprofiles/search", url))
        .bearer_auth(&token)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            if let Ok(body) = r.json::<serde_json::Value>().await {
                let mut profiles = Vec::new();
                if let Some(arr) = body["profiles"].as_array() {
                    for p in arr {
                        profiles.push(QualityProfile {
                            key: p["key"].as_str().unwrap_or("").to_string(),
                            name: p["name"].as_str().unwrap_or("").to_string(),
                            language: p["language"].as_str().unwrap_or("").to_string(),
                            language_name: p["languageName"].as_str().unwrap_or("").to_string(),
                            is_default: p["isDefault"].as_bool().unwrap_or(false),
                            active_rule_count: p["activeRuleCount"].as_i64().unwrap_or(0),
                            is_built_in: p["isBuiltIn"].as_bool().unwrap_or(false),
                        });
                    }
                }
                HttpResponse::Ok().json(ApiResponse {
                    success: true,
                    message: None,
                    data: Some(profiles),
                })
            } else {
                HttpResponse::Ok().json(ApiResponse::<Vec<QualityProfile>> {
                    success: false,
                    message: Some("Failed to parse SonarQube response".into()),
                    data: None,
                })
            }
        }
        Ok(r) => {
            HttpResponse::Ok().json(ApiResponse::<Vec<QualityProfile>> {
                success: false,
                message: Some(format!("SonarQube returned {}", r.status())),
                data: None,
            })
        }
        Err(e) => {
            HttpResponse::Ok().json(ApiResponse::<Vec<QualityProfile>> {
                success: false,
                message: Some(format!("Cannot reach SonarQube: {}", e)),
                data: None,
            })
        }
    }
}

async fn generate_report(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    match crate::services::report::generate_html_report(pool.get_ref(), id).await {
        Ok(path) => HttpResponse::Ok().json(ApiResponse { success: true, message: Some("Report generated".into()), data: Some(path) }),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()> { success: false, message: Some(e), data: None }),
    }
}

async fn generate_pdf_report(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    match crate::services::report::generate_pdf_report(pool.get_ref(), id).await {
        Ok(path) => HttpResponse::Ok().json(ApiResponse { success: true, message: Some("PDF report generated".into()), data: Some(path) }),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()> { success: false, message: Some(e), data: None }),
    }
}

async fn download_pdf_report(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    let pdf_path = format!("/app/reports/scan_{}_report.pdf", id);

    // Auto-generate if PDF doesn't exist yet
    if !tokio::fs::try_exists(&pdf_path).await.unwrap_or(false) {
        if let Err(e) = crate::services::report::generate_pdf_report(pool.get_ref(), id).await {
            return HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: Some(format!("Failed to generate PDF: {}", e)),
                data: None,
            });
        }
    }

    match tokio::fs::read(&pdf_path).await {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/pdf")
            .insert_header(("Content-Disposition", format!("attachment; filename=\"watchtower_scan_{}_report.pdf\"", id)))
            .body(bytes),
        Err(_) => HttpResponse::InternalServerError().json(ApiResponse::<()> {
            success: false,
            message: Some("PDF generation succeeded but file not found".into()),
            data: None,
        }),
    }
}

async fn email_report(pool: web::Data<DbPool>, path: web::Path<i64>) -> HttpResponse {
    let id = path.into_inner();
    // Generate report first
    let report_path = match crate::services::report::generate_html_report(pool.get_ref(), id).await {
        Ok(p) => p,
        Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()> { success: false, message: Some(e), data: None }),
    };

    // Read the HTML for the email body
    let html = tokio::fs::read_to_string(&report_path).await.unwrap_or_default();

    match crate::services::email::send_report_email(
        pool.get_ref(),
        &format!("Watchtower — Scan #{} Report", id),
        &html,
        Some(&report_path),
    ).await {
        Ok(_) => {
            sqlx::query("UPDATE reports SET emailed = 1 WHERE scan_job_id = ?")
                .bind(id)
                .execute(pool.get_ref())
                .await
                .ok();
            HttpResponse::Ok().json(ApiResponse::<()> { success: true, message: Some("Report emailed".into()), data: None })
        }
        Err(e) => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(format!("Email failed: {}", e)), data: None }),
    }
}

async fn list_reports(pool: web::Data<DbPool>) -> HttpResponse {
    let reports: Vec<(i64, i64, String, String, String, bool)> = sqlx::query_as(
        "SELECT id, scan_job_id, format, file_path, created_at, emailed FROM reports ORDER BY id DESC"
    )
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let reports: Vec<Report> = reports.into_iter().map(|r| Report {
        id: r.0,
        scan_job_id: r.1,
        format: r.2,
        file_path: r.3,
        created_at: r.4,
        emailed: r.5,
    }).collect();

    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(reports) })
}

async fn browse_folders(body: web::Json<BrowseFoldersRequest>) -> HttpResponse {
    let base = body.path.as_deref().unwrap_or("/projects");
    let base_path = std::path::Path::new(base);

    if !base_path.exists() {
        return HttpResponse::Ok().json(BrowseFoldersResponse {
            current_path: base.into(),
            parent: base_path.parent().map(|p| p.to_string_lossy().to_string()),
            entries: vec![],
        });
    }

    let mut entries = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir(base).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }
            let path = entry.path().to_string_lossy().to_string();
            let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
            entries.push(FolderEntry { name, path, is_dir });
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    HttpResponse::Ok().json(BrowseFoldersResponse {
        current_path: base.into(),
        parent: base_path.parent().map(|p| p.to_string_lossy().to_string()),
        entries,
    })
}

async fn list_drives() -> HttpResponse {
    let mut drives = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir("/").await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("host-") {
                if let Ok(ft) = entry.file_type().await {
                    if ft.is_dir() {
                        let letter = name.strip_prefix("host-").unwrap_or(&name).to_uppercase();
                        let path = format!("/{}", name);
                        drives.push(DriveInfo { letter, path });
                    }
                }
            }
        }
    }
    drives.sort_by(|a, b| a.letter.cmp(&b.letter));
    HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(drives) })
}

async fn azure_projects(pool: web::Data<DbPool>) -> HttpResponse {
    let client = crate::services::azure_devops::AzureDevOpsClient::from_settings(pool.get_ref()).await;
    match client {
        Some(c) => match c.list_projects().await {
            Ok(projects) => HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(projects) }),
            Err(e) => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(e), data: None }),
        },
        None => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some("Azure DevOps not configured".into()), data: None }),
    }
}

async fn azure_repos(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let project = path.into_inner();
    let client = crate::services::azure_devops::AzureDevOpsClient::from_settings(pool.get_ref()).await;
    match client {
        Some(c) => match c.list_repos(&project).await {
            Ok(repos) => HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(repos) }),
            Err(e) => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(e), data: None }),
        },
        None => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some("Azure DevOps not configured".into()), data: None }),
    }
}

async fn azure_branches(pool: web::Data<DbPool>, path: web::Path<(String, String)>) -> HttpResponse {
    let (project, repo) = path.into_inner();
    let client = crate::services::azure_devops::AzureDevOpsClient::from_settings(pool.get_ref()).await;
    match client {
        Some(c) => match c.list_branches(&project, &repo).await {
            Ok(branches) => HttpResponse::Ok().json(ApiResponse { success: true, message: None, data: Some(branches) }),
            Err(e) => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some(e), data: None }),
        },
        None => HttpResponse::Ok().json(ApiResponse::<()> { success: false, message: Some("Azure DevOps not configured".into()), data: None }),
    }
}

// ── Helpers ──

async fn fetch_scan_jobs(pool: &DbPool, limit: Option<i64>) -> Vec<ScanJob> {
    let query = if let Some(l) = limit {
        format!("SELECT id, scan_type, target, target_source, status, started_at, completed_at,
                duration_seconds, total_findings, critical_count, high_count, medium_count, low_count, info_count,
                tools_run, file_tree, current_tool, tools_total, tools_completed
                FROM scan_jobs ORDER BY id DESC LIMIT {}", l)
    } else {
        "SELECT id, scan_type, target, target_source, status, started_at, completed_at,
                duration_seconds, total_findings, critical_count, high_count, medium_count, low_count, info_count,
                tools_run, file_tree, current_tool, tools_total, tools_completed
                FROM scan_jobs ORDER BY id DESC".into()
    };

    sqlx::query_as::<_, ScanJob>(&query)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

async fn fetch_scan_job(pool: &DbPool, id: i64) -> Option<ScanJob> {
    sqlx::query_as::<_, ScanJob>(
        "SELECT id, scan_type, target, target_source, status, started_at, completed_at,
                duration_seconds, total_findings, critical_count, high_count, medium_count, low_count, info_count,
                tools_run, file_tree, current_tool, tools_total, tools_completed
         FROM scan_jobs WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

async fn fetch_findings(pool: &DbPool, scan_job_id: i64) -> Vec<Finding> {
    sqlx::query_as::<_, Finding>(
        "SELECT id, scan_job_id, tool, severity, title, description, file_path, line_number,
                cwe_id, cvss_score, raw_output, recommendation,
                text_range_start, text_range_end, status, author, rule_url, data_flow
         FROM findings WHERE scan_job_id = ?
         ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 WHEN 'low' THEN 3 ELSE 4 END"
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

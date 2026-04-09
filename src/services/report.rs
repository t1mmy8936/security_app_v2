use crate::db::DbPool;

fn compute_score(critical: i64, high: i64, medium: i64, low: i64, _info: i64) -> i64 {
    let deductions = critical * 15 + high * 8 + medium * 3 + low * 1;
    (100 - deductions).max(0)
}

fn score_to_grade(score: i64) -> &'static str {
    match score {
        90..=100 => "A+",
        80..=89 => "A",
        70..=79 => "B",
        60..=69 => "C",
        50..=59 => "D",
        25..=49 => "E",
        _ => "F",
    }
}

fn grade_color(grade: &str) -> &'static str {
    match grade {
        "A+" | "A" => "#198754",
        "B" => "#20c997",
        "C" => "#ffc107",
        "D" => "#fd7e14",
        "E" => "#dc3545",
        _ => "#ff0000",
    }
}

pub async fn generate_html_report(pool: &DbPool, scan_job_id: i64) -> Result<String, String> {
    let job = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<i64>, i64, i64, i64, i64, i64, i64, Option<String>)>(
        "SELECT scan_type, target, target_source, status, completed_at, duration_seconds,
                total_findings, critical_count, high_count, medium_count, low_count, info_count, tools_run
         FROM scan_jobs WHERE id = ?"
    )
    .bind(scan_job_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("Scan job not found")?;

    let findings: Vec<(String, String, String, Option<String>, Option<String>, Option<i64>, Option<String>, Option<f64>)> = sqlx::query_as(
        "SELECT tool, severity, title, description, file_path, line_number, cwe_id, cvss_score
         FROM findings WHERE scan_job_id = ? ORDER BY
         CASE severity WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 WHEN 'low' THEN 3 ELSE 4 END"
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let duration_str = job.5.map(|d| format!("{}m {}s", d / 60, d % 60)).unwrap_or("N/A".into());

    // Compute per-tool scores
    let mut tool_counts: std::collections::HashMap<String, (i64,i64,i64,i64,i64)> = std::collections::HashMap::new();
    if let Some(ref tools_str) = job.12 {
        for t in tools_str.split(',') {
            tool_counts.entry(t.trim().to_string()).or_insert((0,0,0,0,0));
        }
    }
    for f in &findings {
        let entry = tool_counts.entry(f.0.clone()).or_insert((0,0,0,0,0));
        match f.1.as_str() {
            "critical" => entry.0 += 1,
            "high" => entry.1 += 1,
            "medium" => entry.2 += 1,
            "low" => entry.3 += 1,
            _ => entry.4 += 1,
        }
    }

    let overall_score = compute_score(job.7, job.8, job.9, job.10, job.11);
    let overall_grade = score_to_grade(overall_score);
    let overall_color = grade_color(overall_grade);

    let mut score_cards_html = String::new();
    let mut sorted_tools: Vec<_> = tool_counts.iter().collect();
    sorted_tools.sort_by_key(|(name, _)| name.clone());
    for (tool, (c, h, m, l, i)) in &sorted_tools {
        let s = compute_score(*c, *h, *m, *l, *i);
        let g = score_to_grade(s);
        let gc = grade_color(g);
        score_cards_html.push_str(&format!(
            r#"<div style="background:#16213e;border-radius:8px;padding:15px;flex:1;min-width:200px;">
                <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;">
                    <strong>{tool}</strong>
                    <span style="background:{gc};color:#fff;padding:2px 10px;border-radius:12px;font-weight:bold;">{g}</span>
                </div>
                <div style="background:#0a0a1a;border-radius:4px;height:8px;overflow:hidden;">
                    <div style="background:{gc};height:100%;width:{s}%;border-radius:4px;"></div>
                </div>
                <div style="font-size:12px;color:#888;margin-top:4px;">{s}/100 — {findings} finding(s)</div>
            </div>"#,
            tool = ammonia::clean(tool),
            gc = gc, g = g, s = s,
            findings = c + h + m + l + i,
        ));
    }

    let dash_offset = 326.73 - (326.73 * overall_score as f64 / 100.0);

    let mut findings_html = String::new();
    for f in &findings {
        let sev_color = match f.1.as_str() {
            "critical" => "#dc3545",
            "high" => "#fd7e14",
            "medium" => "#ffc107",
            "low" => "#0dcaf0",
            _ => "#6c757d",
        };

        findings_html.push_str(&format!(
            r#"<tr>
                <td><span style="background:{}; color:#fff; padding:2px 8px; border-radius:4px; font-size:12px;">{}</span></td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
            sev_color,
            f.1.to_uppercase(),
            ammonia::clean(&f.0),
            ammonia::clean(&f.2),
            ammonia::clean(f.4.as_deref().unwrap_or("-")),
            ammonia::clean(f.6.as_deref().unwrap_or("-")),
        ));
    }

    let html = format!(
        r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Security Scan Report — Scan #{scan_id}</title>
    <style>
        body {{ font-family: 'Segoe UI', sans-serif; background: #1a1a2e; color: #e0e0e0; margin: 0; padding: 20px; }}
        .container {{ max-width: 1000px; margin: 0 auto; }}
        h1 {{ color: #ff4444; text-align: center; }}
        .subtitle {{ text-align: center; color: #888; margin-bottom: 30px; }}
        .stats {{ display: flex; gap: 15px; flex-wrap: wrap; margin-bottom: 30px; }}
        .stat {{ background: #16213e; border-radius: 8px; padding: 15px 20px; flex: 1; min-width: 120px; text-align: center; }}
        .stat .value {{ font-size: 28px; font-weight: bold; }}
        .stat .label {{ font-size: 12px; color: #888; }}
        .critical .value {{ color: #dc3545; }}
        .high .value {{ color: #fd7e14; }}
        .medium .value {{ color: #ffc107; }}
        .low .value {{ color: #0dcaf0; }}
        table {{ width: 100%; border-collapse: collapse; background: #16213e; border-radius: 8px; overflow: hidden; }}
        th {{ background: #0f3460; padding: 12px; text-align: left; }}
        td {{ padding: 10px 12px; border-top: 1px solid #333; }}
        tr:hover {{ background: #1a1a4e; }}
        .score-section {{ text-align: center; margin-bottom: 30px; }}
        .score-ring {{ display: inline-block; position: relative; }}
        .score-ring svg {{ transform: rotate(-90deg); }}
        .score-ring .label {{ position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); }}
        .score-ring .label .number {{ font-size: 36px; font-weight: bold; }}
        .score-ring .label .grade {{ font-size: 18px; }}
        .tool-scores {{ display: flex; gap: 15px; flex-wrap: wrap; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>⚔️ Watchtower — Scan Report</h1>
        <p class="subtitle">Scan #{scan_id} | {scan_type} | Target: {target}</p>

        <div class="score-section">
            <h2>Security Score</h2>
            <div class="score-ring">
                <svg width="130" height="130" viewBox="0 0 130 130">
                    <circle cx="65" cy="65" r="52" fill="none" stroke="#333" stroke-width="10"/>
                    <circle cx="65" cy="65" r="52" fill="none" stroke="{overall_color}" stroke-width="10"
                        stroke-dasharray="326.73" stroke-dashoffset="{dash_offset}" stroke-linecap="round"/>
                </svg>
                <div class="label">
                    <div class="number" style="color:{overall_color}">{overall_score}</div>
                    <div class="grade" style="color:{overall_color}">{overall_grade}</div>
                </div>
            </div>
            <div class="tool-scores">{score_cards}</div>
        </div>

        <div class="stats">
            <div class="stat"><div class="value">{total}</div><div class="label">Total Findings</div></div>
            <div class="stat critical"><div class="value">{crit}</div><div class="label">Critical</div></div>
            <div class="stat high"><div class="value">{high}</div><div class="label">High</div></div>
            <div class="stat medium"><div class="value">{med}</div><div class="label">Medium</div></div>
            <div class="stat low"><div class="value">{low}</div><div class="label">Low</div></div>
        </div>

        <p><strong>Tools:</strong> {tools} | <strong>Duration:</strong> {duration}</p>

        <table>
            <thead>
                <tr><th>Severity</th><th>Tool</th><th>Title</th><th>File</th><th>CWE</th></tr>
            </thead>
            <tbody>
                {findings}
            </tbody>
        </table>

        <p style="text-align:center; color:#555; margin-top:30px; font-size:12px;">
            Generated by Watchtower (Rust) — "I find your lack of security disturbing."
        </p>
    </div>
</body>
</html>"##,
        scan_id = scan_job_id,
        scan_type = ammonia::clean(&job.0),
        target = ammonia::clean(&job.1),
        total = job.6,
        crit = job.7,
        high = job.8,
        med = job.9,
        low = job.10,
        tools = job.12.as_deref().unwrap_or("N/A"),
        duration = duration_str,
        findings = findings_html,
        overall_score = overall_score,
        overall_grade = overall_grade,
        overall_color = overall_color,
        dash_offset = dash_offset,
        score_cards = score_cards_html,
    );

    // Save to file
    let report_dir = "/app/reports";
    let _ = tokio::fs::create_dir_all(report_dir).await;
    let file_path = format!("{}/scan_{}_report.html", report_dir, scan_job_id);
    tokio::fs::write(&file_path, &html).await.map_err(|e| e.to_string())?;

    // Insert into reports table
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO reports (scan_job_id, format, file_path, created_at, emailed) VALUES (?, 'html', ?, ?, 0)"
    )
    .bind(scan_job_id)
    .bind(&file_path)
    .bind(&now)
    .execute(pool)
    .await
    .ok();

    Ok(file_path)
}

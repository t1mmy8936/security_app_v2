use crate::db::DbPool;
use crate::models::PdfExportOptions;

fn compute_score(critical: i64, high: i64, medium: i64, low: i64, _info: i64) -> i64 {
    let deductions = critical * 15 + high * 8 + medium * 3 + low;
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

#[allow(clippy::type_complexity)]
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

    let findings: Vec<(String, String, String, Option<String>, Option<String>, Option<i64>, Option<String>, Option<f64>, Option<String>)> = sqlx::query_as(
        "SELECT tool, severity, title, description, file_path, line_number, cwe_id, cvss_score, issue_type
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
    sorted_tools.sort_by_key(|(name, _)| *name);
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
                <td>{}</td>
            </tr>"#,
            sev_color,
            f.1.to_uppercase(),
            ammonia::clean(&f.0),
            ammonia::clean(&f.2),
            ammonia::clean(f.4.as_deref().unwrap_or("-")),
            ammonia::clean(f.6.as_deref().unwrap_or("-")),
            ammonia::clean(f.8.as_deref().unwrap_or("-")),
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
                <tr><th>Severity</th><th>Tool</th><th>Title</th><th>File</th><th>CWE</th><th>Type</th></tr>
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

#[allow(clippy::type_complexity)]
pub async fn generate_pdf_report(pool: &DbPool, scan_job_id: i64, opts: &PdfExportOptions) -> Result<String, String> {
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

    let all_findings: Vec<(String, String, String, Option<String>, Option<String>, Option<i64>, Option<String>, Option<f64>, Option<String>)> = sqlx::query_as(
        "SELECT tool, severity, title, description, file_path, line_number, cwe_id, cvss_score, issue_type
         FROM findings WHERE scan_job_id = ? ORDER BY
         CASE severity WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 WHEN 'low' THEN 3 ELSE 4 END"
    )
    .bind(scan_job_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Apply export filters
    let tool_f   = opts.tool_filter.as_deref().unwrap_or("all");
    let sev_f    = opts.severity_filter.as_deref().unwrap_or("all");
    let itype_f  = opts.issue_type_filter.as_deref().unwrap_or("all");
    let search_f = opts.search_query.as_deref().unwrap_or("").to_lowercase();

    let findings: Vec<_> = all_findings.into_iter().filter(|f| {
        (tool_f  == "all" || f.0 == tool_f) &&
        (sev_f   == "all" || f.1 == sev_f) &&
        (itype_f == "all" || f.8.as_deref().unwrap_or("") == itype_f) &&
        (search_f.is_empty() ||
            f.2.to_lowercase().contains(&search_f) ||
            f.0.to_lowercase().contains(&search_f) ||
            f.4.as_deref().unwrap_or("").to_lowercase().contains(&search_f))
    }).collect();

    // Determine active columns ("" = all)
    const ALL_COLS: &[&str] = &["severity","tool","title","file","cwe","type","cvss","description"];
    let active_cols: Vec<&str> = match &opts.columns {
        Some(cols) if !cols.is_empty() =>
            ALL_COLS.iter().filter(|&&c| cols.iter().any(|s| s == c)).copied().collect(),
        _ => ALL_COLS.to_vec(),
    };
    let col = |name: &str| -> bool { active_cols.contains(&name) };

    let duration_str = job.5.map(|d| format!("{}m {}s", d / 60, d % 60)).unwrap_or("N/A".into());

    // Per-tool scores
    let mut tool_counts: std::collections::HashMap<String, (i64, i64, i64, i64, i64)> = std::collections::HashMap::new();
    if let Some(ref tools_str) = job.12 {
        for t in tools_str.split(',') {
            tool_counts.entry(t.trim().to_string()).or_insert((0, 0, 0, 0, 0));
        }
    }
    for f in &findings {
        let entry = tool_counts.entry(f.0.clone()).or_insert((0, 0, 0, 0, 0));
        match f.1.as_str() {
            "critical" => entry.0 += 1,
            "high"     => entry.1 += 1,
            "medium"   => entry.2 += 1,
            "low"      => entry.3 += 1,
            _          => entry.4 += 1,
        }
    }

    let overall_score = compute_score(job.7, job.8, job.9, job.10, job.11);
    let overall_grade = score_to_grade(overall_score);

    // Tool score rows
    let mut tool_rows = String::new();
    let mut sorted_tools: Vec<_> = tool_counts.iter().collect();
    sorted_tools.sort_by_key(|(name, _)| *name);
    for (tool, (c, h, m, l, i)) in &sorted_tools {
        let s = compute_score(*c, *h, *m, *l, *i);
        let g = score_to_grade(s);
        let bar_color = match g {
            "A+" | "A" => "#2e7d32",
            "B" => "#1565c0",
            "C" => "#f9a825",
            "D" | "E" => "#e65100",
            _ => "#c62828",
        };
        let badge_bg = bar_color;
        tool_rows.push_str(&format!(
            r#"<tr>
                <td class="tool-name">{tool}</td>
                <td><span class="grade-badge" style="background:{badge_bg}">{g}</span></td>
                <td>{s}/100</td>
                <td>{findings}</td>
                <td>
                    <div class="bar-bg">
                        <div class="bar-fill" style="width:{s}%;background:{bar_color}"></div>
                    </div>
                </td>
            </tr>"#,
            tool    = ammonia::clean(tool),
            g       = g,
            s       = s,
            findings = c + h + m + l + i,
            bar_color = bar_color,
            badge_bg  = badge_bg,
        ));
    }

    // Dynamic findings table header
    let mut header_cells = String::new();
    if col("severity")    { header_cells.push_str("<th style=\"width:70px\">Severity</th>"); }
    if col("tool")        { header_cells.push_str("<th style=\"width:90px\">Tool</th>"); }
    if col("title")       { header_cells.push_str("<th>Title</th>"); }
    if col("description") { header_cells.push_str("<th>Description</th>"); }
    if col("file")        { header_cells.push_str("<th>File</th>"); }
    if col("cwe")         { header_cells.push_str("<th style=\"width:70px\">CWE</th>"); }
    if col("type")        { header_cells.push_str("<th style=\"width:100px\">Type</th>"); }
    if col("cvss")        { header_cells.push_str("<th style=\"width:55px\">CVSS</th>"); }

    // Findings rows
    let mut findings_rows = String::new();
    for f in &findings {
        let (sev_bg, sev_text) = match f.1.as_str() {
            "critical" => ("#c62828", "CRITICAL"),
            "high"     => ("#e65100", "HIGH"),
            "medium"   => ("#f9a825", "MEDIUM"),
            "low"      => ("#1565c0", "LOW"),
            _          => ("#546e7a", "INFO"),
        };
        let file = f.4.as_deref().unwrap_or("-");
        let file_display = if file.len() > 60 { format!("...{}", &file[file.len()-57..]) } else { file.to_string() };
        let desc = f.3.as_deref().unwrap_or("-");
        let desc_display = if desc.len() > 200 { format!("{}...", &desc[..200]) } else { desc.to_string() };

        let mut row = "<tr>".to_string();
        if col("severity")    { row.push_str(&format!("<td><span class=\"sev-badge\" style=\"background:{sev_bg}\">{sev_text}</span></td>")); }
        if col("tool")        { row.push_str(&format!("<td class=\"tool-cell\">{}</td>", ammonia::clean(&f.0))); }
        if col("title")       { row.push_str(&format!("<td class=\"title-cell\">{}</td>", ammonia::clean(&f.2))); }
        if col("description") { row.push_str(&format!("<td class=\"desc-cell\">{}</td>", ammonia::clean(&desc_display))); }
        if col("file")        { row.push_str(&format!("<td class=\"file-cell\">{}</td>", ammonia::clean(&file_display))); }
        if col("cwe")         { row.push_str(&format!("<td>{}</td>", ammonia::clean(f.6.as_deref().unwrap_or("-")))); }
        if col("type")        { row.push_str(&format!("<td>{}</td>", ammonia::clean(f.8.as_deref().unwrap_or("-")))); }
        if col("cvss")        { row.push_str(&format!("<td>{}</td>", f.7.map(|v| format!("{:.1}", v)).unwrap_or("-".into()))); }
        row.push_str("</tr>");
        findings_rows.push_str(&row);
    }

    let grade_color_pdf = match overall_grade {
        "A+" | "A" => "#2e7d32",
        "B"         => "#1565c0",
        "C"         => "#f9a825",
        "D" | "E"   => "#e65100",
        _           => "#c62828",
    };

    let pdf_html = format!(r##"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Watchtower Scan Report — #{scan_id}</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: Arial, Helvetica, sans-serif; font-size: 11pt; color: #212121; background: #fff; padding: 20px 24px; }}

  /* Header */
  .header {{ text-align: center; border-bottom: 3px solid #b71c1c; padding-bottom: 12px; margin-bottom: 18px; }}
  .header h1 {{ font-size: 22pt; color: #b71c1c; letter-spacing: 1px; }}
  .header .subtitle {{ font-size: 10pt; color: #555; margin-top: 4px; }}

  /* Overall score */
  .score-block {{ display: table; width: 100%; margin-bottom: 18px; border: 1px solid #e0e0e0; border-radius: 6px; padding: 14px 18px; }}
  .score-left {{ display: table-cell; vertical-align: middle; width: 140px; text-align: center; }}
  .score-circle {{ display: inline-block; width: 90px; height: 90px; border-radius: 50%; border: 8px solid {grade_color_pdf}; line-height: 74px; text-align: center; font-size: 26pt; font-weight: bold; color: {grade_color_pdf}; }}
  .score-grade {{ font-size: 13pt; font-weight: bold; color: {grade_color_pdf}; margin-top: 4px; }}
  .score-right {{ display: table-cell; vertical-align: middle; padding-left: 20px; }}

  /* Stat bar */
  .stat-row {{ display: table; width: 100%; border-collapse: separate; border-spacing: 8px 0; margin-bottom: 18px; }}
  .stat-cell {{ display: table-cell; text-align: center; border: 1px solid #e0e0e0; border-radius: 6px; padding: 10px 6px; }}
  .stat-cell .val {{ font-size: 20pt; font-weight: bold; }}
  .stat-cell .lbl {{ font-size: 8pt; color: #666; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px; }}
  .critical .val {{ color: #c62828; }}
  .high .val     {{ color: #e65100; }}
  .medium .val   {{ color: #f9a825; }}
  .low .val      {{ color: #1565c0; }}

  /* Section heading */
  .section-title {{ font-size: 12pt; font-weight: bold; color: #212121; border-left: 4px solid #b71c1c; padding-left: 8px; margin-bottom: 10px; margin-top: 18px; }}

  /* Tool score table */
  .tool-table {{ width: 100%; border-collapse: collapse; margin-bottom: 18px; font-size: 10pt; }}
  .tool-table th {{ background: #212121; color: #fff; padding: 7px 10px; text-align: left; }}
  .tool-table td {{ padding: 6px 10px; border-bottom: 1px solid #e0e0e0; vertical-align: middle; }}
  .tool-table tr:last-child td {{ border-bottom: none; }}
  .grade-badge {{ display: inline-block; color: #fff; font-weight: bold; padding: 2px 8px; border-radius: 4px; font-size: 9pt; }}
  .bar-bg {{ background: #e0e0e0; border-radius: 3px; height: 8px; width: 100%; }}
  .bar-fill {{ height: 8px; border-radius: 3px; }}

  /* Findings table */
  .findings-table {{ width: 100%; border-collapse: collapse; font-size: 9pt; }}
  .findings-table th {{ background: #212121; color: #fff; padding: 7px 8px; text-align: left; }}
  .findings-table td {{ padding: 5px 8px; border-bottom: 1px solid #eeeeee; vertical-align: top; }}
  .findings-table tr:nth-child(even) td {{ background: #fafafa; }}
  .sev-badge {{ display: inline-block; color: #fff; font-weight: bold; padding: 1px 6px; border-radius: 3px; font-size: 8pt; white-space: nowrap; }}
  .tool-cell {{ white-space: nowrap; color: #444; }}
  .title-cell {{ max-width: 240px; }}
  .file-cell {{ font-size: 8pt; color: #555; max-width: 180px; word-break: break-all; }}

  /* Footer */
  .footer {{ text-align: center; font-size: 8pt; color: #999; margin-top: 24px; border-top: 1px solid #e0e0e0; padding-top: 8px; }}

  /* Page breaks */
  @media print {{ .page-break {{ page-break-before: always; }} }}
</style>
</head>
<body>

<div class="header">
  <h1>&#9876; Watchtower &mdash; Security Scan Report</h1>
  <div class="subtitle">Scan #{scan_id} &nbsp;|&nbsp; {scan_type} &nbsp;|&nbsp; Target: {target} &nbsp;|&nbsp; Duration: {duration}</div>
</div>

<div class="score-block">
  <div class="score-left">
    <div class="score-circle">{overall_score}</div>
    <div class="score-grade">{overall_grade}</div>
  </div>
  <div class="score-right">
    <div class="stat-row">
      <div class="stat-cell"><div class="val">{total}</div><div class="lbl">Total</div></div>
      <div class="stat-cell critical"><div class="val">{crit}</div><div class="lbl">Critical</div></div>
      <div class="stat-cell high"><div class="val">{high}</div><div class="lbl">High</div></div>
      <div class="stat-cell medium"><div class="val">{med}</div><div class="lbl">Medium</div></div>
      <div class="stat-cell low"><div class="val">{low}</div><div class="lbl">Low</div></div>
    </div>
    <div style="font-size:10pt; color:#555;">Tools: {tools}</div>
  </div>
</div>

<div class="section-title">Tool Scores</div>
<table class="tool-table">
  <thead><tr><th>Tool</th><th>Grade</th><th>Score</th><th>Findings</th><th style="width:180px">Score Bar</th></tr></thead>
  <tbody>{tool_rows}</tbody>
</table>

<div class="section-title">Findings ({total} total)</div>
<table class="findings-table">
  <thead><tr>{header_cells}</tr></thead>
  <tbody>{findings_rows}</tbody>
</table>

<div class="footer">Generated by Watchtower (Rust) &mdash; &ldquo;I find your lack of security disturbing.&rdquo;</div>

</body>
</html>"##,
        scan_id       = scan_job_id,
        scan_type     = ammonia::clean(&job.0),
        target        = ammonia::clean(&job.1),
        duration      = duration_str,
        overall_score = overall_score,
        overall_grade = overall_grade,
        grade_color_pdf = grade_color_pdf,
        total         = job.6,
        crit          = job.7,
        high          = job.8,
        med           = job.9,
        low           = job.10,
        tools         = job.12.as_deref().unwrap_or("N/A"),
        tool_rows     = tool_rows,
        findings_rows = findings_rows,
        header_cells  = header_cells,
    );

    // Write print-friendly HTML to a temp file for wkhtmltopdf
    let pdf_html_path = format!("/app/reports/scan_{}_report_print.html", scan_job_id);
    tokio::fs::write(&pdf_html_path, &pdf_html).await.map_err(|e| e.to_string())?;

    let pdf_path = format!("/app/reports/scan_{}_report.pdf", scan_job_id);

    let output = tokio::process::Command::new("wkhtmltopdf")
        .args([
            "--page-size", "A4",
            "--margin-top", "12mm",
            "--margin-bottom", "12mm",
            "--margin-left", "12mm",
            "--margin-right", "12mm",
            "--enable-local-file-access",
            "--no-stop-slow-scripts",
            "--print-media-type",
            &pdf_html_path,
            &pdf_path,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run wkhtmltopdf: {}", e))?;

    // Clean up temp print HTML
    let _ = tokio::fs::remove_file(&pdf_html_path).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wkhtmltopdf failed: {}", stderr));
    }

    // Insert into reports table
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query(
        "INSERT INTO reports (scan_job_id, format, file_path, created_at, emailed) VALUES (?, 'pdf', ?, ?, 0)"
    )
    .bind(scan_job_id)
    .bind(&pdf_path)
    .bind(&now)
    .execute(pool)
    .await
    .ok();

    Ok(pdf_path)
}

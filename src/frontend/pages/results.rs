use leptos::*;
use leptos_router::*;
use crate::models::*;
use super::super::components::severity_badge::SeverityBadge;
use super::super::components::stat_card::StatCard;
use crate::frontend::app::RESTRICTED_TOOLS;

#[component]
pub fn ResultsPage() -> impl IntoView {
    let params = use_params_map();
    let scan_id = move || {
        params.with(|p| p.get("id").cloned().unwrap_or_default().parse::<i64>().unwrap_or(0))
    };

    let (log_entries, _set_log_entries) = create_signal(Vec::<LogEntry>::new());
    let (status_data, _set_status_data) = create_signal(None::<ScanStatusResponse>);
    let (severity_filter, set_severity_filter) = create_signal("all".to_string());
    let (tool_filter, set_tool_filter) = create_signal("all".to_string());
    let (issue_type_filter, set_issue_type_filter) = create_signal("all".to_string());
    let (search_query, set_search_query) = create_signal(String::new());
    let (show_log, set_show_log) = create_signal(false);
    let (_scan_completed, _set_scan_completed) = create_signal(false);

    let advanced_mode = use_context::<ReadSignal<bool>>().unwrap_or_else(|| create_signal(false).0);

    // PDF export signals
    let (show_pdf_opts,    set_show_pdf_opts)    = create_signal(false);
    let (col_severity,     set_col_severity)     = create_signal(true);
    let (col_tool,         set_col_tool)         = create_signal(true);
    let (col_title,        set_col_title)        = create_signal(true);
    let (col_file,         set_col_file)         = create_signal(true);
    let (col_cwe,          set_col_cwe)          = create_signal(true);
    let (col_issue_type,   set_col_issue_type)   = create_signal(true);
    let (col_cvss,         set_col_cvss)         = create_signal(false);
    let (col_description,  set_col_description)  = create_signal(false);
    let (pdf_tool_scope,   set_pdf_tool_scope)   = create_signal("all".to_string());
    let (pdf_type_scope,   set_pdf_type_scope)   = create_signal("all".to_string());
    let (pdf_busy,         set_pdf_busy)         = create_signal(false);
    let (pdf_msg,          set_pdf_msg)          = create_signal(Option::<(bool, String)>::None);

    let do_generate_pdf = {
        let scan_id = scan_id;
        move |_: leptos::ev::MouseEvent| {
            let id = scan_id();
            let mut cols: Vec<String> = vec![];
            if col_severity.get_untracked()    { cols.push("severity".into()); }
            if col_tool.get_untracked()        { cols.push("tool".into()); }
            if col_title.get_untracked()       { cols.push("title".into()); }
            if col_file.get_untracked()        { cols.push("file".into()); }
            if col_cwe.get_untracked()         { cols.push("cwe".into()); }
            if col_issue_type.get_untracked()  { cols.push("type".into()); }
            if col_cvss.get_untracked()        { cols.push("cvss".into()); }
            if col_description.get_untracked() { cols.push("description".into()); }
            let opts = crate::models::PdfExportOptions {
                tool_filter:        Some(pdf_tool_scope.get_untracked()),
                severity_filter:    Some(severity_filter.get_untracked()),
                issue_type_filter:  Some(pdf_type_scope.get_untracked()),
                search_query: {
                    let q = search_query.get_untracked();
                    if q.is_empty() { None } else { Some(q) }
                },
                columns: Some(cols),
            };
            set_pdf_busy.set(true);
            set_pdf_msg.set(None);
            #[cfg(feature = "hydrate")]
            {
                wasm_bindgen_futures::spawn_local(async move {
                    match gloo_net::http::Request::post(&format!("/api/reports/{}/generate-pdf", id))
                        .json(&opts)
                    {
                        Ok(req) => match req.send().await {
                            Ok(resp) if resp.ok() => {
                                set_pdf_busy.set(false);
                                set_pdf_msg.set(Some((false, "PDF ready — downloading…".into())));
                                if let Some(window) = web_sys::window() {
                                    let _ = window.open_with_url_and_target(
                                        &format!("/api/reports/{}/download-pdf", id), "_blank");
                                }
                            }
                            Ok(resp) => {
                                let msg = resp.text().await.unwrap_or("Server error".into());
                                set_pdf_busy.set(false);
                                set_pdf_msg.set(Some((true, msg)));
                            }
                            Err(e) => {
                                set_pdf_busy.set(false);
                                set_pdf_msg.set(Some((true, e.to_string())));
                            }
                        },
                        Err(e) => {
                            set_pdf_busy.set(false);
                            set_pdf_msg.set(Some((true, e.to_string())));
                        }
                    }
                });
            }
            #[cfg(not(feature = "hydrate"))]
            { let _ = opts; }
        }
    };

    let findings = create_resource(scan_id, |id| async move {
        fetch_findings(id).await
    });

    let scores = create_resource(scan_id, |id| async move {
        fetch_scores(id).await
    });

    // Reactive memo: combines findings resource with all filter signals so
    // any filter change reliably triggers a re-render without re-fetching.
    let filtered_findings = create_memo(move |_| {
        let sev   = severity_filter.get();
        let tool  = tool_filter.get();
        let itype = issue_type_filter.get();
        let query = search_query.get().to_lowercase();
        let adv   = advanced_mode.get();
        findings.get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|f| {
                (adv || !RESTRICTED_TOOLS.contains(&f.tool.as_str())) &&
                (sev   == "all" || f.severity == sev) &&
                (tool  == "all" || f.tool == tool) &&
                (itype == "all" || f.issue_type.as_deref().unwrap_or("") == itype) &&
                (query.is_empty()
                    || f.title.to_lowercase().contains(&query)
                    || f.tool.to_lowercase().contains(&query)
                    || f.file_path.as_deref().unwrap_or("").to_lowercase().contains(&query))
            })
            .collect::<Vec<_>>()
    });

    let available_tools = create_memo(move |_| {
        let adv = advanced_mode.get();
        let mut tools: Vec<String> = findings.get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
            .iter()
            .map(|f| f.tool.clone())
            .filter(|t| adv || !RESTRICTED_TOOLS.contains(&t.as_str()))
            .collect();
        tools.sort();
        tools.dedup();
        tools
    });

    // Poll for status + logs
    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            let id = scan_id();
            let set_status = _set_status_data;
            let set_logs = _set_log_entries;

            let handle = gloo_timers::callback::Interval::new(3_000, move || {
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(s) = fetch_status(id).await {
                        let done = s.status == "completed" || s.status == "failed";
                        set_status.set(Some(s));
                        if done {
                            if !_scan_completed.get_untracked() {
                                _set_scan_completed.set(true);
                                scores.refetch();
                                findings.refetch();
                            }
                            return;
                        }
                    }
                    if let Ok(logs) = fetch_logs(id).await {
                        set_logs.set(logs);
                    }
                });
            });
            on_cleanup(move || drop(handle));
        });
    }

    view! {
        <div class="page-header">
            <h1>"🔎 Scan Results"</h1>
            <p class="page-subtitle">{move || format!("Scan #{}", scan_id())}</p>
        </div>

        // Status overview
        {move || status_data.get().map(|s| {
            let progress = if let (Some(total), Some(done)) = (s.tools_total, s.tools_completed) {
                if total > 0 { (done as f64 / total as f64 * 100.0) as i32 } else { 0 }
            } else { 0 };

            view! {
                <div class="card mb-3">
                    <div class="scan-status-bar">
                        <span class=format!("badge badge-status-{}", s.status)>{s.status.clone()}</span>
                        {s.current_tool.clone().map(|t| view! {
                            <span class="current-tool">" Running: " <strong>{t}</strong></span>
                        })}
                        {s.duration_seconds.map(|d| view! {
                            <span class="scan-duration">"⏱ " {format!("{}m {}s", d / 60, d % 60)}</span>
                        })}
                        <div class="scan-action-buttons">
                            {(s.status == "running").then(|| {
                                let _sid = scan_id();
                                view! {
                                    <button class="btn-scan-action btn-stop" on:click=move |_| {
                                        #[cfg(feature = "hydrate")]
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let _ = post_scan_action(_sid, "stop").await;
                                        });
                                    }>"⏹ Stop"</button>
                                }
                            })}
                            {(s.status == "stopped").then(|| {
                                let _sid = scan_id();
                                view! {
                                    <button class="btn-scan-action btn-resume" on:click=move |_| {
                                        #[cfg(feature = "hydrate")]
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let _ = post_scan_action(_sid, "resume").await;
                                        });
                                    }>"▶ Resume"</button>
                                }
                            })}
                            {(s.status == "completed").then(|| {
                                let sid = scan_id();
                                view! {
                                    <a class="btn-scan-action btn-pdf"
                                        href=format!("/api/reports/{}/download-pdf", sid)
                                        target="_blank"
                                    >"📥 Download PDF"</a>
                                }
                            })}
                        </div>
                    </div>
                    {(s.status == "running").then(|| view! {
                        <div class="progress-bar-wrapper">
                            <div class="progress-bar" style=format!("width: {}%", progress)>
                                {format!("{}%", progress)}
                            </div>
                        </div>
                    })}
                </div>
            }
        })}

        // Security Score Section
        <Suspense fallback=move || view! { <div></div> }>
            {move || scores.get().map(|data| match data {
                Ok(score_data) => view! {
                    <div class="score-section mb-3">
                        <div class="overall-score-card">
                            <div class="score-ring-container">
                                <svg class="score-ring" viewBox="0 0 120 120">
                                    <circle class="score-ring-bg" cx="60" cy="60" r="52" />
                                    <circle class="score-ring-fill"
                                        cx="60" cy="60" r="52"
                                        style=format!("stroke-dasharray: {} {};",
                                            (score_data.overall_score as f64 / 100.0 * 326.73),
                                            326.73)
                                        data-score=score_data.overall_score
                                    />
                                </svg>
                                <div class="score-ring-text">
                                    <span class="score-ring-value">{score_data.overall_score}</span>
                                    <span class="score-ring-label">"/ 100"</span>
                                </div>
                            </div>
                            <div class="overall-score-info">
                                <div class=format!("overall-grade grade-{}", score_data.overall_grade.to_lowercase().replace("+", "plus"))>
                                    {score_data.overall_grade.clone()}
                                </div>
                                <div class="overall-score-title">"Overall Security Score"</div>
                                <div class="overall-score-desc">
                                    {if score_data.overall_score >= 90 {
                                        "Excellent — minimal vulnerabilities detected"
                                    } else if score_data.overall_score >= 70 {
                                        "Good — some issues need attention"
                                    } else if score_data.overall_score >= 50 {
                                        "Fair — several vulnerabilities found"
                                    } else if score_data.overall_score >= 25 {
                                        "Poor — significant security concerns"
                                    } else {
                                        "Critical — immediate action required"
                                    }}
                                </div>
                            </div>
                        </div>
                        <div class="tool-scores-grid">
                            {score_data.tool_scores.into_iter()
                                .filter(|ts| advanced_mode.get_untracked() || !RESTRICTED_TOOLS.contains(&ts.tool.as_str()))
                                .map(|ts| {
                                let bar_color = if ts.score >= 80 { "var(--success)" }
                                    else if ts.score >= 60 { "var(--medium)" }
                                    else if ts.score >= 40 { "var(--high)" }
                                    else { "var(--critical)" };
                                view! {
                                    <div class="tool-score-card">
                                        <div class="tool-score-header">
                                            <span class="tool-score-name">{ts.tool.clone()}</span>
                                            <span class=format!("tool-score-grade grade-{}", ts.grade.to_lowercase().replace("+", "plus"))>
                                                {ts.grade.clone()}
                                            </span>
                                        </div>
                                        <div class="tool-score-bar-wrapper">
                                            <div class="tool-score-bar" style=format!("width: {}%; background: {}", ts.score, bar_color)></div>
                                        </div>
                                        <div class="tool-score-details">
                                            <span class="tool-score-value">{ts.score}"/100"</span>
                                            <span class="tool-score-findings">{ts.findings}" findings"</span>
                                        </div>
                                        <div class="tool-score-breakdown">
                                            {(ts.critical > 0).then(|| view! {
                                                <span class="tsb-critical">{ts.critical}" C"</span>
                                            })}
                                            {(ts.high > 0).then(|| view! {
                                                <span class="tsb-high">{ts.high}" H"</span>
                                            })}
                                            {(ts.medium > 0).then(|| view! {
                                                <span class="tsb-medium">{ts.medium}" M"</span>
                                            })}
                                            {(ts.low > 0).then(|| view! {
                                                <span class="tsb-low">{ts.low}" L"</span>
                                            })}
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                }.into_view(),
                Err(_) => view! { <div></div> }.into_view(),
            })}
        </Suspense>

        // Stats cards
        {move || status_data.get().map(|s| view! {
            <div class="stats-grid">
                <StatCard label="Total" value=s.total_findings.to_string()/>
                <StatCard label="Critical" value=s.critical_count.to_string() color="critical".to_string()/>
                <StatCard label="High" value=s.high_count.to_string() color="high".to_string()/>
                <StatCard label="Medium" value=s.medium_count.to_string() color="medium".to_string()/>
                <StatCard label="Low" value=s.low_count.to_string() color="low".to_string()/>
                <StatCard label="Info" value=s.info_count.to_string() color="info".to_string()/>
            </div>
        })}

        // Findings section with filters
        <div class="card mt-3">
            <div class="findings-header">
                <h3>"Findings"</h3>
                <div class="findings-filters">
                    <select class="form-control filter-select"
                        on:change=move |ev| set_severity_filter.set(event_target_value(&ev))>
                        <option value="all">"All Severities"</option>
                        <option value="critical">"🔴 Critical"</option>
                        <option value="high">"🟠 High"</option>
                        <option value="medium">"🟡 Medium"</option>
                        <option value="low">"🔵 Low"</option>
                        <option value="info">"⚪ Info"</option>
                    </select>
                    <select class="form-control filter-select"
                        on:change=move |ev| set_tool_filter.set(event_target_value(&ev))>
                        <option value="all">"All Tools"</option>
                        {move || available_tools.get().into_iter().map(|t| view! {
                            <option value={t.clone()}>{t}</option>
                        }).collect_view()}
                    </select>
                    <select class="form-control filter-select"
                        on:change=move |ev| set_issue_type_filter.set(event_target_value(&ev))>
                        <option value="all">"All Types"</option>
                        <option value="Vulnerability">"Vulnerability"</option>
                        <option value="Bug">"Bug"</option>
                        <option value="Code Smell">"Code Smell"</option>
                        <option value="Security Hotspot">"Security Hotspot"</option>
                    </select>
                    <input type="text" class="form-control filter-search"
                        placeholder="Search findings..."
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))/>
                </div>
            </div>

            <Suspense fallback=move || view! { <p>"Loading findings..."</p> }>
                {move || findings.get().map(|data| match data {
                    Ok(_) => {
                        view! {
                            // ── PDF export panel ──────────────────────────────
                            <div class="pdf-export-section">
                                <button class="pdf-export-toggle"
                                    on:click=move |_| set_show_pdf_opts.update(|v| *v = !*v)>
                                    {move || if show_pdf_opts.get() { "▲ PDF Options" } else { "⚙ PDF Export Options" }}
                                </button>
                                {move || show_pdf_opts.get().then(|| view! {
                                    <div class="pdf-export-panel">
                                        <p class="pdf-hint">
                                            "The PDF will include findings matching the active filters above. "
                                            "Choose additional scope and which columns to output."
                                        </p>
                                        <div class="pdf-option-groups">
                                            // Tool scope
                                            <div class="pdf-option-group">
                                                <div class="pdf-option-label">"PDF Tool Scope"</div>
                                                <select class="form-control filter-select"
                                                    on:change=move |ev| set_pdf_tool_scope.set(event_target_value(&ev))>
                                                    <option value="all">"All Tools"</option>
                                                    {move || available_tools.get().into_iter().map(|t| view! {
                                                        <option value={t.clone()}>{t}</option>
                                                    }).collect_view()}
                                                </select>
                                            </div>
                                            // Issue type scope
                                            <div class="pdf-option-group">
                                                <div class="pdf-option-label">"PDF Type Scope"</div>
                                                <select class="form-control filter-select"
                                                    on:change=move |ev| set_pdf_type_scope.set(event_target_value(&ev))>
                                                    <option value="all">"All Types"</option>
                                                    <option value="Vulnerability">"Vulnerability"</option>
                                                    <option value="Bug">"Bug"</option>
                                                    <option value="Code Smell">"Code Smell"</option>
                                                    <option value="Security Hotspot">"Security Hotspot"</option>
                                                </select>
                                            </div>
                                            // Column selection
                                            <div class="pdf-option-group">
                                                <div class="pdf-option-label">"Columns"</div>
                                                <div class="pdf-col-checks">
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=true
                                                            on:change=move |ev| set_col_severity.set(checkbox_checked(&ev))/>
                                                        " Severity"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=true
                                                            on:change=move |ev| set_col_tool.set(checkbox_checked(&ev))/>
                                                        " Tool"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=true
                                                            on:change=move |ev| set_col_title.set(checkbox_checked(&ev))/>
                                                        " Title"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=true
                                                            on:change=move |ev| set_col_file.set(checkbox_checked(&ev))/>
                                                        " File"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=true
                                                            on:change=move |ev| set_col_cwe.set(checkbox_checked(&ev))/>
                                                        " CWE"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=true
                                                            on:change=move |ev| set_col_issue_type.set(checkbox_checked(&ev))/>
                                                        " Type"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=false
                                                            on:change=move |ev| set_col_cvss.set(checkbox_checked(&ev))/>
                                                        " CVSS"
                                                    </label>
                                                    <label class="pdf-col-check">
                                                        <input type="checkbox" checked=false
                                                            on:change=move |ev| set_col_description.set(checkbox_checked(&ev))/>
                                                        " Description"
                                                    </label>
                                                </div>
                                            </div>
                                        </div>
                                        <div class="pdf-actions">
                                            <button class="btn-generate-pdf"
                                                disabled=move || pdf_busy.get()
                                                on:click=do_generate_pdf.clone()>
                                                {move || if pdf_busy.get() { "⏳ Generating…" } else { "📄 Generate & Download PDF" }}
                                            </button>
                                            {move || pdf_msg.get().map(|(_is_err, msg)| view! {
                                                <span class=if _is_err { "pdf-msg pdf-msg-err" } else { "pdf-msg pdf-msg-ok" }>
                                                    {msg}
                                                </span>
                                            })}
                                        </div>
                                    </div>
                                })}
                            </div>

                            // ── Findings list ─────────────────────────────────
                            <div class="findings-count">
                                {move || filtered_findings.get().len()}
                                " finding(s) shown"
                            </div>
                            <div class="findings-list">
                                <For
                                    each=move || filtered_findings.get()
                                    key=|f| f.id
                                    children=|finding| {
                                        let cvss_class = finding.cvss_score.map(|v| {
                                            if v >= 9.0 { "cvss-critical" }
                                            else if v >= 7.0 { "cvss-high" }
                                            else if v >= 4.0 { "cvss-medium" }
                                            else { "cvss-low" }
                                        }).unwrap_or("");
                                        view! {
                                            <div class=format!("finding-card finding-sev-{}", finding.severity)>
                                                <div class="finding-card-header">
                                                    <SeverityBadge severity=finding.severity.clone()/>
                                                    <span class="finding-tool-badge">{finding.tool.clone()}</span>
                                                    {finding.issue_type.clone().map(|it| view! {
                                                        <span class=format!("finding-type-badge finding-type-{}",
                                                            it.to_lowercase().replace(' ', "-"))>{it}</span>
                                                    })}
                                                    <span class="finding-title">{finding.title.clone()}</span>
                                                    {finding.cvss_score.map(|v| view! {
                                                        <span class=format!("finding-cvss {}", cvss_class)>
                                                            "CVSS: " {format!("{:.1}", v)}
                                                        </span>
                                                    })}
                                                </div>
                                                {finding.description.clone().map(|desc| view! {
                                                    <div class="finding-desc">{desc}</div>
                                                })}
                                                <div class="finding-meta">
                                                    {finding.file_path.clone().map(|fp| view! {
                                                        <span class="finding-file">"📄 " {fp}
                                                            {finding.line_number.map(|ln| format!(":{}", ln))}
                                                        </span>
                                                    })}
                                                    {finding.cwe_id.clone().map(|cwe| view! {
                                                        <a href=format!("https://cwe.mitre.org/data/definitions/{}.html",
                                                            cwe.trim_start_matches("CWE-"))
                                                            target="_blank" class="cwe-link">{cwe}</a>
                                                    })}
                                                    {finding.recommendation.clone().map(|rec| view! {
                                                        <span class="finding-rec">"💡 " {rec}</span>
                                                    })}
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_view()
                    }
                    Err(_) => view! { <div class="loading">"Loading..."</div> }.into_view(),
                })}
            </Suspense>
        </div>

        // Collapsible scan log
        <div class="card mt-3">
            <div class="log-header" on:click=move |_| set_show_log.update(|v| *v = !*v)>
                <h3>"📟 Scan Log"</h3>
                <span class="log-toggle">{move || if show_log.get() { "▲ Hide" } else { "▼ Show" }}</span>
            </div>
            {move || show_log.get().then(|| view! {
                <div class="scan-log-terminal">
                    <For
                        each=move || log_entries.get()
                        key=|l| l.timestamp.clone()
                        children=|log| {
                            let level_class = format!("log-{}", log.level);
                            view! {
                                <div class=format!("log-entry {}", level_class)>
                                    <span class="log-time">{log.timestamp.clone()}</span>
                                    {log.tool.clone().map(|t| view! {
                                        <span class="log-tool">{format!("[{}]", t)}</span>
                                    })}
                                    <span class="log-msg">{log.message.clone()}</span>
                                </div>
                            }
                        }
                    />
                </div>
            })}
        </div>
    }
}

fn checkbox_checked(ev: &leptos::ev::Event) -> bool {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|e| e.checked())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = ev; false }
}

fn event_target_value(ev: &leptos::ev::Event) -> String {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()
                .map(|e| e.value())
                .or_else(|| ev.target()
                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|e| e.value())))
            .unwrap_or_default()
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = ev;
        String::new()
    }
}

#[allow(dead_code)]
async fn fetch_scan(id: i64) -> Result<ScanJob, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<ScanJob> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("Not found".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn fetch_status(id: i64) -> Result<ScanStatusResponse, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/status", id))
            .send().await.map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn fetch_logs(id: i64) -> Result<Vec<LogEntry>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/logs", id))
            .send().await.map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

async fn fetch_findings(id: i64) -> Result<Vec<Finding>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/findings", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<Finding>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

async fn fetch_scores(id: i64) -> Result<ScanScoreResponse, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get(&format!("/api/scans/{}/score", id))
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<ScanScoreResponse> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = id; Err("SSR".into()) }
}

#[allow(dead_code)]
async fn post_scan_action(id: i64, action: &str) -> Result<(), String> {
    #[cfg(feature = "hydrate")]
    {
        let _ = gloo_net::http::Request::post(&format!("/api/scans/{}/{}", id, action))
            .send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = (id, action); Err("SSR".into()) }
}

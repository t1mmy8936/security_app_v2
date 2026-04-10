use leptos::*;
use leptos_router::*;
use crate::models::*;
use super::super::components::severity_badge::SeverityBadge;
use super::super::components::stat_card::StatCard;

#[component]
pub fn ResultsPage() -> impl IntoView {
    let params = use_params_map();
    let scan_id = move || {
        params.with(|p| p.get("id").cloned().unwrap_or_default().parse::<i64>().unwrap_or(0))
    };

    let (log_entries, set_log_entries) = create_signal(Vec::<LogEntry>::new());
    let (status_data, set_status_data) = create_signal(None::<ScanStatusResponse>);
    let (severity_filter, set_severity_filter) = create_signal("all".to_string());
    let (tool_filter, set_tool_filter) = create_signal("all".to_string());
    let (search_query, set_search_query) = create_signal(String::new());
    let (show_log, set_show_log) = create_signal(false);

    let scan = create_resource(move || scan_id(), |id| async move {
        fetch_scan(id).await
    });

    let findings = create_resource(move || scan_id(), |id| async move {
        fetch_findings(id).await
    });

    let scores = create_resource(move || scan_id(), |id| async move {
        fetch_scores(id).await
    });

    // Poll for status + logs
    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            let id = scan_id();
            let set_status = set_status_data;
            let set_logs = set_log_entries;

            gloo_timers::callback::Interval::new(3_000, move || {
                let set_status = set_status.clone();
                let set_logs = set_logs.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(s) = fetch_status(id).await {
                        let done = s.status == "completed" || s.status == "failed";
                        set_status.set(Some(s));
                        if done { return; }
                    }
                    if let Ok(logs) = fetch_logs(id).await {
                        set_logs.set(logs);
                    }
                });

            }).forget();
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
                                let sid = scan_id();
                                view! {
                                    <button class="btn-scan-action btn-stop" on:click=move |_| {
                                        #[cfg(feature = "hydrate")]
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let _ = post_scan_action(sid, "stop").await;
                                        });
                                    }>"⏹ Stop"</button>
                                }
                            })}
                            {(s.status == "stopped").then(|| {
                                let sid = scan_id();
                                view! {
                                    <button class="btn-scan-action btn-resume" on:click=move |_| {
                                        #[cfg(feature = "hydrate")]
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let _ = post_scan_action(sid, "resume").await;
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
                            {score_data.tool_scores.into_iter().map(|ts| {
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
                        <option value="critical">"Critical"</option>
                        <option value="high">"High"</option>
                        <option value="medium">"Medium"</option>
                        <option value="low">"Low"</option>
                        <option value="info">"Info"</option>
                    </select>
                    <Suspense fallback=move || view! { <span></span> }>
                        {move || findings.get().map(|data| match data {
                            Ok(ref fl) => {
                                let mut tools: Vec<String> = fl.iter().map(|f| f.tool.clone()).collect();
                                tools.sort();
                                tools.dedup();
                                view! {
                                    <select class="form-control filter-select"
                                        on:change=move |ev| set_tool_filter.set(event_target_value(&ev))>
                                        <option value="all">"All Tools"</option>
                                        {tools.into_iter().map(|t| view! {
                                            <option value={t.clone()}>{t}</option>
                                        }).collect_view()}
                                    </select>
                                }.into_view()
                            }
                            Err(_) => view! { <span></span> }.into_view(),
                        })}
                    </Suspense>
                    <input type="text" class="form-control filter-search"
                        placeholder="Search findings..."
                        on:input=move |ev| set_search_query.set(event_target_value(&ev))/>
                </div>
            </div>

            <Suspense fallback=move || view! { <p>"Loading findings..."</p> }>
                {move || findings.get().map(|data| match data {
                    Ok(finding_list) => {
                        let sev = severity_filter.get();
                        let tool = tool_filter.get();
                        let query = search_query.get().to_lowercase();
                        let filtered: Vec<_> = finding_list.iter().filter(|f| {
                            (sev == "all" || f.severity == sev) &&
                            (tool == "all" || f.tool == tool) &&
                            (query.is_empty() || f.title.to_lowercase().contains(&query)
                                || f.tool.to_lowercase().contains(&query)
                                || f.file_path.as_deref().unwrap_or("").to_lowercase().contains(&query))
                        }).cloned().collect();

                        let count = filtered.len();

                        view! {
                            <div class="findings-count">{count}" finding(s) shown"</div>
                            <div class="findings-list">
                                {filtered.into_iter().map(|finding| {
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
                                }).collect_view()}
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

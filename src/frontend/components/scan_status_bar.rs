use leptos::*;
use crate::models::*;

#[component]
pub fn ScanStatusBar() -> impl IntoView {
    let (scans, set_scans) = create_signal(Vec::<ScanJob>::new());
    let (minimized, set_minimized) = create_signal(false);
    let (dismissed_ids, set_dismissed_ids) = create_signal(Vec::<i64>::new());

    #[cfg(feature = "hydrate")]
    {
        create_effect(move |_| {
            let set_scans = set_scans.clone();
            gloo_timers::callback::Interval::new(5_000, move || {
                let set_scans = set_scans.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(data) = fetch_active_scans().await {
                        set_scans.set(data);
                    }
                });
            }).forget();

            // Initial fetch
            let set_scans2 = set_scans.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(data) = fetch_active_scans().await {
                    set_scans2.set(data);
                }
            });
        });
    }

    let has_active = move || {
        scans.get().iter().any(|s| s.status == "running" || s.status == "pending" || s.status == "stopped")
    };

    view! {
        {move || {
            let scan_list = scans.get();
            let active: Vec<_> = scan_list.iter()
                .filter(|s| s.status == "running" || s.status == "pending" || s.status == "stopped")
                .cloned().collect();
            let dismissed = dismissed_ids.get();
            let recent_done: Vec<_> = scan_list.iter()
                .filter(|s| (s.status == "completed" || s.status == "failed") && !dismissed.contains(&s.id))
                .take(3)
                .cloned().collect();

            let show_bar = !active.is_empty() || !recent_done.is_empty();

            if !show_bar {
                return view! { <div></div> }.into_view();
            }

            let is_min = minimized.get();

            view! {
                <div class=format!("scan-status-bottom-bar {}", if is_min { "minimized" } else { "" })>
                    <div class="ssb-header" on:click=move |_| set_minimized.update(|v| *v = !*v)>
                        <span class="ssb-indicator">
                            {if !active.is_empty() {
                                view! { <span class="ssb-dot ssb-dot-active"></span> }.into_view()
                            } else {
                                view! { <span class="ssb-dot ssb-dot-idle"></span> }.into_view()
                            }}
                            {if active.len() > 0 {
                                format!("{} scan(s) active", active.len())
                            } else {
                                "No active scans".to_string()
                            }}
                        </span>
                        <div class="ssb-header-actions">
                            {(!active.is_empty() || !recent_done.is_empty()).then(|| {
                                let set_dismissed = set_dismissed_ids.clone();
                                let done_ids: Vec<i64> = scan_list.iter()
                                    .filter(|s| s.status == "completed" || s.status == "failed")
                                    .map(|s| s.id).collect();
                                view! {
                                    <button class="ssb-clear-btn"
                                        title="Clear completed"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            set_dismissed.set(done_ids.clone());
                                        }>
                                        "✕ Clear"
                                    </button>
                                }
                            })}
                            <span class="ssb-toggle">{if is_min { "▲" } else { "▼" }}</span>
                        </div>
                    </div>
                    {(!is_min).then(|| view! {
                        <div class="ssb-content">
                            {active.into_iter().map(|scan| {
                                let progress = match (scan.tools_total, scan.tools_completed) {
                                    (Some(total), Some(done)) if total > 0 => (done as f64 / total as f64 * 100.0) as i32,
                                    _ => 0,
                                };
                                let target_short = if scan.target.len() > 40 {
                                    format!("...{}", &scan.target[scan.target.len()-37..])
                                } else {
                                    scan.target.clone()
                                };
                                let scan_class = if scan.status == "stopped" { "ssb-scan-stopped" } else { "ssb-scan-running" };
                                view! {
                                    <div class=format!("ssb-scan {}", scan_class)>
                                        <div class="ssb-scan-info">
                                            <a href=format!("/scans/{}", scan.id) class="ssb-scan-id">
                                                {"#"}{scan.id}
                                            </a>
                                            <span class=format!("badge badge-status-{}", scan.status)>{scan.status.clone()}</span>
                                            <span class="ssb-scan-target" title=scan.target.clone()>{target_short}</span>
                                            {scan.current_tool.clone().map(|t| view! {
                                                <span class="ssb-current-tool">"⚙ " {t}</span>
                                            })}
                                            {(scan.status == "running").then(|| {
                                                let sid = scan.id;
                                                view! {
                                                    <button class="ssb-action-btn ssb-stop-btn" title="Stop scan"
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                            #[cfg(feature = "hydrate")]
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                let _ = post_scan_action(sid, "stop").await;
                                                            });
                                                        }>"⏹"</button>
                                                }
                                            })}
                                            {(scan.status == "stopped").then(|| {
                                                let sid = scan.id;
                                                view! {
                                                    <button class="ssb-action-btn ssb-resume-btn" title="Resume scan"
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                            #[cfg(feature = "hydrate")]
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                let _ = post_scan_action(sid, "resume").await;
                                                            });
                                                        }>"▶"</button>
                                                }
                                            })}
                                            {(scan.status == "running" || scan.status == "stopped").then(|| {
                                                let sid = scan.id;
                                                view! {
                                                    <button class="ssb-action-btn ssb-cancel-btn" title="Cancel and remove"
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                            #[cfg(feature = "hydrate")]
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                let _ = post_scan_action(sid, "cancel").await;
                                                            });
                                                        }>"🗑"</button>
                                                }
                                            })}
                                        </div>
                                        <div class="ssb-progress">
                                            <div class="ssb-progress-bar" style=format!("width:{}%", progress)></div>
                                        </div>
                                        <div class="ssb-scan-meta">
                                            <span>{format!("{}%", progress)}</span>
                                            <span>{format!("{}/{} tools", scan.tools_completed.unwrap_or(0), scan.tools_total.unwrap_or(0))}</span>
                                            <span class="ssb-findings-count">
                                                {(scan.critical_count > 0).then(|| view! { <span class="tsb-critical">{scan.critical_count}" C"</span> })}
                                                {(scan.high_count > 0).then(|| view! { <span class="tsb-high">{scan.high_count}" H"</span> })}
                                                {(scan.medium_count > 0).then(|| view! { <span class="tsb-medium">{scan.medium_count}" M"</span> })}
                                                {(scan.low_count > 0).then(|| view! { <span class="tsb-low">{scan.low_count}" L"</span> })}
                                            </span>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                            {recent_done.into_iter().map(|scan| {
                                let target_short = if scan.target.len() > 40 {
                                    format!("...{}", &scan.target[scan.target.len()-37..])
                                } else {
                                    scan.target.clone()
                                };
                                let is_failed = scan.status == "failed";
                                view! {
                                    <div class=format!("ssb-scan {}", if is_failed { "ssb-scan-failed" } else { "ssb-scan-done" })>
                                        <div class="ssb-scan-info">
                                            <a href=format!("/scans/{}", scan.id) class="ssb-scan-id">
                                                {"#"}{scan.id}
                                            </a>
                                            <span class=format!("badge badge-status-{}", scan.status)>{scan.status.clone()}</span>
                                            <span class="ssb-scan-target" title=scan.target.clone()>{target_short}</span>
                                            <span class="ssb-findings-total">{scan.total_findings}" findings"</span>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    })}
                </div>
            }.into_view()
        }}
    }
}

async fn fetch_active_scans() -> Result<Vec<ScanJob>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/scans")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ScanJob>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Err("SSR".into())
    }
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

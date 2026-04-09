use leptos::*;
use crate::models::*;
use super::super::components::stat_card::StatCard;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let stats = create_resource(|| (), |_| async move {
        fetch_dashboard().await
    });

    view! {
        <div class="page-header">
            <h1>"📊 Dashboard"</h1>
            <p class="page-subtitle">"Security scanning overview"</p>
        </div>

        <Suspense fallback=move || view! { <div class="loading">"Loading dashboard..."</div> }>
            {move || stats.get().map(|data| match data {
                Ok(d) => {
                    let total_findings = d.total_findings.max(1) as f64;
                    let crit_pct = (d.critical_findings as f64 / total_findings * 100.0) as i32;
                    let high_pct = (d.high_findings as f64 / total_findings * 100.0) as i32;
                    let med_pct = (d.medium_findings as f64 / total_findings * 100.0) as i32;
                    let low_pct = (d.low_findings as f64 / total_findings * 100.0) as i32;
                    let info_pct = 100 - crit_pct - high_pct - med_pct - low_pct;

                    view! {
                        <div class="stats-grid">
                            <StatCard label="Total Scans" value=d.total_scans.to_string()/>
                            <StatCard label="Total Findings" value=d.total_findings.to_string()/>
                            <StatCard label="Critical" value=d.critical_findings.to_string() color="critical".to_string()/>
                            <StatCard label="High" value=d.high_findings.to_string() color="high".to_string()/>
                            <StatCard label="Medium" value=d.medium_findings.to_string() color="medium".to_string()/>
                            <StatCard label="Scans Today" value=d.scans_today.to_string()/>
                            <StatCard label="Avg Duration" value=format!("{}s", d.avg_duration)/>
                        </div>

                        // Severity distribution chart
                        {(d.total_findings > 0).then(|| view! {
                            <div class="card mb-3">
                                <h3>"Severity Distribution"</h3>
                                <div class="severity-chart">
                                    <div class="severity-bar-stack">
                                        {(crit_pct > 0).then(|| view! {
                                            <div class="sev-bar sev-bar-critical" style=format!("width:{}%", crit_pct)>
                                                {(crit_pct > 5).then(|| format!("{}%", crit_pct))}
                                            </div>
                                        })}
                                        {(high_pct > 0).then(|| view! {
                                            <div class="sev-bar sev-bar-high" style=format!("width:{}%", high_pct)>
                                                {(high_pct > 5).then(|| format!("{}%", high_pct))}
                                            </div>
                                        })}
                                        {(med_pct > 0).then(|| view! {
                                            <div class="sev-bar sev-bar-medium" style=format!("width:{}%", med_pct)>
                                                {(med_pct > 5).then(|| format!("{}%", med_pct))}
                                            </div>
                                        })}
                                        {(low_pct > 0).then(|| view! {
                                            <div class="sev-bar sev-bar-low" style=format!("width:{}%", low_pct)>
                                                {(low_pct > 5).then(|| format!("{}%", low_pct))}
                                            </div>
                                        })}
                                        {(info_pct > 0).then(|| view! {
                                            <div class="sev-bar sev-bar-info" style=format!("width:{}%", info_pct)>
                                                {(info_pct > 5).then(|| format!("{}%", info_pct))}
                                            </div>
                                        })}
                                    </div>
                                    <div class="severity-legend">
                                        <span class="legend-item"><span class="legend-dot" style="background:var(--critical)"></span>{format!("Critical: {}", d.critical_findings)}</span>
                                        <span class="legend-item"><span class="legend-dot" style="background:var(--high)"></span>{format!("High: {}", d.high_findings)}</span>
                                        <span class="legend-item"><span class="legend-dot" style="background:var(--medium)"></span>{format!("Medium: {}", d.medium_findings)}</span>
                                        <span class="legend-item"><span class="legend-dot" style="background:var(--low)"></span>{format!("Low: {}", d.low_findings)}</span>
                                        <span class="legend-item"><span class="legend-dot" style="background:var(--info)"></span>{format!("Info: {}", d.info_findings)}</span>
                                    </div>
                                </div>
                            </div>
                        })}

                        <div class="card mt-4">
                            <h3>"Recent Scans"</h3>
                            {if d.recent_scans.is_empty() {
                                view! {
                                    <div class="empty-state">
                                        <p>"No scans yet. Go to New Scan to get started."</p>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <table class="data-table">
                                        <thead>
                                            <tr>
                                                <th>"ID"</th>
                                                <th>"Type"</th>
                                                <th>"Target"</th>
                                                <th>"Status"</th>
                                                <th>"Findings"</th>
                                                <th>"Date"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            <For
                                                each=move || d.recent_scans.clone()
                                                key=|s| s.id
                                                children=|scan| view! {
                                                    <tr class="clickable-row" on:click=move |_| {
                                                        let _ = leptos_router::use_navigate()(&format!("/scans/{}", scan.id), Default::default());
                                                    }>
                                                        <td>{scan.id}</td>
                                                        <td><span class="badge badge-type">{scan.scan_type.clone()}</span></td>
                                                        <td class="target-cell">{scan.target.clone()}</td>
                                                        <td><span class=format!("badge badge-status-{}", scan.status)>{scan.status.clone()}</span></td>
                                                        <td>{scan.total_findings}</td>
                                                        <td>{scan.started_at.clone()}</td>
                                                    </tr>
                                                }
                                            />
                                        </tbody>
                                    </table>
                                }.into_view()
                            }}
                        </div>
                    }.into_view()
                },
                Err(_) => view! { <div class="loading">"Loading..."</div> }.into_view(),
            })}
        </Suspense>
    }
}

async fn fetch_dashboard() -> Result<DashboardStats, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/dashboard")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let api: ApiResponse<DashboardStats> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    {
        Err("SSR".into())
    }
}

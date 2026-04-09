use leptos::*;
use crate::models::*;

#[component]
pub fn ScansPage() -> impl IntoView {
    let scans = create_resource(|| (), |_| async { fetch_scans().await });

    view! {
        <div class="page-header">
            <h1>"📋 Scan History"</h1>
            <p class="page-subtitle">"All security scans"</p>
        </div>

        <div class="card">
            <Suspense fallback=move || view! { <div class="loading">"Loading scans..."</div> }>
                {move || scans.get().map(|data| match data {
                    Ok(scan_list) => view! {
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"ID"</th>
                                    <th>"Type"</th>
                                    <th>"Target"</th>
                                    <th>"Status"</th>
                                    <th>"Findings"</th>
                                    <th>"Critical"</th>
                                    <th>"High"</th>
                                    <th>"Duration"</th>
                                    <th>"Date"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || scan_list.clone()
                                    key=|s| s.id
                                    children=|scan| {
                                        let dur = scan.duration_seconds.map(|d| format!("{}m {}s", d / 60, d % 60)).unwrap_or("-".into());
                                        view! {
                                            <tr class="clickable-row" on:click=move |_| {
                                                let _ = leptos_router::use_navigate()(&format!("/scans/{}", scan.id), Default::default());
                                            }>
                                                <td>{scan.id}</td>
                                                <td><span class="badge badge-type">{scan.scan_type.clone()}</span></td>
                                                <td class="target-cell">{scan.target.clone()}</td>
                                                <td><span class=format!("badge badge-status-{}", scan.status)>{scan.status.clone()}</span></td>
                                                <td>{scan.total_findings}</td>
                                                <td class="text-critical">{scan.critical_count}</td>
                                                <td class="text-high">{scan.high_count}</td>
                                                <td>{dur}</td>
                                                <td>{scan.started_at.clone()}</td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    }.into_view(),
                    Err(_) => view! { <div class="loading">"Loading..."</div> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}

async fn fetch_scans() -> Result<Vec<ScanJob>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/scans")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ScanJob>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

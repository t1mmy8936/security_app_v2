use leptos::*;
use crate::models::*;

#[component]
pub fn ScansPage() -> impl IntoView {
    let (refresh_counter, set_refresh_counter) = create_signal(0u32);

    let scans = create_resource(move || refresh_counter.get(), |_| async { fetch_scans().await });

    view! {
        <div class="page-header">
            <h1>"📋 Scan History"</h1>
            <p class="page-subtitle">"All security scans"</p>
        </div>

        <div class="card">
            <Suspense fallback=move || view! { <div class="loading">"Loading scans..."</div> }>
                {move || scans.get().map(|data| match data {
                    Ok(scan_list) => {
                        let set_refresh = set_refresh_counter;
                        view! {
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
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                <For
                                    each=move || scan_list.clone()
                                    key=|s| s.id
                                    children=move |scan| {
                                        let dur = scan.duration_seconds.map(|d| format!("{}m {}s", d / 60, d % 60)).unwrap_or("-".into());
                                        let scan_id = scan.id;
                                        let status = scan.status.clone();
                                        let set_refresh = set_refresh;
                                        view! {
                                            <tr class="clickable-row">
                                                <td on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{scan.id}</td>
                                                <td on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }><span class="badge badge-type">{scan.scan_type.clone()}</span></td>
                                                <td class="target-cell" on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{scan.target.clone()}</td>
                                                <td on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }><span class=format!("badge badge-status-{}", scan.status)>{scan.status.clone()}</span></td>
                                                <td on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{scan.total_findings}</td>
                                                <td class="text-critical" on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{scan.critical_count}</td>
                                                <td class="text-high" on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{scan.high_count}</td>
                                                <td on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{dur}</td>
                                                <td on:click=move |_| {
                                                    leptos_router::use_navigate()(&format!("/scans/{}", scan_id), Default::default());
                                                }>{scan.started_at.clone()}</td>
                                                <td class="scan-actions-cell">
                                                    {(status == "running").then(|| {
                                                        let set_refresh = set_refresh;
                                                        view! {
                                                            <button class="scan-action-btn scan-stop-btn" title="Stop scan"
                                                                on:click=move |e| {
                                                                    e.stop_propagation();
                                                                    #[cfg(feature = "hydrate")]
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                        let _ = post_scan_action(scan_id, "stop").await;
                                                                        set_refresh.update(|v| *v += 1);
                                                                    });
                                                                }>"⏹ Stop"</button>
                                                        }
                                                    })}
                                                    {(status == "stopped").then(|| {
                                                        let set_refresh = set_refresh;
                                                        view! {
                                                            <button class="scan-action-btn scan-resume-btn" title="Resume scan"
                                                                on:click=move |e| {
                                                                    e.stop_propagation();
                                                                    #[cfg(feature = "hydrate")]
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                        let _ = post_scan_action(scan_id, "resume").await;
                                                                        set_refresh.update(|v| *v += 1);
                                                                    });
                                                                }>"▶ Resume"</button>
                                                        }
                                                    })}
                                                    <button class="scan-action-btn scan-cancel-btn" title="Delete scan"
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                            #[cfg(feature = "hydrate")]
                                                            wasm_bindgen_futures::spawn_local(async move {
                                                                let confirmed = web_sys::window()
                                                                    .and_then(|w| w.confirm_with_message("Delete this scan and all its data?").ok())
                                                                    .unwrap_or(false);
                                                                if confirmed {
                                                                    let _ = post_scan_action(scan_id, "cancel").await;
                                                                    set_refresh.update(|v| *v += 1);
                                                                }
                                                            });
                                                        }>"✕ Delete"</button>
                                                </td>
                                            </tr>
                                        }
                                    }
                                />
                            </tbody>
                        </table>
                    }.into_view()},
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

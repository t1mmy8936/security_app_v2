use leptos::*;
use crate::models::*;

#[component]
pub fn ReportsPage() -> impl IntoView {
    let reports = create_resource(|| (), |_| async { fetch_reports().await });

    view! {
        <div class="page-header">
            <h1>"📄 Reports"</h1>
            <p class="page-subtitle">"Generated scan reports"</p>
        </div>

        <div class="card">
            <Suspense fallback=move || view! { <div class="loading">"Loading reports..."</div> }>
                {move || reports.get().map(|data| match data {
                    Ok(report_list) => {
                        if report_list.is_empty() {
                            view! { <p class="empty-state">"No reports generated yet. Run a scan and generate a report from the results page."</p> }.into_view()
                        } else {
                            view! {
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>"ID"</th>
                                            <th>"Scan ID"</th>
                                            <th>"Format"</th>
                                            <th>"Created"</th>
                                            <th>"Emailed"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        <For
                                            each=move || report_list.clone()
                                            key=|r| r.id
                                            children=|report| view! {
                                                <tr>
                                                    <td>{report.id}</td>
                                                    <td>
                                                        <a href=format!("/scans/{}", report.scan_job_id)>
                                                            {format!("Scan #{}", report.scan_job_id)}
                                                        </a>
                                                    </td>
                                                    <td><span class="badge">{report.format.to_uppercase()}</span></td>
                                                    <td>{report.created_at.clone()}</td>
                                                    <td>{if report.emailed { "✅" } else { "—" }}</td>
                                                </tr>
                                            }
                                        />
                                    </tbody>
                                </table>
                            }.into_view()
                        }
                    },
                    Err(_) => view! { <div class="loading">"Loading..."</div> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}

async fn fetch_reports() -> Result<Vec<Report>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/reports")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<Report>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

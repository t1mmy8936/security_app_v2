use leptos::*;
use crate::models::*;

#[component]
pub fn ToolsPage() -> impl IntoView {
    let tools = create_resource(|| (), |_| async { fetch_tools().await });

    view! {
        <div class="page-header">
            <h1>"🛠️ Security Tools"</h1>
            <p class="page-subtitle">"Available scanning tools"</p>
        </div>

        <Suspense fallback=move || view! { <div class="loading">"Loading tools..."</div> }>
            {move || tools.get().map(|data| match data {
                Ok(tool_list) => view! {
                    <div class="tools-grid">
                        <For
                            each=move || tool_list.clone()
                            key=|t| t.name.clone()
                            children=|tool| {
                                let status_class = if tool.available { "tool-available" } else { "tool-unavailable" };
                                view! {
                                    <div class=format!("card tool-card {}", status_class)>
                                        <div class="tool-header">
                                            <h3>{tool.display_name.clone()}</h3>
                                            <span class=format!("badge badge-category")>{tool.category.clone()}</span>
                                        </div>
                                        <p class="tool-desc">{tool.description.clone()}</p>
                                        <div class="tool-status">
                                            {if tool.available {
                                                view! { <span class="status-dot green"></span> " Available" }.into_view()
                                            } else {
                                                view! { <span class="status-dot red"></span> " Unavailable" }.into_view()
                                            }}
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                }.into_view(),
                Err(_) => view! { <div class="loading">"Loading..."</div> }.into_view(),
            })}
        </Suspense>
    }
}

async fn fetch_tools() -> Result<Vec<ToolInfo>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/tools")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ToolInfo>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

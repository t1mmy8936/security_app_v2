use leptos::*;
use crate::models::*;

#[component]
pub fn ScanPage() -> impl IntoView {
    let (scan_type, set_scan_type) = create_signal("full".to_string());
    let (target, set_target) = create_signal(String::new());
    let (target_source, set_target_source) = create_signal("url".to_string());
    let (selected_tools, set_selected_tools) = create_signal(Vec::<String>::new());
    let (is_scanning, set_is_scanning) = create_signal(false);
    let (browse_path, set_browse_path) = create_signal(None::<String>);
    let (folder_search, set_folder_search) = create_signal(String::new());

    let drives = create_resource(|| (), |_| async { fetch_drives().await });
    let presets = create_resource(|| (), |_| async { fetch_presets().await });
    let tools = create_resource(|| (), |_| async { fetch_tools().await });

    let folders = create_resource(
        move || browse_path.get(),
        |path| async move { fetch_folders(path).await },
    );

    let start_scan = create_action(move |_: &()| {
        let st = scan_type.get();
        let tgt = target.get();
        let ts = target_source.get();
        let tls = selected_tools.get();
        async move {
            set_is_scanning.set(true);
            let req = StartScanRequest {
                scan_type: st,
                target: tgt,
                target_source: ts,
                tools: if tls.is_empty() { None } else { Some(tls) },
            };
            match do_start_scan(req).await {
                Ok(scan_id) => {
                    let navigate = leptos_router::use_navigate();
                    navigate(&format!("/scans/{}", scan_id), Default::default());
                }
                Err(e) => {
                    #[cfg(feature = "hydrate")]
                    web_sys::console::error_1(&format!("Scan failed: {}", e).into());
                    #[cfg(not(feature = "hydrate"))]
                    { let _ = e; }
                    set_is_scanning.set(false);
                }
            }
        }
    });

    view! {
        <div class="page-header">
            <h1>"🔍 New Scan"</h1>
            <p class="page-subtitle">"Configure and launch a security scan"</p>
        </div>

        <div class="card">
            <h3>"Scan Configuration"</h3>

            <div class="form-group">
                <label>"Target Source"</label>
                <div class="radio-group">
                    <label class="radio-label">
                        <input type="radio" name="source" value="url"
                            checked=move || target_source.get() == "url"
                            on:change=move |_| set_target_source.set("url".into())/>
                        " URL"
                    </label>
                    <label class="radio-label">
                        <input type="radio" name="source" value="local"
                            checked=move || target_source.get() == "local"
                            on:change=move |_| set_target_source.set("local".into())/>
                        " Local Folder"
                    </label>
                    <label class="radio-label">
                        <input type="radio" name="source" value="azure"
                            checked=move || target_source.get() == "azure"
                            on:change=move |_| set_target_source.set("azure".into())/>
                        " Azure DevOps"
                    </label>
                </div>
            </div>

            <div class="form-group">
                <label>"Target"</label>
                <input type="text" class="form-control"
                    placeholder=move || {
                        if target_source.get() == "url" { "https://example.com" }
                        else { "/projects/my-app" }
                    }
                    prop:value=move || target.get()
                    on:input=move |ev| set_target.set(event_target_value(&ev))/>

                {move || {
                    if target_source.get() == "local" {
                        view! {
                            <div class="folder-browser">
                                <div class="folder-browser-header">
                                    <button class="btn btn-sm"
                                        on:click=move |_| {
                                            set_folder_search.set(String::new());
                                            set_browse_path.set(Some("/projects".into()));
                                        }>
                                        "📂 Browse Projects"
                                    </button>
                                    <Suspense fallback=move || view! { <span>"..."</span> }>
                                        {move || drives.get().map(|data| match data {
                                            Ok(drive_list) => {
                                                if drive_list.is_empty() {
                                                    view! {}.into_view()
                                                } else {
                                                    view! {
                                                        <select class="form-control drive-select"
                                                            on:change=move |ev| {
                                                                let val = event_target_value(&ev);
                                                                if !val.is_empty() {
                                                                    set_folder_search.set(String::new());
                                                                    set_browse_path.set(Some(val));
                                                                }
                                                            }>
                                                            <option value="">"💻 Browse Drive..."</option>
                                                            {drive_list.into_iter().map(|d| {
                                                                let path = d.path.clone();
                                                                view! {
                                                                    <option value={path}>{format!("{}:", d.letter)}</option>
                                                                }
                                                            }).collect_view()}
                                                        </select>
                                                    }.into_view()
                                                }
                                            }
                                            Err(_) => view! {}.into_view(),
                                        })}
                                    </Suspense>
                                </div>
                                {move || browse_path.get().map(|_| view! {
                                    <div class="folder-browser-panel">
                                        <div class="folder-browser-search">
                                            <input type="text" class="form-control"
                                                placeholder="Search files and folders..."
                                                prop:value=move || folder_search.get()
                                                on:input=move |ev| set_folder_search.set(event_target_value(&ev))/>
                                        </div>
                                        <Suspense fallback=move || view! { <div class="folder-loading">"Loading..."</div> }>
                                            {move || folders.get().map(|data| match data {
                                                Ok(resp) => {
                                                    let current = resp.current_path.clone();
                                                    let parent = resp.parent.clone();
                                                    let search = folder_search.get().to_lowercase();
                                                    let filtered: Vec<_> = resp.entries.iter()
                                                        .filter(|e| search.is_empty() || e.name.to_lowercase().contains(&search))
                                                        .cloned()
                                                        .collect();
                                                    view! {
                                                        <div class="folder-browser-nav">
                                                            <span class="folder-current-path">{current.clone()}</span>
                                                            {parent.map(|p| {
                                                                view! {
                                                                    <button class="btn btn-xs"
                                                                        on:click=move |_| {
                                                                            set_folder_search.set(String::new());
                                                                            set_browse_path.set(Some(p.clone()));
                                                                        }>
                                                                        "⬆ Up"
                                                                    </button>
                                                                }
                                                            })}
                                                            <button class="btn btn-xs btn-primary btn-select-current"
                                                                on:click={
                                                                    let cur = current.clone();
                                                                    move |_| {
                                                                        set_target.set(cur.clone());
                                                                        set_browse_path.set(None);
                                                                    }
                                                                }>
                                                                "✓ Select This Folder"
                                                            </button>
                                                            <button class="btn btn-xs folder-close-btn"
                                                                on:click=move |_| set_browse_path.set(None)>
                                                                "✕"
                                                            </button>
                                                        </div>
                                                        <div class="folder-entries">
                                                            {if filtered.is_empty() {
                                                                view! { <div class="folder-empty">"No matching files or folders"</div> }.into_view()
                                                            } else {
                                                                filtered.into_iter().map(|entry| {
                                                                    let path = entry.path.clone();
                                                                    let path2 = entry.path.clone();
                                                                    let is_dir = entry.is_dir;
                                                                    view! {
                                                                        <div class="folder-entry"
                                                                            on:click=move |_| {
                                                                                if is_dir {
                                                                                    set_folder_search.set(String::new());
                                                                                    set_browse_path.set(Some(path.clone()));
                                                                                } else {
                                                                                    set_target.set(path.clone());
                                                                                    set_browse_path.set(None);
                                                                                }
                                                                            }>
                                                                            <span class="folder-entry-icon">
                                                                                {if is_dir { "📁" } else { "📄" }}
                                                                            </span>
                                                                            <span class="folder-entry-name">{entry.name.clone()}</span>
                                                                            {is_dir.then(|| view! {
                                                                                <button class="btn btn-xs btn-select-folder"
                                                                                    on:click=move |ev| {
                                                                                        ev.stop_propagation();
                                                                                        set_target.set(path2.clone());
                                                                                        set_browse_path.set(None);
                                                                                    }>
                                                                                    "Select"
                                                                                </button>
                                                                            })}
                                                                        </div>
                                                                    }
                                                                }).collect_view()
                                                            }}
                                                        </div>
                                                    }.into_view()
                                                }
                                                Err(_) => view! { <div class="folder-empty">"Failed to load folder"</div> }.into_view(),
                                            })}
                                        </Suspense>
                                    </div>
                                })}
                            </div>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }}
            </div>

            <div class="form-group">
                <label>"Scan Preset"</label>
                <Suspense fallback=move || view! { <p>"Loading presets..."</p> }>
                    {move || presets.get().map(|data| match data {
                        Ok(preset_list) => view! {
                            <div class="preset-grid">
                                <For
                                    each=move || preset_list.clone()
                                    key=|p| p.name.clone()
                                    children=move |preset| {
                                        let name = preset.name.clone();
                                        let name2 = preset.name.clone();
                                        view! {
                                            <div class=move || {
                                                if scan_type.get() == name { "preset-card active" } else { "preset-card" }
                                            }
                                            on:click=move |_| set_scan_type.set(name2.clone())>
                                                <h4>{preset.display_name.clone()}</h4>
                                                <p>{preset.description.clone()}</p>
                                                <div class="preset-tools">
                                                    {preset.tools.iter().map(|t| view! {
                                                        <span class="tool-tag">{t.clone()}</span>
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        }.into_view(),
                        Err(_) => view! { <p>"Loading..."</p> }.into_view(),
                    })}
                </Suspense>
            </div>

            <div class="form-group">
                <label>"Tools"</label>
                <Suspense fallback=move || view! { <p>"Loading tools..."</p> }>
                    {move || tools.get().map(|data| match data {
                        Ok(tool_list) => view! {
                            <div class="tools-checkbox-grid">
                                <For
                                    each=move || tool_list.clone()
                                    key=|t| t.name.clone()
                                    children=move |tool| {
                                        let name = tool.name.clone();
                                        let name2 = tool.name.clone();
                                        view! {
                                            <label class="checkbox-label">
                                                <input type="checkbox"
                                                    prop:checked=move || selected_tools.get().contains(&name)
                                                    on:change=move |ev| {
                                                        let checked = event_target_checked(&ev);
                                                        set_selected_tools.update(|v| {
                                                            if checked {
                                                                v.push(name2.clone());
                                                            } else {
                                                                v.retain(|x| x != &name2);
                                                            }
                                                        });
                                                    }/>
                                                " " {tool.display_name.clone()}
                                                <span class="tool-category">{format!(" ({})", tool.category)}</span>
                                            </label>
                                        }
                                    }
                                />
                            </div>
                        }.into_view(),
                        Err(_) => view! { <p>"Loading..."</p> }.into_view(),
                    })}
                </Suspense>
            </div>

            <button class="btn btn-primary btn-lg"
                disabled=move || is_scanning.get() || target.get().is_empty()
                on:click=move |_| start_scan.dispatch(())>
                {move || if is_scanning.get() { "⏳ Starting scan..." } else { "🚀 Execute Order 66" }}
            </button>
        </div>
    }
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        ev.target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
            .map(|e: web_sys::HtmlInputElement| e.checked())
            .unwrap_or(false)
    }
    #[cfg(not(feature = "hydrate"))]
    {
        let _ = ev;
        false
    }
}

async fn fetch_presets() -> Result<Vec<ScanPreset>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/presets")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<ScanPreset>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
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

async fn do_start_scan(req: StartScanRequest) -> Result<i64, String> {
    #[cfg(feature = "hydrate")]
    {
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/scans/start")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
        api.data
            .and_then(|d| d["scan_id"].as_i64())
            .ok_or("No scan_id returned".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_folders(path: Option<String>) -> Result<BrowseFoldersResponse, String> {
    #[cfg(feature = "hydrate")]
    {
        let req = BrowseFoldersRequest { path };
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/browse-folders")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        resp.json::<BrowseFoldersResponse>().await.map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_drives() -> Result<Vec<DriveInfo>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/drives")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<DriveInfo>> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

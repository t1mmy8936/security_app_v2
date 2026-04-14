use leptos::*;
use crate::models::*;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let (saved_msg, set_saved_msg) = create_signal(Option::<String>::None);
    let settings = create_resource(|| (), |_| async { fetch_settings().await });

    // Setting signals
    let (azure_org, set_azure_org) = create_signal(String::new());
    let (azure_pat, set_azure_pat) = create_signal(String::new());
    let (azure_project, set_azure_project) = create_signal(String::new());
    let (smtp_server, set_smtp_server) = create_signal(String::new());
    let (smtp_port, set_smtp_port) = create_signal(String::new());
    let (smtp_username, set_smtp_username) = create_signal(String::new());
    let (smtp_password, set_smtp_password) = create_signal(String::new());
    let (email_from, set_email_from) = create_signal(String::new());
    let (email_to, set_email_to) = create_signal(String::new());
    let (sonar_url, set_sonar_url) = create_signal(String::new());
    let (sonar_token, set_sonar_token) = create_signal(String::new());
    let (sonar_project, set_sonar_project) = create_signal(String::new());
    let (sonar_exclusions, set_sonar_exclusions) = create_signal(String::new());
    let (sonar_qp, set_sonar_qp) = create_signal(String::new());
    let (qp_list, set_qp_list) = create_signal(Vec::<QualityProfile>::new());
    let (qp_loading, set_qp_loading) = create_signal(false);
    let (qp_error, set_qp_error) = create_signal(Option::<String>::None);
    let (openvas_url, set_openvas_url) = create_signal(String::new());
    let (openvas_username, set_openvas_username) = create_signal(String::new());
    let (openvas_password, set_openvas_password) = create_signal(String::new());

    // Populate signals when settings load
    create_effect(move |_| {
        if let Some(Ok(s)) = settings.get() {
            set_azure_org.set(s.azure_devops_org);
            set_azure_pat.set(s.azure_devops_pat);
            set_azure_project.set(s.azure_devops_project);
            set_smtp_server.set(s.smtp_server);
            set_smtp_port.set(s.smtp_port);
            set_smtp_username.set(s.smtp_username);
            set_smtp_password.set(s.smtp_password);
            set_email_from.set(s.email_from);
            set_email_to.set(s.email_to);
            set_sonar_url.set(s.sonarqube_url);
            set_sonar_token.set(s.sonarqube_token);
            set_sonar_project.set(s.sonarqube_project_key);
            set_sonar_exclusions.set(s.sonarqube_exclusions);
            set_sonar_qp.set(s.sonarqube_quality_profile);
            set_openvas_url.set(s.openvas_url);
            set_openvas_username.set(s.openvas_username);
            set_openvas_password.set(s.openvas_password);
        }
    });

    let save_action = create_action(move |_: &()| {
        let pairs = vec![
            SettingPair { key: "azure_devops_org".into(), value: azure_org.get() },
            SettingPair { key: "azure_devops_pat".into(), value: azure_pat.get() },
            SettingPair { key: "azure_devops_project".into(), value: azure_project.get() },
            SettingPair { key: "smtp_server".into(), value: smtp_server.get() },
            SettingPair { key: "smtp_port".into(), value: smtp_port.get() },
            SettingPair { key: "smtp_username".into(), value: smtp_username.get() },
            SettingPair { key: "smtp_password".into(), value: smtp_password.get() },
            SettingPair { key: "email_from".into(), value: email_from.get() },
            SettingPair { key: "email_to".into(), value: email_to.get() },
            SettingPair { key: "sonarqube_url".into(), value: sonar_url.get() },
            SettingPair { key: "sonarqube_token".into(), value: sonar_token.get() },
            SettingPair { key: "sonarqube_project_key".into(), value: sonar_project.get() },
            SettingPair { key: "sonarqube_exclusions".into(), value: sonar_exclusions.get() },
            SettingPair { key: "sonarqube_quality_profile".into(), value: sonar_qp.get() },
            SettingPair { key: "openvas_url".into(), value: openvas_url.get() },
            SettingPair { key: "openvas_username".into(), value: openvas_username.get() },
            SettingPair { key: "openvas_password".into(), value: openvas_password.get() },
        ];
        async move {
            match do_save_settings(pairs).await {
                Ok(_) => set_saved_msg.set(Some("Settings saved!".into())),
                Err(e) => set_saved_msg.set(Some(format!("Error: {}", e))),
            }
        }
    });

    let test_email_action = create_action(move |_: &()| async move {
        match do_test_email().await {
            Ok(msg) => set_saved_msg.set(Some(msg)),
            Err(e) => set_saved_msg.set(Some(format!("Error: {}", e))),
        }
    });

    let test_sonar_action = create_action(move |_: &()| async move {
        match do_test_sonarqube().await {
            Ok(msg) => set_saved_msg.set(Some(msg)),
            Err(e) => set_saved_msg.set(Some(format!("Error: {}", e))),
        }
    });

    let test_openvas_action = create_action(move |_: &()| async move {
        match do_test_openvas().await {
            Ok(msg) => set_saved_msg.set(Some(msg)),
            Err(e) => set_saved_msg.set(Some(format!("Error: {}", e))),
        }
    });

    let load_profiles_action = create_action(move |_: &()| async move {
        set_qp_loading.set(true);
        set_qp_error.set(None);
        match fetch_quality_profiles().await {
            Ok(profiles) => set_qp_list.set(profiles),
            Err(e) => set_qp_error.set(Some(e)),
        }
        set_qp_loading.set(false);
    });

    view! {
        <div class="page-header">
            <h1>"⚙️ Settings"</h1>
            <p class="page-subtitle">"Configure integrations and services"</p>
        </div>

        {move || saved_msg.get().map(|msg| view! {
            <div class="alert alert-info">{msg}</div>
        })}

        <Suspense fallback=move || view! { <div class="loading">"Loading settings..."</div> }>

        // Azure DevOps
        <div class="card mb-3">
            <h3>"Azure DevOps"</h3>
            <div class="form-group">
                <label>"Organization"</label>
                <input type="text" class="form-control" prop:value=move || azure_org.get()
                    on:input=move |ev| set_azure_org.set(event_target_value(&ev))/>
            </div>
            <div class="form-group">
                <label>"Personal Access Token"</label>
                <input type="password" class="form-control" prop:value=move || azure_pat.get()
                    on:input=move |ev| set_azure_pat.set(event_target_value(&ev))/>
            </div>
            <div class="form-group">
                <label>"Default Project"</label>
                <input type="text" class="form-control" prop:value=move || azure_project.get()
                    on:input=move |ev| set_azure_project.set(event_target_value(&ev))/>
            </div>
        </div>

        // Email / SMTP
        <div class="card mb-3">
            <h3>"Email / SMTP"</h3>
            <div class="form-row">
                <div class="form-group">
                    <label>"SMTP Server"</label>
                    <input type="text" class="form-control" prop:value=move || smtp_server.get()
                        on:input=move |ev| set_smtp_server.set(event_target_value(&ev))/>
                </div>
                <div class="form-group">
                    <label>"Port"</label>
                    <input type="text" class="form-control" prop:value=move || smtp_port.get()
                        on:input=move |ev| set_smtp_port.set(event_target_value(&ev))/>
                </div>
            </div>
            <div class="form-row">
                <div class="form-group">
                    <label>"Username"</label>
                    <input type="text" class="form-control" prop:value=move || smtp_username.get()
                        on:input=move |ev| set_smtp_username.set(event_target_value(&ev))/>
                </div>
                <div class="form-group">
                    <label>"Password"</label>
                    <input type="password" class="form-control" prop:value=move || smtp_password.get()
                        on:input=move |ev| set_smtp_password.set(event_target_value(&ev))/>
                </div>
            </div>
            <div class="form-row">
                <div class="form-group">
                    <label>"From"</label>
                    <input type="email" class="form-control" prop:value=move || email_from.get()
                        on:input=move |ev| set_email_from.set(event_target_value(&ev))/>
                </div>
                <div class="form-group">
                    <label>"To"</label>
                    <input type="email" class="form-control" prop:value=move || email_to.get()
                        on:input=move |ev| set_email_to.set(event_target_value(&ev))/>
                </div>
            </div>
            <button class="btn btn-secondary" on:click=move |_| test_email_action.dispatch(())>"Test Email"</button>
        </div>

        // SonarQube
        <div class="card mb-3">
            <h3>"SonarQube"</h3>
            <div class="form-group">
                <label>"Server URL"</label>
                <input type="text" class="form-control" prop:value=move || sonar_url.get()
                    on:input=move |ev| set_sonar_url.set(event_target_value(&ev))
                    placeholder="http://sonarqube:9000"/>
            </div>
            <div class="form-group">
                <label>"Token"</label>
                <input type="password" class="form-control" prop:value=move || sonar_token.get()
                    on:input=move |ev| set_sonar_token.set(event_target_value(&ev))/>
            </div>
            <div class="form-group">
                <label>"Project Key"</label>
                <input type="text" class="form-control" prop:value=move || sonar_project.get()
                    on:input=move |ev| set_sonar_project.set(event_target_value(&ev))
                    placeholder="watchtower-scan"/>
            </div>
            <div class="form-group">
                <label>"Exclusions"</label>
                <input type="text" class="form-control" prop:value=move || sonar_exclusions.get()
                    on:input=move |ev| set_sonar_exclusions.set(event_target_value(&ev))
                    placeholder="**/node_modules/**,**/venv/**"/>
            </div>
            <button class="btn btn-secondary" on:click=move |_| test_sonar_action.dispatch(())>"Test Connection"</button>

            // Quality Profiles section
            <div class="qp-section">
                <h4>"Quality Profiles"</h4>
                <p class="qp-desc">"Select a quality profile to use for SonarQube scans. Leave empty to use the server default."</p>

                <div class="qp-controls">
                    <button class="btn btn-secondary" on:click=move |_| load_profiles_action.dispatch(())
                        disabled=move || qp_loading.get()>
                        {move || if qp_loading.get() { "Loading..." } else { "🔄 Load Profiles" }}
                    </button>
                    <div class="form-group qp-current">
                        <label>"Selected Profile"</label>
                        <input type="text" class="form-control" prop:value=move || sonar_qp.get()
                            on:input=move |ev| set_sonar_qp.set(event_target_value(&ev))
                            placeholder="(server default)"/>
                    </div>
                </div>

                {move || qp_error.get().map(|e| view! {
                    <div class="alert alert-error">"⚠ " {e}</div>
                })}

                {move || {
                    let profiles = qp_list.get();
                    if profiles.is_empty() {
                        return view! { <div></div> }.into_view();
                    }

                    // Group by language
                    let mut langs: Vec<String> = profiles.iter().map(|p| p.language_name.clone()).collect();
                    langs.sort();
                    langs.dedup();

                    view! {
                        <div class="qp-grid">
                            {langs.into_iter().map(|lang| {
                                let lang_profiles: Vec<_> = profiles.iter()
                                    .filter(|p| p.language_name == lang)
                                    .cloned().collect();
                                let lang_name = lang.clone();
                                view! {
                                    <div class="qp-lang-group">
                                        <h5 class="qp-lang-title">{lang_name}</h5>
                                        {lang_profiles.into_iter().map(|p| {
                                            let pname = p.name.clone();
                                            let pname2 = p.name.clone();
                                            let is_selected = move || sonar_qp.get() == pname;
                                            view! {
                                                <div class=move || format!("qp-card {}", if is_selected() { "qp-selected" } else { "" })
                                                    on:click=move |_| set_sonar_qp.set(pname2.clone())>
                                                    <div class="qp-card-header">
                                                        <span class="qp-card-name">{p.name.clone()}</span>
                                                        {p.is_default.then(|| view! { <span class="qp-badge qp-badge-default">"Default"</span> })}
                                                        {p.is_built_in.then(|| view! { <span class="qp-badge qp-badge-builtin">"Built-in"</span> })}
                                                    </div>
                                                    <div class="qp-card-meta">
                                                        <span class="qp-rules">"📏 " {p.active_rule_count} " rules"</span>
                                                        <span class="qp-lang-badge">{p.language.clone()}</span>
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }
                            }).collect_view()}

                            <div class="qp-clear-row">
                                <button class="btn btn-secondary btn-sm" on:click=move |_| set_sonar_qp.set(String::new())>
                                    "✕ Clear Selection (use server default)"
                                </button>
                            </div>
                        </div>
                    }.into_view()
                }}
            </div>
        </div>

        <button class="btn btn-primary btn-lg" on:click=move |_| save_action.dispatch(())>"💾 Save All Settings"</button>

        // OpenVAS
        <div class="card mb-3">
            <h3>"OpenVAS / Greenbone"</h3>
            <p class="form-hint">"Network vulnerability scanner. Requires the Greenbone Community Edition stack (see docker-compose.override.yml). First startup takes 10-20 minutes for feed downloads."</p>
            <div class="form-group">
                <label>"gsad URL"</label>
                <input type="text" class="form-control" prop:value=move || openvas_url.get()
                    on:input=move |ev| set_openvas_url.set(event_target_value(&ev))
                    placeholder="http://gsad:80"/>
            </div>
            <div class="form-group">
                <label>"Username"</label>
                <input type="text" class="form-control" prop:value=move || openvas_username.get()
                    on:input=move |ev| set_openvas_username.set(event_target_value(&ev))
                    placeholder="admin"/>
            </div>
            <div class="form-group">
                <label>"Password"</label>
                <input type="password" class="form-control" prop:value=move || openvas_password.get()
                    on:input=move |ev| set_openvas_password.set(event_target_value(&ev))/>
            </div>
            <button class="btn btn-secondary" on:click=move |_| test_openvas_action.dispatch(())>"Test Connection"</button>
        </div>

        </Suspense>
    }
}

async fn fetch_settings() -> Result<AllSettings, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/settings")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<AllSettings> = resp.json().await.map_err(|e| e.to_string())?;
        api.data.ok_or("No data".into())
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_save_settings(pairs: Vec<SettingPair>) -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let req = SaveSettingsRequest { settings: pairs };
        let body = serde_json::to_string(&req).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post("/api/settings")
            .header("Content-Type", "application/json")
            .body(&body).map_err(|e| e.to_string())?
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(api.message.unwrap_or("Saved".into()))
    }
    #[cfg(not(feature = "hydrate"))]
    { let _ = pairs; Err("SSR".into()) }
}

async fn do_test_email() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/settings/test-email")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("OK".into())) }
        else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_test_sonarqube() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/settings/test-sonarqube")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("OK".into())) }
        else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn do_test_openvas() -> Result<String, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::post("/api/settings/test-openvas")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<()> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success { Ok(api.message.unwrap_or("OK".into())) }
        else { Err(api.message.unwrap_or("Failed".into())) }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

async fn fetch_quality_profiles() -> Result<Vec<QualityProfile>, String> {
    #[cfg(feature = "hydrate")]
    {
        let resp = gloo_net::http::Request::get("/api/sonarqube/profiles")
            .send().await.map_err(|e| e.to_string())?;
        let api: ApiResponse<Vec<QualityProfile>> = resp.json().await.map_err(|e| e.to_string())?;
        if api.success {
            api.data.ok_or("No data".into())
        } else {
            Err(api.message.unwrap_or("Failed".into()))
        }
    }
    #[cfg(not(feature = "hydrate"))]
    { Err("SSR".into()) }
}

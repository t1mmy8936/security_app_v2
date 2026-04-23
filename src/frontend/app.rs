use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use super::pages::*;
use super::components::sidebar::Sidebar;
use super::components::scan_status_bar::ScanStatusBar;

/// Tools hidden from normal view — revealed when Order 66 mode is active.
pub const RESTRICTED_TOOLS: &[&str] = &["sqlmap", "openvas", "bandit", "nmap"];

/// Scan presets hidden from normal view — revealed when Order 66 mode is active.
pub const RESTRICTED_PRESETS: &[&str] = &["web", "network", "full"];

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // ── Order 66 mode: driven by the existing order66.js easter egg ──
    // Seed from localStorage so the state survives a page refresh.
    let initial = {
        #[cfg(feature = "hydrate")]
        {
            web_sys::window()
                .and_then(|w| w.local_storage().ok().flatten())
                .and_then(|s| s.get_item("order66").ok().flatten())
                .map(|v| v == "active")
                .unwrap_or(false)
        }
        #[cfg(not(feature = "hydrate"))]
        { false }
    };

    let (order66_mode, _set_order66_mode) = create_signal(initial);
    provide_context(order66_mode);

    // Listen for the custom event dispatched by order66.js on each toggle.
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let handler = Closure::<dyn FnMut(web_sys::CustomEvent)>::new(move |ev: web_sys::CustomEvent| {
            let active = ev.detail()
                .as_bool()
                .unwrap_or_else(|| {
                    js_sys::Reflect::get(&ev.detail(), &wasm_bindgen::JsValue::from_str("active"))
                        .ok()
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                });
            _set_order66_mode.set(active);
        });

        if let Some(win) = web_sys::window() {
            let _ = win.add_event_listener_with_callback(
                "order66toggle",
                handler.as_ref().unchecked_ref(),
            );
        }
        handler.forget();
    }

    view! {
        <Stylesheet id="leptos" href="/pkg/watchtower.css"/>
        <Title text="Watchtower"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        <Script src="/order66.js"/>

        <Router>
            <div class="app-wrapper">
                <Sidebar/>
                <main class="main-content">
                    <Routes>
                        <Route path="/" view=DashboardPage/>
                        <Route path="/scan" view=ScanPage/>
                        <Route path="/scans" view=ScansPage/>
                        <Route path="/scans/:id" view=ResultsPage/>
                        <Route path="/tools" view=ToolsPage/>
                        <Route path="/settings" view=SettingsPage/>
                        <Route path="/reports" view=ReportsPage/>
                    </Routes>
                </main>
                <ScanStatusBar/>
            </div>
        </Router>
    }
}

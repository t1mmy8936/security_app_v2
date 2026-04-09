use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use super::pages::*;
use super::components::sidebar::Sidebar;
use super::components::scan_status_bar::ScanStatusBar;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

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

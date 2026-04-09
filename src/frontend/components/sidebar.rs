use leptos::*;
use leptos_router::*;

#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <nav class="sidebar">
            <div class="sidebar-header">
                <h2 class="sidebar-title"><span class="sidebar-icon">"⚔️"</span>" Watchtower"</h2>
                <span class="sidebar-version">"v2 • Rust"</span>
            </div>
            <ul class="sidebar-nav">
                <li><A href="/" class="nav-link" exact=true>
                    <span class="nav-icon">"📊"</span>" Dashboard"
                </A></li>
                <li><A href="/scan" class="nav-link">
                    <span class="nav-icon">"🔍"</span>" New Scan"
                </A></li>
                <li><A href="/scans" class="nav-link">
                    <span class="nav-icon">"📋"</span>" Scan History"
                </A></li>
                <li><A href="/tools" class="nav-link">
                    <span class="nav-icon">"🛠️"</span>" Tools"
                </A></li>
                <li><A href="/reports" class="nav-link">
                    <span class="nav-icon">"📄"</span>" Reports"
                </A></li>
                <li><A href="/settings" class="nav-link">
                    <span class="nav-icon">"⚙️"</span>" Settings"
                </A></li>
            </ul>
            <div class="sidebar-footer">
                <p class="sidebar-quote">"\"I find your lack of security disturbing.\""</p>
            </div>
        </nav>
    }
}

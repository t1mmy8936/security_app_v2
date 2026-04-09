use leptos::*;

#[component]
pub fn SeverityBadge(#[prop(into)] severity: String) -> impl IntoView {
    let class = format!("severity-badge severity-{}", severity.to_lowercase());
    view! {
        <span class=class>{severity.to_uppercase()}</span>
    }
}

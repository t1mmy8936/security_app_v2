use leptos::*;

#[component]
pub fn StatCard(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into, optional)] color: Option<String>,
) -> impl IntoView {
    let class = format!("stat-card {}", color.unwrap_or_default());
    view! {
        <div class=class>
            <div class="stat-value">{value}</div>
            <div class="stat-label">{label}</div>
        </div>
    }
}

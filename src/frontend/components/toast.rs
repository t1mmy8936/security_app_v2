use leptos::*;

#[component]
pub fn Toast(
    #[prop(into)] message: String,
    #[prop(into, optional)] variant: Option<String>,
    show: ReadSignal<bool>,
) -> impl IntoView {
    let class = move || {
        let v = variant.clone().unwrap_or_else(|| "success".into());
        let base = format!("toast toast-{}", v);
        if show.get() { format!("{} show", base) } else { base }
    };

    view! {
        <div class=class>
            {message}
        </div>
    }
}

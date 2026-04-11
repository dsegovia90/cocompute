use leptos::prelude::*;

#[component]
pub fn Icon(name: &'static str, #[prop(optional)] class: &'static str) -> impl IntoView {
    let class = if class.is_empty() { "w-5 h-5" } else { class };
    view! {
        <svg class={class}>
            <use href={format!("/static/icons.svg#{name}")}/>
        </svg>
    }
}

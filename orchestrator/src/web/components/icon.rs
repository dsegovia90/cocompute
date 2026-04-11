use leptos::prelude::*;
use crate::web::asset_hash;

#[component]
pub fn Icon(name: &'static str, #[prop(optional)] class: &'static str) -> impl IntoView {
    let class = if class.is_empty() { "w-5 h-5" } else { class };
    let icons_url = asset_hash::ICONS.url();
    view! {
        <svg class={class}>
            <use href={format!("{}#{name}", icons_url)}/>
        </svg>
    }
}

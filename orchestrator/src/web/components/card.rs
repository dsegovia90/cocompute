// SPDX-License-Identifier: AGPL-3.0-only

use leptos::prelude::*;

#[component]
pub fn Card(
    children: Children,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let base = "rounded-xl bg-[#16161E] border border-[#27272A]";
    let class = if class.is_empty() {
        base.to_string()
    } else {
        format!("{base} {class}")
    };
    view! {
        <div class={class}>
            {children()}
        </div>
    }
}

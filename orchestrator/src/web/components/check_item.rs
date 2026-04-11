use leptos::prelude::*;

use super::Icon;

#[component]
pub fn CheckItem(text: &'static str, #[prop(optional)] green: bool) -> impl IntoView {
    let icon_class = if green { "w-4 h-4 shrink-0 text-emerald-500" } else { "w-4 h-4 shrink-0 text-[#A1A1AA]" };
    view! {
        <li class="flex items-center gap-2.5">
            <Icon name="check" class={icon_class}/>
            <span class="text-[#A1A1AA] text-sm">{text}</span>
        </li>
    }
}

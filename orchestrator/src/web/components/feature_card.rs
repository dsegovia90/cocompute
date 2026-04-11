use leptos::prelude::*;

use super::Icon;

#[component]
pub fn FeatureCard(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
    #[prop(optional)] badge: Option<(&'static str, &'static str)>,
) -> impl IntoView {
    view! {
        <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-8 flex flex-col gap-4">
            <Icon name={icon} class="w-8 h-8 text-indigo-500"/>
            <h3 class="text-white font-semibold text-lg">{title}</h3>
            <p class="text-[#A1A1AA] text-sm leading-relaxed">{description}</p>
            {badge.map(|(text, color)| view! {
                <span class={format!("self-start rounded-md px-2.5 py-1 text-xs font-bold {color}")}>{text}</span>
            })}
        </div>
    }
}

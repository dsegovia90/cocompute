use leptos::prelude::*;

#[component]
pub fn TextInput(
    label: &'static str,
    r#type: &'static str,
    placeholder: &'static str,
    #[prop(optional)] name: &'static str,
    #[prop(optional)] required: bool,
    #[prop(optional)] hint: &'static str,
) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1.5">
            <span class="text-[#A1A1AA] text-[13px] font-medium">{label}</span>
            <input
                type={r#type}
                name={name}
                required={required}
                placeholder={placeholder}
                class="h-11 rounded-lg bg-[#111118] border border-[#27272A] px-3.5 text-white text-sm placeholder-[#52525B] outline-none focus:border-indigo-500 transition"
            />
            {(!hint.is_empty()).then(|| view! {
                <span class="text-[#3F3F46] text-[11px]">{hint}</span>
            })}
        </label>
    }
}

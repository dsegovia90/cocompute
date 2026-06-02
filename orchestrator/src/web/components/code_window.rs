use leptos::prelude::*;

/// A static code snippet shown as an editor/terminal window: traffic-light dots, a
/// filename in the title bar, then the code. Fills its parent's height.
#[component]
pub fn CodeWindow(title: &'static str, code: String) -> impl IntoView {
    view! {
        <div class="h-full flex flex-col rounded-xl bg-[#0D0D13] border border-[#27272A] overflow-hidden shadow-xl text-left">
            <div class="flex items-center gap-3 px-4 h-9 shrink-0 bg-[#1A1A22] border-b border-[#27272A]">
                <span class="flex gap-1.5">
                    <span class="w-3 h-3 rounded-full bg-[#FF5F57]"></span>
                    <span class="w-3 h-3 rounded-full bg-[#FEBC2E]"></span>
                    <span class="w-3 h-3 rounded-full bg-[#28C840]"></span>
                </span>
                <span class="rounded-md bg-[#0D0D13] border border-[#27272A] px-2.5 py-1 text-[#A1A1AA] text-xs font-medium">{title}</span>
            </div>
            <pre class="flex-1 m-0 px-4 py-4 font-mono text-xs leading-relaxed text-[#67e8f9] whitespace-pre-wrap break-all overflow-auto">{code}</pre>
        </div>
    }
}

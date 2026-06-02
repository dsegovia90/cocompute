use leptos::prelude::*;

use super::Icon;

/// A dark code box with a copy-to-clipboard button, wired by `static/code-copy.js`.
#[component]
pub fn CodeBlock(code: String) -> impl IntoView {
    view! {
        <div
            class="group relative bg-[#111118] border border-[#27272A] rounded-lg p-4 pr-12"
            data-codeblock
        >
            <button
                type="button"
                class="copy-btn absolute top-2.5 right-2.5 inline-flex items-center justify-center w-7 h-7 rounded-md bg-[#1E1E26] border border-[#27272A] text-[#71717A] hover:text-white hover:border-[#3F3F46] transition opacity-0 group-hover:opacity-100 focus:opacity-100"
                aria-label="Copy to clipboard"
            >
                <Icon name="copy" class="copy-icon w-3.5 h-3.5"/>
                <Icon name="check" class="check-icon w-3.5 h-3.5 hidden text-emerald-400"/>
            </button>
            <pre class="m-0 font-mono text-xs text-[#67e8f9] whitespace-pre-wrap break-all overflow-x-auto">{code}</pre>
        </div>
    }
}

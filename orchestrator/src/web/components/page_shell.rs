// SPDX-License-Identifier: AGPL-3.0-only

use leptos::prelude::*;

#[component]
pub fn PageShell(children: Children) -> impl IntoView {
    view! {
        <div class="min-h-screen bg-[#0A0A0F] font-['Inter',sans-serif]">
            {children()}
        </div>
    }
}

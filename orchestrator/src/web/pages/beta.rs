use axum::{extract::Query, response::Html};
use leptos::prelude::*;
use serde::Deserialize;
use crate::web::components::*;

#[derive(Deserialize)]
pub struct BetaQuery {
    pub error: Option<String>,
    pub success: Option<bool>,
}

#[component]
fn RoleOption(
    value: &'static str,
    title: &'static str,
    description: &'static str,
) -> impl IntoView {
    view! {
        <label class="flex items-center gap-3 rounded-lg bg-[#111118] border border-[#27272A] px-3.5 py-3 cursor-pointer has-[:checked]:border-indigo-500 has-[:checked]:border-2">
            <input type="radio" name="role" value={value} class="peer sr-only"/>
            // Unselected circle
            <span class="w-5 h-5 rounded-full bg-[#27272A] border border-[#3F3F46] flex items-center justify-center shrink-0 peer-checked:hidden"></span>
            // Selected circle
            <span class="w-5 h-5 rounded-full bg-indigo-500 hidden items-center justify-center shrink-0 peer-checked:flex">
                <span class="w-2 h-2 rounded-full bg-white"></span>
            </span>
            <div class="flex flex-col gap-0.5">
                <span class="text-white text-sm font-medium">{title}</span>
                <span class="text-[#52525B] text-xs">{description}</span>
            </div>
        </label>
    }
}

#[component]
fn BetaInvite(error: Option<String>) -> impl IntoView {
    view! {
        <Base title="cocompute — beta invite">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen py-12">
                    <form method="POST" action="/beta" class="w-[480px] rounded-xl bg-[#16161E] border border-[#27272A] px-9 py-10 flex flex-col gap-6">

                        // Header
                        <div class="flex flex-col gap-2">
                            <h1 class="text-white text-2xl font-bold">"cocompute"</h1>
                            <p class="text-[#A1A1AA] text-base font-medium">"Request Beta Access"</p>
                            <p class="text-[#52525B] text-[13px]">"We're launching invite-only. Tell us about yourself."</p>
                        </div>

                        {error.map(|msg| view! {
                            <div class="rounded-lg bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">{msg}</div>
                        })}

                        <TextInput label="Name" r#type="text" name="name" required=true placeholder="Your name"/>
                        <TextInput label="Email" r#type="email" name="email" required=true placeholder="you@example.com"/>

                        // Role selection
                        <fieldset class="flex flex-col gap-2">
                            <legend class="text-[#A1A1AA] text-[13px] font-medium mb-2">"I want to..."</legend>
                            <RoleOption value="consumer" title="Use compute (consumer)" description="Run AI models on shared GPUs"/>
                            <RoleOption value="host" title="Share my GPU (host)" description="Earn credits by sharing idle compute"/>
                            <RoleOption value="both" title="Both" description="Use and share compute"/>
                        </fieldset>

                        <TextInput
                            label="What GPU(s) do you have?"
                            r#type="text"
                            name="gpu"
                            placeholder="e.g. RTX 3090, RTX 4090, M2 Max..."
                            hint="Optional — helps us prioritize your invite"
                        />

                        <hr class="border-[#27272A]"/>

                        // Submit
                        <button
                            type="submit"
                            class="h-12 rounded-lg bg-indigo-500 text-white font-semibold text-[15px] flex items-center justify-center gap-2 hover:bg-indigo-600 transition cursor-pointer"
                        >
                            <Icon name="sparkles" class="w-[18px] h-[18px]"/>
                            "Request Beta Invite"
                        </button>

                        // Footer link
                        <div class="flex justify-center gap-1">
                            <span class="text-[#52525B] text-[13px]">"Already have an invite?"</span>
                            <a href="/login" class="text-indigo-500 text-[13px] font-medium hover:underline">"Sign in"</a>
                        </div>
                    </form>
                </div>
            </PageShell>
        </Base>
    }
}

#[component]
fn BetaConfirmation() -> impl IntoView {
    view! {
        <Base title="cocompute — you're on the list">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen">
                    <div class="w-[400px] rounded-xl bg-[#16161E] border border-[#27272A] px-10 pt-12 pb-10 flex flex-col gap-5 items-center text-center">
                        <h1 class="text-white text-2xl font-bold">"You're on the list!"</h1>
                        <p class="text-[#71717A] text-sm">"Thanks for signing up. We'll reach out when a spot opens up."</p>
                        <a href="/" class="text-indigo-500 text-sm font-medium hover:underline">"Back to home"</a>
                    </div>
                </div>
            </PageShell>
        </Base>
    }
}

pub async fn beta(Query(params): Query<BetaQuery>) -> Html<String> {
    if params.success.unwrap_or(false) {
        crate::web::render(BetaConfirmation())
    } else {
        crate::web::render(BetaInvite(BetaInviteProps { error: params.error }))
    }
}

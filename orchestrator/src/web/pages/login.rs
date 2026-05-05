// SPDX-License-Identifier: AGPL-3.0-only

use axum::{extract::Query, response::Html};
use leptos::prelude::*;
use serde::Deserialize;
use crate::web::components::*;

#[derive(Deserialize)]
pub struct LoginQuery {
    pub error: Option<String>,
}

#[component]
fn Login(error: Option<String>) -> impl IntoView {
    view! {
        <Base title="cocompute — sign in">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen">
                    <form method="POST" action="/login" class="w-[400px] rounded-xl bg-[#16161E] border border-[#27272A] px-10 pt-12 pb-10 flex flex-col gap-7">
                        // Logo + subtitle
                        <div class="flex flex-col gap-2">
                            <h1 class="text-white text-2xl font-bold">"cocompute"</h1>
                            <p class="text-[#71717A] text-sm">"Sign in to your beta account"</p>
                        </div>

                        {error.map(|msg| view! {
                            <div class="rounded-lg bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">{msg}</div>
                        })}

                        <TextInput label="Email" r#type="email" name="email" placeholder="you@example.com"/>
                        <TextInput label="Password" r#type="password" name="password" placeholder="••••••••"/>

                        // Submit
                        <button
                            type="submit"
                            class="h-11 rounded-lg bg-indigo-500 text-white text-sm font-semibold hover:bg-indigo-600 transition cursor-pointer"
                        >
                            "Sign In"
                        </button>

                        // Forgot + Beta links
                        <div class="flex flex-col items-center gap-2">
                            <a href="/forgot" class="text-[#71717A] text-[13px] hover:text-white transition">"Forgot password?"</a>
                            <div class="flex gap-1">
                                <span class="text-[#71717A] text-[13px]">"Want early access?"</span>
                                <a href="/beta" class="text-indigo-500 text-[13px] font-medium hover:underline">
                                    "Request a beta invite →"
                                </a>
                            </div>
                        </div>
                    </form>
                </div>
            </PageShell>
        </Base>
    }
}

pub async fn login(Query(params): Query<LoginQuery>) -> Html<String> {
    crate::web::render(Login(LoginProps { error: params.error }))
}

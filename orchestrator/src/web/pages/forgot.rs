// SPDX-License-Identifier: AGPL-3.0-only

use axum::{extract::Query, response::Html};
use leptos::prelude::*;
use serde::Deserialize;
use crate::web::components::*;

#[derive(Deserialize)]
pub struct ForgotQuery {
    pub sent: Option<bool>,
}

#[component]
fn ForgotPage(sent: bool) -> impl IntoView {
    view! {
        <Base title="cocompute — forgot password">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen">
                    <form method="POST" action="/forgot" class="w-[400px] rounded-xl bg-[#16161E] border border-[#27272A] px-10 pt-12 pb-10 flex flex-col gap-7">
                        <div class="flex flex-col gap-2">
                            <h1 class="text-white text-2xl font-bold">"cocompute"</h1>
                            <p class="text-[#71717A] text-sm">"Enter your email and we'll send a reset link"</p>
                        </div>

                        {sent.then(|| view! {
                            <div class="rounded-lg bg-emerald-500/10 border border-emerald-500/20 px-4 py-3 text-emerald-400 text-sm">
                                "If that email exists, we sent a reset link. Check your inbox."
                            </div>
                        })}

                        <TextInput label="Email" r#type="email" name="email" placeholder="you@example.com"/>

                        <button
                            type="submit"
                            class="h-11 rounded-lg bg-indigo-500 text-white text-sm font-semibold hover:bg-indigo-600 transition cursor-pointer"
                        >
                            "Send Reset Link"
                        </button>

                        <div class="flex justify-center">
                            <a href="/login" class="text-indigo-500 text-[13px] font-medium hover:underline">"Back to login"</a>
                        </div>
                    </form>
                </div>
            </PageShell>
        </Base>
    }
}

pub async fn forgot_page(Query(params): Query<ForgotQuery>) -> Html<String> {
    crate::web::render(ForgotPage(ForgotPageProps { sent: params.sent.unwrap_or(false) }))
}

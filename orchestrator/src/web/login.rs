use axum::response::Html;
use leptos::prelude::*;

use super::components::*;

#[component]
fn Login() -> impl IntoView {
    view! {
        <Base title="cocompute — sign in">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen">
                    <Card class="w-[400px] px-10 pt-12 pb-10 flex flex-col gap-7">
                        // Logo + subtitle
                        <div class="flex flex-col gap-2">
                            <h1 class="text-white text-2xl font-bold">"cocompute"</h1>
                            <p class="text-[#71717A] text-sm">"Sign in to your beta account"</p>
                        </div>

                        <TextInput label="Email" r#type="email" placeholder="you@example.com"/>
                        <TextInput label="Password" r#type="password" placeholder="••••••••"/>

                        // Submit
                        <button
                            type="submit"
                            class="h-11 rounded-lg bg-indigo-500 text-white text-sm font-semibold hover:bg-indigo-600 transition cursor-pointer"
                        >
                            "Sign In"
                        </button>

                        // Beta link
                        <div class="flex justify-center gap-1">
                            <span class="text-[#71717A] text-[13px]">"Want early access?"</span>
                            <a href="/beta" class="text-indigo-500 text-[13px] font-medium hover:underline">
                                "Request a beta invite →"
                            </a>
                        </div>
                    </Card>
                </div>
            </PageShell>
        </Base>
    }
}

pub async fn login() -> Html<String> {
    let html = Login().into_view().to_html();
    Html(format!("<!DOCTYPE html>{html}"))
}

use axum::{extract::State, response::Html};
use leptos::prelude::*;

use crate::web::components::*;

#[component]
fn Quickstart(base_url: String) -> impl IntoView {
    view! {
        <Base title="cocompute — quickstart">
            <PageShell>
                <div class="max-w-3xl mx-auto px-6 py-16">
                    <h1 class="text-white text-4xl font-bold mb-2">"Get started in 60 seconds"</h1>
                    <p class="text-[#71717A] text-base mb-12">"Share your GPU, use the pool. Three steps."</p>

                    // Step 1: Sign up
                    <div class="mb-10">
                        <div class="flex items-center gap-3 mb-3">
                            <span class="flex items-center justify-center w-7 h-7 rounded-full bg-indigo-500/20 text-indigo-400 text-sm font-bold">"1"</span>
                            <h2 class="text-white text-lg font-bold">"Create an account"</h2>
                        </div>
                        <p class="text-[#A1A1AA] text-sm mb-3 ml-10">"Sign up to get your dashboard where you manage hosts, pools, and API keys."</p>
                        <div class="ml-10">
                            <a href="/beta" class="inline-block rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                "Sign up"
                            </a>
                        </div>
                    </div>

                    // Step 2: Install host
                    <div class="mb-10">
                        <div class="flex items-center gap-3 mb-3">
                            <span class="flex items-center justify-center w-7 h-7 rounded-full bg-indigo-500/20 text-indigo-400 text-sm font-bold">"2"</span>
                            <h2 class="text-white text-lg font-bold">"Add your GPU"</h2>
                        </div>
                        <p class="text-[#A1A1AA] text-sm mb-3 ml-10">
                            "From your dashboard, click "<span class="text-white font-medium">"Add Host"</span>" to get a one-line install command. Run it on any machine with Ollama:"
                        </p>
                        <div class="ml-10 bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-xs text-[#67e8f9] break-all">
                            {format!("curl -sSf {base_url}/install.sh | bash -s -- --token YOUR_TOKEN")}
                        </div>
                        <p class="text-[#52525B] text-xs mt-2 ml-10">"Works on Linux (systemd) and macOS (launchd). Runs as a background service."</p>
                    </div>

                    // Step 3: Make an inference call
                    <div class="mb-10">
                        <div class="flex items-center gap-3 mb-3">
                            <span class="flex items-center justify-center w-7 h-7 rounded-full bg-indigo-500/20 text-indigo-400 text-sm font-bold">"3"</span>
                            <h2 class="text-white text-lg font-bold">"Run inference"</h2>
                        </div>
                        <p class="text-[#A1A1AA] text-sm mb-3 ml-10">
                            "Create a pool, add your host, generate an API key, then call the OpenAI-compatible endpoint:"
                        </p>
                        <div class="ml-10 bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-xs text-[#67e8f9] break-all whitespace-pre-wrap">
                            {format!(r#"curl {base_url}/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{{"model":"llama3.2","messages":[{{"role":"user","content":"hello"}}]}}'"#)}
                        </div>
                        <p class="text-[#52525B] text-xs mt-2 ml-10">"Works with any OpenAI-compatible client. Just change the base URL."</p>
                    </div>

                    // That's it
                    <div class="border-t border-[#27272A] pt-8 mt-12">
                        <h2 class="text-white text-lg font-bold mb-2">"That's it."</h2>
                        <p class="text-[#A1A1AA] text-sm mb-4">
                            "Your GPU is now accessible from anywhere. Add it to a pool to share with others and gain access to their GPUs in return."
                        </p>
                        <div class="flex gap-3">
                            <a href="/beta" class="rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                "Get started"
                            </a>
                            <a href="/" class="rounded-lg bg-[#27272A] px-5 py-2.5 text-[#A1A1AA] text-sm font-medium hover:text-white transition">
                                "Learn more"
                            </a>
                        </div>
                    </div>
                </div>
            </PageShell>
        </Base>
    }
}

pub async fn quickstart(State(state): State<crate::AppState>) -> Html<String> {
    crate::web::render(Quickstart(QuickstartProps {
        base_url: state.base_url.clone(),
    }))
}

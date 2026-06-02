use axum::{extract::State, response::Html};
use leptos::prelude::*;

use crate::web::components::*;

/// One numbered step in a 3-step persona flow.
#[component]
fn Step(num: &'static str, title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="mb-10">
            <div class="flex items-center gap-3 mb-3">
                <span class="flex items-center justify-center w-7 h-7 rounded-full bg-indigo-500/20 text-indigo-400 text-sm font-bold">{num}</span>
                <h3 class="text-white text-lg font-bold">{title}</h3>
            </div>
            <div class="ml-10 flex flex-col gap-3">
                {children()}
            </div>
        </div>
    }
}

/// One of the two persona chooser cards at the top of the page.
#[component]
fn ChooserCard(
    href: &'static str,
    eyebrow: &'static str,
    title: &'static str,
    description: &'static str,
    accent_color: &'static str,
) -> impl IntoView {
    let eyebrow_class = format!("text-{accent_color} text-xs font-bold uppercase tracking-wider mb-2");
    view! {
        <a
            href={href}
            class="group rounded-xl bg-[#16161E] border border-[#27272A] p-6 hover:border-[#3F3F46] hover:bg-[#1A1A24] transition flex flex-col"
        >
            <span class={eyebrow_class}>{eyebrow}</span>
            <h2 class="text-white text-xl font-bold mb-2">{title}</h2>
            <p class="text-[#A1A1AA] text-sm leading-relaxed flex-1">{description}</p>
            <div class="mt-4 text-indigo-400 text-sm font-medium">
                "Jump to steps →"
            </div>
        </a>
    }
}

#[component]
fn Quickstart(base_url: String) -> impl IntoView {
    let host_install_cmd = format!(
        r#"curl -sSf {base_url}/install.sh | COCOMPUTE_URL={base_url} bash -s -- --token YOUR_TOKEN"#
    );
    let consumer_curl = format!(
        r#"curl {base_url}/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{{"model":"gemma4","messages":[{{"role":"user","content":"hello"}}]}}'"#
    );
    let list_models_cmd = format!(
        r#"curl {base_url}/v1/models -H "Authorization: Bearer YOUR_API_KEY""#
    );

    view! {
        <Base title="cocompute · quickstart">
            <PageShell>
                <div class="max-w-3xl mx-auto px-6 py-16">
                    // Hero
                    <h1 class="text-white text-4xl font-bold mb-2">"Get started in 60 seconds"</h1>
                    <p class="text-[#71717A] text-base mb-10">"Pick how you want to use cocompute. You can do both."</p>

                    // Persona chooser
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mb-16">
                        <ChooserCard
                            href="#share-gpu"
                            eyebrow="Host"
                            title="Share your GPU"
                            description="You have a GPU sitting idle. Install the host binary, join the pool, and get access to other people's GPUs in return."
                            accent_color="emerald-400"
                        />
                        <ChooserCard
                            href="#use-pool"
                            eyebrow="Consumer"
                            title="Use the pool"
                            description="You want LLM inference but don't want to buy hardware. Sign up, get an API key, point your apps at our OpenAI-compatible endpoint."
                            accent_color="indigo-400"
                        />
                    </div>

                    // ── Path 1: Share your GPU ──
                    <section id="share-gpu" class="scroll-mt-8 mb-20">
                        <div class="flex items-baseline gap-3 mb-2">
                            <span class="text-emerald-400 text-xs font-bold uppercase tracking-wider">"Host"</span>
                        </div>
                        <h2 class="text-white text-2xl font-bold mb-1">"Share your GPU"</h2>
                        <p class="text-[#71717A] text-sm mb-8">"Three steps. About 5 minutes if you already have Ollama installed."</p>

                        <Step num="1" title="Sign up">
                            <p class="text-[#A1A1AA] text-sm">"Create a free account so you can manage your hosts and pools."</p>
                            <div>
                                <a href="/signup" class="inline-block rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                    "Sign up"
                                </a>
                            </div>
                        </Step>

                        <Step num="2" title="Install the host binary">
                            <p class="text-[#A1A1AA] text-sm">
                                "From your dashboard, click "<span class="text-white font-medium">"Add Host"</span>" to get a one-line install command. Run it on any machine that runs Ollama:"
                            </p>
                            <CodeBlock code={host_install_cmd}/>
                            <p class="text-[#52525B] text-xs">
                                "Works on Linux (systemd) and macOS (launchd). Runs as a background service. Anything Ollama supports works: NVIDIA, AMD, Apple Silicon, even CPU."
                            </p>
                        </Step>

                        <Step num="3" title="Add your host to a pool">
                            <p class="text-[#A1A1AA] text-sm">
                                "Back in the dashboard, create a pool (or pick the global pool) and add your host. As soon as your host registers, it shows up online and is ready to serve inference."
                            </p>
                            <p class="text-[#52525B] text-xs">
                                "Share with friends, your team, or the global pool. No tokens, no crypto."
                            </p>
                        </Step>
                    </section>

                    // ── Path 2: Use the pool ──
                    <section id="use-pool" class="scroll-mt-8 mb-12">
                        <div class="flex items-baseline gap-3 mb-2">
                            <span class="text-indigo-400 text-xs font-bold uppercase tracking-wider">"Consumer"</span>
                        </div>
                        <h2 class="text-white text-2xl font-bold mb-1">"Use the pool"</h2>
                        <p class="text-[#71717A] text-sm mb-8">"Three steps. About 2 minutes if you have a curl handy."</p>

                        <Step num="1" title="Sign up">
                            <p class="text-[#A1A1AA] text-sm">"Create a free account."</p>
                            <div>
                                <a href="/signup" class="inline-block rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                    "Sign up"
                                </a>
                            </div>
                        </Step>

                        <Step num="2" title="Create an API key">
                            <p class="text-[#A1A1AA] text-sm">
                                "From your dashboard, find the pool you want to use and click "<span class="text-white font-medium">"New API Key"</span>". Copy the key. It looks like a long random string. You'll only see it once."
                            </p>
                            <p class="text-[#52525B] text-xs">
                                "API keys are scoped to a pool. They only have access to the hosts in that pool."
                            </p>
                        </Step>

                        <Step num="3" title="Make your first call">
                            <p class="text-[#A1A1AA] text-sm">
                                "Drop your key in and call the OpenAI-compatible endpoint:"
                            </p>
                            <CodeBlock code={consumer_curl}/>
                            <p class="text-[#A1A1AA] text-sm mt-2">
                                "Or list which models the pool has available:"
                            </p>
                            <CodeBlock code={list_models_cmd}/>
                            <p class="text-[#52525B] text-xs">
                                "Works with any client that speaks the OpenAI API spec (the official OpenAI SDK, openwebui, llama.cpp clients). Just change the base URL."
                            </p>
                        </Step>
                    </section>

                    // ── Footer CTA ──
                    <div class="border-t border-[#27272A] pt-8 mt-4">
                        <h2 class="text-white text-lg font-bold mb-2">"Stuck?"</h2>
                        <p class="text-[#A1A1AA] text-sm mb-4">
                            "Open an issue on GitHub or read the source. cocompute is AGPL, every line is yours to inspect."
                        </p>
                        <div class="flex flex-wrap gap-3">
                            <a href="/signup" class="rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                "Sign up"
                            </a>
                            <a href="https://github.com/dsegovia90/cocompute" target="_blank" rel="noopener" class="rounded-lg bg-[#27272A] px-5 py-2.5 text-[#A1A1AA] text-sm font-medium hover:text-white transition flex items-center gap-2">
                                <Icon name="github" class="w-4 h-4"/>
                                "View source"
                            </a>
                            <a href="/" class="rounded-lg bg-[#27272A] px-5 py-2.5 text-[#A1A1AA] text-sm font-medium hover:text-white transition">
                                "Back to home"
                            </a>
                        </div>
                    </div>
                </div>
                <script src={crate::web::asset_hash::JS_COPY.url()} defer></script>
            </PageShell>
        </Base>
    }
}

pub async fn quickstart(State(state): State<crate::AppState>) -> Html<String> {
    crate::web::render(Quickstart(QuickstartProps {
        base_url: state.base_url.clone(),
    }))
}

use axum::{extract::State, response::Html};
use axum_extra::extract::cookie::SignedCookieJar;
use leptos::prelude::*;

use crate::web::components::*;

#[component]
fn Landing(logged_in: bool, total_compute: String) -> impl IntoView {
    view! {
        <Base title="cocompute">
            <PageShell>

                // ── Nav ──
                <nav class="flex items-center justify-between px-6 py-4">
                    <span class="text-white font-bold text-lg">"cocompute"</span>
                    <div class="flex items-center gap-5">
                        // Plain link to the out-of-band UptimeRobot status page.
                        // No "all systems operational" dot here on purpose: the
                        // page rendering at all means the orchestrator is up, so
                        // a green dot adds no information. The status page lives
                        // off-domain so users can check it WHEN cocompute.ai is down.
                        <a href="https://stats.uptimerobot.com/hdrVVZOlHE" target="_blank" rel="noopener" class="text-[#A1A1AA] text-sm font-medium hover:text-white transition">
                            "Status"
                        </a>
                        <a href="https://github.com/dsegovia90/cocompute" target="_blank" rel="noopener" class="text-[#A1A1AA] text-sm font-medium hover:text-white transition flex items-center gap-1.5">
                            <Icon name="github" class="w-4 h-4"/>
                            "GitHub"
                        </a>
                        {if logged_in {
                            view! {
                                <a href="/dashboard" class="rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                    "Dashboard"
                                </a>
                            }.into_any()
                        } else {
                            view! {
                                <a href="/login" class="text-[#A1A1AA] text-sm font-medium hover:text-white transition">"Log in"</a>
                                <a href="/beta" class="hidden md:inline-block rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                                    "Sign up"
                                </a>
                            }.into_any()
                        }}
                    </div>
                </nav>

                // ── Hero ──
                <section class="flex flex-col items-center px-6 pt-20 pb-16">
                    <div class="mb-5 inline-flex items-center gap-2 rounded-full bg-[#16161E] border border-[#27272A] px-3 py-1.5">
                        <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                        <span class="text-[#A1A1AA] text-xs font-medium">"Open source · AGPLv3 · Self-host or hosted · No crypto"</span>
                    </div>
                    <h1 class="text-white text-5xl font-bold text-center leading-tight max-w-3xl">
                        "Your GPU, your inference."<br/>
                        <span class="text-[#A1A1AA]">"Open infrastructure for the rest of us."</span>
                    </h1>
                    <p class="mt-5 text-[#A1A1AA] text-base text-center max-w-xl leading-relaxed">
                        "cocompute is open infrastructure for cooperative LLM inference on consumer hardware. Share your GPU (NVIDIA, AMD, Apple Silicon, anything Ollama runs on) over the internet. Use the pool through an OpenAI-compatible API. Self-host the whole stack, or use cocompute.ai."
                    </p>
                    <div class="mt-10 flex flex-wrap items-center justify-center gap-3">
                        <a href="/quickstart" class="rounded-lg bg-indigo-500 px-7 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
                            "Get started"
                        </a>
                        <a href="https://github.com/dsegovia90/cocompute" target="_blank" rel="noopener" class="rounded-lg bg-[#27272A] border border-[#3F3F46] px-7 py-3.5 text-[#A1A1AA] font-semibold hover:text-white hover:border-[#52525B] transition flex items-center gap-2">
                            <Icon name="github" class="w-[18px] h-[18px]"/>
                            "View on GitHub"
                        </a>
                    </div>

                    <div class="mt-8 inline-flex items-center gap-2 rounded-full bg-[#16161E] border border-[#27272A] px-4 py-2">
                        <span class="text-[#A1A1AA] text-xs font-medium">"Total time computed"</span>
                        <span class="text-white text-xs font-semibold tabular-nums">{total_compute}</span>
                    </div>

                    // ── Network animation ──
                    <div
                        id="network-sc"
                        class="mt-12 w-full max-w-4xl rounded-2xl overflow-hidden max-sm:aspect-auto max-sm:min-h-[720px]"
                        style="position:relative;aspect-ratio:2.1/1;min-height:460px"
                    >
                        <canvas
                            id="network-cv"
                            style="position:absolute;top:0;left:0;width:100%;height:100%"
                        />
                        <div
                            id="network-ui"
                            style="position:absolute;top:0;left:0;width:100%;height:100%;pointer-events:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif"
                        />
                    </div>
                    <script src={crate::web::asset_hash::JS_NETWORK.url()} defer></script>
                </section>

                // ── Two-sided value prop ──
                <section class="px-6 py-16 flex flex-col items-center">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-8 max-w-4xl w-full">
                        <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-8">
                            <h3 class="text-emerald-400 text-lg font-bold mb-2">"Have a GPU?"</h3>
                            <p class="text-[#A1A1AA] text-sm leading-relaxed">
                                "Share your idle hardware with the pool. Anything Ollama runs on works: NVIDIA, AMD, Apple Silicon, even CPU. One command to install, runs as a background service. In return, access every GPU in the network."
                            </p>
                        </div>
                        <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-8">
                            <h3 class="text-indigo-400 text-lg font-bold mb-2">"Need inference?"</h3>
                            <p class="text-[#A1A1AA] text-sm leading-relaxed">
                                "Point your apps at cocompute's OpenAI-compatible API. Access GPUs shared by others without buying hardware or paying cloud prices."
                            </p>
                        </div>
                    </div>
                    <a href="/quickstart" class="mt-8 text-indigo-400 text-sm font-medium hover:underline">
                        "Get started in 60 seconds →"
                    </a>
                </section>

                // ── How it works ──
                <section class="px-6 py-16 flex flex-col items-center">
                    <h2 class="text-white text-3xl font-bold text-center">"How it works"</h2>
                    <p class="mt-3 text-[#71717A] text-base text-center">
                        "Open source protocol, hosted as a service, or self-hosted."
                    </p>
                    <div class="mt-10 grid grid-cols-1 md:grid-cols-3 gap-5 w-full max-w-5xl">
                        <FeatureCard
                            icon="monitor"
                            title="Bring your hardware"
                            description="Install cocompute on any machine that runs Ollama. Your GPU joins the pool from your home network. We handle the NAT traversal and hole punching, so no port forwarding, no router config."
                            badge=("FREE", "bg-emerald-500/20 text-emerald-500")
                        />
                        <FeatureCard
                            icon="code"
                            title="OpenAI-compatible API"
                            description="Drop-in replacement for the OpenAI SDK. Same /v1/ endpoints, works with every existing tool and client."
                        />
                        <FeatureCard
                            icon="cpu"
                            title="Cooperative pools"
                            description="Share with friends, your team, or the public pool. Pool-credit accounting tracks reciprocity. No tokens, no crypto."
                        />
                    </div>
                </section>

                // ── Open Source ──
                <section class="px-6 py-16 flex flex-col items-center">
                    <div class="max-w-3xl w-full rounded-2xl bg-gradient-to-br from-[#16161E] to-[#0E0E15] border border-[#27272A] p-10">
                        <h2 class="text-white text-2xl font-bold">"Open source. AGPLv3."</h2>
                        <p class="mt-2 text-[#A1A1AA] text-sm leading-relaxed">
                            "cocompute is free software you can self-host, fork, and modify. The code that runs cocompute.ai is the same code in the public repo. cocompute.ai is the hosted version for people who don't want to operate their own orchestrator."
                        </p>
                        <div class="mt-5 flex flex-wrap gap-3">
                            <a href="https://github.com/dsegovia90/cocompute" target="_blank" rel="noopener" class="rounded-lg bg-[#27272A] border border-[#3F3F46] px-5 py-2.5 text-[#A1A1AA] text-sm font-semibold hover:text-white hover:border-[#52525B] transition flex items-center gap-2">
                                <Icon name="github" class="w-4 h-4"/>
                                "github.com/dsegovia90/cocompute"
                            </a>
                            <a href="/quickstart" class="rounded-lg bg-[#27272A] border border-[#3F3F46] px-5 py-2.5 text-[#A1A1AA] text-sm font-semibold hover:text-white hover:border-[#52525B] transition">
                                "Self-hosting guide"
                            </a>
                        </div>
                    </div>
                </section>

                // ── Self-host vs hosted ──
                <section class="px-6 py-20 flex flex-col items-center max-w-5xl mx-auto">
                    <h2 class="text-white text-3xl font-bold">"Two ways to run cocompute"</h2>
                    <p class="mt-2 text-[#71717A] text-base text-center max-w-xl">
                        "Same protocol either way. Pick whichever fits your trust model."
                    </p>

                    <div class="mt-12 grid grid-cols-1 md:grid-cols-2 gap-6 w-full">
                        // Self-host
                        <Card class="p-8 flex flex-col gap-6">
                            <div>
                                <h3 class="text-white text-2xl font-bold">"Self-host"</h3>
                                <p class="mt-2 text-emerald-500 text-4xl font-bold">"Free"</p>
                                <p class="mt-1 text-[#71717A] text-sm">"AGPLv3 · run your own orchestrator"</p>
                            </div>
                            <hr class="border-[#27272A]"/>
                            <ul class="flex flex-col gap-3.5">
                                <CheckItem text="Full source code, MIT-style stack on your terms"/>
                                <CheckItem text="Complete control over hosts and pools"/>
                                <CheckItem text="Run on your own infra, your own domain"/>
                                <CheckItem text="No telemetry, no third party"/>
                                <CheckItem text="Modify, fork, extend"/>
                            </ul>
                        </Card>
                        // Hosted on cocompute.ai
                        <div class="rounded-xl bg-[#16161E] border-2 border-indigo-500 p-8 flex flex-col gap-6">
                            <div>
                                <h3 class="text-white text-2xl font-bold">"cocompute.ai"</h3>
                                <p class="mt-2 text-emerald-500 text-4xl font-bold">"Free to start"</p>
                                <p class="mt-1 text-[#71717A] text-sm">"Hosted orchestrator · zero ops"</p>
                            </div>
                            <hr class="border-[#27272A]"/>
                            <ul class="flex flex-col gap-3.5">
                                <CheckItem text="One command to register a host" green=true/>
                                <CheckItem text="Join the public pool" green=true/>
                                <CheckItem text="OpenAI-compatible /v1/ API endpoint" green=true/>
                                <CheckItem text="No infra to operate" green=true/>
                                <CheckItem text="Optional paid tier coming for marketplace compute" green=true/>
                            </ul>
                        </div>
                    </div>

                    // ── CTA Footer ──
                    <div class="mt-16 w-full">
                        <hr class="border-[#1E1E26]"/>
                        <div class="flex flex-col items-center gap-4 pt-8" id="beta">
                            <h3 class="text-white text-xl font-semibold">
                                "Ready to run inference on your own terms?"
                            </h3>
                            <p class="text-[#71717A] text-sm">
                                "Free signup. No credit card."
                            </p>
                            <a href="/beta" class="mt-2 flex items-center gap-2 rounded-lg bg-indigo-500 px-8 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
                                <Icon name="sparkles" class="w-[18px] h-[18px]"/>
                                "Sign up"
                            </a>
                            <p class="mt-4 text-[#3F3F46] text-xs">
                                "© 2026 cocompute · "
                                <a href="https://github.com/dsegovia90/cocompute" target="_blank" rel="noopener" class="hover:text-[#A1A1AA] transition">"GitHub"</a>
                                " · AGPLv3"
                            </p>
                        </div>
                    </div>
                </section>
            </PageShell>
        </Base>
    }
}

pub async fn landing(
    State(state): State<crate::AppState>,
    jar: SignedCookieJar,
) -> Html<String> {
    let logged_in = jar.get(crate::auth::SESSION_COOKIE).is_some();
    let total_ms = state.total_compute_cache.get(&state.db).await;
    let total_compute = crate::web::total_compute::humanize_ms(total_ms);
    crate::web::render(Landing(LandingProps { logged_in, total_compute }))
}

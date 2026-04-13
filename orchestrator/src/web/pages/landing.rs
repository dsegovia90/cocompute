use crate::web::components::*;
use axum::response::Html;
use axum_extra::extract::cookie::SignedCookieJar;
use leptos::prelude::*;

#[component]
fn Landing(logged_in: bool) -> impl IntoView {
    view! {
        <Base title="cocompute">
            <PageShell>

                // ── Nav ──
                <nav class="flex items-center justify-between px-6 py-4">
                    <span class="text-white font-bold text-lg">"cocompute"</span>
                    <div class="flex items-center gap-5">
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
                                    "Request Beta Invite"
                                </a>
                            }.into_any()
                        }}
                    </div>
                </nav>

                // ── Hero ──
                <section class="flex flex-col items-center px-6 pt-20 pb-16">
                    <h1 class="text-white text-5xl font-bold text-center leading-tight max-w-3xl">
                        "Your GPU, your inference."<br/>"No cloud needed."
                    </h1>
                    <p class="mt-5 text-[#A1A1AA] text-base text-center max-w-xl leading-relaxed">
                        "cocompute turns your idle GPU into a personal AI inference server. Use your own hardware for free, or tap into the marketplace when you need more."
                    </p>
                    <div class="mt-10">
                        <a href="/beta" class="rounded-lg bg-indigo-500 px-7 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
                            "Request Beta Invite"
                        </a>
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

                // ── How it works ──
                <section class="px-6 py-16 flex flex-col items-center">
                    <h2 class="text-white text-3xl font-bold text-center">"How it works"</h2>
                    <p class="mt-3 text-[#71717A] text-base text-center">
                        "From your own GPU to a global compute network — in three simple steps."
                    </p>
                    <div class="mt-10 grid grid-cols-1 md:grid-cols-3 gap-5 w-full max-w-5xl">
                        <FeatureCard
                            icon="monitor"
                            title="Bring your GPU"
                            description="Install cocompute on any machine with a GPU. Your hardware becomes a personal inference server, accessible anywhere."
                            badge=("FREE", "bg-emerald-500/20 text-emerald-500")
                        />
                        <FeatureCard
                            icon="code"
                            title="Ollama-compatible API"
                            description="Drop-in replacement for Ollama. Same /v1/ API, works with all your existing tools and scripts."
                        />
                        <FeatureCard
                            icon="shopping-bag"
                            title="Marketplace"
                            description="Need more compute? Tap into GPUs shared by others. Or earn credits by sharing your idle GPU."
                        />
                    </div>
                </section>

                // ── Pricing ──
                <section class="px-6 py-20 flex flex-col items-center max-w-5xl mx-auto">
                    <h2 class="text-white text-3xl font-bold">"Simple pricing"</h2>
                    <p class="mt-2 text-[#71717A] text-base">
                        "Use your own hardware for free. Only pay when you need more."
                    </p>

                    <div class="mt-12 grid grid-cols-1 md:grid-cols-2 gap-6 w-full">
                        // Free tier
                        <div class="rounded-xl bg-[#16161E] border-2 border-indigo-500 p-8 flex flex-col gap-6">
                            <div>
                                <h3 class="text-white text-2xl font-bold">"Your GPU"</h3>
                                <p class="mt-2 text-emerald-500 text-4xl font-bold">"Free"</p>
                            </div>
                            <hr class="border-[#27272A]"/>
                            <ul class="flex flex-col gap-3.5">
                                <CheckItem text="Run models on your own GPU" green=true/>
                                <CheckItem text="Ollama-compatible /v1/ API" green=true/>
                                <CheckItem text="Access from anywhere" green=true/>
                                <CheckItem text="Earn credits by sharing idle compute" green=true/>
                                <CheckItem text="No credit card required" green=true/>
                            </ul>
                        </div>
                        // Marketplace tier
                        <Card class="p-8 flex flex-col gap-6">
                            <div>
                                <h3 class="text-white text-2xl font-bold">"Marketplace"</h3>
                                <p class="mt-2 text-white text-3xl font-bold">"Pay as you go"</p>
                                <p class="mt-1 text-[#71717A] text-sm">"Credits · only when you need more"</p>
                            </div>
                            <hr class="border-[#27272A]"/>
                            <ul class="flex flex-col gap-3.5">
                                <CheckItem text="Access GPUs shared by others"/>
                                <CheckItem text="Pay-as-you-go with credits"/>
                                <CheckItem text="Stripe fees passed through at cost"/>
                                <CheckItem text="Credits last weeks at typical usage"/>
                                <CheckItem text="Top up anytime, no subscription"/>
                            </ul>
                        </Card>
                    </div>

                    // ── CTA Footer ──
                    <div class="mt-16 w-full">
                        <hr class="border-[#1E1E26]"/>
                        <div class="flex flex-col items-center gap-4 pt-8" id="beta">
                            <h3 class="text-white text-xl font-semibold">
                                "Ready to run inference on your own terms?"
                            </h3>
                            <p class="text-[#71717A] text-sm">
                                "Invite-only beta — limited spots available."
                            </p>
                            <a href="/beta" class="mt-2 flex items-center gap-2 rounded-lg bg-indigo-500 px-8 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
                                <Icon name="sparkles" class="w-[18px] h-[18px]"/>
                                "Request Beta Invite"
                            </a>
                            <p class="mt-4 text-[#3F3F46] text-xs">"© 2026 cocompute"</p>
                        </div>
                    </div>
                </section>
            </PageShell>
        </Base>
    }
}

pub async fn landing(jar: SignedCookieJar) -> Html<String> {
    let logged_in = jar.get(crate::auth::SESSION_COOKIE).is_some();
    crate::web::render(Landing(LandingProps { logged_in }))
}

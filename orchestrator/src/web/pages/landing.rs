use axum::{extract::State, response::Html};
use axum_extra::extract::cookie::SignedCookieJar;
use leptos::prelude::*;

use crate::web::components::*;

// The landing page is split into one component per section. Each returns an erased
// AnyView (`.into_any()`) so no single Leptos view-tree type gets deep enough to hit
// rustc's recursion limit, and so each section reads on its own.

#[component]
fn Nav(logged_in: bool) -> impl IntoView {
    view! {
        <nav class="flex items-center justify-between px-6 py-4">
            <span class="text-white font-bold text-lg">"cocompute"</span>
            <div class="flex items-center gap-5">
                // Status link is out-of-band (UptimeRobot): the page rendering at all
                // means the orchestrator is up, and the status page stays reachable
                // when cocompute.ai is down.
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
                        <a href="/signup" class="hidden md:inline-block rounded-lg bg-indigo-500 px-5 py-2.5 text-white text-sm font-semibold hover:bg-indigo-600 transition">
                            "Sign up"
                        </a>
                    }.into_any()
                }}
            </div>
        </nav>
    }
    .into_any()
}

#[component]
fn ConnectionCard(api_url: String) -> impl IntoView {
    let tools = [
        "Open WebUI",
        "AnythingLLM",
        "Jan",
        "LibreChat",
        "LangChain",
        "Continue",
        "n8n",
        "your code",
    ];
    view! {
        <div class="flex flex-col rounded-xl bg-[#0D0D13] border border-[#27272A] overflow-hidden shadow-xl text-left">
            <div class="flex items-center gap-3 px-4 h-9 shrink-0 bg-[#1A1A22] border-b border-[#27272A]">
                <span class="flex gap-1.5">
                    <span class="w-3 h-3 rounded-full bg-[#FF5F57]"></span>
                    <span class="w-3 h-3 rounded-full bg-[#FEBC2E]"></span>
                    <span class="w-3 h-3 rounded-full bg-[#28C840]"></span>
                </span>
                <span class="rounded-md bg-[#0D0D13] border border-[#27272A] px-2.5 py-1 text-[#A1A1AA] text-xs font-medium">"connect"</span>
            </div>
            <div class="p-4 flex flex-col gap-4">
                <div>
                    <div class="text-[#52525B] text-[11px] font-semibold uppercase tracking-wider mb-1">"Base URL"</div>
                    <div class="font-mono text-xs text-[#67e8f9] bg-[#111118] border border-[#27272A] rounded-md px-3 py-2 break-all">{api_url}</div>
                </div>
                <div>
                    <div class="text-[#52525B] text-[11px] font-semibold uppercase tracking-wider mb-1">"API key"</div>
                    <div class="font-mono text-xs text-[#67e8f9] bg-[#111118] border border-[#27272A] rounded-md px-3 py-2 break-all">"9f2a3b1c••••••••••••••••"</div>
                </div>
                <div>
                    <div class="text-[#71717A] text-xs mb-2">"Works with anything that speaks the OpenAI API spec"</div>
                    <div class="flex flex-wrap gap-1.5">
                        {tools.iter().map(|t| view! {
                            <span class="rounded-md bg-[#16161E] border border-[#27272A] px-2.5 py-1 text-[#A1A1AA] text-xs font-medium">{*t}</span>
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}

#[component]
fn Hero(logged_in: bool, total_compute: String, base_url: String) -> impl IntoView {
    let host_install_cmd = format!(
        r#"curl -sSf {base_url}/install.sh | COCOMPUTE_URL={base_url} bash -s -- --token YOUR_TOKEN"#
    );
    let api_url = format!("{base_url}/v1");
    view! {
        <section class="flex flex-col items-center px-6 pt-20 pb-16">
            <div class="mb-5 inline-flex items-center gap-2 rounded-full bg-[#16161E] border border-[#27272A] px-3 py-1.5">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
                <span class="text-[#A1A1AA] text-xs font-medium">"Open source · AGPLv3 · Self-host or hosted · No crypto"</span>
            </div>
            <h1 class="text-white text-5xl font-bold text-center leading-tight max-w-4xl text-balance">
                "The fastest way to safely expose your local inference to the internet."
            </h1>
            <p class="mt-5 text-[#A1A1AA] text-base text-center max-w-xl leading-relaxed">
                "Run one command and your Ollama instance (more backends coming soon) becomes an OpenAI-API-compatible endpoint, reachable from anywhere. No open ports, no reverse proxy, nothing to wire up. Your apps just point at a URL and a key."
            </p>
            <div class="mt-10 flex flex-wrap items-center justify-center gap-3">
                {if logged_in {
                    view! {
                        <a href="/dashboard" class="rounded-lg bg-indigo-500 px-7 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
                            "Go to dashboard"
                        </a>
                    }.into_any()
                } else {
                    view! {
                        <a href="/signup" class="rounded-lg bg-indigo-500 px-7 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
                            "Sign up"
                        </a>
                    }.into_any()
                }}
                <a href="https://github.com/dsegovia90/cocompute" target="_blank" rel="noopener" class="rounded-lg bg-[#27272A] border border-[#3F3F46] px-7 py-3.5 text-[#A1A1AA] font-semibold hover:text-white hover:border-[#52525B] transition flex items-center gap-2">
                    <Icon name="github" class="w-[18px] h-[18px]"/>
                    "GitHub"
                </a>
            </div>

            <div class="mt-8 inline-flex items-center gap-2 rounded-full bg-[#16161E] border border-[#27272A] px-4 py-2">
                <span class="text-[#A1A1AA] text-xs font-medium">"Total time computed"</span>
                <span class="text-white text-xs font-semibold tabular-nums">{total_compute}</span>
            </div>

            <div class="mt-12 w-full max-w-4xl grid grid-cols-1 lg:grid-cols-2 gap-5 items-stretch">
                <div class="flex flex-col">
                    <div class="text-xs font-bold uppercase tracking-wider text-emerald-400 mb-2">"On your machine"</div>
                    <div class="flex-1 min-h-0">
                        <CodeWindow title="bash" code={host_install_cmd}/>
                    </div>
                    <p class="mt-2 text-[#52525B] text-xs">"Grab your token after signup, from the dashboard. Linux + macOS, background service."</p>
                </div>
                <div class="flex flex-col">
                    <div class="text-xs font-bold uppercase tracking-wider text-indigo-400 mb-2">"From any app"</div>
                    <ConnectionCard api_url=api_url/>
                    <p class="mt-2 text-[#52525B] text-xs">"Set the base URL and a key. That's the whole integration."</p>
                </div>
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn SafeByDesign() -> impl IntoView {
    view! {
        <section class="px-6 py-16 flex flex-col items-center">
            <h2 class="text-white text-3xl font-bold text-center">"Safe by design"</h2>
            <p class="mt-3 text-[#71717A] text-base text-center max-w-md text-balance">
                "Your machine stays closed. The orchestrator is the only thing facing the internet."
            </p>
            <div class="mt-10 grid grid-cols-1 md:grid-cols-3 gap-5 w-full max-w-5xl">
                <FeatureCard
                    icon="lock"
                    title="No open ports"
                    description="Your machine dials OUT over an encrypted connection (built on iroh). Nothing inbound to port-forward or scan. Your home firewall stays shut."
                />
                <FeatureCard
                    icon="eye-off"
                    title="Your home IP stays hidden"
                    description="Apps hit cocompute's stable HTTPS endpoint. Your host sits behind it, anonymous, and can move anytime without anyone noticing."
                />
                <FeatureCard
                    icon="route"
                    title="Revocable keys"
                    description="256-bit API keys, hashed at rest, scoped to a pool. Rotate or revoke in a click. Single-use, expiring tokens claim each host."
                />
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn ShareSection() -> impl IntoView {
    view! {
        <section class="px-6 py-16 flex flex-col items-center">
            <h2 class="text-white text-3xl font-bold text-center">"Share it when you want"</h2>
            <p class="mt-3 text-[#71717A] text-base text-center max-w-xl text-balance">
                "Pool with friends or your team. Share your endpoint, or tap into theirs, in a few clicks."
            </p>

            <div
                id="network-sc"
                class="mt-10 w-full max-w-4xl rounded-2xl overflow-hidden max-sm:aspect-auto max-sm:min-h-[720px]"
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

            <div class="mt-8 inline-flex items-center gap-2 rounded-full bg-[#16161E] border border-[#27272A] px-4 py-2">
                <span class="rounded-md bg-indigo-500/20 text-indigo-400 text-xs font-bold px-2 py-0.5">"Coming soon"</span>
                <span class="text-[#A1A1AA] text-xs font-medium">"A public marketplace: earn for the compute you share, or pay to burst onto others' GPUs."</span>
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn Mission() -> impl IntoView {
    view! {
        <section class="px-6 py-16 flex flex-col items-center">
            <h2 class="text-white text-3xl font-bold text-center">"Why we built cocompute"</h2>
            <div class="mt-10 grid grid-cols-1 md:grid-cols-2 gap-12 max-w-4xl w-full">
                <div>
                    <h3 class="text-white text-lg font-bold mb-2">"Open models win when they're easy"</h3>
                    <p class="text-[#A1A1AA] text-sm leading-relaxed">
                        "Open-source models are catching up fast, but self-hosting one and reaching it from your apps is still too much work. Every bit of friction we remove is a reason to pick open over a closed API. That is the whole point of the one-command setup."
                    </p>
                </div>
                <div>
                    <h3 class="text-white text-lg font-bold mb-2">"Compute should be shared"</h3>
                    <p class="text-[#A1A1AA] text-sm leading-relaxed">
                        "GPUs sit idle everywhere while people pay cloud premiums for the same cycles. We are building toward an open marketplace: share what you have, use what you need, no gatekeeper taking a cut. Not yet, but it is where this is headed."
                    </p>
                </div>
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn OpenSource() -> impl IntoView {
    view! {
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
                        "Quickstart"
                    </a>
                </div>
            </div>
        </section>
    }
    .into_any()
}

#[component]
fn TwoWays() -> impl IntoView {
    view! {
        <section class="px-6 py-20 flex flex-col items-center max-w-5xl mx-auto">
            <h2 class="text-white text-3xl font-bold">"Two ways to run cocompute"</h2>
            <p class="mt-2 text-[#71717A] text-base text-center max-w-xl text-balance">
                "Same protocol either way. Let us run it, or self-host for full control."
            </p>

            <div class="mt-12 grid grid-cols-1 md:grid-cols-2 gap-6 w-full">
                <Card class="p-8 flex flex-col gap-6">
                    <div>
                        <h3 class="text-white text-2xl font-bold">"Self-host"</h3>
                        <p class="mt-2 text-emerald-500 text-4xl font-bold">"Free"</p>
                        <p class="mt-1 text-[#71717A] text-sm">"AGPLv3 · run your own orchestrator"</p>
                    </div>
                    <hr class="border-[#27272A]"/>
                    <ul class="flex flex-col gap-3.5">
                        <CheckItem text="Full source code, nothing hidden"/>
                        <CheckItem text="Complete control over hosts and pools"/>
                        <CheckItem text="Run on your own infra, your own domain"/>
                        <CheckItem text="No telemetry, no third party"/>
                        <CheckItem text="Modify and extend, changes stay open"/>
                    </ul>
                </Card>
                <div class="rounded-xl bg-[#16161E] border-2 border-indigo-500 p-8 flex flex-col gap-6">
                    <div>
                        <h3 class="text-white text-2xl font-bold">"cocompute.ai"</h3>
                        <p class="mt-2 text-emerald-500 text-4xl font-bold">"Free"</p>
                        <p class="mt-1 text-[#71717A] text-sm">"The easy way · we run it"</p>
                    </div>
                    <hr class="border-[#27272A]"/>
                    <ul class="flex flex-col gap-3.5">
                        <CheckItem text="One command to expose a host" green=true/>
                        <CheckItem text="OpenAI-API-compatible /v1/ endpoint" green=true/>
                        <CheckItem text="No open ports, no infra to operate" green=true/>
                        <CheckItem text="Share with friends or your team" green=true/>
                        <CheckItem text="Paid public marketplace coming soon" green=true/>
                    </ul>
                </div>
            </div>

            <div class="mt-16 w-full">
                <hr class="border-[#1E1E26]"/>
                <div class="flex flex-col items-center gap-4 pt-8" id="signup">
                    <h3 class="text-white text-xl font-semibold">
                        "Expose your local model in the next five minutes."
                    </h3>
                    <p class="text-[#71717A] text-sm">
                        "Free signup. No credit card."
                    </p>
                    <a href="/signup" class="mt-2 flex items-center gap-2 rounded-lg bg-indigo-500 px-8 py-3.5 text-white font-semibold hover:bg-indigo-600 transition">
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
    }
    .into_any()
}

#[component]
fn Landing(logged_in: bool, total_compute: String, base_url: String) -> impl IntoView {
    view! {
        <Base title="cocompute · safely expose your local inference">
            <PageShell>
                <Nav logged_in=logged_in/>
                <Hero logged_in=logged_in total_compute=total_compute base_url=base_url/>
                <SafeByDesign/>
                <ShareSection/>
                <Mission/>
                <OpenSource/>
                <TwoWays/>
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
    crate::web::render(Landing(LandingProps {
        logged_in,
        total_compute,
        base_url: state.base_url.clone(),
    }))
}

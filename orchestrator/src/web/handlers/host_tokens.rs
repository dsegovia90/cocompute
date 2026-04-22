use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use leptos::prelude::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::{
    auth::{self, CurrentUser},
    db::entities::hosts,
    web::components::*,
    AppState,
};

#[component]
fn HostTokenPage(token: String, endpoint_id: String, base_url: String, is_dev: bool) -> impl IntoView {
    let base_cmd = format!(
        "curl -sSf {base_url}/install.sh | bash -s -- --token {token}"
    );
    let dev_base_cmd = format!(
        "cargo run -p cocompute_host -- --orchestrator-url {base_url} --setup-token {token}"
    );
    view! {
        <Base title="cocompute — host setup">
            <PageShell>
                <div class="max-w-2xl mx-auto px-6 py-10">
                    <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-8 flex flex-col gap-6">
                        <div>
                            <h1 class="text-white text-xl font-bold">"Add Host"</h1>
                            <p class="text-[#71717A] text-sm mt-1">"Register a new machine to your account"</p>
                        </div>

                        // Ollama config (optional)
                        <div class="flex flex-col gap-2">
                            <p class="text-[#A1A1AA] text-xs font-medium">"Ollama location (optional, for remote Ollama instances)"</p>
                            <div class="flex gap-2">
                                <input
                                    id="ollama-url"
                                    type="text"
                                    placeholder="http://localhost"
                                    class="h-8 flex-1 rounded-lg bg-[#111118] border border-[#27272A] px-3 text-white text-xs placeholder-[#3F3F46] outline-none focus:border-indigo-500 transition"
                                    oninput="updateCmds()"
                                />
                                <input
                                    id="ollama-port"
                                    type="text"
                                    placeholder="11434"
                                    class="h-8 w-24 rounded-lg bg-[#111118] border border-[#27272A] px-3 text-white text-xs placeholder-[#3F3F46] outline-none focus:border-indigo-500 transition"
                                    oninput="updateCmds()"
                                />
                            </div>
                        </div>

                        // Install command
                        <div>
                            <p class="text-[#A1A1AA] text-sm mb-3">"Run this command on the machine you want to add:"</p>
                            <div id="curl-cmd" class="bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-xs text-[#67e8f9] break-all select-all">
                                {base_cmd.clone()}
                            </div>
                        </div>

                        {is_dev.then(|| view! {
                            <div>
                                <p class="text-[#A1A1AA] text-sm mb-3">"Or run from source (dev):"</p>
                                <div id="dev-cmd" class="bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-xs text-amber-400 break-all select-all">
                                    {dev_base_cmd.clone()}
                                </div>
                            </div>
                        })}

                        <div class="text-[#52525B] text-xs">
                            <p>"This token expires in 1 hour and can only be used once."</p>
                            <p class="mt-1">"Once registered, you can add the host to any of your pools from the dashboard."</p>
                        </div>
                        <a href="/dashboard" class="text-indigo-500 text-sm font-medium hover:underline">"Back to dashboard"</a>
                    </div>
                </div>

                <script>{format!(r#"
                    var baseCmd = `{base_cmd}`;
                    var devCmd = `{dev_base_cmd}`;
                    function updateCmds() {{
                        var url = document.getElementById('ollama-url').value.trim();
                        var port = document.getElementById('ollama-port').value.trim();
                        var extra = '';
                        if (url) extra += ' --ollama-url ' + url;
                        if (port) extra += ' --ollama-port ' + port;
                        document.getElementById('curl-cmd').textContent = baseCmd + extra;
                        var dev = document.getElementById('dev-cmd');
                        if (dev) dev.textContent = devCmd + extra;
                    }}
                "#)}</script>
            </PageShell>
        </Base>
    }
}

/// Generate a one-time host setup token that ties a host to the current user.
pub async fn create_host_token(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Response {
    let raw_token = auth::generate_api_key();
    let token_hash = auth::hash_key(&raw_token);

    let host_token = crate::db::entities::host_tokens::ActiveModel {
        token_hash: Set(token_hash),
        user_id: Set(user.id),
        created_at: Set(chrono::Utc::now()),
        expires_at: Set(chrono::Utc::now() + chrono::Duration::hours(1)),
        ..Default::default()
    };

    if let Err(e) = host_token.insert(&state.db).await {
        tracing::error!("failed to create host token: {e}");
        return Redirect::to("/dashboard").into_response();
    }

    Html(crate::web::render(HostTokenPage(HostTokenPageProps {
        token: raw_token,
        endpoint_id: state.endpoint_id.clone(),
        base_url: state.base_url.clone(),
        is_dev: cfg!(debug_assertions),
    })).0).into_response()
}

#[derive(Deserialize)]
pub struct RenameHostForm {
    pub name: String,
}

/// Rename a host. Requires host ownership.
pub async fn rename_host(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(endpoint_id): axum::extract::Path<String>,
    Form(form): Form<RenameHostForm>,
) -> Response {
    let host = hosts::Entity::find()
        .filter(hosts::Column::EndpointId.eq(&endpoint_id))
        .one(&state.db)
        .await;

    let host = match host {
        Ok(Some(h)) if h.user_id == Some(user.id) => h,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    let name = form.name.trim().to_string();
    let name = if name.is_empty() { None } else { Some(name) };

    let mut active: hosts::ActiveModel = host.into();
    active.name = Set(name);
    let _ = active.update(&state.db).await;

    Redirect::to("/dashboard?saved=true").into_response()
}

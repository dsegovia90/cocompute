use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
};
use leptos::prelude::*;
use sea_orm::{ActiveModelTrait, Set};

use crate::{
    auth::{self, CurrentUser},
    web::components::*,
    AppState,
};

#[component]
fn HostTokenPage(token: String, endpoint_id: String, base_url: String, is_dev: bool) -> impl IntoView {
    let curl_cmd = format!(
        "curl -sSf {base_url}/install.sh | bash -s -- --token {token} --orchestrator {endpoint_id}"
    );
    let cargo_cmd = format!(
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
                        <div>
                            <p class="text-[#A1A1AA] text-sm mb-3">"Run this command on the machine you want to add:"</p>
                            <div class="bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-xs text-[#67e8f9] break-all select-all">
                                {curl_cmd}
                            </div>
                        </div>

                        {is_dev.then(|| view! {
                            <div>
                                <p class="text-[#A1A1AA] text-sm mb-3">"Or run from source (dev):"</p>
                                <div class="bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-xs text-amber-400 break-all select-all">
                                    {cargo_cmd}
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

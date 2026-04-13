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
    db::entities::{api_keys, pool_members, pools},
    web::components::*,
    AppState,
};

#[derive(Deserialize)]
pub struct CreateApiKeyForm {
    label: Option<String>,
}

#[component]
fn ApiKeyCreatedPage(key: String, pool_name: String, label: String) -> impl IntoView {
    view! {
        <Base title="cocompute — API key created">
            <PageShell>
                <div class="max-w-2xl mx-auto px-6 py-10">
                    <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-8 flex flex-col gap-6">
                        <div>
                            <h1 class="text-white text-xl font-bold">"API Key Created"</h1>
                            <p class="text-[#71717A] text-sm mt-1">{format!("Pool: {pool_name}")}</p>
                        </div>
                        <div>
                            <p class="text-[#A1A1AA] text-sm mb-3">"Save this key now. It won't be shown again."</p>
                            <div class="bg-[#111118] border border-[#27272A] rounded-lg p-4 font-mono text-sm text-[#67e8f9] break-all select-all">
                                {key}
                            </div>
                        </div>
                        {(!label.is_empty()).then(|| view! {
                            <p class="text-[#52525B] text-xs">{format!("Label: {label}")}</p>
                        })}
                        <a href="/dashboard" class="text-indigo-500 text-sm font-medium hover:underline">"Back to dashboard"</a>
                    </div>
                </div>
            </PageShell>
        </Base>
    }
}

/// Create a pool-scoped API key.
pub async fn create_pool_api_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(pool_pid): axum::extract::Path<String>,
    Form(form): Form<CreateApiKeyForm>,
) -> Response {
    let pool = pools::Entity::find()
        .filter(pools::Column::Pid.eq(&pool_pid))
        .one(&state.db)
        .await;

    let pool = match pool {
        Ok(Some(p)) => p,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    // Check user is owner or member
    let is_member = pool_members::Entity::find()
        .filter(pool_members::Column::PoolId.eq(pool.id))
        .filter(pool_members::Column::UserId.eq(user.id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .is_some();

    if !is_member {
        return Redirect::to("/dashboard").into_response();
    }

    let raw_key = auth::generate_api_key();
    let key_hash = auth::hash_key(&raw_key);

    let label = form.label.filter(|l| !l.is_empty());

    let api_key = api_keys::ActiveModel {
        key_hash: Set(key_hash),
        created_at: Set(chrono::Utc::now()),
        user_id: Set(Some(user.id)),
        pool_id: Set(Some(pool.id)),
        label: Set(label.clone()),
        ..Default::default()
    };

    if let Err(e) = api_key.insert(&state.db).await {
        tracing::error!("failed to create API key: {e}");
        return Redirect::to("/dashboard").into_response();
    }

    Html(crate::web::render(ApiKeyCreatedPage(ApiKeyCreatedPageProps {
        key: raw_key,
        pool_name: pool.name,
        label: label.unwrap_or_default(),
    })).0).into_response()
}

/// Create a global pool API key. Requires membership in the global pool.
pub async fn create_global_api_key(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<CreateApiKeyForm>,
) -> Response {
    // Find the global pool
    let global_pool = pools::Entity::find()
        .filter(pools::Column::IsGlobal.eq(true))
        .one(&state.db)
        .await;

    let global_pool = match global_pool {
        Ok(Some(p)) => p,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    // Verify user is a member of the global pool
    let is_member = pool_members::Entity::find()
        .filter(pool_members::Column::PoolId.eq(global_pool.id))
        .filter(pool_members::Column::UserId.eq(user.id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .is_some();

    if !is_member {
        return Redirect::to("/dashboard").into_response();
    }

    let raw_key = auth::generate_api_key();
    let key_hash = auth::hash_key(&raw_key);

    let label = form.label.filter(|l| !l.is_empty());

    let api_key = api_keys::ActiveModel {
        key_hash: Set(key_hash),
        created_at: Set(chrono::Utc::now()),
        user_id: Set(Some(user.id)),
        pool_id: Set(Some(global_pool.id)),
        label: Set(label.clone()),
        ..Default::default()
    };

    if let Err(e) = api_key.insert(&state.db).await {
        tracing::error!("failed to create global API key: {e}");
        return Redirect::to("/dashboard").into_response();
    }

    Html(crate::web::render(ApiKeyCreatedPage(ApiKeyCreatedPageProps {
        key: raw_key,
        pool_name: "Global Pool".to_string(),
        label: label.unwrap_or_default(),
    })).0).into_response()
}

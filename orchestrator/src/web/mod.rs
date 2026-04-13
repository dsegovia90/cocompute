pub mod asset_hash;
mod components;
mod handlers;
mod pages;

use axum::{Router, response::Html, routing::{get, post}};
use leptos::prelude::*;

pub fn router(static_dir: &str) -> Router<crate::AppState> {
    Router::new()
        .route("/", get(pages::landing::landing))
        .route("/beta", get(pages::beta::beta).post(handlers::post_beta))
        .route("/login", get(pages::login::login).post(handlers::post_login))
        .route("/logout", post(handlers::post_logout))
        .route("/verify", get(pages::verify::verify_page).post(handlers::post_verify))
        .route("/forgot", get(pages::forgot::forgot_page).post(handlers::post_forgot))
        .route("/reset", get(pages::reset::reset_page).post(handlers::post_reset))
        // Dashboard + pool management (authenticated)
        .route("/dashboard", get(pages::dashboard::dashboard))
        .route("/pools", post(handlers::create_pool))
        .route("/host-token", post(handlers::create_host_token))
        .route("/pools/{pool_pid}/rename", post(handlers::rename_pool))
        .route("/pools/{pool_pid}/api-keys", post(handlers::create_pool_api_key))
        .route("/pools/{pool_pid}/invite", post(handlers::invite_member))
        .route("/pools/{pool_pid}/accept", get(handlers::accept_invite))
        .route("/pools/{pool_pid}/add-host", post(handlers::add_host_to_pool))
        .route("/api-keys/global", post(handlers::create_global_api_key))
        .nest_service("/static", tower_http::services::ServeDir::new(static_dir))
}

pub fn render(view: impl IntoView) -> Html<String> {
    let html = view.to_html();
    Html(format!("<!DOCTYPE html>{html}"))
}

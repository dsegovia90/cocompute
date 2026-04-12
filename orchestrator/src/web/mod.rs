pub mod asset_hash;
mod beta;
mod components;
mod forgot;
mod handlers;
mod landing;
mod login;
mod reset;
mod verify;

use axum::{Router, response::Html, routing::{get, post}};
use leptos::prelude::*;

pub fn router(static_dir: &str) -> Router<crate::AppState> {
    Router::new()
        .route("/", get(landing::landing))
        .route("/beta", get(beta::beta).post(handlers::post_beta))
        .route("/login", get(login::login).post(handlers::post_login))
        .route("/logout", post(handlers::post_logout))
        .route("/verify", get(verify::verify_page).post(handlers::post_verify))
        .route("/forgot", get(forgot::forgot_page).post(handlers::post_forgot))
        .route("/reset", get(reset::reset_page).post(handlers::post_reset))
        .nest_service("/static", tower_http::services::ServeDir::new(static_dir))
}

pub fn render(view: impl IntoView) -> Html<String> {
    let html = view.to_html();
    Html(format!("<!DOCTYPE html>{html}"))
}

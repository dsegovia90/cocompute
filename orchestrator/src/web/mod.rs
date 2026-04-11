mod beta;
mod components;
mod landing;
mod login;

use axum::{Router, response::Html, routing::get};
use leptos::prelude::*;

pub fn router(static_dir: &str) -> Router<crate::AppState> {
    Router::new()
        .route("/", get(landing::landing))
        .route("/beta", get(beta::beta))
        .route("/login", get(login::login))
        .nest_service("/static", tower_http::services::ServeDir::new(static_dir))
}

pub fn render(view: impl IntoView) -> Html<String> {
    let html = view.to_html();
    Html(format!("<!DOCTYPE html>{html}"))
}

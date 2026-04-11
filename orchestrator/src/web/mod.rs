mod beta;
mod components;
mod landing;
mod login;

use axum::{Router, routing::get};

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(landing::landing))
        .route("/beta", get(beta::beta))
        .route("/login", get(login::login))
        .nest_service("/static", tower_http::services::ServeDir::new("orchestrator/static"))
}
